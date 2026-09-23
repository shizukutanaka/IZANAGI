//! Karatsuba multiplication over base-2^64 limbs — `O(n^1.585)`
//! products for multi-word unsigned integers where the schoolbook
//! method is `O(n²)` (Karatsuba & Ofman 1962). Limbs are little-endian
//! `u64` words (`limbs[0]` = least significant); every intermediate
//! uses `u128` so no rounding ever enters the result. Below a cut-off
//! the quadratic schoolbook wins, so [`mul`] dispatches on size.
//!
//! ```
//! use izanagi_kit::karatsuba::mul;
//! assert_eq!(mul(&[3], &[4]), vec![12]);
//! // (2^64 + 5) * (2^64 + 5) = 2^128 + 10·2^64 + 25
//! assert_eq!(mul(&[1, 1], &[1, 1]), vec![1, 2, 1]);
//! assert_eq!(mul(&[0xffff_ffff_ffff_ffff], &[2]), vec![0xffff_ffff_ffff_fffe, 1]);
//! ```

/// Below this limb count the quadratic schoolbook wins.
const CUTOFF: usize = 16;

fn norm(mut v: Vec<u64>) -> Vec<u64> {
    while v.last() == Some(&0) {
        v.pop();
    }
    v
}

/// Schoolbook `O(n·m)` product — the reference implementation.
pub fn schoolbook_mul(a: &[u64], b: &[u64]) -> Vec<u64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut r = vec![0u64; a.len() + b.len()];
    for (i, &ai) in a.iter().enumerate() {
        let mut carry = 0u128;
        for (j, &bj) in b.iter().enumerate() {
            let t = (ai as u128) * (bj as u128) + (r[i + j] as u128) + carry;
            r[i + j] = t as u64;
            carry = t >> 64;
        }
        let mut k = i + b.len();
        while carry != 0 {
            let t = (r[k] as u128) + carry;
            r[k] = t as u64;
            carry = t >> 64;
            k += 1;
        }
    }
    norm(r)
}

/// `a + b` as a magnitude (length max+1 at most).
fn add(a: &[u64], b: &[u64]) -> Vec<u64> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut r = Vec::with_capacity(long.len() + 1);
    let mut carry = 0u128;
    for i in 0..long.len() {
        let s = if i < short.len() { short[i] as u128 } else { 0 };
        let t = (long[i] as u128) + s + carry;
        r.push(t as u64);
        carry = t >> 64;
    }
    if carry != 0 {
        r.push(carry as u64);
    }
    r
}

/// `a - b` assuming `a >= b` (magnitudes).
fn sub(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut r = Vec::with_capacity(a.len());
    let mut borrow = 0i128;
    for i in 0..a.len() {
        let s = if i < b.len() { b[i] as i128 } else { 0 };
        let mut t = (a[i] as i128) - s - borrow;
        if t < 0 {
            t += 1i128 << 64;
            borrow = 1;
        } else {
            borrow = 0;
        }
        r.push(t as u64);
    }
    norm(r)
}

/// In-place `acc += b << (64·shift)` growing `acc` as needed.
fn add_shift(acc: &mut Vec<u64>, b: &[u64], shift: usize) {
    if b.is_empty() {
        return;
    }
    while acc.len() < shift + b.len() + 1 {
        acc.push(0);
    }
    let mut carry = 0u128;
    for (i, &bi) in b.iter().enumerate() {
        let t = (acc[shift + i] as u128) + (bi as u128) + carry;
        acc[shift + i] = t as u64;
        carry = t >> 64;
    }
    let mut k = shift + b.len();
    while carry != 0 {
        let t = (acc[k] as u128) + carry;
        acc[k] = t as u64;
        carry = t >> 64;
        k += 1;
    }
}

fn kara(a: &[u64], b: &[u64]) -> Vec<u64> {
    let n = a.len().max(b.len());
    if n < CUTOFF || a.is_empty() || b.is_empty() {
        return schoolbook_mul(a, b);
    }
    let m = n / 2;
    let a0 = &a[..a.len().min(m)];
    let a1 = if a.len() > m { &a[m..] } else { &[][..] };
    let b0 = &b[..b.len().min(m)];
    let b1 = if b.len() > m { &b[m..] } else { &[][..] };

    let z0 = kara(a0, b0);
    let z2 = kara(a1, b1);
    // z1 = (a0+a1)·(b0+b1) - z0 - z2; each partial sum may grow a
    // limb, and the difference is always non-negative.
    let sa = add(a0, a1);
    let sb = add(b0, b1);
    let z1 = sub(&sub(&kara(&sa, &sb), &z0), &z2);

    let mut r = z0;
    add_shift(&mut r, &z1, m);
    add_shift(&mut r, &z2, 2 * m);
    norm(r)
}

/// Big-unsigned product — picks Karatsuba or schoolbook by size.
/// Little-endian limb order, empty slice means zero.
pub fn mul(a: &[u64], b: &[u64]) -> Vec<u64> {
    kara(&norm(a.to_vec()), &norm(b.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn limbs(rng: &mut SplitMix64, n: usize) -> Vec<u64> {
        let mut v: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
        if let Some(last) = v.last_mut() {
            if *last == 0 {
                *last = 1;
            }
        }
        v
    }

    #[test]
    fn small_cases() {
        assert!(mul(&[], &[1]).is_empty());
        assert_eq!(mul(&[0], &[5]), Vec::<u64>::new());
        assert_eq!(mul(&[7], &[6]), vec![42]);
        // (2^64−1)² = 2^128 − 2^65 + 1 = [1, 0xfffffffffffffffe]
        assert_eq!(mul(&[!0u64], &[!0u64]), vec![1, 0xffff_ffff_ffff_fffe]);
    }

    #[test]
    fn oracle_schoolbook() {
        let mut rng = SplitMix64::new(11);
        for _ in 0..200 {
            let na = rng.below(40) as usize;
            let nb = rng.below(40) as usize;
            let a = limbs(&mut rng, na);
            let b = limbs(&mut rng, nb);
            assert_eq!(mul(&a, &b), schoolbook_mul(&a, &b));
        }
        // Asymmetric lengths force different split shapes.
        for _ in 0..100 {
            let na = rng.below(80) as usize;
            let nb = rng.below(5) as usize + 1;
            let a = limbs(&mut rng, na);
            let b = limbs(&mut rng, nb);
            assert_eq!(mul(&a, &b), schoolbook_mul(&a, &b));
        }
    }

    #[test]
    fn commutative_and_identity() {
        let mut rng = SplitMix64::new(5);
        for _ in 0..50 {
            let na = rng.below(30) as usize;
            let nb = rng.below(30) as usize;
            let a = limbs(&mut rng, na);
            let b = limbs(&mut rng, nb);
            assert_eq!(mul(&a, &b), mul(&b, &a));
            assert_eq!(mul(&a, &[1]), norm(a));
        }
    }

    #[test]
    fn boundary_cutoff() {
        // Sizes straddling CUTOFF recurse through both paths.
        let mut rng = SplitMix64::new(77);
        for n in [15usize, 16, 17, 33, 64] {
            let a = limbs(&mut rng, n);
            let b = limbs(&mut rng, n);
            assert_eq!(mul(&a, &b), schoolbook_mul(&a, &b));
        }
    }
}
