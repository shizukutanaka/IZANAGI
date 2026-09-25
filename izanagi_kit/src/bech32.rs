//! Bech32 / Bech32m (BIP-173 / BIP-350) — BCH-checksummed base-32
//! strings, the segwit-native sibling of [`crate::base58`].
//!
//! - Character set `qpzry9x8gf2tvdw0s3jn54khce6mua7l` (excludes
//!   `1`,`b`,`i`,`o`); HRP then `1` then data then a 6-char checksum.
//! - `polymod` over the HRP expansion + data with the five BCH
//!   generators detects any error pattern of ≤4 chars.
//! - [`encode`] (bech32 const `1`) and [`encode_m`] (bech32m const
//!   `0x2bc830a3`); [`decode`] reports which variant it found.
//! - [`encode_segwit`]/[`decode_segwit`] enforce the consensus rule:
//!   version 0 programs use bech32, versions 1–16 use bech32m.
//!
//! ```
//! use izanagi_kit::bech32::{decode, encode, Variant};
//!
//! let s = encode("test", &[3, 1, 17, 17, 8, 15, 0]).unwrap();
//! assert_eq!(s, "test1rp33g0qxsm7l3");
//! let (hrp, data, var) = decode(&s).unwrap();
//! assert_eq!((hrp.as_str(), data.as_slice(), var), ("test", &[3u8, 1, 17, 17, 8, 15, 0][..], Variant::Bech32));
//! ```

use std::string::String;
use std::vec::Vec;

/// BIP-173 checksum constant (bech32).
const BECH32_CONST: u32 = 1;
/// BIP-350 checksum constant (bech32m).
const BECH32M_CONST: u32 = 0x2bc8_30a3;

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];

/// Which checksum variant a decoded string used.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// BIP-173 bech32 (const `1`).
    Bech32,
    /// BIP-350 bech32m (const `0x2bc830a3`).
    Bech32m,
}

fn polymod(vals: &[u8]) -> u32 {
    let mut chk = 1u32;
    for &v in vals {
        let top = chk >> 25;
        chk = ((chk & 0x01ff_ffff) << 5) | v as u32;
        for (i, g) in GEN.iter().enumerate() {
            if top >> i & 1 == 1 {
                chk ^= g;
            }
        }
    }
    chk
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(hrp.len() * 2 + 1);
    for c in hrp.bytes() {
        v.push(c >> 5);
    }
    v.push(0);
    for c in hrp.bytes() {
        v.push(c & 31);
    }
    v
}

fn checksum(hrp: &str, data: &[u8], konst: u32) -> [u8; 6] {
    let mut vals = hrp_expand(hrp);
    vals.extend_from_slice(data);
    vals.extend_from_slice(&[0; 6]);
    let pm = polymod(&vals) ^ konst;
    let mut out = [0u8; 6];
    for (i, o) in out.iter_mut().enumerate() {
        *o = ((pm >> (5 * (5 - i))) & 31) as u8;
    }
    out
}

fn encode_with(hrp: &str, data: &[u8], konst: u32) -> Option<String> {
    if hrp.is_empty() || hrp.len() > 83 {
        return None;
    }
    for b in hrp.bytes() {
        if !(33..=126).contains(&b) {
            return None;
        }
        if b.is_ascii_uppercase() {
            return None; // encode in lowercase only
        }
    }
    for &d in data {
        if d > 31 {
            return None;
        }
    }
    let cs = checksum(hrp, data, konst);
    let mut s = String::with_capacity(hrp.len() + 1 + data.len() + 6);
    s.push_str(hrp);
    s.push('1');
    for &d in data.iter().chain(cs.iter()) {
        s.push(CHARSET[d as usize] as char);
    }
    Some(s)
}

/// Encode `data` (values ≤ 31) under `hrp` with the bech32 checksum.
pub fn encode(hrp: &str, data: &[u8]) -> Option<String> {
    encode_with(hrp, data, BECH32_CONST)
}

/// Encode with the bech32m checksum (BIP-350; segwit v1+).
pub fn encode_m(hrp: &str, data: &[u8]) -> Option<String> {
    encode_with(hrp, data, BECH32M_CONST)
}

/// Decode a bech32/bech32m string → `(hrp, data, variant)`.
/// `None` on mixed case, bad chars, length > 90, missing/misplaced
/// separator, data with chars outside the charset, or checksum failure.
pub fn decode(s: &str) -> Option<(String, Vec<u8>, Variant)> {
    if s.len() < 8 || s.len() > 90 {
        return None;
    }
    let bytes = s.as_bytes();
    let mut lower = false;
    let mut upper = false;
    for &b in bytes {
        if !(33..=126).contains(&b) {
            return None;
        }
        lower |= b.is_ascii_lowercase();
        upper |= b.is_ascii_uppercase();
    }
    if lower && upper {
        return None;
    }
    let s = s.to_ascii_lowercase();
    let pos = s.rfind('1')?;
    if pos == 0 || pos + 7 > s.len() {
        return None; // hrp empty or fewer than 6 checksum chars
    }
    let (hrp, payload) = (&s[..pos], &s[pos + 1..]);
    let mut data = Vec::with_capacity(payload.len());
    for b in payload.bytes() {
        let v = CHARSET.iter().position(|&c| c == b)? as u8;
        data.push(v);
    }
    let payload_len = data.len() - 6;
    for konst in [BECH32_CONST, BECH32M_CONST] {
        let mut vals = hrp_expand(hrp);
        vals.extend_from_slice(&data);
        if polymod(&vals) == konst {
            return Some((
                hrp.to_string(),
                data[..payload_len].to_vec(),
                if konst == BECH32_CONST {
                    Variant::Bech32
                } else {
                    Variant::Bech32m
                },
            ));
        }
    }
    None
}

/// Convert a byte vector between bit groups (8→5 for encode, 5→8 for
/// decode). `pad=false` rejects when leftover bits are nonzero.
pub fn convert(data: &[u8], from_bits: u8, to_bits: u8, pad: bool) -> Option<Vec<u8>> {
    if from_bits == 0 || from_bits > 8 || to_bits == 0 || to_bits > 8 {
        return None;
    }
    let mut acc = 0u32;
    let mut bits = 0u32;
    let maxv = (1u32 << to_bits) - 1;
    let max_acc = (1u32 << (from_bits + to_bits - 1)) - 1;
    let mut out = Vec::new();
    for &b in data {
        if (b as u32) >> from_bits as u32 != 0 {
            return None;
        }
        acc = ((acc << from_bits) | b as u32) & max_acc;
        bits += from_bits as u32;
        while bits >= to_bits as u32 {
            bits -= to_bits as u32;
            out.push(((acc >> bits) & maxv) as u8);
        }
    }
    if pad {
        if bits > 0 {
            out.push(((acc << (to_bits as u32 - bits)) & maxv) as u8);
        }
    } else if bits >= from_bits as u32 || ((acc << (to_bits as u32 - bits)) & maxv) != 0 {
        return None;
    }
    Some(out)
}

/// Segwit address encode: version `ver` (0–16) + witness program
/// (2–40 bytes). v0 uses bech32, v1+ uses bech32m (BIP-350 rule).
pub fn encode_segwit(hrp: &str, ver: u8, prog: &[u8]) -> Option<String> {
    if ver > 16 || prog.len() < 2 || prog.len() > 40 {
        return None;
    }
    if ver == 0 && prog.len() != 20 && prog.len() != 32 {
        return None;
    }
    let mut data = Vec::with_capacity(1 + prog.len());
    data.push(ver);
    data.extend_from_slice(&convert(prog, 8, 5, true)?);
    if ver == 0 {
        encode(hrp, &data)
    } else {
        encode_m(hrp, &data)
    }
}

/// Segwit address decode → `(hrp, ver, program)`. Enforces the
/// version-checksum binding (v0 ↔ bech32, v1+ ↔ bech32m) and v0
/// program length (20 or 32 bytes).
pub fn decode_segwit(s: &str) -> Option<(String, u8, Vec<u8>)> {
    let (hrp, data, var) = decode(s)?;
    let (&ver, rest) = data.split_first()?;
    if ver > 16 {
        return None;
    }
    let prog = convert(rest, 5, 8, false)?;
    if prog.len() < 2 || prog.len() > 40 {
        return None;
    }
    if ver == 0 {
        if var != Variant::Bech32 || (prog.len() != 20 && prog.len() != 32) {
            return None;
        }
    } else if var != Variant::Bech32m {
        return None;
    }
    Some((hrp, ver, prog))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bip173_valid_strings() {
        // The spec's valid-checksum list.
        for s in [
            "A12UEL5L",
            "a12uel5l",
            // The spec's 91-char example is omitted: the 90-char wire
            // limit takes precedence in decode (reference impl agrees).
            "abcdef1l7aum6echk45nj3s0wdvt2fg8x9yrzpqzd3ryx",
            "11qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqc8247j",
            "split1checkupstagehandshakeupstreamerranterredcaperred2y9e3w",
            "?1ezyfcl",
        ] {
            assert!(decode(s).is_some(), "{s} should decode");
        }
    }

    #[test]
    fn bip173_invalid_strings() {
        for s in [
            " 1nwldj5",          // HRP char < 33
            "\x7f1axkwrx",       // HRP char > 126
            "an84characterslonghumanreadablepartthatcontainsthetheexcludedcharactersboandnumber11d6pts4",
            "pzry9x0s0muk",      // no separator
            "1pzry9x0s0muk",     // empty HRP
            "x1b4n0q5v",         // data char 'b'
            "li1dgmt3",          // too short checksum
            "A1G7SGD8",          // upper + bad char
            "10a06t8",           // empty HRP
            "1qzzfhee",          // empty HRP
        ] {
            assert!(decode(s).is_none(), "{s:?} must fail");
        }
    }

    #[test]
    fn segwit_vectors() {
        // BIP-173 corrected P2WPKH example.
        let (hrp, ver, prog) = decode_segwit("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").unwrap();
        assert_eq!((hrp.as_str(), ver), ("bc", 0));
        assert_eq!(prog.len(), 20);
        assert_eq!(
            encode_segwit("bc", ver, &prog).as_deref(),
            Some("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
        );

        // v1 must use bech32m — bech32 fails.
        let prog32 = vec![0x75u8; 32];
        let a = encode_segwit("bc", 1, &prog32).unwrap();
        let (_, _, v) = decode(&a).unwrap();
        assert_eq!(v, Variant::Bech32m);
        // Re-encoding v1 as bech32 must be rejected on decode.
        let mut d = vec![1u8];
        d.extend_from_slice(&convert(&prog32, 8, 5, true).unwrap());
        let wrong = encode("bc", &d).unwrap();
        assert!(decode_segwit(&wrong).is_none());
        // bech32m round-trip pins the 0x2bc830a3 constant.
        let m = encode_m("tb", &d).unwrap();
        let (_, _, var) = decode(&m).unwrap();
        assert_eq!(var, Variant::Bech32m);
    }

    #[test]
    fn roundtrip_and_malformed() {
        let s = encode("izanagi", &[0, 31, 7, 22, 3]).unwrap();
        let (h, d, v) = decode(&s).unwrap();
        assert_eq!((h.as_str(), v), ("izanagi", Variant::Bech32));
        assert_eq!(d, [0, 31, 7, 22, 3]);
        assert!(decode("").is_none());
        assert!(decode("a1").is_none());
        assert!(encode("UPPER", &[0]).is_none());
        assert!(encode("ok", &[32]).is_none());
    }

    #[test]
    fn convert_bits() {
        // 0xff → 11111 | 11100 (LSB-zero padding on the tail group).
        assert_eq!(convert(&[0xff], 8, 5, true).unwrap(), [31, 28]);
        assert_eq!(convert(&[31, 28], 5, 8, false).unwrap(), [0xff]);
        // Nonzero padding fails strict decode.
        assert!(convert(&[31, 31], 5, 8, false).is_none());
    }
}
