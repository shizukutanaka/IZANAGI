//! Snappy framed format — the container around the `snappy` block
//! codec. A stream opens with the stream-identifier chunk: type
//! `0xFF`, length 6, payload `sNaPpY`. Every chunk is `{u8 type,
//! u24LE len}` where `len` covers the 4-byte masked CRC32C plus data;
//! `0x00` is compressed data, `0x01` uncompressed, `0x02` padding,
//! `0x80`..=`0xFE` skippable user chunks. The CRC is stored masked:
//! `masked = rot right 15 of (crc + 0xA282EAD8)`.
//!
//! ```
//! use izanagi_kit::snappy::{parse, chunks, unmask};
//! let mut d = vec![0xFF, 0x06, 0x00, 0x00];
//! d.extend_from_slice(b"sNaPpY");
//! d.extend_from_slice(&[0x01, 0x08, 0x00, 0x00]); // uncompressed, len 8
//! d.extend_from_slice(&0x12345678u32.to_le_bytes());
//! d.extend_from_slice(b"data");
//! assert!(parse(&d));
//! let c = chunks(&d).next().unwrap();
//! assert_eq!(c.kind, 0x01);
//! assert_eq!(unmask(0x4F730F40), 0x12345678);
//! ```

/// Stream identifier chunk: type byte, then `sNaPpY`.
pub const IDENT_TYPE: u8 = 0xFF;
/// The stream-identifier payload.
pub const IDENT: &[u8; 6] = b"sNaPpY";
/// Compressed-data chunk.
pub const COMPRESSED: u8 = 0x00;
/// Uncompressed-data chunk.
pub const UNCOMPRESSED: u8 = 0x01;
/// Padding chunk.
pub const PADDING: u8 = 0x02;
/// First skippable user-chunk type.
pub const SKIPPABLE_MIN: u8 = 0x80;
/// Last skippable user-chunk type.
pub const SKIPPABLE_MAX: u8 = 0xFE;

/// One chunk header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// Chunk type byte.
    pub kind: u8,
    /// Byte offset of the chunk header.
    pub at: usize,
    /// Declared length — CRC plus data — in bytes.
    pub len: u32,
}

impl Chunk {
    /// True for a skippable chunk (`0x80`..=`0xFE`, and padding).
    pub fn skippable(&self) -> bool {
        self.kind == PADDING || (SKIPPABLE_MIN..=SKIPPABLE_MAX).contains(&self.kind)
    }

    /// Byte offset of this chunk's masked CRC32C.
    pub fn crc_at(&self) -> usize {
        self.at + 4
    }

    /// Byte offset of the payload after the CRC.
    pub fn data_at(&self) -> usize {
        self.at + 8
    }
}

fn u24l(d: &[u8], at: usize) -> Option<u32> {
    let r = d.get(at..at + 3)?;
    Some(u32::from(r[0]) | (u32::from(r[1]) << 8) | (u32::from(r[2]) << 16))
}

/// Check the mandatory stream-identifier chunk.
pub fn parse(d: &[u8]) -> bool {
    d.get(..4) == Some(&[IDENT_TYPE, 0x06, 0x00, 0x00][..]) && d.get(4..10) == Some(&IDENT[..])
}

/// Iterate chunk headers after the identifier (first chunk at 10).
pub fn chunks(d: &[u8]) -> impl Iterator<Item = Chunk> + '_ {
    let mut at = 10usize;
    core::iter::from_fn(move || {
        let kind = *d.get(at)?;
        let len = u24l(d, at + 1)?;
        let c = Chunk { kind, at, len };
        at = at.checked_add(4 + len as usize)?;
        Some(c)
    })
}

/// The CRC mask constant.
pub const MASK_DELTA: u32 = 0xA282_EAD8;

/// Mask a CRC32C for storage in the chunk.
pub fn mask(crc: u32) -> u32 {
    crc.rotate_right(15).wrapping_add(MASK_DELTA)
}

/// Recover the CRC32C from a masked chunk value.
pub fn unmask(masked: u32) -> u32 {
    masked.wrapping_sub(MASK_DELTA).rotate_left(15)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![IDENT_TYPE, 0x06, 0x00, 0x00];
        d.extend_from_slice(IDENT);
        // compressed chunk: len = 4(crc) + 3(data)
        d.extend_from_slice(&[COMPRESSED, 0x07, 0x00, 0x00]);
        d.extend_from_slice(&mask(0x9999).to_le_bytes());
        d.extend_from_slice(b"xyz");
        // skippable chunk
        d.extend_from_slice(&[0x90, 0x04, 0x00, 0x00]);
        d.extend_from_slice(&[0; 4]);
        d
    }

    #[test]
    fn parses_stream_id() {
        assert!(parse(&fixture()));
        assert!(!parse(b""));
        assert!(!parse(b"\xFF\x06\x00\x00sNaPpZ")); // bad ident payload
        assert!(!parse(b"\xFE\x06\x00\x00sNaPpY")); // wrong type
    }

    #[test]
    fn chunk_walk() {
        let d = fixture();
        let cs: Vec<_> = chunks(&d).collect();
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].kind, COMPRESSED);
        assert_eq!(cs[0].at, 10);
        assert_eq!(cs[0].len, 7);
        assert_eq!(cs[0].crc_at(), 14);
        assert_eq!(cs[0].data_at(), 18);
        assert!(cs[1].skippable());
        assert_eq!(cs[1].kind, 0x90);
    }

    #[test]
    fn crc_mask_roundtrip() {
        let crc = 0x1234_5678u32;
        assert_eq!(unmask(mask(crc)), crc);
        assert_eq!(mask(crc), 0x4F73_0F40);
    }
}
