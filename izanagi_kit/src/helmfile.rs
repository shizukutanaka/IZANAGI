//! Helmfile `helmfile.yaml` census.
//!
//! Top-level sections: `releases:` (entries with `name:`,
//! `chart:`, `namespace:`, `values:`, `set:`, `installed:`,
//! `condition:`, `needs:`, `labels:`), `repositories:`,
//! `environments:`, `helmDefaults:`, `templates:`, `helmfiles:`
//! (sub-helmfile path list), `bases:`, `values:`, `secrets:`,
//! `commonLabels:`, `context`, `version`, `kubeContext`,
//! `helmBinary`, `lockFilePath`, `releaseTemplate`,
//! `helmTemplate`, `dependencies`, `hooks`, `prepareHooks`,
//! `preSync`, `postSync`.
//!
//! ```rust
//! let h = "releases:\n- name: app\n  chart: ./chart\n  namespace: prod\nrepositories:\n- name: stable\n  url: https://charts.example.com\n";
//! let c = izanagi_kit::helmfile::Helmfile::parse(h.as_bytes()).unwrap();
//! assert_eq!(c.releases, 1);
//! assert_eq!(c.repositories, 1);
//! ```

use crate::textutil::strip_bom;
/// helmfile.yaml census.
#[derive(Debug, Clone)]
pub struct Helmfile {
    /// `releases:` entries (`- name:` items).
    pub releases: usize,
    /// `repositories:` entries.
    pub repositories: usize,
    /// `environments:` entries.
    pub environments: usize,
    /// `helmfiles:` sub-helmfile paths.
    pub helmfiles: usize,
    /// Recognised top-level sections.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items (all).
    pub items: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "releases",
    "repositories",
    "environments",
    "helmDefaults",
    "templates",
    "helmfiles",
    "bases",
    "values",
    "secrets",
    "commonLabels",
    "context",
    "version",
    "kubeContext",
    "helmBinary",
    "lockFilePath",
    "releaseTemplate",
    "dependencies",
    "hooks",
    "prepareHooks",
    "helmVersion",
    "helmArgs",
];

/// Detect helmfile.yaml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut releases = false;
    let mut tops = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') || s.starts_with('-') {
            continue;
        }
        if let Some(colon) = s.find(':') {
            let key = &s[..colon];
            if TOP_KEYS.contains(&key) {
                tops += 1;
                if key == "releases" {
                    releases = true;
                }
            }
        }
    }
    releases || tops >= 3
}

impl Helmfile {
    /// Census a helmfile.yaml buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            releases: 0,
            repositories: 0,
            environments: 0,
            helmfiles: 0,
            sections: 0,
            settings: 0,
            items: 0,
            comments: 0,
        };
        let mut cur = "";
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                if cur == "releases" {
                    c.releases += 1;
                } else if cur == "repositories" {
                    c.repositories += 1;
                } else if cur == "environments" {
                    c.environments += 1;
                } else if cur == "helmfiles" {
                    c.helmfiles += 1;
                }
                continue;
            }
            if let Some(colon) = s.find(':') {
                let key = s[..colon].trim();
                if !line.starts_with(' ') && !line.starts_with('\t') {
                    cur = "";
                    if TOP_KEYS.contains(&key) {
                        c.sections += 1;
                        cur = key;
                    }
                }
                if !key.is_empty() {
                    c.settings += 1;
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
    fn detects_helmfile() {
        let b = b"releases:\n- name: app\n  chart: ./chart\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_helmfile() {
        let b = concat!(
            "# helmfile\n",
            "helmDefaults:\n",
            "  timeout: 600\n",
            "  wait: true\n",
            "repositories:\n",
            "- name: stable\n",
            "  url: https://charts.example.com\n",
            "- name: bitnami\n",
            "  url: oci://registry.example.com\n",
            "environments:\n",
            "  default:\n",
            "    values:\n",
            "    - values.yaml\n",
            "releases:\n",
            "- name: app\n",
            "  namespace: prod\n",
            "  chart: ./charts/app\n",
            "  installed: true\n",
            "  values:\n",
            "  - app-values.yaml\n",
            "  set:\n",
            "  - name: replicaCount\n",
            "    value: 3\n",
            "- name: db\n",
            "  namespace: prod\n",
            "  chart: bitnami/postgresql\n",
            "  needs:\n",
            "  - app\n",
            "helmfiles:\n",
            "- ./sub/helmfile.yaml\n",
            "- path: ./other/helmfile.yaml\n",
        );
        let c = Helmfile::parse(b.as_bytes()).unwrap();
        assert_eq!(c.releases, 5);
        assert_eq!(c.environments, 1);
        assert_eq!(c.repositories, 2);
        assert_eq!(c.helmfiles, 2);
        assert!(c.sections >= 4);
        assert!(c.items >= 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
