//! rustfmt `rustfmt.toml`/`rustfmt.toml` 形式の検出と構造カウント。
//!
//! `max_width`/`edition`/`hard_tabs`/`newline_style`/`use_small_heuristics`/
//! `indent_style`/`imports_granularity`/`format_macro_matchers` 等の
//! rustfmt 固有オプション行を識別する。
//!
//! ```
//! let b = b"max_width = 100\nedition = \"2021\"\nhard_tabs = false\nnewline_style = \"Auto\"\nuse_small_heuristics = \"Default\"\n";
//! assert!(izanagi_kit::rustfmt::detect(b));
//! let c = izanagi_kit::rustfmt::Rustfmt::parse(b).unwrap();
//! assert_eq!(c.keys, 5);
//! ```

/// Parsed rustfmt.toml summary.
#[derive(Debug, Clone)]
pub struct Rustfmt {
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// rustfmt option names.
const KEYS: &[&str] = &[
    "array_width",
    "attr_fn_like_width",
    "binop_separator",
    "blank_lines_lower_bound",
    "blank_lines_upper_bound",
    "brace_style",
    "chain_width",
    "color",
    "combine_control_expr",
    "comment_width",
    "condense_wildcard_suffixes",
    "control_brace_style",
    "disable_all_formatting",
    "edition",
    "empty_item_single_line",
    "enum_discrim_align_threshold",
    "error_on_line_overflow",
    "error_on_unformatted",
    "file_lines",
    "fn_args_layout",
    "fn_call_width",
    "fn_single_line",
    "force_explicit_abi",
    "format_code_in_doc_comments",
    "format_generated_files",
    "format_macro_bodies",
    "format_macro_matchers",
    "format_strings",
    "generated_marker_line_search_limit",
    "group_imports",
    "hard_tabs",
    "ignore",
    "imports_granularity",
    "imports_indent",
    "imports_layout",
    "indent_style",
    "inline_attribute_width",
    "license_template_path",
    "match_arm_blocks",
    "match_arm_leading_pipes",
    "match_block_trailing_comma",
    "max_width",
    "merge_derives",
    "newline_style",
    "normalize_comments",
    "normalize_doc_attributes",
    "overflow_delimited_expr",
    "remove_nested_parens",
    "reorder_impl_items",
    "reorder_imports",
    "reorder_modules",
    "report_fixme",
    "report_todo",
    "reorder_type_params",
    "required_version",
    "short_array_element_width_threshold",
    "skip_children",
    "skip_macro_invocations",
    "space_after_colon",
    "space_before_colon",
    "spaces_around_ranges",
    "struct_field_align_threshold",
    "struct_lit_single_line",
    "struct_variant_width",
    "style_edition",
    "tab_spaces",
    "trailing_comma",
    "trailing_semicolon",
    "type_punctuation_density",
    "unstable_features",
    "use_field_init_shorthand",
    "use_small_heuristics",
    "use_try_shorthand",
    "version",
    "where_single_line",
    "wrap_comments",
];

fn key_present(t: &str, k: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.starts_with('#') && tr.split('=').next().is_some_and(|p| p.trim() == k)
    })
}

/// Detect a rustfmt config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| key_present(t, k)).count() >= 3
}

impl Rustfmt {
    /// Count categories. Returns `None` when the input does not look like
    /// a rustfmt config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
                if let Some(k) = tr.split('=').next() {
                    if KEYS.contains(&k.trim()) {
                        c.keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`Rustfmt::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<Rustfmt> {
    Rustfmt::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# rustfmt settings\nmax_width = 100\nedition = \"2021\"\nhard_tabs = false\ntab_spaces = 4\nnewline_style = \"Unix\"\nuse_small_heuristics = \"Max\"\nindent_style = \"Block\"\nimports_granularity = \"Crate\"\ngroup_imports = \"StdExternalCrate\"\nreorder_imports = true\nformat_macro_matchers = true\n";
        assert!(detect(b));
        let c = Rustfmt::parse(b).unwrap();
        assert_eq!(c.keys, 11);
        assert_eq!(c.assignments, 11);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_generic_toml() {
        assert!(!detect(
            b"[package]\nname = \"x\"\nversion = \"1.0.0\"\nedition = \"2021\"\n"
        ));
        assert!(!detect(b"foo = 1\nbar = 2\n"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(
            b"# max_width = 100\n# hard_tabs = false\n# newline_style = \"Auto\"\n"
        ));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(Rustfmt::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"max_width");
        assert!(!detect(&b));
    }
}
