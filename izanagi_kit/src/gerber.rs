//! RS-274X (Extended Gerber) file scanning.
//!
//! Text format: `G04` comments, `%XX*%` parameter blocks (`FS`, `MO`, `AD`,
//! `AM`, `LP`, `TF`...), `X..Y..D0N` coordinate words, `M02` end.
//!
//! ```
//! use izanagi_kit::gerber::{parse, Op};
//!
//! let g = b"G04 demo*\n%FSLAX24Y24*%\n%MOMM*%\nX10000Y20000D01*\nM02*\n";
//! let r = parse(g).unwrap();
//! assert!(r.metric);
//! assert_eq!(r.ops.len(), 1);
//! assert_eq!(r.ops[0], Op::Draw);
//! ```

/// A single drawing/state operation derived from a D-code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// D01 — draw (exposure on).
    Draw,
    /// D02 — move (exposure off).
    Move,
    /// D03 — flash.
    Flash,
}

/// Parsed Gerber summary.
pub struct Gerber {
    /// Units are millimetres (`%MOMM*%`) vs inches (`%MOIN*%`); required.
    pub metric: bool,
    /// Coordinate format (x integer digits, x decimals, y digits, y decimals)
    /// from `%FS[LT]A?XnnYnn*%`.
    pub format: Option<(u8, u8, u8, u8)>,
    /// Aperture definitions count (`%AD`).
    pub apertures: usize,
    /// Ordered draw/move/flash ops.
    pub ops: Vec<Op>,
}

/// Parses an RS-274X file. Requires `%MOxx*%` and a terminating `M02`.
pub fn parse(d: &[u8]) -> Option<Gerber> {
    let mut metric = None;
    let mut format = None;
    let mut apertures = 0usize;
    let mut ops = Vec::new();
    let mut ended = false;
    let mut i = 0usize;
    while i < d.len() {
        if i + 1 < d.len() && d[i] == b'%' {
            // extended parameter block until `*%`
            let end = find(d, i + 1, b"*%")?;
            let body = &d[i + 1..end];
            if body.starts_with(b"MO") && body.len() >= 4 {
                metric = Some(match &body[2..4] {
                    b"MM" => true,
                    b"IN" => false,
                    _ => return None,
                });
            } else if body.starts_with(b"FS") {
                // FSLAX24Y24 or FSTAX...
                format = Some(parse_fs(body)?);
            } else if body.starts_with(b"AD") {
                apertures += 1;
            }
            i = end + 2;
        } else if d[i].is_ascii_alphabetic() || d[i].is_ascii_digit() {
            // word block until `*`
            let end = find(d, i, b"*")?;
            let body = &d[i..end];
            if body == b"M02" {
                ended = true;
            }
            // Dnn suffix selects operation: back over trailing digits, then
            // the preceding char must be `D`.
            let mut j = body.len();
            while j > 0 && body[j - 1].is_ascii_digit() {
                j -= 1;
            }
            if j > 0 && body[j - 1] == b'D' {
                match &body[j..] {
                    b"01" | b"1" => ops.push(Op::Draw),
                    b"02" | b"2" => ops.push(Op::Move),
                    b"03" | b"3" => ops.push(Op::Flash),
                    _ => {}
                }
            } else {
                // bare coordinate word ending in Dnn is covered above;
                // G-codes and M-codes pass through
                if body.len() >= 2 && body[0] == b'X' {
                    // coordinate without D code: repeat last op (Gerber modal)
                    if let Some(&last) = ops.last() {
                        ops.push(last);
                    }
                }
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    if !ended {
        return None;
    }
    Some(Gerber {
        metric: metric?,
        format,
        apertures,
        ops,
    })
}

fn find(d: &[u8], from: usize, pat: &[u8]) -> Option<usize> {
    (from..d.len().saturating_sub(pat.len() - 1)).find(|&i| &d[i..i + pat.len()] == pat)
}

fn parse_fs(body: &[u8]) -> Option<(u8, u8, u8, u8)> {
    // e.g. FSLAX24Y24 — last 2 digits of X and Y groups
    let xpos = body.iter().position(|&b| b == b'X')?;
    let ypos = body.iter().position(|&b| b == b'Y')?;
    if ypos <= xpos + 2 || xpos + 3 > body.len() || ypos + 3 > body.len() {
        return None;
    }
    let dig = |b: u8| -> Option<u8> {
        if b.is_ascii_digit() {
            Some(b - b'0')
        } else {
            None
        }
    };
    Some((
        dig(body[xpos + 1])?,
        dig(body[xpos + 2])?,
        dig(body[ypos + 1])?,
        dig(body[ypos + 2])?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let g = b"G04 t*\n%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\nX0Y0D02*\nX10000Y0D01*\nX10000Y10000D03*\nM02*\n";
        let r = parse(g).unwrap();
        assert!(r.metric);
        assert_eq!(r.format, Some((2, 4, 2, 4)));
        assert_eq!(r.apertures, 1);
        assert_eq!(r.ops, vec![Op::Move, Op::Draw, Op::Flash]);
    }

    #[test]
    fn modal_repeat() {
        let g = b"%MOIN*%\nX0Y0D01*\nX100Y100*\nM02*";
        let r = parse(g).unwrap();
        assert!(!r.metric);
        assert_eq!(r.ops.len(), 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"%MOXX*%\nM02*").is_none()); // bad units
        assert!(parse(b"%MOMM*%").is_none()); // no M02
        assert!(parse(b"M02*").is_none()); // no MO
    }
}
