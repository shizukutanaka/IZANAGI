//! ISO-2022-JP stateful decoder (RFC 1468 + the widely used JIS X 0212
//! designation `ESC $ ( D`).
//!
//! Bytes are ASCII unless an escape sequence switches the G0 set: `ESC ( B`
//! ASCII, `ESC ( J` JIS X 0201 roman (same layout here), `ESC $ B` /
//! `ESC $ @` JIS X 0208 (1990/1978), `ESC $ ( D` JIS X 0212. While a 94-set
//! is designated, every pair of `0x21..=0x7E` bytes is one `ku`/`ten`.
//!
//! ```
//! use izanagi_kit::iso2022::Decoder;
//! let mut dec = Decoder::new();
//! let (a, _) = dec.next(b"\x1B$B$3$s$K$A$O\x1B(B", 0).unwrap();
//! assert!(a.is_escape());                       // designation consumed
//! let (ch, w) = dec.next(b"\x1B$B$3$s\x1B(B", 3).unwrap();
//! assert_eq!(w, 2);                             // one 2-byte JIS X 0208 char
//! ```

/// Which character set is currently designated to G0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Set {
    /// `ESC ( B` — plain ASCII (the reset state).
    Ascii,
    /// `ESC ( J` — JIS X 0201 roman.
    JisX0201Roman,
    /// `ESC $ @` — JIS X 0208:1978.
    JisX0208_1978,
    /// `ESC $ B` — JIS X 0208:1983.
    JisX0208_1983,
    /// `ESC $ ( D` — JIS X 0212:1990.
    JisX0212,
}

/// Result of one [`Decoder::next`] call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    /// An escape sequence was consumed and G0 switched to `Set`.
    Escape(Set),
    /// A single-byte character from the current single-byte set.
    Ascii(u8),
    /// A `ku`/`ten` pair from the current two-byte set.
    Char {
        /// Row number (1..=94) in the designated 94-set.
        ku: u8,
        /// Cell number (1..=94) within `ku`.
        ten: u8,
    },
    /// Unrecognised byte or escape; consumed 1 byte.
    Invalid(u8),
    /// Escape or two-byte char cut off at end of input; consumed 0.
    Truncated,
}

impl Out {
    /// `true` when this result was an escape-sequence switch.
    pub fn is_escape(&self) -> bool {
        matches!(self, Out::Escape(_))
    }
}

/// Stateful ISO-2022-JP decoder — keep one per stream.
#[derive(Clone, Copy, Debug)]
pub struct Decoder {
    /// Current G0 designation.
    pub set: Set,
}

impl Decoder {
    /// Fresh decoder in the ASCII state (the RFC 1468 initial state).
    pub fn new() -> Self {
        Decoder { set: Set::Ascii }
    }

    /// Decode one unit at `d[i]`, updating `self.set` on escape sequences.
    /// Returns `None` only when `i >= d.len()`.
    pub fn next(&mut self, d: &[u8], i: usize) -> Option<(Out, usize)> {
        let b = *d.get(i)?;
        if b == 0x1b {
            return Some(self.escape(d, i));
        }
        match self.set {
            Set::Ascii | Set::JisX0201Roman => {
                if b <= 0x7f {
                    Some((Out::Ascii(b), 1))
                } else {
                    Some((Out::Invalid(b), 1))
                }
            }
            Set::JisX0208_1978 | Set::JisX0208_1983 | Set::JisX0212 => match d.get(i + 1) {
                Some(&c) if (0x21..=0x7e).contains(&b) && (0x21..=0x7e).contains(&c) => Some((
                    Out::Char {
                        ku: b - 0x20,
                        ten: c - 0x20,
                    },
                    2,
                )),
                _ => Some((Out::Truncated, 0)),
            },
        }
    }

    fn escape(&mut self, d: &[u8], i: usize) -> (Out, usize) {
        let rest = &d[i + 1..];
        let (set, len) = if rest.starts_with(b"(B") {
            (Set::Ascii, 3)
        } else if rest.starts_with(b"(J") {
            (Set::JisX0201Roman, 3)
        } else if rest.starts_with(b"$@") {
            (Set::JisX0208_1978, 3)
        } else if rest.starts_with(b"$B") {
            (Set::JisX0208_1983, 3)
        } else if rest.starts_with(b"$(D") {
            (Set::JisX0212, 4)
        } else {
            // A proper prefix of a supported designation may complete
            // in a later buffer — report it as truncated, not invalid.
            let seqs: [&[u8]; 5] = [b"(B", b"(J", b"$@", b"$B", b"$(D"];
            if rest.is_empty() || seqs.iter().any(|s| s.starts_with(rest)) {
                return (Out::Truncated, 0);
            }
            return (Out::Invalid(0x1b), 1);
        };
        self.set = set;
        (Out::Escape(set), len)
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Decode a whole buffer with a fresh [`Decoder`]; `Some(char_count)` when
/// every byte is consumed cleanly (escapes don't count), `None` otherwise.
pub fn count(d: &[u8]) -> Option<usize> {
    let (mut dec, mut i, mut n) = (Decoder::new(), 0usize, 0usize);
    while i < d.len() {
        match dec.next(d, i)? {
            (Out::Escape(_), w) => i += w,
            (Out::Invalid(_) | Out::Truncated, _) => return None,
            (_, w) => {
                i += w;
                n += 1;
            }
        }
    }
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_switch_sets() {
        let mut dec = Decoder::new();
        assert_eq!(dec.set, Set::Ascii);
        match dec.next(b"\x1B$B", 0) {
            Some((Out::Escape(Set::JisX0208_1983), 3)) => {}
            _ => panic!("escape"),
        }
        assert_eq!(dec.set, Set::JisX0208_1983);
        dec.next(b"\x1B(B", 0);
        assert_eq!(dec.set, Set::Ascii);
        dec.next(b"\x1B$@", 0);
        assert_eq!(dec.set, Set::JisX0208_1978);
        dec.next(b"\x1B(J", 0);
        assert_eq!(dec.set, Set::JisX0201Roman);
        dec.next(b"\x1B$(D", 0);
        assert_eq!(dec.set, Set::JisX0212);
    }

    #[test]
    fn double_byte_in_94_set() {
        let mut dec = Decoder::new();
        dec.next(b"\x1B$B", 0);
        assert_eq!(dec.next(b"$3", 0), Some((Out::Char { ku: 4, ten: 19 }, 2)));
        // High bytes are invalid in a 94-set; short tail truncates.
        assert_eq!(dec.next(b"\x24\xa4", 0), Some((Out::Truncated, 0)));
        assert_eq!(dec.next(b"\x24", 0), Some((Out::Truncated, 0)));
    }

    #[test]
    fn invalid_and_truncated() {
        let mut dec = Decoder::new();
        assert_eq!(dec.next(b"\x1B", 0), Some((Out::Truncated, 0)));
        // Incomplete designations are truncations, not errors.
        assert_eq!(dec.next(b"\x1B$", 0), Some((Out::Truncated, 0)));
        assert_eq!(dec.next(b"\x1B(", 0), Some((Out::Truncated, 0)));
        assert_eq!(dec.next(b"\x1B$(", 0), Some((Out::Truncated, 0)));
        assert_eq!(dec.next(b"\x1B(Z", 0), Some((Out::Invalid(0x1b), 1)));
        assert_eq!(dec.next(b"\x80", 0), Some((Out::Invalid(0x80), 1)));
        assert_eq!(dec.next(b"", 0), None);
        assert!(!Out::Ascii(b'a').is_escape());
        assert!(Out::Escape(Set::Ascii).is_escape());
    }

    #[test]
    fn count_mixed() {
        let d = b"\x1B$B$3$s$K$A$O\x1B(Bok";
        assert_eq!(count(d), Some(7)); // 5 kana + "ok"
        assert_eq!(count(b"\x1B(Z"), None);
        assert_eq!(count(b"\x1B$B\x24"), None);
    }
}
