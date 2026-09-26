//! OS/2 Linear Executable (`LE` / `LX`) header.
//!
//! Like NE, an `MZ` stub's `e_lfanew` points at the linear header —
//! `LE` (OS/2 and Windows VxD) or `LX` (32-bit OS/2). The header
//! carries byte/word order, format level, CPU and OS types, module
//! flags, page size, object counts, the initial EIP/ESP object
//! pointers and the offsets of the object, page, resource, name,
//! entry, import and fixup tables.
//!
//! ```
//! use izanagi_kit::le::{parse, HEADER};
//!
//! let mut d = vec![0u8; 0x200];
//! d[0] = b'M'; d[1] = b'Z';
//! d[0x3c] = 0x80;
//! let h = &mut d[0x80..];
//! h[0] = b'L'; h[1] = b'E';
//! let put = |h: &mut [u8], at: usize, v: u32| {
//!     h[at] = v as u8; h[at + 1] = (v >> 8) as u8;
//!     h[at + 2] = (v >> 16) as u8; h[at + 3] = (v >> 24) as u8;
//! };
//! h[12] = 2;              // i386
//! h[14] = 3;              // OS/2
//! put(h, 24, 4);          // 4 pages
//! put(h, 44, 0x1000);     // 4 KiB pages
//! put(h, 68, 3);          // 3 objects
//! let l = parse(&d).unwrap();
//! assert_eq!(l.kind, izanagi_kit::le::Kind::Le);
//! assert_eq!(l.pages, 4);
//! assert_eq!(l.objects, 3);
//! ```

/// Minimum header size parsed here (through the fixup tables).
pub const HEADER: usize = 0x80;

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// Signature variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `LE` — OS/2 LX predecessor, Windows VxD.
    Le,
    /// `LX` — 32-bit OS/2 modules.
    Lx,
}

/// A parsed LE/LX header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Le {
    /// File offset of the header.
    pub at: usize,
    /// Signature variant.
    pub kind: Kind,
    /// Byte and word ordering bytes (0 = little).
    pub ordering: (u8, u8),
    /// Executable format level.
    pub format_level: u32,
    /// CPU type (1 286, 2 386, 3 486, 0x20 MIPS, ...).
    pub cpu: u16,
    /// Target OS (1 OS/2, 2 Windows, 3 DOS4, 4 Windows 386).
    pub os: u16,
    /// Module version.
    pub module_version: u32,
    /// Module flags (bit 4 = DLL/library, etc.).
    pub module_flags: u32,
    /// Number of memory pages.
    pub pages: u32,
    /// Entry object index (1-based) and offset.
    pub eip_object: u32,
    /// Initial EIP.
    pub eip: u32,
    /// Stack object index (1-based) and offset.
    pub esp_object: u32,
    /// Initial ESP.
    pub esp: u32,
    /// Memory page size.
    pub page_size: u32,
    /// Bytes on the last page (LE only).
    pub last_page_size: u32,
    /// Fixup size/checksum.
    pub fixup_size: u32,
    /// Object table offset (relative to header) and entry count.
    pub object_table: u32,
    /// Number of objects.
    pub objects: u32,
    /// Object page map offset.
    pub object_pages: u32,
    /// Object iterated-data map offset.
    pub object_iter: u32,
    /// Resource table offset and entry count.
    pub resource_table: u32,
    /// Number of resources.
    pub resources: u32,
    /// Resident name table offset.
    pub resident_names: u32,
    /// Entry table offset.
    pub entry_table: u32,
    /// Module directives table offset.
    pub module_directives: u32,
    /// Module directive count.
    pub module_directive_count: u32,
    /// Fixup page table offset.
    pub fixup_pages: u32,
    /// Fixup record table offset.
    pub fixup_records: u32,
}

impl Le {
    /// True for library modules (module flag bit 4, 0x00000010...
    /// actually bit 15 marks a DLL in LX terminology; both are
    /// exposed raw — this checks the documented 0x8000 bit).
    pub fn is_dll(&self) -> bool {
        self.module_flags & 0x8000 != 0
    }
}

/// Parse the header. Returns `None` when the MZ stub or LE/LX
/// signature is missing or the header is truncated.
pub fn parse(d: &[u8]) -> Option<Le> {
    if d.get(..2)? != b"MZ" {
        return None;
    }
    let at = usize::try_from(le32(d, 0x3c)?).ok()?;
    let h = d.get(at..at.checked_add(HEADER)?)?;
    let kind = match h.get(..2)? {
        b"LE" => Kind::Le,
        b"LX" => Kind::Lx,
        _ => return None,
    };
    Some(Le {
        at,
        kind,
        ordering: (h[4], h[5]),
        format_level: le32(h, 8)?,
        cpu: le16(h, 12)?,
        os: le16(h, 14)?,
        module_version: le32(h, 16)?,
        module_flags: le32(h, 20)?,
        pages: le32(h, 24)?,
        eip_object: le32(h, 28)?,
        eip: le32(h, 32)?,
        esp_object: le32(h, 36)?,
        esp: le32(h, 40)?,
        page_size: le32(h, 44)?,
        last_page_size: le32(h, 48)?,
        fixup_size: le32(h, 52)?,
        object_table: le32(h, 64)?,
        objects: le32(h, 68)?,
        object_pages: le32(h, 72)?,
        object_iter: le32(h, 76)?,
        resource_table: le32(h, 80)?,
        resources: le32(h, 84)?,
        resident_names: le32(h, 88)?,
        entry_table: le32(h, 92)?,
        module_directives: le32(h, 96)?,
        module_directive_count: le32(h, 100)?,
        fixup_pages: le32(h, 104)?,
        fixup_records: le32(h, 108)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(sig: u8) -> Vec<u8> {
        let mut d = vec![0u8; 0x200];
        d[0] = b'M';
        d[1] = b'Z';
        d[0x3c] = 0x80;
        let at = 0x80;
        d[at] = b'L';
        d[at + 1] = sig;
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
        };
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
            d[o + 2] = (v >> 16) as u8;
            d[o + 3] = (v >> 24) as u8;
        };
        w16(&mut d, at + 12, 2); // i386
        w16(&mut d, at + 14, 1); // OS/2
        w32(&mut d, at + 20, 0x8000); // DLL bit
        w32(&mut d, at + 24, 7);
        w32(&mut d, at + 28, 1);
        w32(&mut d, at + 32, 0x1234); // EIP object 1 +0x1234
        w32(&mut d, at + 36, 2);
        w32(&mut d, at + 40, 0x8000); // ESP object 2 +0x8000
        w32(&mut d, at + 44, 0x1000);
        w32(&mut d, at + 64, 0xc4);
        w32(&mut d, at + 68, 3);
        w32(&mut d, at + 80, 0x300);
        w32(&mut d, at + 84, 5);
        d
    }

    #[test]
    fn le_fields() {
        let d = fixture(b'E');
        let l = parse(&d).unwrap();
        assert_eq!(l.kind, Kind::Le);
        assert_eq!(l.cpu, 2);
        assert_eq!(l.os, 1);
        assert!(l.is_dll());
        assert_eq!(l.pages, 7);
        assert_eq!(l.eip_object, 1);
        assert_eq!(l.eip, 0x1234);
        assert_eq!(l.esp_object, 2);
        assert_eq!(l.esp, 0x8000);
        assert_eq!(l.page_size, 0x1000);
        assert_eq!(l.object_table, 0xc4);
        assert_eq!(l.objects, 3);
        assert_eq!(l.resource_table, 0x300);
        assert_eq!(l.resources, 5);
    }

    #[test]
    fn lx_accepted() {
        let d = fixture(b'X');
        let l = parse(&d).unwrap();
        assert_eq!(l.kind, Kind::Lx);
        assert!(l.is_dll());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 32]).is_none());
        let mut d = fixture(b'E');
        d[0x81] = b'Q';
        assert!(parse(&d).is_none());
        d[0x3c] = 0; // points at "MZ" itself
        assert!(parse(&d).is_none());
    }
}
