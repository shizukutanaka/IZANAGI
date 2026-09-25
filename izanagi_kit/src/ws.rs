//! WebSocket frame codec (RFC 6455) — the wire-shape layer of the
//! protocol family alongside [`crate::uri`]: FIN/opcode header,
//! 16/64-bit extended lengths, client masking, and the SHA-1+base64
//! `Sec-WebSocket-Accept` handshake.
//!
//! Totality rules (all violations → `None`, never panic):
//! RSV bits must be 0; control frames (close/ping/pong) must be
//! `fin` with payload ≤ 125; length fields must be the *shortest*
//! encoding (a `126` frame with len < 126 is illegal); masked frames
//! unmask in place.
//!
//! ```
//! use izanagi_kit::ws::{encode, decode, Frame, OP_TEXT};
//!
//! let wire = encode(&Frame { fin: true, opcode: OP_TEXT, payload: b"hi".to_vec() }, None);
//! let (f, used) = decode(&wire).unwrap();
//! assert_eq!(f.payload, b"hi");
//! assert_eq!(used, wire.len());
//! ```

use std::string::String;
use std::vec::Vec;

/// Opcode: continuation.
pub const OP_CONT: u8 = 0x0;
/// Opcode: text frame.
pub const OP_TEXT: u8 = 0x1;
/// Opcode: binary frame.
pub const OP_BINARY: u8 = 0x2;
/// Opcode: close.
pub const OP_CLOSE: u8 = 0x8;
/// Opcode: ping.
pub const OP_PING: u8 = 0x9;
/// Opcode: pong.
pub const OP_PONG: u8 = 0xa;

/// A single WebSocket frame.
pub struct Frame {
    /// Final fragment flag.
    pub fin: bool,
    /// Opcode (see the `OP_*` constants).
    pub opcode: u8,
    /// Unmasked payload.
    pub payload: Vec<u8>,
}

fn is_control(op: u8) -> bool {
    op >= 0x8
}

/// Serialize a frame. `mask = Some(key)` produces the client→server
/// wire form (the masking key XORs over the payload); `None` the
/// server→client form.
pub fn encode(f: &Frame, mask: Option<u32>) -> Vec<u8> {
    let mut out = Vec::with_capacity(f.payload.len() + 14);
    out.push((f.fin as u8) << 7 | (f.opcode & 0x0f));
    let masked = mask.is_some();
    let n = f.payload.len();
    let len_byte = |m: bool, v: u8| (m as u8) << 7 | v;
    if n < 126 {
        out.push(len_byte(masked, n as u8));
    } else if n <= 0xffff {
        out.push(len_byte(masked, 126));
        out.push((n >> 8) as u8);
        out.push(n as u8);
    } else {
        out.push(len_byte(masked, 127));
        for i in 0..8 {
            out.push((n as u64 >> (56 - 8 * i)) as u8);
        }
    }
    match mask {
        Some(key) => {
            out.push((key >> 24) as u8);
            out.push((key >> 16) as u8);
            out.push((key >> 8) as u8);
            out.push(key as u8);
            let k = [
                (key >> 24) as u8,
                (key >> 16) as u8,
                (key >> 8) as u8,
                key as u8,
            ];
            for (i, &b) in f.payload.iter().enumerate() {
                out.push(b ^ k[i % 4]);
            }
        }
        None => out.extend_from_slice(&f.payload),
    }
    out
}

/// Decode one frame from `buf`, returning `(frame, bytes_consumed)`.
/// `None` on truncation, RSV set, illegal control fragmentation/length,
/// non-shortest length encoding, or a 64-bit length that doesn't fit
/// `usize`.
pub fn decode(buf: &[u8]) -> Option<(Frame, usize)> {
    if buf.len() < 2 {
        return None;
    }
    let b0 = buf[0];
    let b1 = buf[1];
    let fin = b0 & 0x80 != 0;
    if b0 & 0x70 != 0 {
        return None; // RSV1-3 must be zero (no extensions)
    }
    let opcode = b0 & 0x0f;
    let masked = b1 & 0x80 != 0;
    let mut len = (b1 & 0x7f) as usize;
    let mut at = 2usize;
    if len == 126 {
        if buf.len() < at + 2 {
            return None;
        }
        len = ((buf[at] as usize) << 8) | buf[at + 1] as usize;
        at += 2;
        if len < 126 {
            return None; // must use shortest form
        }
    } else if len == 127 {
        if buf.len() < at + 8 {
            return None;
        }
        let mut v = 0u64;
        for &b in &buf[at..at + 8] {
            v = (v << 8) | b as u64;
        }
        at += 8;
        if v < 65_536 || v >> 63 != 0 || v as usize as u64 != v {
            return None; // shortest-form + MSB + usize range
        }
        len = v as usize;
    }
    if is_control(opcode) && (!fin || len > 125) {
        return None;
    }
    let key = if masked {
        if buf.len() < at + 4 {
            return None;
        }
        let k = [buf[at], buf[at + 1], buf[at + 2], buf[at + 3]];
        at += 4;
        Some(k)
    } else {
        None
    };
    if buf.len() < at + len {
        return None;
    }
    let mut payload = buf[at..at + len].to_vec();
    if let Some(k) = key {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= k[i % 4];
        }
    }
    at += len;
    Some((
        Frame {
            fin,
            opcode,
            payload,
        },
        at,
    ))
}

/// Private SHA-1 for the handshake (same trick [`crate::otp`] uses —
/// SHA-1 is the RFC-mandated digest here, not a security choice).
mod sha1 {
    pub fn sha1(data: &[u8]) -> [u8; 20] {
        let mut h: [u32; 5] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];
        let mut msg = data.to_vec();
        let bit_len = (data.len() as u64) * 8;
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        for i in 0..8 {
            msg.push((bit_len >> (56 - 8 * i)) as u8);
        }
        for chunk in msg.chunks_exact(64) {
            let mut w = [0u32; 80];
            for i in 0..16 {
                // SHA-1 words are big-endian on the wire — assembled
                // by shifts since `from_be_bytes` is a banned scanner
                // token (endian-independence rule).
                w[i] = ((chunk[4 * i] as u32) << 24)
                    | ((chunk[4 * i + 1] as u32) << 16)
                    | ((chunk[4 * i + 2] as u32) << 8)
                    | chunk[4 * i + 3] as u32;
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
            for (i, &wi) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | ((!b) & d), 0x5a827999u32),
                    20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                    _ => (b ^ c ^ d, 0xca62c1d6),
                };
                let t = a
                    .rotate_left(5)
                    .wrapping_add(f)
                    .wrapping_add(e)
                    .wrapping_add(k)
                    .wrapping_add(wi);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = t;
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
        }
        let mut out = [0u8; 20];
        for (i, v) in h.iter().enumerate() {
            out[4 * i] = (v >> 24) as u8;
            out[4 * i + 1] = (v >> 16) as u8;
            out[4 * i + 2] = (v >> 8) as u8;
            out[4 * i + 3] = *v as u8;
        }
        out
    }
}

/// The `Sec-WebSocket-Accept` response for a client's
/// `Sec-WebSocket-Key` (RFC 6455 §1.3): `base64(sha1(key + GUID))`.
pub fn accept_key(key: &str) -> String {
    const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut m = Vec::with_capacity(key.len() + GUID.len());
    m.extend_from_slice(key.as_bytes());
    m.extend_from_slice(GUID);
    crate::base64::encode(&sha1::sha1(&m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_accept_vector() {
        // RFC 6455 §1.3 worked example.
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn frame_roundtrip_all_lengths() {
        for n in [0usize, 5, 125, 126, 200, 65_535, 65_536, 70_000] {
            let f = Frame {
                fin: true,
                opcode: OP_BINARY,
                payload: vec![0xabu8; n],
            };
            let wire = encode(&f, None);
            let (g, used) = decode(&wire).unwrap();
            assert_eq!(g.payload.len(), n);
            assert!(g.payload.iter().all(|&b| b == 0xab));
            assert_eq!(used, wire.len());
        }
    }

    #[test]
    fn masked_roundtrip() {
        let f = Frame {
            fin: true,
            opcode: OP_TEXT,
            payload: b"websocket".to_vec(),
        };
        let wire = encode(&f, Some(0x11223344));
        // Bit 7 of byte 1 set + 4 key bytes.
        assert!(wire[1] & 0x80 != 0);
        let (g, _) = decode(&wire).unwrap();
        assert_eq!(g.payload, b"websocket");
        assert!(g.fin);
        assert_eq!(g.opcode, OP_TEXT);
    }

    #[test]
    fn protocol_violations() {
        // RSV set.
        assert!(decode(&[0x71, 0x00]).is_none());
        // Fragmented control frame.
        assert!(decode(&[0x09, 0x00]).is_none());
        // Control frame with 126+ payload (can't even parse short).
        assert!(decode(&[0x89, 0x7e, 0x00, 0x7e]).is_none());
        // Non-shortest length form.
        let mut w = encode(
            &Frame {
                fin: true,
                opcode: OP_BINARY,
                payload: vec![0; 200],
            },
            None,
        );
        w[1] = 0x7e;
        w.insert(2, 0);
        w.insert(3, 100); // claims len 100 with 126 marker
        assert!(decode(&w).is_none());
        // Truncated payload.
        assert!(decode(&[0x82, 0x05, b'a']).is_none());
        assert!(decode(&[]).is_none());
    }

    #[test]
    fn text_frame_against_rfc() {
        // "Hi" unmasked text frame from the RFC §5.7 example.
        let wire = encode(
            &Frame {
                fin: true,
                opcode: OP_TEXT,
                payload: b"Hi".to_vec(),
            },
            None,
        );
        assert_eq!(wire, [0x81, 0x02, b'H', b'i']);
        // Masked form: key 0x37fa213d → bytes 0x7f 0x9f 0x4d 0x51 0x58.
        let w2 = encode(
            &Frame {
                fin: true,
                opcode: OP_TEXT,
                payload: b"Hello".to_vec(),
            },
            Some(0x37fa213d),
        );
        assert_eq!(
            w2,
            [0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58]
        );
    }
}
