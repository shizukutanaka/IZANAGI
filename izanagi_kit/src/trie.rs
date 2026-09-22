//! Byte trie — prefix dictionary with deterministic ordered iteration.
//!
//! The companion to [`crate::diff::levenshtein`]: `keys_with_prefix`
//! gathers the candidates, edit distance ranks them, and you have a
//! `did-you-mean` engine. Also the canonical sparse dictionary for
//! identifier tables, glyph indexes, and command registries.
//!
//! Children are `BTreeMap`-keyed, so `keys`/`keys_with_prefix` emit in
//! byte-lexicographic order — a pure function of the inserted set, never
//! of insertion order.
//!
//! ```
//! use izanagi_kit::trie::Trie;
//! let mut t = Trie::new();
//! t.insert(b"go".as_slice());
//! t.insert(b"goto".as_slice());
//! t.insert(b"gold".as_slice());
//! assert!(t.contains(b"goto"));
//! assert!(!t.contains(b"got"));
//! assert_eq!(
//!     t.keys_with_prefix(b"go"),
//!     vec![b"go".to_vec(), b"gold".to_vec(), b"goto".to_vec()]
//! );
//! ```

use std::collections::BTreeMap;

#[derive(Default)]
struct Node {
    /// Transitions keyed by byte — sorted, so all traversals are ordered.
    next: BTreeMap<u8, usize>,
    /// Whether a key ends exactly here.
    term: bool,
}

/// Root index.
const ROOT: usize = 0;

/// A trie over `&[u8]` keys.
pub struct Trie {
    nodes: Vec<Node>,
    size: usize,
}

impl Trie {
    /// Empty trie.
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::default()],
            size: 0,
        }
    }

    /// Build from an iterator of keys. Duplicates collapse.
    pub fn from_keys<'a>(keys: impl IntoIterator<Item = &'a [u8]>) -> Self {
        let mut t = Self::new();
        for k in keys {
            t.insert(k);
        }
        t
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.size
    }

    /// `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Insert `key`. Returns `true` if it was new.
    pub fn insert(&mut self, key: &[u8]) -> bool {
        let mut v = ROOT;
        for &c in key {
            v = match self.nodes[v].next.get(&c) {
                Some(&u) => u,
                None => {
                    let u = self.nodes.len();
                    self.nodes.push(Node::default());
                    self.nodes[v].next.insert(c, u);
                    u
                }
            };
        }
        let fresh = !self.nodes[v].term;
        self.nodes[v].term = true;
        self.size += usize::from(fresh);
        fresh
    }

    /// Remove `key`. Returns `true` if it was present. The trie is never
    /// shrunk — node count is a diagnostic, not the key count.
    pub fn remove(&mut self, key: &[u8]) -> bool {
        let mut v = ROOT;
        for &c in key {
            match self.nodes[v].next.get(&c) {
                Some(&u) => v = u,
                None => return false,
            }
        }
        if self.nodes[v].term {
            self.nodes[v].term = false;
            self.size -= 1;
            true
        } else {
            false
        }
    }

    /// Whether `key` was inserted.
    pub fn contains(&self, key: &[u8]) -> bool {
        self.node_of(key).is_some_and(|v| self.nodes[v].term)
    }

    /// Whether any key starts with `prefix` (including a key equal to it).
    pub fn starts_with(&self, prefix: &[u8]) -> bool {
        self.node_of(prefix).is_some()
    }

    /// Node index for `key`'s final byte, if the path exists.
    fn node_of(&self, key: &[u8]) -> Option<usize> {
        let mut v = ROOT;
        for &c in key {
            v = *self.nodes[v].next.get(&c)?;
        }
        Some(v)
    }

    /// All keys in byte-lexicographic order.
    pub fn keys(&self) -> Vec<Vec<u8>> {
        let mut out = Vec::with_capacity(self.size);
        self.walk(ROOT, &mut Vec::new(), &mut out);
        out
    }

    /// All keys starting with `prefix`, byte-lexicographic. Empty vec when
    /// `prefix` matches nothing; `prefix = b""` lists everything.
    pub fn keys_with_prefix(&self, prefix: &[u8]) -> Vec<Vec<u8>> {
        let Some(v) = self.node_of(prefix) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        self.walk(v, &mut prefix.to_vec(), &mut out);
        out
    }

    /// DFS over a subtree emitting terminal keys.
    fn walk(&self, v: usize, buf: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
        if self.nodes[v].term {
            out.push(buf.clone());
        }
        for (&c, &u) in &self.nodes[v].next {
            buf.push(c);
            self.walk(u, buf, out);
            buf.pop();
        }
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn ordered_set_semantics() {
        let mut rng = SplitMix64::new(0x7A1E);
        for _ in 0..200 {
            // Random alphabet soup key set, plus a reference BTreeSet.
            let mut t = Trie::new();
            let mut oracle: std::collections::BTreeSet<Vec<u8>> = Default::default();
            for _ in 0..(1 + rng.below(20)) {
                let k: Vec<u8> = (0..rng.below(8)).map(|_| rng.below(4) as u8).collect();
                assert_eq!(t.insert(&k), oracle.insert(k.clone()));
            }
            assert_eq!(t.len(), oracle.len());
            assert_eq!(t.keys(), oracle.iter().cloned().collect::<Vec<_>>());
            // Queries: contains for members and a few nonmembers.
            for k in oracle.iter().take(5) {
                assert!(t.contains(k));
            }
            let miss: Vec<u8> = vec![9, 9, 9];
            assert!(!t.contains(&miss));
            // Prefix listings vs oracle.
            let pref: Vec<u8> = (0..rng.below(3)).map(|_| rng.below(4) as u8).collect();
            let want: Vec<Vec<u8>> = oracle
                .iter()
                .filter(|k| k.starts_with(&pref))
                .cloned()
                .collect();
            assert_eq!(t.keys_with_prefix(&pref), want);
            // Removals keep parity.
            let victim: Vec<u8> = (0..rng.below(5)).map(|_| rng.below(4) as u8).collect();
            assert_eq!(t.remove(&victim), oracle.remove(&victim));
        }
    }

    #[test]
    fn empty_key_and_prefix_edge_cases() {
        let mut t = Trie::new();
        t.insert(b"");
        t.insert(b"a");
        assert!(t.contains(b""));
        assert_eq!(t.len(), 2);
        assert_eq!(t.keys_with_prefix(b""), vec![b"".to_vec(), b"a".to_vec()]);
        assert!(t.remove(b""));
        assert!(!t.contains(b""));
        assert!(!t.remove(b""));
        assert!(t.starts_with(b"a"));
        assert!(!t.starts_with(b"b"));
    }

    #[test]
    fn trie_is_deterministic_regardless_of_insert_order() {
        let a = Trie::from_keys([b"ab".as_slice(), b"ba", b"aa"]);
        let b = Trie::from_keys([b"ba".as_slice(), b"aa", b"ab"]);
        assert_eq!(a.keys(), b.keys());
        assert_eq!(a.keys_with_prefix(b"a"), b.keys_with_prefix(b"a"));
    }
}
