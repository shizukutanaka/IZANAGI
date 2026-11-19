//! Version control for recorded replay traces: cut, branch, and compare.
//!
//! A persistent replay trace (the `IZTR` format written by
//! [`encode_trace`]) is an input log plus the per-tick state hashes it
//! produced. Once such logs are kept around they get edited the way any
//! versioned artifact does, and this module provides the three operations
//! that keep the edited log honest:
//!
//! - [`truncate`] — keep ticks `0..tick`, drop the rest. The result is
//!   re-encoded, so its checksum is recomputed and it is a canonical trace.
//! - [`fork`] — branch a trace at a tick: keep its prefix, replay the
//!   prefix to **prove** the recorded hashes still reproduce, then append a
//!   new input suffix and record the hashes it produces.
//! - [`diff`] — compare two traces and return the first tick whose state
//!   hashes differ, with the hash on each side.
//!
//! **Format.** Every operation decodes with [`decode_trace`] and encodes with
//! [`encode_trace`]; none of them reads or writes bytes of its own, so the
//! `IZTR` layout ([`TRACE_FORMAT_VERSION`](crate::replay::TRACE_FORMAT_VERSION) 1, 20-byte header, FNV-1a payload
//! checksum, little-endian integers) is the one [`crate::replay`] defines and
//! nothing here can drift from it. Inputs are therefore validated exactly as
//! [`decode_trace`] validates them: any buffer whose magic is not `IZTR` —
//! a save file (`IZNG`), or any other format such as a proof file (`IZPF`) —
//! is rejected with [`TraceError::BadMagic`] rather than reinterpreted.
//! Outputs are canonical: bytes after the declared payload, which
//! [`decode_trace`] ignores, are not carried over.
//!
//! **Relation to `first_divergence` and desync forensics.** [`diff`] is
//! [`first_divergence`] applied to two decoded hash columns — it reuses it
//! rather than re-implementing the scan — and differs only in reporting a
//! missing hash as `None` instead of `0`, so a tick one side never recorded
//! cannot be confused with a recorded hash of zero. It deliberately compares
//! **only the hash columns**: it does not attribute the divergence to an
//! input mismatch or a simulation bug. That attribution (comparing the input
//! streams, reconvergence, a verdict) is the job of the separate
//! `trace_forensics` module (proposed in its own PR, not on this branch's
//! base); `diff` is the narrow "where do these two files part ways" query.
//!
//! The kit reads no ambient input: these functions take and return bytes,
//! and reading or writing files is the caller's job.

use crate::replay::{
    decode_trace, encode_trace, first_divergence, Divergence, ReplayTrace, TraceError,
};
use crate::world_hash::{hash_state, DetHash};

/// Errors returned by the trace-editing operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceEditError {
    /// An input buffer is not a valid `IZTR` trace; carries the decoder's
    /// reason unchanged.
    Decode(TraceError),
    /// [`fork`] was asked to branch at a tick past the end of the recorded
    /// inputs or hashes, so there is no complete prefix to keep.
    TickOutOfRange {
        /// The requested branch tick.
        tick: usize,
        /// The number of ticks for which both an input and a hash are
        /// recorded.
        len: usize,
    },
    /// Replaying the prefix in [`fork`] did not reproduce the recorded
    /// hashes: the supplied initial state or step function is not the one
    /// the trace was recorded with, so a branch from it would be a lie.
    PrefixMismatch(Divergence),
}

impl From<TraceError> for TraceEditError {
    fn from(e: TraceError) -> Self {
        TraceEditError::Decode(e)
    }
}

impl core::fmt::Display for TraceEditError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TraceEditError::Decode(e) => write!(f, "invalid replay trace: {e}"),
            TraceEditError::TickOutOfRange { tick, len } => write!(
                f,
                "cannot fork at tick {tick}: the trace records only {len} complete tick(s)"
            ),
            TraceEditError::PrefixMismatch(d) => {
                write!(f, "fork prefix does not reproduce the trace: {d}")
            }
        }
    }
}

impl std::error::Error for TraceEditError {}

/// The first tick at which two traces' state-hash columns disagree.
///
/// `left` / `right` are the hashes each trace recorded at `tick`; `None`
/// means that trace ended before `tick` (its hash column is shorter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceDiff {
    /// 0-based tick of the first disagreement.
    pub tick: usize,
    /// The left trace's hash at `tick`, or `None` if it has none there.
    pub left: Option<u64>,
    /// The right trace's hash at `tick`, or `None` if it has none there.
    pub right: Option<u64>,
}

/// Return a new trace holding only ticks `0..tick` of `data`.
///
/// Both columns — input frames and expected hashes — are cut to at most
/// `tick` entries; the seed is kept. A `tick` at or past the end leaves the
/// trace's content unchanged. The result is produced by [`encode_trace`],
/// so its header carries [`TRACE_FORMAT_VERSION`](crate::replay::TRACE_FORMAT_VERSION) and a freshly computed
/// checksum, and it decodes back to exactly the truncated value.
pub fn truncate(data: &[u8], tick: usize) -> Result<Vec<u8>, TraceEditError> {
    let mut trace = decode_trace(data)?;
    trace.inputs.truncate(tick);
    trace.expected.truncate(tick);
    Ok(encode_trace(&trace))
}

/// Branch the trace `data` at `tick`, continuing with `suffix` instead of
/// the recorded inputs from `tick` on.
///
/// `state` must be the initial state the trace was recorded from and `step`
/// the step function, taking one input frame as bytes. The first `tick`
/// recorded inputs are replayed onto `state` and the resulting hashes are
/// checked against the recorded ones: the branch is only produced if they
/// match, which is what guarantees the forked trace's hashes agree with the
/// original's up to the branch point. The `suffix` frames are then stepped
/// and their hashes appended. The seed is kept.
///
/// `tick == 0` replaces the whole run. A `tick` beyond the shorter of the
/// two recorded columns is [`TraceEditError::TickOutOfRange`]; a prefix that
/// does not replay to the recorded hashes is
/// [`TraceEditError::PrefixMismatch`].
pub fn fork<S, F>(
    data: &[u8],
    tick: usize,
    mut state: S,
    suffix: &[Vec<u8>],
    mut step: F,
) -> Result<Vec<u8>, TraceEditError>
where
    S: DetHash,
    F: FnMut(&mut S, &[u8]),
{
    let original = decode_trace(data)?;
    let len = original.inputs.len().min(original.expected.len());
    let (inputs, expected) = match (original.inputs.get(..tick), original.expected.get(..tick)) {
        (Some(i), Some(e)) => (i, e),
        _ => return Err(TraceEditError::TickOutOfRange { tick, len }),
    };
    let mut hashes = Vec::with_capacity(tick + suffix.len());
    for frame in inputs {
        step(&mut state, frame);
        hashes.push(hash_state(&state));
    }
    first_divergence(expected, &hashes).map_err(TraceEditError::PrefixMismatch)?;
    for frame in suffix {
        step(&mut state, frame);
        hashes.push(hash_state(&state));
    }
    let mut frames = inputs.to_vec();
    frames.extend(suffix.iter().cloned());
    Ok(encode_trace(&ReplayTrace {
        seed: original.seed,
        inputs: frames,
        expected: hashes,
    }))
}

/// Compare two traces' state-hash columns.
///
/// `Ok(None)` when both record the same hash sequence (same length, same
/// values); otherwise the first disagreeing tick with each side's hash. A
/// length difference is a disagreement at the first tick the shorter trace
/// lacks. Seeds and input frames are not compared — see the module doc.
pub fn diff(left: &[u8], right: &[u8]) -> Result<Option<TraceDiff>, TraceEditError> {
    let left = decode_trace(left)?;
    let right = decode_trace(right)?;
    Ok(match first_divergence(&left.expected, &right.expected) {
        Ok(()) => None,
        Err(d) => Some(TraceDiff {
            tick: d.tick,
            left: left.expected.get(d.tick).copied(),
            right: right.expected.get(d.tick).copied(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{record_trace, TRACE_FORMAT_VERSION};
    use crate::world_hash::Fnv1a;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Sim {
        acc: u64,
    }

    impl DetHash for Sim {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_u64(self.acc);
        }
    }

    fn step(s: &mut Sim, frame: &[u8]) {
        for b in frame {
            s.acc = s.acc.wrapping_mul(31).wrapping_add(*b as u64 + 1);
        }
        s.acc = s.acc.rotate_left(7) ^ 0x9E37_79B9;
    }

    fn frames() -> Vec<Vec<u8>> {
        vec![vec![1], vec![2, 3], Vec::new(), vec![4], vec![5, 6, 7]]
    }

    fn recorded(seed: u64, inputs: Vec<Vec<u8>>) -> Vec<u8> {
        let expected = record_trace(&mut Sim { acc: seed }, &inputs, |s, f: &Vec<u8>| step(s, f));
        encode_trace(&ReplayTrace {
            seed,
            inputs,
            expected,
        })
    }

    /// The header invariants of the IZTR format, checked from raw bytes.
    fn assert_well_formed(data: &[u8]) {
        assert_eq!(&data[0..4], b"IZTR");
        assert_eq!(
            u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            TRACE_FORMAT_VERSION
        );
        let payload_len = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        assert_eq!(data.len(), 20 + payload_len);
        let mut h = Fnv1a::new();
        h.write_bytes(&data[20..]);
        assert_eq!(h.finish().to_le_bytes(), data[8..16]);
        assert_eq!(encode_trace(&decode_trace(data).unwrap()), data);
    }

    #[test]
    fn truncate_keeps_the_prefix_and_recomputes_the_checksum() {
        let data = recorded(9, frames());
        let full = decode_trace(&data).unwrap();
        let cut = truncate(&data, 2).unwrap();
        assert_well_formed(&cut);
        assert_ne!(cut[8..16], data[8..16], "checksum must be recomputed");
        let t = decode_trace(&cut).unwrap();
        assert_eq!(t.seed, 9);
        assert_eq!(t.inputs, full.inputs[..2]);
        assert_eq!(t.expected, full.expected[..2]);
    }

    #[test]
    fn truncate_boundaries_zero_and_past_the_end() {
        let data = recorded(9, frames());
        let empty = truncate(&data, 0).unwrap();
        assert_well_formed(&empty);
        assert_eq!(
            empty,
            encode_trace(&ReplayTrace {
                seed: 9,
                inputs: Vec::new(),
                expected: Vec::new(),
            })
        );
        assert_eq!(truncate(&data, 5).unwrap(), data);
        assert_eq!(truncate(&data, usize::MAX).unwrap(), data);
    }

    #[test]
    fn truncate_cuts_mismatched_columns_independently() {
        let data = encode_trace(&ReplayTrace {
            seed: 1,
            inputs: vec![vec![1], vec![2], vec![3]],
            expected: vec![10],
        });
        let t = decode_trace(&truncate(&data, 2).unwrap()).unwrap();
        assert_eq!(t.inputs, vec![vec![1], vec![2]]);
        assert_eq!(t.expected, vec![10]);
    }

    #[test]
    fn truncate_drops_trailing_bytes_and_is_deterministic() {
        let mut data = recorded(3, frames());
        let canonical = data.clone();
        data.extend_from_slice(&[0xEE, 0xEE]);
        assert_eq!(truncate(&data, 99).unwrap(), canonical);
        assert_eq!(truncate(&data, 3).unwrap(), truncate(&data, 3).unwrap());
    }

    #[test]
    fn operations_reject_non_trace_formats() {
        let mut proof = recorded(3, frames());
        proof[0..4].copy_from_slice(b"IZPF");
        let mut save = recorded(3, frames());
        save[0..4].copy_from_slice(b"IZNG");
        let good = recorded(3, frames());
        for bad in [&proof, &save] {
            let err = TraceEditError::Decode(TraceError::BadMagic);
            assert_eq!(truncate(bad, 1), Err(err));
            assert_eq!(fork(bad, 0, Sim { acc: 3 }, &[], step), Err(err));
            assert_eq!(diff(bad, &good), Err(err));
            assert_eq!(diff(&good, bad), Err(err));
        }
        let mut corrupt = good.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert_eq!(
            truncate(&corrupt, 1),
            Err(TraceEditError::Decode(TraceError::ChecksumMismatch))
        );
    }

    #[test]
    fn fork_matches_the_original_up_to_the_branch_point() {
        let data = recorded(7, frames());
        let original = decode_trace(&data).unwrap();
        let suffix = vec![vec![42], vec![43, 44]];
        let forked = fork(&data, 3, Sim { acc: 7 }, &suffix, step).unwrap();
        assert_well_formed(&forked);
        let t = decode_trace(&forked).unwrap();
        assert_eq!(t.seed, 7);
        assert_eq!(t.expected[..3], original.expected[..3]);
        assert_eq!(t.inputs[..3], original.inputs[..3]);
        assert_eq!(t.inputs[3..], suffix[..]);
        // Oracle: the fork is exactly what recording the spliced inputs
        // from scratch would have produced.
        let mut spliced = original.inputs[..3].to_vec();
        spliced.extend(suffix.iter().cloned());
        assert_eq!(forked, recorded(7, spliced));
        assert_eq!(
            diff(&data, &forked).unwrap().map(|d| d.tick),
            Some(3),
            "the branch diverges exactly at the fork tick"
        );
    }

    #[test]
    fn fork_with_the_recorded_suffix_reproduces_the_trace() {
        let data = recorded(7, frames());
        let original = decode_trace(&data).unwrap();
        for tick in 0..=5 {
            let rest = original.inputs[tick..].to_vec();
            assert_eq!(
                fork(&data, tick, Sim { acc: 7 }, &rest, step).unwrap(),
                data
            );
        }
        assert_eq!(fork(&data, 5, Sim { acc: 7 }, &[], step).unwrap(), data);
    }

    #[test]
    fn fork_rejects_out_of_range_ticks_and_unreproducible_prefixes() {
        let data = recorded(7, frames());
        assert_eq!(
            fork(&data, 6, Sim { acc: 7 }, &[], step),
            Err(TraceEditError::TickOutOfRange { tick: 6, len: 5 })
        );
        let short = encode_trace(&ReplayTrace {
            seed: 7,
            inputs: frames(),
            expected: decode_trace(&data).unwrap().expected[..2].to_vec(),
        });
        assert_eq!(
            fork(&short, 3, Sim { acc: 7 }, &[], step),
            Err(TraceEditError::TickOutOfRange { tick: 3, len: 2 })
        );
        match fork(&data, 2, Sim { acc: 8 }, &[], step) {
            Err(TraceEditError::PrefixMismatch(d)) => assert_eq!(d.tick, 0),
            other => panic!("expected a prefix mismatch, got {other:?}"),
        }
        // Tick 0 has no prefix to verify, so any initial state may branch.
        assert!(fork(&data, 0, Sim { acc: 8 }, &[vec![1]], step).is_ok());
    }

    #[test]
    fn diff_reports_the_first_divergent_tick_with_both_hashes() {
        let a = recorded(5, frames());
        let mut changed = frames();
        changed[2] = vec![99];
        let b = recorded(5, changed);
        let (ta, tb) = (decode_trace(&a).unwrap(), decode_trace(&b).unwrap());
        assert_eq!(diff(&a, &a).unwrap(), None);
        assert_eq!(
            diff(&a, &b).unwrap(),
            Some(TraceDiff {
                tick: 2,
                left: Some(ta.expected[2]),
                right: Some(tb.expected[2]),
            })
        );
        // Reuses first_divergence: same tick on the decoded columns.
        assert_eq!(
            first_divergence(&ta.expected, &tb.expected)
                .unwrap_err()
                .tick,
            2
        );
    }

    #[test]
    fn diff_reports_a_missing_tick_as_none() {
        let a = recorded(5, frames());
        let cut = truncate(&a, 3).unwrap();
        let full = decode_trace(&a).unwrap();
        let expected = TraceDiff {
            tick: 3,
            left: Some(full.expected[3]),
            right: None,
        };
        assert_eq!(diff(&a, &cut).unwrap(), Some(expected));
        assert_eq!(
            diff(&cut, &a).unwrap(),
            Some(TraceDiff {
                tick: 3,
                left: None,
                right: Some(full.expected[3]),
            })
        );
        let empty = truncate(&a, 0).unwrap();
        assert_eq!(diff(&empty, &empty).unwrap(), None);
    }

    #[test]
    fn diff_ignores_seed_and_inputs() {
        let a = recorded(5, frames());
        let mut t = decode_trace(&a).unwrap();
        t.seed ^= 1;
        t.inputs.clear();
        assert_eq!(diff(&a, &encode_trace(&t)).unwrap(), None);
    }

    #[test]
    fn edit_error_display_is_informative() {
        let d = Divergence {
            tick: 4,
            expected: 1,
            actual: 2,
        };
        for (e, needle) in [
            (TraceEditError::Decode(TraceError::BadMagic), "IZTR"),
            (TraceEditError::TickOutOfRange { tick: 9, len: 3 }, "9"),
            (TraceEditError::PrefixMismatch(d), "tick 4"),
        ] {
            assert!(e.to_string().contains(needle), "{e}");
        }
        fn is_error<E: std::error::Error>(_: &E) {}
        is_error(&TraceEditError::from(TraceError::TooShort));
    }
}
