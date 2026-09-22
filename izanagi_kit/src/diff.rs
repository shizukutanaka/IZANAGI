//! Sequence diff and edit distance — Myers `O(ND)` greedy diff producing
//! minimal edit scripts, plus Levenshtein distance and LCS length.
//!
//! Works on any `Eq` element slice. Outputs are content-defined: the
//! Myers backtrack follows a fixed tie-break (prefer deletions before
//! insertions at equal cost), so identical inputs always produce the
//! identical script. Used by `desync`/`verify` tooling to show a minimal
//! field-level diff between two diverging states, and by content tools
//! for `did-you-mean` suggestions.
//!
//! ```
//! use izanagi_kit::diff::{diff, levenshtein, Edit};
//! let a = [1, 2, 3, 4];
//! let b = [1, 3, 4, 5];
//! let ops = diff(&a, &b);
//! // Minimal script: keep 1, del 2, keep 3 4, ins 5.
//! assert_eq!(levenshtein(&a, &b), 2);
//! assert!(ops.len() >= 2);
//! ```

/// One element-level edit operation in a diff script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    /// `a[a_idx]` survives unchanged into `b`.
    Keep(usize),
    /// `a[a_idx]` is deleted.
    Del(usize),
    /// `b[b_idx]` is inserted.
    Ins(usize),
}

/// Classic Levenshtein edit distance (`O(nm)` DP, insert/delete/substitute
/// all cost 1). Deterministic by construction.
pub fn levenshtein<T: Eq>(a: &[T], b: &[T]) -> u32 {
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m as u32;
    }
    if m == 0 {
        return n as u32;
    }
    let mut prev: Vec<u32> = (0..=m as u32).collect();
    let mut cur = vec![0u32; m + 1];
    for i in 1..=n {
        cur[0] = i as u32;
        for j in 1..=m {
            let sub = prev[j - 1] + u32::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(sub);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

/// Longest-common-subsequence length (`O(nm)` DP).
pub fn lcs_len<T: Eq>(a: &[T], b: &[T]) -> u32 {
    let (n, m) = (a.len(), b.len());
    let mut prev = vec![0u32; m + 1];
    let mut cur = vec![0u32; m + 1];
    for i in 1..=n {
        for j in 1..=m {
            cur[j] = if a[i - 1] == b[j - 1] {
                prev[j - 1] + 1
            } else {
                prev[j].max(cur[j - 1])
            };
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

/// Myers diff: minimal edit script transforming `a` into `b` as a
/// sequence of `Keep`/`Del`/`Ins` ops in positional order. `O(ND)` time,
/// `O(ND)` space for the trace (D ≤ n+m). For a tie at equal cost the
/// greedy frontier prefers a *down* move (insertion); backtracking
/// replays the same choices deterministically.
///
/// The produced script is always minimal: applying it to `a` yields `b`
/// and its `Del+Ins` count equals `n + m - 2·lcs_len(a,b)`.
pub fn diff<T: Eq>(a: &[T], b: &[T]) -> Vec<Edit> {
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return (0..m).map(Edit::Ins).collect();
    }
    if m == 0 {
        return (0..n).map(Edit::Del).collect();
    }
    // Myers O(ND): v[k] = furthest x reachable on diagonal k = x - y.
    // d iterates the edit distance; we store each v for the backtrack.
    let max = n + m;
    let offset = max as i64; // shift k ∈ [-max, max] into [0, 2·max]
    let width = 2 * max + 1;
    let mut trace: Vec<Vec<i64>> = Vec::new();
    let mut v = vec![0i64; width];
    let mut found_d = None;
    'outer: for d in 0..=max as i64 {
        trace.push(v.clone());
        for k in (-d..=d).step_by(2) {
            let ki = (k + offset) as usize;
            // Down (insert) when at left edge or right side is strictly
            // further along the diagonal below.
            let mut x = if k == -d || (k != d && v[ki - 1] < v[ki + 1]) {
                v[ki + 1]
            } else {
                v[ki - 1] + 1
            };
            let mut y = x - k;
            while x < n as i64 && y < m as i64 && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[ki] = x;
            if x >= n as i64 && y >= m as i64 {
                found_d = Some(d);
                trace.push(v.clone());
                break 'outer;
            }
        }
    }
    let _d = match found_d {
        Some(d) => d,
        None => return Vec::new(), // unreachable for finite n,m
    };
    // Backtrack: walk diagonals from (n,m) to (0,0) emitting reversed ops.
    // `d` is the frontier level being stepped back from (1..=D); trace[d]
    // is the V array used during iteration d (= V_{d-1}).
    let mut ops_rev: Vec<Edit> = Vec::new();
    let (mut x, mut y) = (n as i64, m as i64);
    for d in (1..trace.len() - 1).rev() {
        let v_d = &trace[d];
        let k = x - y;
        let ki = (k + offset) as usize;
        // Inverse of the forward rule: decide whether the frontier moved
        // right (Del) or down (Ins) at this step.
        let prev_k = if k == -(d as i64) || (k != d as i64 && v_d[ki - 1] < v_d[ki + 1]) {
            k + 1
        } else {
            k - 1
        };
        let prev_ki = (prev_k + offset) as usize;
        let (x_prev, y_prev) = (v_d[prev_ki], v_d[prev_ki] - prev_k);
        // Snake: from (x_prev,y_prev) or (x_prev,y_prev)+(1|0) to (x,y).
        // Diagonal moves are Keeps in reverse.
        let (x_after_edit, y_after_edit) = if prev_k == k + 1 {
            // Came from a down move: y increased after the edit.
            (x_prev, y_prev + 1)
        } else {
            (x_prev + 1, y_prev)
        };
        while x > x_after_edit && y > y_after_edit {
            ops_rev.push(Edit::Keep((x - 1) as usize));
            x -= 1;
            y -= 1;
        }
        if prev_k == k + 1 {
            ops_rev.push(Edit::Ins((y - 1) as usize));
            y -= 1;
        } else {
            ops_rev.push(Edit::Del((x - 1) as usize));
            x -= 1;
        }
    }
    // Remaining diagonal walk to the origin.
    while x > 0 && y > 0 {
        ops_rev.push(Edit::Keep((x - 1) as usize));
        x -= 1;
        y -= 1;
    }
    ops_rev.reverse();
    ops_rev
}

/// Compact hunk form of [`diff`]: maximal non-`Keep` runs as
/// `(a_start, a_del_count, b_start, b_ins_count)` — the shape of a
/// unified-diff hunk header.
pub fn hunks<T: Eq>(a: &[T], b: &[T]) -> Vec<(usize, usize, usize, usize)> {
    let mut out = Vec::new();
    let (mut ai, mut bi) = (0usize, 0usize);
    let mut open: Option<(usize, usize, usize, usize)> = None;
    for op in diff(a, b) {
        match op {
            Edit::Keep(_) => {
                ai += 1;
                bi += 1;
                if let Some(h) = open.take() {
                    out.push(h);
                }
            }
            Edit::Del(_) => {
                let h = open.get_or_insert((ai, 0, bi, 0));
                h.1 += 1;
                ai += 1;
            }
            Edit::Ins(_) => {
                let h = open.get_or_insert((ai, 0, bi, 0));
                h.3 += 1;
                bi += 1;
            }
        }
    }
    if let Some(h) = open.take() {
        out.push(h);
    }
    out
}

/// Apply an edit script to `a`, producing `b`. Debug-friendly round-trip
/// used by tests and by callers that want to verify a stored script.
pub fn apply<T: Eq + Clone>(a: &[T], b: &[T], script: &[Edit]) -> Vec<T> {
    let mut out = Vec::with_capacity(a.len() + script.len());
    for op in script {
        match op {
            Edit::Keep(i) => out.push(a[*i].clone()),
            Edit::Del(_) => {}
            Edit::Ins(j) => out.push(b[*j].clone()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: apply the script; it must produce b and be minimal
    /// (Del+Ins == n + m - 2·lcs).
    fn check_script(a: &[u8], b: &[u8]) {
        let ops = diff(a, b);
        let got = apply(a, b, &ops);
        assert_eq!(got, b, "apply(script) != b: a={a:?} b={b:?} ops={ops:?}");
        let edits = ops.iter().filter(|o| !matches!(o, Edit::Keep(_))).count() as u32;
        let minimal = a.len() as u32 + b.len() as u32 - 2 * lcs_len(a, b);
        assert_eq!(edits, minimal, "script not minimal for a={a:?} b={b:?}");
        // Op order is consistent: Del/Keep reference increasing a idx,
        // Ins references increasing b idx.
        let mut ai = 0usize;
        let mut bi = 0usize;
        for op in &ops {
            match op {
                Edit::Keep(i) => {
                    assert_eq!(*i, ai);
                    ai += 1;
                    bi += 1;
                }
                Edit::Del(i) => {
                    assert_eq!(*i, ai);
                    ai += 1;
                }
                Edit::Ins(j) => {
                    assert_eq!(*j, bi);
                    bi += 1;
                }
            }
        }
        assert_eq!(ai, a.len());
        assert_eq!(bi, b.len());
    }

    #[test]
    fn diff_round_trip_and_minimality_on_random() {
        let mut rng = SplitMix64::new(0xD1FF);
        for _ in 0..300 {
            let na = rng.below(8);
            let nb = rng.below(8);
            let a: Vec<u8> = (0..na).map(|_| rng.below(5) as u8).collect();
            let b: Vec<u8> = (0..nb).map(|_| rng.below(5) as u8).collect();
            check_script(&a, &b);
            // Plus a mutation-derived pair: b = a with random edits.
            let mut c = a.clone();
            for _ in 0..rng.below(4) {
                if c.is_empty() || rng.below(2) == 0 {
                    c.insert(rng.below(c.len() as u32 + 1) as usize, rng.below(5) as u8);
                } else {
                    c.remove(rng.below(c.len() as u32) as usize);
                }
            }
            check_script(&a, &c);
        }
    }

    #[test]
    fn diff_edges() {
        assert_eq!(diff::<u8>(&[], &[]), Vec::<Edit>::new());
        assert_eq!(diff::<u8>(&[], &[1, 2]), vec![Edit::Ins(0), Edit::Ins(1)]);
        assert_eq!(diff::<u8>(&[1, 2], &[]), vec![Edit::Del(0), Edit::Del(1)]);
        assert_eq!(
            diff::<u8>(&[1, 2, 3], &[1, 2, 3]),
            vec![Edit::Keep(0), Edit::Keep(1), Edit::Keep(2)]
        );
    }

    #[test]
    fn hunks_coalesce_adjacent_edits() {
        let h = hunks(&[1u8, 2, 3, 4, 5], &[1u8, 9, 9, 4, 5]);
        assert_eq!(h, vec![(1, 2, 1, 2)]); // del a[1..3), ins b[1..3)
        let h2 = hunks(&[1u8, 2], &[1u8, 2, 3]);
        assert_eq!(h2, vec![(2, 0, 2, 1)]);
        assert_eq!(hunks::<u8>(&[1], &[1]), vec![]);
    }

    #[test]
    fn levenshtein_is_symmetric_metric() {
        assert_eq!(levenshtein(&[1u8, 2, 3], &[1u8, 3]), 1);
        assert_eq!(levenshtein(&[1u8], &[2u8]), 1);
        assert_eq!(levenshtein::<u8>(&[], &[1, 2]), 2);
        let mut rng = SplitMix64::new(0x1E99);
        for _ in 0..100 {
            let a: Vec<u8> = (0..rng.below(8)).map(|_| rng.below(4) as u8).collect();
            let b: Vec<u8> = (0..rng.below(8)).map(|_| rng.below(4) as u8).collect();
            assert_eq!(levenshtein(&a, &b), levenshtein(&b, &a));
        }
    }
}
