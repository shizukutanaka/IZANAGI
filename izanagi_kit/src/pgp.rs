//! OpenPGP packet framing (RFC 9580 §4.2, obsoleting RFC 4880): the
//! outer packet grammar that wraps keys, signatures, and messages.
//! The first octet of every packet has bit 7 set; bit 6 distinguishes
//! the modern ("new format") header — 6-bit tag + length word — from
//! legacy format — 4-bit tag + 2-bit length type (1-, 2-, or 4-octet
//! length, or indeterminate-to-EOF). New-format lengths of `224..=254`
//! are *partial* body lengths (the body continues after the chunk),
//! which [`payload`] reassembles. [`packets`] walks a keyring or message
//! file; [`tag_name`] names the packet registry; [`armor`]/[`unarmor`]
//! cover the ASCII-armor wrapper including the CRC-24 trailer.
//!
//! ```
//! use izanagi_kit::pgp::{packets, tag_name};
//! // new-format tag 11 (literal data), len 3, body "abc"
//! let d = [0xCB, 0x03, b'a', b'b', b'c'];
//! let ps = packets(&d).unwrap();
//! assert_eq!(ps[0].tag, 11);
//! assert_eq!(tag_name(ps[0].tag), "Literal Data");
//! ```

use std::string::String;
use std::vec::Vec;

/// One packet's parsed header.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pkt {
    /// Packet tag (0–63 new format, 0–15 legacy).
    pub tag: u8,
    /// True for new-format headers (bit 6 set).
    pub new_fmt: bool,
    /// Offset of the packet's first octet.
    pub at: usize,
    /// Header length in bytes — through the *first* length octet for
    /// partial packets (their chunk sequence is re-walked by readers).
    pub head: usize,
    /// Body length; `None` for legacy indeterminate (runs to EOF) and
    /// for partial sequences.
    pub len: Option<u64>,
    /// True when the body is chunked into partial-length segments.
    pub partial: bool,
    /// Offset just past the packet (after the body or the partial chain).
    pub end: usize,
}

/// New-format length word starting at `d[*i]`: returns
/// `(len, partial)`. A `224..=254` first octet yields the 1<<n chunk
/// size and `partial = true`.
fn new_len(d: &[u8], i: &mut usize) -> Option<(u64, bool)> {
    let n = *d.get(*i)?;
    *i += 1;
    match n {
        0..=191 => Some((n as u64, false)),
        192..=223 => {
            let n2 = *d.get(*i)?;
            *i += 1;
            Some((((n as u64 - 192) << 8) + n2 as u64 + 192, false))
        }
        224..=254 => Some((1u64 << (n & 0x1F), true)),
        _ => {
            let b = d.get(*i..*i + 4)?;
            *i += 4;
            Some((
                u64::from(b[0]) << 24
                    | u64::from(b[1]) << 16
                    | u64::from(b[2]) << 8
                    | u64::from(b[3]),
                false,
            ))
        }
    }
}

/// Walk a buffer of consecutive packets. `None` on a first octet
/// without bit 7, a truncated header, or a body that overruns the
/// buffer. A partial body is folded into a single [`Pkt`] whose `end`
/// covers the whole chunk chain; a legacy indeterminate packet consumes
/// the rest of the input.
pub fn packets(d: &[u8]) -> Option<Vec<Pkt>> {
    let mut v = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        let at = i;
        let c = *d.get(i)?;
        if c & 0x80 == 0 {
            return None;
        }
        if c & 0x40 != 0 {
            // new format: tag = low 6 bits, then the length word
            let tag = c & 0x3F;
            i += 1;
            let (len, partial) = new_len(d, &mut i)?;
            if !partial {
                let end = i.checked_add(len as usize)?;
                if end > d.len() {
                    return None;
                }
                v.push(Pkt {
                    tag,
                    new_fmt: true,
                    at,
                    head: i - at,
                    len: Some(len),
                    partial: false,
                    end,
                });
                i = end;
                continue;
            }
            // partial chain: [len-octet chunk-data]* ending on a normal
            // length word. `head` = 1 (tag octet only); readers re-walk
            // the whole chain from at+1.
            let mut clen = len;
            let mut pos = i;
            loop {
                let mut j = pos.checked_add(clen as usize)?;
                let (nl, np) = new_len(d, &mut j)?;
                if np {
                    pos = j;
                    clen = nl;
                    continue;
                }
                let end = j.checked_add(nl as usize)?;
                if end > d.len() {
                    return None;
                }
                v.push(Pkt {
                    tag,
                    new_fmt: true,
                    at,
                    head: 1,
                    len: None,
                    partial: true,
                    end,
                });
                i = end;
                break;
            }
        } else {
            // legacy format: tag = bits 5..2, length type = bits 1..0
            let tag = (c >> 2) & 0x0F;
            let lt = c & 0x03;
            i += 1;
            let len = match lt {
                0 => {
                    let b = *d.get(i)?;
                    i += 1;
                    Some(b as u64)
                }
                1 => {
                    let b = d.get(i..i + 2)?;
                    i += 2;
                    Some(u64::from(b[0]) << 8 | u64::from(b[1]))
                }
                2 => {
                    let b = d.get(i..i + 4)?;
                    i += 4;
                    Some(
                        u64::from(b[0]) << 24
                            | u64::from(b[1]) << 16
                            | u64::from(b[2]) << 8
                            | u64::from(b[3]),
                    )
                }
                _ => None, // indeterminate: to end of input
            };
            let end = match len {
                Some(l) => i.checked_add(l as usize)?,
                None => d.len(),
            };
            if end > d.len() {
                return None;
            }
            v.push(Pkt {
                tag,
                new_fmt: false,
                at,
                head: i - at,
                len,
                partial: false,
                end,
            });
            i = end;
        }
    }
    Some(v)
}

/// Registry name for a packet tag (RFC 9580 §4.3 list): `"Signature"`,
/// `"Public-Key"`, … `"Reserved/Unknown"` for the gaps.
pub fn tag_name(tag: u8) -> &'static str {
    match tag {
        0 => "Reserved",
        1 => "Public-Key Encrypted Session Key",
        2 => "Signature",
        3 => "Symmetric-Key Encrypted Session Key",
        4 => "One-Pass Signature",
        5 => "Secret-Key",
        6 => "Public-Key",
        7 => "Secret-Subkey",
        8 => "Compressed Data",
        9 => "Symmetrically Encrypted Data",
        10 => "Marker",
        11 => "Literal Data",
        12 => "Trust",
        13 => "User ID",
        14 => "Public-Subkey",
        17 => "User Attribute",
        18 => "SEIPD",
        20 => "AEAD Encrypted Data",
        _ => "Reserved/Unknown",
    }
}

/// Packet body. For partial packets the chunk payloads are
/// concatenated (reassembled); for a legacy indeterminate packet it's
/// the span to `end`. `None` only on internal inconsistency.
pub fn payload(d: &[u8], p: &Pkt) -> Option<Vec<u8>> {
    let start = p.at + p.head;
    if !p.partial {
        return d.get(start..p.end).map(|s| s.to_vec());
    }
    let mut out = Vec::new();
    let mut i = start;
    while i < p.end {
        let (clen, partial) = new_len(d, &mut i)?;
        let _ = partial;
        out.extend_from_slice(d.get(i..i + clen as usize)?);
        i += clen as usize;
    }
    Some(out)
}

/// Wrap bytes in ASCII armor: `-----BEGIN PGP <label>-----`, base64
/// body wrapped at 64 columns, and a `=xxxx` CRC-24 trailer (RFC 9580
/// §6 — CRC of the *unarmored* bytes, poly `0x1864CFB`, init `0xB704CE`).
pub fn armor(label: &str, body: &[u8]) -> String {
    let mut s = String::new();
    s.push_str("-----BEGIN PGP ");
    s.push_str(label);
    s.push_str("-----\r\n\r\n");
    let b64 = crate::base64::encode(body);
    for chunk in b64.as_bytes().chunks(64) {
        s.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        s.push_str("\r\n");
    }
    s.push('=');
    let crc = crc24(body);
    s.push_str(&crate::base64::encode(&[
        ((crc >> 16) & 0xFF) as u8,
        ((crc >> 8) & 0xFF) as u8,
        (crc & 0xFF) as u8,
    ]));
    s.push_str("\r\n-----END PGP ");
    s.push_str(label);
    s.push_str("-----\r\n");
    s
}

/// RFC 9580 §6.1 CRC-24 (poly `0x1864CFB`, init `0xB704CE`).
pub fn crc24(d: &[u8]) -> u32 {
    let mut crc = 0xB704CEu32;
    for &b in d {
        crc ^= (b as u32) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x100_0000 != 0 {
                crc ^= 0x1864CFB;
            }
        }
    }
    crc & 0xFF_FFFF
}

/// Decode an armored block back to its packet bytes. Header lines are
/// skipped, the `=xxxx` CRC-24 trailer verified when present. `None` on
/// malformed armor or a bad checksum.
pub fn unarmor(d: &str) -> Option<Vec<u8>> {
    let rest = d.trim_start().strip_prefix("-----BEGIN PGP ")?;
    let nl = rest.find('-')?;
    let after = rest[nl..].strip_prefix("-----")?;
    let mut b64 = String::new();
    let mut crc_line: Option<String> = None;
    let mut in_body = false;
    for l in after.lines() {
        if l.trim().is_empty() && !in_body {
            in_body = true;
            continue;
        }
        if !in_body {
            continue; // Comment:/etc. armor headers
        }
        if let Some(c) = l.strip_prefix('=') {
            crc_line = Some(c.trim().to_string());
            continue;
        }
        if l.starts_with("-----END") {
            break;
        }
        b64.push_str(l.trim());
    }
    let body = crate::base64::decode(&b64)?;
    if let Some(c) = crc_line {
        let raw = crate::base64::decode(&c)?;
        if raw.len() != 3 {
            return None;
        }
        let want = ((raw[0] as u32) << 16) | ((raw[1] as u32) << 8) | raw[2] as u32;
        if crc24(&body) != want {
            return None;
        }
    }
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_format_lengths() {
        // tag 11, len 3 (1-octet)
        let d = [0xCB, 0x03, b'a', b'b', b'c'];
        let p = packets(&d).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].tag, 11);
        assert_eq!(p[0].len, Some(3));
        assert_eq!(payload(&d, &p[0]).unwrap(), b"abc");

        // 2-octet length: tag 13 (0xC0|13=0xCD), len 192 → 0xC0 0x00
        let mut d = vec![0xCD, 0xC0, 0x00];
        d.extend_from_slice(&[0x55; 192]);
        let p = packets(&d).unwrap();
        assert_eq!(p[0].len, Some(192));

        // 5-octet length: 255 + u32be
        let mut d = vec![0xC6, 0xFF, 0, 0, 1, 0];
        d.extend_from_slice(&vec![0; 256]);
        let p = packets(&d).unwrap();
        assert_eq!(p[0].tag, 6);
        assert_eq!(p[0].len, Some(256));
    }

    #[test]
    fn legacy_format() {
        // legacy: bit7=1, bit6=0 → tag in bits 5..2, len-type in 1..0
        // tag 2 (signature), len-type 0 → 1-octet len
        let d = [0x88, 0x05, 1, 2, 3, 4, 5];
        let p = packets(&d).unwrap();
        assert_eq!(p[0].tag, 2);
        assert!(!p[0].new_fmt);
        assert_eq!(p[0].len, Some(5));

        // indeterminate (len-type 3): runs to EOF
        let d = [0x8F, 9, 9, 9];
        let p = packets(&d).unwrap();
        assert_eq!(p[0].len, None);
        assert_eq!(p[0].end, 4);
    }

    #[test]
    fn partial_body() {
        // new-format tag 9, partial: 0xE0 → 1-byte chunk, then
        // 0xE1 → 2-byte chunk, then final len 0x02 + 2 bytes
        let d = [0xC9, 0xE0, b'a', 0xE1, b'b', b'c', 0x02, b'd', b'e'];
        let p = packets(&d).unwrap();
        assert_eq!(p.len(), 1);
        assert!(p[0].partial);
        assert_eq!(payload(&d, &p[0]).unwrap(), b"abcde");
    }

    #[test]
    fn tag_names() {
        assert_eq!(tag_name(2), "Signature");
        assert_eq!(tag_name(6), "Public-Key");
        assert_eq!(tag_name(18), "SEIPD");
        assert_eq!(tag_name(20), "AEAD Encrypted Data");
        assert_eq!(tag_name(63), "Reserved/Unknown");
    }

    #[test]
    fn armor_roundtrip() {
        let body = b"PGP packet bytes here";
        let a = armor("MESSAGE", body);
        assert!(a.starts_with("-----BEGIN PGP MESSAGE-----"));
        assert_eq!(unarmor(&a).unwrap(), body);
        assert_eq!(crc24(b"123456789"), 0x21CF02); // RFC 9580 check value
        let mut bad = a.clone();
        bad.replace_range(40..41, "x");
        assert!(unarmor(&bad).is_none());
    }

    #[test]
    fn bad_inputs() {
        assert!(packets(b"").unwrap().is_empty());
        assert!(packets(&[0x00]).is_none()); // bit 7 clear
        assert!(packets(&[0xCB]).is_none()); // truncated length
        assert!(packets(&[0xCB, 0x10]).is_none()); // body overrun
    }
}
