//! 宿主环境探测的**纯解析**部分：命令输出 / 系统文件内容 → 结构化值。
//!
//! 起进程与读文件留在适配层（那里无覆盖率门槛，也不必为平台差异造桩）；这里只放
//! 「拿到一段文本之后怎么解读」。收益是**解析逻辑在三端都能测**——原先 macOS 的分支
//! 只有在 macOS 上跑得到，Windows / Linux 的分支在 macOS 开发机上等于没有覆盖。

/// 字节 → GiB 的除数（`sysctl -n hw.memsize`、PowerShell CIM 都返回字节）。
const BYTES_PER_GIB: u64 = 1024 * 1024 * 1024;
/// kB → GiB 的除数（`/proc/meminfo` 以 kB 计）。
const KB_PER_GIB: u64 = 1024 * 1024;

/// 从 `/etc/localtime` 软链目标里取 IANA 时区名：
/// `…/zoneinfo/Asia/Shanghai` → `Asia/Shanghai`。
///
/// 取的是「名字」而不是「路径」——这里的 `zoneinfo/` 是 POSIX 软链目标的字面标记，
/// 不涉及 Windows 分隔符语义，故按文本处理而非走 `Path`。
/// 目标以 `zoneinfo/` 结尾（退化的软链）时返回空串，与调用方原有行为一致。
pub fn zoneinfo_name(target: &str) -> Option<String> {
    let (_, zone) = target.split_once("zoneinfo/")?;
    Some(zone.to_string())
}

/// 解析「裸字节数」文本（macOS `sysctl -n hw.memsize` 与 Windows PowerShell CIM
/// 的输出形态相同），向下取整到 GiB。非数字 / 空文本返回 `None`。
pub fn memory_gib_from_bytes(text: &str) -> Option<u32> {
    let bytes: u64 = text.trim().parse().ok()?;
    Some((bytes / BYTES_PER_GIB) as u32)
}

/// 从 `/proc/meminfo` 文本里读 `MemTotal:`（kB）并向下取整到 GiB。
/// 缺该行 / 行内无数字时返回 `None`。
pub fn memory_gib_from_meminfo(text: &str) -> Option<u32> {
    let line = text.lines().find(|line| line.starts_with("MemTotal:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some((kb / KB_PER_GIB) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoneinfo_name_takes_the_segment_after_marker() {
        assert_eq!(
            zoneinfo_name("/var/db/timezone/zoneinfo/Asia/Shanghai"),
            Some("Asia/Shanghai".to_string())
        );
        // 嵌套层级（美洲 / 阿根廷）整段保留
        assert_eq!(
            zoneinfo_name("/usr/share/zoneinfo/America/Argentina/Buenos_Aires"),
            Some("America/Argentina/Buenos_Aires".to_string())
        );
    }

    #[test]
    fn zoneinfo_name_is_none_without_marker() {
        assert_eq!(zoneinfo_name("/etc/localtime"), None);
    }

    #[test]
    fn zoneinfo_name_returns_empty_for_degenerate_link() {
        // 软链目标以 zoneinfo/ 结尾：取不到名字，交回空串由调用方兜底
        assert_eq!(zoneinfo_name("/usr/share/zoneinfo/"), Some(String::new()));
    }

    #[test]
    fn memory_gib_from_bytes_floors_to_gib() {
        // 16 GiB 少一字节 → 15（向下取整）
        assert_eq!(memory_gib_from_bytes("17179869183"), Some(15));
        assert_eq!(memory_gib_from_bytes("17179869184"), Some(16));
        // 前后空白来自命令输出
        assert_eq!(memory_gib_from_bytes("  8589934592\n"), Some(8));
    }

    #[test]
    fn memory_gib_from_bytes_is_none_for_garbage() {
        assert_eq!(memory_gib_from_bytes(""), None);
        assert_eq!(memory_gib_from_bytes("N/A"), None);
    }

    #[test]
    fn memory_gib_from_meminfo_reads_mem_total_line() {
        let text = "MemTotal:       16384000 kB\nMemFree:         1234567 kB\n";
        // 16384000 kB / 1024 / 1024 = 15.625 GiB → 15
        assert_eq!(memory_gib_from_meminfo(text), Some(15));
    }

    #[test]
    fn memory_gib_from_meminfo_is_none_without_the_line() {
        assert_eq!(memory_gib_from_meminfo("MemFree: 100 kB\n"), None);
        assert_eq!(memory_gib_from_meminfo("MemTotal:\n"), None);
    }
}
