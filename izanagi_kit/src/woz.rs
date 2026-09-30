//! WOZ (Apple II floppy image, WOZ1/WOZ2): signature
//! `WOZx` + `0xFF 0x0A 0x0D 0x0A` integrity bytes, then
//! `ID u32le` chunks — `INFO`, `TMAP`, `TRKS`, `WRIT`,
//! `META`, `FLUX`.
//!
//! ```
//! let mut d = b"WOZ2\xFF\x0A\x0D\x0A".to_vec();
//! d.extend_from_slice(b"INFO"); d.extend_from_slice(&[60, 0, 0, 0]);
//! d.extend_from_slice(&[3, 1, 1, 0, 1]); // ver, 5.25", wp, sync, cleaned
//! d.extend_from_slice(&[0; 55]); // creator + pad
//! d.extend_from_slice(b"TMAP"); d.extend_from_slice(&[160, 0, 0, 0]);
//! d.extend_from_slice(&[0xFF; 160]);
//! let p = izanagi_kit::woz::parse(&d).unwrap();
//! assert_eq!(p.version, 2);
//! assert_eq!(p.disk_type, 1);
//! assert!(p.has_tmap);
//! assert!(izanagi_kit::woz::detect(&d));
//! ```

/// Census of a WOZ floppy image.
#[derive(Debug, Clone, PartialEq)]
pub struct Woz {
    /// Signature major version (1 or 2).
    pub version: u8,
    /// `INFO` info version byte.
    pub info_version: u8,
    /// `INFO` disk type (1 = 5.25", 2 = 3.5").
    pub disk_type: u8,
    /// `INFO` write-protected flag.
    pub write_protected: u8,
    /// `INFO` synchronized flag.
    pub synchronized: u8,
    /// `INFO` cleaned flag.
    pub cleaned: u8,
    /// Chunks walked after the 8-byte signature.
    pub chunks: u32,
    /// `FLUX` chunks seen.
    pub flux_chunks: u32,
    /// `TMAP` chunk present.
    pub has_tmap: bool,
    /// `TRKS` chunk present.
    pub has_trks: bool,
    /// `TRKS` payload size.
    pub trks_size: u32,
    /// `META` payload size.
    pub meta_size: u32,
    /// `WRIT` chunks seen.
    pub writ_chunks: u32,
    /// A chunk overran the end of the buffer.
    pub truncated: bool,
}

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on the `WOZx` signature plus integrity bytes.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8
        && b[0] == b'W'
        && b[1] == b'O'
        && b[2] == b'Z'
        && (b[3] == b'1' || b[3] == b'2')
        && b[4] == 0xFF
        && b[5] == 0x0A
        && b[6] == 0x0D
        && b[7] == 0x0A
}

/// Census; `None` without the WOZ signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Woz> {
    if !detect(b) {
        return None;
    }
    let mut w = Woz {
        version: b[3] - b'0',
        info_version: 0,
        disk_type: 0,
        write_protected: 0,
        synchronized: 0,
        cleaned: 0,
        chunks: 0,
        flux_chunks: 0,
        has_tmap: false,
        has_trks: false,
        trks_size: 0,
        meta_size: 0,
        writ_chunks: 0,
        truncated: false,
    };
    let mut i = 8usize;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let size = le32(b, i + 4) as usize;
        if i + 8 + size > b.len() {
            w.truncated = true;
            break;
        }
        match id {
            b"INFO" => {
                if size >= 5 {
                    w.info_version = b[i + 8];
                    w.disk_type = b[i + 9];
                    w.write_protected = b[i + 10];
                    w.synchronized = b[i + 11];
                    w.cleaned = b[i + 12];
                }
            }
            b"TMAP" => w.has_tmap = true,
            b"TRKS" => {
                w.has_trks = true;
                w.trks_size += size as u32;
            }
            b"META" => w.meta_size += size as u32,
            b"WRIT" => w.writ_chunks += 1,
            b"FLUX" => w.flux_chunks += 1,
            _ => {}
        }
        w.chunks += 1;
        i += 8 + size;
    }
    if i != b.len() {
        w.truncated = w.truncated || b.len() - i >= 8;
    }
    Some(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"WOZ1\xFF\x0A\x0D\x0A".to_vec();
        d.extend_from_slice(b"INFO");
        d.extend_from_slice(&[60, 0, 0, 0]);
        d.extend_from_slice(&[2, 1, 0, 0, 1]);
        d.extend_from_slice(&[0; 55]);
        d.extend_from_slice(b"META");
        d.extend_from_slice(&[4, 0, 0, 0]);
        d.extend_from_slice(&[1, 2, 3, 4]);
        d.extend_from_slice(b"TRKS");
        d.extend_from_slice(&[32, 0, 0, 0]);
        d.extend_from_slice(&[0; 32]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"WOZ3\xFF\x0A\x0D\x0A"));
        assert!(!detect(b"WOZ1\x00\x0A\x0D\x0A"));
        assert!(!detect(b"woz"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, 1);
        assert_eq!(p.info_version, 2);
        assert_eq!(p.disk_type, 1);
        assert_eq!(p.cleaned, 1);
        assert_eq!(p.chunks, 3);
        assert_eq!(p.meta_size, 4);
        assert_eq!(p.trks_size, 32);
        assert!(!p.truncated);
    }

    #[test]
    fn truncated_chunk() {
        let mut d = b"WOZ2\xFF\x0A\x0D\x0A".to_vec();
        d.extend_from_slice(b"TRKS");
        d.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        d.extend_from_slice(&[0; 4]);
        let p = parse(&d).unwrap();
        assert!(p.truncated);
        assert_eq!(p.chunks, 0);
    }
}
