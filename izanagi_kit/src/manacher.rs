//! Manacher's algorithm — `O(n)` palindrome structure over bytes:
//! `odd_radii`/`even_radii` (the classic `d1`/`d2` arrays),
//! `longest_palindrome`, and `count_palindromes`. The string-layer
//! complement to [`suffix`](crate::suffix): where suffix arrays index
//! *where* a pattern occurs, Manacher answers the *symmetry*
//! structure — useful for procedural-name linting, seed prettiness
//! checks, and palindromic quest IDs.
//!
//! ```
//! use izanagi_kit::manacher::{longest_palindrome, odd_radii, even_radii};
//! assert_eq!(longest_palindrome(b"forgeeksskeegfor"), Some((3, 10)));
//! assert_eq!(odd_radii(b"aba"), vec![1, 2, 1]);
//! assert_eq!(even_radii(b"abba"), vec![0, 0, 2, 0]);
//! ```

/// Odd-length radii: `d1[i]` = largest `k` such that
/// `data[i−k+1..i+k]` is a palindrome (center `i`, total length
/// `2k−1`). `d1[i] ≥ 1` always — a single byte is one.
pub fn odd_radii(data: &[u8]) -> Vec<usize> {
    let n = data.len();
    let mut d = vec![0usize; n];
    let (mut l, mut r) = (0usize, 0usize); // palindrome [l, r)
    for i in 0..n {
        let mut k = if i >= r {
            1
        } else {
            d[l + r - 1 - i].min(r - i)
        };
        while i + k < n && i >= k && data[i - k] == data[i + k] {
            k += 1;
        }
        d[i] = k;
        if i + k > r {
            l = i + 1 - k;
            r = i + k;
        }
    }
    d
}

/// Even-length radii: `d2[i]` = largest `k` such that
/// `data[i−k..i+k]` is a palindrome (center between `i−1` and `i`,
/// total length `2k`). `d2[i]` can be 0.
pub fn even_radii(data: &[u8]) -> Vec<usize> {
    let n = data.len();
    let mut d = vec![0usize; n];
    let (mut l, mut r) = (0usize, 0usize);
    for i in 0..n {
        let mut k = if i >= r { 0 } else { d[l + r - i].min(r - i) };
        while i + k < n && i > k && data[i - k - 1] == data[i + k] {
            k += 1;
        }
        d[i] = k;
        if i + k > r {
            l = i - k;
            r = i + k;
        }
    }
    d
}

/// Longest palindromic substring as `(start, len)` — ties resolve to
/// the leftmost. `None` on empty input.
pub fn longest_palindrome(data: &[u8]) -> Option<(usize, usize)> {
    if data.is_empty() {
        return None;
    }
    let d1 = odd_radii(data);
    let d2 = even_radii(data);
    let mut best: (usize, usize) = (0, 1);
    for i in 0..data.len() {
        let l1 = 2 * d1[i] - 1;
        if l1 > best.1 {
            best = (i + 1 - d1[i], l1);
        }
        let l2 = 2 * d2[i];
        if l2 > best.1 {
            best = (i - d2[i], l2);
        }
    }
    Some(best)
}

/// Total count of distinct `(start, len)` palindromic substrings —
/// `Σ d1 + Σ d2`. "racecar" has 10.
pub fn count_palindromes(data: &[u8]) -> usize {
    odd_radii(data).iter().sum::<usize>() + even_radii(data).iter().sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn is_pal(data: &[u8], s: usize, e: usize) -> bool {
        (0..(e - s) / 2).all(|k| data[s + k] == data[e - 1 - k])
    }

    fn brute_longest(data: &[u8]) -> Option<(usize, usize)> {
        if data.is_empty() {
            return None;
        }
        let mut best = (0usize, 1usize);
        for s in 0..data.len() {
            for e in s + 1..=data.len() {
                if e - s > best.1 && is_pal(data, s, e) {
                    best = (s, e - s);
                }
            }
        }
        Some(best)
    }

    fn brute_count(data: &[u8]) -> usize {
        let mut set = BTreeSet::new();
        for s in 0..data.len() {
            for e in s + 1..=data.len() {
                if is_pal(data, s, e) {
                    set.insert((s, e));
                }
            }
        }
        set.len()
    }

    #[test]
    fn radii_and_longest_match_brute() {
        let mut rng = SplitMix64::new(0x4A4E);
        for _ in 0..400 {
            let n = rng.below(50) as usize;
            let data: Vec<u8> = (0..n).map(|_| (rng.next_u64() % 3) as u8 + b'a').collect();
            assert_eq!(longest_palindrome(&data), brute_longest(&data));
            assert_eq!(count_palindromes(&data), brute_count(&data));
            // Every radius entry must describe a real palindrome, and
            // one byte beyond must not.
            let d1 = odd_radii(&data);
            let d2 = even_radii(&data);
            for i in 0..n {
                let k = d1[i];
                assert!(is_pal(&data, i + 1 - k, i + k));
                if i + k < n && i >= k {
                    assert!(data[i - k] != data[i + k], "d1[{i}] not maximal");
                }
                let k2 = d2[i];
                if k2 > 0 {
                    assert!(is_pal(&data, i - k2, i + k2));
                }
                if i + k2 < n && i > k2 {
                    assert!(data[i - k2 - 1] != data[i + k2], "d2[{i}] not maximal");
                }
            }
        }
    }

    #[test]
    fn known_answers() {
        assert_eq!(longest_palindrome(b""), None);
        assert_eq!(longest_palindrome(b"x"), Some((0, 1)));
        assert_eq!(longest_palindrome(b"aaa"), Some((0, 3)));
        assert_eq!(longest_palindrome(b"ab"), Some((0, 1)));
        assert_eq!(count_palindromes(b"racecar"), 10);
        assert_eq!(count_palindromes(b"aaaa"), 10);
    }
}
