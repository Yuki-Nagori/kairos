//! shell 引号处理。
//!
//! 拼命令的**领域逻辑**不在这里（哪些命令、什么顺序、传给谁属于对应领域的
//! `services/` 模块），本模块只提供引号原语。

/// 单引号**内部**的转义：把 `'` 变成 `'\''`。
///
/// 只做引号内转义，不负责加外层引号——两者分开，避免调用方以为拿到的是
/// 「已安全」的片段却漏了外层引号。需要完整的安全参数用 [`bash_quote`]。
pub fn bash_single_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

/// 单引号包裹一个参数（含转义）。
pub fn bash_quote(value: &str) -> String {
    format!("'{}'", bash_single_quote(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_embedded_single_quote() {
        assert_eq!(bash_single_quote("a'b"), "a'\\''b");
        assert_eq!(bash_single_quote("'"), "'\\''");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(bash_single_quote("plain text"), "plain text");
        assert_eq!(bash_single_quote(""), "");
    }

    #[test]
    fn escapes_every_quote_in_the_value() {
        assert_eq!(bash_single_quote("a'b'c"), "a'\\''b'\\''c");
    }

    #[test]
    fn bash_quote_wraps_and_escapes_together() {
        assert_eq!(bash_quote("/var/case"), "'/var/case'");
        assert_eq!(bash_quote("/tmp/it's here"), "'/tmp/it'\\''s here'");
    }

    #[test]
    fn bash_quote_of_empty_value_is_an_empty_argument() {
        assert_eq!(bash_quote(""), "''");
    }
}
