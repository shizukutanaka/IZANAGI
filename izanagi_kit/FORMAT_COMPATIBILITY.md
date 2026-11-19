# Format compatibility policy

Which bytes produced by `izanagi_kit` stay readable, by whom, and for how
long. Each format has a different contract. The determinism guarantee that
the hashes rest on is stated in [`DETERMINISM.md`](../DETERMINISM.md).

## World hashes

Defined in `src/world_hash.rs` (its "Hash stability policy" section is the
normative text).

- **Same code, any machine — guaranteed.** The same state hashes
  identically on every target the crate builds for.
- **Across crate versions — not promised.** Adding a field or changing the
  order in which a `DetHash` impl folds its bytes changes the hash. Such a
  change is treated as breaking for every persisted artifact: re-pin the
  regression hashes and bump the affected format versions in the same
  commit. Callers who persist hashes long-term should record the crate
  version alongside them.

## Save files (`savefile`)

Framing: magic `IZNG`, a caller-defined `u32` version, an FNV-1a checksum
and the payload length, all little-endian. The kit owns the framing; the
game owns the payload schema and its version number. `load_bytes` returns
the stored version so the game can reject or migrate an old save, and
`SaveHeader::is_compatible` checks it against the game's current version.

## Replay traces (`replay::encode_trace` / `decode_trace`)

Framing: magic `IZTR` (deliberately distinct from `IZNG`, so a trace can
never load as a save or the other way round), a `u32` version, an FNV-1a
checksum and the payload length. The payload is the seed, the per-tick
input frames and the expected hashes. All integers are little-endian.
Input frames are opaque bytes: their encoding is the game's half of the
contract, exactly as with save payloads.

### Format version (`TRACE_FORMAT_VERSION`)

Unlike the save-file version, the trace version is owned by the kit. It is
the public constant `replay::TRACE_FORMAT_VERSION` (currently `1`), written
by `encode_trace` at bytes 4..8 of the header.

Compatibility policy when the version changes:

- **The layout never changes silently.** Any change to the trace byte
  layout or payload schema bumps `TRACE_FORMAT_VERSION` in the same
  commit. The golden-encoding test in `src/replay.rs` pins the exact bytes
  of version 1 and may only be regenerated together with such a bump.
- **A build reads exactly one version.** `decode_trace` accepts only
  traces whose version equals its own `TRACE_FORMAT_VERSION`. Any other
  version is rejected with `TraceError::UnsupportedVersion(v)`, which carries
  the stored version. The check runs before the checksum and payload are
  read, so a trace from another version is never half-parsed.
- **"Different version" is distinct from "corrupt".** `UnsupportedVersion`
  is its own error, separate from `BadMagic`, `ChecksumMismatch`, `TooShort`
  and `TrailingBytes`, so a caller can tell a trace written by another build
  from a damaged file and report it, keep it, or migrate it.
- **No automatic migration.** The kit does not upgrade old traces. A trace
  is evidence: rewriting it would change what it proves. A caller who
  needs an old trace on a newer build re-records it from the original seed
  and inputs, or keeps the build that wrote it.
- **A version bump does not by itself make old hashes valid.** Even a
  trace that decodes cleanly only replays when the code it is checked
  against hashes state the same way (see "World hashes" above).

## `.game` content files

The grammar is SPEC.md §9.1 (see [`SPEC.md`](./SPEC.md)), kept in 1:1
agreement with `src/parser.rs` by `tests/parser_matches_its_specification.rs`.
`gamec --fmt` writes the canonical form and `gamec --check` reports files
that are not in it. The format carries no version field: a file is valid
exactly when the current parser and validator accept it, and `gamec` is the
CI gate for that.
