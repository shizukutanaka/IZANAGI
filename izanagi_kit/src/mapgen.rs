//! Deterministic procedural dungeon generation.
//!
//! A seed-driven "two-step constructive" generator (cf. arXiv:1906.04660): place
//! non-overlapping rectangular rooms, then connect them with L-shaped corridors.
//! Connecting each room to the previous one guarantees the whole dungeon is
//! traversable. Set [`GenParams::extra_loops`] to add cyclic shortcuts on top
//! of that spanning chain (cf. Dormans' cyclic dungeon generation).
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
    /// Top-left x coordinate.
    pub x: u32,
    /// Top-left y coordinate.
    pub y: u32,
    /// Width in cells.
    pub w: u32,
    /// Height in cells.
    pub h: u32,
}

impl Rect {
    /// Centre cell (integer, biased toward the top-left on even extents).
    ///
    /// Computed in `u64` and clamped to `i32::MAX` so a rectangle placed near
    /// the `u32` coordinate ceiling reports a saturated centre rather than
    /// panicking. Identical to the naive `x + w/2` for any real map.
    #[inline]
    pub fn center(&self) -> (i32, i32) {
        let cx = (self.x as u64 + self.w as u64 / 2).min(i32::MAX as u64) as i32;
        let cy = (self.y as u64 + self.h as u64 / 2).min(i32::MAX as u64) as i32;
        (cx, cy)
    }

    /// Area in cells (`w × h`). Saturating: a degenerate rectangle with
    /// near-`u32::MAX` extents reports `u32::MAX` rather than panicking.
    #[inline]
    pub fn area(&self) -> u32 {
        self.w.saturating_mul(self.h)
    }

    /// Inset the rectangle by `n` cells on every side, returning `None` when
    /// the result would have zero (or negative) width or height.
    /// Useful for placing interior decorations and spawn-zones that must stay
    /// away from room walls.
    pub fn shrink(&self, n: u32) -> Option<Rect> {
        let n2 = n.saturating_mul(2);
        if n2 >= self.w || n2 >= self.h {
            return None;
        }
        Some(Rect {
            x: self.x + n,
            y: self.y + n,
            w: self.w - n2,
            h: self.h - n2,
        })
    }

    /// Do the two rooms touch or overlap when `self` is grown by one cell on
    /// every side? The padding guarantees at least a one-cell wall between
    /// placed rooms.
    fn intersects_padded(&self, other: &Rect) -> bool {
        // i64 throughout: room extents are bounded by the map for generated
        // rooms, but `x + w` can still exceed u32 for adversarial rectangles,
        // and the ±1 padding can step past i32::MIN/MAX at the edges.
        let ax0 = self.x as i64 - 1;
        let ax1 = (self.x as i64 + self.w as i64) + 1;
        let ay0 = self.y as i64 - 1;
        let ay1 = (self.y as i64 + self.h as i64) + 1;
        let bx0 = other.x as i64;
        let bx1 = other.x as i64 + other.w as i64;
        let by0 = other.y as i64;
        let by1 = other.y as i64 + other.h as i64;
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
    /// Number of *extra* corridors to carve between random non-adjacent room
    /// pairs after the main chain is built, turning the dungeon's connectivity
    /// from a tree into a graph with cycles.
    ///
    /// The base generator connects each room only to the previous one — a pure
    /// chain, so there is exactly one path between any two rooms. Real dungeon
    /// design wants *loops*: shortcuts, danger/reward alternate routes,
    /// lock-and-key topology (Joris Dormans' cyclic-generation work,
    /// Everything Procedural / PROCJAM 2016). Each extra loop links two rooms
    /// that aren't already chain-adjacent with an L-shaped corridor.
    ///
    /// `0` (the default) reproduces the original chain output **bit-for-bit** —
    /// the loop phase draws zero times from the RNG when this is 0, so existing
    /// seeds are unaffected.
    pub extra_loops: u32,
}

impl Default for GenParams {
    fn default() -> Self {
        Self {
            max_rooms: 30,
            min_room: 4,
            max_room: 10,
            extra_loops: 0,
        }
    }
}

/// Tuning for [`generate_cave`]. `Default` gives a balanced open cave.
#[derive(Clone, Copy, Debug)]
pub struct CaveParams {
    /// Initial wall probability for interior cells, as a percent (`0..=100`).
    /// Around 45 gives classic open caverns; higher yields tighter tunnels.
    pub wall_percent: u32,
    /// Number of cellular-automata smoothing passes. 4–6 is typical; more
    /// passes produce smoother, blobbier walls.
    pub steps: u32,
}

impl Default for CaveParams {
    fn default() -> Self {
        Self {
            wall_percent: 45,
            steps: 5,
        }
    }
}

/// Tuning for [`generate_bsp`]. `Default` gives a classic rooms-and-corridors
/// dungeon.
#[derive(Clone, Copy, Debug)]
pub struct BspParams {
    /// A partition stops splitting once neither axis can yield two children of
    /// at least this size (cells). Larger values → fewer, bigger rooms.
    pub min_leaf: u32,
    /// Hard cap on recursion depth (so the partition count ≤ `2^max_depth`).
    pub max_depth: u32,
    /// Minimum room side length (cells).
    pub room_min: u32,
}

impl Default for BspParams {
    fn default() -> Self {
        Self {
            min_leaf: 8,
            max_depth: 5,
            room_min: 4,
        }
    }
}

/// Tuning for [`generate_drunkard`] (the "drunkard's walk" / 穴掘り法 digging
/// generator). `Default` carves roughly 40% of the interior into one organic,
/// guaranteed-connected cavern.
#[derive(Clone, Copy, Debug)]
pub struct DrunkardParams {
    /// Target fraction of *interior* cells to carve as floor, as a percent
    /// (`1..=100`). The walk stops once this many distinct cells are floor.
    pub fill_percent: u32,
    /// Hard cap on the number of digger steps, so generation always terminates
    /// even if the walk keeps re-treading carved cells. `0` is treated as a
    /// generous default derived from the map area.
    pub max_steps: u32,
}

impl Default for DrunkardParams {
    fn default() -> Self {
        Self {
            fill_percent: 40,
            max_steps: 0,
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
    /// Rooms placed during generation, in placement order.
    pub rooms: Vec<Rect>,
}

impl Dungeon {
    fn filled(width: u32, height: u32) -> Dungeon {
        Dungeon {
            width,
            height,
            tiles: vec![true; (width as usize).saturating_mul(height as usize)],
            rooms: Vec::new(),
        }
    }

    /// Grid width in cells.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Grid height in cells.
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

    /// Number of rooms placed (zero for all-wall or cave dungeons).
    #[inline]
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Return the room with the greatest area (`w × h`), or `None` for an
    /// all-wall dungeon (e.g. a cave or a too-small map). Useful for placing
    /// bosses, exit stairs, or treasure in the most prominent room.
    pub fn largest_room(&self) -> Option<Rect> {
        self.rooms.iter().max_by_key(|r| r.area()).copied()
    }

    /// Room at `index` in placement order, or `None` if `index ≥ room_count()`.
    /// Provides O(1) named access to `self.rooms[index]` without exposing the
    /// internal `Vec` API everywhere.
    #[inline]
    pub fn room_at(&self, index: usize) -> Option<&Rect> {
        self.rooms.get(index)
    }

    /// The first room that contains `(x, y)` (inclusive on all edges), or
    /// `None` if the point is not inside any room. Negative coordinates never
    /// match. For determinism, the first match in placement order is returned.
    pub fn room_containing(&self, x: i32, y: i32) -> Option<Rect> {
        if x < 0 || y < 0 {
            return None;
        }
        let (ux, uy) = (x as u32, y as u32);
        self.rooms
            .iter()
            .find(|r| ux >= r.x && ux < r.x + r.w && uy >= r.y && uy < r.y + r.h)
            .copied()
    }

    /// All floor cell coordinates `(x, y)` in row-major order (`y` outer,
    /// `x` inner). The primary source for spawn placement when every walkable
    /// cell is needed at once — cheaper than scanning with `is_floor` in
    /// calling code and avoids repeated bounds checks. Returns an empty `Vec`
    /// for an all-wall dungeon.
    pub fn floor_cells(&self) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                if self.is_floor(x, y) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    /// Pick a random floor cell using `rng`. Returns `None` for an all-wall
    /// dungeon. Uses a single `rng` draw so the choice is replay-safe.
    pub fn random_floor_cell(&self, rng: &mut SplitMix64) -> Option<(i32, i32)> {
        let cells = self.floor_cells();
        rng.pick(&cells).copied()
    }

    /// Wall off every floor cell that is not part of the **largest connected
    /// region**, leaving a single fully-connected floor area. Returns the
    /// number of cells converted to wall.
    ///
    /// The post-filter step of a Wolverson-style map-builder pipeline
    /// (Roguelike Celebration 2020): generators like [`generate_cave`] readily
    /// produce isolated pockets a player can never reach, and stamping in
    /// prefabs or extra rooms can strand areas. Running this guarantees that
    /// [`crate::pathfinding::is_reachable`] holds between *any* two remaining
    /// floor cells.
    ///
    /// Connectivity uses the same 8-directional, no-corner-cutting rule as the
    /// pathfinding module (via [`crate::pathfinding::ConnectivityMap`]), so the
    /// kept region matches exactly what the pathfinder considers traversable.
    /// Ties for "largest" break toward the lowest component id (row-major
    /// earliest), so the result is deterministic. Rooms whose centre is no
    /// longer floor afterwards are dropped from [`rooms`](Self::rooms) to keep
    /// the room list consistent with the grid. An all-wall or single-region
    /// dungeon is left unchanged (returns 0).
    pub fn keep_largest_region(&mut self) -> usize {
        let cm = crate::pathfinding::ConnectivityMap::new(self.width, self.height, |x, y| {
            self.is_wall(x, y)
        });
        let keep = match cm.largest_component() {
            Some(id) => id,
            None => return 0, // no floor at all
        };
        let mut filled = 0usize;
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                // Fill any floor cell that belongs to a different component.
                if matches!(cm.component(x, y), Some(c) if c != keep) {
                    self.tiles[(y as u32 * self.width + x as u32) as usize] = true;
                    filled += 1;
                }
            }
        }
        if filled > 0 {
            // Drop rooms whose centre is no longer floor (their region was
            // culled), keeping the room list consistent with the grid.
            let (w, h) = (self.width, self.height);
            let tiles = &self.tiles;
            self.rooms.retain(|r| {
                let (cx, cy) = r.center();
                cx >= 0
                    && cy >= 0
                    && (cx as u32) < w
                    && (cy as u32) < h
                    && !tiles[(cy as u32 * w + cx as u32) as usize]
            });
        }
        filled
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

impl crate::world_hash::DetHash for Rect {
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        hasher.write_u32(self.x);
        hasher.write_u32(self.y);
        hasher.write_u32(self.w);
        hasher.write_u32(self.h);
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
        // Rooms are simulation state: placement order and exact boundaries affect
        // spawn logic. Must be hashed or a desync in room registry goes undetected.
        hasher.write_u32(self.rooms.len() as u32);
        for r in &self.rooms {
            r.det_hash(hasher);
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

    // Extra loop corridors (cyclic connectivity). Skipped entirely — and thus
    // drawing nothing from `rng` — when extra_loops == 0, so the default output
    // is byte-identical to the pure-chain generator.
    let room_count = dungeon.rooms.len();
    if params.extra_loops > 0 && room_count >= 3 {
        for _ in 0..params.extra_loops {
            // Pick two distinct rooms. Reject the pair only if they are already
            // chain-adjacent (indices differ by 1) — those are already linked,
            // so a loop there adds nothing. With >=3 rooms a non-adjacent pair
            // always exists, so a bounded number of tries suffices.
            let mut linked = false;
            for _ in 0..8 {
                let a = rng.below(room_count as u32) as usize;
                let b = rng.below(room_count as u32) as usize;
                if a == b || a.abs_diff(b) == 1 {
                    continue;
                }
                let (ax, ay) = dungeon.rooms[a].center();
                let (bx, by) = dungeon.rooms[b].center();
                // L-shaped corridor, elbow direction chosen from rng for variety.
                if rng.coin(1, 2) {
                    dungeon.carve_h(ax, bx, ay);
                    dungeon.carve_v(ay, by, bx);
                } else {
                    dungeon.carve_v(ay, by, ax);
                    dungeon.carve_h(ax, bx, by);
                }
                linked = true;
                break;
            }
            // If 8 tries all collided (pathological tiny room set), stop early
            // rather than spin — the dungeon is already fully connected.
            if !linked {
                break;
            }
        }
    }

    dungeon
}

/// Generate an organic cave with the **cellular-automata** method (the
/// RogueBasin "cave-like levels" technique; cf. `bracket-lib` and `rot.js`
/// `Cellular`). Complements the rectangular [`generate_dungeon`]: where that
/// places rooms, this grows winding caverns.
///
/// The pipeline is fully deterministic from `rng`:
/// 1. **Seed** — each interior cell starts as floor with probability
///    `100 - wall_percent`; the one-cell border is always wall.
/// 2. **Smooth** — `steps` passes of the 4-5 rule: a cell becomes wall when 5+
///    of its 8 neighbours (out-of-bounds counts as wall) are walls, else floor.
/// 3. **Connect** — every floor cell outside the single largest 4-connected
///    region is filled back to wall, so the returned cave is one connected
///    space with no isolated pockets.
///
/// `rooms` is left empty (caves have no rectangular rooms). The result plugs
/// straight into [`crate::fov`] / [`crate::pathfinding`] via [`Dungeon::is_wall`].
pub fn generate_cave(width: u32, height: u32, rng: &mut SplitMix64, params: CaveParams) -> Dungeon {
    let mut d = Dungeon::filled(width, height);
    if width < 3 || height < 3 {
        return d;
    }
    let (w, h) = (width as i32, height as i32);
    let wp = params.wall_percent.min(100);

    // 1. Random seed (interior only; border stays wall).
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            // Floor with probability (100 - wp)%. Draw once per interior cell.
            if rng.below(100) >= wp {
                d.carve(x, y);
            }
        }
    }

    // 2. Cellular-automata smoothing passes (4-5 rule, OOB = wall).
    for _ in 0..params.steps {
        // Start the next grid all-wall so the border is preserved untouched.
        let mut next = vec![true; (width as usize).saturating_mul(height as usize)];
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let walls = wall_neighbours(&d, x, y);
                let idx = (y as u32 * width + x as u32) as usize;
                next[idx] = walls >= 5;
            }
        }
        d.tiles = next;
    }

    // 3. Cull everything but the largest connected floor region.
    cull_to_largest_region(&mut d);
    d
}

/// Count wall cells in the 8-neighbourhood of `(x, y)`; out-of-bounds is wall.
fn wall_neighbours(d: &Dungeon, x: i32, y: i32) -> u32 {
    let mut n = 0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            if d.is_wall(x + dx, y + dy) {
                n += 1;
            }
        }
    }
    n
}

/// Keep only the largest 4-connected region of floor cells; fill the rest.
/// No-op when there is no floor.
fn cull_to_largest_region(d: &mut Dungeon) {
    let (w, h) = (d.width as i32, d.height as i32);
    let size = (d.width as usize).saturating_mul(d.height as usize);
    let mut region = vec![u32::MAX; size]; // region id per cell; MAX = unassigned/wall
    let mut sizes: Vec<u32> = Vec::new();
    let idx = |x: i32, y: i32| (y as u32 * d.width + x as u32) as usize;

    let mut stack: Vec<(i32, i32)> = Vec::new();
    for sy in 0..h {
        for sx in 0..w {
            if d.is_wall(sx, sy) || region[idx(sx, sy)] != u32::MAX {
                continue;
            }
            // Flood-fill a new region (4-connectivity).
            let id = sizes.len() as u32;
            let mut count = 0u32;
            stack.push((sx, sy));
            region[idx(sx, sy)] = id;
            while let Some((cx, cy)) = stack.pop() {
                count += 1;
                for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    if d.is_floor(nx, ny) && region[idx(nx, ny)] == u32::MAX {
                        region[idx(nx, ny)] = id;
                        stack.push((nx, ny));
                    }
                }
            }
            sizes.push(count);
        }
    }

    // Find the largest region; ties resolve to the lowest id (deterministic).
    let Some(best) = sizes
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0)))
        .map(|(i, _)| i as u32)
    else {
        return; // no floor at all
    };

    // Fill any floor cell not in the winning region.
    for y in 0..h {
        for x in 0..w {
            let i = idx(x, y);
            if region[i] != u32::MAX && region[i] != best {
                d.tiles[i] = true;
            }
        }
    }
}

/// Generate a dungeon by **binary space partitioning** — the third classic
/// family alongside [`generate_dungeon`] (room rejection) and [`generate_cave`]
/// (cellular automata); cf. `bracket-lib`'s BSP builders.
///
/// The interior is recursively split into sub-rectangles (orientation biased by
/// aspect ratio, split point drawn from `rng`) until a partition is too small to
/// halve or `max_depth` is reached. One room is carved inside each leaf, and on
/// the way back up the recursion each split joins its two child rooms with an
/// L-shaped corridor — so the whole dungeon is guaranteed connected. Rooms never
/// overlap because each lives in its own partition. Maps too small to partition
/// return all-wall rather than panicking.
pub fn generate_bsp(width: u32, height: u32, rng: &mut SplitMix64, params: BspParams) -> Dungeon {
    let mut d = Dungeon::filled(width, height);
    if width < 5 || height < 5 {
        return d;
    }
    let min_leaf = params.min_leaf.max(3);
    let room_min = params.room_min.max(2);
    // Work inside a one-cell border so corridors/rooms never touch the edge.
    let area = Rect {
        x: 1,
        y: 1,
        w: width - 2,
        h: height - 2,
    };
    bsp_build(&mut d, rng, area, params.max_depth, min_leaf, room_min);
    d
}

/// Recursively partition `area`, carve rooms at the leaves, and connect each
/// split's two children. Returns a representative connection point (a carved
/// room centre) for this subtree so the parent can wire it to its sibling.
fn bsp_build(
    d: &mut Dungeon,
    rng: &mut SplitMix64,
    area: Rect,
    depth: u32,
    min_leaf: u32,
    room_min: u32,
) -> (i32, i32) {
    let can_split_w = area.w >= 2 * min_leaf;
    let can_split_h = area.h >= 2 * min_leaf;

    if depth == 0 || (!can_split_w && !can_split_h) {
        return carve_leaf_room(d, rng, area, room_min);
    }

    // Choose split orientation: bias toward halving the longer side, else coin.
    let vertical = if can_split_w && can_split_h {
        if area.w * 100 > area.h * 125 {
            true
        } else if area.h * 100 > area.w * 125 {
            false
        } else {
            rng.coin(1, 2)
        }
    } else {
        can_split_w
    };

    let (a, b) = if vertical {
        let cut = rng.range(min_leaf as i32, (area.w - min_leaf) as i32 + 1) as u32;
        (
            Rect {
                x: area.x,
                y: area.y,
                w: cut,
                h: area.h,
            },
            Rect {
                x: area.x + cut,
                y: area.y,
                w: area.w - cut,
                h: area.h,
            },
        )
    } else {
        let cut = rng.range(min_leaf as i32, (area.h - min_leaf) as i32 + 1) as u32;
        (
            Rect {
                x: area.x,
                y: area.y,
                w: area.w,
                h: cut,
            },
            Rect {
                x: area.x,
                y: area.y + cut,
                w: area.w,
                h: area.h - cut,
            },
        )
    };

    let ca = bsp_build(d, rng, a, depth - 1, min_leaf, room_min);
    let cb = bsp_build(d, rng, b, depth - 1, min_leaf, room_min);
    // L-shaped corridor between the two child rooms; randomise the elbow.
    if rng.coin(1, 2) {
        d.carve_h(ca.0, cb.0, ca.1);
        d.carve_v(ca.1, cb.1, cb.0);
    } else {
        d.carve_v(ca.1, cb.1, ca.0);
        d.carve_h(ca.0, cb.0, cb.1);
    }
    ca
}

/// Carve one room inside `area` (with a 1-cell inset when the leaf is large
/// enough so neighbouring rooms keep a wall between them), record it, and return
/// its centre. The room always fits within `area`, so no carve escapes bounds.
fn carve_leaf_room(d: &mut Dungeon, rng: &mut SplitMix64, area: Rect, room_min: u32) -> (i32, i32) {
    let inset_w = if area.w >= room_min + 2 { 1 } else { 0 };
    let inset_h = if area.h >= room_min + 2 { 1 } else { 0 };
    let span_w = area.w - 2 * inset_w;
    let span_h = area.h - 2 * inset_h;

    let rw = if span_w <= room_min {
        span_w.max(1)
    } else {
        rng.range(room_min as i32, span_w as i32 + 1) as u32
    };
    let rh = if span_h <= room_min {
        span_h.max(1)
    } else {
        rng.range(room_min as i32, span_h as i32 + 1) as u32
    };

    let rx = area.x + inset_w + rng.range(0, (span_w - rw) as i32 + 1) as u32;
    let ry = area.y + inset_h + rng.range(0, (span_h - rh) as i32 + 1) as u32;

    let room = Rect {
        x: rx,
        y: ry,
        w: rw,
        h: rh,
    };
    d.carve_room(&room);
    d.rooms.push(room);
    room.center()
}

/// Generate an organic cavern by **drunkard's walk** (the classic roguelike
/// "穴掘り法" digging method): a single digger starts at the centre and takes
/// random cardinal steps, carving every interior cell it visits, until the
/// target fill fraction is reached.
///
/// Because the digger moves one cell at a time and carves continuously, the
/// result is **guaranteed to be a single 4-connected floor region** — unlike
/// [`generate_cave`], which seeds with noise and must cull disconnected blobs.
/// This makes it ideal when you need a winding, fully-traversable cavern with
/// no post-processing.
///
/// Deterministic: the digger starts at a fixed centre and draws exactly one
/// `rng` value (a cardinal direction) per step. The walk is clamped to the
/// interior `[1, width-2] × [1, height-2]`, so it slides along the border
/// rather than escaping. Returns an all-wall dungeon for maps smaller than
/// `3 × 3`.
pub fn generate_drunkard(
    width: u32,
    height: u32,
    rng: &mut SplitMix64,
    params: DrunkardParams,
) -> Dungeon {
    let mut d = Dungeon::filled(width, height);
    if width < 3 || height < 3 {
        return d;
    }
    let (w, h) = (width as i32, height as i32);
    let interior = ((w - 2) as u32) * ((h - 2) as u32);
    let fill = params.fill_percent.clamp(1, 100);
    // Target distinct floor cells; at least 1, never more than the interior.
    let target = (interior * fill / 100).clamp(1, interior);
    // Generous default step cap: enough slack to reach the target on an open
    // walk while still bounding the worst case.
    let max_steps = if params.max_steps == 0 {
        interior.saturating_mul(8).max(64)
    } else {
        params.max_steps
    };

    // Start the digger at the centre of the interior.
    let mut x = w / 2;
    let mut y = h / 2;
    d.carve(x, y);
    let mut carved = 1u32;

    let mut steps = 0u32;
    while carved < target && steps < max_steps {
        steps += 1;
        // One draw per step: a cardinal direction (replay-safe).
        let (dx, dy) = match rng.below(4) {
            0 => (0, -1),
            1 => (1, 0),
            2 => (0, 1),
            _ => (-1, 0),
        };
        // Clamp into the interior so the digger slides along walls.
        x = (x + dx).clamp(1, w - 2);
        y = (y + dy).clamp(1, h - 2);
        if d.is_wall(x, y) {
            d.carve(x, y);
            carved += 1;
        }
    }
    d
}

/// A composable dungeon post-processing pipeline (Wolverson's MapBuilder
/// pattern, Roguelike Celebration 2020): start from any generated [`Dungeon`],
/// then chain **stages** that reshape it — carve extra features, stamp
/// prefabs, cull disconnected pockets — instead of hard-coding one monolithic
/// generator.
///
/// A stage is `FnMut(&mut Dungeon, &mut SplitMix64)`. The key determinism
/// property is **per-stage RNG isolation**: each stage receives its own
/// independent stream, derived from the build's base seed and the stage's
/// position via [`SplitMix64::split`], *not* a single generator threaded
/// through every stage. So the number of random draws one stage makes can
/// never shift another stage's stream — inserting, removing, or reworking a
/// stage's internals leaves every *other* stage's output byte-identical. The
/// whole pipeline is a pure function of `(starting dungeon, base_seed)`.
///
/// ```
/// use izanagi_kit::mapgen::{generate_cave, CaveParams, MapBuilder};
/// use izanagi_kit::rng::SplitMix64;
///
/// let mut rng = SplitMix64::new(42);
/// let cave = generate_cave(48, 32, &mut rng, CaveParams::default());
/// // Guarantee a single connected cavern.
/// let dungeon = MapBuilder::new(cave).keep_largest_region().build(42);
/// ```
pub struct MapBuilder {
    dungeon: Dungeon,
    #[allow(clippy::type_complexity)]
    stages: Vec<Box<dyn FnMut(&mut Dungeon, &mut SplitMix64)>>,
}

impl MapBuilder {
    /// Start a pipeline from an already-generated `dungeon` (from any of the
    /// `generate_*` functions, or hand-built).
    pub fn new(dungeon: Dungeon) -> Self {
        MapBuilder {
            dungeon,
            stages: Vec::new(),
        }
    }

    /// Append a custom stage. It is handed the working dungeon and its own
    /// independent RNG stream (see the type-level docs on isolation). Stages
    /// run in the order added.
    pub fn stage<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut Dungeon, &mut SplitMix64) + 'static,
    {
        self.stages.push(Box::new(f));
        self
    }

    /// Append the built-in [`Dungeon::keep_largest_region`] stage — walls off
    /// every disconnected pocket so the finished map is a single connected
    /// region. Uses no randomness (its RNG stream is ignored).
    pub fn keep_largest_region(self) -> Self {
        self.stage(|d, _rng| {
            d.keep_largest_region();
        })
    }

    /// The number of stages queued so far.
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Run every stage in order and return the finished dungeon. Each stage `i`
    /// is driven by `SplitMix64::new(base_seed).split(i)`, so the result is
    /// fully determined by the starting dungeon and `base_seed`.
    pub fn build(mut self, base_seed: u64) -> Dungeon {
        let base = SplitMix64::new(base_seed);
        for (i, stage) in self.stages.iter_mut().enumerate() {
            let mut stream = base.split(i as u64);
            stage(&mut self.dungeon, &mut stream);
        }
        self.dungeon
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathfinding::dijkstra_map;
    use crate::world_hash::hash_state;

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

    // ── cellular-automata caves ──────────────────────────────────────────────

    #[test]
    fn test_cave_same_seed_is_byte_identical() {
        let mut a = SplitMix64::new(0x5EED);
        let mut b = SplitMix64::new(0x5EED);
        let ca = generate_cave(64, 40, &mut a, CaveParams::default());
        let cb = generate_cave(64, 40, &mut b, CaveParams::default());
        assert_eq!(ca, cb, "same seed must reproduce the cave exactly");
    }

    #[test]
    fn test_cave_different_seed_differs() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        let ca = generate_cave(64, 40, &mut a, CaveParams::default());
        let cb = generate_cave(64, 40, &mut b, CaveParams::default());
        assert_ne!(ca, cb, "different seeds should produce different caves");
    }

    #[test]
    fn test_cave_border_is_solid_wall() {
        let mut rng = SplitMix64::new(7);
        let d = generate_cave(50, 30, &mut rng, CaveParams::default());
        for x in 0..d.width() as i32 {
            assert!(d.is_wall(x, 0) && d.is_wall(x, d.height() as i32 - 1));
        }
        for y in 0..d.height() as i32 {
            assert!(d.is_wall(0, y) && d.is_wall(d.width() as i32 - 1, y));
        }
    }

    #[test]
    fn test_cave_is_fully_connected_and_has_floor() {
        // Connectivity is the headline guarantee: after culling there must be a
        // single reachable floor region.
        let mut rng = SplitMix64::new(0xCAFE);
        let d = generate_cave(80, 45, &mut rng, CaveParams::default());

        // Find any floor cell to seed the flood.
        let mut start = None;
        'outer: for y in 0..d.height() as i32 {
            for x in 0..d.width() as i32 {
                if d.is_floor(x, y) {
                    start = Some((x, y));
                    break 'outer;
                }
            }
        }
        let start = start.expect("a default cave should contain floor");
        let reach = dijkstra_map(&[start], i32::MAX, |x, y| d.is_wall(x, y));
        for y in 0..d.height() as i32 {
            for x in 0..d.width() as i32 {
                if d.is_floor(x, y) {
                    assert!(
                        reach.contains_key(&(x, y)),
                        "floor cell ({x},{y}) unreachable — cave not connected"
                    );
                }
            }
        }
    }

    #[test]
    fn test_cave_has_no_rectangular_rooms() {
        let mut rng = SplitMix64::new(11);
        let d = generate_cave(60, 40, &mut rng, CaveParams::default());
        assert!(d.rooms.is_empty(), "caves carry no Rect rooms");
    }

    #[test]
    fn test_cave_tiny_map_returns_all_wall_without_panicking() {
        let mut rng = SplitMix64::new(3);
        let d = generate_cave(2, 2, &mut rng, CaveParams::default());
        assert!(d.is_wall(0, 0) && d.is_wall(1, 1));
    }

    #[test]
    fn test_cave_all_walls_when_fully_solid_seed() {
        // wall_percent = 100 → no floor seeded → CA keeps it solid → no panic.
        let mut rng = SplitMix64::new(5);
        let params = CaveParams {
            wall_percent: 100,
            steps: 4,
        };
        let d = generate_cave(40, 25, &mut rng, params);
        for y in 0..d.height() as i32 {
            for x in 0..d.width() as i32 {
                assert!(d.is_wall(x, y));
            }
        }
    }

    // ── BSP partition dungeons ───────────────────────────────────────────────

    #[test]
    fn test_bsp_same_seed_is_byte_identical() {
        let mut a = SplitMix64::new(0xB59);
        let mut b = SplitMix64::new(0xB59);
        let da = generate_bsp(64, 40, &mut a, BspParams::default());
        let db = generate_bsp(64, 40, &mut b, BspParams::default());
        assert_eq!(da, db, "same seed must reproduce the BSP map exactly");
    }

    #[test]
    fn test_bsp_different_seed_differs() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        let da = generate_bsp(64, 40, &mut a, BspParams::default());
        let db = generate_bsp(64, 40, &mut b, BspParams::default());
        assert_ne!(da, db);
    }

    #[test]
    fn test_bsp_border_is_solid_wall() {
        let mut rng = SplitMix64::new(7);
        let d = generate_bsp(50, 30, &mut rng, BspParams::default());
        for x in 0..d.width() as i32 {
            assert!(d.is_wall(x, 0) && d.is_wall(x, d.height() as i32 - 1));
        }
        for y in 0..d.height() as i32 {
            assert!(d.is_wall(0, y) && d.is_wall(d.width() as i32 - 1, y));
        }
    }

    #[test]
    fn test_bsp_rooms_placed_and_disjoint() {
        let mut rng = SplitMix64::new(99);
        let d = generate_bsp(64, 40, &mut rng, BspParams::default());
        assert!(!d.rooms.is_empty(), "BSP should carve rooms");
        // Each room lives in its own partition, so no two overlap (padded).
        for (i, a) in d.rooms.iter().enumerate() {
            for b in &d.rooms[i + 1..] {
                assert!(!a.intersects_padded(b), "rooms {a:?} and {b:?} overlap");
            }
        }
    }

    #[test]
    fn test_bsp_is_fully_connected() {
        // The defining guarantee: every floor cell is reachable (corridors join
        // each split's children bottom-up).
        let mut rng = SplitMix64::new(0xBADC0DE);
        let d = generate_bsp(72, 44, &mut rng, BspParams::default());
        let start = d.rooms[0].center();
        let reach = dijkstra_map(&[start], i32::MAX, |x, y| d.is_wall(x, y));
        for y in 0..d.height() as i32 {
            for x in 0..d.width() as i32 {
                if d.is_floor(x, y) {
                    assert!(
                        reach.contains_key(&(x, y)),
                        "floor cell ({x},{y}) unreachable — BSP dungeon not connected"
                    );
                }
            }
        }
    }

    #[test]
    fn test_bsp_tiny_map_returns_all_wall_without_panicking() {
        let mut rng = SplitMix64::new(3);
        let d = generate_bsp(4, 4, &mut rng, BspParams::default());
        for y in 0..d.height() as i32 {
            for x in 0..d.width() as i32 {
                assert!(d.is_wall(x, y));
            }
        }
    }

    // --- largest_room tests ---

    #[test]
    fn test_largest_room_none_when_no_rooms() {
        let mut rng = SplitMix64::new(1);
        let d = generate_cave(40, 25, &mut rng, CaveParams::default());
        assert!(d.largest_room().is_none(), "caves carry no Rect rooms");
    }

    #[test]
    fn test_largest_room_returns_biggest_by_area() {
        let mut rng = SplitMix64::new(42);
        let d = generate_dungeon(80, 50, &mut rng, GenParams::default());
        assert!(!d.rooms.is_empty());
        let biggest = d.largest_room().unwrap();
        let biggest_area = biggest.w * biggest.h;
        for r in &d.rooms {
            assert!(
                r.w * r.h <= biggest_area,
                "a larger room {r:?} exceeds reported largest"
            );
        }
    }

    #[test]
    fn test_largest_room_consistent_across_calls() {
        let mut rng = SplitMix64::new(99);
        let d = generate_dungeon(60, 40, &mut rng, GenParams::default());
        assert_eq!(d.largest_room(), d.largest_room(), "must be deterministic");
    }

    #[test]
    fn test_bsp_small_min_leaf_still_connected() {
        // Aggressive splitting (small min_leaf, deep) must stay connected.
        let mut rng = SplitMix64::new(2024);
        let params = BspParams {
            min_leaf: 5,
            max_depth: 8,
            room_min: 3,
        };
        let d = generate_bsp(60, 40, &mut rng, params);
        let start = d.rooms[0].center();
        let reach = dijkstra_map(&[start], i32::MAX, |x, y| d.is_wall(x, y));
        let unreached = (0..d.height() as i32)
            .flat_map(|y| (0..d.width() as i32).map(move |x| (x, y)))
            .filter(|&(x, y)| d.is_floor(x, y) && !reach.contains_key(&(x, y)))
            .count();
        assert_eq!(unreached, 0, "{unreached} floor cells unreachable");
    }

    #[test]
    fn test_floor_cells_matches_is_floor_scan() {
        let mut rng = SplitMix64::new(999);
        let d = generate_dungeon(20, 15, &mut rng, GenParams::default());
        let expected: Vec<(i32, i32)> = (0..15i32)
            .flat_map(|y| (0..20i32).map(move |x| (x, y)))
            .filter(|&(x, y)| d.is_floor(x, y))
            .collect();
        assert_eq!(d.floor_cells(), expected);
    }

    #[test]
    fn test_floor_cells_cave_has_floor_cells() {
        let mut rng = SplitMix64::new(42);
        let d = generate_cave(20, 15, &mut rng, CaveParams::default());
        let cells = d.floor_cells();
        assert!(!cells.is_empty(), "cave should have floor cells");
        for &(x, y) in &cells {
            assert!(d.is_floor(x, y), "({x},{y}) must be a floor cell");
        }
    }

    #[test]
    fn test_floor_cells_bsp_count_matches_manual() {
        let mut rng = SplitMix64::new(77);
        let d = generate_bsp(30, 20, &mut rng, BspParams::default());
        let manual_count = (0..20i32)
            .flat_map(|y| (0..30i32).map(move |x| (x, y)))
            .filter(|&(x, y)| d.is_floor(x, y))
            .count();
        assert_eq!(d.floor_cells().len(), manual_count);
    }

    #[test]
    fn test_rect_area() {
        let r = Rect {
            x: 0,
            y: 0,
            w: 5,
            h: 3,
        };
        assert_eq!(r.area(), 15);
    }

    #[test]
    fn test_rect_area_zero_dimension() {
        let r = Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 10,
        };
        assert_eq!(r.area(), 0);
    }

    #[test]
    fn test_rect_area_unit() {
        let r = Rect {
            x: 3,
            y: 3,
            w: 1,
            h: 1,
        };
        assert_eq!(r.area(), 1);
    }

    #[test]
    fn test_room_count_matches_rooms_len() {
        let mut rng = SplitMix64::new(55);
        let d = generate_dungeon(40, 30, &mut rng, GenParams::default());
        assert_eq!(d.room_count(), d.rooms.len());
    }

    #[test]
    fn test_room_count_bsp_nonzero() {
        let mut rng = SplitMix64::new(42);
        let d = generate_bsp(30, 20, &mut rng, BspParams::default());
        assert!(d.room_count() > 0);
    }

    #[test]
    fn test_room_count_at_least_one_when_large_enough() {
        let mut rng = SplitMix64::new(1);
        let d = generate_dungeon(50, 40, &mut rng, GenParams::default());
        assert!(d.room_count() >= 1);
    }

    #[test]
    fn test_rect_shrink_returns_inset_rect() {
        let r = Rect {
            x: 2,
            y: 3,
            w: 10,
            h: 8,
        };
        let s = r.shrink(2).unwrap();
        assert_eq!(s.x, 4);
        assert_eq!(s.y, 5);
        assert_eq!(s.w, 6);
        assert_eq!(s.h, 4);
    }

    #[test]
    fn test_rect_shrink_too_large_returns_none() {
        let r = Rect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };
        assert!(r.shrink(2).is_none());
    }

    #[test]
    fn test_rect_shrink_zero_is_identity() {
        let r = Rect {
            x: 1,
            y: 2,
            w: 5,
            h: 7,
        };
        assert_eq!(r.shrink(0).unwrap(), r);
    }

    #[test]
    fn test_random_floor_cell_is_valid_floor() {
        let mut rng = SplitMix64::new(42);
        let d = generate_dungeon(30, 20, &mut rng, GenParams::default());
        let mut rng2 = SplitMix64::new(1);
        let cell = d
            .random_floor_cell(&mut rng2)
            .expect("dungeon has floor cells");
        assert!(d.is_floor(cell.0, cell.1));
    }

    #[test]
    fn test_random_floor_cell_all_wall_returns_none() {
        let mut rng = SplitMix64::new(0);
        let d = generate_dungeon(2, 2, &mut rng, GenParams::default());
        let mut rng2 = SplitMix64::new(0);
        assert!(d.random_floor_cell(&mut rng2).is_none());
    }

    #[test]
    fn test_random_floor_cell_is_deterministic() {
        let mut rng = SplitMix64::new(99);
        let d = generate_dungeon(40, 30, &mut rng, GenParams::default());
        let mut r1 = SplitMix64::new(5);
        let mut r2 = SplitMix64::new(5);
        assert_eq!(d.random_floor_cell(&mut r1), d.random_floor_cell(&mut r2));
    }

    // --- room_at / room_containing ---

    #[test]
    fn test_room_at_returns_room_by_index() {
        let mut rng = SplitMix64::new(1);
        let d = generate_dungeon(40, 30, &mut rng, GenParams::default());
        assert!(d.room_count() > 0);
        assert!(d.room_at(0).is_some());
        assert_eq!(d.room_at(0), d.rooms.first());
    }

    #[test]
    fn test_room_at_out_of_range_returns_none() {
        let mut rng = SplitMix64::new(1);
        let d = generate_dungeon(40, 30, &mut rng, GenParams::default());
        assert!(d.room_at(d.room_count()).is_none());
    }

    #[test]
    fn test_room_containing_finds_center_cell() {
        let mut rng = SplitMix64::new(2);
        let d = generate_dungeon(40, 30, &mut rng, GenParams::default());
        for r in &d.rooms {
            let (cx, cy) = r.center();
            assert!(
                d.room_containing(cx, cy).is_some(),
                "center ({cx},{cy}) of room should be inside a room"
            );
        }
    }

    #[test]
    fn test_room_containing_negative_coord_returns_none() {
        let mut rng = SplitMix64::new(3);
        let d = generate_dungeon(40, 30, &mut rng, GenParams::default());
        assert!(d.room_containing(-1, 5).is_none());
    }

    // --- DetHash rooms field ---

    #[test]
    fn test_det_hash_rooms_included_in_hash() {
        // Adding a room to `rooms` (without touching tiles) must change the hash.
        // Before the fix, Dungeon::det_hash only folded the tile bitmap, so the
        // rooms registry was invisible to replay checksums.
        let mut rng = SplitMix64::new(42);
        let mut d = generate_dungeon(30, 20, &mut rng, GenParams::default());
        let hash_before = hash_state(&d);
        d.rooms.push(Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        });
        let hash_after = hash_state(&d);
        assert_ne!(
            hash_before, hash_after,
            "adding a room to the registry must change the DetHash"
        );
    }

    #[test]
    fn test_det_hash_identical_dungeons_agree() {
        // Two dungeons generated from the same seed must hash identically.
        let mut r1 = SplitMix64::new(7);
        let mut r2 = SplitMix64::new(7);
        let d1 = generate_dungeon(25, 20, &mut r1, GenParams::default());
        let d2 = generate_dungeon(25, 20, &mut r2, GenParams::default());
        assert_eq!(hash_state(&d1), hash_state(&d2));
    }

    #[test]
    fn test_det_hash_different_seeds_differ() {
        // Different seeds produce different dungeons with different hashes.
        let mut r1 = SplitMix64::new(1);
        let mut r2 = SplitMix64::new(2);
        let d1 = generate_dungeon(25, 20, &mut r1, GenParams::default());
        let d2 = generate_dungeon(25, 20, &mut r2, GenParams::default());
        assert_ne!(hash_state(&d1), hash_state(&d2));
    }

    // --- generate_drunkard (drunkard's walk / 穴掘り法) ---

    fn drunkard_is_connected(d: &Dungeon) -> bool {
        let start = d.floor_cells().into_iter().next();
        let Some(start) = start else {
            return true; // no floor → trivially "connected"
        };
        let reach = dijkstra_map(&[start], i32::MAX, |x, y| d.is_wall(x, y));
        d.floor_cells().iter().all(|c| reach.contains_key(c))
    }

    #[test]
    fn test_drunkard_same_seed_is_byte_identical() {
        let mut r1 = SplitMix64::new(0xD161_0001);
        let mut r2 = SplitMix64::new(0xD161_0001);
        let a = generate_drunkard(60, 40, &mut r1, DrunkardParams::default());
        let b = generate_drunkard(60, 40, &mut r2, DrunkardParams::default());
        assert_eq!(hash_state(&a), hash_state(&b), "same seed → identical map");
    }

    #[test]
    fn test_drunkard_is_always_fully_connected() {
        // The headline guarantee: a continuous digger yields a single region.
        for seed in 0..20u64 {
            let mut rng = SplitMix64::new(0xD161_1000 + seed);
            let d = generate_drunkard(50, 30, &mut rng, DrunkardParams::default());
            assert!(
                drunkard_is_connected(&d),
                "drunkard's walk must be fully connected (seed {seed})"
            );
        }
    }

    #[test]
    fn test_drunkard_border_is_solid_wall() {
        let mut rng = SplitMix64::new(0xD161_0003);
        let d = generate_drunkard(40, 25, &mut rng, DrunkardParams::default());
        let (w, h) = (d.width() as i32, d.height() as i32);
        for x in 0..w {
            assert!(
                d.is_wall(x, 0) && d.is_wall(x, h - 1),
                "top/bottom border wall"
            );
        }
        for y in 0..h {
            assert!(
                d.is_wall(0, y) && d.is_wall(w - 1, y),
                "left/right border wall"
            );
        }
    }

    #[test]
    fn test_drunkard_reaches_target_fill_on_open_map() {
        // On a roomy map the walk should reach (about) the requested fill.
        let mut rng = SplitMix64::new(0xD161_0004);
        let d = generate_drunkard(
            60,
            40,
            &mut rng,
            DrunkardParams {
                fill_percent: 30,
                max_steps: 0,
            },
        );
        let interior = (60 - 2) * (40 - 2);
        let floor = d.floor_cells().len() as u32;
        let target = interior * 30 / 100;
        assert!(
            floor >= target,
            "carved {floor} should reach target {target}"
        );
    }

    #[test]
    fn test_drunkard_respects_max_steps_cap() {
        // A tiny step cap stops early; the map stays mostly wall but never panics
        // and is still connected.
        let mut rng = SplitMix64::new(0xD161_0005);
        let d = generate_drunkard(
            60,
            40,
            &mut rng,
            DrunkardParams {
                fill_percent: 90,
                max_steps: 10,
            },
        );
        let floor = d.floor_cells().len() as u32;
        assert!(
            floor <= 11,
            "at most start + 10 steps of new floor (got {floor})"
        );
        assert!(drunkard_is_connected(&d), "even a capped walk is connected");
    }

    #[test]
    fn test_drunkard_tiny_map_returns_all_wall_without_panicking() {
        let mut rng = SplitMix64::new(7);
        let d = generate_drunkard(2, 2, &mut rng, DrunkardParams::default());
        assert!(d.floor_cells().is_empty(), "maps < 3×3 are all wall");
    }

    #[test]
    fn test_drunkard_carries_no_rectangular_rooms() {
        let mut rng = SplitMix64::new(0xD161_0006);
        let d = generate_drunkard(50, 30, &mut rng, DrunkardParams::default());
        assert!(d.rooms.is_empty(), "a cavern carries no Rect rooms");
    }

    // --- extra_loops (cyclic connectivity) ---

    #[test]
    fn test_extra_loops_zero_is_byte_identical_to_default() {
        // The determinism guarantee: extra_loops == 0 must draw zero times from
        // the RNG in the loop phase, so the map is byte-identical to the
        // original chain generator for the same seed.
        let default_params = GenParams::default();
        let explicit_zero = GenParams {
            extra_loops: 0,
            ..GenParams::default()
        };
        let a = generate_dungeon(60, 40, &mut SplitMix64::new(0x5EED), default_params);
        let b = generate_dungeon(60, 40, &mut SplitMix64::new(0x5EED), explicit_zero);
        assert_eq!(a, b, "extra_loops:0 must reproduce the default map exactly");
    }

    #[test]
    fn test_extra_loops_adds_floor_cells() {
        // Carving extra corridors can only add floor (carving is idempotent and
        // never removes floor), so a looped dungeon has >= as many floor cells
        // as the chain one, and strictly more when the loops hit fresh wall.
        let seed = 0x1009_2003;
        let chain = generate_dungeon(60, 40, &mut SplitMix64::new(seed), GenParams::default());
        let looped = generate_dungeon(
            60,
            40,
            &mut SplitMix64::new(seed),
            GenParams {
                extra_loops: 6,
                ..GenParams::default()
            },
        );
        assert!(
            looped.floor_cells().len() >= chain.floor_cells().len(),
            "extra loops never remove floor"
        );
        // The two maps must differ (loops changed something), given enough rooms.
        if chain.rooms.len() >= 3 {
            assert_ne!(chain, looped, "6 extra loops should change a >=3-room map");
        }
    }

    #[test]
    fn test_extra_loops_deterministic() {
        let params = GenParams {
            extra_loops: 5,
            ..GenParams::default()
        };
        let a = generate_dungeon(60, 40, &mut SplitMix64::new(77), params);
        let b = generate_dungeon(60, 40, &mut SplitMix64::new(77), params);
        assert_eq!(a, b, "looped generation is deterministic for a fixed seed");
    }

    #[test]
    fn test_extra_loops_preserves_room_set() {
        // Loops carve corridors between existing rooms; they must not add,
        // remove, or move any Rect room.
        let seed = 0xB004;
        let chain = generate_dungeon(50, 50, &mut SplitMix64::new(seed), GenParams::default());
        let looped = generate_dungeon(
            50,
            50,
            &mut SplitMix64::new(seed),
            GenParams {
                extra_loops: 4,
                ..GenParams::default()
            },
        );
        assert_eq!(
            chain.rooms, looped.rooms,
            "loops must not alter the room set"
        );
    }

    #[test]
    fn test_extra_loops_noop_below_three_rooms() {
        // With < 3 rooms there is no non-adjacent pair to link, so the loop
        // phase is a no-op and output matches extra_loops:0. Force few rooms
        // with a tiny map that fits at most a couple.
        let seed = 0x3;
        let params_loops = GenParams {
            max_rooms: 2,
            min_room: 3,
            max_room: 4,
            extra_loops: 5,
        };
        let params_none = GenParams {
            extra_loops: 0,
            ..params_loops
        };
        let with_loops = generate_dungeon(14, 10, &mut SplitMix64::new(seed), params_loops);
        let without = generate_dungeon(14, 10, &mut SplitMix64::new(seed), params_none);
        if with_loops.rooms.len() < 3 {
            assert_eq!(with_loops, without, "loop phase is a no-op below 3 rooms");
        }
    }

    // --- keep_largest_region ---

    /// Every floor cell reachable from every other floor cell (8-way, no corner
    /// cutting), via the pathfinding oracle — the guarantee keep_largest_region
    /// must establish.
    fn all_floor_mutually_reachable(d: &Dungeon) -> bool {
        let floors = d.floor_cells();
        let Some(&first) = floors.first() else {
            return true; // vacuously true: no floor
        };
        floors
            .iter()
            .all(|&c| crate::pathfinding::is_reachable(first, c, |x, y| d.is_wall(x, y)))
    }

    #[test]
    fn test_keep_largest_region_makes_map_fully_connected() {
        // Caves routinely have isolated pockets; after the filter every
        // remaining floor cell must be mutually reachable.
        for seed in 0..30u64 {
            let mut rng = SplitMix64::new(0xCA7E_0000 + seed);
            let mut d = generate_cave(48, 32, &mut rng, CaveParams::default());
            d.keep_largest_region();
            assert!(
                all_floor_mutually_reachable(&d),
                "seed {seed}: floor not fully connected after keep_largest_region"
            );
        }
    }

    #[test]
    fn test_keep_largest_region_hand_built_two_pockets() {
        // A 5x1 strip split by a wall into a 2-cell region and a 2-cell region;
        // the larger-or-equal one (lowest id = leftmost) is kept.
        // Layout: floor floor WALL floor floor  → two 2-cell regions, tie → keep id 0 (left).
        let mut d = Dungeon::filled(5, 1);
        d.carve(0, 0);
        d.carve(1, 0);
        d.carve(3, 0);
        d.carve(4, 0);
        // both regions size 2 → tie broken toward lowest id (left region).
        let filled = d.keep_largest_region();
        assert_eq!(filled, 2, "the non-kept 2-cell region is walled off");
        assert!(d.is_floor(0, 0) && d.is_floor(1, 0), "left region kept");
        assert!(d.is_wall(3, 0) && d.is_wall(4, 0), "right region culled");
    }

    #[test]
    fn test_keep_largest_region_keeps_strictly_larger_region() {
        // Left region 1 cell, right region 3 cells → the 3-cell region wins.
        let mut d = Dungeon::filled(6, 1);
        d.carve(0, 0); // lone cell
        d.carve(2, 0);
        d.carve(3, 0);
        d.carve(4, 0); // 3-cell region
        let filled = d.keep_largest_region();
        assert_eq!(filled, 1, "the lone cell is culled");
        assert!(d.is_wall(0, 0));
        assert!(d.is_floor(2, 0) && d.is_floor(3, 0) && d.is_floor(4, 0));
    }

    #[test]
    fn test_keep_largest_region_already_connected_is_noop() {
        let mut rng = SplitMix64::new(7);
        let mut d = generate_drunkard(40, 25, &mut rng, DrunkardParams::default());
        assert!(drunkard_is_connected(&d), "drunkard is already connected");
        let before = d.clone();
        let filled = d.keep_largest_region();
        assert_eq!(filled, 0, "a single-region map is unchanged");
        assert_eq!(d, before, "byte-identical when nothing to cull");
    }

    #[test]
    fn test_keep_largest_region_all_wall_is_noop() {
        let mut d = Dungeon::filled(8, 8);
        assert_eq!(d.keep_largest_region(), 0, "no floor → nothing to do");
    }

    #[test]
    fn test_keep_largest_region_is_deterministic() {
        let build = || {
            let mut rng = SplitMix64::new(0xBEEF);
            let mut d = generate_cave(40, 30, &mut rng, CaveParams::default());
            d.keep_largest_region();
            d
        };
        assert_eq!(build(), build());
    }

    #[test]
    fn test_keep_largest_region_drops_culled_rooms() {
        // Two rooms far apart with NO connecting corridor, hand-built so one
        // region is strictly larger. The room in the culled region must be
        // removed from the room list.
        let mut d = Dungeon::filled(20, 5);
        let big = Rect {
            x: 1,
            y: 1,
            w: 6,
            h: 3,
        };
        let small = Rect {
            x: 15,
            y: 1,
            w: 2,
            h: 2,
        };
        d.carve_room(&big);
        d.carve_room(&small);
        d.rooms.push(big);
        d.rooms.push(small);
        assert_eq!(d.rooms.len(), 2);
        let filled = d.keep_largest_region();
        assert!(filled > 0, "the small isolated room is culled");
        assert_eq!(d.rooms.len(), 1, "the culled room is dropped from the list");
        assert_eq!(d.rooms[0], big, "the surviving room is the large one");
    }

    // --- MapBuilder ---

    #[test]
    fn test_mapbuilder_empty_pipeline_is_identity() {
        let mut rng = SplitMix64::new(1);
        let base = generate_cave(30, 20, &mut rng, CaveParams::default());
        let built = MapBuilder::new(base.clone()).build(1);
        assert_eq!(built, base, "no stages → dungeon unchanged");
    }

    #[test]
    fn test_mapbuilder_keep_largest_region_stage_connects_map() {
        for seed in 0..20u64 {
            let mut rng = SplitMix64::new(0x3EED + seed);
            let cave = generate_cave(48, 32, &mut rng, CaveParams::default());
            let built = MapBuilder::new(cave).keep_largest_region().build(seed);
            assert!(
                all_floor_mutually_reachable(&built),
                "seed {seed}: builder pipeline must leave a single connected region"
            );
        }
    }

    #[test]
    fn test_mapbuilder_is_deterministic() {
        let build = || {
            let mut rng = SplitMix64::new(7);
            let cave = generate_cave(40, 30, &mut rng, CaveParams::default());
            MapBuilder::new(cave)
                .stage(|d, rng| {
                    // carve a few random cells from this stage's own stream
                    for _ in 0..10 {
                        let x = rng.below(d.width()) as i32;
                        let y = rng.below(d.height()) as i32;
                        d.carve(x, y);
                    }
                })
                .keep_largest_region()
                .build(0xC0DE)
        };
        assert_eq!(
            build(),
            build(),
            "same start + base_seed → identical output"
        );
    }

    #[test]
    fn test_mapbuilder_stage_count() {
        let b = MapBuilder::new(Dungeon::filled(4, 4))
            .stage(|_, _| {})
            .keep_largest_region();
        assert_eq!(b.stage_count(), 2);
    }

    #[test]
    fn test_mapbuilder_stages_have_isolated_rng_streams() {
        // The headline property: how many times an EARLIER stage draws must not
        // change a LATER stage's stream. Two pipelines whose stage-0 draws a
        // different number of times, but whose stage-1 carves from its own
        // stream, must produce the identical carve.
        let make = |stage0_draws: u32| {
            MapBuilder::new(Dungeon::filled(30, 1))
                .stage(move |_d, rng| {
                    for _ in 0..stage0_draws {
                        rng.below(1000);
                    }
                })
                .stage(|d, rng| {
                    let x = rng.below(d.width()) as i32;
                    d.carve(x, 0);
                })
                .build(0xABCD)
        };
        let a = make(1);
        let b = make(500);
        assert_eq!(
            a, b,
            "stage 1's stream is independent of stage 0's draw count"
        );
        // And the carve actually happened (stage 1 ran).
        assert!(a.floor_cells().len() == 1, "exactly one cell carved");
    }

    #[test]
    fn test_mapbuilder_base_seed_changes_output() {
        // build() must actually thread base_seed into the stages: across a
        // range of seeds the single-carve output cannot be constant. (Any one
        // pair could collide by chance, so assert over many seeds instead.)
        let build = |seed: u64| {
            MapBuilder::new(Dungeon::filled(30, 1))
                .stage(|d, rng| {
                    let x = rng.below(d.width()) as i32;
                    d.carve(x, 0);
                })
                .build(seed)
        };
        let first = build(0);
        let differs = (1..30u64).any(|s| build(s) != first);
        assert!(differs, "output must depend on base_seed");
    }
}
