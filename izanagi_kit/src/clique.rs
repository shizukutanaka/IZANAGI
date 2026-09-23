//! Maximal cliques — Bron–Kerbosch enumeration with pivoting over
//! `u64` adjacency masks (≤ 64 vertices). Reports every clique that
//! cannot be extended, plus `max_clique` for the largest one.
//!
//! Pivot rule (Tomita): branch only on `P \ N(u)` where `u`
//! maximizes `|P ∩ N(u)|` — prunes the recursion to near-optimal
//! depth on sparse graphs.
//!
//! Output is canonical: cliques are emitted in discovery order but
//! `maximal_cliques` sorts the mask list, so the result is a pure
//! function of the input masks.
//!
//! ```
//! use izanagi_kit::clique;
//! // Triangle 0-1-2 plus pendant 3 on vertex 0.
//! let adj = [0b1110u64, 0b0101, 0b0011, 0b0001];
//! assert_eq!(clique::maximal_cliques(&adj), vec![0b0111, 0b1001]);
//! assert_eq!(clique::max_clique(&adj), 0b0111);
//! ```

/// All maximal cliques as sorted `u64` masks. Deterministic.
pub fn maximal_cliques(adj: &[u64]) -> Vec<u64> {
    let n = adj.len().min(64);
    let p: u64 = if n == 64 { !0 } else { (1u64 << n) - 1 };
    // Vertices with no adjacency bits inside the universe still
    // count as singleton maximal cliques.
    let mut out = Vec::new();
    bk(adj, n, 0, p, 0, &mut out);
    out.sort_unstable();
    out
}

/// One maximum-cardinality clique (smallest mask on ties), or `0`
/// when the graph is empty.
pub fn max_clique(adj: &[u64]) -> u64 {
    maximal_cliques(adj)
        .into_iter()
        // (size, !mask): largest size, then largest !mask = smallest mask.
        .max_by_key(|m| (m.count_ones(), !*m))
        .unwrap_or(0)
}

/// Clique count including non-maximal ones is exponential; callers
/// get cardinality via `max_clique(...).count_ones()`.
fn bk(adj: &[u64], n: usize, r: u64, mut p: u64, mut x: u64, out: &mut Vec<u64>) {
    if p == 0 {
        if x == 0 {
            out.push(r);
        }
        return;
    }
    // Pivot: u ∈ P ∪ X maximizing |P ∩ N(u)|.
    let union = p | x;
    let mut pivot: u64 = 0;
    let mut best: u32 = 0;
    let mut have_pivot = false;
    let mut u = union;
    while u != 0 {
        let v = u.trailing_zeros() as usize;
        u &= u - 1;
        let cnt = (p & nbr(adj, n, v)).count_ones();
        if !have_pivot || cnt > best {
            best = cnt;
            pivot = v as u64;
            have_pivot = true;
        }
    }
    let mut todo = p & !nbr(adj, n, pivot as usize);
    while todo != 0 {
        let v = todo.trailing_zeros() as usize;
        todo &= todo - 1;
        let bit = 1u64 << v;
        bk(adj, n, r | bit, p & nbr(adj, n, v), x & nbr(adj, n, v), out);
        p &= !bit;
        x |= bit;
    }
}

fn nbr(adj: &[u64], n: usize, v: usize) -> u64 {
    if v >= n {
        return 0;
    }
    let a = adj[v];
    if n == 64 {
        a
    } else {
        a & ((1u64 << n) - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn is_clique(adj: &[u64], mask: u64) -> bool {
        for (i, &a) in adj.iter().enumerate() {
            if mask >> i & 1 == 1 {
                // every other set bit must be adjacent to i
                let rest = mask & !(1u64 << i);
                if a & rest != rest {
                    return false;
                }
            }
        }
        mask != 0
    }

    fn is_maximal(adj: &[u64], mask: u64) -> bool {
        if !is_clique(adj, mask) {
            return false;
        }
        let n = adj.len();
        for v in 0..n {
            if mask >> v & 1 == 0 && is_clique(adj, mask | (1u64 << v)) {
                return false;
            }
        }
        true
    }

    fn brute_max_size(adj: &[u64]) -> u32 {
        let n = adj.len();
        let mut best = 0;
        for m in 1u64..(1u64 << n) {
            if is_clique(adj, m) && m.count_ones() > best {
                best = m.count_ones();
            }
        }
        best
    }

    #[test]
    fn basics() {
        // Triangle 0-1-2 + pendant 3 on vertex 0.
        let adj = [0b1110u64, 0b0101, 0b0011, 0b0001];
        let ms = maximal_cliques(&adj);
        assert_eq!(ms, vec![0b0111, 0b1001]);
        assert_eq!(max_clique(&adj), 0b0111);
        // Empty graph on 3 vertices → three singletons.
        let empty = [0u64; 3];
        assert_eq!(maximal_cliques(&empty), vec![1, 2, 4]);
        assert_eq!(max_clique(&empty).count_ones(), 1);
        // Complete K4 → one clique.
        let k4 = [0b1110u64, 0b1101, 0b1011, 0b0111];
        assert_eq!(maximal_cliques(&k4), vec![0b1111]);
    }

    #[test]
    fn oracle_small() {
        let mut rng = SplitMix64::new(0xac1e_bd3f_1234_5678);
        for _case in 0..150 {
            let n = 1 + rng.below(8) as usize;
            let mut adj = vec![0u64; n];
            for i in 0..n {
                for j in (i + 1)..n {
                    if rng.below(2) == 0 {
                        adj[i] |= 1 << j;
                        adj[j] |= 1 << i;
                    }
                }
            }
            let ms = maximal_cliques(&adj);
            // Every reported clique is maximal, and every maximal
            // clique is reported (brute scan over all subsets).
            let mut expect: Vec<u64> = (1u64..(1u64 << n))
                .filter(|&m| is_maximal(&adj, m))
                .collect();
            expect.sort_unstable();
            assert_eq!(ms, expect);
            assert_eq!(max_clique(&adj).count_ones(), brute_max_size(&adj));
        }
    }
}
