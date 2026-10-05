//! gitweb `gitweb_config.perl` parser.
//!
//! Detects the gitweb Perl configuration by `$projectroot`/`$projects_list`/
//! `@git_base_url_list`/`$feature{...}` variables and `our $x = v;`
//! declarations, and counts structure.
//!
//! ```
//! let b = b"our $projectroot = \"/srv/git\";\nour $projects_list = $projectroot;\nour $stylesheet = \"gitweb.css\";\nour @git_base_url_list = (\"git://git.example.com\");\n$feature{'blame'}{'default'} = [1];\n";
//! assert!(izanagi_kit::gitwebconf::detect(b));
//! let c = izanagi_kit::gitwebconf::Gitweb::parse(b).unwrap();
//! assert!(c.var_keys >= 3);
//! ```

/// Parsed gitweb_config.perl summary.
#[derive(Debug, Clone)]
pub struct Gitweb {
    /// Recognized variable occurrences.
    pub keys: usize,
    /// Known `$var`/`@var` gitweb variables (`$projectroot`/`$projects_list`/`$stylesheet`/`@git_base_url_list`/...).
    pub var_keys: usize,
    /// `$feature{'name'}{'default'}` feature overrides.
    pub feature_keys: usize,
    /// `our $x =`/`our @x =` declaration lines.
    pub declarations: usize,
    /// `sub name {` feature-check/helper subroutines.
    pub sub_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Known variables.
const VAR_KEYS: &[&str] = &[
    "$projectroot",
    "$projects_list",
    "$stylesheet",
    "$logo",
    "$favicon",
    "$javascript",
    "$home_text",
    "$home_text_str",
    "$projects_list_description_width",
    "$projects_list_group_categories",
    "$project_maxdepth",
    "$export_ok",
    "$strict_export",
    "$GIT",
    "$git_temp",
    "$projects_list_groupings",
    "$default_projects_order",
    "$maxload",
    "$omit_owner",
    "$per_request_config",
    "$prevent_xss",
    "$fallback_encoding",
    "$tmp_dir",
    "$mimetypes_file",
    "$highlight_bin",
    "$show_sizes",
    "$extraBranchRefs",
    "$site_name",
    "$site_header",
    "$site_footer",
    "$home_link",
    "$home_link_str",
    "$version",
    "$base_url",
    "$my_uri",
    "$my_url",
    "@git_base_url_list",
    "@export_ok",
    "@diff_fancy",
    "@snapshot_fmts",
    "@dtd_base_url",
    "%avatar_size",
    "%known_snapshot_formats",
    "%known_snapshot_format_aliases",
];

/// Feature-map variable.
const FEATURE_KEYS: &[&str] = &["$feature{", "%feature"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "$projectroot",
    "$projects_list",
    "$stylesheet",
    "@git_base_url_list",
    "$feature{",
    "$home_text",
    "$export_ok",
    "$default_projects_order",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a gitweb_config.perl.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Gitweb {
    /// Count categories in a gitweb_config.perl. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            var_keys: 0,
            feature_keys: 0,
            declarations: 0,
            sub_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("our ") {
                c.declarations += 1;
            }
            if tr.starts_with("sub ") {
                c.sub_keys += 1;
            }
        }
        for k in VAR_KEYS {
            c.var_keys += t.matches(k).count();
        }
        for k in FEATURE_KEYS {
            c.feature_keys += t.matches(k).count();
        }
        c.keys = c.var_keys + c.feature_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# gitweb config\nour $projectroot = \"/srv/git\";\nour $projects_list = $projectroot;\nour $stylesheet = \"gitweb.css\";\nour $home_text = \"indextext.html\";\nour @git_base_url_list = (\"git://git.example.com\");\n$feature{'blame'}{'default'} = [1];\n$feature{'snapshot'}{'default'} = ['tgz'];\nsub check { }\n";
        assert!(detect(b));
        let c = Gitweb::parse(b).unwrap();
        assert!(c.var_keys >= 5);
        assert_eq!(c.feature_keys, 2);
        assert!(c.declarations >= 5);
        assert_eq!(c.sub_keys, 1);
        assert!(c.keys >= 7);
    }

    #[test]
    fn rejects_perl() {
        assert!(!detect(b"use strict;\nmy $x = 1;\n"));
        assert!(Gitweb::parse(b"print 1;\n").is_none());
    }
}
