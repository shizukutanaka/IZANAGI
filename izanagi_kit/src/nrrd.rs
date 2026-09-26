//! NRRD (Nearly Raw Raster Data) text header (teem `nrrd`).
//!
//! The file opens with `NRRD000x` (format version 1-5), then
//! `key: value` lines and `#` comments until a blank line; binary
//! or ASCII data follows. `fields` preserves order; `get`,
//! `sizes`, `dimension` and `data_at` give typed views.
//!
//! ```
//! use izanagi_kit::nrrd::parse;
//!
//! let h = "NRRD0005\n# comment\ntype: uint8\ndimension: 2\nsizes: 4 3\nencoding: raw\n\n";
//! let n = parse(h.as_bytes()).unwrap();
//! assert_eq!(n.dimension, Some(2));
//! assert_eq!(n.sizes(), Some(vec![4, 3]));
//! assert_eq!(n.get("type"), Some("uint8"));
//! assert_eq!(h.as_bytes()[n.data_at..].len(), 0);
//! ```

/// One `key: value` header field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Lower-cased key.
    pub key: String,
    /// Raw value (trimmed).
    pub value: String,
}

/// A parsed NRRD header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nrrd {
    /// Format version digit (1-5).
    pub version: u8,
    /// All `key: value` fields in order.
    pub fields: Vec<Field>,
    /// `dimension` as an integer, when present.
    pub dimension: Option<usize>,
    /// Offset of the data after the blank line.
    pub data_at: usize,
}

impl Nrrd {
    /// First value for `key` (case-insensitive).
    pub fn get(&self, key: &str) -> Option<&str> {
        let key = key.to_ascii_lowercase();
        self.fields
            .iter()
            .find(|f| f.key == key)
            .map(|f| f.value.as_str())
    }
    /// `sizes` parsed as space-separated integers.
    pub fn sizes(&self) -> Option<Vec<u64>> {
        let v = self.get("sizes")?;
        let mut out = Vec::new();
        for t in v.split_ascii_whitespace() {
            out.push(t.parse().ok()?);
        }
        if out.is_empty() {
            return None;
        }
        Some(out)
    }
}

/// Parse the header. Returns `None` on a bad magic line, a missing
/// terminator, or a malformed field.
pub fn parse(d: &[u8]) -> Option<Nrrd> {
    let s = core::str::from_utf8(d).ok()?;
    let mut it = s.splitn(2, '\n');
    let magic = it.next()?;
    let version = magic.trim_end().strip_prefix("NRRD")?.parse::<u32>().ok()?;
    if !(1..=5).contains(&version) {
        return None;
    }
    let mut fields = Vec::new();
    let mut dimension = None;
    let mut pos = magic.len() + 1;
    for line in it.next()?.split('\n') {
        let t = line.trim();
        pos += line.len() + 1;
        if t.is_empty() {
            return Some(Nrrd {
                version: version as u8,
                fields,
                dimension,
                data_at: pos.min(d.len()),
            });
        }
        if t.starts_with('#') || t.contains(":=") {
            continue; // comments and key:=value "fields" skipped
        }
        if let Some((k, v)) = t.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let value = v.trim().to_string();
            if key == "dimension" {
                dimension = value.parse().ok();
            }
            fields.push(Field { key, value });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_data_offset() {
        let h = "NRRD0004\n# hello\ntype: short\ndimension: 3\nsizes: 64 64 21\nencoding: gzip\nendian: little\n\nDATA";
        let d = h.as_bytes();
        let n = parse(d).unwrap();
        assert_eq!(n.version, 4);
        assert_eq!(n.dimension, Some(3));
        assert_eq!(n.sizes(), Some(vec![64, 64, 21]));
        assert_eq!(n.get("TYPE"), Some("short"));
        assert_eq!(n.get("endian"), Some("little"));
        assert_eq!(n.get("missing"), None);
        assert_eq!(&d[n.data_at..], b"DATA");
    }

    #[test]
    fn attached_data_and_comments() {
        let h = "NRRD0001\ntype: float\nsizes: 1\n\nxyz";
        let n = parse(h.as_bytes()).unwrap();
        assert_eq!(n.sizes(), Some(vec![1]));
        assert_eq!(&h.as_bytes()[n.data_at..], b"xyz");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"not-nrrd\n").is_none());
        assert!(parse(b"NRRD0009\ntype: x\n\n").is_none());
        // no terminating blank line at all (input simply ends)
        assert!(parse(b"NRRD0002\ntype: uint8\nsizes: 4").is_none());
    }
}
