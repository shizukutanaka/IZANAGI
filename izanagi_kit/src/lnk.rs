//! Windows Shell Link (`.lnk`) — MS-SHLLINK binary format.
//!
//! A `.lnk` opens with a fixed 76-byte `SHELL_LINK_HEADER`:
//! size `0x4C`, the `00021401`-class CLSID, `LinkFlags`,
//! `FileAttributes`, three `FILETIME` stamps, file size, icon
//! index, `ShowCommand` and `HotKey`. An optional
//! `LinkTargetIDList` (u16 size + entries), `LinkInfo` block and
//! counted string-data fields follow, gated by flag bits.
//!
//! ```
//! use izanagi_kit::lnk::{parse, CLSID};
//! let mut d = vec![0u8; 76];
//! d[0..4].copy_from_slice(&[0x4C, 0, 0, 0]);
//! d[4..20].copy_from_slice(&CLSID);
//! let l = parse(&d).unwrap();
//! assert_eq!(l.flags, 0);
//! ```

/// Fixed header size and its required value.
pub const HEADER: usize = 76;
/// `CLSID_ShellLink` on disk: `01 14 02 00 .. C0 00 00 00 00 00 00 46`.
pub const CLSID: [u8; 16] = [
    0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

/// `LinkFlags`: a `LinkTargetIDList` follows the header.
pub const HAS_ID_LIST: u32 = 0x0000_0001;
/// `LinkFlags`: a `LinkInfo` block is present.
pub const HAS_LINK_INFO: u32 = 0x0000_0002;
/// `LinkFlags`: a `NAME` string field exists.
pub const HAS_NAME: u32 = 0x0000_0004;
/// `LinkFlags`: a `RELATIVE_PATH` string field exists.
pub const HAS_RELATIVE_PATH: u32 = 0x0000_0008;
/// `LinkFlags`: a `WORKING_DIR` string field exists.
pub const HAS_WORKING_DIR: u32 = 0x0000_0010;
/// `LinkFlags`: an `ARGUMENTS` string field exists.
pub const HAS_ARGUMENTS: u32 = 0x0000_0020;
/// `LinkFlags`: an `ICON_LOCATION` string field exists.
pub const HAS_ICON_LOCATION: u32 = 0x0000_0040;
/// `LinkFlags`: counted strings are UTF-16, else ANSI.
pub const IS_UNICODE: u32 = 0x0000_0080;
/// `LinkFlags`: run without a console window (`SW_SHOWMINNOACTIVE`).
pub const RUN_MINIMIZED: u32 = 0x0000_0400;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}
fn le64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(le32(d, at)?) | u64::from(le32(d, at + 4)?) << 32)
}
fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

/// A parsed `SHELL_LINK_HEADER` plus the ID-list extent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lnk {
    /// `LinkFlags` bitfield (`HAS_*` constants).
    pub flags: u32,
    /// `FileAttributes` (read-only, hidden, directory, …).
    pub file_attrs: u32,
    /// Creation `FILETIME`.
    pub created: u64,
    /// Access `FILETIME`.
    pub accessed: u64,
    /// Write `FILETIME`.
    pub modified: u64,
    /// Target file size (low 32 bits).
    pub file_size: u32,
    /// Icon index.
    pub icon_index: u32,
    /// `ShowCommand` (1 normal, 3 maximized, 7 minimized).
    pub show_command: u32,
    /// `HotKey` virtual-key code pair.
    pub hotkey: u16,
    /// `(offset, len)` of the `LinkTargetIDList` block when
    /// [`HAS_ID_LIST`] is set.
    pub id_list: Option<(usize, usize)>,
    /// Byte offset of the `LinkInfo` block (or the first string
    /// field when `HAS_LINK_INFO` is clear).
    pub link_info_at: usize,
}

impl Lnk {
    /// Test a `LinkFlags` bit.
    pub fn has(&self, flag: u32) -> bool {
        self.flags & flag != 0
    }
    /// True when counted string fields are UTF-16.
    pub fn is_unicode(&self) -> bool {
        self.has(IS_UNICODE)
    }
}

/// Parse the 76-byte header and the optional ID-list size.
/// `None` on a wrong size/CLSID or a truncated buffer.
pub fn parse(d: &[u8]) -> Option<Lnk> {
    let h = d.get(..HEADER)?;
    if le32(h, 0)? != HEADER as u32 || h.get(4..20)? != CLSID {
        return None;
    }
    let flags = le32(h, 20)?;
    let id_list = if flags & HAS_ID_LIST != 0 {
        let n = usize::from(le16(d, HEADER)?);
        if d.len() < HEADER + 2 + n {
            return None;
        }
        Some((HEADER + 2, n))
    } else {
        None
    };
    Some(Lnk {
        flags,
        file_attrs: le32(h, 24)?,
        created: le64(h, 28)?,
        accessed: le64(h, 36)?,
        modified: le64(h, 44)?,
        file_size: le32(h, 52)?,
        icon_index: le32(h, 56)?,
        show_command: le32(h, 60)?,
        hotkey: le16(h, 64)?,
        link_info_at: HEADER + id_list.map(|(_, n)| 2 + n).unwrap_or(0),
        id_list,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(flags: u32, id: usize) -> Vec<u8> {
        let mut d = vec![0u8; HEADER + 2 + id + 16];
        let w = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[o + i] = (v >> (i * 8)) as u8;
            }
        };
        w(&mut d, 0, 76);
        d[4..20].copy_from_slice(&CLSID);
        w(&mut d, 20, flags);
        w(&mut d, 24, 0x20); // archive attr
        w(&mut d, 28, 0x11223344);
        w(&mut d, 32, 0x55667788); // created ft
        w(&mut d, 52, 12345); // file size
        w(&mut d, 60, 7); // show minimized
        d[64] = 0x42;
        d[65] = 0x03; // hotkey Ctrl+B
        if flags & HAS_ID_LIST != 0 {
            d[HEADER] = id as u8;
            d[HEADER + 1] = (id >> 8) as u8;
        }
        d
    }

    #[test]
    fn bare_header() {
        let l = parse(&fixture(0, 0)).unwrap();
        assert_eq!(l.flags, 0);
        assert_eq!(l.file_attrs, 0x20);
        assert_eq!(l.created, 0x55667788_11223344);
        assert_eq!(l.file_size, 12345);
        assert_eq!(l.show_command, 7);
        assert_eq!(l.hotkey, 0x0342);
        assert_eq!(l.id_list, None);
        assert_eq!(l.link_info_at, 76);
        assert!(!l.has(HAS_LINK_INFO));
        assert!(!l.is_unicode());
    }

    #[test]
    fn id_list_extent() {
        let l = parse(&fixture(HAS_ID_LIST | IS_UNICODE, 30)).unwrap();
        assert_eq!(l.id_list, Some((78, 30)));
        assert_eq!(l.link_info_at, 76 + 2 + 30);
        assert!(l.is_unicode());
        assert!(l.has(HAS_ID_LIST));
        // declared list longer than the buffer -> None
        let mut d = fixture(HAS_ID_LIST, 4);
        d[HEADER] = 200;
        assert!(parse(&d).is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 40]).is_none());
        let mut d = fixture(0, 0);
        d[0] = 0x4B; // wrong header size
        assert!(parse(&d).is_none());
        let mut d2 = fixture(0, 0);
        d2[19] = 0x47; // CLSID tail wrong
        assert!(parse(&d2).is_none());
    }
}
