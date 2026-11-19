# Replay trace format (`IZTR`, version 1)

This document specifies the on-disk byte format that
[`encode_trace`](./src/replay.rs) writes and [`decode_trace`](./src/replay.rs)
reads. A trace persists a [`ReplayTrace`](./src/replay.rs): the run's seed, one
opaque input frame per tick, and the expected per-tick state hashes that
`record_trace` produced for those inputs.

The source of truth is `src/replay.rs`. The exact bytes for a fixed trace are
pinned by `GOLDEN_TRACE` in that file's tests; a change to the layout without a
`TRACE_FORMAT_VERSION` bump fails `test_encode_trace_matches_golden_encoding`.

## 1. Conventions

- Every integer is **little-endian**, regardless of the host (never
  native-endian; the rule SPEC.md G10 states for world hashes applies to bytes
  that outlive the process too).
- Counts and lengths are `u32`; the seed, the checksum and each hash are `u64`.
- The encoding is a pure function of the `ReplayTrace` value: the same trace
  produces the same bytes on every platform.
- The encoder writes counts with `as u32`. A trace with more than `u32::MAX`
  frames, hashes, or bytes in one frame is outside the format; the encoder
  does not check this.

## 2. Constants

| Name | Value | Visibility |
|---|---|---|
| `TRACE_MAGIC` | ASCII `IZTR` = `49 5A 54 52` | private |
| `TRACE_FORMAT_VERSION` | `1` (`u32`) | `pub` |
| `TRACE_HEADER_LEN` | `20` bytes | private |

The magic is deliberately different from the save-file magic `IZNG`
(`crate::savefile`). The two formats share the same header layout, but a trace
must never load as a game save, or the reverse.

## 3. Layout

A file is a 20-byte header followed by `payload_len` payload bytes.

### 3.1 Header (20 bytes)

| Offset | Size | Field | Contents |
|---|---|---|---|
| 0 | 4 | `magic` | `IZTR` |
| 4 | 4 | `version` | `u32` LE, `TRACE_FORMAT_VERSION` |
| 8 | 8 | `checksum` | `u64` LE, FNV-1a of the payload (§4) |
| 16 | 4 | `payload_len` | `u32` LE, payload byte count |

### 3.2 Payload (`payload_len` bytes, from offset 20)

| Field | Size | Contents |
|---|---|---|
| `seed` | 8 | `u64` LE, `ReplayTrace::seed` |
| `input_count` | 4 | `u32` LE, number of input frames |
| per input frame | 4 + `len` | `len: u32` LE, then `len` raw bytes |
| `hash_count` | 4 | `u32` LE, number of expected hashes |
| per hash | 8 | `u64` LE, in tick order |

Inputs and hashes are stored in tick order. `input_count` and `hash_count` are
independent: a trace whose two columns differ in length round-trips as-is, and
`check_trace` reads the mismatch as a divergence at the first missing tick.

The smallest valid payload is 16 bytes (seed, `input_count = 0`,
`hash_count = 0`), so the smallest valid file is 36 bytes.

```text
┌──────────────────────────────────────────────────────────────────┐
│ magic[4] │ version[4LE] │ checksum[8LE] │ payload_len[4LE] │     │
├──────────────────────────────────────────────────────────────────┤
│ seed[8LE] │ input_count[4LE] │ per input: len[4LE] + bytes │     │
│ hash_count[4LE] │ per hash: hash[8LE]                            │
└──────────────────────────────────────────────────────────────────┘
```

## 4. Checksum

`checksum` is 64-bit FNV-1a (`world_hash::Fnv1a`: offset basis
`0xcbf29ce484222325`, prime `0x100000001b3`, `hash ^= byte; hash *= prime` for
each byte) over **the payload bytes only**: `data[20 .. 20 + payload_len]`.

It does **not** cover:

- the header (magic, version, checksum, `payload_len`);
- any bytes after the payload in the buffer.

A corrupted `version` field is therefore reported as `UnsupportedVersion`, not
as `ChecksumMismatch`. A corrupted `payload_len` normally shows up as
`TooShort` or `ChecksumMismatch`, because it changes which bytes are hashed.

## 5. Decoding

`decode_trace` is total: every input returns `Ok` or a `TraceError`, never a
panic. No allocation is sized by a count before that count is bounded by the
bytes actually present. The checks run in this order, and the first failing
check decides the error:

| # | Check | Error |
|---|---|---|
| 1 | `data.len() < 20` | `TooShort` |
| 2 | `data[0..4] != "IZTR"` | `BadMagic` |
| 3 | `version != TRACE_FORMAT_VERSION` | `UnsupportedVersion(version)` |
| 4 | `payload_len > data.len() - 20` | `TooShort` |
| 5 | FNV-1a(payload) `!= checksum` | `ChecksumMismatch` |
| 6 | `payload.len() < 12` (no room for seed + `input_count`) | `TooShort` |
| 7 | `input_count * 4 >` bytes left (checked before allocating) | `TooShort` |
| 8 | per frame: fewer than 4 bytes left for `len` | `TooShort` |
| 9 | per frame: `len >` bytes left | `TooShort` |
| 10 | fewer than 4 bytes left for `hash_count` | `TooShort` |
| 11 | `hash_count * 8 >` bytes left (checked before allocating) | `TooShort` |
| 12 | bytes left over after the last hash | `TrailingBytes` |

Notes:

- Check 4 is written as a subtraction (`data.len() - 20` cannot underflow after
  check 1) so a hostile `payload_len` cannot overflow the addition on 32-bit
  targets.
- Checks 7 and 11 multiply in `u64`, so a hostile count cannot overflow.
- Steps 6–12 run only on a payload that already passed the checksum.
- Bytes after `20 + payload_len` in the buffer are ignored (the same behavior
  as `savefile::load_bytes`). `TrailingBytes` is about extra bytes **inside**
  the declared payload.

### 5.1 Error classes

`TraceError::is_recoverable()` is `true` only for `BadMagic`: the buffer is not
a trace at all, so a slot can be treated as empty rather than damaged. Every
other error means a damaged trace, or one written by a different format
version. `UnsupportedVersion` carries the stored version so a caller can report
or migrate it.

## 6. Versioning

`TRACE_FORMAT_VERSION` changes whenever the layout changes; the layout never
changes silently. A build reads exactly one version: any other stored value is
rejected with `UnsupportedVersion(stored)` before the checksum or payload is
looked at. The golden test vector (§7) is regenerated only together with a
version bump.

## 7. Test vector

This trace:

```text
seed     = 0x0102030405060708
inputs   = [[0xAA, 0xBB], []]
expected = [0x1111222233334444, 0x5555666677778888]
```

encodes to these 62 bytes (`GOLDEN_TRACE` in `src/replay.rs`):

```text
00: 49 5a 54 52                      magic "IZTR"
04: 01 00 00 00                      version 1
08: 70 ac e4 48 8f 03 88 2f          checksum 0x2f88038f48e4ac70
10: 2a 00 00 00                      payload_len 42
14: 08 07 06 05 04 03 02 01          seed
1c: 02 00 00 00                      input_count 2
20: 02 00 00 00 aa bb                frame 0: len 2, bytes AA BB
26: 00 00 00 00                      frame 1: len 0
2a: 02 00 00 00                      hash_count 2
2e: 44 44 33 33 22 22 11 11          hash 0
36: 88 88 77 77 66 66 55 55          hash 1
```

Offsets are hexadecimal.
