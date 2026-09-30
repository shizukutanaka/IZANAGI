//! Zig ZON (Zig Object Notation, `.zon`) parser.
//!
//! Detects ZON by `.{` anonymous-struct literals plus `.field = value`
//! entries, and counts objects, fields, enum-style `.name` references,
//! strings, integers, `true`/`false`/`null` and `//` comments.
//!
//! ```
//! use izanagi_kit::zon::Zon;
//! let src = b".{\n    .name = \"app\",\n    .version = \"0\x2e1\",\n}\n";
//! assert!(izanagi_kit::zon::detect(src));
//! let z = Zon::parse(src).unwrap();
//! assert_eq!(z.objects, 1);
//! assert_eq!(z.fields, 2);
//! ```

/// Parsed census of a `.zon` file.
#[derive(Debug, Clone)]
pub struct Zon {
    /// `.{` object literals.
    pub objects: usize,
    /// `.name = value` field assignments.
    pub fields: usize,
    /// `.name` references without `=` (enum literals / tuple members).
    pub enum_refs: usize,
    /// `".."` strings.
    pub strings: usize,
    /// Integer-looking tokens.
    pub numbers: usize,
    /// `true`/`false`/`null`/`undefined` literals.
    pub literals: usize,
    /// `//` line comments.
    pub comments: usize,
    /// `..` array/tuple elements and other tokens.
    pub misc: usize,
}

fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn ident_char(c: u8) -> bool {
    ident_start(c) || c.is_ascii_digit() || c == b'-'
}

/// Returns `true` when `b` looks like ZON.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(v) => v,
        Err(_) => return false,
    };
    t.contains(".{") && (t.contains('=') || t.contains(".@"))
}

impl Zon {
    /// Counts ZON constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let mut z = Self {
            objects: 0,
            fields: 0,
            enum_refs: 0,
            strings: 0,
            numbers: 0,
            literals: 0,
            comments: 0,
            misc: 0,
        };
        let mut i = 0usize;
        while i < b.len() {
            match b[i] {
                b'/' if b.get(i + 1) == Some(&b'/') => {
                    z.comments += 1;
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                }
                b'"' => {
                    z.strings += 1;
                    i += 1;
                    while i < b.len() && b[i] != b'"' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                }
                b'.' => match b.get(i + 1) {
                    Some(b'{') => {
                        z.objects += 1;
                        i += 1;
                    }
                    Some(c) if ident_start(*c) || *c == b'@' => {
                        let mut j = i + 1;
                        if b[j] == b'@' {
                            j += 1;
                            if b.get(j) == Some(&b'"') {
                                // .@"name" field — skip the quoted name.
                                j += 1;
                                while j < b.len() && b[j] != b'"' {
                                    j += 1;
                                }
                            }
                        } else {
                            while j < b.len() && ident_char(b[j]) {
                                j += 1;
                            }
                        }
                        // Look ahead past whitespace for `=`.
                        let mut k = j;
                        while k < b.len() && matches!(b[k], b' ' | b'\t' | b'\n' | b'\r') {
                            k += 1;
                        }
                        if b.get(k) == Some(&b'=') {
                            z.fields += 1;
                        } else {
                            z.enum_refs += 1;
                        }
                        i = j.saturating_sub(1).max(i);
                    }
                    _ => {}
                },
                b'0'..=b'9' => {
                    z.numbers += 1;
                    while i + 1 < b.len()
                        && (b[i + 1].is_ascii_digit()
                            || matches!(b[i + 1], b'x' | b'o' | b'b' | b'_' | b'a'..=b'f' | b'A'..=b'F' | b'.'))
                    {
                        i += 1;
                    }
                }
                c if ident_start(c) => {
                    let start = i;
                    while i + 1 < b.len() && ident_char(b[i + 1]) {
                        i += 1;
                    }
                    match &b[start..=i] {
                        b"true" | b"false" | b"null" | b"undefined" => z.literals += 1,
                        _ => z.misc += 1,
                    }
                }
                _ => {}
            }
            i += 1;
        }
        Some(z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b".{\n    .name = \"app\",\n    .version = \"0\x2e1\",\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b".{.a = .{}}"));
        assert!(!detect(b"{.a = 1}"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let z = Zon::parse(SRC).unwrap();
        assert_eq!(z.objects, 1);
        assert_eq!(z.fields, 2);
        assert_eq!(z.strings, 2);
    }

    #[test]
    fn counts_kinds() {
        let s = b".{\n    .mode = .debug,\n    .n = 3,\n    .on = true,\n    .inner = .{ .x = null },\n} // tail\n";
        let z = Zon::parse(s).unwrap();
        assert_eq!(z.objects, 2);
        assert_eq!(z.fields, 5);
        assert_eq!(z.enum_refs, 1);
        assert_eq!(z.numbers, 1);
        assert_eq!(z.literals, 2);
        assert_eq!(z.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(Zon::parse(b"{\"a\": 1}").is_none());
        assert!(Zon::parse(b"").is_none());
    }
}
