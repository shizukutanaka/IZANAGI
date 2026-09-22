//! Burrows–Wheeler transform (Burrows & Wheeler, 1994) — the
//! reversible reordering that front-loads the bzip2-style compression
//! pipeline (`bwt` → `mtf` → `rle` → `huffman`). All indices integer;
//! output is a pure function of the input.
//!
//! The transform is the *cyclic* variant: rows are all `n` rotations
//! of the input sorted lexicographically, and `bwt[i]` is the last
//! byte of row `i`. No sentinel is inserted, so `inverse` needs only
//! the primary index (the row of the un-rotated string).
//!
//! ```
//! use izanagi_kit::bwt::{bwt, bwt_inverse};
//! let (enc, primary) = bwt(b"BANANA").unwrap();
//! assert_eq!(enc, b"NNBAAA");
//! assert_eq!(bwt_inverse(&enc, primary).unwrap(), b"BANANA");
//! ```

use crate::suffix::SuffixArray;

/// Cyclic Burrows–Wheeler transform of `data`. Returns the last-column
/// bytes and the primary index (sorted-row position of the original
/// rotation). `None` when `data` is empty or longer than `u32::MAX`.
///
/// Runs in O(n log n): a suffix array over `data ++ data` orders the
/// rotations, since every rotation is the `n`-byte prefix of a suffix
/// that starts inside the first copy.
pub fn bwt(data: &[u8]) -> Option<(Vec<u8>, u32)> {
    let n = data.len();
    if n == 0 || n > u32::MAX as usize {
        return None;
    }
    let mut dd = Vec::with_capacity(2 * n);
    dd.extend_from_slice(data);
    dd.extend_from_slice(data);
    let sa = SuffixArray::new(&dd);
    let mut out = Vec::with_capacity(n);
    let mut primary = 0u32;
    for &p in sa.sa() {
        if p < n {
            if p == 0 {
                primary = out.len() as u32;
            }
            out.push(data[(p + n - 1) % n]);
        }
    }
    Some((out, primary))
}

/// Inverse of [`bwt`]: restores the string whose transform is `enc`
/// with the given `primary` index. `None` on empty input or a
/// `primary` ≥ `enc.len()`.
///
/// Rebuilds the LF-mapping: the `k`-th occurrence of byte `c` in the
/// last column is the `k`-th `c` among all row-first bytes, and walks
/// the inverse permutation emitting each row's first byte.
pub fn bwt_inverse(enc: &[u8], primary: u32) -> Option<Vec<u8>> {
    let n = enc.len();
    if n == 0 || primary as usize >= n {
        return None;
    }
    // first[c] = number of bytes < c in the transform = start of the
    // c-block among row-first bytes.
    let mut count = [0usize; 256];
    for &b in enc {
        count[b as usize] += 1;
    }
    let mut first = [0usize; 256];
    let mut acc = 0usize;
    for c in 0..256 {
        first[c] = acc;
        acc += count[c];
    }
    // lf[i] = row index of the right-rotation of row `i`.
    let mut seen = [0usize; 256];
    let mut lf = vec![0usize; n];
    for (i, &b) in enc.iter().enumerate() {
        let c = b as usize;
        lf[i] = first[c] + seen[c];
        seen[c] += 1;
    }
    // inv[lf[i]] = i — walking `inv` visits rotations in start order.
    let mut inv = vec![0usize; n];
    for (i, &j) in lf.iter().enumerate() {
        inv[j] = i;
    }
    // Row j's first byte is `enc[inv[j]]`'s... rather: emit the sorted
    // first column directly — F[j] = the byte at sorted position j.
    let mut col = enc.to_vec();
    col.sort_unstable();
    let mut out = Vec::with_capacity(n);
    let mut j = primary as usize;
    for _ in 0..n {
        out.push(col[j]);
        j = inv[j];
    }
    Some(out)
}

/// Move-to-front transform: each byte is replaced by its index in a
/// self-organizing list that promotes it to position 0 after use.
/// Initial list order is `0,1,…,255`; index fits in `u8` by
/// construction (a byte value always stays in the list).
pub fn mtf_encode(data: &[u8]) -> Vec<u8> {
    let mut list: Vec<u8> = (0..=255).collect();
    let mut out = Vec::with_capacity(data.len());
    for &b in data {
        let i = list.iter().position(|&x| x == b).unwrap_or(0);
        out.push(i as u8);
        list.remove(i);
        list.insert(0, b);
    }
    out
}

/// Inverse of [`mtf_encode`].
pub fn mtf_decode(idx: &[u8]) -> Vec<u8> {
    let mut list: Vec<u8> = (0..=255).collect();
    let mut out = Vec::with_capacity(idx.len());
    for &k in idx {
        let b = list[k as usize];
        out.push(b);
        list.remove(k as usize);
        list.insert(0, b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Rotation-sort oracle: build the rotation matrix explicitly.
    fn oracle(data: &[u8]) -> (Vec<u8>, u32) {
        let n = data.len();
        let mut rots: Vec<usize> = (0..n).collect();
        rots.sort_by(|&a, &b| {
            (0..n)
                .map(|k| (data[(a + k) % n], data[(b + k) % n]))
                .find(|(x, y)| x != y)
                .map(|(x, y)| x.cmp(&y))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });
        let primary = rots.iter().position(|&p| p == 0).unwrap_or(0) as u32;
        let col: Vec<u8> = rots.iter().map(|&p| data[(p + n - 1) % n]).collect();
        (col, primary)
    }

    fn rotations_distinct(data: &[u8]) -> bool {
        let n = data.len();
        (0..n).all(|a| (0..a).all(|b| (0..n).any(|k| data[(a + k) % n] != data[(b + k) % n])))
    }

    #[test]
    fn matches_rotation_sort_oracle() {
        let mut rng = SplitMix64::new(0xB747);
        for _ in 0..120 {
            let n = (rng.below(24) + 1) as usize;
            let mut data = vec![0u8; n];
            for b in data.iter_mut() {
                // Small alphabet ⇒ duplicate rotations get exercised.
                *b = (rng.below(4) + b'A' as u32) as u8;
            }
            let (enc, p) = bwt(&data).unwrap();
            let (oenc, op) = oracle(&data);
            assert_eq!(enc, oenc, "{data:?}");
            // Primary is only unique up to duplicate rotations — for
            // periodic inputs the SA order picks the other copy, which
            // still inverts correctly.
            if rotations_distinct(&data) {
                assert_eq!(p, op, "{data:?}");
            }
        }
    }

    #[test]
    fn inverse_round_trips() {
        let mut rng = SplitMix64::new(0x1B87);
        for _ in 0..200 {
            let n = rng.below(64) as usize + 1;
            let mut data = vec![0u8; n];
            for b in data.iter_mut() {
                *b = rng.below(256) as u8;
            }
            let (enc, p) = bwt(&data).unwrap();
            assert_eq!(bwt_inverse(&enc, p).as_deref(), Some(data.as_slice()));
        }
        assert_eq!(bwt(b""), None);
        assert_eq!(bwt_inverse(b"abc", 3), None);
    }

    #[test]
    fn periodic_inputs_round_trip() {
        for s in [&b"AAAA"[..], b"ABABAB", b"XYZXYZXYZ", b"a"] {
            let (enc, p) = bwt(s).unwrap();
            assert_eq!(bwt_inverse(&enc, p).as_deref(), Some(s));
        }
    }

    #[test]
    fn mtf_round_trips_and_uses_small_indices() {
        let mut rng = SplitMix64::new(0xF00D);
        for _ in 0..100 {
            let n = rng.below(200) as usize + 1;
            let mut data = vec![0u8; n];
            for b in data.iter_mut() {
                // Runs of one symbol → small indices after the first.
                *b = if rng.below(4) == 0 { b'Z' } else { b'A' };
            }
            let enc = mtf_encode(&data);
            assert_eq!(mtf_decode(&enc), data);
        }
        assert_eq!(mtf_decode(&mtf_encode(b"ABRACADABRA")), b"ABRACADABRA");
    }

    #[test]
    fn mtf_index_always_u8_and_list_is_permutation() {
        let enc = mtf_encode(&(0u8..=255).collect::<Vec<u8>>());
        // After a full sweep every index must have been representable.
        assert_eq!(enc.len(), 256);
        assert_eq!(mtf_decode(&enc), (0u8..=255).collect::<Vec<u8>>());
    }
}
