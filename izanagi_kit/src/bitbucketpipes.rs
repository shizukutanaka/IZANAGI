//! Bitbucket Pipelines `bitbucket-pipelines.yml` census.
//!
//! `pipelines:` tree with `branches:`/`pull-requests:`/`tags:`/
//! `bookmarks:`/`custom:`/`default:` selectors containing
//! `- step:` blocks (`name:`, `script:`, `pipe:`, `oidc:`,
//! `deployment:`), plus `image:`/`definitions:`/`options:`/
//! `clone:`/`caches:`/`services:`.
//!
//! ```rust
//! let k = b"pipelines:\n  default:\n    - step:\n        name: Build\n        script:\n          - make build\n";
//! assert!(izanagi_kit::bitbucketpipes::detect(k));
//! ```

/// bitbucket-pipelines.yml census.
#[derive(Debug, Clone)]
pub struct BitbucketPipes {
    /// `- step` blocks.
    pub steps: usize,
    /// `script:` list commands.
    pub commands: usize,
    /// `pipe:` usages.
    pub pipes: usize,
    /// Selector keys (`branches`/`pull-requests`/`tags`/`custom`/`default`).
    pub selectors: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SELECTORS: &[&str] = &[
    "branches",
    "pull-requests",
    "tags",
    "bookmarks",
    "custom",
    "default",
];

const STEP_KEYS: &[&str] = &["script:", "pipe:", "step:"];

fn key_of(s: &str) -> &str {
    match s.find(':') {
        Some(i) => s[..i].trim(),
        None => s.trim(),
    }
}

/// Detect bitbucket-pipelines.yml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut has_pipelines = false;
    let mut has_step = false;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') && s == "pipelines:" {
            has_pipelines = true;
        }
        let stripped = s.strip_prefix("- ").unwrap_or(s);
        if STEP_KEYS.iter().any(|k| stripped.starts_with(k)) {
            has_step = true;
        }
    }
    has_pipelines && has_step
}

impl BitbucketPipes {
    /// Census a bitbucket-pipelines.yml buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            steps: 0,
            commands: 0,
            pipes: 0,
            selectors: 0,
            settings: 0,
            comments: 0,
        };
        let mut in_script = false;
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(inner0) = s.strip_prefix("- ") {
                let inner = inner0.trim();
                if inner.starts_with("step:") {
                    c.steps += 1;
                    in_script = false;
                    continue;
                }
                if inner.starts_with("pipe:") {
                    c.pipes += 1;
                    in_script = false;
                    continue;
                }
                if in_script {
                    c.commands += 1;
                    continue;
                }
            }
            if s.contains(':') {
                let k = key_of(s);
                if SELECTORS.contains(&k) {
                    c.selectors += 1;
                }
                in_script = k == "script";
                if s.starts_with("pipe:") || key_of(s) == "pipe" {
                    c.pipes += 1;
                }
                if !k.is_empty() {
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
    fn detects_pipeline() {
        let b = b"image: node:20\npipelines:\n  default:\n    - step:\n        name: Build\n        script:\n          - npm ci\n          - npm run build\n  branches:\n    main:\n      - step:\n          script:\n            - pipe: atlassian/slack-notify:1.0\n";
        assert!(detect(b));
        let c = BitbucketPipes::parse(b).unwrap();
        assert_eq!(c.steps, 2);
        assert_eq!(c.commands, 2);
        assert_eq!(c.pipes, 1);
        assert_eq!(c.selectors, 2);
    }

    #[test]
    fn rejects_other_yaml() {
        // GitHub Actions uses jobs:, not pipelines:.
        assert!(!detect(
            b"name: ci\njobs:\n  build:\n    steps:\n      - run: x\n"
        ));
        assert!(!detect(b"pipelines:\n  list:\n  - a\n"));
        // Comment-only mention.
        assert!(!detect(b"# pipelines:\n#   - step:\nkey: v\n"));
    }
}
