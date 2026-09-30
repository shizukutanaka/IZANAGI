//! ZPAQ archive (`zpaq`, levels 1/2): stream begins `zPQ` + level
//! byte + memory byte, then blocks — each `h` starts a block with a
//! 13-byte HCOMP header (`hh hm ph pn n` + n comp bytes + `END`),
//! data is framed by `d`/`i` segment markers; ends with `i`.
//!
//! ```
//! let mut d = b"zPQ".to_vec();
//! d.extend_from_slice(&[2, 8]); // level 2, memory
//! d.extend_from_slice(b"h");
//! d.extend_from_slice(&[0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0]); // hcomp hdr
//! d.push(0); // END
//! d.extend_from_slice(b"i"); // end marker
//! let p = izanagi_kit::zpaq::parse(&d).unwrap();
//! assert_eq!(p.level, 2);
//! assert_eq!(p.blocks, 1);
//! assert!(izanagi_kit::zpaq::detect(&d));
//! ```

/// Census of a ZPAQ archive stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Zpaq {
    /// ZPAQ level (1 or 2).
    pub level: u8,
    /// Memory size exponent byte.
    pub mem_size: u8,
    /// `h` block-start markers seen.
    pub blocks: u32,
    /// `i` block-end markers seen.
    pub block_ends: u32,
    /// `d` data-segment markers seen.
    pub data_segments: u32,
    /// HCOMP headers parsed (13-byte + n comp bytes + END).
    pub hcomp_headers: u32,
    /// HCOMP `n` (component count) totals.
    pub hcomp_components: u32,
    /// Bytes consumed by walked structure.
    pub payload_len: u32,
    /// A block/header ran past the end.
    pub truncated: bool,
}

/// `true` on `zPQ` + level 1 or 2.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 5 && b[..3] == *b"zPQ" && (b[3] == 1 || b[3] == 2)
}

/// Census; `None` without `zPQ`. Walks `h`/`d`/`i` markers from
/// offset 5; a marker inside compressed payload may be miscounted,
/// so counts are a census approximation (documented, no decode).
#[must_use]
pub fn parse(b: &[u8]) -> Option<Zpaq> {
    if !detect(b) {
        return None;
    }
    let mut z = Zpaq {
        level: b[3],
        mem_size: b[4],
        blocks: 0,
        block_ends: 0,
        data_segments: 0,
        hcomp_headers: 0,
        hcomp_components: 0,
        payload_len: 0,
        truncated: false,
    };
    let mut i = 5usize;
    while i < b.len() {
        match b[i] {
            b'h' => {
                z.blocks += 1;
                // HCOMP header: hh hm ph pn n (5 bytes) + n bytes + END.
                if i + 6 <= b.len() {
                    let n = b[i + 5] as usize;
                    if i + 6 + n < b.len() && b[i + 6 + n] == 0 {
                        z.hcomp_headers += 1;
                        z.hcomp_components += n as u32;
                        z.payload_len += (6 + n + 1) as u32;
                        i += 6 + n + 1;
                        continue;
                    }
                }
                i += 1;
            }
            b'i' => {
                z.block_ends += 1;
                z.payload_len += 1;
                i += 1;
            }
            b'd' => {
                z.data_segments += 1;
                z.payload_len += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if z.block_ends < z.blocks {
        z.truncated = true;
    }
    Some(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"zPQ".to_vec();
        d.extend_from_slice(&[2, 8]);
        d.extend_from_slice(b"h");
        // hh hm ph pn n=2 + 2 comp bytes + END
        d.extend_from_slice(&[0, 0, 1, 1, 2, 0xAA, 0xBB, 0]);
        d.extend_from_slice(b"d");
        d.extend_from_slice(&[9, 9, 9]); // junk payload bytes
        d.extend_from_slice(b"i");
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"zPQ3"));
        assert!(!detect(b"zpaq level 2"));
    }

    #[test]
    fn parses_stream() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.level, 2);
        assert_eq!(p.blocks, 1);
        assert_eq!(p.hcomp_headers, 1);
        assert_eq!(p.hcomp_components, 2);
        assert_eq!(p.data_segments, 1);
        assert_eq!(p.block_ends, 1);
        assert!(!p.truncated);
    }

    #[test]
    fn missing_end_truncated() {
        let mut d = fixture();
        d.pop(); // drop the 'i'
        let p = parse(&d).unwrap();
        assert!(p.truncated);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"not zpaq").is_none());
    }
}
