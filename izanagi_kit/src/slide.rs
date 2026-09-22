//! Sliding-window min/max in `O(n)` via a monotonic deque — the
//! classic monotone-queue trick for every window of width `w`:
//! `segtree`'s `O(n log n)` static answer compressed to linear when
//! the query range slides by one. The standard input for tick-window
//! aggregates (worst-case latency over the last `w` frames, extreme
//! values inside a moving patrol corridor).
//!
//! ```
//! use izanagi_kit::slide::{slide_max, slide_min};
//! assert_eq!(slide_min(&[3, 1, 4, 1, 5], 3), vec![1, 1, 1]);
//! assert_eq!(slide_max(&[3, 1, 4, 1, 5], 3), vec![4, 4, 5]);
//! ```

/// Per-window minima of `data[i..i+w]` for `i` in `0..=n−w`.
/// `w == 0` or `w > n` → empty. `w == 1` → `data` itself.
pub fn slide_min(data: &[i64], w: usize) -> Vec<i64> {
    slide(data, w, |a, b| a <= b)
}

/// Per-window maxima (same shape as [`slide_min`]).
pub fn slide_max(data: &[i64], w: usize) -> Vec<i64> {
    slide(data, w, |a, b| a >= b)
}

/// Monotonic deque: `q` holds indices, ordered so the queue's head
/// is always the window's best element; `keep` decides whether the
/// new element evicts the tail (pop while `keep(new, tail_val)`).
fn slide(data: &[i64], w: usize, keep: fn(i64, i64) -> bool) -> Vec<i64> {
    let n = data.len();
    if w == 0 || w > n {
        return Vec::new();
    }
    let mut q: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut out = Vec::with_capacity(n - w + 1);
    for i in 0..n {
        while let Some(&j) = q.back() {
            if keep(data[i], data[j]) {
                q.pop_back();
            } else {
                break;
            }
        }
        q.push_back(i);
        // Evict the head once it has slid out of the window.
        while let Some(&j) = q.front() {
            if j + w <= i {
                q.pop_front();
            } else {
                break;
            }
        }
        if i + 1 >= w {
            match q.front() {
                Some(&j) => out.push(data[j]),
                None => break,
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute-force window scan.
    fn oracle(data: &[i64], w: usize, take_min: bool) -> Vec<i64> {
        let n = data.len();
        if w == 0 || w > n {
            return Vec::new();
        }
        (0..=n - w)
            .map(|i| {
                let win = &data[i..i + w];
                let best = if take_min {
                    win.iter().min()
                } else {
                    win.iter().max()
                };
                best.copied().unwrap_or(0)
            })
            .collect()
    }

    #[test]
    fn matches_bruteforce_every_window() {
        let mut rng = SplitMix64::new(0x511D_3DEC);
        for _ in 0..400 {
            let n = rng.below(60) as usize;
            let w = rng.below(n as u32 + 2) as usize;
            let data: Vec<i64> = (0..n).map(|_| (rng.below(200) as i64) - 100).collect();
            assert_eq!(slide_min(&data, w), oracle(&data, w, true));
            assert_eq!(slide_max(&data, w), oracle(&data, w, false));
        }
        // Edge shapes.
        assert!(slide_min(&[], 0).is_empty());
        assert!(slide_min(&[1, 2], 5).is_empty());
        assert_eq!(slide_min(&[7, 3], 1), vec![7, 3]);
        assert_eq!(slide_max(&[7, 3], 1), vec![7, 3]);
        assert_eq!(slide_min(&[5, 4, 3, 2, 1], 5), vec![1]);
        assert_eq!(slide_max(&[5, 4, 3, 2, 1], 5), vec![5]);
        // Monotone input: min is the window head / max the tail.
        let asc: Vec<i64> = (0..10).collect();
        assert_eq!(slide_min(&asc, 4), vec![0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(slide_max(&asc, 4), vec![3, 4, 5, 6, 7, 8, 9]);
    }
}
