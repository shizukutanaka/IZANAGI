//! Well-Known Text — OGC geometry serialization for [`geo`](crate::geo).
//!
//! Parses `POINT`, `LINESTRING`, `POLYGON`, `MULTIPOINT`,
//! `MULTILINESTRING`, `MULTIPOLYGON` (all optionally `EMPTY`), and
//! `GEOMETRYCOLLECTION`. 2-D only: `Z`/`M`/`ZM` ordinate tags are
//! rejected. Coordinates are [`Fixed`] Q16.16 — parsed by decimal-digit
//! arithmetic (exact, no float); magnitudes beyond ±32767 are rejected.
//! [`write()`] emits canonical `TAG(x y, ...)` text, and
//! `parse∘write` is the identity.
//!
//! ```
//! use izanagi_kit::wkt::{Geo, parse, write};
//! let g = parse("POINT(30 10)").unwrap();
//! assert_eq!(write(&g), "POINT(30 10)");
//! ```

use crate::fixed::Fixed;

/// A point — `(x, y)` as `Fixed`.
pub type P = (Fixed, Fixed);

/// A parsed WKT geometry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Geo {
    /// `POINT(x y)`
    Point(P),
    /// `LINESTRING(x y, x y, ...)`
    LineString(Vec<P>),
    /// `POLYGON((ring...), (hole...), ...)` — first ring is the shell.
    Polygon(Vec<Vec<P>>),
    /// `MULTIPOINT(...)` — both `(1 2, 3 4)` and `((1 2), (3 4))` forms.
    MultiPoint(Vec<P>),
    /// `MULTILINESTRING((...), (...))`
    MultiLineString(Vec<Vec<P>>),
    /// `MULTIPOLYGON(((...)), ((...)))`
    MultiPolygon(Vec<Vec<Vec<P>>>),
    /// `GEOMETRYCOLLECTION(g, g, ...)`
    Collection(Vec<Geo>),
    /// Any `TAG EMPTY`.
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok {
    Word(usize, usize), // offset,len into input (upper-cased elsewhere)
    Num(i64),           // Fixed raw
    LParen,
    RParen,
    Comma,
}

fn lex(s: &str) -> Option<Vec<Tok>> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b'A'..=b'Z' | b'a'..=b'z' => {
                let st = i;
                while i < b.len() && b[i].is_ascii_alphabetic() {
                    i += 1;
                }
                out.push(Tok::Word(st, i - st));
            }
            b'0'..=b'9' | b'+' | b'-' | b'.' => {
                let st = i;
                if matches!(b[i], b'+' | b'-') {
                    i += 1;
                }
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                out.push(Tok::Num(parse_fixed(&s[st..i])?));
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Decimal string → `Fixed` raw, by digit arithmetic (exact).
fn parse_fixed(s: &str) -> Option<i64> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (ip, fp) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if ip.is_empty() && fp.is_empty()
        || !ip.bytes().all(|b| b.is_ascii_digit())
        || !fp.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let ip: i64 = if ip.is_empty() { 0 } else { ip.parse().ok()? };
    let mut frac: i64 = 0;
    let mut scale: i64 = 1;
    for b in fp.bytes() {
        if scale > 1_000_000 {
            break; // beyond Fixed resolution — truncate, deterministically
        }
        frac = frac * 10 + i64::from(b - b'0');
        scale *= 10;
    }
    let raw = ip.checked_mul(65536)?.checked_add(frac * 65536 / scale)?;
    let raw = if neg { -raw } else { raw };
    if raw.abs() > i64::from(i32::MAX) {
        return None;
    }
    Some(raw)
}

struct P1<'a> {
    t: &'a [Tok],
    s: &'a str,
    i: usize,
}

impl P1<'_> {
    fn next(&mut self) -> Option<Tok> {
        let t = *self.t.get(self.i)?;
        self.i += 1;
        Some(t)
    }
    fn peek(&self) -> Option<Tok> {
        self.t.get(self.i).copied()
    }
    fn word(&mut self) -> Option<String> {
        match self.next()? {
            Tok::Word(st, n) => Some(self.s[st..st + n].to_uppercase()),
            _ => None,
        }
    }
    fn expect(&mut self, t: Tok) -> Option<()> {
        if self.next()? == t {
            Some(())
        } else {
            None
        }
    }
    fn point(&mut self) -> Option<P> {
        let x = match self.next()? {
            Tok::Num(v) => v,
            _ => return None,
        };
        let y = match self.next()? {
            Tok::Num(v) => v,
            _ => return None,
        };
        Some((Fixed::from_raw(x as i32), Fixed::from_raw(y as i32)))
    }
    fn point_list(&mut self) -> Option<Vec<P>> {
        // '(' x y (',' x y)* ')'
        self.expect(Tok::LParen)?;
        let mut v = vec![self.point()?];
        while self.peek() == Some(Tok::Comma) {
            self.i += 1;
            v.push(self.point()?);
        }
        self.expect(Tok::RParen)?;
        Some(v)
    }
    fn ring_list(&mut self) -> Option<Vec<Vec<P>>> {
        self.expect(Tok::LParen)?;
        let mut v = vec![self.point_list()?];
        while self.peek() == Some(Tok::Comma) {
            self.i += 1;
            v.push(self.point_list()?);
        }
        self.expect(Tok::RParen)?;
        Some(v)
    }
    fn poly_list(&mut self) -> Option<Vec<Vec<Vec<P>>>> {
        self.expect(Tok::LParen)?;
        let mut v = vec![self.ring_list()?];
        while self.peek() == Some(Tok::Comma) {
            self.i += 1;
            v.push(self.ring_list()?);
        }
        self.expect(Tok::RParen)?;
        Some(v)
    }
    fn geometry(&mut self, depth: usize) -> Option<Geo> {
        if depth > 64 {
            return None;
        }
        let tag = self.word()?;
        if tag.ends_with('M') || tag.ends_with('Z') {
            return None; // 2-D only
        }
        if tag == "EMPTY" {
            // tolerate bare EMPTY with no tag
            return Some(Geo::Empty);
        }
        // Optional "EMPTY" after the tag.
        if self.peek().is_some() && matches!(self.peek(), Some(Tok::Word(_, _))) && {
            let save = self.i;
            match self.word().as_deref() {
                Some("EMPTY") => true,
                _ => {
                    self.i = save;
                    false
                }
            }
        } {
            return Some(Geo::Empty);
        }
        match tag.as_str() {
            "POINT" => {
                self.expect(Tok::LParen)?;
                let p = self.point()?;
                self.expect(Tok::RParen)?;
                Some(Geo::Point(p))
            }
            "LINESTRING" => Some(Geo::LineString(self.point_list()?)),
            "POLYGON" => Some(Geo::Polygon(self.ring_list()?)),
            "MULTIPOINT" => {
                self.expect(Tok::LParen)?;
                let mut v = Vec::new();
                loop {
                    if self.peek() == Some(Tok::LParen) {
                        self.i += 1;
                        v.push(self.point()?);
                        self.expect(Tok::RParen)?;
                    } else {
                        v.push(self.point()?);
                    }
                    if self.peek() == Some(Tok::Comma) {
                        self.i += 1;
                    } else {
                        break;
                    }
                }
                self.expect(Tok::RParen)?;
                Some(Geo::MultiPoint(v))
            }
            "MULTILINESTRING" => Some(Geo::MultiLineString(self.ring_list()?)),
            "MULTIPOLYGON" => Some(Geo::MultiPolygon(self.poly_list()?)),
            "GEOMETRYCOLLECTION" => {
                self.expect(Tok::LParen)?;
                let mut v = vec![self.geometry(depth + 1)?];
                while self.peek() == Some(Tok::Comma) {
                    self.i += 1;
                    v.push(self.geometry(depth + 1)?);
                }
                self.expect(Tok::RParen)?;
                Some(Geo::Collection(v))
            }
            _ => None,
        }
    }
}

/// Parse WKT; `None` on malformed input, trailing junk, or non-2D tags.
pub fn parse(s: &str) -> Option<Geo> {
    let t = lex(s)?;
    let mut p = P1 { t: &t, s, i: 0 };
    let g = p.geometry(0)?;
    if p.i != t.len() {
        return None;
    }
    Some(g)
}

fn num(raw: i64) -> String {
    let neg = raw < 0;
    let a = raw.unsigned_abs();
    let ip = a / 65536;
    let mut fp = a % 65536;
    if fp == 0 {
        return format!("{}{}", if neg { "-" } else { "" }, ip);
    }
    // Emit shortest exact decimal: fp/65536 has a finite expansion
    // (denominator is a power of two).
    let mut digits = String::new();
    while fp != 0 {
        fp *= 10;
        digits.push(char::from_digit((fp / 65536) as u32, 10).unwrap_or('0'));
        fp %= 65536;
    }
    format!("{}{}.{}", if neg { "-" } else { "" }, ip, digits)
}

fn emit_pts(v: &[P], out: &mut String) {
    for (i, (x, y)) in v.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&num(x.raw() as i64));
        out.push(' ');
        out.push_str(&num(y.raw() as i64));
    }
}

fn emit_rings(v: &[Vec<P>], out: &mut String) {
    for (i, r) in v.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('(');
        emit_pts(r, out);
        out.push(')');
    }
}

/// Canonical serialization: `TAG(body)` with minimal decimals.
pub fn write(g: &Geo) -> String {
    let mut out = String::new();
    emit_geo(g, &mut out);
    out
}

fn emit_geo(g: &Geo, out: &mut String) {
    match g {
        Geo::Empty => out.push_str("EMPTY"),
        Geo::Point((x, y)) => {
            out.push_str("POINT(");
            out.push_str(&num(x.raw() as i64));
            out.push(' ');
            out.push_str(&num(y.raw() as i64));
            out.push(')');
        }
        Geo::LineString(v) => {
            out.push_str("LINESTRING(");
            emit_pts(v, out);
            out.push(')');
        }
        Geo::Polygon(v) => {
            out.push_str("POLYGON(");
            emit_rings(v, out);
            out.push(')');
        }
        Geo::MultiPoint(v) => {
            out.push_str("MULTIPOINT(");
            emit_pts(v, out);
            out.push(')');
        }
        Geo::MultiLineString(v) => {
            out.push_str("MULTILINESTRING(");
            emit_rings(v, out);
            out.push(')');
        }
        Geo::MultiPolygon(v) => {
            out.push_str("MULTIPOLYGON(");
            for (i, r) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('(');
                emit_rings(r, out);
                out.push(')');
            }
            out.push(')');
        }
        Geo::Collection(v) => {
            out.push_str("GEOMETRYCOLLECTION(");
            for (i, c) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                emit_geo(c, out);
            }
            out.push(')');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ogc_examples() {
        assert_eq!(
            parse("POINT (30 10)"),
            Some(Geo::Point((Fixed::from_int(30), Fixed::from_int(10))))
        );
        assert!(matches!(parse("LINESTRING (30 10, 10 30, 40 40)"),
            Some(Geo::LineString(v)) if v.len() == 3));
        assert!(
            matches!(parse("POLYGON ((30 10, 40 40, 20 40, 10 20, 30 10))"),
            Some(Geo::Polygon(v)) if v.len() == 1 && v[0].len() == 5)
        );
        assert!(
            matches!(parse("POLYGON ((35 10, 45 45, 15 40, 10 20, 35 10), (20 30, 35 35, 30 20, 20 30))"),
            Some(Geo::Polygon(v)) if v.len() == 2)
        );
        assert!(matches!(parse("MULTIPOINT (10 40, 40 30, 20 20, 30 10)"),
            Some(Geo::MultiPoint(v)) if v.len() == 4));
        assert!(
            matches!(parse("MULTIPOINT ((10 40), (40 30), (20 20), (30 10))"),
            Some(Geo::MultiPoint(v)) if v.len() == 4)
        );
        assert!(
            matches!(parse("MULTILINESTRING ((10 10, 20 20, 10 40), (40 40, 30 30, 40 20, 30 10))"),
            Some(Geo::MultiLineString(v)) if v.len() == 2)
        );
        assert!(
            matches!(parse("MULTIPOLYGON (((30 20, 45 40, 10 40, 30 20)), ((15 5, 40 10, 10 20, 5 10, 15 5)))"),
            Some(Geo::MultiPolygon(v)) if v.len() == 2)
        );
        assert!(
            matches!(parse("GEOMETRYCOLLECTION (POINT (40 10), LINESTRING (10 10, 20 20))"),
            Some(Geo::Collection(v)) if v.len() == 2)
        );
        assert_eq!(parse("POINT EMPTY"), Some(Geo::Empty));
    }

    #[test]
    fn write_parse_roundtrip() {
        for s in [
            "POINT(-30.5 10.25)",
            "LINESTRING(30 10,10 30)",
            "POLYGON((30 10,40 40,20 40,30 10))",
            "MULTIPOINT(10 40,40 30)",
            "MULTILINESTRING((10 10,20 20))",
            "MULTIPOLYGON(((30 20,45 40,30 20)))",
            "GEOMETRYCOLLECTION(POINT(40 10),LINESTRING(1 2,3 4))",
        ] {
            let g = parse(s).unwrap();
            assert_eq!(parse(&write(&g)), Some(g), "{s}");
        }
        assert_eq!(
            write(&parse("POINT(1.5 -2.25)").unwrap()),
            "POINT(1.5 -2.25)"
        );
    }

    #[test]
    fn malformed_and_3d_rejected() {
        for bad in [
            "",
            "POINT",
            "POINT()",
            "POINT(30)",
            "POINT(30 10",
            "POINT(30, 10)",
            "LINESTRING(30 10,)",
            "POLYGON(30 10)",
            "POINT Z(1 2 3)",
            "POINT M(1 2 3)",
            "SRID=4326;POINT(1 2)",
            "TRIANGLE((0 0,1 1,0 0))",
            "POINT(1e2 3)",
            "POINT(40000 0)",
            "POINT(1 2)junk",
        ] {
            assert!(parse(bad).is_none(), "{bad}");
        }
    }
}
