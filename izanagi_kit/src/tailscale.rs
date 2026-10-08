//! Tailscale ACL policy file (`acl.hujson`) census.
//!
//! HuJSON (JSON + comments/trailing commas) policy document
//! with top-level sections: `acls` (entries `{ "action":
//! "accept"|"deny", "src": [...], "dst": [...] }`),
//! `tagOwners`, `groups`, `hosts`, `tests`, `autoApprovers`,
//! `ssh`, `nodeAttrs`, `derpMap`, `dnsConfig`, `grants`,
//! `postures`, `sshTests`, `autoApprovers`, `ipsets`,
//! `subnetRouterPolicies`.
//!
//! ```rust
//! let a = "{\n\"acls\": [\n{\"action\": \"accept\", \"src\": [\"autogroup:member\"], \"dst\": [\"*:*\"]}\n],\n\"tagOwners\": {\"tag:prod\": [\"admin\"]}\n}\n";
//! let c = izanagi_kit::tailscale::Tailscale::parse(a.as_bytes()).unwrap();
//! assert_eq!(c.acls, 1);
//! ```

use crate::textutil::strip_bom;
/// tailscale acl census.
#[derive(Debug, Clone)]
pub struct Tailscale {
    /// `acls`/`grants` rule entries (objects with `action`/`src`).
    pub acls: usize,
    /// Recognised top-level policy sections.
    pub sections: usize,
    /// `key:` pairs.
    pub settings: usize,
    /// `action` values seen (accept/deny).
    pub actions: usize,
    /// `//` comment lines.
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "acls",
    "tagOwners",
    "groups",
    "hosts",
    "tests",
    "autoApprovers",
    "ssh",
    "nodeAttrs",
    "derpMap",
    "dnsConfig",
    "grants",
    "postures",
    "sshTests",
    "ipsets",
    "subnetRouterPolicies",
    "via",
    "routes",
    "exitNode",
    "appConnectors",
];

/// Detect tailscale acl content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut tops = 0usize;
    for line in t.lines() {
        let s = line.trim().trim_start_matches('{').trim_start();
        if s.is_empty() || s.starts_with("//") {
            continue;
        }
        if let Some(rest) = s.strip_prefix('"') {
            if let Some(end) = rest.find('"') {
                let key = &rest[..end];
                if TOP_KEYS.contains(&key) {
                    tops += 1;
                }
            }
        }
    }
    tops >= 1 && t.contains('{')
}

impl Tailscale {
    /// Census an acl.hujson buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            acls: 0,
            sections: 0,
            settings: 0,
            actions: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") {
                c.comments += 1;
                continue;
            }
            for seg in s.split('{') {
                let seg = seg.trim();
                let mut key_count = 0usize;
                let mut is_action = false;
                let mut is_src = false;
                let mut is_top = false;
                for piece in seg.split(',') {
                    let piece = piece.trim().trim_end_matches('}').trim();
                    if let Some(rest) = piece.strip_prefix('"') {
                        if let Some(end) = rest.find('"') {
                            let key = &rest[..end];
                            if !key.is_empty()
                                && piece.len() > end + 2
                                && piece[end + 2..].trim_start().starts_with(':')
                            {
                                key_count += 1;
                                if TOP_KEYS.contains(&key) {
                                    is_top = true;
                                }
                                if key == "action" {
                                    is_action = true;
                                    c.actions += 1;
                                }
                                if key == "src" {
                                    is_src = true;
                                }
                            }
                        }
                    }
                }
                c.settings += key_count;
                c.sections += usize::from(is_top);
                if is_action && is_src {
                    c.acls += 1;
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
    fn detects_acl() {
        let b = b"{\"acls\": []}";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"a\":1}"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_acl() {
        let b = concat!(
            "// tailscale acl\n",
            "{\n",
            "  \"acls\": [\n",
            "    { \"action\": \"accept\", \"src\": [\"autogroup:member\"], \"dst\": [\"*:*\"] },\n",
            "    { \"action\": \"accept\", \"src\": [\"tag:prod\"], \"dst\": [\"tag:prod:*\"] }\n",
            "  ],\n",
            "  \"tagOwners\": {\n",
            "    \"tag:prod\": [\"admin@example.com\"]\n",
            "  },\n",
            "  \"groups\": { \"group:admins\": [\"admin@example.com\"] },\n",
            "  \"hosts\": { \"server1\": \"100.64.0.1\" },\n",
            "  \"tests\": [\n",
            "    { \"src\": \"admin@example.com\", \"accept\": [\"server1:22\"], \"deny\": [\"server1:80\"] }\n",
            "  ],\n",
            "  \"ssh\": [\n",
            "    { \"action\": \"accept\", \"src\": [\"group:admins\"], \"dst\": [\"tag:prod\"], \"users\": [\"root\"] }\n",
            "  ]\n",
            "}\n",
        );
        let c = Tailscale::parse(b.as_bytes()).unwrap();
        assert_eq!(c.acls, 3);
        assert_eq!(c.sections, 6);
        assert!(c.settings >= 14);
        assert_eq!(c.actions, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
