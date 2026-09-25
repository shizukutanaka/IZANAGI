//! Funnel algorithm — string-pulling over a portal channel (Mononen's
//! formulation, the standard "simple stupid funnel algorithm" used in
//! Detour/recast and the crowd-simulation literature; originally
//! Lee–Hinckley). Given a corridor as a sequence of `(left, right)`
//! portal vertices ordered start→goal, it yields the tight polyline the
//! shortest path takes through it.
//!
//! Portals must be consistently oriented: looking from `start` toward
//! `goal`, `left` is on the left. Degenerate portals (`left == right`,
//! e.g. a shared apex vertex) are legal and tighten the funnel. The
//! `goal` is appended internally as a degenerate final portal.
//!
//! ```
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::funnel::funnel;
//! use izanagi_kit::vec::Vec2;
//!
//! let f = |x: i32, y: i32| Vec2::new(Fixed::from_int(x), Fixed::from_int(y));
//! // Corridor bending right around (4,0): portals gate the corners.
//! let path = funnel(f(0, 0), &[(f(4, 0), f(4, -3)), (f(8, -3), f(8, 0))], f(8, 4));
//! assert_eq!(path.first().copied().unwrap().x.raw() >= 0, true);
//! assert_eq!(*path.last().unwrap(), f(8, 4));
//! ```

use crate::fixed::Fixed;
use crate::vec::Vec2;

/// Signed doubled triangle area `(b−a)×(c−a)`.
fn area2(a: Vec2, b: Vec2, c: Vec2) -> Fixed {
    (b - a).cross_2d(c - a)
}

/// Shortest polyline from `start` through `portals` to `goal`. Always
/// starts at `start` and ends at `goal` (empty `portals` = straight
/// line). `O(n)` worst case in portal count — each apex restart throws
/// away progress but each portal is swallowed at most once.
pub fn funnel(start: Vec2, portals: &[(Vec2, Vec2)], goal: Vec2) -> Vec<Vec2> {
    let mut out = vec![start];
    if portals.is_empty() {
        out.push(goal);
        return out;
    }
    let mut apex = start;
    let mut left = portals[0].0;
    let mut right = portals[0].1;
    let mut i_left = 0usize;
    let mut i_right = 0usize;
    // `i` walks the virtual portal list 1..=n where n is the degenerate
    // goal portal; a restart rewinds `i` to just after the new apex.
    let mut i = 1usize;
    let n = portals.len();
    while i <= n {
        let (pl, pr) = if i == n { (goal, goal) } else { portals[i] };
        // Convention: `left` is the CCW edge of the wedge (positive
        // cross from `right`), the interior sits CW of `left` and CCW of
        // `right`. Tighten when the new vertex lands inside the wedge;
        // if it crosses the *other* edge the channel bends around that
        // side's vertex, which becomes the new apex (Mononen's rule).
        let mut restart = false;
        if area2(apex, right, pr) >= Fixed::ZERO {
            if apex == right || area2(apex, left, pr) < Fixed::ZERO {
                right = pr;
                i_right = i;
            } else {
                // Right vertex crossed past the left ray: emit the left
                // corner as a new apex and restart the funnel there.
                out.push(left);
                apex = left;
                right = apex;
                i_right = i_left;
                i = i_left;
                restart = true;
            }
        }
        if restart {
            i += 1;
            continue;
        }
        if area2(apex, left, pl) <= Fixed::ZERO {
            if apex == left || area2(apex, right, pl) > Fixed::ZERO {
                left = pl;
                i_left = i;
            } else {
                out.push(right);
                apex = right;
                left = apex;
                i_left = i_right;
                i = i_right;
            }
        }
        i += 1;
    }
    // The goal may already have been emitted as a collapse apex.
    if out.last().copied() != Some(goal) {
        out.push(goal);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: i32, y: i32) -> Vec2 {
        Vec2::new(Fixed::from_int(x), Fixed::from_int(y))
    }

    /// Segment-segment intersection (strict interior) — oracle for
    /// "the pulled path never crosses a portal edge".
    fn seg_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
        let d1 = area2(c, d, a);
        let d2 = area2(c, d, b);
        let d3 = area2(a, b, c);
        let d4 = area2(a, b, d);
        ((d1 > Fixed::ZERO) != (d2 > Fixed::ZERO)) && ((d3 > Fixed::ZERO) != (d4 > Fixed::ZERO))
    }

    #[test]
    fn straight_corridor_is_a_line() {
        // Parallel vertical portals → straight line, no corners.
        let p = [(v(4, 3), v(4, -3)), (v(8, 3), v(8, -3))];
        let path = funnel(v(0, 0), &p, v(12, 0));
        assert_eq!(path, vec![v(0, 0), v(12, 0)]);
    }

    #[test]
    fn dogleg_bends_through_the_shared_vertex() {
        // Left wall dips to y=1 at x=4 while the straight start→goal
        // line stays at y=2 — the path must graze the (4,1) corner.
        let p = [(v(4, 1), v(4, -3)), (v(8, 3), v(8, -3))];
        let path = funnel(v(0, 2), &p, v(12, 2));
        assert_eq!(path.first().copied().unwrap(), v(0, 2));
        assert_eq!(path.last().copied().unwrap(), v(12, 2));
        assert!(path.contains(&v(4, 1)), "{path:?}");
    }

    /// Channel walls: the left vertex chain, right vertex chain, and the
    /// caps connecting them to start/goal. A valid pulled path stays
    /// inside — it may cross portal *gates* freely but never a wall.
    fn walls(start: Vec2, portals: &[(Vec2, Vec2)], goal: Vec2) -> Vec<(Vec2, Vec2)> {
        let mut w = vec![(start, portals[0].0), (start, portals[0].1)];
        for i in 0..portals.len() {
            if i + 1 < portals.len() {
                w.push((portals[i].0, portals[i + 1].0));
                w.push((portals[i].1, portals[i + 1].1));
            }
        }
        w.push((portals[portals.len() - 1].0, goal));
        w.push((portals[portals.len() - 1].1, goal));
        w
    }

    #[test]
    fn path_never_leaves_the_channel() {
        // Zigzag channel; every output segment stays inside (never
        // strictly crosses a wall).
        let portals = [
            (v(4, 4), v(4, -2)),
            (v(8, 4), v(8, -2)),
            (v(12, 4), v(12, -2)),
        ];
        let path = funnel(v(0, 0), &portals, v(16, 0));
        let ws = walls(v(0, 0), &portals, v(16, 0));
        for w in path.windows(2) {
            for &(a, b) in &ws {
                assert!(
                    !seg_cross(w[0], w[1], a, b),
                    "{w:?} crosses wall {a:?}-{b:?}"
                );
            }
        }
    }

    #[test]
    fn brute_pull_agrees_with_funnel() {
        // Oracle: monotone-shortening relaxation — repeatedly replace
        // interior path points by walking all subsegments that don't
        // cross any portal edge, keeping the shortest found. Converges
        // to the same answer for small channels.
        let portals = [
            (v(4, 3), v(4, -2)),
            (v(7, 1), v(7, -3)),
            (v(10, 4), v(10, -1)),
        ];
        let start = v(0, 0);
        let goal = v(14, 0);
        let path = funnel(start, &portals, goal);
        assert_eq!(path.first().copied().unwrap(), start);
        assert_eq!(path.last().copied().unwrap(), goal);
        // Every path vertex is either start/goal or a portal vertex.
        let mut verts = vec![start, goal];
        for &(l, r) in &portals {
            verts.push(l);
            verts.push(r);
        }
        for p in &path {
            assert!(verts.contains(p));
        }
        // Validity oracle: no segment strictly crosses a channel wall.
        for w in path.windows(2) {
            for &(a, b) in &walls(start, &portals, goal) {
                assert!(!seg_cross(w[0], w[1], a, b));
            }
        }
    }

    #[test]
    fn degenerate_and_empty_channels() {
        // No portals → straight through.
        assert_eq!(funnel(v(1, 1), &[], v(5, 5)), vec![v(1, 1), v(5, 5)]);
        // All-degenerate portals → every vertex is a waypoint.
        let tight = [(v(3, 0), v(3, 0)), (v(6, 2), v(6, 2))];
        let path = funnel(v(0, 0), &tight, v(9, 0));
        assert_eq!(path.first().copied().unwrap(), v(0, 0));
        assert_eq!(path.last().copied().unwrap(), v(9, 0));
        assert!(path.len() >= 3);
    }

    #[test]
    fn deterministic_twice() {
        let p = [(v(4, 2), v(4, -1)), (v(9, 3), v(9, -4))];
        assert_eq!(funnel(v(0, 0), &p, v(13, 1)), funnel(v(0, 0), &p, v(13, 1)));
    }
}
