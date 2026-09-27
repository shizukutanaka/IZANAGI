//! BLEND — the Blender scene file. Byte 0 carries `BLENDER` followed
//! by a pointer-size flag (`_` = 4, `-` = 8), an endianness flag
//! (`v` = little, `V` = big) and three ASCII version digits. The rest
//! of the file is a chain of blocks `{4-byte code, u32 size,
//! old_memory_address:ptr, sdna_index:u32, count:u32}` in the header's
//! endianness, ending at an `ENDB` block; the `DNA1` block holds the
//! structural DNA used to interpret `sdna_index`.
//!
//! ```
//! use izanagi_kit::blend::{parse, blocks};
//! let mut d = b"BLENDER_v280".to_vec();
//! for (code, size) in [(b"REND", 4u32), (b"ENDB", 0u32)] {
//!     d.extend_from_slice(code);
//!     d.extend_from_slice(&size.to_le_bytes());
//!     d.extend_from_slice(&[0; 4]);          // old mem addr (32-bit)
//!     d.extend_from_slice(&0u32.to_le_bytes()); // sdna index
//!     d.extend_from_slice(&1u32.to_le_bytes()); // count
//!     d.extend(std::iter::repeat(0).take(size as usize));
//! }
//! let b = parse(&d).unwrap();
//! assert_eq!(b.version, *b"280");
//! assert_eq!(blocks(&b, &d).count(), 2);
//! ```

/// A parsed `BLENDER` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blend {
    /// Three ASCII version digits, e.g. `b"280"`.
    pub version: [u8; 3],
    /// True when block fields are big-endian (`V` flag).
    pub big_endian: bool,
    /// Pointer width the file was written with (4 or 8).
    pub ptr_size: usize,
}

fn u32e(b: &Blend, d: &[u8], at: usize) -> Option<u32> {
    let raw: [u8; 4] = d.get(at..at + 4)?.try_into().ok()?;
    Some(if b.big_endian {
        // big-endian .blend — folded by hand so no *ne/*be helper is used
        (u32::from(raw[0]) << 24)
            | (u32::from(raw[1]) << 16)
            | (u32::from(raw[2]) << 8)
            | u32::from(raw[3])
    } else {
        u32::from_le_bytes(raw)
    })
}

/// Parse the 12-byte header; `None` without `BLENDER` or valid flags.
pub fn parse(d: &[u8]) -> Option<Blend> {
    if d.get(..7)? != b"BLENDER" {
        return None;
    }
    let ptr_size = match *d.get(7)? {
        b'_' => 4,
        b'-' => 8,
        _ => return None,
    };
    let big_endian = match *d.get(8)? {
        b'v' => false,
        b'V' => true,
        _ => return None,
    };
    let version: [u8; 3] = d.get(9..12)?.try_into().ok()?;
    if !version.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(Blend {
        version,
        big_endian,
        ptr_size,
    })
}

/// One block header (addresses omitted — they carry no order info).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Four-character block code, e.g. `b"ENDB"`, `b"DNA1"`, `b"OB  "`.
    pub code: [u8; 4],
    /// Byte offset of the block header itself.
    pub at: usize,
    /// Declared payload size in bytes.
    pub size: u32,
    /// Index into the structural DNA (meaningless before `DNA1`).
    pub sdna_index: u32,
    /// Number of structures the payload contains.
    pub count: u32,
}

/// Byte offset where block headers begin.
pub const BLOCKS_AT: usize = 12;

/// Iterate block headers until `ENDB` or the data runs out.
pub fn blocks<'d>(b: &'d Blend, d: &'d [u8]) -> impl Iterator<Item = Block> + 'd {
    let head = 16 + b.ptr_size;
    let mut at = BLOCKS_AT;
    let mut done = false;
    core::iter::from_fn(move || {
        if done {
            return None;
        }
        let code: [u8; 4] = d.get(at..at + 4)?.try_into().ok()?;
        let size = u32e(b, d, at + 4)?;
        let sdna_index = u32e(b, d, at + 8 + b.ptr_size)?;
        let count = u32e(b, d, at + 12 + b.ptr_size)?;
        let blk = Block {
            code,
            at,
            size,
            sdna_index,
            count,
        };
        at = at.checked_add(head)?.checked_add(size as usize)?;
        done = blk.code == *b"ENDB";
        Some(blk)
    })
}

/// Byte offset of the `DNA1` block's payload, if present.
pub fn dna_at(b: &Blend, d: &[u8]) -> Option<usize> {
    blocks(b, d)
        .find(|bl| bl.code == *b"DNA1")
        .map(|bl| bl.at + 16 + b.ptr_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"BLENDER_v280".to_vec();
        for (code, size, sdna) in [(b"REND", 4u32, 0u32), (b"DNA1", 8, 1), (b"ENDB", 0, 0)] {
            d.extend_from_slice(code);
            d.extend_from_slice(&size.to_le_bytes());
            d.extend_from_slice(&[0; 4]);
            d.extend_from_slice(&sdna.to_le_bytes());
            d.extend_from_slice(&1u32.to_le_bytes());
            d.extend(std::iter::repeat(0).take(size as usize));
        }
        d
    }

    #[test]
    fn parses_header_and_blocks() {
        let d = fixture();
        let b = parse(&d).unwrap();
        assert_eq!(b.version, *b"280");
        assert!(!b.big_endian && b.ptr_size == 4);
        let bl: Vec<_> = blocks(&b, &d).collect();
        assert_eq!(bl.len(), 3);
        assert_eq!(bl[0].code, *b"REND");
        assert_eq!(bl[0].at, BLOCKS_AT);
        assert_eq!(bl[1].sdna_index, 1);
        assert_eq!(bl[2].code, *b"ENDB");
        assert!(dna_at(&b, &d).is_some());
    }

    #[test]
    fn big_endian_and_64bit() {
        let mut d = b"BLENDER-V283".to_vec();
        d.extend_from_slice(b"ENDB");
        d.extend_from_slice(&[0; 4]); // size
        d.extend_from_slice(&[0; 8]); // old mem addr (64-bit)
        d.extend_from_slice(&[0; 4]); // sdna
        d.extend_from_slice(&[0, 0, 0, 1]); // count = 1, big-endian
        let b = parse(&d).unwrap();
        assert!(b.big_endian && b.ptr_size == 8);
        assert_eq!(blocks(&b, &d).count(), 1);
        assert!(dna_at(&b, &d).is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"BLENDX_v280").is_none());
        let mut d = fixture();
        d[7] = b'x';
        assert!(parse(&d).is_none());
        d[7] = b'_';
        d[10] = b'z';
        assert!(parse(&d).is_none());
        // truncated block → iterator just stops
        assert_eq!(
            blocks(
                &parse(&d[..16]).unwrap_or(parse(&fixture()).unwrap()),
                &d[..16]
            )
            .count(),
            0
        );
    }
}
