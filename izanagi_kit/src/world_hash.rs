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

    /// Hash a signed 64-bit integer in little-endian byte order.
    /// Completes the integer-write family alongside `write_u32`, `write_u64`,
    /// and `write_i32`. Useful for hashing large tick counters, cumulative
    /// damage totals, or other wide simulation state fields.
    #[inline]
    pub fn write_i64(&mut self, value: i64) {
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

/// Convenience: reduce any [`DetHash`] value to a single 64-bit checksum. This
/// is the per-frame "state hash" the replay harness compares.
#[inline]
pub fn hash_state<T: DetHash + ?Sized>(value: &T) -> u64 {
    let mut hasher = Fnv1a::new();
    value.det_hash(&mut hasher);
    hasher.finish()
}

// Primitive impls. Fixed-width little-endian so the folded bytes are identical
// on every target (no native-endian or pointer-width leakage).
impl DetHash for u8 {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(*self as u32);
    }
}

impl DetHash for u32 {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(*self);
    }
}

impl DetHash for u64 {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u64(*self);
    }
}

impl DetHash for i32 {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(*self);
    }
}

impl DetHash for i64 {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i64(*self);
    }
}

impl DetHash for bool {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_bytes(&[*self as u8]);
    }
}

impl DetHash for char {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(*self as u32);
    }
}

impl<A: DetHash, B: DetHash> DetHash for (A, B) {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.0.det_hash(hasher);
        self.1.det_hash(hasher);
    }
}

impl<A: DetHash, B: DetHash, C: DetHash> DetHash for (A, B, C) {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.0.det_hash(hasher);
        self.1.det_hash(hasher);
        self.2.det_hash(hasher);
    }
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

    #[test]
    fn test_tuple2_det_hash_same_as_sequential_writes() {
        let pair = (1u32, 2u32);
        let h_tuple = hash_state(&pair);

        let mut hasher = Fnv1a::new();
        1u32.det_hash(&mut hasher);
        2u32.det_hash(&mut hasher);
        let h_manual = hasher.finish();

        assert_eq!(h_tuple, h_manual);
    }

    #[test]
    fn test_tuple2_order_matters() {
        let a = (1u32, 2u32);
        let b = (2u32, 1u32);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_tuple3_det_hash_same_as_sequential_writes() {
        let triple = (1u32, 2u32, 3u32);
        let h_triple = hash_state(&triple);

        let mut hasher = Fnv1a::new();
        1u32.det_hash(&mut hasher);
        2u32.det_hash(&mut hasher);
        3u32.det_hash(&mut hasher);
        let h_manual = hasher.finish();

        assert_eq!(h_triple, h_manual);
    }

    #[test]
    fn test_write_i64_differs_from_empty() {
        let mut h = Fnv1a::new();
        h.write_i64(0);
        assert_ne!(h.finish(), Fnv1a::new().finish());
    }

    #[test]
    fn test_i64_det_hash_matches_write_i64() {
        let val: i64 = -1234567890123;
        let h1 = hash_state(&val);
        let mut h = Fnv1a::new();
        h.write_i64(val);
        assert_eq!(h1, h.finish());
    }

    #[test]
    fn test_i64_negative_differs_from_positive() {
        let pos = hash_state(&42i64);
        let neg = hash_state(&(-42i64));
        assert_ne!(pos, neg, "sign must affect the hash");
    }
}
