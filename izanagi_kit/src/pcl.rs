//! PCL 5 escape-sequence scanning.
//!
//! Commands are `ESC` + parameterized char (`*`, `&`, `(`...) + group char
//! + value digits + terminator (uppercase) or continuation (lowercase).
//!
//! Examples: reset `ESC E`, raster `ESC *r#A`, orientation `ESC &l#O`,
//! typeface `ESC (s#T`, and `ESC %...` mode enter/exit.
//!
//! ```
//! use izanagi_kit::pcl::{parse, Group};
//!
//! let d = [0x1Bu8, b'E', 0x1B, b'&', b'l', b'1', b'O'];
//! let p = parse(&d).unwrap();
//! assert_eq!(p.cmds.len(), 2);
//! assert_eq!(p.cmds[1].group, Group::PageControl);
//! assert_eq!(p.cmds[1].value, 1);
//! ```

/// A decoded PCL escape sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PclCmd {
    /// Parameterized byte (e.g. `&`, `*`, `(`, or `None` for 2-char `ESC x`).
    pub param_byte: Option<u8>,
    /// Group byte (e.g. `l`, `s`, `r`).
    pub group_byte: Option<u8>,
    /// Numeric value (0 when absent, e.g. `ESC E`).
    pub value: u32,
    /// Terminator byte (uppercase) or the second char of 2-char commands.
    pub terminator: u8,
    /// Group classification.
    pub group: Group,
}

/// Common PCL command groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// `ESC E` — printer reset.
    Reset,
    /// `ESC %..` — PCL/HPGL mode switching.
    ModeSelect,
    /// `ESC &l..` — job/page control (paper, orientation, copies...).
    PageControl,
    /// `ESC *r..` — raster graphics.
    Raster,
    /// `ESC (s..`/`ESC )s..` — fonts.
    Font,
    /// Any other sequence.
    Other,
}

/// Parsed PCL stream.
#[derive(Debug)]
pub struct Pcl {
    /// Escape sequences in order.
    pub cmds: Vec<PclCmd>,
    /// Number of non-escape data bytes seen.
    pub data_bytes: usize,
}

/// Scans a PCL stream. Returns `None` when input contains no escape
/// sequence at all (not a printer stream) or a sequence is malformed.
pub fn parse(d: &[u8]) -> Option<Pcl> {
    const ESC: u8 = 0x1B;
    let mut cmds = Vec::new();
    let mut data_bytes = 0usize;
    let mut i = 0usize;
    while i < d.len() {
        if d[i] != ESC {
            data_bytes += 1;
            i += 1;
            continue;
        }
        if i + 1 >= d.len() {
            return None;
        }
        // `ESC %<digits><X>` — PCL/HPGL mode select
        if d[i + 1] == b'%' {
            let mut j = i + 2;
            let mut value: u32 = 0;
            while j < d.len() && d[j].is_ascii_digit() {
                value = value.checked_mul(10)?.checked_add((d[j] - b'0') as u32)?;
                j += 1;
            }
            if j >= d.len() || !d[j].is_ascii_uppercase() {
                return None;
            }
            cmds.push(PclCmd {
                param_byte: Some(b'%'),
                group_byte: None,
                value,
                terminator: d[j],
                group: Group::ModeSelect,
            });
            i = j + 1;
            continue;
        }
        // 2-char command: ESC + @ E 9 Z = -12345X etc.
        if d[i + 1].is_ascii_uppercase() {
            cmds.push(PclCmd {
                param_byte: None,
                group_byte: None,
                value: 0,
                terminator: d[i + 1],
                group: if d[i + 1] == b'E' {
                    Group::Reset
                } else {
                    Group::Other
                },
            });
            i += 2;
            continue;
        }
        // parameterized: ESC <param> <group> <digits> <term/lower-continue>
        if i + 2 >= d.len() {
            return None;
        }
        let param_byte = d[i + 1];
        let group_byte = d[i + 2];
        if !matches!(param_byte, b'*' | b'&' | b'(' | b')' | b'%')
            || !group_byte.is_ascii_lowercase()
        {
            return None;
        }
        let mut j = i + 3;
        let mut value: u32 = 0;
        let mut neg = false;
        if j < d.len() && d[j] == b'-' {
            neg = true;
            j += 1;
        }
        while j < d.len() && d[j].is_ascii_digit() {
            value = value.checked_mul(10)?.checked_add((d[j] - b'0') as u32)?;
            j += 1;
        }
        let _ = neg; // values stored as magnitude
        if j >= d.len() {
            return None;
        }
        let term = d[j];
        // lowercase terminator = same-group continuation; walk the chain
        let mut last_term = term;
        let mut jj = j;
        while last_term.is_ascii_lowercase() {
            // chained: next is group byte again then digits then terminator
            if jj + 2 >= d.len() {
                return None;
            }
            jj += 1;
            while jj < d.len() && d[jj].is_ascii_digit() {
                jj += 1;
            }
            if jj >= d.len() {
                return None;
            }
            last_term = d[jj];
        }
        if !last_term.is_ascii_uppercase() {
            return None;
        }
        let group = match (param_byte, group_byte, last_term) {
            (_, b'l', _) => Group::PageControl,
            (b'*', b'r', _) => Group::Raster,
            (b'(' | b')', b's', _) => Group::Font,
            _ => Group::Other,
        };
        cmds.push(PclCmd {
            param_byte: Some(param_byte),
            group_byte: Some(group_byte),
            value,
            terminator: last_term,
            group,
        });
        i = jj + 1;
    }
    if cmds.is_empty() {
        None
    } else {
        Some(Pcl { cmds, data_bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        // ESC E  ESC &l0O  ESC *r1A  ESC (s3T  text bytes
        let d = [
            0x1B, b'E', 0x1B, b'&', b'l', b'0', b'O', 0x1B, b'*', b'r', b'1', b'A', 0x1B, b'(',
            b's', b'3', b'T', b'H', b'i',
        ];
        let p = parse(&d).unwrap();
        assert_eq!(p.cmds.len(), 4);
        assert_eq!(p.cmds[0].group, Group::Reset);
        assert_eq!(p.cmds[1].group, Group::PageControl);
        assert_eq!(p.cmds[2].group, Group::Raster);
        assert_eq!(p.cmds[3].group, Group::Font);
        assert_eq!(p.cmds[3].value, 3);
        assert_eq!(p.data_bytes, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"hello").is_none());
        assert!(parse(&[0x1B]).is_none());
        assert!(parse(&[0x1B, b'&']).is_none());
        assert!(parse(&[0x1B, b'&', b'L', b'0', b'O']).is_none()); // uppercase group byte
    }
}
