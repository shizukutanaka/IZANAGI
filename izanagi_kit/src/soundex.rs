//! Refined Soundex — the NARA phonetic code (Russell–Odell 1918,
//! refined by the US census): consonants map to digits 1–6 by
//! articulation group, runs collapse *unless* a vowel sits between
//! them (H and W do NOT separate), vowels drop, and the result is
//! first-letter + three digits, zero-padded. The first letter's own
//! group counts as the run head — that is why "Pfister" is P236,
//! not P1236.
//!
//! Deterministic, ASCII-only by design (non-letters are skipped);
//! pairs the byte-distance machinery (`editdist`, `jaro`) with a
//! pronunciation key for name matching.
//!
//! ```
//! use izanagi_kit::soundex::{soundex, soundex_eq};
//!
//! assert_eq!(soundex(b"Robert"), *b"R163");
//! assert_eq!(soundex(b"Rupert"), *b"R163");
//! assert_eq!(soundex(b"Ashcraft"), *b"A261"); // H doesn't split the run
//! assert!(soundex_eq(b"Robert", b"Rupert"));
//! ```

/// Map an ASCII letter to its Soundex digit (or `0` for vowel/Y,
/// `9` for the non-separating H/W).
fn code(c: u8) -> u8 {
    match c.to_ascii_uppercase() {
        b'B' | b'F' | b'P' | b'V' => 1,
        b'C' | b'G' | b'J' | b'K' | b'Q' | b'S' | b'X' | b'Z' => 2,
        b'D' | b'T' => 3,
        b'L' => 4,
        b'M' | b'N' => 5,
        b'R' => 6,
        b'H' | b'W' => 9,
        _ => 0,
    }
}

/// Encode `name` as `[first letter, d1, d2, d3]` — the classic
/// 4-character Soundex. An empty or non-letter-led input yields
/// `b"0000"`.
pub fn soundex(name: &[u8]) -> [u8; 4] {
    let first = name
        .iter()
        .find(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase());
    let first = match first {
        Some(f) => f,
        None => return *b"0000",
    };
    let mut out = [first, b'0', b'0', b'0'];
    let mut slot = 1usize;
    // Run head starts at the first letter's own group.
    let mut prev = code(first);
    let mut seen_first = false;
    for &c in name {
        if !seen_first {
            if c.eq_ignore_ascii_case(&first) {
                seen_first = true;
            }
            continue;
        }
        let d = code(c);
        if d == 9 {
            continue; // H/W: skipped, run survives across them
        }
        if d == 0 {
            prev = 0; // vowel/Y: resets the run
            continue;
        }
        if d != prev && slot < 4 {
            out[slot] = b'0' + d;
            slot += 1;
        }
        prev = d;
    }
    out
}

/// Do two names encode to the same Soundex?
pub fn soundex_eq(a: &[u8], b: &[u8]) -> bool {
    soundex(a) == soundex(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_vectors() {
        for &(name, want) in &[
            (b"Robert" as &[u8], *b"R163"),
            (b"Rupert", *b"R163"),
            (b"Rubin", *b"R150"),
            (b"Ashcraft", *b"A261"),
            (b"Ashcroft", *b"A261"),
            (b"Tymczak", *b"T522"),
            (b"Pfister", *b"P236"),
            (b"Gutierrez", *b"G362"),
        ] {
            assert_eq!(soundex(name), want, "{:?}", String::from_utf8_lossy(name));
        }
    }

    #[test]
    fn equivalence() {
        assert!(soundex_eq(b"Robert", b"Rupert"));
        assert!(soundex_eq(b"SMITH", b"smyth"));
        assert!(!soundex_eq(b"Robert", b"Rubin"));
    }

    #[test]
    fn edges() {
        assert_eq!(soundex(b""), *b"0000");
        assert_eq!(soundex(b"123"), *b"0000");
        assert_eq!(soundex(b"A"), *b"A000");
        // Short names zero-pad (first letter's code isn't emitted).
        assert_eq!(soundex(b"Li"), *b"L000");
        assert_eq!(soundex(b"Lloyd"), *b"L300"); // run collapse over first letter
                                                 // Case-insensitive.
        assert_eq!(soundex(b"robert"), soundex(b"ROBERT"));
        // Leading non-letters are skipped for the first letter.
        assert_eq!(soundex(b"  Robert"), *b"R163");
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(soundex(b"Wozniak"), soundex(b"Wozniak"));
    }
}
