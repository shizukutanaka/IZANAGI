//! SipHash-2-4 keyed hash (Aumasson & Bernstein, 2012) — the
//! 64-bit PRF that became Rust's default `HashMap` hasher. A
//! 128-bit key is required, which makes `siphash` a *keyed*
//! fingerprint rather than a checksum: two keys produce
//! unrelated digests of the same message. Everything is u64
//! add/xor/rotate — exact on every platform, and the reference
//! vectors from the paper are verified in tests.
//!
//! `write` accepts arbitrary byte runs: any split of a message
//! yields the same digest, so it composes with `wal`/`delta`
//! streams. `siphash(k0, k1, data)` is the one-shot form.
//!
//! ```
//! use izanagi_kit::siphash::siphash;
//! let key: [u8; 16] = [
//!     0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
//! ];
//! assert_eq!(siphash(&key, b""), 0x726f_db47_dd0e_0e31);
//! assert_eq!(siphash(&key, &[0]), 0x74f8_39c5_93dc_67fd);
//! ```

fn rotl(x: u64, b: u32) -> u64 {
    x.rotate_left(b)
}

fn sipround(v: &mut [u64; 4]) {
    v[0] = v[0].wrapping_add(v[1]);
    v[1] = rotl(v[1], 13);
    v[1] ^= v[0];
    v[0] = rotl(v[0], 32);
    v[2] = v[2].wrapping_add(v[3]);
    v[3] = rotl(v[3], 16);
    v[3] ^= v[2];
    v[0] = v[0].wrapping_add(v[3]);
    v[3] = rotl(v[3], 21);
    v[3] ^= v[0];
    v[2] = v[2].wrapping_add(v[1]);
    v[1] = rotl(v[1], 17);
    v[1] ^= v[2];
    v[2] = rotl(v[2], 32);
}

/// Streaming SipHash-2-4 state. Feed with [`SipHash::write`],
/// read with [`SipHash::finish`].
pub struct SipHash {
    v: [u64; 4],
    buf: [u8; 8],
    buflen: usize,
    len: usize,
}

impl SipHash {
    /// New state for the 128-bit `key` (interpreted as two
    /// little-endian u64 halves, exactly as the paper).
    pub fn new(key: &[u8; 16]) -> Self {
        let k0 = u64::from_le_bytes([
            key[0], key[1], key[2], key[3], key[4], key[5], key[6], key[7],
        ]);
        let k1 = u64::from_le_bytes([
            key[8], key[9], key[10], key[11], key[12], key[13], key[14], key[15],
        ]);
        SipHash {
            v: [
                k0 ^ 0x736f_6d65_7073_6575,
                k1 ^ 0x646f_7261_6e64_6f6d,
                k0 ^ 0x6c79_6765_6e65_7261,
                k1 ^ 0x7465_6462_7974_6573,
            ],
            buf: [0; 8],
            buflen: 0,
            len: 0,
        }
    }

    fn block(&mut self, m: u64) {
        self.v[3] ^= m;
        sipround(&mut self.v);
        sipround(&mut self.v);
        self.v[0] ^= m;
    }

    /// Feed `data`; an internal 8-byte staging buffer makes the
    /// digest independent of how the caller splits the stream.
    pub fn write(&mut self, data: &[u8]) {
        self.len = self.len.wrapping_add(data.len());
        let mut i = 0;
        if self.buflen > 0 {
            let take = (8 - self.buflen).min(data.len());
            self.buf[self.buflen..self.buflen + take].copy_from_slice(&data[..take]);
            self.buflen += take;
            i += take;
            if self.buflen == 8 {
                let m = u64::from_le_bytes(self.buf);
                self.block(m);
                self.buflen = 0;
            }
        }
        while i + 8 <= data.len() {
            let m = u64::from_le_bytes([
                data[i],
                data[i + 1],
                data[i + 2],
                data[i + 3],
                data[i + 4],
                data[i + 5],
                data[i + 6],
                data[i + 7],
            ]);
            self.block(m);
            i += 8;
        }
        if i < data.len() {
            self.buf[..data.len() - i].copy_from_slice(&data[i..]);
            self.buflen = data.len() - i;
        }
    }

    /// Final digest. The last block carries the message length in
    /// its top byte, per the spec.
    pub fn finish(mut self) -> u64 {
        let mut b = ((self.len & 0xff) as u64) << 56;
        for (i, &x) in self.buf[..self.buflen].iter().enumerate() {
            b |= (x as u64) << (8 * i);
        }
        self.block(b);
        self.v[2] ^= 0xff;
        for _ in 0..4 {
            sipround(&mut self.v);
        }
        self.v[0] ^ self.v[1] ^ self.v[2] ^ self.v[3]
    }
}

/// One-shot SipHash-2-4 of `data` under 128-bit `key`.
pub fn siphash(key: &[u8; 16], data: &[u8]) -> u64 {
    let mut s = SipHash::new(key);
    s.write(data);
    s.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

    #[test]
    fn reference_vectors() {
        // Vectors from the SipHash paper (Aumasson–Bernstein):
        // key = 00..0f, message = 00..len-1.
        let expect: [(usize, u64); 8] = [
            (0, 0x726f_db47_dd0e_0e31),
            (1, 0x74f8_39c5_93dc_67fd),
            (2, 0x0d6c_8009_d9a9_4f5a),
            (7, 0xab02_00f5_8b01_d137),
            (8, 0x93f5_f579_9a93_2462),
            (15, 0xa129_ca61_49be_45e5),
            (16, 0x3f2a_cc7f_57c2_9bdb),
            (63, 0x958a_324c_eb06_4572),
        ];
        for (len, want) in expect {
            let msg: Vec<u8> = (0..len as u8).collect();
            assert_eq!(siphash(&KEY, &msg), want, "len {len}");
        }
    }

    #[test]
    fn split_independence() {
        let msg: Vec<u8> = (0..200u8).map(|b| b.wrapping_mul(37)).collect();
        let whole = siphash(&KEY, &msg);
        for cut in [0usize, 1, 7, 8, 9, 63, 64, 129, 199, 200] {
            let mut s = SipHash::new(&KEY);
            s.write(&msg[..cut]);
            s.write(&msg[cut..]);
            assert_eq!(s.finish(), whole, "cut {cut}");
        }
    }

    #[test]
    fn keyed_difference_and_determinism() {
        let a = siphash(&KEY, b"hello");
        let b = siphash(&[9u8; 16], b"hello");
        assert_ne!(a, b);
        assert_eq!(a, siphash(&KEY, b"hello"));
        // Mutating one input byte changes the digest.
        assert_ne!(siphash(&KEY, b"hello"), siphash(&KEY, b"jello"));
    }

    #[test]
    fn all_lengths_stable() {
        // Every length 0..64 digests deterministically and the
        // staged write path agrees with the single write.
        for len in 0..64usize {
            let msg: Vec<u8> = (0..len as u8).collect();
            let mut s = SipHash::new(&KEY);
            for &b in &msg {
                s.write(&[b]);
            }
            assert_eq!(s.finish(), siphash(&KEY, &msg), "len {len}");
        }
    }
}
