//! Parser for CircleCI configs (`.circleci/config.yml`).
//!
//! Extracts the top-level `version`, counts `jobs:`/`workflows:`/
//! `executors:`/`commands:`/`orbs:` entries, `- ` step items, and
//! `checkout`/`run`/`docker` image usages.
//!
//! ```
//! let b = b"version: 2\x2e1\njobs:\n  build:\n    docker:\n      - image: cimg/base\nworkflows:\n  main:\n    jobs:\n      - build\n";
//! assert!(izanagi_kit::circleci::detect(b));
//! let c = izanagi_kit::circleci::Circleci::parse(b).unwrap();
//! assert_eq!(c.jobs, 1);
//! assert_eq!(c.workflows, 1);
//! assert_eq!(c.docker_images, 1);
//! ```

/// Parsed `.circleci/config.yml` summary.
#[derive(Debug, Clone)]
pub struct Circleci {
    /// Top-level `version:` (`2`/`2\x2e1`).
    pub version: String,
    /// Entries under `jobs:`.
    pub jobs: usize,
    /// Entries under `workflows:` (workflow names, `version:` excluded).
    pub workflows: usize,
    /// Entries under `executors:`.
    pub executors: usize,
    /// Entries under `commands:`.
    pub commands: usize,
    /// Entries under `orbs:`.
    pub orbs: usize,
    /// `- ` items inside `steps:` blocks.
    pub steps: usize,
    /// `- checkout` step usages.
    pub checkouts: usize,
    /// `- run` step usages.
    pub run_steps: usize,
    /// `- image:`/`image:` entries inside `docker:` blocks.
    pub docker_images: usize,
    /// `machine:` keys.
    pub machines: usize,
    /// `macos:` keys.
    pub macos: usize,
    /// `requires:` entries (workflow DAG edges).
    pub requires: usize,
    /// `#` comment lines.
    pub comments: usize,
}

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn blocks<'a>(t: &'a str, key: &str) -> Vec<Vec<&'a str>> {
    blocks_at(t, key, None)
}

/// Blocks whose header sits at indent 0 — `jobs:` inside `workflows:` must
/// not be counted as a top-level `jobs:` section.
fn top_blocks<'a>(t: &'a str, key: &str) -> Vec<Vec<&'a str>> {
    blocks_at(t, key, Some(0))
}

fn blocks_at<'a>(t: &'a str, key: &str, hdr: Option<usize>) -> Vec<Vec<&'a str>> {
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
        if cur.is_none() && !tr.is_empty() && is_key(tr, key) && hdr.map_or(true, |h| i == h) {
            cur = Some((i, Vec::new()));
        }
    }
    if let Some((_, v)) = cur.take() {
        out.push(v);
    }
    out
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

/// Loose children (any non-blank line) at the shallowest indent of
/// top-level `key:` blocks.
fn top_items(t: &str, key: &str) -> usize {
    top_blocks(t, key)
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

/// `name:`-style keys (ending `:`) at the shallowest indent of top-level
/// `key:` blocks — skips `version: 2` inside `workflows:`.
fn top_keys(t: &str, key: &str) -> usize {
    top_blocks(t, key)
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
                    l.len() - tr.len() == at && tr.ends_with(':') && !tr.starts_with('-')
                })
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
                let v = v.trim().trim_matches(['"', '\'']).trim_end_matches(',');
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Returns `true` when `b` looks like a `.circleci/config.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let has = |k: &str| t.lines().any(|l| is_key(l.trim_start(), k));
    has("orbs") || (has("jobs") && has("workflows")) || (has("executors") && has("jobs"))
}

impl Circleci {
    /// Parses `b` as a CircleCI config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let checkouts = t
            .lines()
            .filter(|l| l.trim_start().starts_with("- checkout"))
            .count();
        let run_steps = t
            .lines()
            .filter(|l| l.trim_start().starts_with("- run"))
            .count();
        let docker_images = blocks(t, "docker")
            .iter()
            .flatten()
            .filter(|l| {
                let tr = l.trim_start();
                tr.contains("image:")
            })
            .count();
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        Some(Self {
            version: val_after(t, "version").unwrap_or("").to_string(),
            jobs: top_items(t, "jobs"),
            workflows: top_keys(t, "workflows"),
            executors: top_keys(t, "executors"),
            commands: top_keys(t, "commands"),
            orbs: top_items(t, "orbs"),
            steps: dash_items(t, "steps"),
            checkouts,
            run_steps,
            docker_images,
            machines: blocks(t, "machine").len(),
            macos: blocks(t, "macos").len(),
            requires: blocks(t, "requires").len(),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# ci
version: 2\x2e1
orbs:
  go: circleci/go@1
jobs:
  build:
    docker:
      - image: cimg/go:1
    steps:
      - checkout
      - run: go test
  test:
    docker:
      - image: cimg/base
    steps:
      - run: echo hi
workflows:
  version: 2
  main:
    jobs:
      - build
      - test
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"version: 2"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let c = Circleci::parse(SRC).unwrap();
        assert_eq!(c.version, "2\x2e1");
        assert_eq!(c.orbs, 1);
        assert_eq!(c.jobs, 2);
        assert_eq!(c.workflows, 1);
        assert_eq!(c.steps, 3);
        assert_eq!(c.checkouts, 1);
        assert_eq!(c.run_steps, 2);
        assert_eq!(c.docker_images, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Circleci::parse(b"\x01\x02").is_none());
    }
}
