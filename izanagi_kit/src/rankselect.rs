//! Rank/select on a static bitvector — the Jacobson two-level
//! directory (`L0` absolute rank per 512-bit superblock, `L1`
//! intra-superblock rank per word) so `rank1` is two lookups and a
//! popcount, `select1` a bounded scan. Bit `i` lives in
//! `words[i/64] >> (i%64)` — the same little-endian convention as
//! [`crate::bits`].
//!
//! ```
//! use izanagi_kit::rankselect::RankSelect;
//!
//! let bv = RankSelect::from_words(&[0b10110], 5);
//! assert_eq!(bv.rank1(3), 2); // bits 1 and 2
//! assert_eq!(bv.select1(2), Some(4));
//! assert_eq!(bv.select0(0), Some(0));
//! ```
//!
//! References: Jacobson (1989) rank directories; González–Grabowski–
//! Mäkinen–Navarro (2005) two-level select; Gog & Petri VByte/SDSL.

/// Words per superblock: `512` bits between `L0` entries.
const WORDS_PER_SB: usize = 8;

/// Static bitvector with `O(1)`-ish `rank` and bounded-scan
/// `select`. Build once, query many — mutation is not offered
/// because rebuilding counters is the whole point.
pub struct RankSelect {
    words: Vec<u64>,
    /// Number of valid bits (last word may be partially used).
    n: usize,
    /// `l0[i]` = rank before superblock `i`.
    l0: Vec<u64>,
    /// `l1[j]` = rank within its superblock before word `j`
    /// (flat, same length as `words`).
    l1: Vec<u64>,
    /// Total number of 1-bits.
    ones: u64,
}

impl RankSelect {
    /// Index the first `n` bits of `words` (trailing bits in the
    /// last word are masked out of every answer).
    pub fn from_words(words: &[u64], n: usize) -> Self {
        let mut w: Vec<u64> = words.to_vec();
        if n < w.len() * 64 {
            let keep = w.len() - n / 64;
            w.truncate(n / 64 + usize::from(n % 64 != 0));
            let _ = keep;
            if n % 64 != 0 {
                let last = w.len() - 1;
                w[last] &= (1u64 << (n % 64)) - 1;
            }
        }
        let n_sb = w.len() / WORDS_PER_SB + usize::from(w.len() % WORDS_PER_SB != 0);
        let mut l0 = vec![0u64; n_sb + 1];
        let mut l1 = vec![0u64; w.len()];
        let mut run = 0u64;
        for (i, &word) in w.iter().enumerate() {
            if i % WORDS_PER_SB == 0 {
                l0[i / WORDS_PER_SB] = run;
            }
            l1[i] = run - l0[i / WORDS_PER_SB];
            run += word.count_ones() as u64;
        }
        l0[n_sb] = run;
        Self {
            words: w,
            n,
            l0,
            l1,
            ones: run,
        }
    }

    /// Number of valid bits.
    pub fn len(&self) -> usize {
        self.n
    }

    /// Whether the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Total number of 1-bits.
    pub fn count_ones(&self) -> u64 {
        self.ones
    }

    /// Number of 1-bits in `[0, i)` — `i` may equal `len()`.
    pub fn rank1(&self, i: usize) -> u64 {
        if i > self.n {
            return self.ones;
        }
        if i == 0 {
            return 0;
        }
        let wi = (i - 1) / 64;
        let sb = wi / WORDS_PER_SB;
        let base = self.l0[sb] + self.l1[wi];
        let mask = if i % 64 == 0 {
            !0u64
        } else {
            (1u64 << (i % 64)) - 1
        };
        base + (self.words[wi] & mask).count_ones() as u64
    }

    /// Number of 0-bits in `[0, i)`.
    pub fn rank0(&self, i: usize) -> u64 {
        i as u64 - self.rank1(i)
    }

    /// Position of the `k`-th 1-bit (0-indexed); `None` when
    /// `k >= count_ones()`.
    pub fn select1(&self, k: u64) -> Option<usize> {
        if k >= self.ones {
            return None;
        }
        // First superblock whose cumulative rank passes k.
        let sb = self.l0.partition_point(|&r| r <= k) - 1;
        let start = sb * WORDS_PER_SB;
        let end = ((sb + 1) * WORDS_PER_SB).min(self.words.len());
        let mut rem = k - self.l0[sb];
        for w in start..end {
            let c = self.words[w].count_ones() as u64;
            if rem < c {
                return Some(w * 64 + Self::select_in_word(self.words[w], rem)?);
            }
            rem -= c;
        }
        None
    }

    /// Position of the `k`-th 0-bit (0-indexed).
    pub fn select0(&self, k: u64) -> Option<usize> {
        if k >= self.n as u64 - self.ones {
            return None;
        }
        // Brute scan over words — select0 is rare enough to keep it
        // simple and obviously correct.
        let mut rem = k;
        for (i, &w) in self.words.iter().enumerate() {
            let valid = if i + 1 == self.words.len() && self.n % 64 != 0 {
                (1u64 << (self.n % 64)) - 1
            } else {
                !0
            };
            let z = (!w & valid).count_ones() as u64;
            if rem < z {
                return Some(i * 64 + Self::select_in_word(!w & valid, rem)?);
            }
            rem -= z;
        }
        None
    }

    /// Index of the `k`-th set bit inside `w` (`k < popcnt(w)`).
    fn select_in_word(w: u64, k: u64) -> Option<usize> {
        let mut rem = k;
        for b in 0..8 {
            let byte = (w >> (b * 8)) & 0xff;
            let c = byte.count_ones() as u64;
            if rem < c {
                for bit in 0..8 {
                    if byte >> bit & 1 == 1 {
                        if rem == 0 {
                            return Some(b * 8 + bit as usize);
                        }
                        rem -= 1;
                    }
                }
            }
            rem -= c;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn naive_rank(w: &[u64], n: usize, i: usize) -> u64 {
        let mut c = 0;
        for b in 0..i.min(n) {
            if w[b / 64] >> (b % 64) & 1 == 1 {
                c += 1;
            }
        }
        c
    }

    #[test]
    fn rank_select_oracle() {
        let mut r = SplitMix64::new(0xFEED_FACE);
        for _ in 0..100 {
            let n = (r.below(600) + 1) as usize;
            let mut w: Vec<u64> = (0..n.div_ceil(64)).map(|_| r.next_u64()).collect();
            if n % 64 != 0 {
                let last = w.len() - 1;
                w[last] &= (1u64 << (n % 64)) - 1;
            }
            let bv = RankSelect::from_words(&w, n);
            assert_eq!(bv.len(), n);
            for i in 0..=n {
                assert_eq!(bv.rank1(i), naive_rank(&w, n, i), "rank1({i}) n={n}");
                assert_eq!(bv.rank0(i), i as u64 - naive_rank(&w, n, i));
            }
            let ones: Vec<usize> = (0..n).filter(|&b| w[b / 64] >> (b % 64) & 1 == 1).collect();
            assert_eq!(bv.count_ones() as usize, ones.len());
            for (k, &pos) in ones.iter().enumerate() {
                assert_eq!(bv.select1(k as u64), Some(pos), "select1({k})");
            }
            assert_eq!(bv.select1(ones.len() as u64), None);
            let zeros: Vec<usize> = (0..n).filter(|&b| w[b / 64] >> (b % 64) & 1 == 0).collect();
            for (k, &pos) in zeros.iter().enumerate() {
                assert_eq!(bv.select0(k as u64), Some(pos), "select0({k})");
            }
            assert_eq!(bv.select0(zeros.len() as u64), None);
        }
    }

    #[test]
    fn edge_cases() {
        let bv = RankSelect::from_words(&[], 0);
        assert!(bv.is_empty());
        assert_eq!(bv.rank1(0), 0);
        assert_eq!(bv.select1(0), None);
        let all = RankSelect::from_words(&[!0, !0], 128);
        assert_eq!(all.count_ones(), 128);
        assert_eq!(all.select0(0), None);
        assert_eq!(all.select1(127), Some(127));
        let sparse = RankSelect::from_words(&[1], 64);
        assert_eq!(sparse.select1(0), Some(0));
        assert_eq!(sparse.select0(62), Some(63)); // 63 zeros total
        assert_eq!(sparse.select0(63), None);
    }
}
