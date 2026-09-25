//! Otsu's method — automatic bilevel thresholding for a grayscale
//! histogram. Picks the threshold `t` that maximizes the
//! *between-class variance*: the two intensity classes are as
//! different from each other as possible, equivalently minimizing
//! total within-class variance. Pure histogram arithmetic — the
//! companion to `ccl` for turning a gray image into a labelable
//! bitmap.
//!
//! ```
//! use izanagi_kit::otsu::{otsu_threshold, threshold_image};
//!
//! // Bimodal image: dark half + bright half.
//! let img: Vec<u8> = (0..100)
//!     .map(|i| if i < 50 { 30 } else { 200 })
//!     .collect();
//! let t = otsu_threshold(&img);
//! assert_eq!(t, 30); // boundary between the two classes
//! let bin = threshold_image(&img, t);
//! assert!(!bin[0] && bin[99]);
//! ```

/// Otsu's threshold for a grayscale image (0–255 each). Returns the
/// first intensity maximizing between-class variance — matching the
/// canonical "smallest maximizer" convention, so threshold `t` means
/// `pixel > t` is foreground. Empty or single-tone input returns 0.
pub fn otsu_threshold(img: &[u8]) -> u8 {
    let mut hist = [0u64; 256];
    for &p in img {
        hist[p as usize] += 1;
    }
    let total = img.len() as u64;
    if total == 0 {
        return 0;
    }
    let mut sum_all = 0u64;
    for (i, &h) in hist.iter().enumerate() {
        sum_all += h * i as u64;
    }
    // Between-class variance ∝ (Σ_all·w₀ − Σ₀·total)² / (w₀·(total−w₀)).
    // Compare as f64-free integer cross-multiplication.
    let mut w0 = 0u64;
    let mut sum0 = 0u64;
    let mut best: i128 = -1;
    let mut best_t = 0u8;
    for (t, &h) in hist.iter().enumerate().take(255) {
        w0 += h;
        sum0 += h * t as u64;
        let w1 = total - w0;
        if w0 == 0 || w1 == 0 {
            continue;
        }
        // numerator of σ_b² proportional expression — keep as
        // squared difference *denominator-free* form: comparing
        // d²/(w0·w1) across t needs the denominator, so compute the
        // actual fraction scaled by w0·w1 into i128.
        let d = (sum_all * w0) as i128 - (sum0 * total) as i128;
        let num = d * d;
        let den = (w0 * w1) as i128;
        let score = num / den;
        if score > best {
            best = score;
            best_t = t as u8;
        }
    }
    best_t
}

/// Apply `t`: `img[i] > t` → `true` (foreground).
pub fn threshold_image(img: &[u8], t: u8) -> Vec<bool> {
    img.iter().map(|&p| p > t).collect()
}

/// Full pipeline on `pixels`: returns `(threshold, binary)`.
pub fn binarize(img: &[u8]) -> (u8, Vec<bool>) {
    let t = otsu_threshold(img);
    (t, threshold_image(img, t))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force oracle: within-class variance minimized ⇔
    /// between-class maximized; compute σ_b² directly in rationals.
    fn oracle(img: &[u8]) -> u8 {
        let mut best = (0u8, 0f64);
        for t in 0..255u32 {
            let (mut w0, mut s0, mut w1, mut s1) = (0u64, 0u64, 0u64, 0u64);
            for &p in img {
                if p as u32 <= t {
                    w0 += 1;
                    s0 += p as u64;
                } else {
                    w1 += 1;
                    s1 += p as u64;
                }
            }
            if w0 == 0 || w1 == 0 {
                continue;
            }
            let m0 = s0 as f64 / w0 as f64;
            let m1 = s1 as f64 / w1 as f64;
            let sb = w0 as f64 * w1 as f64 * (m0 - m1) * (m0 - m1);
            if sb > best.1 {
                best = (t as u8, sb);
            }
        }
        best.0
    }

    #[test]
    fn bimodal_cases() {
        // Two tight clusters.
        assert_eq!(
            otsu_threshold(
                &[10; 50]
                    .iter()
                    .chain([200; 50].iter())
                    .copied()
                    .collect::<Vec<_>>()
            ),
            10
        );
        // Skewed masses: small bright blob on dark field.
        let img: Vec<u8> = (0..200).map(|i| if i < 5 { 180 } else { 40 }).collect();
        assert_eq!(otsu_threshold(&img), 40);
        // Noisy bimodal.
        let img: Vec<u8> = (0..300)
            .map(|i| {
                if i % 2 == 0 {
                    60 + (i % 7) as u8
                } else {
                    180 - (i % 5) as u8
                }
            })
            .collect();
        let t = otsu_threshold(&img);
        assert!((66..=175).contains(&t), "t={t}");
    }

    #[test]
    fn matches_brute_oracle() {
        // Deterministic random-ish images.
        for seed in 0..50u64 {
            let img: Vec<u8> = (0..257usize)
                .map(|i| {
                    let x = (i as u64)
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(seed + 1);
                    ((x >> 33) % 256) as u8
                })
                .collect();
            assert_eq!(otsu_threshold(&img), oracle(&img), "seed {seed}");
        }
    }

    #[test]
    fn edge_cases() {
        assert_eq!(otsu_threshold(&[]), 0);
        assert_eq!(otsu_threshold(&[77; 100]), 0);
        assert_eq!(otsu_threshold(&[0, 255]), 0);
        assert_eq!(binarize(&[0, 128, 255]), (0, vec![false, true, true]));
        // threshold_image direct: pixel > t is foreground.
        assert_eq!(
            threshold_image(&[0, 128, 255], 128),
            vec![false, false, true]
        );
    }

    #[test]
    fn deterministic_twice() {
        let img: Vec<u8> = (0..255).collect();
        assert_eq!(otsu_threshold(&img), otsu_threshold(&img));
        assert_eq!(binarize(&img), binarize(&img));
    }
}
