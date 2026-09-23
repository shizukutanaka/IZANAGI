//! Hopcroft DFA minimization — partition refinement into
//! Nerode-equivalence classes, `O(k·n log n)` for `k` symbols.
//!
//! The input is a *complete* DFA: states `0..n`, a total
//! `delta[s][c]` for every `c` in `0..sigma`, and an
//! `accept` bitmap. Two states merge iff no input string
//! distinguishes them. The result's block ids are
//! canonicalized to the smallest member state, so the answer
//! depends only on the language — not on state ordering.
//!
//! Implementation: the splitter-worklist form — refine each
//! block by the pre-image `δ⁻¹_c(S)` of a splitter block `S`,
//! re-enqueueing the smaller half (Hopcroft's `n log n` trick).
//! Unreachable states minimize among themselves; call
//! [`reachable`] first to prune them if needed.
//!
//! ```
//! use izanagi_kit::dfamin::minimize;
//! // states 0,1,2 all behave identically on {a,b}
//! let m = minimize(3, 2, &[vec![1, 2], vec![0, 2], vec![1, 2]], &[false; 3], 0);
//! assert_eq!(m.n, 1);
//! assert_eq!(m.block_of, vec![0, 0, 0]);
//! ```
//!
//! References: Hopcroft (1971), Knuutila's reinvestigation
//! (TUCS 2001), cp-algorithms "Dfa minimization".

use std::collections::BTreeMap;

/// The minimized DFA: `n` blocks renumbered `0..n`, `start`
/// and `accept` in block ids, `delta[b][c]` total.
pub struct MinDfa {
    /// Block id of each original state — canonicalized to the
    /// smallest member state *before* renumbering.
    pub block_of: Vec<usize>,
    /// Renumbered block id of each original state (`0..n`).
    pub compact_of: Vec<usize>,
    /// Number of equivalence classes.
    pub n: usize,
    /// Start state's compact block id.
    pub start: usize,
    /// `accept[b]` for each block.
    pub accept: Vec<bool>,
    /// `delta[b][c]` in compact block ids.
    pub delta: Vec<Vec<usize>>,
}

/// States reachable from `start` — DFS over `delta`.
pub fn reachable(start: usize, delta: &[Vec<usize>]) -> Vec<bool> {
    let n = delta.len();
    if start >= n {
        return vec![false; n];
    }
    let mut seen = vec![false; n];
    let mut stack = vec![start];
    seen[start] = true;
    while let Some(s) = stack.pop() {
        for &t in &delta[s] {
            if t < n && !seen[t] {
                seen[t] = true;
                stack.push(t);
            }
        }
    }
    seen
}

/// Minimize the complete DFA `(n, sigma, delta, accept)` from
/// `start` — partition refinement with splitter worklist.
/// `delta[s]` must have length `sigma` and values `< n`.
pub fn minimize(
    n: usize,
    sigma: usize,
    delta: &[Vec<usize>],
    accept: &[bool],
    start: usize,
) -> MinDfa {
    if n == 0 {
        return MinDfa {
            block_of: Vec::new(),
            compact_of: Vec::new(),
            n: 0,
            start: 0,
            accept: Vec::new(),
            delta: Vec::new(),
        };
    }
    // blocks: sorted member lists; blk[s] = current block index
    let mut blk = vec![0usize; n];
    let mut blocks: Vec<Vec<usize>> = Vec::new();
    {
        // initial partition: non-accepting (id 0), accepting (id 1)
        let mut acc = Vec::new();
        let mut rej = Vec::new();
        for (s, &a) in accept.iter().enumerate() {
            if a {
                acc.push(s);
            } else {
                rej.push(s);
            }
        }
        if !rej.is_empty() {
            blocks.push(rej);
        }
        if !acc.is_empty() {
            blocks.push(acc);
        }
        for (b, members) in blocks.iter().enumerate() {
            for &s in members {
                blk[s] = b;
            }
        }
    }
    // pre-image map: for splitter S and symbol c, the set of
    // states whose c-image lies in S
    let mut work: Vec<usize> = (0..blocks.len()).collect();
    while let Some(splitter) = work.pop() {
        for c in 0..sigma {
            // group source states by current block: the
            // pre-image of `splitter` under symbol `c`
            let mut hit: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            for s in 0..n {
                if blk[delta[s][c]] == splitter {
                    hit.entry(blk[s]).or_default().push(s);
                }
            }
            for (b, xs) in hit {
                if xs.len() == blocks[b].len() {
                    continue; // all of b hits the splitter — no split
                }
                let in_set: Vec<bool> = {
                    let mut mark = vec![false; n];
                    for &s in &xs {
                        mark[s] = true;
                    }
                    mark
                };
                let mut inside = Vec::new();
                let mut outside = Vec::new();
                for &s in &blocks[b] {
                    if in_set[s] {
                        inside.push(s);
                    } else {
                        outside.push(s);
                    }
                }
                let new_idx = blocks.len();
                // the fresh block takes the smaller half —
                // Hopcroft's n log n trick (re-processing only
                // the smaller side keeps every state's
                // re-queue count logarithmic)
                let (keep, fresh) = if inside.len() <= outside.len() {
                    (outside, inside)
                } else {
                    (inside, outside)
                };
                for &s in &fresh {
                    blk[s] = new_idx;
                }
                blocks[b] = keep;
                blocks.push(fresh);
                // only the fresh (smaller) block is queued —
                // if b was still pending it stays scheduled,
                // which is what the classic proof requires
                work.push(new_idx);
            }
        }
    }
    // canonical block id: smallest member state
    let mut canon = vec![0usize; blocks.len()];
    for (b, members) in blocks.iter().enumerate() {
        canon[b] = members.iter().copied().min().unwrap_or(!0);
    }
    let block_of: Vec<usize> = (0..n).map(|s| canon[blk[s]]).collect();
    // renumber blocks compactly by canonical id
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    order.sort_by_key(|&b| canon[b]);
    let mut renum = vec![0usize; blocks.len()];
    for (i, &b) in order.iter().enumerate() {
        renum[b] = i;
    }
    let k = order.len();
    let compact_of: Vec<usize> = (0..n).map(|s| renum[blk[s]]).collect();
    let mut m_accept = vec![false; k];
    let mut m_delta = vec![vec![0usize; sigma]; k];
    for (i, &b) in order.iter().enumerate() {
        let rep = blocks[b][0];
        m_accept[i] = accept[rep];
        for c in 0..sigma {
            m_delta[i][c] = renum[blk[delta[rep][c]]];
        }
    }
    MinDfa {
        block_of,
        compact_of,
        n: k,
        start: renum[blk[start]],
        accept: m_accept,
        delta: m_delta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Naive signature-iteration oracle: repeatedly refine
    /// blocks by (accept, successors' block) signatures.
    fn naive_minimize(n: usize, sigma: usize, delta: &[Vec<usize>], accept: &[bool]) -> Vec<usize> {
        let mut blk: Vec<usize> = accept.iter().map(|&a| a as usize).collect();
        loop {
            let mut sig: BTreeMap<Vec<usize>, usize> = BTreeMap::new();
            let mut next = vec![0usize; n];
            let mut changed = false;
            for s in 0..n {
                let mut v = Vec::with_capacity(sigma + 1);
                v.push(blk[s]);
                for c in 0..sigma {
                    v.push(blk[delta[s][c]]);
                }
                let id = match sig.get(&v) {
                    Some(&id) => id,
                    None => {
                        let id = sig.len();
                        sig.insert(v, id);
                        id
                    }
                };
                if id != blk[s] {
                    changed = true;
                }
                next[s] = id;
            }
            blk = next;
            if !changed {
                return blk;
            }
        }
    }

    /// Canon ids from a raw partition.
    fn canon_ids(blk: &[usize]) -> Vec<usize> {
        let mut min_of: BTreeMap<usize, usize> = BTreeMap::new();
        for &b in blk {
            // blocks here are arbitrary ids — group by id
            min_of.entry(b).or_insert(!0);
        }
        for (s, &b) in blk.iter().enumerate() {
            let m = min_of.get_mut(&b).unwrap();
            *m = (*m).min(s);
        }
        blk.iter().map(|b| min_of[b]).collect()
    }

    #[test]
    fn known_vectors() {
        // two identical states merge
        let m = minimize(3, 2, &[vec![1, 2], vec![0, 2], vec![1, 2]], &[false; 3], 0);
        assert_eq!(m.n, 1);
        assert_eq!(m.block_of, vec![0, 0, 0]);
        // accept-separated: even-length "aa…" — two classes
        let m = minimize(2, 1, &[vec![1], vec![0]], &[true, false], 0);
        assert_eq!(m.n, 2);
        assert_ne!(m.block_of[0], m.block_of[1]);
        // symmetric rejecters merge: 1 and 2 both reject and
        // send a→0(accept), b→each other — indistinguishable
        let m = minimize(
            3,
            2,
            &[vec![1, 2], vec![0, 2], vec![0, 1]],
            &[true, false, false],
            0,
        );
        assert_eq!(m.n, 2);
        assert_eq!(m.block_of[1], m.block_of[2]);
        assert!(m.accept[m.start]);
        // unreachable states keep their own classes:
        // {0 F}, {1 T}, {2 →0}, {3 →2} are all distinct
        let m = minimize(
            4,
            1,
            &[vec![1], vec![0], vec![0], vec![2]],
            &[false, true, false, false],
            0,
        );
        assert_eq!(m.n, 4);
        assert!(!m.accept[m.start]);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(41);
        for _round in 0..200 {
            let n = (rng.below(10) + 1) as usize;
            let sigma = (rng.below(3) + 1) as usize;
            let delta: Vec<Vec<usize>> = (0..n)
                .map(|_| (0..sigma).map(|_| rng.below(n as u32) as usize).collect())
                .collect();
            let accept: Vec<bool> = (0..n).map(|_| rng.below(2) == 1).collect();
            let start = rng.below(n as u32) as usize;
            let m = minimize(n, sigma, &delta, &accept, start);
            let want = canon_ids(&naive_minimize(n, sigma, &delta, &accept));
            assert_eq!(m.block_of, want, "round {_round}");
            // compact machine simulates identically on a drive
            let mut s = start;
            let mut mb = m.start;
            for _ in 0..20 {
                let c = rng.below(sigma as u32) as usize;
                s = delta[s][c];
                mb = m.delta[mb][c];
                assert_eq!(accept[s], m.accept[mb]);
                assert_eq!(mb, m.compact_of[s]);
            }
        }
    }

    #[test]
    fn reachable_flag() {
        let r = reachable(
            0,
            &[vec![1, 2], vec![3, 3], vec![1, 0], vec![4, 4], vec![2, 2]],
        );
        assert_eq!(r, vec![true, true, true, true, true]);
        let r = reachable(
            0,
            &[vec![1, 2], vec![3, 3], vec![0, 0], vec![4, 4], vec![2, 2]],
        );
        assert_eq!(r, vec![true, true, true, true, true]);
        let r = reachable(0, &[vec![1, 1], vec![2, 2], vec![0, 0]]);
        assert_eq!(r, vec![true, true, true]);
        let r = reachable(2, &[vec![1], vec![0], vec![1]]);
        assert_eq!(r, vec![true, true, true]); // 2→1→0 reaches all
        let r = reachable(2, &[vec![1], vec![1], vec![2]]);
        assert_eq!(r, vec![false, false, true]); // self-loop only
    }
}
