//! Lazy segment tree — the canonical range-add aggregate:
//! `add(l, r, x)` plus `sum`/`min`/`max`/`get` over `i64`,
//! `O(log n)` per op. Where [`crate::segtree`] does point
//! updates and [`crate::segbeats`] handles `chmin`/`chmax`
//! clamps, this is the plain affine lazy tree every
//! competitive-programming pipeline rests on.
//!
//! One lazy tag per node — the pending uniform `add` — so
//! composition is trivial (`pending += x`) and `push` needs
//! no tag ordering: the only op that ever lands lazily IS
//! `add`, which commutes with itself.
//!
//! ```
//! use izanagi_kit::lazyseg::LazySeg;
//! let mut t = LazySeg::new(&[1, 2, 3, 4, 5]);
//! t.add(0, 3, 10);
//! assert_eq!(t.sum(0, 5), 45);
//! assert_eq!(t.min(0, 5), 4);
//! assert_eq!(t.get(0), 11);
//! ```
//!
//! References: the atcoder library `lazy_segtree` instantiated
//! at the `(min/max/sum) + add` monoid, cp-algorithms lazy
//! propagation.

const NEG: i64 = i64::MIN;
const POS: i64 = i64::MAX;

/// Range-add lazy segment tree over `i64`.
pub struct LazySeg {
    n: usize,
    sum: Vec<i64>,
    mn: Vec<i64>,
    mx: Vec<i64>,
    /// Pending uniform add per node — `0` = nothing held.
    lazy: Vec<i64>,
}

impl LazySeg {
    /// Build over `a` — `O(n)`.
    pub fn new(a: &[i64]) -> LazySeg {
        let n = a.len().max(1);
        // non-power-of-2 recursion needs 4n heap cells
        let mut t = LazySeg {
            n,
            sum: vec![0; 4 * n],
            mn: vec![POS; 4 * n],
            mx: vec![NEG; 4 * n],
            lazy: vec![0; 4 * n],
        };
        if !a.is_empty() {
            t.build(a, 1, 0, a.len());
        }
        t
    }

    fn build(&mut self, a: &[i64], v: usize, l: usize, r: usize) {
        if r - l == 1 {
            self.sum[v] = a[l];
            self.mn[v] = a[l];
            self.mx[v] = a[l];
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
        self.mn[v] = self.mn[lc].min(self.mn[rc]);
        self.mx[v] = self.mx[lc].max(self.mx[rc]);
    }

    /// Fold a pending `+x` into node `v` spanning `len` leaves.
    fn apply(&mut self, v: usize, len: usize, x: i64) {
        self.sum[v] += x * len as i64;
        self.mn[v] += x;
        self.mx[v] += x;
        if len > 1 {
            self.lazy[v] += x;
        }
    }

    fn push(&mut self, v: usize, len: usize) {
        if self.lazy[v] == 0 || len == 1 {
            return;
        }
        let x = self.lazy[v];
        // left child spans floor(len/2) leaves — the midpoint
        // m=(l+r)/2 lands left-of-center for odd len
        let half = len / 2;
        self.apply(2 * v, half, x);
        self.apply(2 * v + 1, len - half, x);
        self.lazy[v] = 0;
    }

    fn n_is_empty(&self) -> bool {
        self.mx[1] == NEG
    }

    /// `a[i] += x` for `i` in `[l, r)`.
    pub fn add(&mut self, l: usize, r: usize, x: i64) {
        if !self.n_is_empty() && x != 0 {
            self.add_rec(l, r, x, 1, 0, self.n);
        }
    }

    fn add_rec(&mut self, ql: usize, qr: usize, x: i64, v: usize, l: usize, r: usize) {
        if qr <= l || r <= ql {
            return;
        }
        if ql <= l && r <= qr {
            self.apply(v, r - l, x);
            return;
        }
        self.push(v, r - l);
        let m = (l + r) / 2;
        self.add_rec(ql, qr, x, 2 * v, l, m);
        self.add_rec(ql, qr, x, 2 * v + 1, m, r);
        self.pull(v);
    }

    /// Sum of `a[l..r)`.
    pub fn sum(&mut self, l: usize, r: usize) -> i64 {
        if self.n_is_empty() {
            return 0;
        }
        self.sum_rec(l, r, 1, 0, self.n).0
    }

    /// Minimum of `a[l..r)` — `i64::MAX` on the empty range.
    pub fn min(&mut self, l: usize, r: usize) -> i64 {
        if self.n_is_empty() {
            return POS;
        }
        self.sum_rec(l, r, 1, 0, self.n).1
    }

    /// Maximum of `a[l..r)` — `i64::MIN` on the empty range.
    pub fn max(&mut self, l: usize, r: usize) -> i64 {
        if self.n_is_empty() {
            return NEG;
        }
        self.sum_rec(l, r, 1, 0, self.n).2
    }

    /// `a[i]` — descends so lazy tags resolve.
    pub fn get(&mut self, i: usize) -> i64 {
        self.sum(i, i + 1)
    }

    /// Point assign — implemented as `add(i, i+1, x - get(i))`
    /// so the lazy machinery stays single-tag.
    pub fn set(&mut self, i: usize, x: i64) {
        let cur = self.get(i);
        self.add(i, i + 1, x - cur);
    }

    /// Combined `(sum, min, max)` read — one descent, pushes on
    /// the way down so covered children's ledgers are exact.
    fn sum_rec(&mut self, ql: usize, qr: usize, v: usize, l: usize, r: usize) -> (i64, i64, i64) {
        if qr <= l || r <= ql {
            return (0, POS, NEG);
        }
        if ql <= l && r <= qr {
            return (self.sum[v], self.mn[v], self.mx[v]);
        }
        self.push(v, r - l);
        let m = (l + r) / 2;
        let (s1, mn1, mx1) = self.sum_rec(ql, qr, 2 * v, l, m);
        let (s2, mn2, mx2) = self.sum_rec(ql, qr, 2 * v + 1, m, r);
        (s1 + s2, mn1.min(mn2), mx1.max(mx2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let mut t = LazySeg::new(&[1, 2, 3, 4, 5]);
        t.add(0, 3, 10);
        assert_eq!(t.sum(0, 5), 45);
        assert_eq!(t.min(0, 5), 4);
        assert_eq!(t.max(0, 5), 13);
        assert_eq!(t.get(0), 11);
        assert_eq!(t.get(4), 5);
        // empty and singleton
        let mut e = LazySeg::new(&[]);
        e.add(0, 0, 7);
        assert_eq!(e.sum(0, 0), 0);
        assert_eq!(e.min(0, 0), POS);
        let mut s = LazySeg::new(&[9]);
        s.add(0, 1, 1);
        assert_eq!(s.get(0), 10);
        // point set
        t.set(2, -5);
        assert_eq!(t.get(2), -5);
        assert_eq!(t.min(0, 5), -5);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(19);
        for round in 0..150 {
            let n = (rng.below(24) + 1) as usize;
            let mut t = LazySeg::new(&vec![0; n]);
            let mut oracle = vec![0i64; n];
            for _ in 0..80 {
                match rng.below(4) {
                    0 => {
                        let i = rng.below(n as u32) as usize;
                        let x = rng.below(21) as i64 - 10;
                        t.set(i, x);
                        oracle[i] = x;
                    }
                    1 => {
                        let l = rng.below((n + 1) as u32) as usize;
                        let r = l + rng.below((n + 1 - l) as u32) as usize;
                        let x = rng.below(21) as i64 - 10;
                        t.add(l, r, x);
                        for v in oracle[l..r].iter_mut() {
                            *v += x;
                        }
                    }
                    2 => {
                        let l = rng.below((n + 1) as u32) as usize;
                        let r = l + rng.below((n + 1 - l) as u32) as usize;
                        assert_eq!(
                            t.sum(l, r),
                            oracle[l..r].iter().sum(),
                            "sum {l}..{r} round {round}"
                        );
                        let want_min = oracle[l..r].iter().min().copied().unwrap_or(POS);
                        let want_max = oracle[l..r].iter().max().copied().unwrap_or(NEG);
                        assert_eq!(t.min(l, r), want_min, "min {l}..{r} round {round}");
                        assert_eq!(t.max(l, r), want_max, "max {l}..{r} round {round}");
                    }
                    _ => {
                        let i = rng.below(n as u32) as usize;
                        assert_eq!(t.get(i), oracle[i], "get {i} round {round}");
                    }
                }
            }
        }
    }

    #[test]
    fn boundaries() {
        // negative values, full-range adds, i64 extremes
        let mut t = LazySeg::new(&[-5, -5, -5]);
        t.add(0, 3, 5);
        assert_eq!(t.sum(0, 3), 0);
        t.add(1, 2, i64::MAX / 2);
        assert_eq!(t.get(1), i64::MAX / 2);
        t.add(0, 1, i64::MIN / 2);
        assert_eq!(t.get(0), -i64::MAX / 2 - 1); // i64::MIN/2 exact
                                                 // reads between writes stay consistent after deep lazy descent
        t.add(0, 3, 3);
        assert_eq!(t.sum(0, 3), 3 * 3 + i64::MAX / 2 + (-i64::MAX / 2 - 1));
    }
}
