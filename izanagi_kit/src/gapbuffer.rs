//! Gap buffer — the Emacs-style editable byte sequence: an O(1)
//! cursor, O(k) insert/delete of `k` bytes at the cursor, and a
//! move that slides the gap to any position. Where [`crate::rope`]
//! and [`crate::piecetable`] optimize for large structured edits,
//! the gap buffer is the simplest structure that keeps edits
//! contiguous — ideal for line editors and REPL input.
//!
//! Invariant: `buf` holds all content, but `[gap_start, gap_end)`
//! is scratch space. Logical index `i < gap_start` maps to
//! `buf[i]`; `i ≥ gap_start` maps to `buf[i + gap_len]`.
//!
//! ```
//! use izanagi_kit::gapbuffer::GapBuffer;
//! let mut g = GapBuffer::new();
//! g.insert(b"world");
//! g.move_to(0);
//! g.insert(b"hello ");
//! assert_eq!(g.to_vec(), b"hello world");
//! g.delete_before(6); // "hello "
//! assert_eq!(g.to_vec(), b"world");
//! ```

/// A gap-buffered editable byte sequence.
#[derive(Clone, Debug)]
pub struct GapBuffer {
    buf: Vec<u8>,
    gap_start: usize,
    gap_end: usize, // exclusive — content resumes at gap_end
}

impl GapBuffer {
    /// Empty buffer.
    pub fn new() -> GapBuffer {
        GapBuffer {
            buf: Vec::new(),
            gap_start: 0,
            gap_end: 0,
        }
    }

    /// Pre-allocate `cap` bytes of gap.
    pub fn with_capacity(cap: usize) -> GapBuffer {
        GapBuffer {
            buf: vec![0; cap],
            gap_start: 0,
            gap_end: cap,
        }
    }

    /// Content length (excludes the gap).
    pub fn len(&self) -> usize {
        self.buf.len() - (self.gap_end - self.gap_start)
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Cursor position — always `≤ len()`.
    pub fn cursor(&self) -> usize {
        self.gap_start
    }

    /// Byte at logical position `i`, if in range.
    pub fn get(&self, i: usize) -> Option<u8> {
        if i >= self.len() {
            return None;
        }
        if i < self.gap_start {
            Some(self.buf[i])
        } else {
            Some(self.buf[i + (self.gap_end - self.gap_start)])
        }
    }

    /// Slide the gap so the cursor sits at `pos` (clamped to
    /// `len`) — copies the bytes the gap crosses over.
    pub fn move_to(&mut self, pos: usize) {
        let pos = pos.min(self.len());
        if pos < self.gap_start {
            // move gap left: pull the bytes [pos, gap_start)
            // behind the gap
            let count = self.gap_start - pos;
            let dst = self.gap_end - count;
            self.buf.copy_within(pos..self.gap_start, dst);
            self.gap_end = dst;
            self.gap_start = pos;
        } else if pos > self.gap_start {
            // move gap right: pull the bytes [gap_end,
            // gap_end + (pos − gap_start)) in front of the gap
            let count = pos - self.gap_start;
            self.buf
                .copy_within(self.gap_end..self.gap_end + count, self.gap_start);
            self.gap_start = pos;
            self.gap_end += count;
        }
    }

    /// Ensure at least `need` free bytes in the gap.
    fn reserve(&mut self, need: usize) {
        if self.gap_end - self.gap_start >= need {
            return;
        }
        let after = self.buf.len() - self.gap_end;
        let grow = need.max(16);
        let new_len = self.buf.len() + grow;
        let mut buf = vec![0u8; new_len];
        buf[..self.gap_start].copy_from_slice(&self.buf[..self.gap_start]);
        let new_gap_end = new_len - after;
        buf[new_gap_end..].copy_from_slice(&self.buf[self.gap_end..]);
        self.buf = buf;
        self.gap_end = new_gap_end;
    }

    /// Insert `bytes` at the cursor; cursor ends after them.
    pub fn insert(&mut self, bytes: &[u8]) {
        self.reserve(bytes.len());
        self.buf[self.gap_start..self.gap_start + bytes.len()].copy_from_slice(bytes);
        self.gap_start += bytes.len();
    }

    /// Insert one byte.
    pub fn insert_byte(&mut self, b: u8) {
        self.reserve(1);
        self.buf[self.gap_start] = b;
        self.gap_start += 1;
    }

    /// Delete up to `n` bytes *before* the cursor — returns the
    /// number actually deleted.
    pub fn delete_before(&mut self, n: usize) -> usize {
        let take = n.min(self.gap_start);
        self.gap_start -= take;
        take
    }

    /// Delete up to `n` bytes *after* the cursor — returns the
    /// number actually deleted.
    pub fn delete_after(&mut self, n: usize) -> usize {
        let avail = self.buf.len() - self.gap_end;
        let take = n.min(avail);
        self.gap_end += take;
        take
    }

    /// Content before the cursor.
    pub fn before(&self) -> Vec<u8> {
        self.buf[..self.gap_start].to_vec()
    }

    /// Content after the cursor.
    pub fn after(&self) -> Vec<u8> {
        self.buf[self.gap_end..].to_vec()
    }

    /// The whole content as one vector.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.len());
        v.extend_from_slice(&self.buf[..self.gap_start]);
        v.extend_from_slice(&self.buf[self.gap_end..]);
        v
    }
}

impl Default for GapBuffer {
    fn default() -> GapBuffer {
        GapBuffer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let mut g = GapBuffer::new();
        assert!(g.is_empty() && g.cursor() == 0 && g.get(0).is_none());
        g.insert(b"world");
        g.move_to(0);
        g.insert(b"hello ");
        assert_eq!(g.to_vec(), b"hello world");
        assert_eq!(g.before(), b"hello ");
        assert_eq!(g.after(), b"world");
        g.move_to(11);
        g.insert_byte(b'!');
        assert_eq!(g.to_vec(), b"hello world!");
        // delete semantics
        g.move_to(6);
        assert_eq!(g.delete_before(6), 6);
        assert_eq!(g.to_vec(), b"world!");
        assert_eq!(g.delete_before(1), 0);
        assert_eq!(g.delete_after(2), 2);
        assert_eq!(g.to_vec(), b"rld!");
        // clamped moves
        g.move_to(999);
        assert_eq!(g.cursor(), 4);
        g.move_to(0);
        assert_eq!(g.cursor(), 0);
        // get over the gap
        let g2 = GapBuffer::with_capacity(4);
        assert_eq!(g2.len(), 0);
        let mut g2 = g2;
        g2.insert(b"ab");
        g2.move_to(1);
        g2.insert(b"Z");
        assert_eq!(g2.to_vec(), b"aZb");
        assert_eq!(g2.get(2), Some(b'b'));
        assert_eq!(g2.get(3), None);
    }

    /// Shadow Vec<u8> oracle — every op replayed on a plain vec
    /// plus a cursor index; byte-exact comparison of the full
    /// content and every indexed read.
    #[test]
    fn oracle_random_ops() {
        let mut rng = SplitMix64::new(13);
        for _round in 0..150 {
            let mut g = GapBuffer::new();
            let mut want: Vec<u8> = Vec::new();
            let mut cur = 0usize;
            for _ in 0..80 {
                match rng.below(6) {
                    0 => {
                        let k = 1 + rng.below(8);
                        let bytes: Vec<u8> = (0..k).map(|_| b'a' + rng.below(26) as u8).collect();
                        g.insert(&bytes);
                        for (i, &b) in bytes.iter().enumerate() {
                            want.insert(cur + i, b);
                        }
                        cur += bytes.len();
                    }
                    1 => {
                        let b = b'a' + rng.below(26) as u8;
                        g.insert_byte(b);
                        want.insert(cur, b);
                        cur += 1;
                    }
                    2 => {
                        let n = rng.below(10) as usize;
                        let took = g.delete_before(n);
                        let real = n.min(cur);
                        assert_eq!(took, real);
                        want.drain(cur - real..cur);
                        cur -= real;
                    }
                    3 => {
                        let n = rng.below(10) as usize;
                        let took = g.delete_after(n);
                        let real = n.min(want.len() - cur);
                        assert_eq!(took, real);
                        want.drain(cur..cur + real);
                    }
                    _ => {
                        let pos = rng.below((want.len() as u32) + 1) as usize;
                        g.move_to(pos);
                        cur = pos;
                    }
                }
                assert_eq!(g.to_vec(), want, "content diverged");
                assert_eq!(g.len(), want.len());
                assert_eq!(g.cursor(), cur);
                assert_eq!(g.before(), &want[..cur]);
                assert_eq!(g.after(), &want[cur..]);
                for (i, &b) in want.iter().enumerate() {
                    assert_eq!(g.get(i), Some(b));
                }
                assert_eq!(g.get(want.len()), None);
            }
        }
    }
}
