//! Proof-carrying execution: a run ships with a compact, self-checking
//! witness of what it computed — the IZPF proof format.
//!
//! A recorded [`crate::replay::ReplayTrace`] (IZTR) says *"these inputs
//! produce these hashes"* and carries every per-tick hash. An
//! [`ExecutionProof`] (IZPF) makes the same claim in bounded size and binds
//! it cryptographically: it commits to the run's seed, the hash of the
//! initial state, a SHA-256 digest of the input frames, and a SHA-256 chain
//! over **every** per-tick state hash, while storing only sparse checkpoints
//! and the final hash in the clear. Whoever holds the proof and the inputs
//! can then check the result two ways:
//!
//! - [`verify_proof`] — **re-execution**: replay the inputs from the
//!   verifier's own initial state and require every committed value to
//!   match. A wrong result is localised to the first checkpoint it misses.
//! - [`verify_proof_against_trace`] — **cross-check without a simulation**:
//!   require a recorded IZTR trace and the proof to describe the same run
//!   (same seed, same inputs, same hashes), e.g. on a server that stores
//!   traces but cannot run the game.
//!
//! What the proof is, and is not:
//!
//! - It is **not** a succinct proof in the SNARK sense: verifying the
//!   *computation* still costs a full re-execution. What it buys is that a
//!   claimed result (final hash, checkpoints) cannot be edited — nor the
//!   inputs swapped — without the commitment failing, and that a mismatch is
//!   reported as a typed [`ProofRejection`] instead of a bare `false`.
//! - The per-tick state hashes themselves are 64-bit FNV-1a
//!   ([`crate::world_hash`]), which detect accidental divergence but are not
//!   collision-resistant against an adversary who controls the state. The
//!   SHA-256 layers protect the proof's *own* fields, not the strength of
//!   the state hash underneath.
//!
//! The byte format reuses the trace's framing so the two decoders fail the
//! same ways: magic `b"IZPF"` (deliberately distinct from IZTR's
//! `b"IZTR"`), a version ([`PROOF_FORMAT_VERSION`]), an FNV-1a checksum of
//! the payload, and a payload length — all integers little-endian.
//!
//! ```
//! use izanagi_kit::proof::{decode_proof, encode_proof, prove, verify_proof};
//! use izanagi_kit::world_hash::{DetHash, Fnv1a};
//!
//! #[derive(Clone)]
//! struct Counter(u64);
//! impl DetHash for Counter {
//!     fn det_hash(&self, h: &mut Fnv1a) { h.write_u64(self.0); }
//! }
//! fn step(c: &mut Counter, frame: &[u8]) {
//!     for b in frame { c.0 = c.0.wrapping_mul(31).wrapping_add(u64::from(*b)); }
//! }
//!
//! let inputs = vec![vec![1u8], vec![2, 3], vec![], vec![4]];
//! let proof = prove(7, &mut Counter(0), &inputs, 2, step);
//!
//! // The proof survives a round trip through bytes ...
//! let restored = decode_proof(&encode_proof(&proof)).unwrap();
//! assert_eq!(restored, proof);
//! // ... and an independent re-execution accepts it.
//! assert!(verify_proof(&restored, &mut Counter(0), &inputs, step).is_ok());
//! // A different starting state is rejected.
//! assert!(verify_proof(&restored, &mut Counter(1), &inputs, step).is_err());
//! ```

use crate::replay::ReplayTrace;
use crate::sha256::Sha256;
use crate::world_hash::{hash_state, DetHash, Fnv1a};

/// Magic bytes opening every encoded proof.
const PROOF_MAGIC: &[u8; 4] = b"IZPF";

/// Header size: magic(4) + version(4) + checksum(8) + payload length(8).
const PROOF_HEADER_LEN: usize = 24;

/// Fixed payload size excluding the checkpoint hashes: seed(8) +
/// initial_hash(8) + ticks(8) + input_digest(32) + checkpoint_interval(4) +
/// checkpoint_count(8) + final_hash(8) + commitment(32).
const PROOF_FIXED_PAYLOAD_LEN: usize = 108;

/// Domain-separation tag for [`input_digest`].
const INPUT_DOMAIN: &[u8] = b"IZPF/inputs/v1";

/// Domain-separation tag for [`execution_commitment`].
const CHAIN_DOMAIN: &[u8] = b"IZPF/chain/v1";

/// Version of the proof format written by [`encode_proof`] and required by
/// [`decode_proof`]. Bump it — never silently change the layout — when the
/// payload schema or either commitment's definition changes.
pub const PROOF_FORMAT_VERSION: u32 = 1;

/// A proof that a run starting from a state hashing to `initial_hash`,
/// consuming exactly the input frames digested in `input_digest`, produced
/// the per-tick state hashes committed to in `commitment`.
///
/// `checkpoints[k]` is the state hash after tick `(k + 1) * checkpoint_interval`
/// (ticks counted from 1), for every such tick `<= ticks`; an interval of 0
/// stores no checkpoints. `final_hash` is the state hash after the last
/// tick, or `initial_hash` when `ticks == 0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionProof {
    /// The run's seed, echoed verbatim (the same role as
    /// [`ReplayTrace::seed`]).
    pub seed: u64,
    /// [`hash_state`] of the state before the first tick.
    pub initial_hash: u64,
    /// Number of ticks executed — one per input frame.
    pub ticks: u64,
    /// [`input_digest`] of the input frames, in tick order.
    pub input_digest: [u8; 32],
    /// Ticks between stored checkpoints; 0 stores none.
    pub checkpoint_interval: u32,
    /// State hashes at every `checkpoint_interval`-th tick.
    pub checkpoints: Vec<u64>,
    /// State hash after the last tick (`initial_hash` for an empty run).
    pub final_hash: u64,
    /// [`execution_commitment`] over the seed, initial hash, inputs and every
    /// per-tick state hash.
    pub commitment: [u8; 32],
}

/// Why a proof was not accepted by [`verify_proof`] or
/// [`verify_proof_against_trace`]. Checks run in the order the variants are
/// listed, so the first reported reason is the most fundamental one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProofRejection {
    /// The proof's checkpoint list is inconsistent with its own `ticks` and
    /// `checkpoint_interval` — it was not produced by [`prove`].
    Malformed,
    /// The proof and the trace name different seeds (trace check only).
    SeedMismatch {
        /// Seed in the proof.
        proof: u64,
        /// Seed in the trace.
        trace: u64,
    },
    /// The proof covers a different number of ticks than the inputs given
    /// (or, for a trace, than its inputs/hashes).
    LengthMismatch {
        /// Ticks the proof claims.
        claimed: u64,
        /// Ticks actually supplied.
        actual: u64,
    },
    /// The supplied input frames do not match the proof's input digest.
    InputsMismatch,
    /// The verifier's initial state does not hash to the proof's
    /// `initial_hash` (re-execution only).
    InitialStateMismatch {
        /// Hash the proof claims.
        claimed: u64,
        /// Hash of the verifier's state.
        actual: u64,
    },
    /// The state hash at a checkpoint differs from the one the proof stores.
    CheckpointMismatch {
        /// The tick (counted from 1) whose hash differs.
        tick: u64,
        /// Hash the proof claims.
        claimed: u64,
        /// Hash actually observed.
        actual: u64,
    },
    /// The state hash after the last tick differs from the proof's.
    FinalHashMismatch {
        /// Hash the proof claims.
        claimed: u64,
        /// Hash actually observed.
        actual: u64,
    },
    /// Every stored value matched, but the commitment over all per-tick
    /// hashes did not: some tick between checkpoints diverged, or a stored
    /// field was edited after proving.
    CommitmentMismatch,
}

impl core::fmt::Display for ProofRejection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProofRejection::Malformed => {
                write!(f, "proof is malformed: checkpoint count does not match its interval")
            }
            ProofRejection::SeedMismatch { proof, trace } => {
                write!(f, "proof seed {proof:#018x} != trace seed {trace:#018x}")
            }
            ProofRejection::LengthMismatch { claimed, actual } => {
                write!(f, "proof covers {claimed} ticks but {actual} were supplied")
            }
            ProofRejection::InputsMismatch => {
                write!(f, "input frames do not match the proof's input digest")
            }
            ProofRejection::InitialStateMismatch { claimed, actual } => write!(
                f,
                "initial state hash {actual:#018x} != proof's {claimed:#018x}"
            ),
            ProofRejection::CheckpointMismatch {
                tick,
                claimed,
                actual,
            } => write!(
                f,
                "checkpoint at tick {tick}: proof claims {claimed:#018x}, observed {actual:#018x}"
            ),
            ProofRejection::FinalHashMismatch { claimed, actual } => write!(
                f,
                "final hash: proof claims {claimed:#018x}, observed {actual:#018x}"
            ),
            ProofRejection::CommitmentMismatch => write!(
                f,
                "execution commitment mismatch: a tick between checkpoints diverged or the proof was edited"
            ),
        }
    }
}

impl std::error::Error for ProofRejection {}

/// SHA-256 digest of a sequence of input frames, in order.
///
/// Each frame is length-prefixed (u64 little-endian) and the sequence is
/// prefixed by its count, so `[[1, 2]]` and `[[1], [2]]` digest differently.
pub fn input_digest(inputs: &[Vec<u8>]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.write(INPUT_DOMAIN);
    h.write(&(inputs.len() as u64).to_le_bytes());
    for frame in inputs {
        h.write(&(frame.len() as u64).to_le_bytes());
        h.write(frame);
    }
    h.finish()
}

/// The SHA-256 commitment binding a run: seed, initial state hash, input
/// digest, tick count, and every per-tick state hash in order.
///
/// [`prove`] stores this value; both verifiers recompute it from what they
/// observed, so a single differing tick anywhere in the run changes it.
pub fn execution_commitment(
    seed: u64,
    initial_hash: u64,
    input_digest: &[u8; 32],
    tick_hashes: &[u64],
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.write(CHAIN_DOMAIN);
    h.write(&seed.to_le_bytes());
    h.write(&initial_hash.to_le_bytes());
    h.write(input_digest);
    h.write(&(tick_hashes.len() as u64).to_le_bytes());
    for hash in tick_hashes {
        h.write(&hash.to_le_bytes());
    }
    h.finish()
}

/// Run `inputs` on `state` with `step` and return the proof of that run.
///
/// `state` is advanced in place (like [`crate::replay::record_trace`]); its
/// hash is taken before the first tick and after every tick. A
/// `checkpoint_interval` of 0 stores no intermediate checkpoints — the
/// proof is then constant-size, and a mismatch is still detected (by the
/// final hash or the commitment) but not localised.
pub fn prove<S, F>(
    seed: u64,
    state: &mut S,
    inputs: &[Vec<u8>],
    checkpoint_interval: u32,
    mut step: F,
) -> ExecutionProof
where
    S: DetHash,
    F: FnMut(&mut S, &[u8]),
{
    let initial_hash = hash_state(state);
    let mut tick_hashes = Vec::with_capacity(inputs.len());
    for frame in inputs {
        step(state, frame);
        tick_hashes.push(hash_state(state));
    }
    let digest = input_digest(inputs);
    ExecutionProof {
        seed,
        initial_hash,
        ticks: tick_hashes.len() as u64,
        input_digest: digest,
        checkpoint_interval,
        checkpoints: checkpoints_of(&tick_hashes, checkpoint_interval),
        final_hash: tick_hashes.last().copied().unwrap_or(initial_hash),
        commitment: execution_commitment(seed, initial_hash, &digest, &tick_hashes),
    }
}

/// Verify `proof` by re-executing `inputs` from `state` with `step`.
///
/// `state` must be the verifier's own reconstruction of the starting state
/// (typically built from `proof.seed`); it is advanced in place. Cheap
/// checks run first — structure, length, input digest, initial hash — so a
/// proof for different inputs is rejected without simulating anything.
pub fn verify_proof<S, F>(
    proof: &ExecutionProof,
    state: &mut S,
    inputs: &[Vec<u8>],
    mut step: F,
) -> Result<(), ProofRejection>
where
    S: DetHash,
    F: FnMut(&mut S, &[u8]),
{
    check_structure(proof)?;
    check_inputs(proof, inputs)?;
    let initial_hash = hash_state(state);
    if initial_hash != proof.initial_hash {
        return Err(ProofRejection::InitialStateMismatch {
            claimed: proof.initial_hash,
            actual: initial_hash,
        });
    }
    let mut tick_hashes = Vec::with_capacity(inputs.len());
    for frame in inputs {
        step(state, frame);
        tick_hashes.push(hash_state(state));
    }
    check_hashes(proof, &tick_hashes)
}

/// Verify that `proof` and a recorded IZTR `trace` describe the same run,
/// without executing anything.
///
/// Checks the seed, that the trace's inputs match the proof's digest, and
/// that the trace's per-tick hashes reproduce the checkpoints, final hash
/// and commitment. The proof's `initial_hash` cannot be checked this way —
/// a trace does not record the starting state — so it is taken on trust
/// here and is only enforced by [`verify_proof`].
pub fn verify_proof_against_trace(
    proof: &ExecutionProof,
    trace: &ReplayTrace,
) -> Result<(), ProofRejection> {
    check_structure(proof)?;
    if proof.seed != trace.seed {
        return Err(ProofRejection::SeedMismatch {
            proof: proof.seed,
            trace: trace.seed,
        });
    }
    if trace.expected.len() != trace.inputs.len() {
        return Err(ProofRejection::LengthMismatch {
            claimed: proof.ticks,
            actual: trace.expected.len().min(trace.inputs.len()) as u64,
        });
    }
    check_inputs(proof, &trace.inputs)?;
    check_hashes(proof, &trace.expected)
}

/// The checkpoint hashes a proof with this interval stores.
fn checkpoints_of(tick_hashes: &[u64], interval: u32) -> Vec<u64> {
    if interval == 0 {
        return Vec::new();
    }
    tick_hashes
        .iter()
        .skip(interval as usize - 1)
        .step_by(interval as usize)
        .copied()
        .collect()
}

/// Number of checkpoints a proof over `ticks` ticks at `interval` stores.
fn expected_checkpoints(ticks: u64, interval: u32) -> u64 {
    if interval == 0 {
        0
    } else {
        ticks / u64::from(interval)
    }
}

fn check_structure(proof: &ExecutionProof) -> Result<(), ProofRejection> {
    let expected = expected_checkpoints(proof.ticks, proof.checkpoint_interval);
    if proof.checkpoints.len() as u64 != expected {
        return Err(ProofRejection::Malformed);
    }
    Ok(())
}

fn check_inputs(proof: &ExecutionProof, inputs: &[Vec<u8>]) -> Result<(), ProofRejection> {
    if inputs.len() as u64 != proof.ticks {
        return Err(ProofRejection::LengthMismatch {
            claimed: proof.ticks,
            actual: inputs.len() as u64,
        });
    }
    if input_digest(inputs) != proof.input_digest {
        return Err(ProofRejection::InputsMismatch);
    }
    Ok(())
}

/// Compare observed per-tick hashes against the proof's checkpoints, final
/// hash and commitment, in that order. The caller has already checked that
/// `tick_hashes.len() == proof.ticks`.
fn check_hashes(proof: &ExecutionProof, tick_hashes: &[u64]) -> Result<(), ProofRejection> {
    let observed = checkpoints_of(tick_hashes, proof.checkpoint_interval);
    let interval = u64::from(proof.checkpoint_interval);
    for (k, (&claimed, &actual)) in proof.checkpoints.iter().zip(observed.iter()).enumerate() {
        if claimed != actual {
            return Err(ProofRejection::CheckpointMismatch {
                tick: (k as u64 + 1) * interval,
                claimed,
                actual,
            });
        }
    }
    let final_hash = tick_hashes.last().copied().unwrap_or(proof.initial_hash);
    if final_hash != proof.final_hash {
        return Err(ProofRejection::FinalHashMismatch {
            claimed: proof.final_hash,
            actual: final_hash,
        });
    }
    let commitment = execution_commitment(
        proof.seed,
        proof.initial_hash,
        &proof.input_digest,
        tick_hashes,
    );
    if commitment != proof.commitment {
        return Err(ProofRejection::CommitmentMismatch);
    }
    Ok(())
}

/// Encode an [`ExecutionProof`] into its canonical byte form.
///
/// A pure function of the proof value: the same proof always produces the
/// same bytes on every platform. Layout (all integers little-endian):
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ magic "IZPF"[4] │ version[4] │ checksum[8] │ payload_len[8]      │
/// ├──────────────────────────────────────────────────────────────────┤
/// │ seed[8] │ initial_hash[8] │ ticks[8] │ input_digest[32]          │
/// │ checkpoint_interval[4] │ checkpoint_count[8] │ per checkpoint[8] │
/// │ final_hash[8] │ commitment[32]                                   │
/// └──────────────────────────────────────────────────────────────────┘
/// ```
///
/// The checksum is FNV-1a over the payload — it catches corruption, while
/// the SHA-256 commitment inside catches edits to the claims themselves.
pub fn encode_proof(proof: &ExecutionProof) -> Vec<u8> {
    let mut payload = Vec::with_capacity(PROOF_FIXED_PAYLOAD_LEN + 8 * proof.checkpoints.len());
    payload.extend_from_slice(&proof.seed.to_le_bytes());
    payload.extend_from_slice(&proof.initial_hash.to_le_bytes());
    payload.extend_from_slice(&proof.ticks.to_le_bytes());
    payload.extend_from_slice(&proof.input_digest);
    payload.extend_from_slice(&proof.checkpoint_interval.to_le_bytes());
    payload.extend_from_slice(&(proof.checkpoints.len() as u64).to_le_bytes());
    for hash in &proof.checkpoints {
        payload.extend_from_slice(&hash.to_le_bytes());
    }
    payload.extend_from_slice(&proof.final_hash.to_le_bytes());
    payload.extend_from_slice(&proof.commitment);
    let mut hasher = Fnv1a::new();
    hasher.write_bytes(&payload);
    let mut out = Vec::with_capacity(PROOF_HEADER_LEN + payload.len());
    out.extend_from_slice(PROOF_MAGIC);
    out.extend_from_slice(&PROOF_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&hasher.finish().to_le_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

/// Decode an [`ExecutionProof`] written by [`encode_proof`].
///
/// Total: truncated buffers, hostile lengths and garbage all return a
/// [`ProofError`], never panic, and no allocation is sized by a count the
/// buffer does not back. Bytes after the declared payload are ignored, as
/// in [`crate::replay::decode_trace`]. Decoding does **not** verify the
/// proof — pass the result to [`verify_proof`] or
/// [`verify_proof_against_trace`] for that.
pub fn decode_proof(data: &[u8]) -> Result<ExecutionProof, ProofError> {
    let mut header = Reader { data, pos: 0 };
    let magic = header.take(4).ok_or(ProofError::TooShort)?;
    if magic != PROOF_MAGIC {
        return Err(ProofError::BadMagic);
    }
    let version = header.u32().ok_or(ProofError::TooShort)?;
    if version != PROOF_FORMAT_VERSION {
        return Err(ProofError::UnsupportedVersion(version));
    }
    let checksum = header.u64().ok_or(ProofError::TooShort)?;
    let payload_len = header.u64().ok_or(ProofError::TooShort)?;
    if payload_len > header.remaining() as u64 {
        return Err(ProofError::TooShort);
    }
    let payload = header
        .take(payload_len as usize)
        .ok_or(ProofError::TooShort)?;
    let mut hasher = Fnv1a::new();
    hasher.write_bytes(payload);
    if hasher.finish() != checksum {
        return Err(ProofError::ChecksumMismatch);
    }
    parse_payload(payload)
}

fn parse_payload(payload: &[u8]) -> Result<ExecutionProof, ProofError> {
    let mut r = Reader {
        data: payload,
        pos: 0,
    };
    let seed = r.u64().ok_or(ProofError::TooShort)?;
    let initial_hash = r.u64().ok_or(ProofError::TooShort)?;
    let ticks = r.u64().ok_or(ProofError::TooShort)?;
    let input_digest = r.digest().ok_or(ProofError::TooShort)?;
    let checkpoint_interval = r.u32().ok_or(ProofError::TooShort)?;
    let count = r.u64().ok_or(ProofError::TooShort)?;
    if count != expected_checkpoints(ticks, checkpoint_interval) {
        return Err(ProofError::Malformed);
    }
    // Checkpoints are fixed-size, so the bound is exact and is checked before
    // allocating; the 40 bytes of final hash + commitment must still follow.
    let needed = count.checked_mul(8).ok_or(ProofError::TooShort)?;
    if needed > r.remaining() as u64 {
        return Err(ProofError::TooShort);
    }
    let mut checkpoints = Vec::with_capacity(count as usize);
    for _ in 0..count {
        checkpoints.push(r.u64().ok_or(ProofError::TooShort)?);
    }
    let final_hash = r.u64().ok_or(ProofError::TooShort)?;
    let commitment = r.digest().ok_or(ProofError::TooShort)?;
    if r.remaining() != 0 {
        return Err(ProofError::TrailingBytes);
    }
    Ok(ExecutionProof {
        seed,
        initial_hash,
        ticks,
        input_digest,
        checkpoint_interval,
        checkpoints,
        final_hash,
        commitment,
    })
}

/// Bounds-checked little-endian reader: every read returns `None` rather
/// than indexing past the end.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if n > self.remaining() {
            return None;
        }
        let out = self.data.get(self.pos..self.pos + n)?;
        self.pos += n;
        Some(out)
    }

    fn u32(&mut self) -> Option<u32> {
        let b = self.take(4)?;
        let mut a = [0u8; 4];
        a.copy_from_slice(b);
        Some(u32::from_le_bytes(a))
    }

    fn u64(&mut self) -> Option<u64> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Some(u64::from_le_bytes(a))
    }

    fn digest(&mut self) -> Option<[u8; 32]> {
        let b = self.take(32)?;
        let mut a = [0u8; 32];
        a.copy_from_slice(b);
        Some(a)
    }
}

/// Errors returned by [`decode_proof`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofError {
    /// The buffer is shorter than the header, or a declared length or count
    /// exceeds the bytes present.
    TooShort,
    /// The first four bytes are not `b"IZPF"` — not a proof at all (an IZTR
    /// trace lands here too).
    BadMagic,
    /// The format version is not [`PROOF_FORMAT_VERSION`]; the stored
    /// version is carried so the caller can report or migrate it.
    UnsupportedVersion(u32),
    /// The payload's FNV-1a hash does not match the stored checksum.
    ChecksumMismatch,
    /// The checkpoint count disagrees with the stored tick count and
    /// interval — the payload was not produced by [`encode_proof`].
    Malformed,
    /// Bytes remain after the commitment.
    TrailingBytes,
}

impl ProofError {
    /// `true` for a wrong-file condition ([`BadMagic`](Self::BadMagic))
    /// rather than corruption, mirroring
    /// [`crate::replay::TraceError::is_recoverable`].
    #[inline]
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ProofError::BadMagic)
    }

    /// A short, allocation-free description of the error.
    #[inline]
    pub fn message(&self) -> &'static str {
        match self {
            ProofError::TooShort => "execution proof too short",
            ProofError::BadMagic => "not an execution proof (bad magic)",
            ProofError::UnsupportedVersion(_) => "unsupported execution proof format version",
            ProofError::ChecksumMismatch => "execution proof corrupted (checksum mismatch)",
            ProofError::Malformed => "execution proof malformed (checkpoint count)",
            ProofError::TrailingBytes => "execution proof has trailing bytes",
        }
    }
}

impl core::fmt::Display for ProofError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProofError::UnsupportedVersion(v) => write!(
                f,
                "unsupported execution proof format version {v} (this build reads version {})",
                PROOF_FORMAT_VERSION
            ),
            other => f.write_str(other.message()),
        }
    }
}

impl std::error::Error for ProofError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{decode_trace, encode_trace, record_trace};
    use crate::rng::SplitMix64;

    /// State driven by both the input bytes and a seeded RNG, so a wrong
    /// seed, a wrong frame or a skipped tick all change the hashes.
    #[derive(Clone)]
    struct Sim {
        acc: u64,
        rng: SplitMix64,
    }

    impl DetHash for Sim {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_u64(self.acc);
            self.rng.det_hash(h);
        }
    }

    fn new_sim(seed: u64) -> Sim {
        Sim {
            acc: 0,
            rng: SplitMix64::new(seed),
        }
    }

    fn step(s: &mut Sim, frame: &[u8]) {
        for b in frame {
            s.acc = s.acc.rotate_left(5) ^ u64::from(*b);
        }
        s.acc = s.acc.wrapping_add(s.rng.next_u64() & 0xff);
    }

    fn inputs(n: usize) -> Vec<Vec<u8>> {
        (0..n).map(|i| vec![i as u8; i % 4]).collect()
    }

    fn hashes_of(seed: u64, inputs: &[Vec<u8>]) -> Vec<u64> {
        let mut s = new_sim(seed);
        record_trace(&mut s, inputs, |s: &mut Sim, f: &Vec<u8>| step(s, f))
    }

    #[test]
    fn honest_proof_verifies_by_reexecution() {
        let ins = inputs(10);
        let proof = prove(9, &mut new_sim(9), &ins, 3, step);
        assert_eq!(proof.ticks, 10);
        assert_eq!(proof.checkpoints.len(), 3);
        assert_eq!(verify_proof(&proof, &mut new_sim(9), &ins, step), Ok(()));
    }

    #[test]
    fn checkpoints_and_final_hash_agree_with_record_trace() {
        // Oracle: the trusted trace recorder, not hand-computed constants.
        let ins = inputs(12);
        let hashes = hashes_of(4, &ins);
        let proof = prove(4, &mut new_sim(4), &ins, 4, step);
        assert_eq!(proof.checkpoints, vec![hashes[3], hashes[7], hashes[11]]);
        assert_eq!(proof.final_hash, hashes[11]);
        assert_eq!(proof.initial_hash, hash_state(&new_sim(4)));
        assert_eq!(
            proof.commitment,
            execution_commitment(4, proof.initial_hash, &input_digest(&ins), &hashes)
        );
    }

    #[test]
    fn empty_run_and_zero_interval_are_well_defined() {
        let proof = prove(1, &mut new_sim(1), &[], 5, step);
        assert_eq!(proof.ticks, 0);
        assert!(proof.checkpoints.is_empty());
        assert_eq!(proof.final_hash, proof.initial_hash);
        assert_eq!(verify_proof(&proof, &mut new_sim(1), &[], step), Ok(()));

        let ins = inputs(7);
        let sparse = prove(1, &mut new_sim(1), &ins, 0, step);
        assert!(sparse.checkpoints.is_empty());
        assert_eq!(verify_proof(&sparse, &mut new_sim(1), &ins, step), Ok(()));
        // An interval longer than the run stores nothing and is still valid.
        let long = prove(1, &mut new_sim(1), &ins, 100, step);
        assert!(long.checkpoints.is_empty());
        assert_eq!(verify_proof(&long, &mut new_sim(1), &ins, step), Ok(()));
    }

    #[test]
    fn proving_is_deterministic() {
        let ins = inputs(20);
        let a = prove(3, &mut new_sim(3), &ins, 2, step);
        let b = prove(3, &mut new_sim(3), &ins, 2, step);
        assert_eq!(a, b);
        assert_eq!(encode_proof(&a), encode_proof(&b));
    }

    #[test]
    fn wrong_initial_state_is_rejected_before_simulating() {
        let ins = inputs(5);
        let proof = prove(2, &mut new_sim(2), &ins, 1, step);
        let mut steps = 0;
        let r = verify_proof(&proof, &mut new_sim(99), &ins, |s: &mut Sim, f: &[u8]| {
            steps += 1;
            step(s, f)
        });
        assert!(matches!(
            r,
            Err(ProofRejection::InitialStateMismatch { .. })
        ));
        assert_eq!(steps, 0);
    }

    #[test]
    fn swapped_or_truncated_inputs_are_rejected() {
        let ins = inputs(6);
        let proof = prove(2, &mut new_sim(2), &ins, 2, step);
        let mut other = ins.clone();
        other[4] = vec![0xAA];
        assert_eq!(
            verify_proof(&proof, &mut new_sim(2), &other, step),
            Err(ProofRejection::InputsMismatch)
        );
        assert_eq!(
            verify_proof(&proof, &mut new_sim(2), &ins[..5], step),
            Err(ProofRejection::LengthMismatch {
                claimed: 6,
                actual: 5
            })
        );
        // Framing matters: the same bytes split differently are other inputs.
        assert_ne!(
            input_digest(&[vec![1, 2]]),
            input_digest(&[vec![1], vec![2]])
        );
    }

    #[test]
    fn nondeterministic_step_is_localised_to_the_first_missed_checkpoint() {
        let ins = inputs(9);
        let proof = prove(5, &mut new_sim(5), &ins, 3, step);
        let mut tick = 0;
        let buggy = |s: &mut Sim, f: &[u8]| {
            tick += 1;
            step(s, f);
            if tick == 5 {
                s.acc ^= 1;
            }
        };
        match verify_proof(&proof, &mut new_sim(5), &ins, buggy) {
            Err(ProofRejection::CheckpointMismatch { tick, claimed, .. }) => {
                assert_eq!(tick, 6);
                assert_eq!(claimed, proof.checkpoints[1]);
            }
            other => panic!("expected a checkpoint mismatch, got {other:?}"),
        }
    }

    #[test]
    fn divergence_between_checkpoints_is_caught_by_the_commitment() {
        // One intermediate hash differs while the only checkpoint (tick 4)
        // and the final hash agree: neither stored value can see it, only
        // the chain over every tick can.
        let ins = inputs(4);
        // The pure commitment case: identical checkpoints and final hash,
        // one differing intermediate hash.
        let mut edited = hashes_of(5, &ins);
        edited[1] ^= 1;
        let p = prove(5, &mut new_sim(5), &ins, 4, step);
        let trace = ReplayTrace {
            seed: 5,
            inputs: ins.clone(),
            expected: edited,
        };
        assert_eq!(
            verify_proof_against_trace(&p, &trace),
            Err(ProofRejection::CommitmentMismatch)
        );
    }

    #[test]
    fn edited_claims_are_rejected() {
        let ins = inputs(8);
        let proof = prove(6, &mut new_sim(6), &ins, 2, step);

        let mut forged = proof.clone();
        forged.final_hash ^= 1;
        assert!(matches!(
            verify_proof(&forged, &mut new_sim(6), &ins, step),
            Err(ProofRejection::FinalHashMismatch { .. })
        ));

        let mut forged = proof.clone();
        forged.seed ^= 1;
        assert_eq!(
            verify_proof(&forged, &mut new_sim(6), &ins, step),
            Err(ProofRejection::CommitmentMismatch)
        );

        let mut forged = proof.clone();
        forged.checkpoints.pop();
        assert_eq!(
            verify_proof(&forged, &mut new_sim(6), &ins, step),
            Err(ProofRejection::Malformed)
        );
    }

    #[test]
    fn proof_cross_checks_against_its_iztr_trace() {
        let ins = inputs(10);
        let proof = prove(8, &mut new_sim(8), &ins, 3, step);
        let trace = ReplayTrace {
            seed: 8,
            inputs: ins.clone(),
            expected: hashes_of(8, &ins),
        };
        // Through the persistent IZTR form, as a server would store it.
        let stored = decode_trace(&encode_trace(&trace)).unwrap();
        assert_eq!(verify_proof_against_trace(&proof, &stored), Ok(()));

        let other_seed = ReplayTrace {
            seed: 9,
            ..trace.clone()
        };
        assert_eq!(
            verify_proof_against_trace(&proof, &other_seed),
            Err(ProofRejection::SeedMismatch { proof: 8, trace: 9 })
        );
        let mut short = trace.clone();
        short.expected.pop();
        assert!(matches!(
            verify_proof_against_trace(&proof, &short),
            Err(ProofRejection::LengthMismatch { .. })
        ));
        let mut wrong = trace;
        wrong.expected[2] ^= 1;
        assert_eq!(
            verify_proof_against_trace(&proof, &wrong),
            Err(ProofRejection::CheckpointMismatch {
                tick: 3,
                claimed: proof.checkpoints[0],
                actual: proof.checkpoints[0] ^ 1,
            })
        );
    }

    #[test]
    fn encoding_round_trips_and_has_the_documented_size() {
        for (n, interval) in [(0usize, 0u32), (1, 1), (10, 3), (33, 0), (64, 8)] {
            let ins = inputs(n);
            let proof = prove(n as u64, &mut new_sim(1), &ins, interval, step);
            let bytes = encode_proof(&proof);
            assert_eq!(
                bytes.len(),
                PROOF_HEADER_LEN + PROOF_FIXED_PAYLOAD_LEN + 8 * proof.checkpoints.len()
            );
            assert_eq!(&bytes[..4], b"IZPF");
            assert_eq!(decode_proof(&bytes), Ok(proof));
        }
    }

    #[test]
    fn decoder_rejects_every_truncation_and_bit_flip() {
        let ins = inputs(10);
        let bytes = encode_proof(&prove(1, &mut new_sim(1), &ins, 2, step));
        for cut in 0..bytes.len() {
            assert!(decode_proof(&bytes[..cut]).is_err(), "cut at {cut}");
        }
        for i in 0..bytes.len() {
            let mut b = bytes.clone();
            b[i] ^= 0x01;
            assert!(decode_proof(&b).is_err(), "flip at byte {i}");
        }
        // Trailing data after the declared payload is ignored, like IZTR.
        let mut padded = bytes.clone();
        padded.push(0);
        assert!(decode_proof(&padded).is_ok());
    }

    #[test]
    fn decoder_classifies_errors() {
        assert_eq!(decode_proof(&[]), Err(ProofError::TooShort));
        let trace = encode_trace(&ReplayTrace {
            seed: 1,
            inputs: vec![],
            expected: vec![],
        });
        let err = decode_proof(&trace).unwrap_err();
        assert_eq!(err, ProofError::BadMagic);
        assert!(err.is_recoverable());

        let good = encode_proof(&prove(1, &mut new_sim(1), &inputs(4), 2, step));
        let mut v = good.clone();
        v[4] = 2;
        assert_eq!(decode_proof(&v), Err(ProofError::UnsupportedVersion(2)));
        assert!(!ProofError::UnsupportedVersion(2).is_recoverable());

        // A hostile payload length never allocates or panics.
        let mut huge = good.clone();
        huge[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(decode_proof(&huge), Err(ProofError::TooShort));
    }

    /// Re-frame a payload with a correct checksum, to reach parser checks
    /// that the checksum would otherwise shadow.
    fn reframe(payload: &[u8]) -> Vec<u8> {
        let mut h = Fnv1a::new();
        h.write_bytes(payload);
        let mut out = Vec::new();
        out.extend_from_slice(PROOF_MAGIC);
        out.extend_from_slice(&PROOF_FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&h.finish().to_le_bytes());
        out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn parser_rejects_inconsistent_counts_and_trailing_bytes() {
        let good = encode_proof(&prove(1, &mut new_sim(1), &inputs(4), 2, step));
        let payload = &good[PROOF_HEADER_LEN..];

        let mut bad_count = payload.to_vec();
        // checkpoint_count sits after seed, initial_hash, ticks, digest, interval.
        bad_count[60..68].copy_from_slice(&3u64.to_le_bytes());
        assert_eq!(
            decode_proof(&reframe(&bad_count)),
            Err(ProofError::Malformed)
        );

        let mut hostile = payload.to_vec();
        hostile[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        hostile[56..60].copy_from_slice(&1u32.to_le_bytes());
        hostile[60..68].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(decode_proof(&reframe(&hostile)), Err(ProofError::TooShort));

        let mut trailing = payload.to_vec();
        trailing.push(0);
        assert_eq!(
            decode_proof(&reframe(&trailing)),
            Err(ProofError::TrailingBytes)
        );
        assert_eq!(decode_proof(&reframe(payload)).map(|_| ()), Ok(()));
    }

    #[test]
    fn messages_and_display_are_informative() {
        for e in [
            ProofError::TooShort,
            ProofError::BadMagic,
            ProofError::UnsupportedVersion(7),
            ProofError::ChecksumMismatch,
            ProofError::Malformed,
            ProofError::TrailingBytes,
        ] {
            assert!(!e.message().is_empty());
            assert!(!e.to_string().is_empty());
        }
        assert!(ProofError::UnsupportedVersion(7).to_string().contains('7'));
        let r = ProofRejection::CheckpointMismatch {
            tick: 12,
            claimed: 1,
            actual: 2,
        };
        assert!(r.to_string().contains("tick 12"));
        for r in [
            ProofRejection::Malformed,
            ProofRejection::SeedMismatch { proof: 1, trace: 2 },
            ProofRejection::LengthMismatch {
                claimed: 1,
                actual: 2,
            },
            ProofRejection::InputsMismatch,
            ProofRejection::InitialStateMismatch {
                claimed: 1,
                actual: 2,
            },
            ProofRejection::FinalHashMismatch {
                claimed: 1,
                actual: 2,
            },
            ProofRejection::CommitmentMismatch,
        ] {
            assert!(!r.to_string().is_empty());
        }
    }
}
