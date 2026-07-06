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

    /// Hash a single unsigned byte. Fills the gap between `write_bytes` (slice)
    /// and `write_u16` (two bytes) — useful for single-byte enum discriminants,
    /// status flags, and small palette indices.
    #[inline]
    pub fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    /// Hash a signed 16-bit integer in little-endian byte order. Completes the
    /// 16-bit pair alongside `write_u16`. Useful for signed tile offsets and
    /// small health deltas stored as `i16`.
    #[inline]
    pub fn write_i16(&mut self, value: i16) {
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

/// Final avalanche mix (SplitMix64 finalizer) — spreads every input bit across
/// the whole 64-bit word so a commutative combine cannot leak structure.
#[inline]
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Order-independent checksum of a collection: the result is identical for any
/// permutation of `items`, so a container whose iteration order is *not*
/// canonical (a `HashMap`, entities gathered in arrival order, a parallel
/// reduction) can still be hashed into a stable value **without sorting first**.
///
/// [`hash_state`] on a slice/`Vec` is order-*dependent* by design (it
/// length-prefixes and folds in sequence); when order is genuinely irrelevant,
/// sorting into a canonical order just to feed `hash_state` is wasted work.
/// `hash_unordered` removes that step: each element is hashed independently via
/// [`hash_state`], passed through an avalanche mix, then combined with
/// `wrapping_add` — commutative and associative, hence permutation-invariant.
///
/// **Multiset, not set**: duplicates count. `[a, a]` and `[a]` hash
/// differently (the sum accumulates and the folded cardinality differs), which
/// is usually what game state wants — two identical components are not the same
/// as one. The element count is folded in, so multisets that happen to share
/// an element-sum still differ, and the empty collection maps to a fixed,
/// distinct value.
///
/// This is an ordinary 64-bit hash: collisions are possible and it is not a
/// reversible encoding. It is purely additive to the module — it changes no
/// existing [`hash_state`]/[`DetHash`] output and touches no pinned hash. To
/// fold an unordered collection into a larger `DetHash`, write its result:
/// `hasher.write_u64(hash_unordered(items))`.
///
/// ```
/// use izanagi_kit::world_hash::hash_unordered;
/// let a = [10u32, 20, 30];
/// let b = [30u32, 10, 20];
/// assert_eq!(hash_unordered(&a), hash_unordered(&b), "permutation-invariant");
/// assert_ne!(hash_unordered(&[10u32, 10]), hash_unordered(&[10u32]), "multiset");
/// ```
pub fn hash_unordered<'a, T, I>(items: I) -> u64
where
    T: DetHash + 'a,
    I: IntoIterator<Item = &'a T>,
{
    let mut acc: u64 = 0;
    let mut count: u64 = 0;
    for item in items {
        acc = acc.wrapping_add(mix64(hash_state(item)));
        count = count.wrapping_add(1);
    }
    // Fold the cardinality so different-size multisets that sum equal still
    // differ, and an empty collection gets a distinct (mix of 0 and 0) value.
    mix64(acc ^ count.wrapping_mul(FNV_PRIME))
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

/// Length-prefix prevents `("ab","c")` from hashing identically to `("a","bc")`:
/// the same byte sequence with a different split between two adjacent string
/// fields. Mirrors the `[T]` impl which length-prefixes slices for the same reason.
impl DetHash for str {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.len() as u32);
        hasher.write_str(self);
    }
}

/// Delegates to `str` so `String` and `&str` produce identical hashes.
impl DetHash for String {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.as_str().det_hash(hasher);
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

    // --- write_u8 ---

    #[test]
    fn test_write_u8_matches_write_bytes_single_byte() {
        let mut h_method = Fnv1a::new();
        h_method.write_u8(0xAB);
        let mut h_bytes = Fnv1a::new();
        h_bytes.write_bytes(&[0xAB]);
        assert_eq!(h_method.finish(), h_bytes.finish());
    }

    #[test]
    fn test_write_u8_differs_from_write_u16() {
        let mut h_u8 = Fnv1a::new();
        h_u8.write_u8(5);
        let mut h_u16 = Fnv1a::new();
        h_u16.write_u16(5);
        assert_ne!(h_u8.finish(), h_u16.finish());
    }

    #[test]
    fn test_write_u8_distinct_values_differ() {
        let mut h1 = Fnv1a::new();
        h1.write_u8(0);
        let mut h2 = Fnv1a::new();
        h2.write_u8(1);
        assert_ne!(h1.finish(), h2.finish());
    }

    // --- write_i16 ---

    #[test]
    fn test_write_i16_matches_write_bytes_le() {
        let val: i16 = -1000;
        let mut h_method = Fnv1a::new();
        h_method.write_i16(val);
        let mut h_bytes = Fnv1a::new();
        h_bytes.write_bytes(&val.to_le_bytes());
        assert_eq!(h_method.finish(), h_bytes.finish());
    }

    #[test]
    fn test_write_i16_negative_differs_from_positive() {
        let mut h_pos = Fnv1a::new();
        h_pos.write_i16(100);
        let mut h_neg = Fnv1a::new();
        h_neg.write_i16(-100);
        assert_ne!(h_pos.finish(), h_neg.finish());
    }

    #[test]
    fn test_write_i16_differs_from_write_u16_same_bits_when_negative() {
        let signed: i16 = -1;
        let unsigned: u16 = 0xFFFF;
        // Both are 0xFFFF in bits; they should hash identically (LE bytes are the same).
        let mut h_i16 = Fnv1a::new();
        h_i16.write_i16(signed);
        let mut h_u16 = Fnv1a::new();
        h_u16.write_u16(unsigned);
        assert_eq!(
            h_i16.finish(),
            h_u16.finish(),
            "same bit pattern = same hash"
        );
    }

    // --- DetHash for str / String: prefix-split collision guard ---

    /// Two adjacent string fields with different splits of the same byte run
    /// MUST hash differently. Before the length-prefix fix, ("ab","c") and
    /// ("a","bc") wrote the identical byte sequence [97,98,99] and produced
    /// the same hash, making distinct game states indistinguishable.
    #[test]
    fn test_str_det_hash_prefix_split_does_not_collide() {
        let h_ab_c = {
            let mut h = Fnv1a::new();
            "ab".det_hash(&mut h);
            "c".det_hash(&mut h);
            h.finish()
        };
        let h_a_bc = {
            let mut h = Fnv1a::new();
            "a".det_hash(&mut h);
            "bc".det_hash(&mut h);
            h.finish()
        };
        assert_ne!(
            h_ab_c, h_a_bc,
            "prefix-split collision: ('ab','c') and ('a','bc') must hash differently"
        );
    }

    /// Each string hashes to a value that includes its length: empty and
    /// single-char must differ, and two strings of equal content but different
    /// positions in the sequence must still yield distinct cumulative hashes.
    #[test]
    fn test_str_det_hash_is_length_sensitive() {
        assert_ne!(
            hash_state(""),
            hash_state("a"),
            "empty and non-empty strings must hash differently"
        );
        assert_ne!(
            hash_state("abc"),
            hash_state("ab"),
            "strings of different lengths must hash differently"
        );
    }

    /// String and &str produce the same hash (String delegates to str impl).
    #[test]
    fn test_string_and_str_det_hash_agree() {
        let owned = String::from("hello world");
        let borrowed: &str = "hello world";
        assert_eq!(hash_state(owned.as_str()), hash_state(borrowed));
    }

    /// The length prefix is exactly write_u32(len) followed by the raw bytes,
    /// so call sites that must produce a stable cross-build encoding can
    /// replicate it without going through the DetHash trait.
    #[test]
    fn test_str_det_hash_encoding_is_len_then_bytes() {
        let s = "hi";
        let h_trait = hash_state(s);
        let mut h_manual = Fnv1a::new();
        h_manual.write_u32(s.len() as u32);
        h_manual.write_str(s);
        assert_eq!(h_trait, h_manual.finish(), "DetHash for str must be write_u32(len) ++ raw bytes");
    }

    // --- hash_unordered ---

    #[test]
    fn test_hash_unordered_is_permutation_invariant() {
        let a = [1u32, 2, 3, 4, 5];
        let b = [5u32, 3, 1, 4, 2];
        let c = [3u32, 1, 2, 5, 4];
        assert_eq!(hash_unordered(&a), hash_unordered(&b));
        assert_eq!(hash_unordered(&a), hash_unordered(&c));
    }

    #[test]
    fn test_hash_unordered_is_multiset_not_set() {
        // Duplicates count: [a, a] must differ from [a].
        assert_ne!(hash_unordered(&[7u32, 7]), hash_unordered(&[7u32]));
        // And two copies differ from three copies.
        assert_ne!(hash_unordered(&[7u32, 7]), hash_unordered(&[7u32, 7, 7]));
    }

    #[test]
    fn test_hash_unordered_is_content_sensitive() {
        assert_ne!(hash_unordered(&[1u32, 2, 3]), hash_unordered(&[1u32, 2, 4]));
    }

    #[test]
    fn test_hash_unordered_empty_is_distinct_and_stable() {
        let empty: [u32; 0] = [];
        let h = hash_unordered(&empty);
        // Stable across calls.
        assert_eq!(h, hash_unordered(&empty));
        // Distinct from any single-element multiset tried here.
        assert_ne!(h, hash_unordered(&[0u32]));
        assert_ne!(h, hash_unordered(&[1u32]));
    }

    #[test]
    fn test_hash_unordered_distinguishes_cardinality_on_equal_sum() {
        // wrapping_add alone could collide two different-size multisets whose
        // element hashes sum equal; folding the count in must separate them.
        assert_ne!(hash_unordered(&[0u32]), hash_unordered(&[0u32, 0]));
    }

    #[test]
    fn test_hash_unordered_matches_across_container_types() {
        // A slice and a Vec of the same elements hash identically (both yield
        // &T items), and a HashMap's values (arbitrary iteration order) match
        // the sorted slice of the same values.
        use std::collections::HashMap;
        let v = vec![100u32, 200, 300];
        assert_eq!(hash_unordered(&v), hash_unordered(v.as_slice()));

        let mut m = HashMap::new();
        m.insert("a", 100u32);
        m.insert("b", 200);
        m.insert("c", 300);
        assert_eq!(
            hash_unordered(m.values()),
            hash_unordered(&[100u32, 200, 300]),
            "HashMap value order must not affect the result"
        );
    }

    #[test]
    fn test_hash_unordered_does_not_perturb_hash_state() {
        // Sanity: the new function is a free function; ordinary hash_state on a
        // slice is unchanged and still order-DEPENDENT (the contrast that
        // motivates hash_unordered).
        assert_ne!(
            hash_state(&[1u32, 2, 3][..]),
            hash_state(&[3u32, 2, 1][..]),
            "hash_state stays order-dependent"
        );
    }
}
