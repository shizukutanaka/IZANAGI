//! ESP — IPsec Encapsulating Security Payload (RFC 4303): 32-bit SPI +
//! 32-bit sequence number, then encrypted payload, then trailer
//! (padding + pad_len + next_header) and optional ICV. SPI values
//! 1–255 are reserved; 0 is only legal internally.
//!
//! ```
//! // SPI 0x1000, seq 7, 12-byte payload, 2-byte pad, next = TCP
//! let d = [
//!     0x00u8, 0x00, 0x10, 0x00, // SPI
//!     0x00, 0x00, 0x00, 0x07, // seq
//!     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // ciphertext
//!     0x02, 0x06, // pad_len 2, next_header 6 (TCP)
//! ];
//! let e = izanagi_kit::esp::parse(&d).unwrap();
//! assert_eq!(e.spi, 0x1000);
//! assert_eq!(e.sequence, 7);
//! assert_eq!(e.next_header, Some(6));
//! ```

/// A parsed ESP packet (header + trailer; payload bytes stay opaque).
#[derive(Clone, Debug)]
pub struct Esp {
    /// Security Parameter Index — selectors pair this with dst addr.
    pub spi: u32,
    /// Anti-replay sequence counter.
    pub sequence: u32,
    /// Byte length of the encrypted payload region (between the 8-byte
    /// header and the trailer).
    pub payload_len: usize,
    /// Trailer `pad_len` when the buffer ends with a plausible trailer.
    pub pad_len: Option<u8>,
    /// Trailer `next_header` (IP protocol number) when plausible.
    pub next_header: Option<u8>,
}

/// Parse an ESP packet: at least the 8-byte header, and when ≥ 2 more
/// bytes are present the trailing two bytes are exposed as
/// `pad_len`/`next_header` if `pad_len + 2 + payload` fits the buffer.
/// The ICV length is not knowable without SA state — callers pass the
/// portion they received.
pub fn parse(d: &[u8]) -> Option<Esp> {
    if d.len() < 8 {
        return None;
    }
    let spi = (u32::from(*d.first()?) << 24)
        | (u32::from(*d.get(1)?) << 16)
        | (u32::from(*d.get(2)?) << 8)
        | u32::from(*d.get(3)?);
    let sequence = (u32::from(*d.get(4)?) << 24)
        | (u32::from(*d.get(5)?) << 16)
        | (u32::from(*d.get(6)?) << 8)
        | u32::from(*d.get(7)?);
    if spi == 0 {
        return None; // SPI 0 is never on the wire
    }
    let (pad_len, next_header) = if d.len() >= 10 {
        let pl = *d.get(d.len() - 2)?;
        let nh = *d.last()?;
        // pad must not cover the header itself
        let usable = usize::from(pl).checked_add(10)? <= d.len();
        (usable.then_some(pl), usable.then_some(nh))
    } else {
        (None, None)
    };
    Some(Esp {
        spi,
        sequence,
        payload_len: d.len() - 8,
        pad_len,
        next_header,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_only() {
        let d = [0xde, 0xad, 0xbe, 0xef, 0, 0, 0, 1];
        let e = parse(&d).unwrap();
        assert_eq!(e.spi, 0xdeadbeef);
        assert_eq!(e.sequence, 1);
        assert_eq!(e.payload_len, 0);
        assert_eq!(e.next_header, None);
    }

    #[test]
    fn trailer_plausible() {
        let mut d = vec![0, 0, 0, 9, 0, 0, 0, 2];
        d.extend_from_slice(&[1, 2, 3, 4]); // payload
        d.extend_from_slice(&[0, 0x01, 0x04]); // pad byte + pad_len 1 + UDP
        let e = parse(&d).unwrap();
        assert_eq!(e.pad_len, Some(1));
        assert_eq!(e.next_header, Some(4));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0, 0, 0, 0, 0, 0, 0, 1]).is_none()); // SPI 0
        assert!(parse(&[0, 0, 0, 1]).is_none());
    }
}
