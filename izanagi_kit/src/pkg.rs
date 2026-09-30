//! macOS `.pkg` installer: an XAR archive whose heap carries a
//! `PackageInfo` document (plus `Payload`/`Scripts`/`Distribution`
//! members). Detection: `xar` header + the `PackageInfo` byte string
//! somewhere in the payload heap.
//!
//! ```
//! // xar header: magic, 28-byte header, toc lengths, checksum id
//! let mut f = vec![0u8; 32];
//! f[..4].copy_from_slice(&[0x78, 0x61, 0x72, 0x21]); // "xar!"
//! f[4..6].copy_from_slice(&[0x00, 0x1c]); // header_size = 28
//! f[6..8].copy_from_slice(&[0x00, 0x01]); // version
//! f[8..16].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 4]); // toc compressed
//! f[16..24].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 8]); // toc uncompressed
//! f[28..32].copy_from_slice(b"junk");
//! f.extend_from_slice(b"...PackageInfo...\x00Payload");
//! let p = izanagi_kit::pkg::parse(&f).unwrap();
//! assert!(p.has_package_info);
//! assert!(p.has_payload);
//! ```

use std::vec::Vec;

/// A detected macOS installer package.
#[derive(Clone, Debug)]
pub struct Pkg {
    /// XAR format version.
    pub version: u16,
    /// Declared XAR header size.
    pub header_size: u16,
    /// Compressed TOC length in bytes.
    pub toc_compressed: u64,
    /// `PackageInfo` found in the heap bytes — the pkg signature.
    pub has_package_info: bool,
    /// `Payload` member name found.
    pub has_payload: bool,
    /// `Distribution` (distribution-style flat package) found.
    pub has_distribution: bool,
    /// Other member names spotted in the heap (`Scripts`, `Resources`,
    /// `Bom`, `PackageInfo` excluded).
    pub members: Vec<std::string::String>,
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || hay.windows(needle.len()).any(|w| w == needle)
}

/// Parse a `.pkg`: valid XAR header whose heap mentions
/// `PackageInfo`. Returns `None` otherwise.
pub fn parse(d: &[u8]) -> Option<Pkg> {
    let x = crate::xar::parse(d)?;
    let (_, toc_len) = x.toc()?;
    let heap = d.get(usize::from(x.header_size).checked_add(toc_len)?..)?;
    if !contains(heap, b"PackageInfo") {
        return None;
    }
    let mut members: Vec<std::string::String> = Vec::new();
    for cand in [
        &b"Scripts"[..],
        &b"Resources"[..],
        &b"Bom"[..],
        &b"locate.plist"[..],
    ] {
        if contains(heap, cand) {
            members.push(String::from_utf8_lossy(cand).into_owned());
        }
    }
    Some(Pkg {
        version: x.version,
        header_size: x.header_size,
        toc_compressed: x.toc_compressed,
        has_package_info: true,
        has_payload: contains(heap, b"Payload"),
        has_distribution: contains(heap, b"Distribution"),
        members,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut f = vec![0u8; 32];
        f[..4].copy_from_slice(&[0x78, 0x61, 0x72, 0x21]);
        f[4..6].copy_from_slice(&[0x00, 0x1c]);
        f[6..8].copy_from_slice(&[0x00, 0x01]);
        f[8..16].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 4]);
        f[16..24].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 8]);
        f[28..32].copy_from_slice(b"junk");
        f.extend_from_slice(b"\x00\x00PackageInfo\x00Payload\x00Scripts\x00Distribution");
        f
    }

    #[test]
    fn flat_pkg() {
        let p = parse(&fixture()).unwrap();
        assert!(p.has_package_info);
        assert!(p.has_payload);
        assert!(p.has_distribution);
        assert!(p.members.contains(&String::from("Scripts")));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut f = fixture();
        f.truncate(32); // heap without PackageInfo
        assert!(parse(&f).is_none());
        let mut g = fixture();
        g[0] = b'z'; // bad magic
        assert!(parse(&g).is_none());
    }
}
