//! BDF — Adobe Bitmap Distribution Format (Glyph Bitmap Distribution
//! Format spec). ASCII text: a `STARTFONT` header, font-wide metrics,
//! optional `STARTPROPERTIES` block, then `CHARS n` glyphs each with
//! `STARTCHAR`/`ENCODING`/`SWIDTH`/`DWIDTH`/`BBX`/`BITMAP`/`ENDCHAR`.
//!
//! Bitmap rows are hex digits; each row holds `bbx.w` bits starting
//! at the most significant bit of the first byte.
//!
//! ```
//! let src = b"STARTFONT 2.1\nFONT mini\nSIZE 8 75 75\n\
//!   FONTBOUNDINGBOX 8 8 0 -1\nSTARTPROPERTIES 0\nCHARS 1\n\
//!   STARTCHAR A\nENCODING 65\nSWIDTH 500 0\nDWIDTH 8 0\nBBX 8 8 0 -1\n\
//!   BITMAP\n18\n24\n42\n42\n7E\n42\n42\n00\nENDCHAR\nENDFONT\n";
//! let b = izanagi_kit::bdf::parse(src).unwrap();
//! let g = izanagi_kit::bdf::glyph(&b, 65).unwrap();
//! assert_eq!(izanagi_kit::bdf::row(g, 0), Some(0x18));
//! assert_eq!(izanagi_kit::bdf::row(g, 4), Some(0x7E));
//! assert!(izanagi_kit::bdf::bit(g, 0, 3));
//! ```

/// Bounding box: width, height, x-offset, y-offset (device pixels).
#[derive(Debug, Clone, Copy)]
pub struct Bbx {
    /// Glyph pixel width.
    pub w: i32,
    /// Glyph pixel height.
    pub h: i32,
    /// Horizontal offset of the leftmost pixel from the origin.
    pub x: i32,
    /// Vertical offset of the bottom pixel from the baseline.
    pub y: i32,
}

/// One glyph.
#[derive(Debug)]
pub struct Glyph {
    /// STARTCHAR name.
    pub name: String,
    /// ENCODING code point (-1 when unencoded).
    pub enc: i32,
    /// DWIDTH — advance in pixels.
    pub dwidth: i32,
    /// Bounding box.
    pub bbx: Bbx,
    /// Bitmap rows, each row's low `bbx.w` meaningful bits packed
    /// MSB-first as read from hex (row byte count = ceil(w/8)).
    pub bits: Vec<u8>,
}

/// A parsed BDF font.
#[derive(Debug)]
pub struct Bdf {
    /// FONT line contents.
    pub name: String,
    /// SIZE fields: `(point, xres, yres)`.
    pub size: (i32, i32, i32),
    /// Font-wide bounding box.
    pub bbox: Bbx,
    /// STARTPROPERTIES key/value pairs (verbatim).
    pub props: Vec<(String, String)>,
    /// Glyphs in file order.
    pub glyphs: Vec<Glyph>,
}

fn ints<const N: usize>(s: &str) -> Option<[i32; N]> {
    let mut out = [0i32; N];
    let mut it = s.split_whitespace();
    for o in out.iter_mut() {
        *o = it.next()?.trim().parse().ok()?;
    }
    Some(out)
}

fn hex_row(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.len() != 2 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u8::from_str_radix(s, 16).ok()
}

/// Parses the file. `None` on a missing `STARTFONT`/`FONT`/`CHARS`
/// line, a glyph block that never ends, or a malformed hex row.
pub fn parse(src: &[u8]) -> Option<Bdf> {
    let text = core::str::from_utf8(src).ok()?;
    let mut lines = text.lines().map(str::trim);

    let mut name = String::new();
    let mut size = (0, 0, 0);
    let mut bbox = Bbx {
        w: 0,
        h: 0,
        x: 0,
        y: 0,
    };
    let mut props = Vec::new();
    let mut glyphs = Vec::new();

    // Header until CHARS.
    let first = lines.next()?;
    if !first.starts_with("STARTFONT") {
        return None;
    }
    let mut glyph_count: Option<usize> = None;
    while let Some(l) = lines.next() {
        if let Some(r) = l.strip_prefix("FONT ") {
            name = r.trim().to_string();
        } else if let Some(r) = l.strip_prefix("SIZE ") {
            let v = ints::<3>(r)?;
            size = (v[0], v[1], v[2]);
        } else if let Some(r) = l.strip_prefix("FONTBOUNDINGBOX ") {
            let v = ints::<4>(r)?;
            bbox = Bbx {
                w: v[0],
                h: v[1],
                x: v[2],
                y: v[3],
            };
        } else if let Some(r) = l.strip_prefix("STARTPROPERTIES") {
            let n: usize = r.trim().parse().ok()?;
            for _ in 0..n {
                let pl = lines.next()?;
                if let Some(r) = pl.strip_prefix("COMMENT") {
                    props.push(("COMMENT".to_string(), r.trim().to_string()));
                } else {
                    let (k, v) = pl.split_once(' ').unwrap_or((pl, ""));
                    props.push((k.to_string(), v.trim().to_string()));
                }
            }
        } else if l.starts_with("ENDPROPERTIES") {
            // tolerant — spec requires it after the props
        } else if let Some(r) = l.strip_prefix("CHARS ") {
            glyph_count = Some(r.trim().parse().ok()?);
            break;
        }
    }
    let count = glyph_count?;

    // Glyphs.
    for _ in 0..count {
        let mut g = Glyph {
            name: String::new(),
            enc: -1,
            dwidth: 0,
            bbx: Bbx {
                w: 0,
                h: 0,
                x: 0,
                y: 0,
            },
            bits: Vec::new(),
        };
        let mut in_bitmap = false;
        let mut closed = false;
        let mut rows_wanted = 0i64;
        let mut bytes_per_row = 0usize;
        for l in lines.by_ref() {
            if in_bitmap {
                if l == "ENDCHAR" {
                    if g.bits.len() as i64 != rows_wanted * bytes_per_row as i64 {
                        return None;
                    }
                    closed = true;
                    break;
                }
                // one hex string per row, ceil(w/8) bytes
                let mut i = 0;
                let s = l;
                while i + 2 <= s.len() {
                    g.bits.push(hex_row(&s[i..i + 2])?);
                    i += 2;
                }
                if s.len() % 2 != 0 {
                    return None;
                }
                continue;
            }
            if let Some(r) = l.strip_prefix("STARTCHAR ") {
                g.name = r.trim().to_string();
            } else if let Some(r) = l.strip_prefix("ENCODING ") {
                g.enc = r.split_whitespace().next()?.parse().ok()?;
            } else if let Some(r) = l.strip_prefix("DWIDTH ") {
                let v = ints::<2>(r)?;
                g.dwidth = v[0];
            } else if l.starts_with("SWIDTH") {
                // scalable width — parsed but not needed for bitmaps
            } else if let Some(r) = l.strip_prefix("BBX ") {
                let v = ints::<4>(r)?;
                g.bbx = Bbx {
                    w: v[0],
                    h: v[1],
                    x: v[2],
                    y: v[3],
                };
            } else if l == "BITMAP" {
                in_bitmap = true;
                rows_wanted = g.bbx.h as i64;
                bytes_per_row = (g.bbx.w as usize).div_ceil(8);
            }
        }
        if !closed {
            return None; // glyph block never closed
        }
        glyphs.push(g);
    }

    Some(Bdf {
        name,
        size,
        bbox,
        props,
        glyphs,
    })
}

/// The glyph for code point `enc`.
pub fn glyph(b: &Bdf, enc: i32) -> Option<&Glyph> {
    b.glyphs.iter().find(|g| g.enc == enc)
}

/// Row `r`'s packed bits (row index 0 is the TOP row of the glyph —
/// BITMAP rows are stored top-down). `None` out of range.
pub fn row(g: &Glyph, r: usize) -> Option<u8> {
    let bpr = (g.bbx.w as usize).div_ceil(8);
    g.bits.get(r.checked_mul(bpr)?).copied()
}

/// Pixel at `(x, r)`: bit `7-x` of the row's first byte when w ≤ 8.
/// `false` out of range. (For wide glyphs use [`row_at`].)
pub fn bit(g: &Glyph, r: usize, x: usize) -> bool {
    row_at(g, r, x).unwrap_or_default()
}

/// Pixel at row `r`, column `x` — MSB-first inside each row byte.
pub fn row_at(g: &Glyph, r: usize, x: usize) -> Option<bool> {
    if x >= g.bbx.w as usize || r >= g.bbx.h as usize {
        return None;
    }
    let bpr = (g.bbx.w as usize).div_ceil(8);
    let byte = *g.bits.get(r.checked_mul(bpr)? + x / 8)?;
    Some(byte & (0x80 >> (x % 8)) != 0)
}

/// Renders `g` as `#`/`.` lines (bbx.h rows × bbx.w cols).
pub fn render(g: &Glyph) -> Vec<String> {
    let mut out = Vec::with_capacity(g.bbx.h as usize);
    for r in 0..g.bbx.h as usize {
        let mut s = String::with_capacity(g.bbx.w as usize);
        for x in 0..g.bbx.w as usize {
            s.push(if bit(g, r, x) { '#' } else { '.' });
        }
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"STARTFONT 2.1\nFONT mini\nSIZE 8 75 75\n\
        FONTBOUNDINGBOX 8 8 0 -1\nSTARTPROPERTIES 2\n\
        FOUNDRY \"test\"\nWEIGHT 400\nENDPROPERTIES\nCHARS 1\n\
        STARTCHAR A\nENCODING 65\nSWIDTH 500 0\nDWIDTH 8 0\nBBX 8 8 0 -1\n\
        BITMAP\n18\n24\n42\n42\n7E\n42\n42\n00\nENDCHAR\nENDFONT\n";

    #[test]
    fn parses_font_and_glyph() {
        let b = parse(SRC).unwrap();
        assert_eq!(b.name, "mini");
        assert_eq!(b.size, (8, 75, 75));
        assert_eq!(b.props.len(), 2);
        assert_eq!(b.props[0].0, "FOUNDRY");
        assert_eq!(b.glyphs.len(), 1);
        let g = glyph(&b, 65).unwrap();
        assert_eq!(g.enc, 65);
        assert_eq!(g.dwidth, 8);
        assert!(glyph(&b, 66).is_none());
    }

    #[test]
    fn rows_bits_render() {
        let b = parse(SRC).unwrap();
        let g = glyph(&b, 65).unwrap();
        assert_eq!(row(g, 0), Some(0x18));
        assert_eq!(row(g, 4), Some(0x7E));
        assert_eq!(row(g, 9), None);
        assert!(bit(g, 0, 3));
        assert!(bit(g, 0, 4));
        assert!(!bit(g, 0, 0));
        assert!(!row_at(g, 4, 0).unwrap()); // 0x7E MSB clear
        assert!(row_at(g, 4, 1).unwrap());
        let r = render(g);
        assert_eq!(r[0], "...##...");
        assert_eq!(r[4], ".######.");
        assert_eq!(r.len(), 8);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"NOTFONT\n").is_none());
        // unterminated glyph
        assert!(parse(b"STARTFONT 2.1\nCHARS 1\nSTARTCHAR X\n").is_none());
        // bad hex row
        assert!(parse(
            b"STARTFONT 2.1\nFONT x\nCHARS 1\nSTARTCHAR X\n\
            ENCODING 1\nBBX 8 1 0 0\nBITMAP\nZZ\nENDCHAR\n"
        )
        .is_none());
        // row count mismatch (declared h=2, only 1 row)
        assert!(parse(
            b"STARTFONT 2.1\nFONT x\nCHARS 1\nSTARTCHAR X\n\
            ENCODING 1\nBBX 8 2 0 0\nBITMAP\nFF\nENDCHAR\n"
        )
        .is_none());
    }
}
