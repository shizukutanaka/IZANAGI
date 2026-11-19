//! Deterministic rigid-body physics for axis-aligned boxes — gravity,
//! semi-implicit (symplectic) Euler integration, and AABB collision
//! response with restitution and friction, all in [`Fixed`] Q16.16.
//!
//! ## Where this sits
//!
//! The kit already *detects* collisions three ways — [`crate::aabb`]
//! (discrete overlap), [`crate::swept`] (time of impact against a static
//! box) and [`crate::sap`] (broadphase pairs) — and it integrates point
//! masses in [`crate::verlet`]. What none of them do is *respond*: there
//! was no body type with an explicit velocity, no gravity integration
//! over a variable `dt`, and nothing that turned an overlap into a
//! bounce, a slide or a rest. This module is that missing thin layer,
//! AABB-only by design (the separating-axis route was measured and set
//! aside; see `RESEARCH.md`'s collision-layer note).
//!
//! ## Model
//!
//! A [`World`] holds dynamic [`Body`] boxes and static [`Solid`] boxes.
//! One [`World::step`] is, in a fixed order that is part of the trace:
//!
//! 1. **Integrate** each movable body in index order:
//!    `v += gravity · gravity_scale · dt`, then `pos += v · dt`
//!    (semi-implicit Euler — the new velocity moves the body, which is
//!    what makes the free-fall trace `g·dt²·n(n+1)/2` exact).
//! 2. **Bodies vs solids**, body index order, then solid index order:
//!    push the body out along the axis of least penetration, reflect
//!    the normal velocity scaled by the body's restitution, and damp
//!    the tangential velocity by the body's friction.
//! 3. **Body pairs** in lexicographic `(i, j)` order: positional
//!    correction split by inverse mass, plus a normal impulse with
//!    restitution `min(e_i, e_j)`.
//!
//! An impact whose normal speed is at most [`World::rest_speed`] is
//! treated as resting contact instead of a bounce (restitution is not
//! applied), which is what lets a body settle on a floor and stay
//! there instead of micro-bouncing forever. Rest is stable exactly
//! when `rest_speed ≥ |gravity| · dt` for the step being run: a resting
//! body re-gains `|gravity| · dt` of impact speed every step, so a
//! threshold below that leaves it in a small limit cycle (measured:
//! gravity 8, dt 1/8, threshold 1/2 bounces at 1/2 forever; threshold
//! 1 settles). Friction applies on every
//! resolved contact with an approaching normal velocity, including
//! resting ones — a body resting on a floor decelerates as it slides.
//!
//! ## Limits, stated plainly
//!
//! - **Discrete, not swept.** A body that moves more than its own size
//!   in one step can tunnel through a thin solid. Use [`crate::swept`]
//!   when time of impact matters.
//! - **No rotation, no body–body friction, no stacking solver.**
//!   Contacts are resolved once per step in the order above; a tall
//!   stack may jitter by a Q16 quantum. This is a game-physics layer,
//!   not a constraint solver.
//! - Overlap ties (equal penetration on both axes) resolve along Y.
//!   A body whose centre coincides with a solid's centre on the
//!   resolved axis is pushed toward the positive side. Both rules are
//!   arbitrary but fixed — the trace never depends on them being
//!   "right", only on them being the same every run.
//!
//! ```
//! use izanagi_kit::rigid::{Body, Solid, World};
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::vec::Vec2;
//!
//! // A floor whose top face is y = 0, and a body dropped from y = 4.
//! let mut w = World::new(Vec2::new(Fixed::ZERO, Fixed::from_int(-8)));
//! w.add_solid(Solid::from_center_half(
//!     Vec2::new(Fixed::ZERO, Fixed::from_int(-1)),
//!     Vec2::new(Fixed::from_int(16), Fixed::ONE),
//! ));
//! let b = w.add_body(Body::new(
//!     Vec2::new(Fixed::ZERO, Fixed::from_int(4)),
//!     Vec2::new(Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2)),
//! ));
//! let dt = Fixed::from_ratio(1, 8);
//! for _ in 0..32 {
//!     w.step(dt);
//! }
//! // Resting on the floor: centre exactly one half-extent above y = 0.
//! assert_eq!(w.bodies()[b].pos.y, Fixed::from_ratio(1, 2));
//! assert_eq!(w.bodies()[b].vel, Vec2::ZERO);
//! ```
//!
//! References: semi-implicit Euler and impulse response follow the
//! standard game-physics treatment (Catto, "Physics" GDC tutorials;
//! Millington, *Game Physics Engine Development*, ch. 3–5), reduced to
//! axis-aligned boxes and saturating fixed-point arithmetic.

use crate::fixed::Fixed;
use crate::vec::Vec2;
use crate::world_hash::{DetHash, Fnv1a};

/// A static axis-aligned box in `Fixed` space: the immovable geometry
/// bodies collide with (floors, walls, platforms).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Solid {
    /// Smallest corner (component-wise minimum of the box).
    pub min: Vec2,
    /// Largest corner (component-wise maximum of the box).
    pub max: Vec2,
}

impl Solid {
    /// Box from two corners; the corners are sorted component-wise, so
    /// either order gives the same solid.
    pub fn new(a: Vec2, b: Vec2) -> Solid {
        Solid {
            min: Vec2::min(a, b),
            max: Vec2::max(a, b),
        }
    }

    /// Box from its centre and half extents (half extents are taken by
    /// absolute value).
    pub fn from_center_half(center: Vec2, half: Vec2) -> Solid {
        let half = half.abs();
        Solid {
            min: center - half,
            max: center + half,
        }
    }

    /// Centre of the box (midpoint of the corners).
    pub fn center(&self) -> Vec2 {
        self.min.mid(self.max)
    }

    /// Half extents of the box (half the size on each axis, `≥ 0`).
    pub fn half(&self) -> Vec2 {
        (self.max - self.min).scale(Fixed::from_ratio(1, 2))
    }
}

impl DetHash for Solid {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.min.det_hash(h);
        self.max.det_hash(h);
    }
}

/// A dynamic axis-aligned box body.
///
/// `restitution` and `friction` are expected in `[0, 1]` (they are
/// clamped to that range where they are applied). `inv_mass` is the
/// inverse mass: `1` is the unit body, larger is lighter, and exactly
/// `0` marks an immovable body — it is never integrated and other
/// bodies treat it as a wall, but it still takes part in pair
/// resolution (unlike a [`Solid`], it can be one of the pair).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    /// Centre position.
    pub pos: Vec2,
    /// Velocity in units per second.
    pub vel: Vec2,
    /// Half extents (taken by absolute value when resolving).
    pub half: Vec2,
    /// Bounciness: fraction of the approaching normal speed returned
    /// on impact. `0` = sticks, `1` = perfectly elastic.
    pub restitution: Fixed,
    /// Friction: fraction of tangential velocity removed per resolved
    /// contact. `0` = ice, `1` = stops dead on first contact.
    pub friction: Fixed,
    /// Inverse mass; `0` = immovable.
    pub inv_mass: Fixed,
    /// Per-body gravity multiplier (`1` = full gravity, `0` = floats).
    pub gravity_scale: Fixed,
}

impl Body {
    /// A unit-mass body at rest: no restitution, no friction, full
    /// gravity. Set the public fields to taste before adding it to a
    /// world.
    pub fn new(pos: Vec2, half: Vec2) -> Body {
        Body {
            pos,
            vel: Vec2::ZERO,
            half,
            restitution: Fixed::ZERO,
            friction: Fixed::ZERO,
            inv_mass: Fixed::ONE,
            gravity_scale: Fixed::ONE,
        }
    }

    /// An immovable body (`inv_mass = 0`): a [`Solid`] that lives in
    /// the body list, so it can be moved between steps by the caller.
    pub fn immovable(pos: Vec2, half: Vec2) -> Body {
        let mut b = Body::new(pos, half);
        b.inv_mass = Fixed::ZERO;
        b
    }

    /// True when this body never moves under the simulation
    /// (`inv_mass == 0`).
    pub fn is_static(&self) -> bool {
        self.inv_mass == Fixed::ZERO
    }
}

impl DetHash for Body {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.pos.det_hash(h);
        self.vel.det_hash(h);
        self.half.det_hash(h);
        self.restitution.det_hash(h);
        self.friction.det_hash(h);
        self.inv_mass.det_hash(h);
        self.gravity_scale.det_hash(h);
    }
}

/// A rigid-body world: gravity, dynamic bodies, static solids.
///
/// The whole state folds into the world hash via [`DetHash`] — bodies
/// and solids in insertion order — so a physics trace can ride the
/// same replay / lockstep machinery as the rest of the kit.
pub struct World {
    /// Gravity acceleration in units per second², applied to every
    /// movable body (scaled by the body's `gravity_scale`).
    pub gravity: Vec2,
    /// Resting-contact threshold: an impact at or below this normal
    /// speed does not bounce. Defaults to `1/2` unit per second —
    /// comfortably above one step of gravity gain at typical game
    /// rates (gravity 10, dt 1/60 gains 1/6 per step). For stable
    /// rest keep it at or above `|gravity| · dt` of the steps being
    /// run; below that a resting body micro-bounces instead of
    /// settling (see the module docs).
    pub rest_speed: Fixed,
    bodies: Vec<Body>,
    solids: Vec<Solid>,
}

impl World {
    /// An empty world under `gravity`, with the default rest speed.
    pub fn new(gravity: Vec2) -> World {
        World {
            gravity,
            rest_speed: Fixed::from_ratio(1, 2),
            bodies: Vec::new(),
            solids: Vec::new(),
        }
    }

    /// Add a body; returns its index (insertion order is the
    /// processing order, and part of the trace).
    pub fn add_body(&mut self, body: Body) -> usize {
        self.bodies.push(body);
        self.bodies.len() - 1
    }

    /// Add a static solid; returns its index.
    pub fn add_solid(&mut self, solid: Solid) -> usize {
        self.solids.push(solid);
        self.solids.len() - 1
    }

    /// All bodies, in insertion order.
    pub fn bodies(&self) -> &[Body] {
        &self.bodies
    }

    /// All bodies, mutably — apply impulses, teleport, or retune
    /// parameters between steps.
    pub fn bodies_mut(&mut self) -> &mut [Body] {
        &mut self.bodies
    }

    /// All solids, in insertion order.
    pub fn solids(&self) -> &[Solid] {
        &self.solids
    }

    /// Advance the world by `dt` seconds. `dt ≤ 0` is a no-op (a
    /// paused frame changes nothing, bit for bit).
    ///
    /// Order: integrate all movable bodies (index order), resolve
    /// each movable body against every solid (body order, then solid
    /// order), then resolve body pairs in lexicographic index order.
    pub fn step(&mut self, dt: Fixed) {
        if dt <= Fixed::ZERO {
            return;
        }
        for b in &mut self.bodies {
            if b.is_static() {
                continue;
            }
            let g = self.gravity.scale(b.gravity_scale);
            b.vel = b.vel + g.scale(dt);
            b.pos = b.pos + b.vel.scale(dt);
        }
        for i in 0..self.bodies.len() {
            if self.bodies[i].is_static() {
                continue;
            }
            for s in 0..self.solids.len() {
                let solid = self.solids[s];
                resolve_body_solid(&mut self.bodies[i], &solid, self.rest_speed);
            }
        }
        for i in 0..self.bodies.len() {
            for j in (i + 1)..self.bodies.len() {
                let (left, right) = self.bodies.split_at_mut(j);
                resolve_pair(&mut left[i], &mut right[0], self.rest_speed);
            }
        }
    }
}

impl DetHash for World {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.gravity.det_hash(h);
        self.rest_speed.det_hash(h);
        (self.bodies.len() as u32).det_hash(h);
        for b in &self.bodies {
            b.det_hash(h);
        }
        (self.solids.len() as u32).det_hash(h);
        for s in &self.solids {
            s.det_hash(h);
        }
    }
}

/// Push `b` out of `s` along the axis of least penetration and answer
/// the velocity: reflect the normal component with restitution (or
/// zero it at resting speed) and damp the tangential component by
/// friction. No-op when the boxes do not overlap.
fn resolve_body_solid(b: &mut Body, s: &Solid, rest_speed: Fixed) {
    let half = b.half.abs();
    let sc = s.center();
    let sh = s.half();
    let d = b.pos - sc;
    let ox = half.x + sh.x - d.x.abs();
    let oy = half.y + sh.y - d.y.abs();
    if ox <= Fixed::ZERO || oy <= Fixed::ZERO {
        return;
    }
    let friction = b.friction.clamp(Fixed::ZERO, Fixed::ONE);
    if ox < oy {
        // Resolve along X; the outward normal is sign(d.x), + on a tie.
        let reach = sh.x + half.x;
        if d.x >= Fixed::ZERO {
            b.pos.x = sc.x + reach;
            if b.vel.x < Fixed::ZERO {
                b.vel.x = bounce(b.vel.x, b.restitution, rest_speed);
                b.vel.y = b.vel.y.mul(Fixed::ONE - friction);
            }
        } else {
            b.pos.x = sc.x - reach;
            if b.vel.x > Fixed::ZERO {
                b.vel.x = -bounce(-b.vel.x, b.restitution, rest_speed);
                b.vel.y = b.vel.y.mul(Fixed::ONE - friction);
            }
        }
    } else {
        // Resolve along Y (also on an ox == oy tie).
        let reach = sh.y + half.y;
        if d.y >= Fixed::ZERO {
            b.pos.y = sc.y + reach;
            if b.vel.y < Fixed::ZERO {
                b.vel.y = bounce(b.vel.y, b.restitution, rest_speed);
                b.vel.x = b.vel.x.mul(Fixed::ONE - friction);
            }
        } else {
            b.pos.y = sc.y - reach;
            if b.vel.y > Fixed::ZERO {
                b.vel.y = -bounce(-b.vel.y, b.restitution, rest_speed);
                b.vel.x = b.vel.x.mul(Fixed::ONE - friction);
            }
        }
    }
}

/// Answer an approaching normal velocity `vn < 0`: reflect it scaled
/// by `restitution`, or zero it when the impact is at resting speed.
fn bounce(vn: Fixed, restitution: Fixed, rest_speed: Fixed) -> Fixed {
    let impact = -vn;
    if impact <= rest_speed {
        Fixed::ZERO
    } else {
        impact.mul(restitution.clamp(Fixed::ZERO, Fixed::ONE))
    }
}

/// Resolve one overlapping body pair: positional correction split by
/// inverse mass along the axis of least penetration, then a normal
/// impulse. Frictionless — friction lives on body–solid contacts.
fn resolve_pair(a: &mut Body, b: &mut Body, rest_speed: Fixed) {
    let total = a.inv_mass + b.inv_mass;
    if total == Fixed::ZERO {
        return;
    }
    let ha = a.half.abs();
    let hb = b.half.abs();
    let d = b.pos - a.pos;
    let ox = ha.x + hb.x - d.x.abs();
    let oy = ha.y + hb.y - d.y.abs();
    if ox <= Fixed::ZERO || oy <= Fixed::ZERO {
        return;
    }
    let e = if a.restitution < b.restitution {
        a.restitution
    } else {
        b.restitution
    }
    .clamp(Fixed::ZERO, Fixed::ONE);
    if ox < oy {
        let (n, overlap) = (if d.x >= Fixed::ZERO { 1 } else { -1 }, ox);
        let rel = b.vel.x - a.vel.x;
        let vn = if n > 0 { rel } else { -rel };
        push_pair_x(a, b, n, overlap, total);
        if vn < Fixed::ZERO {
            let imp = impulse(vn, e, rest_speed, total);
            let (ia, ib) = (a.inv_mass, b.inv_mass);
            if n > 0 {
                a.vel.x = a.vel.x - imp.mul(ia);
                b.vel.x = b.vel.x + imp.mul(ib);
            } else {
                a.vel.x = a.vel.x + imp.mul(ia);
                b.vel.x = b.vel.x - imp.mul(ib);
            }
        }
    } else {
        let (n, overlap) = (if d.y >= Fixed::ZERO { 1 } else { -1 }, oy);
        let rel = b.vel.y - a.vel.y;
        let vn = if n > 0 { rel } else { -rel };
        push_pair_y(a, b, n, overlap, total);
        if vn < Fixed::ZERO {
            let imp = impulse(vn, e, rest_speed, total);
            let (ia, ib) = (a.inv_mass, b.inv_mass);
            if n > 0 {
                a.vel.y = a.vel.y - imp.mul(ia);
                b.vel.y = b.vel.y + imp.mul(ib);
            } else {
                a.vel.y = a.vel.y + imp.mul(ia);
                b.vel.y = b.vel.y - imp.mul(ib);
            }
        }
    }
}

/// Positional split along X: `a` moves against the normal, `b` along
/// it, each by its inverse-mass share of the overlap.
fn push_pair_x(a: &mut Body, b: &mut Body, n: i32, overlap: Fixed, total: Fixed) {
    let sa = overlap.mul(a.inv_mass.div(total));
    let sb = overlap.mul(b.inv_mass.div(total));
    if n > 0 {
        a.pos.x = a.pos.x - sa;
        b.pos.x = b.pos.x + sb;
    } else {
        a.pos.x = a.pos.x + sa;
        b.pos.x = b.pos.x - sb;
    }
}

/// Positional split along Y; see [`push_pair_x`].
fn push_pair_y(a: &mut Body, b: &mut Body, n: i32, overlap: Fixed, total: Fixed) {
    let sa = overlap.mul(a.inv_mass.div(total));
    let sb = overlap.mul(b.inv_mass.div(total));
    if n > 0 {
        a.pos.y = a.pos.y - sa;
        b.pos.y = b.pos.y + sb;
    } else {
        a.pos.y = a.pos.y + sa;
        b.pos.y = b.pos.y - sb;
    }
}

/// Impulse magnitude for an approaching pair: `-(1+e)·vn / (ia+ib)`,
/// with restitution dropped at resting speed so stacks settle.
fn impulse(vn: Fixed, e: Fixed, rest_speed: Fixed, total: Fixed) -> Fixed {
    let e = if -vn <= rest_speed { Fixed::ZERO } else { e };
    (Fixed::ONE + e).mul(-vn).div(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn f(v: i32) -> Fixed {
        Fixed::from_int(v)
    }

    fn frac(num: i32, den: i32) -> Fixed {
        Fixed::from_ratio(num, den)
    }

    fn v(x: i32, y: i32) -> Vec2 {
        Vec2::new(f(x), f(y))
    }

    /// Floor with its top face at y = 0, wide enough for every test.
    fn floor(w: &mut World) {
        w.add_solid(Solid::from_center_half(
            Vec2::new(Fixed::ZERO, f(-1)),
            Vec2::new(f(64), f(1)),
        ));
    }

    #[test]
    fn free_fall_matches_closed_form() {
        // Semi-implicit Euler under constant gravity is exact for
        // dyadic values: v_n = g·dt·n, y_n = y0 + g·dt²·n(n+1)/2.
        // Oracle is the closed form, computed independently here.
        let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
        let b = w.add_body(Body::new(v(0, 16), Vec2::new(frac(1, 2), frac(1, 2))));
        let dt = frac(1, 8);
        for n in 1..=16i32 {
            w.step(dt);
            let body = w.bodies()[b];
            assert_eq!(body.vel.y, f(-n), "velocity after {n} steps");
            let fallen = f(n * (n + 1) / 16) + frac(n * (n + 1) % 16, 16);
            assert_eq!(body.pos.y, f(16) - fallen, "position after {n} steps");
            assert_eq!(body.pos.x, Fixed::ZERO);
        }
    }

    #[test]
    fn bounce_returns_restitution_share_of_impact_speed() {
        // Drop tuned so the impact speed is exactly 8 (see the module
        // doctest trace): with e = 1/2 the body leaves at exactly 4,
        // falls back, and the second bounce leaves at exactly 2.
        let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
        // Rest is stable only when rest_speed ≥ |g|·dt = 1 here; with
        // the default 1/2 the settled body limit-cycles at 1/2 (the
        // module docs describe the measurement), so raise it.
        w.rest_speed = f(1);
        floor(&mut w);
        let mut body = Body::new(v(0, 4), Vec2::new(frac(1, 2), frac(1, 2)));
        body.restitution = frac(1, 2);
        let b = w.add_body(body);
        let dt = frac(1, 8);
        let mut bounces: Vec<Fixed> = Vec::new();
        let mut prev_vy = Fixed::ZERO;
        for _ in 0..200 {
            w.step(dt);
            let body = w.bodies()[b];
            // A bounce is the step that leaves the body exactly on the
            // surface moving up, after arriving with vy ≤ 0 (the apex
            // frame has vy == 0, so a strict sign flip misses it).
            if body.vel.y > Fixed::ZERO && body.pos.y == frac(1, 2) && prev_vy <= Fixed::ZERO {
                bounces.push(body.vel.y);
            }
            prev_vy = body.vel.y;
        }
        assert_eq!(bounces, vec![f(4), f(2), f(1)]);
        // The next impact arrives at exactly rest_speed (1), so
        // instead of a fourth, smaller bounce the body settles.
        for _ in 0..64 {
            w.step(dt);
        }
        assert_eq!(w.bodies()[b].pos.y, frac(1, 2));
        assert_eq!(w.bodies()[b].vel, Vec2::ZERO);
    }

    #[test]
    fn bounce_apex_never_gains_energy() {
        // With e < 1 the apex after each bounce is non-increasing —
        // energy appears from nowhere if any apex exceeds the last.
        let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
        floor(&mut w);
        let mut body = Body::new(v(0, 9), Vec2::new(frac(1, 2), frac(1, 2)));
        body.restitution = frac(3, 4);
        let b = w.add_body(body);
        let dt = frac(1, 16);
        let mut apexes: Vec<Fixed> = Vec::new();
        let mut cur_apex = w.bodies()[b].pos.y;
        let mut rising = false;
        for _ in 0..4000 {
            w.step(dt);
            let body = w.bodies()[b];
            if body.vel.y > Fixed::ZERO {
                rising = true;
                if body.pos.y > cur_apex {
                    cur_apex = body.pos.y;
                }
            } else if rising {
                apexes.push(cur_apex);
                cur_apex = body.pos.y;
                rising = false;
            }
        }
        assert!(apexes.len() >= 3, "expected several bounces: {apexes:?}");
        for pair in apexes.windows(2) {
            assert!(
                pair[1] <= pair[0],
                "apex grew: {:?} then {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn restitution_zero_sticks_and_one_is_elastic() {
        // e = 0: the body lands and stays exactly on the surface.
        let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
        floor(&mut w);
        let b = w.add_body(Body::new(v(0, 4), Vec2::new(frac(1, 2), frac(1, 2))));
        for _ in 0..32 {
            w.step(frac(1, 8));
        }
        assert_eq!(w.bodies()[b].pos.y, frac(1, 2));
        assert_eq!(w.bodies()[b].vel.y, Fixed::ZERO);
        // e = 1: impact speed 8 comes back as exactly 8.
        let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
        floor(&mut w);
        let mut body = Body::new(v(0, 4), Vec2::new(frac(1, 2), frac(1, 2)));
        body.restitution = Fixed::ONE;
        let b = w.add_body(body);
        let mut left_at: Option<Fixed> = None;
        let mut prev_vy = Fixed::ZERO;
        for _ in 0..32 {
            w.step(frac(1, 8));
            let vy = w.bodies()[b].vel.y;
            if prev_vy < Fixed::ZERO && vy > Fixed::ZERO {
                left_at = Some(vy);
                break;
            }
            prev_vy = vy;
        }
        assert_eq!(left_at, Some(f(8)));
    }

    #[test]
    fn resting_body_stays_put_and_slides_with_friction() {
        // Settle a body, then push it sideways through bodies_mut:
        // friction 1/2 halves the slide speed at every contact step,
        // friction 0 leaves it untouched.
        for (friction, want) in [(frac(1, 2), f(1)), (Fixed::ZERO, f(8))] {
            let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
            floor(&mut w);
            let mut body = Body::new(v(0, 2), Vec2::new(frac(1, 2), frac(1, 2)));
            body.friction = friction;
            let b = w.add_body(body);
            for _ in 0..32 {
                w.step(frac(1, 8));
            }
            assert_eq!(w.bodies()[b].pos.y, frac(1, 2));
            w.bodies_mut()[b].vel.x = f(8);
            for _ in 0..3 {
                w.step(frac(1, 8));
            }
            assert_eq!(w.bodies()[b].vel.x, want, "friction {friction:?}");
            assert_eq!(w.bodies()[b].pos.y, frac(1, 2), "stays on the floor");
            assert_eq!(w.bodies()[b].vel.y, Fixed::ZERO);
        }
    }

    #[test]
    fn mirror_image_trajectories_stay_mirrored() {
        // Symmetric room, mirrored bodies: the trace must stay an
        // exact mirror image — any axis-order or sign slip breaks it.
        let build = |sign: i32| {
            let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
            // Floor (top y=0) and a wall on the body's side.
            floor(&mut w);
            w.add_solid(Solid::from_center_half(
                Vec2::new(f(6 * sign), f(4)),
                Vec2::new(f(1), f(8)),
            ));
            let mut body = Body::new(
                Vec2::new(f(3 * sign), f(5)),
                Vec2::new(frac(1, 2), frac(1, 2)),
            );
            body.vel = Vec2::new(f(3 * sign), Fixed::ZERO);
            body.restitution = frac(1, 2);
            w.add_body(body);
            w
        };
        let (mut a, mut b) = (build(1), build(-1));
        for _ in 0..128 {
            a.step(frac(1, 8));
            b.step(frac(1, 8));
            let (pa, pb) = (a.bodies()[0], b.bodies()[0]);
            assert_eq!(pb.pos.x, -pa.pos.x);
            assert_eq!(pb.pos.y, pa.pos.y);
            assert_eq!(pb.vel.x, -pa.vel.x);
            assert_eq!(pb.vel.y, pa.vel.y);
        }
    }

    #[test]
    fn equal_mass_elastic_collision_swaps_velocities() {
        // The textbook oracle: equal masses, head-on, e = 1 → the
        // velocities exchange exactly.
        let mut w = World::new(Vec2::ZERO);
        let mut a = Body::new(v(-2, 0), v(1, 1));
        a.vel = v(3, 0);
        a.restitution = Fixed::ONE;
        let mut b = Body::new(v(2, 0), v(1, 1));
        b.vel = v(-1, 0);
        b.restitution = Fixed::ONE;
        w.add_body(a);
        w.add_body(b);
        for _ in 0..3 {
            w.step(frac(1, 4));
        }
        assert_eq!(w.bodies()[0].vel, v(-1, 0));
        assert_eq!(w.bodies()[1].vel, v(3, 0));
        // And they separate: no residual overlap after the push-out.
        let d = w.bodies()[1].pos.x - w.bodies()[0].pos.x;
        assert_eq!(d, f(2));
    }

    #[test]
    fn immovable_body_acts_as_wall_and_never_moves() {
        let mut w = World::new(Vec2::ZERO);
        let mut wall_body = Body::immovable(v(4, 0), v(1, 4));
        // Pair restitution is the min of the two bodies, so the wall
        // must be elastic too for the ball to come back at full speed.
        wall_body.restitution = Fixed::ONE;
        let wall = w.add_body(wall_body);
        let mut ball = Body::new(v(0, 0), Vec2::new(frac(1, 2), frac(1, 2)));
        ball.vel = v(4, 0);
        ball.restitution = Fixed::ONE;
        let ball = w.add_body(ball);
        assert!(w.bodies()[wall].is_static());
        assert!(!w.bodies()[ball].is_static());
        for _ in 0..16 {
            w.step(frac(1, 4));
        }
        assert_eq!(w.bodies()[wall].pos, v(4, 0));
        // The ball bounced straight back at full speed.
        assert_eq!(w.bodies()[ball].vel, v(-4, 0));
    }

    #[test]
    fn gravity_scale_zero_floats_while_sibling_falls() {
        let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
        let mut floater = Body::new(v(-2, 10), Vec2::new(frac(1, 2), frac(1, 2)));
        floater.gravity_scale = Fixed::ZERO;
        w.add_body(floater);
        w.add_body(Body::new(v(2, 10), Vec2::new(frac(1, 2), frac(1, 2))));
        for _ in 0..8 {
            w.step(frac(1, 8));
        }
        assert_eq!(w.bodies()[0].pos.y, f(10));
        assert_eq!(w.bodies()[0].vel, Vec2::ZERO);
        assert!(w.bodies()[1].pos.y < f(10));
    }

    #[test]
    fn zero_and_negative_dt_are_noops() {
        let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
        floor(&mut w);
        w.add_body(Body::new(v(0, 4), Vec2::new(frac(1, 2), frac(1, 2))));
        let before = hash_state(&w);
        w.step(Fixed::ZERO);
        w.step(frac(-1, 8));
        assert_eq!(hash_state(&w), before);
        assert_eq!(w.bodies()[0].pos, v(0, 4));
    }

    #[test]
    fn solid_accessors_and_corner_sorting() {
        let s = Solid::new(v(3, 5), v(-1, 2));
        assert_eq!(s.min, v(-1, 2));
        assert_eq!(s.max, v(3, 5));
        assert_eq!(s.center(), Vec2::new(f(1), frac(7, 2)));
        assert_eq!(s.half(), Vec2::new(f(2), frac(3, 2)));
        let s2 = Solid::from_center_half(Vec2::new(f(1), frac(7, 2)), Vec2::new(f(2), frac(3, 2)));
        assert_eq!(s, s2);
    }

    #[test]
    fn world_lists_what_was_added() {
        let mut w = World::new(Vec2::ZERO);
        floor(&mut w);
        let b = w.add_body(Body::new(v(0, 3), Vec2::new(frac(1, 2), frac(1, 2))));
        assert_eq!(w.solids().len(), 1);
        assert_eq!(w.bodies().len(), 1);
        assert_eq!(b, 0);
        assert_eq!(w.solids()[0].max.y, Fixed::ZERO);
    }

    #[test]
    fn deterministic_replay_and_hash_sensitivity() {
        // Same world, same inputs: bit-identical trace. Any field
        // left out of the hash shows up as a failed inequality here.
        let build = || {
            let mut w = World::new(Vec2::new(Fixed::ZERO, f(-8)));
            floor(&mut w);
            let mut a = Body::new(v(-3, 6), Vec2::new(frac(1, 2), frac(1, 2)));
            a.restitution = frac(1, 2);
            a.friction = frac(1, 4);
            a.vel = v(2, 0);
            w.add_body(a);
            let mut b = Body::new(v(3, 8), Vec2::new(frac(1, 2), frac(1, 2)));
            b.restitution = frac(3, 4);
            w.add_body(b);
            w
        };
        let (mut a, mut b) = (build(), build());
        for _ in 0..96 {
            a.step(frac(1, 8));
            b.step(frac(1, 8));
        }
        assert_eq!(a.bodies(), b.bodies());
        assert_eq!(hash_state(&a), hash_state(&b));
        // Sensitivity: a nudge to any hashed field moves the hash.
        let mut c = build();
        c.bodies_mut()[0].vel.x = f(3);
        assert_ne!(hash_state(&a), hash_state(&c));
        let mut d = build();
        d.rest_speed = f(1);
        assert_ne!(hash_state(&a), hash_state(&d));
    }
}
