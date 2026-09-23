//! Minimum queue — a FIFO with `O(1)` amortized `min` via two
//! monotone stacks. Unlike [`crate::slide`]'s window deque,
//! `MinQueue` is a persistent queue object: `push`/`pop`/`min`
//! in amortized constant time, plus `MinStack` for the
//! LIFO-only half.
//!
//! Each stack element carries the running minimum of its
//! stack, so `min` is `min(in.peek_min, out.peek_min)` with no
//! scanning. `pop` re-pours `in → out` when `out` drains,
//! rebuilding out's running minima in one pass — amortized
//! `O(1)` because each element pours at most once.
//!
//! ```
//! use izanagi_kit::minq::MinQueue;
//! let mut q = MinQueue::new();
//! for v in [5i64, 3, 7, 1, 4] {
//!     q.push(v);
//! }
//! assert_eq!(q.min(), Some(1));
//! assert_eq!(q.pop(), Some(5));
//! assert_eq!(q.min(), Some(1));
//! assert_eq!(q.pop(), Some(3));
//! assert_eq!(q.pop(), Some(7));
//! assert_eq!(q.pop(), Some(1));
//! assert_eq!(q.min(), Some(4));
//! ```
//!
//! References: folklore "minimum stack" / "minimum queue"
//! (e-maxx, cp-algorithms "Stack modification / Queue
//! modification" sections).

/// FIFO queue with `O(1)` amortized `min`.
#[derive(Clone, Debug, Default)]
pub struct MinQueue {
    /// `(value, running min of this stack)` pairs.
    in_stk: Vec<(i64, i64)>,
    out_stk: Vec<(i64, i64)>,
}

impl MinQueue {
    /// Empty queue.
    pub fn new() -> MinQueue {
        MinQueue::default()
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.in_stk.len() + self.out_stk.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.in_stk.is_empty() && self.out_stk.is_empty()
    }

    /// Enqueue `v`.
    pub fn push(&mut self, v: i64) {
        let m = self.in_stk.last().map(|&(_, m)| m.min(v)).unwrap_or(v);
        self.in_stk.push((v, m));
    }

    /// Dequeue the oldest element, or `None` when empty.
    pub fn pop(&mut self) -> Option<i64> {
        self.pour();
        self.out_stk.pop().map(|(v, _)| v)
    }

    /// Front element without removing it.
    pub fn peek(&self) -> Option<i64> {
        self.out_stk.last().or(self.in_stk.first()).map(|&(v, _)| v)
    }

    /// Minimum over all queued elements, or `None` when empty.
    pub fn min(&self) -> Option<i64> {
        match (
            self.in_stk.last().map(|&(_, m)| m),
            self.out_stk.last().map(|&(_, m)| m),
        ) {
            (None, None) => None,
            (Some(a), None) | (None, Some(a)) => Some(a),
            (Some(a), Some(b)) => Some(a.min(b)),
        }
    }

    fn pour(&mut self) {
        if self.out_stk.is_empty() {
            while let Some((v, _)) = self.in_stk.pop() {
                let m = self.out_stk.last().map(|&(_, m)| m.min(v)).unwrap_or(v);
                self.out_stk.push((v, m));
            }
        }
    }
}

/// LIFO stack with `O(1)` `min` — the one-sided half of
/// [`MinQueue`].
#[derive(Clone, Debug, Default)]
pub struct MinStack {
    stk: Vec<(i64, i64)>,
}

impl MinStack {
    /// Empty stack.
    pub fn new() -> MinStack {
        MinStack::default()
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.stk.len()
    }

    /// Whether the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.stk.is_empty()
    }

    /// Push `v`.
    pub fn push(&mut self, v: i64) {
        let m = self.stk.last().map(|&(_, m)| m.min(v)).unwrap_or(v);
        self.stk.push((v, m));
    }

    /// Pop the top element, or `None` when empty.
    pub fn pop(&mut self) -> Option<i64> {
        self.stk.pop().map(|(v, _)| v)
    }

    /// Minimum over the stack, or `None` when empty.
    pub fn min(&self) -> Option<i64> {
        self.stk.last().map(|&(_, m)| m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let mut q = MinQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.min(), None);
        assert_eq!(q.pop(), None);
        q.push(4);
        q.push(2);
        assert_eq!(q.min(), Some(2));
        assert_eq!(q.peek(), Some(4));
        let mut s = MinStack::new();
        s.push(9);
        s.push(1);
        s.push(5);
        assert_eq!(s.min(), Some(1));
        s.pop();
        assert_eq!(s.min(), Some(1));
        s.pop();
        assert_eq!(s.min(), Some(9));
    }

    /// Full interleaved shadow oracle: every op applied to a
    /// plain `VecDeque`-equivalent `Vec`.
    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0xA1B2);
        for _ in 0..100 {
            let mut q = MinQueue::new();
            let mut o: Vec<i64> = Vec::new();
            for _ in 0..500 {
                match rng.below(4) {
                    0 => {
                        let v = i64::from(rng.below(61)) - 30;
                        q.push(v);
                        o.push(v);
                    }
                    1 => {
                        assert_eq!(q.pop(), o.first().copied());
                        if !o.is_empty() {
                            o.remove(0);
                        }
                    }
                    2 => {
                        assert_eq!(q.min(), o.iter().min().copied());
                    }
                    _ => {
                        assert_eq!(q.len(), o.len());
                        assert_eq!(q.peek(), o.first().copied());
                    }
                }
            }
        }
    }

    /// The pour path is exercised when `out` drains mid-run.
    #[test]
    fn pour_rebuilds_minima() {
        let mut q = MinQueue::new();
        for v in [7, 3, 9, 2, 8] {
            q.push(v);
        }
        assert_eq!(q.pop(), Some(7)); // first pour
        q.push(1);
        for _ in 0..5 {
            q.pop();
        }
        assert_eq!(q.min(), None);
    }
}
