//! Census of a Cloud Custodian `policies.yaml` file.
//!
//! Cloud Custodian: `policies:` root list of `- name:`/`resource:` policy
//! entries with optional `mode:` (`type: periodic`/`cloudtrail`/`pull`),
//! `filters:` blocks (`- type:`/`- key:`/`- value:`/`tag:`/`mark`/`age`/
//! `network-location`), `actions:` blocks (`- type:`/`- notify`/`stop`/
//! `encrypt`/`post-finding`), `region`/`conditions`/`source`/`comments`,
//! `description:` strings. `custodian ` runs them with `policy.yaml`.
//!
//! ```rust
//! let c = izanagi_kit::cloudcustodian::Cloudcustodian::parse(
//!     b"policies:\n  - name: stop-ec2\n    resource: ec2\n    filters: [{\"State.Name\": running}]\n",
//! ).unwrap();
//! assert_eq!(c.policies, 1);
//! ```
#![forbid(unsafe_code)]

/// Cloud Custodian policy census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cloudcustodian {
    /// `- name:` policy entries.
    pub policies: usize,
    /// `filters:`/`actions:` blocks.
    pub blocks: usize,
    /// `- type:`/`mode:` entries inside blocks.
    pub entries: usize,
    /// `resource:`/`region`/`mode` keys seen.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// True if `b` looks like a Custodian policy file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("policies:") && (t.contains("resource:") || t.contains("- name:"))
}

impl Cloudcustodian {
    /// Parse a `policies.yaml` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            policies: 0,
            blocks: 0,
            entries: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
            } else if l.starts_with("- name:") {
                c.policies += 1;
            } else if l.starts_with("filters:")
                || l.starts_with("actions:")
                || l.starts_with("mode:")
            {
                c.blocks += 1;
            } else if l.starts_with("- type:")
                || l.starts_with("- key:")
                || l.starts_with("type: periodic")
                || l.starts_with("- notify")
                || l.starts_with("type:")
            {
                c.entries += 1;
            }
            if l.starts_with("resource:") || l.starts_with("region:") || l.starts_with("region ") {
                c.keys += 1;
            }
        }
        if c.policies == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# custodian\n",
            "policies:\n",
            "  - name: stop-ec2\n",
            "    resource: ec2\n",
            "    region: us-east-1\n",
            "    filters:\n",
            "      - type: value\n",
            "        key: State.Name\n",
            "        value: running\n",
            "    actions:\n",
            "      - type: stop\n",
            "  - name: s3-encrypt\n",
            "    resource: s3\n",
            "    mode:\n",
            "      type: periodic\n",
            "    actions:\n",
            "      - notify\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Cloudcustodian::parse(b.as_bytes()).unwrap();
        assert_eq!(c.policies, 2);
        assert!(c.blocks >= 3);
        assert!(c.entries >= 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"a: b\n"));
        assert!(Cloudcustodian::parse(b"# none\n").is_none());
    }
}
