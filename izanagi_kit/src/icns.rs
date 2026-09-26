//! Apple Icon Image format (.icns).
//!
//! An `icns` file is a 4-byte `icns` magic + u32BE total length, then a
//! sequence of `{ OSType tag (4B), u32BE length (includes the 8-byte
//! tag+len header), data }` blocks. Modern icons embed PNG or
//! JPEG 2000 data under tags like `ic07`–`ic14`; legacy bitmap+mask
//! pairs (`is32`/`il32`/`ih32`/`it32` with `s8mk`/`l8mk`/`h8mk`/`t8mk`)
//! are also common.
//!
//! ```
//! let mut d = Vec::new();
//! d.extend_from_slice(b"icns");
//! d.extend_from_slice(&[0, 0, 0, 20]); // total len = 20
//! d.extend_from_slice(b"ic07");
//! d.extend_from_slice(&[0, 0, 0, 12]); // entry len = 12
//! d.extend_from_slice(b"\x89PNG");
//! let i = izanagi_kit::icns::parse(&d).unwrap();
//! assert_eq!(i.entries.len(), 1);
//! assert_eq!(izanagi_kit::icns::entry_name(i.entries[0].tag), "ic07 (128x128 PNG)");
//! assert_eq!(izanagi_kit::icns::image(&d, &i, b"ic07").unwrap(), b"\x89PNG");
//! ```

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        ((*d.get(at)? as u32) << 24)
            | ((*d.get(at + 1)? as u32) << 16)
            | ((*d.get(at + 2)? as u32) << 8)
            | *d.get(at + 3)? as u32,
    )
}

/// One icon element.
#[derive(Debug, Clone)]
pub struct Entry {
    /// 4-byte OSType tag (`ic07`, `ic11`, `is32`, `s8mk`, …).
    pub tag: [u8; 4],
    /// Absolute offset of the payload (right after tag+len).
    pub data_at: usize,
    /// Payload length in bytes.
    pub len: usize,
}

/// Parsed header.
#[derive(Debug)]
pub struct Icns {
    /// Declared total file length (header included).
    pub total: u32,
    /// Elements in file order.
    pub entries: Vec<Entry>,
}

/// Human-readable description of a known OSType; unknown tags return
/// a `"????"` tag string.
pub fn entry_name(tag: [u8; 4]) -> String {
    let name = match &tag {
        b"ic07" => "ic07 (128x128 PNG)",
        b"ic08" => "ic08 (256x256 PNG)",
        b"ic09" => "ic09 (512x512 PNG)",
        b"ic10" => "ic10 (1024x1024 PNG)",
        b"ic11" => "ic11 (32x32@2x PNG)",
        b"ic12" => "ic12 (64x64@2x PNG)",
        b"ic13" => "ic13 (256x256@2x PNG)",
        b"ic14" => "ic14 (512x512@2x PNG)",
        b"is32" => "is32 (16x16 RGB)",
        b"il32" => "il32 (32x32 RGB)",
        b"ih32" => "ih32 (48x48 RGB)",
        b"it32" => "it32 (128x128 RGB)",
        b"s8mk" => "s8mk (16x16 mask)",
        b"l8mk" => "l8mk (32x32 mask)",
        b"h8mk" => "h8mk (48x48 mask)",
        b"t8mk" => "t8mk (128x128 mask)",
        b"icp4" => "icp4 (16x16 PNG)",
        b"icp5" => "icp5 (32x32 PNG)",
        b"icp6" => "icp6 (64x64 PNG)",
        b"icsb" => "icsb (18x18 PNG)",
        b"icsB" => "icsB (36x36 PNG)",
        _ => "????",
    };
    name.to_string()
}

/// Parses the magic + entry table. `None` on bad magic, truncated
/// length, or an entry length smaller than the 8-byte tag header.
pub fn parse(d: &[u8]) -> Option<Icns> {
    if d.len() < 8 || &d[0..4] != b"icns" {
        return None;
    }
    let total = be32(d, 4)?;
    let mut entries = Vec::new();
    let mut at = 8;
    while at + 8 <= d.len() && at + 8 <= total as usize {
        let mut tag = [0u8; 4];
        tag.copy_from_slice(&d[at..at + 4]);
        let len = be32(d, at + 4)? as usize;
        if len < 8 {
            return None;
        }
        let data_at = at + 8;
        let payload = len - 8;
        if data_at + payload > d.len() {
            return None;
        }
        entries.push(Entry {
            tag,
            data_at,
            len: payload,
        });
        at = data_at + payload;
    }
    Some(Icns { total, entries })
}

/// Payload bytes of the first entry with `tag` — the PNG/JP2K image
/// data for `icXX` tags, or raw RGB/mask bytes for legacy tags.
pub fn image<'a>(d: &'a [u8], i: &Icns, tag: &[u8; 4]) -> Option<&'a [u8]> {
    let e = i.entries.iter().find(|e| &e.tag == tag)?;
    d.get(e.data_at..e.data_at + e.len)
}

/// `true` when `data` looks like a PNG (magic signature).
pub fn is_png(data: &[u8]) -> bool {
    data.starts_with(b"\x89PNG\r\n\x1a\n")
}

/// `true` when `data` looks like JPEG 2000 (`jP  ` box or codestream).
pub fn is_jp2(data: &[u8]) -> bool {
    data.starts_with(b"\x00\x00\x00\x0CjP  ") || data.starts_with(b"\xFF\x4F\xFF\x51")
}

/// Detected payload kind of `tag`'s first entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Embedded PNG.
    Png,
    /// Embedded JPEG 2000.
    Jp2,
    /// Legacy RGB bitmap (is32/il32/ih32/it32).
    Rgb,
    /// 8-bit mask (s8mk/l8mk/h8mk/t8mk).
    Mask,
    /// Anything else.
    Other,
}

/// Classifies the payload of `tag`'s first entry.
pub fn kind(d: &[u8], i: &Icns, tag: &[u8; 4]) -> Kind {
    match image(d, i, tag) {
        Some(p) if is_png(p) => Kind::Png,
        Some(p) if is_jp2(p) => Kind::Jp2,
        _ => match tag {
            b"is32" | b"il32" | b"ih32" | b"it32" => Kind::Rgb,
            b"s8mk" | b"l8mk" | b"h8mk" | b"t8mk" => Kind::Mask,
            _ => Kind::Other,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"icns");
        d.extend_from_slice(&[0, 0, 0, 36]);
        d.extend_from_slice(b"ic07");
        d.extend_from_slice(&[0, 0, 0, 20]);
        d.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        d.extend_from_slice(b"PNG!");
        d.extend_from_slice(b"is32");
        d.extend_from_slice(&[0, 0, 0, 12]);
        d.extend_from_slice(&[1, 2, 3, 4]);
        d
    }

    #[test]
    fn parses_entries() {
        let d = fixture();
        let i = parse(&d).unwrap();
        assert_eq!(i.total, 36);
        assert_eq!(i.entries.len(), 2);
        assert_eq!(&i.entries[0].tag, b"ic07");
        assert_eq!(&i.entries[1].tag, b"is32");
    }

    #[test]
    fn image_and_kind() {
        let d = fixture();
        let i = parse(&d).unwrap();
        assert_eq!(kind(&d, &i, b"ic07"), Kind::Png);
        assert_eq!(kind(&d, &i, b"is32"), Kind::Rgb);
        assert_eq!(kind(&d, &i, b"s8mk"), Kind::Mask);
        assert_eq!(kind(&d, &i, b"zzzz"), Kind::Other);
        assert!(is_png(image(&d, &i, b"ic07").unwrap()));
        assert!(is_jp2(&[0xFF, 0x4F, 0xFF, 0x51])); // codestream magic
        assert!(image(&d, &i, b"ic99").is_none());
        assert_eq!(entry_name(*b"t8mk"), "t8mk (128x128 mask)");
        assert_eq!(entry_name(*b"wxyz"), "????");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"icnX").is_none());
        // entry length smaller than the 8-byte header
        let mut bad2 = fixture();
        bad2[15] = 4; // len = 4 < 8
        assert!(parse(&bad2).is_none());
        // truncated entry payload
        let mut bad3 = fixture();
        bad3.truncate(bad3.len() - 1);
        assert!(parse(&bad3).is_none());
    }
}
