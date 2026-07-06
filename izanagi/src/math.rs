//! 2D and 3D math.
//!
//! Vectors, matrices, rectangles. f32 throughout — games, not science.
//! Operators are overloaded; prefer them over method calls.

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 2D vector.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec2 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    /// Unit vector on X.
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    /// Unit vector on Y.
    pub const Y: Self = Self { x: 0.0, y: 1.0 };
    /// Unit vector (1, 1).
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    /// Construct.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Construct from a scalar.
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    /// Dot product.
    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y
    }

    /// Squared length. Prefer this for distance comparisons.
    pub fn len_sq(self) -> f32 {
        self.dot(self)
    }

    /// Length.
    pub fn len(self) -> f32 {
        self.len_sq().sqrt()
    }

    /// Unit-length version. Returns ZERO for a zero vector.
    pub fn normalize(self) -> Self {
        let l = self.len();
        if l > 1e-6 {
            self / l
        } else {
            Self::ZERO
        }
    }

    /// 2D cross / perp-dot. Signed area of the parallelogram.
    pub fn cross(self, o: Self) -> f32 {
        self.x * o.y - self.y * o.x
    }

    /// Perpendicular (90° CCW).
    pub fn perp(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    /// Linear interpolation between `self` and `to` at `t` in [0, 1].
    pub fn lerp(self, to: Self, t: f32) -> Self {
        self + (to - self) * t
    }

    /// Component-wise clamp.
    pub fn clamp(self, lo: Self, hi: Self) -> Self {
        Self {
            x: self.x.clamp(lo.x, hi.x),
            y: self.y.clamp(lo.y, hi.y),
        }
    }

    /// Distance to another point.
    pub fn distance(self, o: Self) -> f32 {
        (o - self).len()
    }

    /// Squared distance to another point. Faster than [`Vec2::distance`].
    pub fn distance_sq(self, o: Self) -> f32 {
        (o - self).len_sq()
    }

    /// Component-wise minimum.
    pub fn min(self, o: Self) -> Self {
        Self {
            x: self.x.min(o.x),
            y: self.y.min(o.y),
        }
    }

    /// Component-wise maximum.
    pub fn max(self, o: Self) -> Self {
        Self {
            x: self.x.max(o.x),
            y: self.y.max(o.y),
        }
    }

    /// Absolute value of each component.
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Component-wise floor.
    pub fn floor(self) -> Self {
        Self {
            x: self.x.floor(),
            y: self.y.floor(),
        }
    }

    /// Component-wise round.
    pub fn round(self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
        }
    }

    /// Reflect across normal `n` (must be unit length).
    pub fn reflect(self, n: Self) -> Self {
        self - n * (2.0 * self.dot(n))
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}
impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}
impl Mul<Vec2> for f32 {
    type Output = Vec2;
    fn mul(self, v: Vec2) -> Vec2 {
        v * self
    }
}
impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, s: f32) -> Self {
        Self::new(self.x / s, self.y / s)
    }
}
impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Self) {
        *self = *self + o;
    }
}
impl SubAssign for Vec2 {
    fn sub_assign(&mut self, o: Self) {
        *self = *self - o;
    }
}
impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s;
    }
}

/// A 3D vector.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec3 {
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
    /// Z.
    pub z: f32,
}

impl Vec3 {
    /// Zero.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    /// Unit X.
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    /// Unit Y.
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    /// Unit Z.
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    /// Construct.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Dot product.
    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// Cross product.
    pub fn cross(self, o: Self) -> Self {
        Self::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    /// Length.
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Unit-length version. Returns ZERO for a zero vector.
    pub fn normalize(self) -> Self {
        let l = self.len();
        if l > 1e-6 {
            self / l
        } else {
            Self::ZERO
        }
    }

    /// Linear interpolation.
    pub fn lerp(self, to: Self, t: f32) -> Self {
        self + (to - self) * t
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, s: f32) -> Self {
        Self::new(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Self) {
        *self = *self + o;
    }
}

/// An axis-aligned rectangle (2D box).
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Rect {
    /// Lower-left X.
    pub x: f32,
    /// Lower-left Y.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Construct.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Is a point inside this rect?
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }

    /// Does this rect overlap another (AABB test)?
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && self.x + self.w > o.x && self.y < o.y + o.h && self.y + self.h > o.y
    }

    /// Center of the rect.
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

/// 3x3 matrix for 2D transforms (column-major).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Mat3 {
    /// Elements in column-major order.
    pub m: [f32; 9],
}

impl Mat3 {
    /// Identity matrix.
    pub const IDENTITY: Self = Self {
        m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    };

    /// Translation.
    pub fn translation(t: Vec2) -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, t.x, t.y, 1.0],
        }
    }

    /// Uniform scale.
    pub fn scale(s: Vec2) -> Self {
        Self {
            m: [s.x, 0.0, 0.0, 0.0, s.y, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Rotation (radians, CCW).
    pub fn rotation(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self {
            m: [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Transform a 2D point.
    pub fn transform_point(&self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.m[0] * p.x + self.m[3] * p.y + self.m[6],
            self.m[1] * p.x + self.m[4] * p.y + self.m[7],
        )
    }
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul for Mat3 {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        let mut r = [0.0; 9];
        for c in 0..3 {
            for i in 0..3 {
                r[c * 3 + i] = self.m[i] * o.m[c * 3]
                    + self.m[3 + i] * o.m[c * 3 + 1]
                    + self.m[6 + i] * o.m[c * 3 + 2];
            }
        }
        Self { m: r }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec2_arith() {
        let a = Vec2::new(3.0, 4.0);
        assert_eq!(a.len(), 5.0);
        assert_eq!((a * 2.0).x, 6.0);
        assert_eq!((-a).x, -3.0);
        assert!(a.normalize().len() > 0.999);
    }

    #[test]
    fn vec2_distance_sq_matches_distance_squared() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(3.0, 4.0);
        assert!((a.distance_sq(b) - 25.0).abs() < 1e-5);
        assert!((a.distance(b) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn vec2_min_max() {
        let a = Vec2::new(1.0, 5.0);
        let b = Vec2::new(3.0, 2.0);
        assert_eq!(a.min(b), Vec2::new(1.0, 2.0));
        assert_eq!(a.max(b), Vec2::new(3.0, 5.0));
    }

    #[test]
    fn vec2_abs_negative_components() {
        assert_eq!(Vec2::new(-3.0, -4.0).abs(), Vec2::new(3.0, 4.0));
    }

    #[test]
    fn vec2_floor_round() {
        assert_eq!(Vec2::new(1.7, -1.3).floor(), Vec2::new(1.0, -2.0));
        assert_eq!(Vec2::new(1.4, 1.6).round(), Vec2::new(1.0, 2.0));
    }

    #[test]
    fn vec2_reflect_horizontal_floor() {
        // Ball moving down-right reflects off horizontal floor.
        let v = Vec2::new(1.0, -1.0);
        let n = Vec2::new(0.0, 1.0);
        let r = v.reflect(n);
        assert!((r.x - 1.0).abs() < 1e-5);
        assert!((r.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn vec2_lerp_endpoints() {
        let a = Vec2::ZERO;
        let b = Vec2::new(10.0, 10.0);
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
        assert_eq!(a.lerp(b, 0.5), Vec2::new(5.0, 5.0));
    }

    #[test]
    fn vec2_zero_normalize() {
        assert_eq!(Vec2::ZERO.normalize(), Vec2::ZERO);
    }

    #[test]
    fn vec3_cross() {
        assert_eq!(Vec3::X.cross(Vec3::Y), Vec3::Z);
        assert_eq!(Vec3::Y.cross(Vec3::Z), Vec3::X);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(Vec2::new(5.0, 5.0)));
        assert!(!r.contains(Vec2::new(11.0, 5.0)));
    }

    #[test]
    fn rect_overlaps() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        let c = Rect::new(100.0, 100.0, 1.0, 1.0);
        assert!(a.overlaps(&b));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn mat3_identity_is_neutral() {
        let p = Vec2::new(3.0, 5.0);
        assert_eq!(Mat3::IDENTITY.transform_point(p), p);
    }

    #[test]
    fn mat3_translation_then_scale() {
        let t = Mat3::translation(Vec2::new(10.0, 0.0));
        let p = t.transform_point(Vec2::new(1.0, 2.0));
        assert_eq!(p, Vec2::new(11.0, 2.0));
    }

    #[test]
    fn mat3_rotation_90_deg() {
        let r = Mat3::rotation(std::f32::consts::FRAC_PI_2);
        let p = r.transform_point(Vec2::X);
        assert!((p.x).abs() < 1e-5);
        assert!((p.y - 1.0).abs() < 1e-5);
    }
}
