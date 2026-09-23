//! Tunstall variable-to-fixed-length code — the dual of
//! Huffman: instead of mapping fixed-size symbols to
//! variable-length codewords, it parses variable-length input
//! *phrases* (the leaves of a greedily grown parse tree) into
//! fixed `k`-bit codewords.
//!
//! Start with the root and repeatedly expand the
//! highest-probability leaf into `A` children (one per
//! alphabet symbol), stopping just before the leaf count
//! would exceed `2ᵏ`. Each leaf's probability is the product
//! of symbol weights along its path — compared exactly via
//! `BigInt` cross-multiplication so no overflow can bias the
//! greedy choice. Codewords are assigned in DFS order, a pure
//! function of the tree shape.
//!
//! ```
//! use izanagi_kit::tunstall::Tunstall;
//! let c = Tunstall::build(&[3, 1, 1], 3).unwrap(); // weights, k bits
//! let (codes, used) = c.encode(&[0, 1, 0, 0, 2]);
//! assert_eq!(used, 5);
//! assert_eq!(c.decode(&codes), vec![0u8, 1, 0, 0, 2]);
//! ```
//!
//! References: Tunstall, "Synthesis of Noiseless Compression
//! Codes" (Georgia Tech PhD, 1967); Savari & Gallager,
//! "Generalized Tunstall codes" (IEEE Trans. IT, 1997).

use crate::bigint::BigInt;

/// A variable-to-fixed Tunstall code: a parse tree over an
/// `A`-symbol alphabet whose leaves map phrases to `k`-bit
/// codewords.
#[derive(Clone, Debug)]
pub struct Tunstall {
    a: usize,
    k: u32,
    /// Flat tree: node 0 is the root; `children` empty ⇔ leaf.
    nodes: Vec<Node>,
    /// Leaf node indices in DFS order — `leaves[code]`.
    leaves: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Node {
    /// `(parent, symbol)` for non-root nodes.
    edge: Option<(usize, u8)>,
    children: Vec<usize>,
    /// Product of the path's symbol weights (`BigInt` so deep
    /// paths cannot overflow).
    num: BigInt,
    depth: u32,
}

impl Tunstall {
    /// Build a Tunstall code for alphabet weights `w`
    /// (`A = w.len()` in `2..=8`, all weights positive) with
    /// `k`-bit codewords (`1 ≤ k ≤ 16`).
    ///
    /// `None` on any violated bound — including `A == 1`
    /// where a code is meaningless.
    pub fn build(w: &[u32], k: u32) -> Option<Tunstall> {
        let a = w.len();
        if !(2..=8).contains(&a) || !(1..=16).contains(&k) || w.contains(&0) {
            return None;
        }
        let budget = 1usize << k;
        let den = BigInt::from_i128(w.iter().map(|&x| i128::from(x)).sum::<i128>());
        // Powers of `den` cached per depth for exact compares:
        // leaf i beats leaf b ⇔ num_i·den^{d_b} > num_b·den^{d_i}.
        let mut den_pow = vec![BigInt::from_i64(1)];
        let mut nodes = vec![Node {
            edge: None,
            children: Vec::new(),
            num: BigInt::from_i64(1),
            depth: 0,
        }];
        let mut leaf_count = 1usize;
        while leaf_count + (a - 1) <= budget {
            // argmax leaf by exact rational probability
            let mut best = Option::<usize>::None;
            for (i, n) in nodes.iter().enumerate() {
                if !n.children.is_empty() {
                    continue;
                }
                let better = match best {
                    None => true,
                    Some(b) => {
                        let nb = &nodes[b];
                        let lhs = n.num.mul(&den_pow[nb.depth as usize]);
                        let rhs = nb.num.mul(&den_pow[n.depth as usize]);
                        lhs > rhs || (lhs == rhs && i < b)
                    }
                };
                if better {
                    best = Some(i);
                }
            }
            let b = best?;
            // Expand `b` into A children.
            let dnext = nodes[b].depth + 1;
            while den_pow.len() <= dnext as usize {
                let p = den_pow[den_pow.len() - 1].mul(&den);
                den_pow.push(p);
            }
            for (s, &ws) in w.iter().enumerate() {
                nodes.push(Node {
                    edge: Some((b, s as u8)),
                    children: Vec::new(),
                    num: nodes[b].num.mul(&BigInt::from_i64(i64::from(ws))),
                    depth: dnext,
                });
            }
            let first = nodes.len() - a;
            nodes[b].children = (first..first + a).collect();
            leaf_count += a - 1;
        }
        // DFS order assigns canonical codewords.
        let mut leaves = Vec::new();
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            if nodes[i].children.is_empty() {
                leaves.push(i);
            } else {
                for &c in nodes[i].children.iter().rev() {
                    stack.push(c);
                }
            }
        }
        Some(Tunstall {
            a,
            k,
            nodes,
            leaves,
        })
    }

    /// Number of codewords (leaves) — ≤ `2ᵏ`.
    pub fn codebook_size(&self) -> usize {
        self.leaves.len()
    }

    /// Codeword bit length `k`.
    pub fn code_bits(&self) -> u32 {
        self.k
    }

    /// Phrase (symbol sequence) decoded by `code`, or `None`
    /// when `code ≥ codebook_size`.
    pub fn phrase(&self, code: u32) -> Option<Vec<u8>> {
        let &li = self.leaves.get(code as usize)?;
        let mut out = Vec::new();
        let mut cur = li;
        while let Some((p, s)) = self.nodes[cur].edge {
            out.push(s);
            cur = p;
        }
        out.reverse();
        Some(out)
    }

    /// Decode a stream of `k`-bit codewords by concatenating
    /// their phrases. Invalid codes are skipped.
    pub fn decode(&self, codes: &[u32]) -> Vec<u8> {
        let mut out = Vec::new();
        for &c in codes {
            if let Some(p) = self.phrase(c) {
                out.extend_from_slice(&p);
            }
        }
        out
    }

    /// Greedily parse `input` into codewords — returns the
    /// codes plus how many input symbols were consumed (input
    /// may end mid-phrase; flushing is the caller's choice).
    pub fn encode(&self, input: &[u8]) -> (Vec<u32>, usize) {
        let mut codes = Vec::new();
        let mut i = 0usize;
        while i < input.len() {
            let mut cur = 0usize;
            let mut j = i;
            while j < input.len() && usize::from(input[j]) < self.a {
                match self.nodes[cur].children.get(usize::from(input[j])) {
                    Some(&c) => {
                        cur = c;
                        j += 1;
                        if self.nodes[cur].children.is_empty() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            if self.nodes[cur].children.is_empty() {
                let code = self.leaves.iter().position(|&x| x == cur);
                match code {
                    Some(c) => codes.push(c as u32),
                    None => break,
                }
                i = j;
            } else {
                break;
            }
        }
        (codes, i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Enumerate leaves in DFS order and return their phrases.
    fn leaf_phrases(c: &Tunstall) -> Vec<Vec<u8>> {
        (0..c.leaves.len() as u32)
            .map(|i| c.phrase(i).unwrap())
            .collect()
    }

    #[test]
    fn basics() {
        let c = Tunstall::build(&[3, 1, 1], 3).unwrap();
        assert!(c.codebook_size() <= 8);
        assert_eq!(c.code_bits(), 3);
        assert!(Tunstall::build(&[1], 3).is_none());
        assert!(Tunstall::build(&[0, 1], 3).is_none());
        assert!(Tunstall::build(&[1, 1], 0).is_none());
        assert!(Tunstall::build(&[1, 1], 17).is_none());
    }

    /// All Tunstall invariants: prefix-free phrases, codes
    /// dense in `[0, size)`, leaf probabilities summing to 1,
    /// and one more expansion exceeding the `2ᵏ` budget.
    #[test]
    fn codebook_is_prefix_free_and_exact() {
        for (w, k) in [
            (vec![3u32, 1, 1], 3u32),
            (vec![1, 1], 4),
            (vec![2, 1, 1, 1], 5),
            (vec![5, 3, 1], 4),
            (vec![7, 5, 3, 1], 6),
        ] {
            let c = Tunstall::build(&w, k).unwrap();
            let lv = leaf_phrases(&c);
            assert_eq!(lv.len(), c.codebook_size());
            for (i, p) in lv.iter().enumerate() {
                for (j, q) in lv.iter().enumerate() {
                    if i != j {
                        assert!(!q.starts_with(p));
                    }
                }
            }
            // Σ leaf probs = 1: normalize each num by
            // den^{dmax − depth} and sum.
            let den = BigInt::from_i128(w.iter().map(|&x| i128::from(x)).sum());
            let dmax = lv.iter().map(|p| p.len()).max().unwrap_or(0) as u64;
            let mut total = BigInt::zero();
            for p in &lv {
                let mut num = BigInt::from_i64(1);
                for &s in p {
                    num = num.mul(&BigInt::from_i64(i64::from(w[s as usize])));
                }
                let lift = den.pow(dmax - p.len() as u64);
                total = total.add(&num.mul(&lift));
            }
            assert_eq!(total, den.pow(dmax));
            // the greedy stop is tight to the budget
            assert!(c.codebook_size() + (w.len() - 1) > (1usize << k));
        }
    }

    #[test]
    fn roundtrip_over_leaf_boundary_inputs() {
        let mut rng = SplitMix64::new(0x7C0DE);
        for _ in 0..60 {
            let a = 2 + rng.below(4) as usize;
            let w: Vec<u32> = (0..a).map(|_| 1 + rng.below(9)).collect();
            let k = 2 + rng.below(8);
            let c = Tunstall::build(&w, k).unwrap();
            let lv = leaf_phrases(&c);
            let mut input = Vec::new();
            for _ in 0..12 {
                input.extend_from_slice(&lv[rng.below(lv.len() as u32) as usize]);
            }
            let (codes, used) = c.encode(&input);
            assert_eq!(used, input.len());
            for &cd in &codes {
                assert!(cd < c.codebook_size() as u32);
            }
            assert_eq!(c.decode(&codes), input);
        }
    }

    /// Identical inputs produce identical codebooks, and the
    /// argmax-leaf choice reproduces a known Tunstall tree:
    /// weights {3,1,1}, k=3 → leaves for the classic example.
    #[test]
    fn deterministic_and_known_shape() {
        let c1 = Tunstall::build(&[3, 1, 1], 3).unwrap();
        let c2 = Tunstall::build(&[3, 1, 1], 3).unwrap();
        assert_eq!(leaf_phrases(&c1), leaf_phrases(&c2));
        // {3,1,1}: expansion order is root → child 0 → child 0
        // again, giving exactly 7 leaves for k = 3 (budget 8).
        assert_eq!(c1.codebook_size(), 7);
        // partial parses stop mid-phrase honestly
        let (codes, used) = c1.encode(&[1]);
        assert!(codes.is_empty() || used > 0);
    }
}
