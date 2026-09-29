//! `ttyrec` terminal recordings — a chain of little-endian frames
//! `u32 sec, u32 usec, u32 len` each followed by `len` bytes of tty
//! output, exactly consuming the file.
//!
//! ```
//! let mut d = vec![];
//! for (s, u, p) in [(10u32, 20u32, b"hi".as_slice()), (11, 30, b"yo")] {
//!     for v in [s, u, p.len() as u32] {
//!         d.extend_from_slice(&[(v & 0xff) as u8, ((v >> 8) & 0xff) as u8,
//!             ((v >> 16) & 0xff) as u8, ((v >> 24) & 0xff) as u8]);
//!     }
//!     d.extend_from_slice(p);
//! }
//! let t = izanagi_kit::ttyrec::parse(&d).unwrap();
//! assert_eq!(t.frames, 2);
//! assert_eq!(t.payload_bytes, 4);
//! assert_eq!(t.first_sec, 10);
//! assert!(izanagi_kit::ttyrec::detect(&d));
//! ```

/// Census of a `.ttyrec` frame chain.
#[derive(Debug, Clone)]
pub struct Ttyrec {
    /// Frame count.
    pub frames: usize,
    /// Total payload bytes.
    pub payload_bytes: usize,
    /// `sec` of the first frame.
    pub first_sec: u32,
    /// `sec` of the last frame.
    pub last_sec: u32,
    /// Largest single-frame payload length.
    pub max_frame: u32,
}

fn u32le(b: &[u8], o: usize) -> u32 {
    (b[o] as u32) | ((b[o + 1] as u32) << 8) | ((b[o + 2] as u32) << 16) | ((b[o + 3] as u32) << 24)
}

fn scan(b: &[u8]) -> Option<Ttyrec> {
    if b.len() < 12 {
        return None;
    }
    let mut t = Ttyrec {
        frames: 0,
        payload_bytes: 0,
        first_sec: 0,
        last_sec: 0,
        max_frame: 0,
    };
    let mut pos = 0usize;
    while pos < b.len() {
        if pos + 12 > b.len() {
            return None; // truncated header
        }
        let sec = u32le(b, pos);
        let usec = u32le(b, pos + 4);
        let len = u32le(b, pos + 8) as usize;
        if usec >= 1_000_000 || len > 16_777_216 || pos + 12 + len > b.len() {
            return None;
        }
        if t.frames == 0 {
            t.first_sec = sec;
        }
        t.last_sec = sec;
        t.frames += 1;
        t.payload_bytes += len;
        if len as u32 > t.max_frame {
            t.max_frame = len as u32;
        }
        pos += 12 + len;
    }
    (t.frames > 0 && t.payload_bytes > 0).then_some(t)
}

/// Detects a `.ttyrec`: a well-formed frame chain that exactly fits the file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    scan(b).is_some()
}

/// Parses a `.ttyrec`; `None` on a truncated or malformed frame chain.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ttyrec> {
    scan(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(sec: u32, usec: u32, payload: &[u8]) -> Vec<u8> {
        let mut d = vec![];
        for v in [sec, usec, payload.len() as u32] {
            d.extend_from_slice(&[
                (v & 0xff) as u8,
                ((v >> 8) & 0xff) as u8,
                ((v >> 16) & 0xff) as u8,
                ((v >> 24) & 0xff) as u8,
            ]);
        }
        d.extend_from_slice(payload);
        d
    }

    #[test]
    fn parses() {
        let mut d = frame(10, 20, b"hello");
        d.extend(frame(11, 30, b"world!"));
        let t = parse(&d).unwrap();
        assert_eq!(t.frames, 2);
        assert_eq!(t.payload_bytes, 11);
        assert_eq!(t.first_sec, 10);
        assert_eq!(t.last_sec, 11);
        assert_eq!(t.max_frame, 6);
    }

    #[test]
    fn detect_works() {
        assert!(detect(&frame(1, 2, b"x")));
        assert!(!detect(b"short"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        let mut bad = frame(1, 2, b"xy");
        bad.pop(); // payload overrun
        assert!(parse(&bad).is_none());
        let mut bad2 = frame(1, 2_000_000, b"x"); // usec out of range
        bad2.extend(frame(2, 3, b"y"));
        assert!(parse(&bad2).is_none());
        assert!(parse(&frame(1, 2, b"")).is_none()); // no payload
    }
}
