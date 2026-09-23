//! Rope — a balanced binary tree of string chunks for heavyweight
//! string edits (Boehm, Atwater & Plass 1995). Where
//! [`crate::piecetable::PieceTable`] keeps an explicit piece list
//! (`O(#pieces)` position lookup, original store never copied), a
//! rope's position lookup is `O(depth)` descent and `insert`/`remove`
//! touch only the leaves on the split path — the right structure
//! when edits and seeks interleave on large buffers.
//!
//! Structure: leaves carry `Vec<u8>` chunks; internal nodes carry
//! `weight` = byte length of the left subtree. `concat` joins two
//! ropes under a fresh internal node; when the resulting depth
//! violates the balance bound the whole rope is flattened and
//! rebuilt into a near-perfect binary tree — `O(n)` worst case but
//! amortized rare, and the rebuilt shape is a pure function of the
//! leaf sequence, so identical edit histories yield identical
//! trees on every peer.
//!
//! ```
//! use izanagi_kit::rope::Rope;
//!
//! let mut r = Rope::from_text("Hello world!");
//! r.insert_str(5, ", deterministic");
//! r.remove(0, 7);
//! assert_eq!(r.to_string(), "deterministic world!");
//! assert_eq!(r.byte(0), Some(b'd'));
//! ```
//!
//! References: Boehm, Atwater & Plass, "Ropes: an alternative to
//! strings" (1995); the `ropey` and `xi-rope` implementations.

/// One tree node: a byte chunk or an internal join.
enum Node {
    /// Byte chunk (always non-empty).
    Leaf(Vec<u8>),
    /// `weight` = byte length of the left subtree.
    Internal {
        /// Bytes in the left subtree.
        weight: usize,
        /// Bytes in the whole subtree.
        size: usize,
        /// Left subtree.
        left: Box<Node>,
        /// Right subtree.
        right: Box<Node>,
    },
}

impl Node {
    fn size(&self) -> usize {
        match self {
            Node::Leaf(b) => b.len(),
            Node::Internal { size, .. } => *size,
        }
    }

    fn depth(&self) -> usize {
        match self {
            Node::Leaf(_) => 1,
            Node::Internal { left, right, .. } => 1 + left.depth().max(right.depth()),
        }
    }

    fn join(a: Box<Node>, b: Box<Node>) -> Box<Node> {
        let weight = a.size();
        let size = weight + b.size();
        Box::new(Node::Internal {
            weight,
            size,
            left: a,
            right: b,
        })
    }

    fn leaf(b: Vec<u8>) -> Box<Node> {
        Box::new(Node::Leaf(b))
    }
}

/// Balanced rope text buffer over bytes.
#[derive(Default)]
pub struct Rope {
    root: Option<Box<Node>>,
}

impl Rope {
    /// Empty rope.
    pub fn new() -> Self {
        Rope::default()
    }

    /// Rope holding `s`.
    pub fn from_text(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Rope holding `bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Rope {
            root: if bytes.is_empty() {
                None
            } else {
                Some(Node::leaf(bytes.to_vec()))
            },
        }
    }

    /// Total byte length.
    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |r| r.size())
    }

    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Tree depth (leaves = 1); `O(log n)` under the balance rule.
    pub fn depth(&self) -> usize {
        self.root.as_ref().map_or(0, |r| r.depth())
    }

    /// Byte at `pos`, `None` out of bounds.
    pub fn byte(&self, pos: usize) -> Option<u8> {
        let mut cur = self.root.as_deref()?;
        let mut p = pos;
        loop {
            match cur {
                Node::Leaf(b) => return b.get(p).copied(),
                Node::Internal {
                    weight,
                    left,
                    right,
                    ..
                } => {
                    if p < *weight {
                        cur = left;
                    } else {
                        p -= *weight;
                        cur = right;
                    }
                }
            }
        }
    }

    /// `self = self ++ other` (consumes `other`).
    pub fn concat(&mut self, other: Rope) {
        match (self.root.take(), other.root) {
            (None, r) => self.root = r,
            (a, None) => self.root = a,
            (Some(a), Some(b)) => {
                self.root = Some(Node::join(a, b));
                self.rebalance();
            }
        }
    }

    /// Insert `bytes` at `pos` (`pos > len` clamps to the end).
    pub fn insert(&mut self, pos: usize, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let len = self.len();
        let (left, right) = std::mem::take(self).split(pos.min(len));
        let mut new = left;
        new.concat(Rope::from_bytes(bytes));
        new.concat(right);
        *self = new;
    }

    /// Insert `s` at `pos` (clamped).
    pub fn insert_str(&mut self, pos: usize, s: &str) {
        self.insert(pos, s.as_bytes());
    }

    /// Delete `n` bytes starting at `pos` (out-of-range tail clamps).
    pub fn remove(&mut self, pos: usize, n: usize) {
        let len = self.len();
        if n == 0 || pos >= len {
            return;
        }
        let (left, rest) = std::mem::take(self).split(pos);
        let (_, right) = rest.split((pos + n).min(len) - pos);
        let mut new = left;
        new.concat(right);
        *self = new;
    }

    /// `(self[..pos], self[pos..])` — consumes the rope.
    pub fn split(self, pos: usize) -> (Rope, Rope) {
        match self.root {
            None => (Rope::new(), Rope::new()),
            Some(r) => {
                let (l, rr) = split_node(*r, pos);
                (Rope { root: l }, Rope { root: rr })
            }
        }
    }

    /// Bytes `[pos, pos+n)` clamped to bounds.
    pub fn slice(&self, pos: usize, n: usize) -> Vec<u8> {
        let len = self.len();
        if pos >= len || n == 0 {
            return Vec::new();
        }
        let end = (pos + n).min(len);
        let mut out = Vec::with_capacity(end - pos);
        if let Some(r) = self.root.as_deref() {
            slice_into(r, pos, end, &mut out);
        }
        out
    }

    /// Whole content — `O(n)` in-order collect.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.slice(0, self.len())
    }

    /// Flatten + rebuild into a near-perfect tree when depth exceeds
    /// `2·⌈log2 n⌉ + 1` — a sufficient bound for the classic
    /// Fibonacci balancing rule.
    fn rebalance(&mut self) {
        let Some(r) = self.root.take() else { return };
        let n = r.size();
        let mut bound = 0usize;
        while (1usize << bound.min(62)) <= n.max(1) {
            bound += 1;
        }
        if r.depth() <= 2 * bound + 1 {
            self.root = Some(r);
            return;
        }
        let mut leaves: Vec<Vec<u8>> = Vec::new();
        collect_leaves(*r, &mut leaves);
        self.root = build_balanced(&leaves);
    }
}

fn collect_leaves(n: Node, out: &mut Vec<Vec<u8>>) {
    match n {
        Node::Leaf(b) => out.push(b),
        Node::Internal { left, right, .. } => {
            collect_leaves(*left, out);
            collect_leaves(*right, out);
        }
    }
}

fn build_balanced(leaves: &[Vec<u8>]) -> Option<Box<Node>> {
    fn build(l: &[Vec<u8>]) -> Box<Node> {
        if l.len() == 1 {
            Node::leaf(l[0].clone())
        } else {
            let m = l.len() / 2;
            Node::join(build(&l[..m]), build(&l[m..]))
        }
    }
    if leaves.is_empty() {
        None
    } else {
        Some(build(leaves))
    }
}

impl core::fmt::Display for Rope {
    /// Whole content as a `String` (lossy for non-UTF8 bytes).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.to_bytes()))
    }
}

/// Split a node at `pos` into `(left, right)` parts.
fn split_node(n: Node, pos: usize) -> (Option<Box<Node>>, Option<Box<Node>>) {
    match n {
        Node::Leaf(b) => {
            let (a, c) = b.split_at(pos.min(b.len()));
            let (a, c) = (a.to_vec(), c.to_vec());
            (
                (!a.is_empty()).then(|| Node::leaf(a)),
                (!c.is_empty()).then(|| Node::leaf(c)),
            )
        }
        Node::Internal {
            weight,
            left,
            right,
            ..
        } => {
            if pos < weight {
                let (ll, lr) = split_node(*left, pos);
                (ll, join_opt(lr, Some(right)))
            } else {
                let (rl, rr) = split_node(*right, pos - weight);
                (join_opt(Some(left), rl), rr)
            }
        }
    }
}

fn join_opt(a: Option<Box<Node>>, b: Option<Box<Node>>) -> Option<Box<Node>> {
    match (a, b) {
        (None, x) | (x, None) => x,
        (Some(a), Some(b)) => Some(Node::join(a, b)),
    }
}

fn slice_into(n: &Node, lo: usize, hi: usize, out: &mut Vec<u8>) {
    match n {
        Node::Leaf(b) => {
            let end = hi.min(b.len());
            if lo < end {
                out.extend_from_slice(&b[lo..end]);
            }
        }
        Node::Internal {
            weight,
            left,
            right,
            ..
        } => {
            let w = *weight;
            if lo < w {
                slice_into(left, lo, hi.min(w), out);
            }
            if hi > w {
                slice_into(right, lo.saturating_sub(w), hi - w, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_reads_back() {
        let mut r = Rope::from_text("abcdefgh");
        assert_eq!(r.len(), 8);
        assert_eq!(r.byte(0), Some(b'a'));
        assert_eq!(r.byte(7), Some(b'h'));
        assert_eq!(r.byte(8), None);
        r.insert(4, b"XY");
        assert_eq!(r.to_string(), "abcdXYefgh");
        let mut r2 = Rope::from_text("ab");
        r2.insert_str(1, "cd");
        assert_eq!(r2.to_string(), "acdb");
        r.remove(1, 3);
        assert_eq!(r.to_string(), "aXYefgh");
        assert_eq!(r.slice(2, 4), b"Yefg".to_vec());
    }

    #[test]
    fn split_concat_roundtrip() {
        let r = Rope::from_text("hello world");
        let (l, right) = r.split(5);
        assert_eq!(l.to_string(), "hello");
        assert_eq!(right.to_string(), " world");
        let mut l = l;
        l.concat(right);
        assert_eq!(l.to_string(), "hello world");
    }

    #[test]
    fn edge_cases() {
        let mut r = Rope::new();
        assert!(r.is_empty());
        r.insert(0, b"ab");
        r.insert(99, b"cd"); // clamped to end
        assert_eq!(r.to_string(), "abcd");
        r.remove(1, 99); // clamped tail
        assert_eq!(r.to_string(), "a");
        r.remove(0, 1);
        assert!(r.is_empty());
        assert_eq!(r.slice(0, 5), Vec::<u8>::new());
    }

    #[test]
    fn matches_vec_oracle() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0xB0E9);
        let mut rope = Rope::new();
        let mut model: Vec<u8> = Vec::new();
        for _ in 0..500 {
            match rng.next_u64() % 4 {
                0 | 1 => {
                    let pos = if model.is_empty() {
                        0
                    } else {
                        (rng.next_u64() as usize) % (model.len() + 1)
                    };
                    let k = 1 + (rng.next_u64() % 6) as usize;
                    let bytes: Vec<u8> =
                        (0..k).map(|_| b'a' + (rng.next_u64() % 26) as u8).collect();
                    model.splice(pos..pos, bytes.iter().copied());
                    rope.insert(pos, &bytes);
                }
                2 => {
                    if model.is_empty() {
                        continue;
                    }
                    let pos = (rng.next_u64() as usize) % model.len();
                    let n = (1 + rng.next_u64() % 5).min((model.len() - pos) as u64) as usize;
                    model.drain(pos..pos + n);
                    rope.remove(pos, n);
                }
                _ => {
                    if model.is_empty() {
                        continue;
                    }
                    let pos = (rng.next_u64() as usize) % model.len();
                    let n = (rng.next_u64() % 8) as usize;
                    let want: Vec<u8> = model[pos..(pos + n).min(model.len())].to_vec();
                    assert_eq!(rope.slice(pos, n), want);
                }
            }
            assert_eq!(rope.len(), model.len());
        }
        assert_eq!(rope.to_bytes(), model);
        for (i, &b) in model.iter().enumerate() {
            assert_eq!(rope.byte(i), Some(b));
        }
        // Depth stays logarithmic after the churn.
        let log2 = usize::BITS as usize - model.len().max(1).leading_zeros() as usize;
        assert!(rope.depth() <= 4 * log2 + 8);
    }

    #[test]
    fn rebalance_recovers_from_degenerate_edits() {
        // Many point inserts at position 0 grow a degenerate left
        // spine; the bound forces a rebuild.
        let mut r = Rope::from_text("x");
        for _ in 0..2000 {
            r.insert(0, b"y");
        }
        assert_eq!(r.len(), 2001);
        assert!(r.depth() <= 32);
        let s = r.to_bytes();
        assert_eq!(&s[..2000], &[b'y'; 2000]);
        assert_eq!(s[2000], b'x');
    }
}
