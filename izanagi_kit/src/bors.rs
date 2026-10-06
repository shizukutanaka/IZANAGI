//! Bors `bors.toml` merge-bot config census.
//!
//! bors.toml top-level assignments: `status`, `block_labels`,
//! `required_approvals`, `pr_status`, `timeout_sec`, `priority`,
//! `delete_merged_branches`, `cut_body_after`, `commit_title`,
//! `use_squash_merge`, `use_merge_commit`, `update_base_for_deletes`.
//!
//! ```rust
//! let k = b"status = [\"ci\"]\nblock_labels = [\"do-not-merge\"]\nrequired_approvals = 1\n";
//! assert!(izanagi_kit::bors::detect(k));
//! ```

/// Bors config census.
#[derive(Debug, Clone)]
pub struct Bors {
    /// `key =` assignment lines.
    pub assignments: usize,
    /// recognised bors keys present.
    pub keys: usize,
    /// `[section]` lines.
    pub sections: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "status",
    "block_labels",
    "required_approvals",
    "pr_status",
    "timeout_sec",
    "priority",
    "delete_merged_branches",
    "cut_body_after",
    "commit_title",
    "use_squash_merge",
    "use_merge_commit",
    "update_base_for_deletes",
];

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with('[') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"');
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a bors.toml config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `status` alone is too common in TOML configs; require two bors keys
    // or the characteristic `status`+`block_labels`/`pr_status` pairing.
    let mut hits = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if KEYS.contains(&k) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

impl Bors {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            assignments: 0,
            keys: 0,
            sections: 0,
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
            if s.starts_with('[') {
                c.sections += 1;
                continue;
            }
            if let Some(k) = assign_key(line) {
                c.assignments += 1;
                if KEYS.contains(&k) {
                    c.keys += 1;
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
        let b = b"status = [\"ci\"]\nblock_labels = [\"do-not-merge\"]\nrequired_approvals = 1\n";
        assert!(detect(b));
        let c = Bors::parse(b).unwrap();
        assert_eq!(c.keys, 3);
        assert_eq!(c.assignments, 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"status = \"ok\"\n"));
        assert!(!detect(b"# status = [\"ci\"]\n# block_labels = [\"x\"]\n"));
        assert!(!detect(b"name = \"pkg\"\nversion = \"1.0\"\n"));
    }
}
