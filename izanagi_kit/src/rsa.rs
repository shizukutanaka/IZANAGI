//! Textbook RSA over [`BigInt`] — deterministic key generation,
//! raw `m^e mod n` encryption and `s^e` signature verification.
//!
//! "Textbook" means **no padding**: `encrypt` is a pure public
//! function of the message, which is exactly what a deterministic
//! toolkit wants (a replay must reproduce ciphertext bit-for-bit)
//! and exactly why this is *not* wire-grade crypto — do not use
//! it to protect real secrets.
//!
//! [`BigInt`] has no division, so the module supplies its own
//! magnitude arithmetic: binary long division for `rem`,
//! square-and-multiply `modpow`, extended Euclid `modinv`, and
//! fixed-witness Miller–Rabin `is_probable_prime` — all on
//! little-endian `u64` limbs.
//!
//! ```
//! use izanagi_kit::rsa::KeyPair;
//! let kp = KeyPair::generate(7, 128); // 128-bit modulus — toy size
//! let c = kp.encrypt(&izanagi_kit::bigint::BigInt::from_i64(65));
//! let m = kp.decrypt(&c);
//! assert_eq!(m.to_i128(), Some(65));
//! ```

use crate::bigint::BigInt;
use crate::rng::SplitMix64;

// ---- magnitude helpers (little-endian u64 limbs) ----------

/// Trim trailing zero limbs; empty means zero.
fn mnorm(a: &mut Vec<u64>) {
    while a.last() == Some(&0) {
        a.pop();
    }
}

fn mcmp(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        match a[i].cmp(&b[i]) {
            Equal => {}
            o => return o,
        }
    }
    Equal
}

/// `a - b`, requires `a >= b`.
fn msub(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0u64;
    for (i, &x) in a.iter().enumerate() {
        let y = if i < b.len() { b[i] } else { 0 };
        let (d1, b1) = x.overflowing_sub(y);
        let (d2, b2) = d1.overflowing_sub(borrow);
        out.push(d2);
        borrow = u64::from(b1 || b2);
    }
    mnorm(&mut out);
    out
}

/// Bit length of `a` (0 for zero).
fn mbits(a: &[u64]) -> u64 {
    match a.last() {
        None => 0,
        Some(&top) => (a.len() as u64 - 1) * 64 + (64 - top.leading_zeros() as u64),
    }
}

/// Bit `i` of `a`.
fn mbit(a: &[u64], i: u64) -> u64 {
    let limb = (i / 64) as usize;
    if limb >= a.len() {
        0
    } else {
        (a[limb] >> (i % 64)) & 1
    }
}

/// `a << i`.
fn mshl(a: &[u64], i: u64) -> Vec<u64> {
    if a.is_empty() {
        return Vec::new();
    }
    let limbs = (i / 64) as usize;
    let sh = (i % 64) as u32;
    let mut out = vec![0u64; limbs];
    for (k, &x) in a.iter().enumerate() {
        out.push(x << sh);
        if sh != 0 && k > 0 {
            let prev = a[k - 1];
            out[limbs + k] |= prev >> (64 - sh);
        }
    }
    if sh != 0 {
        // top limb spill
        let spill = a[a.len() - 1] >> (64 - sh);
        if spill != 0 {
            out.push(spill);
        }
    }
    mnorm(&mut out);
    out
}

/// `a % m` via binary long division — `m` must be non-zero.
/// Descend shifts from `bits(a) - bits(m)` to 0; at each step
/// `rem < m·2^(sh+1)` so at most one subtraction clears bit
/// `sh`'s contribution.
fn mrem(a: &[u64], m: &[u64]) -> Vec<u64> {
    if mcmp(a, m) == std::cmp::Ordering::Less {
        return a.to_vec();
    }
    let mut rem = a.to_vec();
    let mut sh = mbits(a) - mbits(m);
    loop {
        let shifted = mshl(m, sh);
        if mcmp(&rem, &shifted) != std::cmp::Ordering::Less {
            rem = msub(&rem, &shifted);
        }
        if sh == 0 {
            break;
        }
        sh -= 1;
    }
    rem
}

/// `(a * b) % m`.
fn mmulmod(a: &[u64], b: &[u64], m: &[u64]) -> Vec<u64> {
    // reuse BigInt's schoolbook mul via the public mul, then rem
    let p = BigInt::from_limbs(a).mul(&BigInt::from_limbs(b));
    mrem(p.limbs(), m)
}

/// `base^e mod m`, `e` given as limbs — square & multiply,
/// high bit to low.
fn mmodpow(base: &[u64], e: &[u64], m: &[u64]) -> Vec<u64> {
    let nb = mbits(e);
    let mut acc = vec![1u64]; // 1
    let base_r = mrem(base, m);
    for i in (0..nb).rev() {
        acc = mmulmod(&acc, &acc, m);
        if mbit(e, i) == 1 {
            acc = mmulmod(&acc, &base_r, m);
        }
    }
    acc
}

/// Extended-Euclid `a⁻¹ mod m` (m positive, gcd(a,m)==1).
/// Returns `None` when the gcd isn't 1.
fn modinv(a: &[u64], m: &[u64]) -> Option<Vec<u64>> {
    let mut old_r = m.to_vec();
    let mut r = mrem(a, m);
    let mut old_s = BigInt::zero();
    let mut s = BigInt::from_i64(1);
    while !r.is_empty() {
        // q = old_r / r — quotient magnitude via long division
        let q = mdiv(&old_r, &r);
        // (old_r, r) = (r, old_r - q·r)
        let qr = BigInt::from_limbs(&q).mul(&BigInt::from_limbs(&r));
        let new_r = BigInt::from_limbs(&old_r).sub(&qr);
        let new_r_limbs = if new_r.is_negative() {
            return None; // can't happen in Euclid
        } else {
            new_r.limbs().to_vec()
        };
        old_r = r;
        r = new_r_limbs;
        // (old_s, s) = (s, old_s - q·s)
        let qs = BigInt::from_limbs(&q).mul(&s);
        let new_s = old_s.sub(&qs);
        old_s = s;
        s = new_s;
    }
    // gcd must be 1
    if mcmp(&old_r, &[1]) != std::cmp::Ordering::Equal {
        return None;
    }
    // old_s mod m (can be negative)
    let mp = BigInt::from_limbs(m);
    let mut v = old_s;
    // v mod m via repeated add of m until non-negative, then rem
    while v.is_negative() {
        v = v.add(&mp);
    }
    Some(mrem(v.limbs(), m))
}

/// `a / m` quotient magnitude — binary long division.
fn mdiv(a: &[u64], m: &[u64]) -> Vec<u64> {
    let mut q = Vec::new();
    let mut rem = a.to_vec();
    while mcmp(&rem, m) != std::cmp::Ordering::Less {
        let dr = mbits(&rem);
        let dm = mbits(m);
        let mut sh = dr - dm;
        let shifted = mshl(m, sh);
        if mcmp(&rem, &shifted) == std::cmp::Ordering::Less {
            sh -= 1;
            rem = msub(&rem, &mshl(m, sh));
        } else {
            rem = msub(&rem, &shifted);
        }
        // set bit sh of q
        let limb = (sh / 64) as usize;
        while q.len() <= limb {
            q.push(0);
        }
        q[limb] |= 1u64 << (sh % 64);
    }
    mnorm(&mut q);
    q
}

/// Fixed-witness Miller–Rabin on limbs — deterministic; for
/// the bit sizes this crate targets the first 12 prime bases
/// are far beyond any counterexample.
fn is_probable_prime(n: &[u64]) -> bool {
    // small primes quick reject
    const SMALL: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    if n.is_empty() {
        return false;
    }
    if n.len() == 1 && n[0] < 2 {
        return false;
    }
    for &p in &SMALL {
        let r = mrem(n, &[p]);
        if r.is_empty() {
            // n divisible by p — prime only if n == p
            return mcmp(n, &[p]) == std::cmp::Ordering::Equal;
        }
    }
    // write n-1 = d · 2^s — clear the low bits by halving
    let nm1 = msub(n, &[1]);
    let mut dd = nm1.clone();
    let mut s = 0u64;
    while mbit(&dd, 0) == 0 {
        dd = mdiv(&dd, &[2]);
        s += 1;
    }
    for &a in &SMALL[..6] {
        // witnesses 2,3,5,7,11,13
        let x = mmodpow(&[a], &dd, n);
        if mcmp(&x, &[1]) == std::cmp::Ordering::Equal
            || mcmp(&x, &nm1) == std::cmp::Ordering::Equal
        {
            continue;
        }
        let mut ok = false;
        let mut x = x;
        for _ in 1..s {
            x = mmulmod(&x, &x, n);
            if mcmp(&x, &nm1) == std::cmp::Ordering::Equal {
                ok = true;
                break;
            }
        }
        if !ok {
            return false;
        }
    }
    true
}

/// An RSA key pair: `n = p·q`, `e` public exponent, `d` private.
pub struct KeyPair {
    n: BigInt,
    e: BigInt,
    d: BigInt,
}

impl KeyPair {
    /// Deterministic generation: draw `bits/2`-bit candidate
    /// primes from `SplitMix64(seed)`, fix top+low bits, keep the
    /// first `is_probable_prime` hit for `p` then `q` (rejection
    /// on `p == q`). `e` is 65537 unless it shares a factor with
    /// `φ(n)`, then 17, then 257.
    pub fn generate(seed: u64, bits: u64) -> Self {
        let half = (bits / 2).max(8);
        let p = gen_prime(&mut SplitMix64::new(seed), half);
        let q = gen_prime(&mut SplitMix64::new(seed ^ 0x9E37_79B9), half);
        let np = BigInt::from_limbs(&p).mul(&BigInt::from_limbs(&q));
        // φ = (p-1)(q-1)
        let one = BigInt::from_i64(1);
        let pm1 = BigInt::from_limbs(&p).sub(&one);
        let qm1 = BigInt::from_limbs(&q).sub(&one);
        let phi = pm1.mul(&qm1);
        for &e0 in &[65537u64, 257, 17, 5, 3] {
            let e = vec![e0];
            if let Some(d) = modinv(&e, phi.limbs()) {
                return KeyPair {
                    n: np.clone(),
                    e: BigInt::from_limbs(&e),
                    d: BigInt::from_limbs(&d),
                };
            }
        }
        // gcd failed for every candidate e — regenerate would
        // recurse forever on a pathological seed; return None
        // is not allowed by the signature, so fall back to a
        // different seed deterministically.
        KeyPair::generate(seed.wrapping_add(1), bits)
    }

    /// The modulus `n`.
    pub fn n(&self) -> BigInt {
        self.n.clone()
    }

    /// Public exponent.
    pub fn e(&self) -> BigInt {
        self.e.clone()
    }

    /// `m^e mod n` — raw encryption / signature verification.
    /// `m` is reduced into `[0, n)` first.
    pub fn encrypt(&self, m: &BigInt) -> BigInt {
        let r = BigInt::from_limbs(&mrem(m.limbs(), self.n.limbs()));
        BigInt::from_limbs(&mmodpow(r.limbs(), self.e.limbs(), self.n.limbs()))
    }

    /// `c^d mod n` — raw decryption / signing.
    pub fn decrypt(&self, c: &BigInt) -> BigInt {
        BigInt::from_limbs(&mmodpow(c.limbs(), self.d.limbs(), self.n.limbs()))
    }

    /// Sign `m` — textbook `s = m^d mod n`.
    pub fn sign(&self, m: &BigInt) -> BigInt {
        self.decrypt(m)
    }

    /// Verify `s` against `m`: `s^e mod n == m mod n`.
    pub fn verify(&self, m: &BigInt, s: &BigInt) -> bool {
        let mm = mrem(m.limbs(), self.n.limbs());
        let got = mmodpow(s.limbs(), self.e.limbs(), self.n.limbs());
        mcmp(&got, &mm) == std::cmp::Ordering::Equal
    }
}

/// Draw odd `bits`-bit candidates until `is_probable_prime`
/// accepts — pure function of the rng stream.
fn gen_prime(rng: &mut SplitMix64, bits: u64) -> Vec<u64> {
    let nlimbs = bits.div_ceil(64) as usize;
    loop {
        let mut c: Vec<u64> = (0..nlimbs).map(|_| rng.next_u64()).collect();
        // top bit set (exact bit length), low bit set (odd)
        let top = nlimbs - 1;
        let extra = bits % 64;
        if extra != 0 {
            c[top] &= (1u64 << extra) - 1;
        }
        c[top] |= 1u64 << ((bits - 1) % 64);
        c[0] |= 1;
        mnorm(&mut c);
        if is_probable_prime(&c) {
            return c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mag_arith_sanity() {
        // rem/div/mulmod/modpow on small values vs u128 oracle
        let cases: &[(u128, u128)] = &[
            (0, 7),
            (5, 7),
            (u128::from(u64::MAX), 97),
            (1u128 << 100, 65537),
        ];
        for &(a, m) in cases {
            let al = BigInt::from_i128(a as i128).limbs().to_vec();
            let ml = BigInt::from_i128(m as i128).limbs().to_vec();
            let r = mrem(&al, &ml);
            let want = (a % m) as u64; // m < 2^64 in all cases
            let got = if r.is_empty() { 0 } else { r[0] };
            assert_eq!(got, want, "{a} % {m}");
            let q = mdiv(&al, &ml);
            let wantq = a / m;
            let gotq = q
                .iter()
                .enumerate()
                .fold(0u128, |acc, (i, &l)| acc + ((l as u128) << (64 * i)));
            assert_eq!(gotq, wantq, "{a} / {m}");
        }
        // modpow against u64 oracle for small moduli
        fn powmod(mut b: u128, mut e: u64, m: u128) -> u128 {
            let mut acc = 1u128;
            while e > 0 {
                if e & 1 == 1 {
                    acc = acc * b % m;
                }
                b = b * b % m;
                e >>= 1;
            }
            acc
        }
        for e in [3u64, 17, 65537] {
            let got = mmodpow(&[7], &[e], &[3331]);
            let want = powmod(7, e, 3331);
            assert_eq!(got, vec![want as u64], "7^{e} mod 3331");
        }
    }

    #[test]
    fn keygen_roundtrip() {
        let kp = KeyPair::generate(42, 128);
        let m = BigInt::from_i64(42);
        let c = kp.encrypt(&m);
        assert_ne!(c, m);
        assert_eq!(kp.decrypt(&c), m);
        // signature roundtrip
        let s = kp.sign(&m);
        assert!(kp.verify(&m, &s));
        // tampered signature / message rejected
        assert!(!kp.verify(&BigInt::from_i64(43), &s));
        assert!(!kp.verify(&m, &s.add(&BigInt::from_i64(1))));
    }

    /// Same seed → same keys; different seeds → different
    /// moduli. Messages near `n` reduce mod n first.
    #[test]
    fn determinism_and_reduction() {
        let a = KeyPair::generate(9, 128);
        let b = KeyPair::generate(9, 128);
        let c = KeyPair::generate(10, 128);
        assert_eq!(a.n(), b.n());
        assert_ne!(a.n(), c.n());
        let m = BigInt::from_i64(1000);
        let enc = a.encrypt(&m);
        assert_eq!(a.decrypt(&enc), m);
        // m + n encrypts to the same class as m
        let m2 = m.add(&a.n());
        let enc2 = a.encrypt(&m2);
        assert_eq!(enc, enc2);
    }

    /// `is_probable_prime` agrees with trial division on a
    /// range where the oracle is cheap.
    #[test]
    fn oracle_primality_small() {
        fn trial(n: u64) -> bool {
            if n < 2 {
                return false;
            }
            let mut d = 2u64;
            while d * d <= n {
                if n % d == 0 {
                    return false;
                }
                d += 1;
            }
            true
        }
        for n in 2u64..4000 {
            assert_eq!(is_probable_prime(&[n]), trial(n), "n={n}");
        }
        // Carmichael numbers that fool single-witness tests
        for n in [561u64, 1105, 1729, 41041, 825265] {
            assert!(!is_probable_prime(&[n]), "Carmichael {n}");
        }
    }
}
