//! Debian `.deb` binary package — a thin layer over [`crate::ar`].
//!
//! A `.deb` is an `ar` archive whose first member is
//! `debian-binary` (contents like `2.0\n`), followed by
//! `control.tar.*` and `data.tar.*` members. `parse` resolves the
//! format version, locates both tarballs, and reports their
//! compression from the member extension (gz/xz/zst/bz2/none).
//!
//! ```
//! use izanagi_kit::deb::{parse, Compression};
//!
//! let mut a = b"!<arch>\n".to_vec();
//! let member = |a: &mut Vec<u8>, name: &str, data: &[u8]| {
//!     let mut h = format!("{:<16}{:<12}{:<6}{:<6}{:<8o}{:<10}`\n",
//!         name, 0, 0, 0, 0o100644, data.len());
//!     a.extend_from_slice(h.as_bytes());
//!     a.extend_from_slice(data);
//!     if data.len() % 2 == 1 { a.push(b'\n'); }
//! };
//! member(&mut a, "debian-binary", b"2.0\n");
//! member(&mut a, "control.tar.xz", b"CTL");
//! member(&mut a, "data.tar.gz", b"DATA");
//! let d = parse(&a).unwrap();
//! assert_eq!(d.version, "2.0");
//! assert_eq!(d.control_compression, Compression::Xz);
//! assert_eq!(d.data_compression, Compression::Gzip);
//! ```

use crate::ar;

/// Payload compression implied by a member name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    /// `.tar` uncompressed.
    None,
    /// `.tar.gz`
    Gzip,
    /// `.tar.bz2`
    Bzip2,
    /// `.tar.xz`
    Xz,
    /// `.tar.zst`
    Zstd,
    /// `.tar.lzma` or unknown suffix.
    Other,
}

fn compression_of(name: &str) -> Compression {
    if name.ends_with(".gz") {
        Compression::Gzip
    } else if name.ends_with(".bz2") {
        Compression::Bzip2
    } else if name.ends_with(".xz") {
        Compression::Xz
    } else if name.ends_with(".zst") {
        Compression::Zstd
    } else if name.ends_with(".tar") {
        Compression::None
    } else {
        Compression::Other
    }
}

/// A parsed `.deb` file.
#[derive(Clone, Debug)]
pub struct Deb {
    /// Format version string from `debian-binary` (e.g. "2.0").
    pub version: String,
    /// All member names in order.
    pub members: Vec<String>,
    /// Compression of the control tarball.
    pub control_compression: Compression,
    /// Compression of the data tarball.
    pub data_compression: Compression,
    /// File offsets of the control/data member payloads.
    pub control_at: usize,
    /// Data member payload offset.
    pub data_at: usize,
    /// Control/data member payload lengths.
    pub control_len: usize,
    /// Data member payload length.
    pub data_len: usize,
}

/// Parse a `.deb` (an `ar` archive with a `debian-binary` member).
/// Returns `None` when the ar parse fails, the binary member is
/// missing/not first, or either tarball is absent.
pub fn parse(d: &[u8]) -> Option<Deb> {
    let arc = ar::parse(d)?;
    let first = arc.entries.first()?;
    let strings = arc.string_table.as_slice();
    if first.name_in(d, strings) != "debian-binary" {
        return None;
    }
    let raw = ar::data(d, first)?;
    let version = core::str::from_utf8(raw).ok()?.trim_end().to_string();
    if version.is_empty() {
        return None;
    }
    let mut out = Deb {
        version,
        members: arc.entries.iter().map(|e| e.name_in(d, strings)).collect(),
        control_compression: Compression::Other,
        data_compression: Compression::Other,
        control_at: 0,
        data_at: 0,
        control_len: 0,
        data_len: 0,
    };
    for e in &arc.entries {
        let name = e.name_in(d, strings);
        if name.starts_with("control.tar") {
            out.control_compression = compression_of(&name);
            out.control_at = e.offset;
            out.control_len = e.size;
        } else if name.starts_with("data.tar") {
            out.data_compression = compression_of(&name);
            out.data_at = e.offset;
            out.data_len = e.size;
        }
    }
    if out.control_at == 0 || out.data_at == 0 {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(a: &mut Vec<u8>, name: &str, data: &[u8]) {
        let h = format!(
            "{:<16}{:<12}{:<6}{:<6}{:<8o}{:<10}`\n",
            name,
            0,
            0,
            0,
            0o100644,
            data.len()
        );
        a.extend_from_slice(h.as_bytes());
        a.extend_from_slice(data);
        if data.len() % 2 == 1 {
            a.push(b'\n');
        }
    }

    #[test]
    fn parses_and_classifies() {
        let mut a = b"!<arch>\n".to_vec();
        member(&mut a, "debian-binary", b"2.0\n");
        member(&mut a, "control.tar.xz", b"CTLX");
        member(&mut a, "data.tar", b"DATADATA");
        let d = parse(&a).unwrap();
        assert_eq!(d.version, "2.0");
        assert_eq!(d.members.len(), 3);
        assert_eq!(d.control_compression, Compression::Xz);
        assert_eq!(d.data_compression, Compression::None);
        assert_eq!(&a[d.data_at..d.data_at + d.data_len], b"DATADATA");
        assert_eq!(&a[d.control_at..d.control_at + 4], b"CTLX");
    }

    #[test]
    fn compression_suffixes() {
        for (n, c) in [
            ("data.tar.gz", Compression::Gzip),
            ("data.tar.bz2", Compression::Bzip2),
            ("data.tar.zst", Compression::Zstd),
            ("data.tar.lzma", Compression::Other),
        ] {
            assert_eq!(compression_of(n), c);
        }
    }

    #[test]
    fn rejects() {
        // not ar
        assert!(parse(b"not a deb").is_none());
        // first member not debian-binary
        let mut a = b"!<arch>\n".to_vec();
        member(&mut a, "control.tar.xz", b"X");
        member(&mut a, "debian-binary", b"2.0\n");
        assert!(parse(&a).is_none());
        // no data.tar
        let mut b = b"!<arch>\n".to_vec();
        member(&mut b, "debian-binary", b"2.0\n");
        member(&mut b, "control.tar.gz", b"C");
        assert!(parse(&b).is_none());
    }
}
