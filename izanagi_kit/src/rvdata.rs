//! RPG Maker `.rxdata`/`.rvdata`/`.rvdata2` (Ruby Marshal 4.8) parser.
//!
//! `\x04\x08` header then one serialized object. Recursively walks the
//! Marshal token stream counting objects/arrays/hashes/strings/symbols/
//! fixnums and tracking max depth; `parse` rejects malformed streams.
//!
//! ```
//! use izanagi_kit::rvdata::Rvdata;
//! // Marshal dump: { "a" => [1, 2] }
//! let src = &[4u8, 8, b'{', 6, b'I', b'"', 6, b'a', 6, b':', 6, b'E', b'T', b'[', 7, b'i', 6, b'i', 7];
//! assert!(izanagi_kit::rvdata::detect(src));
//! let m = Rvdata::parse(src).unwrap();
//! assert_eq!(m.hashes, 1);
//! assert_eq!(m.arrays, 1);
//! assert_eq!(m.ints, 2);
//! ```

/// Parsed census of a Marshal stream.
#[derive(Debug, Clone)]
pub struct Rvdata {
    /// Marshal major version byte (`0x04`).
    pub major: u8,
    /// Marshal minor version byte (`0x08`).
    pub minor: u8,
    /// `o` object instances.
    pub objects: usize,
    /// `S` struct instances.
    pub structs: usize,
    /// `[` arrays.
    pub arrays: usize,
    /// `{`/`}` hashes (incl. default-value variant).
    pub hashes: usize,
    /// `"`/`I"` strings.
    pub strings: usize,
    /// `:`/`;` symbols (new + references).
    pub symbols: usize,
    /// `i` fixnums.
    pub ints: usize,
    /// `f` floats.
    pub floats: usize,
    /// `l` bignums.
    pub bignums: usize,
    /// `/` regexps.
    pub regexps: usize,
    /// `u`/`U` user-defined values.
    pub user_defs: usize,
    /// `e`/`d`/`C`/`m`/`M`/`c` extended/wrapped values.
    pub wrappers: usize,
    /// `@`/`;` object & symbol back-references.
    pub refs: usize,
    /// `0`/`T`/`F`/`I`/`p` nils/booleans/ivar-wrappers/procs.
    pub misc: usize,
    /// Deepest container nesting.
    pub max_depth: usize,
    /// Total bytes consumed by the marshal payload.
    pub consumed: usize,
}

const TAGS: &[u8] = b"0TFi:;@[]{}oSfeucUCmpMdlI/\"";

/// Returns `true` when `b` starts with a Marshal 4.8 dump.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 3 && b[0] == 4 && b[1] == 8 && TAGS.contains(&b[2])
}

fn int(b: &[u8], p: &mut usize) -> Option<i64> {
    let c = *b.get(*p)? as i8;
    *p += 1;
    match c {
        0 => Some(0),
        1..=4 => {
            let n = c as usize;
            let s = b.get(*p..*p + n)?;
            *p += n;
            let mut v = 0i64;
            for (i, &x) in s.iter().enumerate() {
                v |= i64::from(x) << (8 * i);
            }
            Some(v)
        }
        -4..=-1 => {
            let n = (-c) as usize;
            let s = b.get(*p..*p + n)?;
            *p += n;
            let mut v = -1i64;
            for (i, &x) in s.iter().enumerate() {
                v = v & !(0xffi64 << (8 * i)) | (i64::from(x) << (8 * i));
            }
            Some(v)
        }
        5..=127 => Some(i64::from(c - 5)),
        _ => Some(i64::from(c + 5)),
    }
}

fn item(b: &[u8], p: &mut usize, m: &mut Rvdata, depth: usize) -> bool {
    let Some(&tag) = b.get(*p) else { return false };
    *p += 1;
    m.max_depth = m.max_depth.max(depth);
    let seq = |n: i64, m: &mut Rvdata, p: &mut usize, arity: usize| -> bool {
        (0..n.saturating_mul(arity as i64)).all(|_| item(b, p, m, depth + 1))
    };
    match tag {
        b'0' | b'T' | b'F' => {
            m.misc += 1;
            true
        }
        b'i' => {
            m.ints += 1;
            int(b, p).is_some()
        }
        b':' => {
            m.symbols += 1;
            match int(b, p) {
                Some(n) if n >= 0 && (*p + n as usize) <= b.len() => {
                    *p += n as usize;
                    true
                }
                _ => false,
            }
        }
        b';' => {
            m.refs += 1;
            int(b, p).is_some()
        }
        b'@' => {
            m.refs += 1;
            int(b, p).is_some()
        }
        b'[' => {
            m.arrays += 1;
            match int(b, p) {
                Some(n) if n >= 0 => seq(n, m, p, 1),
                _ => false,
            }
        }
        b'{' | b'}' => {
            m.hashes += 1;
            match int(b, p) {
                Some(n) if n >= 0 => {
                    if !seq(n, m, p, 2) {
                        return false;
                    }
                    tag != b'}' || item(b, p, m, depth + 1) // '}' has default value
                }
                _ => false,
            }
        }
        b'o' => {
            m.objects += 1;
            if !item(b, p, m, depth + 1) {
                return false;
            }
            match int(b, p) {
                Some(n) if n >= 0 => seq(n, m, p, 2),
                _ => false,
            }
        }
        b'S' => {
            m.structs += 1;
            if !item(b, p, m, depth + 1) {
                return false;
            }
            match int(b, p) {
                Some(n) if n >= 0 => seq(n, m, p, 2),
                _ => false,
            }
        }
        b'f' => {
            m.floats += 1;
            match int(b, p) {
                Some(n) if n >= 0 && (*p + n as usize) <= b.len() => {
                    *p += n as usize;
                    true
                }
                _ => false,
            }
        }
        b'"' => {
            m.strings += 1;
            match int(b, p) {
                Some(n) if n >= 0 && (*p + n as usize) <= b.len() => {
                    *p += n as usize;
                    true
                }
                _ => false,
            }
        }
        b'I' => {
            m.misc += 1;
            if !item(b, p, m, depth + 1) {
                return false;
            }
            match int(b, p) {
                Some(n) if n >= 0 => seq(n, m, p, 2),
                _ => false,
            }
        }
        b'e' | b'm' | b'M' => {
            m.wrappers += 1;
            item(b, p, m, depth + 1)
        }
        b'c' | b'C' | b'd' => {
            m.wrappers += 1;
            item(b, p, m, depth + 1) && item(b, p, m, depth + 1)
        }
        b'p' => {
            m.misc += 1;
            true
        }
        b'u' | b'U' => {
            m.user_defs += 1;
            if !item(b, p, m, depth + 1) {
                return false;
            }
            if tag == b'u' {
                match int(b, p) {
                    Some(n) if n >= 0 && (*p + n as usize) <= b.len() => {
                        *p += n as usize;
                        true
                    }
                    _ => false,
                }
            } else {
                item(b, p, m, depth + 1)
            }
        }
        b'/' => {
            m.regexps += 1;
            match int(b, p) {
                Some(n) if n >= 0 && (*p + n as usize + 1) <= b.len() => {
                    *p += n as usize + 1;
                    true
                }
                _ => false,
            }
        }
        b'l' => {
            m.bignums += 1;
            let Some(&sign) = b.get(*p) else { return false };
            *p += 1;
            if sign != b'+' && sign != b'-' {
                return false;
            }
            match int(b, p) {
                Some(n) if n >= 0 => {
                    let bytes = n as usize * 2;
                    if *p + bytes <= b.len() {
                        *p += bytes;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        _ => false,
    }
}

impl Rvdata {
    /// Parses a Marshal stream; `None` on missing header or truncated data.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if b.len() < 3 || b[0] != 4 || b[1] != 8 {
            return None;
        }
        let mut m = Self {
            major: b[0],
            minor: b[1],
            objects: 0,
            structs: 0,
            arrays: 0,
            hashes: 0,
            strings: 0,
            symbols: 0,
            ints: 0,
            floats: 0,
            bignums: 0,
            regexps: 0,
            user_defs: 0,
            wrappers: 0,
            refs: 0,
            misc: 0,
            max_depth: 0,
            consumed: 0,
        };
        let mut p = 2usize;
        if !item(b, &mut p, &mut m, 0) {
            return None;
        }
        m.consumed = p;
        Some(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_header() {
        assert!(detect(&[4, 8, b'[']));
        assert!(detect(&[4, 8, b'o']));
        assert!(!detect(&[4, 8, 9, 1]));
        assert!(!detect(b"PK\x03\x04"));
    }

    #[test]
    fn counts_array() {
        // [1, [2, 3]]
        let src = &[4u8, 8, b'[', 7, b'i', 6, b'[', 7, b'i', 7, b'i', 8];
        let m = Rvdata::parse(src).unwrap();
        assert_eq!(m.arrays, 2);
        assert_eq!(m.ints, 3);
        assert_eq!(m.max_depth, 2);
    }

    #[test]
    fn rejects_truncated() {
        assert!(Rvdata::parse(&[4, 8, b'[', 9, b'i']).is_none());
    }
}
