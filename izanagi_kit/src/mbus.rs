//! Wired M-Bus (Meter-Bus, EN 13757-2) frame parsing.
//!
//! Frame forms:
//! - ACK:     single byte `0xE5`
//! - Short:   `0x10 C A CSUM 0x16`
//! - Long:    `0x68 L L 0x68 C A CI ...data... CSUM 0x16`
//!
//! The checksum is the byte-wise sum (mod 256) over C..data end.
//!
//! ```
//! use izanagi_kit::mbus::{parse, Frame};
//!
//! // SND_NKE short frame: C=0x40, A=0x01 -> csum 0x41
//! let f = [0x10u8, 0x40, 0x01, 0x41, 0x16];
//! match parse(&f).unwrap() {
//!     Frame::Short(s) => { assert_eq!(s.ctrl, 0x40); assert_eq!(s.addr, 1); }
//!     _ => unreachable!(),
//! }
//! ```

/// Short frame (control) body.
pub struct Short {
    /// C-field (function code).
    pub ctrl: u8,
    /// A-field (primary address).
    pub addr: u8,
}

/// Long frame (control/data) body.
pub struct Long {
    /// L-field: byte count between the two `0x68` delimiters exclusive of them — i.e. C..data.
    pub len: u8,
    /// C-field.
    pub ctrl: u8,
    /// A-field.
    pub addr: u8,
    /// CI-field (data record identification).
    pub ci: u8,
    /// Offset of the user data block.
    pub data_at: usize,
    /// User data length.
    pub data_len: usize,
}

/// A parsed M-Bus frame.
pub enum Frame {
    /// Single-byte acknowledgement `0xE5`.
    Ack,
    /// `0x10 C A CSUM 0x16` control frame.
    Short(Short),
    /// `0x68 L L 0x68 ...` data frame.
    Long(Long),
}

/// Parses one M-Bus frame; the checksum is verified.
pub fn parse(d: &[u8]) -> Option<Frame> {
    match *d.first()? {
        0xE5 => {
            if d.len() == 1 {
                Some(Frame::Ack)
            } else {
                None
            }
        }
        0x10 => {
            if d.len() != 5 || d[4] != 0x16 {
                return None;
            }
            let csum = d[1].wrapping_add(d[2]);
            if csum != d[3] {
                return None;
            }
            Some(Frame::Short(Short {
                ctrl: d[1],
                addr: d[2],
            }))
        }
        0x68 => {
            if d.len() < 9 || d[3] != 0x68 || d[1] != d[2] {
                return None;
            }
            let len = d[1] as usize;
            // frame = 4 head + len + csum + 0x16
            if len < 3 || 4 + len + 2 != d.len() || *d.last()? != 0x16 {
                return None;
            }
            let mut csum: u8 = 0;
            for &b in &d[4..4 + len] {
                csum = csum.wrapping_add(b);
            }
            if csum != d[4 + len] {
                return None;
            }
            Some(Frame::Long(Long {
                len: d[1],
                ctrl: d[4],
                addr: d[5],
                ci: d[6],
                data_at: 7,
                data_len: len - 3,
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_and_short() {
        assert!(matches!(parse(&[0xE5]), Some(Frame::Ack)));
        assert!(parse(&[0xE5, 0]).is_none());

        let f = [0x10, 0x40, 0x01, 0x41, 0x16];
        assert!(parse(&f).is_some());
        let mut bad = f;
        bad[3] = 0x42;
        assert!(parse(&bad).is_none()); // bad csum
        let mut bad2 = f;
        bad2[4] = 0x15;
        assert!(parse(&bad2).is_none()); // bad stop
    }

    #[test]
    fn long() {
        // C=0x08 A=0x01 CI=0x72 data=[0x78, 0x56]
        // len = 3 + 2 = 5; csum = 0x08+0x01+0x72+0x78+0x56 = 0x149 -> 0x49
        let f = [0x68, 5, 5, 0x68, 0x08, 0x01, 0x72, 0x78, 0x56, 0x49, 0x16];
        match parse(&f).unwrap() {
            Frame::Long(l) => {
                assert_eq!(l.len, 5);
                assert_eq!(l.ctrl, 0x08);
                assert_eq!(l.addr, 1);
                assert_eq!(l.ci, 0x72);
                assert_eq!(l.data_len, 2);
                assert_eq!(f[l.data_at], 0x78);
            }
            _ => panic!("not long"),
        }
        let mut bad = f;
        bad[9] = 0;
        assert!(parse(&bad).is_none());
        // duplicated L mismatch
        let mut bad2 = f;
        bad2[2] = 6;
        assert!(parse(&bad2).is_none());
    }
}
