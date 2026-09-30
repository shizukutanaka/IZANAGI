//! OpenWrt UCI (`/etc/config/*`) census.
//!
//! UCI files contain `package 'name'`, `config <type> ['name']` section
//! headers, `option <key> '<value>'` scalar assignments and `list <key>
//! '<value>'` list entries. `parse` counts each and the distinct section
//! types (`interface`, `zone`, `rule`, …).
//!
//! ```rust
//! let u = concat!(
//!     "package network\n",
//!     "config interface 'lan'\n",
//!     "        option proto 'static'\n",
//!     "        list ipaddr '10/24'\n",
//!     "config zone\n",
//!     "        option name 'lan'\n",
//! );
//! let c = izanagi_kit::uci::Uci::parse(u.as_bytes()).unwrap();
//! assert_eq!(c.configs, 2);
//! assert_eq!(c.named, 1);
//! ```

/// UCI configuration census.
#[derive(Debug, Clone)]
pub struct Uci {
    /// `package ` lines.
    pub packages: usize,
    /// `config ` section headers.
    pub configs: usize,
    /// `config` sections carrying an explicit name argument.
    pub named: usize,
    /// `option ` assignments.
    pub options: usize,
    /// `list ` entries.
    pub lists: usize,
    /// Distinct section types (`config <type>`).
    pub section_types: usize,
    /// Comments (`#`).
    pub comments: usize,
}

/// Whether the buffer looks like a UCI config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut cfg = false;
    let mut opt = false;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("config ") {
            cfg = true;
        }
        if s.starts_with("option ") || s.starts_with("package ") {
            opt = true;
        }
    }
    cfg && opt
}

impl Uci {
    /// Parse a UCI configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            packages: 0,
            configs: 0,
            named: 0,
            options: 0,
            lists: 0,
            section_types: 0,
            comments: 0,
        };
        let mut types: Vec<&str> = Vec::new();
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("package ") {
                c.packages += 1;
                continue;
            }
            if let Some(rest) = s.strip_prefix("config ") {
                c.configs += 1;
                let mut it = rest.split_whitespace();
                if let Some(ty) = it.next() {
                    let ty = ty.trim_matches('\'').trim_matches('"');
                    if !types.contains(&ty) {
                        types.push(ty);
                    }
                }
                if it.next().is_some() {
                    c.named += 1;
                }
                continue;
            }
            if s.starts_with("option ") {
                c.options += 1;
                continue;
            }
            if s.starts_with("list ") {
                c.lists += 1;
                continue;
            }
        }
        c.section_types = types.len();
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let b = concat!(
            "package network\n",
            "config interface 'lan'\n",
            "        option proto 'static'\n",
            "        list ipaddr '10/24'\n",
            "        list dns '8'\n",
            "config interface 'wan'\n",
            "        option proto 'dhcp'\n",
            "# comment\n",
            "config zone\n",
            "        option name 'lan'\n",
        );
        let c = Uci::parse(b.as_bytes()).unwrap();
        assert_eq!(c.packages, 1);
        assert_eq!(c.configs, 3);
        assert_eq!(c.named, 2);
        assert_eq!(c.options, 3);
        assert_eq!(c.lists, 2);
        assert_eq!(c.section_types, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Uci::parse(b"hello").is_none());
    }
}
