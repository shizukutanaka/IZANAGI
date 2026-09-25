//! Viterbi decoding: the most likely hidden-state sequence for an HMM,
//! as shortest-path DP. Costs are *additive integers* — pass scaled
//! negative log-probabilities (`⌊−K·log p⌉` in whatever scale `K` you
//! like; `Fixed::ln` can produce them) so the whole decode stays exact
//! and replay-safe. Lower total cost wins; ties resolve to the smaller
//! state index, which is what makes the answer canonical.
//!
//! `O(T·S²)` time, `O(T·S)` traceback table — the textbook algorithm
//! (Viterbi 1967; Forney 1973 for the MLSE formulation). Ties take the
//! smaller predecessor state at each cell: fixed, replay-stable.
//!
//! ```
//! use izanagi_kit::viterbi::viterbi;
//!
//! // Two states: 0 = Fair, 1 = Loaded. Costs are −scaled log probs.
//! let init = vec![10, 10];                       // uniform-ish
//! let trans = vec![vec![7, 10], vec![10, 7]];    // sticky-ish stays
//! let emit = vec![
//!     vec![10, 20, 30], // Fair:  die 1..3 uniform
//!     vec![50, 20, 5],  // Loaded: favors 3
//! ];
//! let obs = vec![0u32, 2, 2];                    // rolls 1,3,3
//! let path = viterbi(&init, &trans, &emit, &obs).unwrap();
//! assert_eq!(path, vec![0, 1, 1]);
//! ```

/// Sentinel for unreachable states: `i64::MAX/4` so `INF + cost` never
/// overflows while remaining larger than any real total.
const INF: i64 = i64::MAX / 4;

/// Most likely state path for `obs`, or `None` for malformed input
/// (any zero-length matrix, `emit` width < max obs + 1, non-rectangular
/// `trans`, empty `obs` — an empty observation sequence has no path to
/// return). Costs are additive: `init[s]` to start in `s`, `trans[a][b]`
/// to move `a→b`, `emit[s][o]` to emit `o` from `s`.
///
/// On cost ties the smaller predecessor state wins at each step — a
/// fixed, replay-stable convention (not a global lexicographic minimum).
pub fn viterbi(
    init: &[i32],
    trans: &[Vec<i32>],
    emit: &[Vec<i32>],
    obs: &[u32],
) -> Option<Vec<u32>> {
    let s = init.len();
    if s == 0 || obs.is_empty() || trans.len() != s || emit.len() != s {
        return None;
    }
    if trans.iter().any(|r| r.len() != s) {
        return None;
    }
    let omax = *obs.iter().max()? as usize;
    if emit.iter().any(|r| r.len() <= omax) {
        return None;
    }
    let t = obs.len();
    // dp[k][s] = min cost of any length-(k+1) path ending in s.
    let mut dp = vec![vec![INF; s]; t];
    let mut back = vec![vec![0u32; s]; t];
    for (st, &c) in init.iter().enumerate() {
        dp[0][st] = c as i64 + emit[st][obs[0] as usize] as i64;
    }
    for k in 1..t {
        for to in 0..s {
            let mut best = INF;
            let mut arg = 0u32;
            for from in 0..s {
                let cand = dp[k - 1][from] + trans[from][to] as i64;
                if cand < best {
                    best = cand;
                    arg = from as u32;
                }
            }
            back[k][to] = arg;
            dp[k][to] = best + emit[to][obs[k] as usize] as i64;
        }
    }
    // Recover: argmin over final column; ties → smaller state index.
    let mut last = 0u32;
    for st in 1..s {
        if dp[t - 1][st] < dp[t - 1][last as usize] {
            last = st as u32;
        }
    }
    let mut path = vec![0u32; t];
    path[t - 1] = last;
    for k in (1..t).rev() {
        path[k - 1] = back[k][path[k] as usize];
    }
    Some(path)
}

/// Total path cost of the optimum — same DP, no traceback. `None` on
/// malformed input or when every path is unreachable (all-INF column).
pub fn viterbi_cost(
    init: &[i32],
    trans: &[Vec<i32>],
    emit: &[Vec<i32>],
    obs: &[u32],
) -> Option<i64> {
    let s = init.len();
    if s == 0 || obs.is_empty() || trans.len() != s || emit.len() != s {
        return None;
    }
    if trans.iter().any(|r| r.len() != s) {
        return None;
    }
    let omax = *obs.iter().max()? as usize;
    if emit.iter().any(|r| r.len() <= omax) {
        return None;
    }
    let mut col: Vec<i64> = (0..s)
        .map(|st| init[st] as i64 + emit[st][obs[0] as usize] as i64)
        .collect();
    for &o in &obs[1..] {
        let mut next = vec![INF; s];
        for to in 0..s {
            for from in 0..s {
                let cand = col[from] + trans[from][to] as i64;
                if cand < next[to] {
                    next[to] = cand;
                }
            }
            next[to] += emit[to][o as usize] as i64;
        }
        col = next;
    }
    let best = col.into_iter().min()?;
    if best >= INF {
        return None;
    }
    Some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute force: enumerate all S^T state sequences, min additive cost,
    /// smallest lexicographic path on ties.
    fn brute(init: &[i32], trans: &[Vec<i32>], emit: &[Vec<i32>], obs: &[u32]) -> Option<Vec<u32>> {
        let s = init.len();
        if s == 0 || obs.is_empty() {
            return None;
        }
        let t = obs.len();
        let mut best: Option<(i64, Vec<u32>)> = None;
        // Iterative product over state indices (mixed-radix enumeration).
        let mut idx = vec![0usize; t];
        loop {
            let mut cost = init[idx[0]] as i64 + emit[idx[0]][obs[0] as usize] as i64;
            let mut ok = true;
            for k in 1..t {
                cost += trans[idx[k - 1]][idx[k]] as i64 + emit[idx[k]][obs[k] as usize] as i64;
                if cost > i64::MAX / 2 {
                    ok = false;
                    break;
                }
            }
            if ok {
                let cand: Vec<u32> = idx.iter().map(|&i| i as u32).collect();
                let better = match &best {
                    None => true,
                    Some((bc, bp)) => cost < *bc || (cost == *bc && cand < *bp),
                };
                if better {
                    best = Some((cost, cand));
                }
            }
            // Increment mixed-radix counter.
            let mut k = t;
            loop {
                if k == 0 {
                    return best.map(|(_, p)| p);
                }
                k -= 1;
                idx[k] += 1;
                if idx[k] < s {
                    break;
                }
                idx[k] = 0;
            }
        }
    }

    #[test]
    fn matches_brute_force_on_random_small_hmms() {
        let mut rng = crate::rng::SplitMix64::new(0xACE);
        for _case in 0..200 {
            let s = 1 + rng.below(4) as usize;
            let t = 1 + rng.below(6) as usize;
            let vocab = 1 + rng.below(4) as usize;
            let init: Vec<i32> = (0..s).map(|_| rng.below(50) as i32).collect();
            let trans: Vec<Vec<i32>> = (0..s)
                .map(|_| (0..s).map(|_| rng.below(50) as i32).collect())
                .collect();
            let emit: Vec<Vec<i32>> = (0..s)
                .map(|_| (0..vocab).map(|_| rng.below(60) as i32).collect())
                .collect();
            let obs: Vec<u32> = (0..t).map(|_| rng.below(vocab as u32)).collect();
            let got = viterbi(&init, &trans, &emit, &obs).unwrap();
            let want = brute(&init, &trans, &emit, &obs).unwrap();
            // Equal total cost is required; exact path equality is not —
            // ties may pick different paths than lex-min enumeration.
            let path_cost = |p: &[u32]| {
                let mut c =
                    init[p[0] as usize] as i64 + emit[p[0] as usize][obs[0] as usize] as i64;
                for k in 1..p.len() {
                    c += trans[p[k - 1] as usize][p[k] as usize] as i64
                        + emit[p[k] as usize][obs[k] as usize] as i64;
                }
                c
            };
            assert_eq!(path_cost(&got), path_cost(&want), "s={s} t={t} obs={obs:?}");
        }
    }

    #[test]
    fn viterbi_cost_agrees_with_path_dp() {
        let mut rng = crate::rng::SplitMix64::new(0xBEE);
        for _ in 0..150 {
            let s = 1 + rng.below(4) as usize;
            let t = 1 + rng.below(6) as usize;
            let v = 1 + rng.below(4) as usize;
            let init: Vec<i32> = (0..s).map(|_| rng.below(40) as i32).collect();
            let trans: Vec<Vec<i32>> = (0..s)
                .map(|_| (0..s).map(|_| rng.below(40) as i32).collect())
                .collect();
            let emit: Vec<Vec<i32>> = (0..s)
                .map(|_| (0..v).map(|_| rng.below(40) as i32).collect())
                .collect();
            let obs: Vec<u32> = (0..t).map(|_| rng.below(v as u32)).collect();
            let cost = viterbi_cost(&init, &trans, &emit, &obs).unwrap();
            let path = viterbi(&init, &trans, &emit, &obs).unwrap();
            let mut check =
                init[path[0] as usize] as i64 + emit[path[0] as usize][obs[0] as usize] as i64;
            for k in 1..path.len() {
                check += trans[path[k - 1] as usize][path[k] as usize] as i64
                    + emit[path[k] as usize][obs[k] as usize] as i64;
            }
            assert_eq!(cost, check);
        }
    }

    #[test]
    fn tie_goes_to_the_smaller_state_index() {
        // Two states, all costs zero → lexicographic all-zeros path.
        let path = viterbi(
            &[0, 0],
            &[vec![0, 0], vec![0, 0]],
            &[vec![0], vec![0]],
            &[0, 0],
        )
        .unwrap();
        assert_eq!(path, vec![0, 0]);
        // Symmetric states, obs pushes nothing → path prefers index 0 at the end.
        let path = viterbi(
            &[5, 5],
            &[vec![0, 0], vec![0, 0]],
            &[vec![3], vec![3]],
            &[0],
        )
        .unwrap();
        assert_eq!(path, vec![0]);
    }

    #[test]
    fn malformed_inputs_are_none_not_panic() {
        assert!(viterbi(&[], &[], &[], &[0]).is_none());
        assert!(viterbi(&[0], &[vec![0]], &[vec![0]], &[]).is_none());
        assert!(viterbi(&[0, 0], &[vec![0]], &[vec![0], vec![0]], &[0]).is_none());
        // obs beyond emission vocabulary.
        assert!(viterbi(&[0], &[vec![0]], &[vec![0]], &[7]).is_none());
        // Non-rectangular trans.
        assert!(viterbi(&[0, 0], &[vec![0, 0], vec![0]], &[vec![0], vec![0]], &[0]).is_none());
    }
}
