//! Grayscale dithering: Bayer ordered dithering and Floyd–Steinberg error
//! diffusion, over `u8` intensity planes.
//!
//! Ordered dithering thresholds each pixel against a periodic Bayer matrix;
//! error diffusion pushes each pixel's quantization error onto its
//! not-yet-processed neighbors. Both are pure integer kernels, so results are
//! bit-exact across platforms.
//!
//! ```
//! use izanagi_kit::dither;
//! let img = vec![128u8; 16]; // uniform mid-gray, 4x4
//! let out = dither::ordered(&img, 4, 4, 2);
//! assert_eq!(out.len(), 16);
//! // Half the pixels snap black, half white — deterministic pattern.
//! let whites = out.iter().filter(|&&v| v == 255).count();
//! assert_eq!(whites, 8);
//! ```

/// Bayer 4×4 threshold matrix, values 0..=15.
const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Bayer 8×8 threshold matrix, values 0..=63.
const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Threshold entry of the Bayer `n`×`n` matrix at `(x % n, y % n)`,
/// for `n` = 4 or 8 (other sizes return 0).
pub fn bayer(x: usize, y: usize, n: usize) -> u8 {
    match n {
        4 => BAYER4[y & 3][x & 3],
        8 => BAYER8[y & 7][x & 7],
        _ => 0,
    }
}

/// Quantize `v` to the nearest of `levels` evenly spaced intensities
/// (2 = pure black/white). `levels` clamps to 2..=256.
pub fn quantize(v: u8, levels: u32) -> u8 {
    let levels = levels.clamp(2, 256);
    let scaled = (v as u32) * (levels - 1) + 127;
    let idx = scaled / 255;
    if idx > levels - 1 {
        return 255;
    }
    (idx * 255 / (levels - 1)) as u8
}

/// Ordered dither: map `gray` (row-major `w`×`h`) to `levels` intensities
/// using the `size`×`size` Bayer matrix (`size` 4 or 8; else no-op copy).
pub fn ordered(gray: &[u8], w: usize, h: usize, levels: u32) -> Vec<u8> {
    ordered_n(gray, w, h, levels, 4)
}

/// Ordered dither with explicit matrix `size` (4 or 8; else copies input).
pub fn ordered_n(gray: &[u8], w: usize, h: usize, levels: u32, size: usize) -> Vec<u8> {
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mut out = vec![0u8; w * h];
    let max_t = size * size;
    for y in 0..h {
        for x in 0..w {
            let v = *gray.get(y * w + x).unwrap_or(&0);
            // Compare v*max_t against the distributed threshold centers.
            let t = bayer(x, y, size) as u32;
            // Level index with Bayer-biased rounding.
            let lv = (levels.clamp(2, 256) - 1) as u64;
            let scaled = (v as u64) * lv * (max_t as u64) + (t as u64) * 255 + (max_t as u64 / 2);
            let idx = (scaled / (255u64 * max_t as u64)).min(lv);
            out[y * w + x] = (idx * 255 / lv) as u8;
        }
    }
    out
}

/// Floyd–Steinberg error-diffusion dither to `levels` intensities.
///
/// Error distributes right 7/16, down-left 3/16, down 5/16, down-right 1/16.
pub fn floyd_steinberg(gray: &[u8], w: usize, h: usize, levels: u32) -> Vec<u8> {
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mut buf: Vec<i32> = gray.iter().take(w * h).map(|&v| v as i32 * 16).collect();
    buf.resize(w * h, 0);
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let v = (buf[i] / 16).clamp(0, 255) as u8;
            let q = quantize(v, levels);
            out[i] = q;
            // Error against the clamped pixel value keeps the feedback loop
            // bounded (unclamped buffers can grow without limit on extreme
            // inputs).
            let err = (v as i32) * 16 - (q as i32) * 16;
            if x + 1 < w {
                buf[i + 1] += err * 7;
            }
            if y + 1 < h {
                if x > 0 {
                    buf[i + w - 1] += err * 3;
                }
                buf[i + w] += err * 5;
                if x + 1 < w {
                    buf[i + w + 1] += err;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bayer_matrices_match_reference() {
        assert_eq!(BAYER4[0], [0, 8, 2, 10]);
        assert_eq!(BAYER4[3], [15, 7, 13, 5]);
        assert_eq!(bayer(0, 0, 8), 0);
        assert_eq!(bayer(1, 0, 8), 32);
        assert_eq!(bayer(5, 5, 8), 17);
        // Wrap-around period.
        assert_eq!(bayer(9, 9, 8), bayer(1, 1, 8));
        assert_eq!(bayer(4, 4, 4), bayer(0, 0, 4));
    }

    #[test]
    fn quantize_binary() {
        assert_eq!(quantize(0, 2), 0);
        assert_eq!(quantize(255, 2), 255);
        assert_eq!(quantize(126, 2), 0);
        assert_eq!(quantize(129, 2), 255);
        // Four levels: 0, 85, 170, 255.
        assert_eq!(quantize(80, 4), 85);
        assert_eq!(quantize(200, 4), 170);
    }

    #[test]
    fn ordered_extremes() {
        let black = ordered(&[0u8; 16], 4, 4, 2);
        assert!(black.iter().all(|&v| v == 0));
        let white = ordered(&[255u8; 16], 4, 4, 2);
        assert!(white.iter().all(|&v| v == 255));
    }

    #[test]
    fn ordered_midgray_mix() {
        let out = ordered(&[128u8; 64], 8, 8, 2);
        let whites = out.iter().filter(|&&v| v == 255).count();
        // Bayer ordering halves the field with a deterministic layout.
        assert_eq!(whites, 32);
    }

    #[test]
    fn fs_extremes_and_empty() {
        assert!(floyd_steinberg(&[], 0, 0, 2).is_empty());
        assert!(floyd_steinberg(&[0u8; 9], 3, 3, 2).iter().all(|&v| v == 0));
        assert!(floyd_steinberg(&[255u8; 9], 3, 3, 2)
            .iter()
            .all(|&v| v == 255));
        // Odd width: last column has no right/down-right neighbor.
        let out = floyd_steinberg(&[255u8; 15], 5, 3, 2);
        assert!(out.iter().all(|&v| v == 255));
        // Constant mid tone dithers to a deterministic black/white mix.
        let out = floyd_steinberg(&[200u8; 15], 5, 3, 2);
        let whites = out.iter().filter(|&&v| v == 255).count();
        assert!((4..=12).contains(&whites));
    }

    #[test]
    fn deterministic_twice() {
        let img: Vec<u8> = (0..96u32).map(|i| (i * 37 % 256) as u8).collect();
        assert_eq!(ordered(&img, 12, 8, 2), ordered(&img, 12, 8, 2));
        assert_eq!(
            floyd_steinberg(&img, 12, 8, 4),
            floyd_steinberg(&img, 12, 8, 4)
        );
        assert_eq!(ordered_n(&img, 12, 8, 2, 8), ordered_n(&img, 12, 8, 2, 8));
    }
}
