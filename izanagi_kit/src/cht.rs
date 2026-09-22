//! Li Chao tree — the minimum envelope of a set of lines, queried in
//! `O(log X)` on a bounded integer domain.
//!
//! Anywhere lines accumulate and you later ask "cheapest at this `x`":
//! DP transitions of the form `dp[i] = min_j (aⱼ·x_i + bⱼ)`, item prices
//! linear in quantity, per-frame cost functions. Integer-exact
//! (`i128` evaluation), insertion order irrelevant — the *set* of lines
//! fully determines the tree.
//!
//! ```
//! use izanagi_kit::cht::LiChao;
//! let mut t = LiChao::new(-10, 10);
//! t.insert(1, 0);   // y = x
//! t.insert(-1, 0);  // y = -x
//! t.insert(0, 5);   // y = 5
//! assert_eq!(t.query_min(3), Some(-3)); // -x wins
//! assert_eq!(t.query_min(0), Some(0));  // both lines cross at 0 < 5
//! ```

/// Line-envelope query structure on `[lo, hi]`.
pub struct LiChao {
    lo: i64,
    hi: i64,
    /// Heap-indexed nodes; `lines[node-1]` holds the line `y = a·x + b`
    /// currently winning that node's segment midpoint, `None` when the
    /// node was never reached by an insert.
    lines: Vec<Option<(i64, i64)>>,
}

impl LiChao {
    /// Envelope over integer domain `[lo, hi]` (inclusive). `lo > hi`
    /// yields an empty structure that answers `None`.
    pub fn new(lo: i64, hi: i64) -> Self {
        Self {
            lo,
            hi,
            lines: Vec::new(),
        }
    }

    /// Insert the line `y = a·x + b` into the envelope.
    pub fn insert(&mut self, a: i64, b: i64) {
        if self.lo > self.hi {
            return;
        }
        self.insert_at(1, self.lo, self.hi, (a, b));
    }

    /// Minimum `a·x + b` over all inserted lines at integer `x`, `i128`
    /// arithmetic. `None` when `x` is outside `[lo, hi]` or no lines.
    pub fn query_min(&self, x: i64) -> Option<i128> {
        if x < self.lo || x > self.hi || self.lines.is_empty() {
            return None;
        }
        let mut best: Option<i128> = None;
        let (mut node, mut l, mut r) = (1usize, self.lo, self.hi);
        loop {
            if let Some(Some((a, b))) = self.lines.get(node - 1) {
                let v = *a as i128 * x as i128 + *b as i128;
                best = Some(match best {
                    None => v,
                    Some(cur) => cur.min(v),
                });
            }
            if l == r {
                break;
            }
            let m = l + (r - l) / 2;
            if x <= m {
                node *= 2;
                r = m;
            } else {
                node = node * 2 + 1;
                l = m + 1;
            }
        }
        best
    }

    fn set_line(&mut self, node: usize, line: (i64, i64)) {
        let idx = node - 1;
        if idx >= self.lines.len() {
            self.lines.resize(idx + 1, None);
        }
        self.lines[idx] = Some(line);
    }

    fn insert_at(&mut self, node: usize, l: i64, r: i64, mut new: (i64, i64)) {
        let m = l + (r - l) / 2;
        let ev = |line: (i64, i64), x: i64| line.0 as i128 * x as i128 + line.1 as i128;
        match self.lines.get(node - 1).copied().flatten() {
            None => self.set_line(node, new),
            Some(mut cur) => {
                // Keep the line that is lower at the midpoint.
                if ev(new, m) < ev(cur, m) {
                    std::mem::swap(&mut cur, &mut new);
                    self.set_line(node, cur);
                }
                if l == r {
                    return;
                }
                // The loser may still win on a side — descend where it does.
                if ev(new, l) < ev(cur, l) {
                    self.insert_at(node * 2, l, m, new);
                } else if ev(new, r) < ev(cur, r) {
                    self.insert_at(node * 2 + 1, m + 1, r, new);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute-force envelope: min over the raw line list.
    fn oracle(lines: &[(i64, i64)], x: i64) -> i128 {
        lines
            .iter()
            .map(|&(a, b)| a as i128 * x as i128 + b as i128)
            .min()
            .unwrap_or(0)
    }

    #[test]
    fn matches_brute_force_envelope() {
        let mut rng = SplitMix64::new(0x11C4_A0A0);
        for _ in 0..150 {
            let lo = (rng.below(21) as i64) - 10;
            let hi = lo + rng.below(15) as i64;
            let mut t = LiChao::new(lo, hi);
            let k = rng.below(8) as usize;
            let lines: Vec<(i64, i64)> = (0..k)
                .map(|_| ((rng.below(9) as i64) - 4, (rng.below(21) as i64) - 10))
                .collect();
            for &(a, b) in &lines {
                t.insert(a, b);
            }
            for _ in 0..12 {
                let x = lo + rng.below((hi - lo + 1) as u32) as i64;
                if lines.is_empty() {
                    assert_eq!(t.query_min(x), None);
                } else {
                    assert_eq!(t.query_min(x), Some(oracle(&lines, x)));
                }
            }
            assert_eq!(t.query_min(hi + 1), None);
        }
        // Single line sanity + empty structure.
        let mut t = LiChao::new(0, 100);
        assert_eq!(t.query_min(5), None);
        t.insert(2, -3);
        assert_eq!(t.query_min(5), Some(7));
        t.insert(-1, 50);
        assert_eq!(t.query_min(60), Some(-10)); // -60+50 beats 117
                                                // Degenerate domain.
        let mut e = LiChao::new(5, 4);
        e.insert(1, 0);
        assert_eq!(e.query_min(5), None);
    }
}
