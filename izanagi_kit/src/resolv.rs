//! `/etc/resolv.conf` resolver configuration format.
//!
//! resolv.conf is a simple `keyword value` file: `nameserver`,
//! `domain`, `search`, `sortlist`, `options` (with colon/equals
//! sub-options like `ndots:5`, `rotate`, `timeout:2`).
//!
//! ```
//! let b = concat!(
//!     "nameserver 192 0 2 1\n",
//!     "nameserver 2001 db8 1\n",
//!     "search corp.local example.com\n",
//!     "options ndots:5 rotate timeout:2\n"
//! ).as_bytes();
//! assert!(izanagi_kit::resolv::detect(b));
//! let c = izanagi_kit::resolv::Resolv::parse(b).unwrap();
//! assert_eq!(c.nameservers, 2);
//! ```

/// Parsed resolv.conf summary.
#[derive(Debug, Clone)]
pub struct Resolv {
    /// `nameserver` lines.
    pub nameservers: usize,
    /// `domain` lines.
    pub domain: usize,
    /// `search` lines (plus token count minus one).
    pub search: usize,
    /// `sortlist` lines.
    pub sortlist: usize,
    /// `options` lines.
    pub options: usize,
    /// `options` sub-option count (`name:value`/`name` tokens after `options`).
    pub sub_options: usize,
    /// `lookup`/`family` legacy keys.
    pub legacy: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like resolv.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    // resolv.conf directives are line-start keywords followed by an
    // argument; `nameserverx`/`searchlight` must not qualify. `domain`,
    // `sortlist`, and `options` are also valid directives (a file with
    // only `search`/`domain`/`options` is still resolv.conf content).
    ["nameserver", "search", "domain", "sortlist", "options"]
        .iter()
        .any(|key| t.lines().any(|l| l.split_whitespace().next() == Some(*key)))
}

impl Resolv {
    /// Parses a resolv.conf summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            nameservers: 0,
            domain: 0,
            search: 0,
            sortlist: 0,
            options: 0,
            sub_options: 0,
            legacy: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim_start();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            let mut it = tr.split_whitespace();
            let Some(kw) = it.next() else { continue };
            match kw {
                "nameserver" => c.nameservers += 1,
                "domain" => c.domain += 1,
                "search" => {
                    c.search += 1;
                    c.sub_options += it.count();
                }
                "sortlist" => c.sortlist += 1,
                "options" => {
                    c.options += 1;
                    c.sub_options += it.count();
                }
                "lookup" | "family" => c.legacy += 1,
                _ => {}
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resolv() {
        let b = concat!(
            "nameserver 192 0 2 1\n",
            "nameserver 2001 db8 1\n",
            "search corp.local example.com\n",
            "sortlist 130 155 160 0 255 0\n",
            "options ndots:5 rotate timeout:2\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Resolv::parse(b).unwrap();
        assert_eq!(c.nameservers, 2);
        assert_eq!(c.search, 1);
        assert_eq!(c.sortlist, 1);
        assert_eq!(c.options, 1);
        assert_eq!(c.sub_options, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_search_or_domain_only() {
        assert!(detect(b"domain example.com\n"));
        assert!(detect(b"search corp.local example.com\n"));
        assert!(detect(b"options timeout:2 rotate\n"));
        assert!(detect(b"sortlist 130.155.160.0/255.255.240.0\n"));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"foo bar\n"));
        // Keyword prefixes without a word boundary are not directives.
        assert!(!detect(b"nameserverx 8.8.8.8\nsearchlight x\n"));
        assert!(Resolv::parse(b"x").is_none());
    }
}
