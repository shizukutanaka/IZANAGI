//! Stata `.dta` — binary dataset (release 104–118) or XML-wrapped
//! `<stata_dta>` (release ≥117).
//!
//! Binary header: `version u8 | byteorder (1=LSF/2=MSF) | filetype=1 |
//! unused | nvar u16 | nobs u32`.
//!
//! ```
//! let d = [113, 1, 1, 0, 3, 0, 5, 0, 0, 0];
//! let t = izanagi_kit::dta::parse(&d).unwrap();
//! assert_eq!(t.release, 113);
//! assert_eq!((t.nvar, t.nobs), (3, 5));
//! ```

/// Parsed Stata dataset header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dta {
    /// `ds_format` release byte (104–118), or the release read from XML.
    pub release: u16,
    /// True for little-endian (`LSF`) datasets; false for `MSF`.
    pub little_endian: bool,
    /// Number of variables (`nvar`), when readable.
    pub nvar: u32,
    /// Number of observations (`nobs`), when readable.
    pub nobs: u32,
    /// True for the XML `<stata_dta>` container (117+).
    pub xml_wrapped: bool,
}

fn le16(d: &[u8], o: usize) -> Option<u32> {
    Some(*d.get(o)? as u32 | ((*d.get(o + 1)? as u32) << 8))
}

fn be16(d: &[u8], o: usize) -> Option<u32> {
    Some(((*d.get(o)? as u32) << 8) | *d.get(o + 1)? as u32)
}

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        ((*d.get(o)? as u32) << 24)
            | ((*d.get(o + 1)? as u32) << 16)
            | ((*d.get(o + 2)? as u32) << 8)
            | *d.get(o + 3)? as u32,
    )
}

/// Parse a `.dta` header; `None` on unknown releases or bad flags.
pub fn parse(d: &[u8]) -> Option<Dta> {
    if d.starts_with(b"<stata_dta") {
        let s = std::str::from_utf8(d).ok()?;
        let rel = s
            .find("<release>")
            .and_then(|i| s[i + 9..].find("</release>").map(|e| &s[i + 9..i + 9 + e]))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        return Some(Dta {
            release: rel,
            little_endian: true,
            nvar: 0,
            nobs: 0,
            xml_wrapped: true,
        });
    }
    let release = *d.first()?;
    if !(104..=118).contains(&release) {
        return None;
    }
    let little_endian = match *d.get(1)? {
        1 => true,
        2 => false,
        _ => return None,
    };
    if *d.get(2)? != 1 {
        return None;
    }
    let (nvar, nobs) = if little_endian {
        (le16(d, 4)?, le32(d, 6)?)
    } else {
        (be16(d, 4)?, be32(d, 6)?)
    };
    Some(Dta {
        release: release as u16,
        little_endian,
        nvar,
        nobs,
        xml_wrapped: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_lsf() {
        let d = [114, 1, 1, 0, 10, 0, 20, 0, 0, 0];
        let t = parse(&d).unwrap();
        assert_eq!((t.release, t.nvar, t.nobs), (114, 10, 20));
        assert!(t.little_endian);
        assert!(!t.xml_wrapped);
    }

    #[test]
    fn binary_msf() {
        let d = [105, 2, 1, 0, 0, 7, 0, 0, 0, 9];
        let t = parse(&d).unwrap();
        assert!(!t.little_endian);
        assert_eq!((t.nvar, t.nobs), (7, 9));
    }

    #[test]
    fn xml() {
        let d = b"<stata_dta><header><release>118</release></header>";
        let t = parse(d).unwrap();
        assert!(t.xml_wrapped && t.release == 118);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[50, 1, 1, 0]).is_none()); // release too old
        assert!(parse(&[114, 9, 1, 0]).is_none()); // bad byteorder
        assert!(parse(&[114, 1, 7, 0]).is_none()); // filetype != 1
    }
}
