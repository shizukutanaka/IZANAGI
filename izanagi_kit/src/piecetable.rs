//! Piece table — the text-buffer structure behind most editors
//! (Crowley 1998). Two backing stores and a piece list:
//!
//! - `original`: the initial content, never mutated — cheap undo and
//!   zero-copy snapshots.
//! - `added`: an append-only store every `insert` writes to.
//! - `pieces`: a sequence of `(source, start, len)` spans over the
//!   two stores; the document is their concatenation.
//!
//! `insert` appends to `added` and splices one new piece in (splitting
//! the piece it lands inside); `delete` trims the range's endpoints
//! and drops covered pieces. Both cost `O(#pieces)` for the position
//! lookup — the flat-`Vec` edit would shift the whole document, and
//! a rope would need a balanced tree. For sim-scale documents
//! (logs, scripts, serialized state) the linear scan is the right
//! trade: no allocation beyond the added bytes and the piece list.
//!
//! Edits are a pure function of the operation sequence — same ops,
//! same piece list, same bytes — so buffers hash identically across
//! peers when fed a replicated op stream.
//!
//! ```
//! use izanagi_kit::piecetable::PieceTable;
//! let mut pt = PieceTable::new(b"hello world");
//! pt.insert(5, b" brave new");
//! pt.delete(0, 6);
//! assert_eq!(&*pt.to_bytes(), b"brave new world");
//! ```

/// Which backing store a piece reads from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Src {
    /// The initial document (immutable).
    Original,
    /// The append-only store every insert writes to.
    Added,
}

/// A span `(source, start, len)` over one backing store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Piece {
    src: Src,
    start: usize,
    len: usize,
}

/// A piece-table text buffer — `insert`/`delete` edit in place;
/// `to_bytes` materializes the document.
#[derive(Clone, Debug)]
pub struct PieceTable {
    original: Vec<u8>,
    added: Vec<u8>,
    pieces: Vec<Piece>,
}

impl PieceTable {
    /// A document initially equal to `content` — one `Original` piece.
    pub fn new(content: &[u8]) -> Self {
        let pieces = if content.is_empty() {
            Vec::new()
        } else {
            vec![Piece {
                src: Src::Original,
                start: 0,
                len: content.len(),
            }]
        };
        PieceTable {
            original: content.to_vec(),
            added: Vec::new(),
            pieces,
        }
    }

    /// Document length in bytes — sum of piece lengths.
    pub fn len(&self) -> usize {
        self.pieces.iter().map(|p| p.len).sum()
    }

    /// Whether the document is empty.
    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// Locate the piece covering logical offset `pos` —
    /// `(piece_index, offset_inside_piece)`; `pos == len()` maps to
    /// one past the last piece (insert-at-end position). `None` only
    /// when `pos > len`.
    fn locate(&self, pos: usize) -> Option<(usize, usize)> {
        let mut acc = 0usize;
        for (i, p) in self.pieces.iter().enumerate() {
            if pos < acc + p.len {
                return Some((i, pos - acc));
            }
            acc += p.len;
        }
        (pos == acc).then_some((self.pieces.len(), 0))
    }

    /// Insert `text` at logical offset `pos` — `false` when
    /// `pos > len()` (fail closed, buffer untouched). Empty `text`
    /// is a no-op `true`.
    pub fn insert(&mut self, pos: usize, text: &[u8]) -> bool {
        if text.is_empty() {
            return pos <= self.len(); // no-op, but only inside the document
        }
        let Some((i, off)) = self.locate(pos) else {
            return false;
        };
        let start = self.added.len();
        self.added.extend_from_slice(text);
        let np = Piece {
            src: Src::Added,
            start,
            len: text.len(),
        };
        if i == self.pieces.len() {
            self.pieces.push(np);
            return true;
        }
        if off == 0 {
            self.pieces.insert(i, np);
            return true;
        }
        // Split the piece at `off`: [left][np][right].
        let p = self.pieces[i];
        self.pieces[i] = Piece {
            src: p.src,
            start: p.start,
            len: off,
        };
        self.pieces.insert(i + 1, np);
        self.pieces.insert(
            i + 2,
            Piece {
                src: p.src,
                start: p.start + off,
                len: p.len - off,
            },
        );
        true
    }

    /// Delete `count` bytes starting at `pos` — `false` when
    /// `pos + count > len()` (fail closed, buffer untouched). Clamps
    /// are NOT applied: an out-of-range delete is a caller bug.
    pub fn delete(&mut self, pos: usize, count: usize) -> bool {
        if count == 0 {
            return pos <= self.len();
        }
        let len = self.len();
        if pos >= len || pos + count > len {
            return false;
        }
        // Split at pos and pos+count, then drop the middle span.
        let end = pos + count;
        let Some((si, soff)) = self.locate(pos) else {
            return false;
        };
        let Some((ei, eoff)) = self.locate(end) else {
            return false;
        };
        // Truncate the first touched piece at the delete start.
        let mut out: Vec<Piece> = self.pieces[..si].to_vec();
        if soff > 0 {
            let p = self.pieces[si];
            out.push(Piece {
                src: p.src,
                start: p.start,
                len: soff,
            });
        }
        // Keep the tail of the last touched piece after the delete end.
        if ei < self.pieces.len() {
            let p = self.pieces[ei];
            if eoff < p.len {
                out.push(Piece {
                    src: p.src,
                    start: p.start + eoff,
                    len: p.len - eoff,
                });
            }
            out.extend_from_slice(&self.pieces[ei + 1..]);
        }
        self.pieces = out;
        // Coalesce adjacent pieces from the same store (cheap hygiene).
        let mut merged: Vec<Piece> = Vec::with_capacity(self.pieces.len());
        for &p in &self.pieces {
            if let Some(last) = merged.last_mut() {
                if last.src == p.src && last.start + last.len == p.start {
                    last.len += p.len;
                    continue;
                }
            }
            merged.push(p);
        }
        self.pieces = merged;
        true
    }

    /// Byte at logical offset `pos` — `None` out of range.
    pub fn get(&self, pos: usize) -> Option<u8> {
        let (i, off) = self.locate(pos)?;
        if i >= self.pieces.len() {
            return None;
        }
        let p = self.pieces[i];
        let b = match p.src {
            Src::Original => self.original[p.start + off],
            Src::Added => self.added[p.start + off],
        };
        Some(b)
    }

    /// Materialize the document — the one `O(len)` flatten.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len());
        for p in &self.pieces {
            match p.src {
                Src::Original => out.extend_from_slice(&self.original[p.start..p.start + p.len]),
                Src::Added => out.extend_from_slice(&self.added[p.start..p.start + p.len]),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: apply the same insert/delete to a plain Vec.
    fn oracle_insert(buf: &mut Vec<u8>, pos: usize, text: &[u8]) {
        buf.splice(pos..pos, text.iter().copied());
    }
    fn oracle_delete(buf: &mut Vec<u8>, pos: usize, count: usize) {
        buf.drain(pos..pos + count);
    }

    #[test]
    fn matches_vec_splice_oracle() {
        let mut rng = SplitMix64::new(0x91EC);
        for _ in 0..300 {
            let init_len = rng.below(24) as usize;
            let init: Vec<u8> = (0..init_len).map(|_| rng.below(256) as u8).collect();
            let mut pt = PieceTable::new(&init);
            let mut oracle = init.clone();
            for _ in 0..24 {
                let len = oracle.len();
                let op = rng.below(3);
                if op == 0 {
                    // insert
                    let pos = rng.below(len as u32 + 1) as usize;
                    let tlen = rng.below(6) as usize;
                    let text: Vec<u8> = (0..tlen).map(|_| rng.below(256) as u8).collect();
                    assert!(pt.insert(pos, &text));
                    oracle_insert(&mut oracle, pos, &text);
                } else if op == 1 && len > 0 {
                    // delete
                    let pos = rng.below(len as u32) as usize;
                    let count = rng.below(len as u32 - pos as u32) as usize + 1;
                    assert!(pt.delete(pos, count));
                    oracle_delete(&mut oracle, pos, count);
                } else {
                    // read check
                    if len > 0 {
                        let pos = rng.below(len as u32) as usize;
                        assert_eq!(pt.get(pos), Some(oracle[pos]));
                    }
                }
                assert_eq!(pt.len(), oracle.len());
                assert_eq!(pt.to_bytes(), oracle);
            }
        }
    }

    #[test]
    fn known_edit_sequences() {
        // Insert inside a piece splits it into left+added+right.
        let mut pt = PieceTable::new(b"ac");
        assert!(pt.insert(1, b"b"));
        assert_eq!(&*pt.to_bytes(), b"abc");
        // Insert at start and end.
        let mut pt = PieceTable::new(b"mid");
        assert!(pt.insert(0, b"pre_"));
        assert!(pt.insert(7, b"_post"));
        assert_eq!(&*pt.to_bytes(), b"pre_mid_post");
        // Delete across piece boundaries.
        let mut pt = PieceTable::new(b"hello");
        assert!(pt.insert(5, b" world"));
        assert!(pt.delete(3, 5)); // drops "lo wo" -> "hel" + "rld"
        assert_eq!(&*pt.to_bytes(), b"helrld");
        // Delete everything.
        let mut pt = PieceTable::new(b"gone");
        assert!(pt.delete(0, 4));
        assert!(pt.is_empty());
        assert_eq!(pt.len(), 0);
    }

    #[test]
    fn rejects_out_of_range_edits() {
        let mut pt = PieceTable::new(b"abc");
        assert!(!pt.insert(4, b"x")); // pos > len
        assert!(!pt.delete(2, 2)); // 2+2 > 3
        assert!(!pt.delete(4, 0)); // pos > len even for empty delete
        assert_eq!(&*pt.to_bytes(), b"abc"); // buffer untouched
        assert_eq!(pt.get(3), None);
        assert_eq!(pt.get(0), Some(b'a'));
    }

    #[test]
    fn empty_and_boundary_semantics() {
        let mut pt = PieceTable::new(b"");
        assert!(pt.is_empty());
        assert!(pt.insert(0, b"x"));
        assert!(!pt.is_empty());
        // Empty insert is a no-op that still reports success inside
        // the document.
        assert!(pt.insert(0, b""));
        assert_eq!(pt.len(), 1);
        // delete(0,0) at len is legal, beyond is not.
        assert!(pt.delete(1, 0));
        assert!(!pt.delete(2, 0));
    }
}
