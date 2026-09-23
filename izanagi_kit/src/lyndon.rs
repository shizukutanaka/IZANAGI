//! Lyndon words — Duval's factorization (1983) and Booth's least
//! rotation (1980). A Lyndon word is strictly smaller than every
//! one of its proper rotations; Duval's algorithm splits any string
//! into Lyndon factors in non-increasing order, in `O(n)` time and
//! `O(1)` extra space. Both are pure functions of the byte string —
//! useful as the canonical-form step for cyclic structures (necklace
//! normalization, board-state canonicalization under rotation).
//!
//! ```
//! use izanagi_kit::lyndon::{is_lyndon, lyndon_factorize, min_rotation};
//! assert!(is_lyndon(b"aab"));
//! assert!(!is_lyndon(b"aba"));
//! assert_eq!(lyndon_factorize(b"babaa"), vec![(0, 1), (1, 2), (3, 1), (4, 1)]);
//! assert_eq!(min_rotation(b"bbab"), 2); // "abbb" is the least rotation
//! ```

/// True when `s` is a Lyndon word — strictly smaller than every
/// proper suffix (equivalent to: strictly smaller than every
/// proper rotation and aperiodic). The empty string is not Lyndon.
pub fn is_lyndon(s: &[u8]) -> bool {
    !s.is_empty() && (1..s.len()).all(|k| s < &s[k..])
}

/// Factorize `s` into `(start, len)` spans of Lyndon words with
/// `factor[0] ≥ factor[1] ≥ …` — Duval's `O(n)` algorithm.
pub fn lyndon_factorize(s: &[u8]) -> Vec<(usize, usize)> {
    let n = s.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        let mut k = i;
        while j < n && s[k] <= s[j] {
            if s[k] < s[j] {
                k = i;
            } else {
                k += 1;
            }
            j += 1;
        }
        // s[i..j) = w^p·w′ with |w| = j−k — emit exactly the `p`
        // complete copies (`i ≤ k` ⟺ floor((j−i)/flen) repeats).
        let flen = j - k;
        while i <= k {
            out.push((i, flen));
            i += flen;
        }
    }
    out
}

/// Booth's least-rotation index: the `i` minimizing the rotation
/// `s[i..] ++ s[..i]`. Ties go to the smallest such `i` — canonical.
/// `s` empty → `0`.
pub fn min_rotation(s: &[u8]) -> usize {
    let n = s.len();
    if n <= 1 {
        return 0;
    }
    // Doubled-string Booth scan with failure reset on the pattern.
    let mut i = 0usize;
    let mut j = 1usize;
    let mut k = 0usize;
    while i + k < 2 * n && j + k < 2 * n {
        let a = s[(i + k) % n];
        let b = s[(j + k) % n];
        if a == b {
            k += 1;
            continue;
        }
        if a > b {
            i += k + 1;
            if i <= j {
                i = j + 1;
            }
        } else {
            j += k + 1;
            if j <= i {
                j = i + 1;
            }
        }
        k = 0;
    }
    i.min(j)
}

#[cfg(test)]
fn rot(s: &[u8], k: usize) -> Vec<u8> {
    let n = s.len();
    if n == 0 {
        return Vec::new();
    }
    let k = k % n;
    let mut v = Vec::with_capacity(n);
    v.extend_from_slice(&s[k..]);
    v.extend_from_slice(&s[..k]);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn is_lyndon_naive(s: &[u8]) -> bool {
        if s.is_empty() {
            return false;
        }
        for k in 1..s.len() {
            if rot(s, k) <= s.to_vec() {
                return false;
            }
        }
        true
    }

    #[test]
    fn known_cases() {
        assert!(is_lyndon(b"a"));
        assert!(is_lyndon(b"ab"));
        assert!(is_lyndon(b"aab"));
        assert!(!is_lyndon(b"ba"));
        assert!(!is_lyndon(b"abab"));
        assert!(!is_lyndon(b"aa"));
    }

    #[test]
    fn is_lyndon_matches_brute_force() {
        let mut rng = SplitMix64::new(17);
        for _ in 0..300 {
            let n = rng.below(12) as usize;
            let s: Vec<u8> = (0..n).map(|_| rng.below(3) as u8 + b'a').collect();
            assert_eq!(is_lyndon(&s), is_lyndon_naive(&s), "{s:?}");
        }
    }

    #[test]
    fn factorization_is_valid() {
        let mut rng = SplitMix64::new(23);
        for _ in 0..300 {
            let n = rng.below(24) as usize;
            let s: Vec<u8> = (0..n).map(|_| rng.below(3) as u8 + b'a').collect();
            let f = lyndon_factorize(&s);
            // Covers the whole string.
            let mut pos = 0;
            for &(st, len) in &f {
                assert_eq!(st, pos);
                assert!(is_lyndon_naive(&s[st..st + len]), "factor not Lyndon");
                pos += len;
            }
            assert_eq!(pos, n);
            // Non-increasing factor order.
            for w in f.windows(2) {
                let a = &s[w[0].0..w[0].0 + w[0].1];
                let b = &s[w[1].0..w[1].0 + w[1].1];
                assert!(a >= b, "factors not non-increasing");
            }
        }
    }

    #[test]
    fn min_rotation_matches_enumeration() {
        let mut rng = SplitMix64::new(31);
        for _ in 0..200 {
            let n = rng.below(14) as usize + 1;
            let s: Vec<u8> = (0..n).map(|_| rng.below(3) as u8 + b'a').collect();
            // Canonical least rotation = smallest index reaching the
            // minimal rotated string.
            let mut best = 0;
            let mut best_rot: Option<Vec<u8>> = None;
            for k in 0..n {
                let r = rot(&s, k);
                if best_rot.as_ref().map_or(true, |b| r < *b) {
                    best_rot = Some(r);
                    best = k;
                }
            }
            assert_eq!(min_rotation(&s), best, "{s:?}");
        }
    }

    #[test]
    fn period_lemma_property() {
        // s = "aaaa...a" (single Lyndon factor of length 1 × n).
        let f = lyndon_factorize(b"aaaaaa");
        assert_eq!(f, vec![(0, 1), (1, 1), (2, 1), (3, 1), (4, 1), (5, 1)]);
    }
}
