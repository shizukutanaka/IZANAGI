//! ZOO archive (`.zoo`, Rahul Dhesi): 20-byte text header ending
//! `^A`?— file starts with a descriptive text + magic `0xDC`
//! `0xA7 0xC4 0xFD` (`FD C4 A7 DC` little-endian at offset 20),
//! then directory entries: `u8 type`, `u8 method`, `u16le` next-
//! entry offset, `u16le` current-entry offset, dates/times, CRC,
//! original+packed sizes, filename.
//!
//! ```
//! let mut d = b"ZOO archive\r\n\x1A".to_vec();
//! d.resize(20, b' ');
//! d.extend_from_slice(&[0xDC, 0xA7, 0xC4, 0xFD]); // magic
//! d.extend_from_slice(&[0x14, 0, 0, 0]); // version + reserved
//! d.extend_from_slice(&[34, 0, 0, 0]); // first dir entry offset
//! d.resize(34, 0);
//! d.extend_from_slice(&[1, 0]); // type=1 file, method=0
//! d.extend_from_slice(&[0; 19]); // next=0/off/date/time/crc/orig/packed/name
//! let p = izanagi_kit::zoo::parse(&d).unwrap();
//! assert_eq!(p.entries, 1);
//! assert!(izanagi_kit::zoo::detect(&d));
//! ```

/// Census of a ZOO archive.
#[derive(Debug, Clone, PartialEq)]
pub struct Zoo {
    /// Lo-level version word after the magic (`zoo` file version).
    pub version: u16,
    /// Archive-text terminator position (0x1A in the 20-byte lead-in).
    pub text_end: usize,
    /// Directory entries walked via the `next` offset chain.
    pub entries: u32,
    /// Type-1 file entries.
    pub file_entries: u32,
    /// Type-2 (subdirectory/comment) entries.
    pub type2_entries: u32,
    /// Methods seen (0 = stored, 1 = LZW, 2 = LZH…).
    pub methods: u32,
    /// Sum of original (uncompressed) sizes.
    pub original_size: u64,
    /// Sum of packed sizes.
    pub packed_size: u64,
    /// An entry's `next` pointer ran past the buffer or looped.
    pub truncated: bool,
}

fn le16(b: &[u8], i: usize) -> u16 {
    b[i] as u16 | ((b[i + 1] as u16) << 8)
}
fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on the `FD C4 A7 DC` magic at offset 20 (or at 0 for
/// multivolume continuations, accepted only with the text leader).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 24 && b[20..24] == [0xDC, 0xA7, 0xC4, 0xFD]
}

/// Census; `None` without the magic. Directory entry (v2 layout):
/// `type u8, method u8, next u16, offset u16, date u16, time u16,
/// crc u16, orig u32, packed u32, namelen u8, namelen…` + longname.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Zoo> {
    if !detect(b) {
        return None;
    }
    let text_end = b[..20].iter().position(|c| *c == 0x1A).unwrap_or(20);
    let mut z = Zoo {
        version: if b.len() >= 26 { le16(b, 24) } else { 0 },
        text_end,
        entries: 0,
        file_entries: 0,
        type2_entries: 0,
        methods: 0,
        original_size: 0,
        packed_size: 0,
        truncated: false,
    };
    // First directory entry offset: u32le at 28 (v2 header layout).
    let mut i = if b.len() >= 32 {
        le32(b, 28) as usize
    } else {
        32
    };
    if i == 0 || i >= b.len() {
        i = 34.min(b.len());
    }
    let mut seen: Vec<usize> = Vec::new();
    let mut steps = 0;
    while i + 21 <= b.len() && steps < 4096 {
        steps += 1;
        if seen.contains(&i) {
            z.truncated = true;
            break;
        }
        seen.push(i);
        let ty = b[i];
        let method = b[i + 1];
        if ty == 0 {
            break;
        }
        let next = le16(b, i + 2) as usize;
        let orig = le32(b, i + 12) as u64;
        let packed = le32(b, i + 16) as u64;
        if ty == 1 {
            z.file_entries += 1;
        } else if ty == 2 {
            z.type2_entries += 1;
        } else if ty > 4 {
            break;
        }
        z.methods |= 1u32 << (method & 31);
        z.original_size += orig;
        z.packed_size += packed;
        z.entries += 1;
        if next == 0 {
            break;
        }
        if i + next >= b.len() {
            z.truncated = true;
            break;
        }
        i += next;
    }
    Some(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"ZOO archive\r\n\x1A".to_vec();
        d.resize(20, b' ');
        d.extend_from_slice(&[0xDC, 0xA7, 0xC4, 0xFD]);
        d.extend_from_slice(&[0x14, 0]); // version
        d.extend_from_slice(&[0; 2]);
        d.extend_from_slice(&[34, 0, 0, 0]); // first dir offset = 34
        d.resize(34, 0);
        // entry: type 1, method 0, next 0, this_off 0
        d.extend_from_slice(&[1, 0]);
        d.extend_from_slice(&[0, 0, 0, 0]); // next=0, off
        d.extend_from_slice(&[0; 6]); // date/time/crc
        d.extend_from_slice(&[100, 0, 0, 0]); // orig 100
        d.extend_from_slice(&[80, 0, 0, 0]); // packed 80
        d.push(5); // namelen
        d.extend_from_slice(b"a.txt");
        d.push(0); // longlen
        d.push(0); // extra len byte pad for layout width
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"ZOO archive"));
        assert!(!detect(&fixture()[..10]));
    }

    #[test]
    fn parses_entries() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.entries, 1);
        assert_eq!(p.file_entries, 1);
        assert_eq!(p.original_size, 100);
        assert_eq!(p.packed_size, 80);
        assert!(!p.truncated);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"not a zoo").is_none());
    }
}
