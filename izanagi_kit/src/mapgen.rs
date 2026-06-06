//! Deterministic procedural dungeon generation.
//!
//! A seed-driven "two-step constructive" generator (cf. arXiv:1906.04660): place
//! non-overlapping rectangular rooms, then connect them with L-shaped corridors.
//! Connecting each room to the previous one guarantees the whole dungeon is
//! traversable.
//!
//! Determinism: every random choice comes from a caller-supplied [`SplitMix64`]
//! drawn in a fixed order (no float, no wall-clock), so a given `(seed, params,
//! size)` always yields the byte-identical map — suitable for replay and for
//! folding into the world hash (`Dungeon` implements [`crate::world_hash::DetHash`]).

use crate::rng::SplitMix64;

/// An axis-aligned room rectangle in grid cells. `(x, y)` is the top-left
/// corner; the room covers `[x, x+w) × [y, y+h)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    /// Centre cell (integer, biased toward the top-left on even extents).
    #[inline]
    pub fn center(&self) -> (i32, i32) {
        ((self.x + self.w / 2) as i32, (self.y + self.h / 2) as i32)
    }

    /// Do the two rooms touch or overlap when `self` is grown by one cell on
    /// every side? The padding guarantees at least a one-cell wall between
    /// placed rooms.
    fn intersects_padded(&self, other: &Rect) -> bool {
        let ax0 = self.x as i32 - 1;
        let ax1 = (self.x + self.w) as i32 + 1;
        let ay0 = self.y as i32 - 1;
        let ay1 = (self.y + self.h) as i32 + 1;
        let bx0 = other.x as i32;
        let bx1 = (other.x + other.w) as i32;
        let by0 = other.y as i32;
        let by1 = (other.y + other.h) as i32;
        ax0 < bx1 && bx0 < ax1 && ay0 < by1 && by0 < ay1
    }
}

/// Tuning for [`generate_dungeon`]. `Default` gives sensible roguelike values.
#[derive(Clone, Copy, Debug)]
pub struct GenParams {
    /// How many placement attempts to make (an upper bound on room count).
    pub max_rooms: u32,
    /// Minimum room side length (cells).
    pub min_room: u32,
    /// Maximum room side length (cells).
    pub max_room: u32,
}

impl Default for GenParams {
    fn default() -> Self {
        Self {
            max_rooms: 30,
            min_room: 4,
            max_room: 10,
        }
    }
}

/// A generated dungeon: a `width × height` grid of wall/floor plus the rooms
/// that were placed (in placement order). Out-of-bounds cells are walls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dungeon {
    width: u32,
    height: u32,
    /// Row-major, `true` = wall. Length `width * height`.
    tiles: Vec<bool>,
    pub rooms: Vec<Rect>,
}

impl Dungeon {
    fn filled(width: u32, height: u32) -> Dungeon {
        Dungeon {
            width,
            height,
            tiles: vec![true; (width as usize) * (height as usize)],
            rooms: Vec::new(),
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    /// Is `(x, y)` a wall? Out-of-bounds is treated as wall, so this drops
    /// straight into [`crate::fov`] / [`crate::pathfinding`] as `is_opaque` /
    /// `is_blocked`.
    #[inline]
    pub fn is_wall(&self, x: i32, y: i32) -> bool {
        if !self.in_bounds(x, y) {
            return true;
        }
        self.tiles[(y as u32 * self.width + x as u32) as usize]
    }

    /// Is `(x, y)` an in-bounds floor cell?
    #[inline]
    pub fn is_floor(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && !self.is_wall(x, y)
    }

    #[inline]
    fn carve(&mut self, x: i32, y: i32) {
        if self.in_bounds(x, y) {
            self.tiles[(y as u32 * self.width + x as u32) as usize] = false;
        }
    }

    fn carve_room(&mut self, room: &Rect) {
        for yy in room.y..room.y + room.h {
            for xx in room.x..room.x + room.w {
                self.carve(xx as i32, yy as i32);
            }
        }
    }

    fn carve_h(&mut self, x0: i32, x1: i32, y: i32) {
        for x in x0.min(x1)..=x0.max(x1) {
            self.carve(x, y);
        }
    }

    fn carve_v(&mut self, y0: i32, y1: i32, x: i32) {
        for y in y0.min(y1)..=y0.max(y1) {
            self.carve(x, y);
        }
    }
}

impl crate::world_hash::DetHash for Dungeon {
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        hasher.write_u32(self.width);
        hasher.write_u32(self.height);
        // Pack the wall bitmap into bytes so the fold is compact and canonical.
        for chunk in self.tiles.chunks(8) {
            let mut byte = 0u8;
            for (i, &wall) in chunk.iter().enumerate() {
                byte |= (wall as u8) << i;
            }
            hasher.write_bytes(&[byte]);
        }
    }
}

/// Generate a dungeon of the given size, drawing all randomness from `rng`.
///
/// Rooms are placed by rejection (size and position drawn from `rng`, rejected
/// on overlap), then each new room is joined to the previous with an L-shaped
/// corridor whose elbow direction is also chosen from `rng`. The result is fully
/// connected. Maps too small for a single room come back all-wall rather than
/// panicking.
pub fn generate_dungeon(
    width: u32,
    height: u32,
    rng: &mut SplitMix64,
    params: GenParams,
) -> Dungeon {
    let mut dungeon = Dungeon::filled(width, height);
    // A room plus its one-cell border needs at least 3×3.
    if width < 3 || height < 3 || params.min_room == 0 {
        return dungeon;
    }
    let min = params.min_room.max(1);
    let max = params.max_room.max(min);

    let mut prev_center: Option<(i32, i32)> = None;
    for _ in 0..params.max_rooms {
        let w = rng.range(min as i32, max as i32 + 1) as u32;
        let h = rng.range(min as i32, max as i32 + 1) as u32;
        // Need a one-cell wall border, so the room must fit within the interior.
        if w + 2 > width || h + 2 > height {
            continue;
        }
        let x = rng.range(1, (width - w) as i32) as u32;
        let y = rng.range(1, (height - h) as i32) as u32;
        let room = Rect { x, y, w, h };
        if dungeon.rooms.iter().any(|r| r.intersects_padded(&room)) {
            continue;
        }
        dungeon.carve_room(&room);
        let (cx, cy) = room.center();
        if let Some((px, py)) = prev_center {
            // L-shaped corridor; randomise which leg comes first for variety.
            if rng.coin(1, 2) {
                dungeon.carve_h(px, cx, py);
                dungeon.carve_v(py, cy, cx);
            } else {
                dungeon.carve_v(py, cy, px);
                dungeon.carve_h(px, cx, cy);
            }
        }
        prev_center = Some((cx, cy));
        dungeon.rooms.push(room);
    }
    dungeon
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathfinding::dijkstra_map;

    #[test]
    fn test_same_seed_is_byte_identical() {
        let mut a = SplitMix64::new(0xABCD);
        let mut b = SplitMix64::new(0xABCD);
        let da = generate_dungeon(60, 40, &mut a, GenParams::default());
        let db = generate_dungeon(60, 40, &mut b, GenParams::default());
        assert_eq!(da, db, "same seed must reproduce the map exactly");
    }

    #[test]
    fn test_different_seed_differs() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        let da = generate_dungeon(60, 40, &mut a, GenParams::default());
        let db = generate_dungeon(60, 40, &mut b, GenParams::default());
        assert_ne!(da, db, "different seeds should produce different maps");
    }

    #[test]
    fn test_border_is_solid_wall() {
        let mut rng = SplitMix64::new(7);
        let d = generate_dungeon(50, 30, &mut rng, GenParams::default());
        for x in 0..d.width() as i32 {
            assert!(d.is_wall(x, 0));
            assert!(d.is_wall(x, d.height() as i32 - 1));
        }
        for y in 0..d.height() as i32 {
            assert!(d.is_wall(0, y));
            assert!(d.is_wall(d.width() as i32 - 1, y));
        }
    }

    #[test]
    fn test_rooms_are_placed_and_disjoint() {
        let mut rng = SplitMix64::new(99);
        let d = generate_dungeon(60, 40, &mut rng, GenParams::default());
        assert!(!d.rooms.is_empty(), "a 60x40 map should fit some rooms");
        // Padded disjointness: no two placed rooms touch or overlap.
        for (i, a) in d.rooms.iter().enumerate() {
            for b in &d.rooms[i + 1..] {
                assert!(!a.intersects_padded(b), "rooms {a:?} and {b:?} overlap");
            }
            // Room interior is carved floor.
            let (cx, cy) = a.center();
            assert!(d.is_floor(cx, cy), "room centre must be floor");
        }
    }

    #[test]
    fn test_every_floor_is_reachable() {
        // The defining guarantee: the dungeon is fully connected. Flood from the
        // first room's centre (orthogonally + diagonally) and check every floor
        // cell was reached.
        let mut rng = SplitMix64::new(0xF00D);
        let d = generate_dungeon(60, 40, &mut rng, GenParams::default());
        let start = d.rooms[0].center();
        let reach = dijkstra_map(&[start], i32::MAX, |x, y| d.is_wall(x, y));
        for y in 0..d.height() as i32 {
            for x in 0..d.width() as i32 {
                if d.is_floor(x, y) {
                    assert!(
                        reach.contains_key(&(x, y)),
                        "floor cell ({x},{y}) is unreachable — dungeon not connected"
                    );
                }
            }
        }
    }

    #[test]
    fn test_tiny_map_returns_all_wall_without_panicking() {
        let mut rng = SplitMix64::new(1);
        let d = generate_dungeon(2, 2, &mut rng, GenParams::default());
        assert!(d.rooms.is_empty());
        assert!(d.is_wall(0, 0) && d.is_wall(1, 1));
    }
}
