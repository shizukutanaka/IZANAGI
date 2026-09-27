//! GUS patch (`.pat`, GF1PATCH110/ID#000002) — fixed 128-byte header:
//! description (60), instrument/voice/channel counts, waveform count,
//! master volume and total waveform data size.
//!
//! ```
//! use izanagi_kit::pat::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 4];
//! d[..12].copy_from_slice(b"GF1PATCH110\0");
//! d[12..22].copy_from_slice(b"ID#000002\0");
//! d[22..28].copy_from_slice(b"Piano ");
//! d[82] = 2;   // instruments
//! d[83] = 32;  // voices
//! d[85] = 4; d[86] = 0; // waveforms = 4
//! d[87] = 127; // master volume
//! let p = parse(&d).unwrap();
//! assert_eq!(p.description, "Piano");
//! assert_eq!((p.instruments, p.voices), (2, 32));
//! assert_eq!(p.waveforms, 4);
//! ```

/// Header magic.
pub const MAGIC: &[u8; 12] = b"GF1PATCH110\0";
/// Format id following the magic (10 bytes including NUL pad).
pub const ID: &[u8; 10] = b"ID#000002\0";
/// Fixed header size in bytes.
pub const HEADER: usize = 129;

/// Parsed `.pat` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pat<'a> {
    /// Patch description (60 bytes, NUL/space padded).
    pub description: &'a str,
    /// Number of instruments in the patch.
    pub instruments: u8,
    /// Voice count (max 32 on the GUS).
    pub voices: u8,
    /// Channel count.
    pub channels: u8,
    /// Number of waveform records.
    pub waveforms: u16,
    /// Master volume (0..127).
    pub master_volume: u16,
    /// Total bytes of waveform data after the header.
    pub data_size: u32,
}

fn text(d: &[u8], at: usize, n: usize) -> &str {
    let s = d.get(at..at + n).unwrap_or(&[]);
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    std::str::from_utf8(&s[..end]).unwrap_or("").trim_end()
}

fn le16(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from(*d.get(i)?) | (u16::from(*d.get(i + 1)?) << 8))
}
fn le32(d: &[u8], i: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(i)?)
            | (u32::from(*d.get(i + 1)?) << 8)
            | (u32::from(*d.get(i + 2)?) << 16)
            | (u32::from(*d.get(i + 3)?) << 24),
    )
}

/// Parse the fixed header.
pub fn parse(d: &[u8]) -> Option<Pat<'_>> {
    if d.get(..12)? != MAGIC || d.get(12..22)? != ID || d.len() < HEADER {
        return None;
    }
    Some(Pat {
        description: text(d, 22, 60),
        instruments: *d.get(82)?,
        voices: *d.get(83)?,
        channels: *d.get(84)?,
        waveforms: le16(d, 85)?,
        master_volume: le16(d, 87)?,
        data_size: le32(d, 89)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        d[..12].copy_from_slice(MAGIC);
        d[12..22].copy_from_slice(ID);
        d[22..27].copy_from_slice(b"Bass ");
        d[82] = 1;
        d[83] = 32;
        d[84] = 16;
        d[85] = 2;
        d[87] = 0x40;
        d[88] = 0x01;
        d[89] = 0x10;
        d
    }

    #[test]
    fn fields() {
        let d = fixture();
        let p = parse(&d).unwrap();
        assert_eq!(p.description, "Bass");
        assert_eq!((p.instruments, p.voices, p.channels), (1, 32, 16));
        assert_eq!(p.waveforms, 2);
        assert_eq!(p.master_volume, 0x0140);
        assert_eq!(p.data_size, 0x10);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[12] = b'X';
        assert!(parse(&d).is_none());
        assert!(parse(&fixture()[..HEADER - 1]).is_none());
    }
}
