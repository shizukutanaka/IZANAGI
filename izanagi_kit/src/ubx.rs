//! UBX — u-blox binary protocol frames (`0xB5 0x62` sync + Fletcher
//! checksum over class..payload).
//!
//! `B5 62 class id lenLE payload ckA ckB` where `ckA/ckB` are the
//! two 8-bit Fletcher sums over `class .. payload` inclusive.
//!
//! ```
//! use izanagi_kit::ubx::parse;
//!
//! // class 0x01 (NAV) id 0x07 (PVT) len 0, checksum of [1,7,0,0]
//! let f = parse(&[0xB5, 0x62, 0x01, 0x07, 0, 0, 8, 25]).unwrap();
//! assert_eq!(f.class, 1);
//! assert!(f.checksum_ok(&[0xB5,0x62,0x01,0x07,0,0,8,25]));
//! ```

/// A parsed UBX frame.
#[derive(Clone, Debug)]
pub struct Ubx {
    /// Message class.
    pub class: u8,
    /// Message id.
    pub id: u8,
    /// Declared payload length.
    pub len: u16,
    /// Payload offset in the input.
    pub payload_at: usize,
    /// Total frame length including sync and checksum.
    pub total: usize,
}

impl Ubx {
    /// Payload slice within `d`.
    pub fn payload<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(self.payload_at..self.payload_at + self.len as usize)
    }

    /// `true` when the two trailing checksum bytes match the Fletcher
    /// sum over `class .. payload`.
    pub fn checksum_ok(&self, d: &[u8]) -> bool {
        let body = match d.get(2..self.payload_at + self.len as usize) {
            Some(b) => b,
            None => return false,
        };
        let (mut a, mut b) = (0u8, 0u8);
        for &x in body {
            a = a.wrapping_add(x);
            b = b.wrapping_add(a);
        }
        d.get(self.payload_at + self.len as usize) == Some(&a)
            && d.get(self.payload_at + self.len as usize + 1) == Some(&b)
    }
}

/// Parse a UBX frame. `None` on bad sync, truncation, or a declared
/// length exceeding the input.
pub fn parse(d: &[u8]) -> Option<Ubx> {
    if d.len() < 8 || d[0] != 0xB5 || d[1] != 0x62 {
        return None;
    }
    let len = (d[4] as u16) | ((d[5] as u16) << 8);
    let payload_at = 6usize;
    let total = payload_at + len as usize + 2;
    if d.len() < total {
        return None;
    }
    Some(Ubx {
        class: d[2],
        id: d[3],
        len,
        payload_at,
        total,
    })
}

/// Fletcher checksum pair over `d`.
pub fn checksum(d: &[u8]) -> (u8, u8) {
    let (mut a, mut b) = (0u8, 0u8);
    for &x in d {
        a = a.wrapping_add(x);
        b = b.wrapping_add(a);
    }
    (a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(class: u8, id: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![0xB5, 0x62, class, id];
        v.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        v.extend_from_slice(payload);
        let (a, b) = checksum(&v[2..]);
        v.push(a);
        v.push(b);
        v
    }

    #[test]
    fn parses() {
        let d = frame(0x01, 0x07, &[1, 2, 3]);
        let f = parse(&d).unwrap();
        assert_eq!((f.class, f.id, f.len), (1, 7, 3));
        assert_eq!(f.payload(&d), Some(&[1, 2, 3][..]));
        assert!(f.checksum_ok(&d));
        let mut bad = d.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(!f.checksum_ok(&bad));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0xB5, 0x62]).is_none());
        assert!(parse(&[0xFF, 0x62, 0, 0, 0, 0, 0, 0]).is_none());
        assert!(parse(&[0xB5, 0x62, 1, 2, 9, 0, 0, 0]).is_none());
    }
}
