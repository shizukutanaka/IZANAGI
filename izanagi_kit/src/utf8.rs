//! Strict UTF-8 decoding via Björn Höhrmann's 12-state DFA —
//! table-driven byte machine that rejects every malformed
//! sequence (overlong encodings, surrogates, out-of-range
//! codepoints, stray continuations, truncation) with zero
//! branches on the payload itself. Complements [`crate::bits`]
//! (wire codec): UTF-8 is the canonical interchange text and a
//! deterministic sim must not let platform decoders define
//! what "valid" means.
//!
//! The DFA: byte → class via `UTF8D[byte]`, then
//! `state = UTF8D[256 + state + class]`; `ACCEPT`/`REJECT` are
//! the only two absorbing results that matter. The codepoint
//! accumulator drops the lead byte's tag bits
//! (`0xff >> class`) and shifts in 6 bits per continuation.
//!
//! ```
//! use izanagi_kit::utf8;
//! assert_eq!(utf8::decode("héllo".as_bytes()), Some(vec![104, 233, 108, 108, 111]));
//! assert!(!utf8::validate(&[0x61, 0x80, 0x62])); // stray continuation
//! assert_eq!(utf8::decode_lossy(&[0x61, 0xc0, 0xaf, 0x62]),
//!            vec![0x61, 0xfffd, 0xfffd, 0x62]);
//! assert_eq!(utf8::encode(0x1f600), Some(vec![0xf0, 0x9f, 0x98, 0x80]));
//! ```
//!
//! References: Höhrmann (2010) "Decoding UTF-8" DFA table —
//! public domain; Unicode §3.9 best practice for the
//! U+FFFD sub-sequence policy in `decode_lossy`.

/// DFA accept state.
const ACCEPT: u32 = 0;
/// DFA reject state.
const REJECT: u32 = 12;
/// Unicode replacement character.
const FFFD: u32 = 0xfffd;

/// Höhrmann's table: `[0..256)` byte→class, `[256+state+class)` transitions.
#[rustfmt::skip]
const UTF8D: [u8; 364] = [
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,  9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,  7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
    8,8,2,2,2,2,2,2,2,2,2,2,2,2,2,2,  2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
    10,3,3,3,3,3,3,3,3,3,3,3,3,4,3,3,  11,6,6,6,5,8,8,8,8,8,8,8,8,8,8,8,

    0,12,24,36,60,96,84,12,12,12,48,72, 12,12,12,12,12,12,12,12,12,12,12,12,
    12, 0,12,12,12,12,12, 0,12, 0,12,12, 12,24,12,12,12,12,12,24,12,24,12,12,
    12,12,12,12,12,12,12,24,12,12,12,12, 12,24,12,12,12,12,12,12,12,24,12,12,
    12,12,12,12,12,12,12,36,12,36,12,12, 12,36,12,12,12,12,12,36,12,36,12,12,
    12,36,12,12,12,12,12,12,12,12,12,12,
];

/// Why decoding stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Utf8ErrorKind {
    /// A byte the DFA rejects outright (overlong, surrogate,
    /// out-of-range, stray continuation, bad lead byte).
    InvalidByte,
    /// Input ended inside a multi-byte sequence.
    Truncated,
}

/// Byte position + reason of the first decode failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Utf8Error {
    /// `InvalidByte`: index of the offending byte.
    /// `Truncated`: index where the unfinished sequence began.
    pub pos: usize,
    /// Which failure class.
    pub kind: Utf8ErrorKind,
}

/// Incremental strict decoder — feed bytes one at a time.
pub struct Decoder {
    state: u32,
    codep: u32,
    /// Byte index where the in-flight sequence started.
    seq_start: usize,
    /// Bytes consumed so far.
    pos: usize,
}

impl Decoder {
    /// Fresh decoder in `ACCEPT`.
    pub fn new() -> Decoder {
        Decoder {
            state: ACCEPT,
            codep: 0,
            pos: 0,
            seq_start: 0,
        }
    }

    /// `true` when between sequences (a complete scalar was just
    /// emitted or nothing is in flight).
    pub fn is_accepting(&self) -> bool {
        self.state == ACCEPT
    }

    /// Feed one byte: `Ok(Some(cp))` on a finished scalar,
    /// `Ok(None)` mid-sequence, `Err` on reject.
    pub fn feed(&mut self, byte: u8) -> Result<Option<u32>, Utf8Error> {
        let class = UTF8D[byte as usize] as u32;
        let out = if self.state != ACCEPT {
            (u32::from(byte) & 0x3f) | (self.codep << 6)
        } else {
            u32::from(byte) & (0xffu32 >> class)
        };
        if self.state == ACCEPT {
            self.seq_start = self.pos;
        }
        self.state = UTF8D[256 + (self.state + class) as usize] as u32;
        self.codep = out;
        let pos = self.pos;
        self.pos += 1;
        match self.state {
            REJECT => Err(Utf8Error {
                pos,
                kind: Utf8ErrorKind::InvalidByte,
            }),
            ACCEPT => Ok(Some(out)),
            _ => Ok(None),
        }
    }

    /// Check end-of-input: `Err` if a sequence is unfinished.
    pub fn finish(&self) -> Result<(), Utf8Error> {
        if self.state == ACCEPT {
            Ok(())
        } else {
            Err(Utf8Error {
                pos: self.seq_start,
                kind: Utf8ErrorKind::Truncated,
            })
        }
    }

    /// Reset to `ACCEPT` (error recovery or stream resync).
    pub fn reset(&mut self) {
        self.state = ACCEPT;
        self.codep = 0;
    }
}

impl Default for Decoder {
    fn default() -> Decoder {
        Decoder::new()
    }
}

/// `true` iff `b` is a strictly valid UTF-8 stream.
pub fn validate(b: &[u8]) -> bool {
    check(b).is_ok()
}

/// Scan `b`: `Ok(codepoint_count)` or the first `Utf8Error`.
pub fn check(b: &[u8]) -> Result<usize, Utf8Error> {
    let mut d = Decoder::new();
    let mut n = 0usize;
    for &x in b {
        match d.feed(x) {
            Ok(Some(_)) => n += 1,
            Ok(None) => {}
            Err(e) => return Err(e),
        }
    }
    d.finish()?;
    Ok(n)
}

/// All codepoints, or `None` at the first error.
pub fn decode(b: &[u8]) -> Option<Vec<u32>> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    for &x in b {
        match d.feed(x) {
            Ok(Some(cp)) => out.push(cp),
            Ok(None) => {}
            Err(_) => return None,
        }
    }
    d.finish().ok()?;
    Some(out)
}

/// Decode with U+FFFD per maximal malformed sub-sequence —
/// on a rejected byte the decoder emits `FFFD` and re-feeds the
/// same byte from `ACCEPT`, so `"a\xC0\xAFb"` yields
/// `a FFFD FFFD b` (the bad lead and its continuation each fail
/// as fresh sequences). A trailing truncation yields one `FFFD`.
pub fn decode_lossy(b: &[u8]) -> Vec<u32> {
    let mut d = Decoder::new();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        match d.feed(b[i]) {
            Ok(Some(cp)) => {
                out.push(cp);
                i += 1;
            }
            Ok(None) => i += 1,
            Err(e) => {
                out.push(FFFD);
                // a byte rejected *as a lead* is consumed; one
                // rejected mid-sequence is re-examined from
                // ACCEPT — consuming either way would loop a
                // stray continuation forever
                if e.pos == d.seq_start {
                    i += 1;
                }
                d.reset();
            }
        }
    }
    if d.finish().is_err() {
        out.push(FFFD);
    }
    out
}

/// Encode one scalar — `None` for surrogates (`U+D800..=DFFF`)
/// and values above `U+10FFFF`.
pub fn encode(cp: u32) -> Option<Vec<u8>> {
    if (0xd800..=0xdfff).contains(&cp) || cp > 0x10ffff {
        return None;
    }
    Some(if cp < 0x80 {
        vec![cp as u8]
    } else if cp < 0x800 {
        vec![0xc0 | (cp >> 6) as u8, 0x80 | (cp & 0x3f) as u8]
    } else if cp < 0x10000 {
        vec![
            0xe0 | (cp >> 12) as u8,
            0x80 | ((cp >> 6) & 0x3f) as u8,
            0x80 | (cp & 0x3f) as u8,
        ]
    } else {
        vec![
            0xf0 | (cp >> 18) as u8,
            0x80 | ((cp >> 12) & 0x3f) as u8,
            0x80 | ((cp >> 6) & 0x3f) as u8,
            0x80 | (cp & 0x3f) as u8,
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Independent naive decoder: lead-byte length + explicit
    /// per-form range checks — a separate code path from the DFA.
    fn naive_decode(b: &[u8]) -> Option<Vec<u32>> {
        let mut out = Vec::new();
        let mut i = 0usize;
        while i < b.len() {
            let x = b[i];
            let (len, cp0, lo, hi) = if x < 0x80 {
                (1usize, u32::from(x), 0u32, 0x7f)
            } else if x >> 5 == 0b110 {
                (2, u32::from(x) & 0x1f, 0x80, 0x7ff)
            } else if x >> 4 == 0b1110 {
                (3, u32::from(x) & 0x0f, 0x800, 0xffff)
            } else if x >> 3 == 0b11110 {
                (4, u32::from(x) & 0x07, 0x10000, 0x10ffff)
            } else {
                return None;
            };
            if i + len > b.len() {
                return None;
            }
            let mut cp = cp0;
            for k in 1..len {
                let c = b[i + k];
                if c >> 6 != 0b10 {
                    return None;
                }
                cp = (cp << 6) | u32::from(c) & 0x3f;
            }
            if cp < lo || cp > hi || (0xd800..=0xdfff).contains(&cp) {
                return None;
            }
            out.push(cp);
            i += len;
        }
        Some(out)
    }

    #[test]
    fn known_vectors() {
        assert_eq!(decode(b"hello"), Some(vec![104, 101, 108, 108, 111]));
        // é U+E9, € U+20AC, 😀 U+1F600
        assert_eq!(decode(&[0xc3, 0xa9]), Some(vec![0xe9]));
        assert_eq!(decode(&[0xe2, 0x82, 0xac]), Some(vec![0x20ac]));
        assert_eq!(decode(&[0xf0, 0x9f, 0x98, 0x80]), Some(vec![0x1f600]));
        // boundary scalars
        assert_eq!(decode(&[0xc2, 0x80]), Some(vec![0x80]));
        assert_eq!(decode(&[0xdf, 0xbf]), Some(vec![0x7ff]));
        assert_eq!(decode(&[0xe0, 0xa0, 0x80]), Some(vec![0x800]));
        assert_eq!(decode(&[0xef, 0xbf, 0xbf]), Some(vec![0xffff]));
        assert_eq!(decode(&[0xf0, 0x90, 0x80, 0x80]), Some(vec![0x10000]));
        assert_eq!(decode(&[0xf4, 0x8f, 0xbf, 0xbf]), Some(vec![0x10ffff]));
        // streaming decoder: mid-sequence state, reset, is_accepting
        let mut d = Decoder::new();
        assert!(d.is_accepting());
        assert_eq!(d.feed(0xf0), Ok(None));
        assert!(!d.is_accepting());
        assert_eq!(d.feed(0x9f), Ok(None));
        assert_eq!(d.feed(0x98), Ok(None));
        assert_eq!(d.feed(0x80), Ok(Some(0x1f600)));
        assert!(d.is_accepting());
        let _ = d.feed(0xe2);
        d.reset();
        assert!(d.is_accepting());
        assert_eq!(d.feed(0x61), Ok(Some(0x61)));
        assert!(d.finish().is_ok());
    }

    #[test]
    fn rejects_every_bad_form() {
        let bad: &[&[u8]] = &[
            &[0x80],                   // stray continuation
            &[0xbf],                   //
            &[0xc0, 0xaf],             // overlong 2-byte NUL
            &[0xc1, 0xbf],             // overlong 2-byte max
            &[0xe0, 0x80, 0x80],       // overlong 3-byte
            &[0xe0, 0x9f, 0xbf],       // overlong (0x7ff as 3-byte)
            &[0xf0, 0x80, 0x80, 0x80], // overlong 4-byte
            &[0xf0, 0x8f, 0xbf, 0xbf], // overlong (0xffff as 4-byte)
            &[0xed, 0xa0, 0x80],       // surrogate D800
            &[0xed, 0xbf, 0xbf],       // surrogate DFFF
            &[0xf4, 0x90, 0x80, 0x80], // U+110000
            &[0xf5, 0x80, 0x80, 0x80], // lead F5 always invalid
            &[0xfe],                   // lead FE
            &[0xff],                   // lead FF
            &[0xc3],                   // truncated 2-byte
            &[0xe2, 0x82],             // truncated 3-byte
            &[0xf0, 0x9f, 0x98],       // truncated 4-byte
            &[0x61, 0x80, 0x62],       // continuation mid-stream
            &[0x61, 0xc3],             // stream ends on a lead byte
        ];
        for (i, b) in bad.iter().enumerate() {
            assert!(!validate(b), "case {i} must be invalid");
            assert_eq!(decode(b), None, "case {i}");
        }
        // error positions
        assert_eq!(
            check(&[0x80]),
            Err(Utf8Error {
                pos: 0,
                kind: Utf8ErrorKind::InvalidByte
            })
        );
        assert_eq!(
            check(&[0x61, 0xc3]),
            Err(Utf8Error {
                pos: 1,
                kind: Utf8ErrorKind::Truncated
            })
        );
        assert_eq!(
            check(&[0x61, 0x80]),
            Err(Utf8Error {
                pos: 1,
                kind: Utf8ErrorKind::InvalidByte
            })
        );
    }

    #[test]
    fn naive_oracle_random() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..600 {
            // random byte soup — most invalid, some valid
            let len = rng.below(24);
            let mut b = Vec::with_capacity(len as usize);
            for _ in 0..len {
                b.push(rng.next_u64() as u8);
            }
            assert_eq!(decode(&b), naive_decode(&b), "bytes {b:02x?}");
            assert_eq!(validate(&b), naive_decode(&b).is_some());
            // biased-toward-valid soup
            let mut v = Vec::new();
            for _ in 0..len {
                let cp = match rng.below(4) {
                    0 => rng.below(0x80),
                    1 => rng.below(0x800),
                    2 => rng.below(0xd800),
                    _ => 0xe000 + rng.below(0x110000 - 0xe000),
                };
                v.extend(encode(cp).unwrap());
            }
            assert_eq!(decode(&v), naive_decode(&v), "valid soup");
            assert_eq!(
                decode(&v).map(|d| d.len()),
                naive_decode(&v).map(|d| d.len())
            );
        }
    }

    #[test]
    fn round_trip() {
        let mut rng = SplitMix64::new(11);
        for _ in 0..2000 {
            let mut cp = rng.below(0x110000);
            while (0xd800..=0xdfff).contains(&cp) {
                cp = rng.below(0x110000);
            }
            let enc = encode(cp).unwrap();
            assert_eq!(decode(&enc), Some(vec![cp]), "cp {cp:#x}");
            assert!(validate(&enc));
        }
        assert_eq!(encode(0xd800), None);
        assert_eq!(encode(0xdfff), None);
        assert_eq!(encode(0x110000), None);
    }

    #[test]
    fn lossy_semantics() {
        assert_eq!(decode_lossy(b"hello"), vec![104, 101, 108, 108, 111]);
        assert_eq!(decode_lossy(&[0x61, 0x80, 0x62]), vec![0x61, 0xfffd, 0x62]);
        assert_eq!(
            decode_lossy(&[0x61, 0xc0, 0xaf, 0x62]),
            vec![0x61, 0xfffd, 0xfffd, 0x62]
        );
        assert_eq!(decode_lossy(&[0x61, 0xc3]), vec![0x61, 0xfffd]);
        assert_eq!(
            decode_lossy(&[0xed, 0xa0, 0x80]),
            vec![0xfffd, 0xfffd, 0xfffd]
        );
        // never panics on arbitrary bytes
        let mut rng = SplitMix64::new(13);
        for _ in 0..2000 {
            let len = rng.below(32);
            let b: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
            let _ = decode_lossy(&b);
        }
    }
}
