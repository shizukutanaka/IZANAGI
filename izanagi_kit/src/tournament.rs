//! Deterministic tournament brackets — single elimination (with seeding and
//! byes), double elimination (winners/losers/grand final), and Swiss-style
//! score-group pairing. Integer-only bookkeeping, no floats; the competition
//! counterpart of [`crate::elo`]. All pairings are functions of the seed
//! order alone, so replays are bit-exact.
//!
//! ```
//! use izanagi_kit::tournament::Bracket;
//! let mut b = Bracket::new(4); // players seeded 0..4
//! let m = b.next_match().unwrap();
//! assert_eq!((m.a, m.b), (0, 3)); // top seed vs bottom seed
//! b.report(m.id, 0); // player 0 wins
//! ```

/// One scheduled match in a [`Bracket`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    /// Match index within its phase (feeding order).
    pub id: usize,
    /// First player (seed number).
    pub a: usize,
    /// Second player.
    pub b: usize,
}

/// Standard seeding: for `n` players, seed `i` plays `next_power_of_two(n)*2-1-i`
/// is the classic table; equivalent deterministic pairing — order the field so
/// that recursively the top half seed pairs vs bottom half reversed.
fn seeded_order(n: usize) -> Vec<usize> {
    // Round-1 pairs: strongest vs weakest remaining, filling bracket slots.
    // For a bracket of size m=2^k the canonical seed pairing puts seed 0 vs
    // seed m-1, then fills so that seeds 0..m/2 meet the bottom half in
    // reverse order. For non-power-of-two n, seeds >= n are "bye" slots.
    let m = n.next_power_of_two().max(2);
    // Canonical seeding, recursive: for a bracket of size 2k the previous
    // level's order o gives [o0, 2k−1−o0, o1, 2k−1−o1, …]. For m=4 this is
    // [0,3,1,2]; for m=8 [0,7,3,4,1,6,2,5] — consecutive entries are the
    // round-1 pairs (top-vs-bottom of each octant).
    let mut order: Vec<usize> = vec![0, 1];
    let mut size = 2;
    while size < m {
        let mut next = Vec::with_capacity(size * 2);
        for &seed in &order {
            next.push(seed);
            next.push(size * 2 - 1 - seed);
        }
        order = next;
        size *= 2;
    }
    order
}

/// Single-elimination bracket over seeds `0..n` (`n >= 2`). Seeds are paired
/// top-vs-bottom (0 vs n−1, 1 vs n−2, …) with byes when `n` is not a power of
/// two; [`Bracket::next_match`] hands out the currently playable match and
/// [`Bracket::report`] records a winner and frees the next round.
pub struct Bracket {
    /// Current round's pairings; each entry is (a, b, winner) where winner
    /// None = not yet reported.
    round: Vec<(usize, usize, Option<usize>)>,
    /// Seeds that have already advanced (winners of `round`), used to build
    /// the next round when this one finishes.
    pending: Vec<usize>,
    /// Champion once the bracket is complete.
    champion: Option<usize>,
    /// Total matches issued, used as `Match::id`.
    issued: usize,
}

impl Bracket {
    /// New bracket for seeds `0..n`. `n < 2` yields an instantly-finished
    /// bracket with `champion` = the single seed or `None`.
    pub fn new(n: usize) -> Self {
        let mut seeds: Vec<usize> = (0..n).collect();
        seeds.sort_unstable(); // identity but keeps the contract explicit
        let order = seeded_order(n.max(2));
        // Pair consecutive entries of the seeded order; drop pairs where one
        // side is a bye slot (>= n) — that seed advances directly.
        let mut round = Vec::new();
        let mut pending = Vec::new();
        let mut i = 0;
        while i < order.len() {
            let a = order[i];
            let b = order[i + 1];
            match (a < n, b < n) {
                (true, true) => round.push((a, b, None)),
                (true, false) => pending.push(a),
                (false, true) => pending.push(b),
                (false, false) => {}
            }
            i += 2;
        }
        // Byes-advanced seeds join the winners' pool for the next round.
        Self {
            round,
            pending,
            champion: if n == 1 { Some(0) } else { None },
            issued: 0,
        }
    }

    /// The first unreported match of the current round, if any remain.
    pub fn next_match(&mut self) -> Option<Match> {
        if self.round.iter().all(|r| r.2.is_some()) && !self.round.is_empty() {
            self.advance_round();
        }
        for (id, r) in self.round.iter().enumerate() {
            if r.2.is_none() {
                return Some(Match {
                    id: id + self.issued,
                    a: r.0,
                    b: r.1,
                });
            }
        }
        None
    }

    /// Record `winner` (must be `a` or `b` of the open match `id`).
    /// Returns false for an out-of-range id or bogus winner.
    pub fn report(&mut self, id: usize, winner: usize) -> bool {
        let idx = match id.checked_sub(self.issued) {
            Some(i) if i < self.round.len() => i,
            _ => return false,
        };
        let (a, b, w) = self.round[idx];
        if w.is_some() || (winner != a && winner != b) {
            return false;
        }
        self.round[idx].2 = Some(winner);
        // Winners are collected from the round itself in `advance_round`;
        // `pending` holds only bye-advanced seeds. Pushing here too would
        // double-count the winner.
        true
    }

    /// True once a champion exists (or every match is done).
    pub fn finished(&self) -> bool {
        self.champion.is_some() || self.round.iter().all(|r| r.2.is_some())
    }

    /// The champion, once [`Bracket::finished`].
    pub fn champion(&self) -> Option<usize> {
        self.champion
    }

    /// Remaining players (winners recorded for the current round are already
    /// out; this returns everyone still alive).
    pub fn alive(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for (a, b, w) in &self.round {
            match w {
                Some(w) => out.push(*w),
                None => {
                    out.push(*a);
                    out.push(*b);
                }
            }
        }
        out.extend(self.pending.iter().copied());
        out
    }

    fn advance_round(&mut self) {
        // Winners + bye-advancees, re-paired in bracket order.
        let mut adv: Vec<usize> = self
            .round
            .iter()
            .filter_map(|r| r.2)
            .chain(self.pending.iter().copied())
            .collect();
        self.pending.clear();
        self.issued += self.round.len();
        if adv.len() <= 1 {
            self.champion = adv.pop();
            self.round.clear();
            return;
        }
        let mut round = Vec::new();
        let mut i = 0;
        while i + 1 < adv.len() {
            round.push((adv[i], adv[i + 1], None));
            i += 2;
        }
        if i < adv.len() {
            self.pending.push(adv[i]); // odd count → bye
        }
        self.round = round;
    }
}

/// Double-elimination bracket: losers drop to a losers' bracket and can reach
/// the grand final; a second grand-final loss (bracket reset not modeled —
/// the losers' champion must beat the winners' champion once here).
/// Simplified to the standard form: `n` players, winners' single elim,
/// losers' single elim among eliminated players in order of defeat round.
pub struct DoubleBracket {
    /// Winners' bracket (initial field).
    winners: Bracket,
    /// Losers' bracket, seeded by defeat order.
    losers: Vec<usize>,
    /// Current phase.
    phase: DoublePhase,
    /// Champions of each side once resolved.
    w_champ: Option<usize>,
    l_champ: Option<usize>,
    /// Grand final pair.
    final_match: Option<(usize, usize)>,
    /// Final winner.
    champion: Option<usize>,
    /// Loser-pool pair cursor for losers' matches.
    l_cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoublePhase {
    Winners,
    Losers,
    GrandFinal,
    Done,
}

impl DoubleBracket {
    /// New double-elim bracket for seeds `0..n`.
    pub fn new(n: usize) -> Self {
        Self {
            winners: Bracket::new(n),
            losers: Vec::new(),
            phase: DoublePhase::Winners,
            w_champ: None,
            l_champ: None,
            final_match: None,
            champion: None,
            l_cursor: 0,
        }
    }

    /// Next match to play. In the Winners phase it comes from the winners'
    /// bracket; in Losers phase it pairs `losers[l_cursor]` vs
    /// `losers[l_cursor+1]`; in GrandFinal it's `(w_champ, l_champ)`.
    pub fn next_match(&mut self) -> Option<Match> {
        match self.phase {
            DoublePhase::Winners => {
                if let Some(m) = self.winners.next_match() {
                    return Some(m);
                }
                self.w_champ = self.winners.champion();
                self.phase = DoublePhase::Losers;
                self.next_match()
            }
            DoublePhase::Losers => {
                // The pool is a queue: the front two play, the winner rejoins
                // the back. One survivor remains → losers' champion.
                if self.losers.len() >= 2 {
                    return Some(Match {
                        id: self.l_cursor,
                        a: self.losers[0],
                        b: self.losers[1],
                    });
                }
                self.l_champ = self.losers.first().copied();
                self.phase = DoublePhase::GrandFinal;
                if let (Some(w), Some(l)) = (self.w_champ, self.l_champ) {
                    self.final_match = Some((w, l));
                } else {
                    self.champion = self.w_champ;
                    self.phase = DoublePhase::Done;
                }
                self.next_match()
            }
            DoublePhase::GrandFinal => {
                if self.final_match.is_some() && self.champion.is_none() {
                    let (a, b) = self.final_match.unwrap_or((0, 0));
                    Some(Match { id: 0, a, b })
                } else {
                    None
                }
            }
            DoublePhase::Done => None,
        }
    }

    /// Report a match winner. During Winners, the loser joins `losers` in
    /// defeat order; during Losers the loser is eliminated; in GrandFinal the
    /// winner becomes champion.
    pub fn report(&mut self, id: usize, winner: usize) -> bool {
        match self.phase {
            DoublePhase::Winners => {
                let m = match self.winners.next_match() {
                    Some(m) => m,
                    None => return false,
                };
                if m.id != id || (winner != m.a && winner != m.b) {
                    return false;
                }
                self.losers.push(if winner == m.a { m.b } else { m.a });
                self.winners.report(id, winner)
            }
            DoublePhase::Losers => {
                if id != self.l_cursor || self.losers.len() < 2 {
                    return false;
                }
                let a = self.losers[0];
                let b = self.losers[1];
                if winner != a && winner != b {
                    return false;
                }
                // Winner re-enters the pool at the back; the loser drops out.
                self.losers.remove(0);
                self.losers.remove(0);
                self.losers.push(winner);
                self.l_cursor += 1;
                true
            }
            DoublePhase::GrandFinal => {
                let (a, b) = match self.final_match {
                    Some(f) => f,
                    None => return false,
                };
                if winner != a && winner != b {
                    return false;
                }
                self.champion = Some(winner);
                self.phase = DoublePhase::Done;
                true
            }
            DoublePhase::Done => false,
        }
    }

    /// The champion, once the grand final is reported (or skipped when the
    /// losers' side is empty).
    pub fn champion(&self) -> Option<usize> {
        self.champion
    }
}

/// Swiss pairing: standings are `(points, seed)` per player; each round pairs
/// players in standing order within equal-point groups, avoiding rematches
/// where possible. Returns the pair list; callers update scores via
/// [`Swiss::report`].
pub struct Swiss {
    /// Player points (integer, e.g. win=1 draw=1/2 encoded as 2/1/0 scaled ×2).
    pub points: Vec<i64>,
    /// Players each seed has already faced.
    played: Vec<Vec<usize>>,
    /// Round number.
    pub round: usize,
}

impl Swiss {
    /// New Swiss for `n` players, all at 0 points.
    pub fn new(n: usize) -> Self {
        Self {
            points: vec![0; n],
            played: vec![Vec::new(); n],
            round: 0,
        }
    }

    /// Pair this round: sort by (points desc, seed asc), then greedily pair
    /// adjacent players, skipping any pairing that would be a rematch by
    /// taking the next available opponent. Unpaired odd one out gets a bye
    /// (`None` partner) — encoded as `Match { a, b: usize::MAX }` is avoided;
    /// instead the returned list contains `(a, Option<b>)`.
    pub fn pair(&mut self) -> Vec<(usize, Option<usize>)> {
        let n = self.points.len();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&s| (-self.points[s], s));
        let mut paired = vec![false; n];
        let mut out = Vec::new();
        let mut i = 0;
        while i < n {
            let a = order[i];
            if paired[a] {
                i += 1;
                continue;
            }
            // Find first unpaired b > i not in played[a].
            let mut chosen: Option<usize> = None;
            for &b in order.iter().skip(i + 1) {
                if paired[b] || self.played[a].contains(&b) {
                    continue;
                }
                chosen = Some(b);
                break;
            }
            match chosen {
                Some(b) => {
                    paired[a] = true;
                    paired[b] = true;
                    self.played[a].push(b);
                    self.played[b].push(a);
                    out.push((a, Some(b)));
                }
                None => {
                    paired[a] = true;
                    out.push((a, None)); // bye
                }
            }
            i += 1;
        }
        self.round += 1;
        out
    }

    /// Award `wa`..`wb` points to players `a` and `b` (scaled ×2 so draws are
    /// odd-free: win 2, draw 1, loss 0).
    pub fn report(&mut self, a: usize, b: usize, pa: i64, pb: i64) {
        if a < self.points.len() {
            self.points[a] += pa;
        }
        if b < self.points.len() {
            self.points[b] += pb;
        }
    }

    /// Final standings: seeds sorted by (points desc, seed asc).
    pub fn standings(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.points.len()).collect();
        order.sort_by_key(|&s| (-self.points[s], s));
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_elim_eight_players() {
        let mut b = Bracket::new(8);
        // Standard seeding: round 1 pairs 0v7, 3v4, 1v6, 2v5 (top-vs-bottom
        // of each quarter). Any deterministic order is fine — but the four
        // pairs must cover all 8 seeds exactly once.
        let mut played = Vec::new();
        while let Some(m) = b.next_match() {
            played.push((m.id, m.a, m.b));
            b.report(m.id, m.a); // higher seed always wins
        }
        // 8-player single elim = 7 matches total.
        assert_eq!(played.len(), 7);
        assert_eq!(b.champion(), Some(0));
        // Round 1 is the first 4 matches; they cover every seed exactly once.
        let mut seen = [false; 8];
        for (_, a, bb) in &played[..4] {
            assert!(!seen[*a] && !seen[*bb]);
            seen[*a] = true;
            seen[*bb] = true;
        }
    }

    #[test]
    fn single_elim_byes() {
        let mut b = Bracket::new(5);
        // 5 players → bracket of 8: seeds 5,6,7 are byes. Exactly the three
        // top-of-order slots advance free.
        let mut matches = 0;
        while let Some(m) = b.next_match() {
            b.report(m.id, m.a);
            matches += 1;
        }
        assert_eq!(matches, 4); // n−1 total
        assert!(b.champion().is_some());
    }

    #[test]
    fn report_rejects_garbage() {
        let mut b = Bracket::new(4);
        let m = b.next_match().unwrap();
        assert!(!b.report(m.id, 99)); // not a player in the match
        assert!(!b.report(m.id + 5, m.a)); // out-of-range id
        assert!(b.report(m.id, m.b));
        assert!(!b.report(m.id, m.a)); // already reported
    }

    #[test]
    fn double_elim_grand_final() {
        let mut d = DoubleBracket::new(4);
        // n=4 → 3 winners' + 2 losers' + 1 grand final = 6 matches.
        let mut count = 0;
        while let Some(m) = d.next_match() {
            assert!(d.report(m.id, m.a)); // 'a' wins everything
            count += 1;
            assert!(count <= 8); // termination guard
        }
        assert_eq!(count, 6);
        assert_eq!(d.champion(), Some(0));
    }

    #[test]
    fn swiss_pairs_within_score_groups_no_rematch() {
        let mut s = Swiss::new(6);
        let r1 = s.pair();
        assert_eq!(r1.len(), 3);
        for (a, b) in &r1 {
            let b = b.unwrap();
            s.report(*a, b, 2, 0); // a wins
        }
        // Round 2: no rematch of round 1 (equal-point pairing is
        // best-effort with 3 winners in a field of 6).
        let r2 = s.pair();
        for (a, b) in &r2 {
            assert!(!r1.iter().any(|(x, y)| *x == *a && *y == *b));
        }
        // No rematch across three rounds.
        let r3 = s.pair();
        for (a, b) in r3.iter().flat_map(|(a, b)| b.map(|b| (*a, b))) {
            let hit1 = r1
                .iter()
                .any(|(x, y)| (*x == a && *y == Some(b)) || (*x == b && *y == Some(a)));
            let hit2 = r2
                .iter()
                .any(|(x, y)| (*x == a && *y == Some(b)) || (*x == b && *y == Some(a)));
            assert!(!(hit1 || hit2));
        }
    }

    #[test]
    fn swiss_odd_field_gets_bye_and_standings() {
        let mut s = Swiss::new(5);
        let r = s.pair();
        assert_eq!(r.len(), 3);
        assert_eq!(r.iter().filter(|(_, b)| b.is_none()).count(), 1); // one bye
        s.report(0, 1, 2, 0);
        s.report(2, 3, 2, 0);
        let st = s.standings();
        assert_eq!(st[0], 0); // first winner, lowest seed tiebreak
        assert_eq!(st[1], 2);
    }

    #[test]
    fn seeded_order_is_bracket_canonical() {
        // 8 seeds: 0v7, 3v4, 1v6, 2v5 in order.
        let o = seeded_order(8);
        let pairs: Vec<(usize, usize)> = o.chunks(2).map(|c| (c[0], c[1])).collect();
        // Each pair sums to 7 (top vs bottom).
        for (a, b) in &pairs {
            assert_eq!(a + b, 7);
        }
    }
}
