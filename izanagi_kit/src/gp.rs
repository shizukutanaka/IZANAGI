//! Guitar Pro binary score — `FICHIER GUITAR PRO v…` banner (v3–v5) or
//! `BCF…`/GP7 compressed forms; v3/v4/v5 carry a version tag in the header.
//!
//! ```
//! let mut d = b"FICHIER GUITAR PRO v5\x2e10".to_vec();
//! d.resize(64, 0);
//! let s = izanagi_kit::gp::parse(&d).unwrap();
//! assert_eq!(s.version, "5\x2e10");
//! assert!(izanagi_kit::gp::detect(&d));
//! ```

/// Parsed Guitar Pro header summary.
#[derive(Debug, Clone)]
pub struct Gp {
    /// Version string from the `FICHIER GUITAR PRO v…` banner, or `"7"` for the
    /// GP7 `BCFZ` form.
    pub version: String,
    /// Major version number (3, 4, 5, 7) or `0` for unknown sub-versions.
    pub major: u8,
    /// `true` for the compressed GP7 container (`BCFZ` …).
    pub compressed: bool,
    /// Song title, decoded from the u8-length title field when readable.
    pub title: String,
}

/// Detects Guitar Pro: `FICHIER GUITAR PRO` banner or GP7 `BCFZ`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"FICHIER GUITAR PRO") || b.starts_with(b"BCFZ")
}

/// Parses the header; `None` on unknown magic.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Gp> {
    if !detect(b) {
        return None;
    }
    if b.starts_with(b"BCFZ") {
        return Some(Gp {
            version: String::from("7"),
            major: 7,
            compressed: true,
            title: String::new(),
        });
    }
    // "FICHIER GUITAR PRO vN.NN" banner, then u8 length + title.
    let banner_end = b.iter().position(|&c| c == 0).unwrap_or(b.len().min(30));
    let banner = std::str::from_utf8(&b[..banner_end]).ok()?;
    let version = banner
        .trim_start_matches("FICHIER GUITAR PRO")
        .trim_start()
        .trim_start_matches('v')
        .to_string();
    let major = version
        .as_bytes()
        .first()
        .copied()
        .unwrap_or(b'0')
        .wrapping_sub(b'0');
    let title = if banner_end < b.len() {
        let mut i = banner_end + 1;
        // skip padding zeros after the banner
        while i < b.len() && b[i] == 0 {
            i += 1;
        }
        if i < b.len() {
            let n = (b[i] as usize).min(b.len() - i - 1);
            std::str::from_utf8(&b[i + 1..i + 1 + n])
                .unwrap_or("")
                .trim_end_matches('\0')
                .to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    Some(Gp {
        version,
        major: if (3..=5).contains(&major) || major == 7 {
            major
        } else {
            0
        },
        compressed: false,
        title,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v5() {
        let mut d = b"FICHIER GUITAR PRO v5\x2e10".to_vec();
        d.push(0);
        d.push(4);
        d.extend_from_slice(b"Song");
        let s = parse(&d).unwrap();
        assert_eq!(s.version, "5\x2e10");
        assert_eq!(s.major, 5);
        assert!(!s.compressed);
        assert_eq!(s.title, "Song");
    }

    #[test]
    fn parses_v3_short_banner() {
        let mut d = b"FICHIER GUITAR PRO v3\x2e00".to_vec();
        d.resize(80, 0);
        let s = parse(&d).unwrap();
        assert_eq!(s.major, 3);
    }

    #[test]
    fn gp7() {
        let s = parse(b"BCFZ\x00\x01\x02").unwrap();
        assert_eq!(s.major, 7);
        assert!(s.compressed);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"RIFF....").is_none());
        assert!(!detect(b"text"));
    }
}
