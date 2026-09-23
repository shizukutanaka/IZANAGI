//! Integer sorts with no comparison: stable counting sort and LSD
//! radix sort over `u64`. `radix_sort` runs eight 8-bit counting
//! passes — total work is linear in `n`, and because every pass is
//! stable the result is a pure function of the input multiset with
//! no pivot luck, no recursion, and no branches on data magnitude.
//!
//! ```
//! use izanagi_kit::radixsort;
//!
//! let mut v = vec![9, 1, u64::MAX, 0, 7, 7];
//! radixsort::radix_sort(&mut v);
//! assert_eq!(v, [0, 1, 7, 7, 9, u64::MAX]);
//! ```
//!
//! References: Knuth TAOCP 5.2.5 (distribution counting / radix
//! sorting), cp-algorithms "Counting sort".

/// Byte width per radix pass.
const DIGIT: u32 = 8;
/// Number of buckets per pass.
const BUCKETS: usize = 1 << DIGIT;

/// One stable counting pass over `f(i)`-style keys extracted by
/// `key`, moving items from `src` into `dst`. `key` must return a
/// value `< BUCKETS`.
fn counting_pass<T: Copy>(src: &[T], dst: &mut Vec<T>, key: impl Fn(&T) -> usize) {
    if src.is_empty() {
        dst.clear();
        return;
    }
    let mut cnt = [0usize; BUCKETS];
    for x in src {
        cnt[key(x)] += 1;
    }
    let mut sum = 0usize;
    for c in cnt.iter_mut() {
        let t = *c;
        *c = sum;
        sum += t;
    }
    dst.clear();
    dst.resize(src.len(), src[0]);
    for x in src {
        let d = key(x);
        dst[cnt[d]] = *x;
        cnt[d] += 1;
    }
}

/// Stable counting sort of values in `[0, bound)`; `None` when
/// `bound > u32::MAX as usize` or any value is `>= bound` — the
/// caller's contract is checked, not assumed, because a silent
/// out-of-range index would corrupt the count rather than error.
pub fn counting_sort(a: &[u64], bound: usize) -> Option<Vec<u64>> {
    if bound == 0 {
        return if a.is_empty() { Some(Vec::new()) } else { None };
    }
    if bound > BUCKETS * BUCKETS * BUCKETS {
        // Guard absurd bucket tables: beyond 2^24 buckets a Vec of
        // counters is just a sparse map in disguise.
        return None;
    }
    let mut cnt = vec![0usize; bound];
    for &x in a {
        if x as usize >= bound {
            return None;
        }
        cnt[x as usize] += 1;
    }
    let mut out = Vec::with_capacity(a.len());
    for (v, &c) in cnt.iter().enumerate() {
        for _ in 0..c {
            out.push(v as u64);
        }
    }
    Some(out)
}

/// In-place LSD radix sort of `a`, ascending: eight stable passes,
/// one per byte, most-significant last — the only random-access is
/// into a 256-entry counter.
pub fn radix_sort(a: &mut [u64]) {
    let mut src: Vec<u64> = a.to_vec();
    let mut dst: Vec<u64> = Vec::with_capacity(a.len());
    for pass in 0..(64 / DIGIT) {
        let shift = pass * DIGIT;
        counting_pass(&src, &mut dst, |x| ((x >> shift) & 0xff) as usize);
        std::mem::swap(&mut src, &mut dst);
    }
    a.copy_from_slice(&src);
}

/// Stable radix sort of `(key, payload)` pairs by `key`: equal keys
/// keep their input order, which is exactly the property radix
/// provides over quicksort when the secondary order is meaningful
/// (e.g. sorting events by tick while preserving insertion order).
pub fn radix_sort_pairs(p: &mut [(u64, u64)]) {
    let mut src: Vec<(u64, u64)> = p.to_vec();
    let mut dst: Vec<(u64, u64)> = Vec::with_capacity(p.len());
    for pass in 0..(64 / DIGIT) {
        let shift = pass * DIGIT;
        counting_pass(&src, &mut dst, |x| ((x.0 >> shift) & 0xff) as usize);
        std::mem::swap(&mut src, &mut dst);
    }
    p.copy_from_slice(&src);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn radix_matches_sort() {
        let mut r = SplitMix64::new(0x5EAD_5EED);
        for _ in 0..50 {
            let n = (r.below(300)) as usize;
            let mut v: Vec<u64> = (0..n).map(|_| r.next_u64()).collect();
            let mut w = v.clone();
            w.sort_unstable();
            radix_sort(&mut v);
            assert_eq!(v, w);
        }
    }

    #[test]
    fn radix_small_ranges() {
        for n in 0..50 {
            let mut v: Vec<u64> = (0..n).map(|i| (i * 37 % 11) as u64).collect();
            v.reverse();
            let mut w = v.clone();
            w.sort_unstable();
            radix_sort(&mut v);
            assert_eq!(v, w);
        }
    }

    #[test]
    fn counting_oracle_and_bounds() {
        let mut r = SplitMix64::new(0xC0FF_EE00);
        for _ in 0..30 {
            let bound = 1 + (r.below(1000)) as usize;
            let a: Vec<u64> = (0..r.below(400))
                .map(|_| r.next_u64() % bound as u64)
                .collect();
            let mut w = a.clone();
            w.sort_unstable();
            assert_eq!(counting_sort(&a, bound), Some(w));
        }
        // Out-of-range value is rejected, not silently miscounted.
        assert_eq!(counting_sort(&[3, 1, 9], 5), None);
        assert_eq!(counting_sort(&[0], 0), None);
        assert_eq!(counting_sort(&[], 0), Some(Vec::new()));
        assert_eq!(counting_sort(&[0], 1), Some(vec![0]));
    }

    #[test]
    fn pairs_are_stable() {
        // Same key, distinguishable payloads: output order within a
        // key must equal input order — radix passes are stable.
        let mut p: Vec<(u64, u64)> = (0..200).map(|i| (i % 17, i)).collect();
        p.reverse();
        // Stable oracle: std's sort_by_key is stable.
        let mut w = p.clone();
        w.sort_by_key(|x| x.0);
        radix_sort_pairs(&mut p);
        assert_eq!(p, w);
        // And prove stability mattered: payloads within a key keep
        // the reversed input order.
        assert_eq!(&p[..4], &[(0, 187), (0, 170), (0, 153), (0, 136)]);
    }
}
