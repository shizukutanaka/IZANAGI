//! Jenkins X manifest census (`jenkins.io`/`jenkins-x.io` API groups).
//!
//! `apiVersion` in the `jenkins.io, jenkins-x.io` API group(s) together with a
//! `kind` in `Environment, PipelineActivity, Release, SourceRepository, Scheduler, Fact, CommitStatus, PipelineStructure, Extension, App, Plugin, Team, User, EnvironmentRoleBinding, GitService`.
//!
//! ```rust
//! let k = b"apiVersion: jenkins.io/v1\nkind: Environment\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
//! assert!(izanagi_kit::jenkinsx::detect(k));
//! ```

use crate::textutil::{kind_val, value_of};
/// Jenkins X manifest census.
#[derive(Debug, Clone)]
pub struct Jenkinsx {
    /// `kind` value, when present.
    pub kind: Option<String>,
    /// `apiVersion`/`kind`/`metadata`/`spec` envelope lines.
    pub envelope: usize,
    /// `- ` list items.
    pub items: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const GROUPS: &[&str] = &["jenkins.io", "jenkins-x.io"];

const KINDS: &[&str] = &[
    "Environment",
    "PipelineActivity",
    "Release",
    "SourceRepository",
    "Scheduler",
    "Fact",
    "CommitStatus",
    "PipelineStructure",
    "Extension",
    "App",
    "Plugin",
    "Team",
    "User",
    "EnvironmentRoleBinding",
    "GitService",
];

fn api_ok(t: &str) -> bool {
    t.lines().any(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("apiVersion") {
            return false;
        }
        let v = value_of(s);
        GROUPS.iter().any(|g| v.starts_with(g))
    })
}

/// Detect a Jenkins X manifest.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    if !api_ok(t) {
        return false;
    }
    match kind_val(t) {
        Some(k) => KINDS.contains(&k.as_str()),
        None => false,
    }
}

impl Jenkinsx {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kind: kind_val(t),
            envelope: 0,
            items: 0,
            settings: 0,
            comments: 0,
        };
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
                continue;
            }
            if s.contains(':') {
                let k = s[..s.find(':').unwrap_or(s.len())].trim();
                if matches!(k, "apiVersion" | "kind" | "metadata" | "spec") {
                    c.envelope += 1;
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
    fn detects() {
        let b = b"apiVersion: jenkins.io/v1\nkind: Environment\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
        assert!(detect(b));
        let c = Jenkinsx::parse(b).unwrap();
        assert_eq!(c.kind.as_deref(), Some("Environment"));
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"apiVersion: v1\nkind: Environment\n"));
        assert!(!detect(b"apiVersion: jenkins.io/v1\nkind: Deployment\n"));
        assert!(!detect(b"# apiVersion: jenkins.io/v1\nkind: Environment\n"));
    }
}
