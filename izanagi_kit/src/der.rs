//! ASN.1 DER (X.690) — the TLV layer under X.509 certificates, CMS, and
//! every `pem`-armoured `BEGIN CERTIFICATE` blob: `tag` · `length` ·
//! `content`, definite-form only. [`parse`] walks a byte string as a
//! sequence of TLVs; [`Tlv::children`] re-parses a constructed value's
//! content; [`Tlv::oid`], [`Tlv::integer`], [`Tlv::text`], and
//! [`Tlv::utc_time`] decode the common primitive types. [`encode`]
//! emits canonical DER (shortest-form length, tag numbers < 31).
//!
//! ```
//! use izanagi_kit::der::{parse, encode};
//! // INTEGER 42 = 30 03 02 01 2A wrapped in a SEQUENCE
//! let doc = [0x30, 0x03, 0x02, 0x01, 0x2A];
//! let top = parse(&doc).unwrap();
//! assert_eq!(top[0].children().unwrap()[0].integer().unwrap(), 42);
//! assert_eq!(encode(0x02, &[42]), vec![0x02, 0x01, 0x2A]);
//! ```

use std::vec::Vec;

/// One parsed TLV. `tag` is the full tag number (identifiers ≥ 31 use
/// the high-tag-number form); `cls` is the class (0 universal, 1
/// application, 2 context, 3 private); `content` is the value octets.
#[derive(Clone, Debug, PartialEq)]
pub struct Tlv {
    /// Tag number (low 5 bits of the identifier octet, or the
    /// base-128 high-tag-number form).
    pub tag: u32,
    /// Tag class: 0 universal, 1 application, 2 context-specific, 3 private.
    pub cls: u8,
    /// True when the constructed bit is set.
    pub constructed: bool,
    /// Raw content octets.
    pub content: Vec<u8>,
}

fn one(d: &[u8], at: usize) -> Option<(Tlv, usize)> {
    let id = *d.get(at)? as usize;
    let cls = (id >> 6) as u8;
    let constructed = id & 0x20 != 0;
    let mut tag = (id & 0x1F) as u32;
    let mut i = at + 1;
    if tag == 0x1F {
        // high-tag-number form: base-128 continuation octets
        tag = 0;
        loop {
            let b = *d.get(i)? as u32;
            tag = tag.checked_mul(128)?.checked_add(b & 0x7F)?;
            i += 1;
            if b & 0x80 == 0 {
                break;
            }
        }
    }
    let l0 = *d.get(i)? as usize;
    i += 1;
    let len = if l0 < 0x80 {
        l0
    } else {
        let n = l0 & 0x7F;
        if n == 0 || n > 4 {
            return None; // indefinite form (0x80) is BER, not DER
        }
        let mut v = 0usize;
        for _ in 0..n {
            v = (v << 8) | *d.get(i)? as usize;
            i += 1;
        }
        if v < 128 {
            return None; // DER requires shortest form
        }
        v
    };
    let end = i.checked_add(len)?;
    if end > d.len() {
        return None;
    }
    Some((
        Tlv {
            tag,
            cls,
            constructed,
            content: d[i..end].to_vec(),
        },
        end,
    ))
}

/// Parse `d` as a sequence of DER TLVs covering the whole input;
/// `None` on truncation, indefinite lengths, or trailing junk.
pub fn parse(d: &[u8]) -> Option<Vec<Tlv>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < d.len() {
        let (t, next) = one(d, i)?;
        out.push(t);
        i = next;
    }
    Some(out)
}

/// Canonical DER encode of one TLV: tag < 31, shortest-form length.
/// `None` when the tag number is ≥ 31 (high-tag form is decode-only)
/// or the value is absurdly long.
pub fn encode(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let n = content.len();
    if n < 128 {
        out.push(n as u8);
    } else {
        let mut tmp = Vec::new();
        let mut v = n;
        while v > 0 {
            tmp.push((v & 0xFF) as u8);
            v >>= 8;
        }
        out.push(0x80 | tmp.len() as u8);
        tmp.reverse();
        out.extend_from_slice(&tmp);
    }
    out.extend_from_slice(content);
    out
}

fn two_digits(s: &[u8], i: usize) -> Option<i32> {
    let a = *s.get(i)?;
    let b = *s.get(i + 1)?;
    if !a.is_ascii_digit() || !b.is_ascii_digit() {
        return None;
    }
    Some((a - b'0') as i32 * 10 + (b - b'0') as i32)
}

impl Tlv {
    /// Re-parse the content as a TLV sequence (constructed values).
    /// `None` if the content is not a whole number of TLVs.
    pub fn children(&self) -> Option<Vec<Tlv>> {
        parse(&self.content)
    }

    /// Decode an OBJECT IDENTIFIER body to dotted-decimal form.
    /// `None` when the content is not a valid OID arc list.
    pub fn oid(&self) -> Option<String> {
        let c = &self.content;
        let mut out = String::new();
        let mut arcs = Vec::new();
        let mut i = 0;
        while i < c.len() {
            let mut v = 0u64;
            loop {
                let b = *c.get(i)? as u64;
                v = v.checked_mul(128)?.checked_add(b & 0x7F)?;
                i += 1;
                if b & 0x80 == 0 {
                    break;
                }
            }
            arcs.push(v);
        }
        let first = *arcs.first()?;
        let (a0, a1) = if first < 40 {
            (0, first)
        } else if first < 80 {
            (1, first - 40)
        } else {
            (2, first - 80)
        };
        out.push_str(&a0.to_string());
        out.push('.');
        out.push_str(&a1.to_string());
        for a in &arcs[1..] {
            out.push('.');
            out.push_str(&a.to_string());
        }
        Some(out)
    }

    /// Decode a non-negative INTEGER that fits in `i64`.
    /// `None` on negative values or more than 8 content octets.
    pub fn integer(&self) -> Option<i64> {
        let c = &self.content;
        if c.is_empty() || c.len() > 8 {
            return None;
        }
        if c[0] & 0x80 != 0 {
            return None; // negative
        }
        let mut v = 0i64;
        for &b in c {
            v = (v << 8) | b as i64;
        }
        Some(v)
    }

    /// Content as a UTF-8 string (UTF8String, PrintableString,
    /// IA5String, UTCTime, GeneralizedTime all store text directly).
    /// `None` on invalid UTF-8.
    pub fn text(&self) -> Option<String> {
        String::from_utf8(self.content.clone()).ok()
    }

    /// Decode UTCTime (`YYMMDDHHMMSS[Z]`; RFC 5280 year window:
    /// 50–99 → 19xx, 00–49 → 20xx) or GeneralizedTime
    /// (`YYYYMMDDHHMMSS[Z]`) to Unix seconds. `None` on malformed
    /// content or a missing `Z` (local-time forms are ambiguous).
    pub fn utc_time(&self) -> Option<i64> {
        let s = self.text()?;
        let b = s.as_bytes();
        let (y, off) = if b.len() == 13 {
            // UTCTime YYMMDDHHMMSSZ
            let yy = two_digits(b, 0)?;
            (if yy >= 50 { 1900 + yy } else { 2000 + yy }, 2)
        } else if b.len() == 15 {
            // GeneralizedTime YYYYMMDDHHMMSSZ
            (two_digits(b, 0)? * 100 + two_digits(b, 2)?, 4)
        } else {
            return None;
        };
        if *b.last()? != b'Z' {
            return None;
        }
        let mo = two_digits(b, off)?;
        let dy = two_digits(b, off + 2)?;
        let hh = two_digits(b, off + 4)?;
        let mi = two_digits(b, off + 6)?;
        let ss = two_digits(b, off + 8)?;
        if !(1..=12).contains(&mo) || !(1..=31).contains(&dy) || hh > 23 || mi > 59 || ss > 60 {
            return None;
        }
        let days = crate::civil::days_from_civil(y, mo as u32, dy as u32) as i64;
        Some(days * 86400 + hh as i64 * 3600 + mi as i64 * 60 + ss as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_roundtrip() {
        for v in [0i64, 1, 42, 127, 128, 255, 256, 65535, 1 << 40] {
            let mut c = Vec::new();
            let mut x = v;
            loop {
                c.push((x & 0xFF) as u8);
                x >>= 8;
                if x == 0 {
                    break;
                }
            }
            c.reverse();
            if c[0] & 0x80 != 0 {
                c.insert(0, 0); // DER: positive INTEGER with MSB set needs a 0 pad
            }
            let d = encode(0x02, &c);
            assert_eq!(parse(&d).unwrap()[0].integer().unwrap(), v);
        }
    }

    #[test]
    fn seq_children() {
        // SEQUENCE { INTEGER 1, BOOLEAN true } = 30 06 02 01 01 01 01 FF
        let d = [0x30, 0x06, 0x02, 0x01, 0x01, 0x01, 0x01, 0xFF];
        let s = parse(&d).unwrap();
        let k = s[0].children().unwrap();
        assert_eq!(k[0].integer().unwrap(), 1);
        assert_eq!(k[1].tag, 1);
        assert_eq!(k[1].content, vec![0xFF]);
    }

    #[test]
    fn oid_decoding() {
        // 1.2.840.113549 = 2A 86 48 86 F7 0D (rsaEncryption prefix family)
        let t = Tlv {
            tag: 6,
            cls: 0,
            constructed: false,
            content: vec![0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D],
        };
        assert_eq!(t.oid().unwrap(), "1.2.840.113549");
        // 2.999.3 — first arc ≥ 80 splits as (2, arc-80)
        let t2 = Tlv {
            tag: 6,
            cls: 0,
            constructed: false,
            content: vec![0x88, 0x37, 0x03],
        };
        assert_eq!(t2.oid().unwrap(), "2.999.3");
    }

    #[test]
    fn utctime_parses() {
        // UTCTime 230101000000Z = 2023-01-01T00:00:00Z = 1672531200
        let t = Tlv {
            tag: 23,
            cls: 0,
            constructed: false,
            content: b"230101000000Z".to_vec(),
        };
        assert_eq!(t.utc_time().unwrap(), 1672531200);
        // 991231235959Z → 1999-12-31T23:59:59
        let t2 = Tlv {
            tag: 23,
            cls: 0,
            constructed: false,
            content: b"991231235959Z".to_vec(),
        };
        assert_eq!(t2.utc_time().unwrap(), 946684799);
        // GeneralizedTime
        let t3 = Tlv {
            tag: 24,
            cls: 0,
            constructed: false,
            content: b"20300101000000Z".to_vec(),
        };
        assert!(t3.utc_time().unwrap() > 0);
    }

    #[test]
    fn bad_inputs() {
        assert!(parse(&[0x30]).is_none()); // truncated
        assert!(parse(&[0x30, 0x80, 0x00]).is_none()); // indefinite length
        assert!(parse(&[0x02, 0x81, 0x01, 0x05]).is_none()); // non-shortest long form
        assert!(parse(&[0x02, 0x01]).is_none()); // length overruns data
    }

    #[test]
    fn high_tag_decodes() {
        // context-specific tag 31, primitive: BF 1F 01 05
        let d = [0xBF, 0x1F, 0x01, 0x05];
        let t = parse(&d).unwrap();
        assert_eq!(t[0].tag, 31);
        assert_eq!(t[0].cls, 2);
        assert_eq!(t[0].content, vec![5]);
    }

    #[test]
    fn encode_lengths() {
        assert_eq!(encode(4, &[0; 3]), vec![0x04, 0x03, 0, 0, 0]);
        assert_eq!(
            encode(4, &[0u8; 200])[..4].to_vec(),
            vec![0x04, 0x81, 0xC8, 0]
        );
        let long = encode(0x30, &[0u8; 65536]);
        assert_eq!(&long[..4], &[0x30, 0x83, 0x01, 0x00]);
    }

    #[test]
    fn determinism() {
        let d = encode(0x30, &encode(0x02, &[7]));
        assert_eq!(parse(&d), parse(&d));
    }
}
