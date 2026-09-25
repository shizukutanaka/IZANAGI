//! TrueType/OpenType (sfnt) container: the offset table + table
//! directory, plus typed views of `head`, `maxp`, and `name`
//! records. All sfnt fields are big-endian; `ttf` is a *structural*
//! walk — no glyph rasterization, no hinting.
//!
//! Covers TrueType (`0x00010000`), OpenType-CFF (`OTTO`), and the
//! Apple `true`/`typ1` variants.
//!
//! ```
//! use izanagi_kit::ttf;
//! let mut f = vec![0, 1, 0, 0]; // sfnt version = TrueType
//! f.extend_from_slice(&[0, 1]); // 1 table
//! f.extend_from_slice(&[0, 128]); // searchRange
//! f.extend_from_slice(&[0, 0]);   // entrySelector
//! f.extend_from_slice(&[0, 0]);   // rangeShift
//! // table record: tag "head", checksum, offset, length
//! f.extend_from_slice(b"head");
//! f.extend_from_slice(&[0, 0, 0, 0]); // checksum
//! f.extend_from_slice(&[0, 0, 0, 28]); // offset
//! f.extend_from_slice(&[0, 0, 0, 4]);  // length
//! f.extend_from_slice(&[0, 0, 0, 0]); // (directory ends at 28)
//! let t = ttf::parse(&f).unwrap();
//! assert_eq!(t.tables.len(), 1);
//! assert_eq!(t.tables[0].name(), "head");
//! ```

fn r16(d: &[u8], at: usize) -> Option<u32> {
    Some((*d.get(at)? as u32) << 8 | *d.get(at + 1)? as u32)
}
fn r32(d: &[u8], at: usize) -> Option<u32> {
    Some(r16(d, at)? << 16 | r16(d, at + 2)?)
}
fn tag4(d: &[u8], at: usize) -> Option<[u8; 4]> {
    let s = d.get(at..at + 4)?;
    Some([s[0], s[1], s[2], s[3]])
}

/// One sfnt table-directory record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    /// 4-byte tag (`"head"`, `"cmap"`, …).
    pub tag: [u8; 4],
    /// Table checksum (as stored; not verified).
    pub checksum: u32,
    /// File offset of the table data.
    pub offset: usize,
    /// Table length in bytes.
    pub length: usize,
}

impl Tab {
    /// The tag as a display string (non-ASCII bytes as `?`).
    pub fn name(&self) -> String {
        self.tag
            .iter()
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '?' })
            .collect()
    }
}

/// A parsed sfnt container.
#[derive(Clone, Debug)]
pub struct Ttf {
    /// The sfnt version tag (`0x00010000` TrueType, `OTTO` CFF, `true`, `typ1`).
    pub sfnt: u32,
    /// The table directory (in file order).
    pub tables: Vec<Tab>,
}

/// `head` table essentials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Head {
    /// Units per em (16..=16384 in valid fonts).
    pub units_per_em: u16,
    /// `loca` entry width: 0 = u16 offsets/2, 1 = u32 offsets.
    pub index_to_loc_format: u16,
}

/// One `name` table record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameRec {
    /// Platform (0 Unicode, 1 Macintosh, 2 ISO, 3 Windows).
    pub platform: u16,
    /// Platform-specific encoding ID.
    pub encoding: u16,
    /// Language ID (0x0409 = en-US under Windows).
    pub language: u16,
    /// Name ID (1=family, 2=subfamily, 4=full, 6=PostScript).
    pub name_id: u16,
    /// The raw string bytes (UTF-16BE under Windows, Mac Roman otherwise).
    pub data: Vec<u8>,
}

impl NameRec {
    /// Name IDs have conventional labels.
    pub fn label(&self) -> &'static str {
        match self.name_id {
            0 => "copyright",
            1 => "family",
            2 => "subfamily",
            3 => "unique-id",
            4 => "full-name",
            5 => "version",
            6 => "postscript-name",
            _ => "other",
        }
    }
}

/// Parse an sfnt (ttf/otf) file's offset table + directory.
pub fn parse(d: &[u8]) -> Option<Ttf> {
    let sfnt = r32(d, 0)?;
    match sfnt {
        0x0001_0000 | 0x4F54544F | 0x74727565 | 0x74797031 => {} // 1.0, OTTO, true, typ1
        _ => return None,
    }
    let num = r16(d, 4)? as usize;
    let mut tables = Vec::with_capacity(num.min(64));
    for i in 0..num {
        let at = 12usize.checked_add(i.checked_mul(16)?)?;
        if at.checked_add(16)? > d.len() {
            return None; // truncated directory
        }
        let tag = tag4(d, at)?;
        let checksum = r32(d, at + 4)?;
        let offset = r32(d, at + 8)? as usize;
        let length = r32(d, at + 12)? as usize;
        tables.push(Tab {
            tag,
            checksum,
            offset,
            length,
        });
    }
    Some(Ttf { sfnt, tables })
}

/// The first table named `tag`, or `None`.
pub fn table<'a>(t: &'a Ttf, tag: &[u8; 4]) -> Option<&'a Tab> {
    t.tables.iter().find(|tab| &tab.tag == tag)
}

/// `head` essentials, if the table exists and fits.
pub fn head(d: &[u8], t: &Ttf) -> Option<Head> {
    let tab = table(t, b"head")?;
    if tab.offset.checked_add(54)? > d.len() || tab.length < 54 {
        return None;
    }
    Some(Head {
        units_per_em: r16(d, tab.offset + 18)? as u16,
        index_to_loc_format: r16(d, tab.offset + 50)? as u16,
    })
}

/// `maxp` glyph count, if present.
pub fn maxp(d: &[u8], t: &Ttf) -> Option<u16> {
    let tab = table(t, b"maxp")?;
    Some(r16(d, tab.offset + 4)? as u16)
}

/// Decode the `name` table into `NameRec`s (raw bytes kept — callers
/// decide UTF-16 vs Mac Roman by `platform`).
pub fn names(d: &[u8], t: &Ttf) -> Option<Vec<NameRec>> {
    let tab = table(t, b"name")?;
    let base = tab.offset;
    if base.checked_add(6)? > d.len() {
        return None;
    }
    let count = r16(d, base + 2)? as usize;
    let str_off = base.checked_add(r16(d, base + 4)? as usize)?;
    let mut out = Vec::with_capacity(count.min(256));
    for i in 0..count {
        let at = base.checked_add(6)?.checked_add(i.checked_mul(12)?)?;
        if at.checked_add(12)? > d.len() {
            break; // truncated record — degrade to what parsed
        }
        let platform = r16(d, at)? as u16;
        let encoding = r16(d, at + 2)? as u16;
        let language = r16(d, at + 4)? as u16;
        let name_id = r16(d, at + 6)? as u16;
        let len = r16(d, at + 8)? as usize;
        let off = r16(d, at + 10)? as usize;
        let start = str_off.checked_add(off)?;
        let end = start.checked_add(len)?;
        if end > d.len() {
            break;
        }
        out.push(NameRec {
            platform,
            encoding,
            language,
            name_id,
            data: d[start..end].to_vec(),
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal sfnt with `head`+`maxp`+`name` tables.
    fn font() -> Vec<u8> {
        let mut f = vec![0, 1, 0, 0, 0, 3, 0, 32, 0, 1, 0, 16];
        // directory: 3 records at 12..60
        let entries = [
            (b"head" as &[u8; 4], 60usize, 54usize),
            (b"maxp" as &[u8; 4], 114, 6),
            (b"name" as &[u8; 4], 120, 30),
        ];
        for (tag, off, len) in entries {
            f.extend_from_slice(tag);
            f.extend_from_slice(&[0, 0, 0, 0]);
            f.extend_from_slice(&[
                (off >> 24) as u8,
                (off >> 16) as u8,
                (off >> 8) as u8,
                off as u8,
            ]);
            f.extend_from_slice(&[
                (len >> 24) as u8,
                (len >> 16) as u8,
                (len >> 8) as u8,
                len as u8,
            ]);
        }
        // head @60: unitsPerEm at +18, indexToLocFormat at +50
        let mut h = vec![0; 54];
        h[18] = 0x08; // unitsPerEm = 0x0800 = 2048
        h[51] = 0x01; // locFormat = long (BE u16)
        f.extend_from_slice(&h);
        // maxp @114: version(4) + numGlyphs(2)
        f.extend_from_slice(&[0, 1, 0, 0, 0x00, 0x21]); // 33 glyphs
                                                        // name @120: format(2) count(2) stringOffset(2) + records + strings
                                                        // one record: platform 3, enc 1, lang 0x409, id 1, "Fz" UTF-16BE
        let mut n = vec![0, 0, 0, 1, 0, 18]; // count=1, strOffset=18
        n.extend_from_slice(&[0, 3, 0, 1, 4, 9, 0, 1, 0, 4, 0, 0]);
        n.extend_from_slice(&[0, 0x46, 0, 0x7A]); // "Fz" UTF-16BE
        f.extend_from_slice(&n);
        f
    }

    #[test]
    fn directory_walks() {
        let d = font();
        let t = parse(&d).unwrap();
        assert_eq!(t.sfnt, 0x0001_0000);
        assert_eq!(t.tables.len(), 3);
        assert_eq!(t.tables[0].name(), "head");
        assert_eq!(t.tables[2].name(), "name");
        let h = head(&d, &t).unwrap();
        assert_eq!(h.units_per_em, 2048);
        assert_eq!(h.index_to_loc_format, 1);
        assert_eq!(maxp(&d, &t), Some(33));
        let ns = names(&d, &t).unwrap();
        assert_eq!(ns.len(), 1);
        assert_eq!(ns[0].label(), "family");
        assert_eq!(ns[0].data, vec![0, 0x46, 0, 0x7A]);
    }

    #[test]
    fn otto_accepted() {
        let mut f = font();
        f[0..4].copy_from_slice(b"OTTO");
        assert_eq!(parse(&f).unwrap().sfnt, 0x4F54_544F);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0, 1, 0, 0]).is_none());
        let mut f = font();
        f[5] = 9; // claim 9 tables, only 3 present
        assert!(parse(&f).is_none());
        let mut g = font();
        g[0] = 0xFF; // wrong magic
        assert!(parse(&g).is_none());
    }

    #[test]
    fn determinism_is_structural() {
        let d = font();
        let a = parse(&d).unwrap();
        let b = parse(&d).unwrap();
        assert_eq!(format!("{:?}", a), format!("{:?}", b));
    }
}
