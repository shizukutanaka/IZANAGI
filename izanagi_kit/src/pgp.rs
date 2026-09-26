//! OpenPGP packet framing (RFC 4880 §4.2): each packet is a tag
//! byte + length + body. Old-format packets (bit6=0) use
//! 1/2/4-byte or indeterminate lengths; new-format (bit6=1) use
//! 1/2/5-byte or *partial-body* lengths (`1 << (b & 31)` chunks).
//! ASCII armor (`-----BEGIN PGP MESSAGE-----`) wraps the same
//! stream in base64 with a CRC-24 trailer.
//!
//! ```
//! use izanagi_kit::pgp;
//! // new-format: tag 11 (literal data), len 3, body "abc"
//! let pkt = [0xCB, 3, b'a', b'b', b'c'];
//! let p = pgp::packets(&pkt).unwrap();
//! assert_eq!(p[0].tag, 11);
//! assert_eq!(pgp::body(&pkt, &p[0]), b"abc");
//! ```

use std::vec::Vec;

/// CRC-24, poly `0x01864CFB`, init `0x00B704CE` — the armor
/// checksum.
pub fn crc24(d: &[u8]) -> u32 {
    let mut crc: u32 = 0x00B7_04CE;
    for &b in d {
        crc ^= (b as u32) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x0100_0000 != 0 {
                crc ^= 0x0186_4CFB;
            }
        }
        crc &= 0x00FF_FFFF;
    }
    crc
}

/// One packet header's view into the input.
#[derive(Clone, Debug)]
pub struct Packet {
    /// Packet tag (`0..=63`).
    pub tag: u8,
    /// Body start offset.
    pub offset: usize,
    /// Body length in bytes (sum of partial chunks when
    /// `partial`; *the chunks are not necessarily contiguous* —
    /// use [`body`] which concatenates them).
    pub size: usize,
    /// Old-format header (`false` = new format).
    pub old: bool,
    /// `true` when new-format partial lengths were used.
    pub partial: bool,
    chunks: Vec<(usize, usize)>,
}

/// RFC 4880 tag → name.
pub fn tag_name(tag: u8) -> &'static str {
    match tag {
        1 => "PKESK",
        2 => "signature",
        3 => "SKESK",
        4 => "one-pass-sig",
        5 => "secret-key",
        6 => "public-key",
        7 => "secret-subkey",
        8 => "compressed",
        9 => "literal",
        10 => "marker",
        11 => "literal-data",
        12 => "trust",
        13 => "user-id",
        14 => "public-subkey",
        17 => "user-attribute",
        18 => "SEIP-data",
        19 => "MDC",
        _ => "other",
    }
}

fn new_len(d: &[u8], i: usize) -> Option<(usize, usize, bool)> {
    // returns (body_len, bytes_consumed, partial)
    let b = *d.get(i)? as usize;
    match b {
        0..=191 => Some((b, 1, false)),
        192..=223 => {
            let b2 = *d.get(i + 1)? as usize;
            Some((((b - 192) << 8) + b2 + 192, 2, false))
        }
        224..=254 => Some((1 << (b & 31), 1, true)),
        _ => Some((
            ((*d.get(i + 1)? as usize) << 24)
                | ((*d.get(i + 2)? as usize) << 16)
                | ((*d.get(i + 3)? as usize) << 8)
                | *d.get(i + 4)? as usize,
            5,
            false,
        )),
    }
}

/// Walks the packet stream. `None` on truncation, bad tag byte
/// (top bit clear), or reserved/old-indeterminate abuse.
pub fn packets(d: &[u8]) -> Option<Vec<Packet>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        let c = *d.get(i)?;
        if c & 0x80 == 0 {
            return None;
        }
        i += 1;
        if c & 0x40 != 0 {
            // new format
            let tag = c & 0x3F;
            let mut body = 0usize;
            let mut start = i;
            let mut chunks = Vec::new();
            let mut partial_any = false;
            loop {
                let (n, nb, partial) = new_len(d, i)?;
                i += nb;
                let end = i.checked_add(n)?;
                if end > d.len() {
                    return None;
                }
                if chunks.is_empty() {
                    start = i;
                }
                chunks.push((i, n));
                body += n;
                i = end;
                if !partial {
                    break;
                }
                partial_any = true;
            }
            out.push(Packet {
                tag,
                offset: start,
                size: body,
                old: false,
                partial: partial_any,
                chunks,
            });
        } else {
            // old format: len-type in low 2 bits
            let tag = (c >> 2) & 0x0F;
            let lt = c & 3;
            let (n, nb): (usize, usize) = match lt {
                0 => (*d.get(i)? as usize, 1),
                1 => (((*d.get(i)? as usize) << 8) | *d.get(i + 1)? as usize, 2),
                2 => (
                    ((*d.get(i)? as usize) << 24)
                        | ((*d.get(i + 1)? as usize) << 16)
                        | ((*d.get(i + 2)? as usize) << 8)
                        | *d.get(i + 3)? as usize,
                    4,
                ),
                _ => (d.len() - i, 0), // indeterminate: to EOF
            };
            i += nb;
            let end = i.checked_add(n)?;
            if end > d.len() {
                return None;
            }
            out.push(Packet {
                tag,
                offset: i,
                size: n,
                old: true,
                partial: false,
                chunks: vec![(i, n)],
            });
            i = end;
            if lt == 3 {
                break; // indeterminate consumed to EOF
            }
        }
    }
    Some(out)
}

/// Packet body bytes — contiguous packets return their slice;
/// `partial` packets' chunks are concatenated into an owned vec.
/// Returns empty on coordinate mismatch.
pub fn body(d: &[u8], p: &Packet) -> Vec<u8> {
    if p.chunks.len() == 1 {
        let (at, n) = p.chunks[0];
        return d.get(at..at + n).unwrap_or(&[]).to_vec();
    }
    let mut out = Vec::with_capacity(p.size);
    for &(at, n) in &p.chunks {
        out.extend_from_slice(d.get(at..at + n).unwrap_or(&[]));
    }
    out
}

/// `true` when `d` starts with an ASCII-armor line.
pub fn is_armored(d: &[u8]) -> bool {
    d.starts_with(b"-----BEGIN PGP ")
}

/// Extracts the base64 payload of an ASCII-armored message and
/// verifies the `=xxxx` CRC-24 trailer when present.
/// Returns the decoded bytes.
pub fn dearmor(d: &[u8]) -> Option<Vec<u8>> {
    if !is_armored(d) {
        return None;
    }
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.lines();
    // header
    let head = lines.next()?;
    if !head.starts_with("-----BEGIN PGP ") || !head.ends_with("-----") {
        return None;
    }
    // skip headers until blank line
    loop {
        let l = lines.next()?;
        if l.is_empty() {
            break;
        }
        if l.starts_with("-----") {
            return None;
        }
    }
    // collect base64 until '=crc' or END
    let mut b64 = std::string::String::new();
    let mut crc_line: Option<&str> = None;
    for l in lines {
        if l.starts_with("-----END PGP ") {
            break;
        }
        if let Some(rest) = l.strip_prefix('=') {
            crc_line = Some(rest);
            continue;
        }
        if l.is_empty() {
            continue;
        }
        b64.push_str(l.trim());
    }
    let data = crate::base64::decode(&b64)?;
    if let Some(c) = crc_line {
        let cb = crate::base64::decode(c.trim())?;
        if cb.len() != 3 {
            return None;
        }
        let want = ((cb[0] as u32) << 16) | ((cb[1] as u32) << 8) | cb[2] as u32;
        if want != crc24(&data) {
            return None;
        }
    }
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_format_round() {
        // tag 1 new-format, 1-byte len
        let pkt = [0xC1, 3, 1, 2, 3];
        let p = packets(&pkt).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].tag, 1);
        assert!(!p[0].old);
        assert_eq!(body(&pkt, &p[0]), vec![1, 2, 3]);
    }

    #[test]
    fn two_byte_and_five_byte_lengths() {
        // tag 11, len 300 → 2-byte form: b0 = 192 + ((300-192)>>8), b1 = (300-192)&255
        let mut pkt = vec![0xCB, 192, 108];
        pkt.extend(std::iter::repeat(0xAA).take(300));
        let p = packets(&pkt).unwrap();
        assert_eq!(p[0].size, 300);
        // 5-byte: len 70000
        let mut pkt2 = vec![0xCB, 255];
        pkt2.extend_from_slice(&[0, 1, 0x11, 0x70]); // 70000 BE
        pkt2.extend(std::iter::repeat(1).take(70000));
        let p2 = packets(&pkt2).unwrap();
        assert_eq!(p2[0].size, 70000);
    }

    #[test]
    fn partial_body_concatenates() {
        // tag 9, partial 512 (b = 224 + 9 = 233 → 1<<9 = 512) then final 1
        let mut pkt = vec![0xC9, 233];
        pkt.extend(std::iter::repeat(7).take(512));
        pkt.push(1); // final len 1
        pkt.push(9);
        let p = packets(&pkt).unwrap();
        assert!(p[0].partial);
        assert_eq!(p[0].size, 513);
        let b = body(&pkt, &p[0]);
        assert_eq!(b.len(), 513);
        assert_eq!(b[512], 9);
    }

    #[test]
    fn old_format() {
        // tag 13 (user id) old format lt=0: c = 0x80 | (13<<2) | 0 = 0xB4
        let pkt = [0xB4, 4, b'u', b's', b'r', b'!'];
        let p = packets(&pkt).unwrap();
        assert_eq!(p[0].tag, 13);
        assert!(p[0].old);
        // indeterminate lt=3 consumes to EOF
        let pkt2 = [0xB7, 9, 9, 9]; // tag 13 lt 3
        let p2 = packets(&pkt2).unwrap();
        assert_eq!(p2[0].size, 3);
    }

    #[test]
    fn crc24_vector() {
        // RFC 4880 test: "123456789" → 0x21CF02
        assert_eq!(crc24(b"123456789"), 0x21CF02);
    }

    #[test]
    fn armor_roundtrip() {
        let payload = b"hello pgp";

        let mut arm = std::string::String::from("-----BEGIN PGP MESSAGE-----\n\n");
        arm.push_str(&crate::base64::encode(payload));
        let crc = crc24(payload);
        let crc_bytes = [(crc >> 16) as u8, (crc >> 8) as u8, crc as u8];
        arm.push('\n');
        arm.push('=');
        arm.push_str(&crate::base64::encode(&crc_bytes));
        arm.push_str("\n-----END PGP MESSAGE-----\n");
        let got = dearmor(arm.as_bytes()).unwrap();
        assert_eq!(got, payload);
        // corrupt the crc → reject
        let bad = arm.replacen("=", "=AAAA", 1);
        assert!(
            dearmor(bad.as_bytes()).is_none() || dearmor(bad.as_bytes()) != Some(payload.to_vec())
        );
    }

    #[test]
    fn names_and_armor_detection() {
        assert!(is_armored(b"-----BEGIN PGP MESSAGE-----\n"));
        assert!(!is_armored(b"\xC1\x03abc"));
        assert_eq!(tag_name(1), "PKESK");
        assert_eq!(tag_name(6), "public-key");
        assert_eq!(tag_name(13), "user-id");
        assert_eq!(tag_name(60), "other");
        assert_eq!(tag_name(0), "other");
    }

    #[test]
    fn malformed_degrades() {
        assert!(packets(&[0x7F]).is_none()); // top bit clear
        assert!(packets(&[0xC1]).is_none()); // no len byte
        assert!(packets(&[0xC1, 5, 1]).is_none()); // truncated body
    }
}
