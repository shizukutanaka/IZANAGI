//! MikroTik RouterOS `/export` script census.
//!
//! Exported RouterOS configuration is organized into `/`-rooted sections
//! (`/interface ethernet`, `/ip address`, `/system identity`) whose lines
//! start with `add`, `set`, `enable`, `disable`, `comment` or `remove`
//! followed by `key=value` pairs and bare flags. `parse` counts sections,
//! verbs and `key=value` pairs.
//!
//! ```rust
//! let r = concat!(
//!     "/interface ethernet\n",
//!     "set [ find default-name=ether1 ] speed=1Gbps comment=\"uplink\"\n",
//!     "/ip address\n",
//!     "add address=192__168__0__1/24 interface=ether1\n",
//! );
//! ```
//! The real form uses `.` separators — replaced above to keep integers
//! only. A real fixture:
//!
//! ```rust
//! let r = concat!(
//!     "/interface ethernet\n",
//!     "set [ find default-name=ether1 ] comment=\"uplink\" speed=1000\n",
//!     "add name=ether2 comment=\"lan\"\n",
//!     "/ip firewall filter\n",
//!     "add chain=forward action=drop\n",
//! );
//! let c = izanagi_kit::routeros::Routeros::parse(r.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.adds, 2);
//! assert_eq!(c.sets, 1);
//! ```

/// RouterOS export census.
#[derive(Debug, Clone)]
pub struct Routeros {
    /// `/` section headers.
    pub sections: usize,
    /// `add` verb lines.
    pub adds: usize,
    /// `set` verb lines.
    pub sets: usize,
    /// `enable`/`disable`/`remove`/`comment` verb lines.
    pub other_verbs: usize,
    /// `key=value` pairs total.
    pub kv_pairs: usize,
    /// `comment=` pairs.
    pub comments: usize,
    /// `disabled=yes`/`disabled=no` pairs.
    pub disabled: usize,
    /// `[ find` selectors.
    pub find_selectors: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a RouterOS export.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut section = false;
    let mut verb = false;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with('/') {
            section = true;
        }
        if s.starts_with("add ") || s.starts_with("set ") || s.starts_with("set [") {
            verb = true;
        }
    }
    section && verb
}

impl Routeros {
    /// Parse a RouterOS export into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sections: 0,
            adds: 0,
            sets: 0,
            other_verbs: 0,
            kv_pairs: 0,
            comments: 0,
            disabled: 0,
            find_selectors: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('/') {
                c.sections += 1;
                continue;
            }
            if s.starts_with("add ") || s == "add" {
                c.adds += 1;
            } else if s.starts_with("set ") || s == "set" {
                c.sets += 1;
            } else if s.starts_with("enable ")
                || s.starts_with("disable ")
                || s.starts_with("remove ")
                || s.starts_with("comment ")
            {
                c.other_verbs += 1;
            }
            for tok in s.split_whitespace() {
                if tok.contains('=') {
                    c.kv_pairs += 1;
                }
                if tok.starts_with("comment=") {
                    c.comments += 1;
                }
                if tok.starts_with("disabled=") {
                    c.disabled += 1;
                }
            }
            if s.contains("[ find") {
                c.find_selectors += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_export() {
        let b = concat!(
            "/interface ethernet\n",
            "set [ find default-name=ether1 ] comment=\"up\" speed=auto\n",
            "add name=ether2 disabled=yes comment=\"lan\"\n",
            "/ip address\n",
            "add address=addr/24 interface=ether2\n",
            "/system identity\n",
            "set name=gw\n",
        );
        let c = Routeros::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.adds, 2);
        assert_eq!(c.sets, 2);
        assert_eq!(c.kv_pairs, 9);
        assert_eq!(c.comments, 2);
        assert_eq!(c.disabled, 1);
        assert_eq!(c.find_selectors, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Routeros::parse(b"hello").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
