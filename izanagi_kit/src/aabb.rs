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
//! - `contains` — true when another box lies entirely inside this one.
//! - `translate` — shift by an offset (saturating so the box never wraps).
//! - `area` / `is_empty` / `center` — size and midpoint queries.
//! - `iter_points` — row-major iteration over the interior cells.
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

    /// Construct from two corners `(x1, y1)` and `(x2, y2)`. The corners need
    /// not be in top-left/bottom-right order; the result always has `w, h ≥ 0`.
    #[inline]
    pub fn from_corners(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        let x = x1.min(x2);
        let y = y1.min(y2);
        let w = x1.max(x2).saturating_sub(x);
        let h = y1.max(y2).saturating_sub(y);
        Aabb { x, y, w, h }
    }

    /// Expand this AABB by `amount` on every side: `x -= amount`, `y -= amount`,
    /// `w += 2·amount`, `h += 2·amount`. Saturating. Negative `amount` shrinks;
    /// the size is clamped to zero so the result is always a valid AABB.
    #[inline]
    pub fn grow(&self, amount: i32) -> Aabb {
        let x = self.x.saturating_sub(amount);
        let y = self.y.saturating_sub(amount);
        let w = (self.w as i64).saturating_add(2 * amount as i64).max(0) as i32;
        let h = (self.h as i64).saturating_add(2 * amount as i64).max(0) as i32;
        Aabb { x, y, w, h }
    }

    /// Contract this AABB by `amount` on every side. Equivalent to
    /// `grow(-amount)`. The size is clamped to zero.
    #[inline]
    pub fn shrink(&self, amount: i32) -> Aabb {
        self.grow(-amount)
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

    /// Area in cells (`w * h`), computed in `i64` and saturated to `i32` so a
    /// large box never wraps.
    #[inline]
    pub fn area(&self) -> i32 {
        let a = self.w as i64 * self.h as i64;
        a.min(i32::MAX as i64) as i32
    }

    /// True when the box encloses no cells (`w == 0` or `h == 0`).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    /// Centre cell (integer, biased toward the top-left on even extents) — the
    /// same convention as [`crate::mapgen::Rect::center`].
    #[inline]
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    /// The smallest AABB enclosing both `self` and `other`. Empty boxes are
    /// excluded from the result: if one side is empty the other is returned; if
    /// both are empty, an empty box at the origin is returned.
    pub fn union(&self, other: &Aabb) -> Aabb {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let r = self.right().max(other.right());
        let b = self.bottom().max(other.bottom());
        Aabb::new(x, y, r - x, b - y)
    }

    /// True when `other` lies entirely within `self` (boundary inclusive). An
    /// empty `other` is never contained.
    #[inline]
    pub fn contains(&self, other: &Aabb) -> bool {
        !other.is_empty()
            && other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// Clamp `(px, py)` to the nearest point inside the box.
    ///
    /// If the box is empty (`w == 0` or `h == 0`) the top-left corner is
    /// returned. Otherwise `x` is clamped to `[self.x, self.right() - 1]` and
    /// `y` to `[self.y, self.bottom() - 1]` — the same half-open boundary used
    /// by `contains_point`. Useful for keeping a cursor or projectile inside an
    /// AABB without manual min/max arithmetic.
    #[inline]
    pub fn clamp_point(&self, px: i32, py: i32) -> (i32, i32) {
        if self.is_empty() {
            return (self.x, self.y);
        }
        (
            px.clamp(self.x, self.right() - 1),
            py.clamp(self.y, self.bottom() - 1),
        )
    }

    /// Iterate every interior cell `(x, y)` in row-major order (top-to-bottom,
    /// left-to-right). Empty for a zero-area box. Handy for filling or scanning
    /// a rectangular region without manual nested loops.
    pub fn iter_points(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let (x0, y0) = (self.x, self.y);
        let (x1, y1) = (self.right(), self.bottom());
        (y0..y1).flat_map(move |y| (x0..x1).map(move |x| (x, y)))
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

    // --- area / is_empty / center ---

    #[test]
    fn test_area_and_is_empty() {
        assert_eq!(r(0, 0, 4, 3).area(), 12);
        assert!(!r(0, 0, 4, 3).is_empty());
        assert_eq!(r(0, 0, 0, 5).area(), 0);
        assert!(r(0, 0, 0, 5).is_empty());
        assert!(r(0, 0, 5, 0).is_empty());
    }

    #[test]
    fn test_area_saturates() {
        // 50000 * 50000 = 2.5e9 > i32::MAX → saturates rather than wraps.
        assert_eq!(r(0, 0, 50_000, 50_000).area(), i32::MAX);
    }

    #[test]
    fn test_center_biases_top_left_on_even() {
        assert_eq!(r(0, 0, 4, 4).center(), (2, 2));
        assert_eq!(r(2, 3, 5, 3).center(), (4, 4));
    }

    // --- union ---

    #[test]
    fn test_union_two_disjoint_boxes() {
        let u = r(0, 0, 4, 4).union(&r(6, 6, 4, 4));
        assert_eq!(u, r(0, 0, 10, 10));
    }

    #[test]
    fn test_union_overlapping_boxes() {
        let u = r(0, 0, 4, 4).union(&r(2, 2, 4, 4));
        assert_eq!(u, r(0, 0, 6, 6));
    }

    #[test]
    fn test_union_with_empty_returns_other() {
        let non_empty = r(1, 2, 3, 4);
        assert_eq!(r(0, 0, 0, 4).union(&non_empty), non_empty);
        assert_eq!(non_empty.union(&r(0, 0, 0, 4)), non_empty);
    }

    #[test]
    fn test_union_symmetric() {
        let a = r(1, 2, 3, 4);
        let b = r(5, 6, 2, 2);
        assert_eq!(a.union(&b), b.union(&a));
    }

    // --- contains (rect-in-rect) ---

    #[test]
    fn test_contains_fully_inside() {
        assert!(r(0, 0, 10, 10).contains(&r(2, 2, 3, 3)));
        // Boundary-inclusive: an inner box flush to the edges is contained.
        assert!(r(0, 0, 10, 10).contains(&r(0, 0, 10, 10)));
    }

    #[test]
    fn test_contains_partially_outside_is_false() {
        assert!(!r(0, 0, 10, 10).contains(&r(8, 8, 5, 5)));
        assert!(!r(0, 0, 10, 10).contains(&r(-1, 0, 3, 3)));
    }

    #[test]
    fn test_contains_empty_other_is_false() {
        assert!(!r(0, 0, 10, 10).contains(&r(2, 2, 0, 4)));
    }

    // --- iter_points ---

    #[test]
    fn test_iter_points_row_major_order() {
        let pts: Vec<_> = r(1, 1, 2, 2).iter_points().collect();
        assert_eq!(pts, vec![(1, 1), (2, 1), (1, 2), (2, 2)]);
    }

    #[test]
    fn test_iter_points_count_matches_area() {
        let b = r(-3, 5, 4, 6);
        assert_eq!(b.iter_points().count() as i32, b.area());
    }

    #[test]
    fn test_iter_points_empty_box_yields_nothing() {
        assert_eq!(r(0, 0, 0, 5).iter_points().count(), 0);
        assert_eq!(r(0, 0, 5, 0).iter_points().count(), 0);
    }

    #[test]
    fn test_iter_points_all_inside() {
        let b = r(2, 2, 3, 3);
        assert!(b.iter_points().all(|(x, y)| b.contains_point(x, y)));
    }

    #[test]
    fn test_from_corners_ordered() {
        let b = Aabb::from_corners(1, 2, 5, 6);
        assert_eq!(b.x, 1);
        assert_eq!(b.y, 2);
        assert_eq!(b.w, 4);
        assert_eq!(b.h, 4);
    }

    #[test]
    fn test_from_corners_reversed() {
        // Works regardless of which corner is first.
        let b = Aabb::from_corners(5, 6, 1, 2);
        assert_eq!(b.x, 1);
        assert_eq!(b.y, 2);
        assert_eq!(b.w, 4);
        assert_eq!(b.h, 4);
    }

    #[test]
    fn test_from_corners_single_point() {
        let b = Aabb::from_corners(3, 4, 3, 4);
        assert_eq!(b.w, 0);
        assert_eq!(b.h, 0);
        assert!(b.is_empty());
    }

    #[test]
    fn test_grow_expands_all_sides() {
        let b = r(5, 5, 10, 10);
        let g = b.grow(2);
        assert_eq!(g.x, 3);
        assert_eq!(g.y, 3);
        assert_eq!(g.w, 14);
        assert_eq!(g.h, 14);
    }

    #[test]
    fn test_grow_negative_is_shrink() {
        let b = r(0, 0, 10, 10);
        let s = b.grow(-2);
        assert_eq!(s.x, 2);
        assert_eq!(s.y, 2);
        assert_eq!(s.w, 6);
        assert_eq!(s.h, 6);
    }

    #[test]
    fn test_grow_beyond_zero_clamps() {
        let b = r(0, 0, 4, 4);
        let g = b.grow(-10);
        assert_eq!(g.w, 0);
        assert_eq!(g.h, 0);
        assert!(g.is_empty());
    }

    #[test]
    fn test_shrink_symmetric_with_grow() {
        let b = r(2, 2, 8, 8);
        assert_eq!(b.shrink(3), b.grow(-3));
    }

    // --- clamp_point ---

    #[test]
    fn test_clamp_point_inside_unchanged() {
        let b = r(2, 3, 6, 5);
        assert_eq!(b.clamp_point(4, 4), (4, 4));
    }

    #[test]
    fn test_clamp_point_outside_left_top() {
        let b = r(2, 3, 6, 5);
        assert_eq!(b.clamp_point(0, 0), (2, 3));
    }

    #[test]
    fn test_clamp_point_outside_right_bottom() {
        let b = r(2, 3, 6, 5);
        // right-1 = 7, bottom-1 = 7
        assert_eq!(b.clamp_point(100, 100), (7, 7));
    }

    #[test]
    fn test_clamp_point_empty_box_returns_top_left() {
        let b = r(5, 5, 0, 0);
        assert_eq!(b.clamp_point(10, 10), (5, 5));
    }
}
