//! Lazy segment tree beats — `range add` composed with the
//! `chmin`/`chmax` clamps of [`crate::segbeats`]. This is the
//! deferred "beats + lazy add" variant: adding `add` forces a
//! real lazy tag, because a pending add must reach a child's
//! sums *before* the parent's clamp `mx`/`mn` bounds are
//! applied to it — otherwise a child whose stored `mx` is
//! stale-low would be clamped against the pre-add ceiling.
//!
//! Node state: `sum`, `mx`, `smax`, `cmax`, `mn`, `smin`,
//! `cmin` (the beats ledger) plus `lz`, a pending range-add.
//! `push` replays the parent's `lz` into both children via
//! `apply_add` first, then applies the clamp — order matters.
//!
//! ```
//! use izanagi_kit::seglazy::SegLazy;
//! let mut t = SegLazy::new(&[3, 1, 4, 1, 5]);
//! t.add(0, 5, 2); // [5, 3, 6, 3, 7]
//! t.chmin(0, 5, 5); // [5, 3, 5, 3, 5]
//! assert_eq!(t.sum(0, 5), 21);
//! t.chmax(0, 3, 4); // [5, 4, 5, 3, 5]
//! assert_eq!(t.sum(0, 5), 22);
//! ```
//!
//! References: J. Dai (jiry_2) "Segment Tree Beats";
//! library-checker `range_chmin_chmax_add_range_sum`.

/// Segment tree beats with lazy `range add`, `chmin`, `chmax`,
/// and `range sum` over `i64`. All ranges half-open `[l, r)`.
#[derive(Clone, Debug)]
pub struct SegLazy {
    n: usize,
    sum: Vec<i64>,
    mx: Vec<i64>,
    smax: Vec<i64>,
    cmax: Vec<u32>,
    mn: Vec<i64>,
    smin: Vec<i64>,
    cmin: Vec<u32>,
    lz: Vec<i64>,
}

const NEG: i64 = i64::MIN;
const POS: i64 = i64::MAX;

impl SegLazy {
    /// Build over `a` — `O(n)`.
    pub fn new(a: &[i64]) -> SegLazy {
        let n = a.len().max(1);
        let mut t = SegLazy {
            n,
            sum: vec![0; 4 * n],
            mx: vec![NEG; 4 * n],
            smax: vec![NEG; 4 * n],
            cmax: vec![0; 4 * n],
            mn: vec![POS; 4 * n],
            smin: vec![POS; 4 * n],
            cmin: vec![0; 4 * n],
            lz: vec![0; 4 * n],
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

    /// Shift every value in `v`'s range by `x`. The
    /// `NEG`/`POS` sentinels in the second-extreme slots are
    /// left untouched — adding to them would destroy their
    /// "no second extreme" meaning.
    fn apply_add(&mut self, v: usize, len: usize, x: i64) {
        self.sum[v] += x * len as i64;
        if self.mx[v] != NEG {
            self.mx[v] += x;
        }
        if self.smax[v] != NEG {
            self.smax[v] += x;
        }
        if self.mn[v] != POS {
            self.mn[v] += x;
        }
        if self.smin[v] != POS {
            self.smin[v] += x;
        }
        self.lz[v] += x;
    }

    /// Fold `x` into node `v` as `chmin`.
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

    /// Propagate `v`'s pending add, then its extrema, into the
    /// children — add first so children's sums reflect their
    /// current values before the parent's clamp bounds them.
    fn push(&mut self, v: usize, len: usize) {
        if len == 1 {
            return;
        }
        let mlen = len / 2;
        if self.lz[v] != 0 {
            let z = self.lz[v];
            self.apply_add(2 * v, mlen, z);
            self.apply_add(2 * v + 1, len - mlen, z);
            self.lz[v] = 0;
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

    /// `a[i] += x` for `i` in `[l, r)`.
    pub fn add(&mut self, l: usize, r: usize, x: i64) {
        if !self.n_is_empty() {
            self.add_rec(l, r, x, 1, 0, self.n);
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

    /// Value at a single index — `a[i]`.
    pub fn get(&mut self, i: usize) -> i64 {
        self.sum(i, i + 1)
    }

    fn n_is_empty(&self) -> bool {
        self.cmax[1] == 0
    }

    fn add_rec(&mut self, ql: usize, qr: usize, x: i64, v: usize, l: usize, r: usize) {
        if qr <= l || r <= ql {
            return;
        }
        if ql <= l && r <= qr {
            self.apply_add(v, r - l, x);
            return;
        }
        self.push(v, r - l);
        let m = (l + r) / 2;
        self.add_rec(ql, qr, x, 2 * v, l, m);
        self.add_rec(ql, qr, x, 2 * v + 1, m, r);
        self.pull(v);
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
        self.push(v, r - l);
        let m = (l + r) / 2;
        self.sum_rec(ql, qr, 2 * v, l, m) + self.sum_rec(ql, qr, 2 * v + 1, m, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let mut t = SegLazy::new(&[3, 1, 4, 1, 5]);
        t.add(0, 5, 2);
        assert_eq!(t.sum(0, 5), 24);
        t.chmin(0, 5, 5);
        assert_eq!(t.sum(0, 5), 21);
        t.chmax(0, 3, 4);
        assert_eq!(t.sum(0, 5), 22);
        assert_eq!(t.get(2), 5);
        let mut e = SegLazy::new(&[]);
        assert_eq!(e.sum(0, 0), 0);
        e.add(0, 0, 9);
        assert_eq!(e.get(0), 0);
    }

    /// Naive per-element oracle: apply every op directly.
    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0xBEA7);
        for _ in 0..120 {
            let n = 1 + rng.below(12) as usize;
            let a: Vec<i64> = (0..n).map(|_| i64::from(rng.below(41)) - 20).collect();
            let mut t = SegLazy::new(&a);
            let mut o = a.clone();
            for _ in 0..300 {
                let l = rng.below(n as u32) as usize;
                let r = l + rng.below((n - l) as u32) as usize;
                match rng.below(4) {
                    0 => {
                        let x = i64::from(rng.below(21)) - 10;
                        t.add(l, r, x);
                        o[l..r].iter_mut().for_each(|v| *v += x);
                    }
                    1 => {
                        let x = i64::from(rng.below(41)) - 20;
                        t.chmin(l, r, x);
                        o[l..r].iter_mut().for_each(|v| *v = (*v).min(x));
                    }
                    2 => {
                        let x = i64::from(rng.below(41)) - 20;
                        t.chmax(l, r, x);
                        o[l..r].iter_mut().for_each(|v| *v = (*v).max(x));
                    }
                    _ => {
                        assert_eq!(t.sum(l, r), o[l..r].iter().sum::<i64>());
                        let i = rng.below(n as u32) as usize;
                        assert_eq!(t.get(i), o[i]);
                    }
                }
            }
            for (i, &v) in o.iter().enumerate() {
                assert_eq!(t.get(i), v);
            }
        }
    }

    /// The add-before-clamp order inside `push` is exercised
    /// when a node holds both a pending add and a lazy clamp.
    #[test]
    fn add_then_clamp_interleaving() {
        let mut t = SegLazy::new(&[0; 6]);
        let mut o = [0i64; 6];
        let mut rng = SplitMix64::new(42);
        for _ in 0..2000 {
            let l = rng.below(6) as usize;
            let r = l + rng.below((6 - l) as u32) as usize;
            match rng.below(3) {
                0 => {
                    let x = i64::from(rng.below(5));
                    t.add(l, r, x);
                    o[l..r].iter_mut().for_each(|v| *v += x);
                }
                1 => {
                    let x = i64::from(rng.below(10));
                    t.chmin(l, r, x);
                    o[l..r].iter_mut().for_each(|v| *v = (*v).min(x));
                }
                _ => {
                    let x = i64::from(rng.below(3));
                    t.chmax(l, r, x);
                    o[l..r].iter_mut().for_each(|v| *v = (*v).max(x));
                }
            }
        }
        for (i, &v) in o.iter().enumerate() {
            assert_eq!(t.get(i), v);
        }
        assert_eq!(t.sum(0, 6), o.iter().sum());
    }
}
