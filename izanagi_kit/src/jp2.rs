//! JPEG 2000 Part 1 (JP2) container scanner.
//!
//! A JP2 file starts with the 12-byte signature box
//! (`00 00 00 0C` + `jP  ` + `0D 0A 87 0A`), then a box chain of
//! `u32be length` + `4cc type` records (`ftyp`, `jp2h`, `jp2c`, …).
//! A length of 1 signals a 64-bit extended length; 0 means "to end
//! of file".
//!
//! ```
//! let mut d = vec![0, 0, 0, 12];
//! d.extend_from_slice(b"jP  \r\n\x87\n");
//! let mut f = vec![0, 0, 0, 16];
//! f.extend_from_slice(b"ftypjp2 ");
//! f.extend_from_slice(&[0, 0, 0, 0]);
//! d.extend_from_slice(&f);
//! let j = izanagi_kit::jp2::parse(&d).unwrap();
//! assert_eq!(j.brand.as_deref(), Some("jp2 "));
//! ```
//!
//! Reference: ISO/IEC 15444-1 Annex I — the signature box and the
//! box length/type record layout (`length==1` extended form).

/// Parsed JP2 fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Jp2 {
    /// `ftyp` brand (e.g. `jp2 `, `jpm `, `jpx `).
    pub brand: Option<String>,
    /// Box count after the signature box.
    pub boxes: usize,
    /// `jp2h` superbox present.
    pub has_header: bool,
    /// `jp2c` codestream box present.
    pub has_codestream: bool,
    /// `true` when the codestream runs to EOF (`length == 0`).
    pub codestream_to_eof: bool,
    /// All box type strings in order (capped at 16).
    pub box_types: Vec<String>,
}

fn u32be(d: &[u8], i: usize) -> u32 {
    ((d[i] as u32) << 24) | ((d[i + 1] as u32) << 16) | ((d[i + 2] as u32) << 8) | d[i + 3] as u32
}

const SIG: &[u8; 12] = b"\x00\x00\x00\x0CjP  \r\n\x87\n";

/// Parse a JP2 file; `None` unless the signature box and a
/// well-formed box chain are present.
pub fn parse(d: &[u8]) -> Option<Jp2> {
    if d.len() < 12 || &d[..12] != SIG {
        return None;
    }
    let mut boxes = 0usize;
    let mut brand = None;
    let mut has_header = false;
    let mut has_codestream = false;
    let mut codestream_to_eof = false;
    let mut box_types = Vec::new();
    let mut i = 12usize;
    while i + 8 <= d.len() {
        let len = u32be(d, i);
        let ty = &d[i + 4..i + 8];
        // (total box length incl. header, header length)
        let (total, hlen) = match len {
            0 => (d.len() - i, 8usize), // runs to EOF
            1 => {
                if i + 16 > d.len() {
                    break;
                }
                let xl = ((u32be(d, i + 8) as u64) << 32) | u32be(d, i + 12) as u64;
                if xl < 16 || xl as usize > d.len() - i {
                    break;
                }
                (xl as usize, 16usize)
            }
            l if l < 8 => break,
            l => {
                let l = l as usize;
                if l > d.len() - i {
                    break;
                }
                (l, 8usize)
            }
        };
        boxes += 1;
        if box_types.len() < 16 {
            if let Ok(s) = core::str::from_utf8(ty) {
                box_types.push(s.to_string());
            }
        }
        match ty {
            b"ftyp" => {
                if brand.is_none() && i + hlen + 4 <= d.len() {
                    brand = core::str::from_utf8(&d[i + hlen..i + hlen + 4])
                        .ok()
                        .map(|s| s.to_string());
                }
            }
            b"jp2h" => has_header = true,
            b"jp2c" => {
                has_codestream = true;
                codestream_to_eof = len == 0;
            }
            _ => {}
        }
        if len == 0 {
            break;
        }
        i += total;
    }
    Some(Jp2 {
        brand,
        boxes,
        has_header,
        has_codestream,
        codestream_to_eof,
        box_types,
    })
}

/// `true` if the buffer looks like a JP2 file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(ty: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let n = (payload.len() + 8) as u32;
        let mut v = vec![(n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, n as u8];
        v.extend_from_slice(ty);
        v.extend_from_slice(payload);
        v
    }

    fn doc() -> Vec<u8> {
        let mut d = SIG.to_vec();
        d.extend_from_slice(&bx(b"ftyp", b"jp2 \0\0\0\0"));
        d.extend_from_slice(&bx(b"jp2h", b"x"));
        d.extend_from_slice(&bx(b"jp2c", b"y"));
        d
    }

    #[test]
    fn parses() {
        let j = parse(&doc()).unwrap();
        assert_eq!(j.brand.as_deref(), Some("jp2 "));
        assert_eq!(j.boxes, 3);
        assert!(j.has_header);
        assert!(j.has_codestream);
        assert_eq!(j.box_types, ["ftyp", "jp2h", "jp2c"]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0xff; 16]).is_none());
    }

    #[test]
    fn detects() {
        assert!(detect(&doc()));
        assert!(!detect(b"jP"));
    }
}
