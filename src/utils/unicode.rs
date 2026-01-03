// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Unicode utilities for column width calculation.
//! Compatible with cpplint's line length counting (CJK chars = 2 columns).

/// Calculate column width of a string (cpplint-compatible)
/// CJK and other wide characters count as 2 columns, others as 1
pub fn get_column_width(s: &str) -> usize {
    s.chars().map(|c| if is_wide_char(c) { 2 } else { 1 }).sum()
}

/// Check if a character is a wide character (CJK, fullwidth, etc.)
/// Based on Unicode East Asian Width property
pub fn is_wide_char(c: char) -> bool {
    let cp = c as u32;
    // CJK Unified Ideographs and extensions
    if (0x4E00..=0x9FFF).contains(&cp) { return true; }  // CJK Unified
    if (0x3400..=0x4DBF).contains(&cp) { return true; }  // CJK Extension A
    if (0x20000..=0x2A6DF).contains(&cp) { return true; } // CJK Extension B
    if (0x2A700..=0x2B73F).contains(&cp) { return true; } // CJK Extension C
    if (0x2B740..=0x2B81F).contains(&cp) { return true; } // CJK Extension D
    // Fullwidth forms
    if (0xFF01..=0xFF60).contains(&cp) { return true; }  // Fullwidth ASCII
    if (0xFFE0..=0xFFE6).contains(&cp) { return true; }  // Fullwidth symbols
    // CJK punctuation and symbols
    if (0x3000..=0x303F).contains(&cp) { return true; }  // CJK Symbols
    if (0xFF00..=0xFFEF).contains(&cp) { return true; }  // Halfwidth/Fullwidth
    // Hiragana, Katakana
    if (0x3040..=0x309F).contains(&cp) { return true; }  // Hiragana
    if (0x30A0..=0x30FF).contains(&cp) { return true; }  // Katakana
    // Hangul
    if (0xAC00..=0xD7AF).contains(&cp) { return true; }  // Hangul Syllables
    if (0x1100..=0x11FF).contains(&cp) { return true; }  // Hangul Jamo
    false
}

/// Break text at specified column width, preferring to break at punctuation
pub fn break_text_at_width(text: &str, max_width: usize) -> Vec<String> {
    // Use column width (CJK chars = 2 columns)
    if get_column_width(text) <= max_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    // Chinese punctuation (break after these); quotes excluded due to Rust syntax
    let break_after: &[char] = &['。', '，', '、', '；', '：', '！', '？', '）', '】', '》', ' '];

    for c in text.chars() {
        let char_width = if is_wide_char(c) { 2 } else { 1 };
        current.push(c);
        current_width += char_width;

        // Check column width
        if current_width >= max_width {
            // Try to find a good break point near the end
            if let Some(break_pos) = find_break_point(&current, break_after) {
                let (first, rest) = current.split_at(break_pos);
                lines.push(first.to_string());
                current = rest.to_string();
                current_width = get_column_width(&current);
            } else {
                // No good break point, just break at current position
                lines.push(current.clone());
                current.clear();
                current_width = 0;
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

/// Find a good break point in the text (looking backwards from the end)
fn find_break_point(text: &str, break_chars: &[char]) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    // Look for break point in the last 30% of the text
    let search_start = len.saturating_sub(len * 30 / 100).max(len / 2);

    for i in (search_start..len).rev() {
        if break_chars.contains(&chars[i]) {
            // Return byte position after this character
            let byte_pos: usize = chars[..=i].iter().map(|c| c.len_utf8()).sum();
            return Some(byte_pos);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== get_column_width tests ====================

    #[test]
    fn test_column_width_ascii() {
        assert_eq!(get_column_width("hello"), 5);
        assert_eq!(get_column_width("// comment"), 10);
    }

    #[test]
    fn test_column_width_cjk() {
        assert_eq!(get_column_width("中文"), 4);  // 2 chars * 2 columns
        assert_eq!(get_column_width("// 中文"), 7);  // 3 ASCII + 2*2 CJK
    }

    #[test]
    fn test_column_width_japanese() {
        // Hiragana and Katakana are wide characters
        assert_eq!(get_column_width("あいう"), 6);  // 3 chars * 2 columns
        assert_eq!(get_column_width("アイウ"), 6);  // 3 chars * 2 columns
    }

    #[test]
    fn test_column_width_korean() {
        assert_eq!(get_column_width("한글"), 4);  // 2 chars * 2 columns
    }

    #[test]
    fn test_column_width_fullwidth() {
        // Fullwidth ASCII
        assert_eq!(get_column_width("ＡＢＣ"), 6);  // 3 chars * 2 columns
    }

    #[test]
    fn test_column_width_mixed() {
        // "Hello中文" = 5 ASCII + 2*2 CJK = 9
        assert_eq!(get_column_width("Hello中文"), 9);
    }

    #[test]
    fn test_column_width_empty() {
        assert_eq!(get_column_width(""), 0);
    }

    // ==================== is_wide_char tests ====================

    #[test]
    fn test_is_wide_char() {
        assert!(is_wide_char('中'));
        assert!(is_wide_char('，'));
        assert!(!is_wide_char('a'));
        assert!(!is_wide_char(' '));
    }

    #[test]
    fn test_is_wide_char_cjk_ranges() {
        // CJK Unified Ideographs
        assert!(is_wide_char('中'));
        assert!(is_wide_char('文'));
        // CJK Symbols
        assert!(is_wide_char('。'));
        assert!(is_wide_char('、'));
    }

    #[test]
    fn test_is_wide_char_japanese() {
        assert!(is_wide_char('あ'));  // Hiragana
        assert!(is_wide_char('ア'));  // Katakana
    }

    #[test]
    fn test_is_wide_char_korean() {
        assert!(is_wide_char('한'));  // Hangul
    }

    // ==================== break_text_at_width tests ====================

    #[test]
    fn test_break_text_short() {
        let result = break_text_at_width("short text", 100);
        assert_eq!(result, vec!["short text"]);
    }

    #[test]
    fn test_break_text_at_space() {
        let result = break_text_at_width("hello world this is a test", 15);
        assert!(result.len() > 1);
        // All parts should be shorter than max width
        for part in &result {
            assert!(get_column_width(part) <= 15);
        }
    }

    #[test]
    fn test_break_text_chinese_punctuation() {
        // Break at Chinese punctuation
        let result = break_text_at_width("这是一个测试，需要换行。", 15);
        assert!(result.len() > 1);
    }

    #[test]
    fn test_break_text_no_break_point() {
        // Long word without spaces
        let result = break_text_at_width("aaaaaaaaaaaaaaaaaaaa", 10);
        // Should still break even without good break points
        assert!(!result.is_empty());
    }
}
