//! Permutation algebra — `perm[i]` is the image of `i`. Bijection
//! checks, composition, inverse, cycle decomposition, sign, group
//! order, and a Lehmer-code `rank`/`unrank` bijection into
//! `0..n!` for `n ≤ 20` — the machinery behind turn orders,
//! card shuffle verification, and "is this replay's element order a
//! pure relabeling of that one's".
//!
//! ```
//! use izanagi_kit::perm::{apply, compose, inverse, sign, unrank, rank};
//! let p = unrank(5, 3); // 6th permutation of 3 elements
//! assert_eq!(rank(&p), Some(5));
//! assert_eq!(compose(&p, &inverse(&p)), Some(vec![0, 1, 2]));
//! ```

/// The `n`-element identity.
pub fn identity(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

/// Whether `p` is a bijection of `0..n` — every value appears exactly
/// once and in range.
pub fn is_valid(p: &[u32]) -> bool {
    let n = p.len();
    let mut seen = vec![false; n];
    p.iter().all(|&x| {
        let x = x as usize;
        x < n && !std::mem::replace(&mut seen[x], true)
    })
}

/// `a ∘ b`: apply `b` first — `compose(a,b)[i] = a[b[i]]`.
/// `None` when either is not a valid permutation of `0..n`
/// (mismatched or invalid lengths).
pub fn compose(a: &[u32], b: &[u32]) -> Option<Vec<u32>> {
    if a.len() != b.len() || !is_valid(a) || !is_valid(b) {
        return None;
    }
    Some(b.iter().map(|&x| a[x as usize]).collect())
}

/// `p⁻¹`: the unique inverse.
pub fn inverse(p: &[u32]) -> Vec<u32> {
    let mut inv = vec![0u32; p.len()];
    for (i, &x) in p.iter().enumerate() {
        if (x as usize) < p.len() {
            inv[x as usize] = i as u32;
        }
    }
    inv
}

/// Cycle decomposition: each cycle listed starting at its smallest
/// element, cycles ordered by that start — a canonical form, so the
/// output is input-order independent.
pub fn cycles(p: &[u32]) -> Vec<Vec<u32>> {
    let n = p.len();
    let mut seen = vec![false; n];
    let mut out: Vec<Vec<u32>> = Vec::new();
    for s in 0..n {
        if seen[s] {
            continue;
        }
        let mut cyc = Vec::new();
        let mut v = s;
        while !seen[v] {
            seen[v] = true;
            cyc.push(v as u32);
            let nv = p.get(v).copied().unwrap_or(v as u32) as usize;
            if nv >= n {
                break; // invalid permutation — stop the walk
            }
            v = nv;
        }
        // Rotate so the smallest element leads.
        if let Some(m) = cyc
            .iter()
            .position(|&x| x == cyc.iter().copied().min().unwrap_or(0))
        {
            cyc.rotate_left(m);
        }
        out.push(cyc);
    }
    out.sort_by_key(|c| c.first().copied().unwrap_or(0));
    out
}

/// Permutation sign: `+1` even, `−1` odd — `(-1)^(n − #cycles)`.
pub fn sign(p: &[u32]) -> i64 {
    let n = p.len();
    if n == 0 {
        return 1;
    }
    let nc = cycles(p).len() as i64;
    if (n as i64 - nc) % 2 == 0 {
        1
    } else {
        -1
    }
}

/// Group order of `p`: lcm of its cycle lengths.
pub fn order(p: &[u32]) -> u64 {
    cycles(p)
        .iter()
        .map(|c| c.len() as u64)
        .fold(1u64, |acc, l| {
            let g = crate::ntheory::gcd(acc as i64, l as i64) as u64;
            acc / g * l
        })
}

/// Reorder `items` by `p`: `out[i] = items[p[i]]` — `None` when `p`
/// is not a valid permutation of `0..items.len()`.
pub fn apply<T: Clone>(p: &[u32], items: &[T]) -> Option<Vec<T>> {
    if p.len() != items.len() || !is_valid(p) {
        return None;
    }
    Some(p.iter().map(|&x| items[x as usize].clone()).collect())
}

/// Lehmer rank of `p` in `0..n!` — `None` for `n > 20` (factorial
/// overflow) or an invalid permutation.
pub fn rank(p: &[u32]) -> Option<u64> {
    let n = p.len();
    if n > 20 || !is_valid(p) {
        return None;
    }
    let mut r = 0u64;
    for i in 0..n {
        let smaller = p[i + 1..].iter().filter(|&&x| x < p[i]).count() as u64;
        r += smaller * fact(n - 1 - i);
    }
    Some(r)
}

/// The `r`-th permutation of `0..n` under `rank` — `None` for
/// `n > 20` or `r ≥ n!`.
pub fn unrank(mut r: u64, n: usize) -> Vec<u32> {
    if n > 20 || (n > 0 && r >= fact(n)) {
        return Vec::new();
    }
    if n == 0 {
        return Vec::new();
    }
    let mut avail: Vec<u32> = (0..n as u32).collect();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let f = fact(n - 1 - i);
        let k = (r / f) as usize;
        r %= f;
        out.push(avail.remove(k.min(avail.len() - 1)));
    }
    out
}

fn fact(n: usize) -> u64 {
    (1..=n as u64).product()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn rand_perm(rng: &mut SplitMix64, n: usize) -> Vec<u32> {
        let mut p = identity(n);
        for i in (1..n).rev() {
            let j = rng.below(i as u32 + 1) as usize;
            p.swap(i, j);
        }
        p
    }

    #[test]
    fn algebra_axioms() {
        let mut rng = SplitMix64::new(0x9E37_5EED);
        for _ in 0..300 {
            let n = rng.below(9) as usize;
            let (a, b, c) = (
                rand_perm(&mut rng, n),
                rand_perm(&mut rng, n),
                rand_perm(&mut rng, n),
            );
            // Associativity.
            assert_eq!(
                compose(&compose(&a, &b).unwrap_or_default(), &c),
                compose(&a, &compose(&b, &c).unwrap_or_default())
            );
            // Inverse + identity.
            assert_eq!(compose(&a, &inverse(&a)), Some(identity(n)));
            assert_eq!(compose(&a, &identity(n)), Some(a.clone()));
            // p^order = e.
            let mut acc = a.clone();
            for _ in 1..order(&a) {
                acc = compose(&acc, &a).unwrap_or_default();
            }
            assert_eq!(acc, identity(n), "p^{} != e for {a:?}", order(&a));
            // Sign = (-1)^inversions.
            let inv: i64 = (0..n)
                .map(|i| (i + 1..n).filter(|&j| a[i] > a[j]).count() as i64)
                .sum();
            assert_eq!(sign(&a), if inv % 2 == 0 { 1 } else { -1 });
        }
    }

    #[test]
    fn rank_unrank_is_bijection() {
        for n in 0..=7usize {
            let f = fact(n);
            let mut seen = BTreeSet::new();
            for r in 0..f {
                let p = unrank(r, n);
                assert_eq!(rank(&p), Some(r));
                seen.insert(p);
            }
            assert_eq!(seen.len() as u64, f, "unrank not bijective at n={n}");
        }
        assert_eq!(unrank(120, 5), Vec::<u32>::new()); // r ≥ 5!
        assert_eq!(rank(&[1, 1, 0]), None); // not a bijection
    }

    #[test]
    fn apply_and_cycles() {
        let p = vec![2, 0, 1];
        assert_eq!(apply(&p, &[10, 20, 30]), Some(vec![30, 10, 20]));
        assert_eq!(cycles(&p), vec![vec![0, 2, 1]]);
        assert_eq!(order(&p), 3);
        assert_eq!(apply(&p, &[10, 20]), None);
        assert_eq!(
            cycles(&identity(4)),
            vec![vec![0], vec![1], vec![2], vec![3]]
        );
        // Invalid permutations still terminate cycles (no infinite walk).
        assert!(cycles(&[0, 9]).len() <= 2);
    }
}
