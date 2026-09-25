//! Hidden Markov model inference over `Fixed` probabilities — the
//! forward–backward pair Rabiner's 1989 tutorial calls "scaled", so
//! `T` can reach hundreds of steps without the probabilities
//! underflowing to zero (each `α_t`/`β_t` is renormalized by `c_t`).
//! Complements `viterbi` (which finds the best path); here you get
//! per-step state posteriors and the observation likelihood.
//!
//! All quantities are `Fixed` in `[0,1]`. `1/65536`-ulp values are fine
//! for a few dozen steps; for long chains prefer the returned
//! log-likelihood over a raw product (it would truncate to 0).
//!
//! ```
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::hmm::forward;
//!
//! let half = Fixed::from_ratio(1, 2);
//! // Two states, fair transitions, each emits obs 0 or 1 fairly.
//! let init = [half, half];
//! let trans = [vec![half, half], vec![half, half]];
//! let emit = [vec![half, half], vec![half, half]];
//! let alpha = forward(&init, &trans, &emit, &[0, 1, 0]).unwrap();
//! assert_eq!(alpha.len(), 3);
//! ```

use crate::fixed::Fixed;

/// Scaled forward pass: `α_t(i) = P(o_1..o_t, S_t = i) / Π c_k` per
/// Rabiner's scaling, where `c_t = 1/Σ_i α̃_t(i)` renormalizes each
/// step so `Σ_i α_t(i) = 1` (within truncation). Returns `T` rows of
/// length `S`, or `None` on empty input, ragged tables, an out-of-range
/// observation, or a zero-sum step (the sequence is impossible).
pub fn forward(
    init: &[Fixed],
    trans: &[Vec<Fixed>],
    emit: &[Vec<Fixed>],
    obs: &[u32],
) -> Option<Vec<Vec<Fixed>>> {
    let s = init.len();
    let t_len = obs.len();
    if s == 0 || t_len == 0 {
        return None;
    }
    if trans.len() != s
        || emit.len() != s
        || trans.iter().any(|r| r.len() != s)
        || emit
            .iter()
            .any(|r| obs.iter().any(|&o| (o as usize) >= r.len()))
    {
        return None;
    }
    let mut alpha: Vec<Vec<Fixed>> = Vec::with_capacity(t_len);
    // t = 0: α_0(i) = π_i · b_i(o_0), then scale.
    let mut row = Vec::with_capacity(s);
    let mut sum = Fixed::ZERO;
    for i in 0..s {
        row.push(init[i].mul(emit[i][obs[0] as usize]));
        sum = sum + row[i];
    }
    if sum.is_zero() {
        return None;
    }
    for v in row.iter_mut() {
        *v = v.div(sum);
    }
    alpha.push(row);
    // Recursion: α̃_t(i) = b_i(o_t)·Σ_j a_ji·α_{t−1}(j), scaled.
    for (t, &ob) in obs.iter().enumerate().skip(1) {
        let o = ob as usize;
        let mut row = Vec::with_capacity(s);
        let mut sum = Fixed::ZERO;
        for i in 0..s {
            let mut acc = Fixed::ZERO;
            for j in 0..s {
                acc = acc + alpha[t - 1][j].mul(trans[j][i]);
            }
            let v = acc.mul(emit[i][o]);
            row.push(v);
            sum = sum + v;
        }
        if sum.is_zero() {
            return None;
        }
        for v in row.iter_mut() {
            *v = v.div(sum);
        }
        alpha.push(row);
    }
    Some(alpha)
}

/// Log-likelihood `ln P(O)` = `Σ_t ln(Σ_i α̃_t(i))` = `−Σ_t ln c_t`.
/// `None` for the same degenerate inputs as [`forward`]. (This is the
/// evaluation primitive: compare models by which assigns higher `ln P`.)
pub fn log_likelihood(
    init: &[Fixed],
    trans: &[Vec<Fixed>],
    emit: &[Vec<Fixed>],
    obs: &[u32],
) -> Option<Fixed> {
    let s = init.len();
    if s == 0 || obs.is_empty() {
        return None;
    }
    if trans.len() != s
        || emit.len() != s
        || trans.iter().any(|r| r.len() != s)
        || emit
            .iter()
            .any(|r| obs.iter().any(|&o| (o as usize) >= r.len()))
    {
        return None;
    }
    let mut ll = Fixed::ZERO;
    let mut alpha = Vec::with_capacity(s);
    let mut sum = Fixed::ZERO;
    for i in 0..s {
        let v = init[i].mul(emit[i][obs[0] as usize]);
        alpha.push(v);
        sum = sum + v;
    }
    if sum.is_zero() {
        return None;
    }
    ll = ll + sum.ln(); // −ln c_t = ln(Σα̃), sums < 1 → negative
    for v in alpha.iter_mut() {
        *v = v.div(sum);
    }
    for &ob in obs.iter().skip(1) {
        let o = ob as usize;
        let mut next = Vec::with_capacity(s);
        let mut sum = Fixed::ZERO;
        for i in 0..s {
            let mut acc = Fixed::ZERO;
            for j in 0..s {
                acc = acc + alpha[j].mul(trans[j][i]);
            }
            let v = acc.mul(emit[i][o]);
            next.push(v);
            sum = sum + v;
        }
        if sum.is_zero() {
            return None;
        }
        ll = ll + sum.ln();
        for v in next.iter_mut() {
            *v = v.div(sum);
        }
        alpha = next;
    }
    Some(ll)
}

/// Scaled backward pass: `β_T(i) = c_T` and
/// `β_t(i) = c_t·Σ_j a_ij·b_j(o_{t+1})·β_{t+1}(j)` — the convention
/// whose product `α_t·β_t` yields the posterior directly. `None` for
/// the same degenerate inputs as [`forward`].
pub fn backward(
    init: &[Fixed],
    trans: &[Vec<Fixed>],
    emit: &[Vec<Fixed>],
    obs: &[u32],
) -> Option<Vec<Vec<Fixed>>> {
    let s = init.len();
    let t_len = obs.len();
    if s == 0 || t_len == 0 {
        return None;
    }
    if trans.len() != s
        || emit.len() != s
        || trans.iter().any(|r| r.len() != s)
        || emit
            .iter()
            .any(|r| obs.iter().any(|&o| (o as usize) >= r.len()))
    {
        return None;
    }
    let mut beta: Vec<Vec<Fixed>> = vec![vec![Fixed::ZERO; s]; t_len];
    // β_T(i) = 1/Σ_k α̃_T(k)... the scaling constant at T is the
    // reciprocal of the forward row sum — recompute the forward sums.
    let mut scale = Vec::with_capacity(t_len);
    {
        let mut sum = Fixed::ZERO;
        for i in 0..s {
            sum = sum + init[i].mul(emit[i][obs[0] as usize]);
        }
        if sum.is_zero() {
            return None;
        }
        scale.push(Fixed::ONE.div(sum));
        let mut prev: Vec<Fixed> = (0..s)
            .map(|i| init[i].mul(emit[i][obs[0] as usize]).div(sum))
            .collect();
        for &ob in obs.iter().skip(1) {
            let o = ob as usize;
            let mut sum = Fixed::ZERO;
            let mut row = Vec::with_capacity(s);
            for i in 0..s {
                let mut acc = Fixed::ZERO;
                for j in 0..s {
                    acc = acc + prev[j].mul(trans[j][i]);
                }
                let v = acc.mul(emit[i][o]);
                row.push(v);
                sum = sum + v;
            }
            if sum.is_zero() {
                return None;
            }
            scale.push(Fixed::ONE.div(sum));
            prev = row.iter().map(|v| v.div(sum)).collect();
        }
    }
    for slot in beta[t_len - 1].iter_mut() {
        *slot = scale[t_len - 1];
    }
    for t in (0..t_len - 1).rev() {
        let o_next = obs[t + 1] as usize;
        for i in 0..s {
            let mut acc = Fixed::ZERO;
            for j in 0..s {
                acc = acc + trans[i][j].mul(emit[j][o_next]).mul(beta[t + 1][j]);
            }
            beta[t][i] = acc.mul(scale[t]);
        }
    }
    Some(beta)
}

/// Per-step posterior `γ_t(i) = P(S_t = i | O)` —
/// `α_t·β_t` renormalized per step. `None` on degenerate input.
pub fn posterior(
    init: &[Fixed],
    trans: &[Vec<Fixed>],
    emit: &[Vec<Fixed>],
    obs: &[u32],
) -> Option<Vec<Vec<Fixed>>> {
    let alpha = forward(init, trans, emit, obs)?;
    let beta = backward(init, trans, emit, obs)?;
    let t_len = obs.len();
    let s = init.len();
    let mut gamma = vec![vec![Fixed::ZERO; s]; t_len];
    for t in 0..t_len {
        let mut sum = Fixed::ZERO;
        for i in 0..s {
            gamma[t][i] = alpha[t][i].mul(beta[t][i]);
            sum = sum + gamma[t][i];
        }
        if !sum.is_zero() {
            for v in gamma[t].iter_mut() {
                *v = v.div(sum);
            }
        }
    }
    Some(gamma)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fr(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }
    fn to_f64(x: Fixed) -> f64 {
        x.raw() as f64 / 65536.0
    }

    /// f64 oracle forward-backward (plain, unscaled — fine for tiny T).
    fn f64_posterior(
        init: &[f64],
        trans: &[Vec<f64>],
        emit: &[Vec<f64>],
        obs: &[usize],
    ) -> Vec<Vec<f64>> {
        let s = init.len();
        let t_len = obs.len();
        let mut a = vec![vec![0.0; s]; t_len];
        for i in 0..s {
            a[0][i] = init[i] * emit[i][obs[0]];
        }
        for t in 1..t_len {
            for i in 0..s {
                a[t][i] = (0..s).map(|j| a[t - 1][j] * trans[j][i]).sum::<f64>() * emit[i][obs[t]];
            }
        }
        let mut b = vec![vec![1.0; s]; t_len];
        for t in (0..t_len - 1).rev() {
            for i in 0..s {
                b[t][i] = (0..s)
                    .map(|j| trans[i][j] * emit[j][obs[t + 1]] * b[t + 1][j])
                    .sum();
            }
        }
        (0..t_len)
            .map(|t| {
                let mut row: Vec<f64> = (0..s).map(|i| a[t][i] * b[t][i]).collect();
                let sum: f64 = row.iter().sum();
                for v in row.iter_mut() {
                    *v /= sum;
                }
                row
            })
            .collect()
    }

    /// The dishonest-casino lite model Rabiner-style: fair (0) vs
    /// loaded (1) dice; emission is biased on the loaded die.
    fn casino() -> (Vec<Fixed>, Vec<Vec<Fixed>>, Vec<Vec<Fixed>>) {
        let init = vec![fr(1, 2), fr(1, 2)];
        let trans = vec![vec![fr(19, 20), fr(1, 20)], vec![fr(1, 10), fr(9, 10)]];
        // fair: uniform 6; loaded: 6 comes up half the time.
        let fair = vec![fr(1, 6); 6];
        let mut loaded = vec![fr(1, 10); 5];
        loaded.push(fr(1, 2));
        let emit = vec![fair, loaded];
        (init, trans, emit)
    }

    #[test]
    fn forward_rows_normalize_and_detect_impossible() {
        let (pi, tr, em) = casino();
        let obs = [0u32, 1, 5, 5, 5];
        let alpha = forward(&pi, &tr, &em, &obs).unwrap();
        for row in &alpha {
            let sum = row[0] + row[1];
            assert!((sum - Fixed::ONE).raw().abs() < 16, "{sum:?}");
        }
        // obs ≥ emit width → None; empty → None.
        assert_eq!(forward(&pi, &tr, &em, &[6]), None);
        assert_eq!(forward(&pi, &tr, &em, &[]), None);
    }

    #[test]
    fn posterior_matches_f64_oracle() {
        let (pi, tr, em) = casino();
        let obs32 = [0u32, 1, 5, 5, 5, 5, 2, 5];
        let obs: Vec<usize> = obs32.iter().map(|&o| o as usize).collect();
        let g = posterior(&pi, &tr, &em, &obs32).unwrap();
        // backward is exercised directly too (posterior already calls it).
        assert!(backward(&pi, &tr, &em, &obs32).is_some());
        let pi64 = vec![0.5, 0.5];
        let tr64 = vec![vec![0.95, 0.05], vec![0.10, 0.90]];
        let mut loaded = vec![0.10f64; 5];
        loaded.push(0.5);
        let em64 = vec![vec![1.0 / 6.0; 6], loaded];
        let oracle = f64_posterior(&pi64, &tr64, &em64, &obs);
        for t in 0..obs.len() {
            for i in 0..2 {
                let got = to_f64(g[t][i]);
                let want = oracle[t][i];
                // Fixed quantization of inputs + truncation: ~4% slack.
                assert!((got - want).abs() < 0.04, "t={t} s={i}: {got} vs {want}");
            }
        }
        // Sixes in a row should pull the posterior toward loaded (s=1).
        assert!(g[3][1] > g[0][1]);
    }

    #[test]
    fn log_likelihood_tracks_probability() {
        let (pi, tr, em) = casino();
        // Many sixes: likelier than a fair-only sequence under this HMM.
        let sixy = log_likelihood(&pi, &tr, &em, &[5, 5, 5, 5]).unwrap();
        let flat = log_likelihood(&pi, &tr, &em, &[0, 1, 2, 3]).unwrap();
        assert!(sixy > flat, "{sixy:?} vs {flat:?}");
        // ln P < 0 for a nontrivial sequence.
        assert!(sixy < Fixed::ZERO);
        assert_eq!(log_likelihood(&pi, &tr, &em, &[]), None);
    }

    #[test]
    fn deterministic_twice() {
        let (pi, tr, em) = casino();
        let obs = [5u32, 5, 1, 5];
        assert_eq!(
            posterior(&pi, &tr, &em, &obs),
            posterior(&pi, &tr, &em, &obs)
        );
        assert_eq!(
            log_likelihood(&pi, &tr, &em, &obs),
            log_likelihood(&pi, &tr, &em, &obs)
        );
    }
}
