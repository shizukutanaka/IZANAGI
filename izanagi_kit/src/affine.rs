//! 2-D affine transforms on `Fixed` — the `[a b c; d e f; 0 0 1]`
//! matrix familiar from PostScript/SVG/Canvas (`transform(a,b,c,d,e,f)`),
//! composed and applied with `O(1)` integer math. Chains of
//! translate/rotate/scale/shear build up a single matrix that maps
//! sprites, camera shakes, and UI layers without per-point work.
//!
//! ```
//! use izanagi_kit::affine::Affine;
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::vec::Vec2;
//!
//! let m = Affine::translate(Fixed::from_int(3), Fixed::from_int(4));
//! let p = m.apply(Vec2::new(Fixed::from_int(1), Fixed::from_int(2)));
//! assert_eq!(p, Vec2::new(Fixed::from_int(4), Fixed::from_int(6)));
//! ```

use crate::fixed::Fixed;
use crate::vec::Vec2;

/// `[a b c; d e f; 0 0 1]` — `apply` maps `(x,y)` to
/// `(a·x + b·y + c, d·x + e·y + f)`. Composition uses row-vector
/// convention: `self.then(other)` applies `self` first, then `other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Affine {
    /// X basis row: `x' = a·x + b·y + c`.
    pub a: Fixed,
    /// X basis shear term.
    pub b: Fixed,
    /// X translation term.
    pub c: Fixed,
    /// Y basis row: `y' = d·x + e·y + f`.
    pub d: Fixed,
    /// Y basis shear term.
    pub e: Fixed,
    /// Y translation term.
    pub f: Fixed,
}

impl Affine {
    /// The identity transform.
    pub const IDENTITY: Self = Self {
        a: Fixed::ONE,
        b: Fixed::ZERO,
        c: Fixed::ZERO,
        d: Fixed::ZERO,
        e: Fixed::ONE,
        f: Fixed::ZERO,
    };

    /// Raw constructor for `[a b c; d e f; 0 0 1]`.
    pub const fn new(a: Fixed, b: Fixed, c: Fixed, d: Fixed, e: Fixed, f: Fixed) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Translation by `(tx, ty)`.
    pub const fn translate(tx: Fixed, ty: Fixed) -> Self {
        Self::new(Fixed::ONE, Fixed::ZERO, tx, Fixed::ZERO, Fixed::ONE, ty)
    }

    /// Non-uniform scale `(sx, sy)`; uniform when `sx == sy`.
    pub const fn scale(sx: Fixed, sy: Fixed) -> Self {
        Self::new(sx, Fixed::ZERO, Fixed::ZERO, Fixed::ZERO, sy, Fixed::ZERO)
    }

    /// Shear: `x' = x + kx·y`, `y' = ky·x + y`.
    pub const fn shear(kx: Fixed, ky: Fixed) -> Self {
        Self::new(Fixed::ONE, kx, Fixed::ZERO, ky, Fixed::ONE, Fixed::ZERO)
    }

    /// Rotation by `theta` radians (CCW, `Fixed::sin_cos` precision).
    pub fn rotate(theta: Fixed) -> Self {
        let (s, c) = theta.sin_cos();
        Self::new(c, -s, Fixed::ZERO, s, c, Fixed::ZERO)
    }

    /// Rotation about `pivot`: `T(p)·R·T(−p)` — translate by `−p`
    /// first, rotate, translate back.
    pub fn rotate_about(theta: Fixed, pivot: Vec2) -> Self {
        Self::translate(-pivot.x, -pivot.y)
            .then(Self::rotate(theta))
            .then(Self::translate(pivot.x, pivot.y))
    }

    /// Composition: `self.then(other)` first applies `self`, then
    /// `other` — i.e. `other·self` as a matrix product.
    pub fn then(self, o: Self) -> Self {
        Self {
            a: o.a.mul(self.a) + o.b.mul(self.d),
            b: o.a.mul(self.b) + o.b.mul(self.e),
            c: o.a.mul(self.c) + o.b.mul(self.f) + o.c,
            d: o.d.mul(self.a) + o.e.mul(self.d),
            e: o.d.mul(self.b) + o.e.mul(self.e),
            f: o.d.mul(self.c) + o.e.mul(self.f) + o.f,
        }
    }

    /// Apply to a point: `p ↦ M·p` including translation.
    pub fn apply(self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.a.mul(p.x) + self.b.mul(p.y) + self.c,
            self.d.mul(p.x) + self.e.mul(p.y) + self.f,
        )
    }

    /// Apply to a direction/displacement: linear part only, no
    /// translation — the right map for velocities and offsets.
    pub fn apply_delta(self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.a.mul(p.x) + self.b.mul(p.y),
            self.d.mul(p.x) + self.e.mul(p.y),
        )
    }

    /// Determinant `a·e − b·d` (the `|linear|` factor of the map).
    pub fn det(self) -> Fixed {
        self.a.mul(self.e) - self.b.mul(self.d)
    }

    /// Whether the linear part is invertible.
    pub fn is_invertible(self) -> bool {
        !self.det().is_zero()
    }

    /// Matrix inverse via the adjugate: `1/det · [[e, −b, bf−ec],
    /// [−d, a, dc−af]]`. `None` when `det == 0`; `Fixed::div` truncates,
    /// so `m.inverse().apply(m.apply(p))` can differ from `p` by a
    /// couple of ulps.
    pub fn invert(self) -> Option<Self> {
        let det = self.det();
        if det.is_zero() {
            return None;
        }
        Some(Self {
            a: self.e.div(det),
            b: (-self.b).div(det),
            c: (self.b.mul(self.f) - self.e.mul(self.c)).div(det),
            d: (-self.d).div(det),
            e: self.a.div(det),
            f: (self.d.mul(self.c) - self.a.mul(self.f)).div(det),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: i32) -> Fixed {
        Fixed::from_int(v)
    }
    fn v(x: i32, y: i32) -> Vec2 {
        Vec2::new(f(x), f(y))
    }

    #[test]
    fn identity_and_translation() {
        let p = v(3, -2);
        assert_eq!(Affine::IDENTITY.apply(p), p);
        assert_eq!(Affine::translate(f(5), f(-1)).apply(p), v(8, -3));
        assert_eq!(Affine::translate(f(5), f(0)).apply_delta(p), p);
    }

    #[test]
    fn compose_matches_matrix_product_oracle() {
        // f64 oracle: then() on m then n == apply(n(apply(m(p)))).
        let m = Affine::translate(f(2), f(1)).then(Affine::scale(f(3), f(2)));
        let p = v(4, 5);
        // m: translate(2,1) then scale(3,2) → p ↦ ((4+2)*3, (5+1)*2) = (18,12)
        assert_eq!(m.apply(p), v(18, 12));
        let step = Affine::translate(f(2), f(1)).apply(p);
        assert_eq!(Affine::scale(f(3), f(2)).apply(step), m.apply(p));
    }

    #[test]
    fn rotation_quarter_turns_and_pivot() {
        // Quarter-turn: sin(π/2)=1 exactly; cos(π/2) is a few ulps off
        // zero in fixed point — assert within raw slack.
        let q = Affine::rotate(Fixed::HALF_PI);
        let r = q.apply(v(1, 0));
        assert!(r.x.raw().abs() < 64, "{r:?}");
        assert_eq!(r.y, Fixed::ONE);
        // Four quarter-turns ≈ identity within accumulated ulps.
        let four = q.then(q).then(q).then(q);
        let p = v(7, -3);
        let rt = four.apply(p);
        assert!((rt.x - p.x).raw().abs() < 512, "{rt:?}");
        assert!((rt.y - p.y).raw().abs() < 512, "{rt:?}");
        // Pivot rotation about (1,1): the pivot is a fixed point.
        let about = Affine::rotate_about(Fixed::HALF_PI, v(1, 1));
        let piv = about.apply(v(1, 1));
        assert!((piv.x - Fixed::ONE).raw().abs() < 64, "{piv:?}");
        assert!((piv.y - Fixed::ONE).raw().abs() < 64, "{piv:?}");
    }

    #[test]
    fn inverse_roundtrips_and_singular_is_none() {
        let m = Affine::translate(f(3), f(-2))
            .then(Affine::scale(f(2), f(4)))
            .then(Affine::shear(f(0), f(1)));
        let inv = m.invert().unwrap();
        // det of product = product of dets: 1·(2·4)·1 = 8.
        assert_eq!(m.det(), f(8));
        // inv∘m should be ≈ identity: apply to a point and compare raw
        // (division truncation can cost a few ulps, so allow slack).
        let p = v(6, 1);
        let rt = inv.apply(m.apply(p));
        assert!((rt.x - p.x).abs().raw().abs() < 256, "{rt:?}");
        assert!((rt.y - p.y).abs().raw().abs() < 256, "{rt:?}");
        assert_eq!(Affine::scale(Fixed::ZERO, f(2)).invert(), None);
        assert!(!Affine::scale(Fixed::ZERO, f(2)).is_invertible());
        assert!(m.is_invertible());
        // Raw constructor composes identically to the named helpers.
        let raw = Affine::new(
            f(2),
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            f(2),
            Fixed::ZERO,
        );
        assert_eq!(raw, Affine::scale(f(2), f(2)));
    }

    #[test]
    fn shear_moves_one_axis_only() {
        let s = Affine::shear(f(2), Fixed::ZERO);
        // x' = x + 2y: (1,1) → (3,1).
        assert_eq!(s.apply(v(1, 1)), v(3, 1));
        assert_eq!(s.apply(v(5, 0)), v(5, 0));
    }

    #[test]
    fn deterministic_twice() {
        let m = Affine::rotate(Fixed::from_ratio(1, 3)).then(Affine::translate(f(9), f(0)));
        let p = v(4, 7);
        assert_eq!(m.apply(p), m.apply(p));
        assert_eq!(m.invert(), m.invert());
    }
}
