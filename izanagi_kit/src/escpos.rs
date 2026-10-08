//! ESC/POS receipt-printer command scanning.
//!
//! Key bytes: `ESC @` init, `ESC a n` align, `ESC E n` bold, `ESC d n`
//! feed, `GS V m` cut, `ESC p m t1 t2` drawer kick, `LF`/`FF` feed,
//! `ESC ! n` print mode.
//!
//! ```
//! use izanagi_kit::escpos::{parse, Op};
//!
//! let d = [0x1Bu8, b'@', b'H', b'i', 0x0A, 0x1D, b'V', 0x42, 0x00];
//! let s = parse(&d).unwrap();
//! assert_eq!(s.ops[0], Op::Init);
//! assert_eq!(s.ops[1], Op::Text);
//! assert_eq!(s.ops[3], Op::Cut);
//! ```

/// A decoded ESC/POS operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// `ESC @` — initialize.
    Init,
    /// `ESC a n` — justification.
    Align(u8),
    /// `ESC E n` — bold on/off.
    Bold(u8),
    /// `ESC ! n` — print mode.
    PrintMode(u8),
    /// `ESC d n` — print and feed n lines.
    FeedLines(u8),
    /// `GS V m` — cut (may have 1-2 param bytes).
    Cut,
    /// `ESC p m t1 t2` — cash drawer pulse.
    Drawer,
    /// `ESC i`/`ESC m` — partial cut.
    PartialCut,
    /// `LF`/`FF`/`CR` — feed/form feed.
    Feed,
    /// Printable text bytes (contiguous run stored as len).
    Text,
    /// Unknown escape/other byte (value kept).
    Other(u8),
}

/// Scanned ESC/POS stream.
#[derive(Debug)]
pub struct EscPos {
    /// Operations in order.
    pub ops: Vec<Op>,
    /// Number of `Op::Text` runs' total byte count.
    pub text_bytes: usize,
}

/// Scans an ESC/POS byte stream. Returns `None` when empty or when a
/// recognized prefix lacks its parameter bytes.
pub fn parse(d: &[u8]) -> Option<EscPos> {
    if d.is_empty() {
        return None;
    }
    const ESC: u8 = 0x1B;
    const GS: u8 = 0x1D;
    let mut ops = Vec::new();
    let mut text_bytes = 0usize;
    let mut i = 0usize;
    let push_text = |ops: &mut Vec<Op>| ops.push(Op::Text);
    while i < d.len() {
        let b = d[i];
        if b == ESC {
            if i + 1 >= d.len() {
                return None;
            }
            match d[i + 1] {
                b'@' => {
                    ops.push(Op::Init);
                    i += 2;
                }
                b'a' | b'E' | b'!' | b'd' => {
                    if i + 2 >= d.len() {
                        return None;
                    }
                    let arg = d[i + 2];
                    ops.push(match d[i + 1] {
                        b'a' => Op::Align(arg),
                        b'E' => Op::Bold(arg),
                        b'!' => Op::PrintMode(arg),
                        _ => Op::FeedLines(arg),
                    });
                    i += 3;
                }
                b'p' => {
                    if i + 4 >= d.len() {
                        return None;
                    }
                    ops.push(Op::Drawer);
                    i += 5;
                }
                b'i' | b'm' => {
                    ops.push(Op::PartialCut);
                    i += 2;
                }
                _ => {
                    ops.push(Op::Other(ESC));
                    i += 1;
                }
            }
        } else if b == GS {
            if i + 2 >= d.len() {
                return None;
            }
            if d[i + 1] == b'V' {
                ops.push(Op::Cut);
                // `GS V m` for m in {0,1,48,49}; `GS V m n` when m is
                // 65/66 ('A'/'B' feed-and-cut functions carry an extra n).
                i += if d[i + 2] >= 65 { 4 } else { 3 };
            } else {
                ops.push(Op::Other(GS));
                i += 1;
            }
        } else if b == 0x0A || b == 0x0C || b == 0x0D {
            ops.push(Op::Feed);
            i += 1;
        } else if b >= 0x20 || b == 0x09 {
            // text run
            let start = i;
            while i < d.len() && (d[i] >= 0x20 || d[i] == 0x09) {
                i += 1;
            }
            text_bytes += i - start;
            push_text(&mut ops);
        } else {
            ops.push(Op::Other(b));
            i += 1;
        }
    }
    Some(EscPos { ops, text_bytes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = [
            0x1B, b'@', 0x1B, b'a', 1, b'H', b'i', b'!', 0x0A, 0x1D, b'V', 0x42, 0, 0x1B, b'p', 0,
            25, 250,
        ];
        let s = parse(&d).unwrap();
        assert_eq!(s.ops[0], Op::Init);
        assert_eq!(s.ops[1], Op::Align(1));
        assert_eq!(s.ops[2], Op::Text);
        assert_eq!(s.ops[3], Op::Feed);
        assert_eq!(s.ops[4], Op::Cut);
        assert_eq!(s.ops[5], Op::Drawer);
        assert_eq!(s.text_bytes, 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x1B]).is_none());
        assert!(parse(&[0x1B, b'a']).is_none());
        assert!(parse(&[0x1B, b'p', 0, 1]).is_none());
    }
}
