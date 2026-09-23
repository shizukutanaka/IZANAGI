//! Winnowing document fingerprints (Schleimer, Wilkerson &
//! Aiken, SIGMOD 2003): hash every `k`-gram, then from each
//! window of `w` consecutive hashes select the *rightmost*
//! minimum. The emitted `(position, hash)` pairs are a
//! deterministic sparse fingerprint — two documents sharing a
//! long run share the corresponding fingerprints, which is why
//! plagiarism detectors and code-similarity tools use it.
//!
//! The guarantee worth stating precisely: for *every* window the
//! fingerprint contains that window's selected minimum — so any
//! copied run of `≥ k + w − 1` bytes must surface a shared hash.
//!
//! ```
//! use izanagi_kit::winnow::{kgram_hashes, winnow, fingerprint};
//! let h = kgram_hashes(b"abcdefg", 2, 0);
//! assert_eq!(h.len(), 6);
//! let fp = winnow(&h, 3);
//! assert!(!fp.is_empty());
//! // whole-document convenience
//! assert_eq!(fingerprint(b"abcdefg", 2, 3, 0), fp);
//! ```
use std::collections::BTreeSet;

/// Hash each `k`-byte window of `doc` — `doc.len() − k + 1`
/// hashes (`0` when `doc` is shorter than `k`). The hash is a
/// rolling seeded mix: cheap, order-sensitive, deterministic.
pub fn kgram_hashes(doc: &[u8], k: usize, seed: u64) -> Vec<u64> {
    if k == 0 || doc.len() < k {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(doc.len() - k + 1);
    for i in 0..=doc.len() - k {
        let mut h = seed;
        for &b in &doc[i..i + k] {
            h = (h ^ b as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        }
        out.push(h ^ (h >> 31));
    }
    out
}

/// Winnow `hashes` with window `w`: every window contributes its
/// rightmost minimum; consecutive windows sharing the same
/// minimum index emit it once.
///
/// Schleimer's `O(n)` walk: `r` tracks the current window's
/// selected index. When `r` slides out of the window it is
/// recomputed from scratch; otherwise only the new tail hash can
/// beat it — `≤` comparison makes ties resolve rightward, which
/// is exactly what lets adjacent windows coalesce.
pub fn winnow(hashes: &[u64], w: usize) -> Vec<(usize, u64)> {
    if w == 0 || hashes.len() < w {
        return Vec::new();
    }
    let mut out: Vec<(usize, u64)> = Vec::new();
    let mut r = !0usize; // no current minimum yet
    let mut last: Option<usize> = None;
    for i in 0..=hashes.len() - w {
        if r < i || r >= i + w {
            // no usable minimum (first window, or it slid out)
            // — rescan, taking the rightmost minimum
            r = i;
            for j in i + 1..i + w {
                if hashes[j] <= hashes[r] {
                    r = j;
                }
            }
        } else if hashes[i + w - 1] <= hashes[r] {
            r = i + w - 1;
        }
        if last != Some(r) {
            out.push((r, hashes[r]));
            last = Some(r);
        }
    }
    out
}

/// `kgram_hashes` + `winnow` in one call. Documents shorter
/// than `k` get a single whole-document hash so they still
/// fingerprint.
pub fn fingerprint(doc: &[u8], k: usize, w: usize, seed: u64) -> Vec<(usize, u64)> {
    if k == 0 || w == 0 {
        return Vec::new();
    }
    if doc.len() < k {
        // whole document becomes one gram
        let mut h = seed;
        for &b in doc {
            h = (h ^ b as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        }
        return vec![(0, h ^ (h >> 31))];
    }
    let h = kgram_hashes(doc, k, seed);
    if h.len() < w {
        // fewer grams than a window: emit the global min once
        let mut r = 0usize;
        for j in 1..h.len() {
            if h[j] <= h[r] {
                r = j;
            }
        }
        return vec![(r, h[r])];
    }
    winnow(&h, w)
}

/// Similarity estimate between two fingerprints: the Jaccard
/// index of the selected hash *values* (positions stripped).
pub fn similarity(a: &[(usize, u64)], b: &[(usize, u64)]) -> (u64, u64) {
    let sa: BTreeSet<u64> = a.iter().map(|&(_, h)| h).collect();
    let sb: BTreeSet<u64> = b.iter().map(|&(_, h)| h).collect();
    let inter = sa.intersection(&sb).count() as u64;
    let union = sa.union(&sb).count() as u64;
    (inter, union)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Definition oracle: for every window compute the rightmost
    /// argmin by brute scan; the emitted sequence must equal the
    /// deduplicated run of per-window selections.
    fn naive_winnow(hashes: &[u64], w: usize) -> Vec<(usize, u64)> {
        if w == 0 || hashes.len() < w {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut last = None;
        for i in 0..=hashes.len() - w {
            let mut r = i;
            for j in i + 1..i + w {
                if hashes[j] <= hashes[r] {
                    r = j;
                }
            }
            if last != Some(r) {
                out.push((r, hashes[r]));
                last = Some(r);
            }
        }
        out
    }

    #[test]
    fn basics() {
        let h = kgram_hashes(b"abcdefg", 2, 0);
        assert_eq!(h.len(), 6);
        let fp = winnow(&h, 3);
        assert!(!fp.is_empty());
        assert_eq!(fingerprint(b"abcdefg", 2, 3, 0), fp);
        // short document fallback
        assert_eq!(fingerprint(b"ab", 5, 3, 0).len(), 1);
        // fewer grams than a window → single global min
        assert_eq!(fingerprint(b"abc", 2, 5, 0).len(), 1);
        // empty / degenerate params
        assert_eq!(fingerprint(b"", 2, 3, 0).len(), 1);
        assert_eq!(winnow(&h, 0), Vec::<(usize, u64)>::new());
        assert_eq!(winnow(&[], 2), Vec::<(usize, u64)>::new());
        assert_eq!(kgram_hashes(b"ab", 0, 0), Vec::<u64>::new());
        assert_eq!(kgram_hashes(b"ab", 3, 0), Vec::<u64>::new());
        // similarity
        let a = fingerprint(b"the quick brown fox", 2, 3, 0);
        let b2 = fingerprint(b"the quick brown dog", 2, 3, 0);
        let (i, u) = similarity(&a, &b2);
        assert!(i > 0 && u >= i);
        let c = fingerprint(b"completely different", 2, 3, 0);
        let (i2, _u2) = similarity(&a, &c);
        assert!(i2 <= i);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(37);
        for _ in 0..200 {
            let len = 1 + rng.below(60) as usize;
            let doc: Vec<u8> = (0..len).map(|_| rng.below(8) as u8).collect();
            let k = 1 + rng.below(6) as usize;
            let w = 1 + rng.below(8) as usize;
            let h = kgram_hashes(&doc, k, rng.next_u64());
            let got = winnow(&h, w);
            let want = naive_winnow(&h, w);
            assert_eq!(got, want, "len {len} k {k} w {w}");
            // coverage: every window's selected index is emitted
            if h.len() >= w {
                for i in 0..=h.len() - w {
                    let mut r = i;
                    for j in i + 1..i + w {
                        if h[j] <= h[r] {
                            r = j;
                        }
                    }
                    assert!(
                        got.iter().any(|&(idx, _)| idx == r),
                        "window {i} min {r} missing"
                    );
                }
            }
        }
    }

    /// Shared-run property: docs sharing a run of ≥ k+w−1 bytes
    /// must emit at least one identical fingerprint hash.
    #[test]
    fn shared_run_surfaces() {
        let mut rng = SplitMix64::new(53);
        for _ in 0..100 {
            let shared: Vec<u8> = (0..20).map(|_| rng.below(26) as u8).collect();
            let mut a = Vec::new();
            a.extend_from_slice(&(0..5).map(|_| rng.below(26) as u8).collect::<Vec<u8>>());
            a.extend_from_slice(&shared);
            let mut b = Vec::new();
            b.extend_from_slice(&(0..7).map(|_| rng.below(26) as u8).collect::<Vec<u8>>());
            b.extend_from_slice(&shared);
            let fa = fingerprint(&a, 3, 4, 0);
            let fb = fingerprint(&b, 3, 4, 0);
            let (inter, _) = similarity(&fa, &fb);
            assert!(
                inter >= 1,
                "shared 20-byte run produced no shared fingerprint"
            );
        }
    }
}
