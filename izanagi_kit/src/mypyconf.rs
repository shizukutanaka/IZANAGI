//! mypy `mypy.ini` / `[mypy]` parser.
//!
//! Detects mypy config by `[mypy]` / `[mypy-<module>]` sections plus
//! `strict`/`disallow_*`/`warn_*`/`check_untyped_defs`/`follow_imports`
//! option keys, and counts structure.
//!
//! ```
//! let b = b"[mypy]\nstrict = True\ndisallow_untyped_defs = True\nwarn_return_any = True\n[mypy-requests]\nignore_missing_imports = True\n";
//! assert!(izanagi_kit::mypyconf::detect(b));
//! let c = izanagi_kit::mypyconf::Mypy::parse(b).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// Parsed mypy.ini summary.
#[derive(Debug, Clone)]
pub struct Mypy {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `[mypy]`/`[mypy-*]` section headers.
    pub sections: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Known option keys.
const KEYS: &[&str] = &[
    "strict",
    "mypy_path",
    "files",
    "modules",
    "packages",
    "exclude",
    "namespace_packages",
    "explicit_package_bases",
    "ignore_missing_imports",
    "follow_imports",
    "follow_imports_for_stubs",
    "follow_untyped_imports",
    "python_executable",
    "no_site_packages",
    "no_silence_site_packages",
    "no_fast_deps",
    "platform",
    "python_version",
    "disallow_untyped_calls",
    "untyped_calls_exclude",
    "disallow_untyped_defs",
    "disallow_incomplete_defs",
    "disallow_untyped_decorators",
    "implicit_optional",
    "strict_optional",
    "warn_return_any",
    "warn_unused_configs",
    "warn_unused_ignores",
    "warn_no_return",
    "warn_unreachable",
    "warn_redundant_casts",
    "warn_awaitable",
    "warn_incomplete_stub",
    "strict_equality",
    "disable_error_code",
    "enable_error_code",
    "implicit_reexport",
    "allow_redefinition",
    "local_partial_types",
    "incremental",
    "cache_dir",
    "sqlite_cache",
    "custom_typeshed_dir",
    "warn_incomplete_override",
    "mangle_none_value",
    "always_true",
    "always_false",
    "strict_bytes",
    "disallow_subclassing_any",
    "disallow_any_generics",
    "disallow_any_expr",
    "disallow_any_explicit",
    "disallow_any_decorated",
    "disallow_any_unimported",
    "check_untyped_defs",
    "check_untyped_defs_before_timeout",
    "slow_patching",
    "plugins",
    "env_file",
    "junit_xml",
    "cobertura_xml_report",
    "html_report",
    "linecount_report",
    "linecoverage_report",
    "txt_report",
    "xml_report",
    "any_exprs_report",
    "unused_ignores_report",
    "force_uppercase_builtins",
    "force_union_syntax",
    "pretty",
    "show_column_numbers",
    "show_error_codes",
    "hide_error_context",
    "color_output",
    "error_summary",
    "show_absolute_path",
    "soft_error_limit",
    "install_types",
    "non_interactive",
    "python_version",
    "fast_exit",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a mypy config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if t.contains("[mypy]") || t.contains("[mypy-") {
        return true;
    }
    let hits = KEYS.iter().filter(|k| key_present(t, k)).count();
    hits >= 4
}

impl Mypy {
    /// Count categories. Returns `None` when the input does not look like
    /// a mypy config.
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
        let b = b"[mypy]\nstrict = True\ndisallow_untyped_defs = True\nwarn_return_any = True\npython_version = 3.11\nplugins = pydantic.mypy\n[mypy-requests]\nignore_missing_imports = True\n";
        assert!(detect(b));
        let c = Mypy::parse(b).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.assignments, 6);
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[main]\nfoo = bar\n"));
        assert!(Mypy::parse(b"").is_none());
    }
}
