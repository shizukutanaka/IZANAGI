//! `izanagi_kit` — zero-dependency building blocks for a game simulation that
//! **replays bit-identically**, and the tools to prove yours does.
//!
//! Eleven of these modules do nothing but interrogate a simulation. They audit
//! it for nondeterminism, search its state space for one that breaks an
//! invariant, *prove* no such state exists, minimise any counterexample to its
//! shortest form, monitor properties that span time rather than one tick, and
//! check that saving and reloading changes nothing. The rest of the crate is
//! the deterministic substrate they rest on — fixed-point maths, seeded RNG,
//! world hashing — plus ordinary game systems written so they stay hashable
//! and replay-safe.
//!
//! Run `cargo run --example verify_pipeline_demo` first: it puts one small
//! simulation with a planted bug through the entire pipeline.
//!
//! # How to read this crate
//!
//! Everything here serves one promise: **a simulation that replays
//! bit-identically**. Not every module carries the same weight in keeping that
//! promise, so they fall into four tiers. Start at the top.
//!
//! | Tier | What it is | Modules |
//! |---|---|---|
//! | **1. Determinism substrate** | Load-bearing. Break one of these and replay breaks. Read these first. | [`fixed`], [`mod@vec`], [`rng`], [`rng_xoshiro`], [`noise`], [`world_hash`], [`replay`], [`rollback`], [`sim`], [`dst`], [`shrink`], [`prop`], [`plan`], [`mod@explore`], [`temporal`], [`recovery`], [`verify`], [`netinput`], [`cmdqueue`], [`bits`], [`savefile`], [`timestep`] |
//! | **2. Deterministic algorithms** | Where nondeterminism usually sneaks into a game (unordered iteration, float, address dependence). These are the vetted versions. | [`pathfinding`], [`fov`], [`geometry`], [`gridcast`], [`graph`], [`pack`], [`zorder`], [`msquares`], [`flow`], [`hungarian`], [`lsystem`], [`poly`], [`rdp`], [`fenwick`], [`ahocor`], [`diff`], [`trie`], [`segtree`], [`bipartite`], [`tsp`], [`rle`], [`segment`], [`euler`], [`rmq`], [`closestpair`], [`interval`], [`cron`], [`fuzzy`], [`stats`], [`markov`], [`lttb`], [`ntheory`], [`lca`], [`huffman`], [`treap`], [`kmp`], [`vclock`], [`merkle`], [`bloom`], [`delta`], [`lzss`], [`rolling`], [`suffix`], [`kdtree`], [`twosat`], [`minimax`], [`perm`], [`conv`], [`manacher`], [`gauss`], [`bezier`], [`kmv`], [`cms`], [`quantile`], [`chash`], [`lzw`], [`minhash`], [`zfunc`], [`bellman`], [`frac`], [`slide`], [`lis`], [`dagsp`], [`bfprt`], [`raster`], [`linrec`], [`mcflow`], [`knapsack`], [`coloring`], [`dsurb`], [`cht`], [`bwt`], [`hld`], [`mincut`], [`miller`], [`wavelet`], [`sam`], [`hamdp`], [`dlx`], [`arborescence`], [`xorbasis`], [`eertree`], [`mo`], [`histrect`], [`stable`], [`wdsu`], [`dominators`], [`fenwick2d`], [`circulation`], [`biconn`], [`simhash`], [`gf2`], [`rsfec`], [`fmidx`], [`zerobfs`], [`dpll`], [`intervaltree`], [`centroid`], [`piecetable`], [`steiner`], [`wal`], [`mapgen`], [`maze`], [`hexgrid`], [`delaunay`], [`wfc`], [`tilemap`], [`spatial_hash`], [`influence`], [`voronoi`], [`passability`], [`autotile`], [`turn`], [`entity`], [`sparse_set`], [`observe`], [`arch`], [`relations`], [`multimap`] |
//! | **3. Content pipeline** | Author game data as text, then prove it is well-formed before it reaches the sim — the verification gate for hand- or LLM-authored content. | [`content`], [`parser`], [`serializer`], [`validator`], [`loader`], [`diag_json`] |
//! | **4. Gameplay conveniences** | Ordinary systems (inventory, shops, quests, UI…), written so they are hashable and replay-safe. Useful, but nothing in tier 1 depends on them — treat them as worked examples you may freely replace. | everything else |
//!
//! If you only adopt one thing, adopt tier 1: implement
//! [`sim::Simulation`] for your state and call [`sim::audit`] on it.
//!
//! Full module list:
//!
//! - [`entity`] / [`sparse_set`] — sparse-set ECS storage with generational
//!   handles (cheap composition changes, O(1) lookup).
//! - [`fixed`] — Q16.16 fixed-point for cross-platform-deterministic math.
//! - [`fov`] — symmetric shadowcasting field-of-view (integer, deterministic).
//! - [`geometry`] — integer Bresenham line drawing and line-of-sight.
//! - [`gridcast`] — Amanatides–Woo grid ray traversal (`grid_ray`,
//!   `ray_blocked_at`, `clear_los`): exact all-cells-a-segment-enters
//!   casting for LOS, bullets and cone-free sweeping — integer-only with
//!   exact corner-exit semantics.
//! - [`graph`] — deterministic graph algorithms on adjacency lists: Tarjan
//!   SCC, articulation points, bridges, Kahn lexicographic topological
//!   sort, and a rank-based `UnionFind` — connectivity analysis for maps,
//!   tech trees and dependency graphs.
//! - [`pack`] — skyline (bottom-left) rectangle bin packing (Jylänki 2010):
//!   deterministic texture-atlas / inventory / dialog tiling.
//! - [`zorder`] — Morton (z-order) and Hilbert space-filling-curve codes +
//!   `spatial_sort`: deterministic locality-preserving ordering for spatial
//!   keys and cache-friendly sweeps.
//! - [`msquares`] — marching-squares iso-contour extraction on integer
//!   scalar fields (canonical saddle resolution; doubled-coordinate
//!   segments + loop chaining).
//! - [`flow`] — Edmonds–Karp max-flow / min-cut (`FlowNet`) for bottleneck
//!   and partition analysis on capacity networks.
//! - [`hungarian`] — `assign_min_cost`: Kuhn–Munkres minimum-cost
//!   assignment on `n ≤ m` matrices (unit→target matching, build order
//!   where rows are interchangeable).
//! - [`lsystem`] — deterministic L-system rewriting + integer turtle
//!   (`expand`, `turtle_cells`, `DIRS_4`/`DIRS_8`/`DIRS_HEX`) for
//!   branching plants and procedural structures.
//! - [`poly`] — exact `i128` 2D polygon ops: `area2` shoelace,
//!   `point_in_polygon` (even-odd + boundary), `convex_hull` (Andrew
//!   monotone chain), `ear_clip` triangulation.
//! - [`rdp`] — Ramer–Douglas–Peucker polyline simplification in integer
//!   coordinates (canonical downstream pass for `msquares` contours).
//! - [`fenwick`] — Fenwick tree / BIT over `i64`: `O(log n)` prefix sums,
//!   point updates, and an order-statistic `lower_bound` for running
//!   leaderboards and weighted picks.
//! - [`ahocor`] — Aho–Corasick multi-pattern `&[u8]` matcher
//!   (`O(text + hits)`, all overlapping occurrences, scan-order output).
//! - [`diff`] — Myers `O(ND)` minimal edit scripts (`diff`, `hunks`,
//!   `apply`) plus `levenshtein` / `lcs_len` — field-level diffs for
//!   desync reports and `did-you-mean` diagnostics.
//! - [`trie`] — byte trie with `BTreeMap` children: byte-lexicographic
//!   `keys`/`keys_with_prefix` listings, the did-you-mean candidate
//!   source paired with `diff::levenshtein`.
//! - [`segtree`] — `SegTree` range min/max/sum over `i64` with point
//!   `set`: `O(log n)` sliding-window aggregates where `fenwick` only
//!   does prefix sums.
//! - [`bipartite`] — `hopcroft_karp` `O(E·√V)` maximum bipartite
//!   matching (plus a `kuhn_match` parity oracle) — unweighted
//!   unit↔job pairing, complementing `hungarian`'s weighted version.
//! - [`tsp`] — deterministic tour heuristics (`nn_tour` seed,
//!   `tsp_2opt` first-improvement descent, `tour_cost`) with
//!   canonicalized rotation/orientation — patrol and visit-all routes.
//! - [`rle`] — run-length coding (`encode`/`decode`,
//!   `encode_u32`/`decode_u32`): the cheapest lossless layer before
//!   `bits` wire packing; malformed input decodes to `None`.
//! - [`mapgen`] — seed-driven procedural dungeon generation (rooms, cellular caves, BSP, drunkard's-walk, Bridson Poisson-disc scatter; deterministic).
//! - [`pathfinding`] — deterministic 8-way A*, weighted A* (ε-admissible), Jump Point Search (8-way `jps`, 4-way `jps4`), Dijkstra maps + rescanned flee/safety maps + coefficient blending (`combine_maps`) + farthest-cell stair placement (`farthest_cell`), O(1) reachability via precomputed connected components (`ConnectivityMap`), auto-explore.
//! - [`plan`] — planning-based test synthesis: BFS search over a deterministic simulation's state space for a shortest input sequence satisfying a goal predicate (`plan_inputs`) — a "can the player reach X" test becomes an executable replay.
//! - [`replay`] — replay trace recording, desync detection and rollback, plus production desync repro bundles (`DesyncReport`).
//! - [`rollback`] — rollback-netcode building blocks: a bounded snapshot ring (`SnapshotRing`) and a GGRS-style development sync test (`sync_test`) that catches step-function nondeterminism by rolling back and re-simulating every frame.
//! - [`sim`] — the one canonical `Simulation` trait (deterministic state machine + ordered inputs, cf. Schneider 1990) that every verification tool consumes, plus `audit()`: a single call running double-run and rollback-resimulation checks over your simulation.
//! - [`dst`] — Deterministic Simulation Testing harness: seed sweeps with per-tick invariant checks, double-run nondeterminism detection, swarm testing (per-seed random action subsets, Groce et al. ISSTA 2012), and one-line `(seed, tick)` failure reproduction.
//! - [`shrink`] — delta debugging (`ddmin`, Zeller & Hildebrandt, IEEE TSE 2002): reduce a failing input sequence to a 1-minimal one, so a 800-step failure becomes the three steps that actually caused it.
//! - [`prop`] — property-based testing (QuickCheck, Claessen & Hughes ICFP 2000): generate random input sequences from seeded sub-streams, check a property, and hand back any counterexample already shrunk to 1-minimal (`forall_inputs`, `forall_states`); plus model-based / differential testing (Hughes 2016) that runs the real simulation in lockstep with a trusted reference model and reports the first diverging command (`forall_model`).
//! - [`mod@explore`] — archive-based state-space exploration (Go-Explore, Ecoffet et al., Nature 2021): remember every distinct state reached, return to one deterministically, and explore onward — reaching deep states that memoryless random play cannot, and handing back the replayable path to each (`explore`, `explore_until`).
//! - [`temporal`] — temporal property monitors (runtime verification; LTL₃ three-valued semantics, Bauer/Leucker/Schallhart ACM TOSEM 2011): assert properties that span *time* rather than one state — `always`/`eventually`/`until`/`precedes`/`responds_within` — with an anytime verdict during a run and a definite one at the end (`Monitor`, `MonitorSet`, `check_run`).
//! - [`recovery`] — crash-recovery testing: inject a save/restore cycle after *every* input and check the run continues identically (`restart_test`, `restart_test_bytes`). Distinguishes an unloadable save, one that drops a hashed field, and — the case a hash comparison alone cannot see — one that drops state the hash does not cover.
//! - [`verify`] — bounded model checking (Clarke/Emerson/Sistla ACM TOPLAS 1986; SPIN, Holzmann IEEE TSE 1997): enumerate every reachable state breadth-first and either **prove** an invariant holds throughout or return the shortest input sequence that breaks it (`check_invariant`, `reachable_states`). Three-way result, so a proof is never confused with a search that ran out of budget. `check_temporal` extends this to temporal properties by the product construction (Vardi & Wolper, LICS 1986), crossing the state space with a [`temporal`] monitor — and refuses liveness properties rather than reporting a proof it has not made.
//! - [`netinput`] — the deterministic-lockstep input path: prediction and misprediction detection (`NetInputBuffer<P,I>`), an adaptive input-delay controller (`AdaptiveDelay`) that tunes buffering to the measured misprediction rate, and `DelayScheduler` executing captured input `delay` ticks later (1500 Archers, GDC 2001) without gaps or double-booked ticks as the delay moves.
//! - [`rng`] — SplitMix64 seeded PRNG (replay-safe randomness) with named independent sub-streams (`SplitMix64::split`).
//! - [`rng_xoshiro`] — opt-in xoshiro256++ PRNG (`Xoshiro256pp`): 2²⁵⁶ period, higher statistical quality, seeded from SplitMix64, with `jump()` for parallel streams.
//! - [`msglog`] — bounded ring-buffer message log with `DetHash`.
//! - [`terminal`] — headless cell screen buffer with 24-bit ANSI output.
//! - [`timestep`] — fixed-timestep accumulator with death-spiral guard.
//! - [`timer`] — tick-based `Cooldown` and `TimerQueue<E>` for delayed events.
//! - [`turn`] — energy/speed-based turn scheduler with non-destructive turn-order forecast.
//! - [`mod@vec`] — fixed-point Vec2/Vec3 (dot/cross/len/normalize/scale/DetHash).
//! - [`shufflebag`] — draw-without-replacement bag randomizer with auto-refill (`ShuffleBag<T>`).
//! - [`equipment`] — worn-item loadout per body slot with aggregate `StatsModifier` (`Equipment<T>`, `EquipSlot`).
//! - [`progression`] — experience accumulation and integer level curves (`Progression`, `LevelCurve`).
//! - [`meta`] — cross-run meta-progression: permanent unlock flags and all-time best records (`MetaProgress<K,R>`).
//! - [`identify`] — scrambled per-seed item appearances revealed on demand (`Identification<T,L>`).
//! - [`lightmap`] — additive integer illumination map for torchlit dungeons (`LightMap`).
//! - [`faction`] — inter-faction reputation and alignment queries (`FactionMap<K>`).
//! - [`threat`] — per-combatant aggro / target-selection table (`ThreatTable<K>`).
//! - [`pool`] — bounded regenerating resource pool: mana/stamina/hunger (`Pool`).
//! - [`tween`] — time-driven eased value interpolation over a tick span (`Tween`, `TweenSequence`).
//! - [`wallet`] — fungible currency balances for shops/economy (`Wallet<C>`).
//! - [`shop`] — buy/sell price listings against a wallet-backed till (`Shop<K,C>`, `Listing`).
//! - [`dialogue`] — branching NPC conversation tree (`Dialogue`, `DialogueNode`, `Choice`).
//! - [`trigger`] — condition→action rule set for scripted game events (`TriggerSet<K,C,A>`, `Trigger<C,A>`).
//! - [`eventqueue`] — intra-tick FIFO game event queue (`EventQueue<E>`).
//! - [`quest`] — quest and objective tracking (`Quest`, `Objective`, `QuestState`).
//! - [`calendar`] — cyclical integer time-of-day / day-night cycle (`Calendar`).
//! - [`recipe`] — item crafting / recipe system (`Recipe<K,O>`, `Ingredient<K>`).
//! - [`visibility`] — tri-state fog-of-war / exploration memory (`VisibilityMap`, `Visibility`) layered on top of FOV.
//! - [`world_hash`] — FNV-1a per-frame state checksum for bit-exact replay, `hash_unordered` for permutation-invariant multiset hashing, and `LabeledDigest` for per-subsystem hash breakdowns that localize desyncs.
//! - [`camera`] — integer camera / viewport (world↔screen coordinate mapping).
//! - [`change`] — dirty-flag change detection (`Changed<T>`, `ChangeTracker`).
//! - [`observe`] — structural-change events on component storage (`Observed<T>`, `ComponentEvent`): the push half of change awareness — inserts, overwrites and removals arrive as a drainable [`eventqueue`] stream.
//! - [`combat`] — integer combat formula (stats, melee/ranged, hit roll).
//! - [`damage`] — typed damage (`DamageType`) and per-type resistance/vulnerability profiles (`ResistanceProfile`).
//! - [`encounter`] — procedural group-encounter rolling (`EncounterPack`: count ranges + appearance chances per slot).
//! - [`affix`] — procedural item affixes (`AffixGenerator`: weighted prefix/suffix pools → "Rusty Sword of Dragonslaying").
//! - [`fsm`] — table-driven finite state machine for game AI (`Fsm<S,E>`).
//! - [`inventory`] — slot-based inventory (`Inventory<T>`) for roguelike items.
//! - [`keymap`] — key-to-action mapping (`KeyMap<K,A>`) for deterministic input.
//! - [`easing`] — integer easing curves (quad/cubic in/out/in-out) over `Fixed`.
//! - [`status`] — timed status effects / buff-debuff tracking (`StatusSet<K>`).
//! - [`cmdqueue`] — deterministic command queue (replay-safe input abstraction).
//! - [`content`] / [`parser`] / [`serializer`] / [`validator`] / [`loader`] —
//!   the content pipeline: author game elements as text (with `extends`
//!   field-level prefab overlays), serialize them back, validate them, load
//!   into the ECS.
//!
//! - [`ability`] — unified ability/skill system (`AbilitySet<K,E>`, `Ability<E>`, `AbilityResult`) with mana, cooldown, and range checks.
//! - [`behavior`] — hierarchical behavior trees for game AI (`BehaviorTree<A>`, `BehaviorNode<A>`, `BehaviorStatus`).
//! - [`aabb`] — axis-aligned bounding box (`Aabb`) collision detection.
//! - [`arch`] — archetype-based component storage (`ArchTable<Row>`) for cache-friendly multi-component iteration.
//! - [`menu`] — keyboard-navigable list menu (`Menu<T>`) for roguelike UI.
//! - [`textlayout`] — word-wrap, truncate, and alignment helpers for terminal UI.
//! - [`inputbuf`] — input buffer with hold/repeat detection (`InputBuffer<K>`).
//! - [`spatial_hash`] — spatial hash grid (`SpatialHash<K>`) for broad-phase queries.
//! - [`noise`] — deterministic integer value noise and hash functions.
//! - [`tilemap`] — multi-layer tile map (`TileMap<T>`, `LayeredMap<T>`).
//! - [`influence`] — grid-based influence map (`InfluenceMap`) for AI steering.
//! - [`relations`] — entity parent/child relationships (`Relations`).
//! - [`assets`] — typed asset handle store (`AssetStore<T>`, `AssetHandle<T>`).
//! - [`profiler`] — tick profiler (`Profiler`) and structured event log (`EventLog<E>`).
//! - [`hfsm`] — hierarchical FSM (`HFsm<S,E>`): parent states + wildcard transitions + `is_in` ancestry queries.
//! - [`hud`] — HUD primitives: fill bar (`BarWidget`), stat line, panel layout (`HudPanel`).
//! - [`autotile`] — bitmask auto-tiling (`compute_mask`, `SimpleTileTable`).
//! - [`diag_json`] — machine-readable diagnostic serialization: a bespoke JSON schema (`diag_json`) and industry-standard SARIF 2.1.0 (`diag_sarif`) for CI code-scanning integration.
//! - [`passability`] — grid-based passability / collision layer (`PassabilityGrid`).
//! - [`savefile`] — versioned binary save-file framing (`save_bytes`, `load_bytes`, `SaveHeader`).
//! - [`wfc`] — Wave Function Collapse procedural tile-map generation (`WfcRules`, `wfc_solve`, `WfcGrid`).
//! - [`multimap`] — multi-floor dungeon stack (`MultiMap`, `Connector`).
//! - [`bits`] — LSB-first bit-level wire codec (`BitWriter`/`BitReader`): packed bitfields, ranged integers, and canonical protobuf-style varint+zigzag — the lockstep-netcode packet primitive (Gaffer serialization strategies) the byte-oriented [`savefile`] container doesn't cover.
//! - [`voronoi`] — exact nearest-seed spatial partition (`voronoi_partition`, `voronoi_flood` through passable terrain) and `mst_edges` (Kruskal MST): the scatter → territory → connectivity procgen pipeline — pairs with [`mapgen::poisson_disc`].
//! - [`delaunay`] — Bowyer–Watson integer triangulation (`delaunay`, `delaunay_edges`): the "connect nearby rooms" primitive that makes corridor carving organic instead of tree-like (TinyKeep-style).
//! - [`hexgrid`] — axial hex-grid math (redblobgames formulation): distance, lines, rings, spirals, offset conversion, and `hex_astar` shortest paths — six-neighbor maps for hex-Civ boards and hex WFC.
//! - [`maze`] — Wilson's algorithm uniform random spanning trees rendered into [`mapgen::Dungeon`] walls: perfect mazes (exactly one path between any two cells) for roguelike cave-ins and puzzle floors.
//! - [`sam`] — suffix automaton (`Sam`): `contains`, `occurrences`, `longest_common`, `distinct_substrings`, `longest_repeated` over a `O(n)`-state structure — the compressed successor of `suffix`'s array for substring-heavy audits.
//! - [`hamdp`] — `tsp_exact`: Held–Karp exact TSP over `u32` edge weights for `n <= 16` cities, returning cost plus a lexicographically-smallest optimal tour — the optimal oracle behind `tsp`'s heuristic tour.
//! - [`dlx`] — `exact_cover`: Algorithm X exact cover over a binary incidence matrix, returning the lexicographically-first solution — placement puzzles, polyomino packing, constraint floors.
//! - [`arborescence`] — `directed_mst`: Edmonds' minimum-cost arborescence (directed spanning tree into a root) via cycle contraction, returning total weight plus the chosen edge indices.
//! - [`xorbasis`] — `XorBasis`: GF(2) linear basis over `u64` in reduced row-echelon form — `contains`, `max_xor`, `rank`, `kth`-smallest span element; the canonical xor-span certificate.
//! - [`eertree`] — `Eertree`: palindromic tree (eertree) over `&[u8]` — `distinct_palindromes`, `palindromic_substring_count`, `occurrences`, `longest_palindrome` in one online `O(n)` construction.
//! - [`mo`] — Mo's offline range queries: `mos_order` (block-sorted query permutation) plus `range_distinct` — batched window answers without a data structure.
//! - [`histrect`] — `largest_rectangle` (histogram via monotonic stack) and `maximal_rectangle` (binary matrix via running heights) — building footprints, warehouse slotting, territory blobs.
//! - [`stable`] — `stable_match`: Gale–Shapley proposer-optimal stable marriage plus the `is_stable` blocking-pair verifier — NPC pairing, draft picks, quest assignment.
//! - [`wdsu`] — `WeightedDsu`: potential-annotated union-find where `unite(u, v, w)` asserts `pot[v] - pot[u] == w` and contradictions return `false` — relative-height and offset constraints.
//! - [`dominators`] — `Dominators`: Cooper–Harvey–Kennedy iterative dominator tree plus `frontier` dominance frontiers — spawn gating, dependency scheduling, single-entry regions.
//! - [`fenwick2d`] — `Fenwick2d`: 2-D binary indexed tree with `rect_sum` / `prefix` / `add` over a dense integer grid — heatmaps and resource fields.
//! - [`circulation`] — `feasible_circulation`: lower/upper-capacity feasible flow via super-source/sink reduction — supply routes, upkeep pipes, demand schedules.
//! - [`biconn`] — `biconnected_components`: Tarjan edge-stack decomposition into maximal biconnected edge sets — bridges surface as singletons; failure-containment zones.
//! - [`simhash`] — Charikar 64-bit fingerprints: `simhash` / `weighted_simhash` + `hamming` + `near_dupes` — near-duplicate detection for generated content.
//! - [`gf2`] — GF(2⁸) field arithmetic (AES polynomial): `add`/`mul`/`inv`/`pow`/`div` plus exp/log `Tables` — the arithmetic core erasure codes build on.
//! - [`rsfec`] — `ReedSolomon`: Vandermonde-coded Reed–Solomon erasure coding over [`gf2`] — `k` data shards survive any `m` losses; lockstep packet-loss recovery.
//! - [`fmidx`] — `FmIndex`: FM-index over a cyclic BWT (C table + spaced Occ checkpoints + full SA) — sublinear `count`/`locate` for text search over generated content.
//! - [`zerobfs`] — `zero_one_bfs` (deque) + `dial` (bucket queue) — linear-ish shortest paths when edge weights are tiny integers.
//! - [`dpll`] — `solve`: DPLL SAT over CNF — unit propagation, pure-literal elimination, smallest-var split; canonical models for puzzle rules and placement constraints.
//! - [`intervaltree`] — `IntervalTree`: centered interval tree — `stab`/`overlap` queries over half-open `[lo,hi)` spans; sorted `u32` hits.
//! - [`centroid`] — `Centroid`: centroid decomposition of a forest — `parent`/`children`/`depth`/`order`/`roots` plus centroid-tree `lca`; depth `≤ ceil(log2 n)`.
//! - [`piecetable`] — `PieceTable`: editor-style text buffer — immutable `original` + append-only `added` + piece list; `insert`/`delete`/`get`/`to_bytes`.
//! - [`steiner`] — `steiner_tree`: Kou–Markowsky–Berman 2-approximation — metric closure, terminal MST, path unfolding, cycle prune.
//! - [`wal`] — `Wal` + `decode`: write-ahead log codec — `[kind|len|crc|payload]` records with Fnv1a checksums; torn-tail-tolerant replay reports `stopped_at`.
//!
//! All modules are `std`-only and contain no `unsafe`.

#![forbid(unsafe_code)]
// Publication-grade doc hygiene: every public item must be documented, and
// intra-doc links must resolve. Both are `deny` (not `warn`) — the crate is
// already clean, so this keeps it that way as a hard gate rather than a
// warning that can accumulate unnoticed.
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
// No panicking paths in shipped code. A measurement of the implementation
// (excluding `#[cfg(test)]`) found 6 `unwrap`, 4 `expect` and zero `panic!` —
// all of them provably guarded — so this is a gate that documents an existing
// property rather than a migration. `not(test)` because test code uses
// `unwrap` freely and should: a panicking assertion *is* a failing test.
//
// Deliberately NOT denied: `clippy::indexing_slicing`. The crate indexes grids
// and dense arrays about 700 times behind bounds it has already established,
// and rewriting those as `get().ok_or(...)` would make the code worse, not
// safer. Constructor `assert!`s on impossible configuration (a zero-capacity
// ring, a zero-length timestep) also stay: they report a programming error at
// the moment it is made.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

/// The README's code blocks, compiled and run as doctests.
///
/// A quickstart nothing compiles is a quickstart that stops working without
/// anyone noticing. This costs one hidden item and makes `cargo test` the
/// thing that finds out.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeExamplesAreCompiled;

pub mod aabb;
pub mod ability;
pub mod affix;
pub mod ahocor;
pub mod arborescence;
pub mod arch;
pub mod assets;
pub mod autotile;
pub mod behavior;
pub mod bellman;
pub mod bezier;
pub mod bfprt;
pub mod biconn;
pub mod bipartite;
pub mod bits;
pub mod bloom;
pub mod bwt;
pub mod calendar;
pub mod camera;
pub mod centroid;
pub mod change;
pub mod chash;
pub mod cht;
pub mod circulation;
pub mod closestpair;
pub mod cmdqueue;
pub mod cms;
pub mod coloring;
pub mod combat;
pub mod content;
pub mod conv;
pub mod cron;
pub mod dagsp;
pub mod damage;
pub mod delaunay;
pub mod delta;
pub mod diag_json;
pub mod dialogue;
pub mod dice;
pub mod diff;
pub mod dlx;
pub mod dominators;
pub mod dpll;
pub mod dst;
pub mod dsurb;
pub mod easing;
pub mod eertree;
pub mod encounter;
pub mod entity;
pub mod equipment;
pub mod euler;
pub mod eventqueue;
pub mod explore;
pub mod faction;
pub mod fenwick;
pub mod fenwick2d;
pub mod fixed;
pub mod flow;
pub mod fmidx;
pub mod fov;
pub mod frac;
pub mod fsm;
pub mod fuzzy;
pub mod gauss;
pub mod geometry;
pub mod gf2;
pub mod graph;
pub mod gridcast;
pub mod hamdp;
pub mod hexgrid;
pub mod hfsm;
pub mod histrect;
pub mod hld;
pub mod hud;
pub mod huffman;
pub mod hungarian;
pub mod identify;
pub mod influence;
pub mod inputbuf;
pub mod interval;
pub mod intervaltree;
pub mod inventory;
pub mod kdtree;
pub mod keymap;
pub mod kmp;
pub mod kmv;
pub mod knapsack;
pub mod lca;
pub mod lightmap;
pub mod linrec;
pub mod lis;
pub mod loader;
pub mod lsystem;
pub mod lttb;
pub mod lzss;
pub mod lzw;
pub mod manacher;
pub mod mapgen;
pub mod markov;
pub mod maze;
pub mod mcflow;
pub mod menu;
pub mod merkle;
pub mod meta;
pub mod miller;
pub mod mincut;
pub mod minhash;
pub mod minimax;
pub mod mo;
pub mod msglog;
pub mod msquares;
pub mod multimap;
pub mod netinput;
pub mod noise;
pub mod ntheory;
pub mod observe;
pub mod pack;
pub mod parser;
pub mod passability;
pub mod pathfinding;
pub mod perm;
pub mod piecetable;
pub mod plan;
pub mod poly;
pub mod pool;
pub mod profiler;
pub mod progression;
pub mod prop;
pub mod quantile;
pub mod quest;
pub mod random_table;
pub mod raster;
pub mod rdp;
pub mod recipe;
pub mod recovery;
pub mod relations;
pub mod replay;
pub mod rle;
pub mod rmq;
pub mod rng;
pub mod rng_xoshiro;
pub mod rollback;
pub mod rolling;
pub mod rsfec;
pub mod sam;
pub mod savefile;
pub mod segment;
pub mod segtree;
pub mod serializer;
pub mod shop;
pub mod shrink;
pub mod shufflebag;
pub mod sim;
pub mod simhash;
pub mod slide;
pub mod sparse_set;
pub mod spatial_hash;
pub mod stable;
pub mod stats;
pub mod status;
pub mod steiner;
pub mod suffix;
pub mod temporal;
pub mod terminal;
pub mod textlayout;
pub mod threat;
pub mod tilemap;
pub mod timer;
pub mod timestep;
pub mod treap;
pub mod trie;
pub mod trigger;
pub mod tsp;
pub mod turn;
pub mod tween;
pub mod twosat;
pub mod validator;
pub mod vclock;
pub mod vec;
pub mod verify;
pub mod visibility;
pub mod voronoi;
pub mod wal;
pub mod wallet;
pub mod wavelet;
pub mod wdsu;
pub mod wfc;
pub mod world_hash;
pub mod xorbasis;
pub mod zerobfs;
pub mod zfunc;
pub mod zorder;

pub use aabb::Aabb;
pub use ability::{Ability, AbilityResult, AbilitySet};
pub use affix::{Affix, AffixGenerator, AffixSlot, AffixedItem};
pub use arch::ArchTable;
pub use assets::{AssetHandle, AssetStore};
pub use autotile::{compute_all, compute_mask, SimpleTileTable};
pub use behavior::{BehaviorNode, BehaviorStatus, BehaviorTree};
pub use calendar::Calendar;
pub use camera::Camera;
pub use change::{ChangeTracker, Changed};
pub use cmdqueue::CmdQueue;
pub use combat::{
    apply_resistance, base_damage, critical_strike, melee_attack, ranged_attack, roll_damage,
    roll_to_hit, splash_attack, Stats, StatsModifier, StrikeResult,
};
pub use content::{Content, Diagnostic, ExtendsError, Prefab, Severity, Tile};
pub use damage::{DamageType, ResistanceProfile};
pub use diag_json::severity_filter;
pub use dialogue::{Choice, Dialogue, DialogueNode};
pub use dice::Dice;
pub use dst::{
    dst_determinism_sweep, dst_replay, dst_swarm_replay, dst_swarm_sweep, dst_sweep, swarm_subset,
    DstFailure, SwarmFailure,
};
pub use easing::{
    ease_in_back, ease_in_bounce, ease_in_circ, ease_in_cubic, ease_in_expo, ease_in_out_back,
    ease_in_out_bounce, ease_in_out_circ, ease_in_out_cubic, ease_in_out_expo, ease_in_out_quad,
    ease_in_out_quart, ease_in_out_quint, ease_in_out_sine, ease_in_quad, ease_in_quart,
    ease_in_quint, ease_in_sine, ease_out_back, ease_out_bounce, ease_out_circ, ease_out_cubic,
    ease_out_expo, ease_out_quad, ease_out_quart, ease_out_quint, ease_out_sine, ease_reversed,
    linear,
};
pub use encounter::{EncounterPack, EncounterSlot};
pub use entity::{Entity, EntityAllocator};
pub use equipment::{EquipSlot, Equipment};
pub use eventqueue::EventQueue;
pub use explore::{explore, explore_sim, explore_sim_until, explore_until, Archive, ExploreConfig};
pub use faction::{FactionMap, FRIENDLY_THRESHOLD, HOSTILE_THRESHOLD, MAX_REP, MIN_REP};
pub use fixed::Fixed;
pub use fov::{can_see, compute_fov, compute_fov_dist, fov_count_filtered, fov_to_vec};
pub use fsm::Fsm;
pub use geometry::{
    chebyshev_distance, cone, cone_visible, diamond, knockback, line, line_of_sight,
    manhattan_distance, ray_blocked_at, ray_cast, rect_contains, rect_perimeter, reflect_point,
    rotate_90_ccw, rotate_90_cw, vec_toward, Distance,
};
pub use hfsm::HFsm;
pub use hud::{BarWidget, HudPanel, StatLine};
pub use identify::Identification;
pub use influence::InfluenceMap;
pub use inputbuf::{InputBuffer, KeySource, ListKeySource};
pub use inventory::Inventory;
pub use keymap::KeyMap;
pub use lightmap::{LightMap, MAX_LIGHT};
pub use loader::{load_level, LoadedLevel, Position, Render};
pub use mapgen::{
    generate_bsp, generate_cave, generate_drunkard, generate_dungeon, BspParams, CaveParams,
    DrunkardParams, Dungeon, GenParams, MapBuilder, Rect,
};
pub use menu::{Menu, MenuItem};
pub use meta::MetaProgress;
pub use msglog::MsgLog;
pub use multimap::{Connector, MultiMap};
pub use netinput::{AdaptiveDelay, DelayScheduler, NetInputBuffer};
pub use noise::{
    fbm_1d, fbm_1d_wrap, fbm_2d, fbm_2d_in_range, fbm_2d_wrap, fbm_3d, hash_1d, hash_2d, hash_3d,
    noise_3d_in_range, normalize_noise, ridge_noise_2d, value_noise_1d, value_noise_1d_wrap,
    value_noise_2d, value_noise_2d_wrap, value_noise_3d,
};
pub use observe::{ComponentEvent, Observed};
pub use parser::{error_count, parse, warning_count};
pub use passability::PassabilityGrid;
pub use pathfinding::{
    astar, auto_explore, combine_maps, descend, dijkstra_map, farthest_cell, flee_map, flood_fill,
    is_path_clear, is_reachable, jps, jps4, nearest_reachable, octile_distance, path_cost,
    path_to_direction_vec, smooth_path, step_toward, weighted_astar, ConnectivityMap, DijkstraMap,
};
pub use plan::plan_inputs;
pub use pool::Pool;
pub use profiler::{EventLog, LogEntry, Profiler};
pub use progression::{LevelCurve, Progression};
pub use prop::{forall_inputs, forall_model, forall_states, ModelFailure, PropFailure};
pub use quest::{Objective, Quest, QuestState};
pub use random_table::RandomTable;
pub use recipe::{Ingredient, Recipe};
pub use recovery::{restart_test, restart_test_bytes, restart_test_sim, RestartFailure};
pub use relations::Relations;
pub use replay::{
    check_trace, count_divergences, desync_report, desync_report_labeled, first_divergence,
    first_divergence_labeled, record_trace, resimulate, DesyncPolicy, DesyncReport, Divergence,
    LabeledDivergence,
};
pub use rng::SplitMix64;
pub use rng_xoshiro::Xoshiro256pp;
pub use rollback::{sync_test, SnapshotRing, SyncTestFailure};
pub use savefile::{
    estimate_save_size, load_bytes, load_bytes_migrated, load_bytes_owned, save_bytes,
    validate_integrity, LoadError, Migrator, SaveHeader,
};
pub use serializer::{content_eq, serialize};
pub use shop::{Listing, Shop};
pub use shrink::{is_one_minimal, shrink_inputs, shrink_simulation_inputs};
pub use shufflebag::ShuffleBag;
pub use sim::{audit, AuditReport, Simulation};
pub use sparse_set::{join, join3, join3_mut, join_mut, SparseSet};
pub use spatial_hash::SpatialHash;
pub use status::{Effect, StatTarget, StatusSet};
pub use temporal::{check_run, Monitor, MonitorSet, Verdict};
pub use terminal::{Cell, Screen};
pub use textlayout::{
    center, count_lines, fit_to_box, justify, measure_lines, pad_left, pad_lines, pad_right,
    truncate, truncate_lines, wrap_words, wrap_words_max_lines,
};
pub use threat::ThreatTable;
pub use tilemap::{LayeredMap, TileMap};
pub use timer::{Cooldown, TimerQueue};
pub use timestep::FixedTimestep;
pub use trigger::{Trigger, TriggerSet};
pub use turn::Scheduler;
pub use tween::{Tween, TweenSequence};
pub use validator::{is_loadable, validate};
pub use vec::{Vec2, Vec3};
pub use verify::{
    check_invariant, check_temporal, reachable_states, Counterexample, NotSafety, Verification,
};
pub use visibility::{Visibility, VisibilityMap};
pub use wallet::Wallet;
pub use wfc::{
    wfc_solve, wfc_solve_backtrack, wfc_solve_partial, wfc_solve_retry, wfc_solve_with_selector,
    CellSelector, LowestEntropySelector, WfcGrid, WfcResult, WfcRules,
};
pub use world_hash::{
    field_coverage, hash_covers, hash_state, hash_state_mixed, hash_unordered, uncovered_fields,
    DetHash, FieldMutator, Fnv1a, LabeledDigest,
};
