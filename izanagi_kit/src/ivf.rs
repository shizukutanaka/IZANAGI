//! IVF (Indeo Video File) — the minimal VP8/VP9/AV1 container used by
//! libvpx: `DKIF` + `u16le` version + `u16le` header length (32) +
//! fourcc codec + `u16` w/h + `u32` timebase den/num + `u32` frame
//! count, then `[u32 size][u64 timestamp][payload]` frames.
//!
//! ```
//! use izanagi_kit::ivf::{detect, parse};
//!
//! let mut d = Vec::new();
//! d.extend_from_slice(b"DKIF");
//! d.extend_from_slice(&[0, 0]); // version
//! d.extend_from_slice(&[32, 0]); // header length
//! d.extend_from_slice(b"VP90"); // fourcc
//! d.extend_from_slice(&[128, 0]); // width
//! d.extend_from_slice(&[96, 0]); // height
//! d.extend_from_slice(&[30, 0, 0, 0]); // timebase denominator
//! d.extend_from_slice(&[1, 0, 0, 0]); // numerator
//! d.extend_from_slice(&[1, 0, 0, 0]); // declared frames
//! d.extend_from_slice(&[0, 0, 0, 0]);
//! d.extend_from_slice(&[4, 0, 0, 0]); // frame size
//! d.extend_from_slice(&[0; 8]); // timestamp
//! d.extend_from_slice(b"DATA");
//! assert!(detect(&d));
//! let v = parse(&d).unwrap();
//! assert_eq!(v.codec, "VP90");
//! assert_eq!(v.frames, 1);
//! ```

fn le16(b: &[u8]) -> u16 {
    u16::from(b[0]) | u16::from(b[1]) << 8
}

fn le32(b: &[u8]) -> u32 {
    u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16 | u32::from(b[3]) << 24
}

/// Parsed IVF header + frame census.
#[derive(Debug, Clone, PartialEq)]
pub struct Ivf {
    /// `u16le` version (normally 0).
    pub version: u16,
    /// Header length field (normally 32).
    pub header_len: u16,
    /// Codec fourcc (`VP80`, `VP90`, `AV01`, …).
    pub codec: String,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Timebase denominator (fps numerator).
    pub timebase_den: u32,
    /// Timebase numerator.
    pub timebase_num: u32,
    /// Frame count declared in the header.
    pub declared_frames: u32,
    /// Frame records actually walked.
    pub frames: u32,
    /// Trailing bytes after the last complete frame.
    pub trailing: u32,
}

/// `true` on `DKIF` + plausible header length.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 32 && b[..4] == *b"DKIF" && le16(&b[6..8]) >= 32
}

/// Parses header + frame list; `None` without `DKIF`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ivf> {
    if !detect(b) {
        return None;
    }
    let hdr = usize::from(le16(&b[6..8]));
    if b.len() < hdr {
        return None;
    }
    let codec = String::from_utf8_lossy(&b[8..12]).to_string();
    let mut v = Ivf {
        version: le16(&b[4..6]),
        header_len: hdr as u16,
        codec,
        width: le16(&b[12..14]),
        height: le16(&b[14..16]),
        timebase_den: le32(&b[16..20]),
        timebase_num: le32(&b[20..24]),
        declared_frames: le32(&b[24..28]),
        frames: 0,
        trailing: 0,
    };
    let mut pos = hdr;
    while pos + 12 <= b.len() {
        let sz = usize::try_from(le32(&b[pos..pos + 4])).unwrap_or(0);
        if sz > b.len() - pos - 12 {
            break;
        }
        v.frames += 1;
        pos += 12 + sz;
    }
    v.trailing = u32::try_from(b.len() - pos).unwrap_or(u32::MAX);
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"DKIF");
        d.extend_from_slice(&[0, 0, 32, 0]);
        d.extend_from_slice(b"VP90");
        d.extend_from_slice(&[128, 0, 96, 0]);
        d.extend_from_slice(&[30, 0, 0, 0]);
        d.extend_from_slice(&[1, 0, 0, 0]);
        d.extend_from_slice(&[2, 0, 0, 0]);
        d.extend_from_slice(&[0, 0, 0, 0]);
        for sz in [4u8, 4u8] {
            d.extend_from_slice(&[sz, 0, 0, 0]);
            d.extend_from_slice(&[0; 8]);
            d.extend_from_slice(b"DATA");
        }
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"DKIF"));
        assert!(!detect(b"XDKIF..........................."));
    }

    #[test]
    fn parses() {
        let v = parse(&fixture()).unwrap();
        assert_eq!(v.version, 0);
        assert_eq!(v.header_len, 32);
        assert_eq!(v.codec, "VP90");
        assert_eq!(v.width, 128);
        assert_eq!(v.height, 96);
        assert_eq!(v.timebase_den, 30);
        assert_eq!(v.timebase_num, 1);
        assert_eq!(v.declared_frames, 2);
        assert_eq!(v.frames, 2);
        assert_eq!(v.trailing, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"short").is_none());
        let mut bad = fixture();
        bad[0] = b'X';
        assert!(parse(&bad).is_none());
    }
}
