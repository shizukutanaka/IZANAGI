//! EAP (RFC 3748) — Extensible Authentication Protocol frames.
//!
//! `u8 code | u8 id | u16 len | [u8 type for Request/Response] | data`.
//! Codes: 1 Request, 2 Response, 3 Success, 4 Failure.
//!
//! ```
//! let d = [1u8, 42, 0, 5, 1]; // Request id=42, type=1 (Identity)
//! let e = izanagi_kit::eap::parse(&d).unwrap();
//! assert_eq!(e.code, izanagi_kit::eap::Code::Request);
//! assert_eq!(e.id, 42);
//! assert_eq!(e.eap_type, Some(1));
//! ```

/// EAP packet code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// 1 — request.
    Request,
    /// 2 — response.
    Response,
    /// 3 — success (no type byte).
    Success,
    /// 4 — failure (no type byte).
    Failure,
}

/// Parsed EAP frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eap {
    /// Packet code.
    pub code: Code,
    /// Identifier — matches requests to responses.
    pub id: u8,
    /// EAP method type byte (Request/Response only: 1 Identity, 4 MD5, 13 TLS, …).
    pub eap_type: Option<u8>,
    /// Data octets after the header (and type byte when present).
    pub data_len: usize,
}

/// Parse an EAP frame; `None` on bad code or inconsistent length.
pub fn parse(d: &[u8]) -> Option<Eap> {
    if d.len() < 4 {
        return None;
    }
    let code = match d[0] {
        1 => Code::Request,
        2 => Code::Response,
        3 => Code::Success,
        4 => Code::Failure,
        _ => return None,
    };
    let len = ((d[2] as usize) << 8) | d[3] as usize;
    if len < 4 || len > d.len() {
        return None;
    }
    let (eap_type, data_len) = match code {
        Code::Request | Code::Response => {
            if len < 5 {
                return None;
            }
            (Some(d[4]), len - 5)
        }
        _ => (None, len - 4),
    };
    Some(Eap {
        code,
        id: d[1],
        eap_type,
        data_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_identity() {
        let e = parse(&[1, 7, 0, 9, 1, b't', b'e', b's', b't']).unwrap();
        assert_eq!(e.code, Code::Request);
        assert_eq!(e.id, 7);
        assert_eq!(e.eap_type, Some(1));
        assert_eq!(e.data_len, 4);
    }

    #[test]
    fn response_tls() {
        let e = parse(&[2, 9, 0, 6, 13, 0x80]).unwrap();
        assert_eq!(e.code, Code::Response);
        assert_eq!(e.eap_type, Some(13)); // EAP-TLS
    }

    #[test]
    fn success_failure() {
        let s = parse(&[3, 1, 0, 4]).unwrap();
        assert_eq!(s.code, Code::Success);
        assert_eq!(s.eap_type, None);
        let f = parse(&[4, 1, 0, 4]).unwrap();
        assert_eq!(f.code, Code::Failure);
        assert_eq!(f.data_len, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0, 0, 0, 4]).is_none()); // bad code
        assert!(parse(&[1, 0, 0, 3]).is_none()); // len < 4
        assert!(parse(&[3, 0, 0, 100]).is_none()); // len > input
        assert!(parse(&[1, 0, 0, 4]).is_none()); // request without type byte
    }
}
