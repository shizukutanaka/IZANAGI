//! Mypy `mypy.ini` / `setup.cfg` `[mypy]` の検出と構造カウント。
//!
//! `[mypy]`/`[mypy-<module>]` セクションと `python_version`/`strict`/`warn_*`/
//! `disallow_*`/`ignore_missing_imports`/`plugins` 等の既知キーを分類する。
//!
//! ```
//! let c = izanagi_kit::mypyconf::parse(
//!     b"[mypy]\npython_version = 3.12\nwarn_return_any = True\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::mypyconf::detect(b"[mypy-requests]\nignore_missing_imports = True\n"));
//! ```

/// セクション名の既知パターン(`[mypy]` or `[mypy-*]`)。
fn is_section(name: &str) -> bool {
    name == "mypy" || name.starts_with("mypy-")
}
/// 既知オプションキー。
const KEYS: &[&str] = &[
    "abort_after_listening",
    "allow_redefinition",
    "allow_redefinition_new",
    "allow_untyped_defs",
    "always_false",
    "always_true",
    "atomic",
    "backpressure",
    "cache_dir",
    "cache_fine_grained",
    "check_untyped_defs",
    "color_output",
    "config_file",
    "custom_typeshed_dir",
    "disallow_any_decorated",
    "disallow_any_expr",
    "disallow_any_explicit",
    "disallow_any_generics",
    "disallow_any_unimported",
    "disallow_incomplete_defs",
    "disallow_subclassing_any",
    "disallow_untyped_calls",
    "disallow_untyped_decorators",
    "disallow_untyped_defs",
    "disable_bytearray_promotion",
    "disable_bytes_promotion",
    "disable_error_code",
    "disallow_any_explicit",
    "enable_error_code",
    "error_summary",
    "exclude",
    "explicit_package_bases",
    "extra_checks",
    "fast_module_lookup",
    "files",
    "follow_imports",
    "follow_imports_for_stubs",
    "hide_error_codes",
    "ignore_errors",
    "ignore_missing_imports",
    "implicit_reexport",
    "implicit_optional",
    "incremental",
    "install_types",
    "junit_xml",
    "junit_xsd",
    "local_partial_types",
    "log_config",
    "mypy_path",
    "namespace_packages",
    "no_site_packages",
    "non_interactive",
    "notify",
    "packages",
    "per_module_options",
    "plugins",
    "pretty",
    "python_executable",
    "python_version",
    "scripts_are_modules",
    "show_absolute_path",
    "show_column_numbers",
    "show_error_context",
    "skip_c_version_check",
    "skip_version_check",
    "sqlite_cache",
    "strict",
    "strict_bytes",
    "strict_concatenate",
    "strict_equality",
    "strict_equality_for_any",
    "strict_equality_for_none",
    "strict_optional",
    "txt_report",
    "warn_incomplete_stub",
    "warn_no_return",
    "warn_redundant_casts",
    "warn_return_any",
    "warn_unreachable",
    "warn_unused_configs",
    "warn_unused_ignores",
    "xunit_xml",
    "xml_report",
    "html_report",
    "linecount_report",
    "linecoverage_report",
    "cobertura_xml_report",
    "any_exprs_report",
    "text_report",
    "debug_cache",
    "debug_serialize",
    "variance",
    "version",
    "warn_deprecated_calls",
    "warn_untyped_fields",
];

/// Mypy 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[mypy]`/`[mypy-*]` セクション。
    pub sections: usize,
    /// 既知オプション行。
    pub options: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が mypy 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut keys = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && is_section(&t[1..t.len() - 1]) {
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
            if is_section(&t[1..t.len() - 1]) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if KEYS.contains(&t[..pos].trim()) {
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

    const SAMPLE: &[u8] = b"[mypy]\npython_version = 3.12\nwarn_return_any = True\nwarn_unused_configs = True\nignore_missing_imports = False\ndisallow_untyped_defs = True\ncheck_untyped_defs = True\nplugins = pydantic.mypy\nstrict_optional = True\n\n[mypy-requests.*]\nignore_missing_imports = True\n\n[mypy-numpy]\nfollow_imports = skip\n\n[other]\nx = 1\n";

    #[test]
    fn mypyconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.options, 10);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_mypy() {
        assert!(!detect(b"[flake8]\nmax-line-length = 100\n"));
        assert!(!detect(b"hello\n"));
    }
}
