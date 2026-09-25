//! QOI ("Quite OK Image") codec — a minimal, fast lossless image format.
//!
//! Streaming byte-oriented codec with five opcodes: run, index, diff, luma,
//! and literal RGB/RGBA. `encode` supports 3- or 4-channel input; `decode`
//! reconstructs the pixels and reports `(width, height, channels)`. All
//! multi-byte header fields are big-endian per the spec and spelled out by
//! hand for endian independence.
//!
//! ```
//! use izanagi_kit::qoi;
//! let px = [255u8, 0, 0, 255, 0, 255, 0, 255]; // two RGBA pixels
//! let enc = qoi::encode(&px, 2, 1, 4);
//! let (dec, w, h, ch) = qoi::decode(&enc).unwrap();
//! assert_eq!((w, h, ch), (2, 1, 4));
//! assert_eq!(dec, px);
//! ```

const OP_RUN: u8 = 0xc0;
const OP_INDEX: u8 = 0x00;
const OP_DIFF: u8 = 0x40;
const OP_LUMA: u8 = 0x80;
const OP_RGB: u8 = 0xfe;
const OP_RGBA: u8 = 0xff;
const MASK_TAG: u8 = 0xc0;
const END: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];

fn push_u32be(v: u32, out: &mut Vec<u8>) {
    out.push((v >> 24) as u8);
    out.push((v >> 16) as u8);
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

fn read_u32be(b: &[u8]) -> u32 {
    ((b[0] as u32) << 24) | ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | b[3] as u32
}

fn hash_idx(r: u8, g: u8, b: u8, a: u8) -> usize {
    (r as usize * 3 + g as usize * 5 + b as usize * 7 + a as usize * 11) & 63
}

/// Encode `pixels` (`w`×`h`×`channels`, `channels` 3 or 4) into a QOI stream.
/// Empty input or unsupported channel count yields an empty vector.
pub fn encode(pixels: &[u8], w: u32, h: u32, channels: u8) -> Vec<u8> {
    let n_px = w as usize * h as usize;
    if n_px == 0 || (channels != 3 && channels != 4) || pixels.len() < n_px * channels as usize {
        return Vec::new();
    }
    let ch = channels as usize;
    let mut out = Vec::with_capacity(n_px + 22);
    out.extend_from_slice(b"qoif");
    push_u32be(w, &mut out);
    push_u32be(h, &mut out);
    out.push(channels);
    out.push(0); // colorspace: sRGB

    let mut index = [[0u8; 4]; 64];
    let mut prev = [0u8, 0, 0, 255];
    let mut run: u8 = 0;
    for i in 0..n_px {
        let px = &pixels[i * ch..i * ch + ch];
        let (r, g, b) = (px[0], px[1], px[2]);
        let a = if ch == 4 { px[3] } else { 255 };
        if r == prev[0] && g == prev[1] && b == prev[2] && a == prev[3] {
            run += 1;
            if run == 62 || i + 1 == n_px {
                out.push(OP_RUN | (run - 1));
                run = 0;
            }
            continue;
        }
        if run > 0 {
            out.push(OP_RUN | (run - 1));
            run = 0;
        }
        let idx = hash_idx(r, g, b, a);
        if index[idx] == [r, g, b, a] {
            out.push(OP_INDEX | idx as u8);
        } else {
            index[idx] = [r, g, b, a];
            if a == prev[3] {
                // Channel deltas wrap mod 256 and may exceed i8 when
                // differenced again — compute in i16.
                let dr = i16::from(r.wrapping_sub(prev[0]) as i8);
                let dg = i16::from(g.wrapping_sub(prev[1]) as i8);
                let db = i16::from(b.wrapping_sub(prev[2]) as i8);
                if (-2..=1).contains(&dr) && (-2..=1).contains(&dg) && (-2..=1).contains(&db) {
                    out.push(
                        OP_DIFF
                            | (((dr + 2) as u8) << 4)
                            | (((dg + 2) as u8) << 2)
                            | ((db + 2) as u8),
                    );
                } else {
                    let dr_dg = dr - dg;
                    let db_dg = db - dg;
                    if (-32..=31).contains(&dg)
                        && (-8..=7).contains(&dr_dg)
                        && (-8..=7).contains(&db_dg)
                    {
                        out.push(OP_LUMA | ((dg + 32) as u8));
                        out.push((((dr_dg + 8) as u8) << 4) | ((db_dg + 8) as u8 & 0x0f));
                    } else {
                        out.push(OP_RGB);
                        out.extend_from_slice(&[r, g, b]);
                    }
                }
            } else {
                out.push(OP_RGBA);
                out.extend_from_slice(&[r, g, b, a]);
            }
        }
        prev = [r, g, b, a];
    }
    out.extend_from_slice(&END);
    out
}

/// Decode a QOI stream. Returns `(pixels, width, height, channels)` or
/// `None` on bad magic, truncated data, or overlong pixel stream.
pub fn decode(data: &[u8]) -> Option<(Vec<u8>, u32, u32, u8)> {
    if data.len() < 14 || &data[..4] != b"qoif" {
        return None;
    }
    let w = read_u32be(&data[4..8]);
    let h = read_u32be(&data[8..12]);
    let channels = data[12];
    if channels != 3 && channels != 4 {
        return None;
    }
    let ch = channels as usize;
    let n_px = w.checked_mul(h)? as usize;
    let mut px = Vec::with_capacity(n_px * ch);
    let mut index = [[0u8; 4]; 64];
    let mut cur = [0u8, 0, 0, 255];
    let mut pos = 14usize;
    let emit = |px: &mut Vec<u8>, cur: &[u8; 4]| {
        if ch == 4 {
            px.extend_from_slice(cur);
        } else {
            px.extend_from_slice(&cur[..3]);
        }
    };
    let mut written = 0usize;
    while written < n_px && pos < data.len() {
        let b1 = data[pos];
        pos += 1;
        if b1 == OP_RGB {
            if pos + 3 > data.len() {
                return None;
            }
            cur[0] = data[pos];
            cur[1] = data[pos + 1];
            cur[2] = data[pos + 2];
            pos += 3;
        } else if b1 == OP_RGBA {
            if pos + 4 > data.len() {
                return None;
            }
            cur.copy_from_slice(&data[pos..pos + 4]);
            pos += 4;
        } else if b1 & MASK_TAG == OP_INDEX {
            cur = index[(b1 & 0x3f) as usize];
        } else if b1 & MASK_TAG == OP_DIFF {
            let dr = ((b1 >> 4) & 3) as i8 - 2;
            let dg = ((b1 >> 2) & 3) as i8 - 2;
            let db = (b1 & 3) as i8 - 2;
            cur[0] = cur[0].wrapping_add(dr as u8);
            cur[1] = cur[1].wrapping_add(dg as u8);
            cur[2] = cur[2].wrapping_add(db as u8);
        } else if b1 & MASK_TAG == OP_LUMA {
            if pos >= data.len() {
                return None;
            }
            let b2 = data[pos];
            pos += 1;
            let dg = (b1 & 0x3f) as i16 - 32;
            let dr_dg = ((b2 >> 4) & 0x0f) as i16 - 8;
            let db_dg = (b2 & 0x0f) as i16 - 8;
            cur[0] = cur[0].wrapping_add((dg + dr_dg) as i8 as u8);
            cur[1] = cur[1].wrapping_add(dg as i8 as u8);
            cur[2] = cur[2].wrapping_add((dg + db_dg) as i8 as u8);
        } else {
            // OP_RUN: length = (b1 & 0x3f) + 1
            let run = (b1 & 0x3f) as usize + 1;
            if written + run > n_px {
                return None;
            }
            for _ in 0..run {
                emit(&mut px, &cur);
            }
            written += run;
            continue;
        }
        index[hash_idx(cur[0], cur[1], cur[2], cur[3])] = cur;
        emit(&mut px, &cur);
        written += 1;
    }
    if written != n_px {
        return None;
    }
    Some((px, w, h, channels))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn roundtrip_random_rgba() {
        let mut rng = SplitMix64::new(42);
        let px: Vec<u8> = (0..(17 * 9 * 4))
            .map(|_| (rng.next_u32() & 0xff) as u8)
            .collect();
        let enc = encode(&px, 17, 9, 4);
        let (dec, w, h, ch) = decode(&enc).unwrap();
        assert_eq!((w, h, ch, dec), (17, 9, 4, px));
    }

    #[test]
    fn roundtrip_rgb_and_runs() {
        // Long solid stretch exercises OP_RUN; gradient exercises LUMA/DIFF.
        let mut px = vec![7u8; 120 * 3];
        for i in 40..120 {
            px[i * 3] = (i as u8).wrapping_mul(3);
            px[i * 3 + 1] = (i as u8).wrapping_mul(2);
            px[i * 3 + 2] = i as u8;
        }
        let enc = encode(&px, 40, 3, 3);
        let (dec, w, h, ch) = decode(&enc).unwrap();
        assert_eq!((w, h, ch, dec), (40, 3, 3, px));
    }

    #[test]
    fn run_opcode_present_and_capped() {
        // 100 identical pixels → run opcodes split at 62.
        let px = vec![9u8; 100 * 4];
        let enc = encode(&px, 100, 1, 4);
        // Expect at least one OP_RUN byte (0xc0|len-1).
        assert!(enc[14..].iter().any(|&b| b & MASK_TAG == OP_RUN));
        let (dec, ..) = decode(&enc).unwrap();
        assert_eq!(dec, px);
    }

    #[test]
    fn diff_opcode_small_delta() {
        // Two pixels differing by +1 in green only → OP_DIFF.
        let px = [10u8, 10, 10, 255, 10, 11, 10, 255];
        let enc = encode(&px, 2, 1, 4);
        // First px vs initial [0,0,0,255]: deltas (10,10,10) → OP_LUMA.
        assert_eq!(enc[14] & MASK_TAG, OP_LUMA);
        // Second px delta (0,+1,0) → OP_DIFF tag.
        assert_eq!(enc[16] & MASK_TAG, OP_DIFF);
    }

    #[test]
    fn decode_rejects_bad_input() {
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(b"qoif"), None);
        assert_eq!(decode(b"nope............."), None);
        // Truncated pixel data: header says 4 px, stream ends.
        let mut d = b"qoif".to_vec();
        d.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 2, 4, 0]);
        d.push(OP_RGB);
        assert_eq!(decode(&d), None);
        // Channel value 5 rejected.
        let mut d = b"qoif".to_vec();
        d.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 5, 0]);
        assert_eq!(decode(&d), None);
    }

    #[test]
    fn deterministic_twice() {
        let px = [1u8, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(encode(&px, 2, 1, 4), encode(&px, 2, 1, 4));
    }
}
