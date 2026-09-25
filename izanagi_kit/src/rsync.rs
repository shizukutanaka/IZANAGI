//! rsync-style byte-stream delta — Tridgell's two-hash protocol:
//! the sender rolls a weak 32-bit sum (rsync's a/b checksum) over a
//! fixed-size window and confirms candidates with MD5, emitting
//! `Copy(block)` ops for matches and `Literal` runs otherwise;
//! `apply` rebuilds the new stream from the basis plus ops.
//!
//! Where [`crate::delta`] diffs key→value maps, this diffs raw
//! byte strings — the classic remote-sync primitive. The weak sum
//! rolls in O(1): `a = Σxᵢ`, `b = Σ(L+1−i)·xᵢ` (both mod 2¹⁶),
//! so `a' = a − x_out + x_in`, `b' = b − L·x_out + a'`.
//!
//! ```
//! use izanagi_kit::rsync::{apply, delta, signature, Op};
//!
//! let basis = b"the quick brown fox";
//! let new = b"the quick red fox";
//! let sig = signature(basis, 4);
//! let ops = delta(new, &sig, 4);
//! assert_eq!(apply(basis, &ops, 4), new);
//! assert!(ops.iter().any(|o| matches!(o, Op::Copy { .. })));
//! ```

use crate::md5::md5;

/// One basis-block digest entry: weak rolling sum + strong MD5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sig {
    /// rsync a/b weak checksum of the block.
    pub weak: u32,
    /// MD5 of the block.
    pub strong: [u8; 16],
    /// Block length in bytes (short only for the tail block).
    pub len: u32,
}

/// Weak checksum pair `(a, b)` → packed `b<<16 | a`.
fn weak(data: &[u8]) -> u32 {
    let mut a = 0u32;
    let mut b = 0u32;
    let l = data.len() as u32;
    for (i, &x) in data.iter().enumerate() {
        a = (a + x as u32) & 0xffff;
        b = (b + (l - i as u32) * x as u32) & 0xffff;
    }
    (b << 16) | a
}

/// Signature of `basis` split into `block`-byte chunks (the last
/// chunk keeps its natural shorter length; `block` of 0 yields an
/// empty signature).
pub fn signature(basis: &[u8], block: usize) -> Vec<Sig> {
    if block == 0 {
        return Vec::new();
    }
    basis
        .chunks(block)
        .map(|c| Sig {
            weak: weak(c),
            strong: md5(c),
            len: c.len() as u32,
        })
        .collect()
}

/// One delta op: `Lit` emits raw bytes; `Copy` splices `len` bytes
/// starting at `block`·`block_size` in the basis (`len` is shorter
/// only for the basis's tail block).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Literal bytes to append.
    Lit(Vec<u8>),
    /// Copy `len` bytes of basis block `idx`.
    Copy {
        /// Basis block index.
        idx: u32,
        /// Bytes to copy (≤ block size).
        len: u32,
    },
}

/// Compute the delta from `basis`'s `sig` to `new`: a byte-wise
/// scan emitting `Copy` on confirmed matches, `Lit` otherwise.
/// `block` must equal the size used for `signature`.
pub fn delta(new: &[u8], sig: &[Sig], block: usize) -> Vec<Op> {
    let mut ops: Vec<Op> = Vec::new();
    let mut lit: Vec<u8> = Vec::new();
    if sig.is_empty() || block == 0 {
        if !new.is_empty() {
            ops.push(Op::Lit(new.to_vec()));
        }
        return ops;
    }
    // block → (weak → sig indices), last block may be shorter.
    let mut i = 0usize;
    let n = new.len();
    let mut win = Vec::with_capacity(block + 1);
    let mut a = 0u32; // rolling a
    let mut b = 0u32; // rolling b
    let mut wlen = 0usize;
    while i < n {
        // Extend window to the current match length (full block,
        // or the remaining tail which may equal the last sig len).
        let tail = n - i;
        let last_len = sig[sig.len() - 1].len as usize;
        let target = if tail >= block {
            block
        } else if tail == last_len {
            tail // allow matching the short tail block
        } else {
            0 // can't match: drain literals
        };
        if target == 0 {
            lit.extend_from_slice(&new[i..]);
            break;
        }
        // Fill/roll the window to `target` bytes.
        while wlen < target && i + wlen < n {
            let x = new[i + wlen] as u32;
            a = (a + x) & 0xffff;
            b = (b + (target - wlen) as u32 * x) & 0xffff;
            win.push(new[i + wlen]);
            wlen += 1;
        }
        // When target shrank (tail), recompute from scratch.
        if wlen > target {
            win.clear();
            win.extend_from_slice(&new[i..i + target]);
            a = 0;
            b = 0;
            for (j, &x) in win.iter().enumerate() {
                a = (a + x as u32) & 0xffff;
                b = (b + (target - j) as u32 * x as u32) & 0xffff;
            }
            wlen = target;
        }
        let w = (b << 16) | a;
        let mut matched = false;
        if wlen == target {
            for (idx, s) in sig.iter().enumerate() {
                if s.weak == w && s.len as usize == wlen && s.strong == md5(&win) {
                    if !lit.is_empty() {
                        ops.push(Op::Lit(std::mem::take(&mut lit)));
                    }
                    ops.push(Op::Copy {
                        idx: idx as u32,
                        len: wlen as u32,
                    });
                    i += wlen;
                    wlen = 0;
                    win.clear();
                    a = 0;
                    b = 0;
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            // Slide one byte: emit first, roll the rest.
            lit.push(new[i]);
            let xout = new[i] as u64;
            let a64 = (a as u64 + 0x10000 - xout) & 0xffff;
            let b64 = (b as u64 + 0x1_0000_0000 - (wlen as u64) * xout + a64) & 0xffff;
            a = a64 as u32;
            b = b64 as u32;
            i += 1;
            wlen -= 1;
            win.remove(0);
        }
    }
    if !lit.is_empty() {
        ops.push(Op::Lit(lit));
    }
    ops
}

/// Rebuild the new byte string from `basis` + `ops`.
pub fn apply(basis: &[u8], ops: &[Op], block: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(basis.len());
    for op in ops {
        match op {
            Op::Lit(bytes) => out.extend_from_slice(bytes),
            Op::Copy { idx, len } => {
                let start = *idx as usize * block;
                if start < basis.len() {
                    let end = (start + *len as usize).min(basis.len());
                    out.extend_from_slice(&basis[start..end]);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(basis: &[u8], new: &[u8], block: usize) -> Vec<Op> {
        let ops = delta(new, &signature(basis, block), block);
        assert_eq!(apply(basis, &ops, block), new, "round-trip failed");
        ops
    }

    #[test]
    fn identical_is_all_copies() {
        let basis = b"0123456789abcdef" as &[u8];
        let ops = round_trip(basis, basis, 4);
        assert!(ops.iter().all(|o| matches!(o, Op::Copy { .. })));
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn disjoint_is_one_literal() {
        let basis = b"0123456789abcdef" as &[u8];
        let ops = round_trip(basis, b"ZZZZZZZZZZZZZZZZ", 4);
        assert_eq!(ops, vec![Op::Lit(b"ZZZZZZZZZZZZZZZZ".to_vec())]);
    }

    #[test]
    fn insertion_mid_stream() {
        let basis = b"abcdefghijklmnop" as &[u8];
        let new = b"abcdXXijklmnop" as &[u8];
        let ops = round_trip(basis, new, 4);
        // First 4 bytes copy, then literal XX, then copies of 2/3.
        assert!(matches!(ops[0], Op::Copy { idx: 0, .. }));
        assert!(ops.iter().any(|o| matches!(o, Op::Lit(v) if v == b"XX")));
    }

    #[test]
    fn tail_and_edges() {
        // Basis with a short tail block.
        let basis = b"0123456789abc" as &[u8];
        round_trip(basis, basis, 4);
        round_trip(basis, b"0123456789abd", 4);
        // New shorter than a block.
        round_trip(basis, b"012", 4);
        // Empty new → no ops → empty output.
        assert!(delta(b"", &signature(basis, 4), 4).is_empty());
        // Empty basis → whole new is literal.
        assert_eq!(delta(b"xy", &[], 4), vec![Op::Lit(b"xy".to_vec())]);
        // block 0.
        assert!(signature(basis, 0).is_empty());
    }

    #[test]
    fn larger_randomized_round_trip() {
        // LCG-based deterministic pseudo-random streams.
        let mut s = 12345u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        let basis: Vec<u8> = (0..3000).map(|_| next()).collect();
        let mut new = basis.clone();
        new[100..110].copy_from_slice(b"0123456789");
        new.truncate(2500);
        new.extend_from_slice(&basis[..40]);
        round_trip(&basis, &new, 64);
        // Most of the stream is unchanged → many copies.
        let ops = delta(&new, &signature(&basis, 64), 64);
        assert!(ops.iter().filter(|o| matches!(o, Op::Copy { .. })).count() > 20);
    }

    #[test]
    fn deterministic_twice() {
        let basis = b"aaaabbbbccccdddd" as &[u8];
        let new = b"aaaabbbbXXccdddd";
        let s = signature(basis, 4);
        assert_eq!(delta(new, &s, 4), delta(new, &s, 4));
    }
}
