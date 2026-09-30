//! Apple `.nib` archive scanner.
//!
//! Compiled `.nib` files are NSKeyedArchiver archives: modern ones are
//! binary property lists (`bplist00` magic + `$archiver` /
//! `NSKeyedArchiver` object names in the plist body); older keyed
//! archives may appear as XML plists with `<key>$archiver</key>` —
//! both are detected here.
//!
//! ```
//! let mut f = b"bplist00".to_vec();
//! f.extend_from_slice(b"    $archiver NSKeyedArchiver $objects ");
//! let n = izanagi_kit::nib::parse(&f).unwrap();
//! assert!(n.binary_plist);
//! assert!(n.keyed_archiver);
//! ```
//!
//! Reference: Apple `NSKeyedArchiver` archive format notes (the
//! `bplist00` magic + `$archiver`/`$objects` keys used by compiled
//! Interface Builder output).

/// Parsed `.nib` statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Nib {
    /// `true` for binary-plist (`bplist00`) archives, `false` for XML
    /// plist form.
    pub binary_plist: bool,
    /// `true` when `$archiver`/`NSKeyedArchiver` markers exist.
    pub keyed_archiver: bool,
    /// Byte length of the archive.
    pub len: usize,
}

/// Parse a `.nib` file; `None` when no plist/archiver markers exist.
pub fn parse(d: &[u8]) -> Option<Nib> {
    let binary_plist = d.starts_with(b"bplist");
    let xml_plist = d.starts_with(b"<?xml") && d.len() > 6;
    if !binary_plist && !xml_plist {
        return None;
    }
    let body = core::str::from_utf8(d)
        .ok()
        .map(|s| s.contains("NSKeyedArchiver") || s.contains("$archiver"));
    let keyed_archiver = body.unwrap_or_else(|| {
        // bplist may embed non-UTF8; fall back to a byte scan.
        d.windows(15).any(|w| w == b"NSKeyedArchiver") || d.windows(9).any(|w| w == b"$archiver")
    });
    if !keyed_archiver {
        return None;
    }
    Some(Nib {
        binary_plist,
        keyed_archiver,
        len: d.len(),
    })
}

/// `true` if the buffer looks like a `.nib` archive.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nib() -> Vec<u8> {
        let mut f = b"bplist00".to_vec();
        f.extend_from_slice(b"  $archiver NSKeyedArchiver $objects ");
        f
    }

    #[test]
    fn parses() {
        let n = parse(&nib()).unwrap();
        assert!(n.binary_plist);
        assert!(n.keyed_archiver);
        assert_eq!(n.len, nib().len());
    }

    #[test]
    fn xml_plist() {
        let n = parse(b"<?xml version=\"1\x2e0\"?><plist><key>$archiver</key></plist>").unwrap();
        assert!(!n.binary_plist);
        assert!(n.keyed_archiver);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"random data").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&nib()));
        assert!(!detect(b"bplist"));
    }
}
