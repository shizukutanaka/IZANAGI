//! ESP — IPsec Encapsulating Security Payload (RFC 4303): 32-bit SPI +
//! 32-bit sequence number, then the opaque encrypted region (payload +
//! padding + trailer + optional ICV). The trailer's `pad_len` and
//! `next_header` live inside the ciphertext, so a raw parser must not
//! guess them — they only become meaningful after authentication and
//! decryption, and are therefore not reported here.
//!
//! ```
//! // SPI 0x1000, sequence 7, followed by opaque ciphertext
//! let d = [
//!     0x00u8, 0x00, 0x10, 0x00, // SPI
//!     0x00, 0x00, 0x00, 0x07, // sequence
//!     1, 2, 3, 4, // ciphertext (payload + trailer)
//! ];
//! let e = izanagi_kit::esp::parse(&d).unwrap();
//! assert_eq!(e.spi, 0x1000);
//! assert_eq!(e.sequence, 7);
//! assert_eq!(e.payload_len, 4);
//! ```

/// A parsed ESP packet: header fields only — everything after the
/// 8-byte header is encrypted and stays opaque.
#[derive(Clone, Debug)]
pub struct Esp {
    /// Security Parameter Index — selectors pair this with dst addr.
    /// SPI 0 is never on the wire (1–255 are also reserved).
    pub spi: u32,
    /// Anti-replay sequence counter.
    pub sequence: u32,
    /// Bytes after the 8-byte header: encrypted payload + padding +
    /// trailer + optional ICV. Length of the ICV depends on the SA and
    /// is not separable here.
    pub payload_len: usize,
}

/// Parse an ESP header: requires the 8-byte SPI+sequence prefix and a
/// nonzero SPI. The remainder is ciphertext and is reported by length
/// only.
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
        return None;
    }
    Some(Esp {
        spi,
        sequence,
        payload_len: d.len() - 8,
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
    }

    #[test]
    fn ciphertext_stays_opaque() {
        // Whatever the trailing bytes happen to be, they are not
        // decoded as trailer fields.
        let d = [0, 0, 0, 9, 0, 0, 0, 2, 1, 2, 3, 0, 4];
        let e = parse(&d).unwrap();
        assert_eq!(e.payload_len, 5);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0, 0, 0, 0, 0, 0, 0, 1]).is_none()); // SPI 0
        assert!(parse(&[0, 0, 0, 1]).is_none());
    }
}
