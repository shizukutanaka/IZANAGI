//! Prefect `prefect.yaml` / deployment YAML detection and census.
//!
//! Detects `prefect-version:`/`deployments:`/`entrypoint:`/`work_pool:`/
//! `flow_name:` markers and counts top-level sections (`name`/
//! `prefect-version`/`build`/`push`/`pull`/`deployments`/`definitions`/
//! `control`), deployment entries (`- name:`), deployment keys
//! (`entrypoint`/`flow_name`/`flow_id`/`version`/`tags`/`description`/
//! `parameters`/`enforce_parameter_schema`/`work_pool`/`work_queue_name`/
//! `job_variables`/`schedules`/`schedule`/`triggers`/`enforce_parameter_schema`/
//! `paused`), schedule keys (`cron`/`interval`/`rrule`/`timezone`/`anchor_date`/
//! `active`/`day_or`/`minute_offset`), `{{` templating references, and `#`
//! comment lines.
//!
//! ```
//! let b = b"name: my-project\nprefect-version: 2.x\nbuild: null\npull: null\ndeployments:\n  - name: etl-deploy\n    entrypoint: flows/etl.py:main\n    work_pool:\n      name: default\n    schedules:\n      - cron: \"0 6 * * *\"\n        timezone: UTC\n";
//! assert!(izanagi_kit::prefect::detect(b));
//! let c = izanagi_kit::prefect::Prefect::parse(b).unwrap();
//! assert_eq!(c.deploys, 1);
//! assert_eq!(c.sections, 5);
//! ```

/// Parsed Prefect deployment file summary.
#[derive(Debug, Clone)]
pub struct Prefect {
    /// top-level sections (`name`/`build`/`pull`/`deployments`/`definitions`/…).
    pub sections: usize,
    /// `- name:` deployment entries.
    pub deploys: usize,
    /// deployment keys (`entrypoint`/`work_pool`/`parameters`/…).
    pub deployment_keys: usize,
    /// schedule keys (`cron`/`interval`/`rrule`/`timezone`/…).
    pub schedule_keys: usize,
    /// `{{` Jinja/variable references.
    pub refs: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "name:",
    "prefect-version:",
    "build:",
    "push:",
    "pull:",
    "deployments:",
    "definitions:",
    "control:",
    "dockerfile:",
    "image:",
    "steps:",
    "concurrency_limit:",
];

const DEPLOY_KEYS: &[&str] = &[
    "entrypoint:",
    "flow_name:",
    "flow_id:",
    "version:",
    "tags:",
    "description:",
    "parameters:",
    "enforce_parameter_schema:",
    "work_pool:",
    "work_queue_name:",
    "job_variables:",
    "schedules:",
    "schedule:",
    "triggers:",
    "paused:",
    "path:",
    "infra_overrides:",
    "infrastructure:",
    "storage:",
    "timestamp:",
];

const SCHEDULE_KEYS: &[&str] = &[
    "cron:",
    "interval:",
    "rrule:",
    "timezone:",
    "anchor_date:",
    "active:",
    "day_or:",
    "minute_offset:",
    "slug:",
];

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim_start(), key))
}

/// Detects Prefect deployment YAML files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    (has_key(&t, "deployments")
        && (has_key(&t, "entrypoint")
            || has_key(&t, "flow_name")
            || has_key(&t, "work_pool")
            || has_key(&t, "schedules")))
        || has_key(&t, "prefect-version")
}

impl Prefect {
    /// Parses a Prefect YAML file, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            sections: 0,
            deploys: 0,
            deployment_keys: 0,
            schedule_keys: 0,
            refs: t.matches("{{").count(),
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let key = tr.trim_start_matches("- ").trim_start();
            if indent(l) == 0 && SECTIONS.iter().any(|k| tr.starts_with(k)) {
                c.sections += 1;
            }
            if tr.starts_with("- name:") {
                c.deploys += 1;
            }
            if DEPLOY_KEYS.iter().any(|k| key.starts_with(k)) {
                c.deployment_keys += 1;
            }
            if SCHEDULE_KEYS.iter().any(|k| key.starts_with(k)) {
                c.schedule_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# deployments:\n# entrypoint: a.py:flow\n"));
        assert!(!detect(b"# prefect-version: 3.0\n"));
    }

    #[test]
    fn detects_and_counts() {
        let b = b"name: my-project\nprefect-version: 2.x\nbuild: null\npull: null\ndeployments:\n  - name: etl-deploy\n    entrypoint: flows/etl.py:main\n    work_pool:\n      name: default\n    schedules:\n      - cron: \"0 6 * * *\"\n        timezone: UTC\n  - name: nightly\n    entrypoint: flows/n.py:run\n    parameters: {}\n";
        let c = Prefect::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.deploys, 2);
        assert!(c.deployment_keys >= 3);
        assert!(c.schedule_keys >= 2);
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(Prefect::parse(b"key: value\n").is_none());
        assert!(!detect(b"version: 1\njobs:\n  x: y\n"));
    }
}
