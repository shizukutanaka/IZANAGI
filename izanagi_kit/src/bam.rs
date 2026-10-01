//! BAM (Binary Alignment/Map, hts-specs `SAMv1.tex`): `BAM\x01`
//! magic, `l_text` i32le + SAM-text header, `n_ref` i32le + per-ref
//! `l_name`/`name`/`l_ref` dictionary, then a chain of alignment
//! records each prefixed by `block_size` i32le (core fields
//! `refID`,`pos`,`l_read_name`,`mapq`,`bin`,`n_cigar_op`,`flag`,
//! `l_seq`,`next_refID`,`next_pos`,`tlen` = 32 bytes + read name,
//! cigar, seq, qual, aux tags).
//!
//! ```
//! let mut d = b"BAM\x01\x0b\x00\x00\x00@HD\tVN:1.6\n".to_vec();
//! d.extend_from_slice(&1i32.to_le_bytes()); // n_ref
//! d.extend_from_slice(&5i32.to_le_bytes()); // l_name (NUL included)
//! d.extend_from_slice(b"chr1\0");
//! d.extend_from_slice(&1000i32.to_le_bytes()); // l_ref
//! let b = izanagi_kit::bam::parse(&d).unwrap();
//! assert_eq!(b.refs, 1);
//! assert_eq!(b.records, 0);
//! assert!(izanagi_kit::bam::detect(&d));
//! ```

/// Census of a BAM stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Bam {
    /// `l_text` — SAM header text length.
    pub header_len: u32,
    /// `@HD`/`@SQ`/`@RG`/`@PG`/`@CO` line counts inside the header.
    pub hd_lines: u32,
    /// `@SQ` dictionary entries declared in the header text.
    pub sq_lines: u32,
    /// `n_ref` reference dictionary entries.
    pub refs: u32,
    /// Alignment record `block_size` chain entries walked.
    pub records: u32,
    /// Truncated mid-record.
    pub truncated: bool,
    /// Total declared record bytes (sum of `block_size` + 4 each).
    pub record_bytes: u64,
}

fn i32le(b: &[u8], i: usize) -> Option<i32> {
    let x = b.get(i..i + 4)?;
    Some((x[0] as i32) | ((x[1] as i32) << 8) | ((x[2] as i32) << 16) | ((x[3] as i32) << 24))
}

/// `true` on the `BAM\x01` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"BAM\x01")
}

/// Census; `None` without the magic.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Bam> {
    if !detect(b) {
        return None;
    }
    let mut i = 4usize;
    let l_text = i32le(b, i)? as usize;
    i += 4;
    let text_end = i.saturating_add(l_text).min(b.len());
    let text = core::str::from_utf8(&b[i.min(text_end)..text_end]).unwrap_or("");
    let hd_lines = text.lines().filter(|l| l.starts_with('@')).count() as u32;
    let sq_lines = text.lines().filter(|l| l.starts_with("@SQ")).count() as u32;
    i = text_end;
    let n_ref = i32le(b, i).unwrap_or(0).max(0) as u32;
    i += 4;
    let mut refs = 0u32;
    while refs < n_ref {
        let l_name = match i32le(b, i) {
            Some(v) if v > 0 => v as usize,
            _ => break,
        };
        i += 4 + l_name;
        if i + 4 > b.len() {
            break;
        }
        i += 4; // l_ref
        refs += 1;
    }
    let mut records = 0u32;
    let mut record_bytes = 0u64;
    let mut truncated = false;
    while i + 4 <= b.len() {
        let block = i32le(b, i).unwrap_or(0);
        i += 4;
        if block <= 0 || i + block as usize > b.len() {
            truncated = true;
            break;
        }
        records += 1;
        record_bytes += block as u64 + 4;
        i += block as usize;
        if records > 10_000_000 {
            break;
        }
    }
    if i < b.len() && !truncated {
        truncated = b.len() - i < 4;
    }
    Some(Bam {
        header_len: l_text as u32,
        hd_lines,
        sq_lines,
        refs,
        records,
        truncated,
        record_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"BAM\x01".to_vec();
        let text = b"@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:1000\n";
        d.extend_from_slice(&(text.len() as i32).to_le_bytes());
        d.extend_from_slice(text);
        d.extend_from_slice(&1i32.to_le_bytes());
        d.extend_from_slice(&5i32.to_le_bytes());
        d.extend_from_slice(b"chr1\0");
        d.extend_from_slice(&1000i32.to_le_bytes());
        // one alignment record: block_size=40 (32B core + 4B read name + 4B qual)
        d.extend_from_slice(&40i32.to_le_bytes());
        d.extend_from_slice(&[0u8; 40]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"BAM"));
        assert!(!detect(b"@HD\tVN:1.6"));
    }

    #[test]
    fn parses() {
        let d = fixture();
        let b = parse(&d).unwrap();
        assert_eq!(b.hd_lines, 2);
        assert_eq!(b.sq_lines, 1);
        assert_eq!(b.refs, 1);
        assert_eq!(b.records, 1);
        assert!(!b.truncated);
        assert_eq!(b.record_bytes, 44);
    }

    #[test]
    fn truncated_record() {
        let mut d = fixture();
        d.extend_from_slice(&999i32.to_le_bytes());
        let b = parse(&d).unwrap();
        assert!(b.truncated);
        assert_eq!(b.records, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
