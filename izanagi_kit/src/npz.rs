//! NPZ — NumPy's zipped `.npy` bundle: a `PK` zip container whose
//! members each hold a `.npy` array.
//!
//! ```
//! use izanagi_kit::{npz, npy};
//!
//! // Build one stored `.npy` member via the kit zip writer.
//! let mut n = b"\x93NUMPY".to_vec();
//! n.push(1); n.push(0);
//! let h = b"{'descr': '<i8', 'fortran_order': False, 'shape': (2,), }";
//! let mut hdr = Vec::from(&h[..]);
//! let pad = 16 - ((10 + hdr.len()) % 16);
//! hdr.extend(std::iter::repeat(b' ').take(pad - 1));
//! hdr.push(b'\n');
//! n.extend_from_slice(&u16::to_le_bytes(hdr.len() as u16));
//! n.extend_from_slice(&hdr);
//! n.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
//!
//! let mut w = izanagi_kit::zip::ZipWriter::new();
//! w.add_stored("arr_0.npy", &n);
//! let z = w.finish();
//!
//! let e: Vec<_> = npz::arrays(&z).unwrap();
//! assert_eq!(e.len(), 1);
//! assert_eq!(e[0].0, "arr_0");
//! assert_eq!(e[0].1.shape, vec![2]);
//! ```

use crate::{npy, zip};
use std::string::String;
use std::vec::Vec;

/// Iterate the members as `(basename, npy header)` pairs.
/// Entries that fail to extract or aren't valid `.npy` are skipped;
/// `None` only when the zip itself doesn't list.
pub fn arrays(d: &[u8]) -> Option<Vec<(String, npy::Npy)>> {
    let entries = zip::list(d)?;
    let mut out = Vec::new();
    for e in &entries {
        let Some(name) = e.name.strip_suffix(".npy") else {
            continue;
        };
        let Some(raw) = zip::extract(d, &e.name) else {
            continue;
        };
        let Some(n) = npy::parse(&raw) else {
            continue;
        };
        out.push((String::from(name), n));
    }
    Some(out)
}

/// Extract one array's raw data bytes (decompressing the member).
pub fn array_data(d: &[u8], name: &str) -> Option<Vec<u8>> {
    let file = if name.ends_with(".npy") {
        String::from(name)
    } else {
        format!("{name}.npy")
    };
    let raw = zip::extract(d, &file)?;
    let n = npy::parse(&raw)?;
    let lo = n.data_at;
    let hi = usize::try_from(n.data_len()?).ok()?.checked_add(lo)?;
    raw.get(lo..hi).map(|s| s.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn npy_bytes() -> Vec<u8> {
        let mut n = b"\x93NUMPY".to_vec();
        n.push(1);
        n.push(0);
        let h = b"{'descr': '<i8', 'fortran_order': False, 'shape': (4,), }";
        let mut hdr = Vec::from(&h[..]);
        let pad = 16 - ((10 + hdr.len()) % 16);
        hdr.extend(std::iter::repeat(b' ').take(pad - 1));
        hdr.push(b'\n');
        n.extend_from_slice(&u16::to_le_bytes(hdr.len() as u16));
        n.extend_from_slice(&hdr);
        n.extend_from_slice(&[
            9, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        n
    }

    #[test]
    fn round_trip() {
        let mut w = zip::ZipWriter::new();
        w.add_stored("a.npy", &npy_bytes());
        w.add_stored("b.npy", &npy_bytes());
        w.add_stored("README.txt", b"hi");
        let z = w.finish();
        let e = arrays(&z).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].0, "a");
        assert_eq!(e[0].1.shape, vec![4]);
        assert_eq!(e[0].1.descr, "<i8");
        assert_eq!(array_data(&z, "a").unwrap().len(), 32);
        assert!(array_data(&z, "missing").is_none());
        assert!(arrays(b"not a zip").is_none());
    }
}
