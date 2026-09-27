//! CoAP message header parsing (RFC 7252).
//!
//! Fixed header: `version:2 | type:2 | tkl:4`, `code u8`,
//! `msg_id u16BE`, then `tkl` token bytes and options/payload
//! (options are `delta:4 | len:4` with 13/14 extended forms; the
//! payload marker is `0xFF`).
//!
//! ```
//! use izanagi_kit::coap;
//! let d = [0x42, 0x01, 0x12, 0x34, 0xAB, 0xCD]; // ver1 CON GET mid 0x1234 tkl2
//! let c = coap::parse(&d).unwrap();
//! assert_eq!(c.code_class, 0);
//! assert_eq!(c.code_detail, 1);
//! assert_eq!(c.token, vec![0xAB, 0xCD]);
//! ```

use std::vec::Vec;

/// Message type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    /// 0 — confirmable.
    Confirmable,
    /// 1 — non-confirmable.
    NonConfirmable,
    /// 2 — acknowledgement.
    Ack,
    /// 3 — reset.
    Reset,
}

/// One option header (delta/decoded number, len).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionHdr {
    /// Absolute option number (delta accumulated).
    pub number: u32,
    /// Option value length.
    pub len: usize,
    /// Byte offset of the option value.
    pub at: usize,
}

/// A parsed message.
#[derive(Clone, Debug, PartialEq)]
pub struct Coap {
    /// `version` (1).
    pub version: u8,
    /// Message type.
    pub ty: Type,
    /// Token length (0..=8).
    pub tkl: u8,
    /// Code class (3 MSB of `code`).
    pub code_class: u8,
    /// Code detail (5 LSB).
    pub code_detail: u8,
    /// `msg_id`.
    pub msg_id: u16,
    /// Token bytes.
    pub token: Vec<u8>,
    /// Option headers (delta-decoded numbers).
    pub options: Vec<OptionHdr>,
    /// Payload offset when `0xFF` marker present.
    pub payload_at: Option<usize>,
}

fn ext(v: u8, d: &[u8], at: usize) -> Option<(u32, usize)> {
    match v {
        0..=12 => Some((v as u32, 0)),
        13 => Some((13 + *d.get(at)? as u32, 1)),
        14 => {
            let b = d.get(at..at.checked_add(2)?)?;
            Some((269 + (((b[0] as u32) << 8) | b[1] as u32), 2))
        }
        _ => None, // 15 = payload marker, handled by caller
    }
}

/// Parses a CoAP message: version 1, `tkl ≤ 8`, options walk to the
/// `0xFF` payload marker or end of input.
pub fn parse(d: &[u8]) -> Option<Coap> {
    if d.len() < 4 {
        return None;
    }
    let version = d[0] >> 6;
    if version != 1 {
        return None;
    }
    let ty = match (d[0] >> 4) & 3 {
        0 => Type::Confirmable,
        1 => Type::NonConfirmable,
        2 => Type::Ack,
        _ => Type::Reset,
    };
    let tkl = d[0] & 0x0F;
    if tkl > 8 {
        return None;
    }
    let code = d[1];
    let msg_id = ((d[2] as u16) << 8) | d[3] as u16;
    let mut at = 4usize;
    let token = d.get(at..at.checked_add(tkl as usize)?)?.to_vec();
    at += tkl as usize;
    let mut options = Vec::new();
    let mut num = 0u32;
    let mut payload_at = None;
    while at < d.len() {
        if d[at] == 0xFF {
            at += 1;
            payload_at = Some(at);
            break;
        }
        let hdr = d[at];
        at += 1;
        let (delta, dn) = ext(hdr >> 4, d, at)?;
        at += dn;
        let (len, ln) = ext(hdr & 0x0F, d, at)?;
        at += ln;
        let len = len as usize;
        num = num.checked_add(delta)?;
        if at.checked_add(len)? > d.len() {
            return None;
        }
        options.push(OptionHdr {
            number: num,
            len,
            at,
        });
        at += len;
        if options.len() > 1024 {
            return None;
        }
    }
    Some(Coap {
        version,
        ty,
        tkl,
        code_class: code >> 5,
        code_detail: code & 0x1F,
        msg_id,
        token,
        options,
        payload_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_message() {
        // CON GET mid=0xBEEF tkl=0 + Uri-Path opt(11, len 3) "tmp" + payload
        let d = [
            0x40, 0x01, 0xBE, 0xEF, // header, tkl 0
            0xB3, b't', b'm', b'p', // delta 11, len 3
            0xFF, 0x01, 0x02, // payload marker + payload
        ];
        let c = parse(&d).unwrap();
        assert_eq!(c.ty, Type::Confirmable);
        assert_eq!(c.msg_id, 0xBEEF);
        assert_eq!(c.options.len(), 1);
        assert_eq!(c.options[0].number, 11);
        assert_eq!(c.options[0].len, 3);
        assert_eq!(c.payload_at, Some(9));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0x00]).is_none());
        assert!(parse(&[0xC0, 0, 0, 0]).is_none()); // version 3
        assert!(parse(&[0x49, 0, 0, 0]).is_none()); // tkl 9
                                                    // option extends past end
        assert!(parse(&[0x40, 0x01, 0, 0, 0xB3, b't']).is_none());
    }
}
