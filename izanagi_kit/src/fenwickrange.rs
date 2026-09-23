//! Range-update Fenwick trees — the two-BIT constructions that extend
//! [`crate::fenwick`] from point updates to range updates. Two
//! flavors, both `O(log n)` per operation and pure `i64` throughout:
//!
//! * [`RangePoint`]: range **add** + point **query**
//!   (`add(l, r, v)` adds `v` to every `a[i]`, `l <= i < r`).
//! * [`RangeSum`]: range **add** + range **sum query**
//!   (prefix sums over the implicit array via `B1`/`B2` pair:
//!   `sum(0..=r) = sum(B1)·(r+1) − sum(B2)`).
//!
//! ```
//! use izanagi_kit::fenwickrange::{RangePoint, RangeSum};
//! let mut p = RangePoint::new(8);
//! p.add(2, 6, 5); // a[2..6] += 5
//! assert_eq!(p.get(3), 5);
//! assert_eq!(p.get(6), 0);
//! let mut s = RangeSum::new(8);
//! s.add(1, 5, 7);
//! assert_eq!(s.sum(0, 8), 28);
//! assert_eq!(s.sum(2, 4), 14);
//! ```

/// Range-add + point-query Fenwick tree over an implicit `i64` array.
pub struct RangePoint {
    t: Vec<i64>,
}

impl RangePoint {
    /// Zeroed array of `n` cells.
    pub fn new(n: usize) -> Self {
        Self { t: vec![0; n + 1] }
    }

    fn bump(&mut self, mut i: usize, v: i64) {
        while i < self.t.len() {
            self.t[i] += v;
            i += i & (!i + 1);
        }
    }

    /// Add `v` to every element of `a[l..r)` (half-open).
    pub fn add(&mut self, l: usize, r: usize, v: i64) {
        if l >= r || l >= self.t.len() {
            return;
        }
        let r = r.min(self.t.len());
        self.bump(l + 1, v);
        self.bump(r + 1, -v);
    }

    /// Current value of `a[i]`.
    pub fn get(&self, i: usize) -> i64 {
        let mut i = i + 1;
        let mut s = 0i64;
        while i > 0 {
            s += self.t[i];
            i -= i & (!i + 1);
        }
        s
    }

    /// Cell count.
    pub fn len(&self) -> usize {
        self.t.len() - 1
    }

    /// True when `n == 0`.
    pub fn is_empty(&self) -> bool {
        self.t.len() <= 1
    }
}

/// Range-add + range-sum Fenwick pair over an implicit `i64` array.
pub struct RangeSum {
    b1: Vec<i64>,
    b2: Vec<i64>,
}

impl RangeSum {
    /// Zeroed array of `n` cells.
    pub fn new(n: usize) -> Self {
        Self {
            b1: vec![0; n + 1],
            b2: vec![0; n + 1],
        }
    }

    fn bump(t: &mut [i64], mut i: usize, v: i64) {
        while i < t.len() {
            t[i] += v;
            i += i & (!i + 1);
        }
    }

    fn prefix(t: &[i64], mut i: usize) -> i64 {
        let mut s = 0i64;
        while i > 0 {
            s += t[i];
            i -= i & (!i + 1);
        }
        s
    }

    /// Add `v` to every element of `a[l..r)` (half-open).
    pub fn add(&mut self, l: usize, r: usize, v: i64) {
        if l >= r || l >= self.b1.len() {
            return;
        }
        let r = r.min(self.b1.len());
        // a[i] += v for i in [l, r) (0-based half-open). With the
        // difference-array view (a[i] = prefix d), that is
        // d[l] += v, d[r] -= v. Storing d in B1 and (d[i]·i) in B2:
        //   P(x) = sum_{j<x} a[j] = sum(B1,x)·x − sum(B2,x).
        // BIT positions are d-index+1: bumps land at l+1 and r+1.
        Self::bump(&mut self.b1, l + 1, v);
        Self::bump(&mut self.b1, r + 1, -v);
        Self::bump(&mut self.b2, l + 1, v * (l as i64));
        Self::bump(&mut self.b2, r + 1, -v * (r as i64));
    }

    fn prefix_sum(&self, i: usize) -> i64 {
        // sum of a[0..i)
        if i == 0 {
            return 0;
        }
        Self::prefix(&self.b1, i) * (i as i64) - Self::prefix(&self.b2, i)
    }

    /// Sum of `a[l..r)` (half-open).
    pub fn sum(&self, l: usize, r: usize) -> i64 {
        self.prefix_sum(r) - self.prefix_sum(l)
    }

    /// Cell count.
    pub fn len(&self) -> usize {
        self.b1.len() - 1
    }

    /// True when `n == 0`.
    pub fn is_empty(&self) -> bool {
        self.b1.len() <= 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn range_point_basic() {
        let mut p = RangePoint::new(8);
        p.add(0, 8, 3);
        p.add(2, 6, 5);
        assert_eq!(p.get(0), 3);
        assert_eq!(p.get(3), 8);
        assert_eq!(p.get(7), 3);
    }

    #[test]
    fn range_point_oracle() {
        let mut rng = SplitMix64::new(41);
        for _ in 0..20 {
            let n = 1 + rng.below(64) as usize;
            let mut p = RangePoint::new(n);
            let mut a = vec![0i64; n];
            for _ in 0..400 {
                let l = rng.below(n as u32 + 1) as usize;
                let r = rng.below(n as u32 + 1) as usize;
                let (l, r) = (l.min(r), l.max(r));
                let v = rng.below(200) as i64 - 100;
                p.add(l, r, v);
                for x in a.iter_mut().take(r).skip(l) {
                    *x += v;
                }
            }
            for (i, &x) in a.iter().enumerate() {
                assert_eq!(p.get(i), x);
            }
        }
    }

    #[test]
    fn range_sum_basic() {
        let mut s = RangeSum::new(8);
        s.add(1, 5, 7);
        assert_eq!(s.sum(0, 8), 28);
        assert_eq!(s.sum(2, 4), 14);
        assert_eq!(s.sum(0, 1), 0);
        s.add(0, 8, 1);
        assert_eq!(s.sum(0, 8), 36);
    }

    #[test]
    fn range_sum_oracle() {
        let mut rng = SplitMix64::new(43);
        for _ in 0..20 {
            let n = 1 + rng.below(64) as usize;
            let mut s = RangeSum::new(n);
            let mut a = vec![0i64; n];
            for _ in 0..400 {
                match rng.below(2) {
                    0 => {
                        let l = rng.below(n as u32) as usize;
                        let r = 1 + l + rng.below((n - l) as u32) as usize;
                        let v = rng.below(200) as i64 - 100;
                        s.add(l, r, v);
                        for x in a.iter_mut().take(r).skip(l) {
                            *x += v;
                        }
                    }
                    _ => {
                        let l = rng.below(n as u32) as usize;
                        let r = 1 + l + rng.below((n - l) as u32) as usize;
                        let want: i64 = a[l..r].iter().sum();
                        assert_eq!(s.sum(l, r), want);
                    }
                }
            }
        }
    }
}
