//! Windows `boot.ini` census.
//!
//! boot.ini is an INI used by NTLDR: `[boot loader]`
//! (`timeout=`/`default=`), `[operating systems]` —
//! `multi(0)disk(0)rdisk(0)partition(1)\\WINDOWS="Microsoft Windows"`
//! or `scsi()`/`signature()`/`ramdisk()` ARC paths with `/fastdetect`
//! `/NOGUIBOOT`/`/SOS`/`/DEBUG`/`/BAUDRATE`/`/3GB`/`/PAE` switches.
//!
//! ```rust
//! let c = izanagi_kit::bootini::Bootini::parse(b"[boot loader]\ntimeout=30\n[operating systems]\nmulti(0)disk(0)rdisk(0)partition(1)\\\\WINDOWS=\"W\"\n").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

/// `boot.ini` census.
#[derive(Debug, Clone)]
pub struct Bootini {
    /// `[section]` headers.
    pub sections: usize,
    /// `key=value` entries (options + ARC paths).
    pub entries: usize,
    /// ARC-path OS entries (`multi(`/`scsi(`/`signature(`/`ramdisk(`).
    pub arc_paths: usize,
    /// `/`-prefixed boot switches.
    pub switches: usize,
    /// `;` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a boot.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("[boot loader]") || t.contains("[operating systems]"))
        && (t.contains("multi(")
            || t.contains("scsi(")
            || t.contains("signature(")
            || t.contains("timeout=")
            || t.contains("default="))
}

impl Bootini {
    /// Parse a boot.ini into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            arc_paths: 0,
            switches: 0,
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
                c.sections += 1;
                continue;
            }
            if s.contains('=') {
                c.entries += 1;
                let key = s.split('=').next().unwrap_or("");
                if key.starts_with("multi(")
                    || key.starts_with("scsi(")
                    || key.starts_with("signature(")
                    || key.starts_with("ramdisk(")
                {
                    c.arc_paths += 1;
                }
            }
            // count `/`-switches anywhere on the line
            for tok in s.split_whitespace() {
                if tok.starts_with('/') && tok.len() > 1 {
                    c.switches += 1;
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
    fn parses_bootini() {
        let b = concat!(
            "[boot loader]\n",
            "timeout=30\n",
            "default=multi(0)disk(0)rdisk(0)partition(1)\\WINDOWS\n",
            "[operating systems]\n",
            "multi(0)disk(0)rdisk(0)partition(1)\\WINDOWS=\"Microsoft Windows XP\" /fastdetect /NOGUIBOOT\n",
            "multi(0)disk(0)rdisk(0)partition(2)\\WINNT=\"Windows 2000\" /SOS\n",
            "scsi(0)disk(0)rdisk(0)partition(3)\\WINNT=\"NT\" /DEBUG /BAUDRATE=57600\n",
        );
        let c = Bootini::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.entries, 5);
        assert_eq!(c.arc_paths, 3);
        assert_eq!(c.switches, 5);
    }

    #[test]
    fn rejects_other() {
        assert!(Bootini::parse(b"[foo]\nx=1\n").is_none());
    }
}
