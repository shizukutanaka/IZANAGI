//! Inverse kinematics on `Fixed`/`Vec2`: the closed-form two-bone solver
//! (law of cosines, elbow up/down) and cyclic coordinate descent (CCD) for
//! chains of arbitrary length. Angles come from [`Fixed::atan2`]'s CORDIC, so
//! every solve is bit-exact for replays — the kinematics counterpart of
//! `spring`/`verlet`.
//!
//! ```
//! use izanagi_kit::{ik, vec::Vec2, fixed::Fixed};
//! let elbow = ik::two_bone(
//!     Vec2::new(Fixed::ZERO, Fixed::ZERO),
//!     Vec2::new(Fixed::from_int(2), Fixed::ZERO),
//!     Fixed::ONE, Fixed::ONE, false,
//! ).unwrap();
//! // Two unit bones reach (2,0) fully extended → elbow anywhere on the chain.
//! assert!(elbow.x > Fixed::ZERO);
//! ```

use crate::fixed::Fixed;
use crate::vec::Vec2;

/// `acos(x)` via `atan2(√(1 − x²), x)`; `x` is clamped into `[-1, 1]`.
fn acos(x: Fixed) -> Fixed {
    let c = x.clamp(Fixed::ZERO - Fixed::ONE, Fixed::ONE);
    let s = (Fixed::ONE - c.mul(c)).sqrt();
    Fixed::atan2(s, c)
}

/// Rotate `v` by `angle` radians.
fn rotate(v: Vec2, angle: Fixed) -> Vec2 {
    let (s, c) = Fixed::sin_cos(angle);
    Vec2::new(v.x.mul(c) - v.y.mul(s), v.x.mul(s) + v.y.mul(c))
}

/// Closed-form elbow position for a two-bone chain rooted at `joint` whose
/// end must reach `target`, with bone lengths `l1` and `l2`. Returns `None`
/// when the target is strictly unreachable (`|target − joint| > l1 + l2` or
/// inside `|l1 − l2|`); a reachable-with-full-extension target returns the
/// collinear elbow. `flip` picks the elbow-below solution.
pub fn two_bone(joint: Vec2, target: Vec2, l1: Fixed, l2: Fixed, flip: bool) -> Option<Vec2> {
    let d = target - joint;
    let dist = d.len();
    let lsum = l1 + l2;
    let ldiff = (l1 - l2).abs();
    if dist > lsum || dist < ldiff {
        return None;
    }
    if dist.raw() == 0 {
        // Concentric: any elbow on the l1-circle works; return +x.
        return Some(joint + Vec2::new(l1, Fixed::ZERO));
    }
    // Law of cosines: interior angle at the root between target dir and l1.
    // cos α = (d² + l1² − l2²) / (2·d·l1). Compute in i64 raw space to dodge
    // the Fixed::mul i32 overflow on large lengths.
    let dn = (dist.raw() as i64) * (dist.raw() as i64);
    let a1 = (l1.raw() as i64) * (l1.raw() as i64);
    let a2 = (l2.raw() as i64) * (l2.raw() as i64);
    let denom = 2 * (dist.raw() as i64) * (l1.raw() as i64);
    let cos_raw = if denom == 0 {
        1i64 << 16
    } else {
        (((dn + a1 - a2) << 16) / denom).clamp(-(1i64 << 16), 1i64 << 16)
    };
    let alpha = acos(Fixed::from_raw(cos_raw as i32));
    let base = Fixed::atan2(d.y, d.x);
    let ang = if flip { base - alpha } else { base + alpha };
    let (s, c) = Fixed::sin_cos(ang);
    Some(joint + Vec2::new(c.mul(l1), s.mul(l1)))
}

/// One result of a [`ccd`] solve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IkResult {
    /// Whether the chain's tip reached `target` within `eps`.
    pub reached: bool,
    /// Iterations actually spent (≤ `iters`).
    pub iters: u32,
}

/// Cyclic coordinate descent on a planar chain: `joints[0]` is the fixed
/// root, `lengths[i]` the bone from `joints[i]` to `joints[i+1]`; the chain
/// has `lengths.len() + 1` joints. Rotates each joint's outgoing bone toward
/// the target, end to root, for up to `iters` sweeps or until the tip is
/// within `eps` of `target`. Joint positions are updated in place; bone
/// lengths are preserved exactly (each step is a rigid rotation).
pub fn ccd(
    joints: &mut [Vec2],
    lengths: &[Fixed],
    target: Vec2,
    eps: Fixed,
    iters: u32,
) -> IkResult {
    if joints.len() != lengths.len() + 1 {
        return IkResult {
            reached: false,
            iters: 0,
        };
    }
    let mut spent = 0;
    for it in 0..iters {
        // End-to-root: rotate bone i around joint i so the tip nears target.
        for i in (0..lengths.len()).rev() {
            let tip = *joints
                .last()
                .unwrap_or(&Vec2::new(Fixed::ZERO, Fixed::ZERO));
            let cur = tip - joints[i];
            let want = target - joints[i];
            if cur.len().raw() == 0 || want.len().raw() == 0 {
                continue;
            }
            let a_cur = Fixed::atan2(cur.y, cur.x);
            let a_want = Fixed::atan2(want.y, want.x);
            let delta = a_want - a_cur;
            for j in (i + 1)..joints.len() {
                joints[j] = joints[i] + rotate(joints[j] - joints[i], delta);
            }
        }
        spent = it + 1;
        let tip = *joints
            .last()
            .unwrap_or(&Vec2::new(Fixed::ZERO, Fixed::ZERO));
        if (tip - target).len() <= eps {
            return IkResult {
                reached: true,
                iters: spent,
            };
        }
    }
    let tip = *joints
        .last()
        .unwrap_or(&Vec2::new(Fixed::ZERO, Fixed::ZERO));
    IkResult {
        reached: (tip - target).len() <= eps,
        iters: spent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fi(n: i32) -> Fixed {
        Fixed::from_int(n)
    }
    fn fr(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }
    fn v(x: Fixed, y: Fixed) -> Vec2 {
        Vec2::new(x, y)
    }
    fn dist(a: Vec2, b: Vec2) -> f64 {
        let d = a - b;
        (d.x.raw() as f64 / 65536.0).hypot(d.y.raw() as f64 / 65536.0)
    }

    #[test]
    fn two_bone_reachable_and_unreachable() {
        let j = v(Fixed::ZERO, Fixed::ZERO);
        // Too far: reach 2 > 1+1.
        assert!(two_bone(j, v(fi(3), Fixed::ZERO), Fixed::ONE, Fixed::ONE, false).is_none());
        // Too close: inside the |l1−l2| dead zone (equal bones have none).
        assert!(two_bone(j, v(fr(2, 5), Fixed::ZERO), fi(2), Fixed::ONE, false).is_none());
        // Equal bones reach any interior distance (elbow folds back).
        assert!(two_bone(j, v(fr(2, 5), Fixed::ZERO), Fixed::ONE, Fixed::ONE, false).is_some());
        // Full extension: elbow lies on the segment.
        let e = two_bone(j, v(fi(2), Fixed::ZERO), Fixed::ONE, Fixed::ONE, false).unwrap();
        assert!(dist(e, v(Fixed::ZERO, Fixed::ZERO)) - 1.0 < 0.01);
        assert!(dist(e, v(fi(2), Fixed::ZERO)) - 1.0 < 0.01);
    }

    #[test]
    fn two_bone_elbow_obeys_lengths_and_flip_symmetry() {
        let j = v(Fixed::ZERO, Fixed::ZERO);
        let t = v(fi(1), fi(1)); // dist √2, l1=1.5, l2=1
        let up = two_bone(j, t, fr(3, 2), Fixed::ONE, false).unwrap();
        let dn = two_bone(j, t, fr(3, 2), Fixed::ONE, true).unwrap();
        // Bone lengths preserved.
        assert!((dist(j, up) - 1.5).abs() < 0.01);
        assert!((dist(up, t) - 1.0).abs() < 0.01);
        assert!((dist(j, dn) - 1.5).abs() < 0.01);
        assert!((dist(dn, t) - 1.0).abs() < 0.01);
        // Flip mirrors across the target axis: same projection on the axis,
        // opposite perpendicular offsets.
        let u = v(fi(1), fi(1)); // unit-ish axis (magnitude irrelevant)
        assert!((up.dot(u).raw() - dn.dot(u).raw()).abs() < 300);
        let perp = v(Fixed::ZERO - fi(1), fi(1));
        assert!((up.dot(perp).raw() + dn.dot(perp).raw()).abs() < 300);
        assert_ne!(up, dn);
    }

    #[test]
    fn two_bone_right_angle_pins() {
        // 3-4-5 triangle: root at origin, target at (5,0), l1=3, l2=4 →
        // elbow at (1.8, ±2.4): cos α = (25+9−16)/30 = 0.6.
        let j = v(Fixed::ZERO, Fixed::ZERO);
        let t = v(fi(5), Fixed::ZERO);
        let e = two_bone(j, t, fi(3), fi(4), false).unwrap();
        assert!((e.x.raw() - 117965).abs() < 300); // x≈1.8
        assert!((e.y.raw() - 157286).abs() < 300); // y≈2.4
        let e2 = two_bone(j, t, fi(3), fi(4), true).unwrap();
        assert!((e2.y.raw() + 157286).abs() < 300);
    }

    #[test]
    fn ccd_reaches_target() {
        // Chain root at origin, three unit bones along +x.
        let mut joints = vec![
            v(Fixed::ZERO, Fixed::ZERO),
            v(Fixed::ONE, Fixed::ZERO),
            v(fi(2), Fixed::ZERO),
            v(fi(3), Fixed::ZERO),
        ];
        let lengths = vec![Fixed::ONE, Fixed::ONE, Fixed::ONE];
        let r = ccd(&mut joints, &lengths, v(fi(1), fi(1)), fr(1, 50), 32);
        assert!(r.reached);
        let tip = joints[3];
        assert!(dist(tip, v(fi(1), fi(1))) < 0.03);
        // Root stays put and lengths are preserved.
        assert_eq!(joints[0], v(Fixed::ZERO, Fixed::ZERO));
        for i in 0..3 {
            assert!((dist(joints[i], joints[i + 1]) - 1.0).abs() < 0.02);
        }
    }

    #[test]
    fn ccd_out_of_reach_stretches_and_reports() {
        let mut joints = vec![v(Fixed::ZERO, Fixed::ZERO), v(Fixed::ONE, Fixed::ZERO)];
        let lengths = vec![Fixed::ONE];
        let r = ccd(&mut joints, &lengths, v(fi(10), Fixed::ZERO), fr(1, 100), 8);
        assert!(!r.reached);
        assert_eq!(r.iters, 8);
        // Chain points at the target anyway.
        assert!(joints[1].x > Fixed::ZERO && joints[1].y.abs() < fr(1, 10));
    }

    #[test]
    fn ccd_deterministic_replay() {
        let mut a = vec![
            v(Fixed::ZERO, Fixed::ZERO),
            v(Fixed::ONE, fi(1)),
            v(fr(3, 2), fi(1)),
        ];
        let mut b = a.clone();
        let l = vec![fr(3, 2), fi(1)];
        let t = v(fi(-1), fi(2));
        assert_eq!(
            ccd(&mut a, &l, t, fr(1, 100), 12),
            ccd(&mut b, &l, t, fr(1, 100), 12)
        );
        assert_eq!(a, b);
        // Mismatched chain spec degrades to unreached, no panic.
        let mut bad = vec![v(Fixed::ZERO, Fixed::ZERO)];
        assert!(!ccd(&mut bad, &l, t, fr(1, 100), 4).reached);
    }
}
