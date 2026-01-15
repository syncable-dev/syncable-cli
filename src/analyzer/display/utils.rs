//! String manipulation and visual width calculation utilities

/// Calculate visual width of a string, handling ANSI color codes
pub fn visual_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip ANSI escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break; // End of escape sequence
                    }
                }
            }
        } else {
            // Simple width calculation for common cases
            // Most characters are width 1, some are width 0 or 2
            width += char_width(ch);
        }
    }

    width
}

/// Simple character width calculation without external dependencies
pub fn char_width(ch: char) -> usize {
    match ch {
        // Control characters have width 0
        '\u{0000}'..='\u{001F}' | '\u{007F}' => 0,
        // Combining marks have width 0
        '\u{0300}'..='\u{036F}' => 0,
        // Emoji and symbols (width 2)
        '\u{2600}'..='\u{26FF}' |    // Miscellaneous Symbols
        '\u{2700}'..='\u{27BF}' |    // Dingbats
        '\u{1F000}'..='\u{1F02F}' |  // Mahjong Tiles
        '\u{1F030}'..='\u{1F09F}' |  // Domino Tiles
        '\u{1F0A0}'..='\u{1F0FF}' |  // Playing Cards
        '\u{1F100}'..='\u{1F1FF}' |  // Enclosed Alphanumeric Supplement
        '\u{1F200}'..='\u{1F2FF}' |  // Enclosed Ideographic Supplement
        '\u{1F300}'..='\u{1F5FF}' |  // Miscellaneous Symbols and Pictographs
        '\u{1F600}'..='\u{1F64F}' |  // Emoticons
        '\u{1F650}'..='\u{1F67F}' |  // Ornamental Dingbats
        '\u{1F680}'..='\u{1F6FF}' |  // Transport and Map Symbols
        '\u{1F700}'..='\u{1F77F}' |  // Alchemical Symbols
        '\u{1F780}'..='\u{1F7FF}' |  // Geometric Shapes Extended
        '\u{1F800}'..='\u{1F8FF}' |  // Supplemental Arrows-C
        '\u{1F900}'..='\u{1F9FF}' |  // Supplemental Symbols and Pictographs
        // Full-width characters (common CJK ranges)
        '\u{1100}'..='\u{115F}' |  // Hangul Jamo
        '\u{2E80}'..='\u{2EFF}' |  // CJK Radicals
        '\u{2F00}'..='\u{2FDF}' |  // Kangxi Radicals
        '\u{2FF0}'..='\u{2FFF}' |  // Ideographic Description
        '\u{3000}'..='\u{303E}' |  // CJK Symbols and Punctuation
        '\u{3041}'..='\u{3096}' |  // Hiragana
        '\u{30A1}'..='\u{30FA}' |  // Katakana
        '\u{3105}'..='\u{312D}' |  // Bopomofo
        '\u{3131}'..='\u{318E}' |  // Hangul Compatibility Jamo
        '\u{3190}'..='\u{31BA}' |  // Kanbun
        '\u{31C0}'..='\u{31E3}' |  // CJK Strokes
        '\u{31F0}'..='\u{31FF}' |  // Katakana Phonetic Extensions
        '\u{3200}'..='\u{32FF}' |  // Enclosed CJK Letters and Months
        '\u{3300}'..='\u{33FF}' |  // CJK Compatibility
        '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
        '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
        '\u{A000}'..='\u{A48C}' |  // Yi Syllables
        '\u{A490}'..='\u{A4C6}' |  // Yi Radicals
        '\u{AC00}'..='\u{D7AF}' |  // Hangul Syllables
        '\u{F900}'..='\u{FAFF}' |  // CJK Compatibility Ideographs
        '\u{FE10}'..='\u{FE19}' |  // Vertical Forms
        '\u{FE30}'..='\u{FE6F}' |  // CJK Compatibility Forms
        '\u{FF00}'..='\u{FF60}' |  // Fullwidth Forms
        '\u{FFE0}'..='\u{FFE6}' => 2,
        // Most other printable characters have width 1
        _ => 1,
    }
}

/// Truncate string to specified visual width, preserving color codes
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    let current_visual_width = visual_width(s);
    if current_visual_width <= max_width {
        return s.to_string();
    }

    // For strings with ANSI codes, we need to be more careful
    if s.contains('\x1b') {
        // Simple approach: strip ANSI codes, truncate, then re-apply if needed
        let stripped = strip_ansi_codes(s);
        if visual_width(&stripped) <= max_width {
            return s.to_string();
        }

        // Truncate the stripped version
        let mut result = String::new();
        let mut width = 0;
        for ch in stripped.chars() {
            let ch_width = char_width(ch);
            if width + ch_width > max_width.saturating_sub(3) {
                result.push_str("...");
                break;
            }
            result.push(ch);
            width += ch_width;
        }
        return result;
    }

    // No ANSI codes - simple truncation
    let mut result = String::new();
    let mut width = 0;

    for ch in s.chars() {
        let ch_width = char_width(ch);
        if width + ch_width > max_width.saturating_sub(3) {
            result.push_str("...");
            break;
        }
        result.push(ch);
        width += ch_width;
    }

    result
}

/// Get terminal width, defaulting to 100 if unavailable
pub fn get_terminal_width() -> usize {
    term_size::dimensions().map(|(w, _)| w).unwrap_or(100)
}

/// Smart truncate with single-char ellipsis "…" for cleaner look
pub fn smart_truncate(s: &str, max_width: usize) -> String {
    let current_width = visual_width(s);
    if current_width <= max_width {
        return s.to_string();
    }

    // Use single-char ellipsis for cleaner appearance
    let mut result = String::new();
    let mut width = 0;
    let target_width = max_width.saturating_sub(1); // Leave room for "…"

    for ch in strip_ansi_codes(s).chars() {
        let ch_width = char_width(ch);
        if width + ch_width > target_width {
            break;
        }
        result.push(ch);
        width += ch_width;
    }
    result.push('…');
    result
}

/// Format ports list: deduplicate, limit to max_show, add "+N" if more
pub fn format_ports_smart(ports: &[u16], max_show: usize) -> String {
    if ports.is_empty() {
        return "-".to_string();
    }

    // Deduplicate and sort
    let mut unique_ports: Vec<u16> = ports.to_vec();
    unique_ports.sort_unstable();
    unique_ports.dedup();

    if unique_ports.len() <= max_show {
        unique_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let shown: Vec<String> = unique_ports
            .iter()
            .take(max_show)
            .map(|p| p.to_string())
            .collect();
        let remaining = unique_ports.len() - max_show;
        format!("{} +{}", shown.join(", "), remaining)
    }
}

/// Format a list of strings smartly: show up to max_show items, add "+N" if more
/// Each item is truncated to max_item_width if needed
pub fn format_list_smart(items: &[String], max_show: usize, max_item_width: usize) -> String {
    if items.is_empty() {
        return "-".to_string();
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<&String> = items
        .iter()
        .filter(|item| seen.insert(item.as_str()))
        .collect();

    if unique.len() <= max_show {
        unique
            .iter()
            .map(|s| {
                if visual_width(s) > max_item_width {
                    smart_truncate(s, max_item_width)
                } else {
                    s.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let shown: Vec<String> = unique
            .iter()
            .take(max_show)
            .map(|s| {
                if visual_width(s) > max_item_width {
                    smart_truncate(s, max_item_width)
                } else {
                    s.to_string()
                }
            })
            .collect();
        let remaining = unique.len() - max_show;
        format!("{} +{}", shown.join(", "), remaining)
    }
}

/// Strip ANSI escape codes from a string
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip ANSI escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break; // End of escape sequence
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_width_basic() {
        assert_eq!(visual_width("hello"), 5);
        assert_eq!(visual_width(""), 0);
        assert_eq!(visual_width("123"), 3);
    }

    #[test]
    fn test_visual_width_with_ansi() {
        assert_eq!(visual_width("\x1b[31mhello\x1b[0m"), 5);
        assert_eq!(visual_width("\x1b[1;32mtest\x1b[0m"), 4);
    }

    #[test]
    fn test_truncate_to_width() {
        assert_eq!(truncate_to_width("hello world", 5), "he...");
        assert_eq!(truncate_to_width("hello", 10), "hello");
        assert_eq!(truncate_to_width("hello world", 8), "hello...");
    }

    #[test]
    fn test_strip_ansi_codes() {
        assert_eq!(strip_ansi_codes("\x1b[31mhello\x1b[0m"), "hello");
        assert_eq!(strip_ansi_codes("plain text"), "plain text");
        assert_eq!(
            strip_ansi_codes("\x1b[1;32mgreen\x1b[0m text"),
            "green text"
        );
    }
}
