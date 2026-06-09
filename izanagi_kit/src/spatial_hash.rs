//! Integer spatial hash grid for broad-phase collision queries.
//!
//! `SpatialHash<K>` partitions world space into fixed-size cells and maps each
//! occupied cell to the set of entity keys inside it. Insert, remove, and
//! query are all O(1) average (linear in the number of results returned).
//!
//! A "cell" is an integer grid square of side `cell_size` world units.
//! World coordinate `(x, y)` maps to cell `(x.div_euclid(cell_size),
//! y.div_euclid(cell_size))` — Euclidean division so negative coordinates
//! also round toward −∞ (no surprises near the origin).
//!
//! The main use cases:
//! - `insert(key, x, y)` — register an entity at a world position.
//! - `remove(key, x, y)` — deregister it (must supply the same position).
//! - `query_cell(x, y)` — all entities whose anchor falls in the same cell.
//! - `query_rect(x, y, w, h)` — all entities in any cell that overlaps the
//!   rectangle `[x, x+w) × [y, y+h)`.
//! - `move_entity(key, old_x, old_y, new_x, new_y)` — atomic remove+insert.
//!
//! `DetHash` (gated on `K: DetHash + Ord`) folds each bucket in canonical
//! (sorted-key) order so the hash is insertion-order-independent.

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::HashMap;

/// Spatial hash grid keyed by entity identifiers of type `K`.
#[derive(Clone, Debug)]
pub struct SpatialHash<K> {
    cell_size: i32,
    /// Maps cell coordinates to the list of keys in that cell.
    cells: HashMap<(i32, i32), Vec<K>>,
}

impl<K: Eq + Clone> SpatialHash<K> {
    /// Create a new grid. `cell_size` is clamped to a minimum of 1.
    pub fn new(cell_size: i32) -> Self {
        SpatialHash {
            cell_size: cell_size.max(1),
            cells: HashMap::new(),
        }
    }

    /// The cell size this grid was constructed with.
    #[inline]
    pub fn cell_size(&self) -> i32 {
        self.cell_size
    }

    /// Convert a world coordinate to a cell index (Euclidean division).
    #[inline]
    fn cell_coord(&self, v: i32) -> i32 {
        v.div_euclid(self.cell_size)
    }

    /// Insert `key` at world position `(x, y)`.
    /// Inserting the same key at the same position twice is harmless but adds
    /// a duplicate; callers should `remove` before re-inserting at a new pos.
    pub fn insert(&mut self, key: K, x: i32, y: i32) {
        let cx = self.cell_coord(x);
        let cy = self.cell_coord(y);
        self.cells.entry((cx, cy)).or_default().push(key);
    }

    /// Remove `key` from the cell at `(x, y)`.  Removes the first occurrence
    /// only; no-op if the key is not present in that cell.
    pub fn remove(&mut self, key: &K, x: i32, y: i32) {
        let cx = self.cell_coord(x);
        let cy = self.cell_coord(y);
        if let Some(bucket) = self.cells.get_mut(&(cx, cy)) {
            if let Some(pos) = bucket.iter().position(|k| k == key) {
                bucket.swap_remove(pos);
            }
            if bucket.is_empty() {
                self.cells.remove(&(cx, cy));
            }
        }
    }

    /// Move `key` from old position to new position atomically.
    pub fn move_entity(&mut self, key: K, old_x: i32, old_y: i32, new_x: i32, new_y: i32) {
        self.remove(&key, old_x, old_y);
        self.insert(key, new_x, new_y);
    }

    /// All keys whose anchor lies in the same cell as `(x, y)`.
    /// Returns an empty slice if the cell is unoccupied.
    pub fn query_cell(&self, x: i32, y: i32) -> &[K] {
        let cx = self.cell_coord(x);
        let cy = self.cell_coord(y);
        self.cells
            .get(&(cx, cy))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// All keys in any cell that overlaps the axis-aligned rectangle
    /// `[x, x+w) × [y, y+h)`.  `w` and `h` are clamped to 0 on the low end.
    /// The returned `Vec` may contain duplicates only if the same key was
    /// inserted into multiple cells — normal usage (one position per key) is
    /// duplicate-free.
    pub fn query_rect(&self, x: i32, y: i32, w: i32, h: i32) -> Vec<K> {
        if w <= 0 || h <= 0 {
            return Vec::new();
        }
        let x1 = self.cell_coord(x);
        let y1 = self.cell_coord(y);
        // Inclusive upper cell: (x+w-1) / cell_size etc.
        let x2 = self.cell_coord(x.saturating_add(w) - 1);
        let y2 = self.cell_coord(y.saturating_add(h) - 1);

        let mut out = Vec::new();
        for cy in y1..=y2 {
            for cx in x1..=x2 {
                if let Some(bucket) = self.cells.get(&(cx, cy)) {
                    out.extend_from_slice(bucket);
                }
            }
        }
        out
    }

    /// All keys within Chebyshev distance `radius` of `(cx, cy)`.
    ///
    /// Chebyshev distance (king-move) is equivalent to a square: this queries
    /// the cell range `[cx-radius, cx+radius] × [cy-radius, cy+radius]`.
    /// Returns an empty vec for negative `radius`.
    pub fn query_radius(&self, cx: i32, cy: i32, radius: i32) -> Vec<K> {
        if radius < 0 {
            return Vec::new();
        }
        self.query_rect(
            cx.saturating_sub(radius),
            cy.saturating_sub(radius),
            2 * radius + 1,
            2 * radius + 1,
        )
    }

    /// All keys within Euclidean distance `radius` of `(qx, qy)`.
    ///
    /// Unlike [`query_radius`](Self::query_radius) (Chebyshev / square), this
    /// uses `dx² + dy² ≤ radius²` on the *cell rectangle's closest point* to
    /// the query centre, so cells that partly overlap the circle are
    /// conservatively included (no false negatives). Returns an empty `Vec` for
    /// negative `radius`.
    pub fn query_radius_euclidean(&self, qx: i32, qy: i32, radius: i32) -> Vec<K> {
        if radius < 0 {
            return Vec::new();
        }
        let r2 = (radius as i64) * (radius as i64);
        let cs = self.cell_size as i64;
        let qcx = self.cell_coord(qx);
        let qcy = self.cell_coord(qy);
        // How many cells can radius span?
        let cr = radius / self.cell_size + 1;
        let mut out = Vec::new();
        for cy in (qcy - cr)..=(qcy + cr) {
            for cx_cell in (qcx - cr)..=(qcx + cr) {
                // Physical bounds of this cell: [cx_cell*cs, (cx_cell+1)*cs)
                let cell_x0 = cx_cell as i64 * cs;
                let cell_y0 = cy as i64 * cs;
                let closest_x = (qx as i64).clamp(cell_x0, cell_x0 + cs - 1);
                let closest_y = (qy as i64).clamp(cell_y0, cell_y0 + cs - 1);
                let dx = qx as i64 - closest_x;
                let dy = qy as i64 - closest_y;
                if dx * dx + dy * dy <= r2 {
                    if let Some(bucket) = self.cells.get(&(cx_cell, cy)) {
                        out.extend_from_slice(bucket);
                    }
                }
            }
        }
        out
    }

    /// Total number of entity-cell registrations (not unique entities).
    pub fn len(&self) -> usize {
        self.cells.values().map(|v| v.len()).sum()
    }

    /// True when no entities are registered.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Remove all entities.
    pub fn clear(&mut self) {
        self.cells.clear();
    }

    /// Number of non-empty cells.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Return `true` if `key` is registered in the cell containing `(x, y)`.
    ///
    /// This is a targeted membership test — it does not search other cells.
    /// O(k) where k is the number of keys in the cell (usually very small).
    pub fn contains(&self, key: &K, x: i32, y: i32) -> bool {
        let cx = self.cell_coord(x);
        let cy = self.cell_coord(y);
        self.cells
            .get(&(cx, cy))
            .map(|bucket| bucket.contains(key))
            .unwrap_or(false)
    }

    /// Number of keys registered in the cell containing `(x, y)`.
    /// Returns `0` for an empty or absent cell. Allocation-free.
    pub fn density(&self, x: i32, y: i32) -> usize {
        let cx = self.cell_coord(x);
        let cy = self.cell_coord(y);
        self.cells.get(&(cx, cy)).map(|v| v.len()).unwrap_or(0)
    }

    /// Iterate over all non-empty cells as `(cell_coord, keys)` pairs.
    ///
    /// `cell_coord` is the grid-cell index (not world coords); multiply by
    /// `cell_size()` to get the world-space top-left corner of the cell.
    /// Iteration order reflects the internal `HashMap` — not sorted.
    pub fn iter_cells(&self) -> impl Iterator<Item = ((i32, i32), &[K])> {
        self.cells
            .iter()
            .map(|(coord, bucket)| (*coord, bucket.as_slice()))
    }

    /// Iterate over every registered key across all cells. Order reflects the
    /// internal `HashMap` bucket order — not sorted and not stable across
    /// inserts. Allocation-free (no intermediate collection).
    ///
    /// Useful for "process every entity in the spatial index" passes (e.g.
    /// end-of-frame position sync) where cell membership is irrelevant.
    pub fn iter_keys(&self) -> impl Iterator<Item = &K> {
        self.cells.values().flat_map(|bucket| bucket.iter())
    }

    /// Return an owned `Vec` of all keys in the grid cell with coordinates
    /// `(cx, cy)` (cell-space, not world-space). Unlike [`query_cell`](Self::query_cell)
    /// which returns a borrowed slice, the owned `Vec` lets the caller mutate
    /// the hash while iterating over the result. Returns an empty `Vec` for
    /// unoccupied cells.
    pub fn all_in_cell(&self, cx: i32, cy: i32) -> Vec<K>
    where
        K: Clone,
    {
        self.cells.get(&(cx, cy)).cloned().unwrap_or_default()
    }
}

impl<K: Eq + Clone + Ord + DetHash> DetHash for SpatialHash<K> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        // Sort cells by coordinate for canonical order.
        let mut cells: Vec<(&(i32, i32), &Vec<K>)> = self.cells.iter().collect();
        cells.sort_by_key(|(coord, _)| *coord);
        hasher.write_u32(self.cell_size as u32);
        hasher.write_u32(cells.len() as u32);
        for ((cx, cy), bucket) in cells {
            hasher.write_i32(*cx);
            hasher.write_i32(*cy);
            let mut sorted = bucket.clone();
            sorted.sort();
            hasher.write_u32(sorted.len() as u32);
            for k in sorted {
                k.det_hash(hasher);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn grid() -> SpatialHash<u32> {
        SpatialHash::new(10)
    }

    #[test]
    fn test_insert_and_query_cell() {
        let mut g = grid();
        g.insert(1, 5, 5);
        assert_eq!(g.query_cell(5, 5), &[1]);
        assert_eq!(g.query_cell(14, 5), &[] as &[u32]); // different cell
    }

    #[test]
    fn test_query_cell_groups_entities_in_same_cell() {
        let mut g = grid();
        g.insert(1, 0, 0);
        g.insert(2, 9, 9); // same cell (cell_size=10)
        let result = g.query_cell(0, 0);
        assert!(result.contains(&1));
        assert!(result.contains(&2));
    }

    #[test]
    fn test_remove_entity() {
        let mut g = grid();
        g.insert(1, 5, 5);
        g.remove(&1, 5, 5);
        assert!(g.query_cell(5, 5).is_empty());
    }

    #[test]
    fn test_remove_nonexistent_is_noop() {
        let mut g = grid();
        g.remove(&99, 0, 0); // should not panic
    }

    #[test]
    fn test_move_entity() {
        let mut g = grid();
        g.insert(1, 5, 5);
        g.move_entity(1, 5, 5, 15, 15);
        assert!(g.query_cell(5, 5).is_empty());
        assert_eq!(g.query_cell(15, 15), &[1]);
    }

    #[test]
    fn test_query_rect_single_cell() {
        let mut g = grid();
        g.insert(1, 5, 5);
        let result = g.query_rect(0, 0, 10, 10);
        assert!(result.contains(&1));
    }

    #[test]
    fn test_query_rect_multi_cell() {
        let mut g = grid();
        g.insert(1, 5, 5); // cell (0,0)
        g.insert(2, 15, 5); // cell (1,0)
        g.insert(3, 25, 5); // cell (2,0) — outside rect
        let result = g.query_rect(0, 0, 20, 10);
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(!result.contains(&3));
    }

    #[test]
    fn test_query_rect_zero_size_returns_empty() {
        let mut g = grid();
        g.insert(1, 5, 5);
        assert!(g.query_rect(0, 0, 0, 10).is_empty());
        assert!(g.query_rect(0, 0, 10, 0).is_empty());
    }

    #[test]
    fn test_negative_coordinates() {
        let mut g = grid();
        g.insert(1, -5, -5); // cell (-1,-1)
        let result = g.query_cell(-5, -5);
        assert_eq!(result, &[1]);
        // (-1,-1) cell: world range [-10,0)×[-10,0)
        let rect = g.query_rect(-10, -10, 10, 10);
        assert!(rect.contains(&1));
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut g = grid();
        assert!(g.is_empty());
        g.insert(1, 0, 0);
        g.insert(2, 5, 5);
        assert_eq!(g.len(), 2);
        assert!(!g.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut g = grid();
        g.insert(1, 0, 0);
        g.clear();
        assert!(g.is_empty());
    }

    #[test]
    fn test_cell_count_decreases_on_remove() {
        let mut g = grid();
        g.insert(1, 0, 0);
        g.insert(2, 100, 100);
        assert_eq!(g.cell_count(), 2);
        g.remove(&1, 0, 0);
        assert_eq!(g.cell_count(), 1);
    }

    #[test]
    fn test_query_radius_includes_center() {
        let mut g = grid();
        g.insert(1, 5, 5);
        assert!(g.query_radius(5, 5, 0).contains(&1));
    }

    #[test]
    fn test_query_radius_includes_nearby() {
        let mut g = grid();
        g.insert(1, 5, 5);
        g.insert(2, 14, 14); // within Chebyshev radius 10 of (5,5)
        g.insert(3, 100, 100); // far away
        let result = g.query_radius(5, 5, 10);
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(!result.contains(&3));
    }

    #[test]
    fn test_query_radius_negative_returns_empty() {
        let mut g = grid();
        g.insert(1, 5, 5);
        assert!(g.query_radius(5, 5, -1).is_empty());
    }

    #[test]
    fn test_det_hash_same_state_same_hash() {
        let mut a = grid();
        let mut b = grid();
        a.insert(1, 5, 5);
        a.insert(2, 15, 15);
        b.insert(1, 5, 5);
        b.insert(2, 15, 15);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_different_positions_different_hash() {
        let mut a = grid();
        let mut b = grid();
        a.insert(1, 5, 5);
        b.insert(1, 15, 5);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_query_radius_euclidean_contains_nearby() {
        let mut g: SpatialHash<i32> = SpatialHash::new(1);
        g.insert(1, 3, 0); // distance 3 from origin
        g.insert(2, 0, 4); // distance 4 from origin
        g.insert(3, 6, 6); // distance > 5 from origin (≈8.5)
        let res = g.query_radius_euclidean(0, 0, 5);
        assert!(res.contains(&1));
        assert!(res.contains(&2));
        assert!(!res.contains(&3));
    }

    #[test]
    fn test_query_radius_euclidean_centre_included() {
        let mut g: SpatialHash<i32> = SpatialHash::new(1);
        g.insert(42, 5, 5);
        let res = g.query_radius_euclidean(5, 5, 0);
        assert!(res.contains(&42));
    }

    #[test]
    fn test_query_radius_euclidean_empty_for_negative_radius() {
        let mut g: SpatialHash<i32> = SpatialHash::new(1);
        g.insert(1, 0, 0);
        assert!(g.query_radius_euclidean(0, 0, -1).is_empty());
    }

    #[test]
    fn test_query_radius_euclidean_subset_of_chebyshev() {
        // Euclidean query should return a subset of Chebyshev query (or equal).
        let mut g: SpatialHash<i32> = SpatialHash::new(1);
        for i in -5i32..=5 {
            for j in -5i32..=5 {
                g.insert(i * 11 + j, i, j);
            }
        }
        let cheb: std::collections::HashSet<i32> = g.query_radius(0, 0, 4).into_iter().collect();
        let eucl: std::collections::HashSet<i32> =
            g.query_radius_euclidean(0, 0, 4).into_iter().collect();
        assert!(eucl.is_subset(&cheb), "euclidean must be ⊆ chebyshev");
    }

    #[test]
    fn test_contains_true_when_present() {
        let mut g = grid();
        g.insert(7u32, 3, 3);
        assert!(g.contains(&7, 3, 3));
        assert!(g.contains(&7, 9, 9)); // same cell (cell_size=10)
    }

    #[test]
    fn test_contains_false_when_absent() {
        let mut g = grid();
        g.insert(7u32, 3, 3);
        assert!(!g.contains(&7, 15, 3)); // different cell
        assert!(!g.contains(&99, 3, 3)); // different key
    }

    #[test]
    fn test_contains_false_after_remove() {
        let mut g = grid();
        g.insert(1u32, 0, 0);
        g.remove(&1, 0, 0);
        assert!(!g.contains(&1, 0, 0));
    }

    #[test]
    fn test_density_zero_for_empty_cell() {
        let g = grid();
        assert_eq!(g.density(5, 5), 0);
    }

    #[test]
    fn test_density_counts_keys_in_cell() {
        let mut g = grid();
        g.insert(1u32, 0, 0);
        g.insert(2, 5, 5); // same cell
        g.insert(3, 50, 50); // different cell
        assert_eq!(g.density(0, 0), 2);
        assert_eq!(g.density(50, 50), 1);
    }

    #[test]
    fn test_iter_cells_covers_all_occupied() {
        let mut g = grid();
        g.insert(1u32, 0, 0);
        g.insert(2, 100, 100);
        let cell_keys: Vec<_> = g.iter_cells().flat_map(|(_, ks)| ks.to_vec()).collect();
        assert!(cell_keys.contains(&1));
        assert!(cell_keys.contains(&2));
        assert_eq!(cell_keys.len(), 2);
    }

    #[test]
    fn test_iter_cells_empty_grid() {
        let g = grid();
        assert_eq!(g.iter_cells().count(), 0);
    }

    #[test]
    fn test_det_hash_insertion_order_independent() {
        let mut a = grid();
        let mut b = grid();
        a.insert(1, 5, 5);
        a.insert(2, 5, 5);
        b.insert(2, 5, 5);
        b.insert(1, 5, 5);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_iter_keys_yields_all_registered_keys() {
        let mut g = grid();
        g.insert(10, 0, 0);
        g.insert(20, 50, 50); // different cell
        g.insert(30, 5, 5); // same cell as (0,0) with cell_size=10
        let mut keys: Vec<u32> = g.iter_keys().copied().collect();
        keys.sort();
        assert_eq!(keys, vec![10, 20, 30]);
    }

    #[test]
    fn test_iter_keys_empty_grid() {
        let g = grid();
        assert_eq!(g.iter_keys().count(), 0);
    }

    #[test]
    fn test_all_in_cell_returns_owned_vec() {
        let mut g = grid(); // cell_size = 10
        g.insert(1u32, 0, 0);
        g.insert(2u32, 5, 5); // same cell (0,0)
        g.insert(3u32, 15, 0); // cell (1,0)
        let mut v = g.all_in_cell(0, 0);
        v.sort();
        assert_eq!(v, vec![1, 2]);
    }

    #[test]
    fn test_all_in_cell_empty_cell_returns_empty() {
        let g = grid();
        assert!(g.all_in_cell(99, 99).is_empty());
    }

    #[test]
    fn test_all_in_cell_does_not_alias_internal_bucket() {
        // Modifying the returned Vec must not affect the hash.
        let mut g = grid();
        g.insert(42u32, 0, 0);
        let mut v = g.all_in_cell(0, 0);
        v.clear();
        // Grid still has the key.
        assert_eq!(g.query_cell(0, 0).len(), 1);
    }
}
