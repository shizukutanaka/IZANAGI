//! Minecraft `eula.txt` census.
//!
//! `#` comment banner ("By changing the setting below to TRUE you are
//! indicating your agreement to our EULA", plus a timestamp line like
//! `#Tue Jan 01 00:00:00 UTC 2026`) followed by `eula=true|false`.
//!
//! ```rust
//! let e = "#By changing the setting below to TRUE you are agreeing\neula=true\n";
//! let c = izanagi_kit::eula::Eula::parse(e.as_bytes()).unwrap();
//! assert!(c.accepted);
//! ```

/// eula.txt census.
#[derive(Debug, Clone)]
pub struct Eula {
    /// `eula=` line seen with value `true`.
    pub accepted: bool,
    /// `#` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like a Minecraft eula.txt.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let l = t.to_lowercase();
    l.contains("eula=") || l.contains("eula =")
}

impl Eula {
    /// Parse an eula.txt into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            accepted: false,
            comments: 0,
        };
        let mut saw = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let Some(eq) = s.find('=') else {
                continue;
            };
            if s[..eq].trim().eq_ignore_ascii_case("eula") {
                saw = true;
                c.accepted = s[eq + 1..].trim().eq_ignore_ascii_case("true");
            }
        }
        saw.then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_eula() {
        let b = concat!(
            "#By changing the setting below to TRUE you are indicating your agreement\n",
            "#Tue Jan 01 00:00:00 UTC 2026\n",
            "eula=true\n",
        );
        let c = Eula::parse(b.as_bytes()).unwrap();
        assert!(c.accepted);
        assert_eq!(c.comments, 2);
    }

    #[test]
    fn parses_rejected() {
        let c = Eula::parse(b"eula=false\n").unwrap();
        assert!(!c.accepted);
    }

    #[test]
    fn rejects_other() {
        assert!(Eula::parse(b"foo=1").is_none());
    }
}
