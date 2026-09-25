//! Twitter-style snowflake IDs — 64-bit time-ordered identifiers composing a
//! millisecond timestamp, a worker/machine id, and a per-millisecond sequence
//! (the layout Sonyflake and Discord also use), the sortable sibling of
//! [`crate::ulid`]'s 128-bit form.
//!
//! Layout (63 bits, sign always 0):
//!
//! ```text
//! [ 41 bits: ms since custom epoch | 10 bits: worker | 12 bits: sequence ]
//! ```
//!
//! [`Snowflake::next`] generates strictly-increasing ids; on sequence overflow
//! (4096 ids in one ms) the timestamp field is bumped instead of returning a
//! duplicate, so the id space stays total and order-stable even for logical
//! clocks. [`decompose`]/[`compose`] pack and unpack fields; the epoch is
//! configured per generator via [`Snowflake::new`].
//!
//! ```
//! use izanagi_kit::snowflake::{Snowflake, decompose};
//! let mut sf = Snowflake::new(1_600_000_000_000, 3); // epoch, worker
//! let id = sf.next(1_600_000_000_500);
//! let (ts, worker, seq) = decompose(id);
//! assert_eq!((worker, seq), (3, 0));
//! ```

const WORKER_BITS: u32 = 10;
const SEQ_BITS: u32 = 12;
const MAX_WORKER: u64 = (1 << WORKER_BITS) - 1; // 1023
const MAX_SEQ: u64 = (1 << SEQ_BITS) - 1; // 4095
const TS_MASK: u64 = (1 << 41) - 1;

/// Pack `(ms_since_epoch, worker, seq)` into a snowflake id.
pub fn compose(ts: u64, worker: u64, seq: u64) -> u64 {
    ((ts & TS_MASK) << (WORKER_BITS + SEQ_BITS))
        | ((worker & MAX_WORKER) << SEQ_BITS)
        | (seq & MAX_SEQ)
}

/// Unpack `id` into `(ms_since_epoch, worker, seq)`.
pub fn decompose(id: u64) -> (u64, u64, u64) {
    (
        id >> (WORKER_BITS + SEQ_BITS),
        (id >> SEQ_BITS) & MAX_WORKER,
        id & MAX_SEQ,
    )
}

/// A snowflake generator bound to one worker id and a custom epoch.
///
/// The generator never goes backwards: if `now_ms` is earlier than the last
/// issued timestamp (clock skew), it reuses the previous timestamp and keeps
/// incrementing the sequence instead — the classic NTP-rollback fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snowflake {
    epoch: u64,
    worker: u64,
    last_ts: u64,
    seq: u64,
}

impl Snowflake {
    /// A generator for `worker` (masked to 10 bits) with custom `epoch` ms.
    pub fn new(epoch: u64, worker: u64) -> Self {
        Self {
            epoch,
            worker: worker & MAX_WORKER,
            last_ts: 0,
            seq: 0,
        }
    }

    /// The worker id (0..1023).
    pub fn worker(&self) -> u64 {
        self.worker
    }

    /// Next id at wall time `now_ms` (real ms since Unix epoch; the
    /// configured epoch is subtracted — clamped at 0 so pre-epoch times are
    /// not negative).
    ///
    /// Monotonicity rules: same ms → `seq+1`; seq overflow → last_ts+1;
    /// earlier `now_ms` → reuse `last_ts`. Always strictly increasing.
    pub fn next(&mut self, now_ms: u64) -> u64 {
        let ts = now_ms.saturating_sub(self.epoch);
        if ts == self.last_ts {
            self.seq += 1;
            if self.seq > MAX_SEQ {
                self.seq = 0;
                self.last_ts += 1;
            }
        } else if ts > self.last_ts {
            self.last_ts = ts;
            self.seq = 0;
        } else {
            // Clock went backwards: keep last_ts, take next seq.
            self.seq += 1;
            if self.seq > MAX_SEQ {
                self.seq = 0;
                self.last_ts += 1;
            }
        }
        compose(self.last_ts, self.worker, self.seq)
    }

    /// Batch `n` ids at `now_ms` (just `next` n times — seq spills to
    /// virtual ms on overflow, so more than 4096 ids in one call still
    /// strictly increase).
    pub fn batch(&mut self, now_ms: u64, n: usize) -> Vec<u64> {
        (0..n).map(|_| self.next(now_ms)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPOCH: u64 = 1_600_000_000_000;

    #[test]
    fn compose_decompose_round_trip() {
        for &(ts, w, s) in &[
            (0u64, 0u64, 0u64),
            (123_456, 1, 0),
            (123_456, 1023, 4095),
            (TS_MASK, MAX_WORKER, MAX_SEQ),
        ] {
            assert_eq!(decompose(compose(ts, w, s)), (ts, w, s));
        }
        // Field masking: out-of-range inputs wrap, never corrupt neighbors.
        let id = compose(1, 1024 + 5, 4096 + 7);
        assert_eq!(decompose(id), (1, 5, 7));
    }

    #[test]
    fn ids_are_sortable_by_time() {
        let mut sf = Snowflake::new(EPOCH, 7);
        let a = sf.next(EPOCH + 10);
        let b = sf.next(EPOCH + 11);
        assert!(a < b); // later ms → larger id
        assert_eq!(decompose(a), (10, 7, 0));
        assert_eq!(decompose(b), (11, 7, 0));
    }

    #[test]
    fn same_ms_increments_seq() {
        let mut sf = Snowflake::new(EPOCH, 1);
        let a = sf.next(EPOCH + 100);
        let b = sf.next(EPOCH + 100);
        let c = sf.next(EPOCH + 100);
        assert!(a < b && b < c);
        assert_eq!(decompose(b).2, 1);
        assert_eq!(decompose(c).2, 2);
    }

    #[test]
    fn sequence_overflow_bumps_virtual_ms() {
        let mut sf = Snowflake::new(EPOCH, 1);
        let first = sf.next(EPOCH + 50);
        let mut last = first;
        for _ in 0..4095 {
            last = sf.next(EPOCH + 50); // fills seq 1..4095
        }
        let over = sf.next(EPOCH + 50); // seq exhausted → ts 51, seq 0
        assert!(first < last && last < over);
        assert_eq!(decompose(over), (51, 1, 0));
        // The next real ms after the virtual bump lands at ts 52.
        let next = sf.next(EPOCH + 52);
        assert!(next > over);
        assert_eq!(decompose(next), (52, 1, 0));
        // A wall time *behind* the virtual ts (51 < 52) counts as a rollback:
        // it keeps ts 52 and extends the sequence there.
        assert_eq!(decompose(sf.next(EPOCH + 51)), (52, 1, 1));
    }

    #[test]
    fn clock_rollback_stays_monotonic() {
        let mut sf = Snowflake::new(EPOCH, 2);
        let a = sf.next(EPOCH + 1000);
        let b = sf.next(EPOCH + 500); // backwards 500ms
        let c = sf.next(EPOCH + 1000);
        assert!(a < b && b < c);
        assert_eq!(decompose(b).0, 1000); // kept last_ts, seq 1
        assert_eq!(decompose(c).2, 2);
    }

    #[test]
    fn batch_and_worker_mask() {
        let mut sf = Snowflake::new(EPOCH, 1024 + 9);
        assert_eq!(sf.worker(), 9);
        let ids = sf.batch(EPOCH + 1, 5000); // > 4096
        for w in ids.windows(2) {
            assert!(w[0] < w[1]);
        }
        // Deterministic replay: fresh generator repeats the stream.
        let mut sf2 = Snowflake::new(EPOCH, 9);
        assert_eq!(ids, sf2.batch(EPOCH + 1, 5000));
    }
}
