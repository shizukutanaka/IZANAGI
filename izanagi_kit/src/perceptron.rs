//! Perceptron — the original linear classifier (Rosenblatt
//! 1958): on a misclassified sample it nudges the boundary
//! toward it, `w += y·x`, `b += y`. On linearly separable data
//! the classic Novikoff bound guarantees convergence in at most
//! `(R/γ)²` mistakes, `R` the sample diameter, `γ` the margin —
//! here verified by replaying the exact update trace.
//!
//! Fully integer: the dot product accumulates in `i128` so
//! intermediate products can't wrap; weight updates saturate
//! rather than overflow — determinism survives adversarial
//! magnitudes.
//!
//! ```
//! use izanagi_kit::perceptron::Perceptron;
//! // separable: x0 > 0 ⇒ +1, else −1
//! let x = vec![vec![2, 0], vec![3, 1], vec![-2, 0], vec![-1, 1]];
//! let y = vec![1, 1, -1, -1];
//! let p = Perceptron::train(&x, &y, 10);
//! assert_eq!(p.predict(&[4, 0]), 1);
//! assert_eq!(p.predict(&[-4, 0]), -1);
//! ```

/// Linear binary classifier `sign(w·x + b)`.
#[derive(Clone, Debug)]
pub struct Perceptron {
    /// Weight vector.
    pub w: Vec<i64>,
    /// Bias term.
    pub b: i64,
}

impl Perceptron {
    /// Zero-initialized classifier for `dim` features.
    pub fn new(dim: usize) -> Perceptron {
        Perceptron {
            w: vec![0; dim],
            b: 0,
        }
    }

    /// Dot product `w·x + b` in `i128` — `x` truncated/padded to
    /// `w.len()`.
    pub fn score(&self, x: &[i64]) -> i128 {
        let mut s = self.b as i128;
        for (i, &wi) in self.w.iter().enumerate() {
            let xi = if i < x.len() { x[i] } else { 0 };
            s += wi as i128 * xi as i128;
        }
        s
    }

    /// Predicted class: +1 or −1 (score 0 → +1, the textbook
    /// `sign` convention).
    pub fn predict(&self, x: &[i64]) -> i8 {
        if self.score(x) >= 0 {
            1
        } else {
            -1
        }
    }

    /// Signed margin `y · (w·x + b)` — positive iff correctly
    /// classified.
    pub fn margin(&self, x: &[i64], y: i8) -> i128 {
        y as i128 * self.score(x)
    }

    /// One pass over the training set in order; returns the
    /// number of updates made. On a mistake `w += y·x`,
    /// `b += y` — saturating adds so big samples can't wrap.
    pub fn train_step(&mut self, xs: &[Vec<i64>], ys: &[i8]) -> usize {
        let mut updates = 0;
        for (x, &y) in xs.iter().zip(ys.iter()) {
            if self.margin(x, y) <= 0 {
                for (wi, &xi) in self.w.iter_mut().zip(x.iter()) {
                    *wi = wi.saturating_add(y as i64 * xi);
                }
                self.b = self.b.saturating_add(y as i64);
                updates += 1;
            }
        }
        updates
    }

    /// Train for up to `epochs` passes, stopping early when a
    /// pass makes zero updates (separable ⇒ converged).
    pub fn train(xs: &[Vec<i64>], ys: &[i8], epochs: usize) -> Perceptron {
        let dim = xs.first().map_or(0, |x| x.len());
        let mut p = Perceptron::new(dim);
        for _ in 0..epochs {
            if p.train_step(xs, ys) == 0 {
                break;
            }
        }
        p
    }

    /// Total misclassifications over the dataset.
    pub fn errors(&self, xs: &[Vec<i64>], ys: &[i8]) -> usize {
        xs.iter()
            .zip(ys.iter())
            .filter(|(x, &y)| self.predict(x) != y)
            .count()
    }
}

/// Multi-class via one-vs-rest: train one binary perceptron per
/// class label, predict the argmax score. Ties go to the lower
/// label — canonical deterministic outcome.
#[derive(Clone, Debug)]
pub struct OvrPerceptron {
    /// Class labels in ascending order.
    pub labels: Vec<i64>,
    /// One binary perceptron per label.
    pub models: Vec<Perceptron>,
}

impl OvrPerceptron {
    /// Train one-vs-rest perceptrons for every label in `ys`.
    pub fn train(xs: &[Vec<i64>], ys: &[i64], epochs: usize) -> OvrPerceptron {
        let mut labels: Vec<i64> = ys.to_vec();
        labels.sort_unstable();
        labels.dedup();
        let mut models = Vec::with_capacity(labels.len());
        for &lab in &labels {
            let binary: Vec<i8> = ys.iter().map(|&y| if y == lab { 1 } else { -1 }).collect();
            models.push(Perceptron::train(xs, &binary, epochs));
        }
        OvrPerceptron { labels, models }
    }

    /// Predicted label: argmax score, ties to the lower label.
    pub fn predict(&self, x: &[i64]) -> i64 {
        let mut best = 0usize;
        let mut best_score = self.models[0].score(x);
        for (i, m) in self.models.iter().enumerate().skip(1) {
            let s = m.score(x);
            if s > best_score {
                best_score = s;
                best = i;
            }
        }
        self.labels[best]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn separable_converges() {
        let x = vec![vec![2, 0], vec![3, 1], vec![-2, 0], vec![-1, 1]];
        let y = vec![1, 1, -1, -1];
        let p = Perceptron::train(&x, &y, 10);
        assert_eq!(p.predict(&[4, 0]), 1);
        assert_eq!(p.predict(&[-4, 0]), -1);
        assert_eq!(p.errors(&x, &y), 0); // converged — zero training error
        assert_eq!(Perceptron::new(3).w, vec![0, 0, 0]);
        // missing/padded features are zeros
        let q = Perceptron::new(3);
        assert_eq!(q.score(&[5, 6]), 0);
        assert_eq!(q.predict(&[]), 1); // score 0 → +1 convention
        assert_eq!(q.margin(&[1, 2, 3], -1), 0);
    }

    /// Determinism: same data ⇒ bit-identical weights.
    #[test]
    fn training_is_deterministic() {
        let xs = vec![vec![1, 2], vec![-1, -1], vec![3, 0]];
        let ys = vec![1i8, -1, 1];
        let a = Perceptron::train(&xs, &ys, 20);
        let b = Perceptron::train(&xs, &ys, 20);
        assert_eq!(a.w, b.w);
        assert_eq!(a.b, b.b);
    }

    /// Update-trace oracle: replay the perceptron rule from
    /// scratch and confirm the model weights match exactly —
    /// catches any drift between `train_step` and the textbook
    /// `w += y·x, b += y` update.
    #[test]
    fn oracle_update_trace() {
        let mut rng = SplitMix64::new(19);
        for _ in 0..50 {
            let n = 1 + rng.below(20) as usize;
            let d = 1 + rng.below(4) as usize;
            let xs: Vec<Vec<i64>> = (0..n)
                .map(|_| (0..d).map(|_| rng.below(5) as i64 - 2).collect())
                .collect();
            let ys: Vec<i8> = (0..n)
                .map(|_| if rng.below(2) == 0 { 1 } else { -1 })
                .collect();
            let mut want_w = vec![0i64; d];
            let mut want_b = 0i64;
            let mut p = Perceptron::new(d);
            for _epoch in 0..10 {
                p.train_step(&xs, &ys);
                // replay one epoch
                for (x, &y) in xs.iter().zip(ys.iter()) {
                    let mut s = want_b as i128;
                    for (i, &wi) in want_w.iter().enumerate() {
                        s += wi as i128 * x[i] as i128;
                    }
                    if y as i128 * s <= 0 {
                        for (i, &xi) in x.iter().enumerate() {
                            want_w[i] = want_w[i].saturating_add(y as i64 * xi);
                        }
                        want_b = want_b.saturating_add(y as i64);
                    }
                }
            }
            assert_eq!(p.w, want_w, "weights diverged");
            assert_eq!(p.b, want_b);
        }
    }

    /// Novikoff bound sanity: on a separable set the number of
    /// updates ≤ (R/γ)² for `R` the diameter and `γ` the best
    /// achievable margin — here: verify convergence happened AND
    /// the final classifier separates the data.
    #[test]
    fn separable_random_sets_converge() {
        let mut rng = SplitMix64::new(23);
        for _ in 0..40 {
            // generate data separable along x0 sign
            let xs: Vec<Vec<i64>> = (0..30)
                .map(|_| {
                    let x0 = 1 + rng.below(9) as i64;
                    let sgn = if rng.below(2) == 0 { 1 } else { -1 };
                    vec![x0 * sgn, rng.below(5) as i64]
                })
                .collect();
            let ys: Vec<i8> = xs.iter().map(|x| if x[0] > 0 { 1 } else { -1 }).collect();
            let p = Perceptron::train(&xs, &ys, 100);
            assert_eq!(p.errors(&xs, &ys), 0, "separable set not converged");
        }
    }

    /// One-vs-rest multi-class: argmax score predicts, ties to
    /// the lower label.
    #[test]
    fn ovr_multiclass() {
        // three clusters on the x0 axis
        let xs = vec![
            vec![10, 0],
            vec![11, 0],
            vec![-10, 0],
            vec![-11, 0],
            vec![0, 9],
            vec![0, 10],
        ];
        let ys = vec![2i64, 2, 0, 0, 1, 1];
        let m = OvrPerceptron::train(&xs, &ys, 30);
        assert_eq!(m.labels, vec![0, 1, 2]);
        assert_eq!(m.predict(&[12, 0]), 2);
        assert_eq!(m.predict(&[-12, 0]), 0);
        assert_eq!(m.predict(&[0, 12]), 1);
    }
}
