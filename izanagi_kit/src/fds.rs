//! Famicom Disk System image (`.fds`).
//!
//! Two encodings exist: the fwNES-style image with a 16-byte header
//! (`FDS\x1A` + disk-side count + padding), and the raw headerless form
//! that is simply `N` sides of `65500` bytes each.
//!
//! ```
//! use izanagi_kit::fds::parse;
//!
//! let mut d = b"FDS\x1A".to_vec();
//! d.push(2);            // two disk sides
//! d.resize(16, 0);
//! d.resize(16 + 2 * 65500, 0xFF);
//! let f = parse(&d).unwrap();
//! assert_eq!(f.sides, 2);
//! assert_eq!(f.side_len(0), Some(65500));
//! assert_eq!(f.side_at(1), Some(16 + 65500));
//! ```

/// fwNES header size in bytes.
pub const HEADER: usize = 16;
/// Bytes per disk side (QDC/QD pack).
pub const SIDE: usize = 65500;

/// A parsed FDS image descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct Fds {
    /// Number of disk sides declared (headered) or inferred (raw).
    pub sides: u8,
    /// Offset of side 0's data.
    pub data_at: usize,
    /// Whether the fwNES header was present.
    pub has_header: bool,
}

impl Fds {
    /// Length of side `i` in bytes (`None` for out-of-range index).
    pub fn side_len(&self, i: usize) -> Option<usize> {
        if i < usize::from(self.sides) {
            Some(SIDE)
        } else {
            None
        }
    }

    /// File offset of side `i`'s data.
    pub fn side_at(&self, i: usize) -> Option<usize> {
        self.side_len(i).map(|_| self.data_at + i * SIDE)
    }

    /// Side `i`'s bytes.
    pub fn side<'a>(&self, d: &'a [u8], i: usize) -> Option<&'a [u8]> {
        let at = self.side_at(i)?;
        d.get(at..at + SIDE)
    }
}

/// Parse an FDS image. `None` when the image is empty, the declared side
/// count does not fit the file, or a raw file's length is not a whole
/// number of sides.
pub fn parse(d: &[u8]) -> Option<Fds> {
    if d.len() >= HEADER && &d[..4] == b"FDS\x1A" {
        let sides = d[4];
        if sides == 0 || HEADER + usize::from(sides) * SIDE > d.len() {
            return None;
        }
        return Some(Fds {
            sides,
            data_at: HEADER,
            has_header: true,
        });
    }
    if d.is_empty() || d.len() % SIDE != 0 {
        return None;
    }
    let sides = d.len() / SIDE;
    if sides > usize::from(u8::MAX) {
        return None;
    }
    Some(Fds {
        sides: sides as u8,
        data_at: 0,
        has_header: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn headered(sides: u8) -> Vec<u8> {
        let mut d = b"FDS\x1A".to_vec();
        d.push(sides);
        d.resize(HEADER, 0);
        d.resize(HEADER + usize::from(sides) * SIDE, 0xEE);
        d
    }

    #[test]
    fn headered_image() {
        let f = parse(&headered(2)).unwrap();
        assert!(f.has_header);
        assert_eq!(f.sides, 2);
        assert_eq!(f.side_at(1), Some(HEADER + SIDE));
        assert_eq!(f.side_at(2), None);
        assert_eq!(f.side(&headered(2), 0).unwrap()[0], 0xEE);
    }

    #[test]
    fn raw_image() {
        let d = vec![0xAB; 3 * SIDE];
        let f = parse(&d).unwrap();
        assert!(!f.has_header);
        assert_eq!(f.sides, 3);
        assert_eq!(f.side_at(0), Some(0));
        assert_eq!(f.side_len(2), Some(SIDE));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        // bad magic + wrong length
        assert!(parse(&vec![0u8; SIDE + 1]).is_none());
        // declared sides larger than file
        let mut d = headered(1);
        d[4] = 4;
        assert!(parse(&d).is_none());
        // zero sides declared
        let mut d = headered(1);
        d[4] = 0;
        assert!(parse(&d).is_none());
    }
}
