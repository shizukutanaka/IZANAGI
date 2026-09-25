//! Filename-style glob matching — `*`, `?`, `[abc]`, `[a-z]`, `[!a-z]`,
//! and `\` escapes — in the classic two-pointer-with-star-backtrack
//! scan (van Rossum's `fnmatch` shape): `O(n·m)` worst-case but linear
//! in practice, no recursion or heap. Matching is bytewise, so
//! non-ASCII text compares by raw bytes (case-sensitive, like POSIX
//! `fnmatch` without `FNM_CASEFOLD`).
//!
//! ```
//! use izanagi_kit::glob::glob_match;
//!
//! assert!(glob_match("src/**/*.rs", "src/net/http.rs"));
//! assert!(!glob_match("src/*.rs", "src/lib.RS")); // case-sensitive
//! assert!(glob_match("save_[0-9].dat", "save_7.dat"));
//! assert!(!glob_match("save_[0-9].dat", "save_x.dat"));
//! ```

/// Whether `pattern` matches the whole `text` (anchored both ends).
/// Malformed classes degrade gracefully: an unclosed `[` is a literal
/// `[`, `[!` means "not in set", `[^` is accepted as the same negation.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let (mut px, mut tx) = (0usize, 0usize);
    let (mut star_p, mut star_t): (Option<usize>, usize) = (None, 0);
    while tx < t.len() {
        if px < p.len() {
            match p[px] {
                b'?' => {
                    px += 1;
                    tx += 1;
                    continue;
                }
                b'[' => {
                    if let Some((matched, next)) = class_match(p, px, t[tx]) {
                        if matched {
                            px = next;
                            tx += 1;
                            continue;
                        }
                    } else {
                        // Unclosed `[` — treat as literal.
                        if t[tx] == b'[' {
                            px += 1;
                            tx += 1;
                            continue;
                        }
                    }
                }
                b'\\' if px + 1 < p.len() => {
                    if t[tx] == p[px + 1] {
                        px += 2;
                        tx += 1;
                        continue;
                    }
                }
                b'*' => {
                    star_p = Some(px);
                    star_t = tx;
                    px += 1;
                    continue;
                }
                c => {
                    if t[tx] == c {
                        px += 1;
                        tx += 1;
                        continue;
                    }
                }
            }
        }
        // Mismatch: rewind to the last `*` and let it absorb one more.
        if let Some(sp) = star_p {
            px = sp + 1;
            star_t += 1;
            tx = star_t;
            continue;
        }
        return false;
    }
    while px < p.len() && p[px] == b'*' {
        px += 1;
    }
    px == p.len()
}

/// `[` at `p[i]` — parse the class and try `c`. Returns `(matched,
/// index just past `]`)`, or `None` if the class never closes.
fn class_match(p: &[u8], i: usize, c: u8) -> Option<(bool, usize)> {
    let mut j = i + 1;
    let negate = j < p.len() && (p[j] == b'!' || p[j] == b'^');
    if negate {
        j += 1;
    }
    // A `]` first is a literal member of the set.
    let mut matched = false;
    let mut first = true;
    while j < p.len() && (first || p[j] != b']') {
        first = false;
        if j + 2 < p.len() && p[j + 1] == b'-' && p[j + 2] != b']' {
            if p[j] <= c && c <= p[j + 2] {
                matched = true;
            }
            j += 3;
        } else {
            if p[j] == c {
                matched = true;
            }
            j += 1;
        }
    }
    if j >= p.len() {
        return None; // never closed
    }
    Some((matched != negate, j + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_and_question() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "main.RS")); // case sensitive
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
        assert!(glob_match("a?c", "a.c")); // `?` matches any byte, `.` too
        assert!(glob_match("a*bc*z", "aXbcYz"));
        assert!(glob_match("*", "anything at all"));
        assert!(glob_match("", ""));
        assert!(!glob_match("", "x"));
        assert!(glob_match("a**b", "ab"));
    }

    #[test]
    fn char_classes_and_negation() {
        assert!(glob_match("[abc]", "b"));
        assert!(!glob_match("[abc]", "d"));
        assert!(glob_match("[a-z]", "m"));
        assert!(!glob_match("[a-z]", "M"));
        assert!(glob_match("[!a-z]", "M"));
        assert!(!glob_match("[!a-z]", "m"));
        assert!(glob_match("[^0-9]", "q"));
        // Ranges and singletons mix; `]` first is literal.
        assert!(glob_match("[]x]", "]"));
        assert!(glob_match("[a-cx-z]", "y"));
        // `-` at the edge is literal.
        assert!(glob_match("[a-]", "-"));
        assert!(glob_match("file.[ch]", "file.c"));
        assert!(!glob_match("file.[ch]", "file.rs"));
    }

    #[test]
    fn escapes_and_malformed() {
        assert!(glob_match(r"a\*b", "a*b"));
        assert!(!glob_match(r"a\*b", "aXb"));
        assert!(glob_match(r"\?", "?"));
        assert!(glob_match(r"[", "["));
        assert!(glob_match("[", "["));
        assert!(!glob_match("[", "a"));
        // Trailing backslash is a literal backslash.
        assert!(glob_match("x\\", "x\\"));
    }

    #[test]
    fn pathlike_patterns() {
        assert!(glob_match("src/**/*.rs", "src/net/http.rs"));
        assert!(glob_match("src/*.rs", "src/lib.rs"));
        // `*` crosses `/` — this is fnmatch-without-FNM_PATHNAME style.
        assert!(glob_match("*.rs", "src/lib.rs"));
        assert!(glob_match("data/??-*.bin", "data/01-save.bin"));
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(
            glob_match("a*[b-d]?e", "axqwe"),
            glob_match("a*[b-d]?e", "axqwe")
        );
        assert!(glob_match("a*[b-d]*e", "abce"));
    }
}
