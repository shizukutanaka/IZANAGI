//! EDN (Extensible Data Notation, Clojure) parser.
//!
//! Tokenises the file once: counts maps/vectors/sets/lists, `:keyword`s,
//! strings, integers, `true`/`false`/`nil`, `\c` char literals, `#tag`
//! tagged elements, `#_` discards and `;` comments.
//!
//! ```
//! use izanagi_kit::edn::Edn;
//! let src = b"{:name \"app\" :tags [:a :b] :deps #{:x}}\n";
//! assert!(izanagi_kit::edn::detect(src));
//! let e = Edn::parse(src).unwrap();
//! assert_eq!(e.maps, 1);
//! assert_eq!(e.keywords, 6);
//! assert_eq!(e.sets, 1);
//! ```

/// Parsed census of an EDN document.
#[derive(Debug, Clone)]
pub struct Edn {
    /// `{` maps.
    pub maps: usize,
    /// `[` vectors.
    pub vectors: usize,
    /// `#{` sets.
    pub sets: usize,
    /// `(` lists.
    pub lists: usize,
    /// `:keyword` occurrences.
    pub keywords: usize,
    /// Plain symbols.
    pub symbols: usize,
    /// `".."` strings.
    pub strings: usize,
    /// Integer/number-looking tokens.
    pub numbers: usize,
    /// `true`/`false` literals.
    pub booleans: usize,
    /// `nil` literals.
    pub nils: usize,
    /// `\c` char literals.
    pub chars: usize,
    /// `#tag` tagged elements.
    pub tagged: usize,
    /// `#_` discard markers.
    pub discards: usize,
    /// `;` line comments.
    pub comments: usize,
}

fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic()
        || matches!(
            c,
            b'*' | b'+' | b'!' | b'_' | b'?' | b'<' | b'>' | b'=' | b'/' | b'-' | b'.'
        )
}

fn ident_char(c: u8) -> bool {
    ident_start(c) || c.is_ascii_digit() || c == b'#'
}

fn has_keyword(b: &[u8]) -> bool {
    for i in 0..b.len().saturating_sub(1) {
        if b[i] == b':'
            && ident_start(b[i + 1])
            && (i == 0
                || matches!(
                    b[i - 1],
                    b' ' | b'\t' | b'\n' | b'\r' | b'{' | b'[' | b'(' | b','
                ))
        {
            return true;
        }
    }
    false
}

/// Returns `true` when `b` looks like EDN.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match std::str::from_utf8(b) {
        Ok(v) => v.trim_start(),
        Err(_) => return false,
    };
    let sb = s.as_bytes();
    match sb.first() {
        Some(b'{') | Some(b'[') | Some(b'(') | Some(b':') => has_keyword(sb),
        Some(b'#') => {
            matches!(sb.get(1), Some(b'{') | Some(b'_'))
                || sb.get(1).is_some_and(|c| ident_start(*c))
        }
        _ => false,
    }
}

impl Edn {
    /// Counts EDN constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let mut e = Self {
            maps: 0,
            vectors: 0,
            sets: 0,
            lists: 0,
            keywords: 0,
            symbols: 0,
            strings: 0,
            numbers: 0,
            booleans: 0,
            nils: 0,
            chars: 0,
            tagged: 0,
            discards: 0,
            comments: 0,
        };
        let mut i = 0usize;
        while i < b.len() {
            match b[i] {
                b';' => {
                    e.comments += 1;
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                }
                b'"' => {
                    e.strings += 1;
                    i += 1;
                    while i < b.len() && b[i] != b'"' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                }
                b'\\' => {
                    e.chars += 1;
                    i += 1;
                }
                b'#' => match b.get(i + 1) {
                    Some(b'{') => {
                        e.sets += 1;
                        i += 1;
                    }
                    Some(b'_') => {
                        e.discards += 1;
                        i += 1;
                    }
                    Some(c) if ident_start(*c) => e.tagged += 1,
                    _ => {}
                },
                b'{' => e.maps += 1,
                b'[' => e.vectors += 1,
                b'(' => e.lists += 1,
                b':' => {
                    if b.get(i + 1).is_some_and(|c| ident_start(*c)) {
                        e.keywords += 1;
                        while i + 1 < b.len() && ident_char(b[i + 1]) && b[i + 1] != b':' {
                            i += 1;
                        }
                    }
                }
                b' ' | b'\t' | b'\n' | b'\r' | b',' | b'}' | b']' | b')' => {}
                _ => {
                    let start = i;
                    while i + 1 < b.len()
                        && !matches!(
                            b[i + 1],
                            b' ' | b'\t'
                                | b'\n'
                                | b'\r'
                                | b','
                                | b'('
                                | b')'
                                | b'['
                                | b']'
                                | b'{'
                                | b'}'
                                | b'"'
                                | b';'
                        )
                    {
                        i += 1;
                    }
                    let tok = &b[start..=i];
                    if tok == b"nil" {
                        e.nils += 1;
                    } else if tok == b"true" || tok == b"false" {
                        e.booleans += 1;
                    } else if tok.iter().any(|c| c.is_ascii_digit()) {
                        e.numbers += 1;
                    } else {
                        e.symbols += 1;
                    }
                }
            }
            i += 1;
        }
        Some(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"{:name \"app\" :tags [:a :b] :deps #{:x}}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"[:a :b]"));
        assert!(detect(b"{:a 1}"));
        assert!(!detect(b"{\"a\": 1}"));
        assert!(!detect(b"hello world"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let e = Edn::parse(SRC).unwrap();
        assert_eq!(e.maps, 1);
        assert_eq!(e.vectors, 1);
        assert_eq!(e.sets, 1);
        assert_eq!(e.keywords, 6);
        assert_eq!(e.strings, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"{:n nil :ok true :i 42 :c \\x :s 'sym' :t #inst \"2020\" :d #_ :gone} ; tail\n";
        let e = Edn::parse(s).unwrap();
        assert_eq!(e.nils, 1);
        assert_eq!(e.booleans, 1);
        assert!(e.numbers >= 1);
        assert_eq!(e.chars, 1);
        assert_eq!(e.tagged, 1);
        assert_eq!(e.discards, 1);
        assert_eq!(e.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(Edn::parse(b"{\"json\": 1}").is_none());
        assert!(Edn::parse(b"").is_none());
    }
}
