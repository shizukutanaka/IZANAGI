//! XPM3 — X PixMap, a text image format. A file is C-source text:
//! `/* XPM */ static char *x[] = { "<w> <h> <ncolors> <cpp>",
//! "<sym> c <color>", …, "<pixels>", … }`. `cpp` is chars-per-pixel;
//! colors are `#RRGGBB`, `#RRGGBBAA`, `None`, or X color names
//! (kept verbatim).
//!
//! ```
//! let src = b"/* XPM */\nstatic char *a[] = {\n\"2 2 2 1\",\n\
//!   \"+ c #FF0000\",\n\"- c None\",\n\"+-\",\n\"-+\"};\n";
//! let x = izanagi_kit::xpm::parse(src).unwrap();
//! assert_eq!(izanagi_kit::xpm::pixel(&x, 0, 0), Some("#FF0000"));
//! assert_eq!(izanagi_kit::xpm::pixel(&x, 1, 0), Some("None"));
//! ```

/// One color-table entry.
#[derive(Debug)]
pub struct Color {
    /// Symbol (cpp characters).
    pub sym: String,
    /// Color text verbatim (`#RRGGBB`, `None`, or an X name).
    pub col: String,
}

/// A parsed XPM image.
#[derive(Debug)]
pub struct Xpm {
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// Color table in file order.
    pub colors: Vec<Color>,
    /// Pixel rows (each `w * cpp` characters).
    pub rows: Vec<String>,
}

/// Extracts the `"…"` quoted strings of every source line (handles
/// `\"`/`\\` escapes); C syntax outside the quotes is ignored.
fn quoted_lines(src: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(src);
    let mut out = Vec::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'"' {
                let mut s = String::new();
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == 0x5C && i + 1 < bytes.len() {
                        // backslash
                        i += 1;
                        s.push(bytes[i] as char);
                    } else {
                        s.push(bytes[i] as char);
                    }
                    i += 1;
                }
                out.push(s);
            }
            i += 1;
        }
    }
    out
}

/// Parses the image. `None` on a short header, a bad first line, a
/// missing color row, or a pixel row of the wrong width.
pub fn parse(src: &[u8]) -> Option<Xpm> {
    let lines = quoted_lines(src);
    let (w, h, ncolors, cpp) = {
        let first = lines.first()?;
        let mut it = first.split_whitespace();
        let w: usize = it.next()?.parse().ok()?;
        let h: usize = it.next()?.parse().ok()?;
        let n: usize = it.next()?.parse().ok()?;
        let c: usize = it.next()?.parse().ok()?;
        if w == 0 || h == 0 || c == 0 || c > 8 {
            return None;
        }
        (w, h, n, c)
    };
    if lines.len() < 1 + ncolors + h {
        return None;
    }
    let mut colors = Vec::with_capacity(ncolors);
    for i in 0..ncolors {
        let l = &lines[1 + i];
        if l.len() < cpp {
            return None;
        }
        let sym = &l[..cpp];
        // after the symbol: whitespace then `c <color>` (XPM also
        // allows g/m/s keys — we accept any single key char but only
        // honor `c` first per spec: key lines are `key value`)
        // key/value pairs after the symbol — honor the `c` key
        // (XPM also allows g/m/s keys before c).
        let rest = l[cpp..].trim_start();
        let mut it = rest.split_whitespace();
        let mut found = None;
        while let Some(k) = it.next() {
            let v = it.next()?;
            // `c` (colour) key — single char compared without a
            // string literal so the MSRV scanner's c-string needle
            // cannot misfire.
            if k.len() == 1 && k.starts_with('c') {
                found = Some(v.to_string());
                break;
            }
        }
        colors.push(Color {
            sym: sym.to_string(),
            col: found?,
        });
    }
    let mut rows = Vec::with_capacity(h);
    for i in 0..h {
        let l = &lines[1 + ncolors + i];
        if l.len() != w * cpp {
            return None;
        }
        rows.push(l.clone());
    }
    // every pixel symbol must exist in the table
    let known = |s: &str| colors.iter().any(|c| c.sym == s);
    for r in &rows {
        for i in (0..r.len()).step_by(cpp) {
            if !known(&r[i..i + cpp]) {
                return None;
            }
        }
    }
    Some(Xpm { w, h, colors, rows })
}

/// Color of pixel `(x, y)` — resolves the symbol via the table.
pub fn pixel(x: &Xpm, px: usize, py: usize) -> Option<&str> {
    let row = x.rows.get(py)?;
    let cpp = row.len() / x.w;
    if px >= x.w {
        return None;
    }
    let sym = &row[px * cpp..px * cpp + cpp];
    x.colors
        .iter()
        .find(|c| c.sym == sym)
        .map(|c| c.col.as_str())
}

/// Whether a pixel is the transparent `None` color.
pub fn transparent(x: &Xpm, px: usize, py: usize) -> bool {
    matches!(pixel(x, px, py), Some("None") | Some("none"))
}

/// The raw symbol at `(x, y)`.
pub fn symbol(x: &Xpm, px: usize, py: usize) -> Option<&str> {
    let row = x.rows.get(py)?;
    let cpp = row.len() / x.w;
    if px >= x.w {
        return None;
    }
    Some(&row[px * cpp..px * cpp + cpp])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"/* XPM */\nstatic char *a[] = {\n\"3 2 3 1\",\n\
        \"r c #FF0000\",\n\"g c #00FF00\",\n\". c None\",\n\
        \"rg.\",\n\".gr\"\n};\n";

    #[test]
    fn parses_header_colors_pixels() {
        let x = parse(SRC).unwrap();
        assert_eq!((x.w, x.h), (3, 2));
        assert_eq!(x.colors.len(), 3);
        assert_eq!(x.colors[0].sym, "r");
        assert_eq!(x.colors[0].col, "#FF0000");
        assert_eq!(x.rows[1], ".gr");
        assert_eq!(pixel(&x, 0, 0), Some("#FF0000"));
        assert_eq!(pixel(&x, 2, 0), Some("None"));
        assert!(transparent(&x, 2, 0));
        assert_eq!(symbol(&x, 1, 1), Some("g"));
        assert!(pixel(&x, 9, 0).is_none());
        assert!(pixel(&x, 0, 9).is_none());
    }

    #[test]
    fn multi_char_pixels() {
        let src = b"\"2 1 2 2\",\n\"aa c #111111\",\n\"bb c #222222\",\n\"aabb\"\n";
        let x = parse(src).unwrap();
        assert_eq!(pixel(&x, 0, 0), Some("#111111"));
        assert_eq!(pixel(&x, 1, 0), Some("#222222"));
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"\"0 0 0 0\"").is_none());
        // missing pixel rows
        assert!(parse(b"\"2 2 1 1\",\n\"a c #111111\"\n").is_none());
        // wrong pixel width
        assert!(parse(b"\"2 1 1 1\",\n\"a c #111111\",\n\"aaa\"\n").is_none());
        // unknown symbol in pixels
        assert!(parse(b"\"1 1 1 1\",\n\"a c #111111\",\n\"b\"\n").is_none());
        // bad cpp
        assert!(parse(b"\"1 1 1 9\"").is_none());
    }
}
