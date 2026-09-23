# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **`avltree`** — height-balanced AVL tree over `u64` keys (`insert`/`remove`/`contains`/`values`/`min`/`max`/`height`): LL/RR/LR/RL rotations restore `|h(l) - h(r)| <= 1` after every mutation; a `BTreeSet` shadow oracle audits the invariant after every op.
- **`bandit`** — deterministic multi-armed bandits (`reward`/`mean`/`greedy_pick`/`ucb1_pick`/`eps_greedy_pick`/`best_arm`, `isqrt`, `SCALE`): UCB1 evaluated in Q8 fixed point (`mean·256 + isqrt(2·log2(total)·SCALE²·256/pulls)`), plus a seeded ε-greedy — every pick is a pure function of pull history.
- **`zobrist`** — incremental Zobrist hashing (`toggle`/`move_piece`/`hash`/`rehash`/`hash_with_side`, `side_key`): `(piece, square)` keys drawn from a seeded `SplitMix64`; XOR toggling gives exact undo, verified against `rehash` every step.
- **`rsa`** — textbook RSA over `BigInt` (`KeyPair::generate`/`encrypt`/`decrypt`/`sign`/`verify`, `n`/`e`): seeded probable-prime keygen; the module supplies its own magnitude arithmetic (binary long-division `rem`/`div`, `modpow`, extended-Euclid `modinv`, fixed-witness Miller–Rabin) because `BigInt` has no division. No padding — not wire-grade crypto.
- **`json`** — strict integer-subset JSON parser (`parse`/`render`, `Json`, `Error`): RFC 8259 minus floats; `\uXXXX` escapes decode through surrogate pairing (lone surrogates rejected); canonical `render` (sorted keys, minimal escapes) satisfies `parse(render(x)) == x`.
- **`sufftree`** — Ukkonen online suffix tree over bytes (`new`/`contains`/`occurrences`/`count`/`longest_repeat`/`n_nodes`): explicit terminator `SENT` makes every suffix own a leaf; active point + suffix links + skip/count walks; naive-oracle `count`/`occurrences`/LRS agreement.
- **`redblack`** — red-black BST over `u64` (`insert`/`remove`/`contains`/`iter`/`min`/`max`/`height`/`check`/`len`/`is_empty`): arena nodes, CLRS fixups, `check()` audits BST order + no red-red + equal black height; `BTreeSet` shadow oracle.
- **`apsp`** — all-pairs shortest paths (`floyd_warshall`/`has_negative_cycle`/`johnson`): i64 edges with i128 internals; Johnson re-weights via Bellman–Ford potentials so `w' >= 0` — three-way oracle agreement.
- **`pagerank`** — integer PageRank (`pagerank`, `PageRank{scores,iterations,converged}`): Q32 fixed point with teleport + dangling-share distribution; all-floor semantics, `converged` flag reports termination.
- **`automaton`** — NFA→DFA toolkit (`Nfa::{literal,union,concat,star,plus,optional,determinize,accepts}`, `Dfa::{accepts,alphabet,complete,complement,intersect,minimize}`): subset construction, `complement(alphabet)` totalizes first, `minimize` delegates to `dfamin`.
- **`aastree`** — AA tree over u64 (`contains`/`insert`/`remove`/`iter`/`values`/`min`/`max`/`height`/`check`): Andersson's two-invariant red-black (`level == left+1`, NIL = 0); arena nodes + freelist, skew/split only; `check()` runs the true in-order audit with the level identity — BTreeSet shadow oracle.
- **`lz4`** — LZ4 block codec (`compress`/`decompress`): greedy 4-byte hash-table parse, u16 window, nibble + 255-extension lengths; strict decoder rejects zero/out-of-range offsets and match-bearing tails, overlapping matches copy byte-wise; round-trip oracle + truncation sweep.
- **`christofides`** — metric TSP 1.5-approximation (`christofides`, `tour_cost`): Prim MST (canonical `(cost, vertex)` picks) -> odd-degree subset -> min-weight perfect matching (subset DP `|T| <= 20`, sorted greedy beyond) -> `euler::euler_walk` on the multigraph -> shortcut; n<=8 brute-force `cost <= 3*opt/2` oracle.
- **`sha3`** — Keccak-f[1600] sponge (`sha3_256`/`sha3_512`/`shake128`/`shake256`, streaming `Digest256`): theta/rho/pi/chi/iota x24 on 25 u64 lanes, `0x06`/`0x1F` domain suffixes + pad10*1; NIST vectors, chunking-independence, and prefix-stable XOF squeezes that keep the rate position across `finalize` calls.
- **`minkowski`** — convex Minkowski sum/diff (`sum`/`diff`/`collide`/`free_at`): CCW canonicalization from the lowest `(y, x)` vertex, angular edge-vector merge `O(n+m)`, `diff(a,b) = sum(a, -b)` turning collision into a point-in-polygon test; brute pair-sums -> `poly::convex_hull` oracle.
- **`shamir`** — (k,n) threshold secret sharing over GF(p) (`split`/`reconstruct`): seeded coefficients, shares at `x = 1..=n`, Lagrange-at-0 reconstruction; deterministic Miller-Rabin vets the prime, k−1 subsets provably recover nothing.
- **`postman`** — Chinese postman route inspection (`closed_postman`/`open_postman`): odd-degree vertices -> `bellman::shortest` metric closure -> min-weight matching via a `free`-parameterized subset DP (`free=2` selects trail endpoints in one pass) -> `euler::euler_walk`; full-enumeration optimality oracle.
- **`fst`** — minimal acyclic DFA dictionary (`compile`/`contains`/`enumerate`/`longest_prefix`/`next_state`/`is_final`/`states`): trie + bottom-up hash-consing register yields the unique minimal automaton; pure function of the word set.
- **`fountain`** — LT fountain codec over GF(2) (`degree`/`neighbors`/`droplet`/`decode`): robust-soliton degree `R = isqrt(k)*iln(2k)/4`, neighborhoods are pure `(k,i,seed)` partial Fisher-Yates samples, belief-propagation peeling decode.
- **`edt`** — Felzenszwalb-Huttenlocher squared Euclidean distance transform (`edt1`/`edt2`/`BIG`): two 1-D lower-envelope passes, rational breakpoints in i128, `BIG = i64::MAX/4` no-feature sentinel; brute-force nearest-site oracle.
- **`bplus`** — B+ tree ordered map `u64 -> u64` (`insert`/`get`/`contains`/`remove`/`range`/`keys`/`len`/`check`): order 4..64, copy-up leaf splits vs move-up internal splits (separator = smallest key of the right subtree), borrow-then-merge underflow; `check()` audits separators, fill, parents, and the leaf chain.
- **`bentley`** — Bentley-Ottmann sweep (`intersections`/`any_intersection`, `Pt`): BTreeMap event queue keyed by exact `(x, y)`, status resorted by `(y at x, slope)` so concurrent-at-p segments sit adjacently; verticals never enter status — they pair-test against every involved segment for their whole x-line; collinear overlaps report shared endpoints only.
- **`comb`** — combinatorial `(n choose k)` machinery (`choose`/`choose128`/`rank`/`unrank`/`next_comb`/`all`): exact `choose128` via the `acc = C(n-k+i, i)` invariant (each division exact — no gcd juggling), lexicographic combinadics over `[0, C(n,k))`.
- **`gcm`** — AES-128-GCM AEAD (`seal`/`open`): bit-serial GHASH with `R = 0xE1<<120`, accumulator *chained* across aad and ciphertext (separate-hash XOR composition is a spec violation), J0 = IV‖0^31‖1 for 12-byte IVs else GHASH-derived; NIST vectors cross-verified against an independent implementation.
- **`polyclip`** — Sutherland-Hodgman polygon clipping (`clip`/`clip_i64`/`area2`): clipper winding normalized via the i128 shoelace sign, intersection parameter `t = cross(cd, a-s)/cross(cd, sd)`, consecutive/wraparound dedup ring output in exact `Frac`.
- **`mmheap`** — min-max heap (`push`/`pop_min`/`pop_max`/`peek_min`/`peek_max`/`from_slice`/`len`/`check`): alternating min/max levels make both ends O(1); `pop_max` must skip the move when the max sits in the tail slot (the popped element IS the max).
- **`mstree`** — merge-sort tree range counting (`count_range`/`count_le`/`count_ge`/`sorted`): recursive mid-split node layout (4n arena — a size-2n heap layout only works for power-of-2 n), `O(log^2 n)` per query.
- **`clique`** — Bron–Kerbosch maximal cliques over `u64` adjacency masks (`maximal_cliques`/`max_clique`): Tomita pivot on `P \ N(u)` maximizes `|P ∩ N(u)|`; sorted mask output is canonical.
- **`dtw`** — dynamic time warping (`distance`/`distance_windowed`/`path`): exact i64 sum-of-|diff| DP, Sakoe–Chiba band, border cells stay INF (exhausted prefix vs nonempty prefix is unalignable — free leftovers silently undercut).
- **`pathcover`** — DAG minimum path cover (`path_cover`/`min_path_cover_size`/`topo_order`): bipartite `L_u—R_v` matching reduction, cover = n − |matching|; None on cycles; canonical smallest-first Kahn order.
- **`jps`** — Jump Point Search (`find`): 8-way uniform-cost over jump points with natural + forced neighbor pruning; corner-touching is part of the paper's optimality lemmas — strict no-corner-cut silently changes reachability.
- **`goap`** — goal-oriented action planner (`plan`/`named`, `Action`): bitmask-world Dijkstra keyed `(cost, seq, state)` yields canonical lex-min plans as a pure function.
- **`btree`** — behavior trees (`BehaviorTree::tick`, `Node`/`Status`): sequences/selectors resume at the Running child via per-node `mem`; Conditions map Running→Failure.
- **`verlet`** — integer Verlet integration (`World::step`, `P2`): Jakobsen relaxation in Q16.16; undamped runs conserve energy — the link oscillates through equilibrium, damping is required for convergence.
- **`vnoise`** — integer value noise + fBm (`Noise2::value`/`fbm`/`lattice`): seeded SplitMix64 lattice hashes, smootherstep bilinear interpolation, arithmetic-shift cell flooring is negative-coordinate safe.
- **`bdd`** — reduced ordered binary decision diagrams (`Bdd`, `Op`): `(var,lo,hi)` hash-consing over a BTreeMap unique table makes equal functions share node ids; memoized Shannon `apply`, `restrict`, `exists`, exact `u128` `count_sat`.
- **`simplex`** — exact-rational LP solver (`maximize` → `Solution`): two-phase primal simplex on a `Frac` tableau under Bland's smallest-index entering/leaving rule — degenerate cycling is a theorem-level impossibility (Beale's example terminates at 1/20).
- **`peg`** — packrat PEG parser (`parse`/`parse_full`, `Expr`, `Grammar`): `(rule,pos)`-memoized ordered choice; left-recursion guard fails the offending alternative, `Star`/`Plus` break on no-progress — verified against a naive re-evaluation oracle on random grammars.
- **`radixheap`** — monotone radix heap (`RadixHeap`): `msb(key XOR last)` bucketing — under `bit_len(key − last)` a stale high bucket can hide a key smaller than the scan frontier; pushes below the last pop are rejected.
- **`life`** — sparse deterministic Game of Life (`World`): the `BTreeSet` live set is its own canonical state; B3/S23 via a single sorted neighbor-count pass, dense-grid oracle verified.
- **`scapegoat`** — α weight-balanced scapegoat tree (`insert`/`remove`/`contains`/`values`/`min`/`max`/`height`/`n_rebuilds`): insert rebuilds the lowest ancestor violating `4·size(child) > 3·size(node)` by median split; deletes let imbalance drift until `len < α·max_size` — the per-node α invariant is only guaranteed after inserts.
- **`leftist`** — leftist heap (`push`/`pop`/`peek`/`meld`/`from_slice`/`sorted`/`right_spine_len`): merge-only core on the `O(log n)` right spine, rank = null-path length; `from_slice` pairwise-melds in `O(n)`.
- **`beam`** — deterministic beam search (`beam_search` → `BeamResult{best, score, path, levels, expanded}`, `beam_moves` over `minimax::Game`): children ranked by `(score, generation order)` so the search is a pure function of `expand`'s output order; parent-link path reconstruction.
- **`perceptron`** — integer perceptron (`train`/`train_step`/`predict`/`margin`/`errors`, `OvrPerceptron` one-vs-rest): `i128` score accumulation + saturating `w += y·x` updates keep adversarial magnitudes deterministic; the update-trace oracle replays the textbook rule bit-for-bit.
- **`saddleback`** — saddleback search in a row+column sorted matrix (`search`/`contains`/`is_sorted_matrix`): top-right walk discards a row or column per step, `O(r + c)`; ragged-row cells beyond `m[0].len()` are unreachable and ignored.
- **`sieve`** — linear SPF sieve + Eratosthenes + segmented primes (`spf_sieve`/`factor`/`is_prime_table`/`factor_map`/`primes_up_to`/`Primes{list,bound,is_prime,factor,phi,tau,sigma}`/`primes_between`): segmented marking clamps `ceil(lo/p)·p` up to `p^2`; `sigma(0)` needs the same `v == 0` early return as `phi`/`tau` or the empty factor map yields 1.
- **`grundy`** — Sprague–Grundy impartial-game numbers (`mex`/`take_away`/`position_grundy`/`nim_sum`/`winning_move`/`losing`/`detect_period`): a reported period `(start, p)` is a theorem only when the last `memory` cells verified periodic — the `.max(1)` floor stops a vacuous `(n−1, 1)` report when `memory = 0`.
- **`gapbuffer`** — Emacs-style gap buffer over `Vec<u8>` (`insert`/`insert_byte`/`delete_before`/`delete_after`/`move_to`/`before`/`after`/`get`/`to_vec`): `move_to` uses `copy_within` in both directions; deletes return the actual byte count so callers learn about clamps.
- **`robin`** — Robin Hood open-addressing set over u64 (`insert`/`contains`/`remove`/`len`/`values`/`max_probe_len`/`total_probe_len`/`capacity`): probe-length stealing keeps lookup walks short, backward-shift deletion needs no tombstones, growth at 0.75 rehashes in slot order so layout stays a pure function of `(inserts, seed)`.
- **`winnow`** — winnowing document fingerprints (Schleimer 2003) (`kgram_hashes`/`winnow`/`fingerprint`/`similarity`): every window contributes its rightmost minimum — the initial window's argmin needs a `!0` sentinel or the walk assumes position 0 is the minimum; a shared run of ≥ k+w−1 bytes must surface a shared hash.
- **`bspline`** — integer-exact de Boor B-spline evaluation over `Frac` control points (`eval`): degenerate spans blend with `alpha = 0` (never skip the blend — `continue` leaves `d[j]` stale), and the right endpoint `t == u[n+1]` maps to the last non-empty span ≤ n, the left-continuous semantics a naive `s = n` clamp breaks at degree 0.
- **`bmassey`** — Berlekamp–Massey shortest LFSR over GF(p) (`massey` → `C[0]=1` connection polynomial, `holds` verifier): the exhaustive `min_complexity` oracle over `GF(p)^L` itself had a bug (`continue 'outer` instead of `return l`) the known vectors caught.
- **`xortrie`** — bitwise trie over u64 (`insert`/`remove`/`contains` multiplicities, `max_xor`/`min_xor`, `max_xor_pair` in `O(n·64)` with canonical `(lo,hi)` dedup): `len` counts live multiplicities, so the set-equality oracle needs distinct-key ops.
- **`orset`** — add-wins observed-remove set CRDT (`add`/`remove`/`contains`/`values`/`len`/`is_empty`/`merge`): removes cover only the dots the replica has seen, so concurrent adds always survive and re-adds live again; `remove` reports whether a *live* dot existed (covered dots never leave the adds map).
- **`lww`** — LWW-element-set CRDT (`add`/`remove`/`contains`/`values`/`added_since`/`merge`): the `(clock, replica)` stamp winner decides each element's fate with equal clocks resolving toward remove — orset's dual, where a concurrent remove never observed can still win by clock.
- **`utf8`** — Höhrmann strict-DFA UTF-8 codec (`validate`/`check`/`decode`/`decode_lossy`/`encode`, streaming `Decoder`/`Utf8Error{pos,kind}`): 364-entry class table rejects overlongs, surrogates, and `>0x10FFFF` at the byte level; lossy resync consumes a lead-rejected byte but re-feeds a mid-sequence-rejected one, which is the only scheme that terminates on stray continuations.
- **`lazyseg`** — canonical lazy segment tree (`add`/`sum`/`min`/`max`/`get`/`set`): a single pending-add tag composes trivially; the push must scale the folded sum by each child's real leaf count `len/2` (ceil costs the excess the oracle caught).
- **`tonelli`** — Tonelli–Shanks modular square roots (`sqrt_mod` → sorted `(lo, p−lo)` pair, `is_quadratic_residue`): `p−1 = q·2ˢ` decomposition plus a sequential least-non-residue scan keeps it a pure function of `(a, p)`; `p ≡ 3 (mod 4)` short-circuits to the direct formula.
- **`bsgs`** — baby-step giant-step discrete log (`discrete_log`): `O(√p)` baby table + giant walk returns the *least* `x` because every earlier bucket is ruled out; `g^{−m} = g^{p−1−m}` avoids a modular-inverse helper, and the `g = 0` base is resolved before the walk since `f` is a true inverse only for `g ≠ 0`.
- **`dfamin`** — Hopcroft DFA minimization (`minimize` → `MinDfa`, `reachable`): splitter-worklist partition refinement re-queueing only the smaller half, block ids canonicalized to the least member; a compact minimized machine is emitted that simulates the original transition-for-transition.
- **`varint`** — canonical LEB128 unsigned + zigzag signed codec (`encode_u64`/`encode_i64`/`decode_u64`/`decode_i64`/`decode_all`): every value maps to exactly one byte string (trailing zero-group and 10-byte overflow/padding rejected), byte-identical round-trip oracle over all bit boundaries.
- **`hornsat`** — Dowling–Gallier linear Horn SAT (`Clause{pos,neg}`, `imp`/`fact`/`goal` helpers, `solve` least-model closure): per-variable watch lists + `remaining` counters in a FIFO unit-propagation queue, `O(Σ|neg|)`; brute-force 2^n oracle agreement over random instances.
- **`bigint`** — sign-magnitude arbitrary-precision integer over u64 limbs (`zero`/`from_i64`/`from_i128`/`from_limbs`/`neg`/`abs`/`is_negative`/`add`/`sub`/`mul`/`pow`/`cmp`/`eq`/`limbs`/`to_i128`/`to_u64`): schoolbook magnitude ops with u128 products, `-(i128::MIN)` handled via the `m==1<<127` special case; 2,000-op i128 `checked_*` oracle.
- **`segbeats`** — segment tree beats: range `chmin`/`chmax`/`sum`/`get` via max/second-max/count-max and the mirror min ledgers; the push is just "clamp children to the parent's `mx`/`mn`" (no explicit lazy tags) and sum queries must push too, since children under a lazily-updated node hold stale sums.
- **`ett`** — Euler-tour tree dynamic-forest connectivity (`link`/`cut`/`connected`/`tree_size`): each tree's cyclic edge tour lives in one implicit treap (seeded priorities, parent pointers for root climbs and in-order indices), each vertex owns a permanent vertex node, and `link` = reroot both + `U+[uv]+V+[vu]` while `cut` isolates the span between the two half-edge occurrences; BFS adjacency oracle replay.
- **`fibheap`** — Fredman–Tarjan Fibonacci heap over an arena (`FibHeap{push,meld,peek,pop,decrease_key,len,is_empty}`): circular sibling rings + lazy cut/cascade decrease-key, degree consolidation on pop, `(key, seq)` FIFO tie order makes the trace a pure function; a `live` bit rejects stale handles.
- **`halton`** — Halton low-discrepancy sequence (`radical_inverse`, `Halton`/`point`/`reset` + `Iterator`): radical inverses returned as exact `Frac` — the `b^k` stratification (first `b^k` points hit every `j/b^k` cell exactly once) holds as a machine-checked property, not a float approximation.
- **`dihedral`** — square-lattice dihedral group `D4` (`D4::{apply,compose,inverse}` + `ALL`): `(swap, sx, sy)` closed form composes without matrix materialization; the full 8×8 Cayley table is oracle-checked against unique pointwise behavior on probe points.
- **`turnpike`** — turnpike reconstruction (`distances`, `reconstruct`, `canonical_turnpike`): Skiena backtracking over a `BTreeMap` multiset, DFS-first is deterministic; the `need` presence check is multiplicity-aware (duplicate implied distances must each be backed) — the single-presence version silently corrupts the residual multiset.
- **`kpaths`** — Yen's k-shortest loopless paths (`yen`, `path_cost`): spur deviations ban shared-prefix next-edges and prefix nodes; candidates sit in a canonical `(cost, path)` heap — the contract is the k smallest *costs* (equal-cost order is implementation-defined), which the brute-force oracle verifies.
- **`fps`** — formal power series over `GF(998244353)` (`add`/`sub`/`mul`/`scale`/`trunc`/`derivative`/`integral`/`inv`/`log`/`exp`/`pow`/`divmod`/`eval`): Newton iterations drive a *tracked degree* (`m = min(2m, n)`) — keying the loop on `g.len()` stalls forever when a product's tail coefficients vanish under `norm`; `exp∘log` round-trips verified against factorial-inverse coefficient vectors.
- **`rectunion`** — union area of axis-aligned rectangles (`union_area` → `i128`): x-sweep edge events with an `O(n²)` per-slab y-interval union — honest simplicity over a coordinate-compressed segment tree; inverted endpoints and degenerate slivers normalize.
- **`intervalgraph`** — interval scheduling (`max_independent_set`, `min_rooms`, `weighted_select` → `(weight, indices)`): earliest-finish greedy under canonical `(end, start, index)` tie-break, sweep-depth room count (interval graphs are perfect: depth = chromatic number), and `O(n log n)` weighted DP — degenerate `[s,s)` items are unselectable in every API; brute-force subset oracles over n≤8.
- **`stirling`** — Stirling numbers mod `998244353` (`s1`/`us1`/`s2`/`s1_row`/`s2_row`/`bell`/`falling_coeffs`): signed first kind verified by enumerating every permutation's cycle count via `perm`, second kind cross-checked against the closed `1/k!·Σ(−1)^j C(k,j)(k−j)^n` form, `falling_coeffs` validated by the `xⁿ̲ = Σs(n,k)xᵏ` Horner identity.
- **`onion`** — convex onion layers (`onion_layers`, `onion_depth`): iterated `convex_hull` peels; a point on a hull edge is *not* a vertex under the hull's strict-corner contract, so it survives its own peel and can surface as a degenerate deeper layer — honest semantics, documented in the module docs.
- **`karp`** — Karp minimum mean cycle (`min_mean_cycle` → `Option<(Frac, Vec<usize>)>`): `O(V·E)` length-k walk DP; the cycle is extracted by backtracking the *whole* n-edge walk to the minimizing vertex and cutting at the first repeat — trimming only the suffix finds a walk, not a cycle. Brute-force simple-cycle oracle on n≤6 digraphs with negative weights and self-loops.
- **`hirschberg`** — linear-space Needleman–Wunsch (`align` → `{score, ops}`): mid-split `L[j] + R[n−j]` recursion in `O(min)` live memory, leftmost-split tie-break for a canonical script; the oracle applies the ops to `a` and requires `b` byte-exact plus the optimal `O(nm)` score.
- **`matchain`** — matrix-chain order (`order(&[dims])` → `Option<(u64, Vec<Step>)>`): `O(n³)` interval DP with first-argmin splits; steps emit postorder so dependencies precede their multiply. Full Catalan enumeration oracle.
- **`seamcarve`** — integer seam carving (`energy`/`find_vseam`/`remove_vseam`): squared one-sided-gradient energy `u32`, 8-connected seam DP with leftmost ties, malformed seams rejected via `None` rather than clamped.
- **`ost`** — order-statistic treap (`Ost`): priority `splitmix64(seed ⊕ mix(key))` makes the shape a pure function of the key set; `select`/`rank`/`lower_bound` over subtree sizes, recursive size/heap invariant check + `BTreeSet` shadow oracle.

- **`kkpart`** — Karmarkar–Karp largest-differencing partition (`partition` → `Option<(u64, Vec<bool>)>`): each residual heap element carries `(plus, minus)` index bitmasks, so the returned `d` is a *concrete achievable* difference — `d ≥ optimal` holds by construction, verified against brute `2ⁿ` enumeration on n≤9.
- **`xfast`** — x-fast predecessor/successor trie over `u64`: 65 level tables of prefix→`(min,max)` leaf bounds; `pred`/`succ` binary-search the levels for the divergence node (`O(log U)` table ops) then resolve from the diverging child's leaf bounds. `BTreeSet` shadow oracle, 60×400 ops.
- **`seglazy`** — segment tree beats + lazy range add (`SegLazy::add`/`chmin`/`chmax`/`sum`/`get`): `push` replays a pending `lz` add into children via `apply_add` *before* clamping them to the parent's extrema — the order is load-bearing or a stale-low child is clamped against the pre-add ceiling. Naive per-element oracle, 120×300 ops + add/clamp interleaving test.
- **`tunstall`** — Tunstall variable-to-fixed code (`Tunstall::build`/`encode`/`decode`/`phrase`): greedy highest-probability-leaf expansion to `2ᵏ` codewords, probabilities compared exactly via `BigInt` cross-multiplication; DFS-order canonical codes; prefix-free + Σprob=1 verified exactly over `BigInt`.
- **`ortho`** — exact Gram–Schmidt QR over `Frac` (`qr` → `(Mat, Mat)`): unnormalized orthogonal `Q` (`QᵀQ = diag`), unit-diagonal `R`, `A = Q·R` exactly; dependent columns return the zero vector and `rank` counts nonzero columns — brute Gaussian-elimination rank oracle on 150 random matrices.

- **`ratbezier`** — rational Bézier evaluation over `Frac` (`eval`, `eval_deriv`): homogeneous `(w·x, w·y, w)` lifts interpolated by de Casteljau and divided back exactly — every curve point is an exact rational pair; the derivative comes from the degree-1 difference curve through the quotient rule.
- **`polya`** — Pólya/Burnside orbit counting (`burnside`, `necklaces`, `bracelets`): `#orbits = (1/|G|)·Σk^{cycles(g)}` summed over `BigInt` with exact limb-wise division by `|G|`; `necklaces(4,3) = 24`, `bracelets(6,2) = 13`.
- **`minq`** — `MinQueue`/`MinStack` minimum structures: every stack slot stores its running minimum, so `pop`/`min` are `O(1)` amortized; the `in→out` pour rebuilds out's minima in a single pass.
- **`ssw`** — Smith–Waterman local alignment (`local`, `score`): 0-floor restart cells, earliest-`i`-then-`j` canonical winner, traceback to the restart for exact witness ranges; verified against the max over all substring pairs of the global NW score.
- **`ternary`** — discrete ternary argmin (`argmin_seq`, `argmin_domain`): *strict* unimodality is the contract — on a flat staircase an equality probe cannot confine the argmin to either side, so the tail window is evaluated exhaustively and the result is global-min-verified (`None` rather than a silent wrong index).

- **`ec`** — elliptic-curve group law over GF(p) (`Curve::add`/`double`/`neg`/`mul`, `on_curve`): affine chord-tangent formulas with `i128` intermediates; `p < 5` (needs the extended law) and non-field use are rejected honestly — verified via the cyclic group table of `y² = x³ + 2x + 2/F17` and scalar-law oracles.
- **`adic`** — truncated p-adic integers (`Adic::new`/`add`/`sub`/`mul`/`neg`/`inv`/`div`/`val`/`lift`/`trunc`): exact residues mod `pᵏ`; units are `gcd(v,p)=1` (composite `p` handled correctly — `2 mod 4ᵏ` is not a unit), `inv` via i64 `extgcd` with the modulus bounded by `new`.
- **`gray`** — binary-reflected Gray codes (`to_gray`/`from_gray`/`sequence`, `SubsetWalk` iterator): unit-step subset traversal; `last_flip` reports which bit changed.
- **`partitions`** — integer partitions (`count`/`count_bounded`/`count_distinct`/`enumerate`): pentagonal `p(n)` over `BigInt`, the `p(n,m)` and distinct-parts DP triangles shadowing each other (`Σ_m p(n,m) = p(n)`, distinct = odd parts by Euler's theorem, both directions checked), descending-lex enumeration.
- **`chrompoly`** — exact chromatic counting (`count`/`count_poly`/`chromatic`): deletion–contraction `P(G) = P(G−e) − P(G/e)` over `BigInt`, canonical smallest-edge recursion; verified against brute `kⁿ` coloring enumeration (400 random graphs) and the C₄ closed form `k(k−1)(k²−3k+3)`.
- **`pell`** — Pell equations `x² − d·y² = ±1` (`isqrt`/`surd_cf`/`solve`/`negative`/`power`): exact `(m,d,a)` continued-fraction recurrence for `√d`, `BigInt` convergents; OEIS fundamentals and the d=61 pair (1766319049, 226153980) verified, `Z[√d]` powers stay solutions.
- **`farey`** — Farey sequence `F_n` + lattice sums (`farey`/`neighbor`/`stern_brocot`/`floor_sum`): one-arithmetic-step next-term recurrence, `|F_n| = 1 + Σφ(k)` oracle, neighbor determinants = 1, ACL `floor_sum` in `u128` checked by direct summation.
- **`mobius`** — Möbius function + Dirichlet convolution (`mu_sieve`/`mu`/`convolve`/`invert`/`coprime_count`/`sigma_sum`): linear-sieve `μ`, `f ∗ μ` recovers divisor-summed `g` exactly (200 roundtrips), inclusion–exclusion coprime counting vs direct gcd enumeration.
- **`jacobi`** — Kronecker symbol `(a|n)` for all integers: binary quadratic-reciprocity algorithm, `n ∈ {−1,0,1}` extension conventions; verified against the Euler criterion `a^((p−1)/2) mod p` (4000 cases) and multiplicativity `(a|mn) = (a|m)(a|n)` (3000 cases).
- **`egypt`** — Egyptian fractions (`decompose`/`expand`/`verify`): Fibonacci–Sylvester greedy `d = ⌈den/num⌉` over `Frac` — distinct unit fractions only, `i128` overflow honestly `None`, numerator-descent invariant verified as the termination proof.
- **`lucas`** — Lucas sequences (`lucas`/`lucas_mod`): exact `i128` `(U_k, V_k)` + fast doubling mod odd `m` returning `(U,V,Qᵏ)`; residue-parity halving, `V²−D·U²=4Qⁿ` identity oracle, doubling vs scan on 3000 cases.
- **`frobenius`** — coin problem (`frobenius`/`representable`): min-residue Dijkstra `dist[r]` = smallest representable `≡ r (mod min)`, `g = max dist − m`; Sylvester closed form for k=2, DP representability oracle, McNuggets 43.
- **`josephus`** — Josephus problem (`survivor`/`survivor2`/`order`): O(n) recurrence, `2l` closed form for k=2, full elimination order in `O(n log n)` via the order-statistic treap — brute `Vec` oracle over 400 cases.
- **`bernoulli`** — exact Bernoulli numbers + Faulhaber (`bernoulli`/`faulhaber`): Akiyama–Tanigawa over `Frac` (B₁ = +1/2 convention); direct power-sum oracle, `B_{2k+1} = 0`, and the generating recurrence `Σ C(n+1,j)·Bⱼ = n+1` all verified.
- **`eulerian`** — Eulerian numbers (`eulerian`/`permutations`): BigInt recurrence `(n−k)·⟨n−1,k−1⟩ + (k+1)·⟨n−1,k⟩`; row-sum `n!`, the Worpitzky identity `xⁿ = Σ ⟨n k⟩·C(x+k,n)`, and `permutations` enumeration length = `⟨n k⟩` all verified.

### Fixed
- **`SpatialHash` iteration order was nondeterministic** (`spatial_hash.rs`) —
  cells were stored in a `HashMap`, and `iter_keys` / `all_occupied_cells`
  returned them in bucket order, which varies per process because the standard
  hasher is randomly seeded. Both methods documented themselves as unordered,
  and `iter_keys`' own docs suggested "process every entity in the spatial
  index (e.g. end-of-frame position sync)" as a use case — an order-dependent
  pass over a per-process-random order, which is precisely the desync the
  engine's `World` shipped and fixed by moving to `BTreeMap`. Documenting a
  footgun is not the same as removing it, least of all in the one crate whose
  entire promise is bit-identical replay.

  `cells` is now a `BTreeMap`, so both methods yield ascending `(cx, cy)` order
  and `iter_keys` stays allocation-free. Cost is O(log n) rather than O(1) per
  cell lookup, the same trade the engine made. No observable output changed:
  `DetHash` already sorted before hashing, so the pinned determinism hashes and
  the `kit_bridge` integration hash are unmoved. Two tests pin the new
  guarantee, using insertion order as the oracle — build the same grid forwards
  and backwards and require identical readback — and reverting the struct to
  `HashMap` fails them.

### Added — cuckoo hashing, bitboards, CYK, half-plane intersection, y-fast trie

Ordered-set structures, board algebra, formal-language recognition, and
exact convex-region geometry, all in published-work form with independent
oracles:

- **`cuckoo`** — two-table cuckoo hashing (Pagh & Rodler 2001): at most two
  probes per lookup, tombstone-free removal, and a journaled displacement
  chain that rolls back completely when the kick budget is exhausted — a
  failed insert can never orphan a stored key. Fixed capacity means peers
  never diverge on a rehash policy.
- **`bitboard`** — 8x8 `u64` bitboards: file-masked directional shifts and
  dumb7fill occluded fills for rook/bishop/queen ray attacks, leaper
  attacks for knight/king/pawn — verified against per-square ray-walk and
  single-step oracles over all 64 squares plus 4000 random occupations.
- **`cyk`** — CYK recognizer for Chomsky-normal-form grammars: `bin[a][b]`
  pre-tabulated as an lhs bitset, O(n³) parse triangle, `accepts`/`derive`/
  `cell` granularity — oracle-checked against memoized recursive expansion
  on 150 random grammars.
- **`halfplane`** — exact `Frac` half-plane intersection: atan2-free
  direction sort (quadrant + cross product), sign-free tighter-merge,
  closing-vertex completion, collinear/duplicate vertex canonicalization
  and CCW normalization — `None` for empty, unbounded, or degenerate
  regions, oracle-verified against pairwise-intersection enumeration.
- **`yfast`** — y-fast–style clustered predecessor set: rep-ordered
  buckets covering `(prev_rep, rep]` that split at their median, rep
  promotion on removal, strict `predecessor` / inclusive `successor`
  matching `veb` semantics — BTreeSet oracle over all operations.

### Added — SA-IS, HyperLogLog, range coder, Swiss table, magic bitboards

Linear-time string indexing, log-log-space estimation, interval
coding, dense-table hashing, and perfect-hashed move generation,
each in published-work form with an independent oracle:

- **`sais`** — SA-IS induced-sorting suffix array: S/L typing,
  LMS bucket placement, induced L/S passes, one recursion on the
  reduced string — `O(n)` where `suffix` uses `O(n log n)`
  prefix doubling.
- **`hll`** — HyperLogLog `p`-bit register sketch: elementwise-max
  merge = union stream sketching, `i128` fixed-point raw estimate,
  and integer `ln` (Q32 atanh series) for the small-range
  linear-counting correction — no `f64` anywhere.
- **`arith`** — Subbotin carry-less range coder: streaming
  `encode`/`decode` in input order with a caller-supplied
  frequency model; carry propagation is avoided by truncating
  `range`, so the wire is a pure function of the input.
- **`swiss`** — SwissTable-style `u64` set: 16-slot probe groups,
  7-bit `h2` control fingerprints, tombstone deletion, and a
  seeded deterministic layout — `cuckoo`'s design opposite.
- **`magic`** — magic bitboards: per-square `(occ·m)>>shift`
  perfect hashes for rook/bishop/queen attack sets, with the
  table scatter written to hashed slots and an honest
  dumb7fill-oracle fallback.

### Added — implicit treap, meet-in-the-middle, GF(p) linear systems, recursive vEB, multi-word Myers

Sequence editing, halved enumerations, modular linear algebra,
deep predecessor sets, and wide-pattern edit distance:

- **`imptreap`** — implicit-key treap over a slab arena: `split`/
  `merge`/`insert`/`remove`/`reverse`/`get`, seeded-priority
  min-heap merges, lazy `rev` flags — `get` accumulates flip
  parity down the descent (a pending parent flip inverts the
  child's effective flag), `Vec` oracle on every op.
- **`meetmid`** — meet-in-the-middle subset search:
  `subset_sums` (all `2^(n/2)` `i128` sums per half), `subset_sum`
  with index-witness reconstruction, `count_subsets` range
  counting via `partition_point`, `best_fit` — brute-force
  enumeration oracle.
- **`modlin`** — GF(p) linear algebra over `u64` residues:
  `rref`, `rank`, `solve` (free variables zeroed), `nullspace`
  basis — `u128` inner products, Fermat inverses; subset-
  enumerated rank and exhaustive-solution oracles.
- **`veb3`** — recursive van Emde Boas `O(log log U)` set: leaf
  `u64` masks below `LEAF_BITS`, summary+cluster recursion above;
  min held outside clusters (CLRS), predecessor falls back to
  `self.min` when the summary has no earlier cluster — BTreeSet
  oracle across insertion/deletion/query orders.
- **`bigedit`** — multi-word Myers bit-vector edit distance for
  patterns beyond 64 bytes: `ceil(m/64)`-word carry chaining in
  the `(Eq&Pv)+Pv` add and the `Ph`/`Mh` shifts, score from the
  last word's `m−1` bit only — `dist` and `find_leq`, cross-
  checked against the single-word `editdist` and a DP oracle.

### Added — SHA-512, Ed25519, blossom, EPA, alpha hull

Signature-grade hashing, twisted-Edwards signatures, general
matching, penetration depth, and scaled point-set boundaries —
each in published-work form with an independent oracle:

- **`sha512`** — FIPS 180-4 streaming SHA-512: `write`/`finish`
  incremental digest with a 128-bit length field, manual BE word
  loads (no `to_be_bytes` — the repo's width rule), NIST vectors
  plus split-invariance and 111–128-byte boundary coverage.
- **`ed25519`** — RFC 8032 sign/verify/keypair: radix-51 limb
  GF(p) field (the radix-64 column sums overflow `u128`), `[u64;4]`
  mod-L scalars by bit-fold reduction, complete Edwards addition
  (`D = 2·Z1·Z2`), canonical `decompress` rejecting non-canonical
  encodings — RFC TEST 1–3 vectors, tamper, group-law sanity.
- **`blossom`** — Edmonds' blossom maximum matching on general
  graphs: `base[]` blossom contraction inside BFS, `O(n³)`,
  exhaustive-subset oracle for n ≤ 9 plus a two-triangle case
  where contraction is actually needed.
- **`epa`** — expanding polytope penetration depth: seeds the
  origin-enclosing simplex from the GJK loop, expands the closest
  edge along its outward normal until support stalls — exact
  `Frac` depth² + integer MTV axis, SAT-witness brute oracle.
- **`alphahull`** — alpha shape over `delaunay`: keeps triangles
  whose circumradius² fits `α²` (exact `i128` rational
  circumcenter), boundary = once-used edges; the `abc/(4·area)`
  formula double-checks every radius in tests.

### Added — Link–Cut trees, splay BST, Myers edit distance, GJK, TLSF

Dynamic trees, self-adjusting search, bit-parallel string metrics,
exact convex distance, and O(1) segregated allocation, each in
published-work form with an independent oracle:

- **`linkcut`** — Link–Cut dynamic tree (Sleator & Tarjan): `link`/
  `cut`/`connected`/`lca`/`path_min` over a rooted forest via
  aux-path splays with lazy `rev` propagation; `access`'s return
  value doubles as the LCA.
- **`splay`** — bottom-up splay BST: zig / zig-zig / zig-zag
  promotion makes the tree shape a pure function of the operation
  sequence — amortized balance with zero RNG and zero bookkeeping.
- **`editdist`** — Myers' one-`u64` automaton: `dist` for whole
  Levenshtein and `find_leq` for approximate-search end positions;
  the left-column injection bit selects the free-start boundary,
  patterns > 64 words fall back to DP.
- **`gjk`** — integer 2-D GJK: the closest point of the Minkowski
  difference is carried as a `Frac` rational, so `distance2`
  returns exact squared distance and `overlap` is exact —
  verified against SAT and a rational brute-force oracle.
- **`tlsf`** — TLSF-style segregated allocator: two-level (fl, sl)
  bins, neighbor coalescing, and deterministic lowest-address fit;
  `alloc` refuses only when no free block fits, enforced by a
  shadow-oracle invariant.

### Added — radix sort, rank/select, de Bruijn, continued fractions, Earley

Word-level sorts, dense-index primitives, cyclic sequences, exact
rational approximation, and unrestricted grammar recognition, all
in published-work form with independent oracles:

- **`radixsort`** — LSD radix sort over `u64` (eight stable 8-bit
  counting passes), counting sort over `[0, bound)`, and a stable
  `(key, payload)` variant — insertion order survives equal keys,
  the property tick-sorted event queues actually need.
- **`rankselect`** — Jacobson two-level rank/select bitvector:
  `rank0/1` is two lookups plus a popcount, `select0/1` a bounded
  superblock scan — the dense-index primitive `wavelet`/`elias`
  build on.
- **`debruijn`** — de Bruijn `B(k, n)` via FKM Lyndon-word
  concatenation, an `is_debruijn` verifier, and a `window_hash`
  cyclic-window fingerprint.
- **`cf`** — exact continued fractions over `Frac`: canonical
  `to_cf`/`from_cf` round-trip, `convergents`, and `best_approx`
  (closest rational with denominator <= cap, ties to the smaller
  denominator) — oracle-checked against exhaustive enumeration.
- **`earley`** — Earley chart parser for arbitrary CFGs: position
  queues keep scan-ahead items out of the current set, ε-rules and
  mixed-length productions work without CNF conversion — `cyk`'s
  restriction lifted.

### Added
- **`tests/no_nondeterminism_in_sim.rs`** — the non-float half of "replay-safe",
  which nothing checked. `no_float_in_sim.rs` rejects `f32`/`f64`; this rejects
  wall-clock reads, thread spawning, pointer identity and explicit
  `RandomState` outright, and holds every `HashMap`/`HashSet` in library code
  to an allowlist that states, per module, why its iteration order cannot reach
  output. Exact counts, so a module already on the list still trips when it
  grows a new map — "this module was fine before" is not an argument about the
  map just added.

### Added — scatter → territory → wire primitives

The standard procgen pipeline (scatter seeds → partition territory → wire
connectivity) and the lockstep packet primitive, all in published-work form:

- **`mapgen::poisson_disc`** — Bridson's minimal-separation scatter (SIGGRAPH
  2007): `radius/√2` cell grid + active list + annulus rejection sampling.
  Blue-noise placement for rooms, resources, and spawn points.
- **`voronoi`** — exact nearest-seed spatial partition:
  `voronoi_partition` under a selectable `geometry::Distance` metric (lowest
  seed index wins ties) and `voronoi_flood`, the BFS variant that respects
  passable terrain so walls partition territory. `mst_edges` gives the Kruskal
  minimum-spanning edges that connect scattered sites — the
  scatter → territory → connectivity pipeline in three calls. `VoronoiGrid`
  implements `DetHash` and is pinned in `det_hash_golden`.
- **`noise::worley_2d`** — Worley cellular noise (SIGGRAPH'96): exact integer
  `F1`/`F2` feature distances and owning cell id, `worley_2d_f2_minus_f1` for
  vein/crater ridges, `worley_2d_in_range` for quantized output. Uses a 5×5
  scan — the classic 3×3 is only approximate (documented in the module header);
  the extra rings make exactness provable rather than empirical.
- **`bits`** — LSB-first bit-level wire codec: `BitWriter`/`BitReader` with
  packed bitfields, ranged integers (`write_ranged`/`read_ranged`), and
  canonical protobuf-style varint + zigzag. The reader rejects non-canonical
  encodings, keeping value ↔ bytes bijective — the packet primitive lockstep
  netcode layers on (Gaffer serialization strategies).
- **`examples/scatter_pipeline_demo`** — the pipeline end to end:
  `poisson_disc` seeds → `voronoi_partition` territories → `mst_edges`
  corridors, then serialized into a `BitWriter` packet and rendered in the
  terminal.

### Added — wiring, mazes, and hex-grid math

- **`delaunay`** — integer-exact Delaunay triangulation (`delaunay`,
  `delaunay_edges`): Bowyer–Watson incremental insertion in its
  split + Lawson-flip form, with the incircle test evaluated as an `i128`
  determinant and cocircular (`det == 0`) treated as outside so diagonal
  choices stay canonical. The super-triangle sits at distance ~`span²`
  (not ~`span`) — a tighter super-triangle lets circumcircles through
  (hull edge, super vertex) capture interior points, flips hull edges into
  super-touching triangles, and leaves holes after they're dropped
  (regression-tested as `hull_edge_cannot_flip_to_super_vertex`).
- **`hexgrid`** — axial-coordinate hex math in the redblobgames recipe set:
  `Hex`, `DIRECTIONS`, `distance`, `line` (cube lerp + `cube_round` with a
  fixed away-from-zero tiebreak), `ring`, `spiral`, odd/even-r offset
  conversion, `random_in_range`. `Hex` implements `DetHash` (pinned).
- **`maze`** — `wilson_maze`: Wilson's loop-erased random-walk algorithm
  (STOC'96) produces perfect mazes drawn uniformly from spanning trees —
  DFS backtracker mazes are provably non-uniform. Cells on odd coordinates,
  corridors at +1, output is a `mapgen::Dungeon`.
- **`pathfinding::min_cost_path`** — weighted A* over an arbitrary
  `enter_cost: FnMut(i32,i32) -> Option<i32>` (the `None` corner-cut rule
  matches the 8-directional `astar`). `mapgen::carve_corridors` layers the
  TinyKeep recipe on top — floor costs 1, walls `wall_cost` — and bridges
  diagonal steps so carved corridors stay 4-connected.
- **`voronoi::mst_edges_over`** — restricted Kruskal: MST of a point set
  over an allowed edge set (e.g. the Delaunay wiring) rather than the
  complete graph; returns a forest on disconnected inputs.
- **`mapgen::Dungeon` write surface** — `new_walled`, `carve_floor`,
  `fill_wall` (builders like `wilson_maze`/`carve_corridors` need them).

### Added — ray traversal, graph structure, and bin packing

- **`gridcast`** — Amanatides–Woo grid ray traversal (Eurographics'87):
  `grid_ray` walks every cell a segment *enters* — not one cell per column
  like Bresenham — in entry order, all integer arithmetic on doubled
  coordinates with the next-boundary comparison cross-multiplied in `i128`.
  A ray exiting exactly through a corner enters only the diagonally-adjacent
  cell (the two side cells get zero-length intersection), keeping the walk
  minimal and the traversal symmetric: `grid_ray(a,b)` is `grid_ray(b,a)`
  reversed. `ray_blocked_at` and `clear_los` layer LOS queries on top.
  Verified against an independent slab-test oracle over 500 random rays.
- **`graph`** — deterministic graph algorithms on `u32` adjacency lists,
  all iterative (no recursion-depth limit): Tarjan SCC (`strongly_connected`,
  components in reverse topological order), `articulation_points`, `bridges`
  (parallel edges handled correctly — they are never bridges),
  `topo_sort` (Kahn with ascending ready-queue = lexicographically smallest
  order, `None` on cycle), and `UnionFind` (path-halving + union-by-rank with
  smaller-index tie-break so representatives are pure functions of the union
  sequence). Verified against mutual-reachability, remove-and-recount, and
  linear-extension oracles on random graphs.
- **`pack`** — skyline (bottom-left) rectangle bin packing from Jylänki's
  *A Thousand Ways to Pack the Bin* (2010): `pack_skyline` drops each
  rectangle to the lowest fitting skyline segment in input order — a pure
  function of the input slice — for texture atlases, inventory grids and
  dialog tiling. Placement invariants (no overlap, inside the bin) are
  oracle-checked on random inputs.
- **`hexgrid::hex_astar`** — hex-grid A* in the redblobgames formulation:
  unit-cost steps over the six `DIRECTIONS` with exact hex `distance` as
  the consistent heuristic (every node expanded at most once), deterministic
  `(f, h, q, r)` heap order, ordered-map bookkeeping, and a `max_steps`
  budget since the hex lattice is unbounded. Matches a BFS oracle on random
  obstacle fields.

### Added — space-filling curves, contours, flow, assignment, rewriting

- **`zorder`** — Morton (1966) z-order and Skilling Hilbert space-filling
  codes: `morton_encode`/`morton_decode` in 2D/3D and 64-bit widths,
  `hilbert_encode`/`hilbert_decode`, `spatial_sort` for
  locality-preserving point ordering, and `morton_key` over signed
  coordinates via sign-bit bias. Hilbert's encode-time quadrant transform
  rotates against the *full* grid mask `n-1` (decode uses the level size
  `s-1`) — the asymmetric detail that published writeups routinely get
  wrong. Verified by exhaustive round-trips at every resolution and the
  4-adjacency of consecutive Hilbert indices.
- **`msquares`** — marching-squares iso-contour extraction on integer
  scalar fields: `case_index`, `contour_segments`, `contour_loops`.
  Segment endpoints live on edge midpoints in doubled coordinates (no
  fractions, no interpolation parameter); the two saddle cases are
  resolved canonically by the bilinear-centre asymptotic decider. Chains
  close into loops; interior-vertex even-degree and loop-consumption
  invariants are oracle-checked on random fields.
- **`flow`** — `FlowNet`, an Edmonds–Karp max-flow / min-cut network over
  `u32` capacities: `add_edge`/`add_edge_undirected`, `max_flow`,
  `min_cut` (the residual-reachable source partition), `flow_on`
  (per-add-call pushed flow). BFS neighbour order is insertion order, so
  the returned cut is deterministic as well as the (theorem-unique) flow
  value. Verified on random networks: cut capacity equals flow value and
  flow is conserved at every interior vertex.
- **`hungarian`** — `assign_min_cost`, the Kuhn–Munkres O(n³) potential
  method on `n ≤ m` `i64` matrices (negatives allowed, so a profit matrix
  is just a negated one). Every row receives a distinct column; ties break
  deterministically under fixed scan order. Verified against a
  brute-force enumeration of all injective assignments.
- **`lsystem`** — deterministic Lindenmayer rewriting plus an integer
  turtle: `expand` applies rules in parallel (the 0L form — stochastic
  variants intentionally omitted), `turtle_cells` interprets `F/G/f/g`,
  `+`/`-`, `[`/`]` over `DIRS_4`/`DIRS_8`/`DIRS_HEX` direction sets and
  draws through `gridcast` so diagonal strokes never skip cells. Verified
  on the Fibonacci system, branch-state restoration, and 8-connectivity
  of expanded plants.

### Added — polygon predicates, polyline simplification, prefix sums, multi-pattern scan, diffs

- **`poly`** — exact `i128` 2D polygon ops: `area2` shoelace,
  `point_in_polygon` (even-odd with exact boundary classification),
  `convex_hull` (Andrew monotone chain, collinear edge points dropped),
  and `ear_clip` triangulation. The ear test rejects candidates whose
  boundary carries another polygon vertex — a reflex vertex on the ear
  edge lets the triangle bleed into the notch (found via an L-shape
  area mismatch). Oracle-checked: triangle areas partition the polygon
  exactly, the hull strictly convex and containing every input point.
- **`rdp`** — Ramer–Douglas–Peucker polyline simplification (`simplify`,
  `simplify_loop`) on integer points: the perpendicular-distance test is
  cross-multiplied into `i128` (`|cross|²` vs `eps²·|chord|²`), so
  `epsilon` bounds true distance. The canonical downstream pass for
  `msquares` contours; loop anchoring uses the farthest-from-centroid
  point so the cut is content-defined. Idempotent, order-preserving.
- **`fenwick`** — `Fenwick`, a binary indexed tree over `i64`: `add`
  (bool, no panic on out-of-range), `prefix_sum`, `range_sum`, `total`,
  `lower_bound` order statistic, and `from_slice`'s O(n) direct build.
  Every query oracle-checked against a brute-force prefix-sum model.
- **`ahocor`** — `AhoCorasick`, the CACM'75 multi-pattern `&[u8]`
  matcher: trie + fail links built in BFS order, `scan` emits every
  occurrence including overlapping and suffix-contained matches as
  `(end_pos, pattern)` pairs in scan order. BTreeMap children keep
  memory proportional to edges. Oracle-checked against brute-force
  substring search at every position.
- **`diff`** — Myers O(ND) minimal edit scripts: `diff` produces
  `Keep`/`Del`/`Ins` ops, `hunks` coalesces them into unified-diff
  `(a_start, del, b_start, ins)` runs, `apply` verifies the round trip,
  plus `levenshtein` and `lcs_len`. For field-level desync reports and
  did-you-mean diagnostics. Oracle-checked: applied scripts reproduce
  the target and edit counts hit the `n+m−2·lcs` minimum.

### Added — prefix dictionaries, range aggregates, matchings, tours, compaction

- **`trie`** — byte trie with `BTreeMap` children (`insert`, `remove`,
  `contains`, `starts_with`, `keys`, `keys_with_prefix`) — every listing
  is byte-lexicographic, a pure function of the key set. The
  did-you-mean candidate source paired with `diff::levenshtein`.
  Oracle-checked against a `BTreeSet` model through random
  insert/remove/prefix sequences.
- **`segtree`** — `SegTree` range min/max/sum over `i64` with point
  `set`; `range_stats` returns all three aggregates in one `O(log n)`
  walk. Covers arbitrary windows where `fenwick` only answers prefix
  sums. Oracle-checked against brute-force interval scans on random
  update/query mixes.
- **`bipartite`** — `hopcroft_karp` `O(E·√V)` maximum bipartite matching
  plus `kuhn_match`, a naive augmenting-path implementation kept as the
  parity oracle. Unweighted unit↔job pairing, complementing
  `hungarian`'s weighted assignment. Sizes agree with the oracle on 300
  random graphs; matched edges verified present in `adj` with no right
  node reused.
- **`tsp`** — deterministic tour heuristics: `nn_tour` nearest-neighbour
  construction and `tsp_2opt` first-improvement descent on an `i64`
  matrix, canonicalized to city 0 start with `tour[1] < tour[n−1]`
  orientation; `tour_cost` validates the permutation. Patrol routes and
  visit-all missions. Oracle-checked: 2-opt never worse than the NN
  seed, and hits the brute-force optimum on the large majority of
  n ≤ 8 instances.
- **`rle`** — run-length coding: `encode`/`decode` over `(count, byte)`
  pairs (255+ runs split automatically) plus `encode_u32`/`decode_u32`
  for integer streams. The cheapest lossless compaction layer before
  `bits` wire packing; malformed input (odd length, zero count) decodes
  to `None` rather than panicking. Round-trip exact on random
  run-heavy models.

### Added — geometric predicates, Eulerian walks, static range queries, interval sets

- **`segment`** — integer-exact segment predicates: `segments_intersect`
  (straddle test plus four endpoint-on-segment cases), `point_on_segment`,
  `point_segment_dist2`, and `segment_dist2`, all in `i128` orientation
  math. `dist2` returns the **ceiling** of the true squared distance, so
  `== 0` still means "exactly touches" — floor division silently
  collapsed sub-unit rational distances to zero and broke that
  invariant. Oracle-checked against an independent intersection
  formulation and dense point sampling.
- **`euler`** — `euler_walk`: Hierholzer Eulerian circuit/path over an
  undirected multigraph in `O(E)`. Degree parity decides
  `EulerKind::Circuit`/`Path` (self-loops count as degree 2, parallel
  edges as separate edges); a BFS reachability pass rejects
  disconnected inputs and wrong odd counts return `None`. Edge
  consumption is first-unused in input order, so the walk is a pure
  function of the edge list. Oracle-checked by replaying the walk
  against the input multiset.
- **`rmq`** — `SparseTable`: static `O(1)` `range_min`/`range_max` after
  an `O(n log n)` build over `i64`. Min/max are idempotent, so two
  overlapping blocks compose the answer directly — the query-hot,
  read-only complement to `segtree`'s updatable tree. Oracle-checked
  against brute-force scans on every range of random inputs.
- **`closestpair`** — `closest_pair`: `O(n log n)` divide-and-conquer
  closest pair of points with `i128` squared distances and a y-merge
  strip scan. All ties resolve to the lexicographically smallest
  `(dist², p, q)`, so the answer is a pure function of the point set —
  verified identical under input shuffling and against an `O(n²)`
  exhaustive argmin on 400 random sets.
- **`interval`** — `IntervalSet`: a sorted, disjoint, half-open set of
  `i64` intervals with `insert` (merges touched and bridged spans),
  `remove` (splits intervals it cuts through), `clip`, and
  `O(log n)` `contains`/`overlaps`/`overlapping` queries via
  `partition_point`. Occupancy/reservation bookkeeping. Oracle-checked
  against a `BTreeSet` point model through random insert/remove/clip
  sequences, with the sorted-non-adjacent invariant asserted each step.

### Added — scheduling, fuzzy ranking, streaming stats, seeded naming, downsampling

- **`cron`** — `Cron::parse` + `next_after`/`next_n_after`: POSIX 5-field
  cron expressions answered as next-fire times on the millisecond
  timeline. Bitset field sets with a day-loop scan bounded by a 5-year
  horizon; the POSIX OR-rule applies only when *both* day-of-month and
  day-of-week are restricted, `7 ≡ 0` Sunday aliases are merged, and
  Quartz-style `a/n` means `a..hi` step `n`. Oracle-checked against a
  minute-scanning brute force across random calendars, leap years, and
  the rare-by-construction "impossible" schedules (`31 feb`) that return
  `None` forever.
- **`fuzzy`** — fzf-style subsequence scoring for command palettes and
  did-you-mean pickers: `score`, `rank`, `rank_str`. Consecutive runs
  dominate the weighting (fzf's scoring shape), word-boundary starts
  (start/`_-. /` separators/camel humps) boost, gaps cost; the total
  order is score desc → length → bytes → index, so equal keys cannot
  tie. Oracle-checked by a subsequence predicate over thousands of
  random pattern/candidate pairs.
- **`stats`** — `RunningStats`: Welford online mean/variance in
  `i64·SCALE` fixed point, plus `min`/`max`/`count` and Chan's
  parallel `merge` — telemetry and balance dashboards without storing
  samples, and shardable for map-reduce style aggregation. Oracle-checked
  against an exact two-pass `i128` oracle with bounded fixed-point drift.
- **`markov`** — `NameGen`: order-`k` byte-level Markov chain trained on
  a word corpus, sampled through `SplitMix64` draws over cumulative-weight
  tables kept in `BTreeMap` — the generated name is a pure function of
  `(corpus, k, seed)` on every platform. Oracle-checked by asserting every
  order-`k` gram of each output actually occurs in the corpus, and by
  a skewed corpus that must emit `ab` ~9× more often than `ac`.
- **`lttb`** — `lttb`: Largest-Triangle-Three-Buckets downsampling
  (Steinarsson 2013) for dense time-series onto small charts. Bucket
  boundaries come from floor division of the interior point count, each
  bucket keeps the point maximizing `i128` twice-area against the
  previous pick and the next bucket's centroid; endpoints are always
  preserved and the output is an order-preserving subsequence. The
  lone-spike case every `keep-every-k-th` sampler destroys is pinned.

### Added — number theory, ancestor queries, entropy coding, ordered sets, single-pattern scan

- **`ntheory`** — `gcd`/`lcm`/`extgcd`/`mod_inv`/`mod_pow`/`crt2`/`crt`:
  the modular-arithmetic backbone for periodic-event alignment and
  residue addressing. Euclidean gcd over `unsigned_abs` (so
  `i64::MIN` arguments cannot overflow), Bézout coefficients from the
  extended algorithm drive `mod_inv`, binary `mod_pow` keeps
  intermediates in `u128`, and `crt` composes pairwise even when the
  moduli share factors — inconsistent systems return `None` instead
  of a wrong answer. Oracle-checked against a brute-force gcd scan,
  the Bézout identity, and CRT congruence/uniqueness/inconsistency
  classes over random terms.
- **`lca`** — `Lca`: binary-lifting lowest common ancestor over a
  rooted forest given as a `parent[]` array. `O(n log n)` build,
  `O(log n)` `lca`/`ancestor`/`dist`/`depth`; cycles and out-of-range
  parents mark the touched nodes invalid so queries return `None`
  rather than looping or corrupting answers. Oracle-checked against
  an ancestor-set enumeration on random forests, plus deep chains
  and self-queries.
- **`huffman`** — `encode`/`decode`/`build_book`: canonical Huffman
  codec for the wire layer between `rle` and `bits`. The two-queue
  merge makes the tree a pure function of the frequency table, and
  canonical assignment (codes in `(length, symbol)` order) means the
  self-contained stream only carries a `(symbol, length)` table —
  no tree shape. Malformed input (truncated tables, lengths > 32,
  bit-count overruns, trailing partial codes) decodes to `None`.
  Round-trips over random and skewed alphabets, prefix-freeness, and
  the Kraft equality are property-checked.
- **`treap`** — `Treap`: a deterministic ordered `u64` set. Priorities
  are `splitmix64(key ^ seed)`, so tree shape is unique for a given
  key set — insertion order cannot leak into iteration or structure,
  which is exactly what a lockstep dictionary needs. Arena storage
  plus merge/split gives `insert`/`remove`/`contains`/`min`/`max` and
  `rank`/`select` order statistics. Oracle-checked against `BTreeSet`
  across mixed op sequences; shuffled insertion orders yield
  identical `(key, depth)` signatures, and the heap invariant is
  walked directly.
- **`kmp`** — `Kmp`: Knuth–Morris–Pratt single-pattern search with a
  `Stream` variant that accepts bytes one at a time and reports
  absolute match starts across chunk boundaries — the delimiter/
  sentinel scanner for packet streams. `ahocor` remains the
  multi-pattern layer. Oracle-checked against a naive
  every-position scan over tiny alphabets (forcing overlaps) and by
  stream-vs-batch equivalence over random chunkings.

### Added — causality tracking, hash trees, membership filters, snapshot sync, substring compression

- **`vclock`** — `VClock`: Lamport/Fidge–Mattern vector clocks over a
  `BTreeMap<u32,u64>` actor table. `tick`/`merge`/`compare` yield the
  four-way causal order (`Before`/`After`/`Equal`/`Concurrent`), so
  diverged peer histories are provably distinguishable — the predicate
  a lockstep desync report needs to say "peer A's frame never saw peer
  B's input". Verified against the partial-order axioms, the
  least-upper-bound property of `merge`, and a message-flow simulation
  in which every received snapshot must precede the merge result.
- **`merkle`** — `Merkle`: binary hash tree over domain-tagged `u64`
  leaf hashes (`root`/`proof`/`verify`/`first_diff`). Odd nodes are
  promoted unchanged rather than duplicated, avoiding Bitcoin-style
  malleability. Peers compare roots to learn *whether* state diverged
  and descend to the first bad leaf in `O(log n)` node comparisons —
  the stored-state analogue of `replay::first_divergence`. Verified by
  proof round-trips on every leaf, naive leaf-scan equality for
  `first_diff`, and order sensitivity of the root.
- **`bloom`** — `Bloom`: seeded Bloom filter using Kirsch–Mitzenmacher
  double hashing (`h1 + i·h2`) over a `Fnv1a` pair, so the filter is a
  pure function of `(seed, params, inserted multiset)`. One-sided
  error: inserted keys always test present, `definitely_absent` names
  the safe direction, `sizing` gives the `(bits, probes)` heuristic
  and `load_permille` a false-positive-rate proxy. Verified: zero
  false negatives across geometries/seeds, measured FPR inside the
  information-theoretic bound, and bit-array equality under reordered
  insertion.
- **`delta`** — `Delta`/`diff_sorted`/`apply_sorted`: snapshot deltas
  over ordered `u64→u64` maps, with a canonical wire form (ascending
  keys as delta varints over `bits`) that rejects truncated,
  non-increasing, and trailing-garbage encodings. `Del` of an absent
  key fails closed. Verified by round-trip identity on random map
  pairs, minimality of the op count, and malformed-stream rejection.
- **`lzss`** — `compress`/`decompress`: greedy LZ77-family codec on
  the `bits` wire — 4096-byte window, match length 3–18, tokens are
  `0`+8-bit literal or `1`+12-bit `(offset−1)`+4-bit length behind a
  u64 raw-length header. Smallest-offset tie-break makes output a pure
  function of input; completing the `rle`→`lzss`→`huffman` ladder.
  Verified by round-trips over noise/repetitive/run inputs, measurable
  compression on periodic text, and rejection of truncation,
  out-of-window offsets, and absurd length claims.

### Added — rolling fingerprints, suffix arrays, spatial index, 2-SAT, game search

- **`rolling`** — `Rolling`/`hash_bytes`/`find_all`/`chunks`: Rabin–Karp
  mod-2^64 polynomial fingerprints. A fixed-window rolling hash
  (push/pop update in `O(1)`), a multi-match byte search confirmed
  byte-wise, and `chunks` — content-defined boundaries emitted when
  `hash & mask == 0` inside `[min, max]` enforcement. A local edit only
  perturbs nearby chunks, which is what makes delta sync cheap.
  Verified against one-shot recompute, naive all-positions scan, chunk
  width bounds/determinism, and prefix stability under single-byte edits.
- **`suffix`** — `SuffixArray`: prefix-doubling suffix array plus Kasai
  LCP. `search` returns every occurrence in `O(pat·log n + hits)`,
  `lcp`/`lcp_array` expose shared prefixes, `longest_repeated` and
  `distinct_substrings` measure how much a corpus repeats — the audit
  layer for `markov`-style generators. Verified against naive suffix
  sort, direct LCP computation, all-position scan, and `BTreeSet`
  substring counts.
- **`kdtree`** — `KdTree`: static median-split 2-D spatial index over
  `(i32, i32)`. `nearest`/`within`/`in_rect` run in expected `O(log n)`
  with `i128` squared distances and lexicographic tie-breaks — a pure
  function of the point set, insensitive to input order. Verified
  against brute-force scans for all three queries plus input-order
  permutation invariance.
- **`twosat`** — `TwoSat`/`Lit`: 2-SAT over the implication graph,
  reusing `graph::strongly_connected` (Aspvall–Plass–Tarjan).
  `add_clause`/`add_unit`/`add_implies`/`add_equiv`/`add_xor`;
  `solve` returns a canonical assignment picked by SCC order, `None`
  when a variable meets its negation in one component. Verified
  against exhaustive `2^n` satisfiability and the solver's own `check`.
- **`minimax`** — `Game` trait + `score`/`best_move`: deterministic
  negamax with alpha-beta. Moves are tried in the canonical order
  `Game::moves` supplies, ties keep the first candidate, terminal
  scores fold in ply so fast wins rank above slow ones — the
  exhaustive counterpart to `mcts`. Verified over the full
  Tic-Tac-Toe tree: alpha-beta equals unpruned negamax at every
  reachable position, perfect self-play draws, and a win-in-1 plus a
  maximally-delayed loss hit their known ply-discounted values.

### Added — rope, BK-tree, smallest circle, slope trick, NFA regex

- `rope` — balanced rope text buffer: `concat`/`insert`/`remove`/`split`/`slice`/`to_string` in `O(log n)` on node weights; depth > `2·⌈log2 n⌉+1` triggers a whole-leaf rebuild so degenerate edit patterns recover in amortized linear time.
- `bktree` — Burkhard–Keller metric tree indexing `diff::levenshtein` distances: `insert`/`contains`/`within` (band-pruned descent) / `nearest` (best-first bound update); results sorted by `(dist, key)`.
- `mincircle` — Welzl smallest enclosing circle on `Frac` points: sorted+deduped input makes the iterative triple loop a pure function of the point set (MEC uniqueness); collinear points fall back to the widest-pair diameter circle.
- `slopetrick` — slope-trick convex piecewise-linear primitive (`SlopeTrick`): `O(log n)` `add_a_minus_x`/`add_x_minus_a`/`add_abs` in the canonical four-line heap form, `clear_left`/`clear_right` prefix/suffix mins, `slide(a,b)` sliding-window min, `shift` domain translate, `eval`/`min`/`argmin`.
- `regex` — Thompson-construction byte regex (`Regex`): literals, `.`, `[a-z]`/`[^..]`/`\d`/`\w`/`\s` classes, `|`/`?`/`+`/`*` and groups; pike loop with `seen`-set ε-closure, unanchored match reseeds `start` per step; syntax errors return `None` (no panic).

### Added — succinct/monotone structures, BLAKE2s, calipers, SMAWK

- `elias` — Elias–Fano monotone `u64` sequence: `access`/`rank`/`successor`/`successor_strict` over a unary-gap high bitmap + verbatim low bits; construction sorts a copy so the index is a pure function of the multiset.
- `patricia` — crit-bit (PATRICIA) radix tree over `u64`: `insert`/`contains`/`min`/`max`/`floor`/`ceil`/`predecessor`/`successor` and sorted-order `iter`; bound queries prune by subtree min/max since untested high bits make branch membership non-trivial.
- `blake2s` — BLAKE2s-256: streaming `Blake2s` + `blake2s`/`blake2s_keyed` (RFC 7693 keyed-MAC mode, no HMAC needed); key block stays buffered so an empty message finishes correctly.
- `rotcal` — rotating calipers on a convex hull: `diameter` (pair + squared distance), `min_width` and `min_rect_area` as exact `Frac` values; four-calipers pointers are edge-0 seeded so monotone advance can't miss extrema.
- `smawk` — SMAWK row-argmin `O(rows+cols)` for totally monotone implicit matrices: ties to smallest column; the engine for Monge-DP speedups.

### Added — keyed hashes, pairing heap, sorting network, offline LCA

- `siphash` — SipHash-2-4 keyed 64-bit PRF: streaming `SipHash` state (split-independent via an 8-byte staging buffer) + one-shot `siphash`; paper vectors verified.
- `hmac` — RFC 2104 HMAC-SHA256 over `sha256`: `Hmac`/`hmac_sha256`, keys longer than the 64-byte block hashed first; RFC 4231 test cases verified.
- `pairingheap` — arena pairing heap: O(1) `meld` (heap arenas appended and indices shifted), canonical `(prio, key)` pop order from a two-pass pairing.
- `bitonic` — Batcher bitonic sorting network: `network(n)` emits the fixed `(i, j, ascending)` comparator sequence (data-oblivious — identical trace on every peer), `sort` pads non-power-of-two inputs with `!0` sentinels.
- `offlinelca` — Tarjan offline LCA: one iterative DFS + DSU answers a whole query batch; same-`tree_of` guard shields cross-tree queries (DSU state persists across roots) and invalid/unreachable vertices answer `None`, matching `lca` semantics.

### Added — range-add Fenwicks, geohash, octree, base64, König vertex cover

- `fenwickrange` — `RangePoint` (range-add / point-query difference BIT) and `RangeSum` (two-BIT range-add + range-sum via `P(x)=prefix(B1,x)·x−prefix(B2,x)`), 0-based half-open API over 1-based internals.
- `geohash` — integer microdegree (`e6`) lat/lon geohash: `encode`/`decode` (returns cell bounds), `cell_span`, 8-neighbor cells clamped to the world — lon-first interleaved 5-bit bisections.
- `octree` — bucketed 3-D `i64` point index (BUCKET=8, MAX_DEPTH=32): sorted-canonical range queries, best-first `nearest` with `(dist, pt)` tie-break; subdivision stops at unit cells so identical-coordinate stacks never hang.
- `base64` — RFC 4648 strict codec: `encode`/`decode` plus `encode_url`/`decode_url`; decode rejects bad padding, interior `=`, and non-alphabet bytes.
- `vertexcover` — König minimum vertex cover on bipartite graphs: maximum matching via `bipartite::hopcroft_karp` + alternating-reachability BFS gives `(L\Z)∪(R∩Z)`.

### Added — ordered sets, learned indexes, big-integer product, block cipher

- `skiplist` — `SkipList`: deterministic `u64` ordered set — level = `trailing_zeros(hash(key,seed))` (geometric, pure function of key set), `insert`/`remove`/`contains`/`iter`, lane shape insertion-order-independent.
- `karatsuba` — `mul`/`schoolbook_mul`: Karatsuba multiplication over base-2^64 limbs — `O(n^1.585)` multi-word unsigned product, u128 intermediates, `CUTOFF=16` dispatch.
- `lsm` — `Lsm`: log-structured merge index — BTreeMap memtable, frozen sorted runs newest-first, tombstone deletes, tiered `compact`; observable state is a pure function of the op log.
- `pgm` — `PgmIndex`: PGM-style learned index over sorted `u64` — rational-slope segments with exact `ε` bound, predict + windowed binary search for `get`/`rank`.
- `aes` — `Aes128`: AES-128 block encrypt/decrypt — S-box computed via `gf2` inverse + affine (no stored table), FIPS-197 vectors verified.

### Added — one-time MAC, predecessor sets, compressed bitmaps, string canonicalization, lattice transforms

- `poly1305` — `Poly1305` + `poly1305`: RFC 8439 one-time authenticator (5×26-bit DJB limbs, incremental `write`/`finish`, split-invariant; §2.5.2 tag verified).
- `veb` — `Veb`: proto van Emde Boas `u32` set — `insert`/`remove`/`contains`/`min`/`max`/`predecessor`/`successor` via two-level sqrt bitset layout, BTreeSet-parity iteration.
- `roaring` — `Roaring`: roaring bitmap over `u32` — array/bitset containers with 4096-entry conversion, ascending `iter`, `union`/`intersect`/`difference`/`symmetric_difference`.
- `lyndon` — `lyndon_factorize` (Duval, `O(n)`) + `min_rotation` (Booth) + `is_lyndon` — cyclic/canonical forms for necklaces and rotation-symmetric states.
- `sosdp` — subset/superset zeta–Möbius transforms + `or_convolve`/`and_convolve`/`xor_convolve` (Walsh–Hadamard) — `O(n·2^n)` counting transforms over bitmask tables.

### Added — stream cipher, cryptographic digest, clustering, static spatial index

- `chacha` — `ChaCha20`: RFC 8439 ChaCha20 keystream — `[SIGMA|key|counter|nonce]` block function, streaming XOR apply
- `sha256` — `Sha256`/`sha256`: FIPS 180-4 SHA-256 — incremental writes, canonical big-endian padding, all NIST vectors
- `dbscan` — `dbscan`: squared-distance density clustering — canonical index-order expansion, pure function of `(points, eps², min_pts)`
- `kmeans` — `kmeans`: deterministic integer k-means — farthest-point initialization, Lloyd iterations to assignment fixpoint
- `rtree` — `Rtree`: STR bulk-loaded static R-tree — pure-function point index, canonical ascending-order queries

### Added — deletion-capable filters, entropy coding, interior poles, perfect hashing, independent sets

- `cuckoof` — `CuckooFilter`: deletion-capable membership filter — two candidate buckets `h2 = h1 ^ hash(fp)`, seeded bounded kick chain
- `rans` — `Rans`: rANS entropy codec — largest-remainder normalized table, single-state integer coder, sub-bit/symbol compression
- `polylabel` — `polylabel`: pole of inaccessibility — integer-only branch-and-bound maximizing min squared distance to a polygon boundary
- `mphf` — `Mphf`: CHD minimal perfect hash — static `n` keys → `[0,n)` bijection, pure function of `(key set, seed)`
- `mis` — `maximal_independent_set`: canonical greedy MIS — ascending-order inclusion, verified independent + maximal per definition

### Added — indexed queues, persistent trees, checksums, tree search, generational handles

- `iheap` — `IHeap`: indexed binary heap — key-addressed `set`/`decrease`/`increase`/`remove`, canonical `(priority, key)` pop order
- `pstree` — `PersistentTree`: chairman persistent segment tree — version roots per prefix, `kth`/`freq`/`range_count` over any slice
- `crc` — `crc32`/`Crc32`: streaming IEEE CRC-32 — chunk-invariant, detects every single-bit flip
- `mcts` — `mcts`: seeded UCB1 Monte-Carlo tree search over `minimax::Game` — integer-only statistics, pure function of `(position, budget, seed)`
- `slotmap` — `Slotmap`: generational `u64` handle store — stale handles structurally rejected, canonical slot-order iteration

### Added — membership filters, caches, weighted sampling, spatial index, array-to-tree bridge

- `xorfilter` — `XorFilter`: static xor filter (Graf & Lemire) — ~0.4% false-positive rate, zero false negatives for baked-in keys, deterministic BFS-peel construction with automatic seed retry
- `lru` — `Lru`: bounded LRU cache over `u64` — canonical (stamp, key) eviction, `get`/`peek`/`by_recency`/`pop_lru`
- `vose` — `AliasTable`: Vose's alias method — O(1) weighted sampling with an exact integer distribution over `u128` internals
- `quadtree` — `Quadtree`: bucketed dynamic point index over `u32` — canonical sorted `query`/`count`, best-first `nearest`
- `cartesian` — `Cartesian`: O(n) cartesian tree — heap on values + BST on positions, `rmq`/`subtree_range`/`inorder`

### Added — approximate string search, swarm pathfinding, expression eval, convex collision, buddy allocation

Round-27 survey additions (192 → 197 modules):

- **`bitap`** — `Bitap`: Shift-And bit-parallel search (Wu & Manber
  1992) — patterns ≤ 64 bytes track match state in one `u64`;
  `fuzzy_search(text, k)` adds one state word per allowed
  substitution (Hamming only — insertions/deletions are *not*
  tolerated). The substitution term needs its own bit-0 seed
  (`(prev << 1) | 1`): the empty prefix is always true, and
  without it substituting `pattern[0]` can never fire — a real
  correctness bug the naive per-window Hamming oracle caught.
  End positions (index past the hit) returned ascending.
- **`flowfield`** — `FlowField`: one-to-all swarm pathfinding —
  reverse Dijkstra integration field from a goal set plus a
  per-cell direction field, so every agent moves O(1)/step after
  one build. 8-connected, diagonal moves need both orthogonals
  free (no corner cutting), integer √2≈1.5 scaling
  (`2·cost` ortho / `3·cost` diag) keeps distances exact.
  Verified by an independent field re-build plus structural
  invariants: every direction strictly descends and every
  reachable cell's path terminates on a goal.
- **`shunting`** — `shunting_yard` + `eval`/`eval_rpn`: Dijkstra's
  infix→postfix with strict `i64` evaluation — `+ - * / %`, the
  right-associative unary `Neg`, parentheses. Operand/operator
  alternation and paren depth are checked during the parse, so a
  `Some` postfix is always evaluable; `/` and `%` truncate toward
  zero like Rust, and division-by-zero, `i64::MIN / -1`, and every
  overflow return `None` rather than panicking. Recursive-descent
  oracle, 2000 random expressions.
- **`sat`** — `collide`/`overlap`: separating-axis convex collision
  in `i128` — only edge normals are candidate axes, early-exit on
  the first separator. Boundary contact counts as overlap;
  `depth` is in projection units (true depth = `depth/|axis|`) and
  the axis is canonically oriented centroid-to-centroid. Points
  are legal (their projection inside every facet slab is
  containment). Oracle: inclusive edge-intersection + containment
  both directions, 3000 random hull comparisons.
- **`buddy`** — `Buddy`: binary buddy allocator — power-of-two
  blocks split on demand and eagerly coalesce on free, with sorted
  lowest-address free lists so the allocator state is a pure
  function of its operation sequence and the canonical invariant
  (no two buddies simultaneously free) always holds. Byte-shadow
  oracle + full drain check across 300 arenas; double-free and
  foreign addresses return `false`.

### Added — interval queries, tree decomposition, text buffers, approximate Steiner, write-ahead log

Round-26 survey additions (187 → 192 modules):

- **`intervaltree`** — `IntervalTree`: centered interval tree over
  half-open `[lo, hi)` spans — `stab` (point hits) and `overlap`
  (range hits) return sorted `u32` indices. The pivot is the
  midpoint of the endpoint span, which provably lands strictly
  inside ≥1 interval so recursion always shrinks (an endpoint
  median does not — it can equal a leaf `hi` and never terminate).
  `overlap(l,l)` degenerates to `stab(l)`; `l>r` fails closed to
  empty. Naive all-interval oracle, 300 randomized comparisons.
- **`centroid`** — `Centroid`: centroid decomposition of a forest —
  `parent`/`children`/`depth`/`order`/`roots` per vertex plus a
  `lca` on the centroid tree (`O(log n)` walks; also the basis for
  `O(log n)`-amortized tree distance queries). `sizes` uses an
  iterative `seen`-marked DFS and `find` walks only spanning-tree
  edges toward the oversized side — both required for guaranteed
  termination on cyclic inputs, where the classic guarantees are
  stated for trees anyway. Oracle: every removal's pieces ≤ half.
- **`piecetable`** — `PieceTable`: editor-style text buffer
  (Crowley 1998) — immutable `original` + append-only `added` +
  a piece list; `insert` appends and splits one piece, `delete`
  trims endpoints and drops the middle, adjacent same-store pieces
  coalesce. `O(#pieces)` locate — the right trade at sim scale
  versus `O(n)` Vec shifts. Vec-splice oracle, 300 randomized
  comparisons; out-of-range edits fail closed (`false`).
- **`steiner`** — `steiner_tree`: Kou–Markowsky–Berman
  2-approximate Steiner tree — per-terminal Dijkstra metric
  closure, Kruskal MST on terminals, shortest-path unfolding, and
  a final MST over the unioned edges to prune cycles (weight can
  only drop). `None` when terminals are disconnected. Oracle:
  Dreyfus–Wagner exact DP (`k ≤ 5`) — `opt ≤ w ≤ 2·opt` on 300
  random graphs, plus tree-shape and spanning checks.
- **`wal`** — `Wal` + `decode`: write-ahead log codec —
  `[kind:u8|len:u32|crc:u64|payload]` records, CRC = domain-
  separated Fnv1a over `"walv1" ‖ kind ‖ len ‖ payload`. Replay is
  torn-tail tolerant: decoding stops at the first truncated or
  checksum-mismatched record and reports `stopped_at` — verified
  at *every* cut position and under bit flips; `truncate(stopped_at)`
  repairs the log.

### Added — field arithmetic, erasure coding, compressed indexes, lightweight shortest paths, SAT

Round-25 survey additions (182 → 187 modules):

- **`gf2`** — GF(2⁸) field arithmetic over the AES polynomial `0x11B`:
  `add`/`sub` (xor), peasant `mul`, exp/log `Tables` built on
  generator 3, `mul_t`, `inv`, `pow`, `div` (`None` on divide by
  zero) — the arithmetic core erasure codes and universal hashing
  build on. Field axioms oracle-checked; the generator is verified
  to cycle through all 255 nonzero elements.
- **`rsfec`** — `ReedSolomon`: Vandermonde Reed–Solomon erasure
  coding over `gf2` — coding matrix `V(total,data)·V_top⁻¹` keeps
  the top `data` rows the identity while every `data`-row subset
  stays invertible. `encode` / `encode_shards` fill parity shards,
  `reconstruct` recovers `k` data shards from any surviving `k`
  and re-encodes missing parity — lockstep packet-loss recovery.
  Exhaustive 4-of-8 (all 70 subsets) + random-loss oracle checks.
- **`fmidx`** — `FmIndex`: FM-index over a cyclic BWT — `C` table
  plus `Occ` checkpoints every 32 rows plus the full rotation
  suffix array. `count`/`locate`/`range` run sublinear backward
  search; cyclic occurrence semantics (patterns may wrap end→start)
  are documented and oracle-checked against naive cyclic
  enumeration.
- **`zerobfs`** — `zero_one_bfs` (deque: 0-edges front, 1-edges
  back, `O(V+E)`) and `dial` (bucket array of `cap·(n−1)` slots,
  `O(cap·V+E)`) — linear-ish shortest paths when edge weights are
  tiny integers. Edges exceeding the declared bound fail closed
  (`None`). `bellman`-compatible distance vectors, Bellman–Ford
  oracle-checked.
- **`dpll`** — `solve`: DPLL SAT over CNF — unit propagation and
  pure-literal elimination to a fixpoint, then a deterministic
  smallest-variable split (`true` first). Returned models are
  canonical (unset variables `false`), so `solve` is a pure
  function of the clause set. Brute-force `2^n` oracle checks
  satisfiability and validates every returned model.

### Added — dominator trees, 2-D prefix sums, feasible flow, biconnectivity, similarity fingerprints

Round-24 survey additions (177 → 182 modules):

- **`dominators`** — `Dominators`: Cooper–Harvey–Kennedy iterative
  dominator tree over an arbitrary successor closure — `idom`,
  `dominators(v)` root-chains, sorted `children`, and Cytron
  `frontier` dominance frontiers. Spawn gating, dependency
  scheduling, single-entry regions.
- **`fenwick2d`** — `Fenwick2d`: 2-D binary indexed tree over a dense
  `w × h` `i64` grid — `add`, `prefix`, `rect_sum` (inclusion–
  exclusion on four corners), `get`, `total`; heatmap and resource
  fields where 1-D BITs cannot answer rectangles.
- **`circulation`** — `feasible_circulation(n, edges, demands)`:
  feasible circulation with per-edge lower/upper capacities and
  vertex demands, via `req[v] = demand − lo_in + lo_out` and a
  super-source/sink reduction over `flow`. Returns per-edge flows
  `lo + residual` or `None`; skipped (self-loop / forced `lo==hi`)
  edges carry their lower bound.
- **`biconn`** — `biconnected_components`: Tarjan edge-stack
  decomposition into maximal biconnected edge sets — edges on a
  common simple cycle land together, bridges and articulation
  failures surface as singleton components. Handles multigraphs
  (parallel edges form real 2-cycles).
- **`simhash`** — Charikar locality-sensitive fingerprints:
  `simhash` / `weighted_simhash` (each feature hash votes ±weight
  per bit, positive majority wins, ties → 0), `hamming`, and
  `near_dupes` — near-duplicate detection for generated content
  with multiset semantics.

### Added — palindrome structures, offline range queries, maximal rectangles, stable matching, weighted union-find

Round-23 survey additions (172 → 177 modules):

- **`eertree`** — `Eertree`: palindromic tree over `&[u8]` —
  `distinct_palindromes`, `palindromic_substring_count` (`Σ occ`),
  `occurrences(pat)` (center-out node walk), `longest_palindrome`;
  online extension with the imaginary-root (`len = −1`) terminating
  every suffix-link search, `occ` propagated `len`-descending on
  `finish`.
- **`mo`** — Mo's offline range queries: `mos_order` (block-sorted,
  serpentine query permutation via integer `isqrt`) plus
  `range_distinct` driving `add`/`remove` over a `BTreeMap` counter —
  batched window answers with no data structure.
- **`histrect`** — `largest_rectangle`: monotonic-stack largest
  histogram rectangle `O(n)` (virtual `h = 0` flushes the stack) and
  `maximal_rectangle`: per-row running heights feeding the same scan —
  building footprints, warehouse slotting, territory blobs.
- **`stable`** — `stable_match`: Gale–Shapley proposer-optimal stable
  marriage over full `n × n` preference permutations (proposal order
  provably irrelevant to the unique optimum), plus `is_stable`, an
  independent blocking-pair verifier.
- **`wdsu`** — `WeightedDsu`: potential-annotated union-find —
  `unite(u, v, w)` asserts `pot[v] − pot[u] == w` and returns `false`
  on contradiction (component unchanged), `diff` answers
  `pot[v] − pot[u]` inside a component, `same`, `components`.

### Added — substring automata, exact tours, exact cover, directed trees, xor bases

Round-22 survey additions (167 → 172 modules):

- **`sam`** — `Sam`: suffix automaton built by online extension — `contains`,
  `occurrences` (`finish` propagates end-position counts down suffix links
  in `len`-descending order), `longest_common`, `distinct_substrings`
  (`Σ len[v] − len[link[v]]`), `longest_repeated`; `len`/`is_empty` for the
  text itself.
- **`hamdp`** — `tsp_exact`: Held–Karp exact TSP `O(n²·2ⁿ)` for `n ≤ 16`
  cities over `u32` weights (`u32::MAX` = absent edge); returns cost plus
  the lexicographically-smallest optimal tour, reconstructed via greedy
  prefix + exact tail re-solve.
- **`dlx`** — `exact_cover`: Algorithm X over a binary incidence matrix —
  fewest-row column heuristic (lowest index on ties), covered-column bitset
  plus a journal of disabled rows for exact undo; returns the
  lexicographically-first sorted row-id solution.
- **`arborescence`** — `directed_mst`: Edmonds' minimum-cost arborescence
  via cycle contraction (min incoming edge → cycle → contract with
  `w − best_in` adjustment → expand); returns total weight plus the chosen
  edge indices, `None` when any non-root vertex is unreachable.
- **`xorbasis`** — `XorBasis`: GF(2) linear basis over `u64` kept in
  reduced row-echelon form at every insert — `contains`, `max_xor`,
  `rank`, `kth`-smallest span element, `span_size`, `vectors`; canonical
  form makes the result insertion-order-independent.

### Added — reversible transforms, tree path queries, min cut, primality, wavelet queries

Round-21 survey additions (162 → 167 modules):

- **`bwt`** — cyclic Burrows–Wheeler transform `bwt`/`bwt_inverse`
  (rotation ordering from `SuffixArray` over the doubled string, LF-mapping
  inverse) plus `mtf_encode`/`mtf_decode` — the reversible front half of a
  bzip2-style pipeline ahead of `rle`/`huffman`.
- **`hld`** — heavy-light decomposition (`Hld`): `path_vertices`, `path_segments`
  (`O(log n)` flat ranges for `segtree`/`fenwick`), `subtree_segment` over a
  `parent[]` tree; deterministic heavy-child tie-break (max size, lowest index).
- **`mincut`** — `global_min_cut`: Stoer–Wagner `O(n³)` global min cut
  returning cut weight + one side; parallel edges sum, ties break to the
  lowest vertex index.
- **`miller`** — `is_prime`/`factor`: exact `u64` primality via the
  deterministic 7-base Miller–Rabin set (`{2,325,9375,…,1795265022}` covers
  all `n < 2^64`), sorted factorization via trial division + Brent rho with
  a fixed polynomial schedule.
- **`wavelet`** — `WaveletMatrix`: `access`/`rank`/`freq_less`/`range_freq`/
  `quantile` on `u32` sequences in `O(bits)` — layered stable-partition
  bitplanes; all-integer, allocation-once.

### Added — min-cost flow, knapsacks, coloring, undoable connectivity, line envelopes
- **`mcflow`** — `FlowNet::min_cost_max_flow`: Edmonds–Karp augmentation
  to maximum flow, then cycle-canceling via `bellman::negative_cycle` —
  push each negative residual cycle's bottleneck until cost is minimal.
  Signed per-unit costs, exact integer result, deterministic ordering.
- **`knapsack`** — `knapsack_01`/`knapsack_unbounded`: `O(n·W)` exact
  knapsack with witness reconstruction. The 0/1 variant keeps the full
  DP table so reconstruction can compare layers (ties keep earliest
  items — deterministic loadouts).
- **`coloring`** — `dsatur`/`is_proper`: DSATUR coloring with
  saturation → degree → index tie-breaks; exact on bipartite graphs and
  cycles, a strong deterministic heuristic otherwise.
- **`dsurb`** — `DsuRollback`: union-find with `snapshot`/`rollback` for
  hypothetical connectivity queries. Union-by-size plus a merge journal;
  no path compression (it can't be undone) so `find` is `O(log n)`.
- **`cht`** — `LiChao`: Li Chao tree over a bounded integer domain —
  `insert` lines `y = a·x + b`, `query_min` in `O(log X)` with `i128`
  evaluation. For `min_j (aⱼ·x + bⱼ)` transitions and cost curves.

### Added — subsequence, DAG paths, selection, rasterization, recurrence jump
- **`lis`** — `lis`/`lis_len`: patience-sorting longest increasing
  subsequence in `O(n log n)` returning an actual witness subsequence
  (verified against an `O(n²)` DP oracle on 400 random inputs).
- **`dagsp`** — `dag_paths`/`critical_path`: DAG shortest and longest
  distances in `O(V+E)` over `topo_sort`. `critical_path` returns the
  critical vertex chain — longest path initialised `dist[v]=0` at every
  vertex (a path may start at a non-source when all incoming edges are
  negative — the initialisation subtlety an oracle caught).
- **`bfprt`** — `select`/`median`: Blum–Floyd–Pratt–Rivest–Tarjan
  median-of-medians selection — deterministic `O(n)` worst case, Dutch-
  flag three-way partition, no RNG anywhere.
- **`raster`** — `line`/`circle`/`fill_polygon`: integer grid
  rasterization. The Bresenham line is reversal-symmetric by construction
  (canonical lexicographic direction); `fill_polygon` classifies cell
  centres with `i128` rational crossings — no boundary ambiguity.
- **`linrec`** — `linrec`/`linrec_mod`: k-th term of a linear recurrence
  via companion-matrix exponentiation in `O(d³ log k)`; exact `i128`
  (None on overflow) or modular `u64` with `u128` intermediates.

### Added — set similarity, Z-scan, signed shortest paths, rationals, window aggregates
- **`minhash`** — MinHash similarity signatures: `k` independent seeded
  minima per set; `estimate` returns Jaccard in permille (error ~ `1/√k`,
  no floats), `union` merges positionwise, and `jaccard_exact` ships as a
  public sorted-merge oracle for verification.
- **`zfunc`** — Z-algorithm prefix-match array in `O(n)`: `z` per-position
  prefix-match lengths, `z_search`/`z_search_bytes` substring search via
  `pat+sep+text` (falls back to naive scan when the separator byte appears
  in either side), `borders`, `min_period`.
- **`bellman`** — Bellman–Ford single-source shortest paths with signed
  edges (`O(V·E)`): returns `None` when a negative cycle is reachable;
  `negative_cycle` connects a super-source to return an actual cycle's
  vertex list. Extends `pathfinding` to the negative-weight domain.
- **`frac`** — `Frac` normalized `i128` rationals: `gcd=1, den>0` kept
  structurally so equality is value equality; `+`/`-`/`*`/`÷`(Option),
  compare, `abs`, `neg`, `split_mixed`. The user-facing counterpart of
  `gauss`/`bezier`'s internal fractions.
- **`slide`** — `slide_min`/`slide_max` monotonic-deque sliding-window
  extrema in `O(n)` — `segtree`'s range answer compressed to linear when
  the range slides by one.

### Added — summary sketches, rendezvous hashing, adaptive dictionary compression

- **`kmv`** — `Kmv::new`/`add`/`add_hash`/`estimate`/`merge`: K-minimum-
  values distinct-count sketch over seeded Fnv1a hashes. Exact while
  `distinct < k`; above that the integer-only estimate `(k−1)·2⁶⁴/vₖ`
  (u128 intermediate, `u64` saturated). `merge` is union of minima —
  idempotent, commutative, equal to the merged stream. Verified
  against `BTreeSet` exact counts with a 4σ integer window.
- **`cms`** — `CountMin::new`/`add`/`add_count`/`estimate`/`merge`/
  `total`/`depth`/`width`: count-min sketch frequency estimation on a
  `depth×width` counter matrix with independent seeded row hashes.
  One-sided error by construction — collisions only add, so estimates
  never undercount. `merge` refuses (`None`) on dimension/seed
  mismatch rather than producing meaningless counters.
- **`quantile`** — `Quantile::new`/`add`/`quantile`/`len`: Greenwald–
  Khanna ε-approximate quantiles — `(v,g,δ)` tuples with periodic
  `g_i + g_{i+1} + δ_{i+1} ≤ ⌊2εn⌋` compaction, all-integer arithmetic.
  Every returned value satisfies `|true_rank − φ·n| ≤ ε·n`, verified
  per-decile against a sorted oracle on 50k streams.
- **`chash`** — `pick`/`pick_seed`/`pick_top`/`pick_top_seed`/
  `distribution`: rendezvous (HRW) consistent hashing — each key maps
  to the maximum seeded weight node; removing a node remaps only the
  keys it owned (minimal disruption), and the assignment is a pure
  function of `(nodes, key)` so every peer agrees with zero
  communication. `pick_top` yields ordered replication groups.
- **`lzw`** — `encode`/`decode`: LZW phrase-table codec emitting
  fixed-width 12-bit codes on `bits` (256 literals + dictionary grown
  to 4096 then frozen). The wire carries no dictionary — the decoder
  rebuilds it in lockstep, handling the KwKwK (`code == dict.len()`)
  case structurally. Truncated/out-of-range wires return `None` or an
  exact prefix; round-trip verified on random, repetitive, and
  text-modeled inputs.

### Added — permutation algebra, NTT convolution, palindrome structure, exact linear algebra, integer curves

- **`perm`** — `identity`/`is_valid`/`compose`/`inverse`/`cycles`/
  `sign`/`order`/`apply`/`rank`/`unrank`: permutation group operations
  with canonical cycle form (least element first, cycles sorted) plus
  Lehmer-code factoradic ranking — `unrank` maps `r ∈ 0..n!` onto
  permutations bijectively for `n ≤ 20`, so low seed bits literally
  pick a spawn ordering. Verified by associativity/inverse/
  `p^order = e` group axioms, sign = inversion parity, and exhaustive
  rank/unrank round-trips for `n ≤ 7`.
- **`conv`** — `convolve`/`convolve_i64`: number-theoretic-transform
  convolution mod `998244353` (primitive root 3, `u128` intermediates)
  — `O(n log n)` polynomial products for dice-sum distributions and
  generating-function counts with none of float FFT's rounding
  nondeterminism. `convolve_i64` returns `None` rather than a wrapped
  answer when the exact coefficients would exceed the modulus.
  Verified against naive `O(n²)` modular convolution, commutativity,
  and signed-value recovery.
- **`manacher`** — `odd_radii`/`even_radii`/`longest_palindrome`/
  `count_palindromes`: Manacher's `O(n)` palindrome structure (the
  classic `d1`/`d2` arrays) for string symmetry — name linting, seed
  prettiness, palindromic quest IDs. Verified against exhaustive
  palindrome enumeration plus per-entry truth/maximality of both
  radius arrays.
- **`gauss`** — `det`/`solve`/`rank`: Bareiss fraction-free Gaussian
  elimination over `i64` matrices with `i128` intermediates — exact
  determinants (no "pivot too small" ambiguity), exact rational
  solutions as reduced `(num, den)` pairs, and exact rank; `None` is
  a *proof* of singularity. `det` and `rank` run independent
  elimination paths that verify `det == 0 ⟺ rank < n`. Checked
  against permutation-expansion determinants and full `A·x = b`
  rational verification.
- **`bezier`** — `cubic_pos`/`catmull_pos`/`flatten_cubic`/
  `flatten_catmull`: integer-exact parametric curves evaluated at
  rational `t = num/den` — every coordinate a reduced `i128`
  fraction, so camera paths, projectile arcs, and patrol routes are
  bit-identical across machines. Catmull-Rom interpolates both inner
  knots exactly; flatteners emit endpoint-preserving polylines.
  Verified against an independent Horner-form oracle, endpoint
  restoration, and the interpolation property.

### Added — the verification family

Eleven modules that do nothing but interrogate a simulation. Each is grounded
in published work, and each is tested against an independent oracle rather than
hand-computed expectations.

- **`sim`** — one `Simulation` trait every checking tool consumes, plus
  `audit()`: double-run and rollback-resimulation in a single call. (Schneider,
  *State Machine Approach*, ACM Comput. Surv. 22(4), 1990.)
- **`verify`** — bounded model checking. `check_invariant` enumerates every
  reachable state and either **proves** an invariant holds or returns the
  shortest input sequence that breaks it; `check_temporal` extends that to
  ordering properties by crossing the state space with a `temporal` monitor.
  The result is three-way, so a proof is never confused with a search that ran
  out of budget, and liveness is refused rather than falsely proved.
  (Clarke/Emerson/Sistla, ACM TOPLAS 1986; Holzmann, IEEE TSE 1997; Vardi &
  Wolper, LICS 1986.)
- **`temporal`** — runtime verification of properties that span time:
  `always` / `eventually` / `until` / `precedes` / `responds_within`, with an
  LTL₃ anytime verdict during a run and a definite one at the end. (Bauer,
  Leucker & Schallhart, ACM TOSEM 20(4), 2011; Dwyer, Avrunin & Corbett, ICSE
  1999.)
- **`recovery`** — crash-recovery testing. Injects a save/restore cycle after
  every input and distinguishes an unloadable save, one that drops a hashed
  field, and one that drops state the hash does not cover — the last of which a
  hash comparison alone cannot see. (FoundationDB simulation testing; Pillai et
  al., OSDI 2014.)
- **`explore`** — archive-based state-space exploration, reaching states that
  memoryless random play cannot. (Ecoffet et al., *First return, then explore*,
  Nature 590, 2021.)
- **`prop`** — property-based testing with automatic shrinking
  (`forall_inputs`, `forall_states`), plus model-based / differential testing
  (`forall_model`) that runs the real simulation in lockstep with a trusted
  reference and reports the first diverging command. (Claessen & Hughes, ICFP
  2000; Hughes, LNCS 9600, 2016; McKeeman, DTJ 1998.)
- **`shrink`** — delta debugging: reduce a failing input sequence to a
  1-minimal one. (Zeller & Hildebrandt, IEEE TSE 28(2), 2002.)
- **`plan`** — BFS synthesis of a shortest input sequence reaching a goal.
- **`dst`** — seed sweeps, double-run nondeterminism detection, and swarm
  testing. (Groce et al., ISSTA 2012.)
- **`rollback`** — bounded snapshot ring and a GGRS-style `sync_test`.
- **`world_hash`** — `DetHash` coverage probes (mutation testing for state
  hashes) and an opt-in `hash_state_mixed` after measuring FNV-1a's avalanche.

### Added — elsewhere

- `netinput::AdaptiveDelay` and `DelayScheduler` (1500 Archers, GDC 2001).
- `pathfinding`: `ConnectivityMap`, `farthest_cell`, `combine_maps`, `jps4`.
- `mapgen::MapBuilder` and `Dungeon::keep_largest_region`.
- `wfc::CellSelector`, a pluggable collapse ordering, bit-identical by default.
- `examples/verify_pipeline_demo.rs` — the whole pipeline on one simulation
  with a planted bug, every printed claim asserted.
- `content`: `prefab <name> extends <base>` — a field-level patch overlay for
  `.game` prefabs. Stats merge key-wise (child wins), flags union base-first,
  and `glyph`/`color` override only when the child declares them, so an
  explicit reset is distinguishable from "not mentioned". Resolution is lazy
  via `Content::resolve_prefab`; missing bases and cycles surface as
  `ExtendsError` diagnostics in the validator and as loud load failures.
  `gamec --fmt` keeps the overlay rather than flattening it, so round-trips
  preserve `extends`.
- `observe::Observed<T>` — a `SparseSet` wrapper that pushes `Added`/
  `Replaced`/`Removed` events into an internal queue in program order: the
  push half of change awareness, next to `change::Changed`'s pull. Events are
  data — ordered, hashable, replayable — and the storage's `DetHash` folds
  pending events in, so a forgotten drain surfaces as a desync rather than a
  silent skip. Opt-in; the default path is bit-identical.
- `tests/bench.rs` — a timing harness in the engine's bench style, head-to-
  head on `SparseSet`+`join` vs `arch::ArchTable`. It is what closed N18.

### Changed

- **The crate now leads with what it does.** The headline described where the
  code came from and quoted an engine version that did not exist; the
  crates.io description advertised the commodity half and omitted the eleven
  modules that are the reason to choose this crate.
- **Zero panic paths in shipped code**, compiler-enforced in both crates via
  `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used,
  clippy::panic))]`. The twelve existing sites were rewritten rather than
  allowed; the engine's `States<S>` was restructured so an empty stack is not a
  representable value.
- `Fixed::mul`/`div` proven not to need i128 intermediates (N19 closed).
- N18 (archetype storage for the core) closed by measurement: `ArchTable`
  wins multi-component joins ~16x but loses scattered point lookups ~4x and
  churn ~2x to `SparseSet`, and the join delta is ~0.1% of a 60fps frame —
  not worth an iteration-order semantics change. `ArchTable` stays as the
  opt-in primitive for scan-heavy consumers.
- `geometry::isqrt` was a verbatim copy of `fixed::isqrt_u64`; it now
  delegates. Same output on every `i64` input.

### Removed

- Four superseded audit documents whose headline numbers had become false
  (`STRENGTHS_WEAKNESSES.md`, `FEATURE_AUDIT.md`, `IMPROVEMENTS.md`,
  `PRODUCT_AUDIT.md`). Recoverable from git history.
- `explore::exact_cells` and `temporal::awaiting_response` — public API that
  duplicated something else or had no caller. `temporal::finish_state` is now
  private.
- This repository's git hooks are no longer shipped inside the published crate.

### Fixed — process

The build now checks claims that used to rot silently: documentation facts
(`tests/docs_are_current.rs`), the engine's float boundary
(`izanagi/tests/float_boundary.rs`), that no public function goes unexercised
(`tests/public_api_is_exercised.rs`), and the README's own quickstart, which is
compiled as a doctest.

### Fixed
- **`hud::HudPanel::merge` overflowed for panels at extreme positions**
  (`hud.rs`) — the bounding-box span `x1 - x0` was a raw `i32` subtract that
  panicked when merged panels sat at opposite i32 extremes (`HudPanel` fields
  are public). Computed in `i64` with the span clamped to `u32::MAX`;
  behaviour-identical for any real layout.
- **`SplitMix64::gaussian_approx` overflowed at extreme spread** (`rng.rs`) —
  `spread + 1` panicked at `u32::MAX` and the 4-sample `i32` sum overflowed as
  `spread` approached `i32::MAX` (with `center + result` a further hazard).
  The bound is now `saturating_add(1)` and the accumulation runs in `i64` with
  a final clamp to `i32`. Draws and result are bit-identical for any sane
  spread (the bound only changes at `u32::MAX`), so replays match; PINNED
  hashes unchanged.
- **`geometry::cone` overflowed for extreme facing vectors** (`geometry.rs`) —
  the 45° angle test `2·dot² ≥ |o|²·|f|²` ran in `i64`, but a near-`i32::MAX`
  facing makes `|f|²` approach `i64::MAX` (and `i32::MIN²` summed twice *exceeds*
  it), so both the magnitude and the comparison overflowed even at a tiny
  `range`. Widened `f_mag_sq` and the comparison to `i128`. Behaviour-identical
  for ordinary direction vectors; PINNED hashes unchanged. Extended the
  geometry totality test to sweep extreme facing components.
- **`mapgen::Rect::center` overflowed near the coordinate ceiling**
  (`mapgen.rs`) — `x + w/2` was a raw `u32` add that panicked for a rectangle
  placed near `u32::MAX` (public fields make it reachable). Computed in `u64`
  with a clamp to `i32::MAX`; identical to the naive form for any real map.
  Also widened the private `intersects_padded` to `i64` (same `x + w` class).
- **`mapgen::Rect::area` overflowed for degenerate extents** (`mapgen.rs`) —
  `w * h` was a raw `u32` multiply that panicked in debug for rectangles with
  near-`u32::MAX` extents (the fields are public, so any pair is reachable).
  Switched to `saturating_mul`; `largest_room` now routes through `area()`.
  Internal cellular-automata/region buffers use `usize::saturating_mul` for
  their size to stay panic-free (and wrap-free in release) on 32-bit targets.
- **`pathfinding::octile` / `octile_distance` overflowed at extreme
  coordinates** (`pathfinding.rs`) — the coordinate subtraction
  (`i32::MAX - i32::MIN`) and the `×10` cost scaling both panicked in debug.
  The public pure `octile_distance` is now total: widened to `i64` with a
  saturating clamp to `i32::MAX`, which keeps the heuristic admissible.
  Identical for normal map sizes; PINNED hashes unchanged. Surfaced by the
  systematic robustness sweep.
- **`turn::Scheduler::next_turn` overflowed at extreme energy/speed**
  (`turn.rs`) — `ACTION_COST - energy` panicked when a caller set energy to
  `i32::MIN` via `set_energy`, and `speed * units` panicked for near-`i32::MAX`
  speed. The catch-up computation now runs in `i64` with a saturating clamp
  back to `i32` per actor; the energy deduction uses `saturating_sub`.
  Behaviour-preserving for ordinary speeds/energies; all turn unit tests pass.
- **`Dice::max`/`average_x100` overflowed at extreme counts/sides** (`dice.rs`)
  — `max` computed `count as i64 * sides as i64`, which overflows i64 at
  `u32::MAX × u32::MAX ≈ 1.8e19`; `average_x100` had the same risk with an
  extra `×50` factor. Switched both to `i128` for the product with a final
  clamp back to the result type. Identical for normal dice values; PINNED
  hashes unchanged, all 26 dice unit tests pass. Surfaced by the systematic
  robustness sweep.
- **`TileMap::iter_rect` overflowed on extreme anchor** (`tilemap.rs`) —
  `(rx + rw)` and `(ry + rh)` were raw `i32` adds that panic when `rx` is near
  `i32::MAX`. Switched to `saturating_add` before the `.min(self.width as i32)`
  clamp — behavior-preserving for in-bounds rects, total for extreme anchors.
  All 67 tilemap unit tests pass.
- **`wfc` overflowed on large grid dimensions** (`wfc.rs`) — `wfc_solve` and
  `WfcGrid` indexing computed `width * height` (and `y * width + x`) in `i32`,
  panicking for any grid where the product exceeds `i32::MAX` (e.g. 60000×60000).
  The internal `propagate` BFS likewise added neighbour offsets in raw `i32`.
  Switched the size product to `(width as usize).saturating_mul(height as usize)`
  (matching `passability` and `tilemap`), every flat-index expression to direct
  `usize` arithmetic, and the neighbour adds to `saturating_add` for
  defence-in-depth. Behavior-preserving for normal grids — all 47 wfc unit
  tests pass; PINNED hashes unchanged. Surfaced by the systematic robustness
  sweep — three iterations running, three real bugs found.
- **`InfluenceMap::normalize` overflow on extreme spans** (`influence.rs`) —
  `cur_max - cur_min` overflows `i32` when the source straddles `i32::MIN`/`MAX`;
  the same for `target_max - target_min`; and `(v - cur_min) * target_span` is a
  full-span product that can reach ~`1.8e19` (overflowing `i64` too). Switched
  the spans to `i64` and the rescaling multiply to `i128`, with a final clamp
  back to `i32`. Behavior-preserving for normal-sized influence maps (all 43
  influence unit tests pass). Surfaced by the robustness lens.
- **`Camera` coordinate math overflowed at extreme positions/dimensions**
  (`camera.rs`) — `center`, `world_to_screen`, `world_to_screen_unclamped`,
  `screen_to_world`, `world_rect`, `pan`, and the internal `clamp_origin` all
  did raw `i32` add/sub on `top_left ± screen_{w,h}` and on world-vs-top_left
  deltas, panicking for extreme positions or huge viewports (`u32::MAX` as
  `i32` is `-1`, defeating subsequent `.max(0)` clamps). Switched to
  `saturating_*` for the coordinate adds and to `i64` intermediates inside
  `clamp_origin`/`pan` so the bounds math survives the full range. All 49
  camera unit tests pass; PINNED hashes unchanged.
- **`Color::lerp`/`scale` overflow on large ratios** (`content.rs`) — Both take
  a caller-controlled `i32` `num`/`den`. The per-channel math computed
  `(cb - ca) * num` (lerp) and `c * num` (scale) in `i32`, so a large `num`
  (e.g. brightening by a big factor) overflowed — `255 * i32::MAX` panics in
  debug. Both `den == 0` guards were already present; the overflow was in the
  multiply. Switched the channel arithmetic to `i64` before the `clamp(0, 255)`,
  which is overflow-safe and identical for normal ratios (so PINNED hashes and
  all 39 content unit tests are unchanged). Surfaced by the robustness lens;
  `tests/robustness.rs` gains `color_ops_are_total_at_extreme_ratios`. This is
  the 10th module in the raw-`i32` overflow class, now all saturated/widened.
- **`HudPanel` coordinate helpers overflowed at extreme positions** (`hud.rs`)
  — `inner_x`/`inner_y` (`self.x + 1`), `contains` (`self.x + self.w as i32`),
  and `corners` (the *outer* `self.x + …` add — the inner `saturating_sub` did
  not protect it) overflow-panic for a panel positioned near `i32::MAX`.
  (`inner_w`/`inner_h` already saturated.) All now use `saturating_add`.
  Total/panic-free; all 52 hud unit tests pass; robustness coverage added. This
  closes the raw-`i32`-arithmetic overflow class across the kit's last
  coordinate-bearing module.
- **`terminal` draw ops overflowed coordinates at extreme positions**
  (`terminal.rs`) — `fill_rect`, `draw_str`, `draw_box`, `draw_double_box`, and
  `draw_h_line` computed cell coordinates with raw `x + dx` / `x + w as i32 - 1`,
  which overflow-panic for an origin near `i32::MAX` — despite `draw_box`'s doc
  promising "Fully clipped — no panic for out-of-bounds positions" (the `put`/
  `set` clip is safe, but the *caller* arithmetic panicked first). Dimension
  casts are now clamped to `i32` range and every coordinate offset uses
  `saturating_add`, so a saturated (off-screen) coordinate is harmlessly clipped
  instead of panicking. Honors the documented contract; all 21 terminal unit
  tests pass; robustness coverage added.
- **`autotile` overflow panics at coordinate extremes** (`autotile.rs`) —
  `compute_mask` probed its 8 neighbours with raw `x ± 1` / `y ± 1`, which
  overflow-panic for a cell at `i32::MIN`/`i32::MAX`; `compute_region` iterated
  `x..x + w` / `y..y + h`, whose upper bounds overflow for an origin near
  `i32::MAX`. Both are public. Switched neighbour offsets to `saturating_add`/
  `saturating_sub` (at the coordinate edge the out-of-range neighbour harmlessly
  coincides with the cell) and the region range ends to `saturating_add`. Now
  total; all 26 autotile unit tests pass; robustness coverage added.
- **`PassabilityGrid` overflowed `i32` on large dimensions** (`passability.rs`)
  — `new`/`from_fn` computed the cell count as `(w * h) as usize` — an `i32`
  multiply that panics for `w*h > i32::MAX` — and every index used
  `(y * width + x) as usize`, an `i32` expression that overflows for large
  widths. `tilemap` already does this correctly in `usize`; `passability` was
  inconsistent. Switched the cell count to `(w as usize).saturating_mul(h as
  usize)` and all five index sites to `usize` arithmetic, matching `tilemap`.
  Construction and indexing are now total. A robustness test exercises a
  large-dimension grid; all 39 passability unit tests still pass.
- **`EncounterPack::roll`/`roll_counts` overflow panic at the full count span**
  (`encounter.rs`) — `slot.min + rng.below(slot.max - slot.min + 1)` overflowed
  `u32` when the span equalled `u32::MAX` (a slot with `min = 0, max = u32::MAX`),
  panicking instead of rolling. Replaced both call sites with a `roll_count`
  helper that computes the inclusive span in `u64` and folds it with the same
  wide-multiply `below` uses — so the result is **identical** to the old
  `below(span + 1)` for every representable span (PINNED-safe, all encounter unit
  tests unchanged) and consumes the same single draw, while the full-range case
  no longer panics. Surfaced by extending the robustness lens to the
  `encounter` module (the per-`min==max` audit had missed `min=0, max=MAX`).
- **`SpatialHash` queries no longer iterate the `O(area)` cell span**
  (`spatial_hash.rs`) — Every rect/radius query walked each cell coordinate in
  the query region, so a large area — e.g. a whole-world or huge-radius query,
  reachable via the public API — did `O(span_w × span_h)` work and effectively
  hung, even though the result depends only on the handful of populated cells.
  Added a `for_each_cell_in_rect` helper that picks the cheaper traversal: walk
  the span when it is small, otherwise scan the populated cells and filter,
  sorting the matches into the same `(cy, cx)` order so output is byte-identical
  and deterministic regardless of path. **All five** affected queries now route
  through it — `query_rect`, `query_rect_count`, `query_radius_euclidean`,
  `query_radius_count`, and `count_in_radius_euclidean` (the latter three each
  had their own independent span loop; the radius→rect bound arithmetic
  `qcx ± cr` was also saturated against overflow). Work is now bounded by
  `min(span, populated cells)`. Two complexity-lens tests pin this — a
  full-coordinate-range rect query and a `radius = i32::MAX` query each return
  instantly with the correct keys. PINNED hashes unchanged; all 52 spatial_hash
  unit tests still pass (output preserved exactly).
- **Overflow-panic sites in `aabb` and `spatial_hash`** (`aabb.rs`,
  `spatial_hash.rs`) — Extending the robustness lens to the remaining spatial
  modules surfaced five more raw-arithmetic overflow panics on extreme inputs:
  `Aabb::union` (`r - x`, `b - y`) and `Aabb::intersection` (`min_right - ix`
  before the positivity check) overflow when a box spans the full coordinate
  range → `saturating_sub`; `Aabb::nearest_corner` (`(px - self.x).abs()` ×4)
  overflows for opposite-extreme point/box coords → i64 differences;
  `SpatialHash::query_rect`/`query_rect_count` (`saturating_add(w) - 1`) could
  underflow the trailing `- 1` → `.saturating_sub(1)`; and
  `SpatialHash::query_radius` (`2 * radius + 1`) overflows for large radius →
  `saturating_mul(2).saturating_add(1)`. `Aabb::area` (i64+clamp),
  `right`/`bottom`/`translate` (saturating), and `cell_size.max(1)` were already
  safe. Normal values are unaffected; PINNED hashes unchanged. (A separate
  pre-existing limitation — `query_rect` iterating an O(area) cell span for a
  huge query region — is noted but out of scope for this overflow pass.)
- **Overflow-panic cluster in `combat`** (`combat.rs`) — `heal`, `modified`
  (×3), `base_damage`, `roll_damage`, and `splash_attack` (×3, including a raw
  multiply) used raw `i32` `+`/`-`/`*`, so extreme stat values or modifiers
  triggered "attempt to add/subtract with overflow" panics in debug builds —
  violating the kit's saturating, panic-free, deterministic policy (a panic on
  one peer is the worst desync). Converted all to `saturating_*`. Normal
  gameplay values are unaffected (`saturating_*` ≡ raw there), so both PINNED
  hashes are unchanged. Surfaced by the new robustness lens.
- **`Distance::between` overflow on extreme coordinates** (`geometry.rs`) — The
  squared/Euclidean metrics computed `dx*dx + dy*dy` in `i64`, but `dx` reaches
  `2^32` for full-span coordinates so `dx*dx ≈ 1.8e19` overflows `i64` — and the
  Newton-method `isqrt` separately overflowed on `i64::MAX` (initial `x == n`, so
  `x + n/x` wraps), despite the doc promising saturation. Switched the sum of
  squares to `saturating_*` and replaced `isqrt` with the overflow-safe
  bit-by-bit method (identical floor-sqrt results for in-range inputs). Surfaced
  by the new robustness lens.
- **`Changed::reset` doc stated a false postcondition** (`change.rs`) — The
  `reset` doc claimed "Sets `changed_at` to `tick` so a subsequent
  `is_changed_since(tick)` returns `false`", but `is_changed_since` uses "at or
  after" (`changed_at >= since_tick`), so after `reset(T)` the query
  `is_changed_since(T)` returns `true` — only `is_changed_since(T + 1)` returns
  `false` (as the existing test and the usage-pattern test, which queries with
  `last_processed + 1`, already assert). A user following the doc literally would
  query with `tick`, always see "changed", and re-process every component —
  silently defeating the dirty-flag optimization. Corrected the `reset` and
  `is_changed_since` docs to state the `+ 1` convention; added a workflow test
  pinning the corrected contract (process at `T` → `reset(T)` → query `T+1` is
  false → a later `mark` re-arms). Doc-only behavior change; no API/hash impact.
- **`load_bytes` could panic on a hostile payload length (32-bit)** (`savefile.rs`)
  — The bounds check `data.len() < 20 + payload_len` added a 20-byte header to an
  attacker-controllable `u32` payload length. On a 32-bit target a declared
  length near `u32::MAX` overflows `usize`, wraps to a small value, slips past
  the guard, and then `&data[20..20 + payload_len]` panics on an inverted slice
  range — a panic on malformed input, violating the module's "fail cleanly with
  a `LoadError`" contract. Rewrote the guard as `payload_len > data.len() - 20`
  (the `>= 20` minimum is already enforced above, so the subtraction cannot
  underflow), overflow-safe on every target and equivalent on 64-bit. Three
  boundary tests added (declared len `u32::MAX` → `TooShort` not panic,
  exact-fit decodes, one-past-buffer rejected). No wire-format change; the
  golden-save bytes are unaffected.
- **`SparseSet` ignored entity generation — stale handles aliased recycled slots**
  (`sparse_set.rs`) — `slot()`, the lookup chokepoint behind `get`/`get_mut`/
  `contains`/`remove`/`swap`, keyed only on `entity.index()` and ignored
  `entity.generation()`, even though the full `Entity` was stored in
  `dense_entities`. The generational-handle design exists precisely to reject a
  stale handle to a despawned-then-respawned slot, but the component store
  silently defeated it: after freeing `e0` (idx 0, gen 0) and allocating
  `e1` (idx 0, gen 1) at the recycled index, `set.get(e0)` returned `e1`'s
  component (a use-after-free analog), and `set.get(e1)` could inherit `e0`'s
  leftover component. Fixed by having `slot()` verify
  `dense_entities[pos] == entity` (full equality including generation),
  rejecting any generation mismatch. Live-handle access is unchanged (the
  stored entity equals the query handle), so both PINNED determinism hashes are
  unaffected — verified by the full suite including `test_final_hash_is_pinned`
  and `test_roguelike_final_hash_is_pinned`. Three new tests pin the contract
  (stale read, fresh-handle-at-recycled-index, stale get_mut/remove); verified
  end-to-end — all three fail against the old index-only lookup.
- **`first_diff`/`diff` under-reported vs `content_eq`** (`serializer.rs`) — The
  round-trip oracle `content_eq` compares every field (prefab name/glyph/color/
  stats/flags, tile name/glyph/color, level name/width/height/rows/spawns), but
  the two diagnostic helpers that are documented to explain a `content_eq`
  failure were incomplete: `first_diff` checked no tiles at all and, for levels,
  only `name` + `rows` (skipping `width`, `height`, `spawns`); `diff` likewise
  skipped level `width`/`height`/`spawns`. Two `Content` values unequal under
  `content_eq` — differing only in level dimensions, spawns, or (for
  `first_diff`) tile contents — would return `None`/empty, falsely reporting "no
  difference" and breaking the documented contract ("`None` when they are equal
  under `content_eq`"). A round-trip fuzz failure in those fields would print an
  empty diagnostic. Both helpers now mirror the oracle exactly. Added six tests
  (including an exhaustive per-field mutation check that asserts both diagnostics
  agree with `content_eq`); verified end-to-end — the new tests fail against the
  old helpers. No wire-format or hash impact; PINNED hashes unaffected.
- **`Fixed::mid` doc contradicted its rounding behavior** (`fixed.rs`) — The
  doc claimed the midpoint "rounds toward zero", but the implementation
  `(a + b) >> 1` is an arithmetic shift that floors toward negative infinity
  (e.g. midpoint of raw `-1` and `0` is `-1`, not `0`). Floor is the correct,
  intended convention — it matches `Fixed::floor`/`Fixed::fract` and keeps
  `mid` symmetric — so the fix corrects the doc rather than the code (no
  behavior change, no hash impact). The existing `mid` tests all used even
  sums, leaving the rounding direction unverified; two new tests now pin the
  floor behavior on odd sums and the no-overflow guarantee at the extremes.
- **`MultiMap::det_hash` omits `floors` field** (`multimap.rs`) — The
  `floors: Vec<Dungeon>` field — the entire dungeon stack — was absent from
  `DetHash`, which folded only `current_floor` and `connectors`. Two
  multi-floor stacks with completely different dungeon layouts but matching
  current-floor index and connector lists hashed identically. `floors` is
  mutable simulation state (`current_mut` hands out `&mut Dungeon`, so floors
  are carved/edited at runtime), making this a live desync hole for any
  multi-floor game. Fixed by folding the floor count + each `Dungeon` (in stack
  order) between `current_floor` and the connectors. Three new tests prove:
  (a) different floor layouts now differ, (b) mutating a floor via
  `current_mut` changes the hash, (c) identical-seed stacks still agree.
  Verified end-to-end by reverting the fold and confirming the layout test
  fails. `PINNED_FINAL_HASH` and `PINNED_ROGUELIKE_HASH` are unaffected.
- **`Dungeon::det_hash` omits `rooms` field** (`mapgen.rs`) — The
  `rooms: Vec<Rect>` field was silently absent from `DetHash`, meaning two
  dungeons with identical tile bitmaps but different room registries (e.g.
  different placement order or room boundaries) hashed identically. `rooms`
  is simulation-observable state: game logic that queries room positions for
  spawn placement or center-point navigation would diverge across a replay
  checkpoint without detection. Added `impl DetHash for Rect` (hashing `x, y,
  w, h` in order) and folded `rooms` length + each `Rect` into
  `Dungeon::det_hash`. `Dungeon` already derives `PartialEq` over `rooms`, so
  the hash now agrees with equality. Three new tests prove: (a) appending a
  room changes the hash, (b) identical-seed dungeons still agree, (c)
  different seeds produce different hashes. `PINNED_FINAL_HASH` and
  `PINNED_ROGUELIKE_HASH` are unaffected.
- **`TimerQueue::det_hash` omits `period` field** (`timer.rs`) — The
  `period: Option<u32>` field of each queue entry was silently absent from the
  `DetHash` impl. A one-shot timer (`period = None`) and a recurring timer
  (`period = Some(n)`) with identical `remaining` ticks and event hashed to the
  same value, making two fundamentally different scheduler states
  indistinguishable in replay checksums. Fixed by adding
  `entry.period.det_hash(hasher)` between `remaining` and `event` in the fold.
  Three new tests prove: (a) one-shot vs. recurring timers differ, (b) recurring
  timers with different periods differ, (c) identical queues still agree.
  `PINNED_FINAL_HASH` and `PINNED_ROGUELIKE_HASH` are unaffected (no
  `TimerQueue` in those simulations). No golden hash update required.
- **`range_closed(i32::MIN, i32::MAX)` span overflow** (`rng.rs`) — When
  `lo = i32::MIN` and `hi = i32::MAX`, the closed-range span is `2^32` which
  overflows `u32` to `0`. The old code fell into `below(0)` — which returns `0`
  WITHOUT drawing — silently returning `i32::MIN` regardless of the seed, and
  consuming no draw (breaking the expected draw count). Fixed by computing `span`
  as `i64` and using a 128-bit wide multiply directly (same low-bias technique as
  `below`), handling all spans in `[2, 2^32]` without overflow. Two new tests pin
  the correct draw/distribution behavior; existing consistency test
  (`range_closed(lo,hi) == range(lo,hi+1)`) still passes (same arithmetic path
  for spans that fit u32).
- **Residual per-byte-u32 string encoding in `DetHash` impls** (`profiler.rs`,
  `menu.rs`, `loader.rs`) — Three `DetHash` impls still used the pre-fix
  per-byte `write_u32(*b as u32)` pattern for string fields instead of the
  standard length-prefixed `field.det_hash(hasher)` protocol. Unified to
  use `DetHash for str` everywhere: `Profiler` section names, `Menu` item
  labels (also changed `disabled` from `write_u32` to `write_bool` for
  correctness), and `loader::Stats` stat keys. Also added three new `Menu`
  discrimination tests (label, disabled flag, item count). No golden hash
  updates required (none of these types are in `det_hash_golden.rs`).
- **`StatLine::det_hash` omits `unit` field** (`hud.rs`) — The `unit:
  Option<&'static str>` field was silently absent from the DetHash impl, making
  `StatLine::new("HP", 42)` and `StatLine::with_unit("HP", 42, "points")`
  hash identically. Two distinct HUD states were indistinguishable in replay
  checksums. Also replaced the inefficient per-byte `write_u32(*b as u32)`
  label encoding with the standard `self.label.det_hash(hasher)` (length-prefixed
  via the Round 6 str fix). Four new tests pin the corrected behavior; the
  `StatLine(HP,42)` golden hash is updated
  (`0x5f347e68e7f43f07` → `0xb5df64faa084432d`). `PINNED_FINAL_HASH` and
  `PINNED_ROGUELIKE_HASH` are unaffected.
- **`DetHash for str`/`String` prefix-split collision** (`world_hash.rs`) — Two
  adjacent string fields with no length separator produced identical hashes for
  different field-value splits: `("ab","c")` and `("a","bc")` both wrote the
  raw bytes `[97,98,99]` to the hasher, making distinct game states
  indistinguishable in replay/lockstep checks. Fixed by adding a `write_u32(len)`
  length-prefix in `impl DetHash for str` (mirroring `impl DetHash for [T]`);
  `DetHash for String` now delegates to the `str` impl. `Ability::det_hash` and
  `Affix::det_hash` updated from raw `write_str` to `.det_hash()` to benefit from
  the same invariant.

  **Wire-format break**: `AbilitySet<u32,u32>` golden hash updated
  (`0x9a4f20ee7738fab8` → `0x6be775165615ef30`). `PINNED_FINAL_HASH` and
  `PINNED_ROGUELIKE_HASH` are unaffected (no string fields in those simulations).
  Four new tests pin the encoding contract and prove the collision is eliminated.

### Added
- **`SplitMix64::split(stream_id)`** (`rng.rs`) — named, independent RNG
  sub-streams. Pure function of `(parent state, stream_id)`: doesn't draw
  from or mutate the parent, so forking one child never perturbs a sibling
  regardless of draw order or count. Purely additive; existing draw paths
  and pinned hashes unchanged. 7 unit + 5 property tests.
- **`Xoshiro256pp`** (`rng_xoshiro.rs`, new module) — opt-in xoshiro256++
  generator (Blackman & Vigna 2018): 2²⁵⁶ period vs. `SplitMix64`'s 2⁶⁴, for
  workloads drawing very large volumes of randomness. Isolated from every
  existing draw path (SplitMix64 stays the default); seeded from
  `SplitMix64` per the algorithm authors' recommendation. `jump()` advances
  2¹²⁸ draws for non-overlapping parallel streams. Correctness pinned
  against hand-computed reference vectors, not just self-consistency. 13
  unit + 4 property tests.
- **`world_hash::hash_unordered`** (`world_hash.rs`) — permutation-invariant
  multiset hash: each element hashed via `hash_state`, avalanche-mixed, and
  combined with `wrapping_add` (commutative + associative). Lets a
  non-canonically-ordered container (`HashMap`, arrival-order entities) hash
  to a stable value without sorting first. Multiset semantics (duplicates
  count); purely additive, no existing hash output changes. 7 unit + 4
  property tests.
- **`diag_json::diag_sarif` + `gamec --sarif`** (`diag_json.rs`,
  `bin/gamec.rs`) — SARIF 2.1.0 diagnostic output, the format GitHub Code
  Scanning's `upload-sarif` action consumes for inline PR annotations.
  Mirrors the existing `--json` flag's exit-code contract. 14 tests,
  verified end-to-end by piping real `gamec --sarif` output through a JSON
  parser.
- **Formal EBNF grammar for the `.game` format** (`SPEC.md` §9.1) — derived
  directly from `parser.rs` so every production maps to a real parse arm,
  documenting the exact lexical/boundary rules (line length, dimension
  bounds, color/glyph/int/uint token shapes). Docs only.
- **`Fixed` rounding behavior documented** (`fixed.rs`) — a "Rounding"
  section spells out that `mul` floors toward −∞ (arithmetic `>>`) while
  `div`/`from_ratio` truncate toward zero (integer `/`), diverging only for
  negative operands. Surfaced and documented a `to_int_trunc` misnomer: despite
  its name it floors toward −∞, not truncates toward zero. Behavior
  unchanged; pinned by 4 new regression tests.
- **`#![warn(missing_docs)]` enforced crate-wide** (`lib.rs`) — resolved all
  172 resulting warnings across 39 files (struct fields, enum variants,
  constructor/accessor functions with no doc comment). Docs only.
- **Rust quickstart added to `README.md`** — an entity/`SparseSet`
  component storage + `SplitMix64::split` + full content-pipeline
  (`parse`→`validate`→`load_level`) example; compiled and run as a real
  example before being copied into the README.
- **Cargo workspace formed at the repository root** — the sibling
  `izanagi` engine crate (previously a zip archive at the repo root) was
  extracted into `izanagi/`, and both crates joined a workspace so
  `cargo test --workspace` runs both suites in one command (3362 tests: 188
  engine + 3174 kit). `izanagi/examples/kit_bridge.rs` demonstrates the two
  crates composing: a deterministic roguelike turn loop runs entirely in
  kit types and is rendered through the engine's `Backend`, with the
  per-turn world-hash trace asserted identical between a headless run and
  an engine-hosted run.
- **Packaging metadata corrected** (`Cargo.toml`) — `repository` pointed at
  a nonexistent standalone repo; corrected to the actual monorepo. The
  `"no-std-friendly"` keyword was inaccurate (the crate is `std`-only);
  replaced with `"roguelike"`.
- **Property coverage for `Vec2`/`Vec3` composite ops** (`tests/properties.rs`)
  — Added algebraic laws for the vector operations whose consistency is *not*
  implied by the underlying `Fixed` laws: `dot` commutativity, `len_sq ==
  dot(self,self)` and `len_sq >= 0`, `scale` by `1`/`0`, `cross`(a,a) is the
  zero vector, 2-D/3-D cross antisymmetry, and `normalize(0) == None`. Small
  components keep products/sums clear of saturation so the laws hold exactly.
  Property lens now 20 tests.
- **Property coverage for `Fixed::step_toward`** (`tests/properties.rs`) — Added
  the exact invariants of the gameplay-critical approach function (velocity
  ramps, health drains): the result never leaves `[start, target]` (no
  overshoot), never moves away from the target, a zero step is a no-op, an
  at-target value stays put, and a step covering the whole gap reaches the
  target exactly. `step_toward` clamps rather than interpolates, so these bounds
  hold exactly. Property lens now 18 tests.
- **Property coverage for the `Fixed` rounding family** (`tests/properties.rs`)
  — Added the defining algebraic laws of `floor`/`ceil`/`round`/`fract`, checked
  over thousands of inputs: the exact reconstruction `floor(x) + fract(x) == x`;
  `fract(x) ∈ [0, 1)`; `floor(x) ≤ x ≤ ceil(x)`; `ceil − floor ∈ {0, 1}` and
  equals `0` iff `x.is_integer()`; `round(x) ∈ {floor, ceil}`; and idempotence
  (`floor`/`ceil`/`round` results are integers and fixed under re-application).
  Brings the property lens to 17 tests.
- **Property coverage for easing endpoints** (`tests/properties.rs`) — All 33
  easing curves share a universal contract — `ease(0) ≈ 0` and `ease(1) ≈ 1` —
  that a scaling/offset bug would break uniformly. Added a property test pinning
  the endpoint contract across every easing function (quad/cubic/quart/quint,
  sine, circ, expo, back, bounce, elastic, smoothstep). Property lens now 21 tests.
- **Differential coverage for `Vec2` trig (`rotate`, `angle`, `from_angle`)**
  (`tests/differential.rs`) — These compose `sin_cos`/`atan2` into vector
  operations where a composition bug could hide; validated against `f64`:
  `from_angle(θ) ≈ (cos θ, sin θ)`, `v.angle() ≈ atan2(y, x)`, and `rotate(θ)`
  matches the `f64` rotation matrix (plus length preservation). Small vectors
  for `rotate` keep the magnitude-scaled trig error within an absolute
  tolerance. Differential lens now 12 tests.
- **Differential coverage for `Fixed::atan2`, `hypot`, `pow`**
  (`tests/differential.rs`) — Extended the `f64`-oracle lens to the remaining
  transcendentals. `atan2` (CORDIC vectoring mode) is validated across all four
  `(x, y)` sign quadrants — a quadrant/sign error stays internally consistent yet
  diverges from real `atan2`, so only an independent oracle catches it; proven by
  injecting a disabled `x<0` π-correction and confirming the test fails.
  Measured errors are tiny (`atan2` 6.5e-4, `hypot` 2.2e-4, `pow` 3.2e-4);
  tolerances sit ~3× above. Brings the differential lens to 9 tests.
- **Robustness-lens coverage for `noise`, `passability`, `autotile`**
  (`tests/robustness.rs`) — Extended the totality lens across the procedural and
  grid modules. `noise` (all `value_noise`/`fbm`/`_wrap` variants) was audited
  and confirmed total + bounded to `[0, 65535]` by construction (`u64`
  accumulation, `period.max(1)` guards, `xi0 + 1 ≤ period` so no overflow) — the
  test pins that for extreme coords/octaves/periods. `passability` and `autotile`
  coverage accompanies their `usize`/saturating fixes (see Fixed).
- **Robustness-lens coverage for `easing`, `encounter`, `random_table`,
  `inputbuf`** (`tests/robustness.rs`) — Extended the totality lens to four more
  modules with caller-controlled numeric inputs, hammering them with
  `u32::MAX`/`0`/empty/degenerate values. This converted "clean by inspection"
  into standing guards and immediately caught a real `EncounterPack` overflow
  panic (fixed above) — the others (`easing` over/under the unit interval,
  `random_table` zero/extreme weights and empty tables, `inputbuf` degenerate
  timing) confirmed total.
- **Complexity / work-bound test perspective** (`tests/complexity.rs`) — The
  other nine lenses all ask "is the answer correct?"; none asks "how much work
  does it take?" — yet an algorithm whose cost scales with an incidental
  magnitude (absolute coordinates, query span) rather than its logical input
  (radius, path length, explored region) can hang or DoS a deterministic engine
  while still returning the right answer (the `SpatialHash` huge-span iteration
  is exactly this class). This lens measures work *deterministically* — no flaky
  timing — by instrumenting the predicate closures these algorithms take and
  counting invocations. Six tests assert: FOV work is *identical* regardless of
  absolute origin position and bounded by `O(r²)`; `ray_cast` calls `is_blocked`
  exactly once per traversed cell after the origin (never the origin, never
  twice); and `astar`, `flood_fill`, `is_reachable` explore a position-
  independent, bounded region. Proven to have teeth — making `ray_cast` probe
  the origin fails the tight per-cell bound.
- **Robustness / totality test perspective** (`tests/robustness.rs`) — Verifies
  the most basic contract of a deterministic engine: every public operation is
  *total* (returns — possibly an error, `None`, empty, or a saturated value)
  rather than *panicking*, for all inputs including degenerate and extreme ones.
  A panic is the worst desync (one peer aborts, others continue), so the kit's
  saturating/panic-free policy needs enforcement, not just discipline. The lens
  hammers `Fixed`, `combat`, `Distance`, `rng`, and `geometry` with
  `i32::MIN`/`MAX`, `0`, empty slices, and out-of-range indices. It immediately
  earned its place by surfacing **two** real overflow-panic bugs (the `combat`
  arithmetic cluster and `Distance::between`/`isqrt`), both fixed above.
- **Conservation / accounting test perspective** (`tests/conservation.rs`) — The
  other lenses are about structure and determinism (hashes, laws, models,
  oracles, symmetry, ordering, API surface); none checks a *quantitative
  conservation* invariant — that an operation neither creates nor destroys the
  quantity it manipulates and the books balance. That axis is where
  gameplay-correctness bugs hide: HP healed past its maximum, a leaked or
  double-counted allocator slot, a duplicated/lost inventory item. Three
  invariants over random operation sequences: `EntityAllocator` books balance
  (`total_slots == count + free_count`, `count == live_entities().len()`),
  `Stats` HP is conserved within `[0, max_hp]` (`take_damage` removes exactly
  `min(amount, hp)` and never goes negative, `heal` adds exactly
  `min(amount, max_hp − hp)` and never overheals), and `Inventory` items are
  conserved (held multiset == added − removed, occupancy ≤ capacity, `add`
  succeeds iff not full). Proven to have teeth — dropping the `max_hp` clamp in
  `heal` fails the lens.
- **API-equivalence test perspective** (`tests/api_equivalence.rs`) — The kit
  documents several batch/convenience methods as *exactly equivalent* to a
  primitive sequence (`apply_all` ≡ a loop of `apply`, `batch_free` ≡ a loop of
  `free`, `extend_all` ≡ `extend_duration` on every key, `batch_alloc` ≡ a loop
  of `allocate`). Those are claims, and batch methods are precisely where an
  optimization quietly diverges from the loop it replaces. This lens builds one
  structure via the shorthand and another via the documented primitive sequence
  over the *same* random data and asserts they are indistinguishable — identical
  canonical hash for `DetHash` types, identical observable state (liveness,
  counts, next-allocate) for the allocator. Four equivalences × 500 random trials
  each; the allocator cases include stale/duplicate frees and near-`u32::MAX`
  saturation. Proven to have teeth — making `batch_free` skip one element fails
  the lens.
- **Order-independence (confluence) test perspective** (`tests/order_independence.rs`)
  — Tests the kit's headline determinism promise directly: a *canonical* state
  hash that is identical regardless of the order the operations that built the
  state arrived in (so lockstep/replay stays robust when peers receive events in
  different orders). Several modules sort their entries inside `det_hash` for
  exactly this, but each ships only a single two-element example test. This lens
  builds each structure from the *same set* of operations under 300 random
  permutations and asserts every permutation yields the identical hash, across
  `SparseSet`, `StatusSet`, `SpatialHash`, and `Relations` (operations use
  distinct keys so the final state is genuinely permutation-independent). Proven
  to have teeth — removing the canonicalizing sort from `StatusSet::det_hash`
  makes the lens fail though that module's own example test still passes.
- **`TimerQueue` split-advance invariance** (`tests/metamorphic.rs`) — Added an
  oracle-free metamorphic law for the timer-tick accounting: for one-shot
  timers, the multiset of fired events is invariant under how the total advance
  is divided (`advance(total)` ≡ advancing in chunks summing to `total`), and
  each one-shot with delay ≤ total fires exactly once. (Recurring timers fire
  once per `advance` call by design, so the law is one-shot-only.) Exercises the
  delay accounting without reimplementing `advance` as an oracle.
- **`TileMap` transform group laws** (`tests/metamorphic.rs`) — Added the
  composition/inverse relations of the map transforms: `rotated_cw ∘ rotated_ccw`
  (and the reverse) is the identity, `flip_h`/`flip_v` are involutions, and
  `flip_h ∘ flip_v` equals a 180° rotation (`rotated_cw` twice). These verify the
  transforms compose correctly — two distinct routes must reach the same map —
  beyond the existing rotation multiset / 4×-identity coverage.
- **Metamorphic-relation test perspective** (`tests/metamorphic.rs`) — A lens for
  the kit's richest algorithms (pathfinding, FOV, map transforms) that have *no
  tractable oracle*: you cannot cheaply state "the FOV of this map is exactly
  these cells". Instead of checking an output, it transforms an input by a
  symmetry that should induce a known change in the output and asserts that
  *relation* — surfacing coordinate-dependent or symmetry-breaking bugs with no
  ground truth. Three relations testing the claim "these spatial algorithms
  depend only on relative geometry": pathfinding is translation-invariant
  (reachability and optimal `path_cost` unchanged when the bounded world is
  shifted), FOV is translation-invariant (the visible set relative to the origin
  is unchanged when origin and opacity map move together), and `TileMap` rotation
  permutes cells (value multiset preserved, dimensions swap, 4× rotation is the
  identity). Deterministic via `SplitMix64`. Proven to have teeth — injecting a
  coordinate-dependent cell skip into FOV (which a single-map absolute test would
  shrug off as "fewer cells") fails the translation-invariance relation.
- **Differential-oracle test perspective** (`tests/differential.rs`) — The
  golden, property, and model-based lenses all check the kit against *itself*
  (past bytes, self-relations, a hand-written reference model). This lens checks
  the deterministic integer-only math against an **independent ground truth**:
  `f64`. The no-float rule binds the simulation layer (scanned by
  `tests/no_float_in_sim.rs`), not tests, so `Fixed`'s CORDIC `sin`/`cos`,
  integer `sqrt`, and Q16.16 `mul`/`div` are validated against the true
  real-valued functions within measured tolerances (`sin`/`cos` < 1e-3, `sqrt` <
  1e-4, arithmetic < 1e-4; observed worst-case errors are ~6× tighter). Six
  tests over 50k deterministic inputs each, plus perfect-square exactness and a
  zero anchor. Catches what property laws cannot: a CORDIC sign/quadrant error
  keeps `sin²+cos²≈1` true yet diverges from real `sin`/`cos`, and a wrong shift
  in `mul` keeps commutativity true yet changes the value. Proven to have teeth
  — injecting a `sin`/`cos` swap fails the differential while the property suite
  stays green.
- **Model-based coverage for `Relations`** (`tests/stateful.rs`) — `Relations`
  keeps dual indices (`parents` + `children`) and rejects cycles, so it is prone
  to a parents/children desync or a wrong cycle-rejection. Added a stateful test
  driving 4000 random `attach`/`detach`/`remove_entity` ops against an
  independent forest model, checking `parent_of`/`children_of` after every step
  and predicting `attach`'s accept/reject from a model cycle check. Proven to
  have teeth — making `detach` skip the `children`-index update desyncs the two
  views and fails the test.
- **Model-based coverage for `AssetStore`** (`tests/stateful.rs`) — `AssetStore`
  is generational like `SparseSet` (each slot tracks a generation that
  `get`/`replace`/`remove` verify), and was audited correct — but had no
  *model-based* guard against a future regression dropping the check (the exact
  `SparseSet` use-after-free). Added a stateful test driving 6000 random
  insert/remove/replace/get ops against a reference of every handle ever issued,
  asserting each resolves to `Some(value)` while live and `None` once removed or
  its slot is recycled. Proven to have teeth — dropping the generation check in
  `get` makes a stale handle resolve to a recycled slot and fails the test.
- **Model-based (stateful) test perspective** (`tests/stateful.rs`) — A lens
  distinct from both the example tests and the *stateless* laws in
  `tests/properties.rs`: a structure is driven through a long random sequence of
  operations alongside an independent reference model, and after *every* step is
  asserted to agree with the model and uphold its public invariants. This is
  where history-dependent bugs live — the kind that only surface after a
  particular sequence (free → recycle an index → insert), which neither a fixed
  example nor a single-input law can reach. Two tests: `SparseSet` against an
  *index-slot* model (`index → (owning entity, value)`, encoding the
  generational-match semantics) over insert/remove/get/contains/clear/iter, and
  `EntityAllocator` against a live-set model over allocate/free/batch-free.
  Deterministic via `SplitMix64`. Proven to have teeth: reverting the `SparseSet`
  generational fix makes the model test fail immediately on a recycled-index
  read — i.e. this lens would have caught that bug on its own.
- **Property / metamorphic test perspective** (`tests/properties.rs`) — A new
  verification lens alongside the kit's example-based unit tests and golden-value
  pins. Rather than asserting a specific output for a specific input, each test
  asserts an algebraic **law** that must hold for *all* inputs — and checks it
  over 3000 inputs per law, generated deterministically by the kit's own
  `SplitMix64` (no `proptest` dependency; reproducible in CI). 16 laws across
  `Fixed` (add/mul commutativity, additive/multiplicative identity and
  zero-annihilation, `abs ≥ 0`, double-negation-except-MIN, clamp
  range/idempotence, min/max partition, lerp endpoints, sqrt monotonicity) and
  `geometry` (rotate-4×-identity, cw/ccw inverse, `reflect_point` involution,
  distance symmetry with chebyshev ≤ manhattan, `line` endpoints/length/king-step
  adjacency, knockback never overshoots its budget nor rests in a wall, cone
  cells in-front-and-in-range). Verified the harness has teeth by injecting a
  false law and confirming failure. Catches a whole class of input-space bugs
  that fixed examples never visit.
- **`validate` warns on overlapping spawns** (`validator.rs`) — The validator
  caught every condition that makes `load_level` fail (undefined prefab refs,
  duplicate names, out-of-bounds/grid-mismatch spawns) but said nothing when two
  spawns were authored onto the **same cell** — a frequent machine-generation
  slip the module exists to catch. `validate` now tracks per-level spawn
  occupancy and emits a warning (not an error: the loader intentionally allows
  stacking, e.g. an item on a monster) naming the level, cell, and a prefab.
  Four tests cover the warning, distinct positions staying quiet, three-on-a-cell
  warning twice, and per-level occupancy not colliding across levels.
- **Raw-Unicode parser panic-freedom fuzz** (`tests/content_pipeline.rs`) — The
  existing `test_parser_never_panics_on_garbage` fuzzes only the grammar token
  vocabulary joined by spaces, leaving the tokenizer's char-boundary slicing
  (`&raw[sb..byte_idx]`) and per-character column arithmetic under-exercised on
  truly arbitrary input. `test_parser_never_panics_on_random_unicode` adds 4000
  deterministic iterations of strings built from arbitrary Unicode scalar values
  (many multi-byte), control characters, whitespace variants, and structural
  chars interleaved without separators — asserting `parse` + `validate` always
  terminate without panicking. Mechanizes the "panic-free on any input" contract
  that was previously only vocabulary-deep.
- **`StatusSet::remove_where` — predicate-based cleanse / dispel** (`status.rs`)
  — The removal API was only `remove(&key)` (one) and `clear()` (all), despite
  `count_with` already offering predicate *counting*. Common mechanics — "remove
  all debuffs", "cure all poison", "strip every buff" — forced callers to
  collect matching keys and remove them one by one. `remove_where(pred)` removes
  every effect for which `pred(key, &effect)` is true and returns the removed
  keys in application order, mirroring `tick`'s expired-key convention and
  `sparse_set::remove_where`. Five tests cover match-some (with key order),
  no-match no-op, match-all, key-predicate matching, and equivalence-to-`remove`
  (same surviving effects and canonical hash). Test count 2345 → 2350.
- **`geometry::cone_visible` — wall-aware breath/cone footprint** (`geometry.rs`)
  — The pure `cone` shape ignores walls, so every caller re-derived the
  line-of-sight filtering (and its easy-to-misjudge endpoint semantics) by hand,
  as the targeting integration test had to. `cone_visible(origin, facing, range,
  is_opaque)` returns the cone cells with clear line of sight from `origin`: the
  blast reaches and **includes** the wall cells it strikes (endpoints never
  block) but culls cells shadowed behind a wall. The result is always a subset
  of `cone(...)` in the same order. Uses the kit's single-ray `line_of_sight`
  model; documented to defer to the shadowcasting `fov::fov_circle` for strict
  circular blasts (so it does not duplicate that). Five tests cover the
  no-walls identity, subset invariant, struck-wall-included/behind-culled
  semantics, bad-arg emptiness, and determinism. Test count 2340 → 2345.
- **`geometry::knockback` — forced linear displacement** (`geometry.rs`) — The
  kit had movement (`pathfinding`) and AoE shapes but no forced-displacement
  primitive for knockback, shield bash, telekinesis, or conveyors.
  `knockback(from, dir, distance, is_blocked)` slides an entity up to `distance`
  cells along `dir`, halting on the last open cell **before** the first blocked
  one, and returns the resting cell. `dir` is direction-only (normalized to an
  8-way step via `signum`, magnitude ignored), so it pairs directly with
  `vec_toward(source, target)` to knock a target away from a blast source. The
  origin is never tested; zero `dir` or non-positive `distance` is a no-op.
  Integer-only and deterministic. Eight tests cover full travel, halting before
  a wall, adjacent-wall no-move, no-op guards, magnitude normalization, diagonal
  pushes, `vec_toward` integration, and determinism. Test count 2332 → 2340.
- **`geometry::cone` — directional breath-weapon / cone-spell shape** (`geometry.rs`)
  — The shape library had `circle`, `diamond`, `ring_annulus`, and
  `chebyshev_ring` but no directional cone, despite cone attacks (dragon breath,
  cone of cold) being a roguelike staple. `cone(origin, facing, range)` returns
  the cells of a 90° cone (±45° around `facing`) out to Euclidean `range`, for
  any non-zero integer `facing` (the eight compass steps and beyond). Membership
  is decided with exact integer arithmetic — in front (`o·facing > 0`), within
  ±45° (`2·(o·facing)² ≥ |o|²·|facing|²`), and in range (`|o|² ≤ range²`) — so
  the shape is bit-identical across targets with no trig or float. The origin is
  excluded; cells return in ascending `(y, x)` order. Like the other pure shapes
  it composes with `line_of_sight`/`ray_cast` for wall-aware breath. Nine tests
  cover axis/edge inclusion, range and angle exclusion, mirror symmetry,
  rotation congruence (a 90°-rotated facing yields the rotated cell-set),
  diagonal facings, and determinism. Test count 2323 → 2332.
- **`geometry::ray_cast` / `ray_blocked_at` — bolt/beam tracing** (`geometry.rs`)
  — The kit had the line-of-cells primitive (`line`) and a boolean visibility
  check (`line_of_sight`), and `ability.rs` documents "projectile type", but
  there was no primitive answering *"trace a bolt from A toward B and return the
  cells it travels through and where it stops."* Callers had to re-hand-roll a
  truncated Bresenham walk. `ray_cast(origin, target, is_blocked)` walks the
  Bresenham line and returns the ordered path up to **and including** the first
  blocked cell (the impact point); the origin is always included and never
  tested (a bolt is not absorbed by the shooter's own tile), and an unobstructed
  shot returns the full line through `target`, so `.last()` is always where the
  bolt landed. `ray_blocked_at` is the companion targeting query, returning
  `Some(impact)` for a shot stopped short of `target` and `None` for a clear shot
  (aiming at a wall is a clear shot *to* that wall). Both are deterministic and
  integer-only, re-exported at the crate root, and covered by 11 tests including
  determinism, origin/target edge cases, and agreement with `line_of_sight`.
- **Golden coverage for `TimerQueue`, `Dungeon`, `MultiMap`** (`tests/det_hash_golden.rs`)
  — Extends the wire-format tripwire from 20 to 23 pinned `DetHash` types,
  closing the enforcement gap behind the recent field-omission fixes. The
  per-module unit tests only assert self-consistency and discrimination — both
  survive a *re-omission* of a field, so a future refactor that drops
  `TimerQueue::period`, `Dungeon::rooms`, or `MultiMap::floors` again would go
  unnoticed there. Pinning the exact `hash_state` of a fixed instance of each
  converts that into a mechanical failure: the golden value flips and forces a
  deliberate, changelog-documented decision. Verified end-to-end by dropping
  `period` from `TimerQueue::det_hash` and confirming the golden test fails
  with the expected diff. `PINNED_FINAL_HASH` and `PINNED_ROGUELIKE_HASH` are
  unaffected.
- **Save-file backward-compatibility golden guard** (`savefile::tests`) — pins
  the *exact on-disk bytes* `save_bytes` emits for a fixed `(version, payload)`
  and decodes a *hardcoded* buffer (standing in for a prior build's output).
  Every other savefile test is a same-build round-trip, so a wire-format change
  (field order, endianness, magic, 20-byte header size, checksum algorithm)
  would keep them all green while silently making every existing player save
  unreadable. Three tests now pin the encoder, backward-compatible decoding, and
  the documented field offsets; an `#[ignore]`d `print_golden_save` regenerates
  the fixture for a *deliberate* format bump. Verified end-to-end by flipping the
  version field to big-endian and confirming the encoder test fails.
- **Exhaustive RNG no-draw contract guard** (`rng::tests::test_degenerate_inputs_consume_no_draw_exhaustive`)
  — pins the determinism-critical S3 invariant that *degenerate inputs consume no
  draw* across **every** consuming `SplitMix64` method in one table-driven test
  asserting `state()` is unchanged. Closes 4 methods whose no-draw promise was
  documented but never verified (only the return value was checked):
  `range_closed(lo>=hi)`, `range_u32(lo>=hi)`, `shuffle(len<2)`, `pick_mut(empty)`.
  A refactor that sneaks a `next_u64()` before a guard now fails uniformly (it
  shifts the draw count and desyncs replays while leaving the return value
  intact). Verified end-to-end by injecting a pre-guard draw into `range_u32` and
  confirming the test fails. Serves as the single anchor new consuming methods
  must extend.
- **No-float invariant guard** (`tests/no_float_in_sim.rs`) — makes the
  "no `f32`/`f64` in the deterministic layer" guarantee mechanical instead of
  discipline-only (the `#![forbid(unsafe_code)]` half is already compiler-
  enforced; this is the float half). Scans every production (non-`#[cfg(test)]`)
  line under `src/`, strips line comments, and rejects `f32`/`f64` type tokens
  and decimal float literals — catching e.g. `let speed = dist as f32 * 0.5;`
  that would otherwise compile, pass all tests, and silently break cross-platform
  bit-identity (x87/SSE rounding, FMA contraction). Floats stay allowed in
  `#[cfg(test)]` modules. Verified end-to-end (flags an injected float, passes
  on clean source) plus a `test_scanner_self_check` so the detectors can't go
  vacuous. 2 tests; zero new dependencies.
- **`DetHash` wire-format golden regression guard** (`tests/det_hash_golden.rs`) —
  pins the exact `hash_state` value of a fixed instance of 20 public `DetHash`
  types (`Fixed`, `Entity`, `Vec2/3`, `Aabb`, `Camera`, `Cooldown`, `Dice`,
  `Stats`, `DamageType`, `ResistanceProfile`, `Screen`, `BarWidget`, `StatLine`,
  `HudPanel`, `Relations`, `MsgLog`, `HFsm`, `AbilitySet`, `BehaviorTree`).
  Closes a coverage gap: `tests/determinism.rs` only exercises ~4 types, and the
  per-module `hash(x)==hash(x)` / `hash(x)!=hash(y)` tests survive any wire-format
  change (both sides move together). A reordered field or `write_u32`→`write_u64`
  swap in any pinned type's `det_hash` now flips exactly one golden line, turning
  a silent cross-version replay/save break into a deliberate, reviewable decision.
  Regenerate via `cargo test --test det_hash_golden print_golden -- --ignored
  --nocapture`. 3 tests (pin + name-uniqueness + collision check).
- **New module `hfsm`** (W7 in `STRENGTHS_WEAKNESSES.md`) — hierarchical finite
  state machine superseding the flat `Fsm<S,E>`: states form a tree and event
  resolution walks the ancestor chain before falling back to wildcards. 25 tests.
  - `hfsm::HFsm<S,E>`: builder API (`new` / `with_parent` / `on` / `on_any`);
    runtime API (`set_parent` / `add_transition` / `add_wildcard`); jump API
    (`set_state` / `reset`); query API (`state` / `initial_state` / `is_in` /
    `parent_of` / `ancestors_of` / `ancestors` / `has_transition` / `peek_next`).
  - `fire(&event) -> bool`: searches exact state → parent → grandparent → … →
    wildcard (`on_any`). Returns `true` on state change; `false` for self-loops or
    no match. No `HashMap`; linear scan → identical sequences produce identical
    traces.
  - `is_in(state)`: `true` when the current state *is* `state` or any descendant —
    enables "is the AI in the combat branch?" without enumerating substates.
  - `DetHash` impl folds current state, the full parent table, and all transitions
    (with `None`/`Some` tag on wildcard vs. specific) into the replay checksum.
  - Zero `unsafe`, no float, no external dependencies. Replay-safe.
- **New module `ability`** (G9 in `STRENGTHS_WEAKNESSES.md`) — unified
  ability/skill system connecting mana cost, cooldown, range, and effect into
  a single `try_use` query. 26 tests + doc test.
  - `ability::Ability<E>`: static definition — name, mana cost, cooldown ticks,
    range (0 = self/melee, no range check), and a generic effect payload `E`.
    `DetHash` impl folds all fields including the effect.
  - `ability::AbilitySet<K, E>`: per-entity collection keyed by `K`, tracks
    individual cooldowns. `with(key, ability)` registers (replaces if duplicate).
    `tick(n)` advances all cooldowns (saturating). `is_ready` / `cooldown_remaining`
    / `get` / `len` / `iter` for querying without firing.
  - `ability::AbilityResult<E>`: `Used { effect: E, mana_cost }` (effect is cloned
    out of the set — no borrow conflict, no `unsafe`) or one of three failure
    variants: `OnCooldown { ticks_remaining }`, `InsufficientMana { have, need }`,
    `OutOfRange { distance, max_range }`, `NotFound`.
  - Checks run in order: cooldown → mana → range. Failures leave all state
    unchanged. Zero `unsafe`, no float, replay-safe.
- **Terminal input source abstraction** (W3 in `STRENGTHS_WEAKNESSES.md`) — clean
  integration point between terminal backends and `InputBuffer`. 6 new tests +
  doc test.
  - `inputbuf::KeySource` trait: `next_key(&mut self) -> Option<Key>` — implement
    for crossterm / termion / raw stdin or any other key event source. Non-blocking
    by convention.
  - `inputbuf::ListKeySource<K>`: replays a predetermined key sequence; useful for
    unit tests and replay injection. Provides `remaining()`, `is_exhausted()`,
    `reset()`.
  - `InputBuffer::pump_from<S: KeySource<Key = K>>(&mut self, source)`: drains all
    pending events from a source into the buffer. Call once per frame before
    `tick()`.
- **Save file schema migration** (W4 in `STRENGTHS_WEAKNESSES.md`) — version-aware
  load that upgrades old saves to the current format. 5 new tests + doc test.
  - `savefile::Migrator` trait: `current_version() -> u32` + `migrate(old, bytes)
    -> Result<Vec<u8>, LoadError>` — implement for each schema-breaking change.
  - `savefile::load_bytes_migrated(data, migrator)`: loads and checksum-validates,
    then calls `migrate` if the stored version differs; on success returns a header
    at `current_version` and the migrated payload.
  - `LoadError::MigrationFailed` — new variant when migration cannot proceed.
- **3-component ECS join** (W1 in `STRENGTHS_WEAKNESSES.md`) — `join3` / `join3_mut`
  in `sparse_set`: inner join of three component stores in canonical ascending-index
  order. Iterates the smallest store for O(min) performance. 6 new tests + doc test.
- **Topological transform propagation** (W6 in `STRENGTHS_WEAKNESSES.md`) — visit
  parent→child edges in BFS topological order for world-transform propagation.
  7 new tests (including a 2-level world-position accumulation scenario).
  - `relations::Relations::root_entities() -> Vec<Entity>`: entities that are
    parents of at least one child but have no parent themselves, sorted by index.
  - `relations::Relations::propagate<F>(&self, f: F)`: visits each `(parent, child)`
    edge in BFS topological order (parent always before child; children of the same
    parent are visited in ascending entity-index order for determinism). Caller
    manages the transform lookup — no storage coupling.
- **New module `behavior`** (G8 in `STRENGTHS_WEAKNESSES.md`) — hierarchical
  behavior trees for game AI: sequence/selector/invert/repeat/succeed/fail
  decorators and action/condition leaf nodes. 30 tests + doc tests.
  - `behavior::BehaviorNode<A>`: parameterized over a caller-chosen action ID
    type `A`; leaf logic supplied via closures at evaluation time — keeps the
    tree data-only (serializable, `Clone`-able, `DetHash`-able) with no closures
    stored in the structure.
  - `behavior::BehaviorTree<A>`: wraps a root node; exposes `evaluate(ctx,
    action, condition)`. Evaluation is stack-recursive, zero heap allocation.
  - `behavior::BehaviorStatus`: `Success / Failure / Running` with `DetHash`.
  - `node_count()` and `depth()` for tree metrics and debugging.
  - Full `DetHash` impl: node type (u8 tag), child count, repeat `times`, and
    action identifier all participate — two structurally different trees produce
    different hashes.
- **WFC backtracking + partial result** (W5 in `STRENGTHS_WEAKNESSES.md`) —
  recovers from contradictions instead of failing immediately. 9 new tests.
  - `wfc::wfc_solve_backtrack(width, height, rules, rng, max_backtracks)`:
    chronological backtracking — on contradiction, restores the pre-collapse
    snapshot, forbids the tried tile, and retries. `max_backtracks = 0` is
    identical to `wfc_solve`. Same seed + same limit → same output (deterministic).
  - `wfc::wfc_solve_partial(width, height, rules, rng)`: runs WFC to completion
    or contradiction and always returns a `WfcGrid`. Contradicted cells have
    bitmask `0`, uncollapsed `> 1`, solved `= 1`. Useful for debugging rule
    sets and visualising partial maps.
- **Entity generation overflow detection** (W2 in `STRENGTHS_WEAKNESSES.md`) —
  tracks slot recycling so diagnostics can catch the theoretical stale-handle
  resurrection hazard. 5 new tests.
  - `entity::EntityAllocator::generation_wrap_count() -> u32`: returns the count
    of times a slot's generation counter wrapped from `u32::MAX` back to `0`.
    Non-zero is a red flag in long-running applications; zero is expected for
    any normal game session. The counter saturates at `u32::MAX` (no overflow).

- **New module `affix`** (G7 in `STRENGTHS_WEAKNESSES.md`) — procedural item
  affixes: "Rusty Sword of Dragonslaying" generation over weighted pools.
  14 tests + doc test.
  - `affix::Affix<M>`: name fragment + `AffixSlot` (prefix/suffix) + modifier
    payload `M`; `Affix::prefix` / `Affix::suffix` constructors.
  - `affix::AffixedItem<T, M>`: base item + up to one prefix and one suffix.
    `display_name(base)` composes the full name; `is_magical` / `affix_count` /
    `modifiers()`; for `M = combat::StatsModifier`, `combined_modifier()` sums
    affix modifiers (saturating) for direct use with `Stats::modified`.
  - `affix::AffixGenerator<M>`: weighted prefix/suffix `RandomTable` pools +
    per-slot grant chances. Fixed draw order (prefix coin → prefix roll →
    suffix coin → suffix roll); degenerate chances and empty pools resolve
    without drawing — replay-safe.
  - `DetHash` on `AffixSlot` / `Affix` / `AffixedItem` so enchanted items fold
    into replay checksums.
- **Multi-floor pathfinding & stair linking** (G5 + G6 in
  `STRENGTHS_WEAKNESSES.md`) — route planning across the dungeon stack.
  10 tests.
  - `multimap::MultiMap::find_floor_path(from, to) -> Option<Vec<&Connector>>`:
    shortest connector route via BFS over the directed floor graph. Expands
    connectors in insertion order so equal-length routes tie-break
    deterministically (first-added wins). `Some(vec![])` for same-floor;
    `None` for out-of-range or unreachable; dangling connectors (to
    out-of-range floors) are skipped safely.
  - `multimap::MultiMap::floor_distance(from, to) -> Option<u32>` and
    `is_floor_reachable(from, to) -> bool` conveniences.
  - `multimap::MultiMap::link_floors(a, ax, ay, b, bx, by)`: add a
    bidirectional staircase pair (down-stair + return stair) in one call.
- **New module `encounter`** (G4 in `STRENGTHS_WEAKNESSES.md`) — procedural
  group-encounter rolling: "2–4 goblins, plus a shaman 30% of the time".
  11 tests + doc test.
  - `encounter::EncounterSlot<T>`: value + `min..=max` count range +
    `chance_percent` appearance probability (`max` clamped up to `min` at
    construction).
  - `encounter::EncounterPack<T>`: ordered slots built via `with_slot` /
    `with_optional_slot` / `push_slot`; rolled via `roll` (flat clones) or
    `roll_counts` (`(value, count)` pairs, identical draw sequence).
  - Deterministic draw contract: slots roll in insertion order; degenerate
    chances (0 / ≥ 100) and fixed counts (`min == max`) resolve **without**
    drawing, mirroring `SplitMix64::coin` / `range_u32` semantics.
  - `min_spawns()` / `max_spawns()` bounds; `DetHash` folds the slot
    configuration in insertion order (gated on `T: DetHash`).
- **Status ↔ combat integration** (G2 in `STRENGTHS_WEAKNESSES.md`) — timed
  buffs/debuffs now fold directly into the combat formula. 9 tests + doc test.
  - `status::StatTarget` enum (`Attack`/`Defense`/`MaxHp`): which `combat::Stats`
    field an effect modifies.
  - `status::StatusSet::stats_modifier(target_of) -> combat::StatsModifier`:
    sums active magnitudes per target (saturating); keys mapped to `None`
    (e.g. DoTs) are skipped. Compose with `Stats::modified`.
  - `status::StatusSet::dot_total(is_dot) -> i32`: per-tick damage total for
    poison/burn/bleed effects, clamped ≥ 0 (a stray negative DoT can never
    heal). Apply via `Stats::take_damage` once per turn.
- **Nested loot/encounter tables** (G3 in `STRENGTHS_WEAKNESSES.md`) — two-tier
  "roll the category, then roll within it" pattern. 6 tests + doc test.
  - `random_table::RandomTable::roll_nested(rng) -> Option<&U>` (for
    `RandomTable<RandomTable<U>>` via a new `AsRef` impl): outer roll picks an
    inner table by weight, inner roll yields the value. Draw counts are
    deterministic: empty outer = 0 draws; non-empty outer = 1 draw + 1 inner
    draw (inner empty = `None` after the single outer draw).
  - `random_table::RandomTable::roll_nested_owned(rng) -> Option<U>`: cloned
    variant dropping the borrow.
- **New module `damage`** — typed damage and per-type resistance profiles,
  closing the "no damage typing / no resistances" combat gap (G1 in
  `STRENGTHS_WEAKNESSES.md`). Integer-only, deterministic, `DetHash`-able
  (replay-safe; pinned hashes unchanged). 19 tests.
  - `damage::DamageType` enum (`Physical/Fire/Cold/Lightning/Poison/Arcane/True`),
    `#[repr(u8)]` with stable `ALL` ordering, `index`/`from_index`, `DetHash`.
  - `damage::ResistanceProfile`: fixed-size `[i32; 7]` of per-type resistance %
    (no `HashMap` → no ordering non-determinism). `new`/`uniform`/`with` builder,
    `get`/`set`/`add` (saturating), `is_immune`/`is_vulnerable`.
  - `ResistanceProfile::apply(damage, ty)`: `True` bypasses; otherwise
    `max(0, dmg × (100 − resist) / 100)` with resist clamped at 100 and negatives
    honoured as vulnerability (`i64` intermediate, no overflow). Verified to match
    `combat::apply_resistance` for the 0..=100 range.
- **New document `STRENGTHS_WEAKNESSES.md`** — product-level strategic analysis
  (7 strengths, 7 weaknesses, 9 missing features with effort estimates), indexing
  which gaps to implement next. Complements `RESEARCH.md` (external-source survey)
  and `IMPROVEMENTS.md` (bug-fix log).
- `aabb::Aabb::is_square() -> bool`: `true` when non-empty and `w == h`.
  Simple predicate for symmetric-region checks and room-template validation (3 tests).
- `aabb::Aabb::nearest_corner(px, py) -> (i32, i32)`: the box corner closest to
  the given point. Equidistant ties prefer left/top. Useful for snap-to-corner
  placement and minimum-separation geometry (3 tests).
- `sparse_set::SparseSet::any(pred) -> bool`: short-circuit predicate test over
  component values. Non-allocating "does anyone have X?" check (3 tests).
- `passability::PassabilityGrid::count_neighbors_passable(x, y) -> usize`:
  complement of `count_neighbors_blocked`; OOB counts as blocked in both (3 tests).
- `tilemap::TileMap::all_where(pred) -> bool`: dual of `any_where`; `true` when
  every cell satisfies the predicate (3 tests).
- `tilemap::TileMap::fill_row(y, tile)`: fill an entire row with one value.
  Out-of-bounds `y` is silently ignored (3 tests).
- `world_hash::Fnv1a::write_u8(value: u8)`: hash a single byte. Fills the gap
  between `write_bytes` and `write_u16` (3 tests).
- `world_hash::Fnv1a::write_i16(value: i16)`: hash a signed 16-bit integer in
  LE byte order; same bits as `write_u16` produce the same hash (3 tests).
- `fixed::Fixed::mid(other) -> Fixed`: midpoint `(self + other) / 2` using i64
  intermediate to avoid overflow (3 tests).
- `vec::Vec2::cross_2d(rhs) -> Fixed`: 2-D pseudo-cross product
  `x * rhs.y − y * rhs.x`. Sign indicates turn direction (3 tests).
- `vec::Vec2::mid(other) -> Vec2`: per-component midpoint via `Fixed::mid` (3 tests).
- `vec::Vec3::is_zero() -> bool`: `true` when all three components are zero (3 tests).
- `vec::Vec3::min_component() / max_component() -> Fixed`: per-axis extremes,
  mirrors Vec2 equivalents (3 tests each).
- `vec::Vec3::mid(other) -> Vec3`: per-component midpoint (3 tests).
- `cmdqueue::CmdQueue::peek_back() -> Option<&C>`: reference to the most
  recently pushed command without consuming it. Mirrors `peek_mut` for the back
  end — useful for "what did the player just queue?" checks (3 tests).
- `cmdqueue::CmdQueue::truncate(n: usize)`: keep only the first `n` commands,
  discarding the rest. Equivalent to a hard input-buffer cap; `truncate(0)` is
  `clear` (3 tests).
- `inventory::Inventory::filled_slots() -> Vec<usize>`: indices of all occupied
  slots in ascending order. Saves callers from filtering `iter()` when only
  positions are needed (3 tests).
- `inventory::Inventory::count_occupied() -> usize`: count of non-empty slots.
  Cleaner than `count_where(|_| true)` at call sites (3 tests).
- `rng::SplitMix64::range_closed(lo, hi) -> i32`: uniform draw from the closed
  interval `[lo, hi]`. More natural for "d20 roll" semantics than `range(1, 21)`
  (3 tests).
- `msglog::MsgLog::is_full() -> bool`: `true` when `len == capacity`, i.e. the
  next push will evict the oldest message. Useful for UI overflow indicators (3 tests).
- `turn::Scheduler::all_actors() -> Vec<A>`: collect all registered actor ids
  into a `Vec`. Convenience wrapper over `iter_actors().collect()` (3 tests).
- `geometry::rotate_90_cw(x, y) -> (i32, i32)` / `rotate_90_ccw(x, y) ->
  (i32, i32)`: 90° rotation in screen coordinates (y-down). CW maps
  Right→Down→Left→Up; CCW is the inverse. Integer-only, replay-safe (4 tests
  each; also re-exported from `izanagi_kit::` top level).
- `fixed::Fixed::step_toward(target, step) -> Fixed`: advance toward `target`
  by at most `|step|`; saturates, never overshoots. Useful for velocity ramps
  and AI approach without manual clamp branches (4 tests).
- `fixed::Fixed::in_range(lo, hi) -> bool`: closed-interval membership check
  (`self >= lo && self <= hi`); clarifies complex call sites (3 tests).
- `vec::Vec2::min_component() / max_component() -> Fixed`: smallest / largest
  component. Useful for aspect-ratio clamping and Chebyshev-distance helpers
  without destructuring the vector (4 tests).
- `mapgen::Dungeon::room_at(index) -> Option<&Rect>`: O(1) named room accessor
  by placement index (2 tests).
- `mapgen::Dungeon::room_containing(x, y) -> Option<Rect>`: returns the first
  room that encloses the world point; negative coordinates return `None` (2 tests).
- `fixed::Fixed::pow2(self) -> Fixed`: squared value (`self.mul(self)`). Saves
  the `x.mul(x)` pattern in squared-distance checks and quadratic formulas.
- `pathfinding::path_to_direction_vec(path) -> Vec<(i32, i32)>`: convert a
  waypoint list to unit-direction steps. Each `(a, b)` pair yields
  `(sign(dx), sign(dy))`. Empty/single-point paths return empty Vec.
- `spatial_hash::SpatialHash::all_occupied_cells() -> Vec<(i32, i32)>`:
  collect all non-empty cell coordinates. Useful for "rescan all regions" and
  debug visualisation without traversing all world entities.
- `world_hash::Fnv1a::write_bool(value: bool)`: explicit bool writer (0/1 byte).
  Completes the write-family; avoids `as u8` casts in `DetHash` impls.
- `loader::LoadedLevel::entities_in_rect(x, y, w, h) -> Vec<Entity>`: all
  entities whose position lies within `[x, x+w) × [y, y+h)`. Avoids building
  a separate spatial index for "all actors in this room" queries.
- `msglog::MsgLog::drain_oldest(n: usize) -> Vec<String>`: remove and return up
  to `n` oldest messages in FIFO order, advancing the ring-buffer head. Use for
  log-rotation ("save recent messages only") without allocating a full snapshot.
- `keymap::KeyMap::all_actions() -> Vec<A>`: collect all distinct actions that
  have at least one key bound. Complement of `action_count()` that returns owned
  values rather than a count. `A: PartialEq` bound required.
- `menu::Menu::value_at(idx: usize) -> Option<&T>`: payload reference at index.
  Symmetric complement of `label_at()`; avoids going through `select()` when
  a renderer needs the data for a specific row.
- `hud::HudPanel::corners() -> [(i32, i32); 4]`: top-left, top-right,
  bottom-left, bottom-right corners. Avoids recomputing `x + w - 1` at every
  call site when drawing box characters.
- `geometry::reflect_point(point, center) -> (i32, i32)`: reflect a point
  through a center: `(2·cx − px, 2·cy − py)`. Saturating i64 arithmetic prevents
  overflow. Useful for symmetric dungeon layouts and mirror-image rooms.
- `content::Color::min_channel(self) -> u8`: minimum of the three RGB channels.
  Complement of `max_channel()`. Useful for computing color saturation
  (`max - min`) and shadow/darkness threshold checks.
- `influence::InfluenceMap::add_map(other: &InfluenceMap)`: pixel-wise addition
  shorthand for `combine(other, 1, 1)`. Clears boilerplate when compositing
  equal-weight influence layers (threat + hunger, etc.). No-op on size mismatch.
- `fov::fov_count_filtered(origin, radius, is_opaque, pred) -> usize`:
  allocation-free count of visible cells matching a predicate. Avoids the
  intermediate `Vec` of `fov_to_vec(...).filter(...)`. Use for "how many hostile
  cells are in view?" queries.
- `turn::Scheduler::pending_count() -> usize`: count of actors whose energy has
  not yet reached `ACTION_COST`. Complement of `actors_ready().len()` without
  the allocation. Useful for UI countdowns and AI planning early-exit.
- `noise::fbm_2d_in_range(x, y, seed, octaves, lo, hi) -> i32`: combines
  `fbm_2d` + `normalize_noise` in one call. Eliminates the two-step pattern
  for range-mapped terrain generation.
- `aabb::Aabb::iter_border() -> impl Iterator<Item = (i32, i32)>`: iterate
  only the perimeter cells of the bounding box (top row → bottom row → left
  column interior → right column interior). Useful for placing walls, rendering
  outlines, or scanning edge cells without visiting the interior. Empty boxes
  yield nothing; single-row/column boxes behave like `iter_points`.
- `tilemap::TileMap::iter_rect(rx, ry, rw, rh) -> impl Iterator<Item = (i32, i32, &T)>`:
  iterate tiles in a rectangular sub-region in row-major order. Cells outside
  the map are silently skipped. Allocation-free alternative to `iter().filter()`
  for fog-of-view updates or local A* searches.
- `tilemap::TileMap::enumerate_row(y) -> impl Iterator<Item = (i32, &T)>`:
  yields `(x, &tile)` for every cell in row `y`. Unlike `row_slice`, includes
  the column coordinate — useful for wall-detection or corridor scanning.
  Returns empty iterator for out-of-bounds `y`.
- `sparse_set::SparseSet::drain() -> Vec<(Entity, T)>`: take all entries and
  clear the set in one call. Equivalent to collecting `iter()` then calling
  `clear()`, but avoids cloning by taking ownership. Useful for batch-despawn
  after a frame without a separate collect+remove loop.
- `cmdqueue::CmdQueue::peek_mut() -> Option<&mut C>`: mutable reference to the
  first command without popping. Lets callers modify a pending command in-place
  (e.g. update a target position) without the overhead of pop + modify + prepend.
- `rng::SplitMix64::from_u32_pair(lo: u32, hi: u32) -> Self`: construct from
  two independent `u32` seeds combined as `lo | (hi << 32)`. Avoids manual
  bit-shifting at call sites that hold a map seed and a run counter separately.
- `inventory::Inventory::remove_where_indexed(pred) -> Option<(usize, T)>`:
  like `remove_where` but also returns the slot index. Use when the UI or log
  needs to report which slot was consumed without a preceding `find` round-trip.
- `passability::PassabilityGrid::count_neighbors_blocked(x, y) -> usize`:
  count of orthogonal (4-direction) blocked neighbours. Out-of-bounds count as
  blocked. Result in `0..=4`. For cellular-automaton rules, connectivity checks,
  and maze-generation CA transitions without allocating a list.
- `vec::Vec2::lerp_clamped(a, b, t)` / `vec::Vec3::lerp_clamped(a, b, t)`:
  component-wise linear interpolation with `t` clamped to `[0, 1]` via
  `Fixed::clamp01`. Use when `t` may exceed the range (user input, animation
  clocks) and extrapolation beyond `[a, b]` is undesired.
- `tilemap::TileMap::row_slice(y: i32) -> Option<&[T]>`: borrow a row as a
  contiguous slice. Row-major layout makes this O(1); `None` for OOB rows.
- `tilemap::TileMap::column_vec(x: i32) -> Vec<T>`: collect column `x` into a
  `Vec` (top-to-bottom). Columns are strided in row-major memory so this
  always allocates; empty `Vec` for OOB columns.
- `sparse_set::SparseSet::find_all_entities(pred) -> Vec<Entity>`: collect
  all entities whose component satisfies `pred`. The multi-result complement
  of `find_entity_where`. Use `iter_sorted` on the result for canonical order.
- `sparse_set::SparseSet::remove_where_returning(pred) -> Vec<(Entity, T)>`:
  like `remove_where` but returns the removed `(entity, value)` pairs instead
  of discarding them — use for "cull dead actors and process their components
  in despawn callbacks."
- `entity::EntityAllocator::live_at_index(index: u32) -> Option<Entity>`: O(1)
  slot lookup with free-list liveness check. Fills the gap between
  `live_entities()` (full scan) and `is_alive(entity)` (needs a handle) for
  serialisation and debug tools that have an index but not a handle.
- `spatial_hash::SpatialHash::clear_at(x, y) -> usize`: remove all keys from
  the cell containing world position `(x, y)`. Returns the count removed; 0
  if the cell was empty. Faster than calling `remove` per key when the entire
  cell needs to be wiped (e.g. room reset, trap triggered).
- `fov::can_see(origin, target, radius, is_opaque) -> bool`: single-cell
  visibility query without materialising the full FOV set. Uses the same
  symmetric shadowcasting as `compute_fov`; result is identical to
  `fov_to_vec(…).contains(&target)` but avoids the allocation. Replay-safe
  (integer-only, deterministic).
- `pathfinding::is_path_clear(path, is_blocked) -> bool`: validate a cached
  path against the current map state. Returns `true` iff every cell in `path`
  is passable. Short-circuits on the first blocked cell. Empty paths are
  considered clear. Use before following a stale A* path to avoid stepping
  through newly-closed doors or actors.
- `influence::InfluenceMap::normalize(target_min, target_max)`: linearly
  rescale all cell values into `[target_min, target_max]`. When all cells are
  equal (span = 0) every cell is set to `target_min`. No-op on empty maps.
  Integer arithmetic; no float. Replay-safe.
- `multimap::MultiMap::move_down() -> bool`: advance to the next (deeper)
  floor. Returns `true` on success, `false` when already on the last floor.
- `multimap::MultiMap::move_up() -> bool`: return to the previous (shallower)
  floor. Returns `true` on success, `false` when already on floor 0.
- `camera::Camera::distance_to_edge(wx, wy) -> i32`: minimum signed distance
  from a world-space point to any viewport edge. Positive = inside the
  viewport by that many cells; 0 = on an edge; negative = off-screen.
  Deterministic integer arithmetic; safe to hash.
- `wfc::WfcGrid::solved_count() -> usize`: count of fully-collapsed cells.
  Shorthand for `len() - count_uncollapsed()`. Useful for "WFC is N% done"
  progress indicators and partial-result consumers.
- `assets::AssetStore::count_by(pred) -> usize`: allocation-free count of
  assets satisfying `pred`. Avoids the `find_all_by(pred).len()` pattern.
- `assets::AssetStore::any_by(pred) -> bool`: short-circuit existence check.
  Equivalent to `find_by(pred).is_some()` but without the `Option<Handle>`.
- `geometry::manhattan_distance(a, b) -> i32`: `|dx| + |dy|`. Named shorthand
  for `Distance::Manhattan.between(a, b)` for use without the enum.
- `geometry::chebyshev_distance(a, b) -> i32`: `max(|dx|, |dy|)`. Named
  shorthand for `Distance::Chebyshev.between` — "king moves" range checks.
- `relations::Relations::has_children(entity) -> bool`: shorthand for
  `child_count(entity) > 0`. Avoids spelling out the comparison at call sites
  that only need "is this a leaf?" without allocating a children list.
- `hud::BarWidget::is_empty() -> bool`: `current <= 0`. The complement of
  `is_full` — useful for "out of mana/HP/fuel" checks and disabling UI elements
  when a resource is fully depleted.
- `profiler::EventLog::since_tick(start_tick: u32) -> impl Iterator`: iterate
  all entries at or after `start_tick`, oldest first. Convenience shorthand for
  `filter_by_tick_range(tick, u32::MAX)`.
- `random_table::RandomTable::remove_at(idx: usize) -> Option<T>`: remove an
  entry by index and return its value. `total_weight` is updated. Entries after
  `idx` shift left. Combined with `weighted_idx`, enables "roll-and-remove"
  without-replacement loot draws.
- `cmdqueue::CmdQueue::pop() -> Option<C>`: FIFO alias for `pop_front`. Reads
  more naturally at typical consumption sites where LIFO/FIFO semantics do not
  need to be highlighted.
- `fsm::Fsm::reset()`: return the FSM to its initial state (the state passed to
  `Fsm::new`) without touching the transition table. Also adds
  `Fsm::initial_state() -> &S` to read it back. Stores the initial state as a
  new private `initial` field (no public API breakage). Useful for "respawn"
  patterns where an actor reverts to its original AI state.
- `msglog::MsgLog::iter_rev() -> impl Iterator<Item = &str>`: iterate messages
  in newest-to-oldest order — the reverse of `iter()`. Useful for terminal UIs
  that draw from the bottom of the screen upward without building a temporary
  reversed `Vec`.
- `change::Changed::into_value(self) -> T`: consume the wrapper and return the
  inner value, discarding the change tick. The move-semantics complement of
  `.value` for contexts where the dirty-flag metadata is no longer needed (e.g.
  removing a component, serialising a final snapshot).
- `aabb::Aabb::split_v(x) -> (Aabb, Aabb)`: split an AABB vertically at world-x
  `x`, returning `(left, right)`. Widths sum to the original. One half is empty
  if `x` is outside the box. Together with `split_h`, forms the BSP primitive
  used in recursive dungeon partitioning.
- `aabb::Aabb::split_h(y) -> (Aabb, Aabb)`: split horizontally at world-y `y`,
  returning `(top, bottom)`. Heights sum to the original.
- `mapgen::Rect::shrink(n) -> Option<Rect>`: inset a room rectangle by `n` cells
  on every side. Returns `None` when the inset would produce zero or negative
  width or height (`n*2 >= w` or `n*2 >= h`). Useful for spawn zones and
  interior decoration placement that must stay away from room walls.
- `mapgen::Dungeon::random_floor_cell(rng) -> Option<(i32, i32)>`: pick a
  uniformly random floor cell using a single `rng` draw. Returns `None` for
  all-wall dungeons. Deterministic and replay-safe; the canonical one-liner for
  "place an item / monster on a random walkable tile."
- `timer::TimerQueue::iter() -> impl Iterator<Item=(u32, &E)>`: iterate all
  pending entries as `(remaining_ticks, &event)` pairs in insertion order
  without consuming or advancing the queue. Useful for UI countdowns, save/load
  serialisation, and test assertions.
- `status::StatusSet::apply_all(effects: &[(K, Effect)])`: batch-apply multiple
  status effects. Equivalent to calling `apply` for each pair in order — same
  max-duration stacking policy. Useful for equipping items and area spells that
  apply several buffs/debuffs at once.
- `status::StatusSet::extend_all(extra_ticks: u32)`: add `extra_ticks` to every
  currently active effect. Saturating on overflow. The "haste / duration boost"
  primitive that avoids iterating the set manually.
- `turn::Scheduler::clear()`: remove all actors without returning their ids.
  Cheaper than `drain` when the id list is not needed (full scene reset). The
  scheduler is empty after the call.
- `combat::Stats::clamp_hp()`: force HP into `[0, max_hp]`. The "repair"
  primitive for HP values set by direct field assignment (save/load, editor
  operations) that may be out of range.
- `world_hash::DetHash for [T]` / `Vec<T>` / `Option<T>`: the three most
  commonly needed collection/wrapper impls were missing. `[T]` folds length
  then elements in order (length-prefix prevents the empty-slice / default-item
  collision); `Vec<T>` delegates to `[T]` so both produce identical hashes;
  `Option<T>` folds a `0` tag for `None` and `1 + value` for `Some`, preventing
  `None` from hashing identically to `Some(default)`. These are critical for any
  aggregate type that stores a `Vec` of components or an optional field as part
  of its `DetHash` impl. Re-exported via the existing blanket trait.
- `sparse_set::SparseSet::swap(a, b) -> bool`: exchange components of two
  entities in place. Returns `true` when both are present; `false` (no change)
  if either is absent; O(1) via the sparse index. Useful for inventory item
  trades and permutation-based sort passes without a temporary staging variable.
- `entity::EntityAllocator::batch_alloc(n) -> Vec<Entity>`: allocate `n`
  entities in one call. Convenience wrapper over `(0..n).map(allocate)` — named
  for clarity at bulk-spawn sites ("spawn 20 enemies at once"). The returned
  entities are all live and have distinct indices.
- `noise::value_noise_3d(x, y, z, seed) -> u32`: trilinear-interpolated 3-D
  value noise in `[0, 65535]`. Extends the 1-D/2-D value noise family to three
  dimensions using 8 corner hashes from `hash_3d` and the same cubic Hermite
  smoothstep. Useful for voxel terrain density, layered cave systems, and
  time-varying procedural generation.
- `noise::fbm_3d(x, y, z, seed, octaves) -> u32`: fractional Brownian motion
  in 3-D over `value_noise_3d`. Mirrors `fbm_2d`; `octaves == 0` returns `0`.
  Re-exported at the crate root alongside the other 3-D noise functions.
- `noise::noise_3d_in_range(x, y, z, seed, lo, hi) -> i32`: convenience
  combinator matching `noise_2d_in_range`. Re-exported at the crate root.
- `rng::SplitMix64::gaussian_approx(center, spread) -> i32`: Bates-distribution
  (Irwin-Hall with 4 samples) bell-curve approximation. Draws 4 values from
  `[0, spread]`, averages them, and centres at `center`. Output range
  `[center − spread, center + spread]`. Consumes exactly 4 draws for `spread >
  0`, 0 for `spread == 0`. Deterministic and replay-safe; useful for damage
  variance, difficulty curves, and procedural stat generation.
- `tilemap::TileMap::flip_h()`: mirror horizontally (each row reversed) in
  place. Dimensions unchanged. Standard room-template reflection primitive.
- `tilemap::TileMap::flip_v()`: mirror vertically (row order reversed) in
  place. Dimensions unchanged.
- `tilemap::TileMap::rotated_cw() -> TileMap<T>`: return a new map rotated 90°
  clockwise; the new dimensions are `height × width`. Useful for placing a
  room template in any of four orientations without pre-generating four variants.
- `tilemap::TileMap::rotated_ccw() -> TileMap<T>`: symmetric 90°
  counter-clockwise rotation; same dimension swap as `rotated_cw`.
- `lib.rs`: re-export `fbm_3d`, `hash_3d`, `noise_3d_in_range`, `value_noise_3d`
  at the crate root, completing the 3-D noise API surface.
- `rng::SplitMix64::sample_n<T: Clone>(slice, n) -> Vec<T>`: partial Fisher-Yates
  without-replacement sampling — returns exactly `min(n, slice.len())` distinct
  elements in a uniformly random order. Draws one value per selected element, so
  the draw count is deterministic: `min(n, len)` draws. Empty slice or `n == 0`
  returns `Vec::new()` without drawing. The canonical "pick k items from a pool"
  primitive for loot tables, draft mechanics, and random encounter rosters.
  Replay-safe.
- `pathfinding::nearest_reachable<P, F>(start, is_passable, pred) -> Option<(i32,i32)>`:
  BFS to the nearest passable cell satisfying `pred`, starting from `start`.
  Returns `None` when `start` is not passable, or when no reachable cell matches
  the predicate. Uses the same 8-directional DIRS compass order and no-corner-
  cutting rule as `flood_fill`. Useful for "find the nearest exit / altar /
  healing station" AI and spawn-placement queries. Re-exported at the crate root.
- `combat::StatsModifier { attack, defense, max_hp }`: additive stat modifier
  struct. `Stats::modified(&self, modifier: &StatsModifier) -> Stats` applies
  the modifier to a snapshot: adds each field, clamps `max_hp` at 0, and clamps
  `hp` to the new ceiling. Pure function — does not mutate the receiver.
  Deterministic and replay-safe; suitable for buff/debuff preview and item
  stat sheets. Re-exported at the crate root.
- `combat::splash_attack(attacker, targets, falloff) -> Vec<i32>`: area-of-effect
  attack — delivers `max(1, attack − falloff×i − defense)` damage to each target
  in `targets` (index 0 = primary hit, `i ≥ 1` = splash), returning the damage
  dealt per target. Applies damage in place via `take_damage`. No RNG involved —
  deterministic given fixed inputs. Re-exported at the crate root.
- `lib.rs`: re-export `apply_resistance`, `splash_attack`, `StatsModifier`
  (combat) and `is_reachable`, `nearest_reachable`, `octile_distance`, `path_cost`
  (pathfinding) at the crate root — closing four previously unexported public
  functions.
- `assets::AssetStore::remove_where<F>(pred) -> usize`: bulk-remove all assets for
  which `pred(&asset)` returns `true`. Returns the count removed. Simpler than
  `retain` for "remove all expired / dead items" patterns that don't need the handle.
- `camera::Camera::screen_center() -> (u32, u32)`: integer midpoint of the viewport
  `(screen_w/2, screen_h/2)`. Useful for centering HUD elements, computing radial
  spawns, and "where is the middle of the screen?" queries.
- `camera::Camera::clamp_world_to_screen(wx, wy) -> (u32, u32)`: convert world
  coords to screen coords, clamping off-screen points to the nearest edge pixel
  instead of returning `None`. Useful for "draw an arrow toward an off-screen target."
- `spatial_hash::SpatialHash::query_radius_count(cx, cy, radius) -> usize`:
  allocation-free Chebyshev-radius count (the count-only complement to `query_radius`).
  Returns 0 for negative radius. Mirrors `count_in_radius_euclidean` for the
  square-radius metric.
- `textlayout::indent(s, n) -> String`: prefix `s` with `n` ASCII spaces. The
  minimal indentation primitive — combined with `wrap_words` for indented dialogue
  boxes and nested list rendering.
- `textlayout::is_blank(s) -> bool`: `true` when `s` is empty or contains only
  ASCII whitespace. The complement of "has content" — useful for filtering empty
  lines from wrapped text and skipping blank tooltips.
- `savefile::LoadError::message() -> &'static str`: short static description of the
  error for logging. Avoids `format!("{}", err)` when only a static string is
  needed; no allocation.
- `diag_json::has_errors(diags) -> bool`: `true` if any diagnostic is
  error-severity. Thin wrapper around `diag_count().0 > 0` for CI abort guards that
  only need a boolean.
- `arch::ArchTable::count_where<F>(pred) -> usize`: count rows where
  `pred(entity, &row)` is true. Allocation-free complement to `iter().filter().count()`
  boilerplate. Useful for "how many enemies have > 50 HP?" queries.
- `arch::ArchTable::any<F>(pred) -> bool`: short-circuit scan returning `true` as
  soon as one row satisfies the predicate. Mirrors `Iterator::any`; avoids
  constructing and discarding iterators in "does any entity have X?" guards.
- `mapgen::Rect::area() -> u32`: product `w × h`. The fundamental size metric for
  rooms — used by `largest_room` internally; now exposed for caller sorting, area
  budget checks, and treasure density calculations.
- `mapgen::Dungeon::room_count() -> usize`: count placed rooms without exposing the
  `rooms` field. An O(1) `rooms.len()` shorthand; zero for caves and all-wall maps.
- `wfc::WfcGrid::count_uncollapsed() -> usize`: cells still awaiting collapse
  (bitmask popcount ≠ 1). Complement of `is_fully_collapsed`; useful for progress
  bars and "abort after N tries" limits during WFC post-processing.
- `wfc::WfcRules::is_valid_tile(tile) -> bool`: range-check `tile < tile_count()`.
  The `allow`/`disallow` family silently ignores out-of-range tiles; this lets
  callers validate before mutating rules.
- `profiler::EventLog::oldest() -> Option<&LogEntry<E>>`: oldest entry still
  retained in the ring (complement of `last`). Useful for "how long since the first
  visible event?" TTL calculations and replay-window diagnostics.
- `replay::replay_ok(expected, actual) -> bool`: `first_divergence().is_ok()`
  shorthand. Removes the `.is_ok()` boilerplate at assert sites and CI gate checks
  that only need a boolean result.
- `fixed::Fixed::clamp01(self) -> Fixed`: clamp a fixed-point value into `[0, 1]`
  with one call. Delegates to `clamp(Fixed::ZERO, Fixed::ONE)`. Eliminates the
  two-argument `clamp` boilerplate for the most common case — normalised weights,
  alpha values, and easing `t` parameters.
- `rng::SplitMix64::range_u32(lo, hi) -> u32`: uniform draw from `[lo, hi)` over
  `u32` values. Mirrors `range` but without the `i32` sign-extension concern;
  `lo >= hi` returns `lo` without drawing (same contract as `range`). Deterministic
  and replay-safe.
- `fov::fov_ring(origin, radius, is_opaque) -> Vec<(i32,i32)>`: visible cells
  exactly on the Chebyshev shell at `radius`. Filters `fov_to_vec` to cells where
  `max(|dx|, |dy|) == radius`. Useful for "what can I see at exactly range R?"
  ring queries (area denial, light halos).
- `passability::PassabilityGrid::invert()`: flip every cell: passable becomes
  blocked, blocked becomes passable. The "negative" of the current grid. Useful for
  building complement masks, "invert the dungeon" effects, and test scaffolding.
- `influence::InfluenceMap::max_value() -> Option<i32>`: maximum value stored in
  any cell (`None` for a zero-size map). O(n) scan via `Iterator::max`. Useful for
  normalising influence layers before rendering a heatmap overlay.
- `hud::BarWidget::is_full() -> bool`: `true` when `current >= max` (or `max <= 0`
  for degenerate bars). Lets callers skip the render or suppress a "healing
  available" prompt without a manual comparison.
- `autotile::SimpleTileTable::set_count() -> usize`: number of entries with a
  non-zero tile id — i.e. how many mask patterns have been explicitly assigned.
  Useful for validating that a table is fully or partially initialised.
- `vec::Vec2::is_zero(self) -> bool`: `true` when both `x` and `y` are
  `Fixed::ZERO`. Delegates to `Fixed::is_zero` so it shares the same zero-check
  semantics. Useful for direction guards ("don't normalise a zero vector").
- `aabb::Aabb::perimeter() -> i32`: sum of all four edge lengths `2 × (w + h)`,
  with saturating arithmetic. Returns `0` for empty boxes. For AoE radius
  estimation, hitbox tuning, and "perimeter ≤ budget?" guards.
- `geometry::midpoint(a, b) -> (i32, i32)`: integer midpoint of two grid cells,
  flooring toward `a` on odd separations. Overflow-safe. Useful for marker
  placement, segment bisection, and symmetric projectile arcs.
- `tilemap::TileMap::any_where(pred) -> bool`: short-circuit scan that returns
  `true` as soon as one cell satisfies the predicate. Avoids the allocating
  `find_all(pred).is_empty()` pattern for simple "does a wall exist here?" checks.
- `combat::Stats::is_bloodied() -> bool`: `true` when HP is below 50% of
  `max_hp`. Classic D&D 4e "bloodied" condition. Useful for AI aggression
  thresholds, conditional abilities, and low-health visual indicators.
- `spatial_hash::SpatialHash::count_in_radius_euclidean(qx, qy, radius) -> usize`:
  count entities within Euclidean radius without allocating a `Vec`. Allocation-
  free counterpart to `query_radius_euclidean` for "how many enemies are nearby?"
  density checks.
- `pathfinding::is_reachable(start, goal, is_blocked) -> bool`: thin wrapper
  around `astar` that discards the path and returns only the reachability boolean.
  Avoids constructing the full path Vec for connectivity-only checks.
- `easing::lerp(a, b, t) -> Fixed`: linear interpolation `a + (b−a)×t`. The
  fundamental building block for all easing curves — pair with any ease function
  via `lerp(a, b, ease_fn(t))`.
- `noise::noise_2d_in_range(x, y, seed, lo, hi) -> i32`: convenience combinator
  for `hash_range(hash_2d(x, y, seed), lo, hi)`. Scatter distinct values across a
  map with a single call; avoids repeating the two-call pattern everywhere.
- `camera::Camera::world_to_screen_unclamped(wx, wy) -> (i32, i32)`: convert a
  world-space point to a screen-space offset without bounds checking. Returns
  negative values or out-of-range coordinates for off-screen points. Complement
  of `world_to_screen` for renderers that need raw offset math (e.g. partial
  sprites, infinite worlds).
- `change::Changed::if_changed(since_tick) -> Option<&T>`: return the inner value
  only when changed at or after `since_tick`, otherwise `None`. Combines
  `is_changed_since` with value access to avoid the two-step pattern at every call
  site in system code.
- `status::StatusSet::active_keys() -> Vec<&K>`: list all keys with active effects
  in application order. Useful for rendering a status HUD, serialising active
  debuffs, or iterating all active effects without exposing the internal `entries`
  field.
- `menu::Menu::cursor_label() -> Option<&str>`: label of the item currently under
  the cursor, or `None` on an empty menu. Avoids `current()?.label.as_str()` at
  every rendering call site.
- `random_table::RandomTable::max_weight() -> u32`: highest single-entry weight in
  the table (`0` for empty/all-zero tables). Useful for "is this table uniform?"
  checks and tuning loot balance without manual iteration.
- `cmdqueue::CmdQueue::contains(pred) -> bool`: check whether any queued command
  satisfies a predicate without consuming the queue. Useful for abort-before-commit
  patterns ("is a Cancel command already queued?").
- `inputbuf::InputBuffer::is_repeating(key) -> bool`: `true` if the key is held
  and past the `initial_delay` threshold — i.e. in the repeat phase. Lets UI code
  distinguish first-press from auto-repeat to suppress single-fire effects.
- `relations::Relations::child_count(entity) -> usize`: count direct children
  without allocating a `Vec`. Drop-in replacement for `children_of(e).len()` in
  hot loops and capacity guards.
- `dice::Dice::span() -> u32`: spread between minimum and maximum outcomes
  (`max() − min()`). Modifier cancels so only `count × (sides − 1)` matters;
  useful for balance heuristics and variance comparisons.
- `timer::Cooldown::extend(extra)`: add ticks to an existing cooldown (saturating).
  The "slow" counterpart to `tick`; pushes back ability availability without
  resetting the full timer.
- `msglog::MsgLog::push_unique(msg)`: push only when the message differs from the
  most recent entry. Deduplicates consecutive identical strings ("you hit the orc"
  spam) without requiring the caller to track the last message.
- `inventory::Inventory::move_to_slot(from, to) -> bool`: relocate an item between
  two slots without a remove/add round-trip. Returns `false` if `from` is empty,
  `to` is occupied, or either index is out of bounds.
- `fsm::Fsm::is_in(state) -> bool`: `self.state() == state` shorthand. Avoids
  importing and comparing state variants at every AI call site.
- `turn::Scheduler::drain() -> Vec<A>`: remove all actors and return their ids in
  insertion order. End-of-floor cleanup in one pass without iterating the scheduler.
- `multimap::MultiMap::remove_connector(from_floor, from_x, from_y) -> bool`:
  remove the connector at a source position. The inverse of `add_connector`; returns
  `false` if no matching connector is found.
- `keymap::KeyMap::action_count() -> usize`: count distinct actions with at least
  one binding. Useful for "are all N actions mapped?" completeness checks.
- `entity::EntityAllocator::free_count() -> usize`: O(1) count of freed (reusable)
  slots. Equivalent to `total_slots() - count()`. Memory diagnostics for systems
  monitoring allocator pressure.
- `sparse_set::SparseSet::remove_where(pred) -> usize`: bulk-remove all entries
  matching a predicate; returns the count removed. Avoids a temporary Vec for
  filtered despawn passes.
- `world_hash::Fnv1a::write_str(s)` + `DetHash for str` / `DetHash for String`:
  fold UTF-8 string bytes into the hasher. Lets entity names, config keys, and
  string fields participate in world state checksums without manual `.as_bytes()`.
- `content::Color::from_hex(s) -> Result<Color, String>`: inverse of `to_hex`;
  delegates to `parse_color` for consistent validation and error messages.
- `content::Content::tile(name) -> Option<&Tile>`: look up a tile definition by
  name. Symmetric with `Content::prefab` and `Content::level`.
- `parser::warning_count(diags) -> usize`: count warning-severity diagnostics —
  the complement of `error_count`. Exposes the warning tally for CI gates that
  report warnings separately from errors.
- `savefile::estimate_save_size(payload_len) -> usize`: predict the byte length
  of a `save_bytes` buffer (`20 + payload_len`) before allocating or writing to
  storage. Avoids over-allocation for streaming writers.
- `textlayout::truncate_lines(lines, max_cols) -> Vec<String>`: apply `truncate`
  to every line in a slice. Batch-clips multi-line HUD panels and dialogue boxes
  to a fixed column width with one call.
- `content::Color::alpha_blend(fg, bg, alpha) -> Color`: composite `fg` over
  `bg` with integer alpha (`0` = fg, `255` = bg) using per-channel
  `(fg*(255-alpha) + bg*alpha)/255`. No float; deterministic. For UI overlays,
  particle fading, and damage indicators.
- `parser::error_count(diags) -> usize`: count error-severity diagnostics in a
  slice — CI / tool convenience, avoids manual iteration.
- `serializer::first_diff(a, b) -> Option<String>`: describe the first semantic
  difference between two `Content` values (prefab count, level rows, etc.) for
  debugging round-trip failures.
- `validator::error_count(diags) -> usize`: same helper as `parser::error_count`
  but exposed from the validator module for gate-check idioms.
- `entity::EntityAllocator::highest_generation() -> u32`: max generation counter
  across all slots (`0` when empty). Useful for diagnostics and save-file audits
  to detect near-exhausted generation space.
- `fov::fov_to_vec_dist(origin, radius, max_dist_sq, is_opaque) -> Vec<(i32,i32)>`:
  like `fov_to_vec` but further filtered to `dist_sq <= max_dist_sq`. One-pass
  light-falloff/torch-radius queries without a second Vec filter.
- `mapgen::Dungeon::largest_room() -> Option<Rect>`: room with the greatest area
  (`w × h`); `None` for all-wall dungeons (caves, tiny maps). Spawn bosses,
  stairs, and treasure in the biggest room.
- `savefile::LoadError::is_recoverable() -> bool`: `true` for `BadMagic` (wrong
  file — treat as empty slot), `false` for `TooShort` or `ChecksumMismatch`
  (corruption — warn and offer to clear). Guides save-slot UI error handling.
- `fixed::Fixed::pow(exp: u32) -> Fixed`: integer exponentiation via repeated
  saturating multiplication. `pow(0)` is `Fixed::ONE`, `pow(1)` is `self`
  exactly; overflow pins to `Fixed::MAX`/`MIN` rather than wrapping. Float-free
  and deterministic.
- `geometry::line_len(a, b) -> usize`: number of cells [`line`] returns without
  allocating — Chebyshev distance plus one. For sizing buffers or range checks
  before tracing a Bresenham ray.
- `menu::Menu::label_at(idx) -> Option<&str>`: display label of the item at
  `idx` (`None` if out of range), without exposing the whole `MenuItem`.
- `keymap::KeyMap::unbind_action(action) -> usize`: remove every binding for a
  given action, returning the count removed. The inverse of `bind_multiple`.
- `noise::hash_range(h, lo, hi) -> i32`: map a raw `u32` hash into `[lo, hi)`
  with an unbiased wide multiply (`lo` for degenerate ranges). Suits per-cell
  scatter tables; distinct from `normalize_noise` which scales the `[0,65535]`
  smooth-noise range. Deterministic and float-free.
- `pathfinding::octile_distance(a, b) -> i32`: the octile heuristic cost A* pays
  to cross open ground (`10`/`14` scale). Estimate path length or range without
  running a search.
- `textlayout::count_lines(text, max_cols) -> usize`: number of lines
  `wrap_words` would produce, for pre-measuring wrapped-block height (dialogue
  boxes, tooltips) without keeping the wrapped `Vec`.
- `timestep::FixedTimestep::reset_accumulator() -> u64`: discard accumulated
  sub-step time (returning the dropped nanoseconds) so a resume after a pause or
  level load does not replay buffered time as a burst of catch-up steps. Leaves
  `total_steps` and the configured rate untouched.
- `aabb::Aabb::touches(&self, other: &Aabb) -> bool`: true when two boxes are
  adjacent (shared edge or corner) but do not overlap. Empty boxes never touch;
  diagonal corner contact counts. Built on `grow(1)` + `overlaps`, all saturating.
  Supports roguelike adjacency / "reach" interaction checks.
- `combat::Stats::missing_hp(&self) -> i32`: health deficit below maximum
  (`max(0, max_hp − hp)`), i.e. healing needed to reach full. Clamps at 0 for
  over-full HP. Useful for healing AI and damage-preview UI.
- `inventory::Inventory::find_mut<F>(pred) -> Option<&mut T>`: mutable complement
  to `find`; returns the first item matching the predicate so it can be modified
  in place without a slot round-trip.
- `msglog::MsgLog::filtered<P>(pred) -> Vec<&str>`: collect references to all
  messages matching a predicate, oldest-to-newest, without mutating the log
  (unlike `retain`). Supports "show only combat messages" filtered views.
- `rng::SplitMix64::pick_index(len) -> Option<usize>`: uniform random index in
  `0..len` (`None` for `len == 0`, no draw). The index-only primitive behind
  `pick`/`pick_mut`, for when data lives in parallel arrays or an ECS store.
  Consumes exactly one draw — replay-deterministic.
- `sparse_set::SparseSet::find_entity_where<F>(pred) -> Option<Entity>`: return
  the first entity whose component satisfies the predicate. The component-query
  complement to `count_matching`; scans in dense order.
- `tilemap::TileMap::bounds_of<P>(pred) -> Option<Aabb>`: minimal half-open
  `Aabb` enclosing every cell matching the predicate (`None` if none match). A
  single cell yields a 1×1 box. Useful for room / spell-area / painted-region
  bounds without manual min/max bookkeeping.
- `turn::Scheduler::peek_next_turn(&self) -> Option<A>`: id of the actor who
  would act on the next `next_turn`, without advancing time or deducting energy.
  Uses the same smallest-id tie-break, so it is a true non-destructive preview
  for "whose turn?" UI and AI look-ahead.
- `change::Changed::is_fresh(max_age_ticks, current_tick) -> bool`: logical
  inverse of `is_stale`; returns `true` when the component was marked within the
  last `max_age_ticks` ticks. Avoids double-negation at call sites and makes
  "only process recently updated components" filters self-documenting.
- `fsm::Fsm::transition_count(from: &S) -> usize`: count outgoing transitions
  from a state without constructing an iterator. Equivalent to
  `transitions_from(state).count()` but avoids the closure overhead for the
  common "does this state have any exits?" guard.
- `status::StatusSet::magnitude_range() -> (i32, i32)`: return the `(min, max)`
  magnitude across all active effects. Returns `(0, 0)` for an empty set. Useful
  for "net buff range" queries and AI tuning without iterating manually.
- `inputbuf::InputBuffer::set_timing(initial_delay, repeat_period)`: update
  hold-repeat timing parameters without clearing the buffer or releasing held keys.
  Clamps `repeat_period` to ≥ 1. Supports "haste" power-ups and in-session
  accessibility setting changes.
- `vec::Vec2::reflect(self, normal: Vec2) -> Vec2`: reflect a vector off a
  surface with the given unit normal via `self − 2·(self·n)·n`. Assumes
  `normal` is already unit length. Useful for physics-style bounce responses
  in collision resolution and beam/projectile ricochet.
- `multimap::MultiMap::connectors_between(from_floor, to_floor) -> Vec<&Connector>`:
  all connectors originating on `from_floor` and leading to `to_floor`. Returns
  an empty `Vec` when none exist. Supports multi-staircase levels where several
  exits between the same floor pair need to be enumerated.
- `dice::Dice::roll_with_reroll(reroll_on, rng) -> i32`: roll once; if the
  result is ≤ `reroll_on`, roll a second time and return that result regardless
  of whether it is better. Consumes two draws on reroll, one otherwise. Implements
  the "reroll on 1" mechanic from roguelike ability checks and tabletop RPGs.
- `timer::Cooldown::fractional_progress(original_ticks) -> Fixed`: fractional
  progress through the cooldown as a Q16.16 `Fixed` value in `[0, 1]` (`0` = just
  started, `1` = ready). `original_ticks == 0` returns `Fixed::ONE`. Suitable as
  a lerp parameter for smooth animation without converting to float or percent.
- `wfc::WfcRules::clear_adjacencies(tile: u8)`: clear all adjacency bitmasks for
  `tile` in every direction, resetting it to "forbidden everywhere". Silently
  ignores out-of-range tile indices. Supports rule-mutation workflows where a tile
  is conditionally disabled at runtime.
- `passability::PassabilityGrid::random_passable(rng: &mut SplitMix64) -> Option<(i32, i32)>`:
  pick a uniformly random passable cell via `SplitMix64::pick`. Draws exactly one
  RNG value when at least one passable cell exists; draws nothing for an all-blocked
  grid. Primary use: spawn-placement, random patrol targets.
- `profiler::EventLog::count_by<F>(pred: F) -> usize`: count stored events
  matching a predicate without allocating — `self.iter().filter(|e| pred(&e.event)).count()`
  packaged as a named method. Avoids the iterator boilerplate in common "how many
  damage events this tick?" patterns.
- `random_table::RandomTable::weighted_idx(&self, rng: &mut SplitMix64) -> Option<usize>`:
  like `roll` but returns the entry index instead of the value. Draws once;
  returns `None` for empty/all-zero tables. Useful when the caller needs to inspect
  or remove the rolled entry rather than just use its value.
- `replay::count_divergences(expected: &[u64], actual: &[u64]) -> usize`: count
  all diverging ticks (unlike `first_divergence` which stops at the first one).
  Ticks beyond the shorter trace count as divergences. Also re-exported at the
  crate root.
- `relations::Relations::reparent_all(old_parent, new_parent) -> bool`: move all
  children of `old_parent` to `new_parent` in one call. Returns `true` if at least
  one child was moved; `false` for same-parent or childless cases. Builds on
  `detach_all_children` + `attach` and preserves the existing `DetHash` contract.
- `assets::AssetStore::handles() -> impl Iterator<Item = AssetHandle<T>>`: iterate
  over live handles without borrowing values. Useful when constructing a list of
  handles for batch remove / validity checks without holding a value borrow.
- `spatial_hash::SpatialHash::all_in_cell(cx, cy) -> Vec<K>`: return an owned
  `Vec` of all keys in cell-space cell `(cx, cy)`. Unlike `query_cell` which
  returns a borrowed slice, the owned `Vec` lets the caller mutate the hash during
  iteration. Returns an empty `Vec` for unoccupied cells.
- `arch::ArchTable::with_capacity(n) -> Self`: pre-allocate both the dense Vec
  and the HashMap for `n` entities, avoiding repeated reallocation on bulk
  insertions. Equivalent to `new()` in all other respects; deterministic hash is
  unchanged.
- `camera::Camera::screen_distance(sx1, sy1, sx2, sy2) -> u32`: Chebyshev
  distance between two screen-space cells — `max(|sx1−sx2|, |sy1−sy2|)`.
  Static method; requires no camera state. Matches the king-move metric used by
  `chebyshev_to_center`. Useful for "is this enemy within cursor range?" UI checks.
- `cmdqueue::CmdQueue::as_slice(&self) -> &[C]`: view pending commands as a
  slice without consuming them. Follows the Rust `as_slice` convention and
  complements the existing `peek()`; callers that need slice-specific methods
  (`windows`, `chunks`, `starts_with`) can use this directly.
- `diag_json::severity_filter(diags, severity) -> Vec<Diagnostic>`: filter a
  diagnostic slice to a given severity level, preserving order. Useful for
  routing errors and warnings to separate outputs (stderr vs. log) before calling
  `diag_json`. Also re-exported at the crate root.
- `hud::HudPanel::pad(left, top, right, bottom) -> HudPanel`: shrink a panel by
  explicit per-side margins, returning a new panel. Width and height saturate at 0
  when margins exceed the panel size. Complements the fixed-1-cell `inner_*`
  helpers with caller-controlled insets.
- `influence::InfluenceMap::gradient_at(x, y) -> Option<(i32, i32)>`: the step
  direction `(dx, dy)` of steepest ascent from `(x, y)` — equivalent to
  `highest_neighbour` without the value. Returns `None` when there are no
  in-bounds neighbours (same condition as `highest_neighbour`). Primary use:
  AI pathfinding step "move toward maximum influence."
- `easing::ease_reversed(t, ease_in) -> Fixed`: converts any ease-in function to
  its ease-out mirror by time-reversal (`1 − f(1 − t)`). Works with all standard
  Penner ease-in functions. Also re-exported at the crate root.
- `autotile::SimpleTileTable::fill_range(start, end, tile_id)`: set all 256-entry
  table slots in the inclusive range `[start, end]` to `tile_id`. Replaces
  repeated `set()` calls when initialising consecutive mask groups (e.g. all
  cardinal-only masks). No-op when `start > end`.
- `rng::SplitMix64::pick_mut(&mut [T]) -> Option<&mut T>`: mutable variant of
  `pick` — select a uniform-random element and hand back a mutable reference so
  the caller can modify it in-place without an index round-trip. No draw for
  empty slices.
- `fixed::Fixed::hypot(a, b) -> Fixed`: Euclidean distance `sqrt(a²+b²)`,
  computed with the existing `mul`+`sqrt` chain. Replaces the manual
  `(a.mul(a).saturating_add(b.mul(b))).sqrt()` pattern at call sites. Saturates
  for very large inputs the same way `mul` does.
- `aabb::Aabb::from_center_size(cx, cy, w, h) -> Aabb`: construct a box from a
  center point and dimensions. Top-left is `(cx - w/2, cy - h/2)` (integer
  division, same truncation bias as `center()`). Negative sizes are clamped to 0.
  Mirrors `glam`'s `from_center_half_size` convention.
- `tilemap::TileMap::mutate_where(pred, transform)`: apply a `FnMut(T) -> T`
  transformation to every cell for which `pred` returns `true`, modifying in
  place. The standard "apply poison decay / environmental effect to all matching
  tiles" primitive. O(n) in map size; no allocation.
- `menu::Menu::remove_item(idx)`: delete the item at `idx` and keep the cursor
  valid — decrements if the cursor was past the removed item, clamps to the new
  last item otherwise. No-op for out-of-bounds indices. Needed for dynamic menus
  (quest log entries, loot picks).
- `inventory::Inventory::get_mut(slot) -> Option<&mut T>`: mutable borrow of the
  item at `slot`. Allows in-place modification (durability tick, stack count)
  without the `remove` + `add` round-trip that changes the slot index.
- `turn::Scheduler::time_until_ready(id) -> Option<i32>`: time units until actor
  `id` first accumulates `ACTION_COST` energy. Returns `Some(0)` when already
  ready, `None` when the actor is unknown. Does not advance the queue — pure read.
  Useful for AI look-ahead ("the goblin acts in N turns") and UI countdowns.
- `msglog::MsgLog::pop() -> Option<String>`: remove and return the most recently
  pushed message (LIFO). Returns `None` for an empty log. Useful for "undo last
  event message" and round-trip tests.
- `fov::fov_to_vec(origin, radius, is_opaque) -> Vec<(i32,i32)>`: collect all
  visible cells into a `Vec` in one call. Equivalent to bracket-lib's
  `field_of_view_set` — wraps `compute_fov` so callers don't need to manage
  a closure-owned collection. Re-exported as `izanagi_kit::fov_to_vec`.
- `combat::roll_damage(rng, base, variance) -> i32`: base damage plus a uniform
  random bonus in `[0, variance]`. `variance == 0` returns `base.max(0)` without
  consuming an RNG draw. Gives attacks a natural spread (e.g. `roll_damage(rng, 5,
  3)` → 5–8) without spelling out the `Dice` formula at every call site.
- `relations::Relations::siblings_of(entity) -> Vec<Entity>`: entities that share
  the same parent as `entity`, excluding `entity` itself. Returns an empty `Vec`
  for roots and only children. Useful for item-group queries ("other items in the
  same container") and squad-member adjacency patterns.
- `timer::Cooldown::percent_remaining(original_ticks) -> u32`: percentage of the
  cooldown still pending as an integer in `[0, 100]`. The inverse of `elapsed`;
  `original_ticks == 0` returns 0 (ready). Useful for progress-bar rendering.
- `savefile::validate_integrity(data) -> Result<(), LoadError>`: check that `data`
  is a structurally valid save file without deserialising the payload. Equivalent
  to `load_bytes(data).map(|_| ())`. Useful for save-slot browser UI that needs a
  "valid / corrupt" indicator cheaply. Re-exported as `izanagi_kit::validate_integrity`.
- `textlayout::fit_to_box(text, width, height) -> Vec<String>`: wrap text to fit
  a `width × height` box in one call — combines `wrap_words_max_lines` with the
  `…` overflow indicator. The typical "dialogue box" call site in roguelike UIs.
  Re-exported as `izanagi_kit::fit_to_box`.
- `change::Changed::is_stale(age_threshold, current_tick) -> bool`: returns `true`
  if the component has not been marked for at least `age_threshold` ticks
  (`ticks_since_change >= age_threshold`). Eliminates the comparison boilerplate
  at TTL / cache-invalidation call sites.
- `spatial_hash::SpatialHash::iter_keys() -> impl Iterator<Item = &K>`: flat
  iterator over every registered key across all cells, without intermediate
  allocation. Useful for "process all entities in the spatial index" end-of-frame
  passes where cell membership is irrelevant.
- `keymap::KeyMap::contains_action(action)`: returns `true` if at least one key
  is currently bound to `action`. Allocation-free (no collect) — use
  `get_keys_for_action` when you also need the key list.
- `keymap::KeyMap::bind_multiple(keys, action)`: bind every key in a slice to
  the same action in one call; existing bindings for any key in the slice are
  replaced. Ergonomic alias for the repeated `bind` pattern seen in default
  key-layout setup code.
- `entity::EntityAllocator::total_slots()`: O(1) count of all slots ever
  created (both live and freed). Useful for memory-budget checks and save-file
  headers that need to know the allocator high-water mark.
- `geometry::rect_contains(x, y, w, h, px, py)`: fast inline rectangle
  point-in-bounds check. Returns `false` for empty rectangles (`w ≤ 0` or
  `h ≤ 0`). Replaces the recurring `px >= x && px < x+w && py >= y && py < y+h`
  pattern at call sites.
- `world_hash::DetHash for (A, B)` and `DetHash for (A, B, C)`: folds tuple
  fields left-to-right, identical to sequential individual `det_hash` calls.
  Enables hashing heterogeneous pairs/triples (e.g. `(Entity, Position)`) used
  as canonical map keys without an intermediate struct.
- `noise::ridge_noise_2d(x, y, seed, octaves)`: ridged multifractal noise —
  folds each FBM octave through `|raw − 32768|` to produce sharp ridges.
  Normalised to `[0, 65535]`. Builds mountain-range and river-valley heightmaps
  from the same deterministic value-noise primitive.
- `aabb::Aabb::from_corners(x1, y1, x2, y2)`: construct from two corners in any
  order — the result always has non-negative `w`/`h`. Matches the bracket-lib and
  glam `from_min_max` convention; avoids manual `min`/`max` arithmetic at call sites.
- `aabb::Aabb::grow(amount)` / `shrink(amount)`: expand or contract by a uniform
  margin on all four sides. Saturating arithmetic. Negative `amount` to `grow` is
  equivalent to `shrink`. Size clamps to zero so the result is always a valid AABB.
- `noise::value_noise_2d_wrap(x, y, seed, period_x, period_y)`: 2-D value noise
  that tiles seamlessly at the given integer period. Corner hashes use `rem_euclid`
  to wrap, so `noise(0, y) == noise(period_x, y)` exactly. Essential for seamless
  dungeon level textures and world-map wrapping.
- `noise::fbm_2d_wrap(x, y, seed, octaves, period)`: tileable 2-D FBM — each octave
  wraps at `period << octave_index` so harmonics also tile. Builds naturally-looking
  wrapping terrain via layered `value_noise_2d_wrap`.
- `random_table::RandomTable::roll_n(n, rng)`: draw `n` independent samples with
  replacement, returning `Vec<T>`. Consumes no draws for `n == 0` or an empty table.
  The canonical "roll the encounter table three times" primitive.
- `rng::SplitMix64::pick(slice)`: uniform random element from a non-empty slice,
  returning `Option<&T>`. Returns `None` without drawing for an empty slice so
  draw count stays a deterministic function of arguments (mirrors `rot.js getItem`).
- `rng::SplitMix64::next_u32()`: advances the stream and returns the upper 32 bits
  of the 64-bit output. Useful when only a 32-bit integer is needed.
- `sparse_set::SparseSet::clear()`: bulk removal of all entries; resets the sparse
  index so no stale slots remain. Equivalent to `retain(|_,_| false)` but O(n)
  without the per-element predicate call overhead.
- `sparse_set::SparseSet::entities()`: entity-handle-only dense iterator. Mirrors
  the `keys()` / `iter_entities()` convention of Bevy and EnTT, avoiding an
  unnecessary component dereference when only entity membership matters.
- `sparse_set::SparseSet::values()` / `values_mut()`: component-value-only iterators.
  Avoids the entity-handle unpacking boilerplate when the caller only needs to read
  or mutate all stored values (e.g. ticking every AI state machine).
- `dice::Dice::roll_advantage(rng)` / `roll_disadvantage(rng)`: D&D 5e advantage
  mechanics — roll twice, keep max / min respectively. Each call consumes exactly
  two draws so the draw count is fixed and replay-deterministic.
- `dice::Dice`: `std::fmt::Display` — pretty-prints as `"3d6+2"`, `"d20"`,
  `"2d8-1"`, etc. Enables round-trip `parse(dice.to_string())` and human-readable
  logging without heap allocation beyond the format string.
- `easing`: back easing family — `ease_in_back`, `ease_out_back`, `ease_in_out_back`.
  Standard Penner overshoot curves (c1 ≈ 1.70158). The ease-in version briefly goes
  below 0; ease-out briefly exceeds 1. Endpoints are exactly 0→1. Pure polynomial,
  no trig, no float.
- `easing`: bounce easing family — `ease_out_bounce`, `ease_in_bounce`,
  `ease_in_out_bounce`. Piecewise polynomial (7.5625·t² in 4 segments) approximating
  a bouncing ball. Stays in `[0, 1]` by construction. No float, no trig.
- `vec::Vec2`: `abs()`, `min(a, b)`, `max(a, b)`, `clamp(lo, hi)` — component-wise
  utilities present in bracket-lib's geometry crate. `abs` maps each component
  through `Fixed::abs`; `min`/`max` use component-wise comparisons; `clamp`
  composes them. All operations are deterministic and saturating, safe to include
  in world hashes.
- `vec::Vec3`: same `abs()`, `min()`, `max()`, `clamp()` additions plus `xy()` —
  project to `Vec2` by dropping `z`. Mirrors the `glam::Vec3::xy()` convention and
  covers the common top-down 2D view of a 3D world-space position.
- `vec::Vec2::Neg`, `vec::Vec3::Neg`, `vec::Vec2::perp`: replaced `Fixed::ZERO - v`
  workarounds with idiomatic `-v` now that `Fixed::Neg` is implemented. Behaviour
  is identical (saturating subtraction and saturating negation are equivalent for
  all Q16.16 values); the change is purely for readability.
- `terminal::Screen::draw_box(x, y, w, h, fg, bg)`: draws a single-line box border
  using Unicode box-drawing characters `┌┐└┘─│`. The interior is untouched.
  Out-of-bounds positions are silently clipped via the existing `put` contract —
  no panic. The missing primitive for roguelike panels, inventory windows, and HUD
  borders.
- `terminal::Screen::resize(width, height)`: replace the front and back buffers
  with blank cells at the new dimensions, discarding all previous content. Required
  for real-world `SIGWINCH` / terminal-resize handling where the caller redraws
  from scratch.
- `profiler::Profiler::avg(section)`: rolling average tick-total for a section
  over the history window. The method was documented in the module doc-comment but
  never implemented; this fills the gap. Only ticks where the section recorded at
  least one sample are counted (silent ticks don't dilute the average).
- `fixed::Fixed`: `core::ops::Neg` implementation. Previous code had to write
  `Fixed::ZERO - v` to negate; the new impl makes `-v` work directly. Saturating:
  `-Fixed::MIN` returns `Fixed::MAX` instead of overflowing.
- `assets::AssetStore::iter_mut()`: mutable `(handle, &mut asset)` iteration in
  ascending slot-index order. Fills the symmetric gap left by `iter()`. Enables
  in-place mass updates (e.g. ticking all sprite animation frames) without
  collect-then-update round-trips.
- `assets::AssetStore::retain(pred)`: remove all assets for which
  `pred(handle, &asset)` returns `false`. Parallel to `SparseSet::retain` — the
  end-of-frame cleanup pattern without a collect+remove loop. Removed slots are
  freed and handles permanently invalidated.
- `keymap::KeyMap::clear()`: remove all bindings at once. Useful for rebuilding a
  key layout at runtime (e.g. when the player changes the control scheme in the
  options menu).
- `wfc::WfcRules::disallow(tile, dir, neighbor)`: the symmetric counterpart of
  `allow`. Clears the corresponding adjacency bit so the combination is no longer
  permitted. Out-of-range arguments are silently ignored (same contract as
  `allow`).
- `savefile::LoadError`: `std::fmt::Display` and `std::error::Error` implementations.
  The type previously only derived `Debug`; the new impls allow it to be used in
  `?`-chains with `anyhow` / `thiserror`-style error plumbing, and to be printed
  as a human-readable message (e.g. "save file checksum mismatch").
- `rng::SplitMix64::shuffle(slice)`: Fisher-Yates in-place shuffle. Draws
  `slice.len() − 1` times using `below` so the draw count is deterministic and
  the sequence replays identically given the same seed and position. Handles
  empty and single-element slices without drawing. The missing primitive that
  every roguelike needs for shuffling room lists, random encounter tables, and
  ability orderings. Re-exported as `izanagi_kit::SplitMix64` (method on the type).
- `hud::BarWidget::percentage()`: fill percentage as an integer in `[0, 100]`.
  Useful for "85% HP" status lines without float arithmetic. Saturates: negative
  current returns 0, over-max returns 100, max ≤ 0 returns 0.
- `arch::ArchTable::upsert(entity, row)`: insert if absent, overwrite if present.
  Complements `insert` (which silently ignores duplicates) with the common
  "move entity = update position" pattern. Implemented as a branch in the hot
  path (checks the HashMap index, updates in-place if found).
- `tilemap::TileMap::iter_mut()`: mutable `(x, y, &mut tile)` row-major iteration.
  Fills the symmetric gap left by the existing shared `iter()`. Needed for any
  in-place map update (fog-of-war ageing, post-gen tile transformations, render
  passes that write back per-cell state). Follows the same coordinate computation
  as `iter()` so no separate index book-keeping is required.
- `random_table::RandomTable::roll_owned(rng)`: returns a cloned `T` instead of
  a `&T`. Removes the borrow chain that `roll()` carries so callers can store the
  loot result in a variable, pass it to another function, or push it into a
  container without lifetime headaches. Requires `T: Clone`; implemented as
  `.roll(rng).cloned()` — no separate draw logic.
- `sparse_set::SparseSet::retain(pred)`: bulk removal of all entries for which
  `pred(entity, &value)` returns `false`. O(n) — each removed entry pays one
  swap-remove. The canonical "remove dead entities at end-of-frame cleanup" pattern,
  matching `Vec::retain` semantics. Avoids collecting entity IDs and calling
  `remove` in a separate pass.
- `influence::InfluenceMap::combine(other, num, den)`: cell-wise weighted addition
  — `self[i] += other[i] * num / den` (saturating). Enables composing multiple
  influence layers with different weights in one call (e.g. threat at −1× plus
  food at +⅔). `den == 0` is treated as 1; maps of different sizes are a no-op.
- `easing::ease_in_out_cubic`: completes the cubic easing family (the `in` and
  `out` variants existed but `in_out` was missing). Formula: `4t³` for `t < 0.5`,
  `1 − 4(1−t)³` for `t ≥ 0.5`. The most widely used of the cubic trio.
- `aabb::Aabb::union(other)`: the smallest AABB enclosing both `self` and `other`.
  Empty boxes are excluded from the result (a union with an empty box returns the
  non-empty side). Equivalent to `bracket-lib`'s `Rect::union`. Useful for
  computing aggregate bounding boxes over a set of colliders.
- `spatial_hash::SpatialHash::query_radius(cx, cy, radius)`: all keys within
  Chebyshev distance `radius` of a center point (equivalent to a square query
  `[cx-r, cx+r] × [cy-r, cy+r]`). Complements `query_rect` with a center+radius
  ergonomic API common to most roguelike spatial indices. Returns empty for
  negative `radius`.
- `multimap::MultiMap::current_mut()`: mutable borrow of the currently active
  `Dungeon` floor. Complements the existing `current()` shared borrow — needed for
  any map-edit operation (door open/close, terrain mutation, item placement) on the
  live floor.
- `entity::EntityAllocator::count()`: O(1) live entity count (`total_slots −
  free_slots`). Useful for diagnostics and save-file headers ("32 entities saved").
- `camera::Camera::pan(dx, dy, world_w, world_h)`: scroll the viewport by a
  world-space delta, clamped at the world boundary. The natural mapping for
  arrow-key map scrolling or tracking a fast-moving entity.
- `camera::Camera::center()`: world-space coordinate at the centre of the
  viewport (`top_left + screen_size/2`). Replaces the manual arithmetic callers
  previously wrote to answer "what is the camera looking at?".
- `passability::PassabilityGrid::iter_passable()`: row-major iterator over `(x,
  y)` of passable cells. Collect it and index with `rng.below(count)` for uniform
  random spawn placement without scanning the full map.
- `passability::PassabilityGrid::iter_blocked()`: companion iterator for blocked
  cells — useful for visualisation and testing that map generation placed walls
  in the expected positions.
- `relations::Relations::descendants_of(entity)`: BFS over the entity's subtree
  (children, grandchildren, …), excluding the root itself. Enables "kill all
  carried items" and hierarchical cleanup without repeated `children_of` calls.
- `relations::Relations::root_of(entity)`: walk up the parent chain to the
  topmost ancestor. Returns `entity` itself if it has no parent.
- `msglog::MsgLog::last()`: returns a reference to the most recently pushed
  message, or `None` if the log is empty. Avoids allocating a full `recent(1)`
  iterator for the common "show the last line in the status bar" pattern.
- `combat::StrikeResult` and `combat::critical_strike`: melee attack with a
  critical-hit chance — rolls `crit_chance` (1..=100 percent) via `roll_to_hit`,
  multiplies base damage by `crit_multiplier` on a crit (minimum 1×), applies to
  the defender, and returns `StrikeResult { damage, critical }`. The RNG contract
  matches `roll_to_hit`: degenerate `crit_chance` (≤ 0 or ≥ 100) consumes no draw.
  Deterministic and replay-safe. Re-exported as `izanagi_kit::{critical_strike, StrikeResult}`.
- `combat::Stats::restore()`: refills HP to `max_hp` in one call (level-up
  rest, potion of full healing).
- `combat::Stats::set_max_hp(new_max)`: adjusts max HP and clamps current HP
  to the new ceiling — the standard level-up HP-increase pattern.
- `status::StatusSet::clear()`: removes all active effects at once (e.g.
  "cure all conditions" spell, death cleanup).
- `status::StatusSet::magnitude_of(key)`: direct magnitude lookup returning 0
  when inactive — eliminates the `.get(k).map_or(0, |e| e.magnitude)` boilerplate.
- `status::StatusSet::remaining_of(key)`: direct duration lookup returning 0
  when inactive — mirrors `magnitude_of`.
- `inventory::Inventory::clear()`: empties all slots while preserving capacity
  (store re-stock, respawn, container reset).
- `inventory::Inventory::remove_where(pred)`: removes and returns the first
  item matching a predicate — the "remove first consumable of type X" operation.
- `inventory::Inventory::iter_mut()`: mutable `(slot_index, &mut item)` iteration
  for in-place updates (durability, stack-size changes, re-identification).
- `fov::compute_fov_dist`: distance-attenuated FOV variant — same symmetric
  shadowcasting as `compute_fov` but the callback receives an extra `dist_sq: i32`
  (squared Euclidean distance from origin). Callers use it to implement light
  falloff, range-scaled fog-of-war, or ambient darkness without a separate sqrt
  pass. Origin is always reported with `dist_sq == 0`. Deterministic and replay-safe;
  the underlying scan logic is shared with `compute_fov` (no duplication). Re-exported
  as `izanagi_kit::compute_fov_dist` (cf. libtcod / rot.js light-source patterns).
- `pathfinding::smooth_path`: greedy Bresenham LOS path simplification ("string
  pull"). Post-processes any `astar`/`weighted_astar` path by skipping waypoints
  for which a straight Bresenham segment has no blocked interior cells, reducing
  the staircase visual of grid paths. Suitable for AI waypoint navigation: the
  actor still steps through each smoothed segment, so no-corner-cutting is
  enforced at runtime. Fully deterministic (same path + same `is_blocked` → same
  output). Re-exported as `izanagi_kit::smooth_path`.
- `pathfinding::dijkstra_map` and `pathfinding::descend` re-exported at the crate
  root (`izanagi_kit::{dijkstra_map, descend}`). These existed in the module but
  were not previously in `lib.rs` — closing the API gap.
- `turn::Scheduler::energy(id)`: returns the current banked energy for an actor
  (`None` if not registered). Useful for save/load and diagnostics; values ≥
  `ACTION_COST` indicate the actor is immediately ready.
- `turn::Scheduler::set_energy(id, energy)`: directly sets banked energy — negative
  values add a delay, values ≥ `ACTION_COST` grant an immediate turn. Designed for
  restoring exact scheduler state from a save file.
- `timer::TimerQueue::cancel_where(pred)`: selectively cancels pending entries whose
  event satisfies a predicate. Returns the count removed. Both one-shot and repeating
  entries are eligible. Fills the gap between `cancel_all` (nuke everything) and
  the lack of any partial cancellation API.
- `dice::Dice`: tabletop dice-notation parsing and rolling — `Dice::parse("3d6+2")`
  (panic-free, `None` on malformed input), `roll(&mut rng)` (replay-deterministic
  via `SplitMix64::dice`), plus `min`/`max`/`average_x100` range queries for
  balancing without floats. The standard data-driven way to author damage / hit
  dice (cf. `bracket-random`'s `roll_str`). Re-exported as `izanagi_kit::Dice`.
- `content::Color::from_hsv`: integer HSV→RGB conversion (`hue` in degrees mod
  360, `sat`/`val` in `0..=255`) for procedural palettes and rainbow/heat
  gradients — no float, deterministic across targets.
- `mapgen::generate_bsp` / `mapgen::BspParams`: binary-space-partition dungeon
  generation — the third classic family alongside `generate_dungeon` (room
  rejection) and `generate_cave` (cellular automata); cf. `bracket-lib`'s BSP
  builders. Recursively splits the interior (orientation biased by aspect ratio,
  split point from `rng`) until partitions hit `min_leaf` or `max_depth`, carves
  one room per leaf (disjoint by construction), and joins each split's two child
  rooms with an L-corridor on the way back up — guaranteeing full connectivity.
  Returns the same `Dungeon` type; too-small maps return all-wall without
  panicking. Re-exported as `izanagi_kit::{generate_bsp, BspParams}`.
- `noise::fbm_2d` / `noise::fbm_1d`: fractional Brownian motion — sum `octaves`
  layers of value noise at doubling frequency / halving amplitude, renormalised
  to `[0, 65535]`. This is the standard primitive for natural terrain; it was
  previously hand-rolled in `noise_terrain_demo`, which now calls the library
  function (output unchanged). Frequency shifts and the amplitude taper are
  bounded so any octave count is panic-free and deterministic. Re-exported at the
  crate root.
- `easing`: extended the Penner family with twelve new integer easers —
  `ease_{in,out,in_out}_quart` (t⁴), `_quint` (t⁵), `_sine` (via the CORDIC
  `Fixed::sin_cos`), and `_circ` (via `Fixed::sqrt`). All float-free and
  bit-identical across targets, matching the kit's determinism guarantee. Brings
  easing coverage from 6 to 18 functions; all re-exported at the crate root.
- `content::Color` operations: `Color::rgb` (const constructor), `lerp(a, b,
  num, den)` (integer-ratio per-channel interpolation for heat-map gradients and
  fades), `grayscale()` (integer Rec. 601 luma), and `scale(num, den)`
  (ratio dimming/brightening). All integer-only and clamping — replaces the
  hand-rolled channel math the demos previously needed.
- `aabb::Aabb` region queries: `area()` (saturating `w*h`), `is_empty()`,
  `center()` (matching `mapgen::Rect::center`'s top-left bias), `contains(&Aabb)`
  (rect-in-rect, boundary-inclusive), and `iter_points()` — row-major iteration
  over the interior cells, closing the parity gap with `bracket-geometry`'s
  `Rect::for_each`/`point_set` for filling and scanning rectangular regions.
- `vec::Vec2` rotation and interpolation: `rotate(angle)` (2-D rotation matrix
  driven by the fixed-point CORDIC `Fixed::sin_cos` — no float, deterministic),
  `angle()` (vector heading via `Fixed::atan2`), `lerp(a, b, t)`,
  `distance(rhs)` and `distance_sq(rhs)`. Closes the gap where the kit shipped
  fixed-point trig but vectors could not rotate or report their heading — the
  core operation for steering, projectiles, and tweening. `Vec3` gains the
  matching `lerp` / `distance` / `distance_sq` for parity.
- `examples/cave_spawn_demo.rs`: ties the three new primitives into one screen — `generate_cave` carves a 64×20 connected cavern, a depth-scaled `RandomTable<Spawn>` (monster weights rising with depth, items flat) scatters monsters and items onto floor cells, and `Distance::Chebyshev` tints the player's awareness radius and counts nearby monsters. Fully deterministic from one seed; the right panel lists the table's weights. (`cargo run --example cave_spawn_demo`)
- `mapgen::generate_cave` / `mapgen::CaveParams`: organic cave generation via the
  **cellular-automata** method (RogueBasin "cave-like levels"; cf. `bracket-lib`
  ch.27 and `rot.js` `Cellular`), complementing the rectangular
  `generate_dungeon`. Deterministic three-stage pipeline driven by `SplitMix64`:
  (1) random seed at `wall_percent`, border kept solid; (2) `steps` passes of the
  4-5 rule (cell becomes wall with 5+ wall neighbours, out-of-bounds counts as
  wall); (3) connectivity cull — every floor cell outside the largest 4-connected
  region is filled back to wall, so the returned cave is guaranteed to be a single
  connected space with no isolated pockets. Returns the same `Dungeon` type (with
  `rooms` empty), so it plugs straight into `fov`/`pathfinding` via
  `Dungeon::is_wall`. Re-exported as `izanagi_kit::{generate_cave, CaveParams}`.
- `random_table::RandomTable<T>`: weighted random selection table for loot drops
  and depth-scaled spawn tables — the canonical roguelike pattern (cf. the *Rust
  Roguelike Tutorial* `random_table.rs` and `bracket-lib`). A typed layer over
  `SplitMix64::weighted_index` that *owns* the candidate values, so `roll` yields
  the value directly. Builder (`with`) and in-place (`push`) APIs; entries with
  weight 0 are stored but never selected; an empty/all-zero table rolls `None`
  without drawing, so selection stays replay-deterministic (exactly one draw per
  non-empty roll). `DetHash` (gated on `T: DetHash`) folds the table config into
  world/replay hashes. Re-exported as `izanagi_kit::RandomTable`.
- `geometry::Distance`: grid distance metrics (`Manhattan`, `Chebyshev`,
  `EuclideanSquared`, `Euclidean`) via `Distance::between(a, b)` — mirrors
  `bracket-lib`'s `DistanceAlg` for picking a metric that matches movement rules
  (Manhattan for 4-way, Chebyshev for 8-way, the Euclidean pair for radial
  range). `Euclidean` returns the floored integer distance using an integer
  square root (Newton's method), so it remains float-free and cross-platform
  deterministic; all metrics saturate rather than overflow. Re-exported as
  `izanagi_kit::Distance`.

- `change::Changed::reset(tick)`: acknowledge a change without modifying the
  wrapped value — sets `changed_at` to `tick` so a subsequent
  `is_changed_since(tick + 1)` returns `false`. The canonical "mark as seen"
  operation for multi-pass systems that should not re-process a change they
  already handled.
- `change::ChangeTracker::reset()`: reset the tick counter to `0`. Needed when
  restoring world state from a save file or rewinding a replay so that all
  `is_changed_since` queries treat restored state as fresh.
- `profiler::Profiler::budget_exceeded(section, budget)`: returns `true` if the
  current-tick total for `section` exceeds `budget`. Zero-overhead convenience
  check for per-frame budget monitoring ("did pathfinding exceed 1 ms this
  tick?") without repeating the `this_tick > N` comparison at each call site.
- `profiler::EventLog::filter_by_tick_range(start, end)`: iterate entries whose
  tick falls within `[start, end]` (inclusive), oldest first. Enables replay
  inspection and assertion patterns like "what events occurred during turns 5–10?"
- `autotile::compute_region(x, y, w, h, is_same)`: compute auto-tile masks for
  only the `w × h` subregion starting at `(x, y)` rather than the entire map.
  `is_same` receives absolute coordinates so diagonal-corner clearing works
  correctly across region boundaries. Use this after any terrain mutation to
  avoid recalculating masks for the whole map.
- `fsm::Fsm::transitions_from(state)`: iterate all events with a defined
  outgoing transition from `state`. Returns `&E` in insertion order. Does not
  include self-loops for unmapped events — only explicitly added transitions.
  Useful for "available actions" UIs and AI planners that need to enumerate what
  can happen from a given state.
- `inputbuf::InputBuffer::all_held()`: iterator over all currently held key
  values. The missing modifier-query API: "is Shift/Ctrl held while I process
  this event?" without having to track modifier keys separately.
- `cmdqueue::CmdQueue::drain_if(pred)`: drain only commands matching a
  predicate; unmatched commands stay in the queue in their original order.
  Enables per-system selective consumption when multiple subsystems share one
  queue (e.g. movement commands vs. UI commands).
- `menu::Menu::set_enabled(idx, enabled)`: toggle the disabled flag on an item
  at runtime. Fills the gap between create-time `add_disabled` and run-time
  "grey out unavailable actions" (shop items you can't afford, abilities on
  cooldown). Silently ignores out-of-range indices.
- `menu::Menu::find_by_label(label)`: linear search for the first item with
  a matching label, returning `Some(idx)` or `None`. Case-sensitive. The missing
  link for search-and-jump-to-item patterns in large menus.
- `arch::ArchTable::retain(pred)`: remove all entries for which `pred(entity,
  &row)` returns `false`. Iterates in reverse dense-array order so swap-removes
  stay O(n) — the canonical "cull dead entities" frame-cleanup primitive,
  matching `Vec::retain` semantics. Parallel to `SparseSet::retain`.
- `arch::ArchTable::values()` / `values_mut()`: entity-handle-free row iterators.
  Mirrors the `SparseSet::values()`/`values_mut()` additions — avoids unpacking
  the entity handle when only component data is needed (e.g. ticking all AI rows).
- `content::Color::invert()`: channel-wise complement `rgb(255−r, 255−g, 255−b)`.
  `const` fn, zero-cost. Useful for selection highlights, contrast checks, and
  visual effects. Double inversion is the identity: `c.invert().invert() == c`.
- `content::Color::luminance()`: perceived luma as a single `u8` using integer
  Rec. 601 weights — the same value as each channel of `grayscale()`, exposed
  directly. Avoids constructing a full `Color` when only the brightness scalar is
  needed (e.g. "is the foreground readable on this background?").
- `loader::Stats`: numeric stats component. Loaded from each spawned prefab's
  `stats` BTreeMap in deterministic (alphabetical) order. Stored in
  `LoadedLevel::stats` (a `SparseSet<Stats>`); absent for prefabs with no stats so
  the common case (no stat block) pays no memory. `get(key)` / `iter()` / `len()`
  / `is_empty()` API. `DetHash`-capable for world hashing.
- `textlayout::wrap_words_max_lines(text, max_cols, max_lines)`: like `wrap_words`
  but caps the output at `max_lines`. When overflow occurs, the last visible line is
  truncated with `…` to signal cut-off content. Useful for bounded text areas such
  as in-game book pages, dialogue boxes, and log panels.
- `hud::HudPanel::split_h(n)` / `split_v(n)`: divide a panel into `n` equal
  horizontal strips (top-to-bottom) or vertical strips (left-to-right). Heights /
  widths are distributed by integer division; any remainder goes to the first strip.
  Returns an empty `Vec` for `n == 0`. Enables declarative multi-pane HUD layouts
  without hard-coding pixel offsets.
- `validator::validate`: glyph printability check — control characters (U+0000–
  U+001F, U+007F) in a prefab or tile glyph are reported as errors. Terminal cells
  have no sensible rendering for control chars; the check prevents invisible/corrupt
  glyphs from reaching `loader::load_level`.
- `validator::validate`: unused-prefab warning — prefabs defined but never
  referenced by any level spawn emit a `Severity::Warning`. Helps authors catch
  dead definitions and renamed-but-not-updated spawn tables.
- `passability::PassabilityGrid::is_passable(x, y)`: convenience inverse of
  `is_blocked` — returns `true` when the cell is open, `false` when blocked or
  out-of-bounds. Eliminates the repetitive `!grid.is_blocked(…)` negation at
  call sites where the positive condition ("can the actor step here?") is cleaner
  to read.
- `camera::Camera::set_screen_size(screen_w, screen_h, world_w, world_h)`:
  resize the viewport dimensions and re-clamp so the full viewport stays within
  the world. The world-space centre is preserved so the view expands
  symmetrically — the correct behaviour for terminal `SIGWINCH` handlers where
  the game redraws after a resize.
- `relations::Relations::detach_all_children(parent)`: orphan every direct child
  of `parent`, making them root entities, and return them as a `Vec<Entity>`.
  Leaves `parent`'s own parent relationship intact. The canonical "drop all
  carried items on death" pattern — call it, then dispatch the returned entities
  to the spawner or inventory-drop system.
- `msglog::MsgLog::get(index)`: random access by logical position (0 = oldest).
  Returns `None` for `index >= len`. Complements `iter()` when a scrollable log
  cursor needs to read a specific line without collecting the full iterator.
- `msglog::MsgLog::first()`: the oldest visible message, or `None` if empty.
  Mirrors `last()` — useful for "show oldest unread event" patterns.
- `msglog::MsgLog::retain(pred)`: keep only messages for which `pred` returns
  `true`, preserving oldest-to-newest order. Rebuilds the ring buffer in-place;
  capacity is unchanged. Useful for filtering combat-only vs. exploration events
  in a multi-channel log.
- `tilemap::TileMap::contains(x, y)`: `true` if `(x, y)` is within map bounds.
  Saves the `get(x,y).is_some()` pattern; works as a bounds guard before calling
  `get_mut` or `swap`.
- `tilemap::TileMap::swap(x1, y1, x2, y2)`: exchange tiles at two cells. No-op
  if either coordinate is out of bounds or both are equal. Useful for sliding-
  puzzle mechanics and map editing without a temporary variable.
- `tilemap::TileMap::count_where(pred)`: count cells for which `pred` returns
  `true`. The idiomatic way to answer "how many walls?", "how many lit cells?",
  etc. without a manual `iter().filter().count()` chain.
- `tilemap::LayeredMap::fill_all(tile)`: fill every cell of every layer with
  `tile`. The single-call equivalent of iterating `layer_mut(i).fill(tile)` for
  all layers — useful for resetting the entire world state on scene transitions.
- `timer::Cooldown::set_ready()`: instantly set `remaining = 0` so
  `is_ready()` returns `true` on the next check. Used by "haste" effects and
  level-up resets that clear all ability cooldowns.
- `timer::Cooldown::elapsed(original)`: ticks consumed from the original charge
  (`original.saturating_sub(remaining)`). Turns a cooldown into a forward
  progress counter for UI fill-bars without storing the original value twice.
- `timer::TimerQueue::peek_next()`: the minimum `remaining` ticks across all
  scheduled entries, or `None` if empty. Answers "when does the next event fire?"
  for UI countdown labels and headless simulation fast-forward.
- `timer::TimerQueue::count_repeating()`: count entries added via
  `schedule_repeat`. Useful for diagnostics and save-file inspection that need
  to distinguish persistent periodic timers from one-shot callbacks.
- `entity::EntityAllocator::live_entities()`: returns all currently allocated
  (not freed) entities as a `Vec<Entity>` in ascending index order. Essential
  for serialisation, debug overlays, and end-of-frame cleanup passes that need
  to enumerate every spawned entity without iterating a component store.
- `fixed::Fixed::floor()`: round toward negative infinity. Implemented with
  arithmetic-shift-right then shift-left — no branches, no overflow.
- `fixed::Fixed::ceil()`: round toward positive infinity. No-op on integers;
  adds one unit then floors otherwise.
- `fixed::Fixed::round()`: round to nearest integer, ties round away from
  negative infinity (0.5 → 1, -0.5 → 0).
- `fixed::Fixed::fract()`: fractional part `self - self.floor()`. Always in
  `[0, 1)` — follows the floor convention rather than IEEE 754's sign-preserving
  convention.
- `replay::Divergence`: implements `Display` — formats as "replay divergence at
  tick N: expected 0x…, got 0x…". Allows desync reports to be printed directly
  in logs and error messages without manual formatting.
- `geometry::rect(x, y, w, h)`: all cells in the rectangle `[x, x+w) × [y, y+h)`
  in row-major order — returns empty for non-positive dimensions. The natural
  primitive for filling rectangular map regions and iterating tile areas without
  a manual nested loop. Mirrors `bracket-lib`'s `Rect::for_each`/`point_set`
  patterns.
- `geometry::ring_annulus(cx, cy, inner_r, outer_r)`: all integer-coordinate points
  whose squared Euclidean distance from `(cx, cy)` falls in `[inner_r², outer_r²]`.
  A negative `inner_r` is treated as 0 (filled circle). Returns empty if
  `outer_r ≤ 0` or `inner_r ≥ outer_r`. Useful for area-of-effect rings, blast
  radii, and torchlight annuli without float sqrt.
- `wfc::WfcRules::get_allowed(tile, dir)`: read back the adjacency bitmask for a
  tile+direction pair — the symmetric counterpart of `allow`/`disallow`. Returns `0`
  for out-of-range arguments. Enables serialising rule sets and writing assertions
  on rule construction without re-exposing the internal `adj` array.
- `wfc::WfcGrid::count_tiles(tile)`: count collapsed cells whose value equals
  `tile`. Useful for post-collapse assertions and debugging ("how many floor tiles
  were generated?").
- `wfc::WfcGrid::to_vec()`: export the grid as `Vec<Option<u8>>` in row-major order.
  `Some(t)` for cells collapsed to tile `t`; `None` for cells still in superposition
  or contradiction. The ergonomic bridge between the WFC representation and downstream
  `TileMap`/renderer code.
- `fsm::Fsm::remove_transition(from, event)`: remove a specific `(from, event)`
  entry from the transition table. No-op if absent. Enables runtime AI behaviour
  modification without rebuilding the entire FSM (e.g. temporarily strip all
  "FleeFromPlayer" transitions to create a berserk state).
- `fsm::Fsm::clear_transitions()`: remove all transitions, leaving the current
  state unchanged. Simplifies the "stunned / frozen" pattern where every event
  must be a self-loop until a recovery signal fires.
- `menu::Menu::select_by_label(label)`: find the first item with a matching
  label, move the cursor there, and return its value (or `None` if not found or
  disabled). The "click by name" primitive for script-driven menus and tests.
- `menu::Menu::next_enabled()`: return the index of the next enabled item after
  the current cursor (wrapping), without moving the cursor. Useful for "preview
  next selection" UI indicators and accessible navigation lookaheads.
- `textlayout::measure_lines(lines)`: return `(max_width, line_count)` for a
  slice of `&str` lines in one allocation-free pass. Allows layout engines to
  size containers before rendering.
- `textlayout::pad_lines(lines, width)`: pad every line to `width` columns using
  `pad_right`, returning a uniform-width block. The standard step before
  rendering a bordered text panel where all content rows must be the same width.
- `hud::HudPanel::merge(panels)`: compute the smallest enclosing `HudPanel` for
  a slice of panels. Returns `None` for an empty slice. Useful for computing
  composite HUD region bounding boxes in layout code.
- `influence::InfluenceMap::fill(value)`: set every cell to `value`. The
  inverse of `clear()` — used to establish a non-zero baseline (e.g. neutral
  territory, ambient light level) before layering sources on top.
- `influence::InfluenceMap::find_peaks(threshold)`: return `(x, y)` coordinates
  of all cells whose value is ≥ `threshold`, in row-major order. The standard
  "gather candidate targets / spawn points" AI query without a manual iteration.
- `change::ChangeTracker::delta_since(last_tick)`: ticks elapsed since
  `last_tick` (`current − last_tick`, saturating). Eliminates the repetitive
  subtraction at call sites — the canonical "has N ticks passed since last
  action?" check for cooldown logic and idle-trigger patterns.
- `cmdqueue::CmdQueue::prepend(cmd)`: insert a command at the front of the
  queue (LIFO priority). For "abort / interrupt" commands that must be processed
  before already-queued actions.
- `cmdqueue::CmdQueue::retain(pred)`: keep only commands for which `pred`
  returns `true`, discard the rest in-place. The complement of `drain_if` — use
  when you want to filter without taking ownership of discarded items.
- `inputbuf::InputBuffer::reset_hold(key)`: reset the hold counter for a key to
  `0` without releasing it. The key stays pressed but the repeat timer restarts,
  so the next fire will re-trigger the initial press. For interrupt patterns such
  as "player jumps mid-repeat — pause movement until the key is re-evaluated."
- `spatial_hash::SpatialHash::contains(key, x, y)`: targeted membership test —
  `true` if `key` is registered in the cell that contains world point `(x, y)`.
  O(k) where k is the cell occupancy (usually very small). Avoids the
  `query_cell(x,y).contains(&key)` pattern that unpacks a slice unnecessarily.
- `spatial_hash::SpatialHash::density(x, y)`: entity count in the cell
  containing `(x, y)`. Returns `0` for absent cells. Allocation-free crowding
  heuristic used by mob AI and spawn systems to avoid over-populated cells.
- `spatial_hash::SpatialHash::iter_cells()`: iterate all non-empty cells as
  `(cell_coord, &[K])` pairs. Enables bulk serialisation and rendering passes
  that want to enumerate every occupied cell without repeated spatial queries.
- `savefile::SaveHeader::new(version)`: `const fn` constructor for `SaveHeader`.
  Removes `SaveHeader { version }` struct-literal boilerplate at call sites and
  enables `const` save-header constants in user code.
- `savefile::load_bytes_owned(data)`: like `load_bytes` but returns an owned
  `Vec<u8>` payload. Useful when the caller cannot hold a reference to the raw
  buffer long enough, or when the payload must outlive the source slice.
  Re-exported at the crate root.
- `noise::value_noise_1d_wrap(x, seed, period)`: 1-D value noise that tiles
  seamlessly at integer period `period`. Completes the wrap family alongside
  `value_noise_2d_wrap` — the missing primitive for seamless 1-D terrain height
  profiles, audio ramps, and scrolling textures.
- `noise::fbm_1d_wrap(x, seed, octaves, period)`: tileable 1-D FBM, mirroring
  `fbm_2d_wrap`. Each octave tiles at `period << octave_index` so all harmonics
  also tile. Returns `[0, 65535]`; `octaves == 0` returns `0`. Re-exported at
  the crate root alongside the other noise functions.
- `terminal::Screen::draw_line(from, to, glyph, fg, bg)`: draw a Bresenham line
  from `from` to `to` (both endpoints inclusive), setting every visited cell to
  `glyph`/`fg`/`bg`. Out-of-bounds cells are silently clipped via the existing `set`
  contract. Delegates to `geometry::line` for the coordinate sequence — the standard
  primitive for laser beams, aiming cursors, and debug overlays.
- `aabb::Aabb::clamp_point(px, py)`: return the nearest point inside the AABB to
  `(px, py)`. Points already inside are returned unchanged; points outside are
  clamped to the boundary. An empty box returns its origin `(x, y)`. The standard
  primitive for confining entity positions to a region without overlap checks.
- `rng::SplitMix64::reseed(seed)`: replace the generator state with `seed`,
  restarting the stream from that point. Equivalent to constructing a new
  `SplitMix64::new(seed)` but in-place — the canonical "restart simulation from
  a new random seed" operation without reallocating the RNG struct.
- `rng::SplitMix64::next_bool()`: return a uniformly random `bool` (50/50
  coin flip) via `coin(1, 2)`. Consumes one draw so the stream advances
  deterministically; mirrors the `coin` method but for callers who want a direct
  boolean rather than a conditional.
- `turn::Scheduler::speed(id)`: return the current speed for actor `id`, or
  `None` if not registered. The read-only counterpart to `set_speed` — useful
  for status displays ("Actor X has speed 150") and save-file serialisation.
- `turn::Scheduler::iter_actors()`: iterate all registered actor ids in insertion
  order. Does not drive the turn queue. Used for serialisation, debug overlays,
  and tests that need to enumerate every actor independently of turn order.
- `profiler::Profiler::section_count(section)`: number of `record()` calls for
  `section` in the current (unflushed) tick. Returns `0` for an unknown or silent
  section. Complements `this_tick` with a call-frequency view — useful for
  detecting runaway per-frame invocations ("pathfinding was called 47 times this
  tick").
- `profiler::Profiler::min(section)`: all-time minimum single-call elapsed for
  `section`. Persists across `begin_tick` calls, mirroring `peak`. Returns `0` for
  an unknown section. Enables displaying the best-case timing alongside `peak` in
  profiler overlays and automated regression checks ("min pathfinding latency
  regressed above baseline").
- `dice::Dice::roll_n_keep_highest(n, keep, rng)`: roll `n` copies of the
  expression, sum the `keep` highest results (classic "4d6 drop lowest" ability-
  score mechanic). `keep` is clamped to `min(keep, n)`; `n == 0` or `keep == 0`
  returns the modifier without drawing. Consumes exactly `n` draws. Sum saturates.
- `relations::Relations::path_to_root(entity)`: return the full ancestor chain
  from `entity` to its root, inclusive — `[entity, parent, grandparent, …, root]`.
  Single-element for root entities. Useful for debug visualisation, access-control
  checks, and any system that needs to walk the ownership hierarchy.
- `relations::Relations::find_common_ancestor(e1, e2)`: lowest common ancestor of
  two entities, or `None` if they share no ancestor (disjoint trees). Builds
  `path_to_root(e1)` as the search set, then walks `e2`'s chain. Essential for
  evaluating whether two entities are in the same "inventory tree" or
  ownership group.
- `passability::PassabilityGrid::set_region(x1, y1, x2, y2, blocked)`: bulk-set
  all cells in the axis-aligned rectangle `[x1, x2] × [y1, y2]` (both endpoints
  inclusive, coordinates in any order). Out-of-bounds cells are silently clipped.
  Equivalent to calling `set_blocked` in a nested loop but O(area) without
  allocation — the standard primitive for sealing doors, opening corridors, and
  placing rectangular obstacles.
- `entity::EntityAllocator::batch_free(entities)`: free multiple entities in one
  call. Stale and duplicate entries are silently ignored (same contract as `free`).
  Enables frame-end bulk cleanup ("free all dead entities this tick") without an
  explicit loop at each call site.
- `multimap::MultiMap::find_connector_to(dest_floor)`: return the first
  `Connector` whose `to_floor` matches `dest_floor`, or `None`. The "find the
  staircase that returns to floor N" navigation primitive — avoids scanning
  `exits_from` on every floor when the caller only knows the destination.
- `status::StatusSet::max_remaining()`: the longest remaining duration across
  all active effects (0 when empty). Complements `total_magnitude` with a
  duration-based aggregate — useful for "how long before all buffs wear off?" UI.
- `status::StatusSet::first_expiring()`: the key and remaining ticks of the
  effect that expires soonest (`None` when empty). Predictive UI primitive for
  "next status change in N ticks" indicators and AI decisions.
- `inventory::Inventory::count_where(pred)`: count occupied slots for which
  `pred` returns `true`. Avoids `.iter().filter().count()` boilerplate for
  common queries like "how many potions?", "how many identified items?".
- `inventory::Inventory::first_empty_slot()`: index of the first unoccupied slot,
  or `None` if full. Non-consuming (read-only). Used for "show next free slot"
  UI indicators and pre-flight add-item checks.
- `msglog::MsgLog::contains(needle)`: `true` if any stored message contains
  `needle` as a substring. One-liner for the common "has the player seen X yet?"
  check without an explicit `iter().any()` chain.
- `msglog::MsgLog::count_where(pred)`: count messages matching a predicate.
  Useful for "how many combat hits this session?" counters and test assertions
  on log content.
- `keymap::KeyMap::get_keys_for_action(action)`: all keys currently bound to
  `action` in insertion order (empty `Vec` if none). The reverse-lookup
  complement of `get` — required for "which key opens inventory?" UI tooltips
  and conflict-detection during control remapping.
- `keymap::KeyMap::swap_bindings(key1, key2)`: atomically exchange the actions
  at two keys. If only one is bound the action migrates to the unbound key; if
  neither is bound it is a no-op. The canonical one-call rebind primitive.
- `change::Changed::was_written_at(tick)`: `true` iff `changed_at == tick` —
  exact-match check stricter than `is_changed_since`, for "did this change THIS
  frame?" patterns without risk of matching earlier unprocessed ticks.
- `change::Changed::ticks_since_change(current_tick)`: saturating elapsed ticks
  since the last `mark` call (`current_tick − changed_at`). Per-component age
  query without passing `ChangeTracker` everywhere — use for cache invalidation,
  freshness indicators, and idle-trigger conditions.
- `combat::Stats::hp_percent()`: current HP as an integer percentage `[0, 100]`.
  Uses `u64` intermediate arithmetic to avoid overflow for large HP values; clamps
  at 100; returns 0 for dead or zero-`max_hp` actors. Replaces the common
  `(hp * 100 / max_hp)` one-liner which overflows for `hp > i32::MAX / 100`.
- `combat::Stats::take_overkill_damage(amount)`: apply `amount` damage and return
  how many hit points overshot zero (overkill). Negative amounts are treated as
  zero. Useful for chaining "excess damage propagates" mechanics (e.g. shield →
  HP spillover, cleave damage to adjacent targets, one-hit-kill detection).
- `camera::Camera::contains_rect(l, t, r, b)`: test whether the world-space
  axis-aligned rectangle `[l, r) × [t, b)` (exclusive right/bottom) overlaps the
  camera's current viewport. Empty rects (`l >= r` or `t >= b`) always return
  `false`. Standard broad-phase visibility cull for particle systems, light
  sources, and HUD markers — cheaper than projecting each corner with
  `world_to_screen`.
- `camera::Camera::chebyshev_to_center(wx, wy)`: Chebyshev ("king-moves") distance
  from the viewport's world-space centre to `(wx, wy)`. `max(|cx−wx|, |cy−wy|)` —
  the natural metric for 8-directional grids. Returns `0` when the point equals
  the centre. Useful for depth-of-field / fog-of-war intensity calculations and
  radial scroll-speed curves without branching on octant.
- `sparse_set::SparseSet::count_matching(pred)`: count entries for which
  `pred(&value)` returns `true`. Non-allocating (no intermediate `Vec`);
  equivalent to `.values().filter(pred).count()` but named for clarity at call
  sites ("how many poisoned entities?", "how many full-HP enemies?").
- `timer::TimerQueue::reschedule(pred, new_delay)`: cancel the first pending entry
  matching `pred` and immediately re-schedule its event as a one-shot at
  `new_delay` ticks. Returns `true` if an entry was found. The rescheduled entry
  is always one-shot regardless of whether the original was a repeating timer —
  for "reset patrol timer without losing the event" patterns.
- `easing::ease_in_expo` / `ease_out_expo` / `ease_in_out_expo`: exponential
  easing family — the last major Penner group. Implements `2^(10·(t−1))` via
  integer bit-shift for the power-of-two component and a 3-term Taylor series
  for the fractional part (max error ≈ 0.4%). Exact 0 at t=0, exact 1 at t=1.
  Re-exported at the crate root alongside the other easing functions.
- `geometry::rect_perimeter(x, y, w, h)`: cells on the outer border of the
  rectangle `[x, x+w) × [y, y+h)` in row-major order. Degenerates to `rect` for
  1-wide or 1-tall inputs. Cell count is `2*(w+h-2)` for w,h ≥ 2. Fills the gap
  between `rect` (solid fill) and `ring_annulus` (Euclidean ring): the
  axis-aligned rectangular outline needed for dungeon-room borders and UI panels.
- `geometry::diamond(cx, cy, r)`: all cells at exactly Manhattan distance `r` from
  `(cx, cy)` — the "4-directional blast ring". Returns empty for `r < 0`, just
  the centre for `r == 0`, and `4·r` cells (in ascending x,y order) otherwise.
  Complements `circle` (Bresenham outline) and `ring_annulus` (Euclidean annulus)
  with the Manhattan-metric ring needed for 4-way movement range queries.
- `noise::normalize_noise(v, lo, hi)`: map a noise value from the standard
  `[0, 65535]` output range to any integer range `[lo, hi]`. Returns `lo` for
  degenerate ranges (`lo >= hi`), saturates at `hi` for `v == 65535`. Eliminates
  the repetitive `lo + v * range / 65535` one-liners at call sites.
- `lib.rs`: re-export `ease_in_back`, `ease_out_back`, `ease_in_out_back`,
  `ease_in_bounce`, `ease_out_bounce`, `ease_in_out_bounce` at the crate root
  (these were implemented in easing.rs but not yet re-exported).
- `arch::ArchTable::swap_rows(entity1, entity2) -> bool`: exchange the row data
  of two entities in-place (O(1) — two indexed moves, no search). Returns `true`
  when both entities are present; `false` if either is absent (no partial swap).
  Useful for sort-swap passes and "trade equipment" mechanics without a temporary
  staging variable.
- `wfc::WfcRules::allowed_count(tile, dir) -> usize`: count of permitted
  neighbour tiles for `(tile, dir)` — the popcount of the adjacency bitmask.
  The cheapest entropy proxy before running full WFC: the lower the count, the
  more constrained the slot. Returns `0` for out-of-range arguments.
- `easing::ease_smoothstep(t) -> Fixed`: classic cubic Hermite smoothstep
  `3t² − 2t³`. Zero first derivative at both endpoints; monotonically increasing
  in `[0, 1]`. Equivalent to the GLSL `smoothstep` shape without branching or
  float. Useful for camera lerp, fade-in/out, and any curve where a gentle
  ease-in/ease-out is needed without Penner's asymmetry.
- `camera::Camera::follow(wx, wy, margin, world_w, world_h)`: lazy camera follow
  — pan the minimum distance needed to keep world point `(wx, wy)` within the
  inner rectangle that is `margin` cells inside all four viewport edges. No-op
  when the target is already inside the inner rect. Pans via the existing
  `pan(dx, dy, …)` so world-boundary clamping is automatic. The standard
  "keep the player on screen" primitive for roguelikes without snap-to-center.
- `relations::Relations::subtree_size(entity) -> usize`: count of all entities
  in the subtree rooted at `entity` (inclusive). Equivalent to
  `1 + descendants_of(entity).len()`. Single-node (leaf/root) returns `1`;
  useful for inventory-weight and hierarchy-depth budgets.
- `loader::Stats::set(key, value)`: insert a new stat or overwrite the value of
  an existing one. Preserves all other entries. The runtime "apply buff" primitive
  — lets systems write stat changes back without remove+re-insert. Complements
  the read-only `get` / `iter` API.
- `world_hash::Fnv1a::write_i64(value)` and `impl DetHash for i64`: extends the
  integer-write family (`write_u32`, `write_u64`, `write_i32`) with signed 64-bit
  support. Useful for hashing large tick counters, cumulative damage totals, or
  other wide simulation state fields. `write_i64` folds the 8 little-endian bytes
  identically across targets; `DetHash for i64` delegates to it.
- `influence::InfluenceMap::clamp_cells(min, max)`: clamp every cell value to
  `[min, max]` in-place. Prevents influence values from growing unbounded when
  many sources overlap, and implements "floor of zero" for threat maps that must
  not go negative. O(n) in map size; no allocation.
- `spatial_hash::SpatialHash::query_rect_count(x, y, w, h) -> usize`: count of
  keys in any cell overlapping the rectangle without allocating a `Vec` — the
  allocation-free complement to `query_rect(…).len()`. Returns `0` for
  non-positive `w` or `h`. Suitable for hot broad-phase budget checks.
- `passability::PassabilityGrid::fill(blocked)`: set every cell to `blocked`
  without reallocation. The single-call "seal entire map" / "clear all walls"
  primitive before carving a new layout. O(n) in map size.
- `cmdqueue::CmdQueue::index(i) -> Option<&C>`: read the command at position
  `i` (0 = oldest) without consuming it. The random-access complement to `peek()`;
  equivalent to `peek().get(i)`. Does not advance the queue.
- `random_table::RandomTable::set_weight(idx, weight)`: update the weight of
  the entry at `idx`; adjusts `total_weight` to stay consistent. Setting to `0`
  makes an entry permanently unselectable ("sold out") without removing it from
  the table. Silently ignores out-of-range indices.
- `replay::find_all_divergences(expected, actual) -> Vec<Divergence>`: collect
  every diverging tick into a `Vec`, unlike `first_divergence` (stops at first)
  or `count_divergences` (count only). Ticks beyond the shorter trace count as
  divergences. Re-exported at the crate root.
- `profiler::EventLog::last() -> Option<&LogEntry<E>>`: the most recently pushed
  entry, or `None` for an empty log. Equivalent to `recent(1).last()` but avoids
  constructing an iterator. The "show last event" status-line primitive.
- `autotile::count_set_bits(mask: u8) -> u32`: popcount of an 8-bit auto-tile
  neighbour mask — `0` = isolated, `8` = fully surrounded interior. Delegates to
  `u8::count_ones`. Useful for terrain classification without decoding individual
  bit directions.
- `assets::AssetStore::find_all_by<F>(pred) -> Vec<AssetHandle<T>>`: collect
  handles of all live assets matching `pred`, in ascending index order. The plural
  complement to `find_by` — use when multiple assets can satisfy a condition.
  Returns an empty `Vec` when the store is empty or no assets match.
- `hud::BarWidget::empty_cells() -> u32`: unfilled cell count (`width −
  filled_cells()`). The complement of `filled_cells` — returns the number of
  blank segments in a health/mana bar without a manual subtraction at every
  call site.
- `diag_json::diag_count(diags) -> (usize, usize)`: count `(errors, warnings)`
  in a diagnostic slice in one pass. Avoids two separate `filter().count()` calls
  when both totals are needed (e.g. CI gate: "fail if errors > 0, warn if
  warnings > 0").
- `terminal::Screen::draw_double_box(x, y, w, h, fg, bg)`: draws a double-line
  box border using Unicode box-drawing characters `╔╗╚╝═║`. Interior untouched;
  out-of-bounds positions silently clipped via `set`. The "dialogue / title"
  variant of `draw_box` — standard in roguelike UIs for emphasising important
  panels.
- `vec::Vec2::from_angle(angle: Fixed) -> Vec2`: construct a unit vector from a
  heading angle (radians as Q16.16 `Fixed`). Delegates to `Fixed::sin_cos`
  (CORDIC) — `(x: cos, y: sin)`. Float-free and deterministic. The inverse of
  `angle()`, enabling angle→vector round-trips for steering, projectile launch,
  and FOV cone calculations.
- `noise::hash_3d(x, y, z, seed) -> u32`: deterministic 3-D integer hash —
  folds `z` into the seed with a mixing multiply, then delegates to `hash_2d`.
  Consistent with `hash_2d` in distribution; distinct from it for any non-zero
  `z`. Useful for voxel terrain, layered dungeon generation, and 3-D particle
  systems.
- `geometry::chebyshev_ring(cx, cy, r) -> Vec<(i32, i32)>`: all cells at
  exactly Chebyshev distance `r` from `(cx, cy)` — the rectangular perimeter
  `rect_perimeter(cx-r, cy-r, 2r+1, 2r+1)`. Returns empty for `r < 0`, the
  single centre cell for `r == 0`. Complements `diamond` (Manhattan ring) with
  the 8-directional movement ring needed for king-move range queries and aura
  effects.
- `timestep::FixedTimestep::accumulator_ns() -> u64`: current sub-step time
  buffered in the accumulator (always in `[0, step_ns)`). Exposes the value
  normally hidden inside the struct so callers can serialise it alongside
  `total_steps` for an exact save/restore of the timestep state — or verify
  that two replay runs are at identical accumulator positions.
- `aabb::Aabb::expand_to_include(px, py) -> Aabb`: grow the box to include
  point `(px, py)`. Points already inside return `self` unchanged; an empty
  `Aabb` becomes a 1×1 box at the point. Saturating arithmetic; never creates
  an invalid AABB. Useful for computing aggregate bounding boxes over a stream
  of points without pre-collecting them.
- `fixed::Fixed::is_zero() -> bool`, `is_positive() -> bool`,
  `is_negative() -> bool`: inline sign-predicate helpers. Eliminate the
  `.raw() == 0` / `.raw() > 0` / `.raw() < 0` patterns at call sites —
  especially useful in guard clauses and branch predicates where the
  intent ("the velocity is positive") is clearer than the raw comparison.
- `rng::SplitMix64::with_state(state: u64) -> Self`: construct a generator
  from a raw state snapshot — the inverse of `state()`. Semantically
  distinct from `new(seed)`: restoring an exact stream position rather than
  seeding from a game-world integer. The canonical "load RNG from save file"
  constructor.
- `pathfinding::path_cost(path: &[(i32, i32)]) -> i32`: total octile cost of
  a path — sums `octile_distance` for each consecutive step pair. An empty
  or single-cell path has cost 0. Matches the A* internal scale (10 ortho,
  14 diag), so the result is directly comparable to A* `g`-scores and
  heuristic estimates.
- `tilemap::TileMap::find_first<P>(pred) -> Option<(i32, i32)>`: first
  matching cell in row-major order, or `None`. The single-result complement
  of `find_all` — avoids allocating a full `Vec` when only the first match
  is needed (e.g. "where is the exit?").
- `combat::apply_resistance(damage: i32, resist_percent: u32) -> i32`:
  reduce damage by a percentage resistance (`max(0, damage*(100−r)/100)`),
  clamping `resist_percent` to `[0, 100]`. Negative damage returns 0.
  Complements `base_damage` (subtraction model) with a percentage-based
  defensive layer common in action RPGs and card games.
- `mapgen::Dungeon::floor_cells() -> Vec<(i32, i32)>`: all floor cells in
  row-major order. The primary convenience primitive for spawn placement when
  the full list is needed at once — avoids a manual `is_floor` scan at every
  call site.
- `easing`: elastic family — `ease_in_elastic`, `ease_out_elastic`,
  `ease_in_out_elastic`. Penner elastic curves: the in-variant oscillates
  below 0 near `t=0` (spring pull-back), the out-variant overshoots above
  1 near `t=1` (spring finish). Computed with the existing CORDIC `sin` and
  Taylor-series `exp2` (same primitives as the expo family). Float-free and
  deterministic. Brings total easing coverage to 21 functions.
- `fov::fov_count(origin, radius, is_opaque) -> usize`: count visible cells
  without allocating a `Vec`. Equivalent to `fov_to_vec(...).len()` but
  skips the intermediate allocation. Use for broad-phase lighting-budget
  queries and per-frame FOV coverage checks.
- `combat::Stats::is_full_hp() -> bool`: `true` when `hp >= max_hp`. The
  complement of `is_bloodied` — used for "suppress heal prompt" guards,
  "skip regen tick" optimisations, and post-battle status summaries.
- `easing::ease_smootherstep(t) -> Fixed`: Ken Perlin's quintic smootherstep
  `6t⁵ − 15t⁴ + 10t³`. Zero first and second derivative at both endpoints;
  eliminates the visible curvature kink of the cubic `smoothstep` at t=0 and
  t=1. The standard choice for terrain-gradient interpolation and high-quality
  camera lerp.
- `noise::turbulence_2d(x, y, seed, octaves) -> u32`: turbulence (absolute-value
  FBM) — folds each octave through `raw.abs_diff(32768)` (mid-fold) before
  accumulating, producing sharp discontinuities analogous to Perlin's original
  turbulence. Normalised to `[0, 65535]`. Builds fire textures, cloud detail,
  and marble-vein heightmaps from the same deterministic value-noise primitive.
- `dice::Dice::is_flat() -> bool`: `true` when `sides <= 1`, meaning every roll
  returns the same value (`count + modifier`). The standard "is this a fixed-
  value entry?" check for loot-table normalisers and display code that skips
  the `d` notation for non-random entries.
- `timer::Cooldown::is_near_ready(threshold) -> bool`: `true` when
  `remaining <= threshold`. Lets UI code highlight an ability that is "almost
  ready" (e.g. glow when ≤ 2 ticks remain) without a manual comparison at
  every render site.
- `timer::TimerQueue::count_where<P>(pred) -> usize`: count scheduled entries
  whose event satisfies `pred`. Non-consuming; complements `cancel_where` with
  a read-only query — useful for "how many heal-over-time ticks are pending?"
  diagnostics and assertions.
- `msglog::MsgLog::join(sep) -> String`: concatenate all log messages
  (oldest-to-newest) with `sep` between each pair. Allocation-lazy: builds
  exactly one `String`. Useful for serialising the full log to a single line,
  snapshot testing, and embedding recent messages in a status report.
- `inventory::Inventory::contains_where<F>(pred) -> bool`: `true` if any
  occupied slot satisfies `pred`. Short-circuits on the first match.
  The "do I have at least one potion?" check without index arithmetic or
  collecting a temporary `Vec`.
- `fixed::Fixed::is_integer() -> bool`: `true` when the fractional part is
  exactly zero (whole-number value). Equivalent to `self.fract().is_zero()`.
  Useful for grid-snapping guards ("only move if position is on a cell
  boundary") and animation completion checks.
- `rng::SplitMix64::within_rect(x, y, w, h) -> (i32, i32)`: uniform random
  grid point in `[x, x+w) × [y, y+h)`. Draws exactly two values for valid
  rectangles; returns corner `(x, y)` without drawing for degenerate `w ≤ 0`
  or `h ≤ 0`. The canonical random-spawn primitive for bounded rooms.
- `world_hash::Fnv1a::write_u16(v: u16)` + `impl DetHash for u16`: fills
  the gap between `u8` and `u32` in the write family. Folds two little-endian
  bytes into the hasher. Useful for 16-bit tile IDs, screen dimensions, and
  colour channel pairs that need to participate in world checksums.
- `timestep::FixedTimestep::total_time_ns() -> u64`: total nanoseconds of
  simulation time stepped so far (`total_steps × step_ns`, saturating). Used
  for save files that record an in-game clock and for time-based event triggers
  that fire after N nanoseconds of simulation.
- `geometry::vec_toward(from, to) -> (i32, i32)`: unit direction vector from
  `from` toward `to` — each component is `−1`, `0`, or `+1`. Returns `(0, 0)`
  for same-point input. The cheapest "which way should I face?" primitive for
  enemy AI and melee direction indicators. Re-exported at the crate root.
- `pathfinding::flood_fill(start, max_dist, is_blocked) -> Vec<(i32,i32)>`:
  BFS collecting all passable cells reachable from `start` within `max_dist`
  steps (8-directional, no corner-cutting). Returns cells in BFS order
  (deterministic — fixed compass neighbour order). Use for "reveal connected
  room", "spread fire", and "count reachable floor cells" patterns.
  Re-exported at the crate root.
- `terminal::Screen::draw_h_line(x, y, len, glyph, fg, bg)`: draw a
  horizontal run of `len` cells all set to `glyph`/`fg`/`bg`. Out-of-bounds
  cells are silently clipped. The common separator line primitive for HUD
  panels and dialogue box dividers — avoids the allocation overhead of
  `draw_line` for the purely horizontal case.
- `serializer::diff(a, b) -> Vec<String>`: collect **all** semantic
  differences between two `Content` values in discovery order. Unlike
  `first_diff` (stops at first divergence), `diff` provides a complete picture
  — useful for "show all errors after a failed round-trip" debugging and
  structured test output. Returns an empty `Vec` when `content_eq(a, b)`.
- `aabb::Aabb::half_extents() -> (i32, i32)`: `(w / 2, h / 2)` — integer
  half-dimensions. Mirrors `center()`'s floor bias. Avoids repeating the `/2`
  arithmetic at every symmetric collision or spawn-offset call site.
- `fov::fov_circle(origin, radius, is_opaque) -> Vec<(i32,i32)>`: visible cells
  within an Euclidean circle (`dx² + dy² ≤ radius²`). Filters `fov_to_vec`
  to a strict disc rather than the Chebyshev square. Returns empty for
  `radius < 0`. Re-exported at the crate root alongside `fov_to_vec`.
- `hud::HudPanel::is_landscape() -> bool`: `true` when `w >= h`. Simple layout
  predicate for adaptive UI: split wide panels horizontally, tall panels
  vertically, without hard-coding orientation at each call site.
- `content::Color::max_channel() -> u8`: `max(r, g, b)`. Useful for
  normalising a colour to full brightness and as an overflow guard when
  brightening — the "value" in HSV terms but computed per-channel.
- `tilemap::TileMap::fill_border(tile)`: set all cells on the outermost
  perimeter (top row, bottom row, left and right columns) to `tile`, leaving
  the interior unchanged. Equivalent to four `fill_rect` calls; the standard
  "seal the dungeon in walls" primitive for post-generation clean-up.
- `camera::Camera::viewport_area() -> u32`: `screen_w × screen_h` (saturating).
  Useful for pre-frame draw-call budgets, renderer pre-allocation, and
  "is the map larger than the screen?" checks without two separate reads.
- `combat::Stats::is_dead() -> bool`: `self.hp <= 0` — the positive complement
  of `is_alive`. Eliminates `!is_alive()` double-negation at death-trigger,
  loot-drop, and "remove dead entities this tick" cleanup sites.
- `random_table::RandomTable::average_weight() -> u32`: integer `total /
  len`, or `0` for an empty table. Useful for "is this table roughly uniform?"
  distribution checks and depth-scaling balance heuristics without manual
  iteration.
- `influence::InfluenceMap::min_value() -> Option<i32>`: minimum value in any
  cell (`None` for a zero-size map). The counterpart to `max_value` — useful for
  "flee from minimum threat" AI and normalising an influence layer to a `[0, 1]`
  range via `(v − min) / (max − min)`.
- `spatial_hash::SpatialHash::cell_coord_from_world(x, y) -> (i32, i32)`:
  expose the internal Euclidean-division world→cell mapping as a public method.
  Lets callers reason about cell boundaries without replicating the formula and
  confirms that `insert(k, x, y)` and `query_cell(x, y)` use the same mapping.
- `status::StatusSet::min_remaining() -> u32`: shortest remaining duration
  across all active effects (0 when empty). Mirrors `max_remaining` — useful for
  "how soon will a buff wear off?" UI and "apply debuff only if it exceeds the
  current shortest" AI guards.
- `relations::Relations::child_at_index(entity, idx) -> Option<Entity>`:
  `idx`-th direct child of `entity` in insertion order, or `None` if fewer than
  `idx + 1` children exist. Avoids the `Vec` allocation of `children_of` when
  only a single child is needed (random child selection, next-in-sequence).
- `change::ChangeTracker::is_at_tick(target: u32) -> bool`: `true` when the
  current tick equals `target`. Concise predicate for "fire exactly on tick N"
  checkpoint and cutscene triggers without a manual `== target` comparison.
- `inputbuf::InputBuffer::count_repeating() -> usize`: count of keys currently
  in the repeating phase (held past `initial_delay`). Useful for "slow time while
  any key auto-repeats" and "disable menu animation during rapid input" effects
  without allocating a `Vec`.
- `cmdqueue::CmdQueue::count<F>(pred) -> usize`: count queued commands matching
  `pred` without draining the queue. The exact-count mirror of `contains` — use
  for rate-limiting guards that allow at most N pending commands of a given type.
- `loader::LoadedLevel::find_entity_at(x, y) -> Option<Entity>`: return the
  first entity at grid position `(x, y)`, or `None` if the cell is empty.
  Iterates the `positions` sparse-set in insertion order; makes no uniqueness
  assumption. The position-to-entity reverse-lookup needed by interaction and
  line-of-sight systems.
- `profiler::Profiler::sections_count() -> usize`: number of distinct profiling
  sections recorded at least once. Diagnostics and save-file headers that need
  the profiler footprint without enumerating each name via `section_names`.
- `replay::divergence_percent(expected, actual) -> u32`: integer percentage
  (0–100) of ticks at which two traces diverge; ticks past the shorter trace
  count as divergences. Empty traces return `0`. For "reject if > N% diverged"
  CI gates and replay-quality dashboards without floating-point.
- `savefile::SaveHeader::is_compatible(current_version) -> bool`: `true` when
  the header's version matches `current_version`. The canonical "can I load
  this save?" guard before deserialising the payload; `false` signals a needed
  migration or an incompatible-save error.
- `noise::turbulence_1d(x, seed, octaves) -> u32`: 1-D turbulence (absolute-value
  FBM) mirroring `turbulence_2d`. Folds each octave through `|raw − 32768|`
  before accumulating, normalised to `[0, 65535]`. Builds sharp height-line
  profiles and audio-style ramps from the same deterministic value-noise base.
- `easing::ease_clamp(t, ease_fn) -> Fixed`: apply `ease_fn` to `t` after
  clamping it to `[0, 1]`. Guard for callers that cannot guarantee `t ∈ [0, 1]`
  (timers that overshoot, progress values that briefly exceed 1). Equivalent to
  `ease_fn(t.clamp01())`; generic over any `Fn(Fixed) -> Fixed`.
- `geometry::rect_center(x, y, w, h) -> (i32, i32)`: centre cell of the
  rectangle via floor division (`(x + w/2, y + h/2)`) — same truncation bias as
  `Aabb::center()`. The standalone complement to `midpoint` for "where is the
  middle of this room?" spawn placement and camera targeting.
- `fixed::Fixed::abs_diff(self, other) -> Fixed`: saturating absolute difference
  `|self − other|`. Avoids the sign-loss of a saturated subtraction followed by
  `abs`. The correct primitive for "how far apart are two fixed-point values?"
  range checks.
- `wfc::WfcGrid::possibilities_at(x, y) -> usize`: count of remaining tile
  possibilities at a cell — `0` for out-of-bounds or contradiction, `1` when
  fully collapsed, `> 1` for superposition. Exposes WFC entropy for debugging
  and custom collapse strategies.

### Fixed
- `content::parse_color`: no longer panics on a 7-byte input containing a
  multi-byte UTF-8 char (e.g. `#aéABC`). It now parses hex straight from bytes
  instead of slicing the `&str` at fixed offsets, restoring the parser's
  panic-free guarantee. Regression covered in unit tests and the garbage-input
  fuzz.
- `fixed::from_int`: saturates instead of silently flipping sign for
  out-of-range inputs (`from_int(32768)` previously wrapped to `i32::MIN` via an
  unchecked `<< 16`), closing the last hole in the "never wrap to the opposite
  extreme" invariant.
- `fixed::from_ratio`: a zero denominator now saturates toward the numerator's
  sign (consistent with `div`) rather than panicking with a divide-by-zero, and
  an out-of-range quotient clamps via `from_wide` instead of a wrapping cast.
- `rng::below(0)`: now returns `0` without consuming a draw, identically in
  debug and release. Previously the bound check was `debug_assert!`-only, so
  release builds could silently advance the stream and desync a replay.
- `ci`: the security-audit job now runs `cargo generate-lockfile` before
  `cargo audit`; `Cargo.lock` is `.gitignored`, so the job previously failed
  with "Couldn't find Cargo.lock" on every run.

### Refactored
- `profiler`: extracted a private `Profiler::section(name) -> Option<&Section>`
  helper and routed the read-only accessors (`this_tick`, `peak`,
  `section_count`, `min`) through it, replacing four copies of the
  `iter().find(|s| s.name == ...)` scan with one. Behaviour and the `DetHash`
  output are unchanged; pinned hashes verified identical.
- `spatial_hash`: `insert`, `remove`, and `query_cell` now delegate to the
  public `cell_coord_from_world` helper instead of computing `cell_coord(x)` /
  `cell_coord(y)` inline, consolidating the world→cell mapping on one code path.
  Pure cleanup — identical cell math, no API or hash change.
- `relations`: extracted a private `children_iter(entity)` iterator and routed
  `children_of`, `child_count`, `child_at_index`, and `remove_entity` through
  it, collapsing four copies of the `self.children` filter-by-parent scan into
  one. Behaviour and `DetHash` output unchanged; pinned hashes verified.
- `inventory`: `find` now builds on the existing `iter()` occupied-slot
  iterator instead of re-implementing the `enumerate().filter_map(as_ref)`
  scan. Same index-order first-match semantics; no API or hash change.
- `assets`: `len`, `is_empty`, `handles`, `remove_where`, and `retain` now
  all route through the canonical `iter()` / `find_all_by()` methods, eliminating
  five separate `slots.iter().enumerate().filter_map(...)` slot-scanning expansions.
  `len` → `iter().count()`, `is_empty` → `iter().next().is_none()`,
  `handles` → `iter().map(|(h, _)| h)`, `remove_where` → `find_all_by(pred)`
  + remove loop, `retain` → `iter().filter(!pred).collect()` + remove loop.
  Behaviour identical; no API or hash change; pinned hashes verified.
- `fsm`: extracted a private `Fsm::find_transition(event) -> Option<&(S,E,S)>` helper
  that searches `table` for `(self.state, event, _)`. Routed `fire`, `has_transition`,
  and `peek_next` through it, collapsing three copies of the
  `table.iter().find(|(f,e,_)| *f==self.state && *e==*event)` predicate into one.
  Also routed `transition_count` through the existing public `transitions_from`,
  removing a parallel `filter().count()` scan. No behaviour change; pinned hashes
  verified.

### Changed
- `README.md`: documented the seven runnable examples (`cargo run --example …`) and refreshed the module overview from 11 to a representative cross-section of the ~50 shipped modules, linking to `GAME_DEV_TAXONOMY.md` and `SPEC.md` for the full capability map and contracts.

### Added
- `examples/timestep_demo.rs`: fixed-timestep accumulator demo. A 9-frame variable real-frame trace (17/17/8/9/50/500/17/16/33 ms) is fed into a 60 Hz `FixedTimestep` (max 5 catch-up steps). The left panel tabulates, per frame, the steps emitted, the leftover accumulator (as a fraction-of-step bar), and the interpolation `alpha`. The 500 ms stall (f5) would demand 30 steps but is **CLAMPED** to 5 (death-spiral guard), and f6 then takes exactly 1 step — proving no catch-up debt is carried over. The right panel also demonstrates frame-pacing independence: the same total time delivered as one big frame vs 400 tiny frames yields an identical `total_steps` (100), confirming render cadence cannot change simulation results. Exercises `FixedTimestep::new`/`advance`/`step_ns`/`total_steps`/`alpha_ratio`. (`cargo run --example timestep_demo`)
- `examples/hud_panels_demo.rs`: HUD widgets demo. Lays out a four-panel RPG character-status screen (VITALS / ATTRIBUTES / PROGRESS / STATUS EFFECTS) entirely from `HudPanel`, `BarWidget`, and `StatLine`. Each `HudPanel` owns a bordered region whose content origin comes from `inner_x`/`inner_y`/`inner_w` (1-cell margin). `BarWidget::render` draws fill bars with distinct fill glyphs (HP `█`, MP `▓`, Stamina `▒`, XP `·`, effects `■`); `StatLine::render`/`with_unit` format attributes and unit-tagged progress values (gold gp, depth floors, time min). A cursor point is hit-tested against all four panels via `HudPanel::contains` and a tooltip is positioned with `HudPanel::translate`. `filled_cells` is reported in the stats line. (`cargo run --example hud_panels_demo`)
- `examples/asset_store_demo.rs`: generational-handle asset store demo. Four `Sprite` assets are inserted into an `AssetStore<Sprite>`, yielding opaque `AssetHandle<Sprite>` keys. The demo recolours one via `get_mut`, upgrades one via `replace` (returning the old value), then `remove`s the torch — invalidating its handle. Two stale-handle reads are then shown to be rejected: `get(torch)` after removal returns `None` (no use-after-free), and after a new `amulet` insert reuses the freed slot with a bumped generation, the old torch handle still returns `None` rather than silently aliasing the amulet. The left panel renders the live sprite table (handle index, coloured glyph, name) plus a sprite-sheet strip; the right panel logs each operation with ok/REJ/info markers. Final: live=4, both stale gets rejected 2/2. (`cargo run --example asset_store_demo`)
- `examples/archetype_demo.rs`: archetype-storage ECS demo. Two `ArchTable<Row>` tables model a swarm: `Mobile{pos,vel,fuel}` and `Static{pos}`. A movement system walks the dense `Mobile` table with `iter_mut` (cache-friendly SoA iteration), integrates positions, bounces entities off arena walls, and decrements fuel; when an entity runs out of fuel it **migrates** Mobile→Static via O(1) `remove` (swap-remove) + `insert`. Five entities are seeded (three with fuel < ticks migrate, two outlive the run and stay mobile); the arena renders trails plus live `●` mobile and `▣` rested markers, with both archetype tables' contents and a migration log shown on the right. `world_hash::hash_state` produces a canonical per-table checksum (independent of swap-remove order). Final: mobile=2, static=3, migrations=3. (`cargo run --example archetype_demo`)
- `examples/multimap_demo.rs`: multi-floor dungeon demo. Generates three deterministic floors via `generate_dungeon` (per-floor `SplitMix64` seeds), wraps them in a `MultiMap`, and wires stair `Connector` records between consecutive floors (down-stair at floor N's last-room centre → up-stair at floor N+1's first-room centre). All three floors render side by side with the active floor (index 1) brightened; `>` descend and `<` ascend glyphs are placed purely from the connector table via `exits_from`, never hard-coded into the map. `connector_at` is probed on the floor-0 down-stair. Exercises `MultiMap::new`/`floor`/`current`/`set_floor`/`floor_count`/`add_connector`/`exits_from`/`connector_at`. (`cargo run --example multimap_demo`)
- `examples/relations_demo.rs`: entity relationship tree demo. Builds a 6-entity ownership forest (Hero wielding Sword[socketed Rune] + Shield, commanding a Familiar[Spark]) via `Relations::attach`, then exercises every query: `parent_of`/`children_of` navigation, `depth`/`is_root`/`is_leaf`/`is_ancestor` structural checks. A cycle attempt `attach(Hero→Rune)` is rejected (returns `false`), shown REJ in red. `remove_entity(Familiar)` orphans Spark (becomes a root), which is then reparented onto Hero via a second `attach`. The left panel renders the final hierarchy as an indented DFS tree coloured by role (root=gold, node=blue, leaf=green) with per-node depth; the right panel logs each operation with ok/REJ/info markers. Final: relations=4, hero_children=3, spark_depth=1. (`cargo run --example relations_demo`)
- `examples/autotile_demo.rs`: bitmask auto-tiling demo. A 26×14 hand-drawn `#` dungeon is auto-tiled into connected box-drawing walls purely from each cell's 8-bit neighbour mask — no manual tile placement. `compute_all(w, h, is_same)` produces every cell's mask in one row-major pass; a 256-entry `SimpleTileTable` reduces each mask to a 4-bit cardinal index (N/E/S/W) which indexes a box-drawing glyph table (`─│┌┐└┘├┤┬┴┼`). The left panel shows the raw `#` map, the right panel the auto-tiled result, with a cardinal-bit glyph legend. `compute_mask(x, y, is_same)` spot-checks one cell in the stats line (probe(5,4) → `0b01000100` → `─`). The diagonal-corner-clearing rule is handled inside `compute_mask`. (`cargo run --example autotile_demo`)
- `examples/input_pipeline_demo.rs`: deterministic input pipeline demo. A 13-tick scripted session demonstrates the three-layer input abstraction: `InputBuffer<char>` (initial_delay=1, repeat_period=2) tracks held keys — initial presses fire immediately (yellow), hold-repeats fire after the delay then every period (orange). `KeyMap<char, Action>` translates raw fired chars to `Action` variants (`MoveNorth/South/East/West`, `Wait`, `OpenInventory`, `Descend`, `Quit`). `CmdQueue<Action>` accumulates translated actions during a tick and drains them in one batch at the tick boundary. The left panel shows all 8 bindings; the right panel shows the per-tick log with yellow=initial and orange=repeat colour coding. Final stats: 13 ticks, 10 commands (6 moves, 2 repeats). (`cargo run --example input_pipeline_demo`)
- `examples/geometry_easing_demo.rs`: geometry and easing curves demo. Left panel (60 cols) renders a two-room dungeon (`RA` cols 0–20, `RB` cols 28–58, corridor cols 20–28 rows 5–8) built from `Aabb` constants. Eight LOS rays from viewer `@` at (10,6) are drawn using `geometry::line`; each ray's visibility is determined by `geometry::line_of_sight` (interior cells only). Five rays land green (visible), three are blocked red. Right panel shows 8-step sparklines for all six easing functions (`linear`, `ease_in_quad`, `ease_out_quad`, `ease_in_out_quad`, `ease_in_cubic`, `ease_out_cubic`) evaluated over `Fixed::from_ratio` with `Fixed::raw()` mapped to block characters `▁▂▃▄▅▆▇█`. Bottom rows report `Aabb::overlaps`, `Aabb::intersection`, `Aabb::contains_point` for four test rectangles and LOS summary counts. (`cargo run --example geometry_easing_demo`)
- `examples/camera_viewport_demo.rs`: camera viewport and world-navigation demo. A 120×36 `TileMap<u8>` (border walls + horizontal/vertical corridor walls with gaps) is viewed through a 60×18 `Camera` viewport embedded in an 80×24 screen. A ten-step scripted player path drives `Camera::recenter` each tick; the camera clamps to the world boundary twice (right edge at t=3, top edge at t=6). `PassabilityGrid::from_tilemap` builds the blocker layer and reports passable/blocked cell counts. `Changed<(i32,i32)>` + `ChangeTracker` demonstrate dirty-flag tracking per sim tick. `MsgLog` (capacity 18) records human-readable step events; `EventLog<MoveEvent>` (capacity 24) records structured events (Stepped + CameraClamp) which are destructured in the stderr summary. `Profiler` records four named work-unit sections (world_gen, pass_build, path_sim, render) with peak query. (`cargo run --example camera_viewport_demo`)
- `examples/menu_textlayout_demo.rs`: character-class selection screen demonstrating `Menu<T>` and the five text-layout helpers. Builds a six-item `Menu<CharClass>` (five enabled classes, Paladin disabled with `add_disabled`), simulates four `move_down()` calls — the fourth auto-skips the locked Paladin entry and lands on Shaman — then renders a two-panel 80×24 screen: left panel lists all classes with a `▶` cursor on the selected row (using `pad_right` to fill each label to the panel width), right panel shows the class name centred with `center`, the description word-wrapped with `wrap_words` (4 lines at 50 cols), and a stats row with `pad_right`/`pad_left` for aligned key/value pairs. The bottom hint bar is clipped to 79 chars with `truncate`, showing the ellipsis. (`cargo run --example menu_textlayout_demo`)
- `examples/ai_behavior_demo.rs`: AI behaviour demo. Four guards patrol a 55×20 dungeon room, each owning a `Fsm<GuardState, GuardEvent>` (Idle → Alert → Chase → Dead), a `Cooldown` (attack rate gate), and a `TimerQueue<()>` (patrol beat, fires every 5 ticks to advance to the next waypoint). `SpatialHash<u32>` (cell_size=4) tracks all entity positions; each tick `query_rect(player ± detect_r)` finds nearby guards and fires `PlayerSpotted`/`PlayerLost` events into each FSM. Chasing guards step toward the player; attacking guards reset their `Cooldown` and trigger player retaliation. The map renders guards coloured by FSM state (g=Idle, G=Alert, C=Chase, %=Dead) with a detection-zone background tint; the right panel shows guard state, HP, CD, and a timestamped event log. (`cargo run --example ai_behavior_demo`)
- `examples/status_effects_demo.rs`: status effects, inventory, and turn scheduling demo. Three actors (Hero / Orc / Mage) fight over 60 scheduler ticks. Demonstrates `StatusSet<K>` (Regen, Haste, Poison applied/ticked/expired), `Inventory<T>` (HealthPotion / Antidote / SpeedDraught consumed in-combat), `Scheduler<u32>` (speed-weighted turn order with runtime `set_speed` for Haste), `Stats` + `melee_attack` / `ranged_attack` (integer combat), and `BarWidget` HP fill-bar rendering. Left panel shows final actor state; right panel shows the turn event log with colour-coded entries (hits orange, heals green, deaths red). (`cargo run --example status_effects_demo`)
- `examples/noise_terrain_demo.rs`: procedural terrain demo. Generates an 80×22 biome map using 3-octave fractional Brownian motion (`value_noise_2d`) and `hash_2d` for sparse landmark scatter. Seven biomes (deep water → coast → sand → grass → forest → mountain → snow) with 24-bit background gradients and a biome-distribution legend bar. (`cargo run --example noise_terrain_demo`)
- `examples/content_pipeline_demo.rs`: content pipeline demo. Runs the full `parse → validate → load_level → render` pipeline on an embedded DSL bundle (hero, goblin, potion, dungeon room). Side by side: left panel shows entity overlay on the tile grid; right panel shows parser diagnostics for an intentionally broken DSL — matching `gamec` human-mode output. (`cargo run --example content_pipeline_demo`)
- `examples/influence_demo.rs`: influence-map + HUD demo. Generates a dungeon, seeds an `InfluenceMap` with monster sources (positive, r=10) and trap sources (negative, r=6), renders the scalar field as a 24-bit heat-map (blue→grey→red), overlays a HUD panel with `BarWidget`/`StatLine` widgets and `highest_neighbour`/`lowest_neighbour` steering arrows. (`cargo run --example influence_demo`)
- `fixed::Fixed::to_int_round()`: round to nearest integer and return as `i32`.
  Combines `round()` + `to_int_trunc()` in a single call — the natural counterpart
  to the existing `to_int_trunc()` for callers that want normal rounding semantics.
- `fixed::Fixed::min(other)` / `max(other)`: component-wise saturating minimum and
  maximum. Deterministic ordering — same as `Ord`-based comparison on the raw `i32`
  representation. The missing scalar clamp primitives; complement `Vec2::min`/`max`
  with the same API on the underlying scalar type.
- `fixed::Fixed::recip()`: multiplicative reciprocal `1 / self`. Implemented as
  `Fixed::ONE.div(self)` — inherits the existing saturating-division contract
  (division by zero returns `Fixed::MAX` or `Fixed::MIN` by sign). Useful for
  computing inverse speeds and normalisation denominators without a double divide.
- `rng::SplitMix64::skip(n)`: advance the stream by `n` draws without returning
  values. Useful for fast-forwarding a seeded replay to a known position, or
  skipping over the draws used by a sub-system without branching on the draw
  count. Consumes exactly `n` draws; `skip(0)` is a no-op.
- `turn::Scheduler::actors_ready()`: returns all actor IDs whose banked energy is
  ≥ `ACTION_COST` — i.e. every actor that would be returned by the next
  `next_actor()` call if ties were broken differently. Does not advance the queue.
  Useful for "who can act this tick?" batch queries in parallel-resolution
  turn modes and test assertions.
- `turn::Scheduler::reset_actor(id)`: set the banked energy for `id` to `0`,
  effectively spending a full turn's worth of energy. The canonical "this actor
  just acted — deduct energy" primitive. No-op for unknown actors. Mirrors the
  `set_energy` contract but named for clarity at call sites.
- `pathfinding::step_toward(from, goal, is_blocked)`: single-step A* helper —
  returns the first waypoint on the shortest path from `from` toward `goal`, or
  `None` if `from == goal` or no path exists. The canonical "monster chases player"
  one-liner that wraps `astar` and plucks `path[1]`. `is_blocked` must return
  `true` for out-of-bounds coordinates to bound the search. Re-exported at the
  crate root as `izanagi_kit::step_toward`.
- `inventory::Inventory::is_full()`: shorthand for `!has_space()` — `true` when
  every slot is occupied. Eliminates the double negation at call sites where the
  positive "full?" condition is clearer than "not has_space?".
- `status::StatusSet::extend_duration(key, added_ticks)`: add ticks to the
  remaining duration of an active effect. Saturating. No-op when the effect is
  not active. The "haste doubles buff duration" primitive without requiring a
  `remove` + `apply` pair that would lose the current magnitude.
- `status::StatusSet::count_with(pred)`: count effects for which
  `pred(key, &effect)` is `true`. Allocation-free alternative to
  `iter().filter().count()` — the natural query for "how many buffs active?",
  "how many debuffs with magnitude ≤ −5?".
- `tilemap::TileMap::find_all(pred)`: collect `(x, y)` of all cells for which
  `pred(&tile)` returns `true`, in row-major order. Closes the gap between
  `count_where` (count only) and `iter()` (full iteration with manual collect).
  Mirrors `bracket-lib`'s `TileMap::find_all` semantics.
- `assets::AssetStore::clear()`: remove all assets in one call, bumping every
  occupied slot's generation so all existing handles are permanently invalidated.
  Repopulates the free list with all indices so subsequent inserts reuse slots.
  O(n) — implements the "flush the asset cache" pattern without a loop over
  individual `remove` calls.
- `assets::AssetStore::find_by(pred)`: return the handle of the first live asset
  for which `pred(&asset)` is `true` (ascending index order), or `None`. The
  standard "where is my player sprite?" asset-manager lookup without requiring
  callers to maintain their own reverse index.
- `aabb::Aabb::corners()`: return all four corners in clockwise order from
  top-left — `[top-left, top-right, bottom-right, bottom-left]` — as
  inclusive-coordinate `(i32, i32)` pairs. Empty boxes return all four equal to
  `(x, y)`. Useful for raycasting, decal placement, and bounding-polygon tests.
- `aabb::Aabb::distance_to_point(px, py)`: Chebyshev distance from `(px, py)` to
  the nearest cell on (or inside) the box. Returns `0` for points inside or for
  empty boxes. The standard "how far is this entity from the room?" query for
  aggro radius, fog-of-war, and attraction-field seeding.
- `menu::Menu::prev_enabled()`: symmetric counterpart to `next_enabled()` —
  return the index of the previous enabled item (wrapping backwards), or `None`
  when all items are disabled. Does not move the cursor. The missing reverse-
  navigation lookahead for "show previous selectable" UI indicators and
  accessible backward-traversal patterns.
- `menu::Menu::count_enabled()`: number of selectable (non-disabled) items.
  Allocation-free alternative to `.iter().filter(|(_, it)| !it.disabled).count()`.
  Useful for "are any items available?" checks and progress indicators ("3 of 5
  abilities unlocked").
- `inputbuf::InputBuffer::release_all()`: release all currently held keys at
  once — semantic alias for `clear()`, named for focus-loss handlers where the
  intent is "simulate a key-up for every pressed key." Ensures no held state
  survives a window-blur or scene transition.
- `fsm::Fsm::peek_next(event)`: returns `Option<&S>` — the state this FSM would
  transition to if `event` were fired now, without actually changing state.
  Returns `None` for unmapped events (self-loop). Useful for AI lookahead ("would
  attacking now trigger Dead?") and "show next state" UI tooltips without
  committing the transition.
- `change::ChangeTracker::set_tick(tick)`: set the tick counter to an explicit
  value. The companion to `reset()` (which always resets to `0`): needed when
  restoring exact simulation state from a save file to preserve the tick offsets
  recorded in all `Changed<T>` components.
- `textlayout::justify(s, width)`: typographic full-justification — distribute
  spaces between words so the line fills exactly `width` columns. Extra spaces
  (when `total_spaces % gaps != 0`) are placed left-to-right. Single-word lines
  and lines wider than `width` fall back to `pad_right`. Re-exported at the crate
  root as `izanagi_kit::justify`.
- `cmdqueue::CmdQueue::pop_front()`: remove and return the front (oldest /
  first-in) command, or `None` if empty. O(n) — shifts remaining elements. For
  the "process exactly one command per tick" rate-limiting pattern without
  draining the entire queue.
- `cmdqueue::CmdQueue::pop_back()`: remove and return the back (newest /
  last-in) command, or `None` if empty. O(1). For LIFO / "cancel last queued
  command" patterns.
- `examples/savefile_demo.rs`: save-file framing demo. Exercises four scenarios — clean round-trip, corrupted payload byte (→ `ChecksumMismatch`), truncated buffer (→ `TooShort`), version mismatch (→ caller-side rejection) — with a hex dump of the 20-byte framing header rendered to the terminal. (`cargo run --example savefile_demo`)
- `examples/replay_demo.rs`: replay & desync-detection demo. Exercises all four `replay` primitives: `record_trace`, `check_trace`, `first_divergence`, `resimulate`. Runs four checks (clean replay → OK, wrong seed → diverges at tick 0, tampered hash → diverges at the correct tick, rollback resimulate == direct run), renders a green/red ANSI report card, and exits with code 1 on any failure — making it directly usable as a smoke test. (`cargo run --example replay_demo`)
- `examples/wfc_demo.rs`: Wave Function Collapse terrain demo. Defines a 5-tile biome tileset (deep water→shallow→sand→grass→mountain) with gradient adjacency constraints, generates an 80×44 fully-collapsed grid via `wfc_solve`, computes grass-layer autotile masks via `compute_all`, and renders an 80×24 24-bit-ANSI snapshot with a tile legend. (`cargo run --example wfc_demo`)
- `examples/roguelike_demo.rs`: runnable terminal demo wiring the full kit end-to-end. Generates a dungeon (seed `0x5EED_1234`), places a player (`@`) and monster roster (`g`, `G`, `o`, `r`, `T`), runs 200 energy-scheduler ticks, and renders two full-colour ANSI snapshots (initial state + final state) using `terminal::Screen`, `Camera`, `MsgLog`, `compute_fov`, `astar`, and `melee_attack`. The demo has zero OS dependencies and runs clean in CI as well as in real colour terminals. (`cargo run --example roguelike_demo`)
- `tests/roguelike_sim.rs`: end-to-end composition determinism proof. A realistic turn-based roguelike loop wires together `mapgen`, `passability`, `pathfinding` (A*), `fov`, `turn` (scheduler), and `combat` (melee): monsters path toward and attack the player, the player retaliates, and a per-turn world hash folds positions, HP, scheduler state, and the player's visible-cell count. Asserts same-seed → bit-identical 400-turn trace, different-seed divergence, non-triviality, and pins a final hash (`0x5286_d142_0200_fe66`, identical in debug and release) as a regression tripwire for the gameplay-module stack — complementing `determinism.rs` which pins the core RNG+fixed+storage stack. 4 new integration tests.
- `passability`: grid-based passability / collision layer (`PassabilityGrid`) (K1). Stores a flat `Vec<bool>` grid (true = blocked, false = passable). Constructors: `new` (all passable), `from_fn(w, h, |x,y|…)`, `from_tilemap(map, is_blocked_pred)`, `from_dungeon(dungeon)`. `is_blocked(x,y)` returns `true` for out-of-bounds. `set_blocked` for runtime mutation. `blocker()` produces a borrow closure directly passable to `astar` / `weighted_astar`. Also: `blocked_count`, `passable_count`, `len`, `is_empty`, `DetHash`. Upgrades K1 from 🔶 to ✅. 15 new tests.
- `savefile`: versioned binary save-file framing (N2/N3). `save_bytes(header, payload)` encodes a `SaveHeader { version: u32 }` + arbitrary payload into a self-describing buffer: magic `b"IZNG"`, version (LE u32), FNV-1a 64-bit checksum (LE u64), payload length (LE u32), payload bytes. `load_bytes(data)` validates magic, checksum, and declared length, returning `(SaveHeader, payload_slice)` or a `LoadError` (TooShort / BadMagic / ChecksumMismatch). The caller uses `header.version` to implement forward/backward compatibility (N3). Zero external deps; pure standard library. Satisfies taxonomy N2 and N3. 13 new tests.
- `arch`: archetype-based component storage (`ArchTable<Row>`) (C4). Stores `(Entity, Row)` pairs in a dense contiguous array for O(n) cache-friendly iteration; a secondary `HashMap<Entity, usize>` index provides O(1) insert/lookup/remove. `Row` is a caller-defined struct bundling multiple component fields (the archetype row), decoupled from `SparseSet` single-component storage. Swap-remove keeps the array packed. API: `insert(entity, row) -> bool`, `remove(entity) -> Option<Row>`, `get/get_mut`, `contains`, `len/is_empty`, `iter/iter_mut`, `clear`. `DetHash` sorts by entity index for canonical replay-safe hashing. Satisfies taxonomy C4. 14 new tests.
- `wfc`: Wave Function Collapse procedural tile-map generation (I5). `WfcRules` stores per-tile adjacency bitmasks (up to 64 tile types as `u64` bits, 4 cardinal directions); `allow(tile, dir, neighbor)` and `allow_symmetric` build the rule set. `wfc_solve(w, h, rules, rng)` entropy-guided BFS collapse: finds the minimum-entropy cell, picks a random tile (`SplitMix64`), propagates constraints via AC-3-style BFS queue, and returns `WfcResult::Ok(WfcGrid)` or `WfcResult::Contradiction`. `WfcGrid` provides `tile_at(x,y)`, `is_fully_collapsed()`, `iter_collapsed()`, `len`, `DetHash`. All arithmetic is integer (bitmask popcount for entropy). Deterministic: identical seed/rules → identical output. 14 new tests.
- `diag_json`: machine-readable JSON serialization of pipeline diagnostics (P4). `diag_json(file, diags)` emits a self-contained JSON object with `file`, `diagnostics` array (`severity`, `line`, `col`, `message` per entry), `errors`, and `warnings` counts. All string values are correctly JSON-escaped (quotes, backslashes, control chars). `gamec` gains a `--json` flag that outputs this format to stdout and suppresses human-readable stderr output, making it consumable by CI tools, editors, and LSP clients. Zero external dependencies; pure hand-written JSON building. Satisfies taxonomy P4. 14 new tests.
- `pathfinding::weighted_astar`: weighted (ε-admissible) A* pathfinding. `weighted_astar(start, goal, is_blocked, weight)` inflates the octile heuristic `f = g + weight × h`; at `weight = 1` it is identical to `astar` (optimal), at `weight > 1` it expands fewer nodes while bounding the returned path cost to `weight × optimal_cost`. Determinism invariants preserved: open-set key `(f, weight*h, x, y)` is total and unique. Weight `0` is clamped to `1`. Satisfies taxonomy J8. 8 new tests.
- `multimap`: multi-floor dungeon stack (`MultiMap`, `Connector`). `MultiMap` wraps a `Vec<Dungeon>` floors with an active `current_floor` index; floors are connected by `Connector` records (positions on floor A → position on floor B, directional). API: `new` (clamps `current_floor` to valid range), `floor_count`, `current_floor`, `set_floor` (clamped), `floor(i)`, `current` (active floor borrow), `add_connector`, `exits_from(floor)`, `connector_at(floor, x, y)`. `DetHash` folds `current_floor` + connectors sorted by `(from_floor, from_x, from_y)` for canonical replay-safe ordering. Both `MultiMap` and `Connector` implement `DetHash`. Satisfies taxonomy I6. 10 new tests.
- `autotile`: bitmask auto-tiling for terrain rendering. `compute_mask(x, y, is_same)` computes an 8-bit neighbour mask (N/NE/E/SE/S/SW/W/NW bits) with automatic diagonal-suppression when flanking cardinals are absent; `compute_all(w, h, is_same)` computes the full map in row-major order. `SimpleTileTable` is a 256-entry `u8→u32` lookup mapping masks to tile variant IDs; supports `set`/`get`/`from_array` and `DetHash`. Satisfies taxonomy I4. 14 new tests.
- `hud`: HUD data primitives for terminal roguelike display. `BarWidget` renders a fill-bar (`[====    ]`, configurable fill/empty chars) using integer `current/max * width` with clamping; `StatLine` formats `"label: value [unit]"` lines; `HudPanel` tracks a positioned bounding box with `inner_*` content-region helpers and `contains`/`translate`. All implement `DetHash`. Satisfies taxonomy M4. 16 new tests.
- `profiler`: tick profiler and structured event log. `Profiler` tracks named sections (identified by `&'static str`) with per-tick totals, call counts, and all-time peak; `begin_tick()` flushes current totals to rolling history; no OS clock dependency (callers supply timestamps as `u64`). `EventLog<E>` is a bounded ring-buffer log of tick-stamped typed events (oldest dropped when full); `push(tick, event)`, `iter()`, `recent(n)`, `clear`. Both implement `DetHash`. Satisfies taxonomy P3. 13 new tests.
- `assets`: typed asset handle store (`AssetStore<T>`, `AssetHandle<T>`). Generational handles prevent stale-read bugs (old handle resolves to `None` after remove + reuse). API: `insert` (→ handle), `get`, `get_mut`, `replace`, `remove`, `is_live`, `len`, `iter` (ascending index order). `DetHash` (gated on `T: DetHash`) folds live assets in index order. Satisfies taxonomy H7. 13 new tests.
- `relations`: entity parent/child relationship store (`Relations`). Forest-of-trees structure; each entity has at most one parent and any number of children. `attach` / `detach` / `remove_entity` (children become roots); cycle detection prevents infinite loops; `parent_of`, `children_of`, `is_ancestor`, `depth`, `is_root`, `is_leaf`. `DetHash` folds (child, parent) pairs in canonical ascending entity-index order. Satisfies taxonomy C6. 14 new tests.
- `influence`: grid-based influence map (`InfluenceMap`) for game AI steering. Integer `i32` scalar field; `add_source(x, y, strength, radius)` radiates Chebyshev-linear falloff; `add_raw` for direct cell edits; `decay(num, den)` for time-based fading (saturating integer multiply); `highest_neighbour`/`lowest_neighbour` for 8-directional steering (chase/flee); `clear`. `DetHash` folds dimensions + cells in row-major order. Satisfies taxonomy J6. 15 new tests.
- `tilemap`: multi-layer tile map (`TileMap<T>`, `LayeredMap<T>`). `TileMap` is a 2-D grid in row-major order with `get`/`set` (OOB = `None`/no-op), `fill`, `fill_rect` (clips to boundary), `iter` (row-major). `LayeredMap` bundles `N` same-size layers; `layer(i)`, `layer_mut(i)`, convenience `get`/`set` by layer index. `DetHash` for both (gated on `T: DetHash`). Also added `DetHash for u8` to `world_hash` (zero-pad to u32). Satisfies taxonomy I3. 15 new tests.
- `noise`: deterministic integer noise functions for procedural generation. `hash_1d(x, seed)` and `hash_2d(x, y, seed)` — fast integer coordinate hashes (SplitMix64-style mixing, output `[0, u32::MAX]`). `value_noise_1d(x, seed)` and `value_noise_2d(x, y, seed)` — smooth cubic Hermite (smoothstep) interpolated integer noise in `[0, 65535]`, using Q16.16 fixed-point arithmetic throughout (no float). Coordinates are Q16.16 (upper 16 bits = integer, lower = fraction) so callers can sample at sub-integer positions. Bit-identical across targets. Satisfies taxonomy D6. 16 new tests.
- `spatial_hash`: integer spatial hash grid (`SpatialHash<K>`) for O(1)-average broad-phase collision queries. Fixed-size cells; Euclidean division for negative-coordinate correctness. API: `insert`, `remove`, `move_entity`, `query_cell`, `query_rect` (spans multiple cells), `clear`, `len`, `cell_count`. `DetHash` (gated on `K: DetHash + Ord`) folds cells in canonical coordinate order, buckets in sorted-key order. Satisfies taxonomy K3. 15 new tests.
- `inputbuf`: input buffer with hold-detection and key-repeat (`InputBuffer<K>`). Tracks held keys by tick count; initial press fires immediately; after `initial_delay` ticks repeats every `repeat_period` ticks (standard roguelike long-press). API: `press`, `release`, `is_held`, `tick(n) -> Vec<K>`, `clear`, `held_count`, `held_ticks`. `DetHash` (gated on `K: DetHash + Ord`) folds held-key state in canonical key order. Pairs with `KeyMap` and `CmdQueue` for the full input pipeline. Satisfies taxonomy G3. 14 new tests.
- `textlayout`: word-wrap and text alignment helpers for fixed-width terminal rendering. `wrap_words(text, max_cols)` breaks at word boundaries (falling back to hard mid-word splits for words longer than the column width); `truncate(s, n)` clips to `n` chars appending `…`; `center(s, w)` / `pad_right(s, w)` / `pad_left(s, w)` pad with spaces for alignment. All functions are pure, allocation-minimal, and char-width based (one scalar = one column). Satisfies taxonomy M3. 26 new tests.
- `menu`: keyboard-navigable list menu (`Menu<T>`, `MenuItem<T>`) for roguelike UI. Items carry a typed payload `T`; each has a display label and optional `disabled` flag. Navigation via `move_up`/`move_down` (wrapping, skips disabled items); `set_cursor` for direct jump; `select` returns the current payload or `None` if disabled/empty. `DetHash` (gated on `T: DetHash`) folds labels + disabled flags + values + cursor position. Pairs with `terminal::Screen` for rendering and `KeyMap`/`CmdQueue` for input. Satisfies taxonomy M2. 18 new tests.
- `aabb`: axis-aligned bounding box (`Aabb`) collision detection. Integer rectangle `[x, x+w) × [y, y+h)` with exclusive right/bottom edges. API: `new` (negative dimensions clamped to zero), `right`/`bottom` edge accessors, `overlaps` (touching edges don't overlap), `contains_point`, `intersection` (returns sub-box or `None`), `translate` (saturating). `DetHash` folds all four fields. Satisfies taxonomy K2. 22 new tests.
- `inventory`: slot-based `Inventory<T>` for roguelike items. Fixed-capacity array of optional slots; items added to the first free slot; removed by index; retrieved by index or predicate. Slot layout is stable (gaps are not compacted, preserving acquisition order). API: `new`, `capacity`, `len`, `is_empty`, `has_space`, `add`, `remove`, `get`, `find`, `iter`, `swap`. `DetHash` (gated on `T: DetHash`) folds slot indices and values in canonical ascending order plus capacity so two identically-filled inventories hash the same regardless of fill history. Satisfies taxonomy L3. 14 new tests.
- `combat`: integer combat formula — `Stats` (hp/max_hp/attack/defense with
  `take_damage`, `heal`, `is_alive`, `hp_fraction`), `base_damage` (attack −
  defense, min 1), `melee_attack` (resolve and apply), `roll_to_hit` (draws
  one RNG value; degenerate 0% / 100% resolved without drawing), and
  `ranged_attack` (hit roll + melee). All integer, no float. `Stats` implements
  `DetHash`. Satisfies taxonomy L2. 17 new tests.
- `fsm`: table-driven finite state machine (`Fsm<S,E>`) for game AI. Transitions
  are `(from_state, event) → to_state` triples; missing transitions are silent
  self-loops (no panic). `fire` returns true on state change; `set_state` forces
  a state bypass (e.g. on death). `DetHash` folds only the current state (the
  table is constant configuration). Demonstrates a guard AI (Idle/Alert/Chase/
  Dead) in tests. Satisfies taxonomy J7. 12 new tests.
- `keymap`: key-to-action mapping (`KeyMap<K,A>`). Translates raw key events
  into typed game actions via a configurable binding table (one key → one
  action). `bind` adds/replaces; `unbind` removes; `get` looks up; `is_bound`
  checks; `translate_all` maps a slice of keys discarding unmapped ones —
  the typical per-tick call before pushing into `CmdQueue`. Replay-safe by
  construction (stateless function). Satisfies taxonomy G1. 13 new tests.
- `change`: dirty-flag change detection (`Changed<T>` and `ChangeTracker`).
  `Changed<T>` wraps a value with its `changed_at` tick; `is_changed_since(n)`
  skips updates in O(1); `get_mut(tick)` mutates and auto-marks. `ChangeTracker`
  is a saturating tick counter incremented each sim step. `DetHash` folds value
  + tick so a spurious re-mark is visible in the world hash. Mirrors Bevy's
  `Changed<T>` filter but without global state — fully replay-safe. Satisfies
  taxonomy C5. 10 new tests.
- `status`: timed buff/debuff tracking (`StatusSet<K>`). Each effect has a
  remaining tick duration and a signed magnitude. `apply` inserts or refreshes
  (max-duration stacking); `tick(n)` advances all effects, removes expired ones
  and returns their keys; `remove` discards immediately. `total_magnitude` sums
  all active magnitudes for a net modifier. `DetHash` folds in canonical key
  order so the hash is insertion-order-independent. Satisfies taxonomy L4.
  11 new tests.
- `easing`: integer easing curves over `Fixed` — `ease_in_quad`,
  `ease_out_quad`, `ease_in_out_quad`, `ease_in_cubic`, `ease_out_cubic`, and
  `linear`. All take `t ∈ [0,1]`, return values in the same range, and are
  bit-identical across targets (no float). Satisfies taxonomy B5. 10 tests.
- `fixed`: added `abs` (saturates MIN→MAX), `sign` (-1/0/1), `clamp(lo,hi)`,
  and `lerp(a,b,t)` — completing taxonomy B6. Also added `Fixed::MAX` /
  `Fixed::MIN` constants (used by `vec` module). 9 new tests.
- `camera`: integer camera and viewport (`Camera`) for world↔screen coordinate
  mapping. `new(cx, cy, screen_w, screen_h, world_w, world_h)` centres on the
  focus and clamps so the full viewport fits within the world. `world_to_screen`
  returns `None` for out-of-viewport world coords; `screen_to_world` clamps to
  the viewport edge. `recenter` moves the focus; `world_rect` returns the
  covered world rectangle. Implements `DetHash`. Satisfies taxonomy F5 —
  completing the F (Presentation) row. 15 new tests.
- `timer`: tick-based `Cooldown` and `TimerQueue<E>`. `Cooldown` is a simple
  saturating countdown (`tick(n)` returns `true` on the ready transition).
  `TimerQueue<E>` is a collection of future events — `schedule(delay, event)`
  and `schedule_repeat(delay, period, event)` — advanced by `advance(ticks)`
  which fires (and returns) all expired events in order and requeues repeating
  ones. Both implement `DetHash`; no float or OS clock anywhere. Satisfies
  taxonomy A4. 14 new tests.
- `cmdqueue`: deterministic command queue (`CmdQueue<C>`) — the replay-safe
  input abstraction. Commands are pushed between simulation ticks and drained
  in one atomic FIFO batch at the tick boundary; draining is the only way to
  consume commands so none are processed twice. Provides `push`, `push_batch`,
  `drain`, `clear`, `peek`, `len`, `is_empty`. `DetHash` (gated on `C:
  DetHash`) folds pending commands in insertion order so the queue participates
  in replay hash checks. Satisfies taxonomy G2. 11 new tests.
- `msglog`: bounded ring-buffer message log (`MsgLog`) for roguelike UI.
  Push is O(1); oldest messages are silently dropped when capacity is reached
  so memory stays bounded. Exposes `push`, `iter` (oldest-to-newest), `recent(n)`,
  `clear`, `len`, `is_empty`, and `capacity`. Implements `DetHash` (folds the
  visible history in order, independent of internal ring-buffer position) so the
  event log participates in snapshot/replay checks. A capacity-0 log discards
  immediately, useful in headless tests. 11 new tests.
- `vec`: fixed-point 2-D and 3-D vectors (`Vec2`/`Vec3`) over `Fixed`. Each
  type exposes `new`/`ZERO`, `dot`, `len_sq`, `len` (integer isqrt), `scale`,
  `normalize` (returns `None` for the zero vector — no panic, no garbage),
  `Add`/`Sub`/`Neg` operators (all saturating, no wrap), and `DetHash`.
  `Vec2` additionally has `perp` (90° CCW rotation). `Fixed` gains `MAX`/`MIN`
  constants. Covered by 21 new unit tests across arithmetic, Pythagorean
  identity, cross-product anti-commutativity, normalize, and saturation.
- `rng::weighted_index`: loot/spawn table draw — picks an index in proportion to
  its weight using a single wide-multiply draw (u64 analogue of `below`). Returns
  `None` without drawing for an empty slice or all-zero weights. Zero-weight
  entries are never chosen; the weight sum accumulates in `u64` so many large
  `u32` weights don't overflow. Covered by empty/all-zero, single-nonzero,
  proportional-distribution, and determinism tests.
- `rng::dice`: tabletop `NdM` roll — sums `count` independent draws from
  `1..=sides`; `sides == 0` or `count == 0` return `0` without drawing. Sum
  saturates instead of wrapping. Draws exactly `count` times, so draw position is
  a deterministic function of the arguments. Covered by sides-zero, count-zero,
  within-bounds, and determinism tests.
- `turn`: energy/speed-based turn scheduler — the roguelike progression core.
  Actors bank energy proportional to `speed` and act on reaching `ACTION_COST`;
  faster actors act more often and leftover energy carries over. Time advances by
  closed-form integer `ceil` division (no float), and same-unit ties break by
  smallest id — fully deterministic. Generic over the actor id; `det_hash` folds
  it (canonical id order) into replay-checked state. Covered by proportionality,
  tie-break, add/remove, set_speed, and determinism tests.
- `terminal`: the presentation layer the "terminal-first" engine was missing. A
  headless `Screen` of `Cell`s (glyph + 24-bit fg/bg) with edge-clipped drawing
  primitives (`set`/`put`/`clear`/`fill_rect`/`draw_str`), double-buffered
  `diff`/`present` change tracking, and a deterministic 24-bit ANSI serialiser
  (`to_ansi`). `Cell`/`Screen` implement `DetHash` so a frame folds into the
  world hash / snapshot tests. No OS I/O — writing to a tty is the caller's job.
- `GAME_DEV_TAXONOMY.md`: thorough categorisation of what a game needs
  (time/math/ECS/rng/determinism/rendering/input/content/world/AI/physics/
  gameplay/UI/persistence/net/tooling), subdivided and mapped to coverage, as
  the engine-enhancement roadmap.
- `geometry`: integer Bresenham `line` (king-move-contiguous, endpoints
  inclusive) and `line_of_sight` (single-ray check; opaque endpoints don't
  block). Float-free and deterministic. Covered by axis/diagonal/adjacency and
  LOS (open / interior wall / opaque-endpoint) tests.
- `replay`: deterministic replay & desync-detection harness, generalising the
  by-hand loop in `tests/determinism.rs`. `record_trace` produces a per-tick
  state-hash trace; `check_trace`/`first_divergence` report the first diverging
  tick (the desync-hunt starting point); `resimulate` clones a snapshot and
  replays inputs onto it (the rollback primitive). Engine-agnostic via a
  `step(&mut state, &input)` closure; state hashed through `DetHash`.
- `world_hash`: `hash_state` (fold any `DetHash` value to a `u64`); `DetHash`
  for `SplitMix64` (folds stream position) so RNG divergence shows in the hash.
- `sparse_set`: multi-component queries `join` and `join_mut` — the inner join
  of two component stores (entities present in both), returned in canonical
  ascending-index order so iteration is deterministic. `join_mut` yields a
  mutable reference to the first component for systems that update `A` from `B`
  (e.g. integrate `Position` by `Velocity`). The smaller store drives the scan.
- `mapgen`: deterministic procedural dungeon generation. Seed-driven rooms
  (rejection-placed with a one-cell border) joined by L-shaped corridors, so the
  map is always fully connected; all randomness comes from a supplied
  `SplitMix64` in fixed order, giving byte-identical maps per `(seed, params,
  size)`. `Dungeon` exposes `is_wall`/`is_floor` (OOB = wall, drop-in for `fov`
  and `pathfinding`) and implements `DetHash`. Covered by reproducibility,
  border, disjoint-rooms, full-connectivity (flood via `dijkstra_map`), and
  tiny-map tests.
- `SPEC.md`: a specification of the kit's API contracts, global invariants
  (zero-dep, no-unsafe, no-float-in-sim, replay-determinism) and a completeness
  checklist used to find and close gaps.
- `world_hash`: `DetHash` implementations for the primitives (`u32`, `u64`,
  `i32`, `bool`, `char`) and for the kit's value types (`Fixed`, `Entity`,
  `Position`, `Render`, `Color`), plus `SparseSet::det_hash` which folds a
  component store in canonical (ascending-index) order with its length — wiring
  the determinism checksum through the actual data types.
- `pathfinding`: `dijkstra_map` (multi-source Dijkstra distance field / flow
  field, budgeted by `max_cost`) and `descend` (deterministic steepest-descent
  step for chase/flee AI).
- `rng`: `range(lo, hi)` (uniform `[lo, hi)`, empty range returns `lo` without
  drawing) and `coin(num, den)` (probability `num/den`, degenerate odds resolved
  without drawing) — both low-bias and draw-count-deterministic.
- `pathfinding`: deterministic 8-directional A* grid pathfinding. Integer octile
  costs (10 orthogonal / 14 diagonal) and heuristic (no float), open set ordered
  by a total `(f, h, x, y)` key so the result is reproducible, fixed-order
  neighbour expansion, and no diagonal corner-cutting through walls. Generic over
  an `is_blocked` closure. Covered by straight-line, diagonal, detour,
  corner-cut, unreachable, and determinism tests.
- `fov`: symmetric recursive shadowcasting field-of-view (Albert Ford's
  algorithm). Integer-only rational slopes (no float), fixed quadrant/column
  iteration order, zero allocation — bit-identical across targets. Generic over
  `is_opaque`/`mark_visible` closures so it works with any grid. Guarantees
  symmetric visibility (A sees B iff B sees A); covered by open-field,
  shadow-casting, symmetry-property, and determinism tests.
- `fixed`: transcendental math without floats — `sqrt` (integer bit-by-bit
  square root) and `sin`/`cos`/`sin_cos`/`atan2` (rotation- and vectoring-mode
  CORDIC, 16 iterations, constants as integer literals). All results are
  bit-identical across targets and safe to feed the world hash. Negative `sqrt`
  saturates to zero; `atan2` is four-quadrant and degenerate-input safe. Covered
  by cardinal-angle, Pythagorean-identity, inverse, and determinism tests.
- `parser`: column-aware diagnostics. Every parse error now carries a 1-based
  column, and `Diagnostic::render` prints the source line with a caret under the
  offending token (rustc/clang style).
- `serializer`: `Content` -> `.game` text, the inverse of the parser. Output is
  canonical and idempotent (`serialize(parse(serialize(c))) == serialize(c)`).
- `content_eq`: semantic equality for round-trip checking.
- `timestep`: fixed-timestep accumulator with a death-spiral guard
  (`max_steps`), using integer nanoseconds.
- `gamec --fmt`: emit canonical `.game` text to stdout.
- Structure-aware round-trip fuzz test (3000+ generated bundles) and a
  canonical-form fuzz test (2000+ bundles).

### Changed
- `fixed`: `Add`/`Sub` now **saturate** instead of wrapping, and `mul`/`div`
  clamp the wide intermediate into range. Division by zero saturates by sign
  instead of panicking. This fixes a latent bug where an overflowing coordinate
  could silently flip to the opposite extreme.

## [0.1.0]

### Added
- `entity` / `sparse_set`: sparse-set ECS storage with generational handles.
- `fixed`: Q16.16 fixed-point scalar.
- `rng`: SplitMix64 seeded PRNG.
- `world_hash`: FNV-1a deterministic state checksum, with an end-to-end
  bit-exact replay test.
- `content` / `parser` / `validator` / `loader`: the text-to-ECS content
  pipeline.
- `gamec`: content checker CLI usable as a CI gate.
