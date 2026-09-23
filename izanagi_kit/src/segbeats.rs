//! Segment tree beats — a segment tree that answers
//! `chmin`/`chmax` (clamp every element of a range from one
//! side) plus a range-sum query in amortized `O(log n)`,
//! where a lazy range-clamp would need `O(n)` and a plain
//! lazy segment tree cannot compose the two at all.
//!
//! Each node tracks the range's `sum`, `max`, second `max`,
//! `cnt_max`, `min`, second `min`, `cnt_min`. A `chmin(x)`
//! lands lazily exactly when `smax < x < max` — then only the
//! `cnt_max` leaders change and `sum` shifts by
//! `(x − max)·cnt_max` without touching children; `chmax` is
//! symmetric. Beats refers to the two-sided min/max ledger
//! that makes the amortization work: total work is bounded by
//! the number of distinct extrema, not the range size.
//!
//! ```
//! use izanagi_kit::segbeats::SegBeats;
//! let mut t = SegBeats::new(&[3, 1, 4, 1, 5]);
//! t.chmin(0, 5, 3); // [3, 1, 3, 1, 3]
//! assert_eq!(t.sum(0, 5), 11);
//! t.chmax(0, 3, 2); // [3, 2, 3, 1, 3]
//! assert_eq!(t.sum(0, 5), 12);
//! ```
//!
//! References: J. Dai (jiry_2) "Segment Tree Beats" (Codeforces
//! blog series, 2018), library-checker `range_chmin_chmax_add_
//! range_sum` minus the add tag.

/// Static segment tree over `i64` with one-sided range clamps
/// and range-sum queries. All ranges are half-open `[l, r)`.
#[derive(Clone, Debug)]
pub struct SegBeats {
    n: usize,
    sum: Vec<i64>,
    mx: Vec<i64>,
    smax: Vec<i64>,
    cmax: Vec<u32>,
    mn: Vec<i64>,
    smin: Vec<i64>,
    cmin: Vec<u32>,
}

const NEG: i64 = i64::MIN;
const POS: i64 = i64::MAX;

impl SegBeats {
    /// Build over `a` — `O(n)` heap of merges.
    pub fn new(a: &[i64]) -> SegBeats {
        let n = a.len().max(1);
        // non-power-of-2 recursion needs 4n heap cells, not 2n
        let mut t = SegBeats {
            n,
            sum: vec![0; 4 * n],
            mx: vec![NEG; 4 * n],
            smax: vec![NEG; 4 * n],
            cmax: vec![0; 4 * n],
            mn: vec![POS; 4 * n],
            smin: vec![POS; 4 * n],
            cmin: vec![0; 4 * n],
        };
        if !a.is_empty() {
            t.build(a, 1, 0, a.len());
        }
        t
    }

    fn build(&mut self, a: &[i64], v: usize, l: usize, r: usize) {
        if r - l == 1 {
            self.sum[v] = a[l];
            self.mx[v] = a[l];
            self.cmax[v] = 1;
            self.mn[v] = a[l];
            self.cmin[v] = 1;
            return;
        }
        let m = (l + r) / 2;
        self.build(a, 2 * v, l, m);
        self.build(a, 2 * v + 1, m, r);
        self.pull(v);
    }

    fn pull(&mut self, v: usize) {
        let (lc, rc) = (2 * v, 2 * v + 1);
        self.sum[v] = self.sum[lc] + self.sum[rc];
        // max side
        if self.mx[lc] == self.mx[rc] {
            self.mx[v] = self.mx[lc];
            self.cmax[v] = self.cmax[lc] + self.cmax[rc];
            self.smax[v] = self.smax[lc].max(self.smax[rc]);
        } else if self.mx[lc] > self.mx[rc] {
            self.mx[v] = self.mx[lc];
            self.cmax[v] = self.cmax[lc];
            self.smax[v] = self.smax[lc].max(self.mx[rc]);
        } else {
            self.mx[v] = self.mx[rc];
            self.cmax[v] = self.cmax[rc];
            self.smax[v] = self.smax[rc].max(self.mx[lc]);
        }
        // min side (mirror)
        if self.mn[lc] == self.mn[rc] {
            self.mn[v] = self.mn[lc];
            self.cmin[v] = self.cmin[lc] + self.cmin[rc];
            self.smin[v] = self.smin[lc].min(self.smin[rc]);
        } else if self.mn[lc] < self.mn[rc] {
            self.mn[v] = self.mn[lc];
            self.cmin[v] = self.cmin[lc];
            self.smin[v] = self.smin[lc].min(self.mn[rc]);
        } else {
            self.mn[v] = self.mn[rc];
            self.cmin[v] = self.cmin[rc];
            self.smin[v] = self.smin[rc].min(self.mn[lc]);
        }
    }

    /// Fold `x` into node `v` as `chmin` — the lazy core:
    /// only the `cnt_max` leaders move, and the min-side
    /// ledgers adjust where the leaders were also the minima.
    fn update_max(&mut self, v: usize, x: i64) {
        self.sum[v] += (x - self.mx[v]) * self.cmax[v] as i64;
        if self.mx[v] == self.mn[v] {
            self.mn[v] = x;
        }
        if self.mx[v] == self.smin[v] {
            self.smin[v] = x;
        }
        self.mx[v] = x;
    }

    /// Fold `x` into node `v` as `chmax` (mirror).
    fn update_min(&mut self, v: usize, x: i64) {
        self.sum[v] += (x - self.mn[v]) * self.cmin[v] as i64;
        if self.mn[v] == self.mx[v] {
            self.mx[v] = x;
        }
        if self.mn[v] == self.smax[v] {
            self.smax[v] = x;
        }
        self.mn[v] = x;
    }

    /// Push `v`'s accumulated extrema into its children — the
    /// standard beats push needs no explicit lazy tag: after a
    /// lazy `chmin` the parent's `mx` already IS the ceiling the
    /// children must respect, so clamping each child to `mx[v]`
    /// (and to `mn[v]` on the floor side) replays exactly the
    /// ops that landed lazily here.
    fn push(&mut self, v: usize, len: usize) {
        if len == 1 {
            return;
        }
        let (mx, mn) = (self.mx[v], self.mn[v]);
        for c in [2 * v, 2 * v + 1] {
            if self.mx[c] > mx {
                self.update_max(c, mx);
            }
            if self.mn[c] < mn {
                self.update_min(c, mn);
            }
        }
    }

    /// `a[i] = min(a[i], x)` for `i` in `[l, r)`.
    pub fn chmin(&mut self, l: usize, r: usize, x: i64) {
        if !self.n_is_empty() {
            self.chmin_rec(l, r, x, 1, 0, self.n);
        }
    }

    /// `a[i] = max(a[i], x)` for `i` in `[l, r)`.
    pub fn chmax(&mut self, l: usize, r: usize, x: i64) {
        if !self.n_is_empty() {
            self.chmax_rec(l, r, x, 1, 0, self.n);
        }
    }

    /// Sum of `a[l..r)`.
    pub fn sum(&mut self, l: usize, r: usize) -> i64 {
        if self.n_is_empty() {
            return 0;
        }
        self.sum_rec(l, r, 1, 0, self.n)
    }

    /// Value at a single index — `a[l]`.
    pub fn get(&mut self, i: usize) -> i64 {
        self.sum(i, i + 1)
    }

    fn n_is_empty(&self) -> bool {
        self.cmax[1] == 0
    }

    fn chmin_rec(&mut self, ql: usize, qr: usize, x: i64, v: usize, l: usize, r: usize) {
        if qr <= l || r <= ql || self.mx[v] <= x {
            return;
        }
        if ql <= l && r <= qr && self.smax[v] < x {
            self.update_max(v, x);
            return;
        }
        self.push(v, r - l);
        let m = (l + r) / 2;
        self.chmin_rec(ql, qr, x, 2 * v, l, m);
        self.chmin_rec(ql, qr, x, 2 * v + 1, m, r);
        self.pull(v);
    }

    fn chmax_rec(&mut self, ql: usize, qr: usize, x: i64, v: usize, l: usize, r: usize) {
        if qr <= l || r <= ql || self.mn[v] >= x {
            return;
        }
        if ql <= l && r <= qr && self.smin[v] > x {
            self.update_min(v, x);
            return;
        }
        self.push(v, r - l);
        let m = (l + r) / 2;
        self.chmax_rec(ql, qr, x, 2 * v, l, m);
        self.chmax_rec(ql, qr, x, 2 * v + 1, m, r);
        self.pull(v);
    }

    fn sum_rec(&mut self, ql: usize, qr: usize, v: usize, l: usize, r: usize) -> i64 {
        if qr <= l || r <= ql {
            return 0;
        }
        if ql <= l && r <= qr {
            return self.sum[v];
        }
        // children below a lazily-updated node hold stale sums —
        // the read must push, exactly like a mutation descent
        self.push(v, r - l);
        let m = (l + r) / 2;
        self.sum_rec(ql, qr, 2 * v, l, m) + self.sum_rec(ql, qr, 2 * v + 1, m, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Naive per-element oracle.
    #[derive(Clone)]
    struct Oracle(Vec<i64>);
    impl Oracle {
        fn chmin(&mut self, l: usize, r: usize, x: i64) {
            for v in self.0[l..r].iter_mut() {
                *v = (*v).min(x);
            }
        }
        fn chmax(&mut self, l: usize, r: usize, x: i64) {
            for v in self.0[l..r].iter_mut() {
                *v = (*v).max(x);
            }
        }
        fn sum(&self, l: usize, r: usize) -> i64 {
            self.0[l..r].iter().sum()
        }
    }

    #[test]
    fn basics() {
        let mut t = SegBeats::new(&[3, 1, 4, 1, 5]);
        t.chmin(0, 5, 3);
        assert_eq!(t.sum(0, 5), 11);
        t.chmax(0, 3, 2);
        assert_eq!(t.sum(0, 5), 12);
        assert_eq!(t.get(0), 3);
        assert_eq!(t.get(3), 1);
        // degenerate: empty tree, singleton, full-width clamp
        let mut e = SegBeats::new(&[]);
        e.chmin(0, 0, 5);
        assert_eq!(e.sum(0, 0), 0);
        let mut s = SegBeats::new(&[7]);
        s.chmax(0, 1, 10);
        assert_eq!(s.get(0), 10);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(91);
        for _ in 0..200 {
            let n = (rng.below(24) + 1) as usize;
            let a: Vec<i64> = (0..n).map(|_| rng.below(40) as i64).collect();
            let mut t = SegBeats::new(&a);
            let mut o = Oracle(a);
            for _ in 0..60 {
                let mut l = rng.below(n as u32) as usize;
                let mut r = rng.below(n as u32) as usize;
                if l > r {
                    std::mem::swap(&mut l, &mut r);
                }
                r += usize::from(l == r);
                let x = rng.below(45) as i64;
                match rng.below(3) {
                    0 => {
                        t.chmin(l, r, x);
                        o.chmin(l, r, x);
                    }
                    1 => {
                        t.chmax(l, r, x);
                        o.chmax(l, r, x);
                    }
                    _ => {
                        let (ql, qr) = (l, r.min(n));
                        assert_eq!(t.sum(ql, qr), o.sum(ql, qr), "n={n} l={l} r={qr}");
                    }
                }
            }
            for i in 0..n {
                assert_eq!(t.get(i), o.0[i], "i={i}");
            }
        }
    }

    #[test]
    fn extremes() {
        // identical elements: second extremum sentinels must
        // not corrupt the lazy fold
        let mut t = SegBeats::new(&[5; 16]);
        t.chmin(0, 16, 4);
        assert_eq!(t.sum(0, 16), 64);
        t.chmax(3, 10, 9);
        assert_eq!(t.sum(0, 16), 4 * 16 + 5 * 7);
        // negatives
        let mut t = SegBeats::new(&[-5, -2, -9, -1]);
        t.chmax(0, 4, -3);
        assert_eq!(t.sum(0, 4), -3 - 2 - 3 - 1);
        t.chmin(1, 3, -7);
        assert_eq!(t.sum(0, 4), -3 - 7 - 7 - 1);
    }
}
