# izanagi_kit

Zero-dependency building blocks for a game simulation that **replays
bit-identically** — and the tools to prove yours does.

Eleven modules do nothing but interrogate a simulation: audit it for
nondeterminism, search its state space for one that breaks an invariant,
*prove* no such state exists, minimise any counterexample to its shortest form,
monitor properties that span time, and check that saving and reloading changes
nothing. The rest of the crate is the deterministic substrate they rest on
(fixed-point maths, seeded RNG, world hashing) plus ordinary game systems
written so they stay hashable and replay-safe.

Every module is `std`-only, contains no `unsafe` (`#![forbid(unsafe_code)]`),
cannot panic in shipped code (`clippy::unwrap_used`/`expect_used`/`panic` are
`deny`), and is covered by tests. There are no runtime dependencies.

## Why

A game engine has a hard floor under a zero-dependency constraint: GPU, window,
and audio I/O all require FFI. Rather than fight that ceiling, this kit leans
into a terminal/headless target where the whole frame is text — which makes the
simulation **fully inspectable and deterministically replayable**. Bit-exact
replay is treated as a first-class feature, not an afterthought.

## Quickstart

Implement one trait, and every tool in the crate applies to your simulation.

```rust
use izanagi_kit::sim::{audit, Simulation};
use izanagi_kit::verify::{check_invariant, Verification};
use izanagi_kit::world_hash::{DetHash, Fnv1a};

// Your game state, however you model it.
#[derive(Clone)]
struct Purse {
    coins: i32,
}

// Two impls: how it advances, and how it hashes.
impl Simulation for Purse {
    type Input = i32;
    fn step(&mut self, delta: &i32) {
        self.coins = (self.coins + delta).clamp(0, 10);
    }
}
impl DetHash for Purse {
    fn det_hash(&self, h: &mut Fnv1a) {
        h.write_i32(self.coins);
    }
}

// Is it deterministic? `audit` double-runs it and rolls it back at every
// frame, then hands you a hash to pin as a regression value.
let report = audit(&Purse { coins: 5 }, &[-3, 2, -3], 2);
assert!(report.is_deterministic());

// Does a rule hold? Not "did random play find a violation" — whether one
// exists at all, across every state the simulation can reach.
let proof = check_invariant(
    Purse { coins: 5 },
    &[-3, 2],
    |p: &Purse, d: &i32| {
        let mut next = p.clone();
        next.step(d);
        next
    },
    |p: &Purse| p.coins >= 0,
    10_000,
);
assert!(matches!(proof, Verification::Holds { .. }));
```

See the whole pipeline applied to one simulation with a planted bug:

```text
cargo run --example verify_pipeline_demo
```

The rest of the crate is what a game needs around that core — entities and
sparse-set storage, fixed-point maths, seeded RNG with named sub-streams,
procedural generation, pathfinding, FOV, a text content pipeline — all written
so nothing they touch can break the replay guarantee.

## Modules

Everything here serves one promise: **a simulation that replays
bit-identically**. Not every module carries the same weight in keeping that
promise, so they fall into four tiers — read top-down:

## Start here

```text
cargo run --example verify_pipeline_demo
```

One dungeon room, one planted bug, and every verification tool in the crate
applied to it in turn — so you can see what the eleven checking modules add up
to without reading eleven sets of docs. Three unrelated techniques (bounded
model checking, property testing, delta debugging) converge on the same
two-input minimal cause, and only one of them can then go on to *prove* the
fixed version has no such state at all.


| Tier | What it is | Modules |
|---|---|---|
| **1. Determinism substrate** | Load-bearing. Break one of these and replay breaks. | `fixed`, `vec`, `rng`, `rng_xoshiro`, `noise`, `world_hash`, `replay`, `rollback`, `sim`, `dst`, `shrink`, `prop`, `plan`, `explore`, `temporal`, `recovery`, `verify`, `netinput`, `cmdqueue`, `bits`, `savefile`, `timestep` |
| **2. Deterministic algorithms** | Where nondeterminism usually sneaks into a game (unordered iteration, float, address dependence) — the vetted versions. | `pathfinding`, `fov`, `geometry`, `gridcast`, `graph`, `pack`, `zorder`, `msquares`, `flow`, `hungarian`, `lsystem`, `poly`, `rdp`, `fenwick`, `ahocor`, `diff`, `trie`, `segtree`, `bipartite`, `tsp`, `rle`, `segment`, `euler`, `rmq`, `closestpair`, `interval`, `mapgen`, `wfc`, `tilemap`, `spatial_hash`, `influence`, `voronoi`, `delaunay`, `hexgrid`, `maze`, `passability`, `autotile`, `turn`, `entity`, `sparse_set`, `observe`, `arch`, `relations`, `multimap` |
| **3. Content pipeline** | Author game data as text, then prove it well-formed before it reaches the sim. | `content`, `parser`, `serializer`, `validator`, `loader`, `diag_json` |
| **4. Gameplay conveniences** | Ordinary systems (inventory, shops, quests, UI…) written to be hashable and replay-safe. Nothing in tier 1 depends on them — worked examples you may freely replace. | everything else |

If you adopt one thing, adopt tier 1: implement `sim::Simulation` for your
state and call `sim::audit` on it — one call runs a double-run check and a
rollback-resimulation check and reports the final hash to pin.

The capability map — with per-feature implementation status — lives in
[`GAME_DEV_TAXONOMY.md`](./GAME_DEV_TAXONOMY.md); contracts are in
[`SPEC.md`](./SPEC.md). The core foundations:

| Module | Responsibility |
|--------|----------------|
| `entity` | Generational entity handles; stale handles are rejected. |
| `sparse_set` / `arch` | O(1) component storage and an archetype table; cheap composition changes. |
| `observe` | `Observed<T>` wraps `SparseSet` and emits `ComponentEvent`s (added / replaced / removed) into a drainable queue — the push half of change detection, complementing `change`'s pull. |
| `fixed` | Q16.16 fixed-point with **saturating** arithmetic for cross-platform determinism. |
| `rng` | SplitMix64 seeded PRNG; replay-safe randomness. |
| `timestep` | Fixed-timestep accumulator with a death-spiral guard. |
| `world_hash` | FNV-1a per-frame state checksum for bit-exact replay assertions. |
| `replay` | Trace recording, desync localisation, and snapshot resimulation (rollback). |
| `sim` | The one `Simulation` trait every verification tool consumes, plus `audit()` — a single call that double-runs the sim, rollback-resimulates it, and reports the final hash. |
| `rollback` | Bounded snapshot ring (`SnapshotRing`) and a GGRS-style `sync_test` that rolls back every frame to catch step-function nondeterminism. |
| `dst` | Deterministic Simulation Testing: seed sweeps with per-tick invariants and one-line `(seed, tick)` failure reproduction. |
| `plan` | Planning-based test synthesis — BFS over the state space for a shortest input sequence satisfying a goal ("can the player reach X" becomes an executable replay). |
| `explore` | Archive-based exploration (Go-Explore) — remember every state reached, return to it deterministically, explore onward. Scales past where BFS stalls; hands back a replayable path per state. |
| `temporal` | Temporal property monitors (runtime verification) — `always` / `eventually` / `until` / `precedes` / `responds_within` over a tick stream, with LTL₃ anytime verdicts and a definite finite-trace verdict. Drops straight into a `dst_sweep` invariant. |
| `recovery` | Crash-recovery testing — inject a save/restore cycle after every input and prove the run continues identically. Catches save formats that silently drop a field, including fields the state hash does not cover. |
| `verify` | Bounded model checking — enumerate every reachable state and either prove an invariant holds throughout, or return the shortest input sequence that breaks it. A proof is reported distinctly from "ran out of budget". `check_temporal` crosses the state space with a `temporal` monitor to verify ordering properties no single-state predicate can express. |
| `shrink` | Delta debugging (`ddmin`) — reduce a failing input sequence to a 1-minimal one. |
| `prop` | Property-based testing (QuickCheck) — random sequences, checked property, counterexample returned already shrunk. Plus model-based / differential testing (`forall_model`) — run the real sim in lockstep with a trusted reference model and shrink any diverging command sequence. |
| `netinput` | Transport-agnostic multi-player input prediction and misprediction detection for rollback netcode (`NetInputBuffer`), plus `AdaptiveDelay` tuning input delay to the measured misprediction rate. |
| `content` / `parser` / `serializer` / `validator` / `loader` | The text→ECS content pipeline (see below). |
| `mapgen` / `wfc` / `multimap` / `maze` | Procedural dungeons (room-placement, cellular-automata caves, BSP partitions, drunkard's-walk caverns, Bridson Poisson-disc scatter, `carve_corridors` TinyKeep-style weighted tunneling, Wilson's uniform-spanning-tree perfect mazes), Wave Function Collapse, multi-level worlds. |
| `fov` / `pathfinding` / `influence` / `fsm` / `hfsm` | Symmetric FOV (binary + distance-attenuated), (weighted) A* + `min_cost_path` over arbitrary enter-costs + path smoothing + Dijkstra flow maps + rescanned flee/safety maps + frontier-seeking auto-explore, influence maps, flat and hierarchical (parent-state + wildcard) state machines. |
| `ability` / `behavior` | Unified skill system (mana/cooldown/range/effect resolution in one call) and hierarchical behavior trees (sequence/selector/invert/repeat leaves) for game AI. |
| `aabb` / `spatial_hash` / `passability` | Axis-aligned bounding-box overlap, spatial-hash broad-phase queries, and a grid passability/collision layer. |
| `geometry` | Bresenham lines / line-of-sight and integer distance metrics (Manhattan, Chebyshev, Euclidean). |
| `gridcast` | Amanatides–Woo grid ray traversal (`grid_ray`, `ray_blocked_at`, `clear_los`) — every cell a segment enters, exact corner-exit semantics, symmetric in either direction. |
| `graph` | Deterministic algorithms on `u32` adjacency lists — Tarjan SCC (`strongly_connected`), `articulation_points`, `bridges`, Kahn lexicographic `topo_sort`, and `UnionFind`. |
| `pack` | Skyline (bottom-left) rectangle bin packing (Jylänki 2010) — deterministic `pack_skyline` for atlases, inventories, dialog tiling. |
| `zorder` | Morton (z-order) and Hilbert space-filling-curve codes (`morton_encode`/`decode`, 2D+3D and 64-bit variants, `hilbert_encode`/`decode`, `spatial_sort`, `morton_key`) — locality-preserving deterministic ordering for spatial keys. |
| `msquares` | Marching-squares iso-contour extraction on integer fields (`case_index`, `contour_segments`, `contour_loops`) — canonical saddle resolution, doubled-coordinate segments, loop chaining. |
| `flow` | Edmonds–Karp max-flow / min-cut (`FlowNet`, `max_flow`, `min_cut`, `flow_on`) — bottleneck and partition analysis on capacity networks. |
| `hungarian` | `assign_min_cost`: Kuhn–Munkres minimum-cost assignment on `n ≤ m` matrices — unit→target matching and build-order style problems. |
| `lsystem` | Deterministic L-system rewriting + integer turtle (`expand`, `turtle_cells`, `DIRS_4`/`DIRS_8`/`DIRS_HEX`) — branching plants and procedural structures drawn through `gridcast`. |
| `poly` | Exact `i128` 2D polygon ops (`area2` shoelace, `point_in_polygon` even-odd + boundary, `convex_hull` Andrew monotone chain, `ear_clip` triangulation). |
| `rdp` | Ramer–Douglas–Peucker polyline simplification (`simplify`, `simplify_loop`) — integer-exact, the canonical downstream pass for `msquares` contours. |
| `fenwick` | Fenwick tree / BIT over `i64` (`add`, `prefix_sum`, `range_sum`, `lower_bound` order statistic) — `O(log n)` running totals for leaderboards and weighted picks. |
| `ahocor` | Aho–Corasick multi-pattern `&[u8]` matcher (`scan` → `(end_pos, pattern)` in scan order, `O(text + hits)`, overlaps included). |
| `diff` | Myers `O(ND)` minimal edit scripts (`diff`, `hunks`, `apply`) + `levenshtein` / `lcs_len` — field-level diffs for desync reports and diagnostics. |
| `trie` | Byte trie (`insert`/`remove`/`contains`/`starts_with`/`keys_with_prefix`) — `BTreeMap` children give byte-lexicographic listings; the did-you-mean candidate source for `diff::levenshtein`. |
| `segtree` | `SegTree` range min / max / sum over `i64` with point `set` (`range_stats` returns all three in one `O(log n)` walk) — sliding-window aggregates where `fenwick` only does prefix sums. |
| `bipartite` | `hopcroft_karp` `O(E·√V)` maximum bipartite matching + `kuhn_match` parity oracle — unweighted unit↔job pairing, complementing `hungarian`'s weighted assignment. |
| `tsp` | Deterministic tour heuristics (`nn_tour` nearest-neighbour, `tsp_2opt` first-improvement descent, `tour_cost`) — canonicalized rotation + orientation; patrol routes and visit-all missions. |
| `rle` | Run-length coding (`encode`/`decode`, `encode_u32`/`decode_u32`) — the cheapest lossless compaction before `bits` wire packing; malformed input decodes to `None`. |
| `segment` | Integer-exact segment predicates (`segments_intersect`, `point_on_segment`, `point_segment_dist2`, `segment_dist2`) — `i128` orientation tests; `dist2` is a ceiling, so `== 0` iff the point is exactly on the segment. |
| `euler` | `euler_walk` — Hierholzer Eulerian circuit/path over an undirected multigraph (self-loops and parallel edges included); `None` on odd-degree or disconnected input. |
| `rmq` | `SparseTable` — static `O(1)` range min/max after `O(n log n)` build; the query-hot complement to `segtree`'s updatable tree. |
| `closestpair` | `closest_pair` — `O(n log n)` divide-and-conquer closest pair of points in `i128` squared distance; lexicographic `(dist², p, q)` tie-break makes the answer content-defined. |
| `interval` | `IntervalSet` — sorted disjoint half-open `i64` intervals (`insert`/`remove`/`clip`/`contains`/`overlaps`); occupancy and reservation bookkeeping. |
| `cron` | `Cron::parse` + `next_after`/`next_n_after` — 5-field cron next-fire times on the millisecond timeline; POSIX OR-rule for dom/dow, `7 ≡ 0` Sunday, Quartz `a/n` steps. Event cooldowns and spawn schedules. |
| `fuzzy` | fzf-style subsequence scoring (`score`, `rank`, `rank_str`) — consecutive-run-dominant weights, boundary bonuses, deterministic total order (score → length → bytes → index). Command palettes and did-you-mean pickers. |
| `stats` | `RunningStats` — Welford online moments in `i64·SCALE` fixed point (`push`/`merge`/`mean`/`variance`/`sample_variance`/`stddev`); Chan parallel-merge keeps shards joinable. Telemetry and balance dashboards without storing samples. |
| `markov` | `NameGen` — order-k byte-level Markov name generator driven by `SplitMix64`; cumulative-weight tables in `BTreeMap` make the chain a pure function of `(corpus, seed)`. NPC/place/item naming. |
| `lttb` | `lttb` — Largest-Triangle-Three-Buckets downsampling (Steinarsson 2013) in `i128` twice-area math; endpoints preserved, order-preserving subsequence. Dense time-series → small HUD charts. |
| `ntheory` | `gcd`/`lcm`/`extgcd`/`mod_inv`/`mod_pow`/`crt2`/`crt` — the modular-arithmetic backbone: Bézout inverses and Chinese Remainder composition for periodic-event alignment and residue addressing. |
| `lca` | `Lca` — binary-lifting lowest common ancestor over a rooted forest (`new`/`depth`/`ancestor`/`lca`/`dist`); cycles and out-of-range parents mark nodes invalid. Zone trees, skill trees, entity hierarchies. |
| `huffman` | `encode`/`decode`/`build_book` — canonical Huffman codec: the wire is a `(symbol, length)` table plus MSB-first packed bits, a pure function of the corpus. The entropy layer between `rle` and `bits`. |
| `treap` | `Treap` — deterministic ordered `u64` set: BST + min-heap over `splitmix64(key^seed)` priorities via merge/split. Shape depends only on the key set, never on insertion order; `rank`/`select` order statistics included. |
| `kmp` | `Kmp` — Knuth–Morris–Pratt single-pattern search (`find`/`find_all`/`count`) plus a `Stream` variant that accepts bytes one at a time for chunk-fed protocols. `ahocor` covers the multi-pattern case. |
| `vclock` | `VClock` — Lamport/Fidge–Mattern vector clocks: `tick`/`merge`/`compare` give the four-way causal order (`Before`/`After`/`Equal`/`Concurrent`), so diverged peer histories are provably distinguishable — the desync-report predicate. |
| `merkle` | `Merkle` — binary hash tree over domain-tagged `u64` leaf hashes: `root`/`proof`/`verify`/`first_diff`. Peers compare roots to learn *whether* state diverged, then descend to the first bad leaf in `O(log n)` node comparisons. |
| `bloom` | `Bloom` — seeded Bloom filter: Kirsch–Mitzenmacher double hashing from a `Fnv1a` pair. One-sided error — inserted keys are always reported present; `sizing` gives the (bits, probes) heuristic for a target item count. |
| `delta` | `diff_sorted`/`apply_sorted`/`Delta` — snapshot deltas over ordered `u64→u64` maps with a canonical wire form (ascending-key delta varints over `bits`). `Del` of an absent key fails closed. |
| `lzss` | `compress`/`decompress` — greedy LZ77-family codec on the `bits` wire (4096 window, match 3–18, length header). Completes the `rle`→`lzss`→`huffman` ladder; malformed streams decode to `None`. |
| `rolling` | `Rolling`/`hash_bytes`/`find_all`/`chunks` — Rabin–Karp mod-2^64 polynomial fingerprints: fixed-window rolling hash, multi-match search, and content-defined chunk boundaries (`hash & mask == 0` inside `[min, max]`), so a local edit only perturbs nearby chunks — the delta-sync fingerprint layer. |
| `suffix` | `SuffixArray` — prefix-doubling suffix array + Kasai LCP: `sa`/`lcp`/`search` (all occurrences in `O(pat log n + hits)`), `longest_repeated`, `distinct_substrings`. Corpus introspection: what did `markov` memorize. |
| `kdtree` | `KdTree` — static median-split 2-D spatial index over `(i32, i32)`: `nearest`/`within`/`in_rect` in expected `O(log n)`, pure function of the point set, squared `i128` distances, lexicographic tie-breaks. |
| `twosat` | `TwoSat` — 2-SAT via the implication graph on `graph::strongly_connected`: `add_clause`/`add_unit`/`add_implies`/`add_equiv`/`add_xor`, `solve` returns a canonical satisfying assignment or `None`. Key-and-lock and pairwise-exclusion constraints. |
| `minimax` | `Game`/`score`/`best_move` — deterministic negamax with alpha-beta: canonical move order, first-move tie-breaks, ply-discounted terminal scores. Exhaustive tactics/board AI with zero randomness. |
| `perm` | `identity`/`is_valid`/`compose`/`inverse`/`cycles`/`sign`/`order`/`apply`/`rank`/`unrank` — permutation algebra plus Lehmer-code factoradic ranking (n ≤ 20): canonical cycle form, parity, group order, and a bijection between perms and `0..n!` for content seeding. |
| `conv` | `convolve`/`convolve_i64` — number-theoretic-transform convolution mod `998244353` (primitive root 3, `u128` intermediates): `O(n log n)` polynomial products for dice-sum distributions and generating-function counts. `convolve_i64` returns `None` rather than a wrapped answer when the exact bound would overflow the modulus. |
| `manacher` | `odd_radii`/`even_radii`/`longest_palindrome`/`count_palindromes` — Manacher's `O(n)` palindrome structure (d1/d2 arrays). String symmetry queries for name linting and seed prettiness. |
| `gauss` | `det`/`solve`/`rank` — Bareiss fraction-free Gaussian elimination over `i64` matrices with `i128` intermediates: exact determinants, exact rational solutions `(num, den)`, and exact rank. `None` = provably singular. |
| `bezier` | `cubic_pos`/`catmull_pos`/`flatten_cubic`/`flatten_catmull` — integer-exact parametric curves evaluated at rational `t = num/den`: every coordinate a reduced `i128` fraction — bit-identical camera paths and patrol routes. |
| `kmv` | `Kmv` — K-minimum-values distinct-count sketch: the `k` smallest seeded hashes; exact below `k`, `(k−1)·2⁶⁴/vₖ` estimate above; merge = union of minima. |
| `cms` | `CountMin` — count-min sketch frequency estimation: `depth×width` counter matrix, one-sided error (`estimate ≥ truth` always), elementwise merge. |
| `quantile` | `Quantile` — Greenwald–Khanna ε-approximate quantiles: `(v,g,δ)` tuples with periodic compaction; `|true_rank − φ·n| ≤ ε·n` on every answer. |
| `chash` | `pick`/`pick_top`/`distribution` — rendezvous (HRW) consistent hashing: argmax seeded weight per key; removing a node remaps only its keys. |
| `lzw` | `encode`/`decode` — LZW phrase-table codec, 12-bit codes on `bits` (dict cap 4096, KwKwK decoder case handled); the wire carries no dictionary — rebuilt in lockstep. |
| `minhash` | `signature`/`estimate`/`union`/`jaccard_exact` — MinHash similarity signatures: `k` independent seeded minima; Jaccard estimate in permille (error ~ `1/√k`), union mergeable. |
| `zfunc` | `z`/`z_search`/`z_search_bytes`/`borders`/`min_period` — Z-algorithm prefix-match array in `O(n)`: substring search, border enumeration, smallest period. |
| `bellman` | `shortest`/`negative_cycle` — Bellman–Ford single-source paths with signed edges (`O(V·E)`); super-source negative-cycle finder returns the actual cycle vertices. |
| `frac` | `Frac` — normalized `i128` rational (`num`/`den` reduced, `den>0`): exact `+`/`-`/`*`/`÷`/compare/mixed-split; `den==0` clamps, division-by-zero returns `None`. |
| `slide` | `slide_min`/`slide_max` — monotonic-deque sliding-window extrema in `O(n)` — the `segtree` answer compressed to linear when the range slides by one. |
| `lis` | `lis`/`lis_len` — patience-sorting longest increasing subsequence in `O(n log n)`; returns an actual witness, not just the length. |
| `dagsp` | `dag_paths`/`critical_path` — DAG shortest AND longest paths in `O(V+E)` on top of `topo_sort`; `critical_path` returns the critical vertex chain (unbounded-start longest path). |
| `bfprt` | `select`/`median` — median-of-medians `O(n)` deterministic selection: worst-case linear k-th element with no randomness, pivot-of-pivots recursion. |
| `raster` | `line`/`circle`/`fill_polygon` — Bresenham line (reversal-symmetric via canonical direction), midpoint circle, and integer-exact scanline polygon fill; every cell center is classified by a rational crossing count in `i128`. |
| `linrec` | `linrec`/`linrec_mod` — k-th term of a linear recurrence `aₙ = Σ cᵢaₙ₋ᵢ₋₁` in `O(d³ log k)` via companion-matrix exponentiation; exact `i128` (None on overflow) and always-total modular variants. |
| `mcflow` | `FlowNet::min_cost_max_flow` — Edmonds–Karp max flow then negative-cycle canceling via `bellman`: exact min cost at max flow, signed per-unit costs, deterministic by insertion order. |
| `knapsack` | `knapsack_01`/`knapsack_unbounded` — `O(n·W)` loot-under-weight with deterministic witness reconstruction (ties keep earliest items). |
| `coloring` | `dsatur`/`is_proper` — DSATUR graph coloring (saturation → degree → index tie-breaks); exact on bipartite/cycles, strong heuristic elsewhere. |
| `dsurb` | `DsuRollback` — union-find with `snapshot`/`rollback`: hypothetical connectivity queries (`connected` under a tentative merge, then undo). No path compression → `O(log n)` find. |
| `cht` | `LiChao` — min-envelope of lines `y = a·x + b` over a bounded integer domain: `insert` + `query_min` in `O(log X)`, `i128` evaluation. |
| `bwt` | `bwt`/`bwt_inverse` + `mtf_encode`/`mtf_decode` — cyclic Burrows–Wheeler transform (rotation order via `SuffixArray` on the doubled string) and move-to-front coding — the reversible front half of a bzip2-style pipeline before `rle`/`huffman`. |
| `hld` | `Hld` — heavy-light decomposition: `path_vertices`, `path_segments` (`O(log n)` flat ranges for `segtree`/`fenwick`), `subtree_segment` over a `parent[]` tree. |
| `mincut` | `global_min_cut` — Stoer–Wagner `O(n³)` global min cut (weight + one side) without choosing terminals; deterministic lowest-index tie-breaks. |
| `miller` | `is_prime`/`factor` — exact `u64` primality via the 7-base deterministic Miller–Rabin set, sorted factorization by trial division + Brent rho (fixed polynomial schedule). |
| `wavelet` | `WaveletMatrix` — `access`/`rank`/`freq_less`/`range_freq`/`quantile` on `u32` sequences in `O(bits)` — layered stable-partition bitplanes, no floating point. |
| `sam` | `Sam` — suffix automaton: `contains`, `occurrences`, `longest_common`, `distinct_substrings`, `longest_repeated` on an `O(n)`-state structure built by online extension + `finish`. |
| `hamdp` | `tsp_exact` — Held–Karp exact TSP for `n ≤ 16` cities over `u32` weights (`u32::MAX` = absent edge); returns cost plus the lexicographically-smallest optimal tour. |
| `dlx` | `exact_cover` — Algorithm X exact cover over a binary incidence matrix; returns the lexicographically-first sorted row-id solution (`n_cols == 0` → empty cover). |
| `arborescence` | `directed_mst` — Edmonds' minimum-cost arborescence (directed spanning tree rooted at `root`) via cycle contraction; returns total weight plus chosen edge indices. |
| `xorbasis` | `XorBasis` — GF(2) linear basis over `u64` in RREF: `contains`, `max_xor`, `rank`, `kth`-smallest span element; insertion-order-independent canonical form. |
| `eertree` | `Eertree` — palindromic tree over `&[u8]`: `distinct_palindromes`, `palindromic_substring_count`, `occurrences(pat)`, `longest_palindrome` in one online `O(n)` build. |
| `mo` | `mos_order`, `range_distinct` — Mo's offline range queries: block-sorted query permutation plus a sliding-window distinct-count driver. |
| `histrect` | `largest_rectangle`, `maximal_rectangle` — monotonic-stack largest histogram rectangle and running-height maximal all-true submatrix. |
| `stable` | `stable_match`, `is_stable` — Gale–Shapley proposer-optimal stable marriage plus a blocking-pair verifier. |
| `wdsu` | `WeightedDsu` — potential-annotated union-find: `unite(u, v, w)` asserts `pot[v] − pot[u] = w`, contradictions rejected, `diff` answers inside a component. |
| `dominators` | `Dominators` — Cooper–Harvey–Kennedy iterative dominator tree (`idom`, `dominators(v)` chains) plus `frontier` dominance frontiers for control-flow style analysis. |
| `fenwick2d` | `Fenwick2d` — 2-D BIT over a dense `w × h` `i64` grid: `add`, `prefix`, `rect_sum`, `get`, `total` in `O(log w · log h)`. |
| `circulation` | `feasible_circulation` — lower/upper-capacity feasible circulation via super-source/sink reduction over `flow`; returns per-edge flows or `None`. |
| `biconn` | `biconnected_components` — Tarjan edge-stack biconnected decomposition: maximal edge sets sharing a common simple cycle; bridges emerge as singletons. |
| `simhash` | `simhash` / `weighted_simhash` / `hamming` / `near_dupes` — Charikar 64-bit locality-sensitive fingerprints over weighted feature multisets. |
| `gf2` | `add`/`sub`/`mul`/`mul_t`/`inv`/`pow`/`div` + `Tables` — GF(2⁸) field arithmetic over the AES polynomial `0x11B`; the arithmetic layer erasure codes build on. |
| `rsfec` | `ReedSolomon` — Vandermonde Reed–Solomon erasure coding over `gf2`: `encode`, `encode_shards`, `reconstruct` recovers `k` data shards from any `k`-of-`k+m` subset. |
| `fmidx` | `FmIndex` — FM-index over a cyclic BWT: `count`/`locate`/`range` via `C` table + spaced `Occ` checkpoints + full suffix array. |
| `zerobfs` | `zero_one_bfs` / `dial` — linear-ish shortest paths for `{0,1}` and small-cap weights; `Option<Vec<Option<u32>>>` distances, `bellman`-compatible. |
| `dpll` | `solve` — DPLL SAT over CNF: unit propagation, pure-literal elimination, smallest-variable split; canonical models (unset vars = `false`). |
| `intervaltree` | `IntervalTree` — centered interval tree: `stab` (point) and `overlap` (range) queries over `[lo,hi)` spans; `u32` results sorted ascending. |
| `centroid` | `Centroid` — centroid decomposition of a forest: `parent`/`children`/`depth`/`order`/`roots` plus centroid-tree `lca`; depth `<= ceil(log2 n)` guaranteed. |
| `piecetable` | `PieceTable` — editor-style text buffer: immutable `original` + append-only `added` + piece list; `insert`/`delete`/`get`/`to_bytes`. |
| `steiner` | `steiner_tree` — Kou–Markowsky–Berman 2-approx Steiner tree: metric closure -> terminal MST -> path unfolding -> cycle prune. |
| `wal` | `Wal` + `decode` — write-ahead log codec: `[kind|len|crc|payload]` records, Fnv1a checksums, torn-tail-tolerant replay (`Decoded.stopped_at`). |
| `bitap` | `Bitap` — Shift-And bit-parallel search: exact + Hamming-fuzzy (`k` substitutions) over ≤64-byte patterns in one `u64` word per level. |
| `flowfield` | `FlowField` — one-to-all pathfinding: Dijkstra integration field + per-cell direction field; 8-connected, no corner cutting, 2·ortho / 3·diag integer costs. |
| `shunting` | `shunting_yard` + `eval`/`eval_rpn` — infix→postfix + strict `i64` evaluation; truncating `/`/`%`, `None` on overflow, div-by-zero, malformed input. |
| `sat` | `collide`/`overlap` — separating-axis convex collision in `i128`: boundary contact counts, witness = min-overlap axis + projection-unit depth. |
| `buddy` | `Buddy` — binary buddy allocator: sorted lowest-address free lists, eager coalescing, canonical state (no two buddies simultaneously free). |
| `xorfilter` | `XorFilter` — static xor-filter membership: ~0.4% false positives, no false negatives, three-slot XOR lookup (Graf & Lemire). |
| `lru` | `Lru` — least-recently-used cache: canonical (stamp, key) eviction over two BTreeMaps. |
| `vose` | `AliasTable` — Vose's alias method: O(1) weighted sampling, exact integer distribution. |
| `quadtree` | `Quadtree` — bucketed dynamic point index: canonical sorted rect queries, best-first `nearest`. |
| `cartesian` | `Cartesian` — O(n) cartesian tree: heap on values, BST on positions; LCA answers range extrema. |
| `iheap` | `IHeap` — indexed min-heap: key-addressed `set`/`decrease`/`remove`, canonical (priority, key) pop order. |
| `pstree` | `PersistentTree` — chairman persistent segment tree: per-version roots, `kth`/`freq`/`range_count` slices. |
| `crc` | `crc32` / `Crc32` — streaming IEEE CRC-32, chunk-invariant wire integrity. |
| `mcts` | `mcts` — seeded UCB1 tree search over `minimax::Game`, integer-only statistics. |
| `slotmap` | `Slotmap` — generational `u64` handles: stale handles structurally rejected, canonical slot-order entries. |
| `cuckoof` | `CuckooFilter` — deletion-capable membership filter: u8 fingerprints, `h2 = h1 ^ hash(fp)`, bounded kicks. |
| `rans` | `Rans` — rANS entropy codec: normalized power-of-2 table, single-state integer coder approaching the entropy bound. |
| `polylabel` | `polylabel` — pole of inaccessibility: integer B&B maximizing min squared distance to a polygon boundary. |
| `mphf` | `Mphf` — CHD minimal perfect hash: static `n` keys → `[0,n)` bijection via per-bucket displacements. |
| `mis` | `maximal_independent_set` — canonical greedy MIS: ascending-order inclusion, pure function of the edge set. |
| `chacha` | `ChaCha20` — RFC 8439 ChaCha20 keystream: counter-mode streaming XOR cipher, pure state. |
| `sha256` | `Sha256` / `sha256` — FIPS 180-4 SHA-256 digest: incremental writes, canonical padding. |
| `dbscan` | `dbscan` — squared-distance density clustering: canonical index-order expansion, `-1` noise. |
| `kmeans` | `kmeans` — deterministic integer k-means: farthest-point init + Lloyd iterations to fixpoint. |
| `rtree` | `Rtree` — STR bulk-loaded static R-tree: pure-function point index, brute-force-equal queries. |
| `poly1305` / `chacha` | RFC 8439 authenticated-encryption pair — `Poly1305` one-time MAC (5×26-bit DJB limbs, incremental `write`/`finish`) alongside the ChaCha20 keystream module. |
| `veb` / `roaring` | Ordered integer sets: proto van Emde Boas `u32` predecessor/successor (two-level sqrt bitset) and a roaring bitmap (array/bitset containers, sorted iteration, union/intersect/difference/symmetric-difference). |
| `lyndon` / `sosdp` | String canonicalization + lattice algebra — Duval Lyndon factorization, Booth least rotation, subset/superset zeta–Möbius, and OR/AND/XOR convolutions over bitmask tables. |
| `skiplist` / `karatsuba` | Seeded-hash-level deterministic skip list (`u64` ordered set — lane shape is a pure function of the key set) + Karatsuba multi-word multiplication over `u64` limbs. |
| `lsm` / `pgm` | Ordered indexes: log-structured merge index (BTreeMap memtable, frozen sorted runs, tombstone deletes, tiered compaction) and a PGM-style learned index (rational-slope segments, exact ε window search). |
| `aes` | AES-128 block cipher (`Aes128::encrypt`/`decrypt`) — S-box computed via `gf2` inverse + affine transform rather than a stored table; FIPS-197 vectors verified. |
| `fenwickrange` / `vertexcover` | Range-add Fenwick variants (point query + two-BIT range sum over `i64`) and König bipartite minimum vertex cover via `hopcroft_karp` + alternating reachability. |
| `geohash` / `octree` | Spatial coding and 3-D indexing: integer microdegree geohash (encode/decode/cell span/neighbors) and a bucketed octree over `i64` points (sorted range queries, best-first nearest). |
| `base64` | RFC 4648 base64 + base64url strict codec — decode rejects malformed padding and non-alphabet bytes. |
| `siphash` / `hmac` | Keyed digests: SipHash-2-4 streaming PRF (`SipHash`/`siphash`, paper vectors) and RFC 2104 HMAC-SHA256 (`Hmac`/`hmac_sha256`, RFC 4231 vectors). |
| `pairingheap` / `bitonic` | Arena pairing heap with O(1) `meld` and `(prio,key)` canonical pop order; Batcher's bitonic sorting network — a fixed comparator sequence per `n` (data-oblivious, lockstep-safe). |
| `offlinelca` | Tarjan offline LCA — DSU + one DFS answers a whole `(u,v)` query batch in ~`O((n+q)·α)`, matching `lca` semantics including cross-tree/cycle `None`. |
| `elias` / `patricia` | Ordered integer sets: Elias–Fano succinct monotone sequence (`access`/`rank`/`successor` over a sorted `u64` list, unary-gap + verbatim-low encoding) and a crit-bit PATRICIA tree (insert/contains/floor/ceil with sorted-order iteration). |
| `blake2s` | BLAKE2s-256 digest (RFC 7693): streaming `Blake2s` state, keyed-MAC mode without the HMAC construction, official unkeyed/keyed vectors. |
| `rotcal` / `smawk` | Rotating calipers on a convex hull (diameter pair, `Frac` min-width, `Frac` min-area rectangle — no floats) and SMAWK `O(n+m)` row-argmin for totally monotone implicit matrices (Monge-DP machinery). |
| `rope` / `bktree` / `mincircle` / `slopetrick` / `regex` | Text & metric structures: a balanced rope (`O(log n)` insert/remove/split/concat with depth-guard rebuild), a Burkhard–Keller metric tree over `diff::levenshtein` (`within`/`nearest` fuzzy-string queries), Welzl smallest enclosing circle on exact `Frac` points, the slope-trick convex piecewise-linear primitive (`O(log n)` abs/ramp adds, sliding-window `slide`, domain `shift`), and a Thompson-NFA byte regex (`.`, classes, `|`/`?`/`+`/`*`, no backtracking). |
| `cuckoo` / `yfast` | Ordered integer sets: a bounded-kick cuckoo hash table (two-table placement, journal rollback on a failed chain, fixed capacity — no rehash policy to diverge on) and a y-fast–style clustered predecessor trie (rep-ordered buckets that split at their median; `predecessor`/`successor`/`min`/`max` without hashing). |
| `bitboard` / `cyk` / `halfplane` | Board, grammar, and geometry primitives: 8x8 `u64` bitboards (file-masked shifts, dumb7fill sliding attacks for rook/bishop/queen, knight/king/pawn leapers), a CYK recognizer for Chomsky-normal-form grammars over bytes (bitset triangle, `accepts`/`derive`/`cell`), and exact `Frac` half-plane intersection (deque construction, CCW normalized vertices, collinear/dup pruning — `None` for empty, unbounded, or degenerate regions). |
| `radixsort` / `rankselect` | Word-level sorting and bit indexing: LSD radix sort over `u64` (eight stable 8-bit counting passes, plus a stable `(key, payload)` variant) and a Jacobson rank/select bitvector (`rank0/1`, `select0/1` over a static word vector). |
| `debruijn` / `cf` / `earley` | Combinatorics and grammar: de Bruijn `B(k,n)` sequences via the FKM Lyndon-word construction, exact continued fractions (`to_cf`/`convergents`/`best_approx` bounded-denominator rounding), and an Earley chart parser for arbitrary CFGs (ε-rules and mixed-length productions — CYK's CNF restriction lifted). |
| `sais` / `hll` | String indexing and cardinality: SA-IS induced-sorting suffix array (`O(n)` for byte alphabets, where `suffix` uses prefix doubling) and HyperLogLog distinct-count sketch (`p`-bit registers, elementwise-max merge, raw estimate + integer `ln` linear-counting small-range correction — no `f64`). |
| `arith` / `swiss` / `magic` | Coding, hashing, and bitboard tables: a Subbotin carry-less range coder (streaming `encode`/`decode` with caller-supplied frequency models), a SwissTable-style `u64` set (16-slot probe groups, 7-bit `h2` control fingerprints, tombstone deletion, seeded deterministic layout), and magic bitboards (per-square multiply-shift perfect hashes of sliding-piece attack sets, seeded table search, dumb7fill-oracle fallback). |
| `linkcut` / `splay` / `editdist` / `gjk` / `tlsf` | Dynamic trees, text, geometry, and memory: a Link–Cut tree (`link`/`cut`/`connected`/`lca`/`path_min` over a rooted forest — LCA via the access tail), a bottom-up splay BST (self-adjusting, RNG-free amortized balance), Myers' bit-vector Levenshtein (`dist` + `find_leq` approximate-search ends, ≤64-pattern words), an integer 2-D GJK closest-distance between convex polygons (rational `Frac` answers, `overlap`), and a TLSF-style segregated allocator (`alloc`/`free` with coalescing, deterministic lowest-address fit, honest `None` refusal). |
| `sha512` / `ed25519` | Signature-grade crypto: FIPS 180-4 SHA-512 streaming digest (manual BE word loads, 128-bit length field) and RFC 8032 Ed25519 sign/verify (radix-51 field limbs + mod-L scalars, complete Edwards addition, canonicality-rejecting decompress) — the `chacha`/`poly1305`/`aes` layer now covers signatures too. |
| `blossom` / `epa` / `alphahull` | Graphs and geometry: Edmonds' blossom maximum matching on general (non-bipartite) graphs (odd-cycle contraction inside BFS, `O(n³)`), EPA penetration depth between overlapping convex polygons (`Frac`-exact MTV depth² + axis — `gjk`'s overlapping-case companion), and alpha-shape boundaries over `delaunay` (edges of triangles with `circumradius² ≤ α²`, exact `i128` circumradii — α sweeps convex hull → carved cavities). |
| `varint` / `hornsat` / `bigint` / `segbeats` / `ett` | Wire, logic, arithmetic, and dynamic structures: canonical LEB128 + zigzag codec (`encode_u64`/`encode_i64`/`decode_u64`/`decode_i64`/`decode_all` — strict minimal-form rejection: no trailing zero-groups, 10th byte `payload==1` only), Dowling–Gallier linear Horn SAT (`Clause{pos,neg}` + per-variable watch lists — least-model unit propagation), sign-magnitude `BigInt` over u64 limbs (`add`/`sub`/`mul`/`pow`/`cmp`, `to_i128`/`to_u64` with `-(i128::MIN)` handled), segment tree beats (`chmin`/`chmax`/`sum`/`get` — second-maximum/second-minimum ledgers amortize the kinks; reads push because child sums go stale under lazy folds), and an Euler-tour tree (`link`/`cut`/`connected`/`tree_size` — implicit treap of directed half-edge occurrences plus a permanent vertex node per id; connectivity = same treap root via parent climb, no splay exposes). |
| `utf8` / `lazyseg` / `tonelli` / `bsgs` / `dfamin` | Codec, lazy ranges, and finite-field/automaton classics: Höhrmann strict-DFA UTF-8 (`validate`/`check`/`decode`/`decode_lossy`/`encode` + streaming `Decoder` — lossy resync consumes a byte rejected as lead, re-feeds one rejected mid-sequence), the canonical lazy segment tree (`add`/`sum`/`min`/`max`/`get`/`set` — single additive tag, push scales the pending sum by the child's real leaf count `len/2`), Tonelli–Shanks modular square roots (`sqrt_mod`/`is_quadratic_residue` — `p−1 = q·2ˢ` decomposition, sequential non-residue scan, sorted `(lo, p−lo)` pair), baby-step giant-step discrete log (`discrete_log` — `g^{−m} = g^{p−1−m}` needs no inverse helper; zero base resolved before the walk because `f` is only a real inverse for `g ≠ 0`), and Hopcroft DFA minimization (`minimize`/`reachable` — splitter worklist re-queueing only the smaller half, block ids canonicalized to the least member, plus a compact minimized machine that simulates identically). |
| `bspline` / `bmassey` / `xortrie` / `orset` / `lww` | Curves, sequences, and replicated state: integer-exact de Boor B-spline evaluation over `Frac` (`eval` — degenerate spans blend with α=0 rather than skipping the step; the right endpoint `t == u[n+1]` maps to the last non-empty span ≤ n, the left-continuous rule a naive `s = n` clamp breaks at degree 0), Berlekamp–Massey shortest LFSR over GF(p) (`massey` → `C[0]=1` connection polynomial, `holds` verifier, exhaustive `min_complexity` oracle), a bitwise xor trie over u64 (`insert`/`remove`/`contains` multiplicities, `max_xor`/`min_xor`, `max_xor_pair` in `O(n·64)` with canonical `(lo,hi)` dedup — `len` counts live multiplicities), an add-wins observed-remove set CRDT (`add`/`remove`/`contains`/`values`/`merge` — removes cover only observed dots so concurrent adds always survive; `remove` reports whether a live dot existed), and an LWW-element-set CRDT (`add`/`remove`/`contains`/`added_since`/`merge` — the `(clock, replica)` stamp winner decides each element's fate, ties resolve toward remove: orset's dual where an unseen concurrent remove can still win by clock). |
| `sieve` / `grundy` / `gapbuffer` / `robin` / `winnow` | Number theory, game math, and editor internals: a linear SPF sieve plus Eratosthenes and segmented primes (`spf_sieve`/`factor`/`is_prime_table`/`factor_map`/`primes_up_to`/`Primes{phi,tau,sigma}`/`primes_between` — segmented marking clamps `ceil(lo/p)·p` to `p^2`), Sprague–Grundy impartial-game numbers (`mex`/`take_away`/`position_grundy`/`nim_sum`/`winning_move`/`losing`/`detect_period` — a reported period is guaranteed only while the last `memory` cells verified periodic), an Emacs-style gap buffer over `Vec<u8>` (contiguous gap, `move_to` via `copy_within`, `insert`/`insert_byte`/`delete_before`/`delete_after` returning the actual counts), a Robin Hood open-addressing set over u64 (probe-length stealing on insert, backward-shift delete with no tombstones, `max_probe_len`/`total_probe_len` diagnostics, grow at 0.75 with slot-order rehash), and winnowing document fingerprints (`kgram_hashes`/`winnow`/`fingerprint`/`similarity` — every window contributes its rightmost minimum, so a shared run of ≥ k+w−1 bytes must surface a shared hash). |
| `scapegoat` / `leftist` / `beam` / `perceptron` / `saddleback` | Self-tuning structures and search: an α weight-balanced scapegoat tree (insert finds the lowest ancestor with `4·size(child) > 3·size(node)` and median-rebuilds it; deletes let imbalance drift until `len < α·max_size` fires a whole-tree rebuild — the per-node α invariant is only guaranteed post-insert), a leftist heap (`merge`-only core on the `O(log n)` right spine, rank = null-path length, `from_slice` pairwise-meld `O(n)`), deterministic beam search (rank by `(score, generation order)`, parent-link path reconstruction, plus `beam_moves` over `minimax::Game`), an integer perceptron (`i128` score accumulation, saturating `w += y·x` updates, one-vs-rest `OvrPerceptron` — Novikoff convergence verified by update-trace oracle), and saddleback search in a row+column sorted matrix (top-right walk drops a row or column per step, `O(r+c)`, ragged cells beyond `m[0].len()` ignored). |
| `avltree` / `bandit` / `zobrist` / `rsa` / `json` | Balanced trees, bandits, and codecs: a height-balanced AVL tree (rotations restore `|h(l) - h(r)| <= 1` on every mutation — a `BTreeSet` shadow audit runs the invariant after every op), deterministic multi-armed bandits (UCB1 in Q8 fixed point — `mean·256 + isqrt(2·log2(total)·SCALE²·256/pulls)` — plus seeded ε-greedy, all replayable), incremental Zobrist board hashing (XOR `toggle` gives exact undo — `hash == rehash(occupied)` verified every step), textbook RSA over `BigInt` (deterministic keygen via seeded probable-prime search, binary long-division `rem`/`modpow`/`modinv`/Miller–Rabin since `BigInt` has no division — *no padding, not wire-grade crypto*), and a strict integer-subset JSON parser (RFC 8259 minus floats, surrogate-pair escapes, canonical `render` with sorted keys — `parse(render(x)) == x`). |
| `sufftree` / `redblack` / `apsp` / `pagerank` / `automaton` | Text trees, graph metrics, and finite machines: Ukkonen's online suffix tree (`contains`/`occurrences`/`count`/`longest_repeat` — an explicit `SENT` terminator gives every suffix its own leaf; active point + suffix links + skip/count walks), a CLRS red-black BST over `u64` (arena nodes, parent+orientation tracking for the NIL sentinel during delete-fixup; `check()` audits BST order, no red-red, equal black height — `BTreeSet` shadow oracle), all-pairs shortest paths (`floyd_warshall` + `johnson` — i64 edges, i128 internals, Johnson re-weights via `bellman::shortest` potentials so `w' >= 0`, negative cycles detected), integer PageRank in Q32 (`SCALE = 1<<32`, teleport + dangling-share distribution, all-floor semantics, honest `converged` flag), and an NFA→DFA toolkit (Thompson builders + subset construction + `complement(alphabet)`/`intersect`/`minimize` delegating to `dfamin` — `complete()` fills only missing transitions via `entry().or_insert`). |
| `aastree` / `lz4` / `christofides` / `sha3` / `minkowski` | Balanced trees, compression, TSP, hashing, and geometry: an AA tree (Arne Andersson's two-invariant red-black — `level(t) == level(left) + 1` with NIL counted as 0; delete-unwind = `decrease_level` then skew×3 + split×2 — a BTreeSet shadow plus an in-order + level audit on every op), an LZ4 block codec (greedy 4-byte hash parse, `u16` window offsets, nibble+255-extension lengths, strict decode: offset >= 1, literals-only tail, overlapping copies decoded byte-wise), Christofides' 1.5-approximate metric TSP (Prim MST with `(cost, vertex)` tie-breaks -> odd-degree set -> min-weight perfect matching via subset DP for <= 20 odds else sorted greedy -> multigraph Euler via `euler::euler_walk` -> shortcut to a canonical permutation), Keccak-f[1600] (25 u64 lanes, theta/rho/pi/chi/iota x24; SHA3-256/512 domain `0x06`, SHAKE128/256 `0x1F`, pad10*1, `Digest256` streaming whose squeeze keeps its rate position across calls), and convex Minkowski sums (CCW canonicalization from the lowest `(y, x)` vertex, edge-vector merge in angular order `O(n+m)`, `diff(a,b) = sum(a, -b)` so `collide` is a point-in-polygon query on the C-obstacle — brute pair-sum hull oracle). |
| `shamir` / `postman` / `fst` / `fountain` / `edt` | Threshold crypto, route inspection, dictionaries, erasure codes, and distance fields: Shamir's (k,n) scheme over GF(p) (seeded coefficients -> eval at x=1..n -> Lagrange at 0; share swaps provably corrupt the secret), Chinese postman (odd-degree set -> `bellman::shortest` metric closure -> min-weight matching via a `free`-parameterized subset DP — `free=2` yields the open-trail endpoints in one pass; `i128::MAX/4` sentinel since plain `i128::MAX` overflows on any positive add), a minimal acyclic DFA dictionary (sorted word set -> trie -> bottom-up hash-consing register = unique minimal automaton; root swapped back to state 0), an LT fountain codec (robust-soliton degree with `R = sqrt(k)*ln(2k)/4` — the `c=1/10` canonical constant degenerates to `R=1` at small k and reverts to ideal soliton, so we use `c=1/4`; neighborhoods are pure `(k,i,seed)` functions over a partial Fisher-Yates; BP peeling returns `None` on coverage stalls), and Felzenszwalb-Huttenlocher squared EDT (two 1-D lower-envelope-of-parabolas passes, breakpoints kept as `(num,den)` rationals — the -inf sentinel must be `i64::MIN`, not `i128::MAX/4`, or `zn*den` overflows i128). |
| `bplus` / `bentley` / `comb` / `gcm` / `polyclip` | Ordered indexes, sweeps, combinatorics, AEAD, and polygon ops: a B+ tree ordered map `u64 -> u64` (copy-up leaf splits vs move-up internal splits — the separator invariant is "smallest key of the right subtree", machine-audited by `check()`; leaf-first-key deletions propagate via a `fix_sep` ancestor walk), a Bentley-Ottmann intersection sweep (BTreeMap event queue, `(y at x, slope)` re-sort makes concurrent-at-p segments adjacent; verticals never enter status — they pair-test against every involved set for their whole x-line; collinear overlaps report shared endpoints only), combinatorial rank/unrank (exact `choose128` via the `acc = C(n-k+i, i)` invariant — every division exact, no gcd passes; lexicographic combinadics + `next_comb`), AES-128-GCM AEAD over `aes` (bit-serial GHASH with `R = 0xE1<<120`, accumulator *chains* across aad and ct — separate-hash XOR composition is a spec violation; NIST vectors + pure-Python cross-verified), and Sutherland-Hodgman polygon clipping in exact `Frac` (clipper winding normalized via the i128 shoelace sign; intersection parameter `t = cross(cd, a-s)/cross(cd, sd)` — the sign flipped formula was caught by an empty-clip regression). |
| `clique` / `dtw` / `mmheap` / `mstree` / `pathcover` | Enumeration, elastic matching, and heap variants: Bron-Kerbosch maximal cliques over `u64` adjacency masks (Tomita pivot on `P \\ N(u)` maximizes `|P \cap N(u)|`; sorted canonical output), exact-i64 dynamic time warping (`distance`, Sakoe-Chiba `distance_windowed`, `path` recovery — border cells stay INF since an exhausted prefix cannot align), a min-max heap (`push`/`pop_min`/`pop_max`, alternating levels, both ends O(1) — `pop_max` skips the move when the max occupies the tail slot), a merge-sort tree for static range counting `O(log^2 n)` (recursive mid-split layout — a size-2n heap layout is power-of-2 `n` only), and DAG minimum path cover via bipartite matching (`n - |matching|` chains, `None` on cycles, canonical smallest-first Kahn). |
| `jps` / `goap` / `btree` / `verlet` / `vnoise` | Game-AI planning and integer fields: Jump Point Search over `&[u64]` grids (`find` — uniform-cost over jump points with natural + forced neighbors; corner-touching is part of the paper's pruning lemmas, a strict no-corner-cut rule silently changes reachability), a goal-oriented action planner (`plan` — bitmask-world Dijkstra keyed `(cost, seq, state)` so the lex-min plan is a pure function), behavior trees with resume semantics (`tick` — sequences/selectors remember the Running child in per-node `mem`, Conditions map Running→Failure), integer Verlet integration (`World::step` — Jakobsen distance-constraint relaxation in `Fixed`; undamped runs conserve energy so links oscillate through equilibrium, damping is what converges), and integer value noise + fBm (`Noise2` — seeded SplitMix64 lattice hashes, smootherstep bilinear interpolation, arithmetic-shift cell flooring is negative-coordinate safe). |
| `bdd` / `simplex` / `peg` / `radixheap` / `life` | Logic, LP, parsing, heaps, automata: reduced ordered binary decision diagrams (`Bdd` — hash-consed `(var,lo,hi)` unique table + memoized Shannon `apply`, so equal formulas share node ids), an exact-rational two-phase simplex LP solver (`maximize` — `Frac` tableau with Bland's smallest-index entering/leaving rule, so termination is a theorem and the pivot trace is deterministic), a packrat PEG parser (`parse`/`parse_full` — `(rule,pos)` memoized ordered choice with a left-recursion guard and a no-progress break in `Star`/`Plus`), a monotone radix heap (`RadixHeap` — `msb(key XOR last)` bucketing, not `bit_len(key − last)`: under subtraction a stale bucket can hide a smaller key below the scan frontier), and sparse B3/S23 Life (`World` — the `BTreeSet` live set IS the canonical state, neighbor counts in one sorted pass). |
| `fibheap` / `halton` / `dihedral` / `turnpike` / `kpaths` | Heaps, sampling, symmetry, reconstruction, path enumeration: a Fibonacci heap over an arena (`FibHeap` — circular sibling rings, cut-and-cascade `decrease_key`, degree consolidation on `pop`, canonical `(key, seq)` FIFO ties; a `live` bit guards stale handles), Halton low-discrepancy points (`radical_inverse`/`Halton` — exact `Frac` digit reversal, `b`-base stratification verified as a property test, Iterator impl), the dihedral group `D4` (`D4::{apply,compose,inverse}` — `(swap, sx, sy)` closed form, the whole 8×8 Cayley table oracle-checked against pointwise application), Skiena's turnpike backtracking (`reconstruct` — `n(n−1)/2`-count inference, multiset `need` checks are *multiplicity-aware*, DFS-first is deterministic, `canonical_turnpike` folds reflections), and Yen's k-shortest loopless paths (`yen` — spur deviations over a canonical `(cost, path)` candidate heap; equal-cost tie order is implementation-defined, the contract is the k smallest costs). |
| `fps` / `rectunion` / `intervalgraph` / `stirling` / `onion` | Formal power series, sweeps, scheduling, combinatorics, and nested hulls: a GF(998244353) FPS kit (`inv`/`log`/`exp`/`pow` — Newton iterations that must double a *tracked degree* `m`, not `g.len()`: a trailing-zero product shrinks `norm`'d `g` and the naive loop stalls forever — the oracle caught it as a timeout, the fix is `m = min(2m, n)` driving), union rectangle area (`union_area` — x-sweep events, per-slab y-interval union recomputed in `O(n²)`, `i128` output), interval-graph scheduling (`max_independent_set` earliest-finish, `min_rooms` sweep depth = chromatic number, `weighted_select` `O(n log n)` DP with canonical predecessor links — degenerate `[s,s)` items are unselectable everywhere), Stirling numbers mod p (`s1`/`us1`/`s2`/`bell`/`falling_coeffs` — signed first-kind verified by enumerating every permutation's cycle count via `perm::unrank`, second kind by the closed `1/k!·Σ(−1)^j C(k,j)(k−j)^n` form), and convex-onion layers (`onion_layers` — a point *on* a hull edge is not a hull vertex, so it survives its peel and can surface as its own degenerate layer). |
| `karp` / `hirschberg` / `matchain` / `seamcarve` / `ost` | Cycle means, alignments, DP orders, carving, and rank lookups: Karp minimum mean cycle (`min_mean_cycle` — `min_v max_k (dp[n][v]−dp[k][v])/(n−k)` over length-k walks; the returned cycle is the first repeat of the full n-edge backtrack — trimming only the suffix finds a walk, not a cycle), Hirschberg linear-space global alignment (`align` — mid-split `L[j]+R[n−j]` recursion, `O(min)` rows of space, leftmost-split canonical script that `apply`-replays to `b`), matrix-chain order (`order` — `O(n³)` DP with leftmost-argmin split table, postorder `Step` list), seam carving (`energy`/`find_vseam`/`remove_vseam` — squared one-sided-gradient energy, DP with leftmost ties, malformed seams rejected not clamped), and a deterministic order-statistic treap (`Ost` — priority is `splitmix64(seed⊕mix(key))`, so shape is a pure function of the key set; `select`/`rank`/`lower_bound` walk subtree sizes). |
| `kkpart` / `xfast` / `seglazy` / `tunstall` / `ortho` | Partitioning, predecessor sets, beats+lazy, phrase codes, and exact QR: Karmarkar–Karp differencing (`partition` — residual heap carries `(plus, minus)` index masks so the returned bound is a concrete achievable partition, hence `d ≥ optimal` by construction), an x-fast trie (`Xfast` — 65 level tables of prefix→`(min,max)` leaf bounds, binary search on levels finds the divergence node in `O(log U)`, leaves double as the `BTreeSet` oracle), lazy segment-tree beats (`SegLazy` — `add`/`chmin`/`chmax`/`sum`: `push` must replay the pending add into children *before* clamping them to parent extrema, else a stale-low child gets clamped against the pre-add ceiling), Tunstall variable-to-fixed codes (`Tunstall` — greedy max-probability leaf expansion to `2ᵏ`, probabilities compared exactly via `BigInt` cross-multiplication so no overflow can bias the greedy choice; DFS-order canonical codewords), and exact Gram–Schmidt QR (`qr` over `Frac` — orthogonal-but-unnormalized `Q` with `QᵀQ = diag`, unit-diagonal `R`, `A = Q·R` exactly; dependent columns return the zero vector). |
| `ratbezier` / `polya` / `minq` / `ssw` / `ternary` | Weighted curves, orbit counting, monotone queues, local alignment, and unimodal search: rational Bézier evaluation (`eval`/`eval_deriv` — homogeneous de Casteljau on `(w·x, w·y, w)` lifts divided back exactly, so every point on a NURBS-style curve is a `Frac` pair; derivative via the degree-1 difference curve and the quotient rule), Pólya/Burnside orbit counts (`burnside`/`necklaces`/`bracelets` — `#orbits = (1/|G|)·Σk^{cycles(g)}` over `BigInt`, with exact limb-wise division by `|G|`; `necklaces(4,3) = 24`, `bracelets(6,2) = 13`), a minimum queue (`MinQueue`/`MinStack` — each stack slot carries its running min so `pop`/`min` are `O(1)` amortized; the `in→out` pour rebuilds out's minima in one pass), Smith–Waterman local alignment (`local` — 0-floor restart cells, earliest-`i`-then-`j` canonical winner cell, traceback to the restart for exact witness ranges; oracle = max over all substring pairs of the global NW score), and discrete ternary search (`argmin_seq`/`argmin_domain` — the contract is *strict* unimodality: on a flat staircase like `11,11,8` an equality probe cannot confine the argmin to either side, so the tail window is evaluated exhaustively and the result is checked against a global-min scan — `None` rather than a silent wrong index). |
| `ec` / `adic` / `gray` / `partitions` / `chrompoly` | Elliptic curves, p-adic arithmetic, Gray codes, partition numbers, and chromatic counting: affine GF(p) curve points (`Curve::add`/`double`/`neg`/`mul` — chord-tangent law over `i128` intermediates, `on_curve` precondition explicit; `p < 5` or non-field use is rejected honestly), truncated p-adic integers (`Adic` — exact `add`/`sub`/`mul`/`neg` mod `pᵏ`, `inv`/`div` for gcd-units with composite `p` handled correctly, `val` the p-adic valuation, `lift`/`trunc` between precisions), binary-reflected Gray codes (`to_gray`/`from_gray`/`sequence`/`SubsetWalk` — one-bit-step subset traversal that also reports *which* bit flipped), integer partitions (`count` via Euler pentagonal over `BigInt`, `count_bounded`/`count_distinct` DP triangles shadowing it — including Euler's theorem distinct-parts = odd-parts checked both ways, `enumerate` in descending-lex order), and exact chromatic counting (`count`/`count_poly`/`chromatic` — deletion–contraction `P(G)=P(G−e)−P(G/e)` over `BigInt`, verified against brute `kⁿ` enumeration and the C₄ closed form). |
| `pell` / `farey` / `mobius` / `jacobi` / `egypt` | Pell equations, Farey sequences, Möbius inversion, the Kronecker symbol, and Egyptian fractions: `pell` solves `x² − d·y² = ±1` exactly (`surd_cf` — the `(m,d,a)` CF recurrence for `√d`, `solve`/`negative`/`power` over `BigInt` convergents; the d = 61 fundamental (1766319049, 226153980) and OEIS fundamentals verified), `farey` generates `F_n` by the one-step next-term recurrence (`farey`/`neighbor`/`stern_brocot` paths + `floor_sum`, the ACL lattice-count primitive over `u128`, oracle-checked by direct summation), `mobius` is divisor-lattice algebra (`mu_sieve` linear sieve + single `mu` via SPF, `convolve`/`invert` Dirichlet convolution/inversion, `coprime_count` inclusion–exclusion, `sigma_sum`; `f ∗ μ` recovers f exactly in a 200-case roundtrip), `jacobi` computes `(a|n)` for all integers by binary reciprocity (verified against the Euler criterion `a^((p−1)/2) mod p` on 4000 random prime cases and multiplicativity `(a|mn) = (a|m)(a|n)`), and `egypt` decomposes proper `Frac`s into distinct unit fractions Fibonacci–Sylvester greedily with the numerator-descent invariant as the termination proof. |
| `lucas` / `frobenius` / `josephus` / `bernoulli` / `eulerian` | Classical discrete math: `lucas` computes `(U_k, V_k)` Lucas sequences exactly in `i128` plus a fast-doubling mod variant (`lucas_mod` returns `(U,V,Qᵏ)` mod odd `m` — halving is driven by the residue's parity, verified by the `V²−D·U²=4Qⁿ` identity and scan-vs-doubling on 3000 cases), `frobenius` solves the coin problem via min-residue Dijkstra (`dist[r]` = smallest representable `≡ r mod m`, `g = max dist − m`; Sylvester's `ab−a−b` and a DP oracle cross-checked, McNuggets = 43), `josephus` gives `J(n,k)` by the `O(n)` recurrence, the `2l` closed form for `k=2`, and the full elimination order in `O(n log n)` through the order-statistic treap (brute `Vec` oracle on 400 cases), `bernoulli` computes `Bₙ` exactly by Akiyama–Tanigawa over `Frac` plus Faulhaber power sums (direct-sum oracle, `B_{2k+1}=0`, the generating recurrence all verified), and `eulerian` computes `⟨n k⟩` over `BigInt` and enumerates descents-k permutations (row-sum `n!`, Worpitzky identity, enumeration length all checked). |
| `catalan` / `derange` / `zeckendorf` / `digit` / `hanoi` | Combinatorial counting + generation: `catalan` is the `BigInt` Catalan family (`binomial` exact, `catalan` recurrence, `ballot` numbers `(a−b)/(a+b)·C(a+b,a)`, `dyck` balanced-string generation — count = `C_n`, all validity + uniqueness checked), `derange` computes derangements `!n` and rencontres numbers `R(n,k)` over `BigInt` plus full enumeration (row-sum `n!` and fixed-point histograms oracle-checked), `zeckendorf` is greedy Zeckendorf Fibonacci representation with `decode`/`is_zeckendorf` (existence AND uniqueness both oracle-verified by non-adjacent-subset enumeration), `digit` does digit DP counting `x ∈ [0,n]` by digit properties (`count_avoid_digit`, `count_digit_sum` — the `started` flag keeps leading zeros from counting as digits; brute oracles to n ≤ 20,000), and `hanoi` produces the optimal `2ⁿ−1` move list, `move_at(k)` pointwise through the `2ⁿ⁻¹−1` split (the `ctz(k+1)` disk characterization verified), and `state` peg assignments after `k` moves — a full legality simulation checks every move. |
| `imptreap` / `meetmid` / `modlin` / `veb3` / `bigedit` | Sequences and frontier structures: implicit-key treap `O(log n)` insert/remove/reverse with lazy flags (arena nodes, seeded priorities — `get` accumulates flip parity down the descent), meet-in-the-middle subset search `O(2^(n/2)·n)` with index witnesses, GF(p) linear algebra (`rref`, `solve`, `rank`, nullspace over `u64` residues), a recursively-structured van Emde Boas `O(log log U)` predecessor/successor set (leaf masks + summary/cluster recursion — min lives outside clusters, the missing-predecessor fallback), and multi-word Myers edit distance for patterns beyond 64 bytes (`⌈m/64⌉`-word carry chaining in the `(Eq&Pv)+Pv` add and the `Ph`/`Mh` left-shifts). |
| `voronoi` / `delaunay` | Exact nearest-seed partition (`voronoi_partition`, `voronoi_flood` through passable terrain), `mst_edges` / `mst_edges_over` (Kruskal MST over a complete or restricted graph), and integer-exact Delaunay triangulation (`delaunay`, `delaunay_edges`) — scatter → territory → connectivity. |
| `steer` / `reroot` / `pbs` / `crdt` / `quat` | Movement, trees, offline search, replicated state, and 3D rotation: Reynolds steering behaviors (`seek`/`flee`/`arrive`/`pursue`/`evade`/`wander`/`separation`/`combine` — force = desired − velocity truncated to `max_force`, all `Fixed`; wander jitters off `SplitMix64`), reroot DP over a tree (`reroot` — the generic `merge`/`lift` monoid pattern; distance sums need a `(sum, count)` pair so the lift adds subtree size, while `+1` lifts only fit max-type aggregates like eccentricity — a degenerate-monoid test caught the distinction), parallel binary search (`parallel_binary_search` — rebuilds `ctx` per pass so `apply` needs no inverse; the contract is monotone predicates, and a non-firing query resolves to `None` by a final check at `t = lo`), CRDTs (`GCounter`/`PNCounter`/`TwoPhaseSet`/`GSet` — joins are commutative-associative-idempotent by construction: elementwise max for counters, union plus sticky tombstones for sets; remove wins *forever* since re-adds would need tagged elements), and fixed-point quaternions (`Quat` — Hamilton `mul`, Cayley `v + 2·(q⃗ × (q⃗×v + w·v))` rotate, `between` with a largest-perpendicular fallback at ~180°, `nlerp` with hemisphere flip; no `slerp` — there is no `acos`, stated in the docs). |
| `catmull` / `viterbi` / `spring` / `pid` / `kalman` / `gnoise` | Curves, decoding, and control/estimation on `Fixed`: Catmull-Rom & Hermite splines (`catmull_rom`, `hermite`, `sample` with mirrored phantom endpoints, `centripetal_sample` — the Barry–Goldman 3-lerp pyramid over √|Δ| knots, so clustered control points can't pinch loops; exact `i128` basis arithmetic), integer Viterbi decoding (`viterbi`/`viterbi_cost` — additive-cost HMM most-likely path, `O(T·S²)` DP with smaller-predecessor tie-break for replay stability), exact critically damped spring (`spring::step`/`Spring` — the closed-form `(Δ + (v₀+ωΔ)t)e^{−ωt}` in `Fixed::exp`, frame-rate independent, no overshoot; SmoothDamp without Unity's rational `exp` approximation), discrete PID (`Pid` — derivative-on-measurement so setpoint steps don't kick, clamped-integral anti-windup, output clamp), scalar Kalman (`Kalman` — predict/update with `K = P/(P+R)`; `r = 0` sensors are trusted exactly), and Perlin gradient noise (`perlin2`/`fbm2` — zero at every lattice point, quintic `6t⁵−15t⁴+10t³` fade, corner-hash gradient selection — the smooth-field gap `noise` value/fbm couldn't fill). |
| `ray` / `funnel` / `affine` / `hmm` / `roots` / `glob` | Geometry queries, path smoothing, transforms, sequence inference, numerics, and matching: analytic ray queries (`hit_aabb` slab, `hit_circle` half-coefficient quadratic, `hit_segment`/`hit_polygon` cross-product `t` — the continuous counterpart to `geometry`'s grid DDA), funnel string-pulling (`funnel` — Mononen's simple funnel over a `(left, right)` portal channel, `O(n)` apex-restart scan; portal gates may be crossed freely, walls never), 2-D affine matrices (`Affine` — `[a b c; d e f; 0 0 1]` translate/rotate/scale/shear, `then` composition, adjugate `invert`), scaled HMM forward-backward (`hmm::forward`/`backward`/`posterior`/`log_likelihood` — Rabiner's `c_t` scaling so long observation runs don't underflow; `viterbi` gives the best path, this gives the marginals), fixed-budget scalar root finders (`roots::bisect`/`secant`/`newton` — replay-stable iteration counts, no tolerance loops), and bytewise glob matching (`glob_match` — `*`/`?`/`[a-z]`/`[!x]`/`\` escapes, two-pointer star-backtrack). |
| `sobol` / `biquad` / `ccl` / `sap` / `pchip` / `align` | Quasi-random sampling, filtering, image analysis, broadphase, interpolation, and sequence alignment: Sobol' direction numbers (`sobol2`/`Sobol2` — a true `(0,2)`-sequence with Joe–Kuo initial `m = 1, 3`, verified by 4-block quadrant and 16-block 4×4 stratification; the `m = 1, 1` shortcut breaks the net property and casual tests miss it), RBJ biquad IIR (`Biquad::lowpass`/`highpass`/`bandpass`/`notch` — cookbook coefficients normalized by `a₀`, Direct-Form-I state, replayable bit-exact), two-pass connected-component labeling (`label4`/`label8` — union-find over provisional labels with raster-compacted final ids; `areas`/`bboxes`/`cells` accessors), sweep-and-prune broadphase (`sap::pairs` — min-x sort + active-list prune, positive-area overlap only so touching edges and zero-area boxes report nothing, `O(n log n + k)` with sorted output), Fritsch–Carlson monotone cubic (`Pchip` — weighted-harmonic-mean knot tangents with sign-flip zeroing and the 3×-secant endpoint cap; Hermite-basis `eval`, flat extrapolation, no spline overshoot on steps), and pairwise sequence alignment (`needleman_wunsch`/`smith_waterman` — integer-score DP + op traceback `=`/`X`/`I`/`D` with fixed diagonal-first tie-break; `editdist` gives the number, this gives the alignment). |
| `lcp` / `worley` / `swept` / `kdf` / `csv` / `ulid` / `aead` | String-structure companions, procedural texture, continuous collision, key derivation, tabular data, sortable IDs, and authenticated encryption: Kasai LCP array over `sais` (`kasai` — `O(n)` rank-array walk; `longest_repeated`, `distinct_substrings`, `longest_common` via a separator splicing trick), Worley/cellular noise (`worley`/`worley_edge` — avalanche-hash feature points, 3×3 neighborhood F1/F2 Euclidean distances, Lipschitz-bounded `Fixed` field), swept AABB continuous collision (`swept_aabb` — Minkowski-expanded slab intervals with exact `Frac` entry/exit times; catches tunneling a discrete overlap can never see, plus a contact normal and truncation-toward-start contact position), PBKDF2 + HKDF (`pbkdf2`/`hkdf_extract`/`hkdf_expand` — RFC 2898/5869 over `hmac_sha256`; HKDF caps at the spec's 255·32 output), RFC 4180 CSV (`parse`/`parse_str`/`emit` — quoted fields, `""` escapes, `CRLF`/`LF`/`CR` endings, lazy-quote junk kept verbatim; `emit` round-trips exactly), ULIDs (`ulid`/`decode`/`Ulid` — 48-bit ms + 80-bit rand in Crockford Base32, lexicographically sortable, seeded monotonic generator that increments `rand` within one timestamp and spills into the clock on overflow), and ChaCha20-Poly1305 AEAD (`seal`/`open` — RFC 8439 construction over `ChaCha20` + `poly1305`; any tampered bit or wrong context returns `None`, never garbled plaintext). |
| `otsu` / `integral` / `pcg` / `jaro` / `md5` / `poisson` / `semver` | Image analysis, pseudo-randomness, string metrics, legacy checksums, blue-noise placement, and version algebra: Otsu automatic threshold (`otsu_threshold` — exact-rational between-class variance maximized over the 256-bin histogram, first-maximizer convention so `pixel > t` is foreground; `threshold_image`/`binarize`), summed-area tables (`Sat` — `(w+1)×(h+1)` zero-border integral image; `sum_rect` rectangle sums in `O(1)` via the `A+D−B−C` inclusion order that can't underflow `u64`, `mean_rect`/`box_mean`/`total`/`dims` — the `ccl`/`worley` raster family companion), PCG PRNG (`Pcg` — O'Neill's permuted congruential `XSH-RR` output over the standard `6364136223846793005` multiplier; canonical seed/sequence ordering, `next_bounded` Lemire multiply-shift — the deterministic sibling of `SplitMix64`/`rng_xoshiro`), Jaro–Winkler similarity (`jaro`/`jaro_winkler` — exact `Fixed` rational of the window-match/transposition formula, winkler prefix boost gated at `j > 7/10` — a different distance family than `editdist`'s Levenshtein or `fuzzy`'s subsequence score), MD5 (`md5` — RFC 1321 little-endian-word digest; compatibility checksum, not crypto), Bridson Poisson-disk sampling (`poisson_disk` — seeded `Pcg` ring darts around an active list with `r/√2` grid acceleration; every pair `≥ min_dist`, blue-noise density where `sobol`'s low discrepancy doesn't enforce separation), and SemVer 2.0 (`SemVer::parse`/`Ord` — strict-spec parse with leading-zero/empty-identifier rejection, numeric<alphanumeric pre-release ordering, build metadata retained but ordering-ignored; `satisfies_caret`/`satisfies_tilde` npm-style ranges). |
| `kcore` / `otp` / `anneal` / `soundex` / `bm25` / `mdp` / `rsync` | Graph cores, one-time passwords, metaheuristics, phonetic codes, IR ranking, planning, and byte-stream sync: Batagelj–Zaversnik k-core decomposition (`coreness`/`degeneracy` — bucket-sorted peeling `O(n+m)`, adjacency-driven), RFC 4226/6238 HOTP/TOTP (`hotp`/`totp` — private SHA-1+HMAC inside the module so the published vectors verify verbatim, dynamic-truncation `DT`, digits clamped 1–9), simulated annealing (`anneal` — geometric `ln`-space cooling `t0→t1`, `SplitMix64` Boltzmann acceptance in `Fixed::exp`, keep-the-best return — the kit's first metaheuristic), refined Soundex (`soundex`/`soundex_eq` — NARA grouping with the first letter's code seeding the run, H/W transparent, vowels resetting — Pfister→P236), Okapi BM25 (`Bm25` — `idf` over the exact rational `(2N+2)/(2df+1)` through `Fixed::ln`, `Fixed` tf-saturation and length normalisation, deterministic index-tie ordering — the first IR ranking primitive), finite-MDP solvers (`value_iteration`/`policy_iteration` — Bellman `max_a Σ p·(r+γV)` in `Fixed`, explicit sweep budgets for replay stability, the control sibling of `hmm`/`kalman`), and rsync-style byte deltas (`signature`/`delta`/`apply` — rolling a/b weak sum + MD5 strong confirm, `Lit`/`Copy` op stream; the raw-byte counterpart to `delta`'s map-level diffs). |
| `dual` / `qlearn` / `porter` / `dither` / `goertzel` / `qoi` / `civil` | Automatic differentiation, reinforcement learning, stemming, halftoning, tone detection, image codec, and calendar arithmetic: forward-mode dual numbers (`Dual` — value+derivative pairs over `Fixed` with `mul`/`div`/`powi`/`exp`/`ln`/`sqrt`/`sin`/`cos` chain rules; the differentiation substrate `roots`/`pid` callers can compose), tabular RL (`QLearn` — ε-greedy `select`, off-policy `update` bootstrapping `max Q'`, on-policy `update_sarsa`; the learned sibling of `mdp`'s planner), Porter 1980 stemming (`stem` — all five steps with the `m` measure, `*v*`/`*d`/`*o` conditions; ~60 canonical final outputs verified, e.g. `relational`→`relat` since step 4 also cuts `-al`), grayscale halftoning (`bayer`/`ordered`/`floyd_steinberg` — 4×4/8×8 Bayer thresholds, FS error diffusion whose feedback uses the *clamped* pixel so pathological fields can't diverge — the `otsu`/`ccl` raster family), Goertzel single-bin power + DTMF (`power`/`power_fixed`/`dtmf` — i64-raw recurrence since resonant sums overflow `i32`, twist check rejects lone tones; all 16 keypad digits verified), the QOI image codec (`encode`/`decode` — run/index/diff/luma/literal ops, 62-run cap, index hashing, truncated-stream `None`), and Hinnant civil-calendar arithmetic (`days_from_civil`/`civil_from_days`/`weekday`/`weekday_iso`/`is_leap`/`days_in_month` — era-floored inverses, exact for years below 1970; the `cron` time family's date substrate). |
| `jwt` / `uuid` / `cbor` / `xxhash` / `elo` / `braille` / `utility` | Tokens, identifiers, codecs, hashing, ratings, terminal rendering, and AI decision-making: HMAC-SHA256 JWT sign/verify (`sign`/`verify` — fixed `HS256` header, `exp`/`nbf`/`iat` claims vs `now`, constant-time MAC compare), RFC 4122 UUID v4 (`uuid4`/`format`/`parse` — `SplitMix64`-driven, version/variant bits stamped; accepts hyphenated or bare hex, any case), deterministic CBOR (`Cbor`/`encode`/`decode`/`decode_prefix` — RFC 7049 canonical: shortest-form lengths, sorted map keys, definite lengths only — `json`'s binary sibling), xxHash32/64 (`xxh32`/`xxh64` — Collet r.5 spec vectors pinned, seed-parameterized), Elo + Glicko-1 (`expected`/`update`/`update_pair` zero-sum exchange; `Glicko` with `decayed`/`update` in i64 Q32 domain since variance denominators underflow Q16), Unicode braille canvas (`Braille` — 2×4-dot cells → `U+2800` blocks, `set`/`get`/`clear_pixel`/`render` for sub-cell terminal resolution), and Utility AI (`Consideration`/`Curve::{Linear,Quad,Inverse,Logistic,Step}` — Dave Mark response curves, compensated product scoring, `choose` argmax with lowest-index ties — deterministic `goap` alternative). |
| `msgpack` / `snowflake` / `sdf` / `ik` / `tournament` / `plot` / `brent` | Codec, ids, fields, kinematics, brackets, terminal charts, and root-finding: MessagePack codec (`Msg`/`encode`/`decode`/`decode_prefix` — smallest canonical int forms, `cbor`'s noncanonical sibling), Twitter snowflake ids (`Snowflake` — 41-bit ms + 10-bit worker + 12-bit seq, monotonic under clock rollback, seq-overflow bumps virtual ms), 2-D signed distance fields (`circle`/`rect`/`segment` + `union`/`intersect`/`subtract`/`smin` + `march` sphere-tracing — Íñigo Quilez formulas, geometry's continuous side), inverse kinematics (`two_bone` closed-form law-of-cosines in i64 raw space, `ccd` cyclic coordinate descent — CORDIC `atan2` driven, bit-exact replays), tournament brackets (`Bracket` single-elim with canonical recursive seeding + byes, `DoubleBracket` winners/losers/queue + grand final, `Swiss` score-group pairing with rematch avoidance), ASCII/Unicode plotting (`sparkline` 8-block levels, `Canvas` bottom-up `set`/`render`, `line` bucket-mean plots, `histogram` proportional bars), and Brent's method (`solve` — bisect/secant/IQI hybrid with Brent's acceptance tests, bracketed `roots` counterpart, best-effort bound on budget exhaustion). |
| `inflate` / `mt` / `fft` / `uri` / `snoise` / `qr` / `calendars` | Decompression, PRNG, transforms, identifiers, fields, codes, and dates: DEFLATE decompressor (`inflate`/`inflate_zlib`/`inflate_gzip` — stored/fixed/dynamic blocks, canonical Huffman, back-references), Mersenne Twister (`Mt19937` — Matsumoto–Nishimura init-by-array + twist, real-f64-free `next_fixed`), radix-2 Cooley–Tukey FFT (`fft`/`ifft`/`dft`/`magnitudes`/`peak_bin` — `Cx` fixed-point complex, non-pow2 falls back to exact DFT), RFC 3986 URI (`parse`/`resolve`/`pct_decode` — scheme/authority/path/query/fragment), Gustavson simplex noise (`simplex2`/`fbm2` — gradient noise's rotation-free sibling, lattice-zero), QR encoder byte-mode v1–10 (`encode` → `ModuleMatrix`/`to_terminal` — GF(2⁸) RS interleave, 8-mask penalty evaluation), and fixed-date calendars (`julian`/`hebrew`/`islamic`/`persian`/`easter` — Reingold–Dershowitz conversions off `civil`'s serial day). |
| `deflate` / `png` / `zip` / `murmur` / `ip` / `uuid7` / `base58` | Compression, images, archives, hashing, addresses, and ids: DEFLATE compressor (`deflate`/`deflate_stored`/`deflate_zlib`/`deflate_gzip`/`adler32` — greedy depth-1 LZ77 into fixed-Huffman blocks, `inflate`'s emit half), PNG codec (`decode`/`encode_gray`/`encode_rgb`/`encode_rgba` — 8-bit non-interlaced gray/palette/RGB/GA/RGBA, all five filters, CRC-verified chunks), ZIP archives (`list`/`extract`/`ZipWriter` — EOCD scan, stored+deflate methods, CRC-checked), MurmurHash3 (`murmur3_32`/`murmur3_128` — x86_32 and x64_128 variants, reference vectors pinned), IP addresses (`Ipv4`/`Ipv6`/`Cidr` — RFC 4291 `::` compression, embedded v4 tails, RFC 5952 canonical format, prefix containment), UUIDv7 (`uuid7`/`Uuid7` — RFC 9562 48-bit ms timestamp + monotonic virtual bump on clock stall — `uuid`'s sortable sibling), and Base58 (`encode`/`decode`/`encode_check`/`decode_check` + `BTC`/`FLICKR` alphabets — big-integer digit emission, double-SHA-256 checksum). |
| `tar` / `gif` / `midi` / `geo` / `bech32` / `punycode` / `ws` | Archives, images, scores, geodesy, checksummed strings, IDN, and framing: POSIX ustar (`list`/`extract`/`TarWriter` — checksum-verified headers, name/prefix split, deterministic writer), GIF89a decoder (`decode` — GIF-specific variable-width LZW with clear/EOI codes and dictionary growth, palette-index output matching `png`'s shape, non-interlaced scope), SMF parser (`parse`/`notes`/`tempo_map` — delta varints, running status, meta/SysEx, note on/off pairing), geodesy (`haversine`/`lambert`/`bearing`/`dest`/`midpoint`/`norm_lon` — spherical + Lambert–Andoyer flattening correction, km in `Fixed` since meters overflow Q16), Bech32/Bech32m (`encode`/`encode_m`/`decode`/`encode_segwit`/`decode_segwit`/`convert` — BCH `polymod` checksum, BIP-173/350 vectors, v0↔bech32 v1+↔bech32m consensus binding), Punycode (`encode`/`decode` — RFC 3492 bootstring, bias adaptation, DNS label cap), and WebSocket frames (`encode`/`decode`/`accept_key` — FIN/opcode, 16/64-bit lengths, masking, RSV+control-frame legality, SHA-1+base64 handshake). |
| `bmp` / `bencode` / `nbt` / `dns` / `checkcode` / `wkt` / `expr` | Images, wire formats, checksums, geometry text, and a calculator: BMP codec (`encode`/`decode` — uncompressed BI_RGB 24/32-bit, bottom-up or top-down, canonical 24-bit writer), Bencode (`decode`/`decode_prefix`/`encode` — BEP 3 ints/strings/lists/dicts, canonical sorted-key emit), Minecraft NBT (`parse`/`encode` — all 13 tags incl. lists/arrays, floats as raw bits, depth-limited), DNS wire (`build_query`/`parse` — compression pointers followed with loop guards, A/AAAA/CNAME/MX/SOA/TXT rdata), check digits (`luhn`/`isbn10`/`isbn13`/`ean13`/`upca`/`iban`/`vin`/`mrz_*` — Luhn, GS1, mod-97 with the country-length registry, ISO 3779, ICAO 9303), Well-Known Text (`parse`/`write` — POINT/LINESTRING/POLYGON/MULTI*/COLLECTION/EMPTY, 2-D `Fixed`, shortest-decimal emit), and a Pratt expression evaluator (`eval` — precedence-climbing TDOP, `i64` literals + vars + min/max/abs/clamp, checked arithmetic). |
| `toml` / `nmea` / `tle` / `obj` / `srt` / `sgf` / `otpauth` | Config, location, orbit, mesh, subtitle, game-record, and MFA formats: TOML (`parse`/`encode` — scalars/arrays/tables/array-of-tables/dotted keys, floats as `Fixed`, datetimes preserved as strings, canonical emit), NMEA 0183 (`parse`/`build`/`gga`/`rmc` — XOR checksum, `Fixed` lat/lon), Two-Line Elements (`tle::parse` — 69-column checksum, epoch/mean-motion fields, `period_seconds`), Wavefront OBJ (`obj::parse` — v/vt/vn/f with negative indices, `triangles`/`bbox`), SubRip subtitles (`srt::parse`/`emit`/`shifted` — `HH:MM:SS,mmm`, canonical CRLF), Smart Game Format (`sgf::parse`/`emit`/`moves`/`game_info`/`coord` — trees, variations, escapes), and otpauth URIs (`otpauth::parse`/`Otp::to_uri`/`Otp::code` — RFC 4648 base32 + HOTP/TOTP). |
| `wav` / `vtt` / `ass` / `pgn` / `stl` / `ini` / `udiff` | Audio, subtitle, game-record, mesh, config, and patch formats: RIFF/WAVE PCM (`parse`/`encode`/`samples_i16` — PCM/WAVEFORMATEXTENSIBLE tags, fmt+data chunks, chunk padding), WebVTT (`parse`/`emit`/`parse_time`/`emit_time` — `WEBVTT` magic, cue ids, `HH:MM:SS.mmm`, settings, NOTE/STYLE/REGION blocks), SSA/ASS (`parse`/`emit`/`ass_time`/`Event::start`/`end`/`text` — `Format:`-driven columns, `H:MM:SS.cc`, verbatim `Text`), PGN (`parse`/`emit`/`result` — `[Tag]` pairs, SAN movetext, `$n` NAGs, `{}` comments, `()` variations, terminators, black-to-move), STL (`parse`/`parse_ascii`/`parse_bin`/`emit`/`bbox` — ASCII solid/facet/vertex + binary 80-byte/u32 form, IEEE `f32` bits decoded to `Fixed` by hand), INI (`parse`/`emit`/`get`/`set` — `[section]`, `=`/`:` pairs, `;`/`#` comments, last-duplicate-wins, order preserved), and unified diffs (`parse`/`emit`/`apply` — `---`/`+++` headers, `@@ -a,b +c,d @@` hunks, ` `/`-`/`+`/`\` lines, exact-match apply). |
| `ics` / `vcf` / `pem` / `fen` / `gpx` / `m3u` / `tga` | Calendar, contact, crypto-envelope, board, GPS, playlist, and image formats: iCalendar (`parse`/`emit`/`parse_dt`/`emit_dt`/`get_prop`/`of_kind` — `VCALENDAR`/`VEVENT` components, `MARKER`-preserved nested, `YYYYMMDDTHHMMSS`), vCard 3.0 (`parse`/`emit`/`get`/`get_all` — folded lines, `NAME;params` resolution, `VERSION` required), PEM (`Pem`/`parse`/`encode` — `BEGIN`/`END` armor, base64 body, 64-col wrap, multi-block), FEN (`parse`/`emit`/`count` — 64-cell board, `w|b`, KQkq bits, en-passant, halfmove/fullmove), GPX (`parse`/`emit` — `wpt`/`rte`/`trkseg`/`trkpt`, `lat`/`lon` attrs → `Fixed`, `ele`/`time`/`name`, tag-scan XML), M3U (`parse`/`emit`/`total_duration` — `#EXTM3U`, `#EXTINF:secs,title`, verbatim directives), and TGA (`Tga`/`encode`/`decode` — 18-byte header, type 2/10, BGR(A), RLE packets, origin bits honored). |
| `der` / `rtf` / `exif` / `rss` / `id3` / `tzif` / `pcap` | Binary-tag, document, metadata, feed, timezone, and capture formats: ASN.1 DER (`parse`/`encode`/`Tlv::children`/`oid`/`integer`/`text`/`utc_time` — TLV walk, high-tag form, shortest-length rule, `civil`-bridged times), RTF (`text` — control words, `\'hh` bytes, `\uN`+`\uc` fallback skip, destination-group elision), EXIF (`parse_tiff`/`parse_jpeg`/`get`/`get_str`/`get_int`/`get_rational`/`entries` — II/MM orders, IFD0+EXIF+GPS sub-IFDs, value/offset resolution), RSS 2.0+Atom (`parse`/`emit` — unified `Item`s, entity/CDATA decode, `link href`), ID3 (`parse` — v2.3/4 synchsafe frames + v1 trailer, `title`/`artist`/`album`/`year`/`track`/`genre`/`frames`, Latin-1/UTF-8/UTF-16), TZif (`parse`/`offset_at`/`type_at` — v1 32-bit + v2/v3 64-bit blocks, transition table, POSIX-TZ footer), and pcap (`parse`/`packet` — us/ns magics, both endians, truncated-tail tolerance, `ts_ns` normalization). |
| `sha1` / `git` / `elf` / `http` / `mime` / `ssh` / `cpio` | Hash, VCS, executable, web, mail, key, and archive formats: SHA-1 (`Sha1`/`sha1`/`sha1_hex`/`hmac_sha1` — RFC 3174 streaming + RFC 2104 HMAC, the public home for git object names and TOTP backends), git (`store`/`parse_obj`/`object_name`/`open_loose`/`write_loose`/`parse_tree`/`emit_tree`/`parse_commit`/`pkt_line`/`pkt_flush`/`pkt_lines`/`ref_line` — `"type N\0body"` content, sha1 naming, zlib loose objects, pkt-line framing), ELF (`parse`/`section_name`/`section` — ELF32/64 × LE/BE, program + section headers, `shstrtab` resolution), HTTP/1.1 (`parse_request`/`parse_response`/`emit_request`/`emit_response`/`header`/`set_header`/`chunked` — request/status lines, Content-Length + chunked transfer), MIME (`parse`/`header`/`content_type`/`boundary`/`parts`/`decode_transfer`/`quoted_printable_decode`/`quoted_printable_encode` — folded headers, `multipart/*` splitting, base64/QP transfer encodings), OpenSSH keys (`string`/`fields`/`blob`/`parse_line`/`authorized_keys`/`fingerprint`/`emit_line` — wire strings/mpints, `authorized_keys` options skip, `SHA256:` fingerprints), and SVR4 newc cpio (`parse`/`find`/`emit` — `070701` hex headers, 4-byte alignment, `TRAILER!!!` terminator). |
| `macho` / `coff` / `wasm` / `x509` / `tls` / `ico` / `webp` | Executable, web, cert, TLS, and image-container formats: Mach-O (`parse`/`parse_fat`/`cmd_name`/`seg_name` — thin 32/64 × LE/BE + FAT multi-arch, load-command walk), COFF/PE (`parse`/`machine_name` — COFF header + optional PE32/PE32+ magic + 40-byte section table), WebAssembly (`parse`/`section` — `\0asm`+version 1, LEB128-sized section stream ids 0–12), X.509 (`parse` → `Cert{version,serial,issuer,subject,validity,sig_alg,signature}` — `der` TLV walk, RDN flattened to `/CN=…`, UTCTime/GeneralizedTime → Unix seconds), TLS records (`records`/`emit`/`handshake` — 5-byte header walk, CCS/alert/handshake/app-data types, 16384 cap, handshake `(type,body)` split), ICO/CUR (`parse`/`image` — 6-byte directory + 16-byte entries, PNG-or-DIB payload), and WebP (`parse`/`chunk` — RIFF `WEBP` chunks, `VP8X`/`VP8L`/`VP8 ` dimension decode — the RIFF wrapper over the pixel codecs). |
| `hexgrid` | Axial-coordinate hex math (`Hex`, `DIRECTIONS`, `distance`, `line`, `ring`, `spiral`, odd/even-r offset conversion, `random_in_range`, `hex_astar` shortest paths) — the redblobgames recipe set, integer-exact and `DetHash`-pinned. |
| `terminal` / `camera` | Headless cell buffer with 24-bit ANSI output, diffing, and a world→screen camera. |
| `turn` / `combat` / `inventory` / `status` / `random_table` / `dice` | Energy scheduler, integer combat, items, buff/debuff timers, weighted loot/spawn tables, `NdM±K` dice notation. |
| `damage` / `encounter` / `affix` / `equipment` | Typed damage + resistance profiles, procedural group encounters, item enchantment, worn loadouts with stat aggregation. |
| `equipment` / `progression` / `lightmap` / `faction` | Worn-item loadout per body slot with cursed/locked-item support, XP/level curves, ambient illumination grid, inter-faction reputation. |
| `meta` | Cross-run meta-progression: permanent unlock flags and all-time best records that survive permadeath (`MetaProgress`). |
| `identify` | Scrambled per-seed item appearances (unidentified potions/scrolls) revealed on demand (`Identification`). |
| `threat` / `pool` / `tween` / `wallet` / `shop` / `dialogue` / `trigger` | Per-combatant aggro tables, bounded regenerating resources (mana/stamina), eased time-driven value interpolation (`Tween`) plus single-clock chained playback (`TweenSequence`), fungible currency wallets, wallet-backed buy/sell price listings, branching NPC conversation trees, condition→action rule sets for scripted events. |
| `savefile` | Versioned, checksummed binary save framing. |
| `bits` | LSB-first bit-level wire codec (`BitWriter`/`BitReader`): packed bitfields, ranged integers, canonical varint+zigzag — the lockstep packet primitive. |
| `noise` | Deterministic integer value-noise and hashing for procedural generation — value/fbm/ridge/turbulence plus Worley cellular noise (exact F1/F2). |

## Runnable examples

20+ self-contained demos render to the terminal via the `terminal` module
(24-bit ANSI, zero OS dependencies — they run unchanged in CI):

```text
cargo run --example roguelike_demo          # mapgen + A* + FOV + scheduler + combat
cargo run --example wfc_demo                 # Wave Function Collapse biome generation
cargo run --example noise_terrain_demo       # 3-octave FBM terrain + biome heat-map
cargo run --example influence_demo           # influence-map heat-map + HUD panel + steering
cargo run --example content_pipeline_demo    # parse → validate → load → render, with diagnostics
cargo run --example replay_demo              # desync detection + rollback (exits non-zero on failure)
cargo run --example savefile_demo            # save framing: round-trip, corruption, versioning
cargo run --example status_effects_demo      # StatusSet + Inventory + Scheduler + combat
cargo run --example ai_behavior_demo         # FSM + SpatialHash + Cooldown + TimerQueue
cargo run --example menu_textlayout_demo     # Menu navigation + word-wrap + text layout helpers
cargo run --example camera_viewport_demo     # Camera viewport + TileMap + ChangeTracker + Profiler
cargo run --example geometry_easing_demo     # line / line_of_sight / Aabb / Fixed easing sparklines
cargo run --example input_pipeline_demo      # KeyMap / InputBuffer / CmdQueue deterministic input
cargo run --example autotile_demo            # bitmask auto-tiling: compute_all + SimpleTileTable
cargo run --example relations_demo           # entity parent/child forest + cycle guard
cargo run --example multimap_demo            # multi-floor dungeon stack + stair connectors
cargo run --example archetype_demo           # ArchTable ECS: dense iteration + O(1) migration
cargo run --example asset_store_demo         # AssetStore<T> generational handle safety
cargo run --example hud_panels_demo          # HudPanel + BarWidget + StatLine status screen
cargo run --example timestep_demo            # FixedTimestep accumulator + death-spiral guard
cargo run --example cave_spawn_demo          # cellular cave + depth-scaled RandomTable + Distance
cargo run --example scatter_pipeline_demo    # poisson_disc → voronoi → MST → BitWriter
```

Pipe any of them to a truecolor terminal for full colour; in a plain pipe the
ANSI bytes still flow to stdout and the summary line prints to stderr.

## The content pipeline

```text
text --[parser]--> Content --[validator]--> --[loader]--> ECS world
            ^                                   |
            +-------------[serializer]----------+
```

Author game elements as plain text, validate them, and load them into a live
world. The round-trip `parse(serialize(c)) == c` is property-tested over
thousands of generated bundles.

### `.game` format

```text
prefab <name> [extends <base>]
  glyph <char>
  color <#RRGGBB>
  stat  <key> <int>
  flag  <name>
tile  <name> <glyph> <#RRGGBB>
level <name> <W>x<H>
  row   <cells>
  spawn <prefab> <x> <y>
```

`extends` makes a prefab a field-level patch of its base: `stat` keys and
`flag`s are merged (the child wins a shared stat key), while `glyph`/`color`
override only where the child writes the line itself. Bases can extend bases;
missing bases and cycles are validation errors. `gamec --fmt` keeps the
overlay — it never flattens a child into its resolved form.

## The `gamec` tool

```text
gamec <file.game>          # validate; non-zero exit on error (CI content gate)
gamec --fmt <file.game>    # validate and emit canonical text to stdout
gamec --check <file.game>  # validate formatting only, no output (like `cargo fmt --check`)
gamec --json <file.game>   # emit all diagnostics as machine-readable JSON to stdout
gamec --sarif <file.game>  # emit diagnostics as SARIF 2.1.0, for GitHub Code Scanning
```

Diagnostics are rustc/clang-style, with the offending source line and a caret:

```text
dungeon.game:2:9: error: glyph must be one character
  glyph @@
        ^
```

### The generate → verify → repair loop

When content is machine-authored — procedural templates or an LLM — this
pipeline is the gate that output must pass before it reaches the sim:

```text
generate text → gamec --json (or --sarif) → feed diagnostics back → repeat
```

The diagnostics are designed for the mistakes machine authors actually make:
references to prefabs that were renamed or never defined, spawn coordinates
outside the level, duplicate names, glyphs that cannot render. `--json` /
`--sarif` exist so the verifier is itself machine-readable — a generator can
consume them and retry without a human parsing prose.

What it does not check: that the content is *good*. "Well-formed and
self-consistent" is the contract; balance and intent remain the author's.

## Build and test

```text
cargo test          # all unit + integration tests
cargo run --bin gamec -- examples/dungeon.game
cargo run --example roguelike_demo            # see "Runnable examples" above
```

## Development

The whole verification gate is one command, run from the **repository root**:

```text
tools/gate.sh
```

It runs fmt, the workspace tests, clippy and rustdoc at zero warnings, the
pinned determinism hashes, the `kit_bridge` integration hash, the
self-asserting pipeline demo, and `cargo package` for both crates. Enable the
git hooks (fast checks on commit, the full gate on push) from the repository
root:

```text
git config core.hooksPath .githooks
```

CI is **not yet enabled** on the hosted repository — the agent sessions that
develop this branch cannot push workflow files. The ready-to-install
definition lives at [`../docs/ci/ci.yml`](../docs/ci/ci.yml), with
instructions in [`../docs/ci/README.md`](../docs/ci/README.md); its main job
runs the same `tools/gate.sh`, so local green and CI green mean the same
thing.

## Determinism notes

- Gameplay math uses `fixed::Fixed`, not floats, so results are reproducible
  across CPUs and compilers.
- Randomness flows through one seeded `SplitMix64` stream advanced in a fixed
  order; never seed from the wall clock in code that must replay.
- `timestep` uses integer nanoseconds, introducing no nondeterminism.
- Fold `world_hash` over canonical (sorted) state each frame and assert the
  sequence in CI to catch divergence.

## Project documents

- [`SPEC.md`](./SPEC.md) — module contracts and invariants.
- [`GAME_DEV_TAXONOMY.md`](./GAME_DEV_TAXONOMY.md) — capability map with
  per-feature implementation status, organized by game-dev discipline.
- [`RESEARCH.md`](./RESEARCH.md) — category-by-category survey of external
  prior art (arXiv papers, comparable OSS), with what was implemented (and
  the commit) or deliberately deferred (and why).
- [`CHANGELOG.md`](./CHANGELOG.md) — release history.

(Superseded audit documents were deleted rather than left to mislead; they
remain in git history, and the facts they tracked are now build-checked —
see `tests/docs_are_current.rs`.)

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option.
