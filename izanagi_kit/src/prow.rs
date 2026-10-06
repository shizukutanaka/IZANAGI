//! Prow CI `config.yaml` census.
//!
//! Prow config top-level keys: `presubmits`, `postsubmits`,
//! `periodics`, `tide`, `plank`, `deck`, `sinker`, `horologium`,
//! `gangway`, `gerrit`, `github`, `jenkins_operators`,
//! `in_repo_config`, `prowjob_namespace`, `pod_namespace`,
//! `branch-protection`, `job_url_prefix`, `slackevents`,
//! `owners_dir_blacklist`, `owners_dir_denylist`,
//! `prowjob_default_entries`, `log_level`, `push_gateway`.
//!
//! ```rust
//! let k = b"presubmits:\n  org/repo:\n  - name: test\n    spec:\n      x: 1\nperiodics:\n- name: nightly\n  cron: '@daily'\ntide:\n  merge_method:\n    org/repo: squash\n";
//! assert!(izanagi_kit::prow::detect(k));
//! ```

/// Prow config census.
#[derive(Debug, Clone)]
pub struct Prow {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "presubmits",
    "postsubmits",
    "periodics",
    "tide",
    "plank",
    "deck",
    "sinker",
    "horologium",
    "gangway",
    "jenkins_operators",
    "in_repo_config",
    "prowjob_namespace",
    "prowjob_default_entries",
    "slackevents",
    "owners_dir_blacklist",
    "owners_dir_denylist",
];

const WEAK: &[&str] = &[
    "gerrit",
    "github",
    "pod_namespace",
    "branch-protection",
    "job_url_prefix",
    "log_level",
    "push_gateway",
    "managed_jobs_info",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect a Prow `config.yaml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // Require two prow-exclusive top-level keys; a lone `periodics:` or
    // `deck:` block is too easy to bump into in other CI YAML.
    let mut strong = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            }
        }
    }
    strong >= 2
}

impl Prow {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
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
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
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
        let b = b"presubmits:\n  org/repo:\n  - name: test\n    spec:\n      x: 1\nperiodics:\n- name: nightly\n  cron: '@daily'\ntide:\n  merge_method:\n    org/repo: squash\n";
        assert!(detect(b));
        let c = Prow::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"periodics:\n- name: x\n"));
        assert!(!detect(b"github:\n  token: x\n"));
        assert!(!detect(b"# presubmits:\n# periodics:\n"));
    }
}
