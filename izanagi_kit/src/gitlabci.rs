//! Parser for GitLab CI files (`.gitlab-ci.yml`).
//!
//! Counts `stages:` items, top-level job keys (non-reserved names),
//! `script:`/`include:`/`rules:`/`cache:`/`artifacts:`/`image:`/`needs:`/
//! `variables:` usages, and comments.
//!
//! ```
//! let b = b"stages:\n  - build\n  - test\nbuild:\n  script: make\ntest:\n  script: cargo test\n";
//! assert!(izanagi_kit::gitlabci::detect(b));
//! let g = izanagi_kit::gitlabci::Gitlabci::parse(b).unwrap();
//! assert_eq!(g.stages, 2);
//! assert_eq!(g.jobs, 2);
//! assert_eq!(g.script_blocks, 2);
//! ```

/// Parsed `.gitlab-ci.yml` summary.
#[derive(Debug, Clone)]
pub struct Gitlabci {
    /// `- ` items under `stages:`.
    pub stages: usize,
    /// Top-level non-reserved keys (job names, incl. `.hidden`).
    pub jobs: usize,
    /// `script:` blocks.
    pub script_blocks: usize,
    /// `before_script:`/`after_script:` blocks.
    pub wrapper_scripts: usize,
    /// Entries under `include:`.
    pub includes: usize,
    /// `rules:` blocks.
    pub rules: usize,
    /// `cache:` blocks.
    pub caches: usize,
    /// `artifacts:` blocks.
    pub artifacts: usize,
    /// `image:` keys.
    pub images: usize,
    /// `needs:` blocks.
    pub needs: usize,
    /// Entries under `variables:`.
    pub variables: usize,
    /// `services:` entries.
    pub services: usize,
    /// `#` comment lines.
    pub comments: usize,
}

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
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

fn dash_items(t: &str, key: &str) -> usize {
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
                .filter(|l| {
                    let tr = l.trim_start();
                    l.len() - tr.len() == at && tr.starts_with('-')
                })
                .count()
        })
        .sum()
}

/// Non-reserved top-level keys (`stages`, `include`, `default`, `workflow`,
/// `variables`, `spec` are not jobs).
fn jobs(t: &str) -> usize {
    const RESERVED: &[&str] = &[
        "stages",
        "include",
        "default",
        "workflow",
        "variables",
        "spec",
        "image",
        "services",
        "cache",
        "before_script",
        "after_script",
    ];
    t.lines()
        .filter(|l| {
            let tr = l.trim_start();
            if tr.is_empty() || tr.starts_with('#') || l.len() - tr.len() != 0 {
                return false;
            }
            if !tr.contains(':') || tr.split(':').next().map_or(true, str::is_empty) {
                return false;
            }
            RESERVED.iter().all(|r| !is_key(tr, r))
        })
        .count()
}

/// Returns `true` when `b` looks like a `.gitlab-ci.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.lines().any(|l| is_key(l.trim_start(), "stages"))
        || t.lines().any(|l| is_key(l.trim_start(), "script"))
        || t.lines().any(|l| is_key(l.trim_start(), "before_script"))
}

impl Gitlabci {
    /// Parses `b` as a `.gitlab-ci.yml`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let key_blocks = |key: &str| blocks(t, key).len();
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        Some(Self {
            stages: dash_items(t, "stages"),
            jobs: jobs(t),
            script_blocks: key_blocks("script"),
            wrapper_scripts: key_blocks("before_script") + key_blocks("after_script"),
            includes: child_items(t, "include").max(dash_items(t, "include")),
            rules: key_blocks("rules"),
            caches: key_blocks("cache"),
            artifacts: key_blocks("artifacts"),
            images: key_blocks("image"),
            needs: key_blocks("needs"),
            variables: child_items(t, "variables"),
            services: dash_items(t, "services"),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# ci
stages:
  - build
  - test
include:
  - local: ci/vars.yml
variables:
  CARGO_TERM_COLOR: always
build:
  image: rust:latest
  stage: build
  script:
    - make
  cache:
    paths:
      - target/
  artifacts:
    paths:
      - target/app
test:
  stage: test
  needs: [build]
  rules:
    - if: $CI_COMMIT_BRANCH
  script:
    - cargo test
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"foo: bar"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let g = Gitlabci::parse(SRC).unwrap();
        assert_eq!(g.stages, 2);
        assert_eq!(g.jobs, 2);
        assert_eq!(g.script_blocks, 2);
        assert_eq!(g.includes, 1);
        assert_eq!(g.rules, 1);
        assert_eq!(g.caches, 1);
        assert_eq!(g.artifacts, 1);
        assert_eq!(g.images, 1);
        assert_eq!(g.needs, 1);
        assert_eq!(g.variables, 1);
        assert_eq!(g.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Gitlabci::parse(b"\x01\x02").is_none());
    }
}
