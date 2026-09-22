//! Write-ahead log — an append-only byte journal with per-record
//! checksums, in the tradition of SQLite's WAL and Aries-style
//! write-ahead logging: mutating ops first get recorded here, so a
//! crash leaves at most a torn tail to discard.
//!
//! Wire format per record (all little-endian):
//!
//! ```text
//! [kind:u8][len:u32][crc:u64][payload × len]
//! ```
//!
//! `crc` is FNV-1a over `"walv1" ‖ kind ‖ len ‖ payload` — domain-
//! separated so a log record can never collide with an application
//! hash. `kind` is `0` for a data record and `1` for a checkpoint
//! marker (payload-empty); anything else is corruption.
//!
//! Replay is **torn-tail tolerant**: decoding stops at the first
//! truncated or checksum-mismatched record and reports `clean = false`
//! plus the byte offset it stopped at — records after a bad one are
//! untrusted by definition (a crash during write can tear a record,
//! and a flipped byte invalidates everything after it via offset
//! desync). A `Wal` in memory is crash-free, so `decode(bytes)` on a
//! `Wal`'s output always reports `clean`.
//!
//! ```
//! use izanagi_kit::wal::{decode, EntryKind, Wal};
//! let mut wal = Wal::new();
//! wal.append(b"set x = 1");
//! wal.checkpoint(); // entries before this are durable
//! wal.append(b"set x = 2");
//! let d = decode(wal.bytes());
//! assert!(d.clean);
//! assert_eq!(d.entries.len(), 3);
//! assert_eq!(d.entries[1].kind, EntryKind::Checkpoint);
//! ```

use crate::world_hash::Fnv1a;

const KIND_DATA: u8 = 0;
const KIND_CHECKPOINT: u8 = 1;
const HEADER: usize = 13; // kind(1) + len(4) + crc(8)

/// What a decoded record is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// An application record (`Wal::append` payload).
    Data,
    /// A durability boundary written by `Wal::checkpoint` —
    /// everything before it is considered applied.
    Checkpoint,
}

/// One decoded record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalEntry {
    /// Record kind.
    pub kind: EntryKind,
    /// Payload bytes — always empty for `Checkpoint`.
    pub payload: Vec<u8>,
}

/// The result of decoding a byte stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    /// Records decoded before stopping — a valid prefix.
    pub entries: Vec<WalEntry>,
    /// `true` iff every byte was consumed (no torn tail).
    pub clean: bool,
    /// Byte offset where decoding stopped (== `bytes.len()` when
    /// `clean`). Useful for truncating the file to the good prefix.
    pub stopped_at: usize,
}

fn checksum(kind: u8, payload: &[u8]) -> u64 {
    let mut h = Fnv1a::new();
    h.write_bytes(b"walv1");
    h.write_u32(kind as u32);
    h.write_u32(payload.len() as u32);
    h.write_bytes(payload);
    h.finish()
}

/// An in-memory write-ahead log — `append` frames records,
/// `bytes` is the wire form, `checkpoint` marks durability.
#[derive(Clone, Debug, Default)]
pub struct Wal {
    buf: Vec<u8>,
}

impl Wal {
    /// An empty log.
    pub fn new() -> Self {
        Wal { buf: Vec::new() }
    }

    /// Frame and append a data record. `O(len)` — appends only.
    pub fn append(&mut self, payload: &[u8]) {
        self.push(KIND_DATA, payload);
    }

    /// Append a checkpoint marker: readers treat everything before
    /// it as already applied to the base state.
    pub fn checkpoint(&mut self) {
        self.push(KIND_CHECKPOINT, &[]);
    }

    fn push(&mut self, kind: u8, payload: &[u8]) {
        self.buf.push(kind);
        self.buf
            .extend_from_slice(&(payload.len() as u32).to_le_bytes());
        self.buf
            .extend_from_slice(&checksum(kind, payload).to_le_bytes());
        self.buf.extend_from_slice(payload);
    }

    /// The encoded log bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Encoded length in bytes.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Truncate the log to `valid_len` bytes — e.g. the
    /// [`Decoded::stopped_at`] offset of a torn tail.
    pub fn truncate(&mut self, valid_len: usize) {
        self.buf.truncate(valid_len);
    }
}

/// Decode a byte stream into entries. Torn-tail tolerant: stops at
/// the first incomplete or checksum-mismatched record and reports
/// `clean = false`. Never fails outright — even garbage input yields
/// an empty prefix.
pub fn decode(bytes: &[u8]) -> Decoded {
    let mut entries = Vec::new();
    let mut at = 0usize;
    loop {
        if at + HEADER > bytes.len() {
            return Decoded {
                entries,
                clean: at == bytes.len(),
                stopped_at: at,
            };
        }
        let kind = bytes[at];
        if kind != KIND_DATA && kind != KIND_CHECKPOINT {
            return Decoded {
                entries,
                clean: false,
                stopped_at: at,
            };
        }
        let len = u32::from_le_bytes([bytes[at + 1], bytes[at + 2], bytes[at + 3], bytes[at + 4]])
            as usize;
        let crc = u64::from_le_bytes(bytes[at + 5..at + 13].try_into().unwrap_or_default());
        let start = at + HEADER;
        let Some(end) = start.checked_add(len).filter(|&e| e <= bytes.len()) else {
            return Decoded {
                entries,
                clean: false,
                stopped_at: at,
            };
        };
        let payload = &bytes[start..end];
        if checksum(kind, payload) != crc {
            return Decoded {
                entries,
                clean: false,
                stopped_at: at,
            };
        }
        let entry_kind = if kind == KIND_CHECKPOINT {
            EntryKind::Checkpoint
        } else {
            EntryKind::Data
        };
        entries.push(WalEntry {
            kind: entry_kind,
            payload: payload.to_vec(),
        });
        at = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_clean() {
        let mut wal = Wal::new();
        for i in 0..20u32 {
            wal.append(format!("op {i}").as_bytes());
            if i % 5 == 0 {
                wal.checkpoint();
            }
        }
        let d = decode(wal.bytes());
        assert!(d.clean);
        assert_eq!(d.stopped_at, wal.len());
        let data: Vec<&[u8]> = d
            .entries
            .iter()
            .filter(|e| e.kind == EntryKind::Data)
            .map(|e| e.payload.as_slice())
            .collect();
        assert_eq!(data.len(), 20);
        assert_eq!(data[0], b"op 0");
        let cks = d
            .entries
            .iter()
            .filter(|e| e.kind == EntryKind::Checkpoint)
            .count();
        assert_eq!(cks, 4);
    }

    #[test]
    fn torn_tail_stops_cleanly_at_every_cut() {
        let mut wal = Wal::new();
        wal.append(b"alpha");
        wal.append(b"beta");
        wal.checkpoint();
        let full = wal.bytes().to_vec();
        for cut in 0..full.len() {
            let d = decode(&full[..cut]);
            // `clean` iff the cut sits exactly on a record boundary.
            assert_eq!(d.clean, d.stopped_at == cut, "cut {cut}");
            assert!(d.stopped_at <= cut);
            // Whatever decoded is a valid prefix.
            let mut expect = Wal::new();
            for e in &d.entries {
                match e.kind {
                    EntryKind::Data => expect.append(&e.payload),
                    EntryKind::Checkpoint => expect.checkpoint(),
                }
            }
            assert_eq!(&full[..d.stopped_at], expect.bytes());
        }
    }

    #[test]
    fn bit_flip_halts_replay() {
        let mut wal = Wal::new();
        wal.append(b"first");
        wal.append(b"second");
        wal.append(b"third");
        let mut bad = wal.bytes().to_vec();
        // Flip a payload byte of the middle record — record 1 is
        // HEADER+5 bytes, so record 2's payload starts at 18+HEADER.
        bad[HEADER + 5 + HEADER + 2] ^= 0xFF;
        let d = decode(&bad);
        assert!(!d.clean);
        assert_eq!(d.entries.len(), 1);
        assert_eq!(d.entries[0].payload, b"first");
        // Flip a header byte (kind) of the first record.
        bad.clone_from_slice(wal.bytes());
        bad[0] = 9;
        let d = decode(&bad);
        assert!(!d.clean);
        assert!(d.entries.is_empty());
        assert_eq!(d.stopped_at, 0);
    }

    #[test]
    fn truncate_removes_torn_tail() {
        let mut wal = Wal::new();
        wal.append(b"keep");
        let good_len = wal.len();
        wal.append(b"torn");
        let full = wal.bytes().to_vec();
        let cut = good_len + 5; // inside the second record
        let d = decode(&full[..cut]);
        assert!(!d.clean);
        assert_eq!(d.stopped_at, good_len);
        wal.truncate(d.stopped_at);
        let d = decode(wal.bytes());
        assert!(d.clean);
        assert_eq!(d.entries.len(), 1);
        assert_eq!(d.entries[0].payload, b"keep");
    }

    #[test]
    fn empty_and_garbage_inputs() {
        let d = decode(b"");
        assert!(d.clean);
        assert!(d.entries.is_empty());
        let d = decode(&[0xAB; 7]);
        assert!(!d.clean);
        assert!(d.entries.is_empty());
        let wal = Wal::new();
        assert!(wal.is_empty());
        assert_eq!(wal.len(), 0);
    }
}
