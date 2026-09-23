//! Slope trick — a convex piecewise-linear function `f` with
//! `O(log n)` updates, stored as two ordered multisets of
//! breakpoints rather than per-point values (the competitive-
//! programming primitive behind "slope trick" DP speedups; cf.
//! KACTL's `LineContainer` sibling).
//!
//! `f(x) = min_f + Σ_{a∈L} max(0, a−x) + Σ_{b∈R} max(0, x−b)`
//!
//! - `L` (max-side breakpoints) means `f` falls with slope `|L|` to
//!   their right… precisely: left of `max L` the function decreases
//!   at `|L|` per unit;
//! - `R` (min-side breakpoints) grows it at `|R|` per unit right
//!   of `min R`;
//! - the flat between `max L` and `min R` is the `argmin` interval.
//!
//! All breakpoints are `i64`, heaps are `BTreeMap` multisets, and
//! domain shifts are lazy offsets — so the whole structure is a
//! pure function of the operation sequence.
//!
//! ```
//! use izanagi_kit::slopetrick::SlopeTrick;
//!
//! let mut f = SlopeTrick::new();
//! f.add_abs(3); // f(x) = |x−3|
//! f.add_abs(5); // f(x) = |x−3|+|x−5|
//! assert_eq!(f.min(), 2);
//! assert_eq!(f.argmin(), (3, 5));
//! assert_eq!(f.eval(4), 2);
//! ```
//!
//! References: the "slope trick" writeups by maspy, drken, and
//! ei1333 (Qiita, 2016–2021); used in Codeforces/AtCoder Monge-DP
//! editorials.

use std::collections::BTreeMap;

/// Convex piecewise-linear `f: i64 → i64` in slope-trick form.
#[derive(Default)]
pub struct SlopeTrick {
    /// Left breakpoints (function drops `|L|`/unit leftward of
    /// `max L`), stored with the lazy offset removed.
    l: BTreeMap<i64, u64>,
    /// Right breakpoints (function grows `|R|`/unit rightward of
    /// `min R`).
    r: BTreeMap<i64, u64>,
    /// Minimum value.
    min_f: i64,
    /// Lazy shift applied to stored `L` keys.
    add_l: i64,
    /// Lazy shift applied to stored `R` keys.
    add_r: i64,
}

fn push(m: &mut BTreeMap<i64, u64>, k: i64) {
    *m.entry(k).or_insert(0) += 1;
}

fn pop_max(m: &mut BTreeMap<i64, u64>) -> Option<i64> {
    let k = *m.keys().next_back()?;
    let v = m[&k] - 1;
    if v == 0 {
        m.remove(&k);
    } else {
        m.insert(k, v);
    }
    Some(k)
}

fn pop_min(m: &mut BTreeMap<i64, u64>) -> Option<i64> {
    let k = *m.keys().next()?;
    let v = m[&k] - 1;
    if v == 0 {
        m.remove(&k);
    } else {
        m.insert(k, v);
    }
    Some(k)
}

impl SlopeTrick {
    /// `f = 0`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current minimum value.
    pub fn min(&self) -> i64 {
        self.min_f
    }

    /// The `argmin` interval `[lo, hi]` — `f` is flat `min_f` on it.
    /// Missing sides yield an open bound (`i64::MIN`/`MAX`).
    pub fn argmin(&self) -> (i64, i64) {
        let lo = self
            .l
            .keys()
            .next_back()
            .map_or(i64::MIN, |k| k + self.add_l);
        let hi = self.r.keys().next().map_or(i64::MAX, |k| k + self.add_r);
        (lo, hi)
    }

    /// `f += max(0, a − x)` — flat right of `a`, slope −1 left of it.
    /// Canonical form (MtSaka / maspy): bump `min_f` by how far `a`
    /// sits above the right argmin edge, push `a` into `R`, and move
    /// the new smallest `R` breakpoint into `L`.
    pub fn add_a_minus_x(&mut self, a: i64) {
        if let Some(&rk) = self.r.keys().next() {
            let rt = rk + self.add_r;
            if a > rt {
                self.min_f += a - rt;
            }
        }
        push(&mut self.r, a - self.add_r);
        if let Some(rt) = pop_min(&mut self.r) {
            push(&mut self.l, rt + self.add_r - self.add_l);
        }
    }

    /// `f += max(0, x − a)` — flat left of `a`, slope +1 right of it.
    /// Mirror of [`add_a_minus_x`](Self::add_a_minus_x).
    pub fn add_x_minus_a(&mut self, a: i64) {
        if let Some(&lk) = self.l.keys().next_back() {
            let lt = lk + self.add_l;
            if lt > a {
                self.min_f += lt - a;
            }
        }
        push(&mut self.l, a - self.add_l);
        if let Some(lt) = pop_max(&mut self.l) {
            push(&mut self.r, lt + self.add_l - self.add_r);
        }
    }

    /// `f += |x − a|`.
    pub fn add_abs(&mut self, a: i64) {
        self.add_a_minus_x(a);
        self.add_x_minus_a(a);
    }

    /// `f(x) := min_{y ≤ x} f(y)` — kill the right-side growth.
    pub fn clear_right(&mut self) {
        self.r.clear();
    }

    /// `f(x) := min_{y ≥ x} f(y)` — kill the left-side decay.
    pub fn clear_left(&mut self) {
        self.l.clear();
    }

    /// Translate the domain: `f(x) := f_old(x − a)` (the whole
    /// `argmin` interval shifts `a` right).
    pub fn shift(&mut self, a: i64) {
        self.add_l += a;
        self.add_r += a;
    }

    /// Sliding window: `f(x) := min_{x−b ≤ y ≤ x−a} f(y)` — the
    /// `argmin` interval becomes `[lo+a, hi+b]`.
    /// Requires `a ≤ b` (the window must be nonempty).
    pub fn slide(&mut self, a: i64, b: i64) {
        self.add_l += a;
        self.add_r += b;
    }

    /// Evaluate `f(x)` — `O(|L|+|R|)`; meant for callers that need
    /// the curve (and for the oracle test), not for hot paths.
    pub fn eval(&self, x: i64) -> i64 {
        let mut v = self.min_f;
        for (&k, &c) in &self.l {
            let l = k + self.add_l;
            if l > x {
                v += (l - x) * c as i64;
            }
        }
        for (&k, &c) in &self.r {
            let r = k + self.add_r;
            if x > r {
                v += (x - r) * c as i64;
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abs_and_min() {
        let mut f = SlopeTrick::new();
        f.add_abs(3);
        assert_eq!(f.min(), 0);
        assert_eq!(f.argmin(), (3, 3));
        f.add_abs(5);
        assert_eq!(f.min(), 2);
        assert_eq!(f.argmin(), (3, 5));
        assert_eq!(f.eval(4), 2);
        assert_eq!(f.eval(0), 8);
        assert_eq!(f.eval(6), 4);
    }

    #[test]
    fn sliding_and_clears() {
        let mut f = SlopeTrick::new();
        f.add_abs(0);
        f.slide(2, 4); // f(x) = min_{y in [x-4, x-2]} |y|
        assert_eq!(f.eval(10), 6); // y ∈ [6,8]: min |y| = 6
        assert_eq!(f.eval(-10), 12); // y ∈ [−14,−12]: min |y| = 12
        let mut g = SlopeTrick::new();
        g.add_abs(0);
        g.clear_right(); // f(x) = min_{y<=x}|y| = 0 for x>=0
        assert_eq!(g.eval(10), 0);
        assert_eq!(g.eval(-10), 10);
        let mut h = SlopeTrick::new();
        h.add_abs(0);
        h.shift(5); // f(x) = |x - 5|
        assert_eq!(h.argmin(), (5, 5));
        assert_eq!(h.eval(9), 4);
    }

    /// Dense oracle: f on grid [-R, R] as a Vec, ops mirrored.
    /// The band is oversized: each slide only pulls values from
    /// `b ≤ 18` away, so after 40 ops a poisoned clamped edge can
    /// never reach the compared core of ±40.
    const R: i64 = 800;

    fn model_add(model: &mut [i64], f: impl Fn(i64) -> i64) {
        for (x, v) in model.iter_mut().enumerate() {
            let x = x as i64 - R;
            *v += f(x);
        }
    }

    #[test]
    fn matches_dense_oracle() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0x5107);
        let span = (2 * R + 1) as usize;
        for t in 0..60 {
            let mut st = SlopeTrick::new();
            let mut model = vec![0i64; span];
            let mut used_slide = false;
            let mut hist: Vec<String> = Vec::new();
            for _ in 0..40 {
                let a = (rng.next_u64() % (2 * R as u64)) as i64 - R;
                match rng.next_u64() % 6 {
                    0 => {
                        st.add_a_minus_x(a);
                        model_add(&mut model, |x| (a - x).max(0));
                        hist.push(format!("amx {a}"));
                    }
                    1 => {
                        st.add_x_minus_a(a);
                        model_add(&mut model, |x| (x - a).max(0));
                        hist.push(format!("xma {a}"));
                    }
                    2 => {
                        st.add_abs(a);
                        model_add(&mut model, |x| (x - a).abs());
                        hist.push(format!("abs {a}"));
                    }
                    3 => {
                        st.clear_right(); // f(x) = min_{y<=x} f(y): prefix mins
                        hist.push("cl_r".to_string());
                        let mut acc = i64::MAX;
                        for v in model.iter_mut() {
                            acc = acc.min(*v);
                            *v = acc;
                        }
                    }
                    4 => {
                        st.clear_left(); // f(x) = min_{y>=x} f(y): suffix mins
                        hist.push("cl_l".to_string());
                        let mut acc = i64::MAX;
                        for v in model.iter_mut().rev() {
                            acc = acc.min(*v);
                            *v = acc;
                        }
                    }
                    _ => {
                        let a = (rng.next_u64() % 10) as i64;
                        let b = a + (rng.next_u64() % 10) as i64;
                        st.slide(a, b);
                        used_slide = true;
                        hist.push(format!("slide {a} {b}"));
                        // g(x) = min_{x-b <= y <= x-a} f(y)
                        let mut g = vec![0i64; span];
                        for x in 0..span {
                            let gx = x as i64 - R;
                            let (lo, hi) = (gx - b, gx - a);
                            let mut best = i64::MAX;
                            let lo = lo.max(-R);
                            let hi = hi.min(R);
                            if lo <= hi {
                                for y in lo..=hi {
                                    best = best.min(model[(y + R) as usize]);
                                }
                                g[x] = best;
                            } else {
                                g[x] = model[x]; // window off-grid: keep
                            }
                        }
                        model = g;
                    }
                }
                // Compare on the safe core: sliding windows reach
                // ±10 past x, so values beyond x ∈ [-40, 40] could
                // depend on f(y) the grid doesn't model.
                for x in -40i64..=40 {
                    assert_eq!(
                        st.eval(x),
                        model[(x + R) as usize],
                        "x={x} t={t} ops={hist:?}"
                    );
                }
                // After a slide the true min may live outside the
                // modelled band — eval() still covers it on the core.
                if !used_slide {
                    assert_eq!(st.min(), *model.iter().min().unwrap_or(&0));
                }
            }
        }
    }
}
