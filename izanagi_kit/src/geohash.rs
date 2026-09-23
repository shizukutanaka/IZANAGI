//! Geohash codec over integer coordinates (Gustavo Niemeyer's
//! latitude/longitude interleaving). Coordinates are microdegrees:
//! `lat_e6` in `[-90_000_000, 90_000_000]`, `lon_e6` in
//! `[-180_000_000, 180_000_000]` — pure integers end to end, so
//! encode/decode/precision are all exact. The code is a base-32
//! string whose prefixes nest: `encode` at length `L` is the
//! `L`-character cell prefix, and `decode` returns the cell bounds
//! (integer microdegree ranges).
//!
//! ```
//! use izanagi_kit::geohash::{encode, decode, neighbors};
//! // Tokyo Tower: 35.658584, 139.745433
//! let h = encode(35_658_584, 139_745_433, 7);
//! assert_eq!(h, "xn76ggr");
//! let ((la0, la1), (lo0, lo1)) = decode(&h).unwrap_or(((0, 0), (0, 0)));
//! assert!(la0 <= 35_658_584 && 35_658_584 < la1);
//! assert!(lo0 <= 139_745_433 && 139_745_433 < lo1);
//! let adj = neighbors(&h);
//! assert_eq!(adj.len(), 8);
//! ```

const BASE32: &[u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";
const LAT_MIN: i64 = -90_000_000;
const LAT_SPAN: i64 = 180_000_000;
const LON_MIN: i64 = -180_000_000;
const LON_SPAN: i64 = 360_000_000;

fn bisect(v: i64, lo: i64, hi: i64) -> (u8, i64, i64) {
    let mid = lo + (hi - lo) / 2;
    if v >= mid {
        (1, mid, hi)
    } else {
        (0, lo, mid)
    }
}

/// Encode integer microdegree `lat_e6`/`lon_e6` into a geohash of
/// `len` characters (5·len bits).
pub fn encode(lat_e6: i64, lon_e6: i64, len: usize) -> String {
    let (mut la0, mut la1) = (LAT_MIN, LAT_MIN + LAT_SPAN);
    let (mut lo0, mut lo1) = (LON_MIN, LON_MIN + LON_SPAN);
    let mut out = String::with_capacity(len);
    let mut even = true; // longitude first
    let mut ch = 0u8;
    let mut bit = 0u8;
    for _ in 0..len * 5 {
        let (b, lo, hi) = if even {
            bisect(lon_e6, lo0, lo1)
        } else {
            bisect(lat_e6, la0, la1)
        };
        if even {
            lo0 = lo;
            lo1 = hi;
        } else {
            la0 = lo;
            la1 = hi;
        }
        ch = (ch << 1) | b;
        bit += 1;
        if bit == 5 {
            out.push(BASE32[ch as usize] as char);
            ch = 0;
            bit = 0;
        }
        even = !even;
    }
    out
}

/// Decode a geohash into its cell bounds `((lat_lo, lat_hi),
/// (lon_lo, lon_hi))` in integer microdegrees — the point cell of
/// every encode that produces this prefix. Returns `None` on bad
/// input (empty, non-base32 char).
pub fn decode(hash: &str) -> Option<((i64, i64), (i64, i64))> {
    if hash.is_empty() {
        return None;
    }
    let (mut la0, mut la1) = (LAT_MIN, LAT_MIN + LAT_SPAN);
    let (mut lo0, mut lo1) = (LON_MIN, LON_MIN + LON_SPAN);
    let mut even = true;
    for c in hash.bytes() {
        let v = BASE32.iter().position(|&b| b == c)? as u8;
        for k in (0..5).rev() {
            let bit = (v >> k) & 1;
            if even {
                let mid = lo0 + (lo1 - lo0) / 2;
                if bit == 1 {
                    lo0 = mid;
                } else {
                    lo1 = mid;
                }
            } else {
                let mid = la0 + (la1 - la0) / 2;
                if bit == 1 {
                    la0 = mid;
                } else {
                    la1 = mid;
                }
            }
            even = !even;
        }
    }
    Some(((la0, la1), (lo0, lo1)))
}

/// Nominal cell width/height of a `len`-char geohash in
/// microdegrees (`(lat_span, lon_span)`) — individual cells can
/// differ by ±1 because each bisection floors its midpoint.
pub fn cell_span(len: usize) -> (i64, i64) {
    let lon_bits = (len * 5).div_ceil(2);
    let lat_bits = len * 5 - lon_bits;
    (
        LAT_SPAN >> lat_bits.min(62) as u32,
        LON_SPAN >> lon_bits.min(62) as u32,
    )
}

/// The 8 adjacent cells of `hash` at the same precision — N, S, E,
/// W and the four diagonals, in a fixed (sorted) order. Edge cells
/// that would leave the world are simply re-encoded from the
/// clamped neighbor center (so the result can contain fewer than 8
/// distinct cells at the poles/dateline).
pub fn neighbors(hash: &str) -> Vec<String> {
    let Some(((la0, la1), (lo0, lo1))) = decode(hash) else {
        return Vec::new();
    };
    let la = la0 + (la1 - la0) / 2;
    let lo = lo0 + (lo1 - lo0) / 2;
    let (h, w) = (la1 - la0, lo1 - lo0);
    let len = hash.len();
    let mut out = Vec::with_capacity(8);
    for dy in -1i64..=1 {
        for dx in -1i64..=1 {
            if dy == 0 && dx == 0 {
                continue;
            }
            let nla = la + dy * h;
            let nlo = lo + dx * w;
            // Clamp into the world rather than wrapping.
            let cla = nla.clamp(LAT_MIN + 1, LAT_MIN + LAT_SPAN - 1);
            let clo = nlo.clamp(LON_MIN + 1, LON_MIN + LON_SPAN - 1);
            out.push(encode(cla, clo, len));
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        // 42.605, -5.603 at 5 chars — matches common geohash
        // calculators ('ezs42' region).
        let h = encode(42_605_000, -5_603_000, 5);
        assert!(h.starts_with("ezs"), "{h}");
    }

    #[test]
    fn decode_inverts_encode() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(7);
        for _ in 0..500 {
            let lat = LAT_MIN + (rng.next_u64() % (LAT_SPAN as u64)) as i64;
            let lon = LON_MIN + (rng.next_u64() % (LON_SPAN as u64)) as i64;
            let len = 1 + rng.below(9) as usize;
            let h = encode(lat, lon, len);
            assert_eq!(h.len(), len);
            let ((a0, a1), (o0, o1)) = decode(&h).unwrap_or(((-1, -2), (-3, -4)));
            assert!(a0 <= lat && lat < a1, "{h} lat {lat} not in [{a0},{a1})");
            assert!(o0 <= lon && lon < o1, "{h} lon {lon} not in [{o0},{o1})");
            // Cell span matches the nominal bit budget within ±1 —
            // integer bisections accumulate floor rounding.
            let (sa, so) = cell_span(len);
            assert!((a1 - a0 - sa).abs() <= 1);
            assert!((o1 - o0 - so).abs() <= 1);
        }
    }

    #[test]
    fn prefix_nests() {
        let h9 = encode(35_658_584, 139_745_433, 9);
        for l in 1..=9 {
            let h = encode(35_658_584, 139_745_433, l);
            assert!(h9.starts_with(&h));
        }
    }

    #[test]
    fn neighbors_cover() {
        let h = encode(0, 0, 6);
        let n = neighbors(&h);
        assert_eq!(n.len(), 8);
        // Every neighbor is a distinct cell of the same length.
        assert!(n.iter().all(|x| x.len() == 6));
        assert!(n.iter().all(|x| *x != h));
    }

    #[test]
    fn decode_rejects() {
        assert!(decode("").is_none());
        assert!(decode("a!b").is_none());
        // 'a', 'i', 'l', 'o' are not in the geohash alphabet.
        assert!(decode("abc").is_none());
    }
}
