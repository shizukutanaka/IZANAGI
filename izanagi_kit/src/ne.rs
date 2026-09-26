//! Windows 16-bit New Executable (`NE`) header.
//!
//! An NE file is a DOS `MZ` stub whose `e_lfanew` points at a
//! 64-byte `NE` header: linker version, entry table, CRC, module
//! flags (bit 15 = library module / DLL), DGROUP model bits, heap
//! and stack sizes, CS:IP and SS:SP entry state, the counts and
//! offsets of the segment/resource/import/name tables, and the
//! expected target OS.
//!
//! ```
//! use izanagi_kit::ne::{parse, HEADER};
//!
//! let mut d = vec![0u8; 0x100];
//! d[0] = b'M'; d[1] = b'Z';
//! d[0x3c] = 0x80; // e_lfanew
//! let h = &mut d[0x80..];
//! h[0] = b'N'; h[1] = b'E';
//! h[4] = 0x40; h[6] = 0x20;    // entry table at +0x40, 32 bytes
//! h[28] = 1;                    // one segment
//! h[34] = 0x40;                 // segment table offset
//! h[0x36] = 2;                  // Windows
//! let n = parse(&d).unwrap();
//! assert_eq!(n.segments, 1);
//! assert_eq!(n.os, 2);
//! assert!(!n.is_dll());
//! ```

/// NE header size in bytes.
pub const HEADER: usize = 64;
/// Module flag: library module (DLL).
pub const FLAG_DLL: u16 = 0x8000;

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

/// A parsed NE header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ne {
    /// File offset of the header (`e_lfanew`).
    pub at: usize,
    /// Linker major/minor version.
    pub linker: (u8, u8),
    /// Entry table offset and size, relative to the header.
    pub entry: (u16, u16),
    /// File CRC stored in the header.
    pub crc: u32,
    /// Module flag word.
    pub flags: u16,
    /// Automatic data segment number (0 = DLL style).
    pub data_segment: u16,
    /// Initial heap allocation.
    pub heap: u16,
    /// Initial stack allocation.
    pub stack: u16,
    /// Entry point CS:IP (segment index, offset).
    pub cs_ip: (u16, u16),
    /// Initial stack SS:SP (segment index, offset).
    pub ss_sp: (u16, u16),
    /// Number of segment table entries.
    pub segments: u16,
    /// Number of module reference entries.
    pub module_refs: u16,
    /// Bytes in the non-resident name table.
    pub nonres_names_len: u16,
    /// Table offsets, relative to the header.
    pub segment_table: u16,
    /// Resource table offset, relative to the header.
    pub resource_table: u16,
    /// Resident name table offset, relative to the header.
    pub resident_names: u16,
    /// Module reference table offset, relative to the header.
    pub module_ref_table: u16,
    /// Imported name table offset, relative to the header.
    pub import_names: u16,
    /// Absolute offset of the non-resident name table.
    pub nonres_names_at: u32,
    /// Movable entry point count.
    pub movable_entries: u16,
    /// Logical-sector alignment shift for segment data.
    pub sector_shift: u16,
    /// Expected target OS (1 OS/2, 2 Windows, 4 European DOS, 5 OS/2 EE).
    pub os: u8,
    /// Additional flag byte (fast-load etc.).
    pub os2_flags: u8,
}

impl Ne {
    /// True when the file is a library module (DLL).
    pub fn is_dll(&self) -> bool {
        self.flags & FLAG_DLL != 0
    }
    /// DGROUP model bits: 0 none, 1 single shared, 2 multiple.
    pub fn dgroup_model(&self) -> u16 {
        (self.flags >> 1) & 3
    }
}

/// Parse the header. Returns `None` when the MZ stub or NE
/// signature is missing or the header is truncated.
pub fn parse(d: &[u8]) -> Option<Ne> {
    if d.get(..2)? != b"MZ" {
        return None;
    }
    let at = usize::try_from(le32(d, 0x3c)?).ok()?;
    let h = d.get(at..at.checked_add(HEADER)?)?;
    if h.get(..2)? != b"NE" {
        return None;
    }
    Some(Ne {
        at,
        linker: (h[2], h[3]),
        entry: (le16(h, 4)?, le16(h, 6)?),
        crc: le32(h, 8)?,
        flags: le16(h, 12)?,
        data_segment: le16(h, 14)?,
        heap: le16(h, 16)?,
        stack: le16(h, 18)?,
        cs_ip: (le16(h, 22)?, le16(h, 20)?),
        ss_sp: (le16(h, 26)?, le16(h, 24)?),
        segments: le16(h, 28)?,
        module_refs: le16(h, 30)?,
        nonres_names_len: le16(h, 32)?,
        segment_table: le16(h, 34)?,
        resource_table: le16(h, 36)?,
        resident_names: le16(h, 38)?,
        module_ref_table: le16(h, 40)?,
        import_names: le16(h, 42)?,
        nonres_names_at: le32(h, 44)?,
        movable_entries: le16(h, 48)?,
        sector_shift: le16(h, 50)?,
        os: h.get(0x36).copied()?,
        os2_flags: h.get(0x37).copied()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 0x200];
        d[0] = b'M';
        d[1] = b'Z';
        d[0x3c] = 0x80;
        let at = 0x80;
        d[at] = b'N';
        d[at + 1] = b'E';
        d[at + 2] = 5;
        d[at + 3] = 3; // linker 5.3
        let w = |d: &mut [u8], o: usize, v: u16| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
        };
        w(&mut d, at + 4, 0x40);
        w(&mut d, at + 6, 0x20);
        w(&mut d, at + 12, 0x8004); // DLL + multiple data
        w(&mut d, at + 16, 0x200);
        w(&mut d, at + 18, 0x1000);
        w(&mut d, at + 20, 0);
        w(&mut d, at + 22, 1); // CS:IP = 1:0
        w(&mut d, at + 24, 0x1000);
        w(&mut d, at + 26, 2); // SS:SP = 2:0x1000
        w(&mut d, at + 28, 3);
        w(&mut d, at + 30, 2);
        w(&mut d, at + 34, 0x40);
        w(&mut d, at + 36, 0x60);
        w(&mut d, at + 38, 0x70);
        w(&mut d, at + 40, 0x50);
        w(&mut d, at + 42, 0x58);
        d[at + 0x36] = 2;
        d
    }

    #[test]
    fn fields_decode() {
        let d = fixture();
        let n = parse(&d).unwrap();
        assert_eq!(n.at, 0x80);
        assert_eq!(n.linker, (5, 3));
        assert_eq!(n.entry, (0x40, 0x20));
        assert!(n.is_dll());
        assert_eq!(n.dgroup_model(), 2);
        assert_eq!(n.heap, 0x200);
        assert_eq!(n.stack, 0x1000);
        assert_eq!(n.cs_ip, (1, 0));
        assert_eq!(n.ss_sp, (2, 0x1000));
        assert_eq!(n.segments, 3);
        assert_eq!(n.module_refs, 2);
        assert_eq!(n.segment_table, 0x40);
        assert_eq!(n.resource_table, 0x60);
        assert_eq!(n.resident_names, 0x70);
        assert_eq!(n.module_ref_table, 0x50);
        assert_eq!(n.import_names, 0x58);
        assert_eq!(n.os, 2);
    }

    #[test]
    fn exe_flag_not_dll() {
        let mut d = fixture();
        d[0x80 + 12] = 0x02; // SINGLEDATA, no DLL bit
        d[0x80 + 13] = 0x00;
        let n = parse(&d).unwrap();
        assert!(!n.is_dll());
        assert_eq!(n.dgroup_model(), 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 64]).is_none());
        let mut d = fixture();
        d[0x80] = b'P'; // PE signature instead
        assert!(parse(&d).is_none());
        d[0x80] = b'N';
        d[0x3c] = 0xff; // e_lfanew lands on non-NE bytes
        assert!(parse(&d).is_none());
    }
}
