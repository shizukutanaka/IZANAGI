//! OCSP response envelope (RFC 6960 §4.2.1), DER form:
//! `OCSPResponse ::= SEQUENCE { responseStatus ENUMERATED,
//!   responseBytes [0] EXPLICIT ResponseBytes OPTIONAL }` where
//! `ResponseBytes` is `SEQUENCE { responseType OID, response OCTET }`.
//! `1.3.6.1.5.5.7.48.1.1` identifies a BasicOCSPResponse.
//!
//! ```
//! use izanagi_kit::{ocsp, der};
//!
//! // SEQUENCE{ ENUM 0 (successful), [0]{ SEQ{ OID basic, OCTET } } }
//! let rb = der::encode(0xa0, &der::encode(0x30, &[
//!     der::encode(0x06, &[0x2b,0x06,0x01,0x05,0x05,0x07,0x30,0x01,0x01]),
//!     der::encode(0x04, &[0xde, 0xad]),
//! ].concat()));
//! let d = der::encode(0x30, &[der::encode(0x0a, &[0x00]), rb].concat());
//! let o = ocsp::parse(&d).unwrap();
//! assert_eq!(o.status, ocsp::Status::Successful);
//! assert!(o.is_basic);
//! ```

use crate::der;
use std::vec::Vec;

/// OCSP `responseStatus` values (RFC 6960).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// `0` — request understood, response valid
    Successful,
    /// `1` — malformed request
    MalformedRequest,
    /// `2` — internal error
    InternalError,
    /// `3` — try again later
    TryLater,
    /// `5` — request must be signed
    SigRequired,
    /// `6` — requester not authorized
    Unauthorized,
    /// Any other enumerated value.
    Other(i64),
}

impl Status {
    /// Map the ENUMERATED integer to a `Status`.
    pub fn from_i64(v: i64) -> Status {
        match v {
            0 => Status::Successful,
            1 => Status::MalformedRequest,
            2 => Status::InternalError,
            3 => Status::TryLater,
            5 => Status::SigRequired,
            6 => Status::Unauthorized,
            other => Status::Other(other),
        }
    }
}

/// A parsed OCSP response envelope (the inner BasicOCSPResponse stays
/// opaque in `response`).
#[derive(Clone, Debug)]
pub struct Ocsp {
    /// Response status.
    pub status: Status,
    /// `responseType` OID in dotted form, when `responseBytes` is
    /// present.
    pub response_oid: Option<std::string::String>,
    /// True when `responseType` is `id-pkix-ocsp-basic`.
    pub is_basic: bool,
    /// Raw inner response bytes (the DER BasicOCSPResponse).
    pub response: Vec<u8>,
}

/// Parse an OCSP response: `SEQUENCE { ENUMERATED status, [0]
/// ResponseBytes? }`. A non-successful status may legally have no
/// responseBytes.
pub fn parse(d: &[u8]) -> Option<Ocsp> {
    let top = der::parse(d)?;
    let top = top.first()?;
    if top.tag != 0x10 || top.cls != 0 || !top.constructed {
        return None;
    }
    let parts = top.children()?;
    if parts.is_empty() || parts.len() > 2 {
        return None;
    }
    let st = parts.first()?;
    if st.tag != 0x0a {
        return None; // ENUMERATED
    }
    let status = Status::from_i64(st.integer()?);
    let mut response_oid = None;
    let mut is_basic = false;
    let mut response = Vec::new();
    if let Some(rb) = parts.get(1) {
        if rb.cls != 2 || rb.tag != 0 {
            return None;
        }
        // [0] EXPLICIT wraps a SEQUENCE
        let inner = der::parse(&rb.content)?;
        let seq = inner.first().filter(|t| t.tag == 0x10)?;
        let rb_parts = seq.children()?;
        if rb_parts.len() != 2 {
            return None;
        }
        let oid_t = rb_parts.first()?;
        if oid_t.tag != 0x06 {
            return None;
        }
        let oid = oid_t.oid()?;
        is_basic = oid == "1\x2e3\x2e6\x2e1\x2e5\x2e5\x2e7\x2e48\x2e1\x2e1";
        response_oid = Some(oid);
        let oct = rb_parts.get(1)?;
        if oct.tag != 0x04 {
            return None;
        }
        response = oct.content.clone();
    }
    Some(Ocsp {
        status,
        response_oid,
        is_basic,
        response,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_basic() {
        let rb = der::encode(
            0xa0,
            &der::encode(
                0x30,
                &[
                    der::encode(
                        0x06,
                        &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x01],
                    ),
                    der::encode(0x04, &[0xde, 0xad]),
                ]
                .concat(),
            ),
        );
        let d = der::encode(0x30, &[der::encode(0x0a, &[0x00]), rb].concat());
        let o = parse(&d).unwrap();
        assert_eq!(o.status, Status::Successful);
        assert_eq!(o.response_oid.as_deref(), Some("1.3.6.1.5.5.7.48.1.1"));
        assert!(o.is_basic);
        assert_eq!(o.response, vec![0xde, 0xad]);
    }

    #[test]
    fn try_later_no_body() {
        let d = der::encode(0x30, &der::encode(0x0a, &[0x03]));
        let o = parse(&d).unwrap();
        assert_eq!(o.status, Status::TryLater);
        assert!(o.response_oid.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&der::encode(0x04, b"x")).is_none());
        // wrong first tag (not ENUMERATED)
        let bad = der::encode(0x30, &der::encode(0x02, &[0x00]));
        assert!(parse(&bad).is_none());
        assert_eq!(Status::from_i64(4), Status::Other(4));
    }
}
