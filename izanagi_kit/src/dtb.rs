//! Flattened device tree blob (`.dtb`, Devicetree/OF format).
//!
//! The 40-byte header is big-endian: magic `0xD00DFEED`, totalsize,
//! offsets to the structure / strings / reserved-map blocks, version,
//! `boot_cpuid`, and the block sizes. The structure block is a stream
//! of big-endian token words: `1` begin-node + NUL-terminated name,
//! `2` end-node, `3` property (`len`, `nameoff`, data, 4-aligned),
//! `4` nop, `9` end. Property names live in the strings block.
//!
//! ```
//! use izanagi_kit::dtb::{parse, Token};
//!
//! let mut d = vec![0u8; 120];
//! let put = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = (v >> 24) as u8; d[at + 1] = (v >> 16) as u8;
//!     d[at + 2] = (v >> 8) as u8; d[at + 3] = v as u8;
//! };
//! put(&mut d, 0, 0xD00DFEED);
//! put(&mut d, 4, 120);   // totalsize
//! put(&mut d, 8, 40);    // off_dt_struct
//! put(&mut d, 12, 100);  // off_dt_strings
//! put(&mut d, 20, 17);   // version
//! put(&mut d, 32, 20);   // size_dt_strings
//! put(&mut d, 36, 60);   // size_dt_struct
//! // struct block: BEGIN_NODE "" , PROP(len 1, nameoff 0, 'x'), END
//! put(&mut d, 40, 1);
//! put(&mut d, 48, 3);
//! put(&mut d, 52, 1);    // prop len
//! put(&mut d, 56, 0);    // nameoff -> strings[0]
//! d[60] = b'x';
//! put(&mut d, 64, 9);    // END
//! d[100] = b'r'; d[101] = b'o'; d[102] = b'o'; d[103] = b't'; // "root"
//! let t = parse(&d).unwrap();
//! let toks: Vec<_> = t.tokens(&d).collect();
//! assert_eq!(toks[0], Token::BeginNode { name_at: 44 });
//! assert_eq!(toks[1], Token::Prop { len: 1, name_off: 0, data_at: 60 });
//! assert_eq!(t.string(&d, 0), Some(b"root".as_ref()));
//! ```

/// Magic word at offset 0 (`0xD00DFEED`).
pub const MAGIC: u32 = 0xD00D_FEED;
/// Header size in bytes.
pub const HEADER: usize = 40;
/// `BEGIN_NODE` token id.
pub const TOK_BEGIN_NODE: u32 = 1;
/// `END_NODE` token id.
pub const TOK_END_NODE: u32 = 2;
/// `PROP` token id.
pub const TOK_PROP: u32 = 3;
/// `NOP` token id.
pub const TOK_NOP: u32 = 4;
/// `END` token id.
pub const TOK_END: u32 = 9;

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}

/// A structure-block token. `name_at`/`data_at` are absolute offsets
/// into the original buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    /// `BEGIN_NODE` — a NUL-terminated name follows at `name_at`.
    BeginNode {
        /// Offset of the node's NUL-terminated name.
        name_at: usize,
    },
    /// `END_NODE`.
    EndNode,
    /// `PROP` — `len` bytes at `data_at`, name at strings block +
    /// `name_off`.
    Prop {
        /// Property payload length in bytes.
        len: u32,
        /// Property name offset within the strings block.
        name_off: u32,
        /// Offset of the payload.
        data_at: usize,
    },
    /// `NOP`.
    Nop,
    /// `END` — the structure block terminator.
    End,
}

/// A parsed device-tree header.
#[derive(Clone, Debug, PartialEq)]
pub struct Dtb {
    /// Total blob size in bytes.
    pub totalsize: u32,
    /// Offset of the structure block.
    pub struct_at: u32,
    /// Offset of the strings block.
    pub strings_at: u32,
    /// Offset of the memory-reserve map.
    pub rsv_at: u32,
    /// Format version (17 is current).
    pub version: u32,
    /// Lowest compatible version.
    pub last_comp_version: u32,
    /// Physical id of the boot CPU.
    pub boot_cpuid: u32,
    /// Size of the strings block.
    pub strings_size: u32,
    /// Size of the structure block.
    pub struct_size: u32,
}

impl Dtb {
    /// Iterator over the structure-block token stream. Ends at `END`
    /// or on any malformed/out-of-bounds token.
    pub fn tokens<'a>(&self, d: &'a [u8]) -> Tokens<'a> {
        Tokens {
            d,
            at: self.struct_at as usize,
            end: self
                .struct_at
                .checked_add(self.struct_size)
                .map_or(d.len(), |e| e as usize)
                .min(d.len()),
            done: false,
        }
    }

    /// A NUL-terminated name inside the strings block (for `Prop`
    /// `name_off` values). `None` when out of range or unterminated.
    pub fn string<'a>(&self, d: &'a [u8], off: u32) -> Option<&'a [u8]> {
        let at = usize::try_from(self.strings_at).ok()? + usize::try_from(off).ok()?;
        let end_block =
            usize::try_from(self.strings_at).ok()? + usize::try_from(self.strings_size).ok()?;
        if at >= end_block {
            return None;
        }
        let nul = d.get(at..end_block)?.iter().position(|&b| b == 0)?;
        d.get(at..at + nul)
    }

    /// A NUL-terminated node name inside the structure block (for
    /// `BeginNode::name_at`).
    pub fn node_name<'a>(&self, d: &'a [u8], at: usize) -> Option<&'a [u8]> {
        let nul = d.get(at..)?.iter().position(|&b| b == 0)?;
        d.get(at..at + nul)
    }
}

/// Token-stream iterator over the structure block.
pub struct Tokens<'a> {
    d: &'a [u8],
    at: usize,
    end: usize,
    done: bool,
}

impl<'a> Tokens<'a> {
    fn word(&self, at: usize) -> Option<u32> {
        if at + 4 > self.end {
            return None;
        }
        be32(self.d, at)
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if self.done {
            return None;
        }
        let t = match self.word(self.at)? {
            TOK_BEGIN_NODE => {
                let name_at = self.at + 4;
                // name is NUL-terminated and padded to a 4-byte boundary
                let nul = self
                    .d
                    .get(name_at..self.end)?
                    .iter()
                    .position(|&b| b == 0)?;
                self.at = (name_at + nul + 1 + 3) & !3;
                Token::BeginNode { name_at }
            }
            TOK_END_NODE => {
                self.at += 4;
                Token::EndNode
            }
            TOK_PROP => {
                let len = self.word(self.at + 4)?;
                let name_off = self.word(self.at + 8)?;
                let data_at = self.at + 12;
                let data_end = data_at.checked_add(len as usize)?;
                if data_end > self.end {
                    self.done = true;
                    return None;
                }
                self.at = (data_end + 3) & !3;
                Token::Prop {
                    len,
                    name_off,
                    data_at,
                }
            }
            TOK_NOP => {
                self.at += 4;
                Token::Nop
            }
            TOK_END => {
                self.done = true;
                Token::End
            }
            _ => return None,
        };
        Some(t)
    }
}

/// Parse an FDT header. Returns `None` on a bad magic or a buffer
/// shorter than 40 bytes.
pub fn parse(d: &[u8]) -> Option<Dtb> {
    if be32(d, 0)? != MAGIC {
        return None;
    }
    Some(Dtb {
        totalsize: be32(d, 4)?,
        struct_at: be32(d, 8)?,
        strings_at: be32(d, 12)?,
        rsv_at: be32(d, 16)?,
        version: be32(d, 20)?,
        last_comp_version: be32(d, 24)?,
        boot_cpuid: be32(d, 28)?,
        strings_size: be32(d, 32)?,
        struct_size: be32(d, 36)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn put(d: &mut [u8], at: usize, v: u32) {
        d[at] = (v >> 24) as u8;
        d[at + 1] = (v >> 16) as u8;
        d[at + 2] = (v >> 8) as u8;
        d[at + 3] = v as u8;
    }

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; 128];
        put(&mut d, 0, MAGIC);
        put(&mut d, 4, 128);
        put(&mut d, 8, 40);
        put(&mut d, 12, 100);
        put(&mut d, 16, 120);
        put(&mut d, 20, 17);
        put(&mut d, 24, 16);
        put(&mut d, 28, 0);
        put(&mut d, 32, 20);
        put(&mut d, 36, 60);
        // BEGIN_NODE "cpu"
        put(&mut d, 40, TOK_BEGIN_NODE);
        d[44] = b'c';
        d[45] = b'p';
        d[46] = b'u';
        // 47 = 0 -> name padded to 48
        // PROP "speed" len 2 at 48
        put(&mut d, 48, TOK_PROP);
        put(&mut d, 52, 2);
        put(&mut d, 56, 0); // nameoff 0 -> "speed"
        d[60] = 0x12;
        d[61] = 0x34;
        // NOP then END_NODE then END
        put(&mut d, 64, TOK_NOP);
        put(&mut d, 68, TOK_END_NODE);
        put(&mut d, 72, TOK_END);
        // strings block at 100
        d[100..105].copy_from_slice(b"speed");
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let t = parse(&image()).unwrap();
        assert_eq!(t.totalsize, 128);
        assert_eq!(t.struct_at, 40);
        assert_eq!(t.strings_at, 100);
        assert_eq!(t.rsv_at, 120);
        assert_eq!(t.version, 17);
        assert_eq!(t.last_comp_version, 16);
        assert_eq!(t.boot_cpuid, 0);
        assert_eq!(t.strings_size, 20);
        assert_eq!(t.struct_size, 60);
    }

    #[test]
    fn tokens_walk_the_structure() {
        let d = image();
        let t = parse(&d).unwrap();
        let toks: Vec<_> = t.tokens(&d).collect();
        assert_eq!(
            toks,
            [
                Token::BeginNode { name_at: 44 },
                Token::Prop {
                    len: 2,
                    name_off: 0,
                    data_at: 60
                },
                Token::Nop,
                Token::EndNode,
                Token::End,
            ]
        );
        assert_eq!(t.node_name(&d, 44), Some(b"cpu".as_ref()));
        assert_eq!(t.string(&d, 0), Some(b"speed".as_ref()));
        assert_eq!(&d[60..62], &[0x12, 0x34]);
    }

    #[test]
    fn truncated_token_stream_stops() {
        let mut d = image();
        // corrupt the PROP len to run past the struct block
        put(&mut d, 52, 1000);
        let t = parse(&d).unwrap();
        let toks: Vec<_> = t.tokens(&d).collect();
        assert_eq!(toks, [Token::BeginNode { name_at: 44 }]);
    }

    #[test]
    fn string_bounds() {
        let d = image();
        let t = parse(&d).unwrap();
        assert_eq!(t.string(&d, 19), Some(b"".as_ref()));
        assert_eq!(t.string(&d, 20), None);
        assert_eq!(t.string(&d, 9999), None);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[0u8; 39]), None);
        let mut d = image();
        d[0] = 0;
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn constants() {
        assert_eq!(MAGIC, 0xD00D_FEED);
        assert_eq!(HEADER, 40);
        assert_eq!(TOK_BEGIN_NODE, 1);
        assert_eq!(TOK_END_NODE, 2);
        assert_eq!(TOK_PROP, 3);
        assert_eq!(TOK_NOP, 4);
        assert_eq!(TOK_END, 9);
    }
}
