//! Performance benchmarks — timing-based, no external dependencies.
//!
//! Run: `cargo test -p izanagi_kit --test bench -- --nocapture`
//!
//! Head-to-head measurements backing the N18 decision in RESEARCH.md: does
//! archetype-packed storage (`ArchTable`) beat per-component `SparseSet`
//! columns + `join`/`join3` enough to justify migrating the kit's storage?
//! Scenario mix mirrors a typical roguelike tick: every entity has `Pos`,
//! ~60% have `Vel`, ~40% have `Hp` — so multi-component systems touch a
//! subset, exactly where archetype packing claims its cache win.

use izanagi_kit::{
    entity::EntityAllocator,
    sparse_set::{join, join3, join_mut, SparseSet},
    ArchTable, Entity,
};
use std::time::Instant;

// ─────────────────────────────────────────────────────────────────
// Harness (same shape as izanagi/tests/bench.rs)
// ─────────────────────────────────────────────────────────────────

fn bench(name: &str, iterations: usize, warmup: usize, mut f: impl FnMut()) {
    for _ in 0..warmup {
        f();
    }

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let t = Instant::now();
        f();
        samples.push(t.elapsed().as_nanos() as u64);
    }

    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let p99 = samples[samples.len() * 99 / 100];
    let min = samples[0];
    println!("[{name:44}] min={min:7}ns  median={median:7}ns  p99={p99:7}ns");
}

// ─────────────────────────────────────────────────────────────────
// Workload
// ─────────────────────────────────────────────────────────────────

const N: usize = 10_000;
const N_VEL: usize = 6_000;
const N_HP: usize = 4_000;
const ITERS: usize = 200;

#[derive(Clone, Copy)]
struct Pos {
    x: i32,
    y: i32,
}
#[derive(Clone, Copy)]
struct Vel {
    x: i32,
    y: i32,
}
#[derive(Clone, Copy)]
struct Hp {
    cur: i32,
    max: i32,
}

#[derive(Clone, Copy)]
struct Pv {
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
}
#[derive(Clone, Copy)]
struct Pvh {
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
    hp: i32,
    max: i32,
}

fn entities() -> Vec<Entity> {
    let mut alloc = EntityAllocator::new();
    alloc.batch_alloc(N)
}

// ─────────────────────────────────────────────────────────────────
// Single-column iteration
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_iter_column_vs_table() {
    let es = entities();
    let mut pos: SparseSet<Pos> = SparseSet::new();
    let mut arch: ArchTable<Pos> = ArchTable::new();
    for (i, &e) in es.iter().enumerate() {
        let p = Pos {
            x: i as i32,
            y: -(i as i32),
        };
        pos.insert(e, p);
        arch.insert(e, p);
    }

    bench("iter: SparseSet.values x10k", ITERS, 20, || {
        let mut s: i64 = 0;
        for v in pos.values() {
            s += v.x as i64 + v.y as i64;
        }
        std::hint::black_box(s);
    });
    bench("iter: ArchTable.values x10k", ITERS, 20, || {
        let mut s: i64 = 0;
        for v in arch.values() {
            s += v.x as i64 + v.y as i64;
        }
        std::hint::black_box(s);
    });
}

// ─────────────────────────────────────────────────────────────────
// Two-component join: the archetype's claimed win
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_join2_vs_table() {
    let es = entities();
    let mut pos: SparseSet<Pos> = SparseSet::new();
    let mut vel: SparseSet<Vel> = SparseSet::new();
    let mut arch: ArchTable<Pv> = ArchTable::new();
    for (i, &e) in es.iter().enumerate() {
        let p = Pos {
            x: i as i32,
            y: i as i32,
        };
        pos.insert(e, p);
        if i < N_VEL {
            let v = Vel { x: 1, y: -1 };
            vel.insert(e, v);
            arch.insert(
                e,
                Pv {
                    x: p.x,
                    y: p.y,
                    vx: v.x,
                    vy: v.y,
                },
            );
        }
    }

    bench("join2: sparse_set::join x6k", ITERS, 20, || {
        let mut s: i64 = 0;
        for (_, p, v) in join(&pos, &vel) {
            s += p.x as i64 + p.y as i64 + v.x as i64 + v.y as i64;
        }
        std::hint::black_box(s);
    });
    bench("join2: ArchTable.iter x6k", ITERS, 20, || {
        let mut s: i64 = 0;
        for (_, r) in arch.iter() {
            s += r.x as i64 + r.y as i64 + r.vx as i64 + r.vy as i64;
        }
        std::hint::black_box(s);
    });
}

// ─────────────────────────────────────────────────────────────────
// Three-component join
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_join3_vs_table() {
    let es = entities();
    let mut pos: SparseSet<Pos> = SparseSet::new();
    let mut vel: SparseSet<Vel> = SparseSet::new();
    let mut hp: SparseSet<Hp> = SparseSet::new();
    let mut arch: ArchTable<Pvh> = ArchTable::new();
    for (i, &e) in es.iter().enumerate() {
        pos.insert(e, Pos { x: i as i32, y: 0 });
        if i < N_VEL {
            vel.insert(e, Vel { x: 1, y: 2 });
        }
        if i < N_HP {
            hp.insert(e, Hp { cur: 5, max: 9 });
        }
        if i < N_HP {
            arch.insert(
                e,
                Pvh {
                    x: i as i32,
                    y: 0,
                    vx: 1,
                    vy: 2,
                    hp: 5,
                    max: 9,
                },
            );
        }
    }

    bench("join3: sparse_set::join3 x4k", ITERS, 20, || {
        let mut s: i64 = 0;
        for (_, p, v, h) in join3(&pos, &vel, &hp) {
            s += p.x as i64 + p.y as i64 + v.x as i64 + v.y as i64 + h.cur as i64 + h.max as i64;
        }
        std::hint::black_box(s);
    });
    bench("join3: ArchTable.iter x4k", ITERS, 20, || {
        let mut s: i64 = 0;
        for (_, r) in arch.iter() {
            s += r.x as i64 + r.y as i64 + r.vx as i64 + r.vy as i64 + r.hp as i64 + r.max as i64;
        }
        std::hint::black_box(s);
    });
}

// ─────────────────────────────────────────────────────────────────
// Mutable join: movement system
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_join_mut_vs_table_mut() {
    let es = entities();
    let mut pos: SparseSet<Pos> = SparseSet::new();
    let mut vel: SparseSet<Vel> = SparseSet::new();
    let mut arch: ArchTable<Pv> = ArchTable::new();
    for (i, &e) in es.iter().enumerate() {
        pos.insert(e, Pos { x: 0, y: 0 });
        if i < N_VEL {
            vel.insert(e, Vel { x: 1, y: -1 });
            arch.insert(
                e,
                Pv {
                    x: 0,
                    y: 0,
                    vx: 1,
                    vy: -1,
                },
            );
        }
    }

    bench("join_mut: sparse_set x6k", ITERS, 20, || {
        for (_, p, v) in join_mut(&mut pos, &vel) {
            p.x += v.x;
            p.y += v.y;
        }
    });
    bench("iter_mut: ArchTable x6k", ITERS, 20, || {
        for (_, r) in arch.iter_mut() {
            r.x += r.vx;
            r.y += r.vy;
        }
    });
}

// ─────────────────────────────────────────────────────────────────
// Point lookup and churn
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_point_lookup() {
    let es = entities();
    let mut pos: SparseSet<Pos> = SparseSet::new();
    let mut arch: ArchTable<Pos> = ArchTable::new();
    for (i, &e) in es.iter().enumerate() {
        pos.insert(e, Pos { x: i as i32, y: 0 });
        arch.insert(e, Pos { x: i as i32, y: 0 });
    }
    // Stride order — point lookups hit scattered slots, not insertion order.
    let probe: Vec<Entity> = (0..N).map(|i| es[(i * 7) % N]).collect();

    bench("get: SparseSet x10k scattered", ITERS, 20, || {
        let mut s: i64 = 0;
        for &e in &probe {
            if let Some(p) = pos.get(e) {
                s += p.x as i64;
            }
        }
        std::hint::black_box(s);
    });
    bench("get: ArchTable x10k scattered", ITERS, 20, || {
        let mut s: i64 = 0;
        for &e in &probe {
            if let Some(p) = arch.get(e) {
                s += p.x as i64;
            }
        }
        std::hint::black_box(s);
    });
}

#[test]
fn bench_churn_remove_insert() {
    let es = entities();
    let mut pos: SparseSet<Pos> = SparseSet::new();
    let mut arch: ArchTable<Pos> = ArchTable::new();
    for (i, &e) in es.iter().enumerate() {
        pos.insert(e, Pos { x: i as i32, y: 0 });
        arch.insert(e, Pos { x: i as i32, y: 0 });
    }

    // Each iteration removes then reinserts the same entity — constant size,
    // isolating the swap-remove + reinsert cost of one churn step.
    let mut c1 = 0usize;
    bench("churn: SparseSet remove+insert", ITERS * 100, 100, || {
        let e = es[c1 % N];
        c1 += 1;
        let v = pos.remove(e).unwrap();
        pos.insert(e, v);
    });
    let mut c2 = 0usize;
    bench("churn: ArchTable remove+insert", ITERS * 100, 100, || {
        let e = es[c2 % N];
        c2 += 1;
        let v = arch.remove(e).unwrap();
        arch.insert(e, v);
    });
}
