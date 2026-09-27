//! rdiff — librsync's wire format (signature/delta streams). Files
//! open with a u32BE magic: `0x72730136`/`0x72730137` mark signature
//! files (MD4/BLAKE2 flavour, then block and strong-sum lengths), and
//! `0x72730236` marks a delta. Delta bodies are a byte-coded command
//! stream: `0` ends it, `0x01`..=`0x40` are immediate literal runs of
//! that many bytes, `0x41`..=`0x44` are literal runs with a 1/2/4/8
//! -byte length prefix, and `0x45`..=`0x54` are copy commands whose
//! (offset, len) operand widths step through the N1/N2/N4/N8 matrix.
//!
//! ```
//! use izanagi_kit::rdiff::{parse, commands, Command, Kind};
//! let mut d = vec![0x72, 0x73, 0x02, 0x36]; // delta magic
//! d.extend_from_slice(&[0x41, 2, 0xAA, 0xBB]); // literal N1 len 2
//! d.push(0);                                   // end
//! assert_eq!(parse(&d).unwrap().kind, Kind::Delta);
//! assert_eq!(commands(&d).next().unwrap(), Command::Literal(2));
//! ```

/// MD4 signature-file magic.
pub const SIG_MD4: u32 = 0x7273_0136;
/// BLAKE2 signature-file magic.
pub const SIG_BLAKE2: u32 = 0x7273_0137;
/// Delta-file magic.
pub const DELTA: u32 = 0x7273_0236;

/// Which stream a file carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Signature (weak+strong rolling sums).
    Signature,
    /// Delta (literal/copy command stream).
    Delta,
}

/// A parsed header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rdiff {
    /// Stream kind.
    pub kind: Kind,
    /// Signature files: block length.
    pub block_len: Option<u32>,
    /// Signature files: strong-sum length.
    pub strong_len: Option<u32>,
    /// Byte offset of the body.
    pub body_at: usize,
}

/// One delta command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `n` literal bytes follow.
    Literal(u64),
    /// Copy `len` bytes from the basis at `offset`.
    Copy {
        /// Basis-file offset.
        offset: u64,
        /// Byte count.
        len: u64,
    },
    /// End of stream.
    End,
    /// An opcode outside the 0x41–0x54 matrix (reserved/legacy).
    Reserved(u8),
}

fn u32b(d: &[u8], at: usize) -> Option<u32> {
    let r = d.get(at..at + 4)?;
    Some(
        (u32::from(r[0]) << 24)
            | (u32::from(r[1]) << 16)
            | (u32::from(r[2]) << 8)
            | u32::from(r[3]),
    )
}

/// Parse the header; `None` without a librsync magic.
pub fn parse(d: &[u8]) -> Option<Rdiff> {
    let magic = u32b(d, 0)?;
    match magic {
        SIG_MD4 | SIG_BLAKE2 => Some(Rdiff {
            kind: Kind::Signature,
            block_len: Some(u32b(d, 4)?),
            strong_len: Some(u32b(d, 8)?),
            body_at: 12,
        }),
        DELTA => Some(Rdiff {
            kind: Kind::Delta,
            block_len: None,
            strong_len: None,
            body_at: 4,
        }),
        _ => None,
    }
}

/// Operand widths for literal commands (`0x41`..=`0x44`): 1/2/4/8.
fn literal_width(op: u8) -> usize {
    1usize << (op - 0x41)
}

/// Operand widths for copy commands (`0x45`..=`0x54`):
/// `(offset_width, len_width)`, each stepping 1/2/4/8 over
/// `op - 0x45` = `off_class * 4 + len_class`.
fn copy_widths(op: u8) -> (usize, usize) {
    let n = op - 0x45;
    (1usize << (n / 4), 1usize << (n % 4))
}

fn read_be(d: &[u8], at: usize, n: usize) -> Option<u64> {
    let mut v = 0u64;
    for &c in d.get(at..at + n)? {
        v = v << 8 | u64::from(c);
    }
    Some(v)
}

/// Iterate delta commands; stops on `End`, truncation, or EOF.
pub fn commands(d: &[u8]) -> impl Iterator<Item = Command> + '_ {
    let mut at = 4usize;
    core::iter::from_fn(move || {
        let op = *d.get(at)?;
        at += 1;
        match op {
            0x00 => Some(Command::End),
            0x01..=0x40 => {
                at = at.checked_add(usize::from(op))?;
                d.get(..at)?;
                Some(Command::Literal(u64::from(op)))
            }
            0x41..=0x44 => {
                let n = literal_width(op);
                let len = read_be(d, at, n)?;
                at += n;
                at = at.checked_add(usize::try_from(len).ok()?)?;
                d.get(..at)?; // literal bytes must fit
                Some(Command::Literal(len))
            }
            0x45..=0x54 => {
                let (ow, lw) = copy_widths(op);
                let offset = read_be(d, at, ow)?;
                let len = read_be(d, at + ow, lw)?;
                at += ow + lw;
                Some(Command::Copy { offset, len })
            }
            _ => Some(Command::Reserved(op)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_magics() {
        let mut s = vec![0x72, 0x73, 0x01, 0x36];
        s.extend_from_slice(&[0, 0, 8, 0]); // block_len 2048, big-endian
        s.extend_from_slice(&[0, 0, 0, 16]); // strong_len 16
        let r = parse(&s).unwrap();
        assert_eq!(r.kind, Kind::Signature);
        assert_eq!(r.block_len, Some(2048));
        assert_eq!(r.strong_len, Some(16));
        assert!(parse(b"").is_none());
        assert!(parse(&[0x72, 0x73, 0x02, 0x37]).is_none());
    }

    #[test]
    fn delta_walk() {
        let mut d = vec![0x72, 0x73, 0x02, 0x36];
        d.extend_from_slice(&[0x41, 2, 0xAA, 0xBB]); // literal N1
                                                     // copy N1_N2: offset u8, len u16BE
        d.extend_from_slice(&[0x46, 0x09, 0x01, 0x00]);
        d.extend_from_slice(&[0x43, 0, 0, 0, 3, 0x11, 0x22, 0x33]); // literal N4
        d.push(0);
        let r = parse(&d).unwrap();
        assert_eq!(r.kind, Kind::Delta);
        let cs: Vec<_> = commands(&d).collect();
        assert_eq!(
            cs,
            [
                Command::Literal(2),
                Command::Copy {
                    offset: 9,
                    len: 256
                },
                Command::Literal(3),
                Command::End
            ]
        );
    }

    #[test]
    fn truncation_and_reserved() {
        let mut d = vec![0x72, 0x73, 0x02, 0x36];
        d.extend_from_slice(&[0x42, 0x01]); // literal N2 truncated mid-length
        assert!(commands(&d).next().is_none());
        let mut d2 = vec![0x72, 0x73, 0x02, 0x36];
        d2.push(0x60); // outside the 0x00..0x54 command space
        assert_eq!(commands(&d2).next(), Some(Command::Reserved(0x60)));
        // immediate literal: op itself is the length
        let mut d3 = vec![0x72, 0x73, 0x02, 0x36];
        d3.extend_from_slice(&[0x02, 0xAA, 0xBB]);
        d3.push(0);
        let cs: Vec<_> = commands(&d3).collect();
        assert_eq!(cs, [Command::Literal(2), Command::End]);
    }
}
