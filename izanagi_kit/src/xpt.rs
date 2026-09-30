//! SAS XPORT (transport) `.xpt` — 80-byte card-image records.
//!
//! Record 1: `HEADER RECORD*******LIBRARY HEADER RECORD!!!…`
//! Record 2: `…SAS     <ver><os>XPT` etc. Member blocks begin with
//! `HEADER RECORD*******MEMBER HEADER RECORD`.
//!
//! ```
//! let mut card = |s: &[u8]| { let mut v = s.to_vec(); v.resize(80, b' '); v };
//! let mut d = card(b"HEADER RECORD*******LIBRARY HEADER RECORD!!!!!!!000000000000000000000000000000");
//! d.extend_from_slice(&card(b"000000000000000000000000SAS     9.4    Linux   XPT.WINNT"));
//! d.extend_from_slice(&card(b"HEADER RECORD*******MEMBER HEADER RECORD!!!!!!!000000000000000000000000000000"));
//! let x = izanagi_kit::xpt::parse(&d).unwrap();
//! assert_eq!(x.members, 1);
//! ```

/// Parsed XPORT container summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xpt {
    /// Total 80-byte card records present.
    pub records: usize,
    /// `MEMBER HEADER RECORD` blocks (one per dataset member).
    pub members: usize,
    /// Version string from record 2 (`SAS` product field, trimmed).
    pub sas_version: String,
}

const CARD: usize = 80;

fn card(d: &[u8], i: usize) -> Option<&[u8]> {
    d.get(i * CARD..i * CARD + CARD)
}

/// Parse an XPT file; `None` unless the first card is the library header.
pub fn parse(d: &[u8]) -> Option<Xpt> {
    let r1 = std::str::from_utf8(card(d, 0)?).ok()?;
    if !r1.starts_with("HEADER RECORD") || !r1.contains("LIBRARY HEADER RECORD") {
        return None;
    }
    let r2 = std::str::from_utf8(card(d, 1).unwrap_or(&[])).unwrap_or("");
    let sas_version = if r2.contains("SAS") {
        let i = r2.find("SAS")? + 3;
        r2[i..].split_whitespace().next().unwrap_or("").to_string()
    } else {
        String::new()
    };
    let records = d.len() / CARD;
    let mut members = 0usize;
    for c in 0..records {
        if let Some(t) = card(d, c).and_then(|b| std::str::from_utf8(b).ok()) {
            if t.contains("MEMBER HEADER RECORD") || t.contains("DSCPTOR HEADER RECORD") {
                members += 1;
            }
        }
    }
    Some(Xpt {
        records,
        members,
        sas_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cards(spec: &[&[u8]]) -> Vec<u8> {
        let mut d = Vec::new();
        for c in spec {
            let mut v = c.to_vec();
            v.resize(CARD, b' ');
            d.extend_from_slice(&v);
        }
        d
    }

    #[test]
    fn basic() {
        let d = cards(&[
            b"HEADER RECORD*******LIBRARY HEADER RECORD!!!!!!!0000",
            b"0000SAS     9.4    Linux",
            b"HEADER RECORD*******MEMBER HEADER RECORD!!!!!!!",
            b"HEADER RECORD*******DSCPTOR HEADER RECORD!!!!!!!",
        ]);
        let x = parse(&d).unwrap();
        assert_eq!(x.records, 4);
        assert_eq!(x.members, 2);
        assert_eq!(x.sas_version, "9.4");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&cards(&[b"NOT A HEADER"])).is_none());
        assert!(parse(&cards(&[b"HEADER RECORD*******MEMBER HEADER RECORD"])).is_none());
    }
}
