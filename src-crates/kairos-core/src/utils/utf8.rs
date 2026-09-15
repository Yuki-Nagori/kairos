//! UTF-8 安全的字符串操作。
//!
//! `&str` 的**字节下标**与**字符概念**混用是 Rust 里最易 panic 的一类 bug：
//! `&s[..n]` 落在多字节字符中间会直接 panic。本模块是唯一入口——禁止手写
//! `is_char_boundary` 退让循环，也禁止裸的 `&s[..n]` 切片。
//!
//! 按**显示宽度**对齐（中英文混排的美化）不在这里：那需要东亚字符宽度表，
//! 本模块只保证**字符数**与**字节安全**。

use std::borrow::Cow;

/// 截断标记。用中文省略号，占 3 字节——预算扣除按它的实际字节数计。
const ELLIPSIS: &str = "…";

/// 小于等于 `max_bytes` 的最大 char 边界；超出长度时返回 `s.len()`。
pub fn floor_char_boundary(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut index = max_bytes;
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// 大于等于 `min_bytes` 的最小 char 边界；超出长度时返回 `s.len()`。
pub fn ceil_char_boundary(s: &str, min_bytes: usize) -> usize {
    if min_bytes >= s.len() {
        return s.len();
    }
    let mut index = min_bytes;
    while index < s.len() && !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

/// 按字节预算截断，落点退到 char 边界上（永不 panic）。
pub fn truncate_bytes(s: &str, max_bytes: usize) -> &str {
    &s[..floor_char_boundary(s, max_bytes)]
}

/// 按字符数截断。
pub fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((index, _)) => &s[..index],
        None => s,
    }
}

/// 按字节预算截断并在超长时追加省略号。
///
/// 结果总字节数不超过 `max_bytes`；预算小到装不下省略号本身时返回空串
/// （不留「超预算的省略号」，那会让调用方按长度做的排版溢出）。
pub fn truncate_bytes_with_ellipsis(s: &str, max_bytes: usize) -> Cow<'_, str> {
    if s.len() <= max_bytes {
        return Cow::Borrowed(s);
    }
    if max_bytes < ELLIPSIS.len() {
        return Cow::Borrowed("");
    }
    let end = floor_char_boundary(s, max_bytes - ELLIPSIS.len());
    Cow::Owned(format!("{}{ELLIPSIS}", &s[..end]))
}

/// 字节下标 → 字符下标；下标落在字符中间时向前取整。
pub fn byte_to_char_index(s: &str, byte_index: usize) -> usize {
    s[..floor_char_boundary(s, byte_index)].chars().count()
}

/// 字符下标 → 字节下标；超出字符数时返回 `s.len()`。
pub fn char_to_byte_index(s: &str, char_index: usize) -> usize {
    truncate_chars(s, char_index).len()
}

/// 右侧用 `pad` 补齐到 `width` 个字符；已达到或超过则原样返回。
pub fn pad_end_chars(s: &str, width: usize, pad: char) -> String {
    let count = s.chars().count();
    if count >= width {
        return s.to_string();
    }
    let mut padded = String::with_capacity(s.len() + (width - count) * pad.len_utf8());
    padded.push_str(s);
    for _ in count..width {
        padded.push(pad);
    }
    padded
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `a` 1 字节 + `中` 3 字节 + `b` 1 字节：边界在 0 / 1 / 4 / 5。
    const MIXED: &str = "a中b";

    #[test]
    fn floor_returns_existing_boundary_unchanged() {
        assert_eq!(floor_char_boundary(MIXED, 0), 0);
        assert_eq!(floor_char_boundary(MIXED, 1), 1);
        assert_eq!(floor_char_boundary(MIXED, 4), 4);
    }

    #[test]
    fn floor_retreats_into_multibyte_character() {
        assert_eq!(floor_char_boundary(MIXED, 2), 1);
        assert_eq!(floor_char_boundary(MIXED, 3), 1);
    }

    #[test]
    fn floor_retreats_to_zero_when_no_boundary_before() {
        // 全部下标都落在首字符内部：只能退到 0
        assert_eq!(floor_char_boundary("中", 1), 0);
        assert_eq!(floor_char_boundary("中", 2), 0);
    }

    #[test]
    fn floor_clamps_at_length() {
        assert_eq!(floor_char_boundary(MIXED, 5), 5);
        assert_eq!(floor_char_boundary(MIXED, 99), 5);
        assert_eq!(floor_char_boundary("", 3), 0);
    }

    #[test]
    fn ceil_returns_existing_boundary_unchanged() {
        assert_eq!(ceil_char_boundary(MIXED, 0), 0);
        assert_eq!(ceil_char_boundary(MIXED, 1), 1);
        assert_eq!(ceil_char_boundary(MIXED, 4), 4);
    }

    #[test]
    fn ceil_advances_into_next_boundary() {
        assert_eq!(ceil_char_boundary(MIXED, 2), 4);
        assert_eq!(ceil_char_boundary(MIXED, 3), 4);
    }

    #[test]
    fn ceil_stops_at_length_when_character_extends_to_end() {
        // `中` 一直延伸到文本末尾：2、3 都不是边界，只能停在 len
        assert_eq!(ceil_char_boundary("a中", 2), 4);
    }

    #[test]
    fn ceil_clamps_at_length() {
        assert_eq!(ceil_char_boundary(MIXED, 5), 5);
        assert_eq!(ceil_char_boundary(MIXED, 99), 5);
        assert_eq!(ceil_char_boundary("", 3), 0);
    }

    #[test]
    fn truncate_bytes_never_splits_character() {
        assert_eq!(truncate_bytes(MIXED, 0), "");
        assert_eq!(truncate_bytes(MIXED, 1), "a");
        assert_eq!(truncate_bytes(MIXED, 3), "a");
        assert_eq!(truncate_bytes(MIXED, 4), "a中");
        assert_eq!(truncate_bytes(MIXED, 5), MIXED);
        assert_eq!(truncate_bytes(MIXED, 99), MIXED);
    }

    #[test]
    fn truncate_chars_counts_characters_not_bytes() {
        assert_eq!(truncate_chars(MIXED, 0), "");
        assert_eq!(truncate_chars(MIXED, 2), "a中");
        assert_eq!(truncate_chars(MIXED, 3), MIXED);
        assert_eq!(truncate_chars(MIXED, 99), MIXED);
    }

    #[test]
    fn ellipsis_borrows_when_within_budget() {
        assert_eq!(truncate_bytes_with_ellipsis(MIXED, 5), MIXED);
        assert!(matches!(
            truncate_bytes_with_ellipsis(MIXED, 5),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn ellipsis_returns_empty_when_budget_too_small() {
        // 省略号占 3 字节：预算 2 装不下，返回空串而不是超预算的省略号
        assert_eq!(truncate_bytes_with_ellipsis(MIXED, 0), "");
        assert_eq!(truncate_bytes_with_ellipsis(MIXED, 2), "");
    }

    #[test]
    fn ellipsis_keeps_total_within_budget() {
        let truncated = truncate_bytes_with_ellipsis(MIXED, 4);
        assert_eq!(truncated, "a…");
        assert!(truncated.len() <= 4);
        // 落点落在 `中` 内部：前缀退到 "a"，省略号补足
        assert_eq!(truncate_bytes_with_ellipsis(MIXED, 3), "…");
    }

    #[test]
    fn byte_index_maps_to_char_index() {
        assert_eq!(byte_to_char_index(MIXED, 0), 0);
        assert_eq!(byte_to_char_index(MIXED, 1), 1);
        assert_eq!(byte_to_char_index(MIXED, 4), 2);
        assert_eq!(byte_to_char_index(MIXED, 5), 3);
        // 落在 `中` 内部：按向前取整算作 1 个字符
        assert_eq!(byte_to_char_index(MIXED, 3), 1);
    }

    #[test]
    fn char_index_maps_to_byte_index() {
        assert_eq!(char_to_byte_index(MIXED, 0), 0);
        assert_eq!(char_to_byte_index(MIXED, 1), 1);
        assert_eq!(char_to_byte_index(MIXED, 2), 4);
        assert_eq!(char_to_byte_index(MIXED, 3), 5);
        assert_eq!(char_to_byte_index(MIXED, 99), 5);
    }

    #[test]
    fn char_and_byte_index_round_trip_on_boundaries() {
        for char_index in 0..=MIXED.chars().count() {
            let byte_index = char_to_byte_index(MIXED, char_index);
            assert_eq!(byte_to_char_index(MIXED, byte_index), char_index);
        }
    }

    #[test]
    fn pad_end_chars_counts_characters() {
        assert_eq!(pad_end_chars("a中", 4, ' '), "a中  ");
        assert_eq!(pad_end_chars("-", 3, '0'), "-00");
    }

    #[test]
    fn pad_end_chars_keeps_wide_enough_input() {
        assert_eq!(pad_end_chars("abc", 3, ' '), "abc");
        assert_eq!(pad_end_chars("abcd", 3, ' '), "abcd");
        assert_eq!(pad_end_chars("", 1, ' '), " ");
    }
}
