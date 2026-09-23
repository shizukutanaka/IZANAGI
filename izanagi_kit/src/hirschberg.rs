//! Hirschberg's linear-space global alignment — Needleman–Wunsch
//! optimal score plus a full edit script in `O(min(|a|, |b|))` extra
//! space instead of `O(|a|·|b|)`.
//!
//! Scoring is fixed and integer: match `+2`, substitution `−1`, gap
//! `−1` (the classic NW cost form). The recursion splits `a` at its
//! midpoint, runs a one-row DP forward over `a[..mid]` and a mirrored
//! DP backward over `a[mid..]`, and chooses the `b`-split `j`
//! maximizing `L[j] + R[n−j]` — that pair `(mid, j)` lies on some
//! optimal alignment, so recursing on both halves yields an optimal
//! script with only `O(n)` live memory.
//!
//! ```
//! use izanagi_kit::hirschberg::{align, Op};
//! let al = align(b"GATTACA", b"GCATGCU");
//! assert_eq!(al.score, 4); // optimal NW score (+2/−1/−1)
//! let mut a = Vec::new();
//! for op in &al.ops {
//!     if let Op::Keep(c) | Op::Sub(c) | Op::Ins(c) = op {
//!         a.push(*c);
//!     }
//! }
//! // Applying the script to a yields b exactly.
//! assert_eq!(a, b"GCATGCU");
//! ```

/// One step of an alignment script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `a[i]` matched `b[j]` (same byte) — advances both.
    Keep(u8),
    /// `a[i]` was substituted by `b[j]` — advances both, payload is `b[j]`.
    Sub(u8),
    /// `a[i]` was deleted (a gap in `b`).
    Del,
    /// `b[j]` was inserted (a gap in `a`) — payload is `b[j]`.
    Ins(u8),
}

/// Alignment result: the optimal score and a canonical script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alignment {
    /// Needleman–Wunsch score (`+2` match, `−1` sub/gap).
    pub score: i64,
    /// Edit script turning `a` into `b`, left to right. Canonical:
    /// for a given score the recursion picks the *smallest* `b`-split
    /// `j`, which biases the script toward earlier ops deterministically.
    pub ops: Vec<Op>,
}

const MATCH: i64 = 2;
const SUB: i64 = -1;
const GAP: i64 = -1;
const NEG: i64 = i64::MIN / 4;

/// One row of NW scores for `a` vs every prefix of `b`
/// (`row[j]` = best score aligning all of `a` to `b[..j]`).
fn score_row(a: &[u8], b: &[u8]) -> Vec<i64> {
    let mut row: Vec<i64> = (0..=b.len()).map(|j| j as i64 * GAP).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = (i + 1) as i64 * GAP;
        for (j, &cb) in b.iter().enumerate() {
            let diag = prev;
            prev = row[j + 1];
            let m = diag + if ca == cb { MATCH } else { SUB };
            let d = row[j + 1] + GAP;
            let g = row[j] + GAP;
            row[j + 1] = m.max(d).max(g);
        }
    }
    row
}

/// The score DP's *last* row for reversed inputs — `rev[j]` is the
/// best score aligning `a` (read backwards) to `b[j..]` reversed,
/// i.e. aligning the suffix pair `a`/`b[j..]`.
fn split(a: &[u8], b: &[u8], ops: &mut Vec<Op>) {
    if a.is_empty() {
        ops.extend(b.iter().map(|&c| Op::Ins(c)));
        return;
    }
    if a.len() == 1 {
        // Single byte: pick the cheapest way to align against b.
        // Best is Keep if the byte occurs in b (first occurrence), else
        // sub the first byte and gap the rest.
        if let Some(p) = b.iter().position(|&c| c == a[0]) {
            ops.extend(b[..p].iter().map(|&c| Op::Ins(c)));
            ops.push(Op::Keep(a[0]));
            ops.extend(b[p + 1..].iter().map(|&c| Op::Ins(c)));
        } else if b.is_empty() {
            ops.push(Op::Del);
        } else {
            ops.push(Op::Sub(b[0]));
            ops.extend(b[1..].iter().map(|&c| Op::Ins(c)));
        }
        return;
    }
    let mid = a.len() / 2;
    let l = score_row(&a[..mid], b);
    // Reverse both suffixes to reuse the same forward routine.
    let rev_a: Vec<u8> = a[mid..].iter().rev().copied().collect();
    let rev_b: Vec<u8> = b.iter().rev().copied().collect();
    let r = score_row(&rev_a, &rev_b);
    let n = b.len();
    // j minimizing-then-first index maximizing L[j] + R[n−j].
    let mut j = 0usize;
    let mut best = NEG;
    for i in 0..=n {
        let s = l[i].saturating_add(r[n - i]);
        if s > best {
            best = s;
            j = i;
        }
    }
    split(&a[..mid], &b[..j], ops);
    split(&a[mid..], &b[j..], ops);
}

/// Optimal global alignment of `a` and `b` — score plus a canonical
/// op script. `O(|a|·|b|)` time, `O(|b|)` working space.
///
/// `Some` always — both empty inputs yield `{score: 0, ops: []}`.
pub fn align(a: &[u8], b: &[u8]) -> Alignment {
    let mut ops = Vec::with_capacity(a.len() + b.len());
    split(a, b, &mut ops);
    let score = ops.iter().fold(0i64, |s, op| {
        s + match op {
            Op::Keep(_) => MATCH,
            Op::Sub(_) | Op::Del | Op::Ins(_) => -1,
        }
    });
    Alignment { score, ops }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Full `O(nm)` Needleman–Wunsch score — the oracle optimum.
    fn nw_score(a: &[u8], b: &[u8]) -> i64 {
        let mut dp = vec![vec![0i64; b.len() + 1]; a.len() + 1];
        for (i, row) in dp.iter_mut().enumerate() {
            row[0] = i as i64 * GAP;
        }
        for (j, cell) in dp[0].iter_mut().enumerate() {
            *cell = j as i64 * GAP;
        }
        for (i, &ca) in a.iter().enumerate() {
            let i = i + 1;
            for (j, &cb) in b.iter().enumerate() {
                let j = j + 1;
                let m = dp[i - 1][j - 1] + if ca == cb { MATCH } else { SUB };
                dp[i][j] = m.max(dp[i - 1][j] + GAP).max(dp[i][j - 1] + GAP);
            }
        }
        dp[a.len()][b.len()]
    }

    /// The script must be internally consistent: consume `a`, emit `b`.
    fn apply(a: &[u8], ops: &[Op]) -> Vec<u8> {
        let mut i = 0usize;
        let mut out = Vec::new();
        for op in ops {
            match *op {
                Op::Keep(c) => {
                    assert_eq!(a[i], c);
                    out.push(c);
                    i += 1;
                }
                Op::Sub(c) => {
                    assert_ne!(a[i], c);
                    out.push(c);
                    i += 1;
                }
                Op::Del => i += 1,
                Op::Ins(c) => out.push(c),
            }
        }
        assert_eq!(i, a.len());
        out
    }

    #[test]
    fn known() {
        // Identical strings: all keeps.
        let al = align(b"ABCD", b"ABCD");
        assert_eq!(al.score, 8);
        assert!(al.ops.iter().all(|o| matches!(o, Op::Keep(_))));
        // Empty sides.
        assert_eq!(align(b"", b"").ops.len(), 0);
        assert_eq!(align(b"AB", b"").score, -2);
        assert_eq!(align(b"", b"AB").score, -2);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0x9a5c_77e1_0002);
        for _ in 0..400 {
            let na = rng.below(20) as usize;
            let nb = rng.below(20) as usize;
            let a: Vec<u8> = (0..na).map(|_| b'A' + rng.below(4) as u8).collect();
            let b: Vec<u8> = (0..nb).map(|_| b'A' + rng.below(4) as u8).collect();
            let al = align(&a, &b);
            assert_eq!(al.score, nw_score(&a, &b), "a={a:?} b={b:?}");
            assert_eq!(apply(&a, &al.ops), b);
        }
    }
}
