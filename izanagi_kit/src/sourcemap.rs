//! Source Map v3 (revision 3) JSON parsing + VLQ mappings decode.
//!
//! The `"mappings"` string is `;`-separated lines of `,`-separated
//! base64-VLQ segments: `[gen_col, source?, orig_line?, orig_col?, name?]`
//! — all deltas relative to the previous field of the previous segment.
//! Reuses `crate::json` for the object layer; integer fields only.
//!
//! ```
//! use izanagi_kit::sourcemap;
//! let s = br#"{"version":3,"sources":["a.ts"],"names":[],"mappings":"AAAA;AACA"}"#;
//! let m = sourcemap::parse(s).unwrap();
//! assert_eq!(m.version, 3);
//! assert_eq!(m.sources, vec!["a.ts".to_string()]);
//! assert_eq!(m.lines.len(), 2);
//! ```

use crate::json::{self, Json};
use std::string::String;
use std::vec::Vec;

/// A decoded mappings segment (fields already de-delta'd).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    /// Generated column.
    pub gen_col: i64,
    /// Source index (`None` for unmapped segments).
    pub source: Option<i64>,
    /// Original line.
    pub orig_line: Option<i64>,
    /// Original column.
    pub orig_col: Option<i64>,
    /// Name index.
    pub name: Option<i64>,
}

/// A parsed source map.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceMap {
    /// `version` — must be 3.
    pub version: i64,
    /// `sources` list.
    pub sources: Vec<String>,
    /// `names` list.
    pub names: Vec<String>,
    /// Decoded `mappings`: outer = generated lines, inner = segments.
    pub lines: Vec<Vec<Segment>>,
}

fn vlq(d: &[u8], at: &mut usize) -> Option<i64> {
    let mut shift = 0u32;
    let mut value: u64 = 0;
    loop {
        let c = *d.get(*at)?;
        *at += 1;
        let digit: u64 = match c {
            b'A'..=b'Z' => (c - b'A') as u64,
            b'a'..=b'z' => (c - b'a' + 26) as u64,
            b'0'..=b'9' => (c - b'0' + 52) as u64,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        value |= (digit & 31) << shift;
        shift += 5;
        if digit & 32 == 0 {
            break;
        }
        if shift > 60 {
            return None;
        }
    }
    // sign bit = LSB; value = upper bits, zigzag-style
    let v = (value >> 1) as i64;
    Some(if value & 1 != 0 { -v } else { v })
}

fn strs(v: Option<&Json>) -> Option<Vec<String>> {
    match v {
        Some(Json::Arr(items)) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                match it {
                    Json::Str(s) => out.push(s.clone()),
                    _ => return None,
                }
            }
            Some(out)
        }
        Some(Json::Null) | None => Some(Vec::new()),
        _ => None,
    }
}

fn get<'a>(m: &'a std::collections::BTreeMap<String, Json>, k: &str) -> Option<&'a Json> {
    m.get(k)
}

/// Parses a Source Map v3 object; decodes the VLQ `mappings`.
pub fn parse(d: &[u8]) -> Option<SourceMap> {
    let root = match json::parse(d) {
        Ok(Json::Obj(m)) => m,
        _ => return None,
    };
    let version = match get(&root, "version") {
        Some(Json::Int(3)) => 3,
        _ => return None,
    };
    let sources = strs(get(&root, "sources"))?;
    let names = strs(get(&root, "names"))?;
    let mappings = match get(&root, "mappings") {
        Some(Json::Str(s)) => s.as_bytes().to_vec(),
        _ => return None,
    };
    let mut lines: Vec<Vec<Segment>> = Vec::new();
    let mut line: Vec<Segment> = Vec::new();
    let mut prev = [0i64; 5]; // gen_col, source, orig_line, orig_col, name
    let mut at = 0usize;
    while at <= mappings.len() {
        if at == mappings.len() || mappings[at] == b';' {
            lines.push(std::mem::take(&mut line));
            prev[0] = 0; // gen_col resets each line
            at += 1;
            continue;
        }
        if mappings[at] == b',' {
            at += 1;
            continue;
        }
        // decode 1/4/5 fields
        let f0 = vlq(&mappings, &mut at)?;
        let mut seg = Segment {
            gen_col: 0,
            source: None,
            orig_line: None,
            orig_col: None,
            name: None,
        };
        prev[0] += f0;
        seg.gen_col = prev[0];
        if at < mappings.len() && mappings[at] != b',' && mappings[at] != b';' {
            let f1 = vlq(&mappings, &mut at)?;
            let f2 = vlq(&mappings, &mut at)?;
            let f3 = vlq(&mappings, &mut at)?;
            prev[1] += f1;
            prev[2] += f2;
            prev[3] += f3;
            seg.source = Some(prev[1]);
            seg.orig_line = Some(prev[2]);
            seg.orig_col = Some(prev[3]);
            if at < mappings.len() && mappings[at] != b',' && mappings[at] != b';' {
                let f4 = vlq(&mappings, &mut at)?;
                prev[4] += f4;
                seg.name = Some(prev[4]);
            }
        }
        line.push(seg);
    }
    Some(SourceMap {
        version,
        sources,
        names,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlq_decoding() {
        // 'A' = 0 → +0; 'C' = 2 → +1; 'D' = 3 → −1 (sign = LSB)
        let mut at = 0;
        assert_eq!(vlq(b"A", &mut at), Some(0));
        let mut at = 0;
        assert_eq!(vlq(b"C", &mut at), Some(1));
        let mut at = 0;
        assert_eq!(vlq(b"D", &mut at), Some(-1));
        // "gC": 'g' = 32 → continuation, low5 = 0; 'C' = 2 → 2<<5 = 64
        let mut at = 0;
        assert_eq!(vlq(b"gC", &mut at), Some(32));
    }

    #[test]
    fn parses_map() {
        let s = br#"{"version":3,"file":"x.js","sources":["a.ts"],"names":[],"mappings":"AAAA,IAAA;ADAA"}"#;
        let m = parse(s).unwrap();
        assert_eq!(m.version, 3);
        assert_eq!(m.lines.len(), 2);
        assert_eq!(m.lines[0].len(), 2);
        assert_eq!(m.lines[0][1].gen_col, 4); // 'I' = 8 → +4
        assert_eq!(m.lines[0][1].source, Some(0));
        assert_eq!(m.lines[0][1].orig_line, Some(0));
        // line 2: 'A'=0 → gen_col 0; 'D'=3 → −1 → source index −1
        assert_eq!(m.lines[1][0].source, Some(-1));
        assert_eq!(m.lines[1][0].orig_line, Some(0));
    }

    #[test]
    fn rejects() {
        assert!(parse(br#"{"version":2}"#).is_none());
        assert!(parse(br#"{"version":3,"mappings":"!!"}"#).is_none());
        assert!(parse(b"[]").is_none());
    }
}
