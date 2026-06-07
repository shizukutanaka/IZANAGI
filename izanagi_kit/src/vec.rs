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
            x: Fixed::ZERO - self.y,
            y: self.x,
        }
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
            x: Fixed::ZERO - self.x,
            y: Fixed::ZERO - self.y,
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
            x: Fixed::ZERO - self.x,
            y: Fixed::ZERO - self.y,
            z: Fixed::ZERO - self.z,
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
    fn test_vec2_fractional_scale() {
        // (4, 8) * 0.5 = (2, 4)
        let v = Vec2::new(fi(4), fi(8));
        let half = fr(1, 2);
        assert_eq!(v.scale(half), Vec2::new(fi(2), fi(4)));
    }
}
