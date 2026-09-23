//! De Bruijn sequences via the FKM necklace-prefix algorithm:
//! concatenate the Lyndon words over `k` symbols whose length
//! divides `n`, in lexicographic order — the output is a cyclic
//! string of length `k^n` in which every `n`-symbol word appears
//! exactly once. Pure function of `(k, n)` — same sequence on
//! every machine, so lockstep peers can each regenerate the shared
//! cyclic test vector instead of exchanging it.
//!
//! ```
//! use izanagi_kit::debruijn::{sequence, is_debruijn};
//!
//! let s = sequence(2, 3); // B(2,3) = 00010111
//! assert_eq!(s, vec![0, 0, 0, 1, 0, 1, 1, 1]);
//! assert!(is_debruijn(&s, 2, 3));
//! ```
//!
//! References: Fredericksen–Kessler–Maiorana (1978) FKM algorithm;
//! Ruskey, "Combinatorial Generation" ch. 7 (necklaces & DB cycles).

/// The de Bruijn sequence `B(k, n)` over symbols `0..k`: length
/// `k^n`, cyclic — `seq[i..i+n]` wraps around. Returns an empty
/// vector for `k == 0` or `n == 0` (the empty language has no
/// sequence). `k` is capped at 256 because the alphabet is a byte.
pub fn sequence(k: u32, n: u32) -> Vec<u8> {
    if k == 0 || n == 0 || k > 256 {
        return Vec::new();
    }
    let kk = k as u8;
    let n = n as usize;
    let mut a = vec![0u8; n + 1];
    let mut out = Vec::new();
    let mut i = 1usize;
    loop {
        // Extend a[1..n] to the repetition of its length-i prefix.
        for j in 1..=(n - i) {
            a[i + j] = a[j];
        }
        // a[1..=i] is the next Lyndon word; emit iff length | n.
        if n % i == 0 {
            out.extend_from_slice(&a[1..=i]);
        }
        i = n;
        while a[i] == kk - 1 {
            i -= 1;
            if i == 0 {
                break;
            }
        }
        if i == 0 {
            break;
        }
        a[i] += 1;
    }
    out
}

/// Number of symbols in `B(k, n)` — `k^n`, saturated at `usize`.
pub fn sequence_len(k: u32, n: u32) -> usize {
    (k as usize).saturating_pow(n)
}

/// Verify that `seq` is a valid de Bruijn sequence `B(k, n)`:
/// length `k^n`, all symbols `< k`, and every `n`-symbol window
/// (wrapping) distinct — hence all `k^n` words covered.
pub fn is_debruijn(seq: &[u8], k: u32, n: u32) -> bool {
    if k == 0 || n == 0 || k > 256 {
        return seq.is_empty();
    }
    let n = n as usize;
    if seq.len() != sequence_len(k, n as u32) {
        return false;
    }
    if seq.iter().any(|&b| b as u32 >= k) {
        return false;
    }
    let mut seen: std::collections::BTreeSet<Vec<u8>> = std::collections::BTreeSet::new();
    for start in 0..seq.len() {
        let w: Vec<u8> = (0..n).map(|d| seq[(start + d) % seq.len()]).collect();
        if !seen.insert(w) {
            return false;
        }
    }
    true
}

/// Roll the `n`-wide cyclic window starting at `pos` into a single
/// mixed value — useful as a deterministic index into the sequence
/// for seeded table generation.
pub fn window_hash(seq: &[u8], pos: usize, n: u32, seed: u64) -> u64 {
    use crate::rng::SplitMix64;
    let mut h = SplitMix64::new(seed);
    let m = seq.len().max(1);
    for d in 0..n as usize {
        h = SplitMix64::new(h.next_u64() ^ seq[(pos + d) % m] as u64);
    }
    h.next_u64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn known_sequences() {
        assert_eq!(sequence(2, 1), vec![0, 1]);
        assert_eq!(sequence(2, 2), vec![0, 0, 1, 1]);
        assert_eq!(sequence(2, 3), vec![0, 0, 0, 1, 0, 1, 1, 1]);
        assert_eq!(
            sequence(2, 4),
            // FKM emits Lyndon words in lex order: 0 | 0001 | 0011 | 01 | 0111 | 1
            b"0000100110101111"
                .iter()
                .map(|b| b - b'0')
                .collect::<Vec<_>>()
        );
        assert_eq!(sequence(1, 5), vec![0]);
        assert_eq!(sequence(0, 4), Vec::<u8>::new());
        assert_eq!(sequence(3, 0), Vec::<u8>::new());
    }

    #[test]
    fn all_kmers_covered() {
        for k in 1..=4 {
            for n in 1..=6 {
                let s = sequence(k, n);
                assert_eq!(s.len(), sequence_len(k, n), "k={k} n={n}");
                assert!(is_debruijn(&s, k, n), "k={k} n={n}");
                // Oracle: enumerate all k^n words independently.
                let mut want: BTreeSet<Vec<u8>> = BTreeSet::new();
                let mut w = vec![0u8; n as usize];
                loop {
                    want.insert(w.clone());
                    let mut p = n as usize;
                    loop {
                        if p == 0 {
                            break;
                        }
                        p -= 1;
                        w[p] += 1;
                        if w[p] < k as u8 {
                            break;
                        }
                        w[p] = 0;
                    }
                    if p == 0 && w.iter().all(|&b| b == 0) {
                        break;
                    }
                }
                let mut got: BTreeSet<Vec<u8>> = BTreeSet::new();
                for st in 0..s.len() {
                    got.insert((0..n as usize).map(|d| s[(st + d) % s.len()]).collect());
                }
                assert_eq!(got, want, "k={k} n={n}");
            }
        }
    }

    #[test]
    fn reject_bad_sequences() {
        assert!(!is_debruijn(&[0, 0, 1, 0], 2, 2)); // wrong multiset
        assert!(!is_debruijn(&[0, 1, 1], 2, 2)); // wrong length
        let mut s = sequence(3, 3);
        s[0] = 3; // out-of-alphabet symbol
        assert!(!is_debruijn(&s, 3, 3));
    }

    #[test]
    fn window_hash_is_deterministic() {
        let s = sequence(3, 4);
        assert_eq!(window_hash(&s, 5, 4, 42), window_hash(&s, 5, 4, 42));
        assert_ne!(window_hash(&s, 5, 4, 42), window_hash(&s, 6, 4, 42));
    }
}
