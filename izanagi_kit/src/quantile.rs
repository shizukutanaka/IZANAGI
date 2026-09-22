//! Greenwald–Khanna ε-approximate quantiles — "what's the p50 / p99
//! of this latency / damage / resource stream?" in `O((1/ε) log εn)`
//! memory instead of storing the stream. Replay-time latency histograms,
//! sim-fairness audits (is the p90 income within spec?), telemetry
//! summaries that fit on the wire. Deterministic: `(eps, stream)` maps
//! to exactly one tuple list, and the returned value always has
//! `|true_rank − φ·n| ≤ ε·n`.
//!
//! ```
//! use izanagi_kit::quantile::Quantile;
//! let mut q = Quantile::new(1, 100); // epsilon = 0.01
//! for v in 0..1000 {
//!     q.add(v * 7);
//! }
//! let p50 = q.quantile(1, 2).unwrap_or(-1);
//! assert!(p50 >= 3400 && p50 <= 3600, "p50 = {p50}");
//! ```

/// One GK tuple: `v` a seen value, `g` how many items it covers,
/// `delta` the uncertainty in its rank.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Tuple {
    v: i64,
    g: u64,
    delta: u64,
}

/// GK sketch over `i64` values. Accuracy parameter is the rational
/// `eps_num/eps_den`; queries are `quantile(phi_num, phi_den)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quantile {
    eps_num: u64,
    eps_den: u64,
    n: u64,
    tuples: Vec<Tuple>,
}

impl Quantile {
    /// Sketch with accuracy `eps_num/eps_den` (clamped into `[1/10000, 1/2]`).
    pub fn new(eps_num: u64, eps_den: u64) -> Self {
        let den = eps_den.max(2);
        let num = eps_num.clamp(1, den / 2).max(den / 10000);
        Quantile {
            eps_num: num,
            eps_den: den,
            n: 0,
            tuples: Vec::new(),
        }
    }

    /// Items seen.
    pub fn len(&self) -> u64 {
        self.n
    }

    /// `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// `floor(2·eps·n)` — the rank band inside which ranks may blur.
    fn band(&self) -> u64 {
        2 * self.eps_num * self.n / self.eps_den
    }

    /// Feed one value.
    pub fn add(&mut self, v: i64) {
        // Sorted insertion: min/max positions keep delta 0 (their rank
        // is known exactly); interior duplicates and new minima get the
        // blurred-rank delta.
        let i = match self.tuples.binary_search_by(|x| x.v.cmp(&v)) {
            Ok(i) | Err(i) => i,
        };
        let at_edge =
            (i == 0 && self.tuples.first().is_some_and(|t| v < t.v)) || i == self.tuples.len();
        let delta = if at_edge {
            0
        } else {
            self.band().saturating_sub(1)
        };
        self.tuples.insert(i, Tuple { v, g: 1, delta });
        self.n += 1;
        let period = (self.eps_den / (2 * self.eps_num)).max(1);
        if self.n % period == 0 {
            self.compress();
        }
    }

    /// GK compaction: merge `i` into `i+1` while
    /// `g_i + g_{i+1} + delta_{i+1} <= band`, right to left, never
    /// merging into the last tuple.
    fn compress(&mut self) {
        if self.tuples.len() < 2 {
            return;
        }
        let band = self.band();
        let mut i = self.tuples.len() - 2;
        loop {
            if self.tuples[i].g + self.tuples[i + 1].g + self.tuples[i + 1].delta <= band {
                self.tuples[i + 1].g += self.tuples[i].g;
                self.tuples.remove(i);
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }

    /// Approximate φ-quantile (`num/den` clamped to `[0,1]`): the
    /// returned value `v` satisfies `|rank(v) − φ·n| ≤ ε·n` where
    /// `rank(v)` counts items `<= v`. `None` on an empty sketch.
    pub fn quantile(&self, num: u64, den: u64) -> Option<i64> {
        if self.tuples.is_empty() {
            return None;
        }
        let den = den.max(1);
        let num = num.min(den);
        let r = num.saturating_mul(self.n).saturating_add(den - 1) / den;
        let limit = r + self.eps_num * self.n / self.eps_den;
        let mut rmin = 0u64;
        for i in 0..self.tuples.len() {
            if rmin + self.tuples[i].g + self.tuples[i].delta > limit {
                return Some(self.tuples[i.saturating_sub(1)].v);
            }
            rmin += self.tuples[i].g;
        }
        self.tuples.last().map(|t| t.v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn small_streams_are_exact_and_extremes() {
        let mut q = Quantile::new(1, 50);
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert_eq!(q.quantile(1, 2), None);
        for v in [5, 1, 3, 2, 4] {
            q.add(v);
        }
        assert_eq!(q.quantile(0, 1), Some(1));
        assert_eq!(q.quantile(1, 1), Some(5));
        assert_eq!(q.quantile(1, 2), Some(3));
        let mut e = Quantile::new(1, 0); // degenerate den -> default
        e.add(9);
        assert_eq!(e.quantile(1, 2), Some(9));
    }

    #[test]
    fn rank_bound_holds_vs_sorted_oracle() {
        let mut rng = SplitMix64::new(0x9E37_9A11);
        for (num, den) in [(1u64, 50u64), (1, 200)] {
            let mut q = Quantile::new(num, den);
            let mut truth = Vec::new();
            for _ in 0..50_000 {
                let v = (rng.below(1_000_000) as i64) - 500_000;
                q.add(v);
                truth.push(v);
            }
            truth.sort();
            let n = truth.len() as u64;
            let eps_n = (num * n / den).max(1);
            // Every decile: |rank(v) - phi*n| <= eps*n.
            for p in 1..10u64 {
                let v = q.quantile(p, 10).unwrap_or(0);
                let rank = truth.partition_point(|&x| x <= v) as u64;
                let target = p * n / 10;
                let d = rank.max(target) - rank.min(target);
                assert!(
                    d <= eps_n + 1,
                    "p{p}/10: v={v} rank={rank} target={target} eps_n={eps_n}"
                );
            }
        }
    }
}
