//! Fixed-point 2-D and 3-D vectors over [`Fixed`](crate::Fixed).
//!
//! All arithmetic uses the same Q16.16 saturating rules as [`Fixed`] so
//! vectors never silently wrap, and the results are bit-identical across
//! targets — safe to fold into the world hash and deterministic in replay.
//!
//! Why not `f32`? See the `fixed` module: float results are not reproducible
//! across compilers/OSes, making them unusable in the lockstep simulation
//! layer (Gaffer on Games "Floating Point Determinism", arXiv determinism audits).

use crate::{
    world_hash::{DetHash, Fnv1a},
    Fixed,
};

// ---------------------------------------------------------------------------
// Vec2
// ---------------------------------------------------------------------------

/// A 2-D vector of [`Fixed`]-point components.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Vec2 {
    pub x: Fixed,
    pub y: Fixed,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 {
        x: Fixed::ZERO,
        y: Fixed::ZERO,
    };

    #[inline]
    pub const fn new(x: Fixed, y: Fixed) -> Self {
        Vec2 { x, y }
    }

    /// Dot product: `x₁·x₂ + y₁·y₂`.
    #[inline]
    pub fn dot(self, rhs: Vec2) -> Fixed {
        self.x.mul(rhs.x) + self.y.mul(rhs.y)
    }

    /// Squared length: `x² + y²`. Stays in Q16.16; can saturate for large
    /// vectors but never wraps.
    #[inline]
    pub fn len_sq(self) -> Fixed {
        self.dot(self)
    }

    /// Length: `√(x² + y²)`. Uses integer `isqrt` internally; the result is
    /// floored to the nearest Q16.16 representable value.
    #[inline]
    pub fn len(self) -> Fixed {
        self.len_sq().sqrt()
    }

    /// Scale by a scalar: `(x·s, y·s)`.
    #[inline]
    pub fn scale(self, s: Fixed) -> Vec2 {
        Vec2 {
            x: self.x.mul(s),
            y: self.y.mul(s),
        }
    }

    /// Returns the unit vector in the same direction, or `None` if this vector
    /// is zero (to avoid a divide-by-zero producing garbage). The caller
    /// decides how to handle the degenerate case so no panic occurs.
    pub fn normalize(self) -> Option<Vec2> {
        let l = self.len();
        if l == Fixed::ZERO {
            return None;
        }
        Some(Vec2 {
            x: self.x.div(l),
            y: self.y.div(l),
        })
    }

    /// Perpendicular vector (rotate 90° CCW): `(-y, x)`.
    #[inline]
    pub fn perp(self) -> Vec2 {
        Vec2 {
            x: -self.y,
            y: self.x,
        }
    }

    /// Rotate counter-clockwise by `angle` radians using the fixed-point CORDIC
    /// [`Fixed::sin_cos`] — no float, deterministic across targets. Applies the
    /// standard 2-D rotation matrix `(x·cos − y·sin, x·sin + y·cos)`.
    pub fn rotate(self, angle: Fixed) -> Vec2 {
        let (sin, cos) = angle.sin_cos();
        Vec2 {
            x: self.x.mul(cos) - self.y.mul(sin),
            y: self.x.mul(sin) + self.y.mul(cos),
        }
    }

    /// Angle of this vector from the +x axis, in radians within `(-π, π]`, via
    /// [`Fixed::atan2`]. The zero vector returns `0`.
    #[inline]
    pub fn angle(self) -> Fixed {
        Fixed::atan2(self.y, self.x)
    }

    /// Unit vector at `angle` radians from the +x axis — the inverse of `angle()`.
    /// Uses CORDIC `sin_cos` so it is float-free and deterministic. The result
    /// has length ≈ 1 (within the Q16.16 rounding of `sin_cos`). Useful for
    /// "aim turret" and "spawn projectile" operations.
    #[inline]
    pub fn from_angle(angle: Fixed) -> Vec2 {
        let (sin, cos) = angle.sin_cos();
        Vec2 { x: cos, y: sin }
    }

    /// Component-wise linear interpolation: `a + (b − a)·t`. Mirrors
    /// [`Fixed::lerp`]; `t` is typically in `[0, 1]` but is not clamped.
    #[inline]
    pub fn lerp(a: Vec2, b: Vec2, t: Fixed) -> Vec2 {
        Vec2 {
            x: Fixed::lerp(a.x, b.x, t),
            y: Fixed::lerp(a.y, b.y, t),
        }
    }

    /// Component-wise absolute value: `(|x|, |y|)`.
    #[inline]
    pub fn abs(self) -> Vec2 {
        Vec2 {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Returns `true` when both components are zero.
    #[inline]
    pub fn is_zero(self) -> bool {
        self.x.is_zero() && self.y.is_zero()
    }

    /// Component-wise minimum: `(min(a.x, b.x), min(a.y, b.y))`.
    #[inline]
    pub fn min(a: Vec2, b: Vec2) -> Vec2 {
        Vec2 {
            x: if a.x <= b.x { a.x } else { b.x },
            y: if a.y <= b.y { a.y } else { b.y },
        }
    }

    /// Component-wise maximum: `(max(a.x, b.x), max(a.y, b.y))`.
    #[inline]
    pub fn max(a: Vec2, b: Vec2) -> Vec2 {
        Vec2 {
            x: if a.x >= b.x { a.x } else { b.x },
            y: if a.y >= b.y { a.y } else { b.y },
        }
    }

    /// Component-wise clamp: each component clamped to `[lo, hi]`.
    #[inline]
    pub fn clamp(self, lo: Vec2, hi: Vec2) -> Vec2 {
        Vec2::min(Vec2::max(self, lo), hi)
    }

    /// Squared distance to `rhs`: `(self − rhs).len_sq()`. Cheaper than
    /// [`distance`](Self::distance) when only comparing ranges.
    #[inline]
    pub fn distance_sq(self, rhs: Vec2) -> Fixed {
        (self - rhs).len_sq()
    }

    /// Euclidean distance to `rhs`: `(self − rhs).len()`.
    #[inline]
    pub fn distance(self, rhs: Vec2) -> Fixed {
        (self - rhs).len()
    }

    /// Reflect this vector off a surface with the given unit `normal`.
    ///
    /// `reflect(n) = self − 2·(self·n)·n`
    ///
    /// The component along `normal` is negated; the perpendicular component is
    /// preserved. Assumes `normal` is already unit length — non-unit normals
    /// produce geometrically incorrect results (scale the reflection).
    pub fn reflect(self, normal: Vec2) -> Vec2 {
        let two_dot = self.dot(normal).mul(Fixed::from_int(2));
        self - normal.scale(two_dot)
    }
}

impl core::ops::Add for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl core::ops::Sub for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl core::ops::Neg for Vec2 {
    type Output = Vec2;
    #[inline]
    fn neg(self) -> Vec2 {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl DetHash for Vec2 {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.x.det_hash(hasher);
        self.y.det_hash(hasher);
    }
}

// ---------------------------------------------------------------------------
// Vec3
// ---------------------------------------------------------------------------

/// A 3-D vector of [`Fixed`]-point components.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Vec3 {
    pub x: Fixed,
    pub y: Fixed,
    pub z: Fixed,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: Fixed::ZERO,
        y: Fixed::ZERO,
        z: Fixed::ZERO,
    };

    #[inline]
    pub const fn new(x: Fixed, y: Fixed, z: Fixed) -> Self {
        Vec3 { x, y, z }
    }

    /// Dot product: `x₁·x₂ + y₁·y₂ + z₁·z₂`.
    #[inline]
    pub fn dot(self, rhs: Vec3) -> Fixed {
        self.x.mul(rhs.x) + self.y.mul(rhs.y) + self.z.mul(rhs.z)
    }

    /// Cross product: `(y₁z₂ - z₁y₂, z₁x₂ - x₁z₂, x₁y₂ - y₁x₂)`.
    #[inline]
    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.y.mul(rhs.z) - self.z.mul(rhs.y),
            y: self.z.mul(rhs.x) - self.x.mul(rhs.z),
            z: self.x.mul(rhs.y) - self.y.mul(rhs.x),
        }
    }

    /// Squared length: `x² + y² + z²`.
    #[inline]
    pub fn len_sq(self) -> Fixed {
        self.dot(self)
    }

    /// Length: `√(x² + y² + z²)`.
    #[inline]
    pub fn len(self) -> Fixed {
        self.len_sq().sqrt()
    }

    /// Scale by a scalar.
    #[inline]
    pub fn scale(self, s: Fixed) -> Vec3 {
        Vec3 {
            x: self.x.mul(s),
            y: self.y.mul(s),
            z: self.z.mul(s),
        }
    }

    /// Unit vector, or `None` for the zero vector.
    pub fn normalize(self) -> Option<Vec3> {
        let l = self.len();
        if l == Fixed::ZERO {
            return None;
        }
        Some(Vec3 {
            x: self.x.div(l),
            y: self.y.div(l),
            z: self.z.div(l),
        })
    }

    /// Component-wise linear interpolation: `a + (b − a)·t`. Mirrors
    /// [`Fixed::lerp`]; `t` is typically in `[0, 1]` but is not clamped.
    #[inline]
    pub fn lerp(a: Vec3, b: Vec3, t: Fixed) -> Vec3 {
        Vec3 {
            x: Fixed::lerp(a.x, b.x, t),
            y: Fixed::lerp(a.y, b.y, t),
            z: Fixed::lerp(a.z, b.z, t),
        }
    }

    /// Project to a [`Vec2`] by dropping `z`.
    #[inline]
    pub fn xy(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }

    /// Component-wise absolute value.
    #[inline]
    pub fn abs(self) -> Vec3 {
        Vec3 {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }

    /// Component-wise minimum.
    #[inline]
    pub fn min(a: Vec3, b: Vec3) -> Vec3 {
        Vec3 {
            x: if a.x <= b.x { a.x } else { b.x },
            y: if a.y <= b.y { a.y } else { b.y },
            z: if a.z <= b.z { a.z } else { b.z },
        }
    }

    /// Component-wise maximum.
    #[inline]
    pub fn max(a: Vec3, b: Vec3) -> Vec3 {
        Vec3 {
            x: if a.x >= b.x { a.x } else { b.x },
            y: if a.y >= b.y { a.y } else { b.y },
            z: if a.z >= b.z { a.z } else { b.z },
        }
    }

    /// Component-wise clamp to `[lo, hi]`.
    #[inline]
    pub fn clamp(self, lo: Vec3, hi: Vec3) -> Vec3 {
        Vec3::min(Vec3::max(self, lo), hi)
    }

    /// Squared distance to `rhs`: `(self − rhs).len_sq()`.
    #[inline]
    pub fn distance_sq(self, rhs: Vec3) -> Fixed {
        (self - rhs).len_sq()
    }

    /// Euclidean distance to `rhs`: `(self − rhs).len()`.
    #[inline]
    pub fn distance(self, rhs: Vec3) -> Fixed {
        (self - rhs).len()
    }
}

impl core::ops::Add for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl core::ops::Sub for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl core::ops::Neg for Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl DetHash for Vec3 {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.x.det_hash(hasher);
        self.y.det_hash(hasher);
        self.z.det_hash(hasher);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Fixed;

    fn fi(n: i32) -> Fixed {
        Fixed::from_int(n)
    }
    fn fr(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    // --- Vec2 ---------------------------------------------------------------

    #[test]
    fn test_vec2_add_sub() {
        let a = Vec2::new(fi(3), fi(4));
        let b = Vec2::new(fi(1), fi(-2));
        assert_eq!(a + b, Vec2::new(fi(4), fi(2)));
        assert_eq!(a - b, Vec2::new(fi(2), fi(6)));
    }

    #[test]
    fn test_vec2_neg() {
        let v = Vec2::new(fi(2), fi(-5));
        assert_eq!(-v, Vec2::new(fi(-2), fi(5)));
    }

    #[test]
    fn test_vec2_dot() {
        // (3,4)·(1,2) = 11
        let a = Vec2::new(fi(3), fi(4));
        let b = Vec2::new(fi(1), fi(2));
        assert_eq!(a.dot(b), fi(11));
    }

    #[test]
    fn test_vec2_len_3_4_is_5() {
        let v = Vec2::new(fi(3), fi(4));
        assert_eq!(v.len(), fi(5));
    }

    #[test]
    fn test_vec2_len_sq() {
        let v = Vec2::new(fi(3), fi(4));
        assert_eq!(v.len_sq(), fi(25));
    }

    #[test]
    fn test_vec2_scale() {
        let v = Vec2::new(fi(2), fi(3));
        assert_eq!(v.scale(fi(4)), Vec2::new(fi(8), fi(12)));
    }

    #[test]
    fn test_vec2_normalize_unit_length() {
        let v = Vec2::new(fi(3), fi(4));
        let u = v.normalize().unwrap();
        // |u| should be ≈1; allow a few Q16.16 LSBs for isqrt rounding.
        let one_sq = u.len_sq();
        let diff = (one_sq.raw() - Fixed::ONE.raw()).abs();
        assert!(diff < 512, "normalized len² ≈ 1, got raw diff {diff}");
    }

    #[test]
    fn test_vec2_normalize_zero_returns_none() {
        assert_eq!(Vec2::ZERO.normalize(), None);
    }

    #[test]
    fn test_vec2_perp_is_orthogonal() {
        let v = Vec2::new(fi(3), fi(4));
        let p = v.perp();
        assert_eq!(v.dot(p), Fixed::ZERO);
    }

    #[test]
    fn test_vec2_rotate_zero_is_near_identity() {
        // CORDIC sin_cos(0) ≈ (0, 1) to a few LSBs, so a zero rotation returns
        // the same vector within CORDIC precision (not bit-exact).
        let v = Vec2::new(fi(3), fi(-4));
        let r = v.rotate(Fixed::ZERO);
        let dx = (r.x.raw() - v.x.raw()).abs();
        let dy = (r.y.raw() - v.y.raw()).abs();
        assert!(
            dx < 600 && dy < 600,
            "rotate(0)≈identity, got ({dx},{dy}) raw"
        );
    }

    #[test]
    fn test_vec2_rotate_quarter_turn_matches_perp() {
        // Rotating +x by ~π/2 should land near +y, i.e. near perp().
        let quarter = fr(355, 226); // ≈ π/2
        let v = Vec2::new(fi(1), fi(0));
        let r = v.rotate(quarter);
        let p = v.perp(); // (0, 1)
        let dx = (r.x.raw() - p.x.raw()).abs();
        let dy = (r.y.raw() - p.y.raw()).abs();
        assert!(dx < 600 && dy < 600, "rotate≈perp, got ({dx},{dy}) raw");
    }

    #[test]
    fn test_vec2_rotate_preserves_length() {
        let v = Vec2::new(fi(3), fi(4)); // len² = 25
        let r = v.rotate(fr(1, 2)); // 0.5 rad
        let diff = (r.len_sq().raw() - fi(25).raw()).abs();
        // Allow CORDIC/mul rounding (a handful of LSBs scaled by the magnitude).
        assert!(
            diff < 4096,
            "rotation should preserve length², raw diff {diff}"
        );
    }

    #[test]
    fn test_vec2_angle_round_trips() {
        // +x axis → angle ≈ 0.
        assert!(Vec2::new(fi(1), fi(0)).angle().raw().abs() < 600);
        // +y axis → angle ≈ π/2 (raw ≈ 102944).
        let a = Vec2::new(fi(0), fi(1)).angle().raw();
        assert!((a - 102_944).abs() < 600, "angle(+y)≈π/2, got raw {a}");
    }

    #[test]
    fn test_vec2_rotate_then_angle_adds() {
        // Rotating +x by θ should give a vector whose angle ≈ θ.
        let theta = fr(1, 2); // 0.5 rad
        let r = Vec2::new(fi(1), fi(0)).rotate(theta);
        let diff = (r.angle().raw() - theta.raw()).abs();
        assert!(diff < 600, "angle after rotate ≈ θ, raw diff {diff}");
    }

    #[test]
    fn test_vec2_lerp_endpoints_and_midpoint() {
        let a = Vec2::new(fi(0), fi(10));
        let b = Vec2::new(fi(10), fi(0));
        assert_eq!(Vec2::lerp(a, b, Fixed::ZERO), a);
        assert_eq!(Vec2::lerp(a, b, Fixed::ONE), b);
        assert_eq!(Vec2::lerp(a, b, fr(1, 2)), Vec2::new(fi(5), fi(5)));
    }

    #[test]
    fn test_vec2_distance_3_4_5() {
        let a = Vec2::new(fi(1), fi(1));
        let b = Vec2::new(fi(4), fi(5)); // dx=3, dy=4
        assert_eq!(a.distance(b), fi(5));
        assert_eq!(a.distance_sq(b), fi(25));
    }

    #[test]
    fn test_vec2_det_hash_changes_on_mutation() {
        use crate::world_hash::hash_state;
        let a = Vec2::new(fi(1), fi(2));
        let b = Vec2::new(fi(1), fi(3));
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_vec2_saturates_not_wraps() {
        let big = Vec2::new(Fixed::MAX, Fixed::MAX);
        let one = Vec2::new(fi(1), fi(1));
        let r = big + one;
        assert_eq!(r.x.raw(), i32::MAX, "must saturate not wrap");
    }

    // --- Vec3 ---------------------------------------------------------------

    #[test]
    fn test_vec3_add_sub() {
        let a = Vec3::new(fi(1), fi(2), fi(3));
        let b = Vec3::new(fi(4), fi(-1), fi(2));
        assert_eq!(a + b, Vec3::new(fi(5), fi(1), fi(5)));
        assert_eq!(a - b, Vec3::new(fi(-3), fi(3), fi(1)));
    }

    #[test]
    fn test_vec3_dot() {
        let a = Vec3::new(fi(1), fi(2), fi(3));
        let b = Vec3::new(fi(4), fi(5), fi(6));
        // 1*4 + 2*5 + 3*6 = 32
        assert_eq!(a.dot(b), fi(32));
    }

    #[test]
    fn test_vec3_cross_standard_basis() {
        let ex = Vec3::new(fi(1), fi(0), fi(0));
        let ey = Vec3::new(fi(0), fi(1), fi(0));
        let ez = Vec3::new(fi(0), fi(0), fi(1));
        // ex × ey = ez
        assert_eq!(ex.cross(ey), ez);
        // ey × ez = ex
        assert_eq!(ey.cross(ez), ex);
        // ez × ex = ey
        assert_eq!(ez.cross(ex), ey);
    }

    #[test]
    fn test_vec3_cross_anticommutative() {
        let a = Vec3::new(fi(2), fi(3), fi(4));
        let b = Vec3::new(fi(5), fi(6), fi(7));
        assert_eq!(a.cross(b), -b.cross(a));
    }

    #[test]
    fn test_vec3_len_sq() {
        let v = Vec3::new(fi(1), fi(2), fi(2));
        // 1+4+4 = 9
        assert_eq!(v.len_sq(), fi(9));
    }

    #[test]
    fn test_vec3_len_pythagorean() {
        let v = Vec3::new(fi(1), fi(2), fi(2));
        assert_eq!(v.len(), fi(3));
    }

    #[test]
    fn test_vec3_normalize_unit_length() {
        let v = Vec3::new(fi(1), fi(2), fi(2)); // len = 3
        let u = v.normalize().unwrap();
        let one_sq = u.len_sq();
        let diff = (one_sq.raw() - Fixed::ONE.raw()).abs();
        assert!(diff < 512, "normalized len² ≈ 1, diff = {diff}");
    }

    #[test]
    fn test_vec3_normalize_zero_returns_none() {
        assert_eq!(Vec3::ZERO.normalize(), None);
    }

    #[test]
    fn test_vec3_det_hash_changes_on_mutation() {
        use crate::world_hash::hash_state;
        let a = Vec3::new(fi(1), fi(2), fi(3));
        let b = Vec3::new(fi(1), fi(2), fi(4));
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_vec3_lerp_endpoints_and_midpoint() {
        let a = Vec3::new(fi(0), fi(10), fi(2));
        let b = Vec3::new(fi(10), fi(0), fi(6));
        assert_eq!(Vec3::lerp(a, b, Fixed::ZERO), a);
        assert_eq!(Vec3::lerp(a, b, Fixed::ONE), b);
        assert_eq!(Vec3::lerp(a, b, fr(1, 2)), Vec3::new(fi(5), fi(5), fi(4)));
    }

    #[test]
    fn test_vec3_distance() {
        // dx=2, dy=3, dz=6 → len = 7
        let a = Vec3::new(fi(0), fi(0), fi(0));
        let b = Vec3::new(fi(2), fi(3), fi(6));
        assert_eq!(a.distance(b), fi(7));
        assert_eq!(a.distance_sq(b), fi(49));
    }

    #[test]
    fn test_vec2_fractional_scale() {
        // (4, 8) * 0.5 = (2, 4)
        let v = Vec2::new(fi(4), fi(8));
        let half = fr(1, 2);
        assert_eq!(v.scale(half), Vec2::new(fi(2), fi(4)));
    }

    #[test]
    fn test_vec2_abs() {
        assert_eq!(Vec2::new(fi(-3), fi(4)).abs(), Vec2::new(fi(3), fi(4)));
        assert_eq!(Vec2::ZERO.abs(), Vec2::ZERO);
    }

    #[test]
    fn test_vec2_min_max() {
        let a = Vec2::new(fi(1), fi(5));
        let b = Vec2::new(fi(3), fi(2));
        assert_eq!(Vec2::min(a, b), Vec2::new(fi(1), fi(2)));
        assert_eq!(Vec2::max(a, b), Vec2::new(fi(3), fi(5)));
    }

    #[test]
    fn test_vec2_clamp() {
        let lo = Vec2::new(fi(0), fi(0));
        let hi = Vec2::new(fi(10), fi(10));
        assert_eq!(
            Vec2::new(fi(-1), fi(15)).clamp(lo, hi),
            Vec2::new(fi(0), fi(10))
        );
        assert_eq!(
            Vec2::new(fi(5), fi(5)).clamp(lo, hi),
            Vec2::new(fi(5), fi(5))
        );
    }

    #[test]
    fn test_vec3_xy_drops_z() {
        let v = Vec3::new(fi(1), fi(2), fi(99));
        assert_eq!(v.xy(), Vec2::new(fi(1), fi(2)));
    }

    #[test]
    fn test_vec3_abs() {
        assert_eq!(
            Vec3::new(fi(-1), fi(2), fi(-3)).abs(),
            Vec3::new(fi(1), fi(2), fi(3))
        );
    }

    #[test]
    fn test_vec3_min_max() {
        let a = Vec3::new(fi(1), fi(5), fi(3));
        let b = Vec3::new(fi(2), fi(1), fi(4));
        assert_eq!(Vec3::min(a, b), Vec3::new(fi(1), fi(1), fi(3)));
        assert_eq!(Vec3::max(a, b), Vec3::new(fi(2), fi(5), fi(4)));
    }

    #[test]
    fn test_vec3_clamp() {
        let lo = Vec3::new(fi(0), fi(0), fi(0));
        let hi = Vec3::new(fi(5), fi(5), fi(5));
        assert_eq!(
            Vec3::new(fi(-1), fi(3), fi(9)).clamp(lo, hi),
            Vec3::new(fi(0), fi(3), fi(5))
        );
    }

    #[test]
    fn test_vec2_reflect_off_floor_reverses_y() {
        // v = (1, 1), normal = (0, 1): dot = 1, 2*dot = 2
        // reflect = (1,1) - (0,2) = (1,-1)
        let v = Vec2::new(fi(1), fi(1));
        let n = Vec2::new(fi(0), fi(1));
        assert_eq!(v.reflect(n), Vec2::new(fi(1), fi(-1)));
    }

    #[test]
    fn test_vec2_reflect_perpendicular_to_normal_unchanged() {
        // v = (1, 0), normal = (0, 1): dot = 0 → reflect = v
        let v = Vec2::new(fi(1), fi(0));
        let n = Vec2::new(fi(0), fi(1));
        assert_eq!(v.reflect(n), v);
    }

    #[test]
    fn test_vec2_reflect_against_vertical_wall() {
        // v = (3, 4), normal = (1, 0): dot = 3, 2*dot = 6
        // reflect = (3,4) - (6,0) = (-3, 4)
        let v = Vec2::new(fi(3), fi(4));
        let n = Vec2::new(fi(1), fi(0));
        assert_eq!(v.reflect(n), Vec2::new(fi(-3), fi(4)));
    }

    #[test]
    fn test_from_angle_zero_is_east() {
        // angle 0 → (cos 0, sin 0) = (1, 0)
        let v = Vec2::from_angle(Fixed::ZERO);
        // CORDIC approximation: within 0.001 of the true value
        let tol = Fixed::from_ratio(1, 1000);
        assert!((v.x - fi(1)).abs() <= tol, "cos(0) ≈ 1, got {:?}", v.x);
        assert!(v.y.abs() <= tol, "sin(0) ≈ 0, got {:?}", v.y);
    }

    #[test]
    fn test_from_angle_round_trips_with_angle() {
        // from_angle(theta).angle() ≈ theta (within CORDIC error)
        let pi = Fixed::from_ratio(355, 113); // approx π
        let half_pi = pi.div(Fixed::from_int(2));
        let v = Vec2::from_angle(half_pi);
        let recovered = v.angle();
        let err = (recovered - half_pi).abs();
        // Allow tolerance of 0.01 radians (CORDIC 16-iter precision)
        assert!(err <= Fixed::from_ratio(1, 100), "err={:?}", err);
    }

    #[test]
    fn test_from_angle_preserves_unit_length() {
        let angle = Fixed::from_ratio(1, 3); // ~0.33 rad
        let v = Vec2::from_angle(angle);
        let len = v.len();
        let diff = (len - fi(1)).abs();
        assert!(diff <= Fixed::from_ratio(1, 100), "len={:?}", len);
    }

    #[test]
    fn test_is_zero_for_zero_vector() {
        assert!(Vec2::ZERO.is_zero());
    }

    #[test]
    fn test_is_zero_false_for_nonzero_x() {
        let v = Vec2 {
            x: fi(1),
            y: Fixed::ZERO,
        };
        assert!(!v.is_zero());
    }

    #[test]
    fn test_is_zero_false_for_nonzero_y() {
        let v = Vec2 {
            x: Fixed::ZERO,
            y: fi(-1),
        };
        assert!(!v.is_zero());
    }
}
