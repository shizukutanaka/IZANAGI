//! Parser for Serverless Framework files (`serverless.yml`).
//!
//! Extracts `service`/`provider`/`runtime`, counts `functions:` entries,
//! `- ` event items, plugins, environment keys, and comments.
//!
//! ```
//! let b = b"service: api\nprovider:\n  name: aws\nfunctions:\n  hello:\n    handler: h.hi\n";
//! assert!(izanagi_kit::serverless::detect(b));
//! let s = izanagi_kit::serverless::Serverless::parse(b).unwrap();
//! assert_eq!(s.service, "api");
//! assert_eq!(s.provider_name, "aws");
//! assert_eq!(s.functions, 1);
//! ```

/// Parsed `serverless.yml` summary.
#[derive(Debug, Clone)]
pub struct Serverless {
    /// `service:` value.
    pub service: String,
    /// `frameworkVersion:` value (empty when absent).
    pub framework: String,
    /// `name:` inside `provider:`.
    pub provider_name: String,
    /// `runtime:` inside `provider:` (or top-level).
    pub runtime: String,
    /// Entries under `functions:`.
    pub functions: usize,
    /// `- ` items inside `events:` blocks.
    pub events: usize,
    /// Events using `http`/`httpApi`/`alb`.
    pub http_events: usize,
    /// `- ` items inside `plugins:`.
    pub plugins: usize,
    /// Entries inside `layers:`.
    pub layers: usize,
    /// Keys inside `environment:` blocks.
    pub env_keys: usize,
    /// `resources:` section present.
    pub resources_section: bool,
    /// `package:` section present.
    pub package: bool,
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

/// `- ` items directly inside blocks named `key`.
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

fn val_in_block<'a>(t: &'a str, block: &str, key: &str) -> Option<&'a str> {
    blocks(t, block).into_iter().next().and_then(|b| {
        b.iter().find_map(|l| {
            let tr = l.trim();
            tr.strip_prefix(key).and_then(|r| {
                r.trim_start_matches(['"', '\''])
                    .trim_start()
                    .strip_prefix(':')
                    .map(|v| {
                        v.trim()
                            .trim_matches(|c| c == '"' || c == '\'')
                            .trim_end_matches(',')
                    })
            })
        })
    })
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

/// Returns `true` when `b` looks like a `serverless.yml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    has_key(t, "service")
        && has_key(t, "provider")
        && (has_key(t, "functions") || has_key(t, "frameworkVersion") || has_key(t, "org"))
}

impl Serverless {
    /// Parses `b` as a Serverless Framework file.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).unwrap_or("");
        let events = blocks(t, "events");
        let http_events = events
            .iter()
            .flatten()
            .filter(|l| {
                let tr = l.trim_start();
                tr.starts_with("- http") || tr.starts_with("- alb")
            })
            .count();
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count();
        Some(Self {
            service: val_after(t, "service").unwrap_or("").to_string(),
            framework: val_after(t, "frameworkVersion").unwrap_or("").to_string(),
            provider_name: val_in_block(t, "provider", "name")
                .unwrap_or("")
                .to_string(),
            runtime: val_in_block(t, "provider", "runtime")
                .or_else(|| val_after(t, "runtime"))
                .unwrap_or("")
                .to_string(),
            functions: child_items(t, "functions"),
            events: dash_items(t, "events"),
            http_events,
            plugins: dash_items(t, "plugins"),
            layers: child_items(t, "layers"),
            env_keys: child_items(t, "environment"),
            resources_section: has_key(t, "resources"),
            package: has_key(t, "package"),
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# sls
service: api
frameworkVersion: \"3\"
provider:
  name: aws
  runtime: nodejs18.x
functions:
  hello:
    handler: h.hi
    events:
      - http:
          path: /
      - schedule: rate(1 hour)
  world:
    handler: w.hi
plugins:
  - serverless-offline
environment:
  TABLE: t
resources:
  Resources: {}
";

    #[test]
    fn detects() {
        assert!(detect(SRC));
        assert!(!detect(b"service: api"));
        assert!(!detect(b"\x00\x01"));
    }

    #[test]
    fn counts_kinds() {
        let s = Serverless::parse(SRC).unwrap();
        assert_eq!(s.service, "api");
        assert_eq!(s.framework, "3");
        assert_eq!(s.provider_name, "aws");
        assert_eq!(s.runtime, "nodejs18.x");
        assert_eq!(s.functions, 2);
        assert_eq!(s.events, 2);
        assert_eq!(s.http_events, 1);
        assert_eq!(s.plugins, 1);
        assert_eq!(s.env_keys, 1);
        assert!(s.resources_section);
        assert_eq!(s.comments, 1);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Serverless::parse(b"\x01\x02").is_none());
    }
}
