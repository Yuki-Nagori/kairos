//! 正则的集中编译与缓存。
//!
//! `Regex::new` 每次调用都重新编译（典型模式几十微秒），放在循环或每帧路径上
//! 是明确的性能缺陷；分散编译还会让同一模式的两份字面量悄悄漂移。低频 / 模式
//! 来自变量的场合走 [`compiled`]（带缓存），高频固定模式用 [`patterns`] 里的
//! 静态量（`LazyLock`，全局只编译一次）。
//!
//! 能不用正则就不用：单字符切分（`split_once`）、前后缀剥离（`strip_prefix` /
//! `strip_suffix`）、定宽列切片都比正则快且更易读，正则留给「多分支 / 重复组 /
//! 需要捕获」的场合。

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use regex::Regex;

use crate::error::{KairosError, Result};

/// 已编译模式的缓存：首次访问才建表。
static CACHE: LazyLock<Mutex<HashMap<String, Arc<Regex>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 取模式对应的正则；未缓存过则编译后放入缓存。
///
/// 返回 `Arc<Regex>` 而非 `&'static Regex`：后者要么 `Box::leak` 无限泄漏，
/// 要么把缓存做成自引用结构，都不划算。
pub fn compiled(pattern: &str) -> Result<Arc<Regex>> {
    if let Some(cached) = lookup(pattern) {
        return Ok(cached);
    }
    let regex = compile(pattern)?;
    store(pattern, &regex);
    Ok(regex)
}

/// 文本是否命中模式。
pub fn is_match(pattern: &str, text: &str) -> Result<bool> {
    Ok(compiled(pattern)?.is_match(text))
}

/// 查缓存。与 [`store`] 分开是为了让编译落在临界区之外——编译可能耗时，
/// 不该占着锁。
fn lookup(pattern: &str) -> Option<Arc<Regex>> {
    lock_cache().get(pattern).map(Arc::clone)
}

fn store(pattern: &str, regex: &Arc<Regex>) {
    lock_cache().insert(pattern.to_string(), Arc::clone(regex));
}

/// 取缓存锁。临界区内只有哈希表查表与插入、不含 panic 落点，故中毒本身不影响
/// 缓存内容——取回内部值比让此后每次正则调用都 panic 更可诊断。
fn lock_cache() -> MutexGuard<'static, HashMap<String, Arc<Regex>>> {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 模式是调用方给的输入，编译失败按参数不合法归类。
fn compile(pattern: &str) -> Result<Arc<Regex>> {
    match Regex::new(pattern) {
        Ok(regex) => Ok(Arc::new(regex)),
        Err(error) => Err(invalid_pattern(pattern, &error)),
    }
}

fn invalid_pattern(pattern: &str, error: &regex::Error) -> KairosError {
    KairosError::validation(format!("无效的正则表达式「{pattern}」：{error}"))
}

/// 预定义模式：高频固定模式在此编译一次，命名用 `SCREAMING_SNAKE_CASE`。
///
/// 每个模式都要有命中的正例与不应命中的边界样例，新增模式必须同时补测试。
pub mod patterns {
    use std::sync::LazyLock;

    use regex::Regex;

    /// 数值 token：整数、小数与科学计数法（求解器文本里的场值与字典数值）。
    pub static FLOAT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?").expect("内置模式必须可编译")
    });

    /// 字典条目 `key value;`：键以字母或下划线开头，值为行内其余内容。
    pub static DICT_ENTRY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s+(.+?);\s*$").expect("内置模式必须可编译")
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    #[test]
    fn compiled_hits_cache_on_second_call() {
        let first = compiled("^cached-[0-9]+$").unwrap();
        let second = compiled("^cached-[0-9]+$").unwrap();
        // 命中缓存即返回同一个实例，说明没有重新编译
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn compiled_rejects_invalid_pattern() {
        let error = compiled("(未闭合").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Validation);
        assert!(error.message().contains("无效的正则表达式"));
    }

    #[test]
    fn invalid_pattern_message_quotes_pattern_and_reason() {
        // 经变量传入：clippy 的 invalid_regex 只查字面量，这里本就是要构造失败
        let broken = "(未闭合";
        let error = invalid_pattern(broken, &Regex::new(broken).unwrap_err());
        assert_eq!(error.kind(), ErrorKind::Validation);
        assert!(error.message().contains("「(未闭合」"));
    }

    #[test]
    fn the_same_pattern_compiles_once_across_calls() {
        // 两次调用之间只应发生一次编译：第二次走缓存分支并直接返回
        assert!(compiled("^once$").is_ok());
        assert!(compiled("^once$").is_ok());
    }

    #[test]
    fn is_match_reports_hit_and_miss() {
        assert!(is_match(r"^\d+$", "123").unwrap());
        assert!(!is_match(r"^\d+$", "12a").unwrap());
    }

    #[test]
    fn is_match_propagates_invalid_pattern() {
        let error = is_match("(未闭合", "任意").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Validation);
    }

    #[test]
    fn builtin_float_pattern_matches_numbers() {
        assert!(patterns::FLOAT.is_match("2.5e-3"));
        assert!(patterns::FLOAT.is_match("-7"));
        assert!(!patterns::FLOAT.is_match("abc"));
    }

    #[test]
    fn builtin_dict_entry_pattern_captures_key_and_value() {
        let captured = patterns::DICT_ENTRY.captures("nu 0.01;").unwrap();
        assert_eq!(&captured[1], "nu");
        assert_eq!(&captured[2], "0.01");
        assert!(patterns::DICT_ENTRY.captures("缺分号").is_none());
    }
}
