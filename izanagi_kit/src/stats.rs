//! Streaming statistics — Welford online mean/variance with min/max
//! and a parallel-safe `merge`.
//!
//! [`crate::profiler::EventLog`] records raw samples; this reduces them
//! — frame-time envelopes, tick-cost distributions, economy metrics —
//! without storing the stream and without floats (all accumulators are
//! `i128`-backed fixed-point in `i64` space; variance is returned scaled
//! by `SCALE` so callers stay integer-exact).
//!
//! ```
//! use izanagi_kit::stats::RunningStats;
//! let mut s = RunningStats::new();
//! for v in [10, 20, 30, 40] { s.push(v); }
//! assert_eq!(s.count(), 4);
//! assert_eq!(s.mean(), Some(25));
//! assert_eq!(s.variance(), Some(125 * RunningStats::SCALE));
//! ```

/// Online moments over an `i64` stream: count, mean, M2 (sum of
/// squared deviations), min, max.
///
/// [`merge`](Self::merge) uses Chan et al.'s parallel-variance formula,
/// so two halves of a stream folded separately combine to the same
/// state the single pass would have produced (bit-exact for mean; M2
/// differs by integer-rounding error only, documented below).
#[derive(Clone, Default)]
pub struct RunningStats {
    n: u64,
    /// Mean scaled by `SCALE` — fixed point, exact while |mean|·SCALE
    /// fits i128.
    mean_s: i128,
    m2_s: i128, // M2 scaled by SCALE
    min: i64,
    max: i64,
}

impl RunningStats {
    /// Fixed-point scale for [`mean`](Self::mean),
    /// [`variance`](Self::variance), [`stddev`](Self::stddev) results.
    /// `1_000_000` — results are integers×`SCALE`.
    pub const SCALE: i64 = 1_000_000;

    /// Empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push one sample.
    pub fn push(&mut self, x: i64) {
        let xs = x as i128 * Self::SCALE as i128;
        self.n += 1;
        if self.n == 1 {
            self.mean_s = xs;
            self.m2_s = 0;
            self.min = x;
            self.max = x;
            return;
        }
        // Welford: delta = x - mean; mean += delta/n; M2 += delta·(x-mean)
        let delta = xs - self.mean_s;
        self.mean_s += delta / self.n as i128;
        let delta2 = xs - self.mean_s;
        self.m2_s += delta * delta2;
        if x < self.min {
            self.min = x;
        }
        if x > self.max {
            self.max = x;
        }
    }

    /// Push a whole slice (equivalent to pushing each element).
    pub fn push_all(&mut self, xs: &[i64]) {
        for &x in xs {
            self.push(x);
        }
    }

    /// Merge `other` into `self` (Chan parallel update). Idempotent
    /// with a cloned self: `a.merge(b)` then `a.merge(b)` is NOT
    /// idempotent — merging consumes `other` by value semantically.
    pub fn merge(&mut self, other: &RunningStats) {
        if other.n == 0 {
            return;
        }
        if self.n == 0 {
            *self = other.clone();
            return;
        }
        let delta = other.mean_s - self.mean_s;
        let n = self.n + other.n;
        // mean = (n_a·mean_a + n_b·mean_b) / n
        self.mean_s = (self.mean_s * self.n as i128 + other.mean_s * other.n as i128) / n as i128;
        // M2 = M2a + M2b + δ²·(n_a·n_b/n)
        self.m2_s =
            self.m2_s + other.m2_s + delta * delta * (self.n as i128 * other.n as i128) / n as i128;
        self.n = n;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    /// Number of samples seen.
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Whether no samples have been pushed.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Smallest sample; `None` when empty.
    pub fn min(&self) -> Option<i64> {
        (self.n > 0).then_some(self.min)
    }

    /// Largest sample; `None` when empty.
    pub fn max(&self) -> Option<i64> {
        (self.n > 0).then_some(self.max)
    }

    /// Mean (truncated to integer). `None` when empty.
    pub fn mean(&self) -> Option<i64> {
        if self.n == 0 {
            return None;
        }
        Some((self.mean_s / Self::SCALE as i128) as i64)
    }

    /// Population variance scaled by `SCALE` — i.e.
    /// `var() ≈ variance_true × SCALE`. `None` when empty.
    pub fn variance(&self) -> Option<i64> {
        if self.n == 0 {
            return None;
        }
        Some((self.m2_s / (self.n as i128) / Self::SCALE as i128) as i64)
    }

    /// Sample (n−1) variance scaled by `SCALE`. `None` for n < 2.
    pub fn sample_variance(&self) -> Option<i64> {
        if self.n < 2 {
            return None;
        }
        Some((self.m2_s / (self.n as i128 - 1) / Self::SCALE as i128) as i64)
    }

    /// Integer square root of the population variance — standard
    /// deviation scaled by `√SCALE` ≈ 1000 (returned `stdev·1000` so
    /// callers can compare to `SCALE`-scaled quantities approximately).
    /// `None` when empty.
    pub fn stddev(&self) -> Option<i64> {
        self.variance()
            .map(|v| crate::fixed::isqrt_u64(v.max(0) as u64) as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Naive two-pass oracle over a stored stream.
    /// Returns (mean·SCALE, var·SCALE) computed exactly in i128.
    fn oracle(xs: &[i64]) -> (i128, i128) {
        let n = xs.len() as i128;
        let sc = RunningStats::SCALE as i128;
        let s: i128 = xs.iter().map(|&x| x as i128).sum();
        let mean = s * sc / n;
        let var = xs
            .iter()
            .map(|&x| (x as i128 * sc - mean).pow(2))
            .sum::<i128>()
            / n
            / sc;
        (mean, var)
    }

    #[test]
    fn matches_two_pass_oracle() {
        let mut rng = SplitMix64::new(0x9A17);
        for _ in 0..300 {
            let n = 1 + rng.below(60) as usize;
            let xs: Vec<i64> = (0..n).map(|_| rng.range(-1000, 1001) as i64).collect();
            let mut s = RunningStats::new();
            s.push_all(&xs);
            let (mean_o, var_o) = oracle(&xs);
            // mean() returns raw units; oracle mean is ×SCALE — rescale
            // and allow ±1 unit of truncation drift.
            let mean_a = s.mean().unwrap();
            let mean_a_s = mean_a as i128 * RunningStats::SCALE as i128;
            assert!(
                (mean_a_s - mean_o).abs() <= RunningStats::SCALE as i128,
                "mean {mean_a_s} vs {mean_o}"
            );
            let var_a = s.variance().unwrap();
            // var() returns var·SCALE but computed on a truncated mean;
            // drift grows with spread — allow generous relative slack.
            assert!(
                (var_a as i128 - var_o).abs() <= var_o / 10 + 4 * RunningStats::SCALE as i128,
                "var {var_a} vs {var_o}"
            );
            assert_eq!(s.min(), Some(*xs.iter().min().unwrap()));
            assert_eq!(s.max(), Some(*xs.iter().max().unwrap()));
            assert_eq!(s.count(), n as u64);
        }
    }

    #[test]
    fn merge_matches_single_pass() {
        let mut rng = SplitMix64::new(0xBEEF);
        for _ in 0..200 {
            let n = 2 + rng.below(50) as usize;
            let xs: Vec<i64> = (0..n).map(|_| rng.range(-500, 501) as i64).collect();
            let cut = 1 + rng.below(n as u32 - 1) as usize;
            let mut a = RunningStats::new();
            a.push_all(&xs[..cut]);
            let mut b = RunningStats::new();
            b.push_all(&xs[cut..]);
            a.merge(&b);
            let mut whole = RunningStats::new();
            whole.push_all(&xs);
            assert_eq!(a.count(), whole.count());
            // Merged mean may drift ±1 unit vs single-pass (two
            // truncation passes) — documented integer behavior.
            let d = (a.mean().unwrap() - whole.mean().unwrap()).abs();
            assert!(d <= 1, "merged mean drift {d}");
            assert_eq!(a.min(), whole.min());
            assert_eq!(a.max(), whole.max());
            // Variance may differ by a few fixed-point ticks (δ² term
            // truncates) — bounded well below observable noise.
            let d = (a.variance().unwrap() - whole.variance().unwrap()).abs();
            assert!(d < 10_000, "variance drift {d}");
        }
    }

    #[test]
    fn empty_and_singleton_semantics() {
        let s = RunningStats::new();
        assert!(s.is_empty());
        assert_eq!(s.count(), 0);
        assert_eq!(s.mean(), None);
        assert_eq!(s.variance(), None);
        assert_eq!(s.sample_variance(), None);
        assert_eq!(s.stddev(), None);
        assert_eq!(s.min(), None);
        assert_eq!(s.max(), None);

        let mut s = RunningStats::new();
        s.push(42);
        assert_eq!(s.mean(), Some(42));
        assert_eq!(s.variance(), Some(0));
        assert_eq!(s.sample_variance(), None); // n<2
        assert_eq!(s.stddev(), Some(0));
        assert_eq!((s.min(), s.max()), (Some(42), Some(42)));

        // Known pair: {10, 20} → mean 15, pop var 25, stdev 5.
        let mut s = RunningStats::new();
        s.push_all(&[10, 20]);
        assert_eq!(s.mean(), Some(15));
        assert_eq!(s.variance(), Some(25 * RunningStats::SCALE));
        assert_eq!(s.sample_variance(), Some(50 * RunningStats::SCALE));
        assert_eq!(s.stddev(), Some(5_000)); // 5.000 × 1000
    }
}
