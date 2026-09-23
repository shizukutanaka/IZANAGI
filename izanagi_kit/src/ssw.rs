//! Smith–Waterman local alignment — the highest-scoring
//! *substring-vs-substring* alignment over integer scores.
//! Unlike [`crate::hirschberg`]'s global alignment, cells can
//! restart at 0, so the best local match is reported wherever
//! it lives.
//!
//! Scoring is the shared `+2` match / `−1` substitution / `−1`
//! gap scheme. `local` returns `(score, a_range, b_range)` —
//! `a[i0..i1)` aligned against `b[j0..j1)` attains the score;
//! empty ranges mean the empty alignment scored 0, which is
//! always achievable. Ties break toward the earliest-ending
//! cell in `i`, then `j`, for a canonical answer.
//!
//! ```
//! use izanagi_kit::ssw::local;
//! let l = local(b"AAAAATGCT", b"GTGCG");
//! assert_eq!(l.score, 6); // "TGC" vs "TGC": 3 matches
//! assert_eq!(&b"AAAAATGCT"[l.a_lo..l.a_hi], b"TGC");
//! ```
//!
//! References: Smith & Waterman (1981) "Identification of
//! common molecular subsequences" (JMB); Gusfield *Algorithms
//! on Strings* §11.

/// The result of [`local`]: best score plus the substring
/// half-open ranges that attain it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Local {
    /// Best achievable score (≥ 0; the empty alignment scores 0).
    pub score: i64,
    /// Range into `a` — `a[a_lo..a_hi)`.
    pub a_lo: usize,
    /// Range into `b` — `b[b_lo..b_hi)`.
    pub b_lo: usize,
    /// Range into `a`, end — see [`Local::a_lo`].
    pub a_hi: usize,
    /// Range into `b`, end — see [`Local::b_lo`].
    pub b_hi: usize,
}

/// Best local alignment score and its witness ranges.
pub fn local(a: &[u8], b: &[u8]) -> Local {
    let (n, m) = (a.len(), b.len());
    // dp[i][j] = best local score ending at a[i-1], b[j-1]
    let mut dp = vec![vec![0i64; m + 1]; n + 1];
    let mut best = Local {
        score: 0,
        a_lo: 0,
        a_hi: 0,
        b_lo: 0,
        b_hi: 0,
    };
    for i in 1..=n {
        for j in 1..=m {
            let sub = dp[i - 1][j - 1] + if a[i - 1] == b[j - 1] { 2 } else { -1 };
            let del = dp[i - 1][j] - 1;
            let ins = dp[i][j - 1] - 1;
            dp[i][j] = 0i64.max(sub).max(del).max(ins);
            if dp[i][j] > best.score {
                best.score = dp[i][j];
                best.a_hi = i;
                best.b_hi = j;
            }
        }
    }
    // Trace the winning cell back to its restart to recover
    // both low endpoints.
    let (mut i, mut j) = (best.a_hi, best.b_hi);
    while i > 0 && j > 0 && dp[i][j] > 0 {
        let sub = dp[i - 1][j - 1] + if a[i - 1] == b[j - 1] { 2 } else { -1 };
        if dp[i][j] == sub {
            i -= 1;
            j -= 1;
        } else if dp[i][j] == dp[i - 1][j] - 1 {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    best.a_lo = i;
    best.b_lo = j;
    best
}

/// Best local alignment *score* only — same algorithm, no
/// traceback. Kept separate for callers that skip the second
/// pass.
pub fn score(a: &[u8], b: &[u8]) -> i64 {
    local(a, b).score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute oracle: best score = max over all substring pairs
    /// of the global Needleman–Wunsch score (recomputed
    /// independently here).
    fn nw(x: &[u8], y: &[u8]) -> i64 {
        let mut prev: Vec<i64> = (0..=y.len() as i64).map(|j| -j).collect();
        let mut cur = prev.clone();
        for (i, &xc) in x.iter().enumerate() {
            cur[0] = -(i as i64) - 1;
            for (j, &yc) in y.iter().enumerate() {
                let s = prev[j] + if xc == yc { 2 } else { -1 };
                cur[j + 1] = s.max(prev[j + 1] - 1).max(cur[j] - 1);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        prev[y.len()]
    }

    fn oracle(a: &[u8], b: &[u8]) -> i64 {
        let mut best = 0i64;
        for i0 in 0..=a.len() {
            for i1 in i0..=a.len() {
                for j0 in 0..=b.len() {
                    for j1 in j0..=b.len() {
                        best = best.max(nw(&a[i0..i1], &b[j0..j1]));
                    }
                }
            }
        }
        best
    }

    /// Alignment verifier: does `a[i0..i1)` vs `b[j0..j1)`
    /// really attain `score`? Recompute the global score of
    /// the returned ranges.
    fn verify(l: &Local, a: &[u8], b: &[u8]) -> bool {
        l.score == nw(&a[l.a_lo..l.a_hi], &b[l.b_lo..l.b_hi])
    }

    #[test]
    fn basics() {
        let l = local(b"AAAAATGCT", b"GTGCG");
        assert_eq!(l.score, 6);
        assert_eq!(&b"AAAAATGCT"[l.a_lo..l.a_hi], b"TGC");
        let z = local(b"", b"abc");
        assert_eq!(z.score, 0);
        let z = local(b"abc", b"");
        assert_eq!(z.score, 0);
        let z = local(b"abc", b"xyz");
        assert_eq!(z.score, 0); // best is the empty alignment
        let z = local(b"abc", b"abc");
        assert_eq!(z.score, 6);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0x5511);
        for _ in 0..150 {
            let n = rng.below(8) as usize;
            let m = rng.below(8) as usize;
            let a: Vec<u8> = (0..n).map(|_| b"ACGT"[rng.below(4) as usize]).collect();
            let b: Vec<u8> = (0..m).map(|_| b"ACGT"[rng.below(4) as usize]).collect();
            let l = local(&a, &b);
            assert_eq!(l.score, oracle(&a, &b), "a={a:?} b={b:?}");
            assert!(verify(&l, &a, &b));
            assert!(l.a_lo <= l.a_hi && l.a_hi <= a.len());
            assert!(l.b_lo <= l.b_hi && l.b_hi <= b.len());
            assert_eq!(score(&a, &b), l.score);
        }
    }

    /// Repeated-substring structure: best local region is a
    /// whole shared block when present.
    #[test]
    fn shared_block_wins() {
        let l = local(b"xxGATTACAyy", b"qqGATTACAww");
        assert_eq!(l.score, 14);
        assert_eq!(&b"xxGATTACAyy"[l.a_lo..l.a_hi], b"GATTACA");
    }
}
