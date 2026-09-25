//! Okapi BM25 — Robertson & Zaragoza's probabilistic ranking
//! function, the workhorse of classical IR (Lucene's default for a
//! decade): `score(q, d) = Σ_t idf(t) · tf·(k1+1) / (tf + k1·(1−b +
//! b·dl/avgdl))` with `idf(t) = ln((N − df + ½)/(df + ½) + 1)`.
//!
//! Integer substrate version: `idf` is evaluated through
//! `Fixed::ln` on the exact rational `(2N+2)/(2df+1)` (the halves
//! cancel), saturating length normalisation uses `Fixed`, and ties
//! break by document index so rankings replay bit-exactly.
//! Terms are byte strings — tokenisation upstream, scoring here.
//!
//! ```
//! use izanagi_kit::bm25::Bm25;
//!
//! let corpus = Bm25::new(&[
//!     &[&b"red"[..], &b"fox"[..]],
//!     &[&b"red"[..], &b"dog"[..], &b"dog"[..]],
//! ]);
//! let ranked = corpus.top_k(&[&b"dog"[..]], 2);
//! assert_eq!(ranked[0].0, 1); // "dog" only lives in doc 1
//! ```

use crate::fixed::Fixed;
use std::collections::BTreeMap;

/// A scored corpus: document term vectors plus the collection
/// statistics BM25 needs.
pub struct Bm25 {
    /// Per-document `(term → tf)` maps in collection order.
    docs: Vec<BTreeMap<Vec<u8>, u32>>,
    /// Document lengths (term counts).
    lens: Vec<u64>,
    /// `df` per term.
    df: BTreeMap<Vec<u8>, u32>,
    /// Σ lens / n, kept as `(sum, n)` for exact ratios.
    avg_num: u64,
    avg_den: u64,
    k1: Fixed,
    b: Fixed,
}

impl Bm25 {
    /// Classic parameters `k1 = 1.2`, `b = 0.75`.
    pub fn new(docs: &[&[&[u8]]]) -> Self {
        Self::with_params(docs, Fixed::from_ratio(6, 5), Fixed::from_ratio(3, 4))
    }

    /// Build with explicit `k1` (term saturation) and `b` (length
    /// normalisation strength).
    pub fn with_params(docs: &[&[&[u8]]], k1: Fixed, b: Fixed) -> Self {
        let mut maps = Vec::with_capacity(docs.len());
        let mut lens = Vec::with_capacity(docs.len());
        let mut df: BTreeMap<Vec<u8>, u32> = BTreeMap::new();
        for doc in docs {
            let mut m: BTreeMap<Vec<u8>, u32> = BTreeMap::new();
            for term in *doc {
                *m.entry(term.to_vec()).or_insert(0) += 1;
            }
            for term in m.keys() {
                *df.entry(term.clone()).or_insert(0) += 1;
            }
            lens.push(doc.len() as u64);
            maps.push(m);
        }
        let sum: u64 = lens.iter().sum();
        Bm25 {
            docs: maps,
            lens,
            df,
            avg_num: sum,
            avg_den: docs.len().max(1) as u64,
            k1,
            b,
        }
    }

    /// `idf(term)` in `Fixed`; absent terms get `ln((2N+2)/1)`.
    pub fn idf(&self, term: &[u8]) -> Fixed {
        let n = self.docs.len() as i64;
        let d = *self.df.get(term).unwrap_or(&0) as i64;
        // (N − df + ½)/(df + ½) + 1 = (2N + 2)/(2df + 1)
        let num = (2 * n + 2).min(i32::MAX as i64) as i32;
        let den = (2 * d + 1).min(i32::MAX as i64) as i32;
        Fixed::from_ratio(num, den).ln()
    }

    /// BM25 score of one document against a bag-of-words query.
    /// `idx` out of range scores zero.
    pub fn score(&self, query: &[&[u8]], idx: usize) -> Fixed {
        if idx >= self.docs.len() {
            return Fixed::ZERO;
        }
        let dl = self.lens[idx].min(i32::MAX as u64) as i32;
        let an = self.avg_num.min(i32::MAX as u64) as i32;
        let ad = self.avg_den.min(i32::MAX as u64) as i32;
        // dl/avgdl as Fixed (avgdl may be 0 → treat as 1: no norm).
        let len_ratio = if self.avg_num == 0 {
            Fixed::ONE
        } else {
            Fixed::from_ratio(
                (dl as i64 * ad as i64 / an as i64).min(i32::MAX as i64) as i32,
                ad,
            )
        };
        let norm = Fixed::ONE - self.b + self.b.mul(len_ratio);
        let mut total = Fixed::ZERO;
        for term in query {
            let tf = *self.docs[idx].get(*term).unwrap_or(&0);
            if tf == 0 {
                continue;
            }
            let tf_f = Fixed::from_int(tf.min(i32::MAX as u32) as i32);
            let denom = tf_f + self.k1.mul(norm);
            if denom <= Fixed::ZERO {
                continue;
            }
            let contrib = self
                .idf(term)
                .mul(tf_f.mul(self.k1 + Fixed::ONE).div(denom));
            total = total + contrib;
        }
        total
    }

    /// Top-`k` documents by score, descending; ties by index.
    /// Documents scoring `≤ 0` are omitted (a negative `idf` adds
    /// no information). Empty corpus returns an empty vector.
    pub fn top_k(&self, query: &[&[u8]], k: usize) -> Vec<(usize, Fixed)> {
        let mut scored: Vec<(usize, Fixed)> = (0..self.docs.len())
            .map(|i| (i, self.score(query, i)))
            .filter(|&(_, s)| s > Fixed::ZERO)
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        scored.truncate(k);
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Bm25 {
        Bm25::new(&[
            &[&b"the"[..], &b"quick"[..], &b"brown"[..], &b"fox"[..]],
            &[&b"the"[..], &b"lazy"[..], &b"dog"[..]],
            &[&b"fox"[..], &b"fox"[..], &b"fox"[..]],
        ])
    }

    #[test]
    fn rare_terms_rank_over_common() {
        let c = corpus();
        // "fox" is in docs 0 and 2; doc 2 repeats it 3×.
        let r = c.top_k(&[&b"fox"[..]], 3);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, 2);
        // "lazy" appears only in doc 1 and gets the highest idf.
        assert_eq!(c.top_k(&[&b"lazy"[..]], 1)[0].0, 1);
    }

    #[test]
    fn document_frequency_scaling() {
        // Doc with tf=3 must outscore tf=1 on the same term,
        // with diminishing returns (BM25 saturation).
        let c = corpus();
        let s3 = c.score(&[&b"fox"[..]], 2);
        let s1 = c.score(&[&b"fox"[..]], 0);
        assert!(s3 > s1);
        // Saturation: 3× the tf ≠ 3× the score.
        let triple = s1.mul(Fixed::from_int(3));
        assert!(s3 < triple);
    }

    #[test]
    fn length_normalisation() {
        // Two docs, same tf for "x": the shorter one wins.
        let c = Bm25::new(&[
            &[
                &b"x"[..],
                &b"filler"[..],
                &b"filler"[..],
                &b"filler"[..],
                &b"filler"[..],
            ],
            &[&b"x"[..]],
        ]);
        let r = c.top_k(&[&b"x"[..]], 2);
        assert_eq!(r[0].0, 1);
        // b = 0 disables length normalisation → tie, index order.
        let flat = Bm25::with_params(
            &[
                &[&b"x"[..], &b"f"[..], &b"f"[..], &b"f"[..], &b"f"[..]],
                &[&b"x"[..]],
            ],
            Fixed::from_ratio(6, 5),
            Fixed::ZERO,
        );
        let r2 = flat.top_k(&[&b"x"[..]], 2);
        assert_eq!((r2[0].1, r2[0].0), (r2[1].1, 0));
    }

    #[test]
    fn idf_distinguishes_rare_from_common() {
        let c = corpus();
        assert!(c.idf(b"lazy") > c.idf(b"fox"));
        assert!(c.idf(b"absent") > c.idf(b"lazy"));
    }

    #[test]
    fn ties_and_edges() {
        let c = corpus();
        assert!(c.score(&[&b"absent"[..]], 0) == Fixed::ZERO);
        assert!(c.top_k(&[&b"absent"[..]], 3).is_empty());
        assert!(c.score(&[&b"fox"[..]], 99) == Fixed::ZERO);
        // Empty corpus.
        let e = Bm25::new(&[]);
        assert!(e.top_k(&[&b"x"[..]], 5).is_empty());
        // k larger than hits.
        assert_eq!(corpus().top_k(&[&b"fox"[..]], 99).len(), 2);
    }

    #[test]
    fn deterministic_twice() {
        let c = corpus();
        assert_eq!(
            c.top_k(&[&b"fox"[..], &b"dog"[..]], 3),
            c.top_k(&[&b"fox"[..], &b"dog"[..]], 3)
        );
    }
}
