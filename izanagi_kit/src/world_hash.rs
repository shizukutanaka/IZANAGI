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
//!
//! # Hash stability policy
//!
//! A state hash is only useful if it means the same thing tomorrow. Two
//! different promises are at stake, and they have different guarantees:
//!
//! - **Across machines, same code — guaranteed.** Every hash here is a
//!   function of integer bytes folded in a fixed order, with no float, no
//!   wall-clock, no pointer values and no unordered iteration. The same state
//!   hashes identically on any target the crate compiles for. This is the
//!   promise lockstep and replay depend on, and it is non-negotiable.
//! - **Across crate versions — not promised by default.** Adding a field to a
//!   type, or changing the order its bytes are folded, changes its hash. Such
//!   a change is *not* a silent bug fix: it invalidates recorded traces and
//!   saved games produced by an earlier version.
//!
//! So when a `DetHash` impl in this crate changes shape, the change is treated
//! as breaking for any persisted artefact: bump the [`savefile`] header version
//! so old saves are rejected (or migrated) loudly rather than loading into a
//! world whose hash no longer matches, and re-pin any hard-coded regression
//! hashes in the same commit. Callers who persist hashes long-term should
//! record the crate version alongside them.
//!
//! When a hash *does* mismatch, [`LabeledDigest`] narrows it to a subsystem
//! instead of a single opaque number, and [`hash_covers`] catches the opposite
//! failure — a hash that wrongly stayed the *same* because a field was never
//! folded in.
//!
//! [`savefile`]: crate::savefile

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
    /// Start a new hasher at the FNV-1a offset basis.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold raw bytes into the hash, one FNV-1a xor-then-multiply step per byte.
    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.hash ^= b as u64;
            self.hash = self.hash.wrapping_mul(FNV_PRIME);
        }
    }

    /// Hash a 32-bit unsigned integer in little-endian byte order.
    #[inline]
    pub fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    /// Hash a 64-bit unsigned integer in little-endian byte order.
    #[inline]
    pub fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    /// Hash a signed 32-bit integer in little-endian byte order.
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

    /// Return the accumulated hash without consuming the hasher.
    #[inline]
    pub fn finish(&self) -> u64 {
        self.hash
    }
}

/// Folds a value's canonical bytes into a hasher.
pub trait DetHash {
    /// Fold `self`'s canonical byte representation into `hasher`.
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

/// [`hash_state`] with an avalanche finalizer applied — a better-distributed
/// checksum of the same state, at the cost of **not** matching `hash_state`.
///
/// # Why this exists
///
/// FNV-1a folds each byte as `hash ^= b; hash *= PRIME`, so the *last* byte
/// written gets exactly one multiply to spread its influence. Measured over
/// random inputs, a single-bit change flips this many of the 64 output bits
/// (ideal is 32):
///
/// | input | `hash_state` | `hash_state_mixed` |
/// |---|---|---|
/// | 4 B | avg 20.3, worst 6 | avg 32.0, worst 17 |
/// | 8 B | avg 25.6, worst 6 | avg 32.0, worst 17 |
/// | 64 B | avg 30.1, worst 6 | avg 32.0, worst 14 |
///
/// A single-bit flip in the final byte moves only ~9 bits on average.
///
/// **This is a distribution-quality gap, not a correctness bug.** Any state
/// difference still changes the hash — the worst observed case flips 6 bits,
/// never 0 — so desync *detection* works exactly as documented. What the weak
/// tail costs is collision probability across large numbers of structured
/// states, which stays negligible at 64 bits for non-adversarial use but is
/// further from the ideal birthday bound than it needs to be.
///
/// So this is offered as an opt-in rather than folded into [`hash_state`]:
/// changing that would alter every hash the crate has ever produced,
/// invalidating recorded traces and saved games (see the hash stability policy
/// in the module docs). Prefer `hash_state_mixed` for **new** long-lived
/// checksums; a future major version may make it the default, paired with a
/// [`savefile`](crate::savefile) header bump.
///
/// The finalizer is the SplitMix64 mix — the same one
/// [`hash_unordered`] already applies for the same reason.
///
/// ```
/// use izanagi_kit::world_hash::{hash_state, hash_state_mixed};
/// // Same input, deliberately different value: pick one and stay with it.
/// assert_ne!(hash_state(&1u32), hash_state_mixed(&1u32));
/// // Still a pure function of the state.
/// assert_eq!(hash_state_mixed(&7u64), hash_state_mixed(&7u64));
/// ```
#[inline]
pub fn hash_state_mixed<T: DetHash + ?Sized>(value: &T) -> u64 {
    mix64(hash_state(value))
}

/// Does mutating `base` with `mutate` change its [`hash_state`]? — a
/// **mutation test** for a hand-written [`DetHash`] impl.
///
/// `DetHash` is written by hand: add a field to a struct, forget to fold it in
/// `det_hash`, and nothing breaks at the time. The state hash silently stops
/// being a faithful summary of the state, and the bug only surfaces much later
/// as an unexplained replay divergence — the hardest failure this kit can
/// produce, because the tick it is *reported* at is not the tick it was
/// *introduced* at. This is the classic mutation-testing argument (DeMillo,
/// Lipton & Sayward, "Hints on Test Data Selection", IEEE Computer 1978;
/// surveyed by Jia & Harman, IEEE TSE 2011) turned on the hash itself: inject a
/// small change and check the oracle notices it. A **surviving mutant** — a
/// field you can change without moving the hash — is exactly a field
/// `det_hash` forgot.
///
/// Returns `true` when the hash changed (that field is covered) and `false`
/// when it did not. `base` is left untouched: the mutation runs on a clone.
///
/// ```
/// use izanagi_kit::world_hash::{hash_covers, DetHash, Fnv1a};
///
/// #[derive(Clone)]
/// struct Player { hp: u32, gold: u32 }
/// impl DetHash for Player {
///     fn det_hash(&self, h: &mut Fnv1a) {
///         h.write_u32(self.hp);
///         // BUG: `gold` is never folded in.
///     }
/// }
/// let p = Player { hp: 100, gold: 5 };
/// assert!(hash_covers(&p, |p| p.hp += 1));
/// assert!(!hash_covers(&p, |p| p.gold += 1), "caught: gold is not hashed");
/// ```
pub fn hash_covers<T: DetHash + Clone>(base: &T, mutate: impl FnOnce(&mut T)) -> bool {
    let before = hash_state(base);
    let mut probe = base.clone();
    mutate(&mut probe);
    hash_state(&probe) != before
}

/// One named field probe for [`field_coverage`] / [`uncovered_fields`]: the
/// field's label, and a function that changes **only that field**. Written as a
/// non-capturing closure at the call site — `("hp", |s: &mut S| s.hp += 1)` —
/// which coerces to this `fn` pointer, keeping the probe list a plain slice
/// with no `dyn` or lifetime plumbing.
pub type FieldMutator<T> = (&'static str, fn(&mut T));

/// Run [`hash_covers`] over a set of named field mutators, returning
/// `(label, covered)` per entry in the order given — a coverage report for a
/// hand-written [`DetHash`] impl.
///
/// Write one mutator per field; any `false` names a field `det_hash` forgot to
/// fold in. Non-capturing closures coerce to the `fn` pointer this takes, so
/// the call site stays a plain list. See [`uncovered_fields`] for the
/// assertion-shaped form.
///
/// ```
/// use izanagi_kit::world_hash::{field_coverage, DetHash, Fnv1a};
///
/// #[derive(Clone)]
/// struct S { a: u32, b: u32 }
/// impl DetHash for S {
///     fn det_hash(&self, h: &mut Fnv1a) { h.write_u32(self.a); h.write_u32(self.b); }
/// }
/// let report = field_coverage(&S { a: 1, b: 2 }, &[
///     ("a", |s: &mut S| s.a += 1),
///     ("b", |s: &mut S| s.b += 1),
/// ]);
/// assert!(report.iter().all(|&(_, covered)| covered));
/// ```
pub fn field_coverage<T: DetHash + Clone>(
    base: &T,
    mutators: &[FieldMutator<T>],
) -> Vec<(&'static str, bool)> {
    mutators
        .iter()
        .map(|&(label, m)| (label, hash_covers(base, m)))
        .collect()
}

/// The labels from `mutators` whose mutation left [`hash_state`] unchanged —
/// i.e. the fields the [`DetHash`] impl fails to cover. Empty means every
/// probed field is hashed, which makes this the one-line regression guard:
///
/// ```
/// use izanagi_kit::world_hash::{uncovered_fields, DetHash, Fnv1a};
///
/// #[derive(Clone)]
/// struct S { a: u32, b: u32 }
/// impl DetHash for S {
///     fn det_hash(&self, h: &mut Fnv1a) { h.write_u32(self.a); h.write_u32(self.b); }
/// }
/// assert!(uncovered_fields(&S { a: 1, b: 2 }, &[
///     ("a", |s: &mut S| s.a += 1),
///     ("b", |s: &mut S| s.b += 1),
/// ]).is_empty(), "every field must be folded into det_hash");
/// ```
///
/// Adding a field to the struct and a line here, but forgetting the
/// `det_hash` update, turns the omission into a **test failure at the commit
/// that introduces it** rather than a replay divergence weeks later.
pub fn uncovered_fields<T: DetHash + Clone>(
    base: &T,
    mutators: &[FieldMutator<T>],
) -> Vec<&'static str> {
    mutators
        .iter()
        .filter(|&&(_, m)| !hash_covers(base, m))
        .map(|&(label, _)| label)
        .collect()
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

/// A labeled, per-subsystem hash breakdown of a composite world state.
///
/// [`hash_state`] collapses an entire world to a single `u64`: it tells you
/// *that* two states differ, never *where*. When a lockstep desync fires, the
/// first question is always "which subsystem drifted?" — and answering it from
/// a single number means bisecting by hand. A `LabeledDigest` keeps one named
/// hash per subsystem instead, so [`replay::first_divergence_labeled`] can
/// report the diverging subsystem, not just the diverging tick.
///
/// This mirrors how production determinism debugging actually works: Factorio's
/// desync reports compare per-subsystem CRCs (FFF-188), `bevy_ggrs` checksums
/// per entity, and the incremental-multiset-hash literature all converge on the
/// same idea — hash in labeled pieces, not one opaque blob.
///
/// [`root`](LabeledDigest::root) folds every `(label, hash)` pair, in insertion
/// order, into one `u64` — order-sensitive and stable, so a `LabeledDigest`
/// doubles as a whole-state hash usable anywhere a plain `hash_state` value is.
///
/// [`replay::first_divergence_labeled`]: crate::replay::first_divergence_labeled
///
/// ```
/// use izanagi_kit::world_hash::LabeledDigest;
/// let mut d = LabeledDigest::new();
/// d.add("positions", &[1u32, 2, 3][..]).add("hp", &[100u32, 80][..]);
/// assert_eq!(d.parts().len(), 2);
/// assert_eq!(d.parts()[0].0, "positions");
/// // root() is stable and order-sensitive.
/// assert_eq!(d.root(), LabeledDigest::from_parts(d.parts()).root());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LabeledDigest {
    parts: Vec<(&'static str, u64)>,
}

impl LabeledDigest {
    /// An empty digest.
    pub fn new() -> Self {
        LabeledDigest { parts: Vec::new() }
    }

    /// Reconstruct a digest from an existing `(label, hash)` slice (e.g. a
    /// snapshot restored from a trace). Preserves order.
    pub fn from_parts(parts: &[(&'static str, u64)]) -> Self {
        LabeledDigest {
            parts: parts.to_vec(),
        }
    }

    /// Hash `value` under `label` and append it. Chainable.
    pub fn add<T: DetHash + ?Sized>(&mut self, label: &'static str, value: &T) -> &mut Self {
        self.parts.push((label, hash_state(value)));
        self
    }

    /// Append a pre-computed subsystem hash under `label` (for callers that
    /// already have a `u64`, e.g. from [`hash_unordered`] or a nested digest's
    /// [`root`](Self::root)). Chainable.
    pub fn add_raw(&mut self, label: &'static str, hash: u64) -> &mut Self {
        self.parts.push((label, hash));
        self
    }

    /// The `(label, hash)` pairs in insertion order.
    pub fn parts(&self) -> &[(&'static str, u64)] {
        &self.parts
    }

    /// Fold every part (label bytes, then hash, in insertion order) into a
    /// single checksum — order-sensitive, so reordering subsystems changes it.
    pub fn root(&self) -> u64 {
        let mut h = Fnv1a::new();
        for (label, hash) in &self.parts {
            h.write_str(label);
            h.write_u64(*hash);
        }
        h.finish()
    }
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
        assert_eq!(
            h_trait,
            h_manual.finish(),
            "DetHash for str must be write_u32(len) ++ raw bytes"
        );
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

    // --- LabeledDigest ---

    #[test]
    fn test_labeled_digest_records_parts_in_order() {
        let mut d = LabeledDigest::new();
        d.add("a", &1u32).add("b", &2u32).add_raw("c", 0xABCD);
        let parts = d.parts();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].0, "a");
        assert_eq!(parts[1].0, "b");
        assert_eq!(parts[2], ("c", 0xABCD));
        // add() hashes via hash_state, so part 0 equals hash_state(&1u32).
        assert_eq!(parts[0].1, hash_state(&1u32));
    }

    #[test]
    fn test_labeled_digest_root_is_order_sensitive() {
        let mut a = LabeledDigest::new();
        a.add("x", &10u32).add("y", &20u32);
        let mut b = LabeledDigest::new();
        b.add("y", &20u32).add("x", &10u32); // same parts, swapped order
        assert_ne!(a.root(), b.root(), "reordering subsystems must change root");
    }

    #[test]
    fn test_labeled_digest_root_stable_and_reconstructible() {
        let mut d = LabeledDigest::new();
        d.add("pos", &[1u32, 2, 3][..]).add("hp", &99u32);
        let root = d.root();
        // Reconstructing from the same parts reproduces the same root.
        assert_eq!(LabeledDigest::from_parts(d.parts()).root(), root);
        // And recomputing on a fresh identical build matches.
        let mut d2 = LabeledDigest::new();
        d2.add("pos", &[1u32, 2, 3][..]).add("hp", &99u32);
        assert_eq!(d2.root(), root);
    }

    #[test]
    fn test_labeled_digest_root_reflects_a_changed_subsystem() {
        let mut a = LabeledDigest::new();
        a.add("pos", &[1u32, 2][..]).add("hp", &99u32);
        let mut b = LabeledDigest::new();
        b.add("pos", &[1u32, 2][..]).add("hp", &98u32); // hp differs
        assert_ne!(a.root(), b.root());
        // But the pos part is identical in both.
        assert_eq!(a.parts()[0], b.parts()[0]);
        assert_ne!(a.parts()[1], b.parts()[1]);
    }

    #[test]
    fn test_labeled_digest_empty_root_is_offset_basis() {
        // An empty digest folds nothing, so root() is the FNV offset basis.
        assert_eq!(LabeledDigest::new().root(), FNV_OFFSET);
    }

    // --- avalanche characteristics (see hash_state_mixed docs) ---

    struct Blob(Vec<u8>);

    impl DetHash for Blob {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_bytes(&self.0);
        }
    }

    /// Average and worst-case output bits flipped by a single-bit input flip,
    /// over `samples` random inputs of `len` bytes. Ideal is 32 of 64.
    fn avalanche(len: usize, samples: usize, mixed: bool) -> (f64, u32) {
        let mut rng = crate::rng::SplitMix64::new(0xA5A1_0CE5);
        let hash = |b: &Blob| {
            if mixed {
                hash_state_mixed(b)
            } else {
                hash_state(b)
            }
        };
        let (mut total, mut count, mut worst) = (0u64, 0u64, 64u32);
        for _ in 0..samples {
            let base: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
            let h0 = hash(&Blob(base.clone()));
            for byte in 0..len {
                for bit in 0..8u8 {
                    let mut m = base.clone();
                    m[byte] ^= 1 << bit;
                    let d = (h0 ^ hash(&Blob(m))).count_ones();
                    total += d as u64;
                    count += 1;
                    worst = worst.min(d);
                }
            }
        }
        (total as f64 / count as f64, worst)
    }

    #[test]
    fn test_single_bit_input_change_always_changes_the_hash() {
        // The property desync detection actually relies on: any difference in
        // the state must move the hash. Weak avalanche is a distribution
        // concern; a *zero* difference would be a correctness bug.
        for len in [1usize, 4, 8, 33] {
            let (_, worst) = avalanche(len, 40, false);
            assert!(
                worst > 0,
                "a single-bit input flip left the hash unchanged at len {len}"
            );
        }
    }

    #[test]
    fn test_fnv1a_avalanche_is_weak_in_the_tail() {
        // Pins the measured weakness documented on hash_state_mixed, so a
        // future change to the hash surfaces here rather than silently.
        // Ideal is 32.0; raw FNV-1a lands well short, especially for short
        // inputs, because the final byte gets only one multiply.
        let (avg4, worst4) = avalanche(4, 60, false);
        let (avg64, worst64) = avalanche(64, 20, false);
        assert!(
            (15.0..26.0).contains(&avg4),
            "4-byte avalanche moved: {avg4:.2}"
        );
        assert!(
            (26.0..32.0).contains(&avg64),
            "64-byte avalanche moved: {avg64:.2}"
        );
        assert!(worst4 < 16, "worst-case 4-byte flip: {worst4}");
        assert!(worst64 < 16, "worst-case 64-byte flip: {worst64}");
    }

    #[test]
    fn test_mixed_hash_reaches_ideal_avalanche() {
        // The finalizer's payoff: average lands on the ideal 32, and the worst
        // case improves substantially over raw FNV-1a.
        for len in [4usize, 8, 64] {
            let (avg, worst) = avalanche(len, 40, true);
            assert!(
                (30.5..33.5).contains(&avg),
                "len {len}: mixed avalanche {avg:.2} is not near the ideal 32"
            );
            assert!(worst >= 10, "len {len}: mixed worst case only {worst} bits");
        }
    }

    #[test]
    fn test_mixed_hash_is_deterministic_and_distinct_from_raw() {
        assert_eq!(hash_state_mixed(&123u64), hash_state_mixed(&123u64));
        assert_ne!(hash_state_mixed(&123u64), hash_state(&123u64));
        // Still injective enough to separate neighbouring values.
        assert_ne!(hash_state_mixed(&1u32), hash_state_mixed(&2u32));
    }

    #[test]
    fn test_mixed_hash_does_not_change_hash_state() {
        // Guard the stability promise: adding the mixed variant must not have
        // perturbed the existing checksum for any value.
        let mut h = Fnv1a::new();
        42u32.det_hash(&mut h);
        assert_eq!(hash_state(&42u32), h.finish());
    }

    // --- hash_covers / field_coverage / uncovered_fields ---

    /// A correct DetHash impl: every field is folded in.
    #[derive(Clone)]
    struct Complete {
        hp: u32,
        gold: u32,
        name: u8,
    }

    impl DetHash for Complete {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_u32(self.hp);
            h.write_u32(self.gold);
            h.write_bytes(&[self.name]);
        }
    }

    /// The bug this feature exists to catch: `gold` is never hashed, so a
    /// state change to it is invisible to the checksum.
    #[derive(Clone)]
    struct Incomplete {
        hp: u32,
        gold: u32,
    }

    impl DetHash for Incomplete {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_u32(self.hp);
            // `gold` deliberately omitted.
        }
    }

    fn complete() -> Complete {
        Complete {
            hp: 100,
            gold: 5,
            name: b'x',
        }
    }

    #[test]
    fn test_hash_covers_true_for_hashed_field() {
        assert!(hash_covers(&complete(), |s| s.hp += 1));
        assert!(hash_covers(&complete(), |s| s.gold += 1));
        assert!(hash_covers(&complete(), |s| s.name = b'y'));
    }

    #[test]
    fn test_hash_covers_catches_forgotten_field() {
        // The headline case: a surviving mutant means det_hash is incomplete.
        let s = Incomplete { hp: 100, gold: 5 };
        assert!(hash_covers(&s, |s| s.hp += 1), "hp is hashed");
        assert!(
            !hash_covers(&s, |s| s.gold += 1),
            "gold is NOT hashed — this is the silent-desync bug"
        );
    }

    #[test]
    fn test_hash_covers_leaves_base_untouched() {
        let base = complete();
        let before = hash_state(&base);
        let _ = hash_covers(&base, |s| s.hp = 999);
        assert_eq!(base.hp, 100, "mutation must run on a clone");
        assert_eq!(hash_state(&base), before);
    }

    #[test]
    fn test_hash_covers_no_op_mutator_is_false() {
        // A mutator that changes nothing cannot prove coverage — no false
        // positives from a hash that simply never moves.
        assert!(!hash_covers(&complete(), |_s| {}));
    }

    #[test]
    fn test_field_coverage_reports_each_label_in_order() {
        let report = field_coverage(
            &complete(),
            &[
                ("hp", |s: &mut Complete| s.hp += 1),
                ("gold", |s: &mut Complete| s.gold += 1),
                ("name", |s: &mut Complete| s.name = b'z'),
            ],
        );
        assert_eq!(
            report,
            vec![("hp", true), ("gold", true), ("name", true)],
            "labels preserved in order, all covered"
        );
    }

    #[test]
    fn test_field_coverage_flags_only_the_missing_field() {
        let report = field_coverage(
            &Incomplete { hp: 1, gold: 2 },
            &[
                ("hp", |s: &mut Incomplete| s.hp += 1),
                ("gold", |s: &mut Incomplete| s.gold += 1),
            ],
        );
        assert_eq!(report, vec![("hp", true), ("gold", false)]);
    }

    #[test]
    fn test_uncovered_fields_empty_for_complete_impl() {
        let missing = uncovered_fields(
            &complete(),
            &[
                ("hp", |s: &mut Complete| s.hp += 1),
                ("gold", |s: &mut Complete| s.gold += 1),
                ("name", |s: &mut Complete| s.name = b'z'),
            ],
        );
        assert!(missing.is_empty(), "complete impl has no gaps: {missing:?}");
    }

    #[test]
    fn test_uncovered_fields_names_the_gap() {
        let missing = uncovered_fields(
            &Incomplete { hp: 1, gold: 2 },
            &[
                ("hp", |s: &mut Incomplete| s.hp += 1),
                ("gold", |s: &mut Incomplete| s.gold += 1),
            ],
        );
        assert_eq!(missing, vec!["gold"]);
    }

    #[test]
    fn test_field_coverage_empty_mutator_list() {
        assert!(field_coverage(&complete(), &[]).is_empty());
        assert!(uncovered_fields(&complete(), &[]).is_empty());
    }

    #[test]
    fn test_hash_covers_is_deterministic() {
        let s = complete();
        let run = || hash_covers(&s, |s| s.gold += 7);
        assert_eq!(run(), run());
        assert!(run());
    }
}
