//! Axis-aligned bounding box (AABB) collision detection.
//!
//! `Aabb` is an integer rectangle defined by its top-left corner and size.
//! All coordinates are `i32` so the box can live anywhere in world space
//! including negative quadrants. The right/bottom edges are exclusive
//! (`x + w`, `y + h`) — the same convention as most 2-D engines.
//!
//! Provided operations:
//! - `overlaps` — true when two boxes share at least one interior point
//!   (touching edges do not count as overlapping).
//! - `contains_point` — true when a point lies strictly inside (or on the
//!   boundary of) the box.
//! - `intersection` — the overlapping sub-box, or `None` if disjoint.
//! - `translate` — shift by an offset (saturating so the box never wraps).
//!
//! All arithmetic is integer and saturating; no float anywhere.

use crate::world_hash::{DetHash, Fnv1a};

/// An axis-aligned bounding box with integer coordinates.
///
/// The represented region is `[x, x+w) × [y, y+h)`.
/// Zero-area boxes (`w == 0` or `h == 0`) are valid but never overlap anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aabb {
    /// Left edge (inclusive).
    pub x: i32,
    /// Top edge (inclusive).
    pub y: i32,
    /// Width in world units (`≥ 0`).
    pub w: i32,
    /// Height in world units (`≥ 0`).
    pub h: i32,
}

impl Aabb {
    /// Construct a new AABB.  Negative `w`/`h` are clamped to `0`.
    #[inline]
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Aabb {
            x,
            y,
            w: w.max(0),
            h: h.max(0),
        }
    }

    /// Exclusive right edge (`x + w`).
    #[inline]
    pub fn right(&self) -> i32 {
        self.x.saturating_add(self.w)
    }

    /// Exclusive bottom edge (`y + h`).
    #[inline]
    pub fn bottom(&self) -> i32 {
        self.y.saturating_add(self.h)
    }

    /// True when `self` and `other` share at least one interior point.
    /// Touching edges (zero-width overlap) do **not** count.
    #[inline]
    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// True when the point `(px, py)` lies within the box (boundary inclusive).
    #[inline]
    pub fn contains_point(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    /// The overlapping sub-rectangle, or `None` if the boxes are disjoint
    /// (including touching edges).
    pub fn intersection(&self, other: &Aabb) -> Option<Aabb> {
        let ix = self.x.max(other.x);
        let iy = self.y.max(other.y);
        let iw = self.right().min(other.right()) - ix;
        let ih = self.bottom().min(other.bottom()) - iy;
        if iw > 0 && ih > 0 {
            Some(Aabb::new(ix, iy, iw, ih))
        } else {
            None
        }
    }

    /// Return a copy of this box shifted by `(dx, dy)` (saturating arithmetic).
    #[inline]
    pub fn translate(&self, dx: i32, dy: i32) -> Aabb {
        Aabb {
            x: self.x.saturating_add(dx),
            y: self.y.saturating_add(dy),
            w: self.w,
            h: self.h,
        }
    }
}

impl DetHash for Aabb {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.x);
        hasher.write_i32(self.y);
        hasher.write_i32(self.w);
        hasher.write_i32(self.h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Aabb {
        Aabb::new(x, y, w, h)
    }

    // --- overlaps ---

    #[test]
    fn test_overlaps_clearly_intersecting() {
        assert!(r(0, 0, 4, 4).overlaps(&r(2, 2, 4, 4)));
    }

    #[test]
    fn test_overlaps_disjoint_x() {
        assert!(!r(0, 0, 4, 4).overlaps(&r(5, 0, 4, 4)));
    }

    #[test]
    fn test_overlaps_disjoint_y() {
        assert!(!r(0, 0, 4, 4).overlaps(&r(0, 5, 4, 4)));
    }

    #[test]
    fn test_overlaps_touching_right_edge_not_overlap() {
        // right edge of first == left edge of second → no overlap
        assert!(!r(0, 0, 4, 4).overlaps(&r(4, 0, 4, 4)));
    }

    #[test]
    fn test_overlaps_touching_bottom_edge_not_overlap() {
        assert!(!r(0, 0, 4, 4).overlaps(&r(0, 4, 4, 4)));
    }

    #[test]
    fn test_overlaps_one_pixel_interior() {
        assert!(r(0, 0, 3, 3).overlaps(&r(2, 2, 3, 3)));
    }

    #[test]
    fn test_overlaps_zero_size_box_never_overlaps() {
        assert!(!r(0, 0, 0, 4).overlaps(&r(0, 0, 4, 4)));
        assert!(!r(0, 0, 4, 0).overlaps(&r(0, 0, 4, 4)));
    }

    #[test]
    fn test_overlaps_fully_contained() {
        assert!(r(1, 1, 2, 2).overlaps(&r(0, 0, 10, 10)));
    }

    // --- contains_point ---

    #[test]
    fn test_contains_point_inside() {
        assert!(r(0, 0, 4, 4).contains_point(2, 2));
    }

    #[test]
    fn test_contains_point_top_left_corner() {
        assert!(r(0, 0, 4, 4).contains_point(0, 0));
    }

    #[test]
    fn test_contains_point_exclusive_right_bottom() {
        assert!(!r(0, 0, 4, 4).contains_point(4, 0));
        assert!(!r(0, 0, 4, 4).contains_point(0, 4));
    }

    #[test]
    fn test_contains_point_outside() {
        assert!(!r(0, 0, 4, 4).contains_point(10, 10));
    }

    // --- intersection ---

    #[test]
    fn test_intersection_overlapping() {
        let result = r(0, 0, 4, 4).intersection(&r(2, 2, 4, 4));
        assert_eq!(result, Some(r(2, 2, 2, 2)));
    }

    #[test]
    fn test_intersection_disjoint_is_none() {
        assert_eq!(r(0, 0, 4, 4).intersection(&r(10, 0, 4, 4)), None);
    }

    #[test]
    fn test_intersection_touching_is_none() {
        assert_eq!(r(0, 0, 4, 4).intersection(&r(4, 0, 4, 4)), None);
    }

    #[test]
    fn test_intersection_fully_contained() {
        let inner = r(1, 1, 2, 2);
        let outer = r(0, 0, 10, 10);
        assert_eq!(inner.intersection(&outer), Some(inner));
    }

    // --- translate ---

    #[test]
    fn test_translate_positive() {
        let moved = r(1, 2, 3, 4).translate(10, 20);
        assert_eq!(moved, r(11, 22, 3, 4));
    }

    #[test]
    fn test_translate_negative() {
        let moved = r(5, 5, 3, 3).translate(-3, -3);
        assert_eq!(moved, r(2, 2, 3, 3));
    }

    #[test]
    fn test_translate_saturates_overflow() {
        let a = r(i32::MAX - 1, 0, 4, 4).translate(100, 0);
        assert_eq!(a.x, i32::MAX);
    }

    // --- negative w/h clamped ---

    #[test]
    fn test_new_negative_dimensions_clamped_to_zero() {
        let a = Aabb::new(0, 0, -5, -3);
        assert_eq!(a.w, 0);
        assert_eq!(a.h, 0);
    }

    // --- det_hash ---

    #[test]
    fn test_det_hash_equal_boxes_equal_hash() {
        assert_eq!(hash_state(&r(1, 2, 3, 4)), hash_state(&r(1, 2, 3, 4)));
    }

    #[test]
    fn test_det_hash_different_boxes_different_hash() {
        assert_ne!(hash_state(&r(0, 0, 4, 4)), hash_state(&r(1, 0, 4, 4)));
    }
}
