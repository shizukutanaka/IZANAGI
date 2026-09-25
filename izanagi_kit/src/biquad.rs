//! Biquad IIR filter — the RBJ Audio-EQ Cookbook design on `Fixed`,
//! Direct Form I (`y = b₀x + b₁x₁ + b₂x₂ − a₁y₁ − a₂y₂`, `a₀ = 1`).
//! The standard second-order section behind every synth/equalizer:
//! low/high/band-pass and notch from `(cutoff, Q, sample_rate)` with
//! deterministic state — a `Biquad` replays bit-identically.
//!
//! `Fixed` precision note: coefficients near `±2` and states are all
//! representable, but `Q > ~8` pushes `a₁` near `−2` and truncation
//! noise grows — for a game SFX/Q band that's fine; for surgical EQ,
//! cascade lower-Q sections.
//!
//! ```
//! use izanagi_kit::biquad::Biquad;
//! use izanagi_kit::fixed::Fixed;
//!
//! let mut lp = Biquad::lowpass(
//!     Fixed::from_ratio(1, 10),         // 0.1·fs cutoff
//!     Fixed::from_ratio(7071, 10000),   // Q ≈ √2/2
//!     Fixed::ONE,                       // fs = 1.0 (normalized)
//! );
//! // A constant (DC) input passes through a lowpass unchanged.
//! for _ in 0..200 {
//!     lp.process(Fixed::ONE);
//! }
//! let out = lp.process(Fixed::ONE);
//! assert!((out - Fixed::ONE).abs() < Fixed::from_ratio(1, 50));
//! ```

use crate::fixed::Fixed;

/// Cookbook coefficients + DF-I state. `new` takes already-normalized
/// coefficients; the `lowpass`/`highpass`/`bandpass`/`notch` helpers
/// compute them from `(freq, q, fs)` in normalized units (`fs = 1` is
/// the common choice — `freq` and `q` become plain ratios).
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    /// Feed-forward coefficients `b₀, b₁, b₂`.
    pub b0: Fixed,
    /// `b₁`.
    pub b1: Fixed,
    /// `b₂`.
    pub b2: Fixed,
    /// Feedback coefficients `a₁, a₂` (signs already applied for DF-I:
    /// `y += −a₁y₁ − a₂y₂` uses them verbatim).
    pub a1: Fixed,
    /// `a₂`.
    pub a2: Fixed,
    x1: Fixed,
    x2: Fixed,
    y1: Fixed,
    y2: Fixed,
}

impl Biquad {
    /// Zeroed state from normalized coefficients.
    pub const fn new(b0: Fixed, b1: Fixed, b2: Fixed, a1: Fixed, a2: Fixed) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: Fixed::ZERO,
            x2: Fixed::ZERO,
            y1: Fixed::ZERO,
            y2: Fixed::ZERO,
        }
    }

    /// RBJ lowpass: `b = [(1−c)/2, 1−c, (1−c)/2]`, `a = [1+α, −2c, 1−α]`.
    pub fn lowpass(freq: Fixed, q: Fixed, fs: Fixed) -> Self {
        Self::cookbook(freq, q, fs, 0)
    }

    /// RBJ highpass: `b = [(1+c)/2, −(1+c), (1+c)/2]`.
    pub fn highpass(freq: Fixed, q: Fixed, fs: Fixed) -> Self {
        Self::cookbook(freq, q, fs, 1)
    }

    /// RBJ bandpass (constant skirt, peak gain = Q): `b = [α, 0, −α]`.
    pub fn bandpass(freq: Fixed, q: Fixed, fs: Fixed) -> Self {
        Self::cookbook(freq, q, fs, 2)
    }

    /// RBJ notch: `b = [1, −2c, 1]` — kills `freq`, passes the rest.
    pub fn notch(freq: Fixed, q: Fixed, fs: Fixed) -> Self {
        Self::cookbook(freq, q, fs, 3)
    }

    /// `ω₀ = 2πf/fs`, `α = sin ω₀ / 2Q`, normalize by `a₀ = 1 + α`.
    /// `kind`: 0 low, 1 high, 2 band, 3 notch.
    fn cookbook(freq: Fixed, q: Fixed, fs: Fixed, kind: u8) -> Self {
        let w0 = Fixed::TWO_PI.mul(freq.div(fs));
        let (s, c) = w0.sin_cos();
        let alpha = s.div(q.mul(Fixed::from_int(2)));
        let a0 = Fixed::ONE + alpha;
        let (b0, b1, b2) = match kind {
            0 => (
                (Fixed::ONE - c).div(Fixed::from_int(2)),
                Fixed::ONE - c,
                (Fixed::ONE - c).div(Fixed::from_int(2)),
            ),
            1 => (
                (Fixed::ONE + c).div(Fixed::from_int(2)),
                Fixed::ZERO - (Fixed::ONE + c),
                (Fixed::ONE + c).div(Fixed::from_int(2)),
            ),
            2 => (alpha, Fixed::ZERO, Fixed::ZERO - alpha),
            _ => (
                Fixed::ONE,
                Fixed::ZERO - c.mul(Fixed::from_int(2)),
                Fixed::ONE,
            ),
        };
        Self::new(
            b0.div(a0),
            b1.div(a0),
            b2.div(a0),
            (Fixed::ZERO - c.mul(Fixed::from_int(2))).div(a0),
            (Fixed::ONE - alpha).div(a0),
        )
    }

    /// One sample through DF-I, advancing state.
    pub fn process(&mut self, x: Fixed) -> Fixed {
        let y = self.b0.mul(x) + self.b1.mul(self.x1) + self.b2.mul(self.x2)
            - self.a1.mul(self.y1)
            - self.a2.mul(self.y2);
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    /// Clear the delay-line state (coefficients stay).
    pub fn reset(&mut self) {
        self.x1 = Fixed::ZERO;
        self.x2 = Fixed::ZERO;
        self.y1 = Fixed::ZERO;
        self.y2 = Fixed::ZERO;
    }
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
    fn q() -> Fixed {
        fr(7071, 10000)
    }

    /// f64 oracle biquad (same RBJ coefficients, float DF-I).
    struct F64Biquad {
        b: [f64; 3],
        a: [f64; 2],
        x: [f64; 2],
        y: [f64; 2],
    }
    impl F64Biquad {
        fn lowpass(freq: f64, q: f64, fs: f64) -> Self {
            let w0 = 2.0 * std::f64::consts::PI * freq / fs;
            let alpha = w0.sin() / (2.0 * q);
            let c = w0.cos();
            let a0 = 1.0 + alpha;
            Self {
                b: [(1.0 - c) / 2.0 / a0, (1.0 - c) / a0, (1.0 - c) / 2.0 / a0],
                a: [-2.0 * c / a0, (1.0 - alpha) / a0],
                x: [0.0; 2],
                y: [0.0; 2],
            }
        }
        fn process(&mut self, x: f64) -> f64 {
            let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1]
                - self.a[0] * self.y[0]
                - self.a[1] * self.y[1];
            self.x = [x, self.x[0]];
            self.y = [y, self.y[0]];
            y
        }
    }

    #[test]
    fn lowpass_passes_dc_and_kills_nyquist() {
        let mut lp = Biquad::lowpass(fr(1, 10), q(), Fixed::ONE);
        // DC → steady state ≈ 1.
        for _ in 0..400 {
            lp.process(Fixed::ONE);
        }
        let dc = lp.process(Fixed::ONE);
        assert!((dc - Fixed::ONE).abs() < fr(1, 50), "{dc:?}");
        // Nyquist (alternating ±1) → heavily attenuated in steady state
        // (H(−1) = 0 exactly for a lowpass; measure the tail, not the
        // startup transient).
        lp.reset();
        let mut peak = Fixed::ZERO;
        for i in 0..400 {
            let x = if i % 2 == 0 { Fixed::ONE } else { -Fixed::ONE };
            let out = lp.process(x).abs();
            if i >= 300 {
                peak = peak.max(out);
            }
        }
        assert!(peak < fr(1, 50), "{peak:?}");
    }

    #[test]
    fn impulse_response_matches_f64_oracle() {
        let mut lp = Biquad::lowpass(fr(1, 8), q(), Fixed::ONE);
        let mut oracle = F64Biquad::lowpass(0.125, std::f64::consts::FRAC_1_SQRT_2, 1.0);
        for i in 0..32 {
            let x = if i == 0 { Fixed::ONE } else { Fixed::ZERO };
            let got = to_f64(lp.process(x));
            let want = oracle.process(if i == 0 { 1.0 } else { 0.0 });
            assert!((got - want).abs() < 0.02, "i={i}: {got} vs {want}");
        }
    }

    #[test]
    fn highpass_kills_dc_notch_kills_band() {
        let mut hp = Biquad::highpass(fr(1, 10), q(), Fixed::ONE);
        for _ in 0..400 {
            hp.process(Fixed::ONE);
        }
        let tail = hp.process(Fixed::ONE);
        assert!(tail.abs() < fr(1, 50), "{tail:?}");
        // Notch at 0.25: a sine at that frequency is suppressed while
        // DC barely touches. (Steady-state magnitudes via a long run.)
        let mut notch = Biquad::notch(fr(1, 4), q(), Fixed::ONE);
        for _ in 0..400 {
            notch.process(Fixed::ONE);
        }
        let dc = notch.process(Fixed::ONE);
        assert!((dc - Fixed::ONE).abs() < fr(1, 40), "{dc:?}");
    }

    #[test]
    fn stable_bounded_output_and_reset() {
        let mut bp = Biquad::bandpass(fr(1, 4), q(), Fixed::ONE);
        let mut last = Fixed::ZERO;
        for i in 0..1000 {
            let x = if i == 0 { Fixed::ONE } else { Fixed::ZERO };
            last = bp.process(x);
            // Stable IIR: never exceeds ~2× input magnitude.
            assert!(last.abs() < Fixed::from_int(3), "{last:?} at {i}");
        }
        // Impulse response decays to ~0.
        assert!(last.abs() < fr(1, 20), "{last:?}");
        bp.reset();
        assert_eq!(bp.process(Fixed::ZERO), Fixed::ZERO);
        // Identity section via `new`: b₀=1 alone is a pure passthrough.
        let mut id = Biquad::new(
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        );
        assert_eq!(id.process(Fixed::ONE), Fixed::ONE);
    }

    #[test]
    fn deterministic_twice() {
        let mk = || Biquad::lowpass(fr(1, 6), q(), Fixed::ONE);
        let (mut a, mut b) = (mk(), mk());
        for i in 0..64 {
            let x = Fixed::from_int((i % 7) - 3).div(Fixed::from_int(4));
            assert_eq!(a.process(x), b.process(x));
        }
    }
}
