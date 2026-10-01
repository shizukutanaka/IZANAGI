//! XID — 12-byte globally unique ID (4-byte timestamp, 3-byte machine id,
//! 2-byte pid, 3-byte counter) encoded in the lowercase Base32hex
//! alphabet, always 20 chars with no padding (e.g. `9m4e2mr0ui3e8a215n4g`).
//!
//! ```
//! let d = b"9m4e2mr0ui3e8a215n4g";
//! let x = izanagi_kit::xid::parse(d).unwrap();
//! assert_eq!(x.len, 20);
//! assert!(x.timestamp > 0);
//! assert!(izanagi_kit::xid::detect(d));
//! ```

/// Decoded XID census.
#[derive(Debug, Clone)]
pub struct Xid {
    /// Always 20.
    pub len: usize,
    /// Seconds timestamp (first 4 bytes, BE).
    pub timestamp: u32,
    /// Machine identifier (3 bytes, BE).
    pub machine: u32,
    /// Process id (2 bytes, BE).
    pub pid: u16,
    /// Random counter (3 bytes, BE).
    pub counter: u32,
    /// Distinct alphabet chars.
    pub distinct: usize,
}

fn crock(c: u8) -> Option<u32> {
    // XID alphabet: base32hex lowercase 0-9 a-v
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'v' | b'A'..=b'V' => Some((c.to_ascii_lowercase() - b'a' + 10) as u32),
        _ => None,
    }
}

/// Decodes 20 Crockford chars into the 12-byte payload (`[u8; 12]`).
fn decode(b: &[u8]) -> Option<[u8; 12]> {
    if b.len() != 20 {
        return None;
    }
    let mut acc = [0u8; 16]; // 20 chars * 5 bits = 100 bits ≤ u128
    let mut v = 0u128;
    for &c in b {
        v = (v << 5) | crock(c)? as u128;
    }
    // take low 96 bits → 12 bytes
    for (i, o) in acc.iter_mut().enumerate() {
        let shift = (15 - i) * 8;
        *o = ((v >> shift) & 0xff) as u8;
    }
    let mut out = [0u8; 12];
    out.copy_from_slice(&acc[4..16]);
    Some(out)
}

/// Detects an XID: exactly 20 Crockford alphabet chars.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() == 20 && b.iter().all(|c| crock(*c).is_some())
}

/// Parses an XID; `None` unless exactly 20 valid chars.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xid> {
    let d = decode(b)?;
    let mut seen = 0u64;
    let mut distinct = 0usize;
    for &c in b {
        let i = crock(c)? as usize;
        if seen & (1 << i) == 0 {
            seen |= 1 << i;
            distinct += 1;
        }
    }
    Some(Xid {
        len: 20,
        timestamp: (d[0] as u32) << 24 | (d[1] as u32) << 16 | (d[2] as u32) << 8 | d[3] as u32,
        machine: (d[4] as u32) << 16 | (d[5] as u32) << 8 | d[6] as u32,
        pid: (d[7] as u16) << 8 | d[8] as u16,
        counter: (d[9] as u32) << 16 | (d[10] as u32) << 8 | d[11] as u32,
        distinct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let x = parse(b"9m4e2mr0ui3e8a215n4g").unwrap();
        assert_eq!(x.len, 20);
        assert!(x.timestamp > 0);
        assert!(x.counter > 0);
    }

    #[test]
    fn decodes_leading() {
        // "00000000000000000000" → all-zero payload
        let x = parse(b"00000000000000000000").unwrap();
        assert_eq!(x.timestamp, 0);
        assert_eq!(x.machine, 0);
        assert_eq!(x.counter, 0);
        assert_eq!(x.distinct, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"9m4e2mr0ui3e8a215n4g"));
        assert!(detect(b"9M4E2MR0UI3E8A215N4G"));
        assert!(!detect(b"9m4e2mr0ui3e8a215n4w")); // 'w' not in alphabet
        assert!(!detect(b"short"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"9m4e2mr0ui3e8a215n4w").is_none());
    }
}
