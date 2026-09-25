//! Player rating systems over [`Fixed`]: Elo (Arpad Elo's expectation model,
//! `1/(1+10^{-Δ/400})`) and Glicko-1 (Mark Glickman's rating-deviation
//! extension that grows uncertainty between games and weights each result by
//! the opponent's reliability).
//!
//! Both are deterministic closed forms — `Fixed::exp` supplies the 10^x
//! transform, so no lookup tables or floats are involved. Ratings live in
//! the familiar scale (Elo arbitrary units; Glicko centered at 1500 with RD
//! 350) but stored as `Fixed`, so replay hashes stay bit-exact.
//!
//! ```
//! use izanagi_kit::{elo, fixed::Fixed};
//! let (a, b) = elo::update_pair(Fixed::from_int(1500), Fixed::from_int(1500), Fixed::ONE, Fixed::from_int(32));
//! assert_eq!(a, Fixed::from_int(1516));
//! assert_eq!(b, Fixed::from_int(1484));
//! ```

use crate::fixed::Fixed;

/// Elo expected score of A versus B: `1/(1+10^((rb-ra)/400))`.
pub fn expected(ra: Fixed, rb: Fixed) -> Fixed {
    let d = rb - ra;
    // 10^(d/400) = exp(d * ln(10)/400)
    let ln10 = Fixed::ln(Fixed::from_int(10));
    let expo = d.mul(ln10).div(Fixed::from_int(400));
    let p = Fixed::exp(expo);
    Fixed::ONE.div(p + Fixed::ONE)
}

/// New rating for A after scoring `score` (in `[0,1]`) versus `rb` with K-factor
/// `k`.
pub fn update(ra: Fixed, rb: Fixed, score: Fixed, k: Fixed) -> Fixed {
    ra + k.mul(score - expected(ra, rb))
}

/// Symmetric update: returns `(new_a, new_b)` where B's score is `1 - score`.
/// Zero-sum: the ratings exchange `k*(score - E)` exactly.
pub fn update_pair(ra: Fixed, rb: Fixed, score: Fixed, k: Fixed) -> (Fixed, Fixed) {
    let gain = k.mul(score - expected(ra, rb));
    (ra + gain, rb - gain)
}

fn ln10_over_400() -> Fixed {
    // q = ln(10)/400, the Glicko scale factor.
    Fixed::ln(Fixed::from_int(10)).div(Fixed::from_int(400))
}

fn pi_sq() -> Fixed {
    Fixed::PI.mul(Fixed::PI)
}

/// `(a*b) >> 32` on Q32.32 operands (i128 intermediate).
fn mul32(a: i64, b: i64) -> i64 {
    ((a as i128 * b as i128) >> 32) as i64
}

/// `(a << 32) / b` on Q32.32 operands (i128 intermediate).
fn div32(a: i64, b: i64) -> i64 {
    if b == 0 {
        return i64::MAX;
    }
    (((a as i128) << 32) / b as i128) as i64
}

/// `floor(sqrt(v))` for u128 (binary search — deterministic, no floats).
fn isqrt(v: u128) -> u64 {
    let mut lo = 0u128;
    let mut hi = 1u128 << 64;
    while hi - lo > 1 {
        let m = (lo + hi) / 2;
        if m * m <= v {
            lo = m;
        } else {
            hi = m;
        }
    }
    lo as u64
}

/// `g(d) = 1/sqrt(1 + 3 q² d² / π²)` — the Glicko reliability weight of an
/// opponent with deviation `d`.
fn g(d: Fixed) -> Fixed {
    let q = ln10_over_400();
    let three_q2_d2 = Fixed::from_int(3).mul(q).mul(q).mul(d).mul(d);
    Fixed::ONE.div((Fixed::ONE + three_q2_d2.div(pi_sq())).sqrt())
}

/// A Glicko-1 rating: mean `r` plus rating deviation `rd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glicko {
    /// Rating estimate (scale like Elo; typical pool centers near 1500).
    pub r: Fixed,
    /// Rating deviation — uncertainty in `r` (68% interval ~ r±rd).
    pub rd: Fixed,
}

impl Glicko {
    /// Standard new-player seed: 1500 ± 350.
    pub const fn seed() -> Self {
        // const-friendly: raw Q16.16 forms of 1500 and 350.
        Self {
            r: Fixed::from_raw(1500 << 16),
            rd: Fixed::from_raw(350 << 16),
        }
    }

    /// Glicko-1 inflation after `periods` rating periods of no games:
    /// `rd' = min(sqrt(rd² + c²·t), rd_max)` with `c` the volatility constant.
    /// Computed on raw Q16 squares in i64 — `Fixed::mul` of two large
    /// deviations would overflow i32.
    pub fn decayed(&self, periods: i32, c: Fixed, rd_max: Fixed) -> Self {
        if periods <= 0 {
            return *self;
        }
        let ct = c.mul(Fixed::from_int(periods));
        let r2 =
            (self.rd.raw() as i64) * (self.rd.raw() as i64) + (ct.raw() as i64) * (ct.raw() as i64);
        let rd_new = Fixed::from_raw(isqrt(r2 as u128) as i32);
        Self {
            r: self.r,
            rd: rd_new.min(rd_max),
        }
    }

    /// Glicko-1 update after one result `s` (in `[0,1]`) against `opp`.
    ///
    /// `E = 1/(1+10^(-g(rd_b)·(ra-rb)/400))`; `d² = [q²·g²·E(1-E)]^{-1}`;
    /// `r' = r + (q / (1/rd² + 1/d²))·g·(s-E)`; `rd' = sqrt(1/(1/rd²+1/d²))`.
    /// When `E` saturates to 0/1 the variance `d²` diverges — the result is a
    /// no-op rating change with `rd` inflated by `c` is NOT applied here;
    /// callers batch periods via [`decayed`](Self::decayed) instead.
    pub fn update(&self, opp: &Glicko, s: Fixed) -> Glicko {
        let q = ln10_over_400();
        let g_opp = g(opp.rd);
        let delta = self.r - opp.r;
        let expo = Fixed::ZERO - g_opp.mul(delta).div(Fixed::from_int(400));
        // E = 1/(1 + 10^expo), and 10^x = exp(x ln 10)
        let e = Fixed::ONE.div(Fixed::exp(expo.mul(Fixed::ln(Fixed::from_int(10)))) + Fixed::ONE);
        let e_comp = e.mul(Fixed::ONE - e);
        if e_comp.raw() <= 0 {
            // Perfectly certain outcome — no information; keep r, decay rd a
            // touch is left to `decayed`.
            return *self;
        }
        // The variance denominators (~1e-5) sit below Q16 resolution, so the
        // tail of the formula runs in i64 Q32.32 (`mul32`/`div32`).
        let (q32, g32, e32, ec32) = (
            (q.raw() as i64) << 16,
            (g_opp.raw() as i64) << 16,
            (e.raw() as i64) << 16,
            ((Fixed::ONE - e).raw() as i64) << 16,
        );
        let rd32 = (self.rd.raw() as i64) << 16;
        let d2_inv = mul32(mul32(mul32(q32, q32), mul32(g32, g32)), mul32(e32, ec32));
        let inv_rd2 = div32(1i64 << 32, mul32(rd32, rd32));
        let denom = inv_rd2 + d2_inv;
        if denom <= 0 {
            return *self;
        }
        let num = mul32(mul32(q32, g32), (s - e).raw() as i64 * 65536);
        let dr = (div32(num, denom) >> 16).clamp(i32::MIN as i64, i32::MAX as i64);
        let r_new = self.r + Fixed::from_raw(dr as i32);
        // rd'² in Q32 equals rd'_raw² (Q16 raw squared) — take the root
        // directly, no rescaling.
        let rd_new_sq = div32(1i64 << 32, denom);
        let rd_new = Fixed::from_raw(isqrt(rd_new_sq as u128).min(i32::MAX as u64) as i32);
        Glicko {
            r: r_new,
            rd: rd_new,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, want: f64, eps: f64) {
        let got = a.raw() as f64 / 65536.0;
        assert!(
            (got - want).abs() <= eps,
            "got {got}, want {want} (eps {eps})"
        );
    }

    #[test]
    fn elo_expectation() {
        // Equal ratings → 0.5.
        close(
            expected(Fixed::from_int(1500), Fixed::from_int(1500)),
            0.5,
            0.001,
        );
        // 400 points up → ~0.909.
        close(
            expected(Fixed::from_int(1900), Fixed::from_int(1500)),
            0.909,
            0.002,
        );
        // Symmetry.
        close(
            expected(Fixed::from_int(1500), Fixed::from_int(1900)),
            0.091,
            0.002,
        );
    }

    #[test]
    fn elo_update_pair_is_zero_sum() {
        let (a, b) = update_pair(
            Fixed::from_int(1600),
            Fixed::from_int(1400),
            Fixed::ONE,
            Fixed::from_int(32),
        );
        close(a, 1607.69, 0.3);
        close(b, 1392.31, 0.3);
        // Ratings exchanged exactly.
        assert_eq!(a - Fixed::from_int(1600), Fixed::from_int(1400) - b);
    }

    #[test]
    fn elo_upset_gains_more() {
        let fav_gain = update(
            Fixed::from_int(1900),
            Fixed::from_int(1500),
            Fixed::ONE,
            Fixed::from_int(16),
        ) - Fixed::from_int(1900);
        let dog_gain = update(
            Fixed::from_int(1500),
            Fixed::from_int(1900),
            Fixed::ONE,
            Fixed::from_int(16),
        ) - Fixed::from_int(1500);
        assert!(dog_gain > fav_gain);
        close(dog_gain, 14.55, 0.2);
    }

    #[test]
    fn glicko_seed_and_decay() {
        let s = Glicko::seed();
        assert_eq!(s.r, Fixed::from_int(1500));
        assert_eq!(s.rd, Fixed::from_int(350));
        let d = s.decayed(10, Fixed::from_ratio(50, 10), Fixed::from_int(350));
        assert!(d.rd >= s.rd);
        assert_eq!(d.r, s.r);
        // No periods → unchanged.
        assert_eq!(s.decayed(0, Fixed::from_int(50), Fixed::from_int(350)), s);
    }

    #[test]
    fn glicko_update_reduces_rd_and_moves_r() {
        let me = Glicko {
            r: Fixed::from_int(1500),
            rd: Fixed::from_int(200),
        };
        let opp = Glicko {
            r: Fixed::from_int(1400),
            rd: Fixed::from_int(30),
        };
        let w = me.update(&opp, Fixed::ONE); // won vs weaker
        assert!(w.r > me.r);
        assert!(w.rd < me.rd); // game reduced uncertainty
                               // Losing instead must move r the other way.
        let l = me.update(&opp, Fixed::ZERO);
        assert!(l.r < me.r);
        // Determinism.
        assert_eq!(w, me.update(&opp, Fixed::ONE));
    }

    #[test]
    fn glicko_oracle() {
        // f64 reference of the same formulas, matched within Fixed error.
        let ra = 1500.0f64;
        let rda = 200.0f64;
        let rb = 1400.0f64;
        let rdb = 30.0f64;
        let s = 1.0f64;
        let q = 10.0f64.ln() / 400.0;
        let g = 1.0 / (1.0 + 3.0 * q * q * rdb * rdb / std::f64::consts::PI.powi(2)).sqrt();
        let e = 1.0 / (1.0 + 10.0f64.powf(-g * (ra - rb) / 400.0));
        let d2 = 1.0 / (q * q * g * g * e * (1.0 - e));
        let denom = 1.0 / (rda * rda) + 1.0 / d2;
        let r_new = ra + q / denom * g * (s - e);
        let rd_new = (1.0 / denom).sqrt();
        let me = Glicko {
            r: Fixed::from_int(1500),
            rd: Fixed::from_int(200),
        };
        let opp = Glicko {
            r: Fixed::from_int(1400),
            rd: Fixed::from_int(30),
        };
        let w = me.update(&opp, Fixed::ONE);
        close(w.r, r_new, 2.0);
        close(w.rd, rd_new, 1.0);
    }
}
