//! GDIFF — the W3C NOTE "Generic Diff Format" command stream. A file
//! opens with the four-byte magic `D1 FF D1 FF` plus a version byte
//! (4 or 5), then a byte-coded command stream: `0` ends the stream,
//! `1`..=246 emit that many literal bytes, `247`/`248` take a u16/u32
//! literal length, `249`..=`253` encode a copy with byte-aligned
//! offset/length operands, and `254` is a u32 application checksum.
//!
//! ```
//! use izanagi_kit::gdiff::{parse, commands, Command};
//! let mut d = vec![0xD1, 0xFF, 0xD1, 0xFF, 4];
//! d.extend_from_slice(&[2, 0xAA, 0xBB]); // 2 literal bytes
//! d.push(0);                             // EOF
//! assert_eq!(parse(&d).unwrap(), 4);
//! let cs: Vec<_> = commands(&d).collect();
//! assert_eq!(cs[0], Command::Literal(2));
//! ```

/// File magic.
pub const MAGIC: &[u8; 4] = &[0xD1, 0xFF, 0xD1, 0xFF];
/// End-of-stream opcode.
pub const OP_EOF: u8 = 0;
/// Checksum opcode.
pub const OP_CHECKSUM: u8 = 254;

/// One decoded command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Copy `n` literal bytes from the stream.
    Literal(u32),
    /// Copy `len` bytes from the source at `offset`.
    Copy {
        /// Source offset.
        offset: u32,
        /// Byte count.
        len: u32,
    },
    /// u32 application checksum follows.
    Checksum,
    /// End of stream.
    Eof,
}

/// Parse the header; returns the version byte.
pub fn parse(d: &[u8]) -> Option<u8> {
    if d.get(..4)? != MAGIC {
        return None;
    }
    Some(*d.get(4)?)
}

/// Iterate commands; stops on `Eof`, a truncated operand, or EOF.
/// `Literal` yields just the length — the bytes follow in the stream
/// and are skipped here.
pub fn commands(d: &[u8]) -> impl Iterator<Item = Command> + '_ {
    let mut at = 5usize;
    core::iter::from_fn(move || {
        let op = *d.get(at)?;
        at += 1;
        let u16l = |d: &[u8], at: usize| -> Option<u32> {
            let r = d.get(at..at + 2)?;
            Some((u32::from(r[0]) << 8) | u32::from(r[1]))
        };
        let u32b = |d: &[u8], at: usize| -> Option<u32> {
            let r = d.get(at..at + 4)?;
            Some(
                (u32::from(r[0]) << 24)
                    | (u32::from(r[1]) << 16)
                    | (u32::from(r[2]) << 8)
                    | u32::from(r[3]),
            )
        };
        match op {
            OP_EOF => Some(Command::Eof),
            1..=246 => {
                at = at.checked_add(usize::from(op))?;
                d.get(..at)?; // literals must fit
                Some(Command::Literal(u32::from(op)))
            }
            247 => {
                let n = u16l(d, at)?;
                at += 2 + n as usize;
                d.get(..at)?;
                Some(Command::Literal(n))
            }
            248 => {
                let n = u32b(d, at)?;
                at = at.checked_add(4 + n as usize)?;
                d.get(..at)?;
                Some(Command::Literal(n))
            }
            249 => {
                let off = u16l(d, at)?;
                let len = u32::from(*d.get(at + 2)?);
                at += 3;
                Some(Command::Copy { offset: off, len })
            }
            250 => {
                let off = u16l(d, at)?;
                let len = u16l(d, at + 2)?;
                at += 4;
                Some(Command::Copy { offset: off, len })
            }
            251 => {
                let off = u32b(d, at)?;
                let len = u32::from(*d.get(at + 4)?);
                at += 5;
                Some(Command::Copy { offset: off, len })
            }
            252 => {
                let off = u32b(d, at)?;
                let len = u16l(d, at + 4)?;
                at += 6;
                Some(Command::Copy { offset: off, len })
            }
            253 => {
                let off = u32b(d, at)?;
                let len = u32b(d, at + 4)?;
                at += 8;
                Some(Command::Copy { offset: off, len })
            }
            OP_CHECKSUM => {
                at = at.checked_add(4)?;
                d.get(..at)?;
                Some(Command::Checksum)
            }
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0xD1, 0xFF, 0xD1, 0xFF, 4];
        d.extend_from_slice(&[3, 0xAA, 0xBB, 0xCC]); // literal 3
        d.extend_from_slice(&[249, 0x12, 0x34, 5]); // copy off 0x1234 len 5
        d.extend_from_slice(&[OP_CHECKSUM, 0, 0, 0, 0]);
        d.push(0);
        d
    }

    #[test]
    fn parses_and_walks() {
        let d = fixture();
        assert_eq!(parse(&d), Some(4));
        let cs: Vec<_> = commands(&d).collect();
        assert_eq!(
            cs,
            [
                Command::Literal(3),
                Command::Copy {
                    offset: 0x1234,
                    len: 5
                },
                Command::Checksum,
                Command::Eof
            ]
        );
    }

    #[test]
    fn wide_operands() {
        let mut d = vec![0xD1, 0xFF, 0xD1, 0xFF, 5];
        d.extend_from_slice(&[247, 1, 0]); // literal u16 len 256
        d.extend_from_slice(&[0; 256]);
        d.extend_from_slice(&[253, 0, 0, 0, 9, 0, 0, 0, 7]); // u32/u32 copy
        d.push(0);
        let cs: Vec<_> = commands(&d).collect();
        assert_eq!(
            cs,
            [
                Command::Literal(256),
                Command::Copy { offset: 9, len: 7 },
                Command::Eof
            ]
        );
    }

    #[test]
    fn rejects_and_stops() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0xD1, 0xFF, 0xD1, 0xFE]).is_none());
        // truncated literal: claims 3 bytes, gives 1
        let d = vec![0xD1, 0xFF, 0xD1, 0xFF, 4, 3, 0xAA];
        assert!(commands(&d).next().is_none());
        // unknown opcode 255? — 255 is unassigned beyond checksum
        let d2 = vec![0xD1, 0xFF, 0xD1, 0xFF, 4, 255];
        assert!(commands(&d2).next().is_none());
    }
}
