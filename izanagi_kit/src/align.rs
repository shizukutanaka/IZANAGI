//! Pairwise sequence alignment — Needleman–Wunsch global and
//! Smith–Waterman local DP over byte strings with integer scores, the
//! bioinformatics pair behind diff tools, fuzzy matchers, and
//! edit-distance-with-traceback. [`editdist`](crate::editdist) gives
//! the Levenshtein *number*; this gives the optimal *alignment*.
//!
//! Scoring is `match_score` on equal bytes, `mismatch` on unequal,
//! `gap` per inserted gap. Ties break diagonal > up (gap in `a`,
//! emitted `D`) > left (gap in `b`, emitted `I`) — fixed and
//! documented so replays never fork.
//!
//! Ops: `=` match, `X` mismatch, `D` deletion (`a` consumes, `b` gaps),
//! `I` insertion (`b` consumes, `a` gaps).
//!
//! ```
//! use izanagi_kit::align::{needleman_wunsch, smith_waterman};
//!
//! let (score, ops) = needleman_wunsch(b"GATTACA", b"GCATGCU", 1, -1, -1);
//! // Global: every byte of both inputs is consumed by the ops.
//! let a_used = ops
//!     .iter()
//!     .filter(|o| matches!(o, b'=' | b'X' | b'D'))
//!     .count();
//! let b_used = ops
//!     .iter()
//!     .filter(|o| matches!(o, b'=' | b'X' | b'I'))
//!     .count();
//! assert_eq!((a_used, b_used), (7, 7));
//! let (best, _) = smith_waterman(b"TGGTG", b"ATCGT", 2, -1, -1);
//! assert!(best > 0);
//! ```

/// Dynamic-programming fill shared by NW and SW. `F[i][j]` = best score
/// for prefixes `a[..i]`, `b[..j]` (local variant floors at 0).
/// Returns the flat `(n+1)×(m+1)` row-major table.
fn fill(a: &[u8], b: &[u8], match_score: i32, mismatch: i32, gap: i32, local: bool) -> Vec<i32> {
    let (n, m) = (a.len(), b.len());
    let w = m + 1;
    let mut f = vec![0i32; (n + 1) * w];
    for i in 0..=n {
        for j in 0..=m {
            f[i * w + j] = if i == 0 || j == 0 {
                // Global edges score i·gap/j·gap (corner 0); local
                // alignments just floor the whole border at 0.
                if local {
                    0
                } else {
                    (i + j) as i32 * gap
                }
            } else {
                let diag = f[(i - 1) * w + (j - 1)]
                    + if a[i - 1] == b[j - 1] {
                        match_score
                    } else {
                        mismatch
                    };
                let up = f[(i - 1) * w + j] + gap;
                let left = f[i * w + (j - 1)] + gap;
                let best = diag.max(up).max(left);
                if local {
                    best.max(0)
                } else {
                    best
                }
            };
        }
    }
    f
}

/// Traceback from `(i,j)` to a stop; emits ops in reverse and flips.
/// Tie order: diagonal, up (`D`), left (`I`).
#[allow(clippy::too_many_arguments)]
fn trace(
    a: &[u8],
    b: &[u8],
    f: &[i32],
    match_score: i32,
    mismatch: i32,
    gap: i32,
    local: bool,
    mut i: usize,
    mut j: usize,
) -> Vec<u8> {
    let w = b.len() + 1;
    let mut ops = Vec::new();
    while i > 0 || j > 0 {
        let cur = f[i * w + j];
        if local && cur == 0 {
            break;
        }
        if i > 0 && j > 0 {
            let diag = f[(i - 1) * w + (j - 1)]
                + if a[i - 1] == b[j - 1] {
                    match_score
                } else {
                    mismatch
                };
            if cur == diag {
                ops.push(if a[i - 1] == b[j - 1] { b'=' } else { b'X' });
                i -= 1;
                j -= 1;
                continue;
            }
        }
        if i > 0 && cur == f[(i - 1) * w + j] + gap {
            ops.push(b'D');
            i -= 1;
        } else {
            ops.push(b'I');
            j -= 1;
        }
    }
    ops.reverse();
    ops
}

/// Needleman–Wunsch global alignment of `a` onto `b`: optimal score
/// plus the op string consuming **all** of both inputs.
pub fn needleman_wunsch(
    a: &[u8],
    b: &[u8],
    match_score: i32,
    mismatch: i32,
    gap: i32,
) -> (i32, Vec<u8>) {
    let f = fill(a, b, match_score, mismatch, gap, false);
    let ops = trace(
        a,
        b,
        &f,
        match_score,
        mismatch,
        gap,
        false,
        a.len(),
        b.len(),
    );
    (f[a.len() * (b.len() + 1) + b.len()], ops)
}

/// Smith–Waterman local alignment: best-scoring alignment of any
/// substring of `a` against any substring of `b`, plus its ops.
/// `0, vec![]` when nothing positive aligns.
pub fn smith_waterman(
    a: &[u8],
    b: &[u8],
    match_score: i32,
    mismatch: i32,
    gap: i32,
) -> (i32, Vec<u8>) {
    let w = b.len() + 1;
    let f = fill(a, b, match_score, mismatch, gap, true);
    // First maximal cell in row-major order — deterministic.
    let (mut best, mut bi, mut bj) = (0i32, 0usize, 0usize);
    for (idx, &v) in f.iter().enumerate() {
        if v > best {
            best = v;
            bi = idx / w;
            bj = idx % w;
        }
    }
    if best == 0 {
        return (0, Vec::new());
    }
    let ops = trace(a, b, &f, match_score, mismatch, gap, true, bi, bj);
    (best, ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Replay ops to recompute the score — self-consistency oracle.
    fn rescore(a: &[u8], b: &[u8], ops: &[u8], ms: i32, mm: i32, gap: i32) -> i32 {
        let (mut i, mut j, mut s) = (0usize, 0usize, 0);
        for &op in ops {
            match op {
                b'=' | b'X' => {
                    s += if a[i] == b[j] { ms } else { mm };
                    i += 1;
                    j += 1;
                }
                b'D' => {
                    s += gap;
                    i += 1;
                }
                b'I' => {
                    s += gap;
                    j += 1;
                }
                _ => panic!("bad op"),
            }
        }
        s
    }

    /// Exhaustive alignment oracle over all op sequences (tiny inputs).
    fn oracle_nw(a: &[u8], b: &[u8], ms: i32, mm: i32, gap: i32) -> i32 {
        #[allow(clippy::too_many_arguments)]
        fn go(
            a: &[u8],
            b: &[u8],
            i: usize,
            j: usize,
            acc: i32,
            ms: i32,
            mm: i32,
            gap: i32,
            best: &mut i32,
        ) {
            if i == a.len() && j == b.len() {
                *best = (*best).max(acc);
                return;
            }
            if i < a.len() && j < b.len() {
                go(
                    a,
                    b,
                    i + 1,
                    j + 1,
                    acc + if a[i] == b[j] { ms } else { mm },
                    ms,
                    mm,
                    gap,
                    best,
                );
            }
            if i < a.len() {
                go(a, b, i + 1, j, acc + gap, ms, mm, gap, best);
            }
            if j < b.len() {
                go(a, b, i, j + 1, acc + gap, ms, mm, gap, best);
            }
        }
        let mut best = i32::MIN;
        go(a, b, 0, 0, 0, ms, mm, gap, &mut best);
        best
    }

    #[test]
    fn nw_matches_exhaustive_oracle() {
        let mut g = crate::rng::SplitMix64::new(0xA11);
        for _case in 0..40 {
            let (na, nb) = (g.range(0, 5) as usize, g.range(0, 5) as usize);
            let a: Vec<u8> = (0..na).map(|_| b"ACGT"[g.below(4) as usize]).collect();
            let b: Vec<u8> = (0..nb).map(|_| b"ACGT"[g.below(4) as usize]).collect();
            let (s, ops) = needleman_wunsch(&a, &b, 2, -1, -2);
            assert_eq!(s, oracle_nw(&a, &b, 2, -1, -2), "{a:?} vs {b:?}");
            assert_eq!(s, rescore(&a, &b, &ops, 2, -1, -2));
        }
    }

    #[test]
    fn nw_identity_and_ops_semantics() {
        let (s, ops) = needleman_wunsch(b"ACGT", b"ACGT", 2, -1, -3);
        assert_eq!(s, 8);
        assert_eq!(ops, b"====");
        // Empty inputs: pure gap run.
        let (s, ops) = needleman_wunsch(b"AA", b"", 1, -1, -2);
        assert_eq!((s, ops.as_slice()), (-4, b"DD".as_slice()));
    }

    #[test]
    fn sw_finds_local_island() {
        // "TTT ACGG TTT" vs "ACGG": local score 8, ignores the noise.
        let (s, ops) = smith_waterman(b"TTTACGGTTT", b"ACGG", 2, -3, -3);
        assert_eq!(s, 8);
        assert_eq!(ops, b"====");
        // Nothing positive → empty.
        let (s, ops) = smith_waterman(b"AAAA", b"TTTT", 1, -2, -2);
        assert_eq!((s, ops.as_slice()), (0, b"".as_slice()));
    }

    #[test]
    fn sw_score_rescores_and_beats_global_when_noise() {
        let a = b"XXGATTACAXX";
        let b = b"YYGATTACAYY";
        let (gs, _) = needleman_wunsch(a, b, 2, -1, -2);
        let (ls, ops) = smith_waterman(a, b, 2, -1, -2);
        // Local skips the flanking mismatches: beats global.
        assert!(ls > gs);
        assert_eq!(ls, rescore(b"GATTACA", b"GATTACA", &ops, 2, -1, -2));
    }

    #[test]
    fn deterministic_twice() {
        let a = needleman_wunsch(b"GATTACA", b"GCATGCU", 1, -1, -1);
        let b = needleman_wunsch(b"GATTACA", b"GCATGCU", 1, -1, -1);
        assert_eq!(a, b);
        let c = smith_waterman(b"GATTACA", b"GCATGCU", 1, -1, -1);
        let d = smith_waterman(b"GATTACA", b"GCATGCU", 1, -1, -1);
        assert_eq!(c, d);
    }
}
