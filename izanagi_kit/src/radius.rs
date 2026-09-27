//! Minimal reader for RADIUS packets (RFC 2865): `code id len`
//! header + 16-byte authenticator + attribute TLVs (`type len value`,
//! `len >= 2`, attribute 26 carries vendor-specific VSA inner TLVs).
//!
//! ```
//! use izanagi_kit::radius::parse;
//!
//! let mut d = vec![1u8, 7, 0, 26]; // Access-Request, id 7, len 26
//! d.extend_from_slice(&[0xAA; 16]); // authenticator
//! d.extend_from_slice(&[1, 6, b'u', b's', b'e', b'r']); // User-Name
//! let p = parse(&d).unwrap();
//! assert_eq!(p.code, 1);
//! assert_eq!(p.attributes.len(), 1);
//! ```

/// One RADIUS attribute (`type len value`).
#[derive(Debug)]
pub struct Attr {
    /// Attribute type (1=User-Name, 26=Vendor-Specific, ...).
    pub kind: u8,
    /// Byte offset of the attribute value (after the len byte).
    pub value_at: usize,
    /// Value length (`len - 2`).
    pub value_len: usize,
}

/// A parsed RADIUS packet.
#[derive(Debug)]
pub struct Radius {
    /// Message code (1=Access-Request, 2=Access-Accept, 3=Reject,
    /// 4/5=Accounting, 11=Access-Challenge).
    pub code: u8,
    /// Identifier byte (request/response matching).
    pub id: u8,
    /// Declared `length` field (== wire length on success).
    pub length: u16,
    /// 16-byte authenticator.
    pub authenticator: [u8; 16],
    /// Attribute headers (values are byte ranges into the input).
    pub attributes: Vec<Attr>,
}

/// Parse a RADIUS packet. `None` on short input, `length < 20`,
/// `length` mismatching the wire size, or an attribute with `len < 2`.
pub fn parse(d: &[u8]) -> Option<Radius> {
    let length = {
        let hi = *d.get(2)? as u16;
        let lo = *d.get(3)? as u16;
        (hi << 8) | lo
    };
    if length < 20 || length as usize != d.len() {
        return None;
    }
    let mut authenticator = [0u8; 16];
    authenticator.copy_from_slice(&d[4..20]);
    let mut attributes = Vec::new();
    let mut at = 20;
    while at < d.len() {
        let kind = *d.get(at)?;
        let len = *d.get(at + 1)? as usize;
        if len < 2 || at + len > d.len() {
            return None;
        }
        attributes.push(Attr {
            kind,
            value_at: at + 2,
            value_len: len - 2,
        });
        at += len;
    }
    Some(Radius {
        code: d[0],
        id: d[1],
        length,
        authenticator,
        attributes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut d = vec![1u8, 7, 0, 26];
        d.extend_from_slice(&[0xAA; 16]);
        d.extend_from_slice(&[1, 6, b'u', b's', b'e', b'r']);
        let p = parse(&d).unwrap();
        assert_eq!(p.code, 1);
        assert_eq!(p.id, 7);
        assert_eq!(p.length, 26);
        assert_eq!(p.authenticator[0], 0xAA);
        assert_eq!(p.attributes.len(), 1);
        assert_eq!(p.attributes[0].kind, 1);
        assert_eq!(p.attributes[0].value_len, 4);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        // length mismatch (declares 24, wire is 20)
        let mut d = vec![2u8, 1, 0, 24];
        d.extend_from_slice(&[0u8; 16]);
        assert!(parse(&d).is_none());
        // length < 20
        let mut d = vec![2u8, 1, 0, 10];
        d.extend_from_slice(&[0u8; 6]);
        assert!(parse(&d).is_none());
        // attribute len < 2
        let mut d = vec![1u8, 7, 0, 22];
        d.extend_from_slice(&[0xAA; 16]);
        d.extend_from_slice(&[1, 1]);
        assert!(parse(&d).is_none());
    }
}
