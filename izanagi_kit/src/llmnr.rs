//! LLMNR (RFC 4795) message classification over `dns`.
//!
//! LLMNR uses the DNS wire format on UDP 5355 but forbids the
//! recursion flags (`RD`/`RA`) and DNSSEC bits — a message that sets
//! `RD` is DNS, not LLMNR. This module delegates the decode to
//! `crate::dns` and applies the flag policy.
//!
//! ```
//! use izanagi_kit::llmnr;
//! // query for "test": tid 1, flags 0 (no RD!), 1 question
//! let mut d = vec![0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
//! d.extend_from_slice(&[4, b't', b'e', b's', b't', 0]);
//! d.extend_from_slice(&[0, 1, 0, 1]); // A IN
//! let l = llmnr::parse(&d).unwrap();
//! assert_eq!(l.questions.len(), 1);
//! ```

/// LLMNR port (UDP/TCP 5355).
pub const PORT: u16 = 5355;

/// A parsed LLMNR message (wraps the DNS wire format).
#[derive(Clone, Debug, PartialEq)]
pub struct Llmnr {
    /// Transaction id.
    pub id: u16,
    /// True when the QR flag marks a response.
    pub is_response: bool,
    /// Question count.
    pub question_count: u16,
    /// Answer + authority + additional totals.
    pub answer_count: u32,
    /// Decoded question names.
    pub questions: Vec<std::string::String>,
}

/// Parses an LLMNR message: DNS wire format with `RD`/`RA`/`CD`
/// bits clear (their presence means the packet is really DNS).
pub fn parse(d: &[u8]) -> Option<Llmnr> {
    let m = crate::dns::parse(d)?;
    // flags u16BE at offset 2: QR 0x8000, RD 0x0100, RA 0x0080, CD 0x0010
    let flags = ((d[2] as u16) << 8) | d[3] as u16;
    if flags & (0x0100 | 0x0080 | 0x0010) != 0 {
        return None;
    }
    let mut questions = Vec::new();
    for q in &m.questions {
        questions.push(q.name.clone());
    }
    Some(Llmnr {
        id: m.id,
        is_response: flags & 0x8000 != 0,
        question_count: m.questions.len() as u16,
        answer_count: (m.answers.len() + m.authorities.len() + m.additionals.len()) as u32,
        questions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn query(flags_hi: u8) -> Vec<u8> {
        let mut d = vec![0xAB, 0xCD, flags_hi, 0, 0, 1, 0, 0, 0, 0, 0, 0];
        d.extend_from_slice(&[3, b'w', b'w', b'w', 0]);
        d.extend_from_slice(&[0, 1, 0, 1]);
        d
    }

    #[test]
    fn parses_query() {
        let l = parse(&query(0)).unwrap();
        assert_eq!(l.id, 0xABCD);
        assert_eq!(l.questions, vec!["www".to_string()]);
        assert!(!l.is_response);
    }

    #[test]
    fn rejects_dns_flags() {
        assert!(parse(&query(0x01)).is_none()); // RD set → DNS, not LLMNR
        assert!(parse(&[0u8; 8]).is_none());
    }
}
