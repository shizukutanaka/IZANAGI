//! BIND `named.conf` configuration format.
//!
//! named.conf uses `statement { ... };` blocks (`options`, `zone`,
//! `logging`, `acl`, `view`, `key`, `controls`, `server`, `include`,
//! `trusted-keys`, `managed-keys`, `masters`, `listen-on`, …) with
//! `key value;` settings and `//`/`#`/`/*` comments.
//!
//! ```
//! let b = concat!(
//!     "options {\n",
//!     "  directory \"/var/named\";\n",
//!     "  recursion yes;\n",
//!     "};\n",
//!     "zone \"example.com\" {\n",
//!     "  type master;\n",
//!     "  file \"db.example.com\";\n",
//!     "};\n"
//! ).as_bytes();
//! assert!(izanagi_kit::namedconf::detect(b));
//! let c = izanagi_kit::namedconf::Namedconf::parse(b).unwrap();
//! assert_eq!(c.blocks, 2);
//! assert_eq!(c.zones, 1);
//! ```

/// Parsed named.conf summary.
#[derive(Debug, Clone)]
pub struct Namedconf {
    /// `name {` top-level block declarations.
    pub blocks: usize,
    /// `zone "..."` declarations.
    pub zones: usize,
    /// `type master|slave|secondary|hint|forward|stub|redirect|delegation-only|in-view|mirror` inside zone blocks.
    pub zone_types: usize,
    /// `include "..."` statements.
    pub includes: usize,
    /// `acl`, `key`, `view`, `controls`, `server`, `trusted-keys`, `managed-keys`, `masters`, `logging`, `statistics-channels` block kind census.
    pub kinds: usize,
    /// Comment lines (`//`, `#`, `/*`).
    pub comments: usize,
    /// `allow-*`/`listen-on`/`forwarders`/`also-notify`/`recursion`/`dnssec-*`/`querylog`/`pid-file`/`directory`/`file`/`masters` settings.
    pub settings: usize,
}

const BLOCK_KW: &[&str] = &[
    "options",
    "logging",
    "acl",
    "view",
    "key",
    "controls",
    "server",
    "trusted-keys",
    "managed-keys",
    "masters",
    "statistics-channels",
    "dnssec-policy",
    "http",
    "tls",
    "dlz",
    "dyndb",
    "parental-agents",
];
const SETTING_PREFIX: &[&str] = &[
    "allow-",
    "listen-on",
    "forwarders",
    "forward",
    "also-notify",
    "notify",
    "recursion",
    "dnssec-",
    "querylog",
    "pid-file",
    "directory",
    "file",
    "masters",
    "max-",
    "min-",
    "rate-limit",
    "check-",
    "transfer-",
    "serial-",
    "zone-statistics",
    "dnskey-",
    "matching-",
    "answer-",
    "additional-",
    "qname-",
    "cookie-",
    "nocookie-",
    "stale-",
    "prefetch",
    "port",
    "auth-",
    "keys",
    "session-key",
    "hostname",
    "server-id",
    "version",
    "hostname",
    "tkey-",
    "management",
    "dlopen",
    "sortlist",
    "topology",
    "rrset-order",
    "dual-stack",
    "empty-",
    "disable-",
    "deny-answer",
    "querylog",
    "resolver-",
    "ipv4only-",
    "must-be-secure",
    "dns64",
    "nta-",
    "root-delegation-only",
    "servfail-ttl",
    "response-policy",
    "catalog-",
    "send-cookie",
    "response-",
    "tcp-",
    "edns-",
    "no-case-compress",
    "message-compression",
    "minimal-any",
    "minimal-responses",
    "glue-",
    "primaries",
];
const ZONE_TYPES: &[&str] = &[
    "master",
    "slave",
    "secondary",
    "hint",
    "forward",
    "stub",
    "redirect",
    "delegation-only",
    "in-view",
    "mirror",
    "static-stub",
    "primary",
];

/// Whether the buffer looks like named.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut score = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("//") || tr.starts_with('#') || tr.starts_with("/*") {
            continue;
        }
        for kw in BLOCK_KW {
            if tr.starts_with(&format!("{kw} ")) || tr.starts_with(&format!("{kw}{{")) {
                score += 2;
            }
        }
        if tr.starts_with("zone") || tr.starts_with("include") || tr.starts_with("type") {
            score += 1;
        }
        if tr.starts_with("options") || tr.starts_with("controls") {
            score += 1;
        }
    }
    score >= 2
}

impl Namedconf {
    /// Parses a named.conf summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            blocks: 0,
            zones: 0,
            zone_types: 0,
            includes: 0,
            kinds: 0,
            comments: 0,
            settings: 0,
        };
        let mut in_zone = false;
        let mut depth = 0usize;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("//") || tr.starts_with('#') || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            let opens = tr.matches('{').count();
            let closes = tr.matches('}').count();
            if in_zone && tr.starts_with("type") {
                for zt in ZONE_TYPES {
                    if tr.contains(&format!("type {zt}")) {
                        c.zone_types += 1;
                        break;
                    }
                }
            }
            if tr.starts_with("zone ") || tr.starts_with("zone\"") || tr.starts_with("zone \"") {
                c.zones += 1;
                in_zone = true;
            }
            if tr.starts_with("include") {
                c.includes += 1;
            }
            for kw in BLOCK_KW {
                if tr.starts_with(&format!("{kw} ")) || tr.starts_with(&format!("{kw}{{")) {
                    c.blocks += 1;
                    c.kinds += 1;
                }
            }
            if tr.starts_with("zone") {
                c.blocks += 1;
            }
            for kw in SETTING_PREFIX {
                if tr.starts_with(kw) && !tr.starts_with("{") {
                    c.settings += 1;
                    break;
                }
            }
            depth += opens;
            depth = depth.saturating_sub(closes);
            if in_zone && depth == 0 {
                in_zone = false;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_conf() {
        let b = concat!(
            "options {\n",
            "  directory \"/var/named\";\n",
            "  recursion yes;\n",
            "};\n",
            "zone \"example.com\" {\n",
            "  type master;\n",
            "  file \"db.example.com\";\n",
            "};\n",
            "zone \"0.168.192.in-addr.arpa\" {\n",
            "  type slave;\n",
            "  masters { 192 0 2 1; };\n",
            "};\n",
            "include \"/etc/named.rfc1912.zones\";\n",
            "// trailing\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Namedconf::parse(b).unwrap();
        assert_eq!(c.zones, 2);
        assert_eq!(c.zone_types, 2);
        assert_eq!(c.includes, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"x = 1;\ny = 2;\n"));
        assert!(Namedconf::parse(b"x").is_none());
    }
}
