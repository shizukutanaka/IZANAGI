//! opkg `.ipk` / `.opk` package: an `ar` archive carrying
//! `debian-binary`, `control.tar.*`, and `data.tar.*` members.
//!
//! ```
//! let ipk = izanagi_kit::ipk::parse(&fixture()).unwrap();
//! assert!(ipk.has_control);
//! assert!(ipk.has_data);
//! assert_eq!(ipk.debian_binary, Some(2));
//!
//! fn member(v: &mut Vec<u8>, name: &str, data: &[u8]) {
//!     let mut h = [b' '; 60];
//!     h[..16].copy_from_slice(format!("{:<16}", name).as_bytes());
//!     h[16..28].copy_from_slice(b"0           ");
//!     h[28..34].copy_from_slice(b"0     ");
//!     h[34..40].copy_from_slice(b"0     ");
//!     h[40..48].copy_from_slice(b"100644  ");
//!     h[48..58].copy_from_slice(format!("{:<10}", data.len()).as_bytes());
//!     h[58] = 0x60;
//!     h[59] = b'\n';
//!     v.extend_from_slice(&h);
//!     v.extend_from_slice(data);
//!     if data.len() % 2 == 1 {
//!         v.push(b'\n');
//!     }
//! }
//! fn fixture() -> Vec<u8> {
//!     let mut v = b"!<arch>\n".to_vec();
//!     member(&mut v, "debian-binary", b"2\x2e0\n");
//!     member(&mut v, "control.tar.gz", b"\x1f\x8b\x08\x00x");
//!     member(&mut v, "data.tar.gz", b"\x1f\x8b\x08\x00y");
//!     v
//! }
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed opkg package.
#[derive(Clone, Debug)]
pub struct Ipk {
    /// Member names in archive order.
    pub members: Vec<String>,
    /// `control.tar.<ext>` member present.
    pub has_control: bool,
    /// `data.tar.<ext>` member present.
    pub has_data: bool,
    /// `debian-binary` format version byte (`2` for `2\x2e0`), if the
    /// member exists and is a single-digit revision.
    pub debian_binary: Option<u8>,
    /// Compression suffix on the data tarball (`gz`, `xz`, `lz4`,
    /// `zst`, or `""` for plain tar).
    pub data_compression: String,
}

/// Parse an `.ipk`/`.opk`: `ar` container holding the three opkg
/// members. Returns `None` when the archive is not ar or lacks either
/// the control or the data tarball.
pub fn parse(d: &[u8]) -> Option<Ipk> {
    let ar = crate::ar::parse(d)?;
    let mut members: Vec<String> = Vec::new();
    let mut has_control = false;
    let mut has_data = false;
    let mut debian_binary = None;
    let mut data_compression = String::new();
    for e in &ar.entries {
        let name = e.name_in(d, &ar.string_table);
        let base = name.rsplit('/').next().unwrap_or("");
        if base == "debian-binary" {
            let body = crate::ar::data(d, e)?;
            let text = std::str::from_utf8(body).ok()?.trim();
            let b = text.as_bytes();
            if b.first()?.is_ascii_digit() {
                debian_binary = Some(b[0] - b'0');
            }
        } else if base.starts_with("control.tar") {
            has_control = true;
        } else if base.starts_with("data.tar") {
            has_data = true;
            if let Some((_, ext)) = base.split_once("data.tar.") {
                data_compression = String::from(ext);
            }
        }
        members.push(name);
    }
    if !has_control || !has_data {
        return None;
    }
    Some(Ipk {
        members,
        has_control,
        has_data,
        debian_binary,
        data_compression,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(v: &mut Vec<u8>, name: &str, data: &[u8]) {
        let mut h = [b' '; 60];
        h[..16].copy_from_slice(format!("{:<16}", name).as_bytes());
        h[16..28].copy_from_slice(b"0           ");
        h[28..34].copy_from_slice(b"0     ");
        h[34..40].copy_from_slice(b"0     ");
        h[40..48].copy_from_slice(b"100644  ");
        h[48..58].copy_from_slice(format!("{:<10}", data.len()).as_bytes());
        h[58] = 0x60;
        h[59] = b'\n';
        v.extend_from_slice(&h);
        v.extend_from_slice(data);
        if data.len() % 2 == 1 {
            v.push(b'\n');
        }
    }

    fn fixture() -> Vec<u8> {
        let mut v = b"!<arch>\n".to_vec();
        member(&mut v, "debian-binary", b"2\x2e0\n");
        member(&mut v, "control.tar.gz", b"\x1f\x8b\x08\x00x");
        member(&mut v, "data.tar.zst", b"\x28\xb5\x2f\xfdy");
        v
    }

    #[test]
    fn parses_members() {
        let p = parse(&fixture()).unwrap();
        assert!(p.has_control);
        assert!(p.has_data);
        assert_eq!(p.debian_binary, Some(2));
        assert_eq!(p.data_compression, "zst");
        assert_eq!(p.members.len(), 3);
    }

    #[test]
    fn rejects_non_ar() {
        assert!(parse(b"not an archive").is_none());
        assert!(parse(b"!<arch>\n").is_none());
    }

    #[test]
    fn rejects_missing_members() {
        let mut v = b"!<arch>\n".to_vec();
        member(&mut v, "control.tar.gz", b"x");
        assert!(parse(&v).is_none());
    }
}
