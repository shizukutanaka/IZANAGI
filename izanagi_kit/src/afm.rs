//! Adobe Font Metrics (`.afm`) file parsing (Adobe spec 5004.AFM).
//!
//! Line-based `Key value` records inside `StartFontMetrics x` …
//! `EndFontMetrics`, with a `StartCharMetrics n` block whose lines are
//! `C <code> ; WX <n> ; N <name> ; B <llx lly urx ury> ;`. Widths are
//! kept as integers (1/1000 em units); `B` boxes stay verbatim.
//!
//! ```
//! use izanagi_kit::afm;
//! let d = b"StartFontMetrics 2.0\nFontName X\nStartCharMetrics 1\n\
//!           C 65 ; WX 600 ; N A ;\nEndCharMetrics\nEndFontMetrics\n";
//! let m = afm::parse(d).unwrap();
//! assert_eq!(m.chars[0].wx, 600);
//! assert_eq!(afm::get(&m, b"FontName"), Some(b"X".as_ref()));
//! ```

use std::vec::Vec;

/// One `C .. ;` character-metrics line.
#[derive(Clone, Debug, PartialEq)]
pub struct CharMetric {
    /// `C` character code (`-1` for unencoded glyphs).
    pub code: i32,
    /// `WX` (or `W0X`) advance width in 1/1000 em.
    pub wx: i32,
    /// `N` glyph name.
    pub name: Vec<u8>,
    /// `B` bounding box as four verbatim integers, if present.
    pub bbox: Option<[i32; 4]>,
}

/// A parsed AFM file.
#[derive(Clone, Debug, PartialEq)]
pub struct Afm {
    /// `StartFontMetrics` version string (`b"2.0"`).
    pub version: Vec<u8>,
    /// Top-level `Key value` pairs (outside sections), in order.
    pub globals: Vec<(Vec<u8>, Vec<u8>)>,
    /// Per-character metrics from `StartCharMetrics`.
    pub chars: Vec<CharMetric>,
}

fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && (s[a] == b' ' || s[a] == b'\t') {
        a += 1;
    }
    while b > a && (s[b - 1] == b' ' || s[b - 1] == b'\t' || s[b - 1] == b'\r') {
        b -= 1;
    }
    &s[a..b]
}

fn int(s: &[u8]) -> Option<i32> {
    let s = trim(s);
    if s.is_empty() {
        return None;
    }
    let (neg, s) = if s[0] == b'-' {
        (true, &s[1..])
    } else {
        (false, s)
    };
    if s.is_empty() || !s.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut v: i64 = 0;
    for &b in s {
        v = v.checked_mul(10)?.checked_add((b - b'0') as i64)?;
        if v > i32::MAX as i64 {
            return None;
        }
    }
    Some(if neg { -(v as i32) } else { v as i32 })
}

/// Splits a `C .. ; WX .. ;` line on `;` and reads `K v` fields.
fn char_line(line: &[u8]) -> Option<CharMetric> {
    let mut code = None;
    let mut wx = None;
    let mut name = None;
    let mut bbox = None;
    for seg in line.split(|&b| b == b';') {
        let seg = trim(seg);
        if seg.is_empty() {
            continue;
        }
        let sp = seg
            .iter()
            .position(|&b| b == b' ' || b == b'\t')
            .unwrap_or(seg.len());
        let (k, v) = (&seg[..sp], trim(&seg[sp..]));
        match k {
            b"C" => code = Some(int(v)?),
            b"WX" | b"W0X" => wx = Some(int(v)?),
            b"N" => name = Some(v.to_vec()),
            b"B" => {
                let nums: Vec<i32> = v
                    .split(|&b| b == b' ')
                    .filter(|w| !w.is_empty())
                    .map(int)
                    .collect::<Option<Vec<i32>>>()?;
                if nums.len() == 4 {
                    bbox = Some([nums[0], nums[1], nums[2], nums[3]]);
                }
            }
            _ => {}
        }
    }
    Some(CharMetric {
        code: code?,
        wx: wx.unwrap_or(0),
        name: name?,
        bbox,
    })
}

/// Parses a whole AFM file. Missing `StartFontMetrics` rejects.
pub fn parse(d: &[u8]) -> Option<Afm> {
    let mut globals = Vec::new();
    let mut chars = Vec::new();
    let mut version = None;
    let mut in_chars = false;
    for raw in d.split(|&b| b == b'\n') {
        let line = trim(raw);
        if line.is_empty() {
            continue;
        }
        if line.starts_with(b"StartFontMetrics") {
            version = Some(trim(&line[16..]).to_vec());
            continue;
        }
        if line.starts_with(b"StartCharMetrics") {
            in_chars = true;
            continue;
        }
        if line.starts_with(b"EndCharMetrics") {
            in_chars = false;
            continue;
        }
        // Any other `Start*`/`End*`/`EndFontMetrics` section: globals only.
        if line.starts_with(b"Start")
            || line.starts_with(b"EndFontMetrics")
            || line.starts_with(b"End") && line.len() > 3 && line[3].is_ascii_uppercase()
        {
            if line.starts_with(b"Start") && !line.starts_with(b"StartFontMetrics") {
                // skip nested sections (KernData, Composites, ...)
                in_chars = false;
            }
            continue;
        }
        if in_chars {
            if let Some(c) = char_line(line) {
                chars.push(c);
            }
            continue;
        }
        let sp = match line.iter().position(|&b| b == b' ' || b == b'\t') {
            Some(p) => p,
            None => continue,
        };
        globals.push((line[..sp].to_vec(), trim(&line[sp..]).to_vec()));
    }
    Some(Afm {
        version: version?,
        globals,
        chars,
    })
}

/// First global `Key` value.
pub fn get<'a>(m: &'a Afm, key: &[u8]) -> Option<&'a [u8]> {
    m.globals
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_and_globals() {
        let d = b"StartFontMetrics 4.1\nFullName Test Font\nStartCharMetrics 2\n\
                  C 65 ; WX 722 ; N A ; B 0 0 700 700 ;\n\
                  C -1 ; WX 250 ; N .notdef ;\n\
                  EndCharMetrics\nEndFontMetrics\n";
        let m = parse(d).unwrap();
        assert_eq!(m.version, b"4.1".to_vec());
        assert_eq!(get(&m, b"FullName"), Some(b"Test Font".as_ref()));
        assert_eq!(m.chars.len(), 2);
        assert_eq!(m.chars[0].bbox, Some([0, 0, 700, 700]));
        assert_eq!(m.chars[1].code, -1);
        assert_eq!(m.chars[1].name, b".notdef".to_vec());
    }

    #[test]
    fn nested_sections_skipped() {
        let d = b"StartFontMetrics 2.0\nStartKernData\nStartKernPairs 1\n\
                  KPX A y -50\nEndKernPairs\nEndKernData\n\
                  StartCharMetrics 1\nC 65 ; WX 600 ; N A ;\nEndCharMetrics\nEndFontMetrics\n";
        let m = parse(d).unwrap();
        assert_eq!(m.chars.len(), 1);
    }

    #[test]
    fn rejects_non_afm() {
        assert!(parse(b"hello").is_none());
        assert!(int(b"1x").is_none());
        assert_eq!(int(b"-250"), Some(-250));
    }
}
