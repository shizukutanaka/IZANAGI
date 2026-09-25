//! Goertzel single-frequency power detection and DTMF tone decoding.
//!
//! The Goertzel recurrence evaluates one Fourier bin in `O(N)` with a single
//! coefficient — cheap tone detection without a full FFT. All arithmetic is
//! Q16.16 fixed-point (`i64` intermediates), so results are bit-exact.
//!
//! ```
//! use izanagi_kit::goertzel;
//! // A strong exact-bin tone is detected with high power; silence is not.
//! assert_eq!(goertzel::power(&[0i32; 64], 697, 8000), 0);
//! ```

use crate::fixed::Fixed;

/// Goertzel power estimate of `freq` (Hz) inside `samples` taken at
/// `sample_rate` (Hz, ≤ 32767). `samples` are signed integer amplitudes (any
/// scale; power is proportional to amplitude²·N²). Returns 0 for empty input
/// or out-of-range frequency.
pub fn power(samples: &[i32], freq: u32, sample_rate: u32) -> u64 {
    power_fixed(
        samples,
        Fixed::from_int(freq.min(0x7fff) as i32),
        sample_rate,
    )
}

/// Same as [`power`] but accepts a fractional target frequency.
pub fn power_fixed(samples: &[i32], freq: Fixed, sample_rate: u32) -> u64 {
    let n = samples.len();
    if n == 0 || sample_rate == 0 {
        return 0;
    }
    // Fixed::from_int is limited to 15 integer bits of range.
    let sr = sample_rate.min(32767) as i32;
    // coeff = 2·cos(2πf/fs)
    let theta = Fixed::TWO_PI.mul(freq.div(Fixed::from_int(sr)));
    let (_, cos) = theta.sin_cos();
    let coeff = i64::from(Fixed::from_int(2).mul(cos).raw());
    // Recurrence in raw Q16.16 i64: resonant sums exceed i32 range.
    let mut s1: i64 = 0;
    let mut s2: i64 = 0;
    for &x in samples {
        let xc = x.clamp(-30_000, 30_000);
        let s = (i64::from(xc) << 16) + ((coeff * s1) >> 16) - s2;
        s2 = s1;
        s1 = s;
    }
    // power = s1² + s2² − coeff·s1·s2 (all in raw units)
    let p1 = i128::from(s1) * i128::from(s1);
    let p2 = i128::from(s2) * i128::from(s2);
    let pc = i128::from(coeff) * i128::from(s1) * i128::from(s2) / 65_536;
    let total = (p1 + p2 - pc) / 65_536;
    total.clamp(0, u64::MAX as i128) as u64
}

/// DTMF row frequencies (Hz).
pub const DTMF_ROWS: [u32; 4] = [697, 770, 852, 941];
/// DTMF column frequencies (Hz).
pub const DTMF_COLS: [u32; 4] = [1209, 1336, 1477, 1633];
/// DTMF keypad layout (`row`, `col`) → digit byte.
pub const DTMF_KEYS: [[u8; 4]; 4] = [*b"123A", *b"456B", *b"789C", *b"*0#D"];

/// Decode a DTMF digit from `samples` at `sample_rate`. Requires each group
/// winner to beat the group runner-up (otherwise `None` — silence or noise).
/// Returns the digit byte (`b'0'`..`b'9'`, `b'*'`, `b'#'`, `b'A'`..`b'D'`).
pub fn dtmf(samples: &[i32], sample_rate: u32) -> Option<u8> {
    if samples.len() < 8 || sample_rate == 0 {
        return None;
    }
    let pick = |freqs: &[u32; 4]| -> Option<(usize, u64)> {
        let mut best = (0usize, 0u64);
        let mut second = 0u64;
        for (i, &f) in freqs.iter().enumerate() {
            let p = power(samples, f, sample_rate);
            if p > best.1 {
                second = best.1;
                best = (i, p);
            } else if p > second {
                second = p;
            }
        }
        // Winner must dominate the runner-up and be non-trivial.
        if best.1 > second.saturating_mul(2) && best.1 > 1_000_000 {
            Some(best)
        } else {
            None
        }
    };
    let (row, rp) = pick(&DTMF_ROWS)?;
    let (col, cp) = pick(&DTMF_COLS)?;
    // Twist check: real DTMF digits carry comparable energy in both groups;
    // a lone tone only leaks into the other group and fails this bound.
    if rp > cp.saturating_mul(16) || cp > rp.saturating_mul(16) {
        return None;
    }
    Some(DTMF_KEYS[row][col])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-only float tone generator (tests may use f64 oracles).
    fn tone(freq: f64, fs: u32, n: usize, amp: i32) -> Vec<i32> {
        (0..n)
            .map(|i| {
                (amp as f64 * (2.0 * std::f64::consts::PI * freq * i as f64 / fs as f64).sin())
                    as i32
            })
            .collect()
    }

    #[test]
    fn detects_exact_bin() {
        let fs = 8000;
        let s = tone(1000.0, fs, 205, 4000);
        let at = power(&s, 1000, fs);
        let off = power(&s, 1200, fs);
        assert!(at > 0);
        assert!(at > off * 10);
    }

    #[test]
    fn silence_and_edges() {
        assert_eq!(power(&[], 1000, 8000), 0);
        assert_eq!(power(&[0i32; 128], 1000, 8000), 0);
        assert_eq!(power(&[5i32; 16], 1000, 0), 0);
        // Constant input has no tone energy at a nonzero bin.
        let dc = power(&[1000i32; 128], 1000, 8000);
        let ac = power(&tone(1000.0, 8000, 128, 1000), 1000, 8000);
        assert!(ac > dc * 20);
    }

    #[test]
    fn dtmf_all_sixteen() {
        let fs = 8000;
        let n = 205;
        for (r, &fr) in DTMF_ROWS.iter().enumerate() {
            for (c, &fc) in DTMF_COLS.iter().enumerate() {
                let s: Vec<i32> = (0..n)
                    .map(|i| {
                        (2000.0
                            * (2.0 * std::f64::consts::PI * fr as f64 * i as f64 / fs as f64).sin()
                            + 2000.0
                                * (2.0 * std::f64::consts::PI * fc as f64 * i as f64 / fs as f64)
                                    .sin()) as i32
                    })
                    .collect();
                assert_eq!(
                    dtmf(&s, fs),
                    Some(DTMF_KEYS[r][c]),
                    "digit {} at r{r} c{c}",
                    DTMF_KEYS[r][c] as char
                );
            }
        }
    }

    #[test]
    fn dtmf_rejects_silence_and_single_tone() {
        let fs = 8000;
        assert_eq!(dtmf(&[0i32; 205], fs), None);
        // Only a row tone: column group has no winner.
        let s = tone(697.0, fs, 205, 4000);
        assert_eq!(dtmf(&s, fs), None);
        assert_eq!(dtmf(&[1i32; 4], fs), None);
    }

    #[test]
    fn fractional_freq_resolves_between_bins() {
        // power_fixed tracks a non-integer target more closely than the
        // nearest integer bin does.
        let fs = 8000;
        let s = tone(1005.0, fs, 205, 4000);
        let frac = power_fixed(&s, Fixed::from_ratio(1005, 1), fs);
        assert!(frac > power(&s, 1200, fs));
        assert_eq!(frac, power_fixed(&s, Fixed::from_int(1005), fs));
    }

    #[test]
    fn deterministic_twice() {
        let s = tone(1336.0, 8000, 205, 3000);
        assert_eq!(power(&s, 1336, 8000), power(&s, 1336, 8000));
    }
}
