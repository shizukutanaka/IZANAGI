//! `setup.cfg` (setuptools INI) census.
//!
//! `[metadata]`/`[options]`/`[options.packages.find]`/
//! `[options.entry_points]`/`[options.extras_require]`/`[aliases]`/
//! `[bdist_wheel]`/`[sdist]`/`[egg_info]`/`[build_ext]`/`[flake8]`/
//! `[coverage:run]`/`[coverage:report]`/`[mypy]`/`[mypy-*]`/`[tool:pytest]`/
//! `[isort]`/`[pylint]`/`[pycodestyle]` + `key = value`/`key =` multi-line.
//!
//! ```rust
//! let s = "[metadata]\nname = mypkg\nversion = 1.0\n[options]\ninstall_requires =\n    requests\n";
//! let c = izanagi_kit::setupcfg::Setupcfg::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// setup.cfg census.
#[derive(Debug, Clone)]
pub struct Setupcfg {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value`/`key =` entries.
    pub settings: usize,
    /// Recognised setuptools/tool key names.
    pub named: usize,
    /// Continuation lines (indented list items).
    pub items: usize,
}

const KEYS: &[&str] = &[
    "name",
    "version",
    "description",
    "long_description",
    "long_description_content_type",
    "author",
    "author_email",
    "maintainer",
    "maintainer_email",
    "license",
    "license_file",
    "license_files",
    "url",
    "download_url",
    "project_urls",
    "classifiers",
    "keywords",
    "platforms",
    "provides",
    "requires",
    "obsoletes",
    "install_requires",
    "setup_requires",
    "python_requires",
    "packages",
    "package_dir",
    "package_data",
    "exclude_package_data",
    "include_package_data",
    "data_files",
    "py_modules",
    "zip_safe",
    "eager_resources",
    "namespace_packages",
    "dependency_links",
    "tests_require",
    "test_suite",
    "test_loader",
    "use_2to3",
    "convert_2to3_doctests",
    "entry_points",
    "console_scripts",
    "gui_scripts",
    "extras_require",
    "find",
    "where",
    "include",
    "exclude",
    "namespaces",
    "universal",
    "tag_date",
    "tag_build",
    "tag_svn_revision",
    "release",
    "source",
    "branch",
    "commit",
    "attr",
    "src",
    "check",
    "strict",
    "all_errors",
    "warn_return_any",
    "warn_unused_configs",
    "disallow_untyped_defs",
    "disallow_incomplete_defs",
    "check_untyped_defs",
    "disallow_untyped_decorators",
    "no_implicit_optional",
    "warn_redundant_casts",
    "warn_unused_ignores",
    "show_error_codes",
    "mypy_path",
    "files",
    "exclude",
    "ignore_missing_imports",
    "follow_imports",
    "python_version",
    "platform",
    "plugins",
    "select",
    "ignore",
    "extend-ignore",
    "per-file-ignores",
    "max-line-length",
    "max-complexity",
    "count",
    "statistics",
    "show-source",
    "enable-extensions",
    "require-plugins",
    "omit",
    "branch",
    "source_pkgs",
    "source",
    "cover_pylib",
    "data_file",
    "parallel",
    "concurrency",
    "contexts",
    "include_namespace_packages",
    "sign",
    "identity",
    "dist_dir",
    "keep_temp",
    "universal",
];

/// Whether the buffer looks like a setup.cfg.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[metadata]")
        || t.contains("[options]")
        || t.contains("[options.entry_points]")
        || t.contains("[options.extras_require]")
        || t.contains("install_requires")
        || t.contains("console_scripts")
        || t.contains("[aliases]")
        || t.contains("[bdist_wheel]")
        || t.contains("[tool:pytest]")
}

impl Setupcfg {
    /// Parse a setup.cfg into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            named: 0,
            items: 0,
        };
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with(['#', ';']) {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
                continue;
            }
            if let Some(eq) = s.find('=') {
                c.settings += 1;
                let key = s[..eq].trim();
                if KEYS.contains(&key) {
                    c.named += 1;
                }
            } else if indent > 0 {
                c.items += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cfg() {
        let b = concat!(
            "[metadata]\n",
            "name = mypkg\n",
            "version = attr: mypkg.__version__\n",
            "description = A package\n",
            "author = Dev\n",
            "author_email = dev@x.io\n",
            "license = MIT\n",
            "classifiers =\n",
            "    Programming Language :: Python :: 3\n",
            "    License :: OSI Approved :: MIT License\n",
            "keywords =\n",
            "    sample\n",
            "project_urls =\n",
            "    Source = https://github.com/x/y\n",
            "[options]\n",
            "packages = find:\n",
            "python_requires = >=3.9\n",
            "install_requires =\n",
            "    requests\n",
            "    click>=8\n",
            "    rich\n",
            "include_package_data = True\n",
            "zip_safe = False\n",
            "[options.packages.find]\n",
            "where = src\n",
            "exclude =\n",
            "    tests*\n",
            "[options.entry_points]\n",
            "console_scripts =\n",
            "    mypkg = mypkg.cli:main\n",
            "[options.extras_require]\n",
            "dev =\n",
            "    pytest\n",
            "    black\n",
            "docs =\n",
            "    sphinx\n",
            "[bdist_wheel]\n",
            "universal = 0\n",
        );
        let c = Setupcfg::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.settings, 23);
        assert_eq!(c.items, 9);
        assert!(c.named >= 15);
    }

    #[test]
    fn rejects_other() {
        assert!(Setupcfg::parse(b"[foo]\na = 1").is_none());
    }
}
