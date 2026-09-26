//! NTP — RFC 5905 48-byte packet header.
//!
//! Byte 0 packs `LI(2) | VN(3) | Mode(3)`; then `stratum`, `poll`
//! (log2 exponent, kept signed raw), `precision`; `root_delay`/
//! `root_dispersion` are 16.16 fixed-point kept as raw `u32`; `refid`
//! 4 bytes; four 64-bit NTP timestamps kept as `{seconds, fraction}`
//! pairs (integer only).
//!
//! ```
//! use izanagi_kit::ntp::parse;
//!
//! let mut p = [0u8; 48];
//! p[0] = 0x24; // VN=4, mode=4 (server)
//! p[1] = 2;    // stratum 2
//! p[2] = 6;    // poll 2^6 s
//! p[40] = 0xE8; p[41] = 0x75; p[42] = 0x93; p[43] = 0x70; // xmit sec
//! let n = parse(&p).unwrap();
//! assert_eq!(n.version(), 4);
//! assert_eq!(n.mode(), 4);
//! assert_eq!(n.transmit.seconds, 0xE8759370);
//! ```

/// A 64-bit NTP timestamp split into integer seconds + fraction.
#[derive(Clone, Debug)]
pub struct Timestamp {
    /// Seconds since 1900-01-01.
    pub seconds: u32,
    /// Fraction (1/2³² s units).
    pub fraction: u32,
}

/// A parsed NTP packet header.
#[derive(Clone, Debug)]
pub struct Ntp {
    /// Packed leap-indicator/version/mode byte.
    pub li_vn_mode: u8,
    /// Stratum (0=unspec, 1=primary, 16=unsync'd).
    pub stratum: u8,
    /// Poll interval log2 (raw signed byte as u8).
    pub poll: u8,
    /// Clock precision log2 (raw).
    pub precision: u8,
    /// Root delay — 16.16 fixed-point bits.
    pub root_delay_bits: u32,
    /// Root dispersion — raw bits.
    pub root_dispersion_bits: u32,
    /// `refid` 4 bytes (ASCII kiss code or IPv4).
    pub refid: [u8; 4],
    /// Reference timestamp.
    pub reference: Timestamp,
    /// Origin timestamp.
    pub origin: Timestamp,
    /// Receive timestamp.
    pub receive: Timestamp,
    /// Transmit timestamp.
    pub transmit: Timestamp,
}

impl Ntp {
    /// Leap indicator (0-3).
    pub fn leap(&self) -> u8 {
        self.li_vn_mode >> 6
    }

    /// Version (1-4 observed; 4 = RFC 5905).
    pub fn version(&self) -> u8 {
        (self.li_vn_mode >> 3) & 7
    }

    /// Mode (3=client, 4=server, 5=broadcast).
    pub fn mode(&self) -> u8 {
        self.li_vn_mode & 7
    }
}

fn u32be(d: &[u8], o: usize) -> Option<u32> {
    let b = d.get(o..o + 4)?;
    Some(((b[0] as u32) << 24) | ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | b[3] as u32)
}

fn ts(d: &[u8], o: usize) -> Option<Timestamp> {
    Some(Timestamp {
        seconds: u32be(d, o)?,
        fraction: u32be(d, o + 4)?,
    })
}

/// Parse a 48-byte NTP packet. `None` when shorter or version is 0.
pub fn parse(d: &[u8]) -> Option<Ntp> {
    if d.len() < 48 {
        return None;
    }
    let li_vn_mode = d[0];
    if (li_vn_mode >> 3) & 7 == 0 {
        return None;
    }
    Some(Ntp {
        li_vn_mode,
        stratum: d[1],
        poll: d[2],
        precision: d[3],
        root_delay_bits: u32be(d, 4)?,
        root_dispersion_bits: u32be(d, 8)?,
        refid: [d[12], d[13], d[14], d[15]],
        reference: ts(d, 16)?,
        origin: ts(d, 24)?,
        receive: ts(d, 32)?,
        transmit: ts(d, 40)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut p = [0u8; 48];
        p[0] = 0x1B; // LI=0 VN=3 mode=3 client
        p[1] = 3;
        p[2] = 10;
        p[47] = 0xFF;
        let n = parse(&p).unwrap();
        assert_eq!((n.leap(), n.version(), n.mode()), (0, 3, 3));
        assert_eq!(n.stratum, 3);
        assert_eq!(n.transmit.fraction, 0xFF);
        assert_eq!(n.refid, [0; 4]);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 10]).is_none());
        let mut p = [0u8; 48];
        p[0] = 0; // version 0
        assert!(parse(&p).is_none());
    }
}
