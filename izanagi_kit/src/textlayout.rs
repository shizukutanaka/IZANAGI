//! Word-wrap and text layout for fixed-width terminal rendering.
//!
//! `wrap_words` breaks a string into lines that fit within `max_cols` columns,
//! breaking at word boundaries (spaces) where possible and falling back to a
//! hard break mid-word when a single word exceeds the column width.
//!
//! The output is a `Vec<String>` of lines, each at most `max_cols` chars wide
//! (measured in Unicode scalar values, not bytes — one `char` = one column,
//! the same assumption `terminal::Screen` makes for ASCII roguelike output).
//!
//! Additional helpers:
//! - `truncate(s, max_cols)` — clip to `max_cols` chars, appending `…` if cut.
//! - `center(s, width)` — pad with spaces to center in `width` columns.
//! - `pad_right(s, width)` — left-align, space-pad to `width`.
//! - `pad_left(s, width)` — right-align, space-pad to `width`.
//!
//! All functions are pure (no allocation state, no I/O) and operate on `&str`.

/// Wrap `text` into lines of at most `max_cols` Unicode scalar values.
///
/// - Breaks prefer the last space that fits on the line.
/// - When no space fits (a single token longer than `max_cols`), the word is
///   split at `max_cols` with the remainder carried to the next line.
/// - A `max_cols` of 0 returns an empty `Vec`.
/// - Trailing whitespace on each line is stripped.
pub fn wrap_words(text: &str, max_cols: usize) -> Vec<String> {
    if max_cols == 0 {
        return Vec::new();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if current.is_empty() {
            // Place the word at the start of the current line.
            if word_len <= max_cols {
                current.push_str(word);
            } else {
                // Hard-break a word that exceeds max_cols.
                hard_break_word(word, max_cols, &mut lines);
                // `hard_break_word` pushes all but the last chunk; the last
                // chunk goes into `current` so it can absorb more words.
                if let Some(last) = lines.pop() {
                    current = last;
                }
            }
        } else {
            let needed = current.chars().count() + 1 + word_len; // +1 for space
            if needed <= max_cols {
                current.push(' ');
                current.push_str(word);
            } else {
                // Flush and start a new line.
                lines.push(current.trim_end().to_owned());
                current = String::new();
                if word_len <= max_cols {
                    current.push_str(word);
                } else {
                    hard_break_word(word, max_cols, &mut lines);
                    if let Some(last) = lines.pop() {
                        current = last;
                    }
                }
            }
        }
    }

    if !current.is_empty() {
        lines.push(current.trim_end().to_owned());
    }

    lines
}

/// Like [`wrap_words`] but caps the output at `max_lines` lines.
///
/// If the wrapped text would produce more lines than `max_lines`, the last
/// visible line is truncated with an `…` ellipsis to signal overflow. A
/// `max_lines` of 0 (or `max_cols` of 0) returns an empty `Vec`.
pub fn wrap_words_max_lines(text: &str, max_cols: usize, max_lines: usize) -> Vec<String> {
    if max_lines == 0 || max_cols == 0 {
        return Vec::new();
    }
    let mut lines = wrap_words(text, max_cols);
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            // Force an ellipsis on the last visible line to signal that more
            // content was cut. If there is room, append; otherwise replace the
            // final char.
            let ch_count = last.chars().count();
            if ch_count < max_cols {
                last.push('…');
            } else {
                let keep: String = last.chars().take(max_cols.saturating_sub(1)).collect();
                *last = keep + "…";
            }
        }
    }
    lines
}

/// Split `word` into chunks of `max_cols` chars, push all but the last into
/// `out`, and push the last chunk too (caller may pop it into `current`).
fn hard_break_word(word: &str, max_cols: usize, out: &mut Vec<String>) {
    let chars: Vec<char> = word.chars().collect();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + max_cols).min(chars.len());
        out.push(chars[start..end].iter().collect());
        start = end;
    }
}

/// Truncate `s` to `max_cols` chars. If truncated, the last visible char is
/// replaced with `…` (U+2026) so the total remains `max_cols` chars wide.
/// If `max_cols == 0` returns an empty string.
pub fn truncate(s: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_cols {
        s.to_owned()
    } else {
        let mut out: String = chars[..max_cols - 1].iter().collect();
        out.push('…');
        out
    }
}

/// Center `s` within `width` columns using space padding.
/// If `s` is wider than `width` it is returned as-is.
pub fn center(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_owned();
    }
    let total_pad = width - len;
    let left_pad = total_pad / 2;
    let right_pad = total_pad - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), s, " ".repeat(right_pad))
}

/// Left-align `s`, space-padding to `width` on the right.
/// If `s` is wider than `width` it is returned as-is.
pub fn pad_right(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_owned();
    }
    format!("{}{}", s, " ".repeat(width - len))
}

/// Distribute extra spaces between words to fill `width` exactly
/// (typographic full justification / newspaper-column style).
///
/// - A single-word line (or a line already at or wider than `width`) is
///   returned left-aligned via `pad_right`.
/// - For `n` gaps between words, extra spaces are distributed left-to-right:
///   the first `total_spaces % gaps` gaps each receive one extra space.
/// - `width == 0` returns an empty string.
pub fn justify(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.is_empty() {
        return " ".repeat(width);
    }
    if words.len() == 1 {
        return pad_right(words[0], width);
    }
    let content_len: usize = words.iter().map(|w| w.chars().count()).sum();
    if content_len >= width {
        return words.join(" ");
    }
    let gaps = words.len() - 1;
    let total_spaces = width - content_len;
    let base = total_spaces / gaps;
    let extra = total_spaces % gaps;
    let mut result = String::new();
    for (i, word) in words.iter().enumerate() {
        result.push_str(word);
        if i < gaps {
            let spaces = base + if i < extra { 1 } else { 0 };
            for _ in 0..spaces {
                result.push(' ');
            }
        }
    }
    result
}

/// Return `(max_width, line_count)` for a slice of string lines.
///
/// `max_width` is the char count of the longest line; `line_count` is
/// `lines.len()`. Both are `0` for an empty slice. Allocation-free — useful
/// for layout planning before committing to a render pass.
pub fn measure_lines(lines: &[&str]) -> (usize, usize) {
    let max_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    (max_w, lines.len())
}

/// Pad every line in `lines` to `width` columns using [`pad_right`], returning
/// a new `Vec<String>`. Lines already at or beyond `width` are returned as-is.
/// Produces a uniform-width block suitable for bordered panels and column
/// rendering.
pub fn pad_lines(lines: Vec<String>, width: usize) -> Vec<String> {
    lines.into_iter().map(|l| pad_right(&l, width)).collect()
}

/// Wrap `text` to fit inside a box of `width` columns and `height` rows.
///
/// Convenience that combines [`wrap_words_max_lines`] — wrapping at `width`
/// and capping at `height` lines. If the text overflows, the last visible
/// line ends with `…`. Both `width == 0` and `height == 0` return an empty
/// `Vec`. The typical "dialogue box" call site pattern in roguelike UIs.
pub fn fit_to_box(text: &str, width: usize, height: usize) -> Vec<String> {
    wrap_words_max_lines(text, width, height)
}

/// Right-align `s`, space-padding to `width` on the left.
/// If `s` is wider than `width` it is returned as-is.
pub fn pad_left(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_owned();
    }
    format!("{}{}", " ".repeat(width - len), s)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- wrap_words ---

    #[test]
    fn test_wrap_empty_string() {
        assert_eq!(wrap_words("", 10), Vec::<String>::new());
    }

    #[test]
    fn test_wrap_zero_cols() {
        assert_eq!(wrap_words("hello world", 0), Vec::<String>::new());
    }

    #[test]
    fn test_wrap_short_fits_on_one_line() {
        assert_eq!(wrap_words("hello", 10), vec!["hello"]);
    }

    #[test]
    fn test_wrap_two_words_fit() {
        assert_eq!(wrap_words("hello world", 20), vec!["hello world"]);
    }

    #[test]
    fn test_wrap_breaks_at_word_boundary() {
        let lines = wrap_words("one two three", 7);
        // "one two" = 7 chars; "three" on next line
        assert_eq!(lines, vec!["one two", "three"]);
    }

    #[test]
    fn test_wrap_multiple_lines() {
        let lines = wrap_words("the quick brown fox", 9);
        assert_eq!(lines, vec!["the quick", "brown fox"]);
    }

    #[test]
    fn test_wrap_hard_break_long_word() {
        let lines = wrap_words("abcdefghij", 4);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn test_wrap_long_word_then_short() {
        let lines = wrap_words("abcdefgh hi", 4);
        // "abcd", "efgh", "hi"
        assert_eq!(lines, vec!["abcd", "efgh", "hi"]);
    }

    #[test]
    fn test_wrap_exact_fit() {
        let lines = wrap_words("abc def", 3);
        assert_eq!(lines, vec!["abc", "def"]);
    }

    #[test]
    fn test_wrap_strips_extra_whitespace() {
        let lines = wrap_words("  hello   world  ", 20);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_single_col() {
        let lines = wrap_words("abc", 1);
        assert_eq!(lines, vec!["a", "b", "c"]);
    }

    // --- wrap_words_max_lines ---

    #[test]
    fn test_wrap_max_lines_no_overflow() {
        // Fits within limit: no ellipsis.
        let lines = wrap_words_max_lines("one two", 10, 3);
        assert_eq!(lines, vec!["one two"]);
    }

    #[test]
    fn test_wrap_max_lines_truncates_with_ellipsis() {
        // "the quick" / "brown fox" / "jumps over" — only first 2 lines kept.
        let lines = wrap_words_max_lines("the quick brown fox jumps over", 9, 2);
        assert_eq!(lines.len(), 2);
        // Last line must end with ellipsis.
        assert!(
            lines.last().unwrap().contains('…'),
            "last line must have ellipsis, got: {:?}",
            lines
        );
    }

    #[test]
    fn test_wrap_max_lines_zero_max_lines_empty() {
        assert_eq!(wrap_words_max_lines("hello", 10, 0), Vec::<String>::new());
    }

    #[test]
    fn test_wrap_max_lines_respects_max_cols() {
        let lines = wrap_words_max_lines("a b c d e f", 3, 2);
        for l in &lines {
            assert!(l.chars().count() <= 3, "line too wide: {l:?}");
        }
    }

    #[test]
    fn test_wrap_max_lines_one_line_limit() {
        // max_cols=10 forces: "hello", "world foo", "bar" — 3 lines capped to 1.
        let lines = wrap_words_max_lines("hello world foo bar", 10, 1);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains('…'), "got: {:?}", lines[0]);
    }

    // --- truncate ---

    #[test]
    fn test_truncate_fits() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_cuts_with_ellipsis() {
        assert_eq!(truncate("hello world", 7), "hello …");
        assert_eq!(truncate("hello world", 7).chars().count(), 7);
    }

    #[test]
    fn test_truncate_zero_cols() {
        assert_eq!(truncate("hello", 0), "");
    }

    #[test]
    fn test_truncate_one_col() {
        assert_eq!(truncate("hello", 1), "…");
    }

    // --- center ---

    #[test]
    fn test_center_even_padding() {
        assert_eq!(center("hi", 6), "  hi  ");
    }

    #[test]
    fn test_center_odd_padding_extra_on_right() {
        assert_eq!(center("hi", 7), "  hi   ");
    }

    #[test]
    fn test_center_already_wide() {
        assert_eq!(center("hello", 3), "hello");
    }

    #[test]
    fn test_center_exact_width() {
        assert_eq!(center("hello", 5), "hello");
    }

    // --- pad_right ---

    #[test]
    fn test_pad_right_pads() {
        assert_eq!(pad_right("hi", 5), "hi   ");
    }

    #[test]
    fn test_pad_right_exact() {
        assert_eq!(pad_right("hi", 2), "hi");
    }

    #[test]
    fn test_pad_right_wider() {
        assert_eq!(pad_right("hello", 3), "hello");
    }

    // --- pad_left ---

    #[test]
    fn test_pad_left_pads() {
        assert_eq!(pad_left("hi", 5), "   hi");
    }

    #[test]
    fn test_pad_left_exact() {
        assert_eq!(pad_left("hi", 2), "hi");
    }

    #[test]
    fn test_pad_left_wider() {
        assert_eq!(pad_left("hello", 3), "hello");
    }

    // --- justify ---

    #[test]
    fn test_justify_two_words_fills_width() {
        // "ab cd" at width 8 → "ab    cd" (4 spaces between)
        assert_eq!(justify("ab cd", 8), "ab    cd");
        assert_eq!(justify("ab cd", 8).chars().count(), 8);
    }

    #[test]
    fn test_justify_three_words_distributes_evenly() {
        // "a b c" content=3, gaps=2, width=7, total_spaces=4, base=2, extra=0
        // "a  b  c" but extra=0 so both gaps get 2 spaces
        let j = justify("a b c", 7);
        assert_eq!(j.chars().count(), 7);
        assert!(j.starts_with('a'));
        assert!(j.ends_with('c'));
    }

    #[test]
    fn test_justify_extra_space_goes_to_left_gaps() {
        // "a b c" width=8, content=3, gaps=2, total_spaces=5, base=2, extra=1
        // gap0=3, gap1=2 → "a   b  c"
        let j = justify("a b c", 8);
        assert_eq!(j.chars().count(), 8);
        assert_eq!(j, "a   b  c");
    }

    #[test]
    fn test_justify_single_word_pads_right() {
        assert_eq!(justify("hi", 5), "hi   ");
    }

    #[test]
    fn test_justify_already_wide_returns_as_is() {
        let j = justify("hello world", 5);
        assert_eq!(j, "hello world");
    }

    #[test]
    fn test_justify_zero_width_returns_empty() {
        assert_eq!(justify("hello", 0), "");
    }

    // --- measure_lines ---

    #[test]
    fn test_measure_lines_empty() {
        assert_eq!(measure_lines(&[]), (0, 0));
    }

    #[test]
    fn test_measure_lines_single() {
        assert_eq!(measure_lines(&["hello"]), (5, 1));
    }

    #[test]
    fn test_measure_lines_multi() {
        let (max_w, count) = measure_lines(&["hi", "hello", "yo"]);
        assert_eq!(max_w, 5);
        assert_eq!(count, 3);
    }

    // --- pad_lines ---

    #[test]
    fn test_pad_lines_all_padded() {
        let lines = pad_lines(vec!["hi".to_owned(), "hello".to_owned()], 8);
        assert_eq!(lines[0].chars().count(), 8);
        assert_eq!(lines[1].chars().count(), 8);
    }

    #[test]
    fn test_pad_lines_no_truncation_for_wide() {
        let lines = pad_lines(vec!["hello world".to_owned()], 5);
        assert_eq!(lines[0], "hello world"); // not truncated
    }

    #[test]
    fn test_pad_lines_empty_input() {
        let lines: Vec<String> = pad_lines(vec![], 10);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_fit_to_box_wraps_and_caps() {
        let text = "one two three four five six seven eight nine ten";
        let lines = fit_to_box(text, 12, 3);
        assert!(lines.len() <= 3);
        for l in &lines {
            assert!(l.chars().count() <= 12);
        }
    }

    #[test]
    fn test_fit_to_box_no_overflow_adds_ellipsis() {
        let text = "word ".repeat(20);
        let lines = fit_to_box(&text, 10, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'), "overflowed line must end with …");
    }

    #[test]
    fn test_fit_to_box_zero_height_returns_empty() {
        let lines = fit_to_box("hello world", 20, 0);
        assert!(lines.is_empty());
    }
}
