//! Apple LZFSE compressed stream (`.lzfse`): a sequence of blocks,
//! each starting with a 4-byte magic — `bvx1`/`bvx2`/`bvxn`
//! (compressed v1/v2/vN), `bvx-` (uncompressed), `bvx$`
//! (end-of-stream). Compressed headers carry `u32le` raw and
//! payload sizes; the stream ends at `bvx$` or EOF.
//!
//! ```
//! let mut d = b"bvx-".to_vec();
//! d.extend_from_slice(&[4, 0, 0, 0]); // n_raw_bytes
//! d.extend_from_slice(b"data");
//! d.extend_from_slice(b"bvx$");
//! let p = izanagi_kit::lzfse::parse(&d).unwrap();
//! assert_eq!(p.blocks, 1);
//! assert!(p.eos);
//! assert!(izanagi_kit::lzfse::detect(&d));
//! ```

/// Census of an LZFSE stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Lzfse {
    /// Total blocks counted.
    pub blocks: u32,
    /// `bvx1` compressed-v1 blocks.
    pub v1_blocks: u32,
    /// `bvx2` compressed-v2 blocks.
    pub v2_blocks: u32,
    /// `bvxn` compressed-vN (LZVN-family) blocks.
    pub vn_blocks: u32,
    /// `bvx-` uncompressed blocks.
    pub raw_blocks: u32,
    /// `bvx$` end-of-stream marker seen.
    pub eos: bool,
    /// Sum of `n_raw_bytes` over blocks that carry the field.
    pub raw_bytes_total: u32,
    /// Sum of `n_payload_bytes` over compressed blocks.
    pub payload_bytes_total: u32,
    /// First block magic was not at offset 0, or a block magic
    /// appeared where none was expected.
    pub garbage: bool,
}

const MAGICS: [&[u8; 4]; 5] = [b"bvx1", b"bvx2", b"bvxn", b"bvx-", b"bvx$"];

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

fn magic_at(b: &[u8], i: usize) -> Option<&'static [u8; 4]> {
    MAGICS
        .iter()
        .find(|m| i + 4 <= b.len() && &b[i..i + 4] == **m)
        .copied()
}

/// `true` when the stream starts with a known block magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 4 && magic_at(b, 0).is_some()
}

/// Census; `None` without a leading block magic. Block payload
/// layout differs per version, so the walk scans for the next
/// magic word — counts are a marker census (documented as such).
#[must_use]
pub fn parse(b: &[u8]) -> Option<Lzfse> {
    if !detect(b) {
        return None;
    }
    let mut z = Lzfse {
        blocks: 0,
        v1_blocks: 0,
        v2_blocks: 0,
        vn_blocks: 0,
        raw_blocks: 0,
        eos: false,
        raw_bytes_total: 0,
        payload_bytes_total: 0,
        garbage: false,
    };
    let mut i = 0usize;
    while i + 4 <= b.len() {
        let Some(m) = magic_at(b, i) else {
            i += 1;
            continue;
        };
        z.blocks += 1;
        match m {
            b"bvx1" => {
                z.v1_blocks += 1;
                if i + 12 <= b.len() {
                    z.raw_bytes_total = z.raw_bytes_total.saturating_add(le32(b, i + 4));
                    z.payload_bytes_total = z.payload_bytes_total.saturating_add(le32(b, i + 8));
                }
            }
            b"bvx2" => {
                z.v2_blocks += 1;
                if i + 12 <= b.len() {
                    z.raw_bytes_total = z.raw_bytes_total.saturating_add(le32(b, i + 4));
                    z.payload_bytes_total = z.payload_bytes_total.saturating_add(le32(b, i + 8));
                }
            }
            b"bvxn" => {
                z.vn_blocks += 1;
                if i + 12 <= b.len() {
                    z.raw_bytes_total = z.raw_bytes_total.saturating_add(le32(b, i + 4));
                    z.payload_bytes_total = z.payload_bytes_total.saturating_add(le32(b, i + 8));
                }
            }
            b"bvx-" => {
                z.raw_blocks += 1;
                if i + 8 <= b.len() {
                    z.raw_bytes_total = z.raw_bytes_total.saturating_add(le32(b, i + 4));
                }
            }
            b"bvx$" => {
                z.eos = true;
                z.blocks -= 1; // EOS is a marker, not a block
                break;
            }
            _ => {}
        }
        i += 4;
    }
    if z.eos {
        // bytes after EOS that aren't another stream are garbage
        if i + 4 < b.len() && magic_at(b, i + 4).is_none() {
            z.garbage = b.len() - (i + 4) > 4;
        }
    }
    Some(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_sizes_saturate() {
        let z = parse(b"bvx-\xff\xff\xff\xffbvx-\xff\xff\xff\xff").unwrap();
        assert_eq!(z.raw_bytes_total, u32::MAX);
    }

    fn fixture() -> Vec<u8> {
        let mut d = b"bvx-".to_vec();
        d.extend_from_slice(&[4, 0, 0, 0]);
        d.extend_from_slice(b"data");
        let mut c = b"bvx2".to_vec();
        c.extend_from_slice(&[100, 0, 0, 0]); // raw
        c.extend_from_slice(&[50, 0, 0, 0]); // payload
        c.extend_from_slice(&[0; 50]);
        d.extend_from_slice(&c);
        d.extend_from_slice(b"bvx$");
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(detect(b"bvxn...."));
        assert!(!detect(b"bvx3"));
        assert!(!detect(b"lzfse"));
    }

    #[test]
    fn parses_blocks() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.blocks, 2);
        assert_eq!(p.raw_blocks, 1);
        assert_eq!(p.v2_blocks, 1);
        assert!(p.eos);
        assert_eq!(p.raw_bytes_total, 104);
        assert_eq!(p.payload_bytes_total, 50);
    }

    #[test]
    fn no_eos_stream() {
        let p = parse(b"bvx-\x04\0\0\0data").unwrap();
        assert_eq!(p.blocks, 1);
        assert!(!p.eos);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"not lzfse").is_none());
    }
}
