//! DirectX Bytecode (`.dxbc`/`.cso`, DX10+): `DXBC` magic, 16-byte
//! container hash, `u32le` total size, then `u32le` chunk count and
//! that many `u32le` chunk offsets; each chunk is a `FOURCC` +
//! `u32le` size + payload (`SHDR`/`SHEX`/`ISGN`/`OSGN`/`PSV0`/`DXIL`…).
//!
//! ```
//! let mut d = b"DXBC".to_vec();
//! d.extend_from_slice(&[0xAB; 16]); // hash
//! d.extend_from_slice(&[1, 0, 0, 0]); // container version
//! d.extend_from_slice(&[0; 4]); // size (filled by tools)
//! d.extend_from_slice(&[1, 0, 0, 0]); // 1 chunk
//! d.extend_from_slice(&[44, 0, 0, 0]); // offset
//! d.extend_from_slice(b"SHDR");
//! d.extend_from_slice(&[8, 0, 0, 0]); // payload size
//! d.extend_from_slice(&[0x50, 0, 0, 0, 6, 0, 0, 0]); // ps_5_0
//! let p = izanagi_kit::dxbc::parse(&d).unwrap();
//! assert_eq!(p.chunks, 1);
//! assert!(izanagi_kit::dxbc::detect(&d));
//! ```

/// Census of a DXBC container.
#[derive(Debug, Clone, PartialEq)]
pub struct Dxbc {
    /// `u32` after the hash (container version marker, usually 1).
    pub one: u32,
    /// Declared total file size.
    pub total_size: u32,
    /// Declared chunk count.
    pub chunks: u32,
    /// Chunk headers actually readable inside the buffer.
    pub resolved_chunks: u32,
    /// `SHDR`/`SHEX` shader chunks.
    pub shader_chunks: u32,
    /// `DXIL` chunks (DXIL bitcode payload).
    pub dxil_chunks: u32,
    /// `ISGN`/`OSGN`/`PCSG` signature chunks.
    pub signature_chunks: u32,
    /// `RDEF`/`RD11`/`SFI0` reflection chunks.
    pub reflection_chunks: u32,
    /// `STAT` statistics chunks.
    pub stat_chunks: u32,
    /// `PSV0`/`PSV1` pipeline-state-validation chunks.
    pub psv_chunks: u32,
    /// Other FOURCC chunks.
    pub other_chunks: u32,
    /// A chunk offset or size pointed outside the buffer.
    pub truncated: bool,
}

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on the `DXBC` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 32 && b[..4] == *b"DXBC"
}

/// Census; `None` without `DXBC`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dxbc> {
    if !detect(b) {
        return None;
    }
    let mut x = Dxbc {
        one: le32(b, 20),
        total_size: le32(b, 24),
        chunks: le32(b, 28),
        resolved_chunks: 0,
        shader_chunks: 0,
        dxil_chunks: 0,
        signature_chunks: 0,
        reflection_chunks: 0,
        stat_chunks: 0,
        psv_chunks: 0,
        other_chunks: 0,
        truncated: false,
    };
    let n = x.chunks as usize;
    if n > 4096 || 32usize.checked_add(n * 4).map_or(true, |e| e > b.len()) {
        x.truncated = true;
        return Some(x);
    }
    for c in 0..n {
        let off = le32(b, 32 + c * 4) as usize;
        if off + 8 > b.len() {
            x.truncated = true;
            continue;
        }
        let fourcc = &b[off..off + 4];
        let size = le32(b, off + 4) as usize;
        if off + 8 + size > b.len() {
            x.truncated = true;
        }
        match fourcc {
            b"SHDR" | b"SHEX" => x.shader_chunks += 1,
            b"DXIL" => x.dxil_chunks += 1,
            b"ISGN" | b"OSGN" | b"PCSG" | b"OSG1" | b"ISG1" => x.signature_chunks += 1,
            b"RDEF" | b"RD11" | b"SFI0" => x.reflection_chunks += 1,
            b"STAT" => x.stat_chunks += 1,
            b"PSV0" | b"PSV1" | b"PSV2" => x.psv_chunks += 1,
            _ => x.other_chunks += 1,
        }
        x.resolved_chunks += 1;
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"DXBC".to_vec();
        d.extend_from_slice(&[0xAB; 16]);
        d.extend_from_slice(&[1, 0, 0, 0]);
        d.extend_from_slice(&[0; 4]); // size
        d.extend_from_slice(&[2, 0, 0, 0]); // 2 chunks
        let off1 = 32 + 8;
        d.extend_from_slice(&(off1 as u32).to_le_bytes());
        d.extend_from_slice(&((off1 + 16) as u32).to_le_bytes());
        d.extend_from_slice(b"SHDR");
        d.extend_from_slice(&[8, 0, 0, 0]);
        d.extend_from_slice(&[0x50, 0, 0, 0, 6, 0, 0, 0]);
        d.extend_from_slice(b"STAT");
        d.extend_from_slice(&[4, 0, 0, 0]);
        d.extend_from_slice(&[0; 4]);
        let total = d.len() as u32;
        d[24..28].copy_from_slice(&total.to_le_bytes());
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"DXIL bitcode"));
        assert!(!detect(&fixture()[..16]));
    }

    #[test]
    fn parses_chunks() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.chunks, 2);
        assert_eq!(p.resolved_chunks, 2);
        assert_eq!(p.shader_chunks, 1);
        assert_eq!(p.stat_chunks, 1);
        assert!(!p.truncated);
    }

    #[test]
    fn bad_offset_flagged() {
        let mut d = fixture();
        d[32] = 0xFF; // chunk 0 offset → out of range
        let p = parse(&d).unwrap();
        assert!(p.truncated);
        assert_eq!(p.resolved_chunks, 1);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"dxbc-ish").is_none());
    }
}
