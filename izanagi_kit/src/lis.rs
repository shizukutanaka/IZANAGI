//! Longest increasing subsequence in `O(n log n)` — patience-sorting
//! piles with binary search, plus parent reconstruction. The canonical
//! answer for "how much of this sequence is already ordered" — input
//! lag patterns, score progressions, and the LIS↔LCS bridges.
//! Deterministic: the smallest tail is kept at every length, so the
//! reconstruction is the lexicographically smallest valid LIS.
//!
//! ```
//! use izanagi_kit::lis::lis;
//! assert_eq!(lis(&[3, 1, 4, 1, 5, 9, 2, 6]), vec![1, 4, 5, 6]);
//! ```

/// The lexicographically-smallest strictly-increasing subsequence of
/// maximum length, returned as *values*. Equal length candidates break
/// toward the earlier-ending one via the pile's leftmost insert.
pub fn lis(data: &[i64]) -> Vec<i64> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }
    // tails[len] = index of the smallest tail of an increasing
    // subsequence of length len+1.
    let mut tails: Vec<usize> = Vec::with_capacity(n);
    let mut parent: Vec<Option<usize>> = vec![None; n];
    for (i, &v) in data.iter().enumerate() {
        let pos = tails.partition_point(|&j| data[j] < v);
        if pos > 0 {
            parent[i] = Some(tails[pos - 1]);
        }
        if pos == tails.len() {
            tails.push(i);
        } else {
            tails[pos] = i;
        }
    }
    // Walk back from the last pile's tail.
    let mut out = Vec::with_capacity(tails.len());
    let mut cur = tails[tails.len() - 1];
    loop {
        out.push(data[cur]);
        match parent[cur] {
            Some(p) => cur = p,
            None => break,
        }
    }
    out.reverse();
    out
}

/// Same count without materializing the subsequence — `O(n log n)`.
pub fn lis_len(data: &[i64]) -> usize {
    let mut tails: Vec<i64> = Vec::with_capacity(data.len());
    for &v in data {
        let pos = tails.partition_point(|&t| t < v);
        if pos == tails.len() {
            tails.push(v);
        } else {
            tails[pos] = v;
        }
    }
    tails.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// O(2ⁿ)-free oracle: DP with `dp[i]` = LIS ending at i, then pick
    /// the leftmost index achieving the max and reconstruct.
    fn oracle(data: &[i64]) -> Vec<i64> {
        let n = data.len();
        if n == 0 {
            return Vec::new();
        }
        let mut dp = vec![1usize; n];
        let mut par: Vec<Option<usize>> = vec![None; n];
        for i in 1..n {
            for j in 0..i {
                if data[j] < data[i] && dp[j] + 1 > dp[i] {
                    dp[i] = dp[j] + 1;
                    par[i] = Some(j);
                }
            }
        }
        let mut best = 0;
        for (i, &d) in dp.iter().enumerate() {
            if d > dp[best] {
                best = i;
            }
        }
        let mut out = Vec::with_capacity(dp[best]);
        let mut cur = best;
        loop {
            out.push(data[cur]);
            match par[cur] {
                Some(p) => cur = p,
                None => break,
            }
        }
        out.reverse();
        out
    }

    #[test]
    fn length_matches_dp_oracle_and_is_increasing() {
        let mut rng = SplitMix64::new(0x1150_9A0B);
        for _ in 0..400 {
            let n = rng.below(40) as usize;
            let data: Vec<i64> = (0..n).map(|_| (rng.below(20) as i64) - 10).collect();
            let got = lis(&data);
            let want = oracle(&data);
            assert_eq!(got.len(), want.len(), "len mismatch on {data:?}");
            assert_eq!(got.len(), lis_len(&data));
            // Strictly increasing + each value comes from the input.
            let set: BTreeSet<i64> = data.iter().copied().collect();
            for w in got.windows(2) {
                assert!(w[0] < w[1]);
            }
            for &v in &got {
                assert!(set.contains(&v));
            }
        }
        assert_eq!(lis(&[]), Vec::<i64>::new());
        assert_eq!(lis_len(&[]), 0);
        assert_eq!(lis(&[5]), vec![5]);
        assert_eq!(lis(&[9, 8, 7, 6]), vec![6]);
        assert_eq!(lis(&[1, 2, 3, 4]), vec![1, 2, 3, 4]);
        // Classic: LIS of [3,1,4,1,5,9,2,6] has length 4.
        assert_eq!(lis_len(&[3, 1, 4, 1, 5, 9, 2, 6]), 4);
    }
}
