//! fzf-style fuzzy subsequence scoring — ranks candidates for a
//! command-palette / did-you-mean search box.
//!
//! Where [`crate::trie::Trie::keys_with_prefix`] answers "which keys
//! start with this prefix" and [`crate::diff::levenshtein`] measures
//! edit distance, this is the ranker for interactive pickers: `ctrl+q`
//! should float `CmdQueue::clear` over `calendar`. Case-insensitive,
//! byte-oriented (like the trie), scoring is pure integer arithmetic —
//! same corpus + same pattern → same order, on every platform.
//!
//! ```
//! use izanagi_kit::fuzzy::{score, rank};
//! assert!(score("cq", "cmd_queue").is_some());
//! assert!(score("z", "cmd_queue").is_none());
//! let top = rank("cq", &["cmd_queue", "clear_queue", "calendar"]);
//! assert_eq!(top[0], 0); // "cmd_queue" wins: boundary hit, short gap
//! ```

/// Scoring weights (fzf-shaped): each matched character earns `CHAR`,
/// consecutive matches `CONSECUTIVE` extra (the dominant signal, as
/// in fzf), word-boundary starts `BOUNDARY` extra; every skipped
/// candidate char costs `GAP`.
const CHAR: i64 = 1;
const CONSECUTIVE: i64 = 10;
const BOUNDARY: i64 = 8;
const GAP: i64 = -1;

/// Is `b` a word boundary after `a`? Start-of-string, separator
/// (`_-. /`), or a lowercase→uppercase camel hump.
fn boundary(a: Option<u8>, b: u8) -> bool {
    match a {
        None => true,
        Some(a) => {
            matches!(a, b'_')
                || matches!(a, b'-' | b'.' | b'/' | b' ')
                || (a.is_ascii_lowercase() && b.is_ascii_uppercase())
        }
    }
}

/// Score `pattern` against `cand` — `Some(score)` when the pattern is a
/// subsequence (case-insensitive), `None` otherwise. Higher is better;
/// exact/prefix matches beat scattered ones through the boundary and
/// consecutive bonuses.
///
/// Scoring is deterministic: the forward pass picks the leftmost
/// greedily-optimal alignment — earliest positions win ties, which is
/// the behaviour users expect from a picker.
pub fn score(pattern: &str, cand: &str) -> Option<i64> {
    let p = pattern.as_bytes();
    let c = cand.as_bytes();
    if p.is_empty() {
        return Some(0);
    }
    if p.len() > c.len() {
        return None;
    }
    // Forward leftmost pass: match each pattern byte at the earliest
    // reachable position, scoring bonuses for runs and boundaries.
    let mut total = 0i64;
    let mut pi = 0usize;
    let mut prev_match: Option<usize> = None; // index of previous matched char
    for (ci, &cb) in c.iter().enumerate() {
        if pi < p.len() && cb.eq_ignore_ascii_case(&p[pi]) {
            total += CHAR;
            if prev_match.is_some_and(|pm| ci == pm + 1) {
                total += CONSECUTIVE;
            }
            if boundary(ci.checked_sub(1).map(|i| c[i]), cb) {
                total += BOUNDARY;
            } else {
                // Non-boundary gap: the skipped stretch since the last
                // match already costs GAP each — accounted below.
            }
            pi += 1;
            prev_match = Some(ci);
        } else {
            total += GAP;
        }
    }
    (pi == p.len()).then_some(total)
}

/// Rank `cands` by [`score`] against `pattern`, best first.
///
/// Order contract (a pure function of `(pattern, cands)`): score
/// descending, then shorter candidate first, then byte-lexicographic,
/// then original index — no equal keys means no ties.
/// Non-matching candidates are excluded.
pub fn rank(pattern: &str, cands: &[&str]) -> Vec<usize> {
    let mut scored: Vec<(i64, usize)> = cands
        .iter()
        .enumerate()
        .filter_map(|(i, c)| score(pattern, c).map(|s| (s, i)))
        .collect();
    scored.sort_by(|(s1, i1), (s2, i2)| {
        s2.cmp(s1)
            .then(cands[*i1].len().cmp(&cands[*i2].len()))
            .then(cands[*i1].as_bytes().cmp(cands[*i2].as_bytes()))
            .then(i1.cmp(i2))
    });
    scored.into_iter().map(|(_, i)| i).collect()
}

/// Ranked candidates as strings, same order as [`rank`].
pub fn rank_str<'a>(pattern: &str, cands: &[&'a str]) -> Vec<&'a str> {
    rank(pattern, cands).into_iter().map(|i| cands[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Oracle: is `p` a subsequence of `c` (case-insensitive)?
    fn is_subseq(p: &[u8], c: &[u8]) -> bool {
        let mut i = 0;
        for &b in c {
            if i < p.len() && b.eq_ignore_ascii_case(&p[i]) {
                i += 1;
            }
        }
        i == p.len()
    }

    #[test]
    fn score_iff_subsequence() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0xF0F1);
        for _ in 0..2000 {
            let plen = 1 + rng.below(5) as usize;
            let clen = 1 + rng.below(15) as usize;
            let p: String = (0..plen)
                .map(|_| (b'a' + rng.below(26) as u8) as char)
                .collect();
            let c: String = (0..clen)
                .map(|_| (b'a' + rng.below(26) as u8) as char)
                .collect();
            assert_eq!(
                score(&p, &c).is_some(),
                is_subseq(p.as_bytes(), c.as_bytes())
            );
        }
    }

    #[test]
    fn ordering_invariants() {
        // Exact match beats prefix-miss beats scattered.
        let order = rank_str("inv", &["infix_v", "inventory", "inv"]);
        assert_eq!(order[0], "inv");
        // Consecutive run beats the same chars spread out.
        assert!(score("inv", "inventory").unwrap() > score("inv", "i_n_v_ented").unwrap());
        // Boundary match beats mid-word.
        assert!(score("q", "cmd_queue").unwrap() > score("q", "inquest").unwrap());
        // Empty pattern scores 0 on everything and matches everything.
        assert_eq!(score("", "anything"), Some(0));
        assert_eq!(rank("x", &["ax", "bx"]), vec![0, 1]);
    }

    #[test]
    fn rank_is_deterministic_and_total() {
        let cands = ["beta", "alphabet", "bet", "aleph", "better", "alphabeta"];
        let r1 = rank("bet", &cands);
        let r2 = rank("bet", &cands);
        assert_eq!(r1, r2);
        // Excluded = non-subsequence.
        for &i in &r1 {
            assert!(is_subseq(b"bet", cands[i].as_bytes()));
        }
        let included: std::collections::BTreeSet<usize> = r1.iter().copied().collect();
        for (i, c) in cands.iter().enumerate() {
            assert_eq!(included.contains(&i), is_subseq(b"bet", c.as_bytes()));
        }
        // Deterministic full order: sorting by the documented key
        // reproduces `rank`'s output exactly.
        let mut want: Vec<usize> = (0..cands.len())
            .filter(|&i| score("bet", cands[i]).is_some())
            .collect();
        want.sort_by(|&a, &b| {
            let (sa, sb) = (
                score("bet", cands[a]).unwrap(),
                score("bet", cands[b]).unwrap(),
            );
            sb.cmp(&sa)
                .then(cands[a].len().cmp(&cands[b].len()))
                .then(cands[a].as_bytes().cmp(cands[b].as_bytes()))
        });
        assert_eq!(r1, want);
    }
}
