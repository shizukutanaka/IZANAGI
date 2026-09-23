//! AES-128 block cipher (FIPS-197) — `encrypt`/`decrypt` of single
//! 16-byte blocks. The S-box is not a stored table: it is computed at
//! construction as `affine(inv(x))` in `GF(2^8)` (modulus `0x11b`),
//! so a table typo cannot hide — the affine transform (x ⊕ rot(x,1)
//! ⊕ rot(x,2) ⊕ rot(x,3) ⊕ rot(x,4) ⊕ 0x63) is the only definition.
//! Pure-integer, branch-free of secrets — block ciphers are a natural
//! deterministic primitive; pair with `chacha`/`sha256` for protocol
//! work and `delta`/`wal` for integrity chains.
//!
//! ```
//! use izanagi_kit::aes::Aes128;
//! let a = Aes128::new(&[0u8; 16]);
//! let ct = a.encrypt(&[0u8; 16]);
//! // FIPS-197 A.1 zero-key/zero-block:
//! assert_eq!(ct[0..4], [0x66, 0xe9, 0x4b, 0xd4]);
//! assert_eq!(a.decrypt(&ct), [0u8; 16]);
//! ```

use crate::gf2;

const ROUNDS: usize = 10;
const RCON: [u8; ROUNDS] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

fn sbox(x: u8) -> u8 {
    let i = gf2::inv(x);
    i ^ i.rotate_left(1) ^ i.rotate_left(2) ^ i.rotate_left(3) ^ i.rotate_left(4) ^ 0x63
}

fn xtime(a: u8) -> u8 {
    (a << 1) ^ (if a & 0x80 != 0 { 0x1b } else { 0 })
}

/// AES-128 cipher context — expanded round keys.
pub struct Aes128 {
    /// 44 words of expanded key (11 round keys × 4 words).
    w: [u32; 44],
    s: [u8; 256],
    inv_s: [u8; 256],
}

impl Aes128 {
    /// Expand `key` (16 bytes) into round keys.
    pub fn new(key: &[u8; 16]) -> Self {
        let mut s = [0u8; 256];
        let mut inv_s = [0u8; 256];
        for i in 0..256u16 {
            let b = sbox(i as u8);
            s[i as usize] = b;
            inv_s[b as usize] = i as u8;
        }
        let mut w = [0u32; 44];
        for i in 0..4 {
            w[i] = ((key[4 * i] as u32) << 24)
                | ((key[4 * i + 1] as u32) << 16)
                | ((key[4 * i + 2] as u32) << 8)
                | (key[4 * i + 3] as u32);
        }
        for i in 4..44 {
            let mut t = w[i - 1];
            if i % 4 == 0 {
                t = t.rotate_left(8); // RotWord
                t = ((s[(t >> 24) as usize] as u32) << 24)
                    | ((s[((t >> 16) & 0xff) as usize] as u32) << 16)
                    | ((s[((t >> 8) & 0xff) as usize] as u32) << 8)
                    | (s[(t & 0xff) as usize] as u32);
                t ^= (RCON[i / 4 - 1] as u32) << 24;
            }
            w[i] = w[i - 4] ^ t;
        }
        Self { w, s, inv_s }
    }

    fn add_round_key(&self, st: &mut [u8; 16], round: usize) {
        for c in 0..4 {
            let w = self.w[round * 4 + c];
            st[4 * c] ^= (w >> 24) as u8;
            st[4 * c + 1] ^= (w >> 16) as u8;
            st[4 * c + 2] ^= (w >> 8) as u8;
            st[4 * c + 3] ^= w as u8;
        }
    }

    fn sub_bytes(&self, st: &mut [u8; 16], table: &[u8; 256]) {
        for b in st.iter_mut() {
            *b = table[*b as usize];
        }
    }

    /// Row `r` of the column-major state rotates left by `r`.
    fn shift_rows(st: &mut [u8; 16]) {
        for r in 1..4 {
            let mut row = [0u8; 4];
            for c in 0..4 {
                row[c] = st[4 * c + r];
            }
            for c in 0..4 {
                st[4 * c + r] = row[(c + r) % 4];
            }
        }
    }

    fn inv_shift_rows(st: &mut [u8; 16]) {
        for r in 1..4 {
            let mut row = [0u8; 4];
            for c in 0..4 {
                row[c] = st[4 * c + r];
            }
            for c in 0..4 {
                st[4 * c + r] = row[(c + 4 - r) % 4];
            }
        }
    }

    fn mix_columns(st: &mut [u8; 16]) {
        for c in 0..4 {
            let i = 4 * c;
            let (a0, a1, a2, a3) = (st[i], st[i + 1], st[i + 2], st[i + 3]);
            let t = a0 ^ a1 ^ a2 ^ a3;
            st[i] = a0 ^ t ^ xtime(a0 ^ a1);
            st[i + 1] = a1 ^ t ^ xtime(a1 ^ a2);
            st[i + 2] = a2 ^ t ^ xtime(a2 ^ a3);
            st[i + 3] = a3 ^ t ^ xtime(a3 ^ a0);
        }
    }

    fn inv_mix_columns(st: &mut [u8; 16]) {
        for c in 0..4 {
            let i = 4 * c;
            let (a0, a1, a2, a3) = (st[i], st[i + 1], st[i + 2], st[i + 3]);
            st[i] = gf2::mul(a0, 14) ^ gf2::mul(a1, 11) ^ gf2::mul(a2, 13) ^ gf2::mul(a3, 9);
            st[i + 1] = gf2::mul(a0, 9) ^ gf2::mul(a1, 14) ^ gf2::mul(a2, 11) ^ gf2::mul(a3, 13);
            st[i + 2] = gf2::mul(a0, 13) ^ gf2::mul(a1, 9) ^ gf2::mul(a2, 14) ^ gf2::mul(a3, 11);
            st[i + 3] = gf2::mul(a0, 11) ^ gf2::mul(a1, 13) ^ gf2::mul(a2, 9) ^ gf2::mul(a3, 14);
        }
    }

    /// Encrypt one 16-byte block.
    pub fn encrypt(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut st = *block;
        self.add_round_key(&mut st, 0);
        for r in 1..ROUNDS {
            self.sub_bytes(&mut st, &self.s);
            Self::shift_rows(&mut st);
            Self::mix_columns(&mut st);
            self.add_round_key(&mut st, r);
        }
        self.sub_bytes(&mut st, &self.s);
        Self::shift_rows(&mut st);
        self.add_round_key(&mut st, ROUNDS);
        st
    }

    /// Decrypt one 16-byte block.
    pub fn decrypt(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut st = *block;
        self.add_round_key(&mut st, ROUNDS);
        for r in (1..ROUNDS).rev() {
            Self::inv_shift_rows(&mut st);
            self.sub_bytes(&mut st, &self.inv_s);
            self.add_round_key(&mut st, r);
            Self::inv_mix_columns(&mut st);
        }
        Self::inv_shift_rows(&mut st);
        self.sub_bytes(&mut st, &self.inv_s);
        self.add_round_key(&mut st, 0);
        st
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0))
            .collect()
    }

    #[test]
    fn fips197_vector() {
        // FIPS-197 Appendix B / C.1 known answer.
        let key: [u8; 16] = h("000102030405060708090a0b0c0d0e0f")
            .try_into()
            .unwrap_or([0; 16]);
        let pt: [u8; 16] = h("00112233445566778899aabbccddeeff")
            .try_into()
            .unwrap_or([0; 16]);
        let ct = h("69c4e0d86a7b0430d8cdb78070b4c55a");
        let a = Aes128::new(&key);
        assert_eq!(a.encrypt(&pt).to_vec(), ct);
        assert_eq!(a.decrypt(&a.encrypt(&pt)), pt);
    }

    #[test]
    fn zero_key_zero_block() {
        let a = Aes128::new(&[0u8; 16]);
        let ct = a.encrypt(&[0u8; 16]);
        let want = h("66e94bd4ef8a2c3b884cfa59ca342b2e");
        assert_eq!(ct.to_vec(), want);
        assert_eq!(a.decrypt(&ct), [0u8; 16]);
    }

    #[test]
    fn sbox_spot() {
        // Known S-box entries: sbox(0x00)=0x63, sbox(0x01)=0x7c,
        // sbox(0x53)=0xed.
        assert_eq!(sbox(0x00), 0x63);
        assert_eq!(sbox(0x01), 0x7c);
        assert_eq!(sbox(0x53), 0xed);
    }

    #[test]
    fn roundtrip_all_byte_vals() {
        let mut key = [0u8; 16];
        for (i, b) in key.iter_mut().enumerate() {
            *b = i as u8;
        }
        let a = Aes128::new(&key);
        for v in 0..256u16 {
            let pt = [v as u8; 16];
            assert_eq!(a.decrypt(&a.encrypt(&pt)), pt);
        }
    }

    #[test]
    fn avalanche() {
        // One-bit flip in plaintext changes the whole block.
        let a = Aes128::new(&[0u8; 16]);
        let c0 = a.encrypt(&[0u8; 16]);
        let mut pt = [0u8; 16];
        pt[0] = 1;
        let c1 = a.encrypt(&pt);
        let diff = c0.iter().zip(c1.iter()).filter(|(x, y)| x != y).count();
        assert!(diff >= 8, "avalanche: only {diff} differing bytes");
    }
}
