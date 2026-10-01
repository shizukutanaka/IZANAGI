//! Open Sound Control (OSC) — `#bundle` packets and plain `/address ,typetags`
//! messages, big-endian and 4-byte aligned.
//!
//! ```
//! let mut d = vec![b'/'];
//! d.extend_from_slice(b"freq");
//! d.extend_from_slice(&[0; 3]); // pad to 8 (4-aligned)
//! d.extend_from_slice(b",f"); // typetags
//! d.extend_from_slice(&[0; 2]); // pad to 4
//! d.extend_from_slice(&[0, 0, 0, 0]); // f32 0
//! let o = izanagi_kit::osc::parse(&d).unwrap();
//! assert_eq!(o.address, "/freq");
//! assert_eq!(o.typetags, "f");
//! assert_eq!(o.args, 4);
//! assert!(izanagi_kit::osc::detect(&d));
//! ```

/// A parsed OSC packet (message, not bundle contents).
#[derive(Debug, Clone)]
pub struct Osc {
    /// `true` for `#bundle` packets.
    pub is_bundle: bool,
    /// Address pattern (`/foo/bar`), empty for bundles.
    pub address: String,
    /// Typetag string after the leading `,` (e.g. `"ifs"`).
    pub typetags: String,
    /// Argument bytes following the typetag block.
    pub args: usize,
    /// For bundles: element count.
    pub elements: usize,
    /// For bundles: the 64-bit timetag.
    pub timetag: u64,
}

fn read_cstr(b: &[u8], off: usize) -> Option<(String, usize)> {
    let rel = b.get(off..)?;
    let end = rel.iter().position(|&c| c == 0)?;
    let s = std::str::from_utf8(&rel[..end]).ok()?.to_string();
    let pad = (end + 1 + 3) & !3;
    Some((s, off + pad))
}

/// Detects OSC: `#bundle\0` or `/addr\0` + `,tags`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.starts_with(b"#bundle\0") {
        return true;
    }
    if b.first() != Some(&b'/') {
        return false;
    }
    let Some((_, off)) = read_cstr(b, 0) else {
        return false;
    };
    b.get(off) == Some(&b',')
}

/// Parses one OSC packet (a single message or a bundle).
#[must_use]
pub fn parse(b: &[u8]) -> Option<Osc> {
    if b.starts_with(b"#bundle\0") {
        if b.len() < 16 {
            return None;
        }
        let timetag = u64::from(b[8]) << 56
            | u64::from(b[9]) << 48
            | u64::from(b[10]) << 40
            | u64::from(b[11]) << 32
            | u64::from(b[12]) << 24
            | u64::from(b[13]) << 16
            | u64::from(b[14]) << 8
            | u64::from(b[15]);
        let mut elements = 0;
        let mut off = 16;
        while off + 4 <= b.len() {
            let len = u32::from(b[off]) << 24
                | u32::from(b[off + 1]) << 16
                | u32::from(b[off + 2]) << 8
                | u32::from(b[off + 3]);
            let len = len as usize;
            if len == 0 || off + 4 + len > b.len() {
                break;
            }
            elements += 1;
            off += 4 + len;
        }
        return Some(Osc {
            is_bundle: true,
            address: String::new(),
            typetags: String::new(),
            args: 0,
            elements,
            timetag,
        });
    }
    if !detect(b) {
        return None;
    }
    let (address, off) = read_cstr(b, 0)?;
    if !address.starts_with('/') {
        return None;
    }
    if b.get(off) != Some(&b',') {
        return None;
    }
    let (typetags, off) = read_cstr(b, off)?;
    let args = b.len().saturating_sub(off);
    Some(Osc {
        is_bundle: false,
        address,
        typetags: typetags.trim_start_matches(',').to_string(),
        args,
        elements: 0,
        timetag: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg() -> Vec<u8> {
        let mut d = vec![b'/'];
        d.extend_from_slice(b"freq");
        d.extend_from_slice(&[0; 3]);
        d.extend_from_slice(b",f");
        d.extend_from_slice(&[0; 2]);
        d.extend_from_slice(&[0; 4]);
        d
    }

    #[test]
    fn parses_message() {
        let o = parse(&msg()).unwrap();
        assert_eq!(o.address, "/freq");
        assert_eq!(o.typetags, "f");
        assert_eq!(o.args, 4);
        assert!(!o.is_bundle);
    }

    #[test]
    fn parses_bundle() {
        let mut d = b"#bundle\0".to_vec();
        d.extend_from_slice(&[0; 8]); // timetag 1 → bytes all zero
                                      // element: size 12 + 12 bytes payload
        d.extend_from_slice(&[0, 0, 0, 12]);
        d.extend_from_slice(&[0; 12]);
        let o = parse(&d).unwrap();
        assert!(o.is_bundle);
        assert_eq!(o.elements, 1);
        assert!(detect(&d));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello").is_none());
        assert!(parse(b"/noTags").is_none());
        assert!(!detect(b"#bundl"));
    }
}
