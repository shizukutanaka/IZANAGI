//! JPEG XL signature/container parsing (ISO/IEC 18181).
//!
//! Two wire forms: the raw codestream starting with
//! `0xFF 0x0A`, and the structured container whose first box is
//! `JXL ` (`0x0000000C 'JXL ' 0x0D0A870A`), optionally followed
//! by an `ftyp` box with brand `jxl `.
//!
//! ```
//! use izanagi_kit::jxl;
//! // container form
//! let mut d = b"\x00\x00\x00\x0CJXL \x0D\x0A\x87\x0A".to_vec();
//! d.extend_from_slice(&[0, 0, 0, 20]); // box size 20
//! d.extend_from_slice(b"ftypjxl \x00\x00\x00\x00jxl ");
//! let j = jxl::parse(&d).unwrap();
//! assert_eq!(j.form, jxl::Form::Container);
//! ```

use std::vec::Vec;

/// Wire form.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Form {
    /// Raw codestream (`0xFF0A`).
    Codestream,
    /// Structured `JXL ` box container.
    Container,
}

/// One container box header.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JxlBox {
    /// Box type four-char code.
    pub ty: [u8; 4],
    /// Declared box size including its own header.
    pub len: u64,
    /// Byte offset of box contents.
    pub at: usize,
}

/// A parsed JPEG XL input.
#[derive(Clone, Debug, PartialEq)]
pub struct Jxl {
    /// Wire form.
    pub form: Form,
    /// Container boxes after the signature box (`ftyp`, `jxll`,
    /// `jxlc`, `jxlp`…).
    pub boxes: Vec<JxlBox>,
    /// True when an `ftyp` box naming brand `jxl ` was seen.
    pub has_ftyp: bool,
}

/// Box signature.
pub const CONTAINER_MAGIC: &[u8; 12] = b"\x00\x00\x00\x0CJXL \x0D\x0A\x87\x0A";

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

fn u64be(d: &[u8], at: usize) -> Option<u64> {
    let s = d.get(at..at.checked_add(8)?)?;
    Some((u32be(s, 0)? as u64) << 32 | u32be(s, 4)? as u64)
}

/// Parses JPEG XL input: codestream or `JXL ` container; for the
/// container, walks box headers (32-bit size; `1` means 64-bit
/// extended size follows; `0` means "to EOF").
pub fn parse(d: &[u8]) -> Option<Jxl> {
    if d.len() < 2 {
        return None;
    }
    if d[0] == 0xFF && d[1] == 0x0A {
        return Some(Jxl {
            form: Form::Codestream,
            boxes: Vec::new(),
            has_ftyp: false,
        });
    }
    if !d.starts_with(CONTAINER_MAGIC) {
        return None;
    }
    let mut at = CONTAINER_MAGIC.len();
    let mut boxes = Vec::new();
    let mut has_ftyp = false;
    while at + 8 <= d.len() {
        let size = u32be(d, at)? as u64;
        let mut ty = [0u8; 4];
        ty.copy_from_slice(d.get(at + 4..at + 8)?);
        let (len, body) = if size == 1 {
            let l = u64be(d, at + 8)?;
            (l, at + 16)
        } else if size == 0 {
            ((d.len() - at) as u64, at + 8)
        } else {
            (size, at + 8)
        };
        let header_len = if size == 1 { 16 } else { 8 };
        if len < header_len {
            return None;
        }
        if len > (d.len() - at) as u64 {
            return None;
        }
        if &ty == b"ftyp" {
            has_ftyp = true;
        }
        boxes.push(JxlBox { ty, len, at: body });
        at = at.checked_add(len as usize)?;
        if boxes.len() > 4096 {
            return None;
        }
    }
    Some(Jxl {
        form: Form::Container,
        boxes,
        has_ftyp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codestream() {
        let j = parse(&[0xFF, 0x0A, 0x00]).unwrap();
        assert_eq!(j.form, Form::Codestream);
    }

    #[test]
    fn container() {
        let mut d = CONTAINER_MAGIC.to_vec();
        d.extend_from_slice(&[0, 0, 0, 20]); // size 20
        d.extend_from_slice(b"ftypjxl \x00\x00\x00\x00jxl ");
        let j = parse(&d).unwrap();
        assert_eq!(j.form, Form::Container);
        assert_eq!(j.boxes.len(), 1);
        assert_eq!(&j.boxes[0].ty, b"ftyp");
        assert!(j.has_ftyp);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0xFF, 0xD8]).is_none()); // jpeg
        assert!(parse(&[0x00, 0x00, 0x00, 0x0C]).is_none());
        assert!(parse(b"").is_none());
    }
}
