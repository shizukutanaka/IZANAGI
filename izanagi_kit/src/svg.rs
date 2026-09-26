//! SVG path data (SVG 1.1 §8) — the `d` attribute minilanguage:
//! `M/L/H/V/C/S/Q/T/A/Z` commands with absolute/relative forms and
//! implicit coordinate repetition (`M x y x2 y2 …` repeats as `L`).
//! All coordinates are `Fixed` (Q16.16) — decimals are parsed
//! exactly (truncate-to-zero past 65536ths).
//!
//! ```
//! let p = izanagi_kit::svg::parse(b"M10 10L20 20l-5 0z").unwrap();
//! assert_eq!(p.len(), 4); // M + L + l + Z
//! let (min, max) = izanagi_kit::svg::bbox(&p).unwrap();
//! assert_eq!(min.x, izanagi_kit::Fixed::from_int(10));
//! assert_eq!(max.y, izanagi_kit::Fixed::from_int(20));
//! ```

use crate::fixed::Fixed;

/// One path segment (all coordinates in `Fixed`, absolute).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seg {
    /// Move-to.
    M {
        /// x
        x: Fixed,
        /// y
        y: Fixed,
    },
    /// Line-to (H/V are folded into L with the unchanged axis).
    L {
        /// x
        x: Fixed,
        /// y
        y: Fixed,
    },
    /// Cubic Bézier.
    C {
        /// control point 1
        c1x: Fixed,
        /// control point 1
        c1y: Fixed,
        /// control point 2
        c2x: Fixed,
        /// control point 2
        c2y: Fixed,
        /// end
        x: Fixed,
        /// end
        y: Fixed,
    },
    /// Smooth cubic (the reflected c1 is precomputed into `c1`).
    S {
        /// first control x (reflection of the previous segment's c2)
        c1x: Fixed,
        /// first control y
        c1y: Fixed,
        /// second control
        c2x: Fixed,
        /// second control y
        c2y: Fixed,
        /// end
        x: Fixed,
        /// end
        y: Fixed,
    },
    /// Quadratic Bézier.
    Q {
        /// control
        cx: Fixed,
        /// control
        cy: Fixed,
        /// end
        x: Fixed,
        /// end
        y: Fixed,
    },
    /// (T is folded into `Q` with the reflected control point
    /// resolved.)
    /// Elliptical arc: rx ry x-rot large-arc sweep x y.
    A {
        /// radius x
        rx: Fixed,
        /// radius y
        ry: Fixed,
        /// x-axis rotation (degrees, Fixed)
        rot: Fixed,
        /// large-arc flag
        large: bool,
        /// sweep flag
        sweep: bool,
        /// end
        x: Fixed,
        /// end
        y: Fixed,
    },
    /// Close path.
    Z,
}

/// Parses a decimal into `Fixed` raw by digit arithmetic —
/// `["+3.14", "-0.5", "1e-2"]` all work; out-of-range → `None`.
fn num(d: &[u8], i: &mut usize) -> Option<i64> {
    let mut j = *i;
    let neg = match d.get(j) {
        Some(b'-') => {
            j += 1;
            true
        }
        Some(b'+') => {
            j += 1;
            false
        }
        _ => false,
    };
    let mut mant: i128 = 0;
    let mut exp: i32 = 0;
    let mut seen = false;
    let mut dot = false;
    while let Some(&b) = d.get(j) {
        if b == b'.' {
            if dot {
                return None;
            }
            dot = true;
            j += 1;
            continue;
        }
        if b.is_ascii_digit() {
            mant = mant.checked_mul(10)?.checked_add((b - b'0') as i128)?;
            if dot {
                exp -= 1;
            }
            seen = true;
            j += 1;
            continue;
        }
        break;
    }
    if !seen {
        return None;
    }
    // optional exponent
    if matches!(d.get(j), Some(b'e') | Some(b'E')) {
        let mut k = j + 1;
        let eneg = matches!(d.get(k), Some(b'-')) && {
            k += 1;
            true
        } || matches!(d.get(k), Some(b'+')) && {
            k += 1;
            false
        };
        let mut e = 0i32;
        let mut seen_e = false;
        while let Some(&b) = d.get(k) {
            if !b.is_ascii_digit() {
                break;
            }
            e = e.saturating_mul(10).saturating_add((b - b'0') as i32);
            seen_e = true;
            k += 1;
        }
        if seen_e {
            exp = exp.saturating_add(if eneg { -e } else { e });
            j = k;
        }
    }
    *i = j;
    // raw Q16.16 = mant * 10^exp * 65536 (truncate toward zero)
    let mut num = mant * 65536;
    if exp >= 0 {
        for _ in 0..exp {
            num = num.checked_mul(10)?;
        }
    } else {
        for _ in 0..-exp {
            num /= 10;
        }
    }
    let v = if neg { -num } else { num };
    if v > i64::MAX as i128 || v < i64::MIN as i128 {
        return None;
    }
    Some(v as i64)
}

#[inline]
fn fx(v: i64) -> Option<Fixed> {
    i32::try_from(v).ok().map(Fixed::from_raw)
}

fn skip_sep(d: &[u8], i: &mut usize) {
    while let Some(&b) = d.get(*i) {
        if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == b',' {
            *i += 1;
        } else {
            break;
        }
    }
}

/// Parses the `d` attribute into absolute-coordinate segments.
/// `None` on unknown commands or malformed numbers.
pub fn parse(d: &[u8]) -> Option<Vec<Seg>> {
    let mut out = Vec::new();
    let mut i = 0;
    let (mut cx, mut cy) = (0i64, 0i64); // current point, raw q16
    let (mut sx, mut sy) = (0i64, 0i64); // subpath start
    let mut last_c2: Option<(i64, i64)> = None; // last cubic ctrl2
    let mut last_q: Option<(i64, i64)> = None; // last quad ctrl
    let mut cmd: Option<u8> = None;
    let mut args_needed = false;

    while i < d.len() {
        skip_sep(d, &mut i);
        if i >= d.len() {
            break;
        }
        let b = d[i];
        if b.is_ascii_alphabetic() {
            cmd = Some(b);
            i += 1;
            if matches!(b, b'z' | b'Z') {
                // Z takes no arguments — emit immediately
                out.push(Seg::Z);
                cx = sx;
                cy = sy;
                last_c2 = None;
                last_q = None;
                cmd = None;
                args_needed = false;
            } else {
                args_needed = true;
            }
            continue;
        }
        if i >= d.len() {
            break;
        }
        let c = cmd?;
        let rel = c.is_ascii_lowercase();
        let up = c.to_ascii_uppercase();
        let read = |i: &mut usize| num(d, i);

        match up {
            b'M' => {
                let x = read(&mut i)?;
                skip_sep(d, &mut i);
                let y = read(&mut i)?;
                let (x, y) = if rel { (x + cx, y + cy) } else { (x, y) };
                cx = x;
                cy = y;
                sx = x;
                sy = y;
                out.push(Seg::M {
                    x: fx(x)?,
                    y: fx(y)?,
                });
                cmd = Some(if rel { b'l' } else { b'L' }); // implicit lineto
                last_c2 = None;
                last_q = None;
            }
            b'L' => {
                let x = read(&mut i)?;
                skip_sep(d, &mut i);
                let y = read(&mut i)?;
                let (x, y) = if rel { (x + cx, y + cy) } else { (x, y) };
                out.push(Seg::L {
                    x: fx(x)?,
                    y: fx(y)?,
                });
                cx = x;
                cy = y;
                last_c2 = None;
                last_q = None;
            }
            b'H' => {
                let x = read(&mut i)?;
                let x = if rel { x + cx } else { x };
                out.push(Seg::L {
                    x: fx(x)?,
                    y: fx(cy)?,
                });
                cx = x;
                last_c2 = None;
                last_q = None;
            }
            b'V' => {
                let y = read(&mut i)?;
                let y = if rel { y + cy } else { y };
                out.push(Seg::L {
                    x: fx(cx)?,
                    y: fx(y)?,
                });
                cy = y;
                last_c2 = None;
                last_q = None;
            }

            b'C' => {
                let x1 = read(&mut i)?;
                skip_sep(d, &mut i);
                let y1 = read(&mut i)?;
                skip_sep(d, &mut i);
                let x2 = read(&mut i)?;
                skip_sep(d, &mut i);
                let y2 = read(&mut i)?;
                skip_sep(d, &mut i);
                let x = read(&mut i)?;
                skip_sep(d, &mut i);
                let y = read(&mut i)?;
                let (x1, y1, x2, y2, x, y) = if rel {
                    (x1 + cx, y1 + cy, x2 + cx, y2 + cy, x + cx, y + cy)
                } else {
                    (x1, y1, x2, y2, x, y)
                };
                out.push(Seg::C {
                    c1x: fx(x1)?,
                    c1y: fx(y1)?,
                    c2x: fx(x2)?,
                    c2y: fx(y2)?,
                    x: fx(x)?,
                    y: fx(y)?,
                });
                cx = x;
                cy = y;
                last_c2 = Some((x2, y2));
                last_q = None;
            }
            b'S' => {
                let x2 = read(&mut i)?;
                skip_sep(d, &mut i);
                let y2 = read(&mut i)?;
                skip_sep(d, &mut i);
                let x = read(&mut i)?;
                skip_sep(d, &mut i);
                let y = read(&mut i)?;
                let (x2, y2, x, y) = if rel {
                    (x2 + cx, y2 + cy, x + cx, y + cy)
                } else {
                    (x2, y2, x, y)
                };
                let (ax, ay) = last_c2
                    .map(|(px, py)| (2 * cx - px, 2 * cy - py))
                    .unwrap_or((cx, cy));
                out.push(Seg::S {
                    c1x: fx(ax)?,
                    c1y: fx(ay)?,
                    c2x: fx(x2)?,
                    c2y: fx(y2)?,
                    x: fx(x)?,
                    y: fx(y)?,
                });
                cx = x;
                cy = y;
                last_c2 = Some((x2, y2));
                last_q = None;
            }
            b'Q' => {
                let x1 = read(&mut i)?;
                skip_sep(d, &mut i);
                let y1 = read(&mut i)?;
                skip_sep(d, &mut i);
                let x = read(&mut i)?;
                skip_sep(d, &mut i);
                let y = read(&mut i)?;
                let (x1, y1, x, y) = if rel {
                    (x1 + cx, y1 + cy, x + cx, y + cy)
                } else {
                    (x1, y1, x, y)
                };
                out.push(Seg::Q {
                    cx: fx(x1)?,
                    cy: fx(y1)?,
                    x: fx(x)?,
                    y: fx(y)?,
                });
                cx = x;
                cy = y;
                last_q = Some((x1, y1));
                last_c2 = None;
            }
            b'T' => {
                let x = read(&mut i)?;
                skip_sep(d, &mut i);
                let y = read(&mut i)?;
                let (x, y) = if rel { (x + cx, y + cy) } else { (x, y) };
                // reflect the previous quad control
                let (qx, qy) = last_q
                    .map(|(px, py)| (2 * cx - px, 2 * cy - py))
                    .unwrap_or((cx, cy));
                out.push(Seg::Q {
                    cx: fx(qx)?,
                    cy: fx(qy)?,
                    x: fx(x)?,
                    y: fx(y)?,
                });
                cx = x;
                cy = y;
                last_q = Some((qx, qy));
                last_c2 = None;
            }
            b'A' => {
                let rx = read(&mut i)?;
                skip_sep(d, &mut i);
                let ry = read(&mut i)?;
                skip_sep(d, &mut i);
                let rot = read(&mut i)?;
                skip_sep(d, &mut i);
                let large = flag(d, &mut i)?;
                let sweep = flag(d, &mut i)?;
                skip_sep(d, &mut i);
                let x = read(&mut i)?;
                skip_sep(d, &mut i);
                let y = read(&mut i)?;
                let (x, y) = if rel { (x + cx, y + cy) } else { (x, y) };
                out.push(Seg::A {
                    rx: fx(rx)?,
                    ry: fx(ry)?,
                    rot: fx(rot)?,
                    large,
                    sweep,
                    x: fx(x)?,
                    y: fx(y)?,
                });
                cx = x;
                cy = y;
                last_c2 = None;
                last_q = None;
            }
            _ => return None,
        }
        args_needed = false;
    }
    if args_needed {
        return None;
    }
    Some(out)
}

fn flag(d: &[u8], i: &mut usize) -> Option<bool> {
    skip_sep(d, i);
    match d.get(*i)? {
        b'0' => {
            *i += 1;
            Some(false)
        }
        b'1' => {
            *i += 1;
            Some(true)
        }
        _ => None,
    }
}

/// Axis-aligned bounds over segment endpoints AND control points
/// (a conservative box — tighter would need arc/curve extrema).
pub fn bbox(segs: &[Seg]) -> Option<(P2, P2)> {
    let mut min = P2 {
        x: Fixed::MAX,
        y: Fixed::MAX,
    };
    let mut max = P2 {
        x: Fixed::MIN,
        y: Fixed::MIN,
    };
    let mut any = false;
    let mut acc = |x: Fixed, y: Fixed| {
        if x < min.x {
            min.x = x;
        }
        if y < min.y {
            min.y = y;
        }
        if x > max.x {
            max.x = x;
        }
        if y > max.y {
            max.y = y;
        }
        any = true;
    };
    for s in segs {
        match *s {
            Seg::M { x, y } | Seg::L { x, y } => acc(x, y),
            Seg::Q { cx, cy, x, y } => {
                acc(cx, cy);
                acc(x, y);
            }
            Seg::C {
                c1x,
                c1y,
                c2x,
                c2y,
                x,
                y,
            } => {
                acc(c1x, c1y);
                acc(c2x, c2y);
                acc(x, y);
            }
            Seg::S {
                c1x,
                c1y,
                c2x,
                c2y,
                x,
                y,
            } => {
                acc(c1x, c1y);
                acc(c2x, c2y);
                acc(x, y);
            }
            Seg::A { rx, ry, x, y, .. } => {
                acc(x - rx, y - ry);
                acc(x + rx, y + ry);
            }
            Seg::Z => {}
        }
    }
    if !any {
        return None;
    }
    Some((min, max))
}

/// A `Fixed` 2-D point.
#[derive(Debug, Clone, Copy)]
pub struct P2 {
    /// x
    pub x: Fixed,
    /// y
    pub y: Fixed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands() {
        let p = parse(b"M10 10L20 20l-5 0h5V5z").unwrap();
        assert_eq!(p.len(), 6);
        match p[2] {
            Seg::L { x, y } => {
                assert_eq!(x, Fixed::from_int(15));
                assert_eq!(y, Fixed::from_int(20));
            }
            _ => panic!(),
        }
        match p[3] {
            Seg::L { x, .. } => assert_eq!(x, Fixed::from_int(20)),
            _ => panic!(),
        }
        match p[4] {
            Seg::L { y, .. } => assert_eq!(y, Fixed::from_int(5)),
            _ => panic!(),
        }
        assert!(matches!(p[5], Seg::Z));
    }

    #[test]
    fn curves_and_arcs() {
        let p = parse(b"M0 0C1 2 3 4 5 6S9 10 11 12Q1 1 2 2T4 4A1 2 30 1 0 5 5").unwrap();
        assert_eq!(p.len(), 6);
        // S reflects C's c2=(3,4) about end (5,6) → c1=(7,8)
        match p[2] {
            Seg::S { c1x, c1y, .. } => {
                assert_eq!(c1x, Fixed::from_int(7));
                assert_eq!(c1y, Fixed::from_int(8));
            }
            _ => panic!(),
        }
        // T reflects Q's ctrl (1,1) about (2,2) → (3,3)
        match p[4] {
            Seg::Q { cx, .. } => assert_eq!(cx, Fixed::from_int(3)),
            _ => panic!(),
        }
        match p[5] {
            Seg::A { large, sweep, .. } => {
                assert!(large);
                assert!(!sweep);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn bbox_and_numbers() {
        let p = parse(b"M-1\x2e5 2\x2e5L3\x2e5 4\x2e5").unwrap();
        let (min, max) = bbox(&p).unwrap();
        assert_eq!(min.x, Fixed::from_raw(-98304)); // -3/2
        assert_eq!(max.y, Fixed::from_raw(294912)); // 9/2
        assert!(bbox(&[]).is_none());
        // implicit repeat: M with 4 coords → M + L
        let p2 = parse(b"M0 0 1 1 2 2").unwrap();
        assert!(matches!(p2[1], Seg::L { .. }));
        assert_eq!(p2.len(), 3);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(b"").unwrap().is_empty());
        assert!(parse(b"X1 2").is_none());
        assert!(parse(b"M").is_none());
        assert!(parse(b"L1").is_none());
        assert!(parse(b"A1 2 3 2 0 5 5").is_none()); // flag not 0/1
    }
}
