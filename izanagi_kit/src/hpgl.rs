//! HP-GL (Hewlett-Packard Graphics Language) command scanning.
//!
//! Commands are two letters followed by comma-separated numeric parameters
//! and a `;` terminator: `IN;`, `SP1;`, `PU0,0;`, `PD100,100;`, `IP..;`.
//!
//! ```
//! use izanagi_kit::hpgl::{parse, Cmd};
//!
//! let r = parse(b"IN;SP1;PU0,0;PD100,0,100,100;PU;").unwrap();
//! assert_eq!(r.len(), 5);
//! assert_eq!(r[3].code, Cmd::Pd);
//! assert_eq!(r[3].params, vec![100, 0, 100, 100]);
//! ```

/// HP-GL command code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    /// `IN` — initialize.
    In,
    /// `IP` — set scaling points P1/P2.
    Ip,
    /// `SC` — scale.
    Sc,
    /// `SP` — select pen.
    Sp,
    /// `PU` — pen up (move).
    Pu,
    /// `PD` — pen down (draw).
    Pd,
    /// `PA` — plot absolute.
    Pa,
    /// `PR` — plot relative.
    Pr,
    /// `CI` — circle.
    Ci,
    /// `AA`/`AR` — arc.
    Arc,
    /// `LB` — label text.
    Lb,
    /// Other two-letter code.
    Other([u8; 2]),
}

/// One parsed HP-GL command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HpglCmd {
    /// Command code.
    pub code: Cmd,
    /// Integer parameters (empty for parameterless commands; `LB` text kept
    /// in `text`).
    pub params: Vec<i32>,
    /// Label text for `LB` (terminated by ETX 0x03 or `;`).
    pub text: Vec<u8>,
}

/// Scans an HP-GL stream; unknown 2-letter codes are kept as `Other`.
/// Returns `None` when no commands are found or the input has a stray byte
/// that is not whitespace/terminator between commands.
pub fn parse(d: &[u8]) -> Option<Vec<HpglCmd>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        let b = d[i];
        if b.is_ascii_whitespace() || b == b';' {
            i += 1;
            continue;
        }
        if !b.is_ascii_uppercase() || i + 1 >= d.len() || !d[i + 1].is_ascii_uppercase() {
            return None;
        }
        let code = &d[i..i + 2];
        i += 2;
        let mut params = Vec::new();
        let mut text = Vec::new();
        if code == b"LB" {
            // label: bytes until ETX or ';'
            while i < d.len() && d[i] != 0x03 && d[i] != b';' {
                text.push(d[i]);
                i += 1;
            }
        } else {
            // integer params separated by ','
            while i < d.len() && d[i] != b';' {
                let neg = d[i] == b'-';
                if neg || d[i] == b'+' {
                    i += 1;
                }
                let start = i;
                while i < d.len() && d[i].is_ascii_digit() {
                    i += 1;
                }
                if start == i {
                    if d[i] == b',' || d[i].is_ascii_whitespace() {
                        i += 1;
                        continue;
                    }
                    return None;
                }
                let mut v: i32 = 0;
                for &c in &d[start..i] {
                    v = v.checked_mul(10)?.checked_add((c - b'0') as i32)?;
                }
                params.push(if neg { -v } else { v });
                if i < d.len() && d[i] == b',' {
                    i += 1;
                }
            }
        }
        if i < d.len() && (d[i] == b';' || d[i] == 0x03) {
            i += 1;
        }
        out.push(HpglCmd {
            code: match code {
                b"IN" => Cmd::In,
                b"IP" => Cmd::Ip,
                b"SC" => Cmd::Sc,
                b"SP" => Cmd::Sp,
                b"PU" => Cmd::Pu,
                b"PD" => Cmd::Pd,
                b"PA" => Cmd::Pa,
                b"PR" => Cmd::Pr,
                b"CI" => Cmd::Ci,
                b"AA" | b"AR" => Cmd::Arc,
                b"LB" => Cmd::Lb,
                _ => Cmd::Other([code[0], code[1]]),
            },
            params,
            text,
        });
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let r = parse(b"IN;IP0,0,8000,8000;SP1;PU100,200;PD300,400,-5,10;LBHI\x03").unwrap();
        assert_eq!(r[0].code, Cmd::In);
        assert_eq!(r[1].params, vec![0, 0, 8000, 8000]);
        assert_eq!(r[3].code, Cmd::Pu);
        assert_eq!(r[4].params, vec![300, 400, -5, 10]);
        assert_eq!(r[5].text, b"HI");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"  \n;").is_none());
        assert!(parse(b"IN;x").is_none()); // lowercase
        assert!(parse(b"PUx").is_none()); // garbage param
    }
}
