//! Integer Verlet integration with distance-constraint relaxation —
//! the classic Jakobsen rope/cloth solver in [`Fixed`] Q16.16.
//!
//! A [`World`] is a point set, per-point "previous position" (implicit
//! velocity), pinned flags, and rigid distance links. `step` integrates
//! `x' = x + (x − x_prev)·damping + a` then relaxes each link `iters`
//! times: both endpoints move half the `(len − rest)` error along their
//! connecting axis. Everything is integer math — the trace is a pure
//! function of `(points, links, pinned, damping, accel, iters, steps)`.
//!
//! ```
//! use izanagi_kit::verlet::{World, P2};
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::vec::Vec2;
//! // Two points, one link of rest length 4. Undamped Verlet
//! // conserves energy — damping < 1 is what converges.
//! let mut w = World::new(&[P2::new(0, 0), P2::new(6, 0)]);
//! w.link_len(0, 1, Fixed::from_int(4));
//! w.pin(0);
//! for _ in 0..64 {
//!     w.step(Vec2::ZERO, Fixed::from_ratio(9, 10), 4);
//! }
//! let d = w.distance(0, 1);
//! assert!((d - Fixed::from_int(4)).abs() < Fixed::from_ratio(1, 4));
//! ```
//!
//! Reference: Jakobsen, "Advanced Character Physics" (GDC 2001).

use crate::fixed::Fixed;
use crate::vec::Vec2;

/// Integer point shorthand for construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct P2 {
    /// X coordinate (whole units).
    pub x: i32,
    /// Y coordinate (whole units).
    pub y: i32,
}

impl P2 {
    /// Integer point.
    pub const fn new(x: i32, y: i32) -> P2 {
        P2 { x, y }
    }
    /// Lift to Q16.16.
    pub fn fixed(self) -> Vec2 {
        Vec2::new(Fixed::from_int(self.x), Fixed::from_int(self.y))
    }
}

/// A rigid distance link between two point indices.
#[derive(Clone, Copy, Debug)]
struct Link {
    a: u32,
    b: u32,
    rest: Fixed,
}

/// A Verlet world: positions, previous positions, pins, links.
pub struct World {
    pos: Vec<Vec2>,
    prev: Vec<Vec2>,
    links: Vec<Link>,
    pinned: Vec<bool>,
}

impl World {
    /// Points at rest (previous == current).
    pub fn new(points: &[P2]) -> World {
        let pos: Vec<Vec2> = points.iter().map(|p| p.fixed()).collect();
        World {
            prev: pos.clone(),
            pinned: vec![false; pos.len()],
            pos,
            links: Vec::new(),
        }
    }

    /// Teleport point `i` (clears its velocity).
    pub fn set_pos(&mut self, i: usize, p: Vec2) {
        if i < self.pos.len() {
            self.pos[i] = p;
            self.prev[i] = p;
        }
    }

    /// Fix point `i` in place (constraints never move it).
    pub fn pin(&mut self, i: usize) {
        if i < self.pinned.len() {
            self.pinned[i] = true;
        }
    }

    /// Release point `i`.
    pub fn unpin(&mut self, i: usize) {
        if i < self.pinned.len() {
            self.pinned[i] = false;
        }
    }

    /// Constrain `a`–`b` to their *current* distance.
    pub fn link(&mut self, a: u32, b: u32) {
        let (ia, ib) = (a as usize, b as usize);
        if ia >= self.pos.len() || ib >= self.pos.len() || a == b {
            return;
        }
        let rest = (self.pos[ib] - self.pos[ia]).len();
        self.links.push(Link { a, b, rest });
    }

    /// Constrain `a`–`b` to an explicit length.
    pub fn link_len(&mut self, a: u32, b: u32, rest: Fixed) {
        let (ia, ib) = (a as usize, b as usize);
        if ia >= self.pos.len() || ib >= self.pos.len() || a == b {
            return;
        }
        self.links.push(Link { a, b, rest });
    }

    /// Current positions.
    pub fn positions(&self) -> &[Vec2] {
        &self.pos
    }

    /// Distance between two points right now.
    pub fn distance(&self, a: usize, b: usize) -> Fixed {
        (self.pos[b] - self.pos[a]).len()
    }

    /// One tick: integrate then relax links `iters` times.
    ///
    /// `accel` is added per-tick (gravity in units/tick², dt fixed to 1
    /// tick); `damping` scales the implicit velocity (`Fixed::ONE` =
    /// undamped, `Fixed::ZERO` = full stop).
    pub fn step(&mut self, accel: Vec2, damping: Fixed, iters: usize) {
        for i in 0..self.pos.len() {
            if self.pinned[i] {
                self.prev[i] = self.pos[i];
                continue;
            }
            let v = (self.pos[i] - self.prev[i]).scale(damping);
            let next = self.pos[i] + v + accel;
            self.prev[i] = self.pos[i];
            self.pos[i] = next;
        }
        for _ in 0..iters {
            self.relax();
        }
    }

    /// One relaxation sweep over all links, in insertion order — the
    /// order is part of the trace, so it's a fixed list, not a set.
    fn relax(&mut self) {
        let half = Fixed::from_ratio(1, 2);
        for li in 0..self.links.len() {
            let Link { a, b, rest } = self.links[li];
            let (ia, ib) = (a as usize, b as usize);
            let delta = self.pos[ib] - self.pos[ia];
            let len = delta.len();
            if len == Fixed::ZERO {
                continue; // coincident points: no direction to pull
            }
            let err = len - rest;
            if err == Fixed::ZERO {
                continue;
            }
            // Each endpoint moves half the error along the axis.
            let corr = delta.scale(err.checked_div(len).unwrap_or(Fixed::ZERO).mul(half));
            let (fa, fb) = (self.pinned[ia], self.pinned[ib]);
            match (fa, fb) {
                (true, true) => {}
                (true, false) => self.pos[ib] = self.pos[ib] - corr.scale(Fixed::from_int(2)),
                (false, true) => self.pos[ia] = self.pos[ia] + corr.scale(Fixed::from_int(2)),
                (false, false) => {
                    self.pos[ia] = self.pos[ia] + corr;
                    self.pos[ib] = self.pos[ib] - corr;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn settles_to_rest() {
        // Pinned anchor + hanging chain: links converge to rest.
        let mut w = World::new(&[P2::new(0, 0), P2::new(2, 0), P2::new(4, 0)]);
        w.link(0, 1);
        w.link(1, 2);
        w.pin(0);
        let grav = Vec2::new(Fixed::ZERO, Fixed::from_ratio(1, 10));
        for _ in 0..64 {
            w.step(grav, Fixed::ONE, 8);
        }
        for pair in [(0usize, 1usize), (1, 2)] {
            let d = w.distance(pair.0, pair.1);
            assert!((d - Fixed::from_int(2)).abs() < Fixed::from_ratio(1, 16));
        }
        // Anchor unmoved.
        assert_eq!(w.positions()[0], Vec2::ZERO);
    }

    #[test]
    fn free_fall_is_quadratic() {
        // One free point under gravity: positions follow
        // a·t·(t+1)/2 exactly (Verlet accumulates velocity).
        let mut w = World::new(&[P2::new(0, 0)]);
        w.pin(0);
        w.unpin(0); // pinned then released — free again
        let a = Vec2::new(Fixed::ZERO, Fixed::from_int(1));
        for t in 1..6 {
            w.step(a, Fixed::ONE, 0);
            let want = Fixed::from_int(t * (t + 1) / 2);
            assert_eq!(w.positions()[0].y, want);
        }
    }

    #[test]
    fn momentum_preserved() {
        // No accel, no links: constant implicit velocity.
        let mut w = World::new(&[P2::new(0, 0)]);
        w.set_pos(0, Vec2::new(Fixed::from_int(5), Fixed::ZERO));
        w.prev[0] = Vec2::new(Fixed::from_int(4), Fixed::ZERO); // v = +1
        w.step(Vec2::ZERO, Fixed::ONE, 0);
        w.step(Vec2::ZERO, Fixed::ONE, 0);
        assert_eq!(w.positions()[0].x, Fixed::from_int(7));
        // Damping zero kills velocity in one tick.
        let mut w2 = World::new(&[P2::new(0, 0)]);
        w2.set_pos(0, Vec2::new(Fixed::from_int(5), Fixed::ZERO));
        w2.prev[0] = Vec2::new(Fixed::from_int(4), Fixed::ZERO);
        w2.step(Vec2::ZERO, Fixed::ZERO, 0);
        w2.step(Vec2::ZERO, Fixed::ONE, 0);
        assert_eq!(w2.positions()[0].x, Fixed::from_int(5));
    }

    #[test]
    fn oracle_convergence() {
        // Two free points + one link: each sweep shrinks the error
        // (halving per iteration) — never grows.
        let mut rng = SplitMix64::new(0x5e2e_1e7a_a11d_f00d);
        for _case in 0..60 {
            let ax = rng.below(20) as i32 - 10;
            let ay = rng.below(20) as i32 - 10;
            let bx = ax + 1 + rng.below(9) as i32;
            let by = ay + rng.below(9) as i32 - 4;
            let mut w = World::new(&[P2::new(ax, ay), P2::new(bx, by)]);
            let rest = Fixed::from_int(1 + rng.below(6) as i32);
            w.link_len(0, 1, rest);
            let err0 = (w.distance(0, 1) - rest).abs().raw();
            let damp = Fixed::from_ratio(9, 10);
            // Energy never appears from nowhere: the error never exceeds
            // the initial stretch (plus small slack for the relax tick).
            for _ in 0..96 {
                w.step(Vec2::ZERO, damp, 1);
                assert!((w.distance(0, 1) - rest).abs().raw() <= err0 + 1024);
            }
            // Damped relaxation converges to a few Q16 quanta.
            assert!((w.distance(0, 1) - rest).abs().raw() <= 64);
        }
    }

    #[test]
    fn deterministic_replay() {
        // Two identical worlds stepped identically are bit-equal.
        let build = || {
            let mut w = World::new(&[P2::new(0, 0), P2::new(3, 0), P2::new(6, 0), P2::new(9, 0)]);
            w.link(0, 1);
            w.link(1, 2);
            w.link(2, 3);
            w.pin(0);
            w
        };
        let (mut a, mut b) = (build(), build());
        let g = Vec2::new(Fixed::ZERO, Fixed::from_ratio(1, 4));
        for _ in 0..32 {
            a.step(g, Fixed::from_ratio(9, 10), 6);
            b.step(g, Fixed::from_ratio(9, 10), 6);
        }
        assert_eq!(a.positions(), b.positions());
    }
}
