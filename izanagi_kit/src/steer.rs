//! Steering behaviors — Reynolds' locomotion layer for game AI ("Steering
//! Behaviors for Autonomous Characters", GDC 1999): `seek`, `flee`, `arrive`,
//! `pursue`, `evade`, `wander`, and the full boid triple
//! `separation`/`alignment`/`cohesion` over the deterministic
//! [`Vec2`]/[`Fixed`] pair. Where `behavior`/`goap` decide *what* an agent
//! wants, this module decides *which force to apply this tick*.
//!
//! Every behavior returns a **steering force**: `desired velocity − current
//! velocity`, truncated to `max_force`. The caller integrates the force into
//! velocity itself (`vel + force·dt`), so several behaviors can be combined
//! before clamping — [`combine`] does that in one step with per-behavior
//! weights. All math is `Fixed`: no floats, no table lookups beyond
//! `Vec2::normalize`, bit-identical on every target.
//!
//! ```
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::steer::seek;
//! use izanagi_kit::vec::Vec2;
//!
//! let pos = Vec2::new(Fixed::ZERO, Fixed::ZERO);
//! let tgt = Vec2::new(Fixed::from_int(10), Fixed::ZERO);
//! let f = seek(pos, Vec2::ZERO, tgt, Fixed::from_int(4), Fixed::ONE);
//! assert!(f.x > Fixed::ZERO);
//! ```

use crate::fixed::Fixed;
use crate::rng::SplitMix64;
use crate::vec::Vec2;

/// Clamp a vector's magnitude to `max` — the shared tail of every behavior
/// and the tool for capping a combined force. A zero vector stays zero.
pub fn truncate(v: Vec2, max: Fixed) -> Vec2 {
    if v.is_zero() {
        return v;
    }
    let len_sq = v.len_sq();
    let max_sq = max.mul(max);
    if len_sq.raw() <= max_sq.raw() {
        return v;
    }
    match v.normalize() {
        Some(u) => u.scale(max),
        None => v,
    }
}

/// The core of every drive-toward behavior: the force that turns `vel` into
/// `desired` without exceeding `max_force`.
fn steer_to(vel: Vec2, desired: Vec2, max_force: Fixed) -> Vec2 {
    truncate(desired - vel, max_force)
}

/// A unit vector toward `target`, or `None` when already on top of it.
fn direction_to(pos: Vec2, target: Vec2) -> Option<Vec2> {
    (target - pos).normalize()
}

/// Accelerate toward `target` at `max_speed`. The canonical seek:
/// `desired = (target − pos)·max_speed/|…|`, `force = desired − vel`.
pub fn seek(pos: Vec2, vel: Vec2, target: Vec2, max_speed: Fixed, max_force: Fixed) -> Vec2 {
    match direction_to(pos, target) {
        Some(d) => steer_to(vel, d.scale(max_speed), max_force),
        None => Vec2::ZERO,
    }
}

/// Accelerate directly away from `threat` — seek with the target mirrored.
pub fn flee(pos: Vec2, vel: Vec2, threat: Vec2, max_speed: Fixed, max_force: Fixed) -> Vec2 {
    match direction_to(threat, pos) {
        Some(d) => steer_to(vel, d.scale(max_speed), max_force),
        None => Vec2::ZERO,
    }
}

/// Seek that decelerates inside `slow_radius`: desired speed scales
/// linearly `max_speed·dist/slow_radius` down to a full stop at the target,
/// so the agent lands instead of orbiting it. A non-positive `slow_radius`
/// degenerates to plain [`seek`].
pub fn arrive(
    pos: Vec2,
    vel: Vec2,
    target: Vec2,
    max_speed: Fixed,
    max_force: Fixed,
    slow_radius: Fixed,
) -> Vec2 {
    let dist = pos.distance(target);
    if dist.is_zero() {
        return steer_to(vel, Vec2::ZERO, max_force);
    }
    let speed = if slow_radius.raw() > 0 && dist.raw() < slow_radius.raw() {
        max_speed.mul(dist.div(slow_radius))
    } else {
        max_speed
    };
    match direction_to(pos, target) {
        Some(d) => steer_to(vel, d.scale(speed), max_force),
        None => Vec2::ZERO,
    }
}

/// Chase a moving target by aiming at where it will be: the classic
/// constant-velocity prediction `target + target_vel·(dist/max_speed)`,
/// which converges on straight-line targets and degrades gracefully on
/// turning ones. Zero `max_speed` degenerates to seeking the present point.
pub fn pursue(
    pos: Vec2,
    vel: Vec2,
    target: Vec2,
    target_vel: Vec2,
    max_speed: Fixed,
    max_force: Fixed,
) -> Vec2 {
    let lead = if max_speed.raw() > 0 {
        pos.distance(target).div(max_speed)
    } else {
        Fixed::ZERO
    };
    seek(
        pos,
        vel,
        target + target_vel.scale(lead),
        max_speed,
        max_force,
    )
}

/// Flee from where a moving threat will be — [`pursue`] mirrored, for
/// dodging hunters rather than their current position.
pub fn evade(
    pos: Vec2,
    vel: Vec2,
    threat: Vec2,
    threat_vel: Vec2,
    max_speed: Fixed,
    max_force: Fixed,
) -> Vec2 {
    let lead = if max_speed.raw() > 0 {
        pos.distance(threat).div(max_speed)
    } else {
        Fixed::ZERO
    };
    flee(
        pos,
        vel,
        threat + threat_vel.scale(lead),
        max_speed,
        max_force,
    )
}

/// Wander: Reynolds' "small random nudges to the heading" in its cheapest
/// deterministic form — a uniformly random force inside the
/// `[-jitter, jitter]` square, truncated to `jitter`. The caller feeds a
/// seeded [`SplitMix64`], so wander is a pure function of the RNG stream:
/// same seed, same walk, replayable. Zero `jitter` yields a zero force.
pub fn wander(rng: &mut SplitMix64, jitter: Fixed) -> Vec2 {
    if jitter.raw() <= 0 {
        return Vec2::ZERO;
    }
    let j = jitter.raw();
    let dx = Fixed::from_raw(rng.range(-j, j));
    let dy = Fixed::from_raw(rng.range(-j, j));
    truncate(Vec2::new(dx, dy), jitter)
}

/// Push away from every neighbor within `radius`, with 1/d falloff —
/// Reynolds' separation rule that keeps a flock from stacking. Each
/// neighbor contributes `(pos − other)/dist²`... the returned force is the
/// sum of unit repulsions, already truncated to `max_force` via the usual
/// `desired − vel` convention handled by the caller; here the raw sum is
/// clamped to `max_force` directly since there is no "desired velocity".
/// Distances of zero are skipped (coincident agents carry no direction).
pub fn separation(pos: Vec2, neighbors: &[Vec2], radius: Fixed, max_force: Fixed) -> Vec2 {
    if radius.raw() <= 0 {
        return Vec2::ZERO;
    }
    let mut fx = 0i64;
    let mut fy = 0i64;
    for other in neighbors {
        let dx = pos.x.raw() as i64 - other.x.raw() as i64;
        let dy = pos.y.raw() as i64 - other.y.raw() as i64;
        let dist = pos.distance(*other);
        if dist.is_zero() || dist.raw() > radius.raw() {
            continue;
        }
        // Repulsion weight ∝ (radius − dist)/radius on the unit direction:
        // nearby agents dominate, edge-of-radius agents fade out. The unit
        // vector is diff·ONE/dist (each |u| ≤ ~1.42·ONE), so the product
        // u·falloff stays under ~1.42·ONE·radius in i64 — no overflow path.
        let falloff = (radius.raw() - dist.raw()) as i64; // raw, ≤ radius
        let one = Fixed::ONE.raw() as i64;
        let d = dist.raw() as i64;
        fx += dx * one / d * falloff / (radius.raw() as i64);
        fy += dy * one / d * falloff / (radius.raw() as i64);
    }
    let clamp32 = |v: i64| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    truncate(
        Vec2::new(Fixed::from_raw(clamp32(fx)), Fixed::from_raw(clamp32(fy))),
        max_force,
    )
}

/// Steer toward the average heading of neighbors inside `radius` — the
/// second boid rule. `neighbors` are `(pos, vel)` pairs; `vel` is the
/// caller's current velocity. The desired velocity is the neighborhood's
/// mean velocity renormalized to `max_speed`, so a lone agent in an
/// empty neighborhood (or one already on-heading) gets zero force.
pub fn alignment(
    pos: Vec2,
    vel: Vec2,
    neighbors: &[(Vec2, Vec2)],
    radius: Fixed,
    max_speed: Fixed,
    max_force: Fixed,
) -> Vec2 {
    if radius.raw() <= 0 {
        return Vec2::ZERO;
    }
    let mut sum = Vec2::ZERO;
    let mut count = 0u32;
    for (np, nv) in neighbors {
        if pos.distance(*np).raw() > radius.raw() {
            continue;
        }
        sum = sum + *nv;
        count += 1;
    }
    if count == 0 {
        return Vec2::ZERO;
    }
    let avg = sum.scale(Fixed::ONE.div(Fixed::from_int(count as i32)));
    match avg.normalize() {
        Some(d) => steer_to(vel, d.scale(max_speed), max_force),
        // Neighbors' velocities cancel out → no consensus to follow.
        None => Vec2::ZERO,
    }
}

/// Steer toward the centroid of neighbors inside `radius` — the third
/// boid rule, literally [`seek`] on the local center of mass. Empty
/// neighborhoods and coincident centroids produce zero force.
pub fn cohesion(
    pos: Vec2,
    vel: Vec2,
    neighbors: &[Vec2],
    radius: Fixed,
    max_speed: Fixed,
    max_force: Fixed,
) -> Vec2 {
    if radius.raw() <= 0 {
        return Vec2::ZERO;
    }
    let mut sum = Vec2::ZERO;
    let mut count = 0u32;
    for np in neighbors {
        if pos.distance(*np).raw() > radius.raw() {
            continue;
        }
        sum = sum + *np;
        count += 1;
    }
    if count == 0 {
        return Vec2::ZERO;
    }
    let center = sum.scale(Fixed::ONE.div(Fixed::from_int(count as i32)));
    seek(pos, vel, center, max_speed, max_force)
}

/// Weighted blend of steering forces, then one clamp: `Σ wᵢ·fᵢ` truncated
/// to `max_force`. Negative weights are honored (they subtract). This is
/// the standard way to layer behaviors — e.g. `seek` at 1.0 plus
/// `separation` at 2.5 produces flocking that still pursues a goal.
pub fn combine(weighted: &[(Vec2, Fixed)], max_force: Fixed) -> Vec2 {
    let mut x = Fixed::ZERO;
    let mut y = Fixed::ZERO;
    for (f, w) in weighted {
        x = x + f.x.mul(*w);
        y = y + f.y.mul(*w);
    }
    truncate(Vec2::new(x, y), max_force)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(raw: i32) -> Fixed {
        Fixed::from_raw(raw)
    }

    fn v(x: i32, y: i32) -> Vec2 {
        Vec2::new(Fixed::from_int(x), Fixed::from_int(y))
    }

    fn approx(a: Fixed, b: Fixed, tol: i32) {
        assert!(
            (a.raw() - b.raw()).abs() <= tol,
            "{:?} vs {:?} tol {}",
            a.raw(),
            b.raw(),
            tol
        );
    }

    #[test]
    fn seek_points_at_the_target_at_max_force() {
        let f = seek(
            v(0, 0),
            Vec2::ZERO,
            v(10, 0),
            Fixed::from_int(4),
            Fixed::ONE,
        );
        approx(f.x, Fixed::ONE, 4);
        assert!(f.y.raw().abs() <= 2);
    }

    #[test]
    fn seek_offsets_an_existing_velocity() {
        // Moving +x at 2 wanting +x at 4 → residual force +x only.
        let f = seek(
            v(0, 0),
            v(2, 0),
            v(9, 0),
            Fixed::from_int(4),
            Fixed::from_int(5),
        );
        assert!(f.x > Fixed::ZERO);
        approx(f.x, Fixed::from_int(2), 8);
    }

    #[test]
    fn flee_points_directly_away() {
        let f = flee(v(0, 0), Vec2::ZERO, v(5, 0), Fixed::from_int(3), Fixed::ONE);
        approx(f.x, Fixed::from_raw(-Fixed::ONE.raw()), 4);
    }

    #[test]
    fn arrive_decelerates_inside_the_slow_radius() {
        let slow = Fixed::from_int(10);
        let far = arrive(
            v(20, 0),
            Vec2::ZERO,
            v(0, 0),
            Fixed::from_int(4),
            Fixed::from_int(10),
            slow,
        );
        let near = arrive(
            v(2, 0),
            Vec2::ZERO,
            v(0, 0),
            Fixed::from_int(4),
            Fixed::from_int(10),
            slow,
        );
        // Both aim −x, but the near force is a fraction of the far one.
        assert!(far.x < Fixed::ZERO && near.x < Fixed::ZERO);
        assert!(near.x.raw() > far.x.raw() / 2); // |near| < |far|/2
                                                 // At the target itself the force just brakes the current velocity.
        let stop = arrive(
            v(0, 0),
            v(1, 0),
            v(0, 0),
            Fixed::from_int(4),
            Fixed::ONE,
            slow,
        );
        assert_eq!(stop, v(-1, 0));
    }

    #[test]
    fn pursue_leads_a_moving_target() {
        let lead = pursue(
            v(0, 0),
            Vec2::ZERO,
            v(4, 0),
            v(0, 2), // target heading +y
            Fixed::from_int(4),
            Fixed::from_int(4),
        );
        // The desired direction gains a +y component vs plain seek.
        let plain = seek(
            v(0, 0),
            Vec2::ZERO,
            v(4, 0),
            Fixed::from_int(4),
            Fixed::from_int(4),
        );
        assert!(lead.y.raw() > plain.y.raw());
    }

    #[test]
    fn evade_aims_away_from_the_predicted_position() {
        let f = evade(
            v(0, 0),
            Vec2::ZERO,
            v(0, 4),
            v(0, 0),
            Fixed::from_int(3),
            Fixed::ONE,
        );
        assert!(f.y < Fixed::ZERO);
    }

    #[test]
    fn wander_is_deterministic_for_a_seed() {
        let mut a = SplitMix64::new(7);
        let mut b = SplitMix64::new(7);
        let j = Fixed::from_int(1);
        for _ in 0..8 {
            assert_eq!(wander(&mut a, j), wander(&mut b, j));
        }
        // And bounded by the jitter.
        let mut r = SplitMix64::new(1);
        for _ in 0..32 {
            let f = wander(&mut r, j);
            assert!(f.len_sq().raw() <= j.mul(j).raw() + 4);
        }
        assert_eq!(wander(&mut r, Fixed::ZERO), Vec2::ZERO);
    }

    #[test]
    fn separation_pushes_out_of_the_crowd() {
        let crowd = [v(1, 0), v(0, 1), v(-1, 0)];
        let f = separation(v(0, 0), &crowd, Fixed::from_int(4), Fixed::from_int(2));
        assert!(f.y < Fixed::ZERO); // the +y neighbor dominates
                                    // Beyond the radius: nothing.
        assert_eq!(
            separation(v(0, 0), &[v(9, 0)], Fixed::from_int(4), Fixed::ONE),
            Vec2::ZERO
        );
        // Coincident neighbors are directionless and skipped.
        assert_eq!(
            separation(v(0, 0), &[v(0, 0)], Fixed::from_int(4), Fixed::ONE),
            Vec2::ZERO
        );
    }

    #[test]
    fn alignment_follows_the_consensus_heading() {
        // Neighbors all moving +x inside radius → force pushes +x.
        let nb = [
            (v(1, 0), v(0, 3)),
            (v(-1, 0), v(0, 3)),
            (v(0, 1), v(0, 3)),
            (v(50, 0), v(9, 9)), // outside radius — ignored
        ];
        let f = alignment(
            v(0, 0),
            Vec2::ZERO,
            &nb,
            Fixed::from_int(4),
            Fixed::from_int(3),
            Fixed::from_int(2),
        );
        assert!(f.y > Fixed::ZERO);
        approx(f.y, Fixed::from_int(2), 8);
        // Agent already on-heading and fast enough: residual is only the
        // speed difference (3 → 2? no: neighbor speed 3 > max 3).
        let on_head = alignment(
            v(0, 0),
            v(0, 3).scale(Fixed::ONE),
            &nb,
            Fixed::from_int(4),
            Fixed::from_int(3),
            Fixed::from_int(5),
        );
        assert!(on_head.len().raw() < Fixed::from_ratio(1, 2).raw() + 16);
        // Empty and cancelling neighborhoods give nothing.
        assert_eq!(
            alignment(
                v(0, 0),
                Vec2::ZERO,
                &[],
                Fixed::from_int(4),
                Fixed::from_int(3),
                Fixed::ONE,
            ),
            Vec2::ZERO
        );
        let cancelling = [(v(1, 0), v(1, 0)), (v(-1, 0), v(-1, 0))];
        assert_eq!(
            alignment(
                v(0, 0),
                Vec2::ZERO,
                &cancelling,
                Fixed::from_int(4),
                Fixed::from_int(3),
                Fixed::ONE,
            ),
            Vec2::ZERO
        );
    }

    #[test]
    fn cohesion_steers_to_the_local_centroid() {
        // Centroid of {(2,0),(0,2)} = (1,1) → +x+y direction.
        let f = cohesion(
            v(0, 0),
            Vec2::ZERO,
            &[v(2, 0), v(0, 2)],
            Fixed::from_int(5),
            Fixed::from_int(3),
            Fixed::ONE,
        );
        assert!(f.x > Fixed::ZERO && f.y > Fixed::ZERO);
        // Neighbors beyond the radius don't shift the center.
        assert_eq!(
            cohesion(
                v(0, 0),
                Vec2::ZERO,
                &[v(9, 0)],
                Fixed::from_int(4),
                Fixed::from_int(3),
                Fixed::ONE,
            ),
            Vec2::ZERO
        );
        // Already at the centroid: zero force.
        assert_eq!(
            cohesion(
                v(1, 0),
                Vec2::ZERO,
                &[v(0, 0), v(2, 0)],
                Fixed::from_int(5),
                Fixed::from_int(3),
                Fixed::ONE,
            ),
            Vec2::ZERO
        );
    }

    #[test]
    fn combine_weights_and_clamps() {
        let a = (v(1, 0).normalize().unwrap(), Fixed::ONE);
        let b = (v(0, 1).normalize().unwrap(), Fixed::ONE);
        let c = combine(&[a, b], Fixed::ONE);
        // 45°-ish, magnitude ≤ 1.
        assert!(c.len_sq().raw() <= Fixed::ONE.raw() + 4);
        assert!(c.x > Fixed::ZERO && c.y > Fixed::ZERO);
        // A negative weight subtracts.
        let d = combine(&[a, (b.0, Fixed::from_raw(-Fixed::ONE.raw()))], Fixed::ONE);
        assert!(d.y < Fixed::ZERO);
    }

    #[test]
    fn truncate_respects_the_cap() {
        let big = v(10, 0);
        assert_eq!(truncate(big, Fixed::ONE).len().raw(), Fixed::ONE.raw());
        assert_eq!(truncate(Vec2::ZERO, Fixed::ONE), Vec2::ZERO);
        let small = v(0, 0) + Vec2::new(f(100), f(0));
        assert_eq!(truncate(small, Fixed::ONE), small);
    }

    #[test]
    fn every_behavior_is_deterministic() {
        let (p, vel, t, tv) = (v(1, 1), v(1, 0), v(5, 4), v(1, 1));
        let (ms, mf) = (Fixed::from_int(3), Fixed::ONE);
        assert_eq!(seek(p, vel, t, ms, mf), seek(p, vel, t, ms, mf));
        assert_eq!(flee(p, vel, t, ms, mf), flee(p, vel, t, ms, mf));
        assert_eq!(
            arrive(p, vel, t, ms, mf, Fixed::from_int(6)),
            arrive(p, vel, t, ms, mf, Fixed::from_int(6))
        );
        assert_eq!(pursue(p, vel, t, tv, ms, mf), pursue(p, vel, t, tv, ms, mf));
        assert_eq!(evade(p, vel, t, tv, ms, mf), evade(p, vel, t, tv, ms, mf));
    }
}
