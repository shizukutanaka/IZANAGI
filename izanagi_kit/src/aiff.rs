//! AIFF / AIFC (Audio Interchange File Format) — the `FORM`/`COMM`/
//! `SSND` chunk structure. Sample rate is an 80-bit IEEE-754
//! *extended* float, decoded by hand into `Fixed` (no float types).
//!
//! ```
//! // 1-channel 8-bit mono at 8000 Hz, 3 frames of data.
//! let mut a = Vec::new();
//! a.extend_from_slice(b"FORM");
//! a.extend_from_slice(&(46u32).to_be_bytes()); // remaining bytes
//! a.extend_from_slice(b"AIFF");
//! a.extend_from_slice(b"COMM");
//! a.extend_from_slice(&(18u32).to_be_bytes());
//! a.extend_from_slice(&[0, 1]); // channels
//! a.extend_from_slice(&[0, 0, 0, 3]); // frames
//! a.extend_from_slice(&[0, 8]); // bits
//! a.extend_from_slice(&[0x40, 0x0B, 0xFA, 0x00, 0, 0, 0, 0, 0, 0]); // 8000
//! a.extend_from_slice(b"SSND");
//! a.extend_from_slice(&(11u32).to_be_bytes());
//! a.extend_from_slice(&[0; 8]); // offset, block
//! a.extend_from_slice(&[0x10, 0x80, 0xFF]); // 3 samples
//! let f = izanagi_kit::aiff::parse(&a).unwrap();
//! let c = izanagi_kit::aiff::comm(&a, &f).unwrap();
//! assert_eq!(c.channels, 1);
//! assert_eq!(izanagi_kit::aiff::ssnd(&a, &f).unwrap(), &[0x10, 0x80, 0xFF]);
//! ```

/// A chunk reference.
#[derive(Debug)]
pub struct Chunk {
    /// 4-byte type tag.
    pub id: [u8; 4],
    /// Offset of the chunk data.
    pub at: usize,
    /// Declared chunk size.
    pub size: usize,
}

/// The `COMM` chunk fields.
#[derive(Debug)]
pub struct Comm {
    /// Channel count.
    pub channels: u16,
    /// Total sample frames.
    pub frames: u32,
    /// Bits per sample.
    pub bits: u16,
    /// Sample rate in Hz (`Fixed` raw Q16.16 — 44100 → `Fixed` value
    /// of 44100.0).
    pub rate: i64,
    /// AIFC compression tag (`NONE` when plain AIFF).
    pub comp: [u8; 4],
}

/// A parsed AIFF file.
#[derive(Debug)]
pub struct Aiff {
    /// `true` for `AIFC`, `false` for `AIFF`.
    pub aifc: bool,
    /// Chunks in file order (type tag + data range).
    pub chunks: Vec<Chunk>,
}

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(((*d.get(at)? as u16) << 8) | *d.get(at + 1)? as u16)
}
fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        ((*d.get(at)? as u32) << 24)
            | ((*d.get(at + 1)? as u32) << 16)
            | ((*d.get(at + 2)? as u32) << 8)
            | (*d.get(at + 3)? as u32),
    )
}

/// Decodes an IEEE-754 80-bit *extended* float into `Fixed` raw
/// (Q16.16). Denormals/NaN/Inf → `None`.
pub fn extended80(d: &[u8]) -> Option<i64> {
    if d.len() < 10 {
        return None;
    }
    let sign = d[0] & 0x80 != 0;
    let exp = be16(d, 0)? & 0x7FFF;
    let mant = (d[2] as u64) << 56
        | (d[3] as u64) << 48
        | (d[4] as u64) << 40
        | (d[5] as u64) << 32
        | (d[6] as u64) << 24
        | (d[7] as u64) << 16
        | (d[8] as u64) << 8
        | d[9] as u64;
    if exp == 0x7FFF {
        return None; // inf/nan
    }
    if exp == 0 && mant == 0 {
        return Some(0);
    }
    if exp == 0 {
        return None; // denormal — treat as malformed
    }
    // value = mant * 2^(exp - 16383 - 63); raw = value * 2^16
    let shift = exp as i32 - 16383 - 63 + 16;
    let raw: i128 = if shift >= 0 {
        if shift > 60 {
            return None; // saturates Q16.16
        }
        (mant as i128) << shift
    } else {
        let m = mant as i128;
        m >> (-shift)
    };
    let v = if sign { -raw } else { raw };
    if v > i64::MAX as i128 || v < i64::MIN as i128 {
        return None;
    }
    Some(v as i64)
}

/// Parses the FORM header and the chunk table. `None` on a bad magic
/// or a chunk range that overruns the file.
pub fn parse(d: &[u8]) -> Option<Aiff> {
    if d.len() < 12 || &d[0..4] != b"FORM" {
        return None;
    }
    let aifc = match &d[8..12] {
        b"AIFF" => false,
        b"AIFC" => true,
        _ => return None,
    };
    let mut chunks = Vec::new();
    let mut at = 12;
    while at + 8 <= d.len() {
        let mut id = [0u8; 4];
        id.copy_from_slice(&d[at..at + 4]);
        let size = be32(d, at + 4)? as usize;
        let data_at = at + 8;
        if data_at.checked_add(size)? > d.len() {
            return None;
        }
        chunks.push(Chunk {
            id,
            at: data_at,
            size,
        });
        // chunks are 2-byte aligned
        at = data_at + size + (size & 1);
    }
    Some(Aiff { aifc, chunks })
}

/// First chunk with the given 4-byte id.
pub fn chunk<'a>(f: &'a Aiff, id: &[u8; 4]) -> Option<&'a Chunk> {
    f.chunks.iter().find(|c| &c.id == id)
}

/// `COMM` fields — channels/frames/bits plus the extended-float
/// sample rate. `None` without COMM or on a bad rate encoding.
pub fn comm(d: &[u8], f: &Aiff) -> Option<Comm> {
    let c = chunk(f, b"COMM")?;
    let cd = d.get(c.at..c.at + c.size)?;
    Some(Comm {
        channels: be16(cd, 0)?,
        frames: be32(cd, 2)?,
        bits: be16(cd, 6)?,
        rate: extended80(cd.get(8..18)?)?,
        comp: {
            // AIFC appends a 4-byte compression type after the rate
            let mut t = *b"NONE";
            if f.aifc && cd.len() >= 22 {
                t.copy_from_slice(&cd[18..22]);
            }
            t
        },
    })
}

/// `SSND` payload bytes (after the offset/blockSize pair).
pub fn ssnd<'a>(d: &'a [u8], f: &Aiff) -> Option<&'a [u8]> {
    let c = chunk(f, b"SSND")?;
    let data = d.get(c.at..c.at + c.size)?;
    data.get(8..)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut a = Vec::new();
        a.extend_from_slice(b"FORM");
        a.extend_from_slice(&(46u32).to_be_bytes());
        a.extend_from_slice(b"AIFF");
        a.extend_from_slice(b"COMM");
        a.extend_from_slice(&(18u32).to_be_bytes());
        a.extend_from_slice(&[0, 2]); // stereo
        a.extend_from_slice(&[0, 0, 0, 4]); // 4 frames
        a.extend_from_slice(&[0, 16]); // 16 bits
                                       // 44100 Hz as 80-bit extended: 0x400E AC44 0000 0000 0000
        a.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
        a.extend_from_slice(b"SSND");
        a.extend_from_slice(&(12u32).to_be_bytes());
        a.extend_from_slice(&[0; 8]);
        a.extend_from_slice(&[1, 2, 3, 4]);
        a
    }

    #[test]
    fn parses_comm_and_ssnd() {
        let d = fixture();
        let f = parse(&d).unwrap();
        assert!(!f.aifc);
        let c = comm(&d, &f).unwrap();
        assert_eq!(c.channels, 2);
        assert_eq!(c.frames, 4);
        assert_eq!(c.bits, 16);
        assert_eq!(c.rate, 44100 << 16); // Q16.16 raw
        assert_eq!(ssnd(&d, &f).unwrap(), &[1, 2, 3, 4]);
        assert!(chunk(&f, b"MARK").is_none());
    }

    #[test]
    fn extended80_edge_cases() {
        assert_eq!(extended80(&[0; 10]), Some(0));
        // inf
        assert!(extended80(&[0x7F, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0]).is_none());
        // -8000: sign bit + 0x400B FA00…
        let neg = extended80(&[0xC0, 0x0B, 0xFA, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(neg, -(8000 << 16));
        // 1.5 → 0x3FFF C000…
        let onehalf = extended80(&[0x3F, 0xFF, 0xC0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(onehalf, 98304); // 1.5 * 65536
                                    // denormal
        assert!(extended80(&[0, 0, 0x40, 0, 0, 0, 0, 0, 0, 0]).is_none());
        assert!(extended80(&[0; 3]).is_none());
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"NOPE").is_none());
        let mut bad = fixture();
        bad[8] = b'W'; // not AIFF/AIFC
        assert!(parse(&bad).is_none());
        let mut bad2 = fixture();
        bad2[17] = 0xFF; // COMM size huge → overrun
        assert!(parse(&bad2).is_none());
    }
}
