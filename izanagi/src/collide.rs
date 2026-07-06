//! 2D collision detection.
//!
//! AABB-vs-AABB, swept AABB, circle, ray. Returns hit info, never panics.
//! All shapes use [`crate::Vec2`] and [`crate::Rect`] from [`crate::math`].

use crate::math::{Rect, Vec2};

/// Result of a swept collision.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Hit {
    /// Time of impact in `[0, 1]` along the motion vector.
    pub t: f32,
    /// Surface normal at the impact point.
    pub normal: Vec2,
}

/// Standard AABB overlap test.
pub fn aabb_vs_aabb(a: &Rect, b: &Rect) -> bool {
    a.overlaps(b)
}

/// Sweep moving rect `a` by `motion` against static rect `b`.
/// Returns the first hit, or `None` if no collision.
pub fn swept_aabb(a: &Rect, motion: Vec2, b: &Rect) -> Option<Hit> {
    // Compute entry / exit times along each axis.
    let (inv_x, inv_y);
    inv_x = if motion.x.abs() < f32::EPSILON {
        f32::INFINITY
    } else {
        1.0 / motion.x
    };
    inv_y = if motion.y.abs() < f32::EPSILON {
        f32::INFINITY
    } else {
        1.0 / motion.y
    };

    let (mut tx_entry, mut tx_exit);
    if motion.x > 0.0 {
        tx_entry = (b.x - (a.x + a.w)) * inv_x;
        tx_exit = ((b.x + b.w) - a.x) * inv_x;
    } else {
        tx_entry = ((b.x + b.w) - a.x) * inv_x;
        tx_exit = (b.x - (a.x + a.w)) * inv_x;
    }
    if motion.x.abs() < f32::EPSILON {
        // Stationary on X — entry must already overlap.
        if a.x + a.w <= b.x || a.x >= b.x + b.w {
            return None;
        }
        tx_entry = -f32::INFINITY;
        tx_exit = f32::INFINITY;
    }

    let (mut ty_entry, mut ty_exit);
    if motion.y > 0.0 {
        ty_entry = (b.y - (a.y + a.h)) * inv_y;
        ty_exit = ((b.y + b.h) - a.y) * inv_y;
    } else {
        ty_entry = ((b.y + b.h) - a.y) * inv_y;
        ty_exit = (b.y - (a.y + a.h)) * inv_y;
    }
    if motion.y.abs() < f32::EPSILON {
        if a.y + a.h <= b.y || a.y >= b.y + b.h {
            return None;
        }
        ty_entry = -f32::INFINITY;
        ty_exit = f32::INFINITY;
    }

    let entry = tx_entry.max(ty_entry);
    let exit = tx_exit.min(ty_exit);

    if entry > exit || (tx_entry < 0.0 && ty_entry < 0.0) || entry > 1.0 {
        return None;
    }

    let normal = if tx_entry > ty_entry {
        Vec2::new(if motion.x > 0.0 { -1.0 } else { 1.0 }, 0.0)
    } else {
        Vec2::new(0.0, if motion.y > 0.0 { -1.0 } else { 1.0 })
    };
    Some(Hit {
        t: entry.max(0.0),
        normal,
    })
}

/// Point inside a circle?
pub fn point_in_circle(p: Vec2, center: Vec2, radius: f32) -> bool {
    (p - center).len_sq() <= radius * radius
}

/// Two circles overlapping?
pub fn circle_vs_circle(a: Vec2, ar: f32, b: Vec2, br: f32) -> bool {
    let r = ar + br;
    (a - b).len_sq() <= r * r
}

/// Ray vs AABB. `dir` does not need to be unit length.
/// Returns hit `t` (multiply by `dir` to get the point) or `None`.
pub fn ray_vs_aabb(origin: Vec2, dir: Vec2, b: &Rect) -> Option<f32> {
    let inv_x = if dir.x.abs() < f32::EPSILON {
        f32::INFINITY
    } else {
        1.0 / dir.x
    };
    let inv_y = if dir.y.abs() < f32::EPSILON {
        f32::INFINITY
    } else {
        1.0 / dir.y
    };

    let tx1 = (b.x - origin.x) * inv_x;
    let tx2 = (b.x + b.w - origin.x) * inv_x;
    let ty1 = (b.y - origin.y) * inv_y;
    let ty2 = (b.y + b.h - origin.y) * inv_y;

    let tmin = tx1.min(tx2).max(ty1.min(ty2));
    let tmax = tx1.max(tx2).min(ty1.max(ty2));

    if tmax < 0.0 || tmin > tmax {
        None
    } else {
        Some(tmin.max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_overlap_basic() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        let c = Rect::new(20.0, 20.0, 1.0, 1.0);
        assert!(aabb_vs_aabb(&a, &b));
        assert!(!aabb_vs_aabb(&a, &c));
    }

    #[test]
    fn swept_horizontal_into_wall() {
        let player = Rect::new(0.0, 0.0, 10.0, 10.0);
        let wall = Rect::new(20.0, 0.0, 10.0, 10.0);
        let hit = swept_aabb(&player, Vec2::new(20.0, 0.0), &wall).unwrap();
        // Should hit at t = 0.5 (closes 10-unit gap with 20-unit motion).
        assert!((hit.t - 0.5).abs() < 1e-4);
        assert_eq!(hit.normal, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn swept_no_hit() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(100.0, 100.0, 10.0, 10.0);
        assert!(swept_aabb(&a, Vec2::new(1.0, 0.0), &b).is_none());
    }

    #[test]
    fn circle_overlap() {
        assert!(circle_vs_circle(Vec2::ZERO, 5.0, Vec2::new(3.0, 0.0), 5.0));
        assert!(!circle_vs_circle(Vec2::ZERO, 1.0, Vec2::new(10.0, 0.0), 1.0));
    }

    #[test]
    fn point_in_circle_test() {
        assert!(point_in_circle(Vec2::ZERO, Vec2::ZERO, 1.0));
        assert!(!point_in_circle(Vec2::new(2.0, 0.0), Vec2::ZERO, 1.0));
    }

    #[test]
    fn ray_hits_box() {
        let b = Rect::new(10.0, -5.0, 10.0, 10.0);
        let t = ray_vs_aabb(Vec2::ZERO, Vec2::X, &b).unwrap();
        assert!((t - 10.0).abs() < 1e-4);
    }

    #[test]
    fn ray_misses_box() {
        let b = Rect::new(10.0, 100.0, 10.0, 10.0);
        assert!(ray_vs_aabb(Vec2::ZERO, Vec2::X, &b).is_none());
    }
}
