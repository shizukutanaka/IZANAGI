//! Sphinx `conf.py` census.
//!
//! Sphinx config is Python assignments (`#` comments):
//! `project`, `author`, `copyright`, `version`, `release`,
//! `extensions`, `templates_path`, `exclude_patterns`,
//! `source_suffix`, `source_encoding`, `master_doc`,
//! `language`, `today_fmt`, `html_theme`, `html_theme_options`,
//! `html_static_path`, `html_logo`, `html_favicon`,
//! `html_sidebars`, `html_additional_pages`, `html_domain_indices`,
//! `htmlhelp_basename`, `latex_documents`, `latex_elements`,
//! `man_pages`, `texinfo_documents`, `epub_title`, `epub_author`,
//! `intersphinx_mapping`, `autodoc_*`, `autosummary_generate`,
//! `todo_include_todos`, `needs_sphinx`, `nitpicky`, `numfig`,
//! `rst_prolog`, `rst_epilog`, `pygments_style`, `default_role`,
//! `add_module_names`, `modindex_common_prefix`,
//! `linkcheck_*`, `gettext_uuid`, `gettext_compact`, `smartquotes`.
//!
//! ```rust
//! let k = b"project = 'x'\nextensions = ['sphinx.ext.autodoc']\ntemplates_path = ['_templates']\nhtml_theme = 'alabaster'\nmaster_doc = 'index'\n";
//! assert!(izanagi_kit::sphinx::detect(k));
//! ```

/// Sphinx conf.py census.
#[derive(Debug, Clone)]
pub struct Sphinx {
    /// `key = value` assignments.
    pub settings: usize,
    /// recognised Sphinx keys present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "extensions",
    "templates_path",
    "exclude_patterns",
    "source_suffix",
    "source_encoding",
    "master_doc",
    "html_theme",
    "html_theme_options",
    "html_static_path",
    "html_logo",
    "html_favicon",
    "html_sidebars",
    "html_additional_pages",
    "html_domain_indices",
    "html_use_index",
    "html_use_modindex",
    "html_use_opensearch",
    "htmlhelp_basename",
    "html_title",
    "html_short_title",
    "html_show_sourcelink",
    "html_show_sphinx",
    "html_show_copyright",
    "latex_documents",
    "latex_elements",
    "latex_logo",
    "latex_additional_files",
    "man_pages",
    "texinfo_documents",
    "epub_title",
    "epub_author",
    "epub_publisher",
    "epub_copyright",
    "intersphinx_mapping",
    "autodoc_member_order",
    "autodoc_default_flags",
    "autodoc_default_options",
    "autodoc_mock_imports",
    "autodoc_typehints",
    "autodoc_class_signature",
    "autosummary_generate",
    "todo_include_todos",
    "needs_sphinx",
    "nitpicky",
    "numfig",
    "rst_prolog",
    "rst_epilog",
    "pygments_style",
    "default_role",
    "add_module_names",
    "modindex_common_prefix",
    "gettext_uuid",
    "gettext_compact",
    "smartquotes",
    "linkcheck_ignore",
    "linkcheck_timeout",
    "linkcheck_retries",
    "linkcheck_workers",
    "linkcheck_anchors",
    "primary_domain",
    "keep_warnings",
    "suppress_warnings",
    "doctest_global_setup",
    "doctest_global_cleanup",
    "doctest_test_doctest_blocks",
    "doctest_path",
    "coverage_modules",
    "imgmath_latex",
    "mathjax_path",
    "graphviz_output_format",
    "extlinks",
    "figure_language_filename",
];

const WEAK: &[&str] = &[
    "project",
    "author",
    "copyright",
    "version",
    "release",
    "language",
    "today_fmt",
    "highlight_language",
    "needs_extensions",
    "needs_functions",
    "tls_verify",
    "user_agent",
    "sys",
    "os",
    "import",
    "logging",
    "mock",
];

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with('[') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"').trim_matches('\'');
            if k.is_empty() || k.contains(' ') || k.starts_with('[') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a Sphinx `conf.py`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `html_theme`/`latex_documents`/`intersphinx_mapping`/
    // `master_doc`/`extensions`/`autodoc_*` are Sphinx-exclusive.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if STRONG.contains(&k) || k.starts_with("autodoc_") || k.starts_with("extlinks") {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Sphinx {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = assign_key(line) {
                c.settings += 1;
                if STRONG.contains(&k) || WEAK.contains(&k) || k.starts_with("autodoc_") {
                    c.keys += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"project = 'x'\nextensions = ['sphinx.ext.autodoc']\ntemplates_path = ['_templates']\nhtml_theme = 'alabaster'\nmaster_doc = 'index'\n";
        assert!(detect(b));
        let c = Sphinx::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"project = 'x'\nversion = '1'\nauthor = 'y'\n"));
        assert!(!detect(
            b"# html_theme = 'x'\n# extensions = []\nproject = 'y'\n"
        ));
    }
}
