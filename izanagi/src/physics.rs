//! Gravity and rigid-body-lite physics.
//!
//! The engine already had collision *detection* ([`crate::collide`]) but
//! nothing that moved: `examples/platformer.rs` hand-rolls gravity, Euler
//! integration and swept-AABB resolution in the example itself. This module
//! is that pattern, generalised and tested once — a [`Physics`] world owns
//! a list of [`Body`] values (AABBs with velocity and mass), applies
//! gravity, integrates with semi-implicit Euler, and resolves overlaps
//! against each other.
//!
//! ```rust
//! use izanagi::physics::{Body, Physics};
//! use izanagi::Rect;
//!
//! let mut physics = Physics::new(); // gravity (0, 980): +y is down, as in the examples
//! let ball = physics.add(Body::new(Rect::new(0.0, 0.0, 10.0, 10.0)));
//! physics.add(Body::fixed(Rect::new(-100.0, 100.0, 300.0, 20.0))); // floor
//!
//! for _ in 0..60 {
//!     physics.step(1.0 / 60.0);
//! }
//! let body = physics.body(ball);
//! assert!(body.is_some());
//! if let Some(body) = body {
//!     assert!(body.grounded);
//!     assert!((body.rect.y - 90.0).abs() < 1e-3);
//! }
//! ```
//!
//! ## Why a standalone world, not ECS components
//!
//! Collision resolution needs two bodies mutably at the same time, and
//! [`crate::World::get_mut`] lends one component at a time — an
//! ECS-resident solver would have to copy every body out, step the copies
//! and write them back, every step. `Physics` owning a `Vec<Body>` in
//! insertion order makes pair resolution index-ordered (so the outcome
//! depends only on the inputs, never on hash iteration order) and
//! copy-free. [`Body`] is still a plain data struct: a game that keeps its
//! render state in the ECS can mirror `body.rect` into a component after
//! each step, the same way `platformer.rs` keeps its `Player` struct
//! outside the ECS. This is also why `Physics` is not a field on
//! [`crate::Engine`]: the subsystem types a game constructs only when it
//! needs them live in their own modules (see `state`, `tilemap`).
//!
//! ## Fixed timestep
//!
//! [`Physics::step`] takes whatever `dt` it is given, but resting contact
//! is only stable when `dt` is: a body at rest re-penetrates by
//! `gravity * dt * dt` every step and is pushed back out, so a varying
//! `dt` makes the resting depth — and the speed below which a bounce is
//! treated as rest — vary with it. Drive it from [`crate::Time`]'s
//! fixed-step accumulator, exactly as `time`'s own docs show:
//!
//! ```rust
//! use izanagi::physics::Physics;
//! use izanagi::Time;
//!
//! let mut time = Time::new();
//! let mut physics = Physics::new();
//! time.advance(0.05);
//! while time.fixed_step() {
//!     physics.step(time.fixed_dt());
//! }
//! ```
//!
//! ## Limits, deliberately
//!
//! - **AABBs only, no rotation.** Circles and rotated boxes belong in a
//!   bigger engine; [`crate::collide`] has the circle overlap tests.
//! - **Discrete resolution.** Overlaps are found after the move, so a body
//!   that travels further than its own size in one step can tunnel
//!   through a thin wall. Use a smaller fixed `dt` for fast games, and
//!   [`crate::collide::swept_aabb`] for projectiles.
//! - **Static bodies are teleported, not swept.** A `fixed` body never
//!   integrates; move its `rect` between steps (a moving platform) and
//!   the next step pushes dynamic bodies out of the way.
//! - **Floats.** This module is on the presentation side of the crate's
//!   determinism boundary (see the crate docs): same inputs give the same
//!   outputs on one machine, but `f32` rounding is not bit-stable across
//!   machines. For a simulation that must replay bit-identically, use
//!   `izanagi_kit`.

use crate::math::{Rect, Vec2};

/// Relaxation passes over the contact pairs per [`Physics::step`].
/// Four is the point where a resting stack's residual penetration
/// (which halves roughly per pass) drops below a hundredth of the
/// per-step sink; more passes buy little and cost linearly.
const SOLVER_PASSES: usize = 4;

/// A rigid body, lite: one AABB with a velocity, a mass and a material.
///
/// Bodies are values owned by a [`Physics`] world; the `usize` returned by
/// [`Physics::add`] is the body's id for [`Physics::body`] /
/// [`Physics::body_mut`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Body {
    /// World-space axis-aligned bounding box.
    pub rect: Rect,
    /// Velocity in units per second.
    pub vel: Vec2,
    /// Inverse mass (`1.0 / mass`). Zero — the value [`Body::fixed`]
    /// uses — makes the body immovable: gravity and impulses from other
    /// bodies have no effect on it, and it is never integrated. Setting
    /// this directly is how a body changes weight mid-game; values at or
    /// below zero are treated as static everywhere.
    pub inv_mass: f32,
    /// Per-body gravity multiplier. `1.0` is full world gravity, `0.0`
    /// floats, `2.0` falls twice as fast, negative values fall up.
    pub gravity_scale: f32,
    /// Bounciness in `[0, 1]`: the fraction of the approaching speed
    /// along a contact normal that is reflected. `0.0` (the default)
    /// lands dead; `1.0` is a perfect bounce. When two bodies meet, the
    /// larger of the two values is used. Impacts slower than one step of
    /// gravity are treated as resting contact and never bounce, whatever
    /// this says — that is what lets a bouncy body come to rest.
    pub restitution: f32,
    /// Friction in `[0, 1]`: the fraction of the tangential relative
    /// velocity removed by a contact, per step. `0.0` (the default) is
    /// ice; `1.0` stops sliding dead in one step. When two bodies meet,
    /// the mean of the two values is used.
    pub friction: f32,
    /// Output of the last [`Physics::step`]: true when another body was
    /// directly below this one in a resolved contact — the platformer
    /// question "may I jump now?". Meaningful for dynamic bodies; reset
    /// at the start of every step.
    pub grounded: bool,
}

impl Body {
    /// A dynamic body of mass 1 at rest. The starting point: set `vel`,
    /// `restitution` or `friction` on the result as needed.
    pub fn new(rect: Rect) -> Self {
        Self::dynamic(rect, 1.0)
    }

    /// A dynamic body with the given mass. A mass at or below zero (or
    /// not finite) cannot be inverted, so it produces a static body —
    /// the same contract as [`Body::fixed`], reached by arithmetic
    /// instead of by asking.
    pub fn dynamic(rect: Rect, mass: f32) -> Self {
        let inv_mass = if mass > 0.0 && mass.is_finite() {
            1.0 / mass
        } else {
            0.0
        };
        Self {
            rect,
            vel: Vec2::ZERO,
            inv_mass,
            gravity_scale: 1.0,
            restitution: 0.0,
            friction: 0.0,
            grounded: false,
        }
    }

    /// A static body: a floor, a wall, a platform. Never moves, never
    /// falls, and pushes dynamic bodies out of itself with its full
    /// weight behind it. Move it by writing a new `rect` between steps.
    pub fn fixed(rect: Rect) -> Self {
        Self {
            rect,
            vel: Vec2::ZERO,
            inv_mass: 0.0,
            gravity_scale: 0.0,
            restitution: 0.0,
            friction: 0.0,
            grounded: false,
        }
    }

    /// True when this body cannot be moved by gravity or by contact.
    pub fn is_static(&self) -> bool {
        self.inv_mass <= 0.0
    }

    /// The body's mass: `1.0 / inv_mass`, or positive infinity for a
    /// static body.
    pub fn mass(&self) -> f32 {
        if self.inv_mass <= 0.0 {
            f32::INFINITY
        } else {
            1.0 / self.inv_mass
        }
    }
}

/// A tiny rigid-body world: a gravity vector and the bodies under it.
///
/// Construct with [`Physics::new`] (gravity `(0, 980)` — screen space,
/// +y down, the convention every engine example uses), add bodies, and
/// call [`Physics::step`] once per fixed timestep.
#[derive(Clone, Debug, PartialEq)]
pub struct Physics {
    /// World gravity in units per second squared, applied to every
    /// dynamic body scaled by its [`Body::gravity_scale`].
    pub gravity: Vec2,
    bodies: Vec<Body>,
}

impl Physics {
    /// A world with gravity `(0, 980)`: downward in screen space.
    pub fn new() -> Self {
        Self::with_gravity(Vec2::new(0.0, 980.0))
    }

    /// A world with a custom gravity vector — `(0, 0)` for a top-down
    /// game, `(0, -980)` for a y-up world.
    pub fn with_gravity(gravity: Vec2) -> Self {
        Self {
            gravity,
            bodies: Vec::new(),
        }
    }

    /// Add a body and return its id. Ids are insertion order and are
    /// never reused or reordered, which is what makes the step's pair
    /// order deterministic.
    pub fn add(&mut self, body: Body) -> usize {
        let id = self.bodies.len();
        self.bodies.push(body);
        id
    }

    /// Borrow the body with this id, if it exists.
    pub fn body(&self, id: usize) -> Option<&Body> {
        self.bodies.get(id)
    }

    /// Mutably borrow the body with this id, if it exists — teleport a
    /// platform, apply a jump impulse, change a weight.
    pub fn body_mut(&mut self, id: usize) -> Option<&mut Body> {
        self.bodies.get_mut(id)
    }

    /// All bodies, in id order. The render loop reads positions here.
    pub fn bodies(&self) -> &[Body] {
        &self.bodies
    }

    /// Number of bodies in the world.
    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    /// True when the world has no bodies.
    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    /// Advance the world by `dt` seconds.
    ///
    /// Semi-implicit Euler: gravity is added to each dynamic body's
    /// velocity, positions move by the new velocity, then every
    /// overlapping pair is pushed apart and given its velocity response
    /// (bounce, then friction), in body-id order. The pair pass runs
    /// `SOLVER_PASSES` times per step: a single pass leaves a stack
    /// ratcheting, because the pair resolved last re-loads a body the
    /// pair resolved first had already settled — the extra passes are
    /// what let a pile of boxes come to rest. A `dt` that is zero,
    /// negative or not finite is a no-op, so a paused game can call
    /// this unconditionally.
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 || dt.is_nan() || dt.is_infinite() {
            return;
        }
        let gravity = self.gravity;
        for body in &mut self.bodies {
            body.grounded = false;
            if body.is_static() {
                continue;
            }
            body.vel += gravity * (body.gravity_scale * dt);
            body.rect.x += body.vel.x * dt;
            body.rect.y += body.vel.y * dt;
        }
        // An impact at or below the speed gravity adds in one step is a
        // resting contact, not a bounce — see `Body::restitution`.
        let rest_speed = gravity.len() * dt;
        for _ in 0..SOLVER_PASSES {
            let mut rest = &mut self.bodies[..];
            while let Some((first, tail)) = rest.split_first_mut() {
                for other in tail.iter_mut() {
                    resolve_pair(first, other, rest_speed);
                }
                rest = tail;
            }
        }
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

/// Push one overlapping pair apart and apply its velocity response.
///
/// `normal` runs from `a` to `b` along the axis of least penetration —
/// resolving along the shallow axis is what makes a body land *on* a
/// floor instead of being ejected sideways through it. Positional
/// correction is split in proportion to inverse mass, so a static body
/// (share zero) never moves and two equal bodies each give way by half.
fn resolve_pair(a: &mut Body, b: &mut Body, rest_speed: f32) {
    let total_inv = a.inv_mass + b.inv_mass;
    if total_inv <= 0.0 {
        return; // Two static bodies: neither can yield, nothing to do.
    }
    let overlap_x = (a.rect.x + a.rect.w).min(b.rect.x + b.rect.w) - a.rect.x.max(b.rect.x);
    let overlap_y = (a.rect.y + a.rect.h).min(b.rect.y + b.rect.h) - a.rect.y.max(b.rect.y);
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return;
    }
    let (normal, penetration, vertical) = if overlap_x < overlap_y {
        let sign = if b.rect.center().x >= a.rect.center().x {
            1.0
        } else {
            -1.0
        };
        (Vec2::new(sign, 0.0), overlap_x, false)
    } else {
        let sign = if b.rect.center().y >= a.rect.center().y {
            1.0
        } else {
            -1.0
        };
        (Vec2::new(0.0, sign), overlap_y, true)
    };

    let a_share = a.inv_mass / total_inv;
    let b_share = b.inv_mass / total_inv;
    a.rect.x -= normal.x * penetration * a_share;
    a.rect.y -= normal.y * penetration * a_share;
    b.rect.x += normal.x * penetration * b_share;
    b.rect.y += normal.y * penetration * b_share;

    if vertical {
        // +y is down, so the body with the smaller centre y is the one
        // standing on the other.
        if a.rect.center().y <= b.rect.center().y {
            a.grounded = true;
        } else {
            b.grounded = true;
        }
    }

    let relative = b.vel - a.vel;
    let vn = relative.dot(normal);
    if vn >= 0.0 {
        return; // Overlapping but already separating: position only.
    }
    let restitution = if -vn <= rest_speed {
        0.0
    } else {
        a.restitution.max(b.restitution).clamp(0.0, 1.0)
    };
    let impulse = -(1.0 + restitution) * vn / total_inv;
    a.vel -= normal * (impulse * a.inv_mass);
    b.vel += normal * (impulse * b.inv_mass);

    let friction = ((a.friction + b.friction) * 0.5).clamp(0.0, 1.0);
    if friction > 0.0 {
        let tangent = Vec2::new(-normal.y, normal.x);
        let vt = (b.vel - a.vel).dot(tangent);
        let drag = -vt * friction / total_inv;
        a.vel -= tangent * (drag * a.inv_mass);
        b.vel += tangent * (drag * b.inv_mass);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Time;

    const DT: f32 = 1.0 / 60.0;

    /// A world with one static floor whose top surface is at `floor_top`.
    fn world_with_floor(floor_top: f32) -> Physics {
        let mut physics = Physics::new();
        physics.add(Body::fixed(Rect::new(-500.0, floor_top, 1000.0, 50.0)));
        physics
    }

    #[test]
    fn free_fall_matches_the_closed_form_of_its_own_integrator() {
        // Semi-implicit Euler has an exact discrete solution: after n
        // steps from rest, v = g*n*dt and y = g*dt^2 * n(n+1)/2. That
        // sum — not a hand-picked constant — is the oracle; the analytic
        // 1/2 g t^2 is then checked with the first-order error the
        // integrator is known to carry (half a step of travel).
        let g = 1000.0;
        let dt = 1.0 / 240.0;
        let n = 240;
        let mut physics = Physics::with_gravity(Vec2::new(0.0, g));
        let ball = physics.add(Body::new(Rect::new(0.0, 0.0, 10.0, 10.0)));
        for _ in 0..n {
            physics.step(dt);
        }
        let body = physics.body(ball).unwrap();
        let expected_v = g * n as f32 * dt;
        let expected_y = g * dt * dt * (n as f32) * (n as f32 + 1.0) / 2.0;
        assert!(
            (body.vel.y - expected_v).abs() < 0.1,
            "v after 1s: {} vs closed form {expected_v}",
            body.vel.y
        );
        assert!(
            (body.rect.y - expected_y).abs() < 0.1,
            "y after 1s: {} vs closed form {expected_y}",
            body.rect.y
        );
        let analytic = 0.5 * g; // 1/2 g t^2 at t = 1
        assert!(
            (body.rect.y - analytic).abs() < 3.0,
            "y after 1s: {} vs analytic {analytic} (first-order error budget)",
            body.rect.y
        );
        assert_eq!(body.vel.x, 0.0);
        assert_eq!(body.rect.x, 0.0);
    }

    #[test]
    fn bounce_height_is_the_drop_height_times_restitution_squared() {
        // Energy bookkeeping with an oracle: a bounce at restitution e
        // keeps e of the impact speed, so it regains e^2 of the drop
        // height — and it can never regain more than the drop. If the
        // impulse ever *added* energy, the second assertion is the one
        // that fails.
        let floor_top = 200.0;
        let rest_y = floor_top - 10.0;
        let drop = 100.0;
        let mut physics = world_with_floor(floor_top);
        let mut body = Body::new(Rect::new(0.0, rest_y - drop, 10.0, 10.0));
        body.restitution = 0.8;
        let ball = physics.add(body);
        let floor = physics.body(0).unwrap();
        assert_eq!(floor.rect.y, floor_top);

        let mut bounced = false;
        let mut apex = rest_y; // smallest y seen after the bounce = highest point
        for _ in 0..600 {
            physics.step(DT);
            let b = physics.body(ball).unwrap();
            assert!(b.rect.y.is_finite() && b.vel.y.is_finite());
            if b.vel.y < 0.0 {
                bounced = true;
            }
            if bounced {
                apex = apex.min(b.rect.y);
            }
        }
        assert!(bounced, "the ball never bounced");
        let regained = rest_y - apex;
        assert!(
            regained <= drop + 1e-3,
            "bounce regained {regained} of a {drop} drop — energy was created"
        );
        assert!(
            (45.0..=80.0).contains(&regained),
            "expected ≈ e^2 * drop = 64 regained, got {regained}"
        );
    }

    #[test]
    fn resting_contact_settles_flush_and_stays_put() {
        // The jitter test: a resting body re-penetrates by g*dt^2 each
        // step and is pushed back out. If either the positional
        // correction or the resting-contact velocity rule is off, the
        // settled position walks or buzzes instead of repeating exactly.
        let floor_top = 200.0;
        let mut physics = world_with_floor(floor_top);
        let mut body = Body::new(Rect::new(0.0, floor_top - 30.0, 10.0, 10.0));
        body.restitution = 0.5; // bouncy, but must still come to rest
        let ball = physics.add(body);
        for _ in 0..300 {
            physics.step(DT);
        }
        let settled = physics.body(ball).unwrap();
        assert!(settled.grounded);
        assert!(
            (settled.rect.y + settled.rect.h - floor_top).abs() < 1e-3,
            "bottom {} is not flush with floor top {floor_top}",
            settled.rect.y + settled.rect.h
        );
        assert!(settled.vel.y.abs() < 1e-6, "resting vel.y = {}", settled.vel.y);
        let frozen = settled.rect;
        for _ in 0..120 {
            physics.step(DT);
            let b = physics.body(ball).unwrap();
            assert!(b.grounded, "grounded flickered off while resting");
            assert!(
                (b.rect.y - frozen.y).abs() < 1e-4,
                "resting body jittered: {} vs {}",
                b.rect.y,
                frozen.y
            );
        }
    }

    #[test]
    fn static_bodies_never_move_and_report_infinite_mass() {
        let mut physics = Physics::new();
        let fixed = physics.add(Body::fixed(Rect::new(5.0, 5.0, 10.0, 10.0)));
        let zero = physics.add(Body::dynamic(Rect::new(50.0, 5.0, 10.0, 10.0), 0.0));
        let negative = physics.add(Body::dynamic(Rect::new(90.0, 5.0, 10.0, 10.0), -3.0));
        for id in [fixed, zero, negative] {
            let b = physics.body(id).unwrap();
            assert!(b.is_static(), "body {id} should be static");
            assert!(b.mass().is_infinite(), "static mass must be infinite");
        }
        for _ in 0..120 {
            physics.step(DT);
        }
        assert_eq!(physics.body(fixed).unwrap().rect, Rect::new(5.0, 5.0, 10.0, 10.0));
        assert_eq!(physics.body(zero).unwrap().rect.y, 5.0);
        assert_eq!(physics.body(negative).unwrap().rect.y, 5.0);

        let m = Body::dynamic(Rect::new(0.0, 0.0, 1.0, 1.0), 2.0);
        assert!(!m.is_static());
        assert_eq!(m.mass(), 2.0);
        assert_eq!(Body::new(Rect::new(0.0, 0.0, 1.0, 1.0)).mass(), 1.0);
    }

    #[test]
    fn zero_gravity_preserves_velocity_exactly() {
        let mut physics = Physics::with_gravity(Vec2::ZERO);
        let mut body = Body::new(Rect::new(10.0, 20.0, 4.0, 4.0));
        body.vel = Vec2::new(30.0, -40.0);
        let id = physics.add(body);
        for _ in 0..100 {
            physics.step(DT);
        }
        let b = physics.body(id).unwrap();
        assert_eq!(b.vel, Vec2::new(30.0, -40.0));
        let expected = Vec2::new(10.0 + 30.0 * 100.0 * DT, 20.0 - 40.0 * 100.0 * DT);
        assert!(
            (b.rect.x - expected.x).abs() < 1e-3 && (b.rect.y - expected.y).abs() < 1e-3,
            "drifted to ({}, {}), expected ({}, {})",
            b.rect.x,
            b.rect.y,
            expected.x,
            expected.y
        );
    }

    #[test]
    fn non_positive_or_non_finite_dt_is_a_no_op() {
        let mut physics = Physics::new();
        let mut body = Body::new(Rect::new(1.0, 2.0, 10.0, 10.0));
        body.vel = Vec2::new(5.0, 6.0);
        let id = physics.add(body);
        let before = *physics.body(id).unwrap();
        for dt in [0.0, -DT, f32::NAN, f32::INFINITY] {
            physics.step(dt);
            assert_eq!(*physics.body(id).unwrap(), before, "step({dt}) changed the world");
        }
    }

    #[test]
    fn identical_worlds_step_identically() {
        fn build() -> Physics {
            let mut physics = world_with_floor(300.0);
            for i in 0..5 {
                let mut b = Body::dynamic(
                    Rect::new(i as f32 * 30.0 - 60.0, 200.0 - i as f32 * 25.0, 18.0, 18.0),
                    1.0 + i as f32 * 0.5,
                );
                b.restitution = 0.3;
                b.friction = 0.2;
                b.vel = Vec2::new(i as f32 * 7.0 - 14.0, 0.0);
                physics.add(b);
            }
            physics
        }
        let mut a = build();
        let mut b = build();
        for _ in 0..600 {
            a.step(DT);
            b.step(DT);
        }
        assert_eq!(a.bodies(), b.bodies(), "same inputs diverged");
        // And the run must have done something — a vacuous equality of
        // untouched worlds proves nothing.
        assert!(a.bodies().iter().any(|body| body.grounded));
    }

    #[test]
    fn elastic_equal_masses_exchange_velocities() {
        // Conservation laws as the oracle: equal masses, head-on, e = 1
        // swap velocities; e = 0 leave together at the mean (zero here).
        for (restitution, expected) in [(1.0, (-10.0, 10.0)), (0.0, (0.0, 0.0))] {
            let mut physics = Physics::with_gravity(Vec2::ZERO);
            let mut left = Body::new(Rect::new(0.0, 0.0, 10.0, 10.0));
            left.vel = Vec2::new(10.0, 0.0);
            left.restitution = restitution;
            let mut right = Body::new(Rect::new(100.0, 0.0, 10.0, 10.0));
            right.vel = Vec2::new(-10.0, 0.0);
            right.restitution = restitution;
            let l = physics.add(left);
            let r = physics.add(right);
            for _ in 0..600 {
                physics.step(DT);
            }
            let (lv, rv) = (physics.body(l).unwrap().vel.x, physics.body(r).unwrap().vel.x);
            assert!(
                (lv - expected.0).abs() < 1e-3 && (rv - expected.1).abs() < 1e-3,
                "e={restitution}: velocities ({lv}, {rv}), expected {expected:?}"
            );
        }
    }

    #[test]
    fn wall_reflects_horizontal_velocity() {
        let mut physics = Physics::with_gravity(Vec2::ZERO);
        physics.add(Body::fixed(Rect::new(100.0, -50.0, 20.0, 200.0)));
        let mut body = Body::new(Rect::new(0.0, 0.0, 10.0, 10.0));
        body.vel = Vec2::new(120.0, 15.0);
        body.restitution = 1.0;
        let id = physics.add(body);
        for _ in 0..120 {
            physics.step(DT);
        }
        let b = physics.body(id).unwrap();
        assert!((b.vel.x + 120.0).abs() < 1e-3, "vel.x = {} (expected -120)", b.vel.x);
        assert_eq!(b.vel.y, 15.0, "a wall bounce must not touch vel.y");
        assert!(
            b.rect.x + b.rect.w <= 100.0 + 1e-3,
            "body ended inside the wall: right edge {}",
            b.rect.x + b.rect.w
        );
    }

    #[test]
    fn friction_removes_a_fixed_fraction_of_the_slide_per_step() {
        // The documented contract, checked as arithmetic: in resting
        // contact with combined friction mu, the slide keeps (1 - mu) of
        // itself each step — geometric decay, not a stop and not a
        // constant drag.
        let floor_top = 100.0;
        let mut physics = world_with_floor(floor_top);
        let floor_id = 0;
        physics.body_mut(floor_id).unwrap().friction = 0.25;
        let mut body = Body::new(Rect::new(0.0, floor_top - 10.0, 10.0, 10.0));
        body.vel = Vec2::new(100.0, 0.0);
        body.friction = 0.25;
        let id = physics.add(body);
        for _ in 0..10 {
            physics.step(DT);
        }
        let expected = 100.0 * 0.75f32.powi(10);
        let vx = physics.body(id).unwrap().vel.x;
        assert!(
            (vx - expected).abs() < expected * 0.02 + 1e-4,
            "vel.x after 10 steps: {vx}, expected geometric decay {expected}"
        );

        // mu = 1 stops the slide in a single contact step.
        let mut physics = world_with_floor(floor_top);
        physics.body_mut(0).unwrap().friction = 1.0;
        let mut body = Body::new(Rect::new(0.0, floor_top - 10.0, 10.0, 10.0));
        body.vel = Vec2::new(100.0, 0.0);
        body.friction = 1.0;
        let id = physics.add(body);
        physics.step(DT);
        assert!(
            physics.body(id).unwrap().vel.x.abs() < 1e-4,
            "mu=1 must stop the slide in one step"
        );
    }

    #[test]
    fn gravity_scale_zero_floats_while_full_scale_falls() {
        let mut physics = Physics::new();
        let floater = physics.add(Body::new(Rect::new(0.0, 0.0, 10.0, 10.0)));
        physics.body_mut(floater).unwrap().gravity_scale = 0.0;
        let faller = physics.add(Body::new(Rect::new(50.0, 0.0, 10.0, 10.0)));
        for _ in 0..60 {
            physics.step(DT);
        }
        assert_eq!(physics.body(floater).unwrap().rect.y, 0.0);
        assert!(physics.body(faller).unwrap().rect.y > 100.0);
    }

    #[test]
    fn accessors_report_and_mutate_the_world() {
        let mut physics = Physics::new();
        assert!(physics.is_empty());
        assert_eq!(physics.len(), 0);
        assert!(physics.body(0).is_none());
        assert!(physics.body_mut(0).is_none());

        let a = physics.add(Body::new(Rect::new(0.0, 0.0, 10.0, 10.0)));
        let b = physics.add(Body::fixed(Rect::new(0.0, 50.0, 100.0, 10.0)));
        assert_eq!((a, b), (0, 1), "ids are insertion order");
        assert_eq!(physics.len(), 2);
        assert!(!physics.is_empty());
        assert_eq!(physics.bodies().len(), 2);
        assert!(physics.body(99).is_none());

        // body_mut is the game's handle between steps: teleport the
        // dynamic body above the floor and give it a push.
        let m = physics.body_mut(a).unwrap();
        m.rect.y = 20.0;
        m.vel = Vec2::new(0.0, 30.0);
        for _ in 0..120 {
            physics.step(DT);
        }
        assert!(physics.body(a).unwrap().grounded);
    }

    #[test]
    fn a_stack_of_two_boxes_settles() {
        // Two dynamic boxes and a floor: the case sequential resolution
        // handles least perfectly (with one pass instead of
        // SOLVER_PASSES this stack ratcheted and ended 0.54 sunk into
        // the floor — the failure that put the passes there). The
        // tolerances here are the honest ones — flush to within a
        // fraction of a unit and stable, not bit-exact. The single-box
        // case above pins the exact behaviour where exactness is
        // achievable.
        let floor_top = 200.0;
        let mut physics = world_with_floor(floor_top);
        physics.add(Body::new(Rect::new(0.0, floor_top - 45.0, 20.0, 20.0)));
        physics.add(Body::new(Rect::new(2.0, floor_top - 80.0, 20.0, 20.0)));
        for _ in 0..300 {
            physics.step(DT);
        }
        let lower = *physics.body(1).unwrap();
        let upper = *physics.body(2).unwrap();
        assert!(lower.grounded && upper.grounded);
        assert!(
            (lower.rect.y + lower.rect.h - floor_top).abs() < 0.25,
            "lower box floats/sinks by {}",
            lower.rect.y + lower.rect.h - floor_top
        );
        assert!(
            (upper.rect.y + upper.rect.h - lower.rect.y).abs() < 0.25,
            "upper box floats/sinks by {}",
            upper.rect.y + upper.rect.h - lower.rect.y
        );
        let (ly, uy) = (lower.rect.y, upper.rect.y);
        for _ in 0..60 {
            physics.step(DT);
        }
        assert!((physics.body(1).unwrap().rect.y - ly).abs() < 0.05, "lower box still moving");
        assert!((physics.body(2).unwrap().rect.y - uy).abs() < 0.05, "upper box still moving");
    }

    #[test]
    fn fixed_timestep_accumulator_drives_the_step() {
        // The composition the module docs prescribe: `Time` decides how
        // many fixed steps a frame holds, `Physics` takes exactly those.
        let mut time = Time::new();
        time.set_fixed_dt(DT);
        let mut physics = Physics::new();
        let ball = physics.add(Body::new(Rect::new(0.0, 0.0, 10.0, 10.0)));
        // 0.051s, not 0.05: in f32, 0.05 - 2/60 lands a hair below the
        // third step's threshold, so exactly-3-steps needs the extra
        // millisecond of slack. The remainder after 3 steps is ~0.001.
        time.advance(0.051);
        let mut steps = 0;
        while time.fixed_step() {
            physics.step(time.fixed_dt());
            steps += 1;
        }
        assert_eq!(steps, 3);
        // Closed form for 3 semi-implicit steps from rest.
        let g = 980.0;
        let expected = g * DT * DT * 3.0 * 4.0 / 2.0;
        let y = physics.body(ball).unwrap().rect.y;
        assert!((y - expected).abs() < 0.05, "y = {y}, expected {expected}");
    }
}
