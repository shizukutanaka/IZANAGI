//! Parser for Helm `Chart.yaml` files.
//!
//! Extracts `apiVersion`/`name`/`version`/`type`/`appVersion`, counts
//! `dependencies:`/`keywords:`/`maintainers:`/`sources:` items, flags
//! `kubeVersion`/`deprecated`/`icon`/`home`, and comments.
//!
//! ```
//! let b = b"apiVersion: v2\nname: webapp\ntype: application\nversion: 0\x2e1\x2e0\ndependencies:\n  - name: redis\n";
//! assert!(izanagi_kit::chart::detect(b));
//! let c = izanagi_kit::chart::Chart::parse(b).unwrap();
//! assert_eq!(c.name, "webapp");
//! assert_eq!(c.api_version, "v2");
//! assert_eq!(c.dependencies, 1);
//! ```

/// Parsed Helm chart metadata summary.
#[derive(Debug, Clone)]
pub struct Chart {
    /// `apiVersion:` value (`v1`/`v2`).
    pub api_version: String,
    /// `name:` value.
    pub name: String,
    /// `version:` value (SemVer).
    pub version: String,
    /// `type:` value (`application`/`library`, empty for v1).
    pub chart_type: String,
    /// `appVersion:` value.
    pub app_version: String,
    /// `- ` items under `dependencies:`.
    pub dependencies: usize,
    /// `- ` items under `keywords:`.
    pub keywords: usize,
    /// `- ` items under `maintainers:`.
    pub maintainers: usize,
    /// `- ` items under `sources:`.
    pub sources: usize,
    /// `kubeVersion:` present.
    pub kube_version: bool,
    /// `deprecated: true`.
    pub deprecated: bool,
    /// `icon:` present.
    pub icon: bool,
    /// `home:`/`home icon` present.
    pub home: bool,
    /// `#` comment lines.
    pub comments: usize,
}

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim_start(), key))
}

fn blocks<'a>(t: &'a str, key: &str) -> Vec<Vec<&'a str>> {
    let mut out: Vec<Vec<&'a str>> = Vec::new();
    let mut cur: Option<(usize, Vec<&'a str>)> = None;
    for l in t.lines() {
        let tr = l.trim_start();
        let i = l.len() - tr.len();
        if let Some((d, v)) = cur.as_mut() {
            if !tr.is_empty() && i <= *d {
                out.push(core::mem::take(v));
                cur = None;
            } else {
                v.push(l);
                continue;
            }
        }
        if cur.is_none() && !tr.is_empty() && is_key(tr, key) {
            cur = Some((i, Vec::new()));
        }
    }
    if let Some((_, v)) = cur.take() {
        out.push(v);
    }
    out
}

/// Non-blank lines at the shallowest indent inside each block.
fn child_items(t: &str, key: &str) -> usize {
    blocks(t, key)
        .iter()
        .map(|b| {
            let at = b
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            b.iter()
                .filter(|l| !l.trim().is_empty() && l.len() - l.trim_start().len() == at)
                .count()
        })
        .sum()
}

fn val_after<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for l in t.lines() {
        let tr = l.trim();
        if let Some(rest) = tr.trim_start_matches(['"', '\'']).strip_prefix(key) {
            let rest = rest.trim_start_matches(['"', '\'']).trim_start();
            if let Some(v) = rest.strip_prefix(':') {
                let v = v
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .trim_end_matches(',');
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Returns `true` when `b` looks like a Helm `Chart.yaml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    has_key(t, "apiVersion")
        && has_key(t, "name")
        && has_key(t, "version")
        && (t.contains("appVersion")
            || t.contains("type: application")
            || t.contains("type: library")
            || has_key(t, "dependencies"))
}

impl Chart {
    /// Parses `b` as a Helm `Chart.yaml`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        let deprecated = t
            .lines()
            .any(|l| is_key(l.trim_start(), "deprecated") && l.contains("true"));
        Some(Self {
            api_version: val_after(t, "apiVersion").unwrap_or("").to_string(),
            name: val_after(t, "name").unwrap_or("").to_string(),
            version: val_after(t, "version").unwrap_or("").to_string(),
            chart_type: val_after(t, "type").unwrap_or("").to_string(),
            app_version: val_after(t, "appVersion").unwrap_or("").to_string(),
            dependencies: child_items(t, "dependencies"),
            keywords: child_items(t, "keywords"),
            maintainers: child_items(t, "maintainers"),
            sources: child_items(t, "sources"),
            kube_version: has_key(t, "kubeVersion"),
            deprecated,
            icon: has_key(t, "icon"),
            home: has_key(t, "home"),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# chart
apiVersion: v2
name: webapp
description: demo
type: application
version: 0\x2e1\x2e0
appVersion: \"1\x2e0\"
kubeVersion: \">=1\x2e20\"
keywords:
  - web
  - app
maintainers:
  - name: a
dependencies:
  - name: redis
    version: \"17.x\"
  - name: pg
deprecated: true
sources:
  - https://x
icon: https://i
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"name: x\nversion: 1"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let c = Chart::parse(SRC).unwrap();
        assert_eq!(c.api_version, "v2");
        assert_eq!(c.name, "webapp");
        assert_eq!(c.version, "0\x2e1\x2e0");
        assert_eq!(c.chart_type, "application");
        assert_eq!(c.app_version, "1\x2e0");
        assert_eq!(c.dependencies, 2);
        assert_eq!(c.keywords, 2);
        assert_eq!(c.maintainers, 1);
        assert_eq!(c.sources, 1);
        assert!(c.kube_version);
        assert!(c.deprecated);
        assert!(c.icon);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Chart::parse(b"\x01\x02").is_none());
    }
}
