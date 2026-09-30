//! IPFIX message (RFC 7011): 16-byte header — `version` u16 = 10,
//! `length` u16 covering the whole message, `export_time`,
//! `sequence`, `observation_domain` — then Sets of
//! `[set_id u16][set_len u16]`: id 2 = template, 3 = options
//! template, ≥256 = data.
//!
//! ```
//! let mut d = vec![0u8; 16];
//! d[0..2].copy_from_slice(&[0, 10]);  // version
//! d[2..4].copy_from_slice(&[0, 24]);  // total length
//! d.extend_from_slice(&[0, 2, 0, 8, 1, 2, 3, 4]); // template set
//! let m = izanagi_kit::ipfix::parse(&d).unwrap();
//! assert_eq!(m.sets.len(), 1);
//! assert_eq!(m.sets[0].kind, izanagi_kit::ipfix::SetKind::Template);
//! ```

use std::vec::Vec;

/// Classification of a Set by its ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetKind {
    /// Set ID 2 — template records.
    Template,
    /// Set ID 3 — options template records.
    Options,
    /// Set IDs 4..=255 — reserved/undefined.
    Reserved,
    /// Set IDs ≥256 — data records referencing a template.
    Data,
}

/// One IPFIX Set.
#[derive(Clone, Debug)]
pub struct IpfixSet {
    /// Raw set ID.
    pub id: u16,
    /// Set classification.
    pub kind: SetKind,
    /// Length including the 4-byte set header.
    pub len: u16,
    /// Offset of the set body inside the input.
    pub body_offset: usize,
}

/// A parsed IPFIX message.
#[derive(Clone, Debug)]
pub struct Ipfix {
    /// `export_time` — seconds since epoch.
    pub export_time: u32,
    /// Export sequence number.
    pub sequence: u32,
    /// Observation domain ID.
    pub domain: u32,
    /// Sets in order.
    pub sets: Vec<IpfixSet>,
}

fn u16be(d: &[u8], o: usize) -> Option<u16> {
    Some((u16::from(*d.get(o)?) << 8) | u16::from(*d.get(o + 1)?))
}
fn u32be(d: &[u8], o: usize) -> Option<u32> {
    let d = d.get(o..o + 4)?;
    Some(
        (u32::from(d[0]) << 24)
            | (u32::from(d[1]) << 16)
            | (u32::from(d[2]) << 8)
            | u32::from(d[3]),
    )
}

/// Parse an IPFIX message; `None` unless version 10 and the declared
/// length covers all listed Sets exactly.
pub fn parse(d: &[u8]) -> Option<Ipfix> {
    if d.len() < 16 {
        return None;
    }
    if u16be(d, 0)? != 10 {
        return None;
    }
    let total = u16be(d, 2)? as usize;
    if total < 16 || total > d.len() {
        return None;
    }
    let body = &d[..total];
    let mut sets = Vec::new();
    let mut off = 16usize;
    while off < total {
        let id = u16be(body, off)?;
        let len = u16be(body, off + 2)? as usize;
        if len < 4 || len > total - off {
            return None;
        }
        let kind = match id {
            2 => SetKind::Template,
            3 => SetKind::Options,
            4..=255 => SetKind::Reserved,
            _ => SetKind::Data,
        };
        sets.push(IpfixSet {
            id,
            kind,
            len: len as u16,
            body_offset: off + 4,
        });
        off += len;
    }
    Some(Ipfix {
        export_time: u32be(body, 4)?,
        sequence: u32be(body, 8)?,
        domain: u32be(body, 12)?,
        sets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(len: u16) -> Vec<u8> {
        let mut v = vec![0u8; 16];
        v[0..2].copy_from_slice(&[0, 10]);
        v[2..4].copy_from_slice(&[(len >> 8) as u8, len as u8]);
        v[4..8].copy_from_slice(&[0, 0, 0, 42]); // export time
        v
    }

    #[test]
    fn sets() {
        let mut d = hdr(16 + 8 + 20);
        d.extend_from_slice(&[0, 2, 0, 8, 0, 0, 0, 0]); // template set, 4B body
        d.extend_from_slice(&[1, 0, 0, 20]); // data set id 256
        d.extend_from_slice(&[0u8; 16]);
        let m = parse(&d).unwrap();
        assert_eq!(m.export_time, 42);
        assert_eq!(m.sets.len(), 2);
        assert_eq!(m.sets[0].kind, SetKind::Template);
        assert_eq!(m.sets[1].kind, SetKind::Data);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut d = hdr(20);
        d[0..2].copy_from_slice(&[0, 9]);
        assert!(parse(&d).is_none()); // not v10
        let mut d = hdr(20);
        d.extend_from_slice(&[0, 2, 0, 3]); // set_len < 4
        assert!(parse(&d).is_none());
    }
}
