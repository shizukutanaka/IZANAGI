//! Reed–Solomon erasure coding over [`crate::gf2`] — GF(2⁸).
//!
//! Splits a payload into `data` equal-length shards and derives
//! `parity` recovery shards such that **any** `data`-sized subset of
//! the `data + parity` shards rebuilds the original. This is the
//! Backblaze/JavaReedSolomon construction: start from a Vandermonde
//! matrix `V(r, c) = r^c` over GF(2⁸), then multiply by the inverse
//! of its top `data × data` square — the product keeps "any `data`
//! rows invertible" while making the top rows the identity, so data
//! shards pass through unchanged.
//!
//! Shard contents are `u8` vectors; every shard must have the same
//! length. Recovery needs only the shard index — each row of the
//! coding matrix is selected by position, so the subset choice is a
//! pure function of which shards survived.
//!
//! ```
//! use izanagi_kit::rsfec::ReedSolomon;
//! let rs = ReedSolomon::new(3, 2).unwrap();
//! let mut shards = vec![vec![1u8, 2, 3], vec![4, 5, 6], vec![7, 8, 9], vec![0; 3], vec![0; 3]];
//! assert!(rs.encode(&mut shards));
//! // Lose two shards — any two.
//! let mut have = vec![Some(shards[0].clone()), None, Some(shards[2].clone()), Some(shards[3].clone()), None];
//! assert!(rs.reconstruct(&mut have));
//! assert_eq!(have[1].as_deref(), Some(&[4u8, 5, 6][..]));
//! assert_eq!(have[4].as_deref(), Some(shards[4].as_slice()));
//! ```

use crate::gf2::{add, inv, mul, pow};

/// Dense `u8` matrix over GF(2⁸), row-major.
#[derive(Clone, Debug)]
struct Matrix {
    rows: usize,
    cols: usize,
    m: Vec<u8>,
}

impl Matrix {
    fn new(rows: usize, cols: usize) -> Self {
        Matrix {
            rows,
            cols,
            m: vec![0u8; rows * cols],
        }
    }

    /// Vandermonde: `v[r][c] = r^c` in GF(2⁸) (0⁰ = 1).
    fn vandermonde(rows: usize, cols: usize) -> Self {
        let mut out = Self::new(rows, cols);
        for r in 0..rows {
            for c in 0..cols {
                out.m[r * cols + c] = pow(r as u8, c as u32);
            }
        }
        out
    }

    fn at(&self, r: usize, c: usize) -> u8 {
        self.m[r * self.cols + c]
    }

    fn times(&self, rhs: &Matrix) -> Option<Matrix> {
        if self.cols != rhs.rows {
            return None;
        }
        let mut out = Matrix::new(self.rows, rhs.cols);
        for r in 0..self.rows {
            for k in 0..self.cols {
                let a = self.at(r, k);
                if a == 0 {
                    continue;
                }
                for c in 0..rhs.cols {
                    let idx = r * rhs.cols + c;
                    out.m[idx] = add(out.m[idx], mul(a, rhs.at(k, c)));
                }
            }
        }
        Some(out)
    }

    fn submatrix(&self, r0: usize, c0: usize, r1: usize, c1: usize) -> Matrix {
        let mut out = Matrix::new(r1 - r0, c1 - c0);
        for r in r0..r1 {
            for c in c0..c1 {
                out.m[(r - r0) * (c1 - c0) + (c - c0)] = self.at(r, c);
            }
        }
        out
    }

    /// Gauss–Jordan inverse over GF(2⁸): augment with identity,
    /// eliminate each column, swap in a nonzero pivot row. `None`
    /// when the matrix is singular (caller picks another row subset).
    fn invert(&self) -> Option<Matrix> {
        if self.rows != self.cols {
            return None;
        }
        let n = self.rows;
        let mut w = vec![0u8; n * 2 * n];
        for r in 0..n {
            for c in 0..n {
                w[r * 2 * n + c] = self.at(r, c);
            }
            w[r * 2 * n + n + r] = 1;
        }
        for col in 0..n {
            // Pivot: first row ≥ col with nonzero entry in `col`.
            let mut piv = col;
            while piv < n && w[piv * 2 * n + col] == 0 {
                piv += 1;
            }
            if piv == n {
                return None; // singular
            }
            if piv != col {
                for c in 0..2 * n {
                    w.swap(piv * 2 * n + c, col * 2 * n + c);
                }
            }
            // Scale pivot row so its `col` entry is 1.
            let scale = inv(w[col * 2 * n + col]);
            for c in 0..2 * n {
                w[col * 2 * n + c] = mul(w[col * 2 * n + c], scale);
            }
            // Eliminate `col` from every other row.
            for r in 0..n {
                if r == col {
                    continue;
                }
                let f = w[r * 2 * n + col];
                if f == 0 {
                    continue;
                }
                for c in 0..2 * n {
                    let idx = r * 2 * n + c;
                    w[idx] = add(w[idx], mul(f, w[col * 2 * n + c]));
                }
            }
        }
        let mut out = Matrix::new(n, n);
        for r in 0..n {
            for c in 0..n {
                out.m[r * n + c] = w[r * 2 * n + n + c];
            }
        }
        Some(out)
    }
}

/// Reed–Solomon encoder/decoder: `data` payload shards + `parity`
/// recovery shards, total ≤ 255.
#[derive(Clone, Debug)]
pub struct ReedSolomon {
    data: usize,
    parity: usize,
    /// Full `total × data` coding matrix: top `data` rows are the
    /// identity (data passes through), bottom `parity` rows generate.
    matrix: Matrix,
}

impl ReedSolomon {
    /// `data` payload shards, `parity` recovery shards. Both must be
    /// nonzero and `data + parity ≤ 255` (the Vandermonde rows are
    /// indexed by a `u8` row base; 256 shards is the field bound but
    /// one spare row keeps the API honest).
    pub fn new(data: usize, parity: usize) -> Option<Self> {
        let total = data.checked_add(parity)?;
        if data == 0 || parity == 0 || total > 255 {
            return None;
        }
        // M = V(total × data) · V(top data square)⁻¹.
        let v = Matrix::vandermonde(total, data);
        let top = v.submatrix(0, 0, data, data);
        let matrix = v.times(&top.invert()?)?;
        Some(ReedSolomon {
            data,
            parity,
            matrix,
        })
    }

    /// Number of payload shards.
    pub fn data_shard_count(&self) -> usize {
        self.data
    }

    /// Number of recovery shards.
    pub fn parity_shard_count(&self) -> usize {
        self.parity
    }

    /// Encode in place: `shards.len() == data + parity`, the first
    /// `data` entries hold the payload, the last `parity` are filled.
    /// All shards must share one length; returns `false` otherwise.
    pub fn encode(&self, shards: &mut [Vec<u8>]) -> bool {
        let total = self.data + self.parity;
        if shards.len() != total {
            return false;
        }
        let len = match shards.first() {
            Some(s) => s.len(),
            None => return false,
        };
        if shards.iter().any(|s| s.len() != len) {
            return false;
        }
        let (data, parity) = shards.split_at_mut(self.data);
        for (i, out) in parity.iter_mut().enumerate() {
            let row = self.data + i;
            for x in 0..len {
                let mut acc = 0u8;
                for (j, d) in data.iter().enumerate() {
                    let coef = self.matrix.at(row, j);
                    if coef != 0 {
                        acc = add(acc, mul(coef, d[x]));
                    }
                }
                out[x] = acc;
            }
        }
        true
    }

    /// Encode to fresh `Vec`s — pure counterpart of [`encode`]: the
    /// input `data` shards plus the computed parity shards.
    ///
    /// [`encode`]: Self::encode
    pub fn encode_shards(&self, data: &[Vec<u8>]) -> Option<Vec<Vec<u8>>> {
        if data.len() != self.data || data.is_empty() {
            return None;
        }
        let len = data[0].len();
        if data.iter().any(|s| s.len() != len) {
            return None;
        }
        let mut all: Vec<Vec<u8>> = data.to_vec();
        all.resize_with(self.data + self.parity, || vec![0u8; len]);
        if self.encode(&mut all) {
            Some(all)
        } else {
            None
        }
    }

    /// Rebuild missing shards in place. `shards.len() == data + parity`,
    /// each entry `Some` (present) or `None` (lost). Needs ≥ `data`
    /// present shards with a common length; on success every `None`
    /// is filled. Failure (`false`) leaves `shards` untouched.
    pub fn reconstruct(&self, shards: &mut [Option<Vec<u8>>]) -> bool {
        let total = self.data + self.parity;
        if shards.len() != total {
            return false;
        }
        let present: Vec<usize> = (0..total).filter(|&i| shards[i].is_some()).collect();
        if present.len() < self.data {
            return false;
        }
        let len = match present.first().and_then(|&i| shards[i].as_ref()) {
            Some(s) => s.len(),
            None => return false,
        };
        if present
            .iter()
            .any(|&i| shards[i].as_ref().map_or(true, |s| s.len() != len))
        {
            return false;
        }
        // Decode matrix: invert the `data`-row subset of M for the
        // first `data` present indices.
        let keep = &present[..self.data];
        let mut sub = Matrix::new(self.data, self.data);
        for (r, &src_row) in keep.iter().enumerate() {
            for c in 0..self.data {
                sub.m[r * self.data + c] = self.matrix.at(src_row, c);
            }
        }
        let inv_m = match sub.invert() {
            Some(m) => m,
            None => return false,
        };
        // present_data[j] = decode[j] · kept shards — recovers the
        // original data shard vector, then re-encode the holes.
        let mut recovered: Vec<Vec<u8>> = vec![vec![0u8; len]; self.data];
        for (j, buf) in recovered.iter_mut().enumerate() {
            for x in 0..len {
                let mut acc = 0u8;
                for (r, &k) in keep.iter().enumerate() {
                    let coef = inv_m.at(j, r);
                    if coef != 0 {
                        let v = match &shards[k] {
                            Some(s) => s[x],
                            None => 0,
                        };
                        acc = add(acc, mul(coef, v));
                    }
                }
                buf[x] = acc;
            }
        }
        for (i, buf) in recovered.iter().enumerate() {
            if shards[i].is_none() {
                shards[i] = Some(buf.clone());
            }
        }
        // Re-derive any missing parity shards from the data row.
        let data: Vec<Vec<u8>> = (0..self.data)
            .map(|i| shards[i].clone().unwrap_or_default())
            .collect();
        for (i, shard) in shards.iter_mut().enumerate().skip(self.data) {
            if shard.is_none() {
                let mut buf = vec![0u8; len];
                for (x, b) in buf.iter_mut().enumerate() {
                    let mut acc = 0u8;
                    for (j, d) in data.iter().enumerate() {
                        let coef = self.matrix.at(i, j);
                        if coef != 0 {
                            acc = add(acc, mul(coef, d[x]));
                        }
                    }
                    *b = acc;
                }
                *shard = Some(buf);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn make(data: usize, parity: usize, len: usize, rng: &mut SplitMix64) -> Vec<Vec<u8>> {
        (0..data + parity)
            .map(|i| {
                if i < data {
                    (0..len).map(|_| rng.below(256) as u8).collect()
                } else {
                    vec![0u8; len]
                }
            })
            .collect()
    }

    #[test]
    fn encode_reconstruct_roundtrip() {
        let mut rng = SplitMix64::new(0xFE5A);
        for _ in 0..60 {
            let data = (rng.below(8) + 1) as usize;
            let parity = (rng.below(4) + 1) as usize;
            let len = (rng.below(24) + 1) as usize;
            let rs = ReedSolomon::new(data, parity).unwrap();
            let original = make(data, parity, len, &mut rng);
            let mut shards = original.clone();
            assert!(rs.encode(&mut shards));
            // Parity is nonzero somewhere for a nonempty payload.
            // Lose exactly `parity` shards — the worst case we survive.
            let total = data + parity;
            let mut lost: Vec<usize> = (0..total).collect();
            // deterministic shuffle of the loss set
            for i in (1..total).rev() {
                let j = rng.below(i as u32 + 1) as usize;
                lost.swap(i, j);
            }
            let mut have: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
            for &i in lost.iter().take(parity) {
                have[i] = None;
            }
            assert!(rs.reconstruct(&mut have), "data={data} parity={parity}");
            for i in 0..total {
                assert_eq!(have[i].as_deref(), Some(shards[i].as_slice()), "shard {i}");
            }
        }
    }

    #[test]
    fn every_subset_recovers() {
        // Exhaustive: with data=4, parity=4, try every 4-of-8 subset.
        let rs = ReedSolomon::new(4, 4).unwrap();
        let mut shards = vec![
            vec![0xAAu8, 0x11, 0x22],
            vec![0xBB, 0x33, 0x44],
            vec![0xCC, 0x55, 0x66],
            vec![0xDD, 0x77, 0x88],
            vec![0; 3],
            vec![0; 3],
            vec![0; 3],
            vec![0; 3],
        ];
        assert!(rs.encode(&mut shards));
        for mask in 0u32..256 {
            if mask.count_ones() as usize != 4 {
                continue;
            }
            let mut have: Vec<Option<Vec<u8>>> = vec![None; 8];
            for i in 0..8 {
                if mask >> i & 1 == 1 {
                    have[i] = Some(shards[i].clone());
                }
            }
            assert!(rs.reconstruct(&mut have), "subset {:08b} failed", mask);
            for i in 0..8 {
                assert_eq!(have[i].as_deref(), Some(shards[i].as_slice()));
            }
        }
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(ReedSolomon::new(0, 3).is_none());
        assert!(ReedSolomon::new(3, 0).is_none());
        assert!(ReedSolomon::new(200, 200).is_none());
        let rs = ReedSolomon::new(3, 2).unwrap();
        let mut wrong = vec![vec![1u8, 2], vec![3, 4]]; // too few shards
        assert!(!rs.encode(&mut wrong));
        let mut ragged = vec![vec![1u8], vec![2, 3], vec![4], vec![0], vec![0]];
        assert!(!rs.encode(&mut ragged));
        let mut have = vec![Some(vec![1u8]), Some(vec![2]), None, None, None];
        assert!(!rs.reconstruct(&mut have)); // only 2 of 3 needed present
                                             // Unequal lengths rejected.
        let mut bad = vec![
            Some(vec![1u8]),
            Some(vec![2, 3]),
            Some(vec![4]),
            Some(vec![5]),
            Some(vec![6]),
        ];
        assert!(!rs.reconstruct(&mut bad));
    }

    #[test]
    fn encode_shards_and_accessors() {
        let rs = ReedSolomon::new(4, 3).unwrap();
        assert_eq!(rs.data_shard_count(), 4);
        assert_eq!(rs.parity_shard_count(), 3);
        let data: Vec<Vec<u8>> = (0..4).map(|i| vec![i as u8, 9, 7]).collect();
        let all = rs.encode_shards(&data).unwrap();
        assert_eq!(all.len(), 7);
        assert_eq!(&all[..4], &data[..]);
        // encode_shards output matches in-place encode.
        let mut inplace = data.clone();
        inplace.extend(vec![vec![0u8; 3]; 3]);
        assert!(rs.encode(&mut inplace));
        assert_eq!(inplace, all);
        // Malformed: wrong shard count or ragged lengths → None.
        assert!(rs.encode_shards(&data[..3]).is_none());
        assert!(rs
            .encode_shards(&[vec![1], vec![2, 3], vec![4], vec![5]])
            .is_none());
    }
}
