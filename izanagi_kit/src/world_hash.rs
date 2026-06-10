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

    /// Hash a 16-bit unsigned integer in little-endian byte order.
    /// Fills the gap between `write_bytes(&[u8])` and `write_u32` for
    /// 16-bit state values (e.g., tile IDs, screen dimensions, colour channels).
    #[inline]
    pub fn write_u16(&mut self, value: u16) {
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

    /// Hash the UTF-8 bytes of `s`. Equivalent to `write_bytes(s.as_bytes())`
    /// but named for call-site clarity when hashing entity names, config keys,
    /// or other string fields. Also powers `impl DetHash for str` and `String`.
    #[inline]
    pub fn write_str(&mut self, s: &str) {
        self.write_bytes(s.as_bytes());
    }

    /// Hash a boolean as a single byte (`0x00` = false, `0x01` = true).
    /// Completes the write-family alongside the integer writers. Using a
    /// dedicated method avoids the cast-to-byte ceremony at every call site
    /// and makes `DetHash` impls on structs with boolean fields readable.
    #[inline]
    pub fn write_bool(&mut self, value: bool) {
        self.write_bytes(&[value as u8]);
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

impl DetHash for u16 {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u16(*self);
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

impl DetHash for str {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_str(self);
    }
}

impl DetHash for String {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_str(self.as_str());
    }
}

/// Fold a slice in order, length-prefixed. Length-prefix prevents `[]` from
/// colliding with `[item]` when the item's hash equals the FNV offset basis.
impl<T: DetHash> DetHash for [T] {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.len() as u32);
        for item in self {
            item.det_hash(hasher);
        }
    }
}

/// Delegates to `[T]` so `Vec<T>` and `&[T]` produce identical hashes.
impl<T: DetHash> DetHash for Vec<T> {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.as_slice().det_hash(hasher);
    }
}

/// `None` folds as the tag byte `0`; `Some(v)` folds tag `1` then `v`.
/// The tag byte prevents `None` from hashing identically to `Some(default)`.
impl<T: DetHash> DetHash for Option<T> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        match self {
            None => hasher.write_u32(0),
            Some(v) => {
                hasher.write_u32(1);
                v.det_hash(hasher);
            }
        }
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

    #[test]
    fn test_write_str_matches_write_bytes() {
        let s = "hello";
        let mut a = Fnv1a::new();
        a.write_str(s);
        let mut b = Fnv1a::new();
        b.write_bytes(s.as_bytes());
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn test_write_str_different_strings_differ() {
        let h1 = {
            let mut h = Fnv1a::new();
            h.write_str("abc");
            h.finish()
        };
        let h2 = {
            let mut h = Fnv1a::new();
            h.write_str("xyz");
            h.finish()
        };
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_write_str_empty_matches_empty_bytes() {
        let mut a = Fnv1a::new();
        a.write_str("");
        let mut b = Fnv1a::new();
        b.write_bytes(&[]);
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn test_write_u16_differs_from_empty() {
        let mut h = Fnv1a::new();
        h.write_u16(0xABCD);
        assert_ne!(h.finish(), Fnv1a::new().finish());
    }

    #[test]
    fn test_u16_det_hash_matches_write_u16() {
        let val: u16 = 0x1234;
        let h1 = hash_state(&val);
        let mut h = Fnv1a::new();
        h.write_u16(val);
        assert_eq!(h1, h.finish());
    }

    #[test]
    fn test_u16_distinct_values_produce_distinct_hashes() {
        let h1 = hash_state(&(0u16));
        let h2 = hash_state(&(1u16));
        let h3 = hash_state(&(u16::MAX));
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
    }

    #[test]
    fn test_vec_det_hash_matches_slice() {
        let v: Vec<u32> = vec![1, 2, 3];
        assert_eq!(hash_state(&v), hash_state(v.as_slice()));
    }

    #[test]
    fn test_slice_det_hash_order_sensitive_and_length_sensitive() {
        let a: &[u32] = &[1, 2, 3];
        let b: &[u32] = &[3, 2, 1];
        let c: &[u32] = &[1, 2];
        assert_ne!(hash_state(a), hash_state(b), "order must matter");
        assert_ne!(hash_state(a), hash_state(c), "length must matter");
        assert_ne!(
            hash_state::<[u32]>(&[]),
            hash_state::<[u32]>(&[0]),
            "empty vs nonempty"
        );
    }

    #[test]
    fn test_option_none_differs_from_some_zero() {
        let none: Option<u32> = None;
        let some_zero: Option<u32> = Some(0);
        assert_ne!(hash_state(&none), hash_state(&some_zero));
    }

    #[test]
    fn test_option_some_matches_tag_plus_value() {
        let val = Some(42u32);
        let h_option = hash_state(&val);
        let mut hasher = Fnv1a::new();
        hasher.write_u32(1);
        42u32.det_hash(&mut hasher);
        assert_eq!(h_option, hasher.finish());
    }

    #[test]
    fn test_option_none_matches_tag_zero() {
        let none: Option<u32> = None;
        let h_option = hash_state(&none);
        let mut hasher = Fnv1a::new();
        hasher.write_u32(0);
        assert_eq!(h_option, hasher.finish());
    }

    // --- write_bool ---

    #[test]
    fn test_write_bool_true_differs_from_false() {
        let mut h_true = Fnv1a::new();
        h_true.write_bool(true);
        let mut h_false = Fnv1a::new();
        h_false.write_bool(false);
        assert_ne!(h_true.finish(), h_false.finish());
    }

    #[test]
    fn test_write_bool_true_matches_write_bytes_one() {
        let mut h_method = Fnv1a::new();
        h_method.write_bool(true);
        let mut h_bytes = Fnv1a::new();
        h_bytes.write_bytes(&[1u8]);
        assert_eq!(h_method.finish(), h_bytes.finish());
    }

    #[test]
    fn test_write_bool_false_matches_write_bytes_zero() {
        let mut h_method = Fnv1a::new();
        h_method.write_bool(false);
        let mut h_bytes = Fnv1a::new();
        h_bytes.write_bytes(&[0u8]);
        assert_eq!(h_method.finish(), h_bytes.finish());
    }
}
