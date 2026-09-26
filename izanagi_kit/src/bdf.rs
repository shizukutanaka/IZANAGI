//! Adobe Glyph Bitmap Distribution Format (BDF, tech note 5005 v2.2) —
//! the plain-text bitmap font format X11 shipped for decades and still
//! used by embedded/terminal renderers. [`parse`] reads the file header
//! (`STARTFONT`, `FONT`, `SIZE`, `FONTBOUNDINGBOX`, optional
//! `STARTPROPERTIES` block) and every glyph record
//! (`STARTCHAR`/`ENCODING`/`SWIDTH`/`DWIDTH`/`BBX`/`BITMAP` hex
//! rows/`ENDCHAR`) into [`Font`]/[`Glyph`]. Bitmap rows are MSB-first,
//! one row per hex string, zero-padded on the right to a byte boundary.
//!
//! ```
//! use izanagi_kit::bdf::parse;
//! let src = "STARTFONT 2.1\nFONT test\nSIZE 8 75 75\nFONTBOUNDINGBOX 8 8 0 -2\nCHARS 1\nSTARTCHAR A\nENCODING 65\nSWIDTH 500 0\nDWIDTH 8 0\nBBX 8 8 0 -2\nBITMAP\n18\n24\n42\n7E\n42\n42\n42\n42\nENDCHAR\nENDFONT\n";
//! let f = parse(src).unwrap();
//! let g = f.glyph_by_encoding(65).unwrap();
//! assert!(g.pixel(3, 0)); // top row 0x18: bits 3,4 only
//! assert!(!g.pixel(0, 0));
//! assert!(g.pixel(5, 1)); // row 1 = 0x24: bits 2,5
//! assert!(!g.pixel(4, 1));
//! ```

use std::string::String;
use std::vec::Vec;

/// A bounding box in glyph metrics space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    /// Width in pixels.
    pub w: i64,
    /// Height in pixels.
    pub h: i64,
    /// X offset (left edge relative to origin).
    pub x: i64,
    /// Y offset (bottom edge relative to baseline).
    pub y: i64,
}

/// One glyph record.
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    /// `STARTCHAR` name.
    pub name: String,
    /// `ENCODING` code point (`-1` maps to `None` — unencoded glyph).
    pub encoding: Option<i64>,
    /// `SWIDTH` scalable width (thousandths of em); `DWIDTH` pixel
    /// advance. Vertical-writing `SWIDTH1`/`DWIDTH1` are kept raw.
    pub swidth: (i64, i64),
    /// `DWIDTH` pixel advance.
    pub dwidth: (i64, i64),
    /// `SWIDTH1`/`DWIDTH1` for vertical text, when present.
    pub v_metrics: Option<((i64, i64), (i64, i64))>,
    /// `BBX` bounding box.
    pub bbx: BBox,
    /// Decoded bitmap: `h` rows × `ceil(w/8)` bytes, MSB-first.
    pub bitmap: Vec<u8>,
}

impl Glyph {
    /// True when pixel `(x, y)` is set — `x` rightward from the bounding
    /// box left edge, `y` downward from its top.
    pub fn pixel(&self, x: i64, y: i64) -> bool {
        if x < 0 || y < 0 || x >= self.bbx.w || y >= self.bbx.h {
            return false;
        }
        let stride = self.bbx.w.div_euclid(8) + if self.bbx.w % 8 == 0 { 0 } else { 1 };
        let idx = (y * stride + x.div_euclid(8)) as usize;
        self.bitmap
            .get(idx)
            .is_some_and(|b| b & (0x80 >> (x % 8)) != 0)
    }
}

/// A parsed BDF font.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    /// `FONT` name string.
    pub name: String,
    /// `SIZE` point size ×10 is NOT scaled: `point` is the integer given.
    pub point: i64,
    /// `SIZE` x/y resolution (dpi).
    pub res: (i64, i64),
    /// `FONTBOUNDINGBOX`.
    pub bbox: BBox,
    /// `STARTPROPERTIES` block contents (`name`, raw value text).
    pub props: Vec<(String, String)>,
    /// Glyphs in file order.
    pub glyphs: Vec<Glyph>,
}

impl Font {
    /// First glyph with this `ENCODING` value.
    pub fn glyph_by_encoding(&self, enc: i64) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| g.encoding == Some(enc))
    }

    /// First glyph with this `STARTCHAR` name.
    pub fn glyph_by_name(&self, name: &str) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| g.name == name)
    }
}

fn ints(l: &str, n: usize) -> Option<Vec<i64>> {
    let v: Vec<i64> = l
        .split_whitespace()
        .map(|t| t.parse::<i64>().ok())
        .collect::<Option<Vec<_>>>()?;
    if v.len() == n {
        Some(v)
    } else {
        None
    }
}

fn hex_row(s: &str, w: i64) -> Option<Vec<u8>> {
    let stride = ((w + 7) / 8) as usize;
    let t = s.trim();
    if t.len() % 2 != 0 || t.len() / 2 < stride || t.len() / 2 > stride + 1 {
        return None;
    }
    let mut row = Vec::with_capacity(t.len() / 2);
    let b = t.as_bytes();
    for k in 0..t.len() / 2 {
        let hi = (b[k * 2] as char).to_digit(16)?;
        let lo = (b[k * 2 + 1] as char).to_digit(16)?;
        row.push(((hi << 4) | lo) as u8);
    }
    // keep exactly `stride` bytes (the pad byte may be absent)
    while row.len() > stride {
        row.pop();
    }
    Some(row)
}

/// Parse a BDF file. `None` on a missing `STARTFONT`/`ENDFONT`, a bad
/// `SIZE`/`FONTBOUNDINGBOX`, a glyph whose bitmap rows don't match its
/// `BBX` height, malformed hex, or a `CHARS` count that disagrees with
/// the records present.
pub fn parse(d: &str) -> Option<Font> {
    let mut lines = d.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = lines.next()?;
    if !first.starts_with("STARTFONT") {
        return None;
    }
    let mut name = String::new();
    let mut point = 0;
    let mut res = (0, 0);
    let mut bbox = BBox {
        w: 0,
        h: 0,
        x: 0,
        y: 0,
    };
    let mut props = Vec::new();
    let mut glyphs = Vec::new();
    let mut want_chars: Option<i64> = None;
    while let Some(l) = lines.next() {
        let (kw, rest) = match l.split_once(char::is_whitespace) {
            Some((k, r)) => (k, r),
            None => (l, ""),
        };
        match kw {
            "FONT" => name = rest.to_string(),
            "SIZE" => {
                let v = ints(rest, 3)?;
                point = v[0];
                res = (v[1], v[2]);
            }
            "FONTBOUNDINGBOX" => {
                let v = ints(rest, 4)?;
                bbox = BBox {
                    w: v[0],
                    h: v[1],
                    x: v[2],
                    y: v[3],
                };
            }
            "STARTPROPERTIES" => {
                let n = ints(rest, 1)?[0];
                let mut got = 0;
                for pl in lines.by_ref() {
                    if pl.starts_with("ENDPROPERTIES") {
                        break;
                    }
                    let (pk, pv) = pl.split_once(char::is_whitespace).unwrap_or((pl, ""));
                    props.push((pk.to_string(), pv.to_string()));
                    got += 1;
                }
                if got != n {
                    return None;
                }
            }
            "CHARS" => want_chars = ints(rest, 1)?[0].into(),
            "STARTCHAR" => {
                let g = glyph(rest, &mut lines)?;
                glyphs.push(g);
            }
            "ENDFONT" => {
                if want_chars.is_some_and(|n| n as usize != glyphs.len()) {
                    return None;
                }
                if name.is_empty() {
                    return None;
                }
                return Some(Font {
                    name,
                    point,
                    res,
                    bbox,
                    props,
                    glyphs,
                });
            }
            _ => {}
        }
    }
    None
}

fn glyph<'a, I: Iterator<Item = &'a str>>(gname: &str, lines: &mut I) -> Option<Glyph> {
    let mut encoding: Option<i64> = None;
    let mut swidth = (0, 0);
    let mut dwidth = (0, 0);
    let mut swidth1: Option<(i64, i64)> = None;
    let mut dwidth1: Option<(i64, i64)> = None;
    let mut bbx = BBox {
        w: 0,
        h: 0,
        x: 0,
        y: 0,
    };
    let mut bitmap = Vec::new();
    while let Some(l) = lines.next() {
        let (kw, rest) = match l.split_once(char::is_whitespace) {
            Some((k, r)) => (k, r),
            None => (l, ""),
        };
        match kw {
            "ENCODING" => {
                let v = ints(rest, 1)?;
                encoding = if v[0] < 0 { None } else { Some(v[0]) };
            }
            "SWIDTH" => {
                let v = ints(rest, 2)?;
                swidth = (v[0], v[1]);
            }
            "DWIDTH" => {
                let v = ints(rest, 2)?;
                dwidth = (v[0], v[1]);
            }
            "SWIDTH1" => {
                let v = ints(rest, 2)?;
                swidth1 = Some((v[0], v[1]));
            }
            "DWIDTH1" => {
                let v = ints(rest, 2)?;
                dwidth1 = Some((v[0], v[1]));
            }
            "BBX" => {
                let v = ints(rest, 4)?;
                bbx = BBox {
                    w: v[0],
                    h: v[1],
                    x: v[2],
                    y: v[3],
                };
            }
            "BITMAP" => {
                for _ in 0..bbx.h {
                    let row = hex_row(lines.next()?, bbx.w)?;
                    bitmap.extend_from_slice(&row);
                }
            }
            "ENDCHAR" => {
                let v_metrics = match (swidth1, dwidth1) {
                    (Some(s), Some(dv)) => Some((s, dv)),
                    _ => None,
                };
                return Some(Glyph {
                    name: gname.to_string(),
                    encoding,
                    swidth,
                    dwidth,
                    v_metrics,
                    bbx,
                    bitmap,
                });
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "STARTFONT 2.1\nCOMMENT a test\nFONT test-family\nSIZE 8 75 75\nFONTBOUNDINGBOX 8 8 0 -2\nSTARTPROPERTIES 1\nCOPYRIGHT \"none\"\nENDPROPERTIES\nCHARS 2\nSTARTCHAR A\nENCODING 65\nSWIDTH 500 0\nDWIDTH 8 0\nBBX 8 8 0 -2\nBITMAP\n18\n24\n42\n7E\n42\n42\n42\n42\nENDCHAR\nSTARTCHAR B\nENCODING 66\nDWIDTH 8 0\nSWIDTH 500 0\nBBX 8 8 0 -2\nBITMAP\n7C\n42\n42\n7C\n42\n42\n42\n7C\nENDCHAR\nENDFONT\n";

    #[test]
    fn parses_font() {
        let f = parse(SRC).unwrap();
        assert_eq!(f.name, "test-family");
        assert_eq!(f.point, 8);
        assert_eq!(f.res, (75, 75));
        assert_eq!(f.bbox.w, 8);
        assert_eq!(f.props.len(), 1);
        assert_eq!(f.glyphs.len(), 2);
        let a = f.glyph_by_encoding(65).unwrap();
        assert_eq!(a.name, "A");
        assert_eq!(a.dwidth, (8, 0));
        assert!(f.glyph_by_name("B").is_some());
        assert!(f.glyph_by_encoding(67).is_none());
    }

    #[test]
    fn bitmap_pixels() {
        let f = parse(SRC).unwrap();
        let a = f.glyph_by_encoding(65).unwrap();
        // row 0 = 0x18 = 00011000 → bits 3,4 set
        assert!(a.pixel(3, 0));
        assert!(a.pixel(4, 0));
        assert!(!a.pixel(0, 0));
        // row 3 = 0x7E = 01111110
        assert!(a.pixel(1, 3) && a.pixel(6, 3));
        assert!(!a.pixel(7, 3));
        assert!(!a.pixel(8, 0) && !a.pixel(-1, 0) && !a.pixel(0, 8));
    }

    #[test]
    fn bad_inputs() {
        assert!(parse("").is_none());
        assert!(parse("STARTFONT 2.1\nFONT x\n").is_none()); // no ENDFONT
                                                             // CHARS count mismatch
        assert!(parse("STARTFONT 2.1\nFONT x\nSIZE 8 75 75\nFONTBOUNDINGBOX 1 1 0 0\nCHARS 2\nSTARTCHAR a\nENCODING 97\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\nENDFONT\n").is_none());
        // bad hex row
        assert!(parse("STARTFONT 2.1\nFONT x\nSIZE 8 75 75\nFONTBOUNDINGBOX 8 1 0 0\nCHARS 1\nSTARTCHAR a\nENCODING 97\nBBX 8 1 0 0\nBITMAP\nZZ\nENDCHAR\nENDFONT\n").is_none());
        // row count short of BBX height
        assert!(parse("STARTFONT 2.1\nFONT x\nSIZE 8 75 75\nFONTBOUNDINGBOX 8 2 0 0\nCHARS 1\nSTARTCHAR a\nENCODING 97\nBBX 8 2 0 0\nBITMAP\nFF\nENDCHAR\nENDFONT\n").is_none());
    }
}
