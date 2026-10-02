//! isort `.isort.cfg` / `setup.cfg` `[isort]` / `pyproject` `[tool.isort]` の検出と構造カウント。
//!
//! `profile`/`multi_line_output`/`known_*`/`force_*`/`lines_between_*` 等の
//! 既知キーをセクション認識で分類する。
//!
//! ```
//! let c = izanagi_kit::isortconf::parse(
//!     b"[isort]\nprofile = black\nmulti_line_output = 3\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::isortconf::detect(b"[tool.isort]\nprofile = \"black\"\n"));
//! ```

/// セクション名の既知パターン。
const SECTIONS: &[&str] = &["isort", "tool.isort", "settings"];
/// 既知オプションキー。
const KEYS: &[&str] = &[
    "add_imports",
    "append_only",
    "atomic",
    "auto_identify_namespace_packages",
    "balanced_wrapping",
    "case_insensitive",
    "case_sensitive",
    "check",
    "check_skip",
    "color_output",
    "combine_as_imports",
    "combine_star",
    "comment_prefix",
    "dedup_headings",
    "default_section",
    "diff",
    "dont_follow_links",
    "ensure_newline_before_comments",
    "extend_skip",
    "extend_skip_glob",
    "extra_standard_library",
    "file_name",
    "filter_files",
    "float_to_top",
    "follow_links",
    "force_alphabetical_sort",
    "force_alphabetical_sort_within_sections",
    "force_grid_wrap",
    "force_single_line",
    "force_sort_within_sections",
    "force_to_top",
    "forced_separate",
    "format_error",
    "format_success",
    "from_first",
    "group_by_package",
    "honor_noqa",
    "ignore_comments",
    "ignore_whitespace",
    "import_headings",
    "include_trailing_comma",
    "indent",
    "jobs",
    "known_custom_local_folder",
    "known_first_party",
    "known_local_folder",
    "known_other",
    "known_pattern",
    "known_standard_library",
    "known_third_party",
    "length_sort",
    "length_sort_straight",
    "line_length",
    "lines_after_imports",
    "lines_between_sections",
    "lines_between_types",
    "magical_comment_prefix",
    "multi_line_output",
    "no_inline_sort",
    "no_lines_before",
    "no_section",
    "no_sections",
    "not_skip",
    "only_modified",
    "order_by_type",
    "overwrite_in_place",
    "profile",
    "py_version",
    "remove_imports",
    "remove_redundant_aliases",
    "rename_imports",
    "reverse_relative",
    "reverse_sort",
    "section_order",
    "sections",
    "settings_file",
    "show_config",
    "show_files",
    "single_line_exclusions",
    "skip",
    "skip_gitignore",
    "skip_glob",
    "slip",
    "sort_order",
    "sort_reexports",
    "sources",
    "split_on_trailing_comma",
    "src_paths",
    "star_first",
    "supported_extensions",
    "title_case",
    "top",
    "treat_all_comments_as_code",
    "treat_comments_as_code",
    "use_parentheses",
    "verbose",
    "verbose_success",
    "virtual_env",
    "wrap_length",
];

/// isort 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[isort]`/`[tool.isort]` セクション。
    pub sections: usize,
    /// 既知オプション行。
    pub options: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が isort 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut keys = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && SECTIONS.contains(&&t[1..t.len() - 1]) {
            secs += 1;
        } else if t.find('=').is_some_and(|p| KEYS.contains(&t[..p].trim())) {
            keys += 1;
        }
    }
    secs >= 1 && keys >= 1
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_isort = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            let name = &t[1..t.len() - 1];
            if SECTIONS.contains(&name) {
                c.sections += 1;
                in_isort = true;
            } else {
                in_isort = false;
                c.misc += 1;
            }
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if in_isort && KEYS.contains(&t[..pos].trim()) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[tool.isort]\nprofile = \"black\"\nmulti_line_output = 3\nline_length = 100\nknown_first_party = [\"myapp\"]\nknown_third_party = [\"requests\"]\nforce_sort_within_sections = true\nlines_between_sections = 1\nskip_glob = [\"*/migrations/*\"]\n\n[tool.black]\nline-length = 100\n";

    #[test]
    fn isortconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.options, 8);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_isort() {
        assert!(!detect(b"[mypy]\npython_version = 3.12\n"));
        assert!(!detect(b"hello\n"));
    }
}
