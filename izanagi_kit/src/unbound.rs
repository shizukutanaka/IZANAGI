//! Unbound `unbound.conf` configuration format.
//!
//! unbound.conf uses YAML-ish `key: value` settings grouped under
//! section headers: `server:`, `remote-control:`, `forward-zone:`,
//! `stub-zone:`, `auth-zone:`, `view:`, `python:`, `dynlib:`, `cachedb:`,
//! `rpz:`, `dnscrypt:`, `edns-subnet:` (legacy), `ipsecmod:` (legacy).
//!
//! ```
//! let b = concat!(
//!     "server:\n",
//!     "  verbosity: 1\n",
//!     "  interface: 192 0 2 1\n",
//!     "  do-ip6: no\n",
//!     "remote-control:\n",
//!     "  control-enable: yes\n",
//!     "forward-zone:\n",
//!     "  name: \".\"\n",
//!     "  forward-addr: 192 0 2 2\n"
//! ).as_bytes();
//! assert!(izanagi_kit::unbound::detect(b));
//! let c = izanagi_kit::unbound::Unbound::parse(b).unwrap();
//! assert_eq!(c.sections, 3);
//! ```

/// Parsed unbound.conf summary.
#[derive(Debug, Clone)]
pub struct Unbound {
    /// Top-level section headers (`server:`, `remote-control:`, …).
    pub sections: usize,
    /// `key: value` setting lines.
    pub settings: usize,
    /// `forward-zone:`/`stub-zone:`/`auth-zone:`/`view:` section count.
    pub zones: usize,
    /// `#`/`;` comment lines plus `include:` directives.
    pub comments_includes: usize,
    /// Boolean-ish values (`yes`, `no`, `true`, `false`) count.
    pub booleans: usize,
}

const SECTIONS: &[&str] = &[
    "server",
    "remote-control",
    "forward-zone",
    "stub-zone",
    "auth-zone",
    "view",
    "python",
    "dynlib",
    "cachedb",
    "rpz",
    "dnscrypt",
    "edns-subnet",
    "ipsecmod",
    "subnet",
    "responses",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like unbound.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut score = 0usize;
    let mut sections = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        let head = tr.split(':').next().unwrap_or("");
        if SECTIONS.contains(&head.trim()) {
            score += 2;
            sections += 1;
        } else if tr.contains(": ")
            && tr.split(':').next().is_some_and(|k| {
                k.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '-' || c == '_')
                    && !k.is_empty()
                    && k.len() < 40
            })
        {
            score += 1;
        }
    }
    // a real unbound.conf always opens at least one named section
    // (`server:`/`forward-zone:`/…) — bare `key: value` config does not
    // qualify on its own
    sections >= 1 && score >= 4
}

impl Unbound {
    /// Parses an unbound.conf summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sections: 0,
            settings: 0,
            zones: 0,
            comments_includes: 0,
            booleans: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments_includes += 1;
                continue;
            }
            let Some((head, rest)) = tr.split_once(':') else {
                continue;
            };
            let head = head.trim();
            if SECTIONS.contains(&head) && rest.trim().is_empty() {
                c.sections += 1;
                match head {
                    "forward-zone" | "stub-zone" | "auth-zone" | "view" => c.zones += 1,
                    _ => {}
                }
                continue;
            }
            if head == "include" {
                c.comments_includes += 1;
                continue;
            }
            c.settings += 1;
            let v = rest.trim();
            if matches!(v, "yes" | "no" | "true" | "false" | "on" | "off") {
                c.booleans += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unbound() {
        let b = concat!(
            "server:\n",
            "  verbosity: 1\n",
            "  interface: 192 0 2 1\n",
            "  do-ip6: no\n",
            "remote-control:\n",
            "  control-enable: yes\n",
            "forward-zone:\n",
            "  name: \".\"\n",
            "  forward-addr: 192 0 2 2\n",
            "stub-zone:\n",
            "  name: \"stub.example.com\"\n",
            "  stub-addr: 192 0 2 3\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Unbound::parse(b).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.zones, 2);
        assert_eq!(c.settings, 8);
        assert_eq!(c.booleans, 2);
        assert_eq!(c.comments_includes, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"key: value\nother: stuff\n"));
        // bare `key: value` config without a named section is not unbound
        assert!(!detect(b"alpha: 1\nbeta: 2\ngamma: 3\ndelta: 4\n"));
        assert!(Unbound::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
