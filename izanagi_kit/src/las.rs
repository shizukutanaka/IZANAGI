//! ASPRS LASer (`.las`) point-cloud header.
//!
//! All fields are little-endian. Floating-point scale/offset/bounds
//! are kept as raw `u64`/`u32` bit patterns — the crate never touches
//! `f64`. Header size grows by version: 227 B (≤1.2), 235 B (1.3 adds
//! a waveform start), 375 B (1.4 adds EVLR + u64 point counts).
//!
//! ```
//! use izanagi_kit::las::parse;
//! let mut d = vec![0u8; 227];
//! d[0..4].copy_from_slice(b"LASF");
//! d[24] = 1; d[25] = 2;                 // version 1.2
//! d[94..96].copy_from_slice(&[227, 0]); // header size
//! d[96..100].copy_from_slice(&[227, 0, 0, 0]); // point offset
//! d[104] = 2;                           // point format 2
//! d[105..107].copy_from_slice(&[28, 0]);  // record len 28
//! d[107..111].copy_from_slice(&[5, 0, 0, 0]); // 5 points
//! let l = parse(&d).unwrap();
//! assert_eq!(l.version, (1, 2));
//! assert_eq!(l.point_count, 5);
//! ```

fn le16(d: &[u8], o: usize) -> Option<u16> {
    Some((*d.get(o)? as u16) | (*d.get(o + 1)? as u16) << 8)
}

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32)
            | (*d.get(o + 1)? as u32) << 8
            | (*d.get(o + 2)? as u32) << 16
            | (*d.get(o + 3)? as u32) << 24,
    )
}

fn le64(d: &[u8], o: usize) -> Option<u64> {
    Some((le32(d, o)? as u64) | (le32(d, o + 4)? as u64) << 32)
}

fn bytes(d: &[u8], o: usize, n: usize) -> Option<Vec<u8>> {
    Some(d.get(o..o + n)?.to_vec())
}

/// Parsed LAS header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Las {
    /// Version as `(major, minor)`.
    pub version: (u8, u8),
    /// Declared header size.
    pub header_size: u16,
    /// Offset where point records begin.
    pub point_offset: u32,
    /// Number of variable-length records.
    pub vlr_count: u32,
    /// Point data format (0–10).
    pub point_format: u8,
    /// Bytes per point record.
    pub point_len: u16,
    /// Legacy u32 point count (0 in LAS 1.4 files that use `points64`).
    pub point_count: u32,
    /// Points per return, `[u32; 5]`.
    pub by_return: [u32; 5],
    /// `[x, y, z]` scale factors as raw IEEE-754 bit patterns.
    pub scale_bits: [u64; 3],
    /// `[x, y, z]` offsets as raw bit patterns.
    pub offset_bits: [u64; 3],
    /// `[max_x, min_x, max_y, min_y, max_z, min_z]` raw bit patterns.
    pub bounds_bits: [u64; 6],
    /// LAS 1.4+: u64 point count (`None` for older versions).
    pub points64: Option<u64>,
    /// LAS 1.4+: u64 points per return (`None` for older).
    pub by_return64: Option<[u64; 15]>,
    /// LAS 1.3+: start of waveform data packet record.
    pub waveform_start: Option<u64>,
    /// LAS 1.4+: start of first EVLR + count.
    pub evlr_start: Option<u64>,
    /// LAS 1.4+: EVLR count.
    pub evlr_count: Option<u32>,
}

/// Parse a LAS header. `None` on bad `LASF` magic, a version outside
/// 1.0–1.4, or a buffer shorter than the version's header size.
pub fn parse(d: &[u8]) -> Option<Las> {
    if d.get(0..4)? != b"LASF" {
        return None;
    }
    let version = (*d.get(24)?, *d.get(25)?);
    let header_size = le16(d, 94)?;
    let minimum = match version {
        (1, 0) | (1, 1) | (1, 2) => 227,
        (1, 3) => 235,
        (1, 4) => 375,
        _ => return None,
    };
    if (header_size as usize) < minimum || d.len() < minimum {
        return None;
    }
    let mut by_return = [0u32; 5];
    for (slot, b) in by_return
        .iter_mut()
        .zip(d.get(108..108 + 20)?.chunks_exact(4))
    {
        *slot = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    }
    let mut scale_bits = [0u64; 3];
    let mut offset_bits = [0u64; 3];
    for i in 0..3 {
        scale_bits[i] = le64(d, 131 + i * 8)?;
        offset_bits[i] = le64(d, 155 + i * 8)?;
    }
    let mut bounds_bits = [0u64; 6];
    for (slot, b) in bounds_bits
        .iter_mut()
        .zip(d.get(179..179 + 48)?.chunks_exact(8))
    {
        *slot = u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    }
    let (waveform_start, evlr_start, evlr_count, points64, by_return64) = match version {
        (1, 3) => (Some(le64(d, 227)?), None, None, None, None),
        (1, 4) => (
            Some(le64(d, 227)?),
            Some(le64(d, 235)?),
            Some(le32(d, 243)?),
            Some(le64(d, 247)?),
            Some({
                let mut v = [0u64; 15];
                for (slot, b) in v.iter_mut().zip(d.get(255..255 + 120)?.chunks_exact(8)) {
                    *slot = u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
                }
                v
            }),
        ),
        _ => (None, None, None, None, None),
    };
    let point_count = le32(d, 107)?;
    let point_format = *d.get(104)?;
    let point_len = le16(d, 105)?;
    if point_format > 10 {
        return None;
    }
    Some(Las {
        version,
        header_size,
        point_offset: le32(d, 96)?,
        vlr_count: le32(d, 100)?,
        point_format,
        point_len,
        point_count,
        by_return,
        scale_bits,
        offset_bits,
        bounds_bits,
        points64,
        by_return64,
        waveform_start,
        evlr_start,
        evlr_count,
    })
}

/// Total point count as `u64` — `points64` for 1.4 when the legacy
/// count is 0, else `point_count`.
pub fn points(l: &Las) -> u64 {
    match l.points64 {
        Some(n) if l.point_count == 0 => n,
        _ => l.point_count as u64,
    }
}

/// Offset of point record `i` (`0 <= i < points(l)`), bounds-checked
/// against the buffer length.
pub fn point_at(l: &Las, data_len: usize, i: u64) -> Option<usize> {
    if i >= points(l) {
        return None;
    }
    let off = (i as usize).checked_mul(l.point_len as usize)?;
    let at = (l.point_offset as usize).checked_add(off)?;
    if at + l.point_len as usize > data_len {
        return None;
    }
    Some(at)
}

/// Trimmed NUL-free system identifier (32-byte field at offset 26).
pub fn system_id(d: &[u8]) -> Option<Vec<u8>> {
    Some(trim(bytes(d, 26, 32)?))
}

/// Trimmed generating-software string (32-byte field at 58).
pub fn software(d: &[u8]) -> Option<Vec<u8>> {
    Some(trim(bytes(d, 58, 32)?))
}

fn trim(b: Vec<u8>) -> Vec<u8> {
    let mut e = b.len();
    while e > 0 && b[e - 1] == 0 {
        e -= 1;
    }
    b[..e].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w16(d: &mut [u8], o: usize, v: u16) {
        d[o] = v as u8;
        d[o + 1] = (v >> 8) as u8;
    }
    fn w32(d: &mut [u8], o: usize, v: u32) {
        for i in 0..4 {
            d[o + i] = (v >> (i * 8)) as u8;
        }
    }
    fn w64(d: &mut [u8], o: usize, v: u64) {
        for i in 0..8 {
            d[o + i] = (v >> (i * 8)) as u8;
        }
    }

    fn fixture(minor: u8) -> Vec<u8> {
        let n = if minor == 4 {
            375
        } else if minor == 3 {
            235
        } else {
            227
        };
        // room for the header plus three 26-byte point records
        let mut d = vec![0u8; n + 80];
        d[0..4].copy_from_slice(b"LASF");
        w16(&mut d, 4, 1); // file source id
        d[24] = 1;
        d[25] = minor;
        d[26..32].copy_from_slice(b"RIEGL ");
        d[58..64].copy_from_slice(b"laspy\x00");
        w16(&mut d, 94, n as u16);
        w32(&mut d, 96, n as u32); // point offset = header size
        w32(&mut d, 100, 0); // vlrs
        d[104] = 2; // point format
        w16(&mut d, 105, 26); // point record len
        w32(&mut d, 107, 3); // point count
        for i in 0..5 {
            w32(&mut d, 108 + i * 4, i as u32);
        }
        w64(&mut d, 131, 0x3F80_0000_0000_0000); // scale x = 0.0078125? raw
        w64(&mut d, 155, 0x4050_0000_0000_0000); // offset x
        if minor == 3 {
            w64(&mut d, 227, 0xABCD);
        }
        if minor == 4 {
            w64(&mut d, 227, 0);
            w64(&mut d, 235, 0xEEEE);
            w32(&mut d, 243, 1);
            w64(&mut d, 247, 5_000_000);
            for i in 0..15 {
                w64(&mut d, 255 + i * 8, i as u64);
            }
        }
        d
    }

    #[test]
    fn v12_fields() {
        let d = fixture(2);
        let l = parse(&d).unwrap();
        assert_eq!(l.version, (1, 2));
        assert_eq!(l.header_size, 227);
        assert_eq!(l.point_format, 2);
        assert_eq!(l.point_len, 26);
        assert_eq!(l.point_count, 3);
        assert_eq!(points(&l), 3);
        assert_eq!(l.by_return[1], 1);
        assert_eq!(l.scale_bits[0], 0x3F80_0000_0000_0000);
        assert_eq!(l.offset_bits[0], 0x4050_0000_0000_0000);
        assert!(l.points64.is_none() && l.waveform_start.is_none());
        assert_eq!(point_at(&l, d.len(), 0), Some(227));
        assert_eq!(point_at(&l, d.len(), 2), Some(227 + 52));
        assert_eq!(point_at(&l, d.len(), 3), None);
        assert_eq!(system_id(&d).unwrap(), b"RIEGL ");
        assert_eq!(software(&d).unwrap(), b"laspy");
    }

    #[test]
    fn v13_and_v14() {
        let d3 = fixture(3);
        let l3 = parse(&d3).unwrap();
        assert_eq!(l3.waveform_start, Some(0xABCD));
        assert!(l3.points64.is_none());
        let d4 = fixture(4);
        let l4 = parse(&d4).unwrap();
        assert_eq!(l4.evlr_start, Some(0xEEEE));
        assert_eq!(l4.evlr_count, Some(1));
        assert_eq!(l4.points64, Some(5_000_000));
        assert_eq!(l4.by_return64.unwrap()[3], 3);
        // legacy count non-zero wins for `points`
        assert_eq!(points(&l4), 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"LASX").is_none());
        let mut d = fixture(2);
        d[25] = 9;
        assert!(parse(&d).is_none());
        let mut d2 = fixture(2);
        d2[104] = 11;
        assert!(parse(&d2).is_none());
        // truncated below the 227-byte minimum header
        let d3 = fixture(2);
        assert!(parse(&d3[..200]).is_none());
    }
}
