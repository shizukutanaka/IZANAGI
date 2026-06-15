//! End-to-end roguelike simulation determinism proof.
//!
//! Where `determinism.rs` pins the *core* stack (RNG + fixed-point + storage +
//! hash), this test proves the **higher-level modules compose deterministically**:
//! a realistic turn-based roguelike loop wiring together `mapgen`, `passability`,
//! `pathfinding`, `fov`, `turn`, and `combat`. The claim is the same — identical
//! seed yields a bit-identical per-turn hash trace — but exercised across the
//! gameplay modules that were added independently.
//!
//! The simulation: generate a dungeon, place a player and several monsters in
//! room centres, then run a turn scheduler. Each monster turn the monster
//! A*-paths toward the player (avoiding walls and other monsters) and melees on
//! contact; the player retaliates against the lowest-id adjacent monster. After
//! every turn the world hash folds in all positions, combatant HP, scheduler
//! state, and the count of cells visible to the player (symmetric FOV).

use izanagi_kit::world_hash::Fnv1a;
use izanagi_kit::{
    astar, compute_fov, generate_dungeon, melee_attack, GenParams, PassabilityGrid, Scheduler,
    SplitMix64, Stats,
};
use std::collections::BTreeMap;

const PLAYER: u32 = 0;
const FOV_RADIUS: i32 = 8;

struct Actor {
    pos: (i32, i32),
    stats: Stats,
}

struct Sim {
    grid: PassabilityGrid,
    actors: BTreeMap<u32, Actor>,
    scheduler: Scheduler<u32>,
}

impl Sim {
    fn new(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let dungeon = generate_dungeon(48, 32, &mut rng, GenParams::default());
        let grid = PassabilityGrid::from_dungeon(&dungeon);

        let mut actors = BTreeMap::new();
        let mut scheduler = Scheduler::new();

        // Player in the first room; monsters in subsequent room centres.
        if let Some(room) = dungeon.rooms.first() {
            actors.insert(
                PLAYER,
                Actor {
                    pos: room.center(),
                    stats: Stats::new(40, 8, 3),
                },
            );
            scheduler.add(PLAYER, 10);
        }
        for (i, room) in dungeon.rooms.iter().enumerate().skip(1) {
            let id = i as u32;
            actors.insert(
                id,
                Actor {
                    pos: room.center(),
                    // Varied speeds and stats keep the schedule non-trivial.
                    stats: Stats::new(12 + (i as i32 % 5), 4, 1),
                },
            );
            scheduler.add(id, 7 + (i as i32 % 4));
        }

        Sim {
            grid,
            actors,
            scheduler,
        }
    }

    /// Chebyshev-adjacent (8-way) including diagonals.
    fn adjacent(a: (i32, i32), b: (i32, i32)) -> bool {
        let dx = (a.0 - b.0).abs();
        let dy = (a.1 - b.1).abs();
        dx <= 1 && dy <= 1 && (dx + dy) != 0
    }

    /// Is `(x, y)` occupied by any actor other than `mover`?
    fn occupied_by_other(&self, mover: u32, x: i32, y: i32) -> bool {
        self.actors
            .iter()
            .any(|(&id, a)| id != mover && a.pos == (x, y))
    }

    fn take_turn(&mut self, id: u32) {
        let actor_pos = match self.actors.get(&id) {
            Some(a) => a.pos,
            None => return, // already dead
        };

        if id == PLAYER {
            // Retaliate against the lowest-id adjacent monster.
            let target = self
                .actors
                .iter()
                .filter(|(&oid, a)| oid != PLAYER && Self::adjacent(actor_pos, a.pos))
                .map(|(&oid, _)| oid)
                .next();
            if let Some(tid) = target {
                self.resolve_attack(PLAYER, tid);
            }
            return;
        }

        // Monster: if adjacent to the player, attack; else step toward them.
        let player_pos = match self.actors.get(&PLAYER) {
            Some(p) => p.pos,
            None => return, // player gone; nothing to do
        };

        if Self::adjacent(actor_pos, player_pos) {
            self.resolve_attack(id, PLAYER);
            return;
        }

        // Path toward the player. Treat walls and other actors as blocked, but
        // allow the goal cell (the player) so a path can be found.
        let path = astar(actor_pos, player_pos, |x, y| {
            if (x, y) == player_pos {
                return false;
            }
            self.grid.is_blocked(x, y) || self.occupied_by_other(id, x, y)
        });
        if let Some(path) = path {
            // path[0] is the current cell; step to path[1] if it exists and is
            // not the player's cell (we don't step onto the player).
            if path.len() >= 2 && path[1] != player_pos {
                let next = path[1];
                if let Some(a) = self.actors.get_mut(&id) {
                    a.pos = next;
                }
            }
        }
    }

    fn resolve_attack(&mut self, attacker: u32, defender: u32) {
        let atk_stats = match self.actors.get(&attacker) {
            Some(a) => a.stats.clone(),
            None => return,
        };
        let mut def_stats = match self.actors.get(&defender) {
            Some(d) => d.stats.clone(),
            None => return,
        };
        melee_attack(&atk_stats, &mut def_stats);
        let dead = !def_stats.is_alive();
        if let Some(d) = self.actors.get_mut(&defender) {
            d.stats = def_stats;
        }
        if dead {
            self.actors.remove(&defender);
            self.scheduler.remove(defender);
        }
    }

    /// Count cells visible to the player via symmetric FOV.
    fn player_visible_count(&self) -> u32 {
        let player_pos = match self.actors.get(&PLAYER) {
            Some(p) => p.pos,
            None => return 0,
        };
        let mut count = 0u32;
        compute_fov(
            player_pos,
            FOV_RADIUS,
            |x, y| self.grid.is_blocked(x, y),
            |_x, _y| count += 1,
        );
        count
    }

    /// Canonical world checksum: actor positions + HP (id order), scheduler
    /// state, and the player's visible-cell count.
    fn hash(&self) -> u64 {
        let mut h = Fnv1a::new();
        // BTreeMap iterates in ascending key order — canonical by construction.
        for (&id, a) in &self.actors {
            h.write_u32(id);
            h.write_i32(a.pos.0);
            h.write_i32(a.pos.1);
            h.write_i32(a.stats.hp);
        }
        self.scheduler.det_hash(&mut h);
        h.write_u32(self.player_visible_count());
        h.finish()
    }

    fn step(&mut self) {
        if let Some(id) = self.scheduler.next_turn() {
            self.take_turn(id);
        }
    }
}

fn run(seed: u64, turns: usize) -> Vec<u64> {
    let mut sim = Sim::new(seed);
    let mut trace = Vec::with_capacity(turns);
    for _ in 0..turns {
        sim.step();
        trace.push(sim.hash());
    }
    trace
}

#[test]
fn test_roguelike_same_seed_identical_trace() {
    let a = run(0x5EED1234, 400);
    let b = run(0x5EED1234, 400);
    assert_eq!(a, b, "identical seed must replay the full sim bit-for-bit");
}

#[test]
fn test_roguelike_different_seed_diverges() {
    let a = run(0xAAAA, 400);
    let b = run(0xBBBB, 400);
    assert_ne!(a, b, "different dungeons should not coincide");
}

#[test]
fn test_roguelike_sim_is_nontrivial() {
    // The simulation must actually evolve (movement, combat, FOV changes),
    // otherwise the determinism assertions pass vacuously.
    let trace = run(0x5EED1234, 400);
    let unique: std::collections::HashSet<u64> = trace.iter().copied().collect();
    assert!(
        unique.len() > 50,
        "sim must evolve: only {} unique states",
        unique.len()
    );
}

#[test]
fn test_roguelike_final_hash_is_pinned() {
    // Regression tripwire for the *composition* of mapgen + passability +
    // pathfinding + fov + turn + combat. Any accidental change to those
    // modules' behaviour or ordering breaks this.
    let trace = run(0x5EED1234, 400);
    assert_eq!(*trace.last().unwrap(), PINNED_ROGUELIKE_HASH);
}

// Pinned from a verified run; regression tripwire for the gameplay stack.
const PINNED_ROGUELIKE_HASH: u64 = 0x5286d1420200fe66;
