//! GF(2⁸) arithmetic — the finite field behind AES and Reed–Solomon.
//!
//! Elements are `u8`; field polynomial is `x⁸ + x⁴ + x³ + x + 1`
//! (AES's `0x11B`, generator `0x03`). Addition and subtraction are
//! both `^` (xor); multiplication is peasant-style with reduction;
//! `inv`/`div`/`pow` run through precomputed log/exp tables.
//!
//! All operations are pure functions of their operands — no state.
//! [`mul`] is a lookup-free peasant multiply; [`inv`], [`pow`] and
//! [`div`] build the 255-entry log/exp [`Tables`] on demand (O(255),
//! deterministic).
//!
//! ```
//! use izanagi_kit::gf2::{mul, inv, pow};
//! assert_eq!(mul(0x57, 0x83), 0xC1); // the classic AES check
//! for a in 1u8..=255 {
//!     assert_eq!(mul(a, inv(a)), 1); // every nonzero element inverts
//! }
//! assert_eq!(pow(0x02, 8), 0x1B);   // 2⁸ mod the AES polynomial
//! ```

/// Field polynomial `x⁸+x⁴+x³+x+1` without the leading bit.
const POLY: u16 = 0x11B;

/// Add two field elements — xor. `sub` is identical (characteristic 2).
pub fn add(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Subtract — identical to [`add`] in GF(2⁸).
pub fn sub(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Multiply — Russian-peasant over the field polynomial.
///
/// ```
/// use izanagi_kit::gf2::mul;
/// assert_eq!(mul(3, 7), 9); // 0b11 · 0b111 = x+1 times x²+x+1 → x³+1
/// ```
pub fn mul(a: u8, b: u8) -> u8 {
    let mut a = a as u16;
    let mut b = b;
    let mut acc = 0u16;
    while b != 0 {
        if b & 1 == 1 {
            acc ^= a;
        }
        b >>= 1;
        a <<= 1;
        if a & 0x100 != 0 {
            a ^= POLY;
        }
    }
    // acc can exceed a byte only if a reduction was missed — the loop
    // above reduces on every shift so a plain cast is exact.
    acc as u8
}

/// Log/exp tables over the multiplicative group (generator `0x03`).
///
/// `exp[i]` = generator^i for `i ∈ 0..255`; `log[x]` = discrete log of
/// nonzero `x` (index 0 is a placeholder — `log(0)` is undefined).
pub struct Tables {
    /// `exp[i]` — generator power table (period 255, doubled to 512
    /// so `exp[log[a] + log[b]]` needs no modular wrap).
    pub exp: [u8; 512],
    /// `log[x]` — discrete logarithm of `x` base `0x03`; `log[0]` is
    /// meaningless (never index it).
    pub log: [u8; 256],
}

impl Tables {
    /// Build the tables — O(255), deterministic.
    pub fn new() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x = 1u8;
        for (i, e) in exp.iter_mut().take(255).enumerate() {
            *e = x;
            log[x as usize] = i as u8;
            // x *= 3 (the generator) in GF(2^8): x·3 = x<<1 ^ x;
            // u16 keeps the carry bit for the reduction.
            let mut nx = ((x as u16) << 1) ^ (x as u16);
            if nx & 0x100 != 0 {
                nx ^= POLY;
            }
            x = nx as u8;
        }
        exp.copy_within(..255, 255);
        Tables { exp, log }
    }
}

impl Default for Tables {
    fn default() -> Self {
        Self::new()
    }
}

/// Table-driven multiply (`0` short-circuits since log(0) doesn't
/// exist). Equivalent to [`mul`].
pub fn mul_t(a: u8, b: u8, t: &Tables) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    t.exp[t.log[a as usize] as usize + t.log[b as usize] as usize]
}

/// Multiplicative inverse — `a·inv(a) = 1` for `a != 0`. `inv(0)` is
/// `0` (a convention — zero has no inverse; callers that need
/// strictness check for it first).
pub fn inv(a: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    let t = Tables::new();
    t.exp[255 - t.log[a as usize] as usize]
}

/// `aⁿ` in the field (`0⁰ = 1` by convention; `0ⁿ⁺¹ = 0`).
pub fn pow(a: u8, n: u32) -> u8 {
    if n == 0 {
        return 1;
    }
    if a == 0 {
        return 0;
    }
    let t = Tables::new();
    let l = t.log[a as usize] as u64;
    t.exp[((l * n as u64) % 255) as usize]
}

/// `a / b` — `None` when `b == 0` (division by zero is rejected rather
/// than silently wrong).
pub fn div(a: u8, b: u8) -> Option<u8> {
    if b == 0 {
        return None;
    }
    if a == 0 {
        return Some(0);
    }
    let t = Tables::new();
    let la = t.log[a as usize] as usize;
    let lb = t.log[b as usize] as usize;
    Some(t.exp[if la >= lb { la - lb } else { la + 255 - lb }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn aes_known_values() {
        assert_eq!(mul(0x57, 0x83), 0xC1);
        assert_eq!(mul(0x57, 0x13), 0xFE);
        assert_eq!(pow(2, 0), 1);
        assert_eq!(pow(2, 1), 2);
        assert_eq!(pow(2, 8), 0x1B); // x⁸ ≡ x⁴+x³+x+1
    }

    #[test]
    fn field_axioms_random() {
        let mut rng = SplitMix64::new(0x62A2);
        let t = Tables::new();
        for _ in 0..2000 {
            let (a, b, c) = (
                rng.below(256) as u8,
                rng.below(256) as u8,
                rng.below(256) as u8,
            );
            // Commutativity + associativity.
            assert_eq!(mul(a, b), mul(b, a));
            assert_eq!(add(a, b), add(b, a));
            assert_eq!(mul(mul(a, b), c), mul(a, mul(b, c)));
            // Distributivity.
            assert_eq!(mul(a, add(b, c)), add(mul(a, b), mul(a, c)));
            // Identity + inverse.
            assert_eq!(mul(a, 1), a);
            if a != 0 {
                assert_eq!(mul(a, inv(a)), 1);
                assert_eq!(div(a, a), Some(1));
                assert_eq!(div(mul(a, b), a), Some(b));
            }
            // Table multiply matches peasant multiply.
            assert_eq!(mul_t(a, b, &t), mul(a, b));
            // pow matches repeated multiplication.
            let n = rng.below(300);
            let mut acc = 1u8;
            for _ in 0..n {
                acc = mul(acc, a);
            }
            assert_eq!(pow(a, n), acc);
        }
        assert_eq!(div(5, 0), None);
        assert_eq!(inv(0), 0);
        assert_eq!(pow(0, 0), 1);
        assert_eq!(pow(0, 7), 0);
    }

    #[test]
    fn generator_covers_all_nonzero() {
        let t = Tables::new();
        let mut seen = [false; 256];
        for i in 0..255 {
            seen[t.exp[i] as usize] = true;
        }
        for (x, &s) in seen.iter().enumerate().skip(1) {
            assert!(s, "generator missed {x}");
        }
        assert!(!seen[0]);
    }
}
