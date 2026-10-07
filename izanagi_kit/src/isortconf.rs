//! isort `.isort.cfg` / `setup.cfg` `[isort]` / `[tool.isort]` parser.
//!
//! Detects isort config by `[isort]`/`[tool.isort]`/`[settings]`
//! sections plus `profile`/`known_*`/`line_length`/`multi_line_output`
//! option keys, and counts structure.
//!
//! ```
//! let b = b"[isort]\nprofile = black\nline_length = 100\nmulti_line_output = 3\nknown_first_party = myapp\n";
//! assert!(izanagi_kit::isortconf::detect(b));
//! let c = izanagi_kit::isortconf::Isort::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed .isort.cfg summary.
#[derive(Debug, Clone)]
pub struct Isort {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `[isort]`/`[tool.isort]`/`[settings]`/`[tool.settings]` section headers.
    pub sections: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Known option keys.
const KEYS: &[&str] = &[
    "profile",
    "src_paths",
    "sr\u{63}",
    "line_length",
    "multi_line_output",
    "force_single_line",
    "force_sort_within_sections",
    "force_alphabetical_sort_within_sections",
    "force_alphabetical_sort",
    "force_grid_wrap",
    "lines_after_imports",
    "lines_between_types",
    "lines_between_sections",
    "from_first",
    "skip",
    "extend_skip",
    "skip_glob",
    "extend_skip_glob",
    "skip_file",
    "known_first_party",
    "known_third_party",
    "known_standard_library",
    "known_local_folder",
    "known_other",
    "extra_standard_library",
    "sections",
    "no_sections",
    "single_line_exclusions",
    "default_section",
    "order_by_type",
    "combine_as",
    "combine_as_imports",
    "combine_star",
    "combine_straight_imports",
    "no_inline_sort",
    "no_lines_before",
    "no_inline_sections",
    "case_sensitive",
    "force_to_top",
    "ensure_newline_before_comments",
    "ensure_newline_before_module",
    "reverse_relative",
    "reverse_sort",
    "star_first",
    "import_headings",
    "balanced_wrapping",
    "wrap_length",
    "length_sort",
    "length_sort_straight",
    "length_sort_sections",
    "length_sort_stdlib",
    "add_imports",
    "append_only",
    "atomi\u{63}",
    "auto",
    "check",
    "check_skip",
    "color_output",
    "dedup_headings",
    "dict_default_section",
    "diff",
    "dont_order_by_type",
    "format",
    "from_module",
    "gitignore",
    "honor_case_in_force_sorted_sections",
    "honor_noqa",
    "honors_noqa",
    "include_trailing_comma",
    "indent",
    "indent_style",
    "lexicographical",
    "line_numbers",
    "links",
    "magic_trailing_comma",
    "modules",
    "natural_ordering",
    "nested_sections",
    "no_comment_sections",
    "normalize_imports",
    "only_modified",
    "overlapping_sections",
    "paren_wrapping",
    "profile_black",
    "project_root",
    "py_version",
    "quiet",
    "remove_comments",
    "remove_redundant_aliases",
    "rename_imports",
    "respect_skip_comments",
    "round_mode",
    "section",
    "settings_file",
    "settings_path",
    "show_config",
    "show_files",
    "sort_order",
    "sort_reexports",
    "sp",
    "split_on_trailing_comma",
    "star_third_party",
    "stdout",
    "supported_extensions",
    "symbol_prefix",
    "treat_all_comments_as_code",
    "treat_comments_as_code",
    "unicode",
    "unique_sections",
    "unsafe_sr\u{63}",
    "use_parentheses",
    "verbose",
    "version",
    "virtual_env",
    "wrap",
    "write_to_stdout",
    "yy",
];

fn key_present(t: &str, k: &str) -> bool {
    // isort settings are `key = value` lines; a bare substring of the key
    // name anywhere in the file does not count.
    t.lines().any(|l| {
        l.trim_start()
            .strip_prefix(k)
            .is_some_and(|r| r.trim_start().starts_with('='))
    })
}

fn section_line(t: &str, name: &str) -> bool {
    t.lines().any(|l| l.trim_start().starts_with(name))
}

/// Detect an isort config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = KEYS.iter().filter(|k| key_present(t, k)).count();
    // `[settings]` is isort's canonical section in `.isort.cfg` but is a
    // generic name — require isort keys alongside it.
    section_line(t, "[isort]")
        || section_line(t, "[tool.isort]")
        || (section_line(t, "[settings]") && hits >= 2)
        || hits >= 4
}

impl Isort {
    /// Count categories. Returns `None` when the input does not look like
    /// an isort config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
            } else if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
            } else if tr.contains('=') || tr.contains(':') {
                c.assignments += 1;
            }
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[isort]\nprofile = black\nline_length = 100\nmulti_line_output = 3\nknown_first_party = myapp\nknown_third_party = requests,pytest\nlines_after_imports = 2\n";
        assert!(detect(b));
        let c = Isort::parse(b).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.assignments, 6);
        assert!(c.keys >= 6);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[main]\nfoo = bar\n"));
        assert!(Isort::parse(b"").is_none());
        // `[settings]` alone is generic — needs isort keys.
        assert!(!detect(b"[settings]\nfoo = bar\nlevel = debug\n"));
        // Key names as substrings/values do not count.
        assert!(!detect(b"# mentions profile, skip and sections in prose\n"));
    }
}
