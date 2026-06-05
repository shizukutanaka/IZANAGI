//! Deterministic world hashing — FNV-1a/64.
//!
//! The arxiv/lockstep literature's strongest determinism check is a per-frame
//! checksum of full simulation state: identical inputs must produce a
//! bit-identical hash sequence. IZANAGI's TerminalBackend already makes frames
//! inspectable; hashing the *state* (not the rendered text) is stronger and
//! cheaper, and turns the snapshot test into a bit-exact replay assertion.
//!
//! Types fold their canonical bytes via `DetHash`. Collections MUST fold in a
//! canonical order (e.g. ascending entity index) — never raw dense order.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Streaming FNV-1a hasher.
#[derive(Clone, Debug)]
pub struct Fnv1a {
    hash: u64,
}

impl Default for Fnv1a {
    fn default() -> Self {
        Self { hash: FNV_OFFSET }
    }
}

impl Fnv1a {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.hash ^= b as u64;
            self.hash = self.hash.wrapping_mul(FNV_PRIME);
        }
    }

    #[inline]
    pub fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    #[inline]
    pub fn write_i32(&mut self, value: i32) {
        self.write_bytes(&value.to_le_bytes());
    }

    #[inline]
    pub fn finish(&self) -> u64 {
        self.hash
    }
}

/// Folds a value's canonical bytes into a hasher.
pub trait DetHash {
    fn det_hash(&self, hasher: &mut Fnv1a);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_hash_is_offset_basis() {
        assert_eq!(Fnv1a::new().finish(), FNV_OFFSET);
    }

    #[test]
    fn test_order_of_writes_changes_hash() {
        let mut a = Fnv1a::new();
        a.write_u32(1);
        a.write_u32(2);
        let mut b = Fnv1a::new();
        b.write_u32(2);
        b.write_u32(1);
        assert_ne!(a.finish(), b.finish(), "hash must be order-sensitive");
    }

    #[test]
    fn test_same_input_same_hash() {
        let mut a = Fnv1a::new();
        a.write_u64(0x1122_3344_5566_7788);
        let mut b = Fnv1a::new();
        b.write_u64(0x1122_3344_5566_7788);
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn test_known_vector_abc() {
        // FNV-1a/64 of "abc" is a published constant.
        let mut h = Fnv1a::new();
        h.write_bytes(b"abc");
        assert_eq!(h.finish(), 0xe71f_a219_0541_574b);
    }
}
