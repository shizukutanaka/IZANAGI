//! Xilinx FPGA bitstream (`.bit`): 13-byte preamble
//! `00 09 0F F0 0F F0 0F F0 0F F0 00 01`, then tagged fields —
//! `a` + `u16be` len + design name, `b` + part number, `c` +
//! creation date, `d` + time, `e` + `u32be` length + raw bitstream
//! (starting with sync word `AA995566`).
//!
//! ```
//! let mut d = vec![0x00, 0x09, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x00, 0x01, 0x00];
//! d.extend_from_slice(b"a");
//! d.extend_from_slice(&[0, 4]);
//! d.extend_from_slice(b"top;");
//! d.extend_from_slice(b"b");
//! d.extend_from_slice(&[0, 6]);
//! d.extend_from_slice(b"7a100t");
//! d.extend_from_slice(b"e");
//! d.extend_from_slice(&[0, 0, 0, 4]);
//! d.extend_from_slice(&[0xAA, 0x99, 0x55, 0x66]);
//! let p = izanagi_kit::bit::parse(&d).unwrap();
//! assert_eq!(p.part.as_deref(), Some("7a100t"));
//! assert!(p.has_sync);
//! assert!(izanagi_kit::bit::detect(&d));
//! ```

/// Census of a Xilinx `.bit` file.
#[derive(Debug, Clone, PartialEq)]
pub struct Bit {
    /// `a` field: design/name string.
    pub design: Option<String>,
    /// `b` field: part number string (e.g. `7a100tcsg324`).
    pub part: Option<String>,
    /// `c` field: creation date (`YYYY/MM/DD`).
    pub date: Option<String>,
    /// `d` field: creation time (`HH:MM:SS`).
    pub time: Option<String>,
    /// `e` field: declared bitstream byte length.
    pub data_len: u32,
    /// Bitstream begins with the `AA995566` sync word.
    pub has_sync: bool,
    /// Declared data length ran past the buffer.
    pub truncated: bool,
    /// Unknown tag seen while walking fields.
    pub unknown_tags: u32,
}

fn be16(b: &[u8], i: usize) -> u16 {
    ((b[i] as u16) << 8) | b[i + 1] as u16
}
fn be32(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | b[i + 3] as u32
}

const PREAMBLE: [u8; 13] = [
    0x00, 0x09, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x00, 0x01, 0x00,
];

/// `true` on the 13-byte Xilinx preamble.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 14 && b[..13] == PREAMBLE && (b[13] == b'a' || b[13] == b'e')
}

/// Census; `None` without the preamble. String fields are stored as
/// ASCII tags `a`/`b`/`c`/`d` with `u16be` lengths; `e` carries the
/// raw stream length as `u32be`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Bit> {
    if !detect(b) {
        return None;
    }
    let mut s = Bit {
        design: None,
        part: None,
        date: None,
        time: None,
        data_len: 0,
        has_sync: false,
        truncated: false,
        unknown_tags: 0,
    };
    let mut i = 13usize;
    while i < b.len() {
        let tag = b[i];
        i += 1;
        if tag == b'e' {
            if i + 4 > b.len() {
                s.truncated = true;
                break;
            }
            s.data_len = be32(b, i);
            i += 4;
            if i + s.data_len as usize > b.len() {
                s.truncated = true;
            }
            s.has_sync = i + 4 <= b.len() && b[i..i + 4] == [0xAA, 0x99, 0x55, 0x66];
            break;
        }
        if i + 2 > b.len() {
            s.truncated = true;
            break;
        }
        let n = be16(b, i) as usize;
        i += 2;
        if i + n > b.len() {
            s.truncated = true;
            break;
        }
        let text = Some(String::from_utf8_lossy(&b[i..i + n]).into_owned());
        match tag {
            b'a' => s.design = text,
            b'b' => s.part = text,
            b'c' => s.date = text,
            b'd' => s.time = text,
            _ => s.unknown_tags += 1,
        }
        i += n;
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(d: &mut Vec<u8>, tag: u8, text: &[u8]) {
        d.push(tag);
        d.extend_from_slice(&(text.len() as u16).to_be_bytes());
        d.extend_from_slice(text);
    }

    fn fixture() -> Vec<u8> {
        let mut d = PREAMBLE.to_vec();
        field(&mut d, b'a', b"counter.ncd");
        field(&mut d, b'b', b"xc7a35t");
        field(&mut d, b'c', b"2024/03/01");
        field(&mut d, b'd', b"12:00:00");
        d.push(b'e');
        d.extend_from_slice(&8u32.to_be_bytes());
        d.extend_from_slice(&[0xAA, 0x99, 0x55, 0x66, 1, 2, 3, 4]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"bitstream"));
        assert!(!detect(&fixture()[..12]));
    }

    #[test]
    fn parses_fields() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.design.as_deref(), Some("counter.ncd"));
        assert_eq!(p.part.as_deref(), Some("xc7a35t"));
        assert_eq!(p.date.as_deref(), Some("2024/03/01"));
        assert_eq!(p.data_len, 8);
        assert!(p.has_sync);
        assert!(!p.truncated);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"not a bitstream").is_none());
    }
}
