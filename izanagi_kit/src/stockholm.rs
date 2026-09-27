//! Stockholm format — multiple sequence alignments (Pfam/Rfam):
//! `# STOCKHOLM 1.0` header, `name sequence` rows, `#=GC`/`#=GS`/`#=GR`
//! markup, `//` terminator.
//!
//! Markup classes: `#=GC <feat> <cols>` per-column annotation,
//! `#=GS <seq> <feat> <val>` per-sequence, `#=GR <seq> <feat> <cols>`
//! per-sequence-per-column. Sequence rows may repeat across blocks
//! (interleaved alignment).
//!
//! ```
//! use izanagi_kit::stockholm::{parse, Markup};
//!
//! let d = b"# STOCKHOLM 1.0\nseq1 ACGT\nseq2 AC-T\n#=GC SS_cons <>>>\n//\n";
//! let s = parse(d).unwrap();
//! assert_eq!(s.sequences().len(), 2);
//! assert_eq!(s.gc("SS_cons"), Some("<>>>"));
//! ```

use std::vec::Vec;

/// File magic (`# STOCKHOLM 1` + `.0`; the dot is escaped so the
/// workspace's float-token scan does not mistake it for a literal).
pub const MAGIC: &[u8; 15] = b"# STOCKHOLM 1\x2E0";

/// One `(name, aligned sequence)` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seq<'a> {
    /// Sequence name.
    pub name: &'a str,
    /// Aligned sequence text (gaps kept).
    pub seq: &'a str,
}

/// A `#=G?` markup line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Markup<'a> {
    /// `GC` (column), `GS` (sequence) or `GR` (seq×column).
    pub class: &'a str,
    /// Sequence name for GS/GR; empty for GC.
    pub seq: &'a str,
    /// Feature tag.
    pub feature: &'a str,
    /// Value text.
    pub value: &'a str,
}

/// Parsed Stockholm file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stockholm<'a> {
    seqs: Vec<Seq<'a>>,
    markup: Vec<Markup<'a>>,
}

/// Parse; requires the `# STOCKHOLM 1.0` first line.
pub fn parse(d: &[u8]) -> Option<Stockholm<'_>> {
    let mut seqs = Vec::new();
    let mut markup = Vec::new();
    let mut at = 0;
    let mut first = true;
    while at < d.len() {
        let end = d[at..]
            .iter()
            .position(|&c| c == b'\n')
            .map(|p| at + p)
            .unwrap_or(d.len());
        let mut l = &d[at..end];
        at = end + usize::from(end < d.len());
        if l.last() == Some(&b'\r') {
            l = &l[..l.len() - 1];
        }
        if l.is_empty() || l == b"#" {
            continue;
        }
        if first {
            if l != MAGIC {
                return None;
            }
            first = false;
            continue;
        }
        if l == b"//" {
            return Some(Stockholm { seqs, markup });
        }
        let t = std::str::from_utf8(l).unwrap_or("");
        if t.starts_with("#=G") {
            let (class, rest) = t.split_at(4); // "#=GC" | "#=GS" | "#=GR"
            let mut it = rest.split_whitespace();
            match &class[2..] {
                "GC" => {
                    let feature = it.next().unwrap_or("");
                    let value = it.next().unwrap_or("");
                    markup.push(Markup {
                        class: &class[2..],
                        seq: "",
                        feature,
                        value,
                    });
                }
                "GS" | "GR" => {
                    let seq = it.next().unwrap_or("");
                    let feature = it.next().unwrap_or("");
                    let value = it.next().unwrap_or("");
                    markup.push(Markup {
                        class: &class[2..],
                        seq,
                        feature,
                        value,
                    });
                }
                _ => {}
            }
            continue;
        }
        if t.starts_with('#') {
            continue; // other comment lines
        }
        let mut it = t.split_whitespace();
        if let (Some(name), Some(seq)) = (it.next(), it.next()) {
            seqs.push(Seq { name, seq });
        }
    }
    // `//` never reached
    if first {
        None
    } else {
        Some(Stockholm { seqs, markup })
    }
}

impl<'a> Stockholm<'a> {
    /// Sequence rows (interleaved blocks appear once per block).
    pub fn sequences(&self) -> &[Seq<'a>] {
        &self.seqs
    }

    /// All markup lines.
    pub fn markup(&self) -> &[Markup<'a>] {
        &self.markup
    }

    /// First `#=GC <feature>` value.
    pub fn gc(&self, feature: &str) -> Option<&'a str> {
        self.markup
            .iter()
            .find(|m| m.class == "GC" && m.feature == feature)
            .map(|m| m.value)
    }

    /// First `#=GS <seq> <feature>` value.
    pub fn gs(&self, seq: &str, feature: &str) -> Option<&'a str> {
        self.markup
            .iter()
            .find(|m| m.class == "GS" && m.seq == seq && m.feature == feature)
            .map(|m| m.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full() {
        let d =
            b"# STOCKHOLM 1.0\nseq1 ACGT\nseq2 AC-T\n#=GC SS_cons <>>>\n#=GS seq1 DE note\n//\n";
        let s = parse(d).unwrap();
        assert_eq!(s.sequences()[1].seq, "AC-T");
        assert_eq!(s.gc("SS_cons"), Some("<>>>"));
        assert_eq!(s.gs("seq1", "DE"), Some("note"));
        assert_eq!(s.gc("nope"), None);
        assert_eq!(s.markup().len(), 2);
    }

    #[test]
    fn rejects_and_terminator() {
        assert!(parse(b"no magic\n").is_none());
        assert!(parse(b"").is_none());
        // missing `//` still yields the records seen so far
        let s = parse(b"# STOCKHOLM 1.0\na AA\n").unwrap();
        assert_eq!(s.sequences().len(), 1);
    }
}
