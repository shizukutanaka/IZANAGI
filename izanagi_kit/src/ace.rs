//! ACE archive (`.ace`, Marcel Lemke): 7-byte preamble
//! (`u16le` CRC + `u16le` header size + `u8` type + `u16le`
//! flags) then the `**ACE**` signature at offset 7, version and
//! host bytes, an optional comment, then `u8 type + u16le flags +
//! u16le size` entry headers (`0x00` MAIN / `0x01` FILE32 / `0x02`
//! RECOVERY / `0x03` AV / `0x04` COMMENT).
//!
//! ```
//! let mut d = vec![0; 7];
//! d.extend_from_slice(b"**ACE**");
//! d.extend_from_slice(&[20, 2]); // ver 2.0, host Win32
//! d.extend_from_slice(&[0; 10]);
//! let p = izanagi_kit::ace::parse(&d).unwrap();
//! assert_eq!(p.version, 20);
//! assert!(izanagi_kit::ace::detect(&d));
//! ```

/// Census of an ACE archive.
#[derive(Debug, Clone, PartialEq)]
pub struct Ace {
    /// `HEAD_CRC` u16le at offset 0.
    pub head_crc: u16,
    /// `HEAD_SIZE` u16le at offset 2.
    pub head_size: u16,
    /// `HEAD_TYPE` byte at offset 4 (0 = main header).
    pub head_type: u8,
    /// `HEAD_FLAGS` u16le at offset 5 (bit 13 = solid, etc.).
    pub head_flags: u16,
    /// ACE version byte (`20` = 2.0).
    pub version: u8,
    /// Host OS byte (0 = MS-DOS, 2 = Win32, …).
    pub host_os: u8,
    /// `ACE` comment length bytes if present in main header.
    pub comment_len: u16,
    /// Sub-headers walked past the main header (`type u8, flags u16, size u16`).
    pub sub_headers: u32,
    /// `0x01` FILE entry headers counted.
    pub file_entries: u32,
    /// `0x02` recovery-block headers.
    pub recovery_entries: u32,
    /// `0x03` authenticity-verification headers.
    pub av_entries: u32,
    /// `0x04` comment headers.
    pub comment_entries: u32,
    /// A header claimed bytes past the end.
    pub truncated: bool,
}

fn le16(b: &[u8], i: usize) -> u16 {
    b[i] as u16 | ((b[i + 1] as u16) << 8)
}

/// `true` on `**ACE**` at offset 7.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 16 && b[7..14] == *b"**ACE**"
}

/// Census; `None` without `**ACE**`. The main header is
/// `head_size` bytes after the CRC/size/type/flags preamble; entry
/// headers (`type u8 + u16le flags + u16le size`) follow.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ace> {
    if !detect(b) {
        return None;
    }
    let mut a = Ace {
        head_crc: le16(b, 0),
        head_size: le16(b, 2),
        head_type: b[4],
        head_flags: le16(b, 5),
        version: b[14],
        host_os: b[15],
        comment_len: 0,
        sub_headers: 0,
        file_entries: 0,
        recovery_entries: 0,
        av_entries: 0,
        comment_entries: 0,
        truncated: false,
    };
    // Main header spans `head_size` bytes from offset 0 (incl. crc).
    let mut i = 7 + a.head_size as usize;
    if i > b.len() {
        i = 16;
        a.truncated = true;
    }
    if i + 2 <= b.len() {
        a.comment_len = le16(b, i);
    }
    let mut steps = 0;
    while i < b.len() && steps < 8192 {
        steps += 1;
        if i + 5 > b.len() {
            a.truncated = true;
            break;
        }
        let ty = b[i];
        let _flags = le16(b, i + 1);
        let size = le16(b, i + 3) as usize;
        if ty > 4 || size < 5 {
            break;
        }
        if i + size > b.len() {
            a.truncated = true;
            break;
        }
        match ty {
            1 => a.file_entries += 1,
            2 => a.recovery_entries += 1,
            3 => a.av_entries += 1,
            4 => a.comment_entries += 1,
            _ => {}
        }
        a.sub_headers += 1;
        i += size;
    }
    Some(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0xAA, 0xBB]; // crc
        d.extend_from_slice(&[9, 0]); // head_size 9
        d.extend_from_slice(&[0, 0, 0]); // type MAIN, flags
        d.extend_from_slice(b"**ACE**");
        d.extend_from_slice(&[20, 2]); // version, host
                                       // file entry: type 1, flags 0, size 8 (+3 payload)
        d.extend_from_slice(&[1, 0, 0, 8, 0]);
        d.extend_from_slice(&[0; 3]);
        // comment entry: type 4, size 6 (+1 payload)
        d.extend_from_slice(&[4, 0, 0, 6, 0, b'x']);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"**ACE**"));
        assert!(!detect(&fixture()[..6]));
    }

    #[test]
    fn parses_entries() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, 20);
        assert_eq!(p.host_os, 2);
        assert_eq!(p.head_type, 0);
        assert_eq!(p.file_entries, 1);
        assert_eq!(p.comment_entries, 1);
        assert_eq!(p.sub_headers, 2);
        assert!(!p.truncated);
    }

    #[test]
    fn truncated_entry() {
        let mut d = fixture();
        d.truncate(d.len() - 2); // cut comment payload
        let p = parse(&d).unwrap();
        assert!(p.truncated);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"ace ace ace").is_none());
    }
}
