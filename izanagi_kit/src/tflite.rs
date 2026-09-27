//! TFLite — TensorFlow Lite model: a FlatBuffers file whose
//! 4-byte identifier at offset 4 is `TFL3`. Built on
//! `crate::flatbuf`.
//!
//! ```
//! use izanagi_kit::tflite;
//!
//! // root table at offset 8, identifier "TFL3"
//! let mut d = vec![0u8; 32];
//! d[0..4].copy_from_slice(&u32::to_le_bytes(8));
//! d[4..8].copy_from_slice(b"TFL3");
//! // root table: i32 vtable distance (say 4), then 2B table data
//! d[8..12].copy_from_slice(&i32::to_le_bytes(4));
//! d[4..8].copy_from_slice(b"TFL3");
//! // vtable at 8-4=4? put it at 4 would clobber ident — craft below
//! let d2 = vec![0u8; 8]; // simpler: identifier-only check
//! assert!(tflite::parse(&d2).is_none());
//! ```

use crate::flatbuf::{self, Flatbuf, Vtable};

/// The TFLite file identifier.
pub const IDENT: &[u8; 4] = b"TFL3";

/// A parsed TFLite buffer.
#[derive(Debug, Clone)]
pub struct Tflite {
    /// The underlying FlatBuffers head.
    pub flatbuf: Flatbuf,
}

/// Parse; requires the FlatBuffers head plus the `TFL3` identifier.
pub fn parse(d: &[u8]) -> Option<Tflite> {
    let fb = flatbuf::parse(d)?;
    if fb.ident.as_ref()? != IDENT {
        return None;
    }
    Some(Tflite { flatbuf: fb })
}

impl Tflite {
    /// Vtable descriptor of the root `Model` table.
    pub fn root_vtable(&self, d: &[u8]) -> Option<Vtable> {
        flatbuf::vtable(d, self.flatbuf.root)
    }

    /// u32 field `id` on the root table (`version` is field 0 in
    /// `tflite::Model` schema).
    pub fn root_u32(&self, d: &[u8], id: usize) -> Option<u32> {
        let vt = self.root_vtable(d)?;
        flatbuf::u32_field(d, self.flatbuf.root, &vt, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal flatbuf: [u32 root=8]["TFL3"][table: i32 vtdist=4 → vtable@8]
    // layout: 0..4 root offset, 4..8 ident, 8..12 table dist, 12.. vtable
    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&u32::to_le_bytes(12)); // root table at 12
        d.extend_from_slice(b"TFL3");
        // vtable at 8..12: vtable_len=6, table_len=4, field0 offset=4
        d.extend_from_slice(&u16::to_le_bytes(6));
        d.extend_from_slice(&u16::to_le_bytes(8));
        // table at 12: i32 = 4 (vtable at 12-4=8), then u32 version at 16
        d.extend_from_slice(&i32::to_le_bytes(4));
        d.extend_from_slice(&u32::to_le_bytes(3));
        d
    }

    #[test]
    fn ident_and_root() {
        let d = fixture();
        let t = parse(&d).unwrap();
        assert_eq!(t.flatbuf.ident, Some(*IDENT));
        let vt = t.root_vtable(&d).unwrap();
        assert_eq!(vt.vtable_len, 6);
        assert_eq!(t.root_u32(&d, 0), Some(3));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[4] = b'X';
        assert!(parse(&d).is_none());
    }
}
