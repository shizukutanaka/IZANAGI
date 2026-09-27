//! RTP packet header parsing (RFC 3550).
//!
//! Fixed header: `V:2 | P:1 | X:1 | CC:4`, `M:1 | PT:7`,
//! `seq u16BE`, `timestamp u32BE`, `ssrc u32BE`, then `CC`
//! CSRC u32s; when `X` is set, `ext_profile u16` +
//! `ext_len u16` (in u32s) follow.
//!
//! ```
//! use izanagi_kit::rtp;
//! let d = [
//!     0x80, 0x60, // v2, pt 96
//!     0x12, 0x34, // seq
//!     0x00, 0x00, 0x00, 0x08, // ts 8
//!     0xDE, 0xAD, 0xBE, 0xEF, // ssrc
//! ];
//! let r = rtp::parse(&d).unwrap();
//! assert_eq!(r.payload_type, 96);
//! ```

use std::vec::Vec;

/// A parsed RTP header.
#[derive(Clone, Debug, PartialEq)]
pub struct Rtp {
    /// `version` (2).
    pub version: u8,
    /// Marker bit.
    pub marker: bool,
    /// `payload_type`.
    pub payload_type: u8,
    /// `sequence_number`.
    pub seq: u16,
    /// `timestamp`.
    pub timestamp: u32,
    /// `ssrc`.
    pub ssrc: u32,
    /// CSRC list (`CC` entries).
    pub csrcs: Vec<u32>,
    /// Extension profile id when `X` set.
    pub ext_profile: Option<u16>,
    /// Extension data length in bytes (`ext_len * 4`).
    pub ext_len_bytes: usize,
    /// Payload offset.
    pub payload_at: usize,
}

fn u16be(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) << 8 | s[1] as u16)
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

/// Parses one RTP packet header: version 2, `CC` CSRCs and the
/// optional extension must fit inside the input.
pub fn parse(d: &[u8]) -> Option<Rtp> {
    if d.len() < 12 {
        return None;
    }
    let version = d[0] >> 6;
    if version != 2 {
        return None;
    }
    let cc = (d[0] & 0x0F) as usize;
    let ext = d[0] & 0x10 != 0;
    let marker = d[1] & 0x80 != 0;
    let payload_type = d[1] & 0x7F;
    let seq = u16be(d, 2)?;
    let timestamp = u32be(d, 4)?;
    let ssrc = u32be(d, 8)?;
    let mut at = 12usize;
    let mut csrcs = Vec::with_capacity(cc);
    for _ in 0..cc {
        csrcs.push(u32be(d, at)?);
        at += 4;
    }
    let mut ext_profile = None;
    let mut ext_len_bytes = 0usize;
    if ext {
        ext_profile = Some(u16be(d, at)?);
        let words = u16be(d, at + 2)? as usize;
        ext_len_bytes = words.checked_mul(4)?;
        at += 4;
        if at.checked_add(ext_len_bytes)? > d.len() {
            return None;
        }
        at += ext_len_bytes;
    }
    Some(Rtp {
        version,
        marker,
        payload_type,
        seq,
        timestamp,
        ssrc,
        csrcs,
        ext_profile,
        ext_len_bytes,
        payload_at: at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn parses_header() {
        let d = [
            0x90, 0x60, // v2 + X set, pt 96
            0x00, 0x01, // seq
            0x00, 0x00, 0x00, 0xA0, // ts
            0x11, 0x22, 0x33, 0x44, // ssrc
            0xBE, 0xDF, // ext profile
            0x00, 0x01, // ext len 1 word
            0xAA, 0xBB, 0xCC, 0xDD, // ext data
            0x55, // payload
        ];
        let r = parse(&d).unwrap();
        assert_eq!(r.seq, 1);
        assert_eq!(r.ssrc, 0x11223344);
        assert_eq!(r.ext_profile, Some(0xBEDF));
        assert_eq!(r.ext_len_bytes, 4);
        assert_eq!(r.payload_at, 20);
    }

    #[test]
    fn csrcs_and_rejects() {
        let d = [
            0x82, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // v2 cc=2
            0, 0, 0, 5, 0, 0, 0, 6, // two csrcs
        ];
        let r = parse(&d).unwrap();
        assert_eq!(r.csrcs, vec![5, 6]);
        assert_eq!(r.payload_at, 20);
        assert!(parse(&[0u8; 8]).is_none());
        let mut bad = d;
        bad[0] = 0x42; // version 1
        assert!(parse(&bad).is_none());
    }
}
