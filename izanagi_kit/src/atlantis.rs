//! Atlantis `atlantis.yaml` server-side repo config census.
//!
//! `version:` (2 or 3) plus project/workflow definitions:
//! `projects:` (`dir`, `workspace`, `workflow`, `autoplan`,
//! `terraform_version`, `apply_requirements`),
//! `workflows:` (`plan`/`apply`/`import`/`state_rm` stages),
//! `policies:`, `repos:`, and the exclusive keys `automerge`,
//! `parallel_plan`, `parallel_apply`,
//! `delete_source_branch_on_merge`, `repo_locks`,
//! `custom_policy_check`, `allowed_regexp_prefixes`,
//! `silence_pr_comments`, `abort_on_execution_order_fail`.
//!
//! ```rust
//! let k = b"version: 3\nprojects:\n- dir: .\n  workflow: default\nworkflows:\n  default:\n    plan:\n      steps: [init, plan]\n";
//! assert!(izanagi_kit::atlantis::detect(k));
//! ```

/// atlantis.yaml census.
#[derive(Debug, Clone)]
pub struct Atlantis {
    /// `projects:` entries.
    pub projects: usize,
    /// `workflows:` named entries.
    pub workflows: usize,
    /// Recognised top-level keys present.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const ATLANTIS_KEYS: &[&str] = &[
    "automerge",
    "parallel_plan",
    "parallel_apply",
    "delete_source_branch_on_merge",
    "repo_locks",
    "custom_policy_check",
    "allowed_regexp_prefixes",
    "silence_pr_comments",
    "abort_on_execution_order_fail",
    "plan_requirements",
    "apply_requirements",
    "import_requirements",
    "gitlab_hostname",
    "pre_workflow_hooks",
    "post_workflow_hooks",
];

const SECTION_KEYS: &[&str] = &["projects", "workflows", "policies", "repos", "version"];

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

/// Detect atlantis.yaml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut has_version = false;
    let mut sections = 0usize;
    let mut exclusive = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            match k {
                "version" => has_version = true,
                "projects" | "workflows" | "policies" | "repos" => sections += 1,
                _ if ATLANTIS_KEYS.contains(&k) => exclusive += 1,
                _ => {}
            }
        }
    }
    has_version && sections >= 1 && (exclusive >= 1 || sections >= 2)
}

impl Atlantis {
    /// Census an atlantis.yaml buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            projects: 0,
            workflows: 0,
            sections: 0,
            settings: 0,
            comments: 0,
        };
        let mut section = "";
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = top_key(line) {
                section = k;
                if SECTION_KEYS.contains(&k) || ATLANTIS_KEYS.contains(&k) {
                    c.sections += 1;
                }
                c.settings += 1;
                continue;
            }
            if s.starts_with("- ") {
                if section == "projects" {
                    c.projects += 1;
                }
                continue;
            }
            if s.contains(':') {
                if section == "workflows" && line.starts_with("  ") && !line.starts_with("   ") {
                    c.workflows += 1;
                }
                c.settings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_repo_config() {
        let b = b"version: 3\nautomerge: true\nprojects:\n- dir: app1\n  workflow: default\n- dir: app2\nworkflows:\n  default:\n    plan:\n      steps: [init, plan]\n";
        assert!(detect(b));
        let c = Atlantis::parse(b).unwrap();
        assert_eq!(c.projects, 2);
        assert_eq!(c.workflows, 1);
    }

    #[test]
    fn detects_minimal() {
        let b = b"version: 3\nprojects:\n- dir: .\nworkflows:\n  default:\n    plan: {}\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other_yaml() {
        // No version key.
        assert!(!detect(b"projects:\n- dir: .\n"));
        // version alone with unrelated keys is not atlantis.
        assert!(!detect(b"version: '3'\nservices:\n  web: {}\n"));
        assert!(!detect(b"version: 3\nkind: Deployment\n"));
    }
}
