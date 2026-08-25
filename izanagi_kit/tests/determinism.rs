//! End-to-end determinism proof.
//!
//! Runs a tiny simulation (entities with fixed-point position + velocity,
//! spawn/despawn driven by a seeded RNG) and records the FNV-1a world hash at
//! every frame. The core claim: for identical (seed, frame count), the entire
//! hash sequence is bit-identical across runs — the foundation of replay,
//! lockstep, and snapshot-as-checksum CI.

use izanagi_kit::{DetHash, Entity, EntityAllocator, Fixed, Fnv1a, SparseSet, SplitMix64};

#[derive(Clone, Copy)]
struct Position {
    x: Fixed,
    y: Fixed,
}

#[derive(Clone, Copy)]
struct Velocity {
    dx: Fixed,
    dy: Fixed,
}

impl DetHash for Position {
    fn det_hash(&self, h: &mut Fnv1a) {
        h.write_i32(self.x.raw());
        h.write_i32(self.y.raw());
    }
}

struct World {
    alloc: EntityAllocator,
    positions: SparseSet<Position>,
    velocities: SparseSet<Velocity>,
    live: Vec<Entity>,
    rng: SplitMix64,
}

impl World {
    fn new(seed: u64) -> Self {
        Self {
            alloc: EntityAllocator::new(),
            positions: SparseSet::new(),
            velocities: SparseSet::new(),
            live: Vec::new(),
            rng: SplitMix64::new(seed),
        }
    }

    fn spawn(&mut self) {
        let e = self.alloc.allocate();
        let vx = self.rng.below(7) as i32 - 3;
        let vy = self.rng.below(7) as i32 - 3;
        self.positions.insert(
            e,
            Position {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
        );
        self.velocities.insert(
            e,
            Velocity {
                dx: Fixed::from_int(vx),
                dy: Fixed::from_int(vy),
            },
        );
        self.live.push(e);
    }

    fn despawn_oldest(&mut self) {
        if self.live.is_empty() {
            return;
        }
        let e = self.live.remove(0);
        self.positions.remove(e);
        self.velocities.remove(e);
        self.alloc.free(e);
    }

    fn integrate(&mut self) {
        // Iterate in canonical (sorted) order so iteration order is never a
        // determinism hazard, then apply velocity to position.
        let updates: Vec<(Entity, Velocity)> = self
            .velocities
            .iter_sorted()
            .iter()
            .map(|(e, v)| (*e, **v))
            .collect();
        for (e, v) in updates {
            if let Some(p) = self.positions.get_mut(e) {
                p.x = p.x + v.dx;
                p.y = p.y + v.dy;
            }
        }
    }

    fn step(&mut self) {
        // RNG-driven population churn, then physics. Fixed op order.
        if self.rng.below(100) < 60 {
            self.spawn();
        }
        if self.rng.below(100) < 25 {
            self.despawn_oldest();
        }
        self.integrate();
    }

    /// Canonical state checksum: sorted positions + entity ids + RNG stream.
    fn hash(&self) -> u64 {
        let mut h = Fnv1a::new();
        for (e, p) in self.positions.iter_sorted() {
            h.write_u32(e.index());
            h.write_u32(e.generation());
            p.det_hash(&mut h);
        }
        h.write_u64(self.rng.state());
        h.finish()
    }
}

fn run(seed: u64, frames: usize) -> Vec<u64> {
    let mut world = World::new(seed);
    let mut trace = Vec::with_capacity(frames);
    for _ in 0..frames {
        world.step();
        trace.push(world.hash());
    }
    trace
}

#[test]
fn test_same_seed_yields_bit_identical_hash_trace() {
    let a = run(0xC0FFEE00, 500);
    let b = run(0xC0FFEE00, 500);
    assert_eq!(a, b, "identical inputs must replay bit-for-bit");
}

#[test]
fn test_different_seed_diverges_somewhere() {
    let a = run(1, 500);
    let b = run(2, 500);
    assert_ne!(a, b, "different seeds should not coincide");
}

#[test]
fn test_trace_is_nontrivial() {
    // Guards against a degenerate sim where nothing happens (which would make
    // the determinism test pass vacuously).
    let trace = run(7, 200);
    let unique: std::collections::HashSet<u64> = trace.iter().copied().collect();
    assert!(unique.len() > 50, "simulation must actually evolve state");
}

#[test]
fn test_final_hash_is_pinned() {
    // Pins the whole stack (RNG + fixed-point + storage + hash). Any accidental
    // change to algorithm or order breaks this — the regression tripwire.
    let trace = run(0xC0FFEE00, 500);
    assert_eq!(*trace.last().unwrap(), PINNED_FINAL_HASH);
}

// Pinned from a verified run; regression tripwire for the whole stack.
const PINNED_FINAL_HASH: u64 = 0xd1a9236e96a2c802;
