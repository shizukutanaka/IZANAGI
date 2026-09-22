//! Seeded Markov-chain text generator — deterministic names for NPCs,
//! places, items, and spawn lists from a caller-supplied corpus.
//!
//! Roguelike name generation is usually ad-hoc string munging. A
//! character-level Markov chain trained on a word list produces
//! pronounceable, corpus-flavored names; driven by
//! [`crate::rng::SplitMix64`] the same `(corpus, seed)` yields the same
//! name on every platform and every replay.
//!
//! ```
//! use izanagi_kit::markov::NameGen;
//! use izanagi_kit::rng::SplitMix64;
//! let corpus = ["gandalf", "saruman", "radagast", "alatar", "pallando"];
//! let mut gen = NameGen::train(&corpus, 2);
//! let mut rng = SplitMix64::new(7);
//! let name = gen.generate(&mut rng, 4, 12).unwrap();
//! assert!(name.len() >= 4 && name.len() <= 12);
//! // Deterministic: same seed → same name, forever.
//! let mut rng2 = SplitMix64::new(7);
//! assert_eq!(name, gen.generate(&mut rng2, 4, 12).unwrap());
//! ```

use crate::rng::SplitMix64;
use std::collections::BTreeMap;

/// Order-k byte-level Markov chain over a word list.
///
/// Transitions are stored as `(context) -> sorted Vec<next-byte>`;
/// sampling indexes the sorted vec by `rng` draw, so the chain is a
/// pure function of the corpus and the rng stream — insertion order
/// and platform hashing never leak in.
pub struct NameGen {
    /// (context bytes) -> (next byte, cumulative weight). Weighted by
    /// occurrence count across the corpus.
    table: BTreeMap<Vec<u8>, Vec<(u8, u32)>>,
    /// Contexts that can begin a word (length k each).
    starts: Vec<Vec<u8>>,
}

const BEGIN: u8 = 0x01; // synthetic start-of-word marker byte
const END: u8 = 0x02; // synthetic end-of-word marker byte

impl NameGen {
    /// Train on `corpus` with context length `k` (2–4 sensible).
    /// Non-ASCII bytes are kept verbatim (names stay byte-flavored to
    /// the corpus). Empty corpus yields an empty table → `generate`
    /// returns `None`.
    pub fn train(corpus: &[&str], k: usize) -> Self {
        let k = k.max(1);
        let mut counts: BTreeMap<Vec<u8>, BTreeMap<u8, u32>> = BTreeMap::new();
        let mut starts = Vec::new();
        for word in corpus {
            let w = word.as_bytes();
            if w.is_empty() {
                continue;
            }
            // Padded sequence: BEGIN…BEGIN + word + END
            let mut seq = Vec::with_capacity(w.len() + k + 1);
            seq.extend(std::iter::repeat(BEGIN).take(k));
            seq.extend_from_slice(w);
            seq.push(END);
            if seq.len() > k {
                starts.push(seq[..k].to_vec());
            }
            for i in 0..seq.len() - k {
                let ctx = seq[i..i + k].to_vec();
                let nxt = seq[i + k];
                *counts.entry(ctx).or_default().entry(nxt).or_default() += 1;
            }
        }
        // Flatten counts → cumulative weights (BTreeMap keeps the
        // next-byte order canonical).
        let table = counts
            .into_iter()
            .map(|(ctx, nexts)| {
                let mut acc = 0u32;
                let mut cum = Vec::with_capacity(nexts.len());
                for (b, c) in nexts {
                    acc += c;
                    cum.push((b, acc));
                }
                (ctx, cum)
            })
            .collect();
        starts.sort();
        starts.dedup();
        Self { table, starts }
    }

    /// Generate one name in `[min_len, max_len]` bytes. Resamples until
    /// the length lands in range (bounded attempts); returns `None`
    /// when the table is empty or `min_len > max_len`.
    ///
    /// The name is a pure function of `(corpus, k, rng-stream)`: each
    /// transition picks by `below(total_weight)` over the cumulative
    /// table in ascending-byte order.
    pub fn generate(&self, rng: &mut SplitMix64, min_len: usize, max_len: usize) -> Option<String> {
        if self.table.is_empty() || min_len > max_len || max_len == 0 {
            return None;
        }
        for _ in 0..64 {
            if let Some(name) = self.roll(rng, max_len) {
                if name.len() >= min_len {
                    return Some(name);
                }
            }
        }
        None
    }

    /// Generate `n` names (sequential rng draws).
    pub fn generate_n(
        &self,
        rng: &mut SplitMix64,
        min_len: usize,
        max_len: usize,
        n: usize,
    ) -> Vec<String> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(name) = self.generate(rng, min_len, max_len) {
                out.push(name);
            }
        }
        out
    }

    /// One unbounded roll up to `max_len` body bytes.
    fn roll(&self, rng: &mut SplitMix64, max_len: usize) -> Option<String> {
        if self.starts.is_empty() {
            return None;
        }
        let mut ctx = self.starts[rng.below(self.starts.len() as u32) as usize].clone();
        let mut out: Vec<u8> = Vec::with_capacity(max_len);
        loop {
            let (next, _) = self.pick(&ctx, rng)?;
            if next == END || out.len() >= max_len {
                break;
            }
            if next != BEGIN {
                out.push(next);
            }
            ctx.remove(0);
            ctx.push(next);
        }
        if out.is_empty() {
            return None;
        }
        // Corpus is caller-controlled bytes; lossy only for display.
        Some(String::from_utf8_lossy(&out).into_owned())
    }

    /// Pick a next byte for `ctx` via cumulative weights.
    fn pick(&self, ctx: &[u8], rng: &mut SplitMix64) -> Option<(u8, u32)> {
        let cums = self.table.get(ctx)?;
        let total = cums.last().map(|&(_, c)| c)?;
        let draw = rng.below(total);
        for &(b, cum) in cums {
            if draw < cum {
                return Some((b, cum));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &[&str] = &[
        "gandalf",
        "saruman",
        "radagast",
        "alatar",
        "pallando",
        "glorfindel",
        "arwen",
        "elrond",
        "celeborn",
        "galadriel",
    ];

    #[test]
    fn deterministic_per_seed_and_valid_outputs() {
        let gen = NameGen::train(CORPUS, 2);
        let mut r1 = SplitMix64::new(42);
        let mut r2 = SplitMix64::new(42);
        for _ in 0..50 {
            let a = gen.generate(&mut r1, 3, 10).unwrap();
            let b = gen.generate(&mut r2, 3, 10).unwrap();
            assert_eq!(a, b);
            assert!(a.len() >= 3 && a.len() <= 10);
            // Only lowercase corpus bytes appear (no markers leak).
            assert!(a.bytes().all(|c| c.is_ascii_lowercase()));
        }
    }

    #[test]
    fn outputs_respect_corpus_grams() {
        // Every order-k gram inside an output must exist in the corpus
        // (the chain can only emit transitions it saw).
        let k = 3;
        let gen = NameGen::train(CORPUS, k);
        let mut grams = std::collections::BTreeSet::new();
        for w in CORPUS {
            let wb = w.as_bytes();
            for i in 0..=wb.len().saturating_sub(k) {
                grams.insert(&wb[i..i + k]);
            }
        }
        let mut rng = SplitMix64::new(7);
        for _ in 0..200 {
            let name = gen.generate(&mut rng, k, 14).unwrap();
            let nb = name.as_bytes();
            for i in 0..=nb.len().saturating_sub(k) {
                assert!(
                    grams.contains(&nb[i..i + k]),
                    "{} has foreign gram in {}",
                    String::from_utf8_lossy(&nb[i..i + k]),
                    name
                );
            }
        }
    }

    #[test]
    fn seeds_diverge_and_degenerate_inputs() {
        let gen = NameGen::train(CORPUS, 2);
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        let names_a = gen.generate_n(&mut a, 3, 10, 20);
        let names_b = gen.generate_n(&mut b, 3, 10, 20);
        assert_ne!(names_a, names_b);
        // Empty corpus / bad ranges → None, no panic.
        let empty = NameGen::train(&[], 2);
        assert_eq!(empty.generate(&mut a, 3, 10), None);
        assert_eq!(gen.generate(&mut a, 10, 3), None);
        assert_eq!(gen.generate(&mut a, 0, 0), None);
        // Single-word corpus still works.
        let one = NameGen::train(&["ab"], 1);
        let mut r = SplitMix64::new(3);
        assert!(one.generate(&mut r, 1, 4).is_some());
    }

    #[test]
    fn weights_favor_frequent_transitions() {
        // Corpus where 'a' is followed by 'b' 9/10 of the time.
        let corpus: Vec<&str> = (0..9).map(|_| "ab").chain(std::iter::once("ac")).collect();
        let gen = NameGen::train(&corpus, 1);
        let mut rng = SplitMix64::new(99);
        let mut bcount = 0;
        for _ in 0..400 {
            let n = gen.generate(&mut rng, 2, 2).unwrap();
            if n == "ab" {
                bcount += 1;
            }
        }
        // Should be ~90%; bound loose enough to never flake (<0.1% tails).
        assert!(bcount > 300 && bcount < 400, "bcount={bcount}");
    }
}
