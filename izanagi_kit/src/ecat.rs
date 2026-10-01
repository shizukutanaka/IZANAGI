//! ECAT 7.x image file header (Siemens/CTI PET/SPECT): 512-byte main
//! header beginning with the `MATRIX` signature (`MATRIX71`, …),
//! followed by `original_file_name` char\[32\], `sw_version` i16,
//! `system_type` i16, `file_type` i16, `serial_number` char\[10\] and
//! `scan_start_time` u32 — all big-endian.
//!
//! ```
//! let mut h = vec![0u8; 512];
//! h[..7].copy_from_slice(b"MATRIX7");
//! h[14..22].copy_from_slice(b"patient1");
//! h[46..48].copy_from_slice(&[0, 72]); // sw_version 72
//! h[50..52].copy_from_slice(&[0, 3]);  // file_type
//! let e = izanagi_kit::ecat::parse(&h).unwrap();
//! assert_eq!(e.sw_version, 72);
//! assert_eq!(e.file_type, 3);
//! ```

use std::string::String;

/// A parsed ECAT main header.
#[derive(Clone, Debug)]
pub struct Ecat {
    /// Magic string, e.g. `MATRIX71`.
    pub magic: String,
    /// Original file name (may be empty).
    pub original_name: String,
    /// Software version that wrote the file.
    pub sw_version: i16,
    /// Scanner system type code.
    pub system_type: i16,
    /// File-type code (e.g. 3 = image).
    pub file_type: i16,
    /// Gantry serial number.
    pub serial: String,
    /// Scan start time (seconds since epoch — platform epoch).
    pub scan_start_time: u32,
    /// Isotope name (`isotope_name` char\[8\]).
    pub isotope: String,
}

fn trim(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim_end().to_string()
}
fn be16(d: &[u8], o: usize) -> i16 {
    ((u16::from(d[o]) << 8) | u16::from(d[o + 1])) as i16
}
fn be32(d: &[u8], o: usize) -> u32 {
    (u32::from(d[o]) << 24)
        | (u32::from(d[o + 1]) << 16)
        | (u32::from(d[o + 2]) << 8)
        | u32::from(d[o + 3])
}

/// Parse an ECAT main header; `None` unless ≥512 bytes with the
/// `MATRIX` signature.
pub fn parse(d: &[u8]) -> Option<Ecat> {
    if d.len() < 512 || !d.starts_with(b"MATRIX") {
        return None;
    }
    Some(Ecat {
        magic: trim(&d[..14]),
        original_name: trim(&d[14..46]),
        sw_version: be16(d, 46),
        system_type: be16(d, 48),
        file_type: be16(d, 50),
        serial: trim(&d[52..62]),
        scan_start_time: be32(d, 62),
        isotope: trim(&d[66..74]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        let mut h = vec![0u8; 512];
        h[..7].copy_from_slice(b"MATRIX7");
        h[14..22].copy_from_slice(b"scan0001");
        h[46] = 0;
        h[47] = 71;
        h[48] = 0;
        h[49] = 5;
        h[50] = 0;
        h[51] = 3;
        h[52..57].copy_from_slice(b"SN123");
        h[62..66].copy_from_slice(&[0, 0, 1, 0x2C]); // 300 s
        h[66..70].copy_from_slice(b"F-18");
        let e = parse(&h).unwrap();
        assert_eq!(e.magic, "MATRIX7");
        assert_eq!(e.original_name, "scan0001");
        assert_eq!(e.sw_version, 71);
        assert_eq!(e.system_type, 5);
        assert_eq!(e.file_type, 3);
        assert_eq!(e.serial, "SN123");
        assert_eq!(e.scan_start_time, 300);
        assert_eq!(e.isotope, "F-18");
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&vec![0u8; 511]).is_none());
        let mut h = vec![0u8; 512];
        assert!(parse(&h).is_none());
        h[..6].copy_from_slice(b"MATRIX");
        assert!(parse(&h).is_some());
    }
}
