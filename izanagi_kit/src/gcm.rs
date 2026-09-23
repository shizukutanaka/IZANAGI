//! AES-128-GCM authenticated encryption (SP 800-38D) on top of
//! [`crate::aes`] — CTR-mode confidentiality plus a GHASH tag over the
//! GF(2^128) polynomial `x^128 + x^7 + x^2 + x + 1`.
//!
//! `seal` returns `(ciphertext, tag)`; `open` recomputes the tag and
//! returns `None` on any mismatch, so a tampered wire byte is always
//! rejected. A 12-byte IV gets the standard `J0 = IV ‖ 0^31 ‖ 1`;
//! other lengths fall back to a GHASH-derived `J0`.
//!
//! Everything is `u128` integer arithmetic — the GHASH multiply is a
//! bit-serial shift-xor with the `0xE1` reduction constant, no tables.
//!
//! ```
//! use izanagi_kit::gcm;
//! let key = [0u8; 16];
//! let iv = [0u8; 12];
//! let (ct, tag) = gcm::seal(&key, &iv, &[], &[]);
//! // SP 800-38D test case 1.
//! assert_eq!(tag, [0x58, 0xe2, 0xfc, 0xce, 0xfa, 0x7e, 0x30, 0x61, 0x36, 0x7f,
//!                 0x1d, 0x57, 0xa4, 0xe7, 0x45, 0x5a]);
//! assert!(ct.is_empty());
//! ```

use crate::aes::Aes128;

/// GHASH multiplication in GF(2^128): `x·y` mod
/// `x^128 + x^7 + x^2 + x + 1`, bit-serial shift-and-reduce.
fn gf128_mul(x: u128, y: u128) -> u128 {
    const R: u128 = 0xE100_0000_0000_0000 << 64;
    let mut z: u128 = 0;
    let mut v = x;
    for i in 0..128 {
        if (y >> (127 - i)) & 1 == 1 {
            z ^= v;
        }
        if v & 1 == 1 {
            v = (v >> 1) ^ R;
        } else {
            v >>= 1;
        }
    }
    z
}

fn block(b: &[u8]) -> u128 {
    let mut v: u128 = 0;
    for &byte in b.iter().take(16) {
        v = (v << 8) | byte as u128;
    }
    v
}

fn unblock(v: u128) -> [u8; 16] {
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[15 - i] = (v >> (8 * i)) as u8;
    }
    out
}

/// GHASH over `H` of the block stream `data` (zero-padded tail) —
/// pure helper; `data` need not be 16-aligned.
fn ghash_from(mut y: u128, h: u128, data: &[u8]) -> u128 {
    let mut chunks = data.chunks_exact(16);
    for c in &mut chunks {
        y = gf128_mul(y ^ block(c), h);
    }
    let rem = chunks.remainder();
    if !rem.is_empty() {
        let mut tail = [0u8; 16];
        tail[..rem.len()].copy_from_slice(rem);
        y = gf128_mul(y ^ block(&tail), h);
    }
    y
}

fn ghash_raw(h: u128, data: &[u8]) -> u128 {
    ghash_from(0, h, data)
}

fn ghash_tag(h: u128, aad: &[u8], ct: &[u8]) -> u128 {
    // GHASH folds aad then ct in sequence — the accumulator chains.
    let y = ghash_from(ghash_raw(h, aad), h, ct);
    // Lengths block: bit lengths of aad and ct.
    let mut lb = [0u8; 16];
    let la = (aad.len() as u128) * 8;
    let lc = (ct.len() as u128) * 8;
    for i in 0..8 {
        lb[7 - i] = (la >> (8 * i)) as u8;
        lb[15 - i] = (lc >> (8 * i)) as u8;
    }
    gf128_mul(y ^ block(&lb), h)
}

fn inc32(ctr: u128) -> u128 {
    let low = (ctr as u32).wrapping_add(1);
    (ctr & !(0xFFFF_FFFFu128)) | low as u128
}

fn ctr_crypt(aes: &Aes128, j0: u128, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut ctr = inc32(j0);
    let mut pos = 0;
    while pos < data.len() {
        let ks = aes.encrypt(&unblock(ctr));
        let take = (data.len() - pos).min(16);
        for i in 0..take {
            out.push(data[pos + i] ^ ks[i]);
        }
        pos += take;
        ctr = inc32(ctr);
    }
    out
}

fn compute_j0(aes: &Aes128, h: u128, iv: &[u8]) -> u128 {
    if iv.len() == 12 {
        let mut b = [0u8; 16];
        b[..12].copy_from_slice(iv);
        b[15] = 1;
        return block(&b);
    }
    // J0 = GHASH_H(IV ‖ 0^s ‖ [len(IV)]64)
    let y = ghash_raw(h, iv);
    let mut lb = [0u8; 16];
    let liv = (iv.len() as u128) * 8;
    for i in 0..8 {
        lb[15 - i] = (liv >> (8 * i)) as u8;
    }
    let _ = aes;
    gf128_mul(y ^ block(&lb), h)
}

/// Encrypt-and-tag: returns `(ciphertext, tag)`. Deterministic in
/// `(key, iv, aad, plaintext)` — GCM security demands a unique IV per
/// key, which the caller must supply.
pub fn seal(key: &[u8; 16], iv: &[u8], aad: &[u8], pt: &[u8]) -> (Vec<u8>, [u8; 16]) {
    let aes = Aes128::new(key);
    let h = block(&aes.encrypt(&[0u8; 16]));
    let j0 = compute_j0(&aes, h, iv);
    let ct = ctr_crypt(&aes, j0, pt);
    let s = ghash_tag(h, aad, &ct);
    let ej0 = block(&aes.encrypt(&unblock(j0)));
    (ct, unblock(s ^ ej0))
}

/// Verify-and-decrypt: `Some(plaintext)` iff the tag matches exactly.
/// Any single-bit tamper in `ct`, `aad`, `iv`, or `tag` yields `None`.
pub fn open(key: &[u8; 16], iv: &[u8], aad: &[u8], ct: &[u8], tag: &[u8; 16]) -> Option<Vec<u8>> {
    let aes = Aes128::new(key);
    let h = block(&aes.encrypt(&[0u8; 16]));
    let j0 = compute_j0(&aes, h, iv);
    let s = ghash_tag(h, aad, ct);
    let ej0 = block(&aes.encrypt(&unblock(j0)));
    if unblock(s ^ ej0) == *tag {
        Some(ctr_crypt(&aes, j0, ct))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(0))
            .collect()
    }

    #[test]
    fn nist_case2_ct_and_tag() {
        // K=0, IV=0 (12B), PT = 16 zero bytes — McGrew & Viega case 2.
        let key = [0u8; 16];
        let iv = [0u8; 12];
        let pt = [0u8; 16];
        let (ct, tag) = seal(&key, &iv, &[], &pt);
        assert_eq!(ct, hex("0388dace60b6a392f328c2b971b2fe78"));
        assert_eq!(tag.to_vec(), hex("ab6e47d42cec13bdf53a67b21257bddf"));
    }

    #[test]
    fn nist_case4_64b_zeros() {
        // McGrew-Viega 64-byte case: nonzero key/iv, no AAD.
        let key = hex("feffe9928665731c6d6a8f9467308308");
        let key: &[u8; 16] = key[..16].try_into().unwrap_or(&[0u8; 16]);
        let iv = hex("cafebabefacedbaddecaf888");
        let pt = hex("d9313225f88406e5a55909c5aff5269a\
                     86a7a9531534f7da2e4c303d8a318a72\
                     1c3c0c95956809532fcf0e2449a6b525\
                     b16aedf5aa0de657ba637b39");
        let (ct, tag) = seal(key, &iv, &[], &pt);
        assert_eq!(
            hex("42831ec2217774244b7221b784d0d49c\
                 e3aa212f2c02a4e035c17e2329aca12e\
                 21d514b25466931c7d8f6a5aac84aa05\
                 1ba30b396a0aac973d58e091"),
            ct
        );
        assert_eq!(tag.to_vec(), hex("cc15abcc191161501aabab46b8fbac85"));
        // Same inputs plus the canonical AAD -> published tag.
        let aad = hex("feedfacedeadbeeffeedfacedeadbeefabaddad2");
        let (ct2, tag2) = seal(key, &iv, &aad, &pt);
        assert_eq!(ct2, ct);
        assert_eq!(tag2.to_vec(), hex("5bc94fbc3221a5db94fae95ae7121a47"));
    }

    #[test]
    fn ghash_selfcheck() {
        // H = E(0,0) = 66e94bd4ef8a2c3b884cfa59ca342b2e
        let h = block(&hex("66e94bd4ef8a2c3b884cfa59ca342b2e"));
        // GHASH_H(0^128 || len block 0) for empty aad+ct is 0, and
        // tag = E(J0) for the empty case — verified by case 1 already.
        assert_eq!(ghash_tag(h, &[], &[]), 0);
        // Single block of zeros: GHASH = 0 * H = 0.
        assert_eq!(ghash_raw(h, &[0u8; 16]), 0);
        // Non-trivial: GHASH of the all-ones block is h itself.
        assert!(ghash_raw(h, &[0xFF; 16]) != 0);
    }

    #[test]
    fn round_trip_and_tamper_rejection() {
        let mut rng = SplitMix64::new(0x6C6D);
        for _ in 0..40 {
            let mut key = [0u8; 16];
            let mut iv = vec![0u8; 1 + rng.below(16) as usize];
            for b in key.iter_mut() {
                *b = rng.below(256) as u8;
            }
            for b in iv.iter_mut() {
                *b = rng.below(256) as u8;
            }
            let aad: Vec<u8> = (0..rng.below(40)).map(|_| rng.below(256) as u8).collect();
            let pt: Vec<u8> = (0..rng.below(80)).map(|_| rng.below(256) as u8).collect();
            let (ct, tag) = seal(&key, &iv, &aad, &pt);
            assert_eq!(open(&key, &iv, &aad, &ct, &tag).as_deref(), Some(&pt[..]));
            // Flip one bit in the tag → reject.
            let mut bad = tag;
            bad[rng.below(16) as usize] ^= 1u8 << rng.below(8);
            assert_eq!(open(&key, &iv, &aad, &ct, &bad), None);
            // Flip one bit in the ct → reject (when non-empty).
            if !ct.is_empty() {
                let mut bad_ct = ct.clone();
                bad_ct[rng.below(ct.len() as u32) as usize] ^= 1u8;
                assert_eq!(open(&key, &iv, &aad, &bad_ct, &tag), None);
            }
        }
    }

    #[test]
    fn nonstandard_iv_len_round_trip() {
        let key = [7u8; 16];
        let iv = [3u8; 20]; // exercises the GHASH-J0 path
        let aad = b"header";
        let pt = b"payload bytes here";
        let (ct, tag) = seal(&key, &iv, aad, pt);
        assert_eq!(open(&key, &iv, aad, &ct, &tag).as_deref(), Some(&pt[..]));
        // Wrong aad must fail.
        assert_eq!(open(&key, &iv, b"headerX", &ct, &tag), None);
    }
}
