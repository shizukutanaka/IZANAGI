//! Pickle — Python's serialization protocol stream: a sequence of
//! opcodes (`MARK` `(`, `STOP` `.`, `INT` `I` + decimal line,
//! `BININT` `J` + u32LE, `PROTO` `\x80` + u8, `FRAME` `\x95` +
//! u64LE, `GLOBAL` `c` + `mod\nname\n`, `STACK_GLOBAL` `\x93`,
//! `MEMO`/`BINPUT`/`GET` …) terminated by `STOP`.
//!
//! ```
//! use izanagi_kit::pickle::{ops, Code};
//!
//! // proto-2 frame: \x80\x02 then I42\n then .
//! let d = b"\x80\x02I42\n.";
//! let v: Vec<_> = ops(d).unwrap();
//! assert_eq!(v[0].code, Code::Proto(2));
//! assert_eq!(v[1].code, Code::Int(42));
//! assert_eq!(v[2].code, Code::Stop);
//! ```

/// One decoded opcode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    /// Opcode kind.
    pub code: Code,
    /// Byte offset of the opcode byte.
    pub at: usize,
    /// Offset just past the opcode and its payload.
    pub end: usize,
}

/// Opcode kind with its payload when fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Code {
    /// `\x80` — protocol version.
    Proto(u8),
    /// `\x95` — 8-byte frame length (skipped; the stream continues).
    Frame(u64),
    /// `(` — push a MARK.
    Mark,
    /// `.` — stream end.
    Stop,
    /// `0` — pop.
    Pop,
    /// `2` — duplicate top.
    Dup,
    /// `I` + decimal text line.
    Int(i64),
    /// `L` + decimal text line (kept as text — arbitrary precision).
    Long(String),
    /// `J` + i32LE.
    BinInt(i32),
    /// `K` + u8.
    BinInt1(u8),
    /// `M` + u16LE.
    BinInt2(u16),
    /// `S'`/`V` + quoted/text line (string kept verbatim).
    Str(String),
    /// `U` + u8 len + bytes; `\x8c` BINUNICODE + u32 len.
    Bytes(usize, usize),
    /// `N`.
    None,
    /// `\x88` / `\x89`.
    Bool(bool),
    /// `c` + `mod\nname\n`.
    Global(String),
    /// `\x93` STACK_GLOBAL.
    StackGlobal,
    /// `R` — reduce (call).
    Reduce,
    /// `b` — BUILD.
    Build,
    /// `t` — TUPLE.
    Tuple,
    /// `]` — EMPTY_LIST.
    EmptyList,
    /// `}` — EMPTY_DICT.
    EmptyDict,
    /// `\x8f` — EMPTY_SET.
    EmptySet,
    /// `\x85` — TUPLE1, `\x86` TUPLE2, `\x87` TUPLE3.
    TupleN(u8),
    /// `a` — append; `e` — appends.
    Append(bool),
    /// `p`+line PUT / `q`+u8 BINPUT / `r`+u32 LONG_BINPUT.
    Put(u32),
    /// `g`+line GET / `h`+u8 BINGET / `j`+u32 LONG_BINGET.
    Get(u32),
    /// Any other opcode byte (payload unknown — kept raw).
    Other(u8),
}

fn line_end(d: &[u8], at: usize) -> Option<usize> {
    d.get(at..)?
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| at + p)
}

fn le32(d: &[u8], i: usize) -> Option<u32> {
    let mut v = 0u32;
    for k in 0..4 {
        v |= u32::from(*d.get(i + k)?) << (8 * k);
    }
    Some(v)
}
fn le64(d: &[u8], i: usize) -> Option<u64> {
    let mut v = 0u64;
    for k in 0..8 {
        v |= u64::from(*d.get(i + k)?) << (8 * k);
    }
    Some(v)
}

/// Walk the opcode stream. Returns the op list (`STOP` included)
/// or `None` on truncation.
pub fn ops(d: &[u8]) -> Option<Vec<Op>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(&b) = d.get(at) {
        let start = at;
        at += 1;
        let code = match b {
            0x80 => {
                let v = *d.get(at)?;
                at += 1;
                Code::Proto(v)
            }
            0x95 => {
                let n = le64(d, at)?;
                at += 8;
                Code::Frame(n)
            }
            b'(' => Code::Mark,
            b'.' => Code::Stop,
            b'0' => Code::Pop,
            b'2' => Code::Dup,
            b'I' => {
                let e = line_end(d, at)?;
                let s = std::str::from_utf8(d.get(at..e)?).ok()?;
                at = e + 1;
                Code::Int(s.trim().parse().ok()?)
            }
            b'L' => {
                let e = line_end(d, at)?;
                let s = std::str::from_utf8(d.get(at..e)?).ok()?;
                at = e + 1;
                Code::Long(s.trim_end_matches('L').trim().into())
            }
            b'J' => {
                let v = le32(d, at)? as i32;
                at += 4;
                Code::BinInt(v)
            }
            b'K' => {
                let v = *d.get(at)?;
                at += 1;
                Code::BinInt1(v)
            }
            b'M' => {
                let v = u16::from(*d.get(at)?) | (u16::from(*d.get(at + 1)?) << 8);
                at += 2;
                Code::BinInt2(v)
            }
            b'S' | b'V' => {
                // 'S' is quoted (S'x'\n); 'V' is unicode text line.
                let e = line_end(d, at)?;
                let raw = std::str::from_utf8(d.get(at..e)?).ok()?;
                at = e + 1;
                let s = if b == b'S' {
                    raw.trim_matches('\'').into()
                } else {
                    raw.into()
                };
                Code::Str(s)
            }
            b'U' => {
                let n = usize::from(*d.get(at)?);
                at += 1;
                let s = at;
                at = at.checked_add(n)?;
                if at > d.len() {
                    return None;
                }
                Code::Bytes(s, n)
            }
            0x8C => {
                let n = usize::try_from(le32(d, at)?).ok()?;
                at += 4;
                let s = at;
                at = at.checked_add(n)?;
                if at > d.len() {
                    return None;
                }
                Code::Bytes(s, n)
            }
            b'N' => Code::None,
            0x88 => Code::Bool(true),
            0x89 => Code::Bool(false),
            b'c' => {
                let e = line_end(d, at)?;
                let module = std::str::from_utf8(d.get(at..e)?).ok()?;
                at = e + 1;
                let e2 = line_end(d, at)?;
                let name = std::str::from_utf8(d.get(at..e2)?).ok()?;
                at = e2 + 1;
                Code::Global(format!("{module}.{name}"))
            }
            0x93 => Code::StackGlobal,
            b'R' => Code::Reduce,
            b'b' => Code::Build,
            b't' => Code::Tuple,
            b']' => Code::EmptyList,
            b'}' => Code::EmptyDict,
            0x8F => Code::EmptySet,
            0x85 => Code::TupleN(1),
            0x86 => Code::TupleN(2),
            0x87 => Code::TupleN(3),
            b'a' => Code::Append(false),
            b'e' => Code::Append(true),
            b'p' => {
                let e = line_end(d, at)?;
                let s = std::str::from_utf8(d.get(at..e)?).ok()?;
                at = e + 1;
                Code::Put(s.trim().parse().ok()?)
            }
            b'q' => {
                let v = u32::from(*d.get(at)?);
                at += 1;
                Code::Put(v)
            }
            b'r' => {
                let v = le32(d, at)?;
                at += 4;
                Code::Put(v)
            }
            b'g' => {
                let e = line_end(d, at)?;
                let s = std::str::from_utf8(d.get(at..e)?).ok()?;
                at = e + 1;
                Code::Get(s.trim().parse().ok()?)
            }
            b'h' => {
                let v = u32::from(*d.get(at)?);
                at += 1;
                Code::Get(v)
            }
            b'j' => {
                let v = le32(d, at)?;
                at += 4;
                Code::Get(v)
            }
            other => Code::Other(other),
        };
        out.push(Op {
            code,
            at: start,
            end: at,
        });
        if out.last().map(|o| o.code == Code::Stop).unwrap_or(false) {
            break;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcodes() {
        // \x80\x02 protocol 2, then memoized int + stop
        let d = b"\x80\x02K*q\x01.";
        let v = ops(d).unwrap();
        assert_eq!(v[0].code, Code::Proto(2));
        assert_eq!(v[1].code, Code::BinInt1(42));
        assert_eq!(v[2].code, Code::Put(1));
        assert_eq!(v[3].code, Code::Stop);
        assert_eq!(v[3].at, 6);
    }

    #[test]
    fn strings_and_globals() {
        let d = b"S'abc'\ncposix\nsystem\nN.";
        let v = ops(d).unwrap();
        assert_eq!(v[0].code, Code::Str(String::from("abc")));
        assert_eq!(v[1].code, Code::Global(String::from("posix.system")));
        assert_eq!(v[2].code, Code::None);
        assert_eq!(v[3].code, Code::Stop);
    }

    #[test]
    fn frame_and_binunicode() {
        let mut d = vec![0x80, 0x04, 0x95];
        d.extend_from_slice(&u64::to_le_bytes(9));
        d.push(0x8C);
        d.extend_from_slice(&u32::to_le_bytes(2));
        d.extend_from_slice(b"hi");
        d.push(b'.');
        let v = ops(&d).unwrap();
        assert_eq!(v[1].code, Code::Frame(9));
        assert_eq!(v[2].code, Code::Bytes(16, 2));
        assert_eq!(v[3].code, Code::Stop);
    }

    #[test]
    fn rejects() {
        assert_eq!(ops(b""), Some(Vec::new()));
        assert!(ops(b"\x80").is_none()); // truncated proto
        assert!(ops(b"U\x05ab").is_none()); // short string
        assert!(ops(b"Inan").is_none()); // unterminated int line
    }
}
