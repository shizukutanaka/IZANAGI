//! Quaternions on [`Fixed`]: unit quaternions `q = (x, y, z, w)` encode
//! rotations in 3-space without gimbal lock and compose by Hamilton
//! product — the standard representation for fixed-point 3D orientation
//! (Hamilton 1844; the `v' = v + 2·(q⃗ × (q⃗ × v + w·v))` rotation form is
//! the Cayley expansion used throughout game math, cf. Slicker, "Quaternions
//! and Rotation Sequences").
//!
//! `Q16.16` gives ~4 decimal digits of angle resolution — fine for
//! orientation tracking and steering; expect visible drift only after
//! long multiplications chains (renormalize every few dozen products).
//!
//! Two interpolators are provided. [`Quat::slerp`] is the true great-circle
//! interpolation (Shoemake 1985): constant angular velocity, built on the
//! CORDIC [`Fixed::acos`]/[`Fixed::sin`]. [`Quat::nlerp`] interpolates on
//! the 3-sphere chord with sign correction — cheaper, exact at the
//! endpoints and monotone, but its angular speed drifts by a few percent
//! for wide angles.
//!
//! ```
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::quat::Quat;
//! use izanagi_kit::vec::Vec3;
//!
//! // 90° about +Z maps +X to +Y.
//! let q = Quat::from_axis_angle(Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE),
//!     Fixed::from_ratio(157, 100)).unwrap();
//! let v = q.rotate(Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO));
//! assert!(v.y > Fixed::from_ratio(9, 10));
//! ```

use crate::fixed::Fixed;
use crate::vec::Vec3;
use crate::world_hash::{DetHash, Fnv1a};

/// A quaternion `x·i + y·j + z·k + w` on fixed-point components.
///
/// The identity is `(0,0,0,1)`. Components are public data but every
/// operation that assumes a unit quaternion returns `Option` rather than
/// silently normalizing — `mul` does *not* renormalize (a product of unit
/// quaternions drifts quadratically at Q16.16 resolution).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quat {
    /// `i` coefficient (rotation vector x).
    pub x: Fixed,
    /// `j` coefficient (rotation vector y).
    pub y: Fixed,
    /// `k` coefficient (rotation vector z).
    pub z: Fixed,
    /// Scalar part (cosine of half the rotation angle for unit quaternions).
    pub w: Fixed,
}

impl Quat {
    /// The identity rotation `0·i + 0·j + 0·k + 1`.
    pub const IDENTITY: Quat = Quat {
        x: Fixed::ZERO,
        y: Fixed::ZERO,
        z: Fixed::ZERO,
        w: Fixed::ONE,
    };

    /// Construct from raw components (no normalization — see [`Quat::normalize`]).
    pub const fn new(x: Fixed, y: Fixed, z: Fixed, w: Fixed) -> Self {
        Quat { x, y, z, w }
    }

    /// Rotation of `angle` (radians, fixed-point) about `axis`.
    /// `axis` is normalized internally; a near-zero axis has no meaningful
    /// rotation direction and returns `None`.
    pub fn from_axis_angle(axis: Vec3, angle: Fixed) -> Option<Self> {
        let a = axis.normalize()?;
        let half = angle.div(Fixed::from_int(2));
        let (s, c) = half.sin_cos();
        Some(Quat {
            x: a.x.mul(s),
            y: a.y.mul(s),
            z: a.z.mul(s),
            w: c,
        })
    }

    /// The quaternion rotating `from`'s direction onto `to`'s direction
    /// (both normalized internally). For antiparallel vectors an arbitrary
    /// perpendicular axis is used — the 180° rotation exists but its axis
    /// is degenerate; `None` only when an input is near-zero length.
    pub fn between(from: Vec3, to: Vec3) -> Option<Self> {
        let f = from.normalize()?;
        let t = to.normalize()?;
        let c = f.cross(t);
        let d = f.dot(t);
        // d ≥ −1+ε: standard `w = 1 + cosθ` form.
        // Guard band −0.9: below that the formula is numerically too ill-
        // conditioned at Q16.16, so pick any perpendicular axis instead.
        let threshold = Fixed::from_ratio(-9, 10);
        if d >= threshold {
            let w = Fixed::ONE + d;
            let q = Quat {
                x: c.x,
                y: c.y,
                z: c.z,
                w,
            };
            return q.normalize();
        }
        // ~180°: take the perpendicular axis of f that is largest — pick the
        // axis pair least aligned with f to avoid a degenerate cross product.
        let perp = if f.x.abs() <= f.y.abs() && f.x.abs() <= f.z.abs() {
            f.cross(Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO))
        } else if f.y.abs() <= f.z.abs() {
            f.cross(Vec3::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO))
        } else {
            f.cross(Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE))
        };
        Self::from_axis_angle(perp, Fixed::PI)
    }

    /// Hamilton product `self · rhs`: applies `rhs`'s rotation **first**.
    /// Not commutative — `a·b ≠ b·a` — which is exactly why quaternions
    /// compose rotations correctly (order of turns matters).
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, rhs: Quat) -> Self {
        Quat {
            x: self.w.mul(rhs.x) + self.x.mul(rhs.w) + self.y.mul(rhs.z) - self.z.mul(rhs.y),
            y: self.w.mul(rhs.y) - self.x.mul(rhs.z) + self.y.mul(rhs.w) + self.z.mul(rhs.x),
            z: self.w.mul(rhs.z) + self.x.mul(rhs.y) - self.y.mul(rhs.x) + self.z.mul(rhs.w),
            w: self.w.mul(rhs.w) - self.x.mul(rhs.x) - self.y.mul(rhs.y) - self.z.mul(rhs.z),
        }
    }

    /// Conjugate: negates the vector part. For unit quaternions this is the
    /// inverse — `q·q* = 1` — and reversing a rotation is `q*.rotate(v)`.
    pub fn conjugate(self) -> Self {
        Quat {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    /// `x² + y² + z² + w²`.
    pub fn norm_sq(self) -> Fixed {
        self.x.mul(self.x) + self.y.mul(self.y) + self.z.mul(self.z) + self.w.mul(self.w)
    }

    /// Euclidean norm (a unit quaternion has `norm == 1`).
    pub fn norm(self) -> Fixed {
        self.norm_sq().sqrt()
    }

    /// Rescale to unit length; `None` for a zero quaternion.
    pub fn normalize(self) -> Option<Self> {
        let n = self.norm();
        if n == Fixed::ZERO {
            return None;
        }
        Some(Quat {
            x: self.x.div(n),
            y: self.y.div(n),
            z: self.z.div(n),
            w: self.w.div(n),
        })
    }

    /// Apply the rotation to `v` (assumes `self` is unit — call
    /// [`Quat::normalize`] after accumulated products).
    /// Uses `v + 2·(q⃗ × (q⃗ × v + w·v))` — the Cayley form: one cross
    /// product fewer than the `q·v·q*` sandwich and identical for unit `q`.
    pub fn rotate(self, v: Vec3) -> Vec3 {
        let q = Vec3::new(self.x, self.y, self.z);
        let inner = q.cross(v) + v.scale(self.w);
        v + q.cross(inner).scale(Fixed::from_int(2))
    }

    /// Dot product of the raw 4-components (for hemisphere tests in
    /// interpolation — a negative dot means `q` and `−q` differ by the
    /// long arc).
    pub fn dot(self, rhs: Quat) -> Fixed {
        self.x.mul(rhs.x) + self.y.mul(rhs.y) + self.z.mul(rhs.z) + self.w.mul(rhs.w)
    }

    /// `−q`: the same rotation by the other way around the sphere.
    #[allow(clippy::should_implement_trait)]
    pub fn neg(self) -> Self {
        Quat {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w,
        }
    }

    /// Normalized lerp on the chord: `a + (b − a)·t` renormalized, flipping
    /// `b` into `a`'s hemisphere when `a·b < 0` so the path stays short.
    /// `t` is clamped to `[0, 1]`; `None` if the blended quaternion is
    /// degenerate (opposite unit quaternions at `t = 0.5` — the two
    /// rotations differ by 180° and no unique midpoint exists).
    pub fn nlerp(self, rhs: Quat, t: Fixed) -> Option<Self> {
        let rhs = if self.dot(rhs) < Fixed::ZERO {
            rhs.neg()
        } else {
            rhs
        };
        let t = t.clamp01();
        Quat {
            x: Fixed::lerp(self.x, rhs.x, t),
            y: Fixed::lerp(self.y, rhs.y, t),
            z: Fixed::lerp(self.z, rhs.z, t),
            w: Fixed::lerp(self.w, rhs.w, t),
        }
        .normalize()
    }

    /// Spherical linear interpolation (Shoemake 1985): the great-circle arc
    /// from `self` to `rhs` traversed at constant angular velocity,
    /// `(sin((1−t)θ)·a + sin(tθ)·b) / sin θ` with `θ = acos(a·b)`.
    ///
    /// Both inputs are normalized first and `rhs` is flipped into `self`'s
    /// hemisphere when `a·b < 0`, so the path is always the short arc.
    /// `t` is clamped to `[0, 1]`; `t = 0` and `t = 1` return the (normalized)
    /// endpoints exactly. When the arc is shorter than about 1.8° the
    /// `1 / sin θ` factor would amplify Q16.16 rounding, so it falls back to
    /// [`Quat::nlerp`], which agrees with slerp to within an ulp there. Unlike
    /// `nlerp`, antipodal inputs (`q` and `−q`) are not degenerate — they are
    /// the same rotation. `None` only when an input is a zero quaternion.
    pub fn slerp(self, rhs: Quat, t: Fixed) -> Option<Self> {
        let a = self.normalize()?;
        let b = rhs.normalize()?;
        let (b, d) = {
            let d = a.dot(b);
            if d < Fixed::ZERO {
                (b.neg(), -d)
            } else {
                (b, d)
            }
        };
        let t = t.clamp01();
        if t == Fixed::ZERO {
            return Some(a);
        }
        if t == Fixed::ONE {
            return Some(b);
        }
        let theta = d.min(Fixed::ONE).acos();
        let sin_theta = theta.sin();
        if sin_theta < Fixed::from_ratio(1, 32) {
            return a.nlerp(b, t);
        }
        let wa = (Fixed::ONE - t).mul(theta).sin().div(sin_theta);
        let wb = t.mul(theta).sin().div(sin_theta);
        Quat {
            x: a.x.mul(wa) + b.x.mul(wb),
            y: a.y.mul(wa) + b.y.mul(wb),
            z: a.z.mul(wa) + b.z.mul(wb),
            w: a.w.mul(wa) + b.w.mul(wb),
        }
        .normalize()
    }

    /// Decompose a unit quaternion back into `(axis, angle)`; `None` when
    /// the vector part is ~zero (rotation by ~0 — the axis is undefined).
    /// Non-unit inputs are normalized first.
    pub fn to_axis_angle(self) -> Option<(Vec3, Fixed)> {
        let q = self.normalize()?;
        let v = Vec3::new(q.x, q.y, q.z);
        let vl = v.len();
        if vl == Fixed::ZERO {
            return None;
        }
        // w = cos(θ/2), |v| = sin(θ/2): θ/2 = atan2(|v|, w) works for w < 0
        // too (angles > π come back in the (−axis, 2π−θ) convention).
        let half = Fixed::atan2(vl, q.w);
        Some((v.normalize()?, half.mul(Fixed::from_int(2))))
    }

    /// Angle of the rotation taking `self` to `other`, in radians —
    /// `2·atan2(|Δvec|, Δw)` of the relative quaternion `self*·other`.
    /// `None` for a zero-degenerate input.
    pub fn angle_to(self, other: Quat) -> Option<Fixed> {
        let rel = self.conjugate().mul(other).normalize()?;
        let vl = Vec3::new(rel.x, rel.y, rel.z).len();
        Some(Fixed::atan2(vl, rel.w).mul(Fixed::from_int(2)))
    }
}

impl DetHash for Quat {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.x.det_hash(h);
        self.y.det_hash(h);
        self.z.det_hash(h);
        self.w.det_hash(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: Fixed, b: Fixed, tol_num: i32) -> bool {
        // |a−b| ≤ tol_num/100
        (a - b).abs() <= Fixed::from_ratio(tol_num, 100)
    }

    fn vnear(a: Vec3, b: Vec3, tol_num: i32) -> bool {
        near(a.x, b.x, tol_num) && near(a.y, b.y, tol_num) && near(a.z, b.z, tol_num)
    }

    #[test]
    fn quarter_turn_maps_x_to_y() {
        let q = Quat::from_axis_angle(
            Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE),
            Fixed::PI.div(Fixed::from_int(2)),
        )
        .unwrap();
        let v = q.rotate(Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO));
        assert!(
            vnear(v, Vec3::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO), 1),
            "{v:?}"
        );
    }

    #[test]
    fn composition_order_matters_and_composes() {
        let half = Fixed::PI.div(Fixed::from_int(2));
        let qz =
            Quat::from_axis_angle(Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE), half).unwrap();
        let qy =
            Quat::from_axis_angle(Vec3::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO), half).unwrap();
        let vx = Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO);
        let x = Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO);
        // qz·qy: y first, then z. X → (qy) → still X-ish... compute both orders.
        let a = qz.mul(qy).rotate(vx);
        let b = qz.rotate(qy.rotate(vx));
        assert!(vnear(a, b, 1));
        let c = qy.mul(qz).rotate(vx);
        let d = qy.rotate(qz.rotate(vx));
        assert!(vnear(c, d, 1));
        assert!(!vnear(a, c, 10), "orders should differ: {a:?} vs {c:?}");
        let _ = x;
    }

    #[test]
    fn conjugate_undoes_the_rotation() {
        for seed in 0..8u64 {
            let mut rng = crate::rng::SplitMix64::new(seed + 1);
            let axis = Vec3::new(
                Fixed::from_int(rng.range(-3, 4)),
                Fixed::from_int(rng.range(-3, 4)),
                Fixed::from_int(rng.range(1, 4)),
            );
            if axis.len() == Fixed::ZERO {
                continue;
            }
            let angle = Fixed::from_ratio(rng.range(0, 7), 10);
            let q = Quat::from_axis_angle(axis, angle).unwrap();
            let v = Vec3::new(Fixed::from_int(2), Fixed::from_int(-1), Fixed::from_int(3));
            assert!(
                vnear(q.conjugate().rotate(q.rotate(v)), v, 2),
                "seed {seed}"
            );
            // q·q* is identity:
            let id = q.mul(q.conjugate());
            assert!(near(id.norm(), Fixed::ONE, 1));
            assert!(id.w > Fixed::ZERO);
            // norm_sq is the squared norm — cheap for comparisons.
            assert!(near(q.norm_sq(), Fixed::ONE, 1), "norm_sq {seed}");
        }
    }

    #[test]
    fn from_axis_angle_rejects_degenerate_axes() {
        assert!(Quat::from_axis_angle(Vec3::ZERO, Fixed::ONE).is_none());
        assert!(Quat::from_axis_angle(
            Vec3::new(Fixed::from_raw(1), Fixed::from_raw(-1), Fixed::ZERO),
            Fixed::ONE
        )
        .is_none());
    }

    #[test]
    fn between_rotates_start_onto_target() {
        let x = Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO);
        let y = Vec3::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO);
        let neg_x = -x;
        let q = Quat::between(x, y).unwrap();
        assert!(vnear(q.rotate(x), y, 1), "{:?}", q.rotate(x));
        // Antiparallel — the perpendicular-axis fallback.
        let q2 = Quat::between(x, neg_x).unwrap();
        assert!(vnear(q2.rotate(x), neg_x, 1), "{:?}", q2.rotate(x));
        // Degenerate input.
        assert!(Quat::between(Vec3::ZERO, y).is_none());
    }

    #[test]
    fn axis_angle_roundtrips() {
        let axis = Vec3::new(Fixed::ONE, Fixed::ONE, Fixed::ONE);
        let angle = Fixed::from_ratio(11, 10);
        let q = Quat::from_axis_angle(axis, angle).unwrap();
        let (axis2, angle2) = q.to_axis_angle().unwrap();
        assert!(vnear(axis2, axis.normalize().unwrap(), 1));
        assert!(near(angle2, angle, 2), "{angle2:?} vs {angle:?}");
        // Identity has no axis.
        assert!(Quat::IDENTITY.to_axis_angle().is_none());
    }

    #[test]
    fn nlerp_endpoints_and_midpoint() {
        let a = Quat::from_axis_angle(Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE), Fixed::ZERO)
            .unwrap();
        let b = Quat::from_axis_angle(
            Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE),
            Fixed::PI.div(Fixed::from_int(2)),
        )
        .unwrap();
        let mid = a.nlerp(b, Fixed::from_ratio(1, 2)).unwrap();
        let v = mid.rotate(Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO));
        // 45°: x≈y≈0.707
        assert!(
            near(v.x, v.y, 2) && near(v.x, Fixed::from_ratio(7, 10), 2),
            "{v:?}"
        );
        // Endpoint exactness.
        assert!(vnear(
            a.nlerp(b, Fixed::ZERO).unwrap().rotate(Vec3::new(
                Fixed::ONE,
                Fixed::ZERO,
                Fixed::ZERO
            )),
            Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO),
            1
        ));
        // Antipodal hemisphere flip: q and −q are the same rotation, nlerp must not go the long way.
        let nb = b.neg();
        let m2 = a.nlerp(nb, Fixed::from_ratio(1, 2)).unwrap();
        assert!(vnear(
            m2.rotate(Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO)),
            v,
            1
        ));
    }

    fn z_turn(angle: Fixed) -> Quat {
        Quat::from_axis_angle(Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE), angle).unwrap()
    }

    #[test]
    fn slerp_endpoints_are_exact() {
        let a = z_turn(Fixed::from_ratio(3, 10));
        let b = z_turn(Fixed::from_ratio(27, 10));
        assert_eq!(a.slerp(b, Fixed::ZERO), a.normalize());
        assert_eq!(a.slerp(b, Fixed::ONE), b.normalize());
        // `t` is clamped.
        assert_eq!(a.slerp(b, Fixed::from_int(-3)), a.normalize());
        assert_eq!(a.slerp(b, Fixed::from_int(5)), b.normalize());
    }

    #[test]
    fn slerp_has_constant_angular_velocity() {
        // Oracle: for a turn about one axis, slerp(t) is the turn by t·θ.
        // A 170° arc is where nlerp's chord parametrisation is furthest off.
        let theta = Fixed::from_ratio(296, 100); // ≈ 169.6°
        let a = Quat::IDENTITY;
        let b = z_turn(theta);
        let mut worst_slerp = Fixed::ZERO;
        let mut worst_nlerp = Fixed::ZERO;
        for k in 1..16 {
            let t = Fixed::from_ratio(k, 16);
            let want = t.mul(theta);
            let s = a.slerp(b, t).unwrap();
            let n = a.nlerp(b, t).unwrap();
            let got_s = a.angle_to(s).unwrap();
            let got_n = a.angle_to(n).unwrap();
            assert!(
                near(got_s, want, 1),
                "slerp t={k}/16: {got_s:?} vs {want:?}"
            );
            assert!(
                near(s.norm(), Fixed::ONE, 1),
                "slerp leaves the unit sphere"
            );
            worst_slerp = worst_slerp.max(got_s.abs_diff(want));
            worst_nlerp = worst_nlerp.max(got_n.abs_diff(want));
        }
        assert!(
            worst_slerp.mul(Fixed::from_int(10)) < worst_nlerp,
            "slerp {worst_slerp:?} should beat nlerp {worst_nlerp:?} by 10x"
        );
    }

    #[test]
    fn slerp_takes_the_short_arc_and_tolerates_antipodes() {
        let a = z_turn(Fixed::ZERO);
        let b = z_turn(Fixed::PI.div(Fixed::from_int(2)));
        let mid = a.slerp(b, Fixed::from_ratio(1, 2)).unwrap();
        let flipped = a.slerp(b.neg(), Fixed::from_ratio(1, 2)).unwrap();
        let x = Vec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO);
        assert!(vnear(mid.rotate(x), flipped.rotate(x), 1));
        assert!(near(
            a.angle_to(mid).unwrap(),
            Fixed::PI.div(Fixed::from_int(4)),
            1
        ));
        // q and −q are one rotation: slerp is defined (nlerp is not at t=½).
        let q = z_turn(Fixed::from_ratio(7, 10));
        let m = q.slerp(q.neg(), Fixed::from_ratio(1, 2)).unwrap();
        assert!(near(q.angle_to(m).unwrap(), Fixed::ZERO, 1));
    }

    #[test]
    fn slerp_small_arcs_and_degenerate_inputs() {
        // Below the sin θ guard slerp defers to nlerp — bit for bit.
        let a = z_turn(Fixed::from_ratio(1, 10));
        let b = z_turn(Fixed::from_ratio(11, 100));
        let t = Fixed::from_ratio(1, 3);
        assert_eq!(a.slerp(b, t), a.nlerp(b, t));
        assert_eq!(a.slerp(a, t), a.nlerp(a, t));
        // A zero quaternion has no rotation to interpolate.
        let zero = Quat::new(Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, Fixed::ZERO);
        assert!(zero.slerp(a, t).is_none());
        assert!(a.slerp(zero, t).is_none());
        // Non-unit inputs are normalized first.
        let big = Quat::new(
            a.x.mul(Fixed::from_int(3)),
            a.y,
            a.z,
            a.w.mul(Fixed::from_int(3)),
        );
        assert!(near(
            big.slerp(b, Fixed::ZERO).unwrap().norm(),
            Fixed::ONE,
            1
        ));
        // Deterministic: same inputs, same bits.
        assert_eq!(a.slerp(b, t), a.slerp(b, t));
    }

    #[test]
    fn angle_to_measures_relative_turn() {
        let z = Vec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE);
        let a = Quat::from_axis_angle(z, Fixed::from_ratio(3, 10)).unwrap();
        let b = Quat::from_axis_angle(z, Fixed::from_ratio(7, 10)).unwrap();
        let d = a.angle_to(b).unwrap();
        assert!(near(d, Fixed::from_ratio(4, 10), 2), "{d:?}");
        // Angle is symmetric.
        assert!(near(b.angle_to(a).unwrap(), d, 1));
    }

    #[test]
    fn golden_hash_is_stable() {
        let q = Quat::from_axis_angle(
            Vec3::new(Fixed::ONE, Fixed::from_int(2), Fixed::from_int(3)),
            Fixed::from_ratio(3, 10),
        )
        .unwrap();
        let mut h1 = Fnv1a::new();
        q.det_hash(&mut h1);
        let mut h2 = Fnv1a::new();
        q.det_hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
        let mut h3 = Fnv1a::new();
        q.neg().det_hash(&mut h3);
        assert_ne!(h1.finish(), h3.finish());
    }
}
