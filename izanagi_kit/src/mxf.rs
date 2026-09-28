//! MXF (SMPTE ST 377): KLV-coded `[16B key][BER len][value]` items.
//! The partition-pack key prefix `06 0E 2B 34 02 05 01 01 0D 01 02 01`
//! opens the file (optionally after a run-in) and marks each header /
//! body / footer partition via `key[13]`.
//!
//! ```
//! use izanagi_kit::mxf::{detect, parse};
//!
//! // One header partition pack: key + short-form BER len + value.
//! let mut d = Vec::new();
//! d.extend_from_slice(&[
//!     0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01, //
//!     0x0D, 0x01, 0x02, 0x01, 0x01, 0x02, 0x04, 0x00,
//! ]);
//! d.push(20); // BER short length
//! d.extend_from_slice(&[0, 1, 0, 3]); // major=1 minor=3
//! d.extend_from_slice(&[0, 0, 0, 1]); // KAG=1
//! d.extend_from_slice(&[0u8; 12]); // this/previous partition fields…
//! assert!(detect(&d));
//! let m = parse(&d).unwrap();
//! assert_eq!(m.partitions, 1);
//! assert_eq!(m.header_partitions, 1);
//! assert_eq!(m.kag, Some(1));
//! ```

/// Parsed MXF KLV census.
#[derive(Debug, Clone, PartialEq)]
pub struct Mxf {
    /// Bytes skipped before the first key (MXF run-in, 0-64KiB).
    pub run_in: u32,
    /// KLV items walked.
    pub items: u32,
    /// Partition-pack keys seen (kind byte `key[14]` 1-4).
    pub partitions: u32,
    /// Partition packs with kind 2 (header).
    pub header_partitions: u32,
    /// Partition packs with kind 3 (body).
    pub body_partitions: u32,
    /// Partition packs with kind 4 (footer).
    pub footer_partitions: u32,
    /// `major` field of the first partition value (usually 1).
    pub major_version: Option<u16>,
    /// `minor` field of the first partition value.
    pub minor_version: Option<u16>,
    /// KAG (kag alignment grid) of the first partition.
    pub kag: Option<u32>,
    /// Trailing bytes not covered by the last item.
    pub trailing: u32,
}

const KEY_PREFIX: [u8; 4] = [0x06, 0x0E, 0x2B, 0x34];
const PARTITION_PREFIX: [u8; 12] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01,
];

fn find_key(b: &[u8]) -> Option<usize> {
    let lim = b.len().min(64 * 1024);
    b[..lim]
        .windows(16)
        .position(|w| w[..4] == KEY_PREFIX && w[..12] == PARTITION_PREFIX)
}

/// `true` when a partition-pack key exists at 0 or in the run-in.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    find_key(b).is_some()
}

/// BER length at `b[i]` → `(len, bytes_consumed)`; `None` on overflow.
fn ber_len(b: &[u8], i: usize) -> Option<(usize, usize)> {
    let first = *b.get(i)?;
    if first & 0x80 == 0 {
        return Some((usize::from(first), 1));
    }
    let n = usize::from(first & 0x7F);
    if n == 0 || n > 8 || i + 1 + n > b.len() {
        return None;
    }
    let mut v = 0usize;
    for &x in &b[i + 1..i + 1 + n] {
        v = v.checked_mul(256)?.checked_add(usize::from(x))?;
    }
    Some((v, 1 + n))
}

/// Walks the KLV chain; `None` without a partition-pack key.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Mxf> {
    let start = find_key(b)?;
    let mut m = Mxf {
        run_in: u32::try_from(start).unwrap_or(u32::MAX),
        items: 0,
        partitions: 0,
        header_partitions: 0,
        body_partitions: 0,
        footer_partitions: 0,
        major_version: None,
        minor_version: None,
        kag: None,
        trailing: 0,
    };
    let mut pos = start;
    while pos + 17 <= b.len() {
        let key = &b[pos..pos + 16];
        if key[..4] != KEY_PREFIX {
            break;
        }
        let (len, used) = ber_len(b, pos + 16)?;
        m.items += 1;
        if key[..12] == PARTITION_PREFIX && key[12] == 0x01 {
            m.partitions += 1;
            match key[13] {
                2 => m.header_partitions += 1,
                3 => m.body_partitions += 1,
                4 => m.footer_partitions += 1,
                _ => {}
            }
            if m.major_version.is_none() {
                let v = pos + 16 + used;
                if v + 8 <= b.len() {
                    m.major_version = Some(u16::from(b[v]) << 8 | u16::from(b[v + 1]));
                    m.minor_version = Some(u16::from(b[v + 2]) << 8 | u16::from(b[v + 3]));
                    m.kag = Some(
                        u32::from(b[v + 4]) << 24
                            | u32::from(b[v + 5]) << 16
                            | u32::from(b[v + 6]) << 8
                            | u32::from(b[v + 7]),
                    );
                }
            }
        }
        let next = pos.checked_add(16 + used)?.checked_add(len)?;
        pos = next;
    }
    m.trailing = u32::try_from(b.len() - pos).unwrap_or(u32::MAX);
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(kind: u8, payload: &[u8]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&PARTITION_PREFIX);
        d.extend_from_slice(&[0x01, kind, 0x04, 0x00]);
        d.push(payload.len() as u8);
        d.extend_from_slice(payload);
        d
    }

    fn fixture() -> Vec<u8> {
        let mut val = Vec::new();
        val.extend_from_slice(&[0, 1, 0, 3]); // major minor
        val.extend_from_slice(&[0, 0, 0, 1]); // KAG
        val.extend_from_slice(&[0u8; 24]);
        let mut d = pack(2, &val);
        d.extend_from_slice(&pack(4, &[0u8; 4]));
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        let mut rin = vec![0u8; 32];
        rin.extend_from_slice(&fixture());
        assert!(detect(&rin));
        assert!(!detect(b"060e2b34 garbage"));
    }

    #[test]
    fn parses() {
        let m = parse(&fixture()).unwrap();
        assert_eq!(m.run_in, 0);
        assert_eq!(m.items, 2);
        assert_eq!(m.partitions, 2);
        assert_eq!(m.header_partitions, 1);
        assert_eq!(m.footer_partitions, 1);
        assert_eq!(m.body_partitions, 0);
        assert_eq!(m.major_version, Some(1));
        assert_eq!(m.minor_version, Some(3));
        assert_eq!(m.kag, Some(1));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"no key").is_none());
        assert!(parse(&[0x06, 0x0E, 0x2B, 0x34]).is_none());
    }
}
