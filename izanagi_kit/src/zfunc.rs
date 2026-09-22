//! Z-algorithm — `z[i]` = length of the longest common prefix of
//! `data` and `data[i..]`, all `n` answers in `O(n)`. The string
//! family's linear-time complement to [`kmp`](crate::kmp),
//! [`manacher`](crate::manacher), and [`suffix`](crate::suffix):
//! borders (prefix = suffix), minimal period, and pattern search via
//! one concatenation, all without quadratic rescans.
//!
//! ```
//! use izanagi_kit::zfunc::{borders, min_period, z};
//! assert_eq!(z(b"ababa"), [0, 0, 3, 0, 1]);
//! assert_eq!(borders(b"ababab"), [4, 2]); // longest first
//! assert_eq!(min_period(b"abababab"), Some(2));
//! ```

/// Z-array: `z[i]` = LCP length of `data[0..]` and `data[i..]`; `z[0]`
/// is conventionally `0`. `O(n)` via the rightmost Z-box.
pub fn z(data: &[u8]) -> Vec<usize> {
    let n = data.len();
    let mut z = vec![0usize; n];
    let (mut l, mut r) = (0usize, 0usize); // half-open [l, r)
    for i in 1..n {
        if i < r {
            z[i] = z[i - l].min(r - i);
        }
        while i + z[i] < n && data[z[i]] == data[i + z[i]] {
            z[i] += 1;
        }
        if i + z[i] > r {
            l = i;
            r = i + z[i];
        }
    }
    z
}

/// All pattern start positions in `text` — `pat` + `0xFF` + `text` in
/// one Z pass (`0xFF` is a separator byte; patterns containing `0xFF`
/// use [`z_search_bytes`] instead). Sorted ascending.
pub fn z_search(pat: &[u8], text: &[u8]) -> Vec<usize> {
    if pat.is_empty() {
        return (0..=text.len()).collect();
    }
    let mut buf = Vec::with_capacity(pat.len() + 1 + text.len());
    buf.extend_from_slice(pat);
    buf.push(0xFF);
    buf.extend_from_slice(text);
    let zv = z(&buf);
    let off = pat.len() + 1;
    (off..zv.len())
        .filter(|&i| zv[i] >= pat.len())
        .map(|i| i - off)
        .collect()
}

/// Like [`z_search`] but with a caller-supplied separator — use when
/// `pat`/`text` can contain `0xFF`.
pub fn z_search_bytes(pat: &[u8], text: &[u8], sep: u8) -> Vec<usize> {
    if pat.is_empty() {
        return (0..=text.len()).collect();
    }
    if pat.contains(&sep) || text.contains(&sep) {
        // No safe separator exists in-band: fall back to per-position
        // prefix matching via the Z-array of `pat` extended per window
        // is quadratic — instead verify each candidate naively.
        return naive_search(pat, text);
    }
    let mut buf = Vec::with_capacity(pat.len() + 1 + text.len());
    buf.extend_from_slice(pat);
    buf.push(sep);
    buf.extend_from_slice(text);
    let zv = z(&buf);
    let off = pat.len() + 1;
    (off..zv.len())
        .filter(|&i| zv[i] >= pat.len())
        .map(|i| i - off)
        .collect()
}

fn naive_search(pat: &[u8], text: &[u8]) -> Vec<usize> {
    if pat.len() > text.len() {
        return Vec::new();
    }
    (0..=text.len() - pat.len())
        .filter(|&i| &text[i..i + pat.len()] == pat)
        .collect()
}

/// Border lengths (proper prefix = suffix), longest first.
/// `ababab` → `[4, 2]`; `aaaa` → `[3, 2, 1]`; `ab` → `[]`.
pub fn borders(data: &[u8]) -> Vec<usize> {
    let n = data.len();
    if n < 2 {
        return Vec::new();
    }
    let zv = z(data);
    let mut out: Vec<usize> = (1..n).filter(|&i| zv[i] == n - i).map(|i| n - i).collect();
    out.sort_unstable_by(|a, b| b.cmp(a));
    out
}

/// Minimal period `p` with `data` a repetition of `data[..p]`
/// (`abcabcabc` → `Some(3)`). Returns `None` when `data` is not a
/// pure repetition; `Some(len)` means "no smaller period" (it repeats
/// trivially once).
pub fn min_period(data: &[u8]) -> Option<usize> {
    let n = data.len();
    if n == 0 {
        return None;
    }
    let zv = z(data);
    for (p, &zp) in zv.iter().enumerate().skip(1) {
        // Period p ⟺ z[p] reaches the end or repeats with remainder.
        if n % p == 0 && zp >= n - p {
            return Some(p);
        }
    }
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn z_matches_naive_and_periodicity() {
        let mut rng = SplitMix64::new(0xABCD_EF01);
        for _ in 0..400 {
            let n = (rng.below(60) + 1) as usize;
            let data: Vec<u8> = (0..n).map(|_| b'a' + (rng.below(3) as u8)).collect();
            let zv = z(&data);
            for i in 0..n {
                let mut want = 0usize;
                if i != 0 {
                    while i + want < n && data[want] == data[i + want] {
                        want += 1;
                    }
                }
                assert_eq!(zv[i], want, "i={i} data={data:?}");
            }
            // min_period vs brute-force period check.
            if let Some(p) = min_period(&data) {
                assert!(n % p == 0);
                for i in p..n {
                    assert_eq!(data[i], data[i % p]);
                }
                // Minimality: no smaller period divides p.
                for q in 1..p {
                    if p % q == 0 {
                        let mut is_period = true;
                        for i in q..n {
                            if data[i] != data[i % q] {
                                is_period = false;
                                break;
                            }
                        }
                        assert!(!is_period, "q={q} is also a period of {data:?}");
                    }
                }
            } else {
                // No full period: verify no divisor works.
                for p in 1..n {
                    if n % p == 0 {
                        let ok = (p..n).all(|i| data[i] == data[i % p]);
                        assert!(!ok);
                    }
                }
            }
            // borders vs brute force.
            let b = borders(&data);
            let mut truth: Vec<usize> = (1..n).filter(|&l| data[..l] == data[n - l..]).collect();
            truth.sort_unstable_by(|x, y| y.cmp(x));
            assert_eq!(b, truth);
        }
    }

    #[test]
    fn search_and_edge_cases() {
        let mut rng = SplitMix64::new(0xFEED_BEEF);
        for _ in 0..300 {
            let tn = (rng.below(80) + 1) as usize;
            let pn = (rng.below(8) + 1) as usize;
            let text: Vec<u8> = (0..tn).map(|_| b'a' + (rng.below(3) as u8)).collect();
            let pat: Vec<u8> = (0..pn).map(|_| b'a' + (rng.below(3) as u8)).collect();
            let want = naive_search(&pat, &text);
            assert_eq!(z_search(&pat, &text), want);
            assert_eq!(z_search_bytes(&pat, &text, 0x00), want);
            // With a separator that appears in-band, falls back cleanly.
            assert_eq!(z_search_bytes(&pat, &text, b'a'), want);
        }
        assert_eq!(z_search(b"", b"abc"), vec![0, 1, 2, 3]);
        assert!(borders(b"ab").is_empty());
        assert_eq!(min_period(b""), None);
        assert_eq!(min_period(b"aaaa"), Some(1));
        assert_eq!(min_period(b"abcabc"), Some(3));
        assert_eq!(min_period(b"ab"), Some(2));
    }
}
