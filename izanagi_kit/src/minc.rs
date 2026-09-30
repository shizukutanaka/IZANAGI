//! MINC medical imaging container detection (McGill BIC):
//! MINC 1 files are NetCDF classic (`CDF\x01` 32-bit offsets or
//! `CDF\x02` 64-bit, or `CDF\x05` "classic model" HDF5-backed);
//! MINC 2 files are HDF5 (`\x89HDF\r\n\x1a\n`). A MINC file also
//! carries `MI` root-group attributes in its variables — detected by
//! a `minc`/`MI` signature string in the header block when present.
//!
//! ```
//! let mut d = b"CDF\x01".to_vec();
//! d.extend_from_slice(&[0u8; 12]); // numrecs + dim list
//! d.extend_from_slice(b"minc-attributes");
//! let m = izanagi_kit::minc::parse(&d).unwrap();
//! assert_eq!(m.variant, izanagi_kit::minc::Variant::Minc1);
//! ```

/// Which MINC generation a file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// MINC 1 — NetCDF classic container.
    Minc1,
    /// MINC 2 — HDF5 container.
    Minc2,
}

/// A detected MINC file.
#[derive(Clone, Debug)]
pub struct Minc {
    /// MINC generation.
    pub variant: Variant,
    /// NetCDF format byte for MINC 1 (`CDF\x01`/`CDF\x02`/`CDF\x05`);
    /// HDF5 superblock version for MINC 2. `None` when not readable.
    pub format_byte: Option<u8>,
    /// Whether the header region mentions `minc`/`MI` attributes.
    pub minc_attrs_seen: bool,
}

/// Detect a MINC file; `None` for anything that is neither a NetCDF
/// classic header nor an HDF5 signature, or that lacks MINC markers.
pub fn parse(d: &[u8]) -> Option<Minc> {
    // HDF5 → MINC 2
    if d.len() >= 8 && &d[..8] == b"\x89HDF\r\n\x1a\n" {
        // superblock version at offset 8 for v0/v1, at 16? we read 8
        let ver = d.get(8).copied();
        let minc_attrs_seen = scan_for_minc(d);
        if !minc_attrs_seen {
            return None;
        }
        return Some(Minc {
            variant: Variant::Minc2,
            format_byte: ver,
            minc_attrs_seen,
        });
    }
    // NetCDF classic → MINC 1
    if d.len() >= 4 && &d[..3] == b"CDF" && matches!(d[3], 0x01 | 0x02 | 0x05) {
        let minc_attrs_seen = scan_for_minc(d);
        if !minc_attrs_seen {
            return None;
        }
        return Some(Minc {
            variant: Variant::Minc1,
            format_byte: Some(d[3]),
            minc_attrs_seen,
        });
    }
    None
}

fn scan_for_minc(d: &[u8]) -> bool {
    // MINC files name variables like `image`, attributes `MI*`; look
    // for a plausible marker within the first 4 KiB.
    let end = d.len().min(4096);
    let hay = &d[..end];
    hay.windows(4).any(|w| w == b"minc")
        || hay
            .windows(3)
            .any(|w| &w[..2] == b"MI" && w[2].is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_netcdf() {
        let mut d = b"CDF\x01".to_vec();
        d.extend_from_slice(&[0u8; 24]);
        d.extend_from_slice(b"MIimage variable");
        let m = parse(&d).unwrap();
        assert_eq!(m.variant, Variant::Minc1);
        assert_eq!(m.format_byte, Some(1));
        assert!(m.minc_attrs_seen);
    }

    #[test]
    fn v2_hdf5() {
        let mut d = b"\x89HDF\r\n\x1a\n".to_vec();
        d.push(0); // superblock version
        d.extend_from_slice(&[0u8; 32]);
        d.extend_from_slice(b"minc-contents");
        let m = parse(&d).unwrap();
        assert_eq!(m.variant, Variant::Minc2);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"CDF\x03xxxx").is_none()); // bad version
                                                  // NetCDF without any MINC marker
        let mut d = b"CDF\x01".to_vec();
        d.extend_from_slice(&[0u8; 64]);
        assert!(parse(&d).is_none());
        // HDF5 without marker
        let mut d = b"\x89HDF\r\n\x1a\n".to_vec();
        d.extend_from_slice(&[0u8; 64]);
        assert!(parse(&d).is_none());
    }
}
