//! Parser for Pulumi project files (`Pulumi.yaml`).
//!
//! Extracts `name`/`runtime`, flags `description`/`main`/`backend`/
//! `template`/`website`/`license`, counts `config:` keys, `plugins:` and
//! `resources:` entries, and comments.
//!
//! ```
//! let b = b"name: infra\nruntime: nodejs\nconfig:\n  aws:region: us-east-1\n";
//! assert!(izanagi_kit::pulumi::detect(b));
//! let p = izanagi_kit::pulumi::Pulumi::parse(b).unwrap();
//! assert_eq!(p.name, "infra");
//! assert_eq!(p.runtime, "nodejs");
//! assert_eq!(p.config_keys, 1);
//! ```

/// Parsed `Pulumi.yaml` summary.
#[derive(Debug, Clone)]
pub struct Pulumi {
    /// `name:` value.
    pub name: String,
    /// `runtime:` value (`nodejs`/`python`/`go`/`dotnet`/`java`/`yaml`).
    pub runtime: String,
    /// `description:` present.
    pub description: bool,
    /// `main:` present.
    pub main: bool,
    /// `backend:`/`url:` section present.
    pub backend: bool,
    /// Keys under `config:` blocks.
    pub config_keys: usize,
    /// Entries under `plugins:`.
    pub plugins: usize,
    /// Entries under `resources:` (YAML programs).
    pub resources: usize,
    /// Entries under `variables:` (YAML programs).
    pub variables: usize,
    /// `template:` section present.
    pub template: bool,
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
        // `Pulumi.yaml` keys are top-level (indent 0) — nested `config:` under
        // `template:` must not be counted as the project `config:` block.
        if cur.is_none() && !tr.is_empty() && i == 0 && is_key(tr, key) {
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

const RUNTIMES: &[&str] = &["nodejs", "python", "go", "dotnet", "java", "yaml"];

/// Returns `true` when `b` looks like a `Pulumi.yaml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    has_key(t, "name")
        && val_after(t, "runtime").is_some_and(|r| RUNTIMES.iter().any(|rt| r.starts_with(rt)))
}

impl Pulumi {
    /// Parses `b` as a `Pulumi.yaml` file.
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
        Some(Self {
            name: val_after(t, "name").unwrap_or("").to_string(),
            runtime: val_after(t, "runtime").unwrap_or("").to_string(),
            description: has_key(t, "description"),
            main: has_key(t, "main"),
            backend: has_key(t, "backend"),
            config_keys: child_items(t, "config"),
            plugins: child_items(t, "plugins"),
            resources: child_items(t, "resources"),
            variables: child_items(t, "variables"),
            template: has_key(t, "template"),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# stack
name: infra
runtime: nodejs
description: stack
main: src/
backend:
  url: s3://b
config:
  aws:region: us-east-1
  project:k: v
plugins:
  providers:
    - name: aws
      version: \"6.x\"
resources:
  bucket:
    type: aws:s3:Bucket
template:
  config:
    k: v
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"name: x\nruntime: none"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let p = Pulumi::parse(SRC).unwrap();
        assert_eq!(p.name, "infra");
        assert_eq!(p.runtime, "nodejs");
        assert!(p.description);
        assert!(p.main);
        assert!(p.backend);
        assert_eq!(p.config_keys, 2);
        assert_eq!(p.plugins, 1);
        assert_eq!(p.resources, 1);
        assert!(p.template);
        assert_eq!(p.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Pulumi::parse(b"\x01\x02").is_none());
    }
}
