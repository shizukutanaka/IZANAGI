//! Parser for Concourse CI pipeline files (`pipeline.yml`).
//!
//! Counts `jobs:`/`resources:`/`resource_types:`/`groups:` entries and
//! `- get:`/`- put:`/`- task:`/`- try:`/`- in_parallel:`/`- aggregate:`/
//! `- do:`/`- set_pipeline:`/`params:`/`source:` step usages.
//!
//! ```
//! let b = b"jobs:\n  - name: build\n    plan:\n      - get: src\n      - task: build\n        config:\n          platform: linux\nresources:\n  - name: src\n    type: git\n";
//! assert!(izanagi_kit::concourse::detect(b));
//! let c = izanagi_kit::concourse::Concourse::parse(b).unwrap();
//! assert_eq!(c.jobs, 1);
//! assert_eq!(c.get_steps, 1);
//! assert_eq!(c.task_steps, 1);
//! ```

/// Parsed Concourse pipeline summary.
#[derive(Debug, Clone)]
pub struct Concourse {
    /// `- ` entries under `jobs:`.
    pub jobs: usize,
    /// `- ` entries under `resources:`.
    pub resources: usize,
    /// `- ` entries under `resource_types:`.
    pub resource_types: usize,
    /// `- ` entries under `groups:`.
    pub groups: usize,
    /// `- get:` step usages.
    pub get_steps: usize,
    /// `- put:` step usages.
    pub put_steps: usize,
    /// `- task:` step usages.
    pub task_steps: usize,
    /// `- try:` usages.
    pub try_steps: usize,
    /// `- in_parallel:`/`in_parallel:` usages.
    pub in_parallel: usize,
    /// `- aggregate:`/`aggregate:` usages (pre-`in_parallel`).
    pub aggregate: usize,
    /// `- do:`/`do:` usages.
    pub do_steps: usize,
    /// `- set_pipeline:`/`set_pipeline:` usages.
    pub set_pipeline: usize,
    /// `params:` keys.
    pub params: usize,
    /// `source:` keys.
    pub sources: usize,
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

fn step(t: &str, name: &str) -> usize {
    let dash = ["- ", name].concat();
    let bare = [name, ":"].concat();
    t.lines()
        .filter(|l| {
            let tr = l.trim_start();
            tr.starts_with(&*dash) || tr == bare.as_str()
        })
        .count()
}

/// Returns `true` when `b` looks like a Concourse pipeline.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let has = |k: &str| t.lines().any(|l| is_key(l.trim_start(), k));
    (has("jobs") || has("resources")) && (has("resource_types") || has("- get") || has("- task"))
}

impl Concourse {
    /// Parses `b` as a Concourse pipeline.
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
            jobs: dash_items(t, "jobs"),
            resources: dash_items(t, "resources"),
            resource_types: dash_items(t, "resource_types"),
            groups: dash_items(t, "groups"),
            get_steps: step(t, "get"),
            put_steps: step(t, "put"),
            task_steps: step(t, "task"),
            try_steps: step(t, "try"),
            in_parallel: step(t, "in_parallel"),
            aggregate: step(t, "aggregate"),
            do_steps: step(t, "do"),
            set_pipeline: step(t, "set_pipeline"),
            params: blocks(t, "params").len(),
            sources: blocks(t, "source").len(),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"jobs:\n  - name: j\n# - get: r\n"));
    }

    const SRC: &[u8] = b"# pipe
resource_types:
  - name: pr
    type: registry-image
resources:
  - name: src
    type: git
    source:
      uri: https://x
  - name: out
    type: s3
jobs:
  - name: build
    plan:
      - get: src
      - task: build
        config:
          platform: linux
      - put: out
  - name: gate
    plan:
      - in_parallel:
          - get: src
      - try:
          task: nested
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"jobs: []"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let c = Concourse::parse(SRC).unwrap();
        assert_eq!(c.jobs, 2);
        assert_eq!(c.resources, 2);
        assert_eq!(c.resource_types, 1);
        assert_eq!(c.get_steps, 2);
        assert_eq!(c.put_steps, 1);
        assert_eq!(c.task_steps, 1);
        assert_eq!(c.in_parallel, 1);
        assert_eq!(c.try_steps, 1);
        assert_eq!(c.params, 0);
        assert_eq!(c.sources, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Concourse::parse(b"\x01\x02").is_none());
    }
}
