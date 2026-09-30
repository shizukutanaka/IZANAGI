//! Windows Registry `.reg` file census.
//!
//! `.reg` files start with `REGEDIT4` or
//! `Windows Registry Editor Version 5.00`, then
//! `[HKEY_LOCAL_MACHINE\\SOFTWARE\\…]` key sections holding
//! `"Name"="value"`, `"Name"=hex:…`, `"Name"=dword:…`,
//! `"Name"=hex(b):`, `"Name"=hex(7):`, `"Name"=hex(2):`,
//! `"Name"=hex(4):`, `@=…` default values and `;` comments.
//!
//! ```rust
//! let c = izanagi_kit::regfile::Regfile::parse(b"Windows Registry Editor Version 5.00\n\n[HKEY_LOCAL_MACHINE\\\\SOFTWARE\\\\X]\n\"A\"=\"b\"\n").unwrap();
//! assert_eq!(c.values, 1);
//! ```

/// `.reg` file census.
#[derive(Debug, Clone)]
pub struct Regfile {
    /// `[HKEY_…]` key sections.
    pub keys: usize,
    /// `"name"=…`/`@=…` value entries.
    pub values: usize,
    /// `hex:`/`hex(N):` binary values.
    pub hex_values: usize,
    /// `dword:` values.
    pub dwords: usize,
    /// `;` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a `.reg` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.starts_with("REGEDIT4")
        || t.starts_with("Windows Registry Editor")
        || t.trim_start_matches(['\u{feff}', ' ', '\n', '\r'])
            .starts_with("Windows Registry Editor")
        || t.contains("[HKEY_")
}

impl Regfile {
    /// Parse a `.reg` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            values: 0,
            hex_values: 0,
            dwords: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.contains(']') {
                c.keys += 1;
                continue;
            }
            if s.contains('=') && (s.starts_with('"') || s.starts_with('@')) {
                c.values += 1;
                let rhs = s.split('=').nth(1).unwrap_or("");
                if rhs.starts_with("hex") {
                    c.hex_values += 1;
                } else if rhs.starts_with("dword:") {
                    c.dwords += 1;
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
    fn parses_reg() {
        let b = concat!(
            "Windows Registry Editor Version 5.00\n",
            "\n",
            "; comment\n",
            "[HKEY_LOCAL_MACHINE\\SOFTWARE\\Vendor\\App]\n",
            "\"InstallPath\"=\"C:\\\\App\"\n",
            "\"Enabled\"=dword:00000001\n",
            "\"Blob\"=hex:01,02,03\n",
            "\"Multi\"=hex(7):61,00,62,00,00,00\n",
            "@=\"default\"\n",
            "[HKEY_CURRENT_USER\\Control Panel\\Desktop]\n",
            "\"Wallpaper\"=\"C:\\\\bg.jpg\"\n",
        );
        let c = Regfile::parse(b.as_bytes()).unwrap();
        assert_eq!(c.keys, 2);
        assert_eq!(c.values, 6);
        assert_eq!(c.hex_values, 2);
        assert_eq!(c.dwords, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Regfile::parse(b"[x]\ny=1\n").is_none());
    }
}
