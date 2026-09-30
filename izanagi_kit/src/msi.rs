//! Windows Installer `.msi`: a CFB (OLE) file whose directory holds
//! `_Tables`/`_StringPool` streams and `\x05`-prefixed summary
//! streams. Detection reuses `crate::ole` and checks for the
//! signature stream names.
//!
//! ```
//! let f = fixture();
//! let m = izanagi_kit::msi::parse(&f).unwrap();
//! assert_eq!(m.tables, 2);
//! assert!(m.has_summary);
//!
//! // Minimal CFB image with an MSI-shaped directory.
//! fn fixture() -> Vec<u8> {
//!     use izanagi_kit::msi::tests_util::cfb;
//!     cfb(&["Root Entry", "\u{5}SummaryInformationDocument", "_Tables", "_StringPool"])
//! }
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed `.msi` directory view.
#[derive(Clone, Debug)]
pub struct Msi {
    /// All directory entry names (storage + stream).
    pub entries: Vec<String>,
    /// Count of Windows Installer table streams (`_Tables`,
    /// `_Columns`, `_StringData`, `_StringPool`, `_Validation`).
    pub tables: usize,
    /// `\x05SummaryInformation(Document)` stream present.
    pub has_summary: bool,
    /// `\x05DigitalSignature` stream present.
    pub has_signature: bool,
    /// CFB major version (3 or 4).
    pub ole_major: u16,
}

fn is_table(name: &str) -> bool {
    matches!(
        name,
        "_Tables" | "_Columns" | "_StringData" | "_StringPool" | "_Validation"
    )
}

/// Parse a `.msi`: `ole` container containing at least one installer
/// table stream. Returns `None` for CFB files that are not MSI
/// (plain `.doc`, `.xls`, `jumplist`, …).
pub fn parse(d: &[u8]) -> Option<Msi> {
    let ole = crate::ole::parse(d)?;
    let mut entries: Vec<String> = Vec::new();
    let mut tables = 0usize;
    let mut has_summary = false;
    let mut has_signature = false;
    for e in &ole.entries {
        let name = &e.name;
        if is_table(name) {
            tables += 1;
        }
        if name.contains("SummaryInformation") {
            has_summary = true;
        }
        if name.contains("DigitalSignature") {
            has_signature = true;
        }
        entries.push(name.clone());
    }
    if tables == 0 {
        return None;
    }
    Some(Msi {
        entries,
        tables,
        has_summary,
        has_signature,
        ole_major: ole.major,
    })
}

/// Test helpers exposed for the doctest (not part of the parser).
#[doc(hidden)]
pub mod tests_util {
    use std::vec::Vec;

    /// Build a minimal major-3 CFB image whose directory lists
    /// `names` (entry 0 is forced to `ENTRY_ROOT` type).
    pub fn cfb(names: &[&str]) -> Vec<u8> {
        let mut d = vec![0u8; 512 * 6];
        d[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
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
        w16(&mut d, 0x1a, 3); // major
        w16(&mut d, 0x1c, 0xfffe); // byte order
        w16(&mut d, 0x1e, 9); // sector shift → 512
        w16(&mut d, 0x20, 6); // mini shift
        w32(&mut d, 0x2c, 1); // dir sector count
        w32(&mut d, 0x30, 1); // FAT sectors
        w32(&mut d, 0x38, 4096); // mini cutoff
        w32(&mut d, 0x3c, 0xFFFFFFFE); // first minifat = ENDOFCHAIN
        w32(&mut d, 0x44, 0xFFFFFFFE); // first difat
        w32(&mut d, 0x4c, 0); // DIFAT[0] = sector 0
        for i in 1..109 {
            w32(&mut d, 0x4c + i * 4, 0xFFFFFFFF); // FREESECT
        }
        // sector 0 = FAT: dir chain in sector 1
        w32(&mut d, 512, 0xFFFFFFFD); // sector0 = FATSECT
        w32(&mut d, 516, 2); // dir sector 1 → continues into sector 2
        w32(&mut d, 520, 0xFFFFFFFE); // sector 2 → ENDOFCHAIN
        let dir = 1024;
        for (i, name) in names.iter().enumerate() {
            let off = dir + i * 128;
            for (j, c) in name.chars().enumerate() {
                let cp = c as u32;
                d[off + j * 2] = cp as u8;
                d[off + j * 2 + 1] = (cp >> 8) as u8;
            }
            let nbytes = (name.chars().count() + 1) * 2;
            d[off + 64] = nbytes as u8;
            d[off + 65] = (nbytes >> 8) as u8;
            d[off + 66] = if i == 0 { 5 } else { 2 }; // root / stream
            w32(&mut d, off + 116, 0xFFFFFFFE);
            w32(&mut d, off + 120, 700);
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msi_directory() {
        let d = tests_util::cfb(&[
            "Root Entry",
            "\u{5}SummaryInformationDocument",
            "_Tables",
            "_StringPool",
            "_Columns",
            "_StringData",
            "\u{5}DigitalSignature",
        ]);
        let m = parse(&d).unwrap();
        assert_eq!(m.tables, 4);
        assert!(m.has_summary);
        assert!(m.has_signature);
        assert_eq!(m.ole_major, 3);
        assert!(m.entries.iter().any(|n| n == "_StringPool"));
    }

    #[test]
    fn rejects_non_msi_cfb() {
        let d = tests_util::cfb(&["Root Entry", "Workbook"]);
        assert!(parse(&d).is_none());
        assert!(parse(b"").is_none());
    }
}
