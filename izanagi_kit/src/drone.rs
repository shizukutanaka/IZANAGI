//! Parser for Drone CI files (`.drone.yml`).
//!
//! Extracts `kind:`/`type:`/`name:`, counts `- ` items under `steps:`,
//! `commands:`/`image:`/`volumes:`/`services:`/`depends_on:`/
//! `environment:` usages, and comments.
//!
//! ```
//! let b = b"kind: pipeline\ntype: docker\nname: build\nsteps:\n  - name: test\n    image: golang\n    commands:\n      - go test\n";
//! assert!(izanagi_kit::drone::detect(b));
//! let d = izanagi_kit::drone::Drone::parse(b).unwrap();
//! assert_eq!(d.kind, "pipeline");
//! assert_eq!(d.steps, 1);
//! assert_eq!(d.images, 1);
//! ```

/// Parsed `.drone.yml` summary.
#[derive(Debug, Clone)]
pub struct Drone {
    /// `kind:` value (`pipeline`/`secret`/`signature`/`template`).
    pub kind: String,
    /// `type:` value (`docker`/`kubernetes`/`ssh`/`exec`).
    pub pipe_type: String,
    /// `name:` value of the pipeline.
    pub name: String,
    /// `- ` items under `steps:` (pipeline step names).
    pub steps: usize,
    /// `image:` keys inside steps/services.
    pub images: usize,
    /// `- ` items under `commands:` blocks.
    pub commands: usize,
    /// `- ` items under `volumes:` blocks.
    pub volumes: usize,
    /// `- ` items under `services:`.
    pub services: usize,
    /// `- ` items under `depends_on:`.
    pub depends_on: usize,
    /// Entries under `environment:` blocks.
    pub env_entries: usize,
    /// `trigger:`/`when:`/`node:` blocks.
    pub triggers: usize,
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

/// Returns `true` when `b` looks like a `.drone.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    if t.contains("kind: pipeline") || t.contains("\"kind\": \"pipeline\"") {
        return true;
    }
    let has = |k: &str| t.lines().any(|l| is_key(l.trim_start(), k));
    has("kind") && has("steps")
}

impl Drone {
    /// Parses `b` as a `.drone.yml`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let images = t
            .lines()
            .filter(|l| is_key(l.trim_start(), "image"))
            .count();
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        Some(Self {
            kind: val_after(t, "kind").unwrap_or("").to_string(),
            pipe_type: val_after(t, "type").unwrap_or("").to_string(),
            name: val_after(t, "name").unwrap_or("").to_string(),
            steps: dash_items(t, "steps"),
            images,
            commands: dash_items(t, "commands"),
            volumes: dash_items(t, "volumes"),
            services: dash_items(t, "services"),
            depends_on: dash_items(t, "depends_on"),
            env_entries: child_items(t, "environment"),
            triggers: blocks(t, "trigger").len() + blocks(t, "when").len(),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# ci
kind: pipeline
type: docker
name: build
steps:
  - name: test
    image: golang
    commands:
      - go build
      - go test
    environment:
      CGO_ENABLED: '0'
  - name: publish
    image: plugins/docker
services:
  - name: pg
    image: postgres:15
trigger:
  branch: main
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"kind: secret"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let d = Drone::parse(SRC).unwrap();
        assert_eq!(d.kind, "pipeline");
        assert_eq!(d.pipe_type, "docker");
        assert_eq!(d.name, "build");
        assert_eq!(d.steps, 2);
        assert_eq!(d.images, 3);
        assert_eq!(d.commands, 2);
        assert_eq!(d.services, 1);
        assert_eq!(d.env_entries, 1);
        assert_eq!(d.triggers, 1);
        assert_eq!(d.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Drone::parse(b"\x01\x02").is_none());
    }
}
