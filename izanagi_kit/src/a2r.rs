//! A2R (Applesauce floppy flux image, A2R2/A2R3):
//! signature `A2Rx` + `0xFF 0x0A 0x0D 0x0A`, then
//! `ID u32le` chunks — `INFO` (version, disk type,
//! write-protect, 32-byte location), `STRM` (flux /
//! nibble / bitstream captures), `META`, `RWCP`,
//! `SLVD`, `SELV`.
//!
//! ```
//! let mut d = b"A2R3\xFF\x0A\x0D\x0A".to_vec();
//! d.extend_from_slice(b"INFO"); d.extend_from_slice(&[37, 0, 0, 0]);
//! d.extend_from_slice(&[3, 1, 1]); // ver, 5.25", wp
//! d.extend_from_slice(&[0; 34]); // location pad
//! d.extend_from_slice(b"STRM"); d.extend_from_slice(&[11, 0, 0, 0]);
//! d.extend_from_slice(&[0, 1]); // slot, type = flux
//! d.extend_from_slice(&[0; 9]); // est size + transfer time + data
//! let p = izanagi_kit::a2r::parse(&d).unwrap();
//! assert_eq!(p.version, 3);
//! assert_eq!(p.streams, 1);
//! assert_eq!(p.flux_streams, 1);
//! assert!(izanagi_kit::a2r::detect(&d));
//! ```

/// Census of an A2R flux image.
#[derive(Debug, Clone, PartialEq)]
pub struct A2r {
    /// Signature version (2 or 3).
    pub version: u8,
    /// `INFO` info version byte.
    pub info_version: u8,
    /// `INFO` disk type (1 = 5.25", 2 = 3.5").
    pub disk_type: u8,
    /// `INFO` write-protected flag.
    pub write_protected: u8,
    /// Chunks walked after the 8-byte signature.
    pub chunks: u32,
    /// `STRM` stream blocks.
    pub streams: u32,
    /// `STRM` blocks with type 1 (raw flux).
    pub flux_streams: u32,
    /// `STRM` blocks with type 2 (nibble).
    pub nibble_streams: u32,
    /// `STRM` blocks with type 3 (bitstream).
    pub bitstream_streams: u32,
    /// `META` chunk present.
    pub has_meta: bool,
    /// `RWCP` (real write control points) present.
    pub has_rwcp: bool,
    /// `SLVD` / `SELV` solved chunks seen.
    pub solved_chunks: u32,
    /// Sum of all `STRM` payload bytes.
    pub data_len: u32,
    /// A chunk overran the end of the buffer.
    pub truncated: bool,
}

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on the `A2Rx` signature plus integrity bytes.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8
        && b[0] == b'A'
        && b[1] == b'2'
        && b[2] == b'R'
        && (b[3] == b'2' || b[3] == b'3')
        && b[4] == 0xFF
        && b[5] == 0x0A
        && b[6] == 0x0D
        && b[7] == 0x0A
}

/// Census; `None` without the A2R signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<A2r> {
    if !detect(b) {
        return None;
    }
    let mut a = A2r {
        version: b[3] - b'0',
        info_version: 0,
        disk_type: 0,
        write_protected: 0,
        chunks: 0,
        streams: 0,
        flux_streams: 0,
        nibble_streams: 0,
        bitstream_streams: 0,
        has_meta: false,
        has_rwcp: false,
        solved_chunks: 0,
        data_len: 0,
        truncated: false,
    };
    let mut i = 8usize;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let size = le32(b, i + 4) as usize;
        if i + 8 + size > b.len() {
            a.truncated = true;
            break;
        }
        match id {
            b"INFO" => {
                if size >= 3 {
                    a.info_version = b[i + 8];
                    a.disk_type = b[i + 9];
                    a.write_protected = b[i + 10];
                }
            }
            b"STRM" => {
                a.streams += 1;
                a.data_len += size as u32;
                if size >= 2 {
                    match b[i + 9] {
                        1 => a.flux_streams += 1,
                        2 => a.nibble_streams += 1,
                        3 => a.bitstream_streams += 1,
                        _ => {}
                    }
                }
            }
            b"META" => a.has_meta = true,
            b"RWCP" => a.has_rwcp = true,
            b"SLVD" | b"SELV" => a.solved_chunks += 1,
            _ => {}
        }
        a.chunks += 1;
        i += 8 + size;
    }
    if i != b.len() {
        a.truncated = a.truncated || b.len() - i >= 8;
    }
    Some(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"A2R2\xFF\x0A\x0D\x0A".to_vec();
        d.extend_from_slice(b"INFO");
        d.extend_from_slice(&[37, 0, 0, 0]);
        d.extend_from_slice(&[2, 1, 0]);
        d.extend_from_slice(&[0; 34]);
        d.extend_from_slice(b"STRM");
        d.extend_from_slice(&[14, 0, 0, 0]);
        d.extend_from_slice(&[1, 2]);
        d.extend_from_slice(&[0; 12]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"A2R1\xFF\x0A\x0D\x0A"));
        assert!(!detect(b"a2r2"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, 2);
        assert_eq!(p.info_version, 2);
        assert_eq!(p.disk_type, 1);
        assert_eq!(p.streams, 1);
        assert_eq!(p.nibble_streams, 1);
        assert_eq!(p.data_len, 14);
        assert_eq!(p.chunks, 2);
        assert!(!p.truncated);
    }

    #[test]
    fn truncated_chunk() {
        let mut d = b"A2R3\xFF\x0A\x0D\x0A".to_vec();
        d.extend_from_slice(b"STRM");
        d.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        let p = parse(&d).unwrap();
        assert!(p.truncated);
        assert_eq!(p.streams, 0);
    }
}
