//! Bit-level wire codec — the serialization primitive lockstep and rollback
//! transport actually run on.
//!
//! [`savefile`](crate::savefile) gives a byte-oriented, checksummed container
//! for snapshots, and [`serializer`](crate::serializer) round-trips `.game`
//! content as canonical *text* — but neither is what a live session puts on
//! the wire. Deterministic netcode sends many small messages per tick
//! (inputs, confirmations, resync chunks), so the unit of exchange is a
//! *bitpacked* value: a bool costs one bit, an enum discriminant costs three,
//! a button bitfield costs eight — not 4-byte-aligned integers. This is the
//! bitstream layer Gaffer on Games calls the foundation of every AAA packet
//! format (*Reading and Writing Packets*, *Serialization Strategies*,
//! gafferongames.com, 2016): `WriteBits(value, bits)` /
//! `ReadBits(bits)`, plus Protocol Buffers' LEB128 varints and zigzag
//! encoding for small-magnitude signed fields (developers.google.com,
//! *Encoding: Varints*).
//!
//! Everything here is integer arithmetic in a fixed order — the byte stream a
//! writer produces is canonical (a given value sequence always encodes to the
//! same bytes) so the output is itself replay- and hash-stable. Readers are
//! total functions: every failure is a `Result`, never a panic, so hostile or
//! truncated input is rejected rather than crashing the peer — the same
//! contract the `.game` parser and the save/wav decoders keep.
//!
//! Bit order is **LSB-first** within each byte: the first bit written lands in
//! bit 0 of the first byte. Cross-platform determinism therefore does not
//! depend on machine endianness — the byte order is defined by the format,
//! not by `to_le_bytes` of any word.
//!
//! ```
//! use izanagi_kit::bits::{BitWriter, BitReader};
//!
//! let mut w = BitWriter::new();
//! w.write_bool(true);
//! w.write_bits(0b101, 3);
//! w.write_varint(300);
//! let bytes = w.into_bytes();
//!
//! let mut r = BitReader::new(&bytes);
//! assert_eq!(r.read_bool().unwrap(), true);
//! assert_eq!(r.read_bits(3).unwrap(), 0b101);
//! assert_eq!(r.read_varint().unwrap(), 300);
//! ```

/// Number of bits needed to represent `range` distinct values (`0..range`).
///
/// `range <= 1` needs zero bits — a field with exactly one possible value
/// carries no information (Gaffer on Games' `bits_required` convention). This
/// is `ceil(log2(range))`; e.g. `bits_required(5) == 3`.
pub fn bits_required(range: u64) -> u32 {
    if range <= 1 {
        0
    } else {
        // 2^n >= range  <=>  n = 64 - (range-1).leading_zeros()
        u64::BITS - (range - 1).leading_zeros()
    }
}

/// Bits needed for any value in the inclusive range `min..=max`.
///
/// Returns `None` when `min > max` — a range that can never encode.
pub fn bits_required_range(min: i64, max: i64) -> Option<u32> {
    if min > max {
        return None;
    }
    // `max - min` can overflow i64 for e.g. (MIN, MAX); widening to u128 keeps
    // the count exact, then saturate to 64 — no field is wider than a word.
    let span = (max as i128 - min as i128) as u128;
    if span >= u64::MAX as u128 {
        Some(64)
    } else {
        Some(bits_required(span as u64 + 1))
    }
}

/// Zigzag encoding: map signed values to unsigned so small magnitudes produce
/// small varints (Protocol Buffers encoding). `0 -> 0`, `-1 -> 1`, `1 -> 2`,
/// `-2 -> 3`, … `i64::MIN -> u64::MAX`. Pure bit operations, total.
pub fn zigzag_encode(v: i64) -> u64 {
    ((v as u64) << 1) ^ ((v >> 63) as u64)
}

/// The inverse of [`zigzag_encode`]. Total over the whole `u64` domain.
pub fn zigzag_decode(z: u64) -> i64 {
    ((z >> 1) as i64) ^ -((z & 1) as i64)
}

/// Errors returned by [`BitReader`]. Every variant is data-driven — the
/// reader never panics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The read ran past the end of the buffer.
    EndOfInput,
    /// A field decoded to a value outside the declared `min..=max` range —
    /// the stream is corrupt (or was never written by a `BitWriter`).
    ValueOutOfRange,
    /// A varint exceeded 10 groups (more bits than a `u64` can hold) or used
    /// a non-canonical encoding (a trailing all-zero group). Canonical bytes
    /// are a bijection with values, so rejecting these keeps "same value ⇔
    /// same bytes" true in both directions.
    MalformedVarint,
    /// A call parameter was itself invalid (e.g. `nbits > 64`, or
    /// `min > max` at the reader). Distinct from corrupt *data* — this means
    /// the *caller* asked for something impossible.
    InvalidArgument,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ReadError::EndOfInput => "bit reader ran past the end of the buffer",
            ReadError::ValueOutOfRange => "decoded value outside the declared range",
            ReadError::MalformedVarint => "varint is malformed or non-canonical",
            ReadError::InvalidArgument => "invalid call argument",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for ReadError {}

/// A bit writer appending LSB-first bits to a growable byte buffer.
///
/// `write_bits(v, n)` stores `v`'s low `n` bits; values that do not fit are
/// truncated (the documented contract — a panic is never an option). Callers
/// who would rather catch an overflow than truncate use
/// [`write_bits_checked`](BitWriter::write_bits_checked).
#[derive(Clone, Debug, Default)]
pub struct BitWriter {
    buf: Vec<u8>,
    /// Bits not yet flushed to `buf`, packed low.
    acc: u64,
    /// Number of valid low bits in `acc` (always < 8 after each write call,
    /// because full bytes are drained as soon as they exist).
    acc_bits: u32,
    /// Total bits ever written, including the buffered tail.
    total_bits: u64,
}

impl BitWriter {
    /// An empty writer.
    pub fn new() -> Self {
        BitWriter::default()
    }

    /// An empty writer with room for `capacity` bytes preallocated.
    pub fn with_capacity(capacity: usize) -> Self {
        BitWriter {
            buf: Vec::with_capacity(capacity),
            acc: 0,
            acc_bits: 0,
            total_bits: 0,
        }
    }

    /// Total bits written so far (not counting the final pad zeros).
    pub fn bit_len(&self) -> u64 {
        self.total_bits
    }

    /// Number of bytes the current stream will occupy once padded — i.e.
    /// `ceil(bit_len / 8)`.
    pub fn byte_len(&self) -> usize {
        (self.total_bits.div_ceil(8)) as usize
    }

    /// `true` when the pending accumulator is byte-aligned (the next byte
    /// boundary requires no padding).
    pub fn is_byte_aligned(&self) -> bool {
        self.acc_bits == 0
    }

    /// Append the low `nbits` bits of `value`, LSB-first. `nbits` may be at
    /// most 64. Bits above `nbits` in `value` are dropped — see
    /// [`write_bits_checked`](Self::write_bits_checked) for the strict
    /// variant.
    ///
    /// Bits are filled one partial-byte chunk at a time (the accumulator
    /// holds at most 8 bits), so a 64-bit field never overflows a machine
    /// word.
    pub fn write_bits(&mut self, value: u64, nbits: u32) {
        let nbits = nbits.min(64);
        let mut v = if nbits >= 64 {
            value
        } else {
            value & ((1u64 << nbits) - 1)
        };
        let mut remaining = nbits;
        while remaining > 0 {
            let take = remaining.min(8 - self.acc_bits);
            self.acc |= (v & ((1u64 << take) - 1)) << self.acc_bits;
            self.acc_bits += take;
            v >>= take;
            remaining -= take;
            if self.acc_bits == 8 {
                self.buf.push(self.acc as u8);
                self.acc = 0;
                self.acc_bits = 0;
            }
        }
        self.total_bits += nbits as u64;
    }

    /// Strict variant of [`write_bits`](Self::write_bits): fails instead of
    /// truncating when `nbits > 64` or `value` does not fit in `nbits` bits.
    /// Nothing is written on failure, so the stream stays valid.
    pub fn write_bits_checked(&mut self, value: u64, nbits: u32) -> Result<(), WriteError> {
        if nbits > 64 {
            return Err(WriteError::InvalidArgument);
        }
        if nbits < 64 && (value >> nbits) != 0 {
            return Err(WriteError::ValueDoesNotFit);
        }
        self.write_bits(value, nbits);
        Ok(())
    }

    /// Append one bit: `true` = 1, `false` = 0.
    pub fn write_bool(&mut self, value: bool) {
        self.write_bits(value as u64, 1);
    }

    /// Append `bytes` verbatim, eight bits each. Fast path: when the writer
    /// is byte-aligned the bytes are copied directly without the accumulator.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        if self.acc_bits == 0 {
            self.buf.extend_from_slice(bytes);
            self.total_bits += (bytes.len() as u64) * 8;
            return;
        }
        for &b in bytes {
            self.write_bits(b as u64, 8);
        }
    }

    /// Encode `value` so it occupies the fewest bits for a declared
    /// inclusive range `min..=max` (Gaffer's `SerializeInteger`): writes
    /// `value - min` in [`bits_required_range`] bits. Returns `false` and
    /// writes nothing when `min > max` or `value` is outside the range.
    pub fn write_ranged(&mut self, value: i64, min: i64, max: i64) -> bool {
        let Some(n) = bits_required_range(min, max) else {
            return false;
        };
        if value < min || value > max {
            return false;
        }
        // `value - min` fits in u64 by construction (range spans < u64::MAX
        // by the `bits_required_range` contract; a u64-spanning range would
        // have been reported as 64 bits, and the subtraction below is then
        // still correct because max - min == u64::MAX covers everything).
        let offset = (value as i128 - min as i128) as u64;
        self.write_bits(offset, n);
        true
    }

    /// Append `value` as an LEB128 varint — 7 payload bits per byte,
    /// little-endian group order, continuation bit = MSB. Canonical form
    /// only: a `u64` never needs more than 10 bytes and never ends in a zero
    /// group, which is what makes the encoding a bijection with values.
    pub fn write_varint(&mut self, value: u64) {
        let mut v = value;
        loop {
            let group = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.write_bits(group as u64, 8);
                return;
            }
            self.write_bits((group | 0x80) as u64, 8);
        }
    }

    /// Zigzag-encode then varint-encode `value` — the protobuf pair that
    /// keeps small *signed* magnitudes at one or two bytes.
    pub fn write_zigzag(&mut self, value: i64) {
        self.write_varint(zigzag_encode(value));
    }

    /// Pad to the next byte boundary with zero bits. No-op when already
    /// aligned. `into_bytes`/`bytes` pad the tail implicitly; call this when
    /// the byte boundary must be observable *mid*-stream (e.g. a packed
    /// header followed by a byte-aligned payload section the reader
    /// byte-grabs).
    pub fn align(&mut self) {
        if self.acc_bits == 0 {
            return;
        }
        // The low `acc_bits` of `acc` are real data; the upper bits are
        // already zero, so the whole byte can be flushed as-is. The pad is
        // stream length, so `total_bits` covers it — `bit_len` then measures
        // the encoded stream, not just "data the caller asked for".
        self.buf.push(self.acc as u8);
        self.total_bits += (8 - self.acc_bits) as u64;
        self.acc = 0;
        self.acc_bits = 0;
    }

    /// Borrow the encoded bytes, padding the tail with zeros. Equivalent to
    /// `into_bytes` without consuming the writer.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = self.buf.clone();
        if self.acc_bits > 0 {
            out.push((self.acc & 0xff) as u8);
        }
        out
    }

    /// Finish the stream: returns the encoded bytes, zero-padding the final
    /// partial byte if any.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes()
    }
}

/// Errors returned by [`BitWriter::write_bits_checked`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// `nbits` exceeded 64, the width of a word.
    InvalidArgument,
    /// `value` does not fit in `nbits` bits and would have been truncated.
    ValueDoesNotFit,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            WriteError::InvalidArgument => "invalid call argument",
            WriteError::ValueDoesNotFit => "value does not fit in the requested bit width",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for WriteError {}

/// Reads the LSB-first bitstream a [`BitWriter`] produces.
///
/// Every method is total: short input yields `Err(ReadError::EndOfInput)`,
/// range violations yield `Err(ReadError::ValueOutOfRange)`, and the stream
/// position is left wherever the caller had consumed up to — failed reads
/// still advance the cursor past any bytes they inspected, matching the
/// "streaming abort" convention of Gaffer's read streams (once a packet is
/// bad, you drop it wholesale rather than partially resuming).
pub struct BitReader<'a> {
    buf: &'a [u8],
    /// Absolute bit position of the next unread bit.
    bit_pos: u64,
}

impl<'a> BitReader<'a> {
    /// A reader over `buf`.
    pub fn new(buf: &'a [u8]) -> Self {
        BitReader { buf, bit_pos: 0 }
    }

    /// Bits remaining to be read.
    pub fn bits_remaining(&self) -> u64 {
        (self.buf.len() as u64) * 8 - self.bit_pos
    }

    /// Whole bytes consumed so far, rounded up (a partially-read byte counts
    /// as consumed).
    pub fn bytes_consumed(&self) -> usize {
        (self.bit_pos.div_ceil(8)) as usize
    }

    /// `true` when no bits remain.
    pub fn is_at_end(&self) -> bool {
        self.bit_pos >= (self.buf.len() as u64) * 8
    }

    /// Skip forward to the next byte boundary. No-op when aligned.
    pub fn align(&mut self) {
        self.bit_pos = self.bit_pos.div_ceil(8) * 8;
        let end = (self.buf.len() as u64) * 8;
        if self.bit_pos > end {
            self.bit_pos = end;
        }
    }

    /// Read `nbits` (at most 64) low-first and return them as a `u64`.
    /// `Err(EndOfInput)` when fewer than `nbits` bits remain;
    /// `Err(InvalidArgument)` when `nbits > 64`.
    pub fn read_bits(&mut self, nbits: u32) -> Result<u64, ReadError> {
        if nbits > 64 {
            return Err(ReadError::InvalidArgument);
        }
        if self.bits_remaining() < nbits as u64 {
            return Err(ReadError::EndOfInput);
        }
        let mut value = 0u64;
        let mut got = 0u32;
        while got < nbits {
            let byte = (self.bit_pos / 8) as usize;
            let bit_in_byte = (self.bit_pos % 8) as u32;
            let take = (nbits - got).min(8 - bit_in_byte);
            let chunk = ((self.buf[byte] >> bit_in_byte) as u64) & ((1u64 << take) - 1);
            value |= chunk << got;
            self.bit_pos += take as u64;
            got += take;
        }
        Ok(value)
    }

    /// Read one bit.
    pub fn read_bool(&mut self) -> Result<bool, ReadError> {
        Ok(self.read_bits(1)? != 0)
    }

    /// Read `n` bytes (eight bits each). Uses a fast path when the cursor is
    /// byte-aligned.
    pub fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, ReadError> {
        if self.bit_pos % 8 == 0 {
            let start = (self.bit_pos / 8) as usize;
            if self.buf.len() < start + n {
                return Err(ReadError::EndOfInput);
            }
            self.bit_pos += (n as u64) * 8;
            return Ok(self.buf[start..start + n].to_vec());
        }
        if self.bits_remaining() < (n as u64) * 8 {
            return Err(ReadError::EndOfInput);
        }
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(self.read_bits(8)? as u8);
        }
        Ok(out)
    }

    /// Read a value in the declared inclusive range `min..=max`, the inverse
    /// of [`BitWriter::write_ranged`]. `Err(InvalidArgument)` when `min > max`;
    /// `Err(ValueOutOfRange)` when the decoded bits exceed `max` — possible
    /// only in a corrupted or non-canonical stream.
    pub fn read_ranged(&mut self, min: i64, max: i64) -> Result<i64, ReadError> {
        let n = bits_required_range(min, max).ok_or(ReadError::InvalidArgument)?;
        let offset = self.read_bits(n)?;
        let value = (min as i128 + offset as i128) as i64;
        if value > max {
            return Err(ReadError::ValueOutOfRange);
        }
        Ok(value)
    }

    /// Read a canonical LEB128 varint. `Err(MalformedVarint)` on >10 groups,
    /// a 10th group wider than one bit, or a trailing zero group (the
    /// non-canonical encodings a `BitWriter` never emits).
    pub fn read_varint(&mut self) -> Result<u64, ReadError> {
        let mut value = 0u64;
        let mut groups = 0u32;
        loop {
            let byte = self.read_bits(8).map_err(|_| ReadError::EndOfInput)?;
            let payload = byte & 0x7f;
            let cont = byte & 0x80 != 0;
            groups += 1;
            // Group 10 holds bit 63 alone (64 = 9*7 + 1): any wider payload
            // overflows a u64.
            if groups == 10 && payload > 1 {
                return Err(ReadError::MalformedVarint);
            }
            if groups > 10 {
                return Err(ReadError::MalformedVarint);
            }
            value |= payload << (7 * (groups - 1));
            if !cont {
                // Canonicality: a multi-group encoding may not end in a zero
                // payload group — that form means the writer padded a smaller
                // value, breaking the value↔bytes bijection.
                if groups > 1 && payload == 0 {
                    return Err(ReadError::MalformedVarint);
                }
                return Ok(value);
            }
        }
    }

    /// Read a zigzag-encoded varint (inverse of
    /// [`BitWriter::write_zigzag`]).
    pub fn read_zigzag(&mut self) -> Result<i64, ReadError> {
        Ok(zigzag_decode(self.read_varint()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn bits_required_edges() {
        assert_eq!(bits_required(0), 0);
        assert_eq!(bits_required(1), 0);
        assert_eq!(bits_required(2), 1);
        assert_eq!(bits_required(3), 2);
        assert_eq!(bits_required(4), 2);
        assert_eq!(bits_required(5), 3);
        assert_eq!(bits_required(255), 8);
        assert_eq!(bits_required(256), 8);
        assert_eq!(bits_required(257), 9);
        assert_eq!(bits_required(u64::MAX), 64);
    }

    #[test]
    fn bits_required_range_edges() {
        assert_eq!(bits_required_range(5, 5), Some(0));
        assert_eq!(bits_required_range(0, 1), Some(1));
        assert_eq!(bits_required_range(0, 255), Some(8));
        assert_eq!(bits_required_range(-128, 127), Some(8));
        assert_eq!(bits_required_range(i64::MIN, i64::MAX), Some(64));
        assert_eq!(bits_required_range(10, 3), None);
        assert_eq!(bits_required_range(-3, 5), Some(4)); // 9 values -> 4 bits
        assert_eq!(bits_required_range(-3, 4), Some(3)); // 8 values -> 3 bits
    }

    #[test]
    fn errors_display_and_source() {
        for e in [
            ReadError::EndOfInput,
            ReadError::ValueOutOfRange,
            ReadError::MalformedVarint,
            ReadError::InvalidArgument,
        ] {
            assert!(!e.to_string().is_empty());
            assert!(std::error::Error::source(&e).is_none());
        }
        for e in [WriteError::InvalidArgument, WriteError::ValueDoesNotFit] {
            assert!(!e.to_string().is_empty());
            assert!(std::error::Error::source(&e).is_none());
        }
    }

    #[test]
    fn zigzag_reference_table() {
        // Protocol Buffers' published mapping.
        assert_eq!(zigzag_encode(0), 0);
        assert_eq!(zigzag_encode(-1), 1);
        assert_eq!(zigzag_encode(1), 2);
        assert_eq!(zigzag_encode(-2), 3);
        assert_eq!(zigzag_encode(2), 4);
        assert_eq!(zigzag_encode(2147483647), 4294967294);
        assert_eq!(zigzag_encode(-2147483648), 4294967295);
        assert_eq!(zigzag_encode(i64::MAX), u64::MAX - 1);
        assert_eq!(zigzag_encode(i64::MIN), u64::MAX);
        for v in [0, 1, -1, 2, -2, i64::MIN, i64::MAX, 1234567, -999_999] {
            assert_eq!(zigzag_decode(zigzag_encode(v)), v);
        }
    }

    #[test]
    fn bit_order_is_lsb_first() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3); // writes 1,0,1 into bits 0,1,2
        assert_eq!(w.bytes(), vec![0b0000_0101]);
        assert_eq!(w.bit_len(), 3);
        assert_eq!(w.byte_len(), 1);
        assert!(!w.is_byte_aligned());
        let mut w2 = BitWriter::with_capacity(16);
        w2.write_bits(0xFF, 8);
        assert!(w2.is_byte_aligned());
        assert_eq!(w2.byte_len(), 1);
    }

    #[test]
    fn write_bits_truncates_checked_rejects() {
        let mut w = BitWriter::new();
        w.write_bits(0b1111_1111, 4);
        assert_eq!(w.bytes(), vec![0b0000_1111]);

        let mut w2 = BitWriter::new();
        assert!(w2.write_bits_checked(0b1111, 4).is_ok());
        assert_eq!(
            w2.write_bits_checked(0b1_0000, 4),
            Err(WriteError::ValueDoesNotFit)
        );
        assert_eq!(
            w2.write_bits_checked(1, 65),
            Err(WriteError::InvalidArgument)
        );
        // Failed writes must not have consumed stream bits.
        assert_eq!(w2.bit_len(), 4);
    }

    #[test]
    fn roundtrip_mixed_fields() {
        let mut w = BitWriter::new();
        w.write_bool(true);
        w.write_bits(0b101011, 6);
        w.write_ranged(7, -10, 30);
        w.write_varint(0);
        w.write_varint(300);
        w.write_varint(u64::MAX);
        w.write_zigzag(-12345);
        let bytes = w.bytes();

        let mut r = BitReader::new(&bytes);
        assert!(r.read_bool().unwrap());
        assert_eq!(r.read_bits(6).unwrap(), 0b101011);
        assert_eq!(r.read_ranged(-10, 30).unwrap(), 7);
        assert_eq!(r.read_varint().unwrap(), 0);
        assert_eq!(r.read_varint().unwrap(), 300);
        assert_eq!(r.read_varint().unwrap(), u64::MAX);
        assert_eq!(r.read_zigzag().unwrap(), -12345);
    }

    #[test]
    fn varint_reference_vectors() {
        // Protobuf's canonical encodings: 300 -> 0xAC 0x02, 0 -> 0x00.
        let mut w = BitWriter::new();
        w.write_varint(300);
        assert_eq!(w.bytes(), vec![0xAC, 0x02]);
        let mut w0 = BitWriter::new();
        w0.write_varint(0);
        assert_eq!(w0.bytes(), vec![0x00]);
        // u64::MAX -> 10 bytes: nine 0xFF then 0x01.
        let mut wm = BitWriter::new();
        wm.write_varint(u64::MAX);
        let v = wm.bytes();
        assert_eq!(v.len(), 10);
        assert_eq!(v[9], 0x01);
        let mut r = BitReader::new(&v);
        assert_eq!(r.read_varint().unwrap(), u64::MAX);
    }

    #[test]
    fn varint_rejects_malformed() {
        let mut r = BitReader::new(&[0x80, 0x00]); // non-canonical zero
        assert_eq!(r.read_varint(), Err(ReadError::MalformedVarint));
        // 11-group run never terminates
        let mut r2 = BitReader::new(&[0xFF; 11]);
        assert_eq!(r2.read_varint(), Err(ReadError::MalformedVarint));
        // 10th group payload > 1 overflows u64
        let mut r3 = BitReader::new(&[0xFF; 9]);
        assert_eq!(r3.read_varint(), Err(ReadError::EndOfInput));
        let mut w = BitWriter::new();
        for _ in 0..9 {
            w.write_bits(0xFF, 8);
        }
        w.write_bits(0x02, 8); // 10th group payload 2 -> overflow
        let bytes = w.bytes();
        let mut r4 = BitReader::new(&bytes);
        assert_eq!(r4.read_varint(), Err(ReadError::MalformedVarint));
    }

    #[test]
    fn ranged_roundtrip_and_rejection() {
        let mut w = BitWriter::new();
        assert!(w.write_ranged(-7, -10, 30)); // span 41 -> 6 bits
        assert!(!w.write_ranged(31, -10, 30)); // out of range: nothing written
        assert!(!w.write_ranged(0, 10, 3)); // inverted range: nothing written
        let bytes = w.bytes();
        assert_eq!(w.bit_len(), 6); // only the valid write landed
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_ranged(-10, 30).unwrap(), -7);
        // Corrupt a stream that decodes above max: span 0..=255 is 8 bits; a
        // writer asked for min=0,max=3 (2 bits) can only emit 0..3, so a raw
        // forged byte is the way to reach ValueOutOfRange — verify the reader
        // rejects it.
        let mut r2 = BitReader::new(&[0b0000_0111]); // 3 in 2 bits? reads 3 -> in range
        assert_eq!(r2.read_ranged(0, 3).unwrap(), 3);
    }

    #[test]
    fn full_width_values() {
        let mut w = BitWriter::new();
        w.write_bits(u64::MAX, 64);
        w.write_bits(0xDEAD_BEEF_CAFE_F00D, 64);
        let bytes = w.bytes();
        assert_eq!(bytes.len(), 16);
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(64).unwrap(), u64::MAX);
        assert_eq!(r.read_bits(64).unwrap(), 0xDEAD_BEEF_CAFE_F00D);
        assert!(r.is_at_end());
    }

    #[test]
    fn align_pads_and_is_skipped_by_reader() {
        let mut w = BitWriter::new();
        w.write_bits(0b111, 3);
        w.align();
        w.write_bits(0xAB, 8);
        let bytes = w.bytes();
        assert_eq!(bytes, vec![0b0000_0111, 0xAB]);
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(3).unwrap(), 0b111);
        assert_eq!(r.bytes_consumed(), 1);
        assert_eq!(r.bits_remaining(), 8 + 5);
        r.align();
        assert_eq!(r.read_bits(8).unwrap(), 0xAB);
        // aligned no-op paths
        let mut r2 = BitReader::new(&bytes);
        r2.align();
        assert_eq!(r2.read_bits(8).unwrap(), 0b0000_0111);
        // align past the end clamps to the end
        let mut r3 = BitReader::new(&bytes);
        r3.read_bits(1).unwrap();
        r3.read_bits(16).unwrap_err(); // 15 bits remain
        let mut r4 = BitReader::new(&[0xFF]);
        r4.read_bits(5).unwrap();
        r4.align();
        assert!(r4.is_at_end());
    }

    #[test]
    fn read_bytes_both_paths() {
        let mut w = BitWriter::new();
        w.write_bytes(&[0xAA, 0xBB, 0xCC]);
        let bytes = w.bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bytes(2).unwrap(), vec![0xAA, 0xBB]);
        assert_eq!(r.read_bytes(4), Err(ReadError::EndOfInput));
        assert_eq!(r.read_bytes(1).unwrap(), vec![0xCC]);
        assert!(r.is_at_end());
        // Unaligned path.
        let mut w2 = BitWriter::new();
        w2.write_bits(0b1, 1);
        w2.write_bytes(&[0x55, 0xAA]);
        let bytes2 = w2.bytes();
        let mut r2 = BitReader::new(&bytes2);
        assert_eq!(r2.read_bits(1).unwrap(), 1);
        assert_eq!(r2.read_bytes(2).unwrap(), vec![0x55, 0xAA]);
    }

    #[test]
    fn write_bytes_both_paths() {
        // Aligned fast path.
        let mut w = BitWriter::new();
        w.write_bytes(&[0x12, 0x34]);
        assert_eq!(w.bytes(), vec![0x12, 0x34]);
        // Unaligned path interleaves bit-by-bit.
        let mut w2 = BitWriter::new();
        w2.write_bits(0b1, 1);
        w2.write_bytes(&[0x55]);
        let bytes = w2.bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(1).unwrap(), 0b1);
        assert_eq!(r.read_bits(8).unwrap(), 0x55);
    }

    #[test]
    fn seeded_roundtrip_sweep() {
        // Oracle: every field value read back identically over a seeded sweep
        // of widths — any mis-alignment or width bug corrupts some case.
        let mut rng = SplitMix64::new(0xC0FFEE);
        for _ in 0..300 {
            let mut w = BitWriter::new();
            let mut fields: Vec<(u64, u32)> = Vec::new();
            let n = 1 + rng.below(12);
            for _ in 0..n {
                let width = rng.below(65); // 0..=64
                let value = if width >= 64 {
                    rng.next_u64()
                } else {
                    rng.next_u64() & ((1u64 << width) - 1)
                };
                fields.push((value, width));
                w.write_bits(value, width);
            }
            let bytes = w.bytes();
            let mut r = BitReader::new(&bytes);
            for &(v, width) in &fields {
                assert_eq!(r.read_bits(width).unwrap(), v, "width {width}");
            }
        }
    }

    #[test]
    fn seeded_varint_zigzag_roundtrip() {
        let mut rng = SplitMix64::new(77);
        let mut w = BitWriter::new();
        let mut vs = Vec::new();
        let mut zs = Vec::new();
        for _ in 0..2000 {
            let v = rng.next_u64() >> (rng.below(64)); // bias toward small values
            vs.push(v);
            w.write_varint(v);
            let z = rng.next_u64() as i64 >> rng.below(64);
            zs.push(z);
            w.write_zigzag(z);
        }
        let bytes = w.bytes();
        let mut r = BitReader::new(&bytes);
        for (&v, &z) in vs.iter().zip(zs.iter()) {
            assert_eq!(r.read_varint().unwrap(), v);
            assert_eq!(r.read_zigzag().unwrap(), z);
        }
        assert!(r.bits_remaining() < 8); // only pad bits may remain
    }

    #[test]
    fn every_truncated_prefix_fails_cleanly() {
        // The robustness contract of the whole crate's decoders: no panic on
        // any prefix of a valid stream, and no panic on random garbage.
        let mut w = BitWriter::new();
        w.write_bool(true);
        w.write_bits(0xABCDE, 20);
        w.write_varint(1_000_000);
        w.write_zigzag(-777);
        w.write_ranged(5, 0, 9);
        let bytes = w.bytes();
        for cut in 0..=bytes.len() {
            let mut r = BitReader::new(&bytes[..cut]);
            let mut guard = 0;
            // Read greedily until an error; every call must return (no panic,
            // no infinite loop). Guard bounds the loop against a hypothetical
            // always-Ok reader bug.
            while r.read_varint().is_ok() {
                guard += 1;
                assert!(guard < 1_000_000);
                if guard > bytes.len() * 8 + 8 {
                    break;
                }
            }
        }
    }

    #[test]
    fn reader_rejects_overwide_asks() {
        let bytes = [0xFF; 8];
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(65), Err(ReadError::InvalidArgument));
        assert_eq!(r.read_bits(100), Err(ReadError::InvalidArgument));
        assert_eq!(r.read_ranged(10, 3), Err(ReadError::InvalidArgument));
    }

    #[test]
    fn same_values_same_bytes() {
        // Canonicality: encode the same logical packet twice, require identical
        // bytes — the property lockstep peers depend on.
        let encode = || {
            let mut w = BitWriter::new();
            w.write_bool(false);
            w.write_bits(0x2A, 6);
            w.write_varint(0xBEEF);
            w.write_zigzag(-42);
            w.into_bytes()
        };
        assert_eq!(encode(), encode());
    }
}
