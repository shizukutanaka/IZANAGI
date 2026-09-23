//! Bentley–Ottmann sweep: every point where two or more segments meet,
//! in exact rational coordinates.
//!
//! Events are endpoints and discovered intersections, keyed by
//! `(x, y)` in a `BTreeMap` priority queue; the status is the set of
//! non-vertical segments crossing the sweep line, resorted at each
//! batch by `(y at x, slope)` — the ordering key that makes
//! endpoint-on-interior touches adjacent to their hosts. Newly
//! adjacent pairs are pair-tested once and their intersection, if it
//! lies strictly after the current point, enters the queue.
//!
//! Semantics: every returned [`Pt`] is a meeting point — a proper
//! interior crossing, an endpoint lying on another segment's interior,
//! or an endpoint shared by two segments. Collinear *overlapping*
//! segments are not reported (a continuum of meetings is meaningless
//! for a point list); their shared endpoints still are.
//!
//! ```
//! use izanagi_kit::bentley;
//! let pts = bentley::intersections(&[
//!     ((0, 0), (4, 4)),
//!     ((0, 4), (4, 0)),
//! ]);
//! assert_eq!(pts.len(), 1);
//! assert_eq!(pts[0].x.num, pts[0].x.den * 2); // x = 2
//! ```

use crate::frac::Frac;
use std::collections::{BTreeMap, BTreeSet};

type Seg = ((i64, i64), (i64, i64));

/// A rational meeting point of two or more segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pt {
    /// X coordinate.
    pub x: Frac,
    /// Y coordinate.
    pub y: Frac,
}

impl PartialOrd for Pt {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for Pt {
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        self.x
            .cmp_frac(&rhs.x)
            .then_with(|| self.y.cmp_frac(&rhs.y))
    }
}

#[derive(Clone, Copy)]
enum Ev {
    Start(u32),
    End(u32),
    Cross,
}

fn norm(mut a: (i64, i64), mut b: (i64, i64)) -> Seg {
    if (b.0, b.1) < (a.0, a.1) {
        std::mem::swap(&mut a, &mut b);
    }
    (a, b)
}

/// `y` of segment `s` (normalized `(p, q)`, `p.x < q.x`) at x — exact.
fn y_at(s: Seg, x: Frac) -> Frac {
    let ((x1, y1), (x2, y2)) = s;
    let dy = Frac::new((y2 - y1) as i128, (x2 - x1) as i128);
    dy * (x - Frac::from_int(x1 as i128)) + Frac::from_int(y1 as i128)
}

fn slope(s: Seg) -> Frac {
    let ((x1, y1), (x2, y2)) = s;
    Frac::new((y2 - y1) as i128, (x2 - x1) as i128)
}

/// The single meeting point of two non-parallel segments, if it lies
/// on both (endpoints inclusive). `None` for disjoint or parallel
/// pairs — including collinear overlap, by contract.
fn seg_isect(a: Seg, b: Seg) -> Option<Pt> {
    let ((x1, y1), (x2, y2)) = a;
    let ((x3, y3), (x4, y4)) = b;
    let dx1 = (x2 - x1) as i128;
    let dy1 = (y2 - y1) as i128;
    let dx2 = (x4 - x3) as i128;
    let dy2 = (y4 - y3) as i128;
    let den = dx1 * dy2 - dy1 * dx2;
    if den == 0 {
        return None;
    }
    let tn = (x3 - x1) as i128 * dy2 - (y3 - y1) as i128 * dx2;
    let un = (x3 - x1) as i128 * dy1 - (y3 - y1) as i128 * dx1;
    // t = tn/den on a, u = un/den on b; both must be in [0,1].
    let in01 = |num: i128| -> bool {
        if den > 0 {
            num >= 0 && num <= den
        } else {
            num <= 0 && num >= den
        }
    };
    if !in01(tn) || !in01(un) {
        return None;
    }
    let t = Frac::new(tn, den);
    Some(Pt {
        x: Frac::from_int(x1 as i128) + t * Frac::from_int(dx1),
        y: Frac::from_int(y1 as i128) + t * Frac::from_int(dy1),
    })
}

/// Whether segment `s` (normalized) contains rational point `p`.
fn seg_contains(s: Seg, p: Pt) -> bool {
    let ((x1, y1), (x2, y2)) = s;
    // Collinearity in exact Frac arithmetic, then both bounds.
    let cross_p = Frac::from_int((x2 - x1) as i128) * (p.y - Frac::from_int(y1 as i128))
        - Frac::from_int((y2 - y1) as i128) * (p.x - Frac::from_int(x1 as i128));
    if cross_p != Frac::from_int(0) {
        return false;
    }
    let (lo_x, hi_x) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    let (lo_y, hi_y) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    let in_x = p.x.cmp_frac(&Frac::from_int(lo_x as i128)) != std::cmp::Ordering::Less
        && p.x.cmp_frac(&Frac::from_int(hi_x as i128)) != std::cmp::Ordering::Greater;
    let in_y = p.y.cmp_frac(&Frac::from_int(lo_y as i128)) != std::cmp::Ordering::Less
        && p.y.cmp_frac(&Frac::from_int(hi_y as i128)) != std::cmp::Ordering::Greater;
    in_x && in_y
}

/// All meeting points of `segs`, sorted by `(x, y)`. Degenerate
/// zero-length segments are ignored.
///
/// ```
/// use izanagi_kit::bentley;
/// // A 2x2 grid: 2 verticals x 2 horizontals -> 4 crossings.
/// let pts = bentley::intersections(&[
///     ((1, 0), (1, 4)),
///     ((3, 0), (3, 4)),
///     ((0, 1), (4, 1)),
///     ((0, 3), (4, 3)),
/// ]);
/// assert_eq!(pts.len(), 4);
/// ```
pub fn intersections(segs: &[Seg]) -> Vec<Pt> {
    let segs: Vec<Seg> = segs
        .iter()
        .map(|&(a, b)| norm(a, b))
        .filter(|&(a, b)| a != b)
        .collect();
    let mut queue: BTreeMap<Pt, Vec<Ev>> = BTreeMap::new();
    for (i, &(p, q)) in segs.iter().enumerate() {
        queue
            .entry(Pt {
                x: Frac::from_int(p.0 as i128),
                y: Frac::from_int(p.1 as i128),
            })
            .or_default()
            .push(Ev::Start(i as u32));
        queue
            .entry(Pt {
                x: Frac::from_int(q.0 as i128),
                y: Frac::from_int(q.1 as i128),
            })
            .or_default()
            .push(Ev::End(i as u32));
    }
    let mut found: BTreeSet<Pt> = BTreeSet::new();
    let mut checked: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut status: Vec<u32> = Vec::new();
    // Vertical segments never enter status: they are tracked for the
    // duration of their x-line so endpoint touches at any height on
    // that line are still pair-tested.
    let mut verticals: Vec<u32> = Vec::new();
    let mut cur_x: Option<Frac> = None;

    while let Some((p, evs)) = queue.pop_first() {
        if cur_x != Some(p.x) {
            verticals.clear();
            cur_x = Some(p.x);
        }
        let mut touched_here: BTreeSet<u32> = BTreeSet::new();
        for &ev in &evs {
            match ev {
                Ev::End(s) => {
                    touched_here.insert(s);
                    if let Some(pos) = status.iter().position(|&x| x == s) {
                        status.remove(pos);
                    }
                }
                Ev::Start(s) => {
                    touched_here.insert(s);
                }
                Ev::Cross => {}
            }
        }
        // Segments already in status that pass exactly through p are
        // also "touched" at this point (endpoint-on-interior).
        for &s in &status {
            if seg_contains(segs[s as usize], p) {
                touched_here.insert(s);
            }
        }
        for &ev in &evs {
            if let Ev::Start(s) = ev {
                if segs[s as usize].0 .0 == segs[s as usize].1 .0 {
                    verticals.push(s);
                } else {
                    status.push(s);
                }
            }
        }
        for &v in &verticals {
            if seg_contains(segs[v as usize], p) {
                touched_here.insert(v);
            }
        }
        // Two or more distinct segments meet at p.
        if touched_here.len() >= 2 {
            found.insert(p);
        }
        // Restore sweep order: y at p.x, ties by slope — the key that
        // makes segs concurrent at p sort adjacently.
        status.sort_by(|&a, &b| {
            let sa = segs[a as usize];
            let sb = segs[b as usize];
            y_at(sa, p.x)
                .cmp_frac(&y_at(sb, p.x))
                .then_with(|| slope(sa).cmp_frac(&slope(sb)))
                .then(a.cmp(&b))
        });
        // Newly adjacent pairs: one pair-test each, cached.
        for w in status.windows(2) {
            let (a, b) = (w[0].min(w[1]), w[0].max(w[1]));
            if checked.insert((a, b)) {
                if let Some(q) = seg_isect(segs[a as usize], segs[b as usize]) {
                    found.insert(q);
                    if q > p {
                        queue.entry(q).or_default().push(Ev::Cross);
                    }
                }
            }
        }
        // Verticals on this x-line meet anything involved at x = p.x:
        // status members plus every segment with an endpoint at p.
        for &v in &verticals {
            for &s in touched_here.iter().chain(status.iter()) {
                if s == v {
                    continue;
                }
                let (a, b) = (v.min(s), v.max(s));
                if checked.insert((a, b)) {
                    if let Some(q) = seg_isect(segs[v as usize], segs[s as usize]) {
                        found.insert(q);
                        if q > p {
                            queue.entry(q).or_default().push(Ev::Cross);
                        }
                    }
                }
            }
        }
    }
    found.into_iter().collect()
}

/// `true` iff any two segments share a point.
pub fn any_intersection(segs: &[Seg]) -> bool {
    !intersections(segs).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn pt(x: i128, xd: i128, y: i128, yd: i128) -> Pt {
        Pt {
            x: Frac::new(x, xd),
            y: Frac::new(y, yd),
        }
    }

    fn brute(segs: &[Seg]) -> Vec<Pt> {
        let n: Vec<Seg> = segs
            .iter()
            .map(|&(a, b)| norm(a, b))
            .filter(|&(a, b)| a != b)
            .collect();
        let mut set: BTreeSet<Pt> = BTreeSet::new();
        for i in 0..n.len() {
            for j in i + 1..n.len() {
                if let Some(p) = seg_isect(n[i], n[j]) {
                    set.insert(p);
                } else {
                    // Parallel pairs still report endpoints lying on
                    // the other segment (collinear-overlap endpoints).
                    for &end in &[n[i].0, n[i].1] {
                        let p = Pt {
                            x: Frac::from_int(end.0 as i128),
                            y: Frac::from_int(end.1 as i128),
                        };
                        if seg_contains(n[j], p) {
                            set.insert(p);
                        }
                    }
                    for &end in &[n[j].0, n[j].1] {
                        let p = Pt {
                            x: Frac::from_int(end.0 as i128),
                            y: Frac::from_int(end.1 as i128),
                        };
                        if seg_contains(n[i], p) {
                            set.insert(p);
                        }
                    }
                }
            }
        }
        set.into_iter().collect()
    }

    #[test]
    fn basics() {
        // Single crossing X.
        let pts = intersections(&[((0, 0), (4, 4)), ((0, 4), (4, 0))]);
        assert_eq!(pts, vec![pt(2, 1, 2, 1)]);
        // Disjoint parallels.
        assert!(intersections(&[((0, 0), (4, 0)), ((0, 2), (4, 2))]).is_empty());
        // Shared endpoint.
        assert_eq!(
            intersections(&[((0, 0), (4, 0)), ((0, 0), (0, 4))]),
            vec![pt(0, 1, 0, 1)]
        );
        // T junction: endpoint on interior.
        assert_eq!(
            intersections(&[((0, 0), (4, 0)), ((2, 0), (2, 3))]),
            vec![pt(2, 1, 0, 1)]
        );
        // Zero-length segments ignored.
        assert!(intersections(&[((1, 1), (1, 1)), ((0, 0), (2, 2))]).is_empty());
        assert!(any_intersection(&[((0, 0), (4, 4)), ((0, 4), (4, 0))]));
        assert!(!any_intersection(&[((0, 0), (4, 0)), ((0, 2), (4, 2))]));
    }

    #[test]
    fn grid_all_crossings() {
        let mut segs: Vec<Seg> = Vec::new();
        for x in 0..5i64 {
            segs.push(((x, 0), (x, 4)));
        }
        for y in 0..5i64 {
            segs.push(((0, y), (4, y)));
        }
        let pts = intersections(&segs);
        assert_eq!(pts.len(), 25);
    }

    #[test]
    fn concurrent_triple_point() {
        // Three segs through (2,2): all pairs meet at one point.
        let segs: Vec<Seg> = vec![((0, 0), (4, 4)), ((0, 4), (4, 0)), ((2, 0), (2, 4))];
        assert_eq!(intersections(&segs), vec![pt(2, 1, 2, 1)]);
    }

    #[test]
    fn sweep_matches_bruteforce() {
        let mut rng = SplitMix64::new(0xBE57);
        for _trial in 0..120 {
            let n = 3 + rng.below(14) as usize;
            let mut segs: Vec<Seg> = Vec::new();
            for _ in 0..n {
                let a = (rng.below(24) as i64, rng.below(24) as i64);
                let b = (rng.below(24) as i64, rng.below(24) as i64);
                if a != b {
                    segs.push((a, b));
                }
            }
            let a: BTreeSet<Pt> = intersections(&segs).into_iter().collect();
            let b: BTreeSet<Pt> = brute(&segs).into_iter().collect();
            let missing: Vec<_> = b.difference(&a).collect();
            let extra: Vec<_> = a.difference(&b).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "mismatch on {segs:?}\nmissing={missing:?}\nextra={extra:?}"
            );
        }
    }

    #[test]
    fn input_order_independence() {
        let mut rng = SplitMix64::new(0x5EED);
        let mut segs: Vec<Seg> = Vec::new();
        for _ in 0..12 {
            segs.push((
                (rng.below(20) as i64, rng.below(20) as i64),
                (rng.below(20) as i64, rng.below(20) as i64),
            ));
        }
        let a = intersections(&segs);
        segs.reverse();
        segs.rotate_left(3);
        assert_eq!(a, intersections(&segs));
    }
}
