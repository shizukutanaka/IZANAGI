//! Parser for Bitrise files (`bitrise.yml`).
//!
//! Extracts `format_version`/`default_step_lib_source`, counts
//! `workflows:`/`apps:`/`envs:`/`trigger_map:`/`stages:`/`pipelines:`
//! entries, `- step@ver` step references, and comments.
//!
//! ```
//! let b = b"format_version: \"13\"\ndefault_step_lib_source: https://x\nworkflows:\n  ci:\n    steps:\n      - script@1:\n          title: hi\n";
//! assert!(izanagi_kit::bitrise::detect(b));
//! let r = izanagi_kit::bitrise::Bitrise::parse(b).unwrap();
//! assert_eq!(r.format_version, "13");
//! assert_eq!(r.workflows, 1);
//! assert_eq!(r.steps, 1);
//! ```

/// Parsed `bitrise.yml` summary.
#[derive(Debug, Clone)]
pub struct Bitrise {
    /// `format_version:` value.
    pub format_version: String,
    /// `default_step_lib_source:` value.
    pub step_lib: String,
    /// Entries under `workflows:`.
    pub workflows: usize,
    /// Entries under `apps:`.
    pub apps: usize,
    /// Entries under `envs:`/`app.envs:` blocks.
    pub envs: usize,
    /// Entries under `trigger_map:`.
    pub trigger_map: usize,
    /// Entries under `stages:`.
    pub stages: usize,
    /// Entries under `pipelines:`.
    pub pipelines: usize,
    /// `- name@ver:` step references.
    pub steps: usize,
    /// `meta:` section present.
    pub meta: bool,
    /// `#` comment lines.
    pub comments: usize,
}

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key).is_some_and(|r| {
        // `key :`/`"key" :`(コロン前の空白と閉じ引用符)も YAML では合法。
        r.trim_start()
            .trim_start_matches(['"', '\''])
            .trim_start()
            .starts_with(':')
    })
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
                let v = v.trim().trim_matches(['"', '\'']).trim_end_matches(',');
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Returns `true` when `b` looks like a `bitrise.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    has_key(t, "format_version")
        || (has_key(t, "workflows") && has_key(t, "steps") && t.contains('@'))
}

impl Bitrise {
    /// Parses `b` as a `bitrise.yml`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let steps = t
            .lines()
            .filter(|l| {
                let tr = l.trim_start();
                tr.starts_with("- ") && tr.contains('@')
            })
            .count();
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        Some(Self {
            format_version: val_after(t, "format_version").unwrap_or("").to_string(),
            step_lib: val_after(t, "default_step_lib_source")
                .unwrap_or("")
                .to_string(),
            workflows: child_items(t, "workflows"),
            apps: child_items(t, "app"),
            envs: child_items(t, "envs"),
            trigger_map: child_items(t, "trigger_map"),
            stages: child_items(t, "stages"),
            pipelines: child_items(t, "pipelines"),
            steps,
            meta: has_key(t, "meta"),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_space_before_colon() {
        // YAML では `key :` も合法。
        let v = String::from_utf8_lossy(SRC).replace("format_version:", "format_version :");
        assert!(detect(v.as_bytes()));
    }

    const SRC: &[u8] = b"# mobile
format_version: \"13\"
default_step_lib_source: https://github.com/bitrise-io/bitrise-steplib
trigger_map:
  - push_branch: main
    workflow: ci
app:
  envs:
    - APP_NAME: demo
workflows:
  ci:
    steps:
      - script@1:
          title: hi
      - git-clone@8:
  deploy:
    steps:
      - deploy-to-bitrise-io@2:
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"workflows: {}"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let r = Bitrise::parse(SRC).unwrap();
        assert_eq!(r.format_version, "13");
        assert_eq!(r.workflows, 2);
        assert_eq!(r.apps, 1);
        assert_eq!(r.envs, 1);
        assert_eq!(r.trigger_map, 1);
        assert_eq!(r.steps, 3);
        assert_eq!(r.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Bitrise::parse(b"\x01\x02").is_none());
    }
}
