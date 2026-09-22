//! Canonical Huffman coding — the byte-oriented entropy codec that
//! sits between [`crate::rle`] and [`crate::bits`] in the wire
//! pipeline. Canonical assignment (codes ordered by `(length,
//! symbol)`) makes the bitstream a pure function of the corpus: the
//! decoder never sees a tree, only a `(symbol, length)` table, so two
//! correct encoders can never disagree.
//!
//! ```
//! use izanagi_kit::huffman::{encode, decode};
//! // Repetitive text over a small alphabet beats the table overhead.
//! let data = b"the quick brown fox jumps over the lazy dog. ".repeat(8);
//! let packed = encode(&data).unwrap();
//! assert!(packed.len() < data.len());
//! assert_eq!(decode(&packed).unwrap(), data);
//! ```

/// Codebook entry: `len[s]` bits, MSB-first, of `code[s]`.
#[derive(Clone)]
pub struct CodeBook {
    /// `len[s]` = bit length of symbol `s`'s code; 0 = absent.
    pub len: [u8; 256],
    /// `code[s]` = the code value, right-justified in `len[s]` bits.
    pub code: [u32; 256],
}

/// Huffman code lengths for `data`, canonical-assigned.
///
/// Merge order is fully deterministic: repeatedly join the two
/// smallest `(weight, tie_key)` items where leaves tie-break by symbol
/// index and internal nodes tie-break by creation order.
pub fn build_book(data: &[u8]) -> CodeBook {
    let mut freq = [0u64; 256];
    for &b in data {
        freq[b as usize] += 1;
    }
    let syms: Vec<u8> = (0..=255u8).filter(|&s| freq[s as usize] > 0).collect();
    let mut len = [0u8; 256];
    let mut code = [0u32; 256];
    match syms.len() {
        0 => return CodeBook { len, code },
        1 => {
            // Degenerate: one symbol gets a 1-bit code so the stream
            // is not empty.
            len[syms[0] as usize] = 1;
            code[syms[0] as usize] = 0;
            return CodeBook { len, code };
        }
        _ => {}
    }

    // Two-queue Huffman: leaves sorted by (freq, symbol); internals
    // appended in creation order — their weights are non-decreasing by
    // induction, so merging is O(n) with no heap. Node ids: leaves are
    // the symbol (0..255), internals are 256 + creation index, so at
    // equal weight a leaf wins the tie (sym < 256 ≤ internal id).
    let mut leaves: Vec<(u64, u32)> = syms.iter().map(|&s| (freq[s as usize], s as u32)).collect();
    leaves.sort();
    let mut li = 0usize;
    let mut internals: std::collections::VecDeque<(u64, u32)> = std::collections::VecDeque::new();
    let mut children: Vec<(u32, u32)> = Vec::new(); // children of internal j
    let mut remaining = leaves.len();
    while remaining + internals.len() > 1 {
        let mut ab = [0u32; 2];
        let mut wsum = 0u64;
        for slot in &mut ab {
            let from_leaf = match (leaves.get(li), internals.front()) {
                (Some(&(lw, _)), Some(&(iw, _))) => lw <= iw,
                (Some(_), None) => true,
                _ => false,
            };
            if from_leaf {
                let (w, s) = leaves[li];
                li += 1;
                remaining -= 1;
                *slot = s;
                wsum += w;
            } else if let Some((w, id)) = internals.pop_front() {
                *slot = id;
                wsum += w;
            }
        }
        children.push((ab[0], ab[1]));
        internals.push_back((wsum, 256 + children.len() as u32 - 1));
    }

    // Walk the final tree (the last-created internal node is the root)
    // assigning leaf depths.
    let mut stack = vec![(256 + children.len() - 1, 0u8)];
    while let Some((node, d)) = stack.pop() {
        if node < 256 {
            len[node] = d.max(1);
        } else {
            let j = node - 256;
            let (l, r) = children[j];
            stack.push((l as usize, d + 1));
            stack.push((r as usize, d + 1));
        }
    }

    // Canonical assignment: sort symbols by (len, sym), assign codes
    // incrementally — the textbook canonical rule.
    let mut order: Vec<u8> = syms;
    order.sort_by_key(|&s| (len[s as usize], s));
    let mut cur = 0u32;
    let mut prev_len = 0u8;
    for &s in &order {
        let l = len[s as usize];
        cur <<= l - prev_len;
        code[s as usize] = cur;
        cur += 1;
        prev_len = l;
    }
    CodeBook { len, code }
}

/// Encode `data` into a self-contained bitstream:
/// `[count u8] (sym, len)* [bit_count u32le] [packed MSB-first bytes]`.
/// Empty input → `Some(vec![])`; a `None` is impossible in practice —
/// the only failure is bit length overflow beyond `u32` capacity
/// (input would have to exceed 512 MiB).
pub fn encode(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() {
        return Some(Vec::new());
    }
    let book = build_book(data);
    let present: Vec<u8> = (0..=255u8).filter(|&s| book.len[s as usize] > 0).collect();
    let mut out = Vec::new();
    out.push((present.len() - 1) as u8); // count-1 so 256 fits
    for &s in &present {
        out.push(s);
        out.push(book.len[s as usize]);
    }
    let total_bits: u64 = data.iter().map(|&b| book.len[b as usize] as u64).sum();
    let total_bits = u32::try_from(total_bits).ok()?;
    out.extend_from_slice(&total_bits.to_le_bytes());
    // Pack MSB-first.
    let mut acc = 0u32;
    let mut nbits = 0u32;
    for &b in data {
        let len = book.len[b as usize] as u32;
        acc = (acc << len) | book.code[b as usize];
        nbits += len;
        while nbits >= 8 {
            nbits -= 8;
            out.push((acc >> nbits) as u8);
            acc &= (1 << nbits) - 1;
        }
    }
    if nbits > 0 {
        out.push((acc << (8 - nbits)) as u8);
    }
    Some(out)
}

/// Decode a stream produced by [`encode`]. `None` on any malformed
/// input (truncated table, invalid lengths, leftover garbage bits).
pub fn decode(buf: &[u8]) -> Option<Vec<u8>> {
    if buf.is_empty() {
        return Some(Vec::new());
    }
    let count = buf[0] as usize + 1;
    let table_bytes = 1 + count * 2;
    if buf.len() < table_bytes + 4 {
        return None;
    }
    let mut len = [0u8; 256];
    for i in 0..count {
        let s = buf[1 + i * 2];
        let l = buf[2 + i * 2];
        if l == 0 || l > 32 {
            return None;
        }
        len[s as usize] = l;
    }
    let bit_count = u32::from_le_bytes([
        buf[table_bytes],
        buf[table_bytes + 1],
        buf[table_bytes + 2],
        buf[table_bytes + 3],
    ]) as usize;
    let payload = &buf[table_bytes + 4..];
    if bit_count > payload.len() * 8 {
        return None;
    }
    // Rebuild canonical codes from (len, sym) — the same rule as
    // `build_book`, so table only needs lengths.
    let mut order: Vec<u8> = (0..=255u8).filter(|&s| len[s as usize] > 0).collect();
    order.sort_by_key(|&s| (len[s as usize], s));
    let mut decode_map: std::collections::BTreeMap<(u8, u32), u8> =
        std::collections::BTreeMap::new();
    let mut cur = 0u32;
    let mut prev_len = 0u8;
    for &s in &order {
        let l = len[s as usize];
        cur <<= l - prev_len;
        decode_map.insert((l, cur), s);
        cur += 1;
        prev_len = l;
    }
    // Walk bits MSB-first.
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut acc_len = 0u8;
    for i in 0..bit_count {
        let byte = payload[i / 8];
        let bit = (byte >> (7 - i % 8)) & 1;
        acc = (acc << 1) | bit as u32;
        acc_len += 1;
        if let Some(&s) = decode_map.get(&(acc_len, acc)) {
            out.push(s);
            acc = 0;
            acc_len = 0;
        }
        if acc_len > 32 {
            return None;
        }
    }
    if acc_len != 0 {
        return None; // trailing partial code — malformed
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn round_trip_on_random_and_skewed() {
        let mut rng = SplitMix64::new(0xC0DE);
        for _ in 0..400 {
            let n = rng.below(600) as usize;
            // Mixed alphabets: skewed (Zipf-ish) and uniform.
            let skew = rng.below(4) == 0;
            let alpha = if skew { 3 + rng.below(4) } else { 256 };
            let data: Vec<u8> = (0..n).map(|_| rng.below(alpha) as u8).collect();
            let packed = encode(&data).unwrap();
            assert_eq!(decode(&packed), Some(data));
        }
        // Empty.
        assert_eq!(encode(&[]), Some(vec![]));
        assert_eq!(decode(&[]), Some(vec![]));
        // Single symbol only.
        assert_eq!(
            decode(&encode(b"aaaaaa").unwrap()),
            Some(b"aaaaaa".to_vec())
        );
    }

    #[test]
    fn canonical_codes_are_prefix_free_and_minimal() {
        let mut rng = SplitMix64::new(0xBEEF);
        for _ in 0..300 {
            let n = 1 + rng.below(400) as usize;
            let data: Vec<u8> = (0..n).map(|_| rng.below(32) as u8).collect();
            let book = build_book(&data);
            let present: Vec<u8> = (0..=255u8).filter(|&s| book.len[s as usize] > 0).collect();
            if present.len() < 2 {
                continue;
            }
            // Prefix-free: no code is a prefix of another.
            for &a in &present {
                for &b in &present {
                    if a == b {
                        continue;
                    }
                    let (la, lb) = (book.len[a as usize], book.len[b as usize]);
                    let (ca, cb) = (book.code[a as usize], book.code[b as usize]);
                    if la <= lb {
                        let shifted = cb >> (lb - la);
                        assert_ne!(ca, shifted, "{a} prefix of {b}");
                    }
                }
            }
            // Kraft inequality holds with equality (complete tree).
            let kraft: f64 = present
                .iter()
                .map(|&s| 2f64.powi(-(book.len[s as usize] as i32)))
                .sum();
            assert!((kraft - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn compression_beats_fixed_width_on_skewed_data() {
        // Highly skewed data must compress below the 8-bit width.
        let data: Vec<u8> = (0..10_000)
            .map(|i| if i % 10 == 0 { 1 } else { 0 })
            .collect();
        let packed = encode(&data).unwrap();
        assert!(
            packed.len() < data.len() / 4,
            "{} vs {}",
            packed.len(),
            data.len()
        );
        assert_eq!(decode(&packed).unwrap(), data);
        // Malformed: truncation and impossible length.
        assert_eq!(decode(&packed[..packed.len() - 1]), None);
        assert_eq!(decode(&[0, b'a', 33]), None);
        assert_eq!(decode(&[0, b'a', 1, 0, 0, 0, 9, 0x80]), None); // 9 bits > 8 available
    }

    #[test]
    fn encode_is_a_pure_function_of_input() {
        let data = b"deterministic huffman should not depend on a hash seed";
        assert_eq!(encode(data), encode(data));
    }
}
