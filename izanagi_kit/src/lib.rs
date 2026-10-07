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
//! | **2. Deterministic algorithms** | Where nondeterminism usually sneaks into a game (unordered iteration, float, address dependence). These are the vetted versions. | [`pathfinding`], [`fov`], [`geometry`], [`gridcast`], [`graph`], [`pack`], [`zorder`], [`msquares`], [`flow`], [`hungarian`], [`lsystem`], [`poly`], [`rdp`], [`fenwick`], [`ahocor`], [`diff`], [`trie`], [`segtree`], [`bipartite`], [`tsp`], [`rle`], [`segment`], [`euler`], [`rmq`], [`closestpair`], [`interval`], [`cron`], [`fuzzy`], [`stats`], [`markov`], [`lttb`], [`ntheory`], [`lca`], [`huffman`], [`treap`], [`kmp`], [`vclock`], [`merkle`], [`bloom`], [`delta`], [`lzss`], [`rolling`], [`suffix`], [`kdtree`], [`twosat`], [`minimax`], [`perm`], [`conv`], [`manacher`], [`gauss`], [`bezier`], [`kmv`], [`cms`], [`quantile`], [`chash`], [`lzw`], [`minhash`], [`zfunc`], [`bellman`], [`frac`], [`slide`], [`lis`], [`dagsp`], [`bfprt`], [`raster`], [`linrec`], [`mcflow`], [`knapsack`], [`coloring`], [`dsurb`], [`cht`], [`bwt`], [`hld`], [`mincut`], [`miller`], [`wavelet`], [`sam`], [`hamdp`], [`dlx`], [`arborescence`], [`xorbasis`], [`eertree`], [`mo`], [`histrect`], [`stable`], [`wdsu`], [`dominators`], [`fenwick2d`], [`circulation`], [`biconn`], [`simhash`], [`gf2`], [`rsfec`], [`fmidx`], [`zerobfs`], [`dpll`], [`intervaltree`], [`centroid`], [`piecetable`], [`steiner`], [`wal`], [`bitap`], [`flowfield`], [`shunting`], [`sat`], [`buddy`], [`xorfilter`], [`lru`], [`vose`], [`quadtree`], [`cartesian`], [`iheap`], [`pstree`], [`crc`], [`mcts`], [`slotmap`], [`cuckoof`], [`rans`], [`polylabel`], [`mphf`], [`mis`], [`chacha`], [`sha256`], [`dbscan`], [`kmeans`], [`rtree`], [`poly1305`], [`veb`], [`roaring`], [`lyndon`], [`sosdp`], [`skiplist`], [`karatsuba`], [`lsm`], [`pgm`], [`aes`], [`fenwickrange`], [`geohash`], [`octree`], [`base64`], [`vertexcover`], [`siphash`], [`hmac`], [`pairingheap`], [`bitonic`], [`offlinelca`], [`elias`], [`patricia`], [`blake2s`], [`rotcal`], [`smawk`], [`rope`], [`bktree`], [`mincircle`], [`slopetrick`], [`regex`], [`cuckoo`], [`bitboard`], [`cyk`], [`halfplane`], [`yfast`], [`radixsort`], [`rankselect`], [`debruijn`], [`cf`], [`earley`], [`sais`], [`hll`], [`arith`], [`swiss`], [`magic`], [`linkcut`], [`splay`], [`editdist`], [`gjk`], [`tlsf`], [`sha512`], [`ed25519`], [`blossom`], [`epa`], [`alphahull`], [`imptreap`], [`meetmid`], [`modlin`], [`veb3`], [`bigedit`], [`varint`], [`hornsat`], [`bigint`], [`segbeats`], [`ett`], [`utf8`], [`lazyseg`], [`tonelli`], [`bsgs`], [`dfamin`], [`bspline`], [`bmassey`], [`xortrie`], [`orset`], [`lww`], [`sieve`], [`grundy`], [`gapbuffer`], [`robin`], [`winnow`], [`scapegoat`], [`leftist`], [`beam`], [`perceptron`], [`saddleback`], [`avltree`], [`bandit`], [`zobrist`], [`rsa`], [`json`], [`sufftree`], [`redblack`], [`apsp`], [`pagerank`], [`automaton`], [`aastree`], [`lz4`], [`christofides`], [`sha3`], [`minkowski`], [`shamir`], [`postman`], [`fst`], [`fountain`], [`edt`], [`bplus`], [`bentley`], [`comb`], [`gcm`], [`polyclip`], [`clique`], [`dtw`], [`mmheap`], [`mstree`], [`pathcover`], [`mod@jps`], [`goap`], [`btree`], [`verlet`], [`vnoise`], [`bdd`], [`simplex`], [`peg`], [`radixheap`], [`life`], [`mapgen`], [`maze`], [`hexgrid`], [`delaunay`], [`wfc`], [`tilemap`], [`spatial_hash`], [`influence`], [`voronoi`], [`passability`], [`autotile`], [`turn`], [`entity`], [`sparse_set`], [`observe`], [`arch`], [`relations`], [`multimap`], [`steer`], [`reroot`], [`pbs`], [`crdt`], [`quat`], [`catmull`], [`viterbi`], [`spring`], [`pid`], [`kalman`], [`gnoise`], [`ray`], [`funnel`], [`affine`], [`hmm`], [`roots`], [`glob`], [`sobol`], [`biquad`], [`ccl`], [`sap`], [`pchip`], [`align`], [`lcp`], [`worley`], [`swept`], [`kdf`], [`csv`], [`ulid`], [`aead`], [`otsu`], [`integral`], [`pcg`], [`jaro`], [`md5`], [`poisson`], [`semver`], [`kcore`], [`otp`], [`anneal`], [`soundex`], [`bm25`], [`mdp`], [`rsync`], [`dual`], [`qlearn`], [`porter`], [`dither`], [`goertzel`], [`qoi`], [`civil`], [`jwt`], [`uuid`], [`cbor`], [`xxhash`], [`elo`], [`braille`], [`utility`], [`msgpack`], [`snowflake`], [`sdf`], [`ik`], [`tournament`], [`plot`], [`brent`], [`mt`], [`uri`], [`calendars`], [`snoise`], [`fft`], [`qr`], [`inflate`], [`deflate`], [`png`], [`zip`], [`murmur`], [`ip`], [`uuid7`], [`base58`], [`tar`], [`gif`], [`midi`], [`geo`], [`bech32`], [`punycode`], [`ws`], [`bmp`], [`bencode`], [`nbt`], [`dns`], [`checkcode`], [`wkt`], [`expr`], [`toml`], [`nmea`], [`tle`], [`obj`], [`srt`], [`sgf`], [`otpauth`], [`wav`], [`vtt`], [`ass`], [`pgn`], [`stl`], [`ini`], [`udiff`], [`ics`], [`vcf`], [`pem`], [`fen`], [`gpx`], [`m3u`], [`tga`], [`der`], [`rtf`], [`exif`], [`rss`], [`id3`], [`tzif`], [`pcap`], [`sha1`], [`git`], [`elf`], [`http`], [`mime`], [`ssh`], [`cpio`], [`macho`], [`coff`], [`wasm`], [`x509`], [`tls`], [`ico`], [`webp`], [`ttf`], [`woff`], [`sqlite`], [`proto`], [`bson`], [`flac`], [`ogg`], [`packfile`], [`classfile`], [`ar`], [`plist`], [`shp`], [`pcapng`], [`mbox`], [`dbf`], [`iso9660`], [`mvt`], [`pgp`], [`ply`], [`fits`], [`qcow2`], [`cab`], [`fat`], [`jpeg`], [`icns`], [`ttc`], [`bdf`], [`pdf`], [`ebml`], [`isobmff`], [`aiff`], [`xpm`], [`gltf`], [`wad`], [`svg`], [`vox`], [`dds`], [`modfile`], [`chip8`], [`ines`], [`tap`], [`ips`], [`nsf`], [`gbs`], [`psid`], [`spc`], [`vgm`], [`psf`], [`figlet`], [`gb`], [`gba`], [`z64`], [`sfc`], [`fds`], [`tzx`], [`d64`], [`lha`], [`atr`], [`cue`], [`d88`], [`xm`], [`it`], [`s3m`], [`arj`], [`pak`], [`pcx`], [`xbm`], [`pnm`], [`ras`], [`farbfeld`], [`vhd`], [`vmdk`], [`vdi`], [`dmg`], [`chd`], [`npy`], [`mat`], [`ethernet`], [`ipv4`], [`ipv6`], [`udp`], [`tcp`], [`icmp`], [`arp`], [`ne`], [`le`], [`aout`], [`dex`], [`optionrom`], [`cbfs`], [`ifd`], [`dicom`], [`nifti`], [`nrrd`], [`nc`], [`grib`], [`pdb`], [`mol2`], [`deb`], [`ole`], [`rar`], [`rpm`], [`x7z`], [`xar`], [`xz`], [`exfat`], [`ext2`], [`hfsplus`], [`minix`], [`ntfs`], [`ufs`], [`xfs`], [`bsp`], [`grp`], [`md2`], [`mdl`], [`mpq`], [`vpk`], [`vtf`], [`sjis`], [`eucjp`], [`iso2022`], [`big5`], [`gbk`], [`euckr`], [`utf16`], [`iff`], [`avi`], [`flv`], [`caf`], [`voc`], [`las`], [`woff2`], [`parquet`], [`avro`], [`arrow`], [`tiff`], [`tds`], [`dxf`], [`eps`], [`thrift`], [`flatbuf`], [`capnp`], [`ion`], [`regf`], [`evtx`], [`prefetch`], [`gds`], [`edif`], [`lef`], [`def`], [`liberty`], [`udf`], [`vhdx`], [`blend`], [`fbx`], [`glb`], [`abc`], [`pmd`], [`pmx`], [`bvh`], [`gzip`], [`bzip2`], [`zstd`], [`lz4f`], [`snappy`], [`brotli`], [`zlib`], [`ups`], [`bps`], [`aps`], [`ppf`], [`gdiff`], [`rdiff`], [`bsdiff`], [`fasta`], [`fastq`], [`gff`], [`bed`], [`genbank`], [`stockholm`], [`newick`], [`sf2`], [`dls`], [`xi`], [`iti`], [`pat`], [`sbi`], [`op2`], [`safetensors`], [`gguf`], [`pickle`], [`npz`], [`onnx`], [`tflite`], [`arff`], [`pkcs12`], [`pkcs8`], [`sshkey`], [`knownhosts`], [`kdbx`], [`htpasswd`], [`netrc`], [`epub`], [`mobi`], [`azw`], [`lit`], [`fb2`], [`kml`], [`osm`], [`passwd`], [`shadow`], [`group`], [`fstab`], [`crontab`], [`utmp`], [`hosts`], [`syslog`], [`prom`], [`graphite`], [`influx`], [`statsd`], [`opentsdb`], [`journal`], [`bibtex`], [`rst`], [`adoc`], [`roff`], [`texinfo`], [`org`], [`pod`], [`po`], [`ts`], [`xliff`], [`resx`], [`arb`], [`strings`], [`ftl`], [`torrent`], [`sfv`], [`pls`], [`xspf`], [`lrc`], [`ccd`], [`nrg`], [`mhd`], [`mrc`], [`edf`], [`jdx`], [`mzml`], [`vtk`], [`xyz`], [`lcov`], [`junit`], [`sarif`], [`cobertura`], [`checkstyle`], [`nunit`], [`gcov`], [`mavlink`], [`ubx`], [`sbus`], [`ntp`], [`obd`], [`candump`], [`ais`], [`ris`], [`medline`], [`csljson`], [`endnote`], [`jats`], [`mods`], [`coins`], [`gcode`], [`off`], [`step`], [`iges`], [`amf`], [`threemf`], [`x3d`], [`tcx`], [`fit`], [`geojson`], [`wkb`], [`topojson`], [`pmtiles`], [`hgt`], [`dhcp`], [`radius`], [`bgp`], [`lldp`], [`vrrp`], [`stp`], [`igmp`], [`modbus`], [`bacnet`], [`canopen`], [`mbus`], [`knx`], [`s7`], [`ethercat`], [`ccsds`], [`mseed`], [`segy`], [`sac`], [`bufr`], [`su`], [`pds`], [`au`], [`wv`], [`tta`], [`dsf`], [`rf64`], [`mp3`], [`ape`], [`gerber`], [`excellon`], [`hpgl`], [`pcl`], [`zpl`], [`escpos`], [`afp`], [`mht`], [`maildir`], [`mailcap`], [`desktop`], [`urlencode`], [`htaccess`], [`webloc`], [`pcf`], [`afm`], [`fnt`], [`fon`], [`pfm`], [`otf`], [`sfd`], [`uimage`], [`bootimg`], [`sparse`], [`ubi`], [`jffs2`], [`trx`], [`imx`], [`cramfs`], [`pyc`], [`llvmbc`], [`jks`], [`odex`], [`sourcemap`], [`dwarf`], [`msf`], [`uasset`], [`fsb`], [`pck`], [`unityfs`], [`roq`], [`bik`], [`xnb`], [`gitpack`], [`gitidx`], [`revlog`], [`svndump`], [`cvsrcs`], [`fossil`], [`bundle`], [`mcr`], [`gci`], [`vms`], [`dsv`], [`srm`], [`eep`], [`fla`], [`mft`], [`usnjrnl`], [`evt`], [`recbin`], [`jumplist`], [`hiberfil`], [`crashdump`], [`mqtt`], [`coap`], [`stun`], [`sip`], [`rtsp`], [`rtp`], [`llmnr`], [`fix`], [`iso8583`], [`ofx`], [`qif`], [`mt940`], [`ach`], [`edi`], [`psd`], [`xcf`], [`djvu`], [`jxl`], [`odf`], [`heif`], [`hdr`], [`dockerfile`], [`procfile`], [`systemd`], [`ninja`], [`makefile`], [`pkgbuild`], [`spec`], [`hcl`], [`cif`], [`mol`], [`cml`], [`fchk`], [`cube`], [`poscar`], [`gro`], [`rdb`], [`resp`], [`sst`], [`ldblog`], [`mdb`], [`gdbm`], [`bdb`], [`aiken`], [`gift`], [`qti`], [`imscc`], [`xapi`], [`opml`], [`apkg`], [`ipk`], [`snap`], [`appimage`], [`pkg`], [`msi`], [`nuget`], [`flatpak`], [`docx`], [`xlsx`], [`pptx`], [`vsdx`], [`xps`], [`jar`], [`kmz`], [`gre`], [`esp`], [`ospf`], [`rip`], [`pim`], [`smb2`], [`snmp`, [`crl`], [`csr`], [`p7b`], [`ocsp`], [`spf`], [`dkim`], [`dmarc`, [`spdx`], [`cyclonedx`], [`swid`], [`osv`], [`intoto`], [`csaf`], [`slsa`, [`securitytxt`], [`adstxt`], [`hostmeta`], [`webfinger`], [`assetlinks`], [`csp`], [`permissions`], [`uue`], [`yenc`], [`hqx`], [`sauce`], [`lnk`], [`icc`], [`luac`] |
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
//! - [`bitap`] — `Bitap`: Shift-And bit-parallel search — exact + Hamming-fuzzy (`k` substitutions) matching over ≤64-byte patterns in a single `u64` word.
//! - [`flowfield`] — `FlowField`: one-to-all pathfinding — Dijkstra integration field + per-cell direction field; 8-connected, no corner cutting, integer √2 costs.
//! - [`shunting`] — `shunting_yard` + `eval`/`eval_rpn`: Dijkstra's infix→postfix with strict `i64` eval; truncating `/ %`, fail-closed on overflow and `/0`.
//! - [`sat`] — `collide`/`overlap`: separating-axis convex collision in `i128` — boundary contact counts; witness is the min-overlap axis + projection depth.
//! - [`buddy`] — `Buddy`: binary buddy allocator — sorted lowest-address free lists, eager coalescing, canonical (no two buddies simultaneously free).
//! - [`xorfilter`] — `XorFilter`: static xor-filter membership (Graf & Lemire) — ~0.4% false positives, zero false negatives, three XOR'd lookups.
//! - [`lru`] — `Lru`: least-recently-used cache over u64 — two BTreeMaps, canonical (stamp, key) eviction order.
//! - [`vose`] — `AliasTable`: Vose's alias method — O(1) weighted sampling, exact integer distribution (w_k·n of n·total outcomes).
//! - [`quadtree`] — `Quadtree`: bucketed dynamic point index — sorted canonical answers, best-first `nearest`.
//! - [`cartesian`] — `Cartesian`: O(n) heap-on-values + BST-on-positions tree — LCA = range extremum (`rmq`).
//! - [`iheap`] — `IHeap`: indexed min-heap — `set`/`decrease`/`increase`/`remove` by key, (priority, key) canonical pop order.
//! - [`pstree`] — `PersistentTree`: chairman persistent segment tree — `kth`/`freq`/`range_count` per slice via version roots.
//! - [`crc`] — CRC-32 (IEEE) streaming checksum — chunk-invariant wire integrity.
//! - [`mcts`] — `mcts`: seeded UCB1 Monte-Carlo tree search over [`minimax::Game`], integer-only UCB.
//! - [`slotmap`] — `Slotmap`: generational handle store — stale handles structurally rejected, LIFO slot recycling.
//! - [`cuckoof`] — `CuckooFilter`: deletion-capable membership filter — two candidate buckets + bounded kick chain, `(keys, seed)` pure.
//! - [`rans`] — `Rans`: rANS entropy codec — largest-remainder normalized table, single-state integer coder.
//! - [`polylabel`] — `polylabel`: pole of inaccessibility — integer branch-and-bound maximizing min squared distance to boundary.
//! - [`mphf`] — `Mphf`: CHD minimal perfect hash — static `n` keys → `[0,n)` bijection via per-bucket displacements.
//! - [`mis`] — `maximal_independent_set`: canonical greedy MIS — ascending-order inclusion, pure function of the edge set.
//! - [`chacha`] — `ChaCha20`: RFC 8439 ChaCha20 keystream — streaming XOR cipher, counter-mode pure state.
//! - [`sha256`] — `Sha256`/`sha256`: FIPS 180-4 SHA-256 digest — incremental write, canonical big-endian padding.
//! - [`dbscan`] — `dbscan`: squared-distance density clustering — canonical index-order expansion, `-1` noise labels.
//! - [`kmeans`] — `kmeans`: deterministic integer k-means — farthest-point init + Lloyd iterations to fixpoint.
//! - [`rtree`] — `Rtree`: STR bulk-loaded static R-tree — pure-function point index, brute-force-equal queries.
//! - [`poly1305`] — `Poly1305`: RFC 8439 one-time authenticator (5×26-bit DJB limbs), incremental + split-invariant.
//! - [`veb`] — `Veb`: proto van Emde Boas `u32` predecessor/successor set (two-level sqrt bitset layout).
//! - [`roaring`] — `Roaring`: roaring bitmap `u32` set (array/bitset container split, sorted iteration, set algebra).
//! - [`lyndon`] — Duval Lyndon factorization + Booth least rotation (necklace/canonical cyclic forms).
//! - [`sosdp`] — subset/superset zeta–Möbius transforms + OR/AND/XOR convolutions on the bitmask lattice.
//! - [`skiplist`] — seeded-hash-level deterministic skip list (`u64` ordered set, lane-shape pure function of key set).
//! - [`karatsuba`] — Karatsuba multiplication over `u64` limbs (O(n^1.585) multi-word product).
//! - [`lsm`] — log-structured merge index (BTreeMap memtable + frozen sorted runs + tombstones).
//! - [`pgm`] — PGM-style learned index (rational-slope segments + exact ε window binary search).
//! - [`aes`] — AES-128 block cipher (S-box computed via `gf2` inverse + affine; FIPS-197 verified).
//! - [`fenwickrange`] — range-add Fenwick variants (point query + two-BIT range sum over `i64`).
//! - [`geohash`] — integer microdegree geohash codec (encode/decode/cell span/neighbors over `e6` lat/lon).
//! - [`octree`] — bucketed 3-D `i64` point index (sorted range queries + best-first nearest).
//! - [`base64`] — RFC 4648 base64 + base64url strict codec (decode rejects malformed padding).
//! - [`vertexcover`] — Kőnig bipartite minimum vertex cover via `hopcroft_karp` + alternating reachability.
//! - [`siphash`] — SipHash-2-4 keyed 64-bit PRF (streaming `SipHash` state, paper vectors verified).
//! - [`hmac`] — HMAC-SHA256 MAC (RFC 2104 over `sha256`, RFC 4231 vectors).
//! - [`pairingheap`] — arena pairing heap: O(1) `meld`, `(prio,key)` canonical pop order.
//! - [`bitonic`] — Batcher bitonic sorting network: fixed comparator sequence per `n` (data-oblivious sort).
//! - [`offlinelca`] — Tarjan offline LCA (DSU + one DFS) answering whole query batches.
//! - [`elias`] — Elias–Fano monotone sequence: unary-gap high bitmap + verbatim lows, `access`/`rank`/`successor`.
//! - [`patricia`] — crit-bit radix tree over `u64` keys: O(1-word) insert/contains plus sorted-order iteration.
//! - [`blake2s`] — BLAKE2s-256 (RFC 7693): streaming digest and keyed-MAC mode without the HMAC construction.
//! - [`rotcal`] — rotating calipers on a convex hull: diameter, min width, min-area rectangle — all exact `Frac`.
//! - [`smawk`] — SMAWK `O(n+m)` row argmin for totally monotone implicit matrices (Monge-DP machinery).
//! - [`rope`] — balanced rope text buffer: `O(log n)` insert/remove/split/concat with a depth-guard rebuild.
//! - [`bktree`] — Burkhard–Keller metric tree over `diff::levenshtein`: `within`/`nearest` fuzzy-string queries.
//! - [`mincircle`] — Welzl smallest enclosing circle on `Frac` points: exact rational center and r².
//! - [`slopetrick`] — slope-trick convex piecewise-linear function: `O(log n)` abs/ramp adds, slides, clamps.
//! - [`regex`] — Thompson-construction byte NFA regex: `.*` classes alternation quantifiers, no backtracking.
//! - [`cuckoo`] — two-table cuckoo hashing: ≤2 probes, tombstone-free removal, journaled kick rollback.
//! - [`bitboard`] — 8x8 u64 bitboards: masked directional shifts and dumb7fill sliding attacks.
//! - [`cyk`] — CYK recognizer for CNF grammars: O(n³) bitset triangle, `accepts`/`derive`/`cell`.
//! - [`halfplane`] — exact `Frac` half-plane intersection: deque build, CCW normalized vertices.
//! - [`yfast`] — y-fast–style clustered predecessor set: rep-ordered buckets, median splits.
//! - [`radixsort`] — stable LSD radix + counting sorts over `u64`, plus `(key, payload)` stability.
//! - [`rankselect`] — Jacobson rank/select bitvector: two-level directory, bounded select scans.
//! - [`debruijn`] — FKM de Bruijn `B(k,n)` sequences plus verifier and cyclic-window hash.
//! - [`cf`] — exact continued fractions over `Frac`: canonical expansion, convergents, best approx.
//! - [`earley`] — Earley chart parser for arbitrary CFGs: ε-rules and mixed-length productions.
//! - [`sais`] — induced-sorting suffix array: S/L typing, LMS naming, `O(n)` for byte alphabets.
//! - [`hll`] — integer HyperLogLog: `p`-bit registers, max merge, fixed-point estimate + `ln` LC.
//! - [`arith`] — Subbotin carry-less range coder: streaming interval subdivision, no carry buffer.
//! - [`swiss`] — SwissTable `u64` set: 16-slot groups, `h2` fingerprints, tombstone deletes.
//! - [`magic`] — magic bitboards: seeded `(occ·m)>>shift` perfect hashing of slider attacks.
//! - [`linkcut`] — Link–Cut dynamic tree: `link`/`cut`/`connected`/`lca`/`path_min` over a forest.
//! - [`splay`] — bottom-up splay BST: amortized balance from access locality, no RNG.
//! - [`editdist`] — Myers bit-vector Levenshtein: `dist` and approximate-search `find_leq` ends.
//! - [`gjk`] — integer 2-D GJK: `Frac`-exact squared distance between convex polygons.
//! - [`tlsf`] — segregated-fit allocator: two-level bins, coalescing, deterministic lowest fit.
//! - [`sha512`] — FIPS 180-4 SHA-512 streaming digest, manual BE word loads.
//! - [`ed25519`] — RFC 8032 EdDSA: radix-51 field + mod-L scalars, sign/verify/keypair.
//! - [`blossom`] — Edmonds' blossom: maximum matching on general (non-bipartite) graphs.
//! - [`epa`] — expanding polytope: `Frac` penetration depth + MTV axis for overlapping convex sets.
//! - [`alphahull`] — alpha shape over `delaunay`: boundary edges at integer `α²`.
//! - [`imptreap`] — implicit-key treap: `O(log n)` insert/remove/reverse with seeded priorities.
//! - [`meetmid`] — meet-in-the-middle: subset sums, counts, and best-fit in `O(2^(n/2)·n)`.
//! - [`modlin`] — GF(p) linear algebra: `rref`, `solve`, `rank`, nullspace basis.
//! - [`veb3`] — recursive van Emde Boas: `O(log log U)` predecessor/successor over u64.
//! - [`bigedit`] — multi-word Myers automaton: bit-vector edit distance for any pattern length.
//! - [`varint`] — canonical LEB128 + zigzag: strict wire codec for lockstep packets.
//! - [`hornsat`] — Dowling–Gallier linear Horn SAT: least-model implication closure.
//! - [`bigint`] — sign-magnitude arbitrary-precision integers: add/sub/mul/pow over u64 limbs.
//! - [`segbeats`] — segment tree beats: range `chmin`/`chmax`/`sum` via second-extremum ledgers.
//! - [`ett`] — Euler-tour tree: `link`/`cut`/`connected` dynamic-forest connectivity.
//! - [`utf8`] — Höhrmann strict-DFA UTF-8: `validate`/`decode`/`decode_lossy`/`encode`.
//! - [`lazyseg`] — canonical lazy segment tree: range `add` + `sum`/`min`/`max`/`get`/`set`.
//! - [`tonelli`] — Tonelli–Shanks modular square roots over odd primes, sorted root pairs.
//! - [`bsgs`] — baby-step giant-step discrete log: least `x` in `O(√p)`, no inverses.
//! - [`dfamin`] — Hopcroft DFA minimization: splitter worklist + canonical block ids.
//! - [`bspline`] — integer-exact de Boor B-spline evaluation over [`frac::Frac`] control points.
//! - [`bmassey`] — Berlekamp–Massey shortest LFSR over GF(p): `massey` + `holds` verifier.
//! - [`xortrie`] — bitwise trie over u64: max/min-xor queries and max-xor pairs.
//! - [`orset`] — add-wins observed-remove set CRDT: concurrent adds always survive.
//! - [`lww`] — LWW-element-set CRDT: `(clock, replica)` stamps pick each element's fate.
//! - [`sieve`] — linear SPF sieve + segmented primes: `phi`/`tau`/`sigma`/`factor` oracles.
//! - [`grundy`] — Sprague–Grundy numbers: mex, take-away tables, `detect_period`.
//! - [`gapbuffer`] — Emacs-style gap buffer over `Vec<u8>` with `copy_within` moves.
//! - [`robin`] — Robin Hood u64 set: probe stealing + backward-shift delete.
//! - [`winnow`] — winnowing fingerprints: rightmost-min per window (Schleimer 2003).
//! - [`scapegoat`] — α weight-balanced BST: rebuild the violating ancestor subtree.
//! - [`leftist`] — leftist heap: merge-only `O(log n)` right-spine priority queue.
//! - [`beam`] — deterministic beam search + `beam_moves` over `minimax::Game`.
//! - [`perceptron`] — integer linear classifier + one-vs-rest multi-class.
//! - [`saddleback`] — `O(r+c)` search in row+column sorted matrices.
//! - [`avltree`] — height-balanced BST: rotations keep `|h(l)-h(r)| ≤ 1`.
//! - [`bandit`] — UCB1 (Q8 fixed point) + ε-greedy multi-armed bandits.
//! - [`zobrist`] — incremental XOR board hash: `toggle` is exact undo.
//! - [`rsa`] — textbook RSA over `BigInt` (no padding; not wire crypto).
//! - [`json`] — strict integer-subset JSON parser + canonical render.
//! - [`sufftree`] — Ukkonen online suffix tree: substring count/occurrences/longest repeat.
//! - [`redblack`] — red-black BST: looser balance than AVL, fewer rotations on delete.
//! - [`apsp`] — Floyd–Warshall + Johnson all-pairs shortest paths, negative-cycle aware.
//! - [`pagerank`] — Q32 integer PageRank: deterministic power iteration to a fixed point.
//! - [`automaton`] — NFA→DFA subset construction + union/concat/star/complement/intersect/minimize.
//! - [`aastree`] — AA tree: the 2-invariant red-black (level = left+1, NIL = 0); skew + split only.
//! - [`lz4`] — LZ4 block codec: greedy hash parse, overlapping-match decoder, strict wire checks.
//! - [`christofides`] — metric TSP 1.5-approximation: MST + min-weight perfect matching on odds + Euler + shortcut.
//! - [`sha3`] — Keccak-f\[1600\] sponge: SHA3-256/512 + SHAKE128/256 XOF, incremental `Digest256`.
//! - [`minkowski`] — convex Minkowski sum/diff: edge-vector merge O(n+m), C-obstacle collide queries.
//! - [`shamir`] — (k,n) threshold secret sharing over GF(p): seeded polynomial eval + Lagrange at 0.
//! - [`postman`] — Chinese postman: odd-degree matching on metric closure → Euler augmentation.
//! - [`fst`] — minimal acyclic DFA dictionary: bottom-up hash-consing register over a sorted word set.
//! - [`fountain`] — Luby transform: robust-soliton degree, pure (k,i,seed) neighborhoods, BP peeling.
//! - [`edt`] — Felzenszwalb–Huttenlocher squared EDT: two 1-D parabola-envelope passes, integer-exact.
//! - [`bplus`] — B+ ordered map u64→u64: copy-up leaf splits, fix_sep ancestor propagation.
//! - [`bentley`] — Bentley–Ottmann sweep over Frac: verticals tracked per x-line, collinear shared endpoints.
//! - [`comb`] — combinadic rank/unrank + next_comb: exact division invariant, no floats.
//! - [`gcm`] — AES-128-GCM AEAD: chained GHASH aad→ct fold, NIST vectors.
//! - [`polyclip`] — Sutherland–Hodgman clip: exact Frac intersections, ring dedup.
//! - [`clique`] — Bron–Kerbosch maximal cliques with pivot over u64 adjacency masks.
//! - [`dtw`] — dynamic time warping: exact i64 elastic distance, Sakoe–Chiba band + path recovery.
//! - [`mmheap`] — min-max heap: alternating min/max levels, O(1) peek both ends.
//! - [`mstree`] — merge-sort tree: static range counting O(log² n) via sorted node runs.
//! - [`pathcover`] — DAG minimum path cover via bipartite matching: n − |matching| chains.
//! - [`mod@jps`] — Jump Point Search: 8-way jump-point pruning with forced neighbors.
//! - [`goap`] — goal-oriented action planning: bitmask-world Dijkstra, lex-min plans.
//! - [`btree`] — behavior trees with resume semantics: Running children remembered.
//! - [`verlet`] — integer Verlet integration + distance-constraint relaxation.
//! - [`vnoise`] — integer value noise + fBm in Fixed Q16.16.
//! - [`bdd`] — reduced ordered BDDs: hash-consed canonical boolean functions.
//! - [`simplex`] — exact-rational LP solver: two-phase simplex, Bland's rule.
//! - [`peg`] — packrat PEG parser: memoized ordered-choice recognition.
//! - [`radixheap`] — monotone radix heap: msb(XOR) buckets for Dijkstra keys.
//! - [`life`] — sparse deterministic B3/S23 Game of Life (BTreeSet state).
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

pub mod a2r;
pub mod aabb;
pub mod aastree;
pub mod abc;
pub mod ability;
pub mod ac;
pub mod ace;
pub mod acemode;
pub mod ach;
pub mod actionlint;
pub mod activemq;
pub mod adguard;
pub mod adic;
pub mod adoc;
pub mod adql;
pub mod adstxt;
pub mod aead;
pub mod aercconf;
pub mod aerospike;
pub mod aes;
pub mod affine;
pub mod affix;
pub mod afm;
pub mod afp;
pub mod ahocor;
pub mod aideconf;
pub mod aiff;
pub mod aiger;
pub mod aiken;
pub mod airbyteconf;
pub mod airflow;
pub mod ais;
pub mod alacritty;
pub mod alembic;
pub mod alertmanager;
pub mod alexrc;
pub mod algolia;
pub mod align;
pub mod alloy;
pub mod allure;
pub mod aln;
pub mod alphahull;
pub mod alto;
pub mod alz;
pub mod amandaconf;
pub mod ambassador;
pub mod amf;
pub mod amfile;
pub mod ampl;
pub mod amplifyconf;
pub mod amqp;
pub mod amr;
pub mod analyze;
pub mod angularconf;
pub mod anneal;
pub mod ansi;
pub mod ansible;
pub mod antex;
pub mod aoe;
pub mod aout;
pub mod apacheconf;
pub mod ape;
pub mod apib;
pub mod apisix;
pub mod apk;
pub mod apkg;
pub mod apmserver;
pub mod apparmor;
pub mod appcache;
pub mod appdaemon;
pub mod appdynamics;
pub mod appengine;
pub mod appimage;
pub mod appjson;
pub mod appveyor;
pub mod aprxconf;
pub mod aps;
pub mod apsp;
pub mod apt;
pub mod ar;
pub mod arb;
pub mod arborescence;
pub mod arch;
pub mod archinstall;
pub mod ardour;
pub mod arduinoconf;
pub mod arff;
pub mod argocd;
pub mod argoevents;
pub mod argorollout;
pub mod argowf;
pub mod argusconf;
pub mod aria2;
pub mod arith;
pub mod arj;
pub mod arkimeconf;
pub mod arma3conf;
pub mod arp;
pub mod arrow;
pub mod arw;
pub mod asciicast;
pub mod asdf;
pub mod asf;
pub mod asn1;
pub mod asoundrc;
pub mod ass;
pub mod assetlinks;
pub mod assets;
pub mod astro;
pub mod asv;
pub mod atlantis;
pub mod atom;
pub mod atr;
pub mod au;
pub mod audacity;
pub mod auditdconf;
pub mod auditrule;
pub mod authelia;
pub mod autofs;
pub mod automaton;
pub mod autotile;
pub mod autoyast;
pub mod avaconf;
pub mod avi;
pub mod avltree;
pub mod avro;
pub mod awscredentials;
pub mod awselb;
pub mod axports;
pub mod ay;
pub mod azurepipe;
pub mod azw;
pub mod babelrc;
pub mod backstage;
pub mod bacnet;
pub mod baculadir;
pub mod bai2;
pub mod bam;
pub mod bandit;
pub mod banditconf;
pub mod bannedips;
pub mod base32;
pub mod base58;
pub mod base64;
pub mod bashrc;
pub mod bazarr;
pub mod bazel;
pub mod bbcode;
pub mod bdb;
pub mod bdd;
pub mod bdf;
pub mod beam;
pub mod bech32;
pub mod bed;
pub mod beets;
pub mod behavior;
pub mod bellman;
pub mod benchstat;
pub mod bencode;
pub mod bentley;
pub mod bentoml;
pub mod bernoulli;
pub mod bezier;
pub mod bfprt;
pub mod bgp;
pub mod bibtex;
pub mod bicep;
pub mod biconn;
pub mod big5;
pub mod bigedit;
pub mod bigint;
pub mod bik;
pub mod biome;
pub mod bipartite;
pub mod biquad;
pub mod bird;
pub mod bit;
pub mod bitap;
pub mod bitboard;
pub mod bitbucketpipes;
pub mod bitcoinconf;
pub mod bitonic;
pub mod bitrise;
pub mod bits;
pub mod bktree;
pub mod blackbird;
pub mod blackconf;
pub mod blake2s;
pub mod blend;
pub mod blocky;
pub mod bloom;
pub mod blossom;
pub mod bm25;
pub mod bmassey;
pub mod bmp;
pub mod bogofilter;
pub mod bootimg;
pub mod bootini;
pub mod borgmatic;
pub mod bors;
pub mod boundary;
pub mod bplus;
pub mod bps;
pub mod braille;
pub mod braket;
pub mod brent;
pub mod brewfile;
pub mod brotli;
pub mod browserconfig;
pub mod browserslist;
pub mod bru;
pub mod bsdiff;
pub mod bsgs;
pub mod bsnes;
pub mod bson;
pub mod bsp;
pub mod bspline;
pub mod btrbk;
pub mod btree;
pub mod btrfs;
pub mod btsnoop;
pub mod buck;
pub mod buddy;
pub mod bufr;
pub mod buildkitd;
pub mod buildkite;
pub mod bukkit;
pub mod bundle;
pub mod bundlerconf;
pub mod bunfig;
pub mod bvh;
pub mod bwt;
pub mod bzip2;
pub mod c3d;
pub mod cab;
pub mod cabal;
pub mod caddyfile;
pub mod caf;
pub mod cairo;
pub mod calamares;
pub mod calendar;
pub mod calendars;
pub mod calico;
pub mod callgrind;
pub mod camera;
pub mod camt;
pub mod candump;
pub mod canopen;
pub mod capacitor;
pub mod capnp;
pub mod capsule;
pub mod capx;
pub mod cardanoconf;
pub mod cargoconf;
pub mod cargolock;
pub mod carla;
pub mod cartesian;
pub mod cartocss;
pub mod carvel;
pub mod casbin;
pub mod casdoor;
pub mod cassandra;
pub mod catalan;
pub mod catmull;
pub mod cbfs;
pub mod cbor;
pub mod ccd;
pub mod ccl;
pub mod ccs;
pub mod ccsds;
pub mod ccx;
pub mod cedar;
pub mod centroid;
pub mod cephconf;
pub mod cerbos;
pub mod certbot;
pub mod certmanager;
pub mod cf;
pub mod cfn;
pub mod cfssl;
pub mod cgitrc;
pub mod chacha;
pub mod change;
pub mod changesets;
pub mod chaosmesh;
pub mod chart;
pub mod chash;
pub mod chasquidconf;
pub mod chd;
pub mod checkcode;
pub mod checkov;
pub mod checkstyle;
pub mod chef;
pub mod cherokee;
pub mod chip8;
pub mod chirpcsv;
pub mod christofides;
pub mod chromaconf;
pub mod chrometrace;
pub mod chrompoly;
pub mod chronyconf;
pub mod cht;
pub mod cibxml;
pub mod cif;
pub mod cilium;
pub mod circleci;
pub mod circulation;
pub mod cirrus;
pub mod citraconf;
pub mod civil;
pub mod clangformat;
pub mod clangtidy;
pub mod clar;
pub mod clashconf;
pub mod classfile;
pub mod clickhouse;
pub mod cliff;
pub mod clique;
pub mod closestpair;
pub mod cloudcustodian;
pub mod cloudinit;
pub mod clusterapi;
pub mod clusterconf;
pub mod cmake;
pub mod cmdbat;
pub mod cmdqueue;
pub mod cml;
pub mod cms;
pub mod cmus;
pub mod cnpg;
pub mod coap;
pub mod cob;
pub mod cobertura;
pub mod cockroach;
pub mod cocosproj;
pub mod codeclimate;
pub mod codecov;
pub mod codespell;
pub mod coff;
pub mod coins;
pub mod colima;
pub mod collectd;
pub mod coloring;
pub mod comb;
pub mod combat;
pub mod commitlint;
pub mod compose;
pub mod composer;
pub mod composerlock;
pub mod conanfile;
pub mod concourse;
pub mod condaenv;
pub mod condarc;
pub mod configureac;
pub mod consul;
pub mod containerd;
pub mod containersconf;
pub mod content;
pub mod contourconf;
pub mod conv;
pub mod cookiejar;
pub mod coq;
pub mod corefile;
pub mod corosync;
pub mod coveralls;
pub mod cpanfile;
pub mod cpio;
pub mod cpplint;
pub mod cr2;
pub mod cramfs;
pub mod crashdump;
pub mod crc;
pub mod crdt;
pub mod creole;
pub mod crio;
pub mod criterion;
pub mod crl;
pub mod crmconf;
pub mod crockford;
pub mod cromwell;
pub mod cron;
pub mod crontab;
pub mod crossplane;
pub mod crowdsec;
pub mod csa;
pub mod csaf;
pub mod csd;
pub mod csljson;
pub mod csp;
pub mod cspell;
pub mod csr;
pub mod csv;
pub mod ctrf;
pub mod cube;
pub mod cuckoo;
pub mod cuckoof;
pub mod cue;
pub mod cuid;
pub mod cupsconf;
pub mod curaconf;
pub mod cve;
pub mod cvsrcs;
pub mod cyclonedx;
pub mod cyk;
pub mod cypher;
pub mod cypressconf;
pub mod d64;
pub mod d88;
pub mod dae;
pub mod dafny;
pub mod dagsp;
pub mod dagster;
pub mod damage;
pub mod dapr;
pub mod dask;
pub mod datadog;
pub mod db2cli;
pub mod dbf;
pub mod dbm;
pub mod dbscan;
pub mod dbt;
pub mod dcd;
pub mod dds;
pub mod deb;
pub mod debconf;
pub mod debruijn;
pub mod def;
pub mod defaultpa;
pub mod defconfig;
pub mod deflate;
pub mod defoldproj;
pub mod dehydrated;
pub mod delaunay;
pub mod delta;
pub mod deluge;
pub mod denoconf;
pub mod denyhosts;
pub mod dependabot;
pub mod der;
pub mod derange;
pub mod desktop;
pub mod detekt;
pub mod devbox;
pub mod devcontainer;
pub mod devfile;
pub mod devspace;
pub mod dex;
pub mod dexidp;
pub mod dfamin;
pub mod dgml;
pub mod dgn;
pub mod dhall;
pub mod dhclientconf;
pub mod dhcp;
pub mod dhcpcdconf;
pub mod diag_json;
pub mod dialogue;
pub mod diameter;
pub mod dice;
pub mod dicom;
pub mod dictd;
pub mod dictzip;
pub mod did;
pub mod dif;
pub mod diff;
pub mod digikamrc;
pub mod digit;
pub mod dihedral;
pub mod dimacs;
pub mod dinit;
pub mod direwolfconf;
pub mod discourse;
pub mod dita;
pub mod dither;
pub mod djvu;
pub mod dkim;
pub mod dls;
pub mod dlt;
pub mod dlx;
pub mod dm3;
pub mod dmarc;
pub mod dmg;
pub mod dnfconf;
pub mod dng;
pub mod dns;
pub mod dnsmasq;
pub mod docbook;
pub mod dockerdaemon;
pub mod dockerfile;
pub mod dockerignore;
pub mod docsify;
pub mod docusaurus;
pub mod docx;
pub mod dolphinconf;
pub mod dominators;
pub mod dosboxconf;
pub mod dossys;
pub mod dot;
pub mod dotenv;
pub mod dovecot;
pub mod dpll;
pub mod dpx;
pub mod dragonflyconf;
pub mod drawio;
pub mod drbdconf;
pub mod dro;
pub mod drone;
pub mod ds9reg;
pub mod dsf;
pub mod dsig;
pub mod dsl;
pub mod dsn;
pub mod dst;
pub mod dsurb;
pub mod dsv;
pub mod dta;
pub mod dtd;
pub mod dted;
pub mod dtw;
pub mod dual;
pub mod dune;
pub mod dunst;
pub mod duplicacy;
pub mod dvcfile;
pub mod dvi;
pub mod dwarf;
pub mod dwg;
pub mod dx;
pub mod dxbc;
pub mod dxf;
pub mod e00;
pub mod e57;
pub mod ead;
pub mod eaglexml;
pub mod ean;
pub mod eap;
pub mod earley;
pub mod earthly;
pub mod easing;
pub mod easyeffects;
pub mod easyrsa;
pub mod ebml;
pub mod ec;
pub mod ecat;
pub mod eck;
pub mod ecsv;
pub mod ed25519;
pub mod edf;
pub mod edi;
pub mod edif;
pub mod editdist;
pub mod editorconfig;
pub mod edn;
pub mod edsk;
pub mod edt;
pub mod eep;
pub mod eertree;
pub mod egypt;
pub mod ejabberd;
pub mod elasticsearch;
pub mod elf;
pub mod elias;
pub mod elo;
pub mod emacs;
pub mod emqx;
pub mod encounter;
pub mod endnote;
pub mod entity;
pub mod envoy;
pub mod envrc;
pub mod epa;
pub mod epd;
pub mod eps;
pub mod epub;
pub mod epwing;
pub mod equipment;
pub mod erf;
pub mod escpos;
pub mod eslintrc;
pub mod esmapping;
pub mod esp;
pub mod esphome;
pub mod essettings;
pub mod etcd;
pub mod ethercat;
pub mod ethernet;
pub mod ett;
pub mod eucjp;
pub mod euckr;
pub mod eula;
pub mod euler;
pub mod eulerian;
pub mod eventqueue;
pub mod evt;
pub mod evtx;
pub mod excalidraw;
pub mod excellon;
pub mod exfat;
pub mod exif;
pub mod exim;
pub mod explore;
pub mod expr;
pub mod exr;
pub mod ext2;
pub mod externalsecrets;
pub mod extmanifest;
pub mod f2fs;
pub mod faction;
pub mod factoriosettings;
pub mod fail2ban;
pub mod fail2banconf;
pub mod falcoconf;
pub mod far;
pub mod farbfeld;
pub mod farey;
pub mod fasta;
pub mod fastq;
pub mod fat;
pub mod fb2;
pub mod fbx;
pub mod fceux;
pub mod fchk;
pub mod fcoe;
pub mod fds;
pub mod feast;
pub mod fen;
pub mod fenwick;
pub mod fenwick2d;
pub mod fenwickrange;
pub mod ferm;
pub mod fetchmailconf;
pub mod fft;
pub mod fgb;
pub mod fhir;
pub mod fibheap;
pub mod fidl;
pub mod figlet;
pub mod filebeat;
pub mod firebase;
pub mod firejailprof;
pub mod firewalld;
pub mod fishconf;
pub mod fit;
pub mod fits;
pub mod fivetranconf;
pub mod fix;
pub mod fixed;
pub mod fixml;
pub mod fla;
pub mod flac;
pub mod flagger;
pub mod flake8conf;
pub mod flatbuf;
pub mod flatpak;
pub mod fldigiconf;
pub mod fleet;
pub mod flif;
pub mod flink;
pub mod flow;
pub mod flowfield;
pub mod fluentbit;
pub mod fluentd;
pub mod fluxcd;
pub mod flv;
pub mod flyio;
pub mod flyway;
pub mod fmidx;
pub mod fnt;
pub mod fon;
pub mod footconf;
pub mod fossil;
pub mod fossilconf;
pub mod fountain;
pub mod fov;
pub mod fpml;
pub mod fps;
pub mod frac;
pub mod frd;
pub mod freetds;
pub mod frigate;
pub mod frobenius;
pub mod frr;
pub mod fsb;
pub mod fsm;
pub mod fst;
pub mod fstab;
pub mod ftl;
pub mod func;
pub mod funnel;
pub mod fusesoc;
pub mod fuzzy;
pub mod fxml;
pub mod fxp;
pub mod gapbuffer;
pub mod garden;
pub mod garnetconf;
pub mod gatekeeper;
pub mod gatewayapi;
pub mod gatsby;
pub mod gauss;
pub mod gb;
pub mod gba;
pub mod gbench;
pub mod gbk;
pub mod gbs;
pub mod gbstudio;
pub mod gci;
pub mod gcm;
pub mod gcode;
pub mod gcov;
pub mod gdbm;
pub mod gdf;
pub mod gdiff;
pub mod gdmconf;
pub mod gds;
pub mod gedasch;
pub mod gemfile;
pub mod gemlock;
pub mod gemrc;
pub mod gemspec;
pub mod genbank;
pub mod geo;
pub mod geohash;
pub mod geojson;
pub mod geometry;
pub mod gerber;
pub mod gerbera;
pub mod gethconf;
pub mod getmailrc;
pub mod gexf;
pub mod gf;
pub mod gf2;
pub mod gff;
pub mod gguf;
pub mod ghosttyconf;
pub mod gif;
pub mod gift;
pub mod git;
pub mod gitattributes;
pub mod gitconfig;
pub mod giteaaction;
pub mod giteaapp;
pub mod gitidx;
pub mod gitignore;
pub mod gitlabci;
pub mod gitlabrb;
pub mod gitleaks;
pub mod gitmodules;
pub mod gitpack;
pub mod gitsecret;
pub mod gitwebconf;
pub mod gjk;
pub mod glade;
pub mod glb;
pub mod glob;
pub mod glsl;
pub mod gltf;
pub mod glusterfs;
pub mod gml;
pub mod gn;
pub mod gnoise;
pub mod gnuplot;
pub mod goap;
pub mod godot;
pub mod goertzel;
pub mod gogsconf;
pub mod golangci;
pub mod gomod;
pub mod goreleaser;
pub mod gostconf;
pub mod gosum;
pub mod gp;
pub mod gpkg;
pub mod gpsd;
pub mod gpx;
pub mod gqrxconf;
pub mod gradle;
pub mod gradlemod;
pub mod grafana;
pub mod grafanaop;
pub mod graph;
pub mod graphite;
pub mod graphml;
pub mod graphql;
pub mod gray;
pub mod grd;
pub mod gre;
pub mod greatexp;
pub mod greetd;
pub mod grib;
pub mod gridcast;
pub mod gro;
pub mod grok;
pub mod group;
pub mod grp;
pub mod grubcfg;
pub mod grubconf;
pub mod grubenv;
pub mod grundy;
pub mod grype;
pub mod gtksrclang;
pub mod gtp;
pub mod gxf;
pub mod gym;
pub mod gzip;
pub mod h2oconf;
pub mod hacf;
pub mod hadolintconf;
pub mod hadoopconf;
pub mod halfplane;
pub mod halton;
pub mod hamdp;
pub mod hanoi;
pub mod haproxy;
pub mod har;
pub mod harakaconf;
pub mod harbor;
pub mod haresources;
pub mod harness;
pub mod hb;
pub mod hcl;
pub mod hdlc;
pub mod hdr;
pub mod headscaleconf;
pub mod heif;
pub mod helix;
pub mod helmfile;
pub mod hes;
pub mod hexchat;
pub mod hexgrid;
pub mod hexo;
pub mod hfe;
pub mod hfs;
pub mod hfsm;
pub mod hfsplus;
pub mod hgignore;
pub mod hgrc;
pub mod hgt;
pub mod hiawatha;
pub mod hiberfil;
pub mod himalayaconf;
pub mod hirschberg;
pub mod histrect;
pub mod hivemq;
pub mod hl7;
pub mod hld;
pub mod hll;
pub mod hlsl;
pub mod hmac;
pub mod hmm;
pub mod hocr;
pub mod homeassistant;
pub mod hopconf;
pub mod hoppscotch;
pub mod hornsat;
pub mod hostapd;
pub mod hostmeta;
pub mod hosts;
pub mod hpgl;
pub mod hqx;
pub mod htaccess;
pub mod htpasswd;
pub mod http;
pub mod httpfile;
pub mod hud;
pub mod huffman;
pub mod hugoconf;
pub mod hungarian;
pub mod hus;
pub mod hydra;
pub mod hydraml;
pub mod hydrogen;
pub mod hyperfine;
pub mod hyprland;
pub mod hysteriaconf;
pub mod i3conf;
pub mod ibmmq;
pub mod ical;
pub mod icc;
pub mod icecast;
pub mod icinga;
pub mod icmp;
pub mod icns;
pub mod ico;
pub mod ics;
pub mod id3;
pub mod ideavim;
pub mod identify;
pub mod idl;
pub mod ifc;
pub mod ifd;
pub mod iff;
pub mod iges;
pub mod igmp;
pub mod iheap;
pub mod ik;
pub mod imap;
pub mod imd;
pub mod imptreap;
pub mod imscc;
pub mod imx;
pub mod ines;
pub mod inffile;
pub mod infinispan;
pub mod inflate;
pub mod influence;
pub mod influx;
pub mod ini;
pub mod inittab;
pub mod inp;
pub mod inputbuf;
pub mod inputrc;
pub mod insomnia;
pub mod instana;
pub mod integral;
pub mod interfaces;
pub mod interfile;
pub mod interval;
pub mod intervalgraph;
pub mod intervaltree;
pub mod intoto;
pub mod inventory;
pub mod ioc;
pub mod ion;
pub mod iosconf;
pub mod ip;
pub mod ipac;
pub mod ipfix;
pub mod ipk;
pub mod ips;
pub mod ipset;
pub mod iptablessave;
pub mod iptc;
pub mod ipv4;
pub mod ipv6;
pub mod ipxact;
pub mod ipxescript;
pub mod ipythonconf;
pub mod irc;
pub mod ircam;
pub mod irssi;
pub mod isabelle;
pub mod isakmp;
pub mod isbn;
pub mod isc;
pub mod iscsi;
pub mod isis;
pub mod ismn;
pub mod iso2022;
pub mod iso8583;
pub mod iso9660;
pub mod isobmff;
pub mod isortconf;
pub mod issn;
pub mod istio;
pub mod it;
pub mod iterm;
pub mod itermdyn;
pub mod iti;
pub mod iv;
pub mod ivf;
pub mod ivy;
pub mod iwdconf;
pub mod jackrc;
pub mod jacobi;
pub mod jar;
pub mod jaro;
pub mod jats;
pub mod jbig2;
pub mod jdx;
pub mod jed;
pub mod jef;
pub mod jekyll;
pub mod jellyfin;
pub mod jenkinsfile;
pub mod jenkinsx;
pub mod jest;
pub mod jffs2;
pub mod jfm;
pub mod jfr;
pub mod jfs;
pub mod jks;
pub mod jmh;
pub mod josephus;
pub mod journal;
pub mod journaldconf;
pub mod jp2;
pub mod jpeg;
pub mod jps;
pub mod jq;
pub mod json;
pub mod jsonnet;
pub mod jsonpath;
pub mod jtl;
pub mod jumplist;
pub mod junit;
pub mod junos;
pub mod jupyterconf;
pub mod justfile;
pub mod jwe;
pub mod jwk;
pub mod jwt;
pub mod jxl;
pub mod jxr;
pub mod k;
pub mod k0sconf;
pub mod k3d;
pub mod k3sconf;
pub mod k6;
pub mod k8gb;
pub mod kafka;
pub mod kalman;
pub mod kamaji;
pub mod kanata;
pub mod kap;
pub mod kapitan;
pub mod karatsuba;
pub mod karmaconf;
pub mod karp;
pub mod karpenter;
pub mod katesyntax;
pub mod kbm;
pub mod kcl;
pub mod kconfig;
pub mod kcore;
pub mod kdbx;
pub mod kdeglobals;
pub mod kdf;
pub mod kdtree;
pub mod keda;
pub mod kedro;
pub mod keepalived;
pub mod keepassxc;
pub mod kern;
pub mod ketl;
pub mod keto;
pub mod keycloak;
pub mod keyd;
pub mod keydbconf;
pub mod keymap;
pub mod keytab;
pub mod kibana;
pub mod kicadpcb;
pub mod kicadpro;
pub mod kicadsch;
pub mod kickstart;
pub mod kif;
pub mod kindconf;
pub mod kittyconf;
pub mod kittyimg;
pub mod kkpart;
pub mod klipperconf;
pub mod kmeans;
pub mod kml;
pub mod kmp;
pub mod kmv;
pub mod kmz;
pub mod knapsack;
pub mod knative;
pub mod knexfile;
pub mod knownhosts;
pub mod knx;
pub mod kodiadv;
pub mod kong;
pub mod kopia;
pub mod kpaths;
pub mod kql;
pub mod kratos;
pub mod krb5conf;
pub mod kserve;
pub mod kss;
pub mod ktlint;
pub mod kubeconfig;
pub mod kubedb;
pub mod kubeflow;
pub mod kubeflowtraining;
pub mod kubemq;
pub mod kubevela;
pub mod kubevirt;
pub mod kuma;
pub mod kustomize;
pub mod kyverno;
pub mod l2tp;
pub mod lab;
pub mod las;
pub mod lazyseg;
pub mod lca;
pub mod lcov;
pub mod lcp;
pub mod ldap;
pub mod ldapconf;
pub mod ldblog;
pub mod ldif;
pub mod ldirectord;
pub mod ldtk;
pub mod le;
pub mod lean;
pub mod leda;
pub mod ledgerjournal;
pub mod lef;
pub mod lefthook;
pub mod leftist;
pub mod lego;
pub mod leiningen;
pub mod lerna;
pub mod lf;
pub mod lha;
pub mod liberty;
pub mod lidarr;
pub mod life;
pub mod lightdm;
pub mod lightmap;
pub mod lighttpd;
pub mod lima;
pub mod limine;
pub mod linkcut;
pub mod linkerd;
pub mod linrec;
pub mod lintstaged;
pub mod liquibase;
pub mod lis;
pub mod lit;
pub mod litmus;
pub mod lldp;
pub mod llmnr;
pub mod llvmbc;
pub mod lmms;
pub mod lndconf;
pub mod lnk;
pub mod loader;
pub mod locxml;
pub mod log4j;
pub mod log4perl;
pub mod logback;
pub mod logindefs;
pub mod logrotate;
pub mod logstash;
pub mod loki;
pub mod longhorn;
pub mod loveconf;
pub mod lp;
pub mod lpf;
pub mod lrc;
pub mod lru;
pub mod lsf;
pub mod lsm;
pub mod lsystem;
pub mod ltsconf;
pub mod lttb;
pub mod luac;
pub mod lucas;
pub mod lucene;
pub mod luhn;
pub mod luigi;
pub mod lvmconf;
pub mod lwo;
pub mod lww;
pub mod ly;
pub mod lyndon;
pub mod lynisconf;
pub mod lz4;
pub mod lz4f;
pub mod lzfse;
pub mod lzip;
pub mod lzss;
pub mod lzw;
pub mod m3u;
pub mod macaroon;
pub mod macho;
pub mod maddyconf;
pub mod maf;
pub mod magefile;
pub mod magic;
pub mod mailcap;
pub mod maildir;
pub mod maildrop;
pub mod makefile;
pub mod mameconf;
pub mod manacher;
pub mod mapfile;
pub mod mapgen;
pub mod mapnikxml;
pub mod mapproxyconf;
pub mod marc;
pub mod markdownlint;
pub mod markov;
pub mod marlinconf;
pub mod mat;
pub mod matchain;
pub mod matplotlibrc;
pub mod matterbridge;
pub mod mattermost;
pub mod maud;
pub mod mavlink;
pub mod maze;
pub mod mbedapp;
pub mod mbox;
pub mod mbsyncrc;
pub mod mbtiles;
pub mod mbus;
pub mod mcap;
pub mod mcflow;
pub mod mch;
pub mod mcr;
pub mod mcserverprops;
pub mod mcts;
pub mod md2;
pub mod md3;
pub mod md5;
pub mod mdb;
pub mod mdl;
pub mod mdp;
pub mod mdx;
pub mod med;
pub mod mediamtx;
pub mod mediawiki;
pub mod medline;
pub mod mednafen;
pub mod meetmid;
pub mod mei;
pub mod meili;
pub mod melonds;
pub mod meltano;
pub mod memcachedconf;
pub mod menu;
pub mod mergify;
pub mod merkle;
pub mod mermaid;
pub mod meson;
pub mod meta;
pub mod metaflow;
pub mod metal3;
pub mod metallb;
pub mod metallib;
pub mod metricbeat;
pub mod metroconf;
pub mod mets;
pub mod mft;
pub mod mhd;
pub mod mht;
pub mod midi;
pub mod miller;
pub mod milvusconf;
pub mod mime;
pub mod mimirconf;
pub mod minc;
pub mod mincircle;
pub mod mincut;
pub mod minhash;
pub mod minica;
pub mod minidlna;
pub mod minikubeconf;
pub mod minimax;
pub mod minio;
pub mod minix;
pub mod minkowski;
pub mod minq;
pub mod mis;
pub mod mise;
pub mod misp;
pub mod mix;
pub mod mixexs;
pub mod mixxx;
pub mod mkdocs;
pub mod mlflow;
pub mod mmheap;
pub mod mml;
pub mod mmlstyle;
pub mod mo;
pub mod mobi;
pub mod mobius;
pub mod mochajson;
pub mod mocharc;
pub mod modbus;
pub mod modeldo;
pub mod modfile;
pub mod modlin;
pub mod modprobeconf;
pub mod mods;
pub mod modsecurity;
pub mod mol;
pub mod mol2;
pub mod monero;
pub mod mongod;
pub mod monit;
pub mod moonrakerconf;
pub mod moonrepo;
pub mod mopidy;
pub mod mosquitto;
pub mod motionconf;
pub mod movelang;
pub mod mp3;
pub mod mpc;
pub mod mpd;
pub mod mpegts;
pub mod mphf;
pub mod mpl2;
pub mod mplayerconf;
pub mod mpq;
pub mod mps;
pub mod mpv;
pub mod mpvconf;
pub mod mqtt;
pub mod mrc;
pub mod mscx;
pub mod mseed;
pub mod msf;
pub mod msglog;
pub mod msgpack;
pub mod msh;
pub mod msi;
pub mod msmtprc;
pub mod msquares;
pub mod mstree;
pub mod mt;
pub mod mt940;
pub mod mtm;
pub mod mtx;
pub mod multimap;
pub mod multus;
pub mod murmur;
pub mod musicxml;
pub mod muttrc;
pub mod mvnsettings;
pub mod mvt;
pub mod mxf;
pub mod mypyconf;
pub mod mysql;
pub mod mzml;
pub mod nagios;
pub mod namedconf;
pub mod nanoid;
pub mod nanorc;
pub mod nas;
pub mod nats;
pub mod navidrome;
pub mod naxsiconf;
pub mod nbd;
pub mod nbt;
pub mod nc;
pub mod ncmpcpp;
pub mod ncpdp;
pub mod ne;
pub mod nebulaconf;
pub mod nef;
pub mod neo4jconf;
pub mod neomuttconf;
pub mod nerdctl;
pub mod netflow;
pub mod netinput;
pub mod netlifyconf;
pub mod netplan;
pub mod netrc;
pub mod networkd;
pub mod neu;
pub mod newick;
pub mod newrelic;
pub mod newsboat;
pub mod newsyslog;
pub mod nextflow;
pub mod nexus;
pub mod nfpm;
pub mod nfsexports;
pub mod nftconf;
pub mod nginx;
pub mod ngircd;
pub mod nib;
pub mod nickel;
pub mod nififlow;
pub mod nifti;
pub mod nimble;
pub mod ninja;
pub mod nist;
pub mod nix;
pub mod nixconf;
pub mod nlogconf;
pub mod nmconnection;
pub mod nmea;
pub mod nntp;
pub mod nodered;
pub mod noise;
pub mod nomad;
pub mod npmlock;
pub mod npmrc;
pub mod npy;
pub mod npz;
pub mod nrg;
pub mod nrrd;
pub mod nsd;
pub mod nsf;
pub mod nsjailcfg;
pub mod nslcdconf;
pub mod nsqconf;
pub mod nsswitch;
pub mod ntfs;
pub mod ntheory;
pub mod ntp;
pub mod ntpconf;
pub mod ntpsec;
pub mod nuconf;
pub mod nuget;
pub mod nugetconfig;
pub mod nullmailerconf;
pub mod nunit;
pub mod nuxt;
pub mod nwc;
pub mod nxconf;
pub mod nzbget;
pub mod oai;
pub mod oathkeeper;
pub mod obd;
pub mod obj;
pub mod obsconf;
pub mod observe;
pub mod ocsp;
pub mod octaverc;
pub mod octoprint;
pub mod octree;
pub mod odbcini;
pub mod odex;
pub mod odf;
pub mod oem;
pub mod off;
pub mod offlineimap;
pub mod offlinelca;
pub mod ofx;
pub mod ogg;
pub mod ogmo;
pub mod okteto;
pub mod ole;
pub mod olm;
pub mod omm;
pub mod onion;
pub mod onnx;
pub mod op2;
pub mod opam;
pub mod opb;
pub mod openapi;
pub mod openbgpd;
pub mod opendkim;
pub mod opendmarc;
pub mod openebs;
pub mod openfaas;
pub mod openfga;
pub mod openhab;
pub mod openlane;
pub mod openmsx;
pub mod openntpd;
pub mod openpulse;
pub mod openrc;
pub mod opensearch;
pub mod opensearchop;
pub mod openssl;
pub mod opentsdb;
pub mod openvpn;
pub mod opml;
pub mod opsjson;
pub mod optionrom;
pub mod orcaslicer;
pub mod orcid;
pub mod orf;
pub mod org;
pub mod orset;
pub mod ortho;
pub mod osc;
pub mod osm;
pub mod osm2pgsqlstyle;
pub mod osmpbf;
pub mod ospf;
pub mod osqueryconf;
pub mod ossecconf;
pub mod ost;
pub mod osv;
pub mod otelcol;
pub mod otf;
pub mod otp;
pub mod otpauth;
pub mod otsu;
pub mod overpass;
pub mod ovf;
pub mod p7b;
pub mod pacemaker;
pub mod pack;
pub mod packer;
pub mod packfile;
pub mod packit;
pub mod pacman;
pub mod paf;
pub mod pagerank;
pub mod pain;
pub mod pairingheap;
pub mod pajek;
pub mod pak;
pub mod pamstack;
pub mod pants;
pub mod paraver;
pub mod parityconf;
pub mod parquet;
pub mod parrec;
pub mod parser;
pub mod partitions;
pub mod paseto;
pub mod passability;
pub mod passwd;
pub mod pat;
pub mod pathcover;
pub mod pathfinding;
pub mod patricia;
pub mod pbs;
pub mod pcap;
pub mod pcapng;
pub mod pcd;
pub mod pcf;
pub mod pcg;
pub mod pchip;
pub mod pck;
pub mod pcl;
pub mod pcsx2conf;
pub mod pcx;
pub mod pdb;
pub mod pdf;
pub mod pdns;
pub mod pds;
pub mod pec;
pub mod peg;
pub mod pell;
pub mod pem;
pub mod perceptron;
pub mod percona;
pub mod perflog;
pub mod perm;
pub mod permissions;
pub mod pes;
pub mod pfconf;
pub mod pfm;
pub mod pgm;
pub mod pgn;
pub mod pgo;
pub mod pgp;
pub mod pgpass;
pub mod pgservice;
pub mod phabricatorconf;
pub mod phylip;
pub mod picard;
pub mod pickle;
pub mod pid;
pub mod pidginconf;
pub mod piecetable;
pub mod pihole;
pub mod pileup;
pub mod pim;
pub mod pinerc;
pub mod pinpoint;
pub mod pipewireconf;
pub mod pipfile;
pub mod pjs;
pub mod pk;
pub mod pkcs12;
pub mod pkcs8;
pub mod pkg;
pub mod pkgbuild;
pub mod pkl;
pub mod pl;
pub mod plan;
pub mod planetilerconf;
pub mod plantuml;
pub mod platformio;
pub mod platformsh;
pub mod playwrightconf;
pub mod plexconf;
pub mod plist;
pub mod plot;
pub mod pls;
pub mod ply;
pub mod pm2;
pub mod pmacctconf;
pub mod pmd;
pub mod pmml;
pub mod pmtiles;
pub mod pmx;
pub mod png;
pub mod pnm;
pub mod pnpmlock;
pub mod pnpmworkspace;
pub mod po;
pub mod pod;
pub mod podfile;
pub mod poetry;
pub mod poisson;
pub mod policyjson;
pub mod poly;
pub mod poly1305;
pub mod polya;
pub mod polybar;
pub mod polyclip;
pub mod polylabel;
pub mod pom;
pub mod pomerium;
pub mod pool;
pub mod pop3;
pub mod portage;
pub mod porter;
pub mod portworx;
pub mod poscar;
pub mod postalconf;
pub mod postcss;
pub mod postfix;
pub mod postgresql;
pub mod postman;
pub mod postsrsdconf;
pub mod ppf;
pub mod ppp;
pub mod pppdconf;
pub mod pprof;
pub mod ppssppconf;
pub mod pptpd;
pub mod pptx;
pub mod precommit;
pub mod prefect;
pub mod prefetch;
pub mod premakeconf;
pub mod preseed;
pub mod prettier;
pub mod privoxy;
pub mod prj;
pub mod procd;
pub mod procfile;
pub mod procmailrc;
pub mod profiler;
pub mod proftpd;
pub mod progression;
pub mod projjson;
pub mod prom;
pub mod promela;
pub mod prometheus;
pub mod promoperator;
pub mod promtailconf;
pub mod prop;
pub mod proselint;
pub mod prosody;
pub mod proto;
pub mod prow;
pub mod prowlarr;
pub mod prusaslicer;
pub mod psd;
pub mod psf;
pub mod psid;
pub mod pstree;
pub mod ptm;
pub mod ptp4l;
pub mod ptx;
pub mod pubspec;
pub mod pulsar;
pub mod pulseclientconf;
pub mod pulumi;
pub mod punycode;
pub mod puppet;
pub mod pureftpd;
pub mod puz;
pub mod pxelinux;
pub mod pyc;
pub mod pylintrc;
pub mod pypirc;
pub mod pyproject;
pub mod pyrightconf;
pub mod pyroconf;
pub mod pytestbench;
pub mod pzserver;
pub mod qasm;
pub mod qbittorrent;
pub mod qcow2;
pub mod qcp;
pub mod qdrantconf;
pub mod qgsproj;
pub mod qif;
pub mod qlearn;
pub mod qmakepro;
pub mod qmap;
pub mod qobj;
pub mod qoi;
pub mod qpf;
pub mod qr;
pub mod qs;
pub mod qsf;
pub mod qt5ctconf;
pub mod qti;
pub mod qtui;
pub mod quadtree;
pub mod quantile;
pub mod quartz;
pub mod quat;
pub mod quest;
pub mod quil;
pub mod rabbitmq;
pub mod radarr;
pub mod radius;
pub mod radiusd;
pub mod radixheap;
pub mod radixsort;
pub mod raf;
pub mod railwayconf;
pub mod rakefile;
pub mod raml;
pub mod random_table;
pub mod rankselect;
pub mod rans;
pub mod rar;
pub mod ras;
pub mod raster;
pub mod ratbezier;
pub mod ray;
pub mod razorconf;
pub mod rc;
pub mod rcloneconf;
pub mod rdata;
pub mod rdb;
pub mod rdiff;
pub mod rdp;
pub mod rdpfile;
pub mod readarr;
pub mod reaper;
pub mod rebarconfig;
pub mod recbin;
pub mod recipe;
pub mod recordio;
pub mod recovery;
pub mod rectunion;
pub mod redblack;
pub mod redisconf;
pub mod redpanda;
pub mod redpen;
pub mod refind;
pub mod regex;
pub mod regf;
pub mod regfile;
pub mod registriesconf;
pub mod rego;
pub mod reiserfs;
pub mod relations;
pub mod relaxng;
pub mod releaseplease;
pub mod releaserc;
pub mod remminaconf;
pub mod renderconf;
pub mod renovate;
pub mod renviron;
pub mod replay;
pub mod requirements;
pub mod reroot;
pub mod res;
pub mod resolv;
pub mod resp;
pub mod resticprofile;
pub mod resx;
pub mod retroarch;
pub mod reviveconf;
pub mod revlog;
pub mod rf64;
pub mod rfa;
pub mod rfb;
pub mod rinex;
pub mod rip;
pub mod ris;
pub mod rkhunter;
pub mod rle;
pub mod rm;
pub mod rmq;
pub mod rng;
pub mod rng_xoshiro;
pub mod roaring;
pub mod robin;
pub mod rocketmq;
pub mod rockspec;
pub mod roff;
pub mod rofi;
pub mod rollback;
pub mod rolling;
pub mod rollup;
pub mod rook;
pub mod roots;
pub mod rope;
pub mod roq;
pub mod rosbag;
pub mod rotcal;
pub mod routeros;
pub mod rpcs3conf;
pub mod rpgmakerconf;
pub mod rpm;
pub mod rprofile;
pub mod rpy;
pub mod rsa;
pub mod rsfec;
pub mod rsnapshot;
pub mod rspamdconf;
pub mod rss;
pub mod rss2email;
pub mod rst;
pub mod rsync;
pub mod rsyslogd;
pub mod rtcp;
pub mod rtf;
pub mod rtorrent;
pub mod rtp;
pub mod rtree;
pub mod rtsp;
pub mod rubocop;
pub mod ruffconf;
pub mod rundeck;
pub mod runit;
pub mod rvdata;
pub mod rw2;
pub mod rx2;
pub mod s3m;
pub mod s6rc;
pub mod s7;
pub mod s98;
pub mod sabnzbd;
pub mod sac;
pub mod saddleback;
pub mod safetensors;
pub mod saif;
pub mod sais;
pub mod salt;
pub mod sam;
pub mod samba;
pub mod samhainconf;
pub mod saml;
pub mod sap;
pub mod sarif;
pub mod sas7bdat;
pub mod sat;
pub mod sauce;
pub mod sav;
pub mod savefile;
pub mod sbf;
pub mod sbi;
pub mod sbt;
pub mod sbus;
pub mod sbv;
pub mod sby;
pub mod scandata;
pub mod scapegoat;
pub mod scc;
pub mod sch;
pub mod scl;
pub mod sconstruct;
pub mod scp;
pub mod scummvm;
pub mod scyllaop;
pub mod sdc;
pub mod sddmconf;
pub mod sdf;
pub mod sdkconfig;
pub mod sdrppconf;
pub mod sealedsecrets;
pub mod seamcarve;
pub mod seccomp;
pub mod secretsbaseline;
pub mod secretsstore;
pub mod securitytxt;
pub mod segbeats;
pub mod seglazy;
pub mod segment;
pub mod segtree;
pub mod segy;
pub mod seldon;
pub mod selinuxfc;
pub mod selinuxte;
pub mod semgrep;
pub mod semver;
pub mod sendmail;
pub mod sentinel;
pub mod sequelizerc;
pub mod serializer;
pub mod serilog;
pub mod serverless;
pub mod serverprop;
pub mod setupcfg;
pub mod sevendtdxml;
pub mod sf2;
pub mod sfc;
pub mod sfd;
pub mod sflow;
pub mod sftp;
pub mod sfv;
pub mod sgf;
pub mod sgi;
pub mod sha1;
pub mod sha256;
pub mod sha3;
pub mod sha512;
pub mod shadow;
pub mod shadowsocksconf;
pub mod shamir;
pub mod shard;
pub mod shellcheckrc;
pub mod shibconf;
pub mod shipwright;
pub mod shop;
pub mod shorewall;
pub mod shp;
pub mod shrink;
pub mod shufflebag;
pub mod shunting;
pub mod sidekiq;
pub mod sieve;
pub mod sievescript;
pub mod sigma;
pub mod sim;
pub mod simhash;
pub mod simplex;
pub mod singboxconf;
pub mod singerconf;
pub mod sip;
pub mod siphash;
pub mod sixel;
pub mod sjis;
pub mod skaffold;
pub mod skiplist;
pub mod skp;
pub mod skywalking;
pub mod slapd;
pub mod slide;
pub mod slob;
pub mod slopetrick;
pub mod slotmap;
pub mod slrnconf;
pub mod slsa;
pub mod slurm;
pub mod smawk;
pub mod smb2;
pub mod smd;
pub mod smi;
pub mod smithy;
pub mod smt2;
pub mod smtp;
pub mod smtpdconf;
pub mod snap;
pub mod snapcast;
pub mod snapcraft;
pub mod snappy;
pub mod sndh;
pub mod snmp;
pub mod snmpdconf;
pub mod snoise;
pub mod snoop;
pub mod snort;
pub mod snowflake;
pub mod snyk;
pub mod sobol;
pub mod socks;
pub mod sol;
pub mod solrconfig;
pub mod solrschema;
pub mod sonar;
pub mod sonarr;
pub mod soniccfg;
pub mod sops;
pub mod sosdp;
pub mod soundex;
pub mod sourcemap;
pub mod sp3;
pub mod spamassassin;
pub mod spark;
pub mod sparql;
pub mod sparse;
pub mod sparse_set;
pub mod spatial_hash;
pub mod spc;
pub mod spdx;
pub mod spec;
pub mod speedscope;
pub mod spef;
pub mod spf;
pub mod sphinx;
pub mod spicenet;
pub mod spigot;
pub mod spire;
pub mod splay;
pub mod spotbugs;
pub mod spring;
pub mod spv;
pub mod sqitchconf;
pub mod sqlite;
pub mod sqlnet;
pub mod squid;
pub mod srcdscfg;
pub mod srhtconf;
pub mod srm;
pub mod srt;
pub mod ssh;
pub mod sshconf;
pub mod sshkey;
pub mod ssmtpconf;
pub mod sssdconf;
pub mod sst;
pub mod ssw;
pub mod stable;
pub mod stack;
pub mod stalwartconf;
pub mod stardict;
pub mod starship;
pub mod stash;
pub mod staticcheckconf;
pub mod stats;
pub mod statsd;
pub mod status;
pub mod steer;
pub mod steiner;
pub mod step;
pub mod stepca;
pub mod stirling;
pub mod stix;
pub mod stl;
pub mod stm;
pub mod stockholm;
pub mod storageconf;
pub mod storybook;
pub mod stp;
pub mod strimzi;
pub mod strings;
pub mod strongswanconf;
pub mod studio3;
pub mod stun;
pub mod stylelint;
pub mod su;
pub mod su2;
pub mod sublime;
pub mod sudoers;
pub mod sudoku;
pub mod suffix;
pub mod sufftree;
pub mod suiconf;
pub mod supabase;
pub mod supervisor;
pub mod surefire;
pub mod suricata;
pub mod svelte;
pub mod svf;
pub mod svg;
pub mod svnconf;
pub mod svndump;
pub mod svp;
pub mod swanctl;
pub mod sway;
pub mod swept;
pub mod swf;
pub mod swid;
pub mod swiftlint;
pub mod swiftmt;
pub mod swiss;
pub mod sylk;
pub mod synapse;
pub mod syncthingconf;
pub mod sysctlconf;
pub mod syslog;
pub mod syslogng;
pub mod sysmonconf;
pub mod systemd;
pub mod systemdboot;
pub mod sysv;
pub mod syx;
pub mod t3d;
pub mod tabbyconf;
pub mod tacacs;
pub mod tact;
pub mod tailscale;
pub mod tailwind;
pub mod talisman;
pub mod tap;
pub mod taprc;
pub mod tar;
pub mod tarantool;
pub mod taskfile;
pub mod tcp;
pub mod tcx;
pub mod td0;
pub mod tdm;
pub mod tds;
pub mod tekton;
pub mod telegraf;
pub mod teleport;
pub mod telnet;
pub mod tempoconf;
pub mod temporal;
pub mod terminal;
pub mod terminfo;
pub mod ternary;
pub mod terragrunt;
pub mod terrariaconf;
pub mod texinfo;
pub mod textile;
pub mod textlayout;
pub mod textlint;
pub mod textmategram;
pub mod tflint;
pub mod tflite;
pub mod tfm;
pub mod tfrecord;
pub mod tftp;
pub mod tga;
pub mod tgf;
pub mod thanosconf;
pub mod threat;
pub mod threemf;
pub mod thrift;
pub mod tidb;
pub mod tiff;
pub mod tilemap;
pub mod tileservergl;
pub mod tilestacheconf;
pub mod tiltfile;
pub mod timer;
pub mod timestep;
pub mod timesyncd;
pub mod tincconf;
pub mod tinyproxy;
pub mod tlaplus;
pub mod tle;
pub mod tlp;
pub mod tls;
pub mod tlsf;
pub mod tmpfilesd;
pub mod tmuxconf;
pub mod tmx;
pub mod tnsnames;
pub mod tomcat;
pub mod toml;
pub mod tonelli;
pub mod topojson;
pub mod torrc;
pub mod torrent;
pub mod tournament;
pub mod toxini;
pub mod tptp;
pub mod traefik;
pub mod transmission;
pub mod travisci;
pub mod treap;
pub mod treesittergram;
pub mod trie;
pub mod trigger;
pub mod tripwireconf;
pub mod trivy;
pub mod trivyop;
pub mod trojanconf;
pub mod trx;
pub mod ts;
pub mod ts3serverini;
pub mod tscn;
pub mod tsconfig;
pub mod tsp;
pub mod tsx;
pub mod tta;
pub mod ttc;
pub mod ttf;
pub mod ttml;
pub mod ttyrec;
pub mod tuicconf;
pub mod tunstall;
pub mod turboconf;
pub mod turn;
pub mod turnpike;
pub mod twee;
pub mod tween;
pub mod twosat;
pub mod txt2tags;
pub mod typeid;
pub mod typeormconf;
pub mod typesense;
pub mod tzif;
pub mod tzx;
pub mod uasset;
pub mod ubi;
pub mod ubootenv;
pub mod ubx;
pub mod ucf;
pub mod uci;
pub mod udevrules;
pub mod udf;
pub mod udiff;
pub mod udp;
pub mod ufs;
pub mod ufwrules;
pub mod uimage;
pub mod ulid;
pub mod ult;
pub mod unattend;
pub mod unbound;
pub mod unison;
pub mod unitconf;
pub mod unityfs;
pub mod unitymanifest;
pub mod unitysettings;
pub mod unocss;
pub mod unrealircd;
pub mod unv;
pub mod upc;
pub mod upf;
pub mod uplugin;
pub mod uproject;
pub mod ups;
pub mod upstart;
pub mod urdf;
pub mod uri;
pub mod urlencode;
pub mod usercss;
pub mod userscript;
pub mod usf;
pub mod usi;
pub mod usnjrnl;
pub mod ust;
pub mod ustx;
pub mod utf16;
pub mod utf8;
pub mod utility;
pub mod utmp;
pub mod uue;
pub mod uuid;
pub mod uuid7;
pub mod v2rayconf;
pub mod vagrant;
pub mod vale;
pub mod validator;
pub mod valkeyconf;
pub mod varint;
pub mod vaultagent;
pub mod vcard;
pub mod vcd;
pub mod vcf;
pub mod vcl;
pub mod vclock;
pub mod vcluster;
pub mod vcpkg;
pub mod vdi;
pub mod veb;
pub mod veb3;
pub mod vec;
pub mod vector;
pub mod velero;
pub mod vercelconf;
pub mod verify;
pub mod verilog;
pub mod verlet;
pub mod vernemq;
pub mod vertexcover;
pub mod vespaconf;
pub mod vf;
pub mod vgm;
pub mod vhd;
pub mod vhdr;
pub mod vhdx;
pub mod victoria;
pub mod vimrc;
pub mod vimsyntax;
pub mod vip;
pub mod virtxml;
pub mod visibility;
pub mod viteconf;
pub mod vitepress;
pub mod viterbi;
pub mod vitess;
pub mod vitestconf;
pub mod vlcrc;
pub mod vlt;
pub mod vmagentconf;
pub mod vmdk;
pub mod vmrk;
pub mod vms;
pub mod vnoise;
pub mod voc;
pub mod volcano;
pub mod voronoi;
pub mod vose;
pub mod votable;
pub mod vox;
pub mod vp3;
pub mod vpk;
pub mod vpr;
pub mod vrrp;
pub mod vrtgdal;
pub mod vscodeconf;
pub mod vsdx;
pub mod vsftpd;
pub mod vsqx;
pub mod vtf;
pub mod vtk;
pub mod vtt;
pub mod vtu;
pub mod vxlan;
pub mod vyper;
pub mod w64;
pub mod wad;
pub mod wafconf;
pub mod wal;
pub mod wallet;
pub mod wandb;
pub mod wasm;
pub mod wav;
pub mod wavelet;
pub mod waybar;
pub mod waypoint;
pub mod wdsu;
pub mod weaviateconf;
pub mod webfinger;
pub mod webloc;
pub mod webmanifest;
pub mod webp;
pub mod webpackconf;
pub mod weechat;
pub mod werf;
pub mod westconf;
pub mod westonconf;
pub mod weztermconf;
pub mod wfc;
pub mod wfdb;
pub mod wfn;
pub mod wgsl;
pub mod whitelist;
pub mod whyml;
pub mod widgetxml;
pub mod wim;
pub mod windowsterminal;
pub mod winini;
pub mod winlogbeat;
pub mod winnow;
pub mod winstonconf;
pub mod wireguard;
pub mod wireplumberconf;
pub mod wiresharkpref;
pub mod wkb;
pub mod wkt;
pub mod wktproj;
pub mod woff;
pub mod woff2;
pub mod woodpecker;
pub mod wordfileuew;
pub mod world_hash;
pub mod worley;
pub mod woz;
pub mod wpasupplicant;
pub mod wrangler;
pub mod wrl;
pub mod ws;
pub mod wsdl;
pub mod wsjtxconf;
pub mod wslconf;
pub mod wv;
pub mod x3d;
pub mod x509;
pub mod x7z;
pub mod xacml;
pub mod xacro;
pub mod xapi;
pub mod xar;
pub mod xbm;
pub mod xbrl;
pub mod xcf;
pub mod xdc;
pub mod xdf;
pub mod xfast;
pub mod xfs;
pub mod xi;
pub mod xib;
pub mod xid;
pub mod xinetdconf;
pub mod xl2tpd;
pub mod xliff;
pub mod xlink;
pub mod xlsx;
pub mod xm;
pub mod xmakeconf;
pub mod xmodmap;
pub mod xmp;
pub mod xmpp;
pub mod xnb;
pub mod xorbasis;
pub mod xorfilter;
pub mod xorgconf;
pub mod xortrie;
pub mod xpath;
pub mod xpm;
pub mod xps;
pub mod xpt;
pub mod xq;
pub mod xqf;
pub mod xrayconf;
pub mod xrdpconf;
pub mod xresources;
pub mod xsd;
pub mod xslt;
pub mod xspf;
pub mod xsvf;
pub mod xunit;
pub mod xxhash;
pub mod xyz;
pub mod xz;
pub mod y4m;
pub mod yamllint;
pub mod yara;
pub mod yarnlock;
pub mod yarnrc;
pub mod yenc;
pub mod yfast;
pub mod yggdrasil;
pub mod yosys;
pub mod ytt;
pub mod yugabyte;
pub mod yuzuconf;
pub mod z64;
pub mod zabbix;
pub mod zapconf;
pub mod zathurarc;
pub mod zeckendorf;
pub mod zedconf;
pub mod zeekconf;
pub mod zeekctl;
pub mod zeekscript;
pub mod zerobfs;
pub mod zerotier;
pub mod zfs;
pub mod zfunc;
pub mod zigbee2mqtt;
pub mod zip;
pub mod zitadel;
pub mod zlib;
pub mod znc;
pub mod zobrist;
pub mod zola;
pub mod zon;
pub mod zone;
pub mod zonemtaconf;
pub mod zoo;
pub mod zookeeper;
pub mod zookeeperop;
pub mod zorder;
pub mod zpaq;
pub mod zpl;
pub mod zshrc;
pub mod zstd;
pub mod zulipconf;
pub mod zypper;

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
    astar, auto_explore, cells_row_major, combine_maps, descend, dijkstra_map, farthest_cell,
    flee_map, flood_fill, is_path_clear, is_reachable, jps, jps4, nearest_reachable,
    octile_distance, path_cost, path_to_direction_vec, smooth_path, step_toward, weighted_astar,
    ConnectivityMap, DijkstraMap,
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

/// 設定/フォーマット検出器の関数型(`detect(&[u8]) -> bool`)。
pub type DetectorFn = fn(&[u8]) -> bool;

/// 設定/フォーマット検出器 `DetectorFn` の全モジュール一覧。
///
/// (モジュール名, 関数ポインタ) の対で、判別器の列挙・一括適用や
/// 全検出器横断の性質テスト(panic非発火など)に使う。
/// `detect` を持つモジュール追加時はこの表にも登録すること。
pub const DETECTORS: &[(&str, DetectorFn)] = &[
    ("a2r", a2r::detect),
    ("ac", ac::detect),
    ("ace", ace::detect),
    ("acemode", acemode::detect),
    ("actionlint", actionlint::detect),
    ("activemq", activemq::detect),
    ("adguard", adguard::detect),
    ("adql", adql::detect),
    ("aercconf", aercconf::detect),
    ("aerospike", aerospike::detect),
    ("aideconf", aideconf::detect),
    ("aiger", aiger::detect),
    ("airbyteconf", airbyteconf::detect),
    ("airflow", airflow::detect),
    ("alacritty", alacritty::detect),
    ("alembic", alembic::detect),
    ("alertmanager", alertmanager::detect),
    ("alexrc", alexrc::detect),
    ("algolia", algolia::detect),
    ("alloy", alloy::detect),
    ("allure", allure::detect),
    ("aln", aln::detect),
    ("alto", alto::detect),
    ("alz", alz::detect),
    ("amandaconf", amandaconf::detect),
    ("ambassador", ambassador::detect),
    ("amfile", amfile::detect),
    ("ampl", ampl::detect),
    ("amplifyconf", amplifyconf::detect),
    ("amr", amr::detect),
    ("angularconf", angularconf::detect),
    ("ansi", ansi::detect),
    ("ansible", ansible::detect),
    ("antex", antex::detect),
    ("aoe", aoe::detect),
    ("apacheconf", apacheconf::detect),
    ("apib", apib::detect),
    ("apisix", apisix::detect),
    ("apk", apk::detect),
    ("apmserver", apmserver::detect),
    ("apparmor", apparmor::detect),
    ("appcache", appcache::detect),
    ("appdaemon", appdaemon::detect),
    ("appdynamics", appdynamics::detect),
    ("appengine", appengine::detect),
    ("appjson", appjson::detect),
    ("appveyor", appveyor::detect),
    ("aprxconf", aprxconf::detect),
    ("apt", apt::detect),
    ("archinstall", archinstall::detect),
    ("ardour", ardour::detect),
    ("arduinoconf", arduinoconf::detect),
    ("argocd", argocd::detect),
    ("argoevents", argoevents::detect),
    ("argorollout", argorollout::detect),
    ("argowf", argowf::detect),
    ("argusconf", argusconf::detect),
    ("aria2", aria2::detect),
    ("arkimeconf", arkimeconf::detect),
    ("arma3conf", arma3conf::detect),
    ("arw", arw::detect),
    ("asciicast", asciicast::detect),
    ("asdf", asdf::detect),
    ("asf", asf::detect),
    ("asn1", asn1::detect),
    ("asoundrc", asoundrc::detect),
    ("astro", astro::detect),
    ("asv", asv::detect),
    ("atlantis", atlantis::detect),
    ("atom", atom::detect),
    ("audacity", audacity::detect),
    ("auditdconf", auditdconf::detect),
    ("auditrule", auditrule::detect),
    ("authelia", authelia::detect),
    ("autofs", autofs::detect),
    ("autoyast", autoyast::detect),
    ("avaconf", avaconf::detect),
    ("awscredentials", awscredentials::detect),
    ("awselb", awselb::detect),
    ("axports", axports::detect),
    ("azurepipe", azurepipe::detect),
    ("babelrc", babelrc::detect),
    ("backstage", backstage::detect),
    ("baculadir", baculadir::detect),
    ("bai2", bai2::detect),
    ("bam", bam::detect),
    ("banditconf", banditconf::detect),
    ("bannedips", bannedips::detect),
    ("base32", base32::detect),
    ("bashrc", bashrc::detect),
    ("bazarr", bazarr::detect),
    ("bazel", bazel::detect),
    ("bbcode", bbcode::detect),
    ("beets", beets::detect),
    ("benchstat", benchstat::detect),
    ("bentoml", bentoml::detect),
    ("bicep", bicep::detect),
    ("biome", biome::detect),
    ("bird", bird::detect),
    ("bit", bit::detect),
    ("bitbucketpipes", bitbucketpipes::detect),
    ("bitcoinconf", bitcoinconf::detect),
    ("bitrise", bitrise::detect),
    ("blackbird", blackbird::detect),
    ("blackconf", blackconf::detect),
    ("blocky", blocky::detect),
    ("bogofilter", bogofilter::detect),
    ("bootini", bootini::detect),
    ("borgmatic", borgmatic::detect),
    ("bors", bors::detect),
    ("boundary", boundary::detect),
    ("braket", braket::detect),
    ("brewfile", brewfile::detect),
    ("browserconfig", browserconfig::detect),
    ("browserslist", browserslist::detect),
    ("bru", bru::detect),
    ("bsnes", bsnes::detect),
    ("btrbk", btrbk::detect),
    ("btrfs", btrfs::detect),
    ("buck", buck::detect),
    ("buildkitd", buildkitd::detect),
    ("buildkite", buildkite::detect),
    ("bukkit", bukkit::detect),
    ("bundlerconf", bundlerconf::detect),
    ("bunfig", bunfig::detect),
    ("cabal", cabal::detect),
    ("caddyfile", caddyfile::detect),
    ("cairo", cairo::detect),
    ("calamares", calamares::detect),
    ("calico", calico::detect),
    ("callgrind", callgrind::detect),
    ("camt", camt::detect),
    ("capacitor", capacitor::detect),
    ("capsule", capsule::detect),
    ("capx", capx::detect),
    ("cardanoconf", cardanoconf::detect),
    ("cargoconf", cargoconf::detect),
    ("cargolock", cargolock::detect),
    ("carla", carla::detect),
    ("cartocss", cartocss::detect),
    ("carvel", carvel::detect),
    ("casbin", casbin::detect),
    ("casdoor", casdoor::detect),
    ("cassandra", cassandra::detect),
    ("ccs", ccs::detect),
    ("cedar", cedar::detect),
    ("cephconf", cephconf::detect),
    ("cerbos", cerbos::detect),
    ("certbot", certbot::detect),
    ("certmanager", certmanager::detect),
    ("cfn", cfn::detect),
    ("cfssl", cfssl::detect),
    ("cgitrc", cgitrc::detect),
    ("changesets", changesets::detect),
    ("chaosmesh", chaosmesh::detect),
    ("chart", chart::detect),
    ("chasquidconf", chasquidconf::detect),
    ("checkov", checkov::detect),
    ("chef", chef::detect),
    ("cherokee", cherokee::detect),
    ("chirpcsv", chirpcsv::detect),
    ("chromaconf", chromaconf::detect),
    ("chrometrace", chrometrace::detect),
    ("chronyconf", chronyconf::detect),
    ("cibxml", cibxml::detect),
    ("cilium", cilium::detect),
    ("circleci", circleci::detect),
    ("cirrus", cirrus::detect),
    ("citraconf", citraconf::detect),
    ("clangformat", clangformat::detect),
    ("clangtidy", clangtidy::detect),
    ("clar", clar::detect),
    ("clashconf", clashconf::detect),
    ("clickhouse", clickhouse::detect),
    ("cliff", cliff::detect),
    ("cloudcustodian", cloudcustodian::detect),
    ("cloudinit", cloudinit::detect),
    ("clusterapi", clusterapi::detect),
    ("clusterconf", clusterconf::detect),
    ("cmake", cmake::detect),
    ("cmdbat", cmdbat::detect),
    ("cmus", cmus::detect),
    ("cnpg", cnpg::detect),
    ("cob", cob::detect),
    ("cockroach", cockroach::detect),
    ("cocosproj", cocosproj::detect),
    ("codeclimate", codeclimate::detect),
    ("codecov", codecov::detect),
    ("codespell", codespell::detect),
    ("colima", colima::detect),
    ("collectd", collectd::detect),
    ("commitlint", commitlint::detect),
    ("compose", compose::detect),
    ("composer", composer::detect),
    ("composerlock", composerlock::detect),
    ("conanfile", conanfile::detect),
    ("concourse", concourse::detect),
    ("condaenv", condaenv::detect),
    ("condarc", condarc::detect),
    ("configureac", configureac::detect),
    ("consul", consul::detect),
    ("containerd", containerd::detect),
    ("containersconf", containersconf::detect),
    ("contourconf", contourconf::detect),
    ("cookiejar", cookiejar::detect),
    ("coq", coq::detect),
    ("corefile", corefile::detect),
    ("corosync", corosync::detect),
    ("coveralls", coveralls::detect),
    ("cpanfile", cpanfile::detect),
    ("cpplint", cpplint::detect),
    ("cr2", cr2::detect),
    ("creole", creole::detect),
    ("crio", crio::detect),
    ("criterion", criterion::detect),
    ("crmconf", crmconf::detect),
    ("crockford", crockford::detect),
    ("cromwell", cromwell::detect),
    ("crossplane", crossplane::detect),
    ("crowdsec", crowdsec::detect),
    ("csa", csa::detect),
    ("csd", csd::detect),
    ("cspell", cspell::detect),
    ("ctrf", ctrf::detect),
    ("cuid", cuid::detect),
    ("cupsconf", cupsconf::detect),
    ("curaconf", curaconf::detect),
    ("cve", cve::detect),
    ("cypher", cypher::detect),
    ("cypressconf", cypressconf::detect),
    ("dae", dae::detect),
    ("dafny", dafny::detect),
    ("dagster", dagster::detect),
    ("dapr", dapr::detect),
    ("dask", dask::detect),
    ("datadog", datadog::detect),
    ("db2cli", db2cli::detect),
    ("dbm", dbm::detect),
    ("dbt", dbt::detect),
    ("debconf", debconf::detect),
    ("defaultpa", defaultpa::detect),
    ("defconfig", defconfig::detect),
    ("defoldproj", defoldproj::detect),
    ("dehydrated", dehydrated::detect),
    ("deluge", deluge::detect),
    ("denoconf", denoconf::detect),
    ("denyhosts", denyhosts::detect),
    ("dependabot", dependabot::detect),
    ("detekt", detekt::detect),
    ("devbox", devbox::detect),
    ("devcontainer", devcontainer::detect),
    ("devfile", devfile::detect),
    ("devspace", devspace::detect),
    ("dexidp", dexidp::detect),
    ("dgml", dgml::detect),
    ("dhall", dhall::detect),
    ("dhclientconf", dhclientconf::detect),
    ("dhcpcdconf", dhcpcdconf::detect),
    ("dictd", dictd::detect),
    ("dictzip", dictzip::detect),
    ("did", did::detect),
    ("digikamrc", digikamrc::detect),
    ("dimacs", dimacs::detect),
    ("dinit", dinit::detect),
    ("direwolfconf", direwolfconf::detect),
    ("discourse", discourse::detect),
    ("dita", dita::detect),
    ("dnfconf", dnfconf::detect),
    ("dng", dng::detect),
    ("dnsmasq", dnsmasq::detect),
    ("docbook", docbook::detect),
    ("dockerdaemon", dockerdaemon::detect),
    ("dockerignore", dockerignore::detect),
    ("docsify", docsify::detect),
    ("docusaurus", docusaurus::detect),
    ("dolphinconf", dolphinconf::detect),
    ("dosboxconf", dosboxconf::detect),
    ("dossys", dossys::detect),
    ("dot", dot::detect),
    ("dotenv", dotenv::detect),
    ("dovecot", dovecot::detect),
    ("dpx", dpx::detect),
    ("dragonflyconf", dragonflyconf::detect),
    ("drawio", drawio::detect),
    ("drbdconf", drbdconf::detect),
    ("drone", drone::detect),
    ("ds9reg", ds9reg::detect),
    ("dsig", dsig::detect),
    ("dsl", dsl::detect),
    ("dtd", dtd::detect),
    ("dune", dune::detect),
    ("dunst", dunst::detect),
    ("duplicacy", duplicacy::detect),
    ("dvcfile", dvcfile::detect),
    ("dvi", dvi::detect),
    ("dxbc", dxbc::detect),
    ("ead", ead::detect),
    ("eaglexml", eaglexml::detect),
    ("ean", ean::detect),
    ("earthly", earthly::detect),
    ("easyeffects", easyeffects::detect),
    ("easyrsa", easyrsa::detect),
    ("eck", eck::detect),
    ("ecsv", ecsv::detect),
    ("editorconfig", editorconfig::detect),
    ("edn", edn::detect),
    ("edsk", edsk::detect),
    ("ejabberd", ejabberd::detect),
    ("elasticsearch", elasticsearch::detect),
    ("emacs", emacs::detect),
    ("emqx", emqx::detect),
    ("envoy", envoy::detect),
    ("envrc", envrc::detect),
    ("epd", epd::detect),
    ("epwing", epwing::detect),
    ("eslintrc", eslintrc::detect),
    ("esmapping", esmapping::detect),
    ("esphome", esphome::detect),
    ("essettings", essettings::detect),
    ("etcd", etcd::detect),
    ("eula", eula::detect),
    ("excalidraw", excalidraw::detect),
    ("exim", exim::detect),
    ("exr", exr::detect),
    ("externalsecrets", externalsecrets::detect),
    ("extmanifest", extmanifest::detect),
    ("f2fs", f2fs::detect),
    ("factoriosettings", factoriosettings::detect),
    ("fail2ban", fail2ban::detect),
    ("fail2banconf", fail2banconf::detect),
    ("falcoconf", falcoconf::detect),
    ("far", far::detect),
    ("fceux", fceux::detect),
    ("fcoe", fcoe::detect),
    ("feast", feast::detect),
    ("ferm", ferm::detect),
    ("fetchmailconf", fetchmailconf::detect),
    ("fhir", fhir::detect),
    ("fidl", fidl::detect),
    ("filebeat", filebeat::detect),
    ("firebase", firebase::detect),
    ("firejailprof", firejailprof::detect),
    ("firewalld", firewalld::detect),
    ("fishconf", fishconf::detect),
    ("fivetranconf", fivetranconf::detect),
    ("fixml", fixml::detect),
    ("flagger", flagger::detect),
    ("flake8conf", flake8conf::detect),
    ("fldigiconf", fldigiconf::detect),
    ("fleet", fleet::detect),
    ("flif", flif::detect),
    ("flink", flink::detect),
    ("fluentbit", fluentbit::detect),
    ("fluentd", fluentd::detect),
    ("fluxcd", fluxcd::detect),
    ("flyio", flyio::detect),
    ("flyway", flyway::detect),
    ("footconf", footconf::detect),
    ("fossilconf", fossilconf::detect),
    ("fpml", fpml::detect),
    ("freetds", freetds::detect),
    ("frigate", frigate::detect),
    ("frr", frr::detect),
    ("func", func::detect),
    ("fusesoc", fusesoc::detect),
    ("fxml", fxml::detect),
    ("fxp", fxp::detect),
    ("garden", garden::detect),
    ("garnetconf", garnetconf::detect),
    ("gatekeeper", gatekeeper::detect),
    ("gatewayapi", gatewayapi::detect),
    ("gatsby", gatsby::detect),
    ("gbench", gbench::detect),
    ("gbstudio", gbstudio::detect),
    ("gdf", gdf::detect),
    ("gdmconf", gdmconf::detect),
    ("gedasch", gedasch::detect),
    ("gemfile", gemfile::detect),
    ("gemlock", gemlock::detect),
    ("gemrc", gemrc::detect),
    ("gemspec", gemspec::detect),
    ("gerbera", gerbera::detect),
    ("gethconf", gethconf::detect),
    ("getmailrc", getmailrc::detect),
    ("gexf", gexf::detect),
    ("gf", gf::detect),
    ("ghosttyconf", ghosttyconf::detect),
    ("gitattributes", gitattributes::detect),
    ("gitconfig", gitconfig::detect),
    ("giteaaction", giteaaction::detect),
    ("giteaapp", giteaapp::detect),
    ("gitignore", gitignore::detect),
    ("gitlabci", gitlabci::detect),
    ("gitlabrb", gitlabrb::detect),
    ("gitleaks", gitleaks::detect),
    ("gitmodules", gitmodules::detect),
    ("gitsecret", gitsecret::detect),
    ("gitwebconf", gitwebconf::detect),
    ("glade", glade::detect),
    ("glsl", glsl::detect),
    ("glusterfs", glusterfs::detect),
    ("gml", gml::detect),
    ("gn", gn::detect),
    ("gnuplot", gnuplot::detect),
    ("godot", godot::detect),
    ("gogsconf", gogsconf::detect),
    ("golangci", golangci::detect),
    ("gomod", gomod::detect),
    ("goreleaser", goreleaser::detect),
    ("gostconf", gostconf::detect),
    ("gosum", gosum::detect),
    ("gp", gp::detect),
    ("gpsd", gpsd::detect),
    ("gqrxconf", gqrxconf::detect),
    ("gradle", gradle::detect),
    ("gradlemod", gradlemod::detect),
    ("grafana", grafana::detect),
    ("grafanaop", grafanaop::detect),
    ("graphml", graphml::detect),
    ("graphql", graphql::detect),
    ("greatexp", greatexp::detect),
    ("greetd", greetd::detect),
    ("grok", grok::detect),
    ("grubcfg", grubcfg::detect),
    ("grubconf", grubconf::detect),
    ("grubenv", grubenv::detect),
    ("grype", grype::detect),
    ("gtksrclang", gtksrclang::detect),
    ("h2oconf", h2oconf::detect),
    ("hacf", hacf::detect),
    ("hadolintconf", hadolintconf::detect),
    ("hadoopconf", hadoopconf::detect),
    ("haproxy", haproxy::detect),
    ("har", har::detect),
    ("harakaconf", harakaconf::detect),
    ("harbor", harbor::detect),
    ("haresources", haresources::detect),
    ("harness", harness::detect),
    ("hb", hb::detect),
    ("headscaleconf", headscaleconf::detect),
    ("helix", helix::detect),
    ("helmfile", helmfile::detect),
    ("hexchat", hexchat::detect),
    ("hexo", hexo::detect),
    ("hfe", hfe::detect),
    ("hfs", hfs::detect),
    ("hgignore", hgignore::detect),
    ("hgrc", hgrc::detect),
    ("hiawatha", hiawatha::detect),
    ("himalayaconf", himalayaconf::detect),
    ("hivemq", hivemq::detect),
    ("hl7", hl7::detect),
    ("hlsl", hlsl::detect),
    ("hocr", hocr::detect),
    ("homeassistant", homeassistant::detect),
    ("hopconf", hopconf::detect),
    ("hoppscotch", hoppscotch::detect),
    ("hostapd", hostapd::detect),
    ("httpfile", httpfile::detect),
    ("hugoconf", hugoconf::detect),
    ("hydra", hydra::detect),
    ("hydraml", hydraml::detect),
    ("hydrogen", hydrogen::detect),
    ("hyperfine", hyperfine::detect),
    ("hyprland", hyprland::detect),
    ("hysteriaconf", hysteriaconf::detect),
    ("i3conf", i3conf::detect),
    ("ibmmq", ibmmq::detect),
    ("ical", ical::detect),
    ("icecast", icecast::detect),
    ("icinga", icinga::detect),
    ("ideavim", ideavim::detect),
    ("idl", idl::detect),
    ("imd", imd::detect),
    ("inffile", inffile::detect),
    ("infinispan", infinispan::detect),
    ("inittab", inittab::detect),
    ("inputrc", inputrc::detect),
    ("insomnia", insomnia::detect),
    ("instana", instana::detect),
    ("interfaces", interfaces::detect),
    ("ioc", ioc::detect),
    ("iosconf", iosconf::detect),
    ("ipac", ipac::detect),
    ("ipset", ipset::detect),
    ("iptablessave", iptablessave::detect),
    ("iptc", iptc::detect),
    ("ipxact", ipxact::detect),
    ("ipxescript", ipxescript::detect),
    ("ipythonconf", ipythonconf::detect),
    ("ircam", ircam::detect),
    ("irssi", irssi::detect),
    ("isabelle", isabelle::detect),
    ("isbn", isbn::detect),
    ("isc", isc::detect),
    ("iscsi", iscsi::detect),
    ("ismn", ismn::detect),
    ("isortconf", isortconf::detect),
    ("issn", issn::detect),
    ("istio", istio::detect),
    ("iterm", iterm::detect),
    ("itermdyn", itermdyn::detect),
    ("iv", iv::detect),
    ("ivf", ivf::detect),
    ("ivy", ivy::detect),
    ("iwdconf", iwdconf::detect),
    ("jackrc", jackrc::detect),
    ("jbig2", jbig2::detect),
    ("jed", jed::detect),
    ("jekyll", jekyll::detect),
    ("jellyfin", jellyfin::detect),
    ("jenkinsfile", jenkinsfile::detect),
    ("jenkinsx", jenkinsx::detect),
    ("jest", jest::detect),
    ("jfm", jfm::detect),
    ("jfr", jfr::detect),
    ("jfs", jfs::detect),
    ("jmh", jmh::detect),
    ("journaldconf", journaldconf::detect),
    ("jp2", jp2::detect),
    ("jq", jq::detect),
    ("jsonnet", jsonnet::detect),
    ("jsonpath", jsonpath::detect),
    ("jtl", jtl::detect),
    ("junos", junos::detect),
    ("jupyterconf", jupyterconf::detect),
    ("justfile", justfile::detect),
    ("jwe", jwe::detect),
    ("jwk", jwk::detect),
    ("jxr", jxr::detect),
    ("k0sconf", k0sconf::detect),
    ("k3d", k3d::detect),
    ("k3sconf", k3sconf::detect),
    ("k6", k6::detect),
    ("k8gb", k8gb::detect),
    ("kafka", kafka::detect),
    ("kamaji", kamaji::detect),
    ("kanata", kanata::detect),
    ("kapitan", kapitan::detect),
    ("karmaconf", karmaconf::detect),
    ("karpenter", karpenter::detect),
    ("katesyntax", katesyntax::detect),
    ("kbm", kbm::detect),
    ("kcl", kcl::detect),
    ("kconfig", kconfig::detect),
    ("kdeglobals", kdeglobals::detect),
    ("keda", keda::detect),
    ("kedro", kedro::detect),
    ("keepalived", keepalived::detect),
    ("keepassxc", keepassxc::detect),
    ("kern", kern::detect),
    ("ketl", ketl::detect),
    ("keto", keto::detect),
    ("keycloak", keycloak::detect),
    ("keyd", keyd::detect),
    ("keydbconf", keydbconf::detect),
    ("keytab", keytab::detect),
    ("kibana", kibana::detect),
    ("kicadpcb", kicadpcb::detect),
    ("kicadpro", kicadpro::detect),
    ("kicadsch", kicadsch::detect),
    ("kickstart", kickstart::detect),
    ("kif", kif::detect),
    ("kindconf", kindconf::detect),
    ("kittyconf", kittyconf::detect),
    ("kittyimg", kittyimg::detect),
    ("klipperconf", klipperconf::detect),
    ("knative", knative::detect),
    ("knexfile", knexfile::detect),
    ("kodiadv", kodiadv::detect),
    ("kong", kong::detect),
    ("kopia", kopia::detect),
    ("kql", kql::detect),
    ("kratos", kratos::detect),
    ("krb5conf", krb5conf::detect),
    ("kserve", kserve::detect),
    ("ktlint", ktlint::detect),
    ("kubeconfig", kubeconfig::detect),
    ("kubedb", kubedb::detect),
    ("kubeflow", kubeflow::detect),
    ("kubeflowtraining", kubeflowtraining::detect),
    ("kubemq", kubemq::detect),
    ("kubevela", kubevela::detect),
    ("kubevirt", kubevirt::detect),
    ("kuma", kuma::detect),
    ("kustomize", kustomize::detect),
    ("kyverno", kyverno::detect),
    ("lab", lab::detect),
    ("ldapconf", ldapconf::detect),
    ("ldif", ldif::detect),
    ("ldirectord", ldirectord::detect),
    ("ldtk", ldtk::detect),
    ("lean", lean::detect),
    ("leda", leda::detect),
    ("ledgerjournal", ledgerjournal::detect),
    ("lefthook", lefthook::detect),
    ("lego", lego::detect),
    ("leiningen", leiningen::detect),
    ("lerna", lerna::detect),
    ("lf", lf::detect),
    ("lidarr", lidarr::detect),
    ("lightdm", lightdm::detect),
    ("lighttpd", lighttpd::detect),
    ("lima", lima::detect),
    ("limine", limine::detect),
    ("linkerd", linkerd::detect),
    ("lintstaged", lintstaged::detect),
    ("liquibase", liquibase::detect),
    ("litmus", litmus::detect),
    ("lmms", lmms::detect),
    ("lndconf", lndconf::detect),
    ("locxml", locxml::detect),
    ("log4j", log4j::detect),
    ("log4perl", log4perl::detect),
    ("logback", logback::detect),
    ("logindefs", logindefs::detect),
    ("logrotate", logrotate::detect),
    ("logstash", logstash::detect),
    ("loki", loki::detect),
    ("longhorn", longhorn::detect),
    ("loveconf", loveconf::detect),
    ("lp", lp::detect),
    ("lpf", lpf::detect),
    ("lsf", lsf::detect),
    ("ltsconf", ltsconf::detect),
    ("lucene", lucene::detect),
    ("luhn", luhn::detect),
    ("luigi", luigi::detect),
    ("lvmconf", lvmconf::detect),
    ("lwo", lwo::detect),
    ("ly", ly::detect),
    ("lynisconf", lynisconf::detect),
    ("lzfse", lzfse::detect),
    ("lzip", lzip::detect),
    ("macaroon", macaroon::detect),
    ("maddyconf", maddyconf::detect),
    ("maf", maf::detect),
    ("magefile", magefile::detect),
    ("maildrop", maildrop::detect),
    ("mameconf", mameconf::detect),
    ("mapfile", mapfile::detect),
    ("mapnikxml", mapnikxml::detect),
    ("mapproxyconf", mapproxyconf::detect),
    ("marc", marc::detect),
    ("markdownlint", markdownlint::detect),
    ("marlinconf", marlinconf::detect),
    ("matplotlibrc", matplotlibrc::detect),
    ("matterbridge", matterbridge::detect),
    ("mattermost", mattermost::detect),
    ("maud", maud::detect),
    ("mbedapp", mbedapp::detect),
    ("mbsyncrc", mbsyncrc::detect),
    ("mch", mch::detect),
    ("mcserverprops", mcserverprops::detect),
    ("md3", md3::detect),
    ("mdx", mdx::detect),
    ("med", med::detect),
    ("mediamtx", mediamtx::detect),
    ("mediawiki", mediawiki::detect),
    ("mednafen", mednafen::detect),
    ("mei", mei::detect),
    ("meili", meili::detect),
    ("melonds", melonds::detect),
    ("meltano", meltano::detect),
    ("memcachedconf", memcachedconf::detect),
    ("mergify", mergify::detect),
    ("mermaid", mermaid::detect),
    ("meson", meson::detect),
    ("metaflow", metaflow::detect),
    ("metal3", metal3::detect),
    ("metallb", metallb::detect),
    ("metallib", metallib::detect),
    ("metricbeat", metricbeat::detect),
    ("metroconf", metroconf::detect),
    ("mets", mets::detect),
    ("milvusconf", milvusconf::detect),
    ("mimirconf", mimirconf::detect),
    ("minica", minica::detect),
    ("minidlna", minidlna::detect),
    ("minikubeconf", minikubeconf::detect),
    ("minio", minio::detect),
    ("mise", mise::detect),
    ("misp", misp::detect),
    ("mix", mix::detect),
    ("mixexs", mixexs::detect),
    ("mixxx", mixxx::detect),
    ("mkdocs", mkdocs::detect),
    ("mlflow", mlflow::detect),
    ("mml", mml::detect),
    ("mmlstyle", mmlstyle::detect),
    ("mochajson", mochajson::detect),
    ("mocharc", mocharc::detect),
    ("modeldo", modeldo::detect),
    ("modprobeconf", modprobeconf::detect),
    ("modsecurity", modsecurity::detect),
    ("monero", monero::detect),
    ("mongod", mongod::detect),
    ("monit", monit::detect),
    ("moonrakerconf", moonrakerconf::detect),
    ("moonrepo", moonrepo::detect),
    ("mopidy", mopidy::detect),
    ("mosquitto", mosquitto::detect),
    ("motionconf", motionconf::detect),
    ("movelang", movelang::detect),
    ("mpc", mpc::detect),
    ("mpd", mpd::detect),
    ("mpegts", mpegts::detect),
    ("mplayerconf", mplayerconf::detect),
    ("mps", mps::detect),
    ("mpv", mpv::detect),
    ("mpvconf", mpvconf::detect),
    ("mscx", mscx::detect),
    ("msmtprc", msmtprc::detect),
    ("mtm", mtm::detect),
    ("mtx", mtx::detect),
    ("multus", multus::detect),
    ("musicxml", musicxml::detect),
    ("muttrc", muttrc::detect),
    ("mvnsettings", mvnsettings::detect),
    ("mxf", mxf::detect),
    ("mypyconf", mypyconf::detect),
    ("mysql", mysql::detect),
    ("nagios", nagios::detect),
    ("namedconf", namedconf::detect),
    ("nanoid", nanoid::detect),
    ("nanorc", nanorc::detect),
    ("nats", nats::detect),
    ("navidrome", navidrome::detect),
    ("naxsiconf", naxsiconf::detect),
    ("nbd", nbd::detect),
    ("ncmpcpp", ncmpcpp::detect),
    ("ncpdp", ncpdp::detect),
    ("nebulaconf", nebulaconf::detect),
    ("nef", nef::detect),
    ("neo4jconf", neo4jconf::detect),
    ("neomuttconf", neomuttconf::detect),
    ("nerdctl", nerdctl::detect),
    ("netlifyconf", netlifyconf::detect),
    ("netplan", netplan::detect),
    ("networkd", networkd::detect),
    ("newrelic", newrelic::detect),
    ("newsboat", newsboat::detect),
    ("newsyslog", newsyslog::detect),
    ("nextflow", nextflow::detect),
    ("nexus", nexus::detect),
    ("nfpm", nfpm::detect),
    ("nfsexports", nfsexports::detect),
    ("nftconf", nftconf::detect),
    ("nginx", nginx::detect),
    ("ngircd", ngircd::detect),
    ("nib", nib::detect),
    ("nickel", nickel::detect),
    ("nififlow", nififlow::detect),
    ("nimble", nimble::detect),
    ("nist", nist::detect),
    ("nix", nix::detect),
    ("nixconf", nixconf::detect),
    ("nlogconf", nlogconf::detect),
    ("nmconnection", nmconnection::detect),
    ("nodered", nodered::detect),
    ("nomad", nomad::detect),
    ("npmlock", npmlock::detect),
    ("npmrc", npmrc::detect),
    ("nsd", nsd::detect),
    ("nsjailcfg", nsjailcfg::detect),
    ("nslcdconf", nslcdconf::detect),
    ("nsqconf", nsqconf::detect),
    ("nsswitch", nsswitch::detect),
    ("ntpconf", ntpconf::detect),
    ("ntpsec", ntpsec::detect),
    ("nuconf", nuconf::detect),
    ("nugetconfig", nugetconfig::detect),
    ("nullmailerconf", nullmailerconf::detect),
    ("nuxt", nuxt::detect),
    ("nwc", nwc::detect),
    ("nxconf", nxconf::detect),
    ("nzbget", nzbget::detect),
    ("oai", oai::detect),
    ("oathkeeper", oathkeeper::detect),
    ("obsconf", obsconf::detect),
    ("octaverc", octaverc::detect),
    ("octoprint", octoprint::detect),
    ("odbcini", odbcini::detect),
    ("oem", oem::detect),
    ("offlineimap", offlineimap::detect),
    ("ogmo", ogmo::detect),
    ("okteto", okteto::detect),
    ("olm", olm::detect),
    ("omm", omm::detect),
    ("opam", opam::detect),
    ("opb", opb::detect),
    ("openapi", openapi::detect),
    ("openbgpd", openbgpd::detect),
    ("opendkim", opendkim::detect),
    ("opendmarc", opendmarc::detect),
    ("openebs", openebs::detect),
    ("openfaas", openfaas::detect),
    ("openfga", openfga::detect),
    ("openhab", openhab::detect),
    ("openlane", openlane::detect),
    ("openmsx", openmsx::detect),
    ("openntpd", openntpd::detect),
    ("openpulse", openpulse::detect),
    ("openrc", openrc::detect),
    ("opensearch", opensearch::detect),
    ("opensearchop", opensearchop::detect),
    ("openssl", openssl::detect),
    ("openvpn", openvpn::detect),
    ("opsjson", opsjson::detect),
    ("orcaslicer", orcaslicer::detect),
    ("orcid", orcid::detect),
    ("orf", orf::detect),
    ("osc", osc::detect),
    ("osm2pgsqlstyle", osm2pgsqlstyle::detect),
    ("osqueryconf", osqueryconf::detect),
    ("ossecconf", ossecconf::detect),
    ("otelcol", otelcol::detect),
    ("overpass", overpass::detect),
    ("ovf", ovf::detect),
    ("pacemaker", pacemaker::detect),
    ("packer", packer::detect),
    ("packit", packit::detect),
    ("pacman", pacman::detect),
    ("paf", paf::detect),
    ("pain", pain::detect),
    ("pajek", pajek::detect),
    ("pamstack", pamstack::detect),
    ("pants", pants::detect),
    ("paraver", paraver::detect),
    ("parityconf", parityconf::detect),
    ("paseto", paseto::detect),
    ("pcsx2conf", pcsx2conf::detect),
    ("pdns", pdns::detect),
    ("percona", percona::detect),
    ("perflog", perflog::detect),
    ("pfconf", pfconf::detect),
    ("pgo", pgo::detect),
    ("pgpass", pgpass::detect),
    ("pgservice", pgservice::detect),
    ("phabricatorconf", phabricatorconf::detect),
    ("phylip", phylip::detect),
    ("picard", picard::detect),
    ("pidginconf", pidginconf::detect),
    ("pihole", pihole::detect),
    ("pileup", pileup::detect),
    ("pinerc", pinerc::detect),
    ("pinpoint", pinpoint::detect),
    ("pipewireconf", pipewireconf::detect),
    ("pipfile", pipfile::detect),
    ("pk", pk::detect),
    ("pkl", pkl::detect),
    ("pl", pl::detect),
    ("planetilerconf", planetilerconf::detect),
    ("plantuml", plantuml::detect),
    ("platformio", platformio::detect),
    ("platformsh", platformsh::detect),
    ("playwrightconf", playwrightconf::detect),
    ("plexconf", plexconf::detect),
    ("pm2", pm2::detect),
    ("pmacctconf", pmacctconf::detect),
    ("pmml", pmml::detect),
    ("pnpmlock", pnpmlock::detect),
    ("pnpmworkspace", pnpmworkspace::detect),
    ("podfile", podfile::detect),
    ("poetry", poetry::detect),
    ("policyjson", policyjson::detect),
    ("polybar", polybar::detect),
    ("pom", pom::detect),
    ("pomerium", pomerium::detect),
    ("portage", portage::detect),
    ("portworx", portworx::detect),
    ("postalconf", postalconf::detect),
    ("postcss", postcss::detect),
    ("postfix", postfix::detect),
    ("postgresql", postgresql::detect),
    ("postsrsdconf", postsrsdconf::detect),
    ("pppdconf", pppdconf::detect),
    ("pprof", pprof::detect),
    ("ppssppconf", ppssppconf::detect),
    ("pptpd", pptpd::detect),
    ("precommit", precommit::detect),
    ("prefect", prefect::detect),
    ("premakeconf", premakeconf::detect),
    ("preseed", preseed::detect),
    ("prettier", prettier::detect),
    ("privoxy", privoxy::detect),
    ("procd", procd::detect),
    ("procmailrc", procmailrc::detect),
    ("proftpd", proftpd::detect),
    ("projjson", projjson::detect),
    ("promela", promela::detect),
    ("prometheus", prometheus::detect),
    ("promoperator", promoperator::detect),
    ("promtailconf", promtailconf::detect),
    ("proselint", proselint::detect),
    ("prosody", prosody::detect),
    ("prow", prow::detect),
    ("prowlarr", prowlarr::detect),
    ("prusaslicer", prusaslicer::detect),
    ("ptm", ptm::detect),
    ("ptp4l", ptp4l::detect),
    ("ptx", ptx::detect),
    ("pubspec", pubspec::detect),
    ("pulsar", pulsar::detect),
    ("pulseclientconf", pulseclientconf::detect),
    ("pulumi", pulumi::detect),
    ("puppet", puppet::detect),
    ("pureftpd", pureftpd::detect),
    ("puz", puz::detect),
    ("pxelinux", pxelinux::detect),
    ("pylintrc", pylintrc::detect),
    ("pypirc", pypirc::detect),
    ("pyproject", pyproject::detect),
    ("pyrightconf", pyrightconf::detect),
    ("pyroconf", pyroconf::detect),
    ("pytestbench", pytestbench::detect),
    ("pzserver", pzserver::detect),
    ("qasm", qasm::detect),
    ("qbittorrent", qbittorrent::detect),
    ("qcp", qcp::detect),
    ("qdrantconf", qdrantconf::detect),
    ("qgsproj", qgsproj::detect),
    ("qmakepro", qmakepro::detect),
    ("qmap", qmap::detect),
    ("qobj", qobj::detect),
    ("qpf", qpf::detect),
    ("qs", qs::detect),
    ("qsf", qsf::detect),
    ("qt5ctconf", qt5ctconf::detect),
    ("qtui", qtui::detect),
    ("quartz", quartz::detect),
    ("quil", quil::detect),
    ("rabbitmq", rabbitmq::detect),
    ("radarr", radarr::detect),
    ("radiusd", radiusd::detect),
    ("raf", raf::detect),
    ("railwayconf", railwayconf::detect),
    ("rakefile", rakefile::detect),
    ("raml", raml::detect),
    ("razorconf", razorconf::detect),
    ("rc", rc::detect),
    ("rcloneconf", rcloneconf::detect),
    ("rdpfile", rdpfile::detect),
    ("readarr", readarr::detect),
    ("reaper", reaper::detect),
    ("rebarconfig", rebarconfig::detect),
    ("recordio", recordio::detect),
    ("redisconf", redisconf::detect),
    ("redpanda", redpanda::detect),
    ("redpen", redpen::detect),
    ("refind", refind::detect),
    ("regfile", regfile::detect),
    ("registriesconf", registriesconf::detect),
    ("rego", rego::detect),
    ("reiserfs", reiserfs::detect),
    ("relaxng", relaxng::detect),
    ("releaseplease", releaseplease::detect),
    ("releaserc", releaserc::detect),
    ("remminaconf", remminaconf::detect),
    ("renderconf", renderconf::detect),
    ("renovate", renovate::detect),
    ("renviron", renviron::detect),
    ("requirements", requirements::detect),
    ("res", res::detect),
    ("resolv", resolv::detect),
    ("resticprofile", resticprofile::detect),
    ("retroarch", retroarch::detect),
    ("reviveconf", reviveconf::detect),
    ("rfb", rfb::detect),
    ("rinex", rinex::detect),
    ("rkhunter", rkhunter::detect),
    ("rm", rm::detect),
    ("rocketmq", rocketmq::detect),
    ("rockspec", rockspec::detect),
    ("rofi", rofi::detect),
    ("rollup", rollup::detect),
    ("rook", rook::detect),
    ("routeros", routeros::detect),
    ("rpcs3conf", rpcs3conf::detect),
    ("rpgmakerconf", rpgmakerconf::detect),
    ("rprofile", rprofile::detect),
    ("rpy", rpy::detect),
    ("rsnapshot", rsnapshot::detect),
    ("rspamdconf", rspamdconf::detect),
    ("rss2email", rss2email::detect),
    ("rsyslogd", rsyslogd::detect),
    ("rtorrent", rtorrent::detect),
    ("rubocop", rubocop::detect),
    ("ruffconf", ruffconf::detect),
    ("rundeck", rundeck::detect),
    ("runit", runit::detect),
    ("rvdata", rvdata::detect),
    ("rw2", rw2::detect),
    ("rx2", rx2::detect),
    ("s6rc", s6rc::detect),
    ("sabnzbd", sabnzbd::detect),
    ("saif", saif::detect),
    ("salt", salt::detect),
    ("samba", samba::detect),
    ("samhainconf", samhainconf::detect),
    ("saml", saml::detect),
    ("sbf", sbf::detect),
    ("sbt", sbt::detect),
    ("sby", sby::detect),
    ("scandata", scandata::detect),
    ("sch", sch::detect),
    ("scl", scl::detect),
    ("sconstruct", sconstruct::detect),
    ("scp", scp::detect),
    ("scummvm", scummvm::detect),
    ("scyllaop", scyllaop::detect),
    ("sdc", sdc::detect),
    ("sddmconf", sddmconf::detect),
    ("sdkconfig", sdkconfig::detect),
    ("sdrppconf", sdrppconf::detect),
    ("sealedsecrets", sealedsecrets::detect),
    ("seccomp", seccomp::detect),
    ("secretsbaseline", secretsbaseline::detect),
    ("secretsstore", secretsstore::detect),
    ("seldon", seldon::detect),
    ("selinuxfc", selinuxfc::detect),
    ("selinuxte", selinuxte::detect),
    ("semgrep", semgrep::detect),
    ("sendmail", sendmail::detect),
    ("sentinel", sentinel::detect),
    ("sequelizerc", sequelizerc::detect),
    ("serilog", serilog::detect),
    ("serverless", serverless::detect),
    ("serverprop", serverprop::detect),
    ("setupcfg", setupcfg::detect),
    ("sevendtdxml", sevendtdxml::detect),
    ("sftp", sftp::detect),
    ("sgi", sgi::detect),
    ("shadowsocksconf", shadowsocksconf::detect),
    ("shard", shard::detect),
    ("shellcheckrc", shellcheckrc::detect),
    ("shibconf", shibconf::detect),
    ("shipwright", shipwright::detect),
    ("shorewall", shorewall::detect),
    ("sidekiq", sidekiq::detect),
    ("sievescript", sievescript::detect),
    ("sigma", sigma::detect),
    ("singboxconf", singboxconf::detect),
    ("singerconf", singerconf::detect),
    ("sixel", sixel::detect),
    ("skaffold", skaffold::detect),
    ("skywalking", skywalking::detect),
    ("slapd", slapd::detect),
    ("slob", slob::detect),
    ("slrnconf", slrnconf::detect),
    ("slurm", slurm::detect),
    ("smd", smd::detect),
    ("smithy", smithy::detect),
    ("smt2", smt2::detect),
    ("smtpdconf", smtpdconf::detect),
    ("snapcast", snapcast::detect),
    ("snapcraft", snapcraft::detect),
    ("snmpdconf", snmpdconf::detect),
    ("snort", snort::detect),
    ("snyk", snyk::detect),
    ("sol", sol::detect),
    ("solrconfig", solrconfig::detect),
    ("solrschema", solrschema::detect),
    ("sonar", sonar::detect),
    ("sonarr", sonarr::detect),
    ("soniccfg", soniccfg::detect),
    ("sops", sops::detect),
    ("sp3", sp3::detect),
    ("spamassassin", spamassassin::detect),
    ("spark", spark::detect),
    ("sparql", sparql::detect),
    ("speedscope", speedscope::detect),
    ("spef", spef::detect),
    ("sphinx", sphinx::detect),
    ("spicenet", spicenet::detect),
    ("spigot", spigot::detect),
    ("spire", spire::detect),
    ("spotbugs", spotbugs::detect),
    ("spv", spv::detect),
    ("sqitchconf", sqitchconf::detect),
    ("sqlnet", sqlnet::detect),
    ("squid", squid::detect),
    ("srcdscfg", srcdscfg::detect),
    ("srhtconf", srhtconf::detect),
    ("sshconf", sshconf::detect),
    ("ssmtpconf", ssmtpconf::detect),
    ("sssdconf", sssdconf::detect),
    ("stack", stack::detect),
    ("stalwartconf", stalwartconf::detect),
    ("stardict", stardict::detect),
    ("starship", starship::detect),
    ("stash", stash::detect),
    ("staticcheckconf", staticcheckconf::detect),
    ("stepca", stepca::detect),
    ("stix", stix::detect),
    ("stm", stm::detect),
    ("storageconf", storageconf::detect),
    ("storybook", storybook::detect),
    ("strimzi", strimzi::detect),
    ("strongswanconf", strongswanconf::detect),
    ("stylelint", stylelint::detect),
    ("sublime", sublime::detect),
    ("sudoers", sudoers::detect),
    ("sudoku", sudoku::detect),
    ("suiconf", suiconf::detect),
    ("supabase", supabase::detect),
    ("supervisor", supervisor::detect),
    ("surefire", surefire::detect),
    ("suricata", suricata::detect),
    ("svelte", svelte::detect),
    ("svf", svf::detect),
    ("svnconf", svnconf::detect),
    ("svp", svp::detect),
    ("swanctl", swanctl::detect),
    ("sway", sway::detect),
    ("swf", swf::detect),
    ("swiftlint", swiftlint::detect),
    ("swiftmt", swiftmt::detect),
    ("synapse", synapse::detect),
    ("syncthingconf", syncthingconf::detect),
    ("sysctlconf", sysctlconf::detect),
    ("syslogng", syslogng::detect),
    ("sysmonconf", sysmonconf::detect),
    ("systemdboot", systemdboot::detect),
    ("sysv", sysv::detect),
    ("syx", syx::detect),
    ("t3d", t3d::detect),
    ("tabbyconf", tabbyconf::detect),
    ("tact", tact::detect),
    ("tailscale", tailscale::detect),
    ("tailwind", tailwind::detect),
    ("talisman", talisman::detect),
    ("taprc", taprc::detect),
    ("tarantool", tarantool::detect),
    ("taskfile", taskfile::detect),
    ("td0", td0::detect),
    ("tdm", tdm::detect),
    ("tekton", tekton::detect),
    ("telegraf", telegraf::detect),
    ("teleport", teleport::detect),
    ("telnet", telnet::detect),
    ("tempoconf", tempoconf::detect),
    ("terminfo", terminfo::detect),
    ("terragrunt", terragrunt::detect),
    ("terrariaconf", terrariaconf::detect),
    ("textile", textile::detect),
    ("textlint", textlint::detect),
    ("textmategram", textmategram::detect),
    ("tflint", tflint::detect),
    ("tfm", tfm::detect),
    ("tfrecord", tfrecord::detect),
    ("tgf", tgf::detect),
    ("thanosconf", thanosconf::detect),
    ("tidb", tidb::detect),
    ("tileservergl", tileservergl::detect),
    ("tilestacheconf", tilestacheconf::detect),
    ("tiltfile", tiltfile::detect),
    ("timesyncd", timesyncd::detect),
    ("tincconf", tincconf::detect),
    ("tinyproxy", tinyproxy::detect),
    ("tlaplus", tlaplus::detect),
    ("tlp", tlp::detect),
    ("tmpfilesd", tmpfilesd::detect),
    ("tmuxconf", tmuxconf::detect),
    ("tmx", tmx::detect),
    ("tnsnames", tnsnames::detect),
    ("tomcat", tomcat::detect),
    ("torrc", torrc::detect),
    ("toxini", toxini::detect),
    ("tptp", tptp::detect),
    ("traefik", traefik::detect),
    ("transmission", transmission::detect),
    ("travisci", travisci::detect),
    ("treesittergram", treesittergram::detect),
    ("tripwireconf", tripwireconf::detect),
    ("trivy", trivy::detect),
    ("trivyop", trivyop::detect),
    ("trojanconf", trojanconf::detect),
    ("ts3serverini", ts3serverini::detect),
    ("tscn", tscn::detect),
    ("tsconfig", tsconfig::detect),
    ("tsx", tsx::detect),
    ("ttyrec", ttyrec::detect),
    ("tuicconf", tuicconf::detect),
    ("turboconf", turboconf::detect),
    ("twee", twee::detect),
    ("txt2tags", txt2tags::detect),
    ("typeid", typeid::detect),
    ("typeormconf", typeormconf::detect),
    ("typesense", typesense::detect),
    ("ubootenv", ubootenv::detect),
    ("ucf", ucf::detect),
    ("uci", uci::detect),
    ("udevrules", udevrules::detect),
    ("ufwrules", ufwrules::detect),
    ("ult", ult::detect),
    ("unattend", unattend::detect),
    ("unbound", unbound::detect),
    ("unison", unison::detect),
    ("unitconf", unitconf::detect),
    ("unitymanifest", unitymanifest::detect),
    ("unitysettings", unitysettings::detect),
    ("unocss", unocss::detect),
    ("unrealircd", unrealircd::detect),
    ("upc", upc::detect),
    ("upf", upf::detect),
    ("uplugin", uplugin::detect),
    ("uproject", uproject::detect),
    ("upstart", upstart::detect),
    ("usercss", usercss::detect),
    ("userscript", userscript::detect),
    ("usi", usi::detect),
    ("ust", ust::detect),
    ("ustx", ustx::detect),
    ("v2rayconf", v2rayconf::detect),
    ("vagrant", vagrant::detect),
    ("vale", vale::detect),
    ("valkeyconf", valkeyconf::detect),
    ("vaultagent", vaultagent::detect),
    ("vcard", vcard::detect),
    ("vcd", vcd::detect),
    ("vcl", vcl::detect),
    ("vcluster", vcluster::detect),
    ("vcpkg", vcpkg::detect),
    ("vector", vector::detect),
    ("velero", velero::detect),
    ("vercelconf", vercelconf::detect),
    ("verilog", verilog::detect),
    ("vernemq", vernemq::detect),
    ("vespaconf", vespaconf::detect),
    ("vf", vf::detect),
    ("vhdr", vhdr::detect),
    ("victoria", victoria::detect),
    ("vimrc", vimrc::detect),
    ("vimsyntax", vimsyntax::detect),
    ("virtxml", virtxml::detect),
    ("viteconf", viteconf::detect),
    ("vitepress", vitepress::detect),
    ("vitess", vitess::detect),
    ("vitestconf", vitestconf::detect),
    ("vlcrc", vlcrc::detect),
    ("vlt", vlt::detect),
    ("vmagentconf", vmagentconf::detect),
    ("vmrk", vmrk::detect),
    ("volcano", volcano::detect),
    ("votable", votable::detect),
    ("vpr", vpr::detect),
    ("vrtgdal", vrtgdal::detect),
    ("vscodeconf", vscodeconf::detect),
    ("vsftpd", vsftpd::detect),
    ("vsqx", vsqx::detect),
    ("vyper", vyper::detect),
    ("w64", w64::detect),
    ("wafconf", wafconf::detect),
    ("wandb", wandb::detect),
    ("waybar", waybar::detect),
    ("waypoint", waypoint::detect),
    ("weaviateconf", weaviateconf::detect),
    ("webmanifest", webmanifest::detect),
    ("webpackconf", webpackconf::detect),
    ("weechat", weechat::detect),
    ("werf", werf::detect),
    ("westconf", westconf::detect),
    ("westonconf", westonconf::detect),
    ("weztermconf", weztermconf::detect),
    ("wfdb", wfdb::detect),
    ("wgsl", wgsl::detect),
    ("whitelist", whitelist::detect),
    ("whyml", whyml::detect),
    ("widgetxml", widgetxml::detect),
    ("wim", wim::detect),
    ("windowsterminal", windowsterminal::detect),
    ("winini", winini::detect),
    ("winlogbeat", winlogbeat::detect),
    ("winstonconf", winstonconf::detect),
    ("wireguard", wireguard::detect),
    ("wireplumberconf", wireplumberconf::detect),
    ("wiresharkpref", wiresharkpref::detect),
    ("wktproj", wktproj::detect),
    ("woodpecker", woodpecker::detect),
    ("wordfileuew", wordfileuew::detect),
    ("woz", woz::detect),
    ("wpasupplicant", wpasupplicant::detect),
    ("wrangler", wrangler::detect),
    ("wrl", wrl::detect),
    ("wsdl", wsdl::detect),
    ("wsjtxconf", wsjtxconf::detect),
    ("wslconf", wslconf::detect),
    ("xacml", xacml::detect),
    ("xbrl", xbrl::detect),
    ("xdc", xdc::detect),
    ("xdf", xdf::detect),
    ("xib", xib::detect),
    ("xid", xid::detect),
    ("xinetdconf", xinetdconf::detect),
    ("xl2tpd", xl2tpd::detect),
    ("xlink", xlink::detect),
    ("xmakeconf", xmakeconf::detect),
    ("xmodmap", xmodmap::detect),
    ("xmp", xmp::detect),
    ("xorgconf", xorgconf::detect),
    ("xpath", xpath::detect),
    ("xq", xq::detect),
    ("xqf", xqf::detect),
    ("xrayconf", xrayconf::detect),
    ("xrdpconf", xrdpconf::detect),
    ("xresources", xresources::detect),
    ("xsd", xsd::detect),
    ("xslt", xslt::detect),
    ("xsvf", xsvf::detect),
    ("xunit", xunit::detect),
    ("y4m", y4m::detect),
    ("yamllint", yamllint::detect),
    ("yara", yara::detect),
    ("yarnlock", yarnlock::detect),
    ("yarnrc", yarnrc::detect),
    ("yggdrasil", yggdrasil::detect),
    ("yosys", yosys::detect),
    ("ytt", ytt::detect),
    ("yugabyte", yugabyte::detect),
    ("yuzuconf", yuzuconf::detect),
    ("zabbix", zabbix::detect),
    ("zapconf", zapconf::detect),
    ("zathurarc", zathurarc::detect),
    ("zedconf", zedconf::detect),
    ("zeekconf", zeekconf::detect),
    ("zeekctl", zeekctl::detect),
    ("zeekscript", zeekscript::detect),
    ("zerotier", zerotier::detect),
    ("zfs", zfs::detect),
    ("zigbee2mqtt", zigbee2mqtt::detect),
    ("zitadel", zitadel::detect),
    ("znc", znc::detect),
    ("zola", zola::detect),
    ("zon", zon::detect),
    ("zone", zone::detect),
    ("zonemtaconf", zonemtaconf::detect),
    ("zoo", zoo::detect),
    ("zookeeper", zookeeper::detect),
    ("zookeeperop", zookeeperop::detect),
    ("zpaq", zpaq::detect),
    ("zshrc", zshrc::detect),
    ("zulipconf", zulipconf::detect),
    ("zypper", zypper::detect),
];

/// 全検出器に入力を流し、合致したモジュール名を全て返す。
///
/// [`DETECTORS`] の並び順(名前昇順)で返る。合致なしなら空 `Vec`。
/// 1入力に複数形式が合致し得るため先勝ちではなく全件を返す。
#[must_use]
pub fn detect_all(input: &[u8]) -> Vec<&'static str> {
    DETECTORS
        .iter()
        .filter(|(_, f)| f(input))
        .map(|(name, _)| *name)
        .collect()
}
/// バイト列パーサーの関数型。戻り型がモジュールごとに異なるため
/// (`Option<T>`/`Vec<T>`/…)呼び捨ての `fn(&[u8])` に正規化する。
pub type ParserFn = fn(&[u8]);

/// フリー関数 `pub fn parse(x: &[u8])` を持つ全モジュールの一覧。
///
/// 各エントリは `let _ = <mod>::parse(b);` のラッパーで、
/// 戻り型の差を吸収して一括適用・性質テストを可能にする。
/// 該当モジュール追加時はこの表にも登録すること。
pub const PARSERS: &[(&str, ParserFn)] = &[
    ("a2r", |b| {
        let _ = a2r::parse(b);
    }),
    ("abc", |b| {
        let _ = abc::parse(b);
    }),
    ("ac", |b| {
        let _ = ac::parse(b);
    }),
    ("ace", |b| {
        let _ = ace::parse(b);
    }),
    ("ach", |b| {
        let _ = ach::parse(b);
    }),
    ("actionlint", |b| {
        let _ = actionlint::parse(b);
    }),
    ("activemq", |b| {
        let _ = activemq::parse(b);
    }),
    ("adoc", |b| {
        let _ = adoc::parse(b);
    }),
    ("aercconf", |b| {
        let _ = aercconf::parse(b);
    }),
    ("aerospike", |b| {
        let _ = aerospike::parse(b);
    }),
    ("afm", |b| {
        let _ = afm::parse(b);
    }),
    ("afp", |b| {
        let _ = afp::parse(b);
    }),
    ("aideconf", |b| {
        let _ = aideconf::parse(b);
    }),
    ("aiff", |b| {
        let _ = aiff::parse(b);
    }),
    ("aiger", |b| {
        let _ = aiger::parse(b);
    }),
    ("airbyteconf", |b| {
        let _ = airbyteconf::parse(b);
    }),
    ("ais", |b| {
        let _ = ais::parse(b);
    }),
    ("alembic", |b| {
        let _ = alembic::parse(b);
    }),
    ("alexrc", |b| {
        let _ = alexrc::parse(b);
    }),
    ("aln", |b| {
        let _ = aln::parse(b);
    }),
    ("alto", |b| {
        let _ = alto::parse(b);
    }),
    ("alz", |b| {
        let _ = alz::parse(b);
    }),
    ("amandaconf", |b| {
        let _ = amandaconf::parse(b);
    }),
    ("amf", |b| {
        let _ = amf::parse(b);
    }),
    ("ampl", |b| {
        let _ = ampl::parse(b);
    }),
    ("amplifyconf", |b| {
        let _ = amplifyconf::parse(b);
    }),
    ("amqp", |b| {
        let _ = amqp::parse(b);
    }),
    ("amr", |b| {
        let _ = amr::parse(b);
    }),
    ("analyze", |b| {
        let _ = analyze::parse(b);
    }),
    ("ansi", |b| {
        let _ = ansi::parse(b);
    }),
    ("antex", |b| {
        let _ = antex::parse(b);
    }),
    ("aoe", |b| {
        let _ = aoe::parse(b);
    }),
    ("aout", |b| {
        let _ = aout::parse(b);
    }),
    ("ape", |b| {
        let _ = ape::parse(b);
    }),
    ("apkg", |b| {
        let _ = apkg::parse(b);
    }),
    ("appcache", |b| {
        let _ = appcache::parse(b);
    }),
    ("appimage", |b| {
        let _ = appimage::parse(b);
    }),
    ("appveyor", |b| {
        let _ = appveyor::parse(b);
    }),
    ("aps", |b| {
        let _ = aps::parse(b);
    }),
    ("aprxconf", |b| {
        let _ = aprxconf::parse(b);
    }),
    ("ar", |b| {
        let _ = ar::parse(b);
    }),
    ("arb", |b| {
        let _ = arb::parse(b);
    }),
    ("archinstall", |b| {
        let _ = archinstall::parse(b);
    }),
    ("ardour", |b| {
        let _ = ardour::parse(b);
    }),
    ("arduinoconf", |b| {
        let _ = arduinoconf::parse(b);
    }),
    ("arff", |b| {
        let _ = arff::parse(b);
    }),
    ("argusconf", |b| {
        let _ = argusconf::parse(b);
    }),
    ("argorollout", |b| {
        let _ = argorollout::parse(b);
    }),
    ("arkimeconf", |b| {
        let _ = arkimeconf::parse(b);
    }),
    ("arma3conf", |b| {
        let _ = arma3conf::parse(b);
    }),
    ("arp", |b| {
        let _ = arp::parse(b);
    }),
    ("arrow", |b| {
        let _ = arrow::parse(b);
    }),
    ("arw", |b| {
        let _ = arw::parse(b);
    }),
    ("asciicast", |b| {
        let _ = asciicast::parse(b);
    }),
    ("asf", |b| {
        let _ = asf::parse(b);
    }),
    ("asn1", |b| {
        let _ = asn1::parse(b);
    }),
    ("assetlinks", |b| {
        let _ = assetlinks::parse(b);
    }),
    ("atom", |b| {
        let _ = atom::parse(b);
    }),
    ("atr", |b| {
        let _ = atr::parse(b);
    }),
    ("au", |b| {
        let _ = au::parse(b);
    }),
    ("audacity", |b| {
        let _ = audacity::parse(b);
    }),
    ("auditdconf", |b| {
        let _ = auditdconf::parse(b);
    }),
    ("auditrule", |b| {
        let _ = auditrule::parse(b);
    }),
    ("autofs", |b| {
        let _ = autofs::parse(b);
    }),
    ("autoyast", |b| {
        let _ = autoyast::parse(b);
    }),
    ("avi", |b| {
        let _ = avi::parse(b);
    }),
    ("avro", |b| {
        let _ = avro::parse(b);
    }),
    ("axports", |b| {
        let _ = axports::parse(b);
    }),
    ("ay", |b| {
        let _ = ay::parse(b);
    }),
    ("azw", |b| {
        let _ = azw::parse(b);
    }),
    ("bacnet", |b| {
        let _ = bacnet::parse(b);
    }),
    ("backstage", |b| {
        let _ = backstage::parse(b);
    }),
    ("baculadir", |b| {
        let _ = baculadir::parse(b);
    }),
    ("bai2", |b| {
        let _ = bai2::parse(b);
    }),
    ("bam", |b| {
        let _ = bam::parse(b);
    }),
    ("base32", |b| {
        let _ = base32::parse(b);
    }),
    ("bazel", |b| {
        let _ = bazel::parse(b);
    }),
    ("bbcode", |b| {
        let _ = bbcode::parse(b);
    }),
    ("bdb", |b| {
        let _ = bdb::parse(b);
    }),
    ("bdf", |b| {
        let _ = bdf::parse(b);
    }),
    ("bgp", |b| {
        let _ = bgp::parse(b);
    }),
    ("bibtex", |b| {
        let _ = bibtex::parse(b);
    }),
    ("bik", |b| {
        let _ = bik::parse(b);
    }),
    ("bit", |b| {
        let _ = bit::parse(b);
    }),
    ("bitcoinconf", |b| {
        let _ = bitcoinconf::parse(b);
    }),
    ("blend", |b| {
        let _ = blend::parse(b);
    }),
    ("bogofilter", |b| {
        let _ = bogofilter::parse(b);
    }),
    ("bootimg", |b| {
        let _ = bootimg::parse(b);
    }),
    ("bps", |b| {
        let _ = bps::parse(b);
    }),
    ("browserconfig", |b| {
        let _ = browserconfig::parse(b);
    }),
    ("bsdiff", |b| {
        let _ = bsdiff::parse(b);
    }),
    ("bsnes", |b| {
        let _ = bsnes::parse(b);
    }),
    ("bson", |b| {
        let _ = bson::parse(b);
    }),
    ("bsp", |b| {
        let _ = bsp::parse(b);
    }),
    ("btrfs", |b| {
        let _ = btrfs::parse(b);
    }),
    ("btsnoop", |b| {
        let _ = btsnoop::parse(b);
    }),
    ("buck", |b| {
        let _ = buck::parse(b);
    }),
    ("bufr", |b| {
        let _ = bufr::parse(b);
    }),
    ("buildkitd", |b| {
        let _ = buildkitd::parse(b);
    }),
    ("buildkite", |b| {
        let _ = buildkite::parse(b);
    }),
    ("bundle", |b| {
        let _ = bundle::parse(b);
    }),
    ("bundlerconf", |b| {
        let _ = bundlerconf::parse(b);
    }),
    ("bvh", |b| {
        let _ = bvh::parse(b);
    }),
    ("bzip2", |b| {
        let _ = bzip2::parse(b);
    }),
    ("c3d", |b| {
        let _ = c3d::parse(b);
    }),
    ("cab", |b| {
        let _ = cab::parse(b);
    }),
    ("cabal", |b| {
        let _ = cabal::parse(b);
    }),
    ("caf", |b| {
        let _ = caf::parse(b);
    }),
    ("calamares", |b| {
        let _ = calamares::parse(b);
    }),
    ("camt", |b| {
        let _ = camt::parse(b);
    }),
    ("candump", |b| {
        let _ = candump::parse(b);
    }),
    ("capnp", |b| {
        let _ = capnp::parse(b);
    }),
    ("capx", |b| {
        let _ = capx::parse(b);
    }),
    ("cardanoconf", |b| {
        let _ = cardanoconf::parse(b);
    }),
    ("cartocss", |b| {
        let _ = cartocss::parse(b);
    }),
    ("cargoconf", |b| {
        let _ = cargoconf::parse(b);
    }),
    ("carla", |b| {
        let _ = carla::parse(b);
    }),
    ("cbfs", |b| {
        let _ = cbfs::parse(b);
    }),
    ("ccd", |b| {
        let _ = ccd::parse(b);
    }),
    ("ccsds", |b| {
        let _ = ccsds::parse(b);
    }),
    ("ccx", |b| {
        let _ = ccx::parse(b);
    }),
    ("cephconf", |b| {
        let _ = cephconf::parse(b);
    }),
    ("chirpcsv", |b| {
        let _ = chirpcsv::parse(b);
    }),
    ("changesets", |b| {
        let _ = changesets::parse(b);
    }),
    ("chaosmesh", |b| {
        let _ = chaosmesh::parse(b);
    }),
    ("chd", |b| {
        let _ = chd::parse(b);
    }),
    ("checkstyle", |b| {
        let _ = checkstyle::parse(b);
    }),
    ("chronyconf", |b| {
        let _ = chronyconf::parse(b);
    }),
    ("cibxml", |b| {
        let _ = cibxml::parse(b);
    }),
    ("cif", |b| {
        let _ = cif::parse(b);
    }),
    ("cliff", |b| {
        let _ = cliff::parse(b);
    }),
    ("cirrus", |b| {
        let _ = cirrus::parse(b);
    }),
    ("citraconf", |b| {
        let _ = citraconf::parse(b);
    }),
    ("classfile", |b| {
        let _ = classfile::parse(b);
    }),
    ("clusterconf", |b| {
        let _ = clusterconf::parse(b);
    }),
    ("cmake", |b| {
        let _ = cmake::parse(b);
    }),
    ("cml", |b| {
        let _ = cml::parse(b);
    }),
    ("coap", |b| {
        let _ = coap::parse(b);
    }),
    ("cob", |b| {
        let _ = cob::parse(b);
    }),
    ("cobertura", |b| {
        let _ = cobertura::parse(b);
    }),
    ("cocosproj", |b| {
        let _ = cocosproj::parse(b);
    }),
    ("codeclimate", |b| {
        let _ = codeclimate::parse(b);
    }),
    ("codecov", |b| {
        let _ = codecov::parse(b);
    }),
    ("codespell", |b| {
        let _ = codespell::parse(b);
    }),
    ("coff", |b| {
        let _ = coff::parse(b);
    }),
    ("coins", |b| {
        let _ = coins::parse(b);
    }),
    ("commitlint", |b| {
        let _ = commitlint::parse(b);
    }),
    ("colima", |b| {
        let _ = colima::parse(b);
    }),
    ("contourconf", |b| {
        let _ = contourconf::parse(b);
    }),
    ("cookiejar", |b| {
        let _ = cookiejar::parse(b);
    }),
    ("coq", |b| {
        let _ = coq::parse(b);
    }),
    ("corosync", |b| {
        let _ = corosync::parse(b);
    }),
    ("coveralls", |b| {
        let _ = coveralls::parse(b);
    }),
    ("cpanfile", |b| {
        let _ = cpanfile::parse(b);
    }),
    ("cpio", |b| {
        let _ = cpio::parse(b);
    }),
    ("cr2", |b| {
        let _ = cr2::parse(b);
    }),
    ("cramfs", |b| {
        let _ = cramfs::parse(b);
    }),
    ("crashdump", |b| {
        let _ = crashdump::parse(b);
    }),
    ("creole", |b| {
        let _ = creole::parse(b);
    }),
    ("crio", |b| {
        let _ = crio::parse(b);
    }),
    ("crl", |b| {
        let _ = crl::parse(b);
    }),
    ("crowdsec", |b| {
        let _ = crowdsec::parse(b);
    }),
    ("crmconf", |b| {
        let _ = crmconf::parse(b);
    }),
    ("crockford", |b| {
        let _ = crockford::parse(b);
    }),
    ("cromwell", |b| {
        let _ = cromwell::parse(b);
    }),
    ("crontab", |b| {
        let _ = crontab::parse(b);
    }),
    ("crossplane", |b| {
        let _ = crossplane::parse(b);
    }),
    ("csa", |b| {
        let _ = csa::parse(b);
    }),
    ("csaf", |b| {
        let _ = csaf::parse(b);
    }),
    ("csd", |b| {
        let _ = csd::parse(b);
    }),
    ("csljson", |b| {
        let _ = csljson::parse(b);
    }),
    ("cspell", |b| {
        let _ = cspell::parse(b);
    }),
    ("csr", |b| {
        let _ = csr::parse(b);
    }),
    ("csv", |b| {
        let _ = csv::parse(b);
    }),
    ("cube", |b| {
        let _ = cube::parse(b);
    }),
    ("cuid", |b| {
        let _ = cuid::parse(b);
    }),
    ("cupsconf", |b| {
        let _ = cupsconf::parse(b);
    }),
    ("curaconf", |b| {
        let _ = curaconf::parse(b);
    }),
    ("cve", |b| {
        let _ = cve::parse(b);
    }),
    ("cvsrcs", |b| {
        let _ = cvsrcs::parse(b);
    }),
    ("cyclonedx", |b| {
        let _ = cyclonedx::parse(b);
    }),
    ("cypher", |b| {
        let _ = cypher::parse(b);
    }),
    ("d64", |b| {
        let _ = d64::parse(b);
    }),
    ("d88", |b| {
        let _ = d88::parse(b);
    }),
    ("dae", |b| {
        let _ = dae::parse(b);
    }),
    ("dbf", |b| {
        let _ = dbf::parse(b);
    }),
    ("dbm", |b| {
        let _ = dbm::parse(b);
    }),
    ("dcd", |b| {
        let _ = dcd::parse(b);
    }),
    ("dds", |b| {
        let _ = dds::parse(b);
    }),
    ("denyhosts", |b| {
        let _ = denyhosts::parse(b);
    }),
    ("deb", |b| {
        let _ = deb::parse(b);
    }),
    ("debconf", |b| {
        let _ = debconf::parse(b);
    }),
    ("def", |b| {
        let _ = def::parse(b);
    }),
    ("defconfig", |b| {
        let _ = defconfig::parse(b);
    }),
    ("defoldproj", |b| {
        let _ = defoldproj::parse(b);
    }),
    ("dependabot", |b| {
        let _ = dependabot::parse(b);
    }),
    ("devbox", |b| {
        let _ = devbox::parse(b);
    }),
    ("der", |b| {
        let _ = der::parse(b);
    }),
    ("desktop", |b| {
        let _ = desktop::parse(b);
    }),
    ("dex", |b| {
        let _ = dex::parse(b);
    }),
    ("dgn", |b| {
        let _ = dgn::parse(b);
    }),
    ("dhclientconf", |b| {
        let _ = dhclientconf::parse(b);
    }),
    ("dhcp", |b| {
        let _ = dhcp::parse(b);
    }),
    ("dhcpcdconf", |b| {
        let _ = dhcpcdconf::parse(b);
    }),
    ("diameter", |b| {
        let _ = diameter::parse(b);
    }),
    ("dicom", |b| {
        let _ = dicom::parse(b);
    }),
    ("digikamrc", |b| {
        let _ = digikamrc::parse(b);
    }),
    ("direwolfconf", |b| {
        let _ = direwolfconf::parse(b);
    }),
    ("dictd", |b| {
        let _ = dictd::parse(b);
    }),
    ("dictzip", |b| {
        let _ = dictzip::parse(b);
    }),
    ("did", |b| {
        let _ = did::parse(b);
    }),
    ("dif", |b| {
        let _ = dif::parse(b);
    }),
    ("dimacs", |b| {
        let _ = dimacs::parse(b);
    }),
    ("discourse", |b| {
        let _ = discourse::parse(b);
    }),
    ("dita", |b| {
        let _ = dita::parse(b);
    }),
    ("djvu", |b| {
        let _ = djvu::parse(b);
    }),
    ("dls", |b| {
        let _ = dls::parse(b);
    }),
    ("dlt", |b| {
        let _ = dlt::parse(b);
    }),
    ("dm3", |b| {
        let _ = dm3::parse(b);
    }),
    ("dmg", |b| {
        let _ = dmg::parse(b);
    }),
    ("dng", |b| {
        let _ = dng::parse(b);
    }),
    ("dns", |b| {
        let _ = dns::parse(b);
    }),
    ("docbook", |b| {
        let _ = docbook::parse(b);
    }),
    ("dockerfile", |b| {
        let _ = dockerfile::parse(b);
    }),
    ("docx", |b| {
        let _ = docx::parse(b);
    }),
    ("dolphinconf", |b| {
        let _ = dolphinconf::parse(b);
    }),
    ("dosboxconf", |b| {
        let _ = dosboxconf::parse(b);
    }),
    ("dpx", |b| {
        let _ = dpx::parse(b);
    }),
    ("dragonflyconf", |b| {
        let _ = dragonflyconf::parse(b);
    }),
    ("drbdconf", |b| {
        let _ = drbdconf::parse(b);
    }),
    ("dro", |b| {
        let _ = dro::parse(b);
    }),
    ("dsf", |b| {
        let _ = dsf::parse(b);
    }),
    ("dsig", |b| {
        let _ = dsig::parse(b);
    }),
    ("dsl", |b| {
        let _ = dsl::parse(b);
    }),
    ("dsn", |b| {
        let _ = dsn::parse(b);
    }),
    ("dsv", |b| {
        let _ = dsv::parse(b);
    }),
    ("dta", |b| {
        let _ = dta::parse(b);
    }),
    ("dtd", |b| {
        let _ = dtd::parse(b);
    }),
    ("dted", |b| {
        let _ = dted::parse(b);
    }),
    ("dvi", |b| {
        let _ = dvi::parse(b);
    }),
    ("dwarf", |b| {
        let _ = dwarf::parse(b);
    }),
    ("dwg", |b| {
        let _ = dwg::parse(b);
    }),
    ("dx", |b| {
        let _ = dx::parse(b);
    }),
    ("dxbc", |b| {
        let _ = dxbc::parse(b);
    }),
    ("e00", |b| {
        let _ = e00::parse(b);
    }),
    ("e57", |b| {
        let _ = e57::parse(b);
    }),
    ("ead", |b| {
        let _ = ead::parse(b);
    }),
    ("eaglexml", |b| {
        let _ = eaglexml::parse(b);
    }),
    ("ean", |b| {
        let _ = ean::parse(b);
    }),
    ("eap", |b| {
        let _ = eap::parse(b);
    }),
    ("earthly", |b| {
        let _ = earthly::parse(b);
    }),
    ("ebml", |b| {
        let _ = ebml::parse(b);
    }),
    ("ecat", |b| {
        let _ = ecat::parse(b);
    }),
    ("edf", |b| {
        let _ = edf::parse(b);
    }),
    ("edi", |b| {
        let _ = edi::parse(b);
    }),
    ("edsk", |b| {
        let _ = edsk::parse(b);
    }),
    ("eep", |b| {
        let _ = eep::parse(b);
    }),
    ("ejabberd", |b| {
        let _ = ejabberd::parse(b);
    }),
    ("elf", |b| {
        let _ = elf::parse(b);
    }),
    ("endnote", |b| {
        let _ = endnote::parse(b);
    }),
    ("epd", |b| {
        let _ = epd::parse(b);
    }),
    ("eps", |b| {
        let _ = eps::parse(b);
    }),
    ("epub", |b| {
        let _ = epub::parse(b);
    }),
    ("epwing", |b| {
        let _ = epwing::parse(b);
    }),
    ("erf", |b| {
        let _ = erf::parse(b);
    }),
    ("escpos", |b| {
        let _ = escpos::parse(b);
    }),
    ("esp", |b| {
        let _ = esp::parse(b);
    }),
    ("ethercat", |b| {
        let _ = ethercat::parse(b);
    }),
    ("ethernet", |b| {
        let _ = ethernet::parse(b);
    }),
    ("evt", |b| {
        let _ = evt::parse(b);
    }),
    ("evtx", |b| {
        let _ = evtx::parse(b);
    }),
    ("excellon", |b| {
        let _ = excellon::parse(b);
    }),
    ("factoriosettings", |b| {
        let _ = factoriosettings::parse(b);
    }),
    ("fail2banconf", |b| {
        let _ = fail2banconf::parse(b);
    }),
    ("exfat", |b| {
        let _ = exfat::parse(b);
    }),
    ("exr", |b| {
        let _ = exr::parse(b);
    }),
    ("ext2", |b| {
        let _ = ext2::parse(b);
    }),
    ("extmanifest", |b| {
        let _ = extmanifest::parse(b);
    }),
    ("f2fs", |b| {
        let _ = f2fs::parse(b);
    }),
    ("fail2ban", |b| {
        let _ = fail2ban::parse(b);
    }),
    ("falcoconf", |b| {
        let _ = falcoconf::parse(b);
    }),
    ("far", |b| {
        let _ = far::parse(b);
    }),
    ("farbfeld", |b| {
        let _ = farbfeld::parse(b);
    }),
    ("fat", |b| {
        let _ = fat::parse(b);
    }),
    ("fbx", |b| {
        let _ = fbx::parse(b);
    }),
    ("fceux", |b| {
        let _ = fceux::parse(b);
    }),
    ("fchk", |b| {
        let _ = fchk::parse(b);
    }),
    ("fcoe", |b| {
        let _ = fcoe::parse(b);
    }),
    ("fds", |b| {
        let _ = fds::parse(b);
    }),
    ("ferm", |b| {
        let _ = ferm::parse(b);
    }),
    ("firejailprof", |b| {
        let _ = firejailprof::parse(b);
    }),
    ("flagger", |b| {
        let _ = flagger::parse(b);
    }),
    ("fetchmailconf", |b| {
        let _ = fetchmailconf::parse(b);
    }),
    ("fgb", |b| {
        let _ = fgb::parse(b);
    }),
    ("fidl", |b| {
        let _ = fidl::parse(b);
    }),
    ("fit", |b| {
        let _ = fit::parse(b);
    }),
    ("fldigiconf", |b| {
        let _ = fldigiconf::parse(b);
    }),
    ("fits", |b| {
        let _ = fits::parse(b);
    }),
    ("fivetranconf", |b| {
        let _ = fivetranconf::parse(b);
    }),
    ("fix", |b| {
        let _ = fix::parse(b);
    }),
    ("fixml", |b| {
        let _ = fixml::parse(b);
    }),
    ("fla", |b| {
        let _ = fla::parse(b);
    }),
    ("flac", |b| {
        let _ = flac::parse(b);
    }),
    ("flatbuf", |b| {
        let _ = flatbuf::parse(b);
    }),
    ("fluxcd", |b| {
        let _ = fluxcd::parse(b);
    }),
    ("flif", |b| {
        let _ = flif::parse(b);
    }),
    ("flv", |b| {
        let _ = flv::parse(b);
    }),
    ("flyio", |b| {
        let _ = flyio::parse(b);
    }),
    ("flyway", |b| {
        let _ = flyway::parse(b);
    }),
    ("fnt", |b| {
        let _ = fnt::parse(b);
    }),
    ("fon", |b| {
        let _ = fon::parse(b);
    }),
    ("footconf", |b| {
        let _ = footconf::parse(b);
    }),
    ("fossil", |b| {
        let _ = fossil::parse(b);
    }),
    ("fpml", |b| {
        let _ = fpml::parse(b);
    }),
    ("frd", |b| {
        let _ = frd::parse(b);
    }),
    ("fsb", |b| {
        let _ = fsb::parse(b);
    }),
    ("fstab", |b| {
        let _ = fstab::parse(b);
    }),
    ("gatekeeper", |b| {
        let _ = gatekeeper::parse(b);
    }),
    ("ftl", |b| {
        let _ = ftl::parse(b);
    }),
    ("fusesoc", |b| {
        let _ = fusesoc::parse(b);
    }),
    ("fxml", |b| {
        let _ = fxml::parse(b);
    }),
    ("fxp", |b| {
        let _ = fxp::parse(b);
    }),
    ("garnetconf", |b| {
        let _ = garnetconf::parse(b);
    }),
    ("gb", |b| {
        let _ = gb::parse(b);
    }),
    ("gba", |b| {
        let _ = gba::parse(b);
    }),
    ("gbs", |b| {
        let _ = gbs::parse(b);
    }),
    ("gbstudio", |b| {
        let _ = gbstudio::parse(b);
    }),
    ("gci", |b| {
        let _ = gci::parse(b);
    }),
    ("gcode", |b| {
        let _ = gcode::parse(b);
    }),
    ("gcov", |b| {
        let _ = gcov::parse(b);
    }),
    ("gdbm", |b| {
        let _ = gdbm::parse(b);
    }),
    ("gdf", |b| {
        let _ = gdf::parse(b);
    }),
    ("gdiff", |b| {
        let _ = gdiff::parse(b);
    }),
    ("gdmconf", |b| {
        let _ = gdmconf::parse(b);
    }),
    ("gds", |b| {
        let _ = gds::parse(b);
    }),
    ("gedasch", |b| {
        let _ = gedasch::parse(b);
    }),
    ("gemrc", |b| {
        let _ = gemrc::parse(b);
    }),
    ("genbank", |b| {
        let _ = genbank::parse(b);
    }),
    ("geojson", |b| {
        let _ = geojson::parse(b);
    }),
    ("gerber", |b| {
        let _ = gerber::parse(b);
    }),
    ("gerbera", |b| {
        let _ = gerbera::parse(b);
    }),
    ("gethconf", |b| {
        let _ = gethconf::parse(b);
    }),
    ("getmailrc", |b| {
        let _ = getmailrc::parse(b);
    }),
    ("gexf", |b| {
        let _ = gexf::parse(b);
    }),
    ("gf", |b| {
        let _ = gf::parse(b);
    }),
    ("gguf", |b| {
        let _ = gguf::parse(b);
    }),
    ("ghosttyconf", |b| {
        let _ = ghosttyconf::parse(b);
    }),
    ("goreleaser", |b| {
        let _ = goreleaser::parse(b);
    }),
    ("giteaaction", |b| {
        let _ = giteaaction::parse(b);
    }),
    ("gitidx", |b| {
        let _ = gitidx::parse(b);
    }),
    ("gitpack", |b| {
        let _ = gitpack::parse(b);
    }),
    ("gostconf", |b| {
        let _ = gostconf::parse(b);
    }),
    ("glade", |b| {
        let _ = glade::parse(b);
    }),
    ("gqrxconf", |b| {
        let _ = gqrxconf::parse(b);
    }),
    ("glb", |b| {
        let _ = glb::parse(b);
    }),
    ("glsl", |b| {
        let _ = glsl::parse(b);
    }),
    ("gltf", |b| {
        let _ = gltf::parse(b);
    }),
    ("glusterfs", |b| {
        let _ = glusterfs::parse(b);
    }),
    ("gml", |b| {
        let _ = gml::parse(b);
    }),
    ("gn", |b| {
        let _ = gn::parse(b);
    }),
    ("gnuplot", |b| {
        let _ = gnuplot::parse(b);
    }),
    ("gp", |b| {
        let _ = gp::parse(b);
    }),
    ("gpkg", |b| {
        let _ = gpkg::parse(b);
    }),
    ("gpsd", |b| {
        let _ = gpsd::parse(b);
    }),
    ("graphite", |b| {
        let _ = graphite::parse(b);
    }),
    ("graphml", |b| {
        let _ = graphml::parse(b);
    }),
    ("graphql", |b| {
        let _ = graphql::parse(b);
    }),
    ("grd", |b| {
        let _ = grd::parse(b);
    }),
    ("gre", |b| {
        let _ = gre::parse(b);
    }),
    ("greetd", |b| {
        let _ = greetd::parse(b);
    }),
    ("grib", |b| {
        let _ = grib::parse(b);
    }),
    ("gro", |b| {
        let _ = gro::parse(b);
    }),
    ("group", |b| {
        let _ = group::parse(b);
    }),
    ("grp", |b| {
        let _ = grp::parse(b);
    }),
    ("grubcfg", |b| {
        let _ = grubcfg::parse(b);
    }),
    ("grubconf", |b| {
        let _ = grubconf::parse(b);
    }),
    ("grubenv", |b| {
        let _ = grubenv::parse(b);
    }),
    ("gtp", |b| {
        let _ = gtp::parse(b);
    }),
    ("gxf", |b| {
        let _ = gxf::parse(b);
    }),
    ("gym", |b| {
        let _ = gym::parse(b);
    }),
    ("gzip", |b| {
        let _ = gzip::parse(b);
    }),
    ("hacf", |b| {
        let _ = hacf::parse(b);
    }),
    ("hadolintconf", |b| {
        let _ = hadolintconf::parse(b);
    }),
    ("hadoopconf", |b| {
        let _ = hadoopconf::parse(b);
    }),
    ("haresources", |b| {
        let _ = haresources::parse(b);
    }),
    ("harness", |b| {
        let _ = harness::parse(b);
    }),
    ("hb", |b| {
        let _ = hb::parse(b);
    }),
    ("hcl", |b| {
        let _ = hcl::parse(b);
    }),
    ("hdlc", |b| {
        let _ = hdlc::parse(b);
    }),
    ("hdr", |b| {
        let _ = hdr::parse(b);
    }),
    ("heif", |b| {
        let _ = heif::parse(b);
    }),
    ("hes", |b| {
        let _ = hes::parse(b);
    }),
    ("hexchat", |b| {
        let _ = hexchat::parse(b);
    }),
    ("hfe", |b| {
        let _ = hfe::parse(b);
    }),
    ("hfs", |b| {
        let _ = hfs::parse(b);
    }),
    ("hfsplus", |b| {
        let _ = hfsplus::parse(b);
    }),
    ("hgt", |b| {
        let _ = hgt::parse(b);
    }),
    ("hiberfil", |b| {
        let _ = hiberfil::parse(b);
    }),
    ("himalayaconf", |b| {
        let _ = himalayaconf::parse(b);
    }),
    ("hivemq", |b| {
        let _ = hivemq::parse(b);
    }),
    ("hysteriaconf", |b| {
        let _ = hysteriaconf::parse(b);
    }),
    ("hlsl", |b| {
        let _ = hlsl::parse(b);
    }),
    ("hocr", |b| {
        let _ = hocr::parse(b);
    }),
    ("hopconf", |b| {
        let _ = hopconf::parse(b);
    }),
    ("hostapd", |b| {
        let _ = hostapd::parse(b);
    }),
    ("hosts", |b| {
        let _ = hosts::parse(b);
    }),
    ("hpgl", |b| {
        let _ = hpgl::parse(b);
    }),
    ("hqx", |b| {
        let _ = hqx::parse(b);
    }),
    ("htaccess", |b| {
        let _ = htaccess::parse(b);
    }),
    ("hus", |b| {
        let _ = hus::parse(b);
    }),
    ("hydrogen", |b| {
        let _ = hydrogen::parse(b);
    }),
    ("ibmmq", |b| {
        let _ = ibmmq::parse(b);
    }),
    ("ical", |b| {
        let _ = ical::parse(b);
    }),
    ("icc", |b| {
        let _ = icc::parse(b);
    }),
    ("icmp", |b| {
        let _ = icmp::parse(b);
    }),
    ("icns", |b| {
        let _ = icns::parse(b);
    }),
    ("ico", |b| {
        let _ = ico::parse(b);
    }),
    ("id3", |b| {
        let _ = id3::parse(b);
    }),
    ("idl", |b| {
        let _ = idl::parse(b);
    }),
    ("ifc", |b| {
        let _ = ifc::parse(b);
    }),
    ("ifd", |b| {
        let _ = ifd::parse(b);
    }),
    ("iff", |b| {
        let _ = iff::parse(b);
    }),
    ("iges", |b| {
        let _ = iges::parse(b);
    }),
    ("igmp", |b| {
        let _ = igmp::parse(b);
    }),
    ("imap", |b| {
        let _ = imap::parse(b);
    }),
    ("imd", |b| {
        let _ = imd::parse(b);
    }),
    ("imscc", |b| {
        let _ = imscc::parse(b);
    }),
    ("imx", |b| {
        let _ = imx::parse(b);
    }),
    ("ines", |b| {
        let _ = ines::parse(b);
    }),
    ("influx", |b| {
        let _ = influx::parse(b);
    }),
    ("inp", |b| {
        let _ = inp::parse(b);
    }),
    ("interfile", |b| {
        let _ = interfile::parse(b);
    }),
    ("intoto", |b| {
        let _ = intoto::parse(b);
    }),
    ("ioc", |b| {
        let _ = ioc::parse(b);
    }),
    ("ion", |b| {
        let _ = ion::parse(b);
    }),
    ("ipfix", |b| {
        let _ = ipfix::parse(b);
    }),
    ("ipk", |b| {
        let _ = ipk::parse(b);
    }),
    ("ips", |b| {
        let _ = ips::parse(b);
    }),
    ("ipset", |b| {
        let _ = ipset::parse(b);
    }),
    ("iptablessave", |b| {
        let _ = iptablessave::parse(b);
    }),
    ("iptc", |b| {
        let _ = iptc::parse(b);
    }),
    ("ipv4", |b| {
        let _ = ipv4::parse(b);
    }),
    ("ipv6", |b| {
        let _ = ipv6::parse(b);
    }),
    ("ipxact", |b| {
        let _ = ipxact::parse(b);
    }),
    ("ipxescript", |b| {
        let _ = ipxescript::parse(b);
    }),
    ("irc", |b| {
        let _ = irc::parse(b);
    }),
    ("ircam", |b| {
        let _ = ircam::parse(b);
    }),
    ("irssi", |b| {
        let _ = irssi::parse(b);
    }),
    ("isabelle", |b| {
        let _ = isabelle::parse(b);
    }),
    ("isakmp", |b| {
        let _ = isakmp::parse(b);
    }),
    ("isbn", |b| {
        let _ = isbn::parse(b);
    }),
    ("isc", |b| {
        let _ = isc::parse(b);
    }),
    ("iscsi", |b| {
        let _ = iscsi::parse(b);
    }),
    ("isis", |b| {
        let _ = isis::parse(b);
    }),
    ("ismn", |b| {
        let _ = ismn::parse(b);
    }),
    ("iso8583", |b| {
        let _ = iso8583::parse(b);
    }),
    ("iso9660", |b| {
        let _ = iso9660::parse(b);
    }),
    ("isobmff", |b| {
        let _ = isobmff::parse(b);
    }),
    ("issn", |b| {
        let _ = issn::parse(b);
    }),
    ("it", |b| {
        let _ = it::parse(b);
    }),
    ("iterm", |b| {
        let _ = iterm::parse(b);
    }),
    ("itermdyn", |b| {
        let _ = itermdyn::parse(b);
    }),
    ("iti", |b| {
        let _ = iti::parse(b);
    }),
    ("iv", |b| {
        let _ = iv::parse(b);
    }),
    ("ivf", |b| {
        let _ = ivf::parse(b);
    }),
    ("iwdconf", |b| {
        let _ = iwdconf::parse(b);
    }),
    ("jar", |b| {
        let _ = jar::parse(b);
    }),
    ("jats", |b| {
        let _ = jats::parse(b);
    }),
    ("jbig2", |b| {
        let _ = jbig2::parse(b);
    }),
    ("jdx", |b| {
        let _ = jdx::parse(b);
    }),
    ("jed", |b| {
        let _ = jed::parse(b);
    }),
    ("jef", |b| {
        let _ = jef::parse(b);
    }),
    ("jffs2", |b| {
        let _ = jffs2::parse(b);
    }),
    ("jfm", |b| {
        let _ = jfm::parse(b);
    }),
    ("jfs", |b| {
        let _ = jfs::parse(b);
    }),
    ("jks", |b| {
        let _ = jks::parse(b);
    }),
    ("journal", |b| {
        let _ = journal::parse(b);
    }),
    ("journaldconf", |b| {
        let _ = journaldconf::parse(b);
    }),
    ("jp2", |b| {
        let _ = jp2::parse(b);
    }),
    ("jpeg", |b| {
        let _ = jpeg::parse(b);
    }),
    ("jq", |b| {
        let _ = jq::parse(b);
    }),
    ("json", |b| {
        let _ = json::parse(b);
    }),
    ("jsonpath", |b| {
        let _ = jsonpath::parse(b);
    }),
    ("jumplist", |b| {
        let _ = jumplist::parse(b);
    }),
    ("junit", |b| {
        let _ = junit::parse(b);
    }),
    ("justfile", |b| {
        let _ = justfile::parse(b);
    }),
    ("jwe", |b| {
        let _ = jwe::parse(b);
    }),
    ("jwk", |b| {
        let _ = jwk::parse(b);
    }),
    ("jxl", |b| {
        let _ = jxl::parse(b);
    }),
    ("jxr", |b| {
        let _ = jxr::parse(b);
    }),
    ("k", |b| {
        let _ = k::parse(b);
    }),
    ("k0sconf", |b| {
        let _ = k0sconf::parse(b);
    }),
    ("k3sconf", |b| {
        let _ = k3sconf::parse(b);
    }),
    ("kanata", |b| {
        let _ = kanata::parse(b);
    }),
    ("kap", |b| {
        let _ = kap::parse(b);
    }),
    ("kbm", |b| {
        let _ = kbm::parse(b);
    }),
    ("kconfig", |b| {
        let _ = kconfig::parse(b);
    }),
    ("kdbx", |b| {
        let _ = kdbx::parse(b);
    }),
    ("kdeglobals", |b| {
        let _ = kdeglobals::parse(b);
    }),
    ("kern", |b| {
        let _ = kern::parse(b);
    }),
    ("ketl", |b| {
        let _ = ketl::parse(b);
    }),
    ("keyd", |b| {
        let _ = keyd::parse(b);
    }),
    ("keydbconf", |b| {
        let _ = keydbconf::parse(b);
    }),
    ("keytab", |b| {
        let _ = keytab::parse(b);
    }),
    ("kicadpcb", |b| {
        let _ = kicadpcb::parse(b);
    }),
    ("kicadpro", |b| {
        let _ = kicadpro::parse(b);
    }),
    ("kicadsch", |b| {
        let _ = kicadsch::parse(b);
    }),
    ("kickstart", |b| {
        let _ = kickstart::parse(b);
    }),
    ("kif", |b| {
        let _ = kif::parse(b);
    }),
    ("kittyimg", |b| {
        let _ = kittyimg::parse(b);
    }),
    ("klipperconf", |b| {
        let _ = klipperconf::parse(b);
    }),
    ("knative", |b| {
        let _ = knative::parse(b);
    }),
    ("kmz", |b| {
        let _ = kmz::parse(b);
    }),
    ("knexfile", |b| {
        let _ = knexfile::parse(b);
    }),
    ("knx", |b| {
        let _ = knx::parse(b);
    }),
    ("kql", |b| {
        let _ = kql::parse(b);
    }),
    ("krb5conf", |b| {
        let _ = krb5conf::parse(b);
    }),
    ("kyverno", |b| {
        let _ = kyverno::parse(b);
    }),
    ("kss", |b| {
        let _ = kss::parse(b);
    }),
    ("kubemq", |b| {
        let _ = kubemq::parse(b);
    }),
    ("l2tp", |b| {
        let _ = l2tp::parse(b);
    }),
    ("las", |b| {
        let _ = las::parse(b);
    }),
    ("lcov", |b| {
        let _ = lcov::parse(b);
    }),
    ("ldap", |b| {
        let _ = ldap::parse(b);
    }),
    ("ldblog", |b| {
        let _ = ldblog::parse(b);
    }),
    ("ldif", |b| {
        let _ = ldif::parse(b);
    }),
    ("ldirectord", |b| {
        let _ = ldirectord::parse(b);
    }),
    ("ldtk", |b| {
        let _ = ldtk::parse(b);
    }),
    ("le", |b| {
        let _ = le::parse(b);
    }),
    ("lean", |b| {
        let _ = lean::parse(b);
    }),
    ("leda", |b| {
        let _ = leda::parse(b);
    }),
    ("ledgerjournal", |b| {
        let _ = ledgerjournal::parse(b);
    }),
    ("lefthook", |b| {
        let _ = lefthook::parse(b);
    }),
    ("lef", |b| {
        let _ = lef::parse(b);
    }),
    ("leiningen", |b| {
        let _ = leiningen::parse(b);
    }),
    ("locxml", |b| {
        let _ = locxml::parse(b);
    }),
    ("lintstaged", |b| {
        let _ = lintstaged::parse(b);
    }),
    ("lha", |b| {
        let _ = lha::parse(b);
    }),
    ("litmus", |b| {
        let _ = litmus::parse(b);
    }),
    ("liberty", |b| {
        let _ = liberty::parse(b);
    }),
    ("lightdm", |b| {
        let _ = lightdm::parse(b);
    }),
    ("limine", |b| {
        let _ = limine::parse(b);
    }),
    ("liquibase", |b| {
        let _ = liquibase::parse(b);
    }),
    ("lit", |b| {
        let _ = lit::parse(b);
    }),
    ("lldp", |b| {
        let _ = lldp::parse(b);
    }),
    ("llmnr", |b| {
        let _ = llmnr::parse(b);
    }),
    ("llvmbc", |b| {
        let _ = llvmbc::parse(b);
    }),
    ("lmms", |b| {
        let _ = lmms::parse(b);
    }),
    ("lndconf", |b| {
        let _ = lndconf::parse(b);
    }),
    ("lnk", |b| {
        let _ = lnk::parse(b);
    }),
    ("log4j", |b| {
        let _ = log4j::parse(b);
    }),
    ("log4perl", |b| {
        let _ = log4perl::parse(b);
    }),
    ("logback", |b| {
        let _ = logback::parse(b);
    }),
    ("logindefs", |b| {
        let _ = logindefs::parse(b);
    }),
    ("logrotate", |b| {
        let _ = logrotate::parse(b);
    }),
    ("loki", |b| {
        let _ = loki::parse(b);
    }),
    ("loveconf", |b| {
        let _ = loveconf::parse(b);
    }),
    ("lp", |b| {
        let _ = lp::parse(b);
    }),
    ("lpf", |b| {
        let _ = lpf::parse(b);
    }),
    ("lrc", |b| {
        let _ = lrc::parse(b);
    }),
    ("ltsconf", |b| {
        let _ = ltsconf::parse(b);
    }),
    ("luac", |b| {
        let _ = luac::parse(b);
    }),
    ("lucene", |b| {
        let _ = lucene::parse(b);
    }),
    ("luhn", |b| {
        let _ = luhn::parse(b);
    }),
    ("luigi", |b| {
        let _ = luigi::parse(b);
    }),
    ("lvmconf", |b| {
        let _ = lvmconf::parse(b);
    }),
    ("lwo", |b| {
        let _ = lwo::parse(b);
    }),
    ("ly", |b| {
        let _ = ly::parse(b);
    }),
    ("lynisconf", |b| {
        let _ = lynisconf::parse(b);
    }),
    ("lz4f", |b| {
        let _ = lz4f::parse(b);
    }),
    ("lzfse", |b| {
        let _ = lzfse::parse(b);
    }),
    ("lzip", |b| {
        let _ = lzip::parse(b);
    }),
    ("macaroon", |b| {
        let _ = macaroon::parse(b);
    }),
    ("macho", |b| {
        let _ = macho::parse(b);
    }),
    ("maf", |b| {
        let _ = maf::parse(b);
    }),
    ("mailcap", |b| {
        let _ = mailcap::parse(b);
    }),
    ("maildrop", |b| {
        let _ = maildrop::parse(b);
    }),
    ("makefile", |b| {
        let _ = makefile::parse(b);
    }),
    ("mameconf", |b| {
        let _ = mameconf::parse(b);
    }),
    ("mapfile", |b| {
        let _ = mapfile::parse(b);
    }),
    ("mapnikxml", |b| {
        let _ = mapnikxml::parse(b);
    }),
    ("mapproxyconf", |b| {
        let _ = mapproxyconf::parse(b);
    }),
    ("marc", |b| {
        let _ = marc::parse(b);
    }),
    ("markdownlint", |b| {
        let _ = markdownlint::parse(b);
    }),
    ("marlinconf", |b| {
        let _ = marlinconf::parse(b);
    }),
    ("mat", |b| {
        let _ = mat::parse(b);
    }),
    ("matplotlibrc", |b| {
        let _ = matplotlibrc::parse(b);
    }),
    ("matterbridge", |b| {
        let _ = matterbridge::parse(b);
    }),
    ("mattermost", |b| {
        let _ = mattermost::parse(b);
    }),
    ("maud", |b| {
        let _ = maud::parse(b);
    }),
    ("mavlink", |b| {
        let _ = mavlink::parse(b);
    }),
    ("mbedapp", |b| {
        let _ = mbedapp::parse(b);
    }),
    ("mbox", |b| {
        let _ = mbox::parse(b);
    }),
    ("mbsyncrc", |b| {
        let _ = mbsyncrc::parse(b);
    }),
    ("mbtiles", |b| {
        let _ = mbtiles::parse(b);
    }),
    ("mbus", |b| {
        let _ = mbus::parse(b);
    }),
    ("mcap", |b| {
        let _ = mcap::parse(b);
    }),
    ("mcr", |b| {
        let _ = mcr::parse(b);
    }),
    ("mcserverprops", |b| {
        let _ = mcserverprops::parse(b);
    }),
    ("md2", |b| {
        let _ = md2::parse(b);
    }),
    ("md3", |b| {
        let _ = md3::parse(b);
    }),
    ("mdb", |b| {
        let _ = mdb::parse(b);
    }),
    ("mdl", |b| {
        let _ = mdl::parse(b);
    }),
    ("mdx", |b| {
        let _ = mdx::parse(b);
    }),
    ("med", |b| {
        let _ = med::parse(b);
    }),
    ("mediawiki", |b| {
        let _ = mediawiki::parse(b);
    }),
    ("medline", |b| {
        let _ = medline::parse(b);
    }),
    ("mednafen", |b| {
        let _ = mednafen::parse(b);
    }),
    ("mei", |b| {
        let _ = mei::parse(b);
    }),
    ("melonds", |b| {
        let _ = melonds::parse(b);
    }),
    ("meltano", |b| {
        let _ = meltano::parse(b);
    }),
    ("memcachedconf", |b| {
        let _ = memcachedconf::parse(b);
    }),
    ("mergify", |b| {
        let _ = mergify::parse(b);
    }),
    ("meson", |b| {
        let _ = meson::parse(b);
    }),
    ("metallib", |b| {
        let _ = metallib::parse(b);
    }),
    ("mets", |b| {
        let _ = mets::parse(b);
    }),
    ("minidlna", |b| {
        let _ = minidlna::parse(b);
    }),
    ("mmlstyle", |b| {
        let _ = mmlstyle::parse(b);
    }),
    ("mft", |b| {
        let _ = mft::parse(b);
    }),
    ("mhd", |b| {
        let _ = mhd::parse(b);
    }),
    ("mht", |b| {
        let _ = mht::parse(b);
    }),
    ("midi", |b| {
        let _ = midi::parse(b);
    }),
    ("mime", |b| {
        let _ = mime::parse(b);
    }),
    ("modsecurity", |b| {
        let _ = modsecurity::parse(b);
    }),
    ("mimirconf", |b| {
        let _ = mimirconf::parse(b);
    }),
    ("mise", |b| {
        let _ = mise::parse(b);
    }),
    ("minc", |b| {
        let _ = minc::parse(b);
    }),
    ("minikubeconf", |b| {
        let _ = minikubeconf::parse(b);
    }),
    ("minix", |b| {
        let _ = minix::parse(b);
    }),
    ("misp", |b| {
        let _ = misp::parse(b);
    }),
    ("mixexs", |b| {
        let _ = mixexs::parse(b);
    }),
    ("mixxx", |b| {
        let _ = mixxx::parse(b);
    }),
    ("mml", |b| {
        let _ = mml::parse(b);
    }),
    ("mobi", |b| {
        let _ = mobi::parse(b);
    }),
    ("modeldo", |b| {
        let _ = modeldo::parse(b);
    }),
    ("modfile", |b| {
        let _ = modfile::parse(b);
    }),
    ("modprobeconf", |b| {
        let _ = modprobeconf::parse(b);
    }),
    ("mods", |b| {
        let _ = mods::parse(b);
    }),
    ("mol", |b| {
        let _ = mol::parse(b);
    }),
    ("mol2", |b| {
        let _ = mol2::parse(b);
    }),
    ("monero", |b| {
        let _ = monero::parse(b);
    }),
    ("moonrakerconf", |b| {
        let _ = moonrakerconf::parse(b);
    }),
    ("mp3", |b| {
        let _ = mp3::parse(b);
    }),
    ("mpegts", |b| {
        let _ = mpegts::parse(b);
    }),
    ("mpl2", |b| {
        let _ = mpl2::parse(b);
    }),
    ("mpq", |b| {
        let _ = mpq::parse(b);
    }),
    ("mps", |b| {
        let _ = mps::parse(b);
    }),
    ("mpvconf", |b| {
        let _ = mpvconf::parse(b);
    }),
    ("mqtt", |b| {
        let _ = mqtt::parse(b);
    }),
    ("mrc", |b| {
        let _ = mrc::parse(b);
    }),
    ("mscx", |b| {
        let _ = mscx::parse(b);
    }),
    ("mseed", |b| {
        let _ = mseed::parse(b);
    }),
    ("msf", |b| {
        let _ = msf::parse(b);
    }),
    ("naxsiconf", |b| {
        let _ = naxsiconf::parse(b);
    }),
    ("msh", |b| {
        let _ = msh::parse(b);
    }),
    ("msi", |b| {
        let _ = msi::parse(b);
    }),
    ("msmtprc", |b| {
        let _ = msmtprc::parse(b);
    }),
    ("mt940", |b| {
        let _ = mt940::parse(b);
    }),
    ("mtm", |b| {
        let _ = mtm::parse(b);
    }),
    ("mtx", |b| {
        let _ = mtx::parse(b);
    }),
    ("musicxml", |b| {
        let _ = musicxml::parse(b);
    }),
    ("muttrc", |b| {
        let _ = muttrc::parse(b);
    }),
    ("mvnsettings", |b| {
        let _ = mvnsettings::parse(b);
    }),
    ("mvt", |b| {
        let _ = mvt::parse(b);
    }),
    ("mxf", |b| {
        let _ = mxf::parse(b);
    }),
    ("mzml", |b| {
        let _ = mzml::parse(b);
    }),
    ("nanoid", |b| {
        let _ = nanoid::parse(b);
    }),
    ("nas", |b| {
        let _ = nas::parse(b);
    }),
    ("nbd", |b| {
        let _ = nbd::parse(b);
    }),
    ("nbt", |b| {
        let _ = nbt::parse(b);
    }),
    ("nc", |b| {
        let _ = nc::parse(b);
    }),
    ("ne", |b| {
        let _ = ne::parse(b);
    }),
    ("nef", |b| {
        let _ = nef::parse(b);
    }),
    ("neomuttconf", |b| {
        let _ = neomuttconf::parse(b);
    }),
    ("nerdctl", |b| {
        let _ = nerdctl::parse(b);
    }),
    ("netflow", |b| {
        let _ = netflow::parse(b);
    }),
    ("netlifyconf", |b| {
        let _ = netlifyconf::parse(b);
    }),
    ("netrc", |b| {
        let _ = netrc::parse(b);
    }),
    ("networkd", |b| {
        let _ = networkd::parse(b);
    }),
    ("nfpm", |b| {
        let _ = nfpm::parse(b);
    }),
    ("neu", |b| {
        let _ = neu::parse(b);
    }),
    ("newick", |b| {
        let _ = newick::parse(b);
    }),
    ("newsboat", |b| {
        let _ = newsboat::parse(b);
    }),
    ("newsyslog", |b| {
        let _ = newsyslog::parse(b);
    }),
    ("nextflow", |b| {
        let _ = nextflow::parse(b);
    }),
    ("nexus", |b| {
        let _ = nexus::parse(b);
    }),
    ("nfsexports", |b| {
        let _ = nfsexports::parse(b);
    }),
    ("nftconf", |b| {
        let _ = nftconf::parse(b);
    }),
    ("ngircd", |b| {
        let _ = ngircd::parse(b);
    }),
    ("nib", |b| {
        let _ = nib::parse(b);
    }),
    ("nififlow", |b| {
        let _ = nififlow::parse(b);
    }),
    ("nifti", |b| {
        let _ = nifti::parse(b);
    }),
    ("nimble", |b| {
        let _ = nimble::parse(b);
    }),
    ("ninja", |b| {
        let _ = ninja::parse(b);
    }),
    ("nist", |b| {
        let _ = nist::parse(b);
    }),
    ("nix", |b| {
        let _ = nix::parse(b);
    }),
    ("nlogconf", |b| {
        let _ = nlogconf::parse(b);
    }),
    ("nmconnection", |b| {
        let _ = nmconnection::parse(b);
    }),
    ("nntp", |b| {
        let _ = nntp::parse(b);
    }),
    ("npmrc", |b| {
        let _ = npmrc::parse(b);
    }),
    ("npy", |b| {
        let _ = npy::parse(b);
    }),
    ("nrg", |b| {
        let _ = nrg::parse(b);
    }),
    ("nrrd", |b| {
        let _ = nrrd::parse(b);
    }),
    ("nsf", |b| {
        let _ = nsf::parse(b);
    }),
    ("nsjailcfg", |b| {
        let _ = nsjailcfg::parse(b);
    }),
    ("nslcdconf", |b| {
        let _ = nslcdconf::parse(b);
    }),
    ("nsqconf", |b| {
        let _ = nsqconf::parse(b);
    }),
    ("ntfs", |b| {
        let _ = ntfs::parse(b);
    }),
    ("ntp", |b| {
        let _ = ntp::parse(b);
    }),
    ("ntpconf", |b| {
        let _ = ntpconf::parse(b);
    }),
    ("ntpsec", |b| {
        let _ = ntpsec::parse(b);
    }),
    ("nuget", |b| {
        let _ = nuget::parse(b);
    }),
    ("nugetconfig", |b| {
        let _ = nugetconfig::parse(b);
    }),
    ("nullmailerconf", |b| {
        let _ = nullmailerconf::parse(b);
    }),
    ("nunit", |b| {
        let _ = nunit::parse(b);
    }),
    ("nwc", |b| {
        let _ = nwc::parse(b);
    }),
    ("nzbget", |b| {
        let _ = nzbget::parse(b);
    }),
    ("oai", |b| {
        let _ = oai::parse(b);
    }),
    ("ocsp", |b| {
        let _ = ocsp::parse(b);
    }),
    ("octaverc", |b| {
        let _ = octaverc::parse(b);
    }),
    ("octoprint", |b| {
        let _ = octoprint::parse(b);
    }),
    ("odex", |b| {
        let _ = odex::parse(b);
    }),
    ("oem", |b| {
        let _ = oem::parse(b);
    }),
    ("off", |b| {
        let _ = off::parse(b);
    }),
    ("offlineimap", |b| {
        let _ = offlineimap::parse(b);
    }),
    ("ofx", |b| {
        let _ = ofx::parse(b);
    }),
    ("ogmo", |b| {
        let _ = ogmo::parse(b);
    }),
    ("ole", |b| {
        let _ = ole::parse(b);
    }),
    ("omm", |b| {
        let _ = omm::parse(b);
    }),
    ("onnx", |b| {
        let _ = onnx::parse(b);
    }),
    ("op2", |b| {
        let _ = op2::parse(b);
    }),
    ("opam", |b| {
        let _ = opam::parse(b);
    }),
    ("opb", |b| {
        let _ = opb::parse(b);
    }),
    ("openapi", |b| {
        let _ = openapi::parse(b);
    }),
    ("openlane", |b| {
        let _ = openlane::parse(b);
    }),
    ("openmsx", |b| {
        let _ = openmsx::parse(b);
    }),
    ("overpass", |b| {
        let _ = overpass::parse(b);
    }),
    ("openntpd", |b| {
        let _ = openntpd::parse(b);
    }),
    ("opentsdb", |b| {
        let _ = opentsdb::parse(b);
    }),
    ("opml", |b| {
        let _ = opml::parse(b);
    }),
    ("optionrom", |b| {
        let _ = optionrom::parse(b);
    }),
    ("orcaslicer", |b| {
        let _ = orcaslicer::parse(b);
    }),
    ("orcid", |b| {
        let _ = orcid::parse(b);
    }),
    ("orf", |b| {
        let _ = orf::parse(b);
    }),
    ("org", |b| {
        let _ = org::parse(b);
    }),
    ("osc", |b| {
        let _ = osc::parse(b);
    }),
    ("osm2pgsqlstyle", |b| {
        let _ = osm2pgsqlstyle::parse(b);
    }),
    ("osmpbf", |b| {
        let _ = osmpbf::parse(b);
    }),
    ("ospf", |b| {
        let _ = ospf::parse(b);
    }),
    ("osqueryconf", |b| {
        let _ = osqueryconf::parse(b);
    }),
    ("ossecconf", |b| {
        let _ = ossecconf::parse(b);
    }),
    ("osv", |b| {
        let _ = osv::parse(b);
    }),
    ("otf", |b| {
        let _ = otf::parse(b);
    }),
    ("p7b", |b| {
        let _ = p7b::parse(b);
    }),
    ("pacemaker", |b| {
        let _ = pacemaker::parse(b);
    }),
    ("packfile", |b| {
        let _ = packfile::parse(b);
    }),
    ("paf", |b| {
        let _ = paf::parse(b);
    }),
    ("pain", |b| {
        let _ = pain::parse(b);
    }),
    ("pajek", |b| {
        let _ = pajek::parse(b);
    }),
    ("pak", |b| {
        let _ = pak::parse(b);
    }),
    ("pamstack", |b| {
        let _ = pamstack::parse(b);
    }),
    ("pants", |b| {
        let _ = pants::parse(b);
    }),
    ("parityconf", |b| {
        let _ = parityconf::parse(b);
    }),
    ("parquet", |b| {
        let _ = parquet::parse(b);
    }),
    ("parrec", |b| {
        let _ = parrec::parse(b);
    }),
    ("paseto", |b| {
        let _ = paseto::parse(b);
    }),
    ("passwd", |b| {
        let _ = passwd::parse(b);
    }),
    ("pat", |b| {
        let _ = pat::parse(b);
    }),
    ("pcap", |b| {
        let _ = pcap::parse(b);
    }),
    ("pcapng", |b| {
        let _ = pcapng::parse(b);
    }),
    ("pcd", |b| {
        let _ = pcd::parse(b);
    }),
    ("pcf", |b| {
        let _ = pcf::parse(b);
    }),
    ("pck", |b| {
        let _ = pck::parse(b);
    }),
    ("pcl", |b| {
        let _ = pcl::parse(b);
    }),
    ("pcsx2conf", |b| {
        let _ = pcsx2conf::parse(b);
    }),
    ("pcx", |b| {
        let _ = pcx::parse(b);
    }),
    ("pdb", |b| {
        let _ = pdb::parse(b);
    }),
    ("pdf", |b| {
        let _ = pdf::parse(b);
    }),
    ("pds", |b| {
        let _ = pds::parse(b);
    }),
    ("pec", |b| {
        let _ = pec::parse(b);
    }),
    ("pes", |b| {
        let _ = pes::parse(b);
    }),
    ("pfconf", |b| {
        let _ = pfconf::parse(b);
    }),
    ("pfm", |b| {
        let _ = pfm::parse(b);
    }),
    ("phylip", |b| {
        let _ = phylip::parse(b);
    }),
    ("pidginconf", |b| {
        let _ = pidginconf::parse(b);
    }),
    ("pileup", |b| {
        let _ = pileup::parse(b);
    }),
    ("pim", |b| {
        let _ = pim::parse(b);
    }),
    ("pinerc", |b| {
        let _ = pinerc::parse(b);
    }),
    ("pjs", |b| {
        let _ = pjs::parse(b);
    }),
    ("pk", |b| {
        let _ = pk::parse(b);
    }),
    ("pkcs12", |b| {
        let _ = pkcs12::parse(b);
    }),
    ("pkcs8", |b| {
        let _ = pkcs8::parse(b);
    }),
    ("pkg", |b| {
        let _ = pkg::parse(b);
    }),
    ("pkgbuild", |b| {
        let _ = pkgbuild::parse(b);
    }),
    ("pl", |b| {
        let _ = pl::parse(b);
    }),
    ("planetilerconf", |b| {
        let _ = planetilerconf::parse(b);
    }),
    ("platformio", |b| {
        let _ = platformio::parse(b);
    }),
    ("platformsh", |b| {
        let _ = platformsh::parse(b);
    }),
    ("pls", |b| {
        let _ = pls::parse(b);
    }),
    ("ply", |b| {
        let _ = ply::parse(b);
    }),
    ("pmacctconf", |b| {
        let _ = pmacctconf::parse(b);
    }),
    ("pmd", |b| {
        let _ = pmd::parse(b);
    }),
    ("postsrsdconf", |b| {
        let _ = postsrsdconf::parse(b);
    }),
    ("projjson", |b| {
        let _ = projjson::parse(b);
    }),
    ("pmtiles", |b| {
        let _ = pmtiles::parse(b);
    }),
    ("pmx", |b| {
        let _ = pmx::parse(b);
    }),
    ("pnm", |b| {
        let _ = pnm::parse(b);
    }),
    ("po", |b| {
        let _ = po::parse(b);
    }),
    ("pod", |b| {
        let _ = pod::parse(b);
    }),
    ("pop3", |b| {
        let _ = pop3::parse(b);
    }),
    ("precommit", |b| {
        let _ = precommit::parse(b);
    }),
    ("poscar", |b| {
        let _ = poscar::parse(b);
    }),
    ("ppf", |b| {
        let _ = ppf::parse(b);
    }),
    ("ppp", |b| {
        let _ = ppp::parse(b);
    }),
    ("pppdconf", |b| {
        let _ = pppdconf::parse(b);
    }),
    ("ppssppconf", |b| {
        let _ = ppssppconf::parse(b);
    }),
    ("pptx", |b| {
        let _ = pptx::parse(b);
    }),
    ("prefetch", |b| {
        let _ = prefetch::parse(b);
    }),
    ("preseed", |b| {
        let _ = preseed::parse(b);
    }),
    ("prj", |b| {
        let _ = prj::parse(b);
    }),
    ("procfile", |b| {
        let _ = procfile::parse(b);
    }),
    ("procmailrc", |b| {
        let _ = procmailrc::parse(b);
    }),
    ("prom", |b| {
        let _ = prom::parse(b);
    }),
    ("promtailconf", |b| {
        let _ = promtailconf::parse(b);
    }),
    ("proselint", |b| {
        let _ = proselint::parse(b);
    }),
    ("prosody", |b| {
        let _ = prosody::parse(b);
    }),
    ("prusaslicer", |b| {
        let _ = prusaslicer::parse(b);
    }),
    ("psd", |b| {
        let _ = psd::parse(b);
    }),
    ("psf", |b| {
        let _ = psf::parse(b);
    }),
    ("psid", |b| {
        let _ = psid::parse(b);
    }),
    ("ptm", |b| {
        let _ = ptm::parse(b);
    }),
    ("ptp4l", |b| {
        let _ = ptp4l::parse(b);
    }),
    ("ptx", |b| {
        let _ = ptx::parse(b);
    }),
    ("pubspec", |b| {
        let _ = pubspec::parse(b);
    }),
    ("puz", |b| {
        let _ = puz::parse(b);
    }),
    ("pxelinux", |b| {
        let _ = pxelinux::parse(b);
    }),
    ("pyc", |b| {
        let _ = pyc::parse(b);
    }),
    ("pypirc", |b| {
        let _ = pypirc::parse(b);
    }),
    ("pyroconf", |b| {
        let _ = pyroconf::parse(b);
    }),
    ("pzserver", |b| {
        let _ = pzserver::parse(b);
    }),
    ("qcow2", |b| {
        let _ = qcow2::parse(b);
    }),
    ("qcp", |b| {
        let _ = qcp::parse(b);
    }),
    ("qgsproj", |b| {
        let _ = qgsproj::parse(b);
    }),
    ("qif", |b| {
        let _ = qif::parse(b);
    }),
    ("qmap", |b| {
        let _ = qmap::parse(b);
    }),
    ("qpf", |b| {
        let _ = qpf::parse(b);
    }),
    ("qsf", |b| {
        let _ = qsf::parse(b);
    }),
    ("qt5ctconf", |b| {
        let _ = qt5ctconf::parse(b);
    }),
    ("qti", |b| {
        let _ = qti::parse(b);
    }),
    ("qtui", |b| {
        let _ = qtui::parse(b);
    }),
    ("radius", |b| {
        let _ = radius::parse(b);
    }),
    ("raf", |b| {
        let _ = raf::parse(b);
    }),
    ("railwayconf", |b| {
        let _ = railwayconf::parse(b);
    }),
    ("rakefile", |b| {
        let _ = rakefile::parse(b);
    }),
    ("rar", |b| {
        let _ = rar::parse(b);
    }),
    ("ras", |b| {
        let _ = ras::parse(b);
    }),
    ("razorconf", |b| {
        let _ = razorconf::parse(b);
    }),
    ("rc", |b| {
        let _ = rc::parse(b);
    }),
    ("rdata", |b| {
        let _ = rdata::parse(b);
    }),
    ("rdb", |b| {
        let _ = rdb::parse(b);
    }),
    ("rdiff", |b| {
        let _ = rdiff::parse(b);
    }),
    ("rdpfile", |b| {
        let _ = rdpfile::parse(b);
    }),
    ("readarr", |b| {
        let _ = readarr::parse(b);
    }),
    ("reaper", |b| {
        let _ = reaper::parse(b);
    }),
    ("rebarconfig", |b| {
        let _ = rebarconfig::parse(b);
    }),
    ("recbin", |b| {
        let _ = recbin::parse(b);
    }),
    ("redpen", |b| {
        let _ = redpen::parse(b);
    }),
    ("releaserc", |b| {
        let _ = releaserc::parse(b);
    }),
    ("refind", |b| {
        let _ = refind::parse(b);
    }),
    ("regf", |b| {
        let _ = regf::parse(b);
    }),
    ("reiserfs", |b| {
        let _ = reiserfs::parse(b);
    }),
    ("relaxng", |b| {
        let _ = relaxng::parse(b);
    }),
    ("releaseplease", |b| {
        let _ = releaseplease::parse(b);
    }),
    ("remminaconf", |b| {
        let _ = remminaconf::parse(b);
    }),
    ("renderconf", |b| {
        let _ = renderconf::parse(b);
    }),
    ("renovate", |b| {
        let _ = renovate::parse(b);
    }),
    ("renviron", |b| {
        let _ = renviron::parse(b);
    }),
    ("res", |b| {
        let _ = res::parse(b);
    }),
    ("resp", |b| {
        let _ = resp::parse(b);
    }),
    ("resx", |b| {
        let _ = resx::parse(b);
    }),
    ("retroarch", |b| {
        let _ = retroarch::parse(b);
    }),
    ("reviveconf", |b| {
        let _ = reviveconf::parse(b);
    }),
    ("revlog", |b| {
        let _ = revlog::parse(b);
    }),
    ("rf64", |b| {
        let _ = rf64::parse(b);
    }),
    ("rfa", |b| {
        let _ = rfa::parse(b);
    }),
    ("rfb", |b| {
        let _ = rfb::parse(b);
    }),
    ("rinex", |b| {
        let _ = rinex::parse(b);
    }),
    ("rip", |b| {
        let _ = rip::parse(b);
    }),
    ("ris", |b| {
        let _ = ris::parse(b);
    }),
    ("rkhunter", |b| {
        let _ = rkhunter::parse(b);
    }),
    ("rm", |b| {
        let _ = rm::parse(b);
    }),
    ("rocketmq", |b| {
        let _ = rocketmq::parse(b);
    }),
    ("rockspec", |b| {
        let _ = rockspec::parse(b);
    }),
    ("roff", |b| {
        let _ = roff::parse(b);
    }),
    ("roq", |b| {
        let _ = roq::parse(b);
    }),
    ("rosbag", |b| {
        let _ = rosbag::parse(b);
    }),
    ("rpcs3conf", |b| {
        let _ = rpcs3conf::parse(b);
    }),
    ("sabnzbd", |b| {
        let _ = sabnzbd::parse(b);
    }),
    ("samhainconf", |b| {
        let _ = samhainconf::parse(b);
    }),
    ("rpgmakerconf", |b| {
        let _ = rpgmakerconf::parse(b);
    }),
    ("rpm", |b| {
        let _ = rpm::parse(b);
    }),
    ("rprofile", |b| {
        let _ = rprofile::parse(b);
    }),
    ("rss2email", |b| {
        let _ = rss2email::parse(b);
    }),
    ("rst", |b| {
        let _ = rst::parse(b);
    }),
    ("rsyslogd", |b| {
        let _ = rsyslogd::parse(b);
    }),
    ("rtcp", |b| {
        let _ = rtcp::parse(b);
    }),
    ("rtp", |b| {
        let _ = rtp::parse(b);
    }),
    ("rtsp", |b| {
        let _ = rtsp::parse(b);
    }),
    ("rw2", |b| {
        let _ = rw2::parse(b);
    }),
    ("rx2", |b| {
        let _ = rx2::parse(b);
    }),
    ("s3m", |b| {
        let _ = s3m::parse(b);
    }),
    ("s7", |b| {
        let _ = s7::parse(b);
    }),
    ("s98", |b| {
        let _ = s98::parse(b);
    }),
    ("sac", |b| {
        let _ = sac::parse(b);
    }),
    ("safetensors", |b| {
        let _ = safetensors::parse(b);
    }),
    ("saif", |b| {
        let _ = saif::parse(b);
    }),
    ("scummvm", |b| {
        let _ = scummvm::parse(b);
    }),
    ("sdrppconf", |b| {
        let _ = sdrppconf::parse(b);
    }),
    ("samba", |b| {
        let _ = samba::parse(b);
    }),
    ("saml", |b| {
        let _ = saml::parse(b);
    }),
    ("sarif", |b| {
        let _ = sarif::parse(b);
    }),
    ("sas7bdat", |b| {
        let _ = sas7bdat::parse(b);
    }),
    ("sav", |b| {
        let _ = sav::parse(b);
    }),
    ("sbf", |b| {
        let _ = sbf::parse(b);
    }),
    ("sbi", |b| {
        let _ = sbi::parse(b);
    }),
    ("sbus", |b| {
        let _ = sbus::parse(b);
    }),
    ("sbv", |b| {
        let _ = sbv::parse(b);
    }),
    ("sby", |b| {
        let _ = sby::parse(b);
    }),
    ("scandata", |b| {
        let _ = scandata::parse(b);
    }),
    ("scc", |b| {
        let _ = scc::parse(b);
    }),
    ("sch", |b| {
        let _ = sch::parse(b);
    }),
    ("scl", |b| {
        let _ = scl::parse(b);
    }),
    ("scp", |b| {
        let _ = scp::parse(b);
    }),
    ("sdc", |b| {
        let _ = sdc::parse(b);
    }),
    ("sddmconf", |b| {
        let _ = sddmconf::parse(b);
    }),
    ("sdkconfig", |b| {
        let _ = sdkconfig::parse(b);
    }),
    ("segy", |b| {
        let _ = segy::parse(b);
    }),
    ("selinuxfc", |b| {
        let _ = selinuxfc::parse(b);
    }),
    ("selinuxte", |b| {
        let _ = selinuxte::parse(b);
    }),
    ("sequelizerc", |b| {
        let _ = sequelizerc::parse(b);
    }),
    ("serilog", |b| {
        let _ = serilog::parse(b);
    }),
    ("sevendtdxml", |b| {
        let _ = sevendtdxml::parse(b);
    }),
    ("sf2", |b| {
        let _ = sf2::parse(b);
    }),
    ("sfc", |b| {
        let _ = sfc::parse(b);
    }),
    ("sfd", |b| {
        let _ = sfd::parse(b);
    }),
    ("sflow", |b| {
        let _ = sflow::parse(b);
    }),
    ("sftp", |b| {
        let _ = sftp::parse(b);
    }),
    ("sfv", |b| {
        let _ = sfv::parse(b);
    }),
    ("sgi", |b| {
        let _ = sgi::parse(b);
    }),
    ("shadow", |b| {
        let _ = shadow::parse(b);
    }),
    ("shard", |b| {
        let _ = shard::parse(b);
    }),
    ("shellcheckrc", |b| {
        let _ = shellcheckrc::parse(b);
    }),
    ("shorewall", |b| {
        let _ = shorewall::parse(b);
    }),
    ("shp", |b| {
        let _ = shp::parse(b);
    }),
    ("sievescript", |b| {
        let _ = sievescript::parse(b);
    }),
    ("sigma", |b| {
        let _ = sigma::parse(b);
    }),
    ("singerconf", |b| {
        let _ = singerconf::parse(b);
    }),
    ("sip", |b| {
        let _ = sip::parse(b);
    }),
    ("sixel", |b| {
        let _ = sixel::parse(b);
    }),
    ("skp", |b| {
        let _ = skp::parse(b);
    }),
    ("slob", |b| {
        let _ = slob::parse(b);
    }),
    ("slrnconf", |b| {
        let _ = slrnconf::parse(b);
    }),
    ("slsa", |b| {
        let _ = slsa::parse(b);
    }),
    ("smb2", |b| {
        let _ = smb2::parse(b);
    }),
    ("smd", |b| {
        let _ = smd::parse(b);
    }),
    ("smi", |b| {
        let _ = smi::parse(b);
    }),
    ("smithy", |b| {
        let _ = smithy::parse(b);
    }),
    ("smt2", |b| {
        let _ = smt2::parse(b);
    }),
    ("smtp", |b| {
        let _ = smtp::parse(b);
    }),
    ("smtpdconf", |b| {
        let _ = smtpdconf::parse(b);
    }),
    ("snap", |b| {
        let _ = snap::parse(b);
    }),
    ("snappy", |b| {
        let _ = snappy::parse(b);
    }),
    ("sndh", |b| {
        let _ = sndh::parse(b);
    }),
    ("snmp", |b| {
        let _ = snmp::parse(b);
    }),
    ("snoop", |b| {
        let _ = snoop::parse(b);
    }),
    ("snort", |b| {
        let _ = snort::parse(b);
    }),
    ("socks", |b| {
        let _ = socks::parse(b);
    }),
    ("sourcemap", |b| {
        let _ = sourcemap::parse(b);
    }),
    ("sp3", |b| {
        let _ = sp3::parse(b);
    }),
    ("sparql", |b| {
        let _ = sparql::parse(b);
    }),
    ("sparse", |b| {
        let _ = sparse::parse(b);
    }),
    ("spc", |b| {
        let _ = spc::parse(b);
    }),
    ("spec", |b| {
        let _ = spec::parse(b);
    }),
    ("ssmtpconf", |b| {
        let _ = ssmtpconf::parse(b);
    }),
    ("srcdscfg", |b| {
        let _ = srcdscfg::parse(b);
    }),
    ("strongswanconf", |b| {
        let _ = strongswanconf::parse(b);
    }),
    ("spef", |b| {
        let _ = spef::parse(b);
    }),
    ("spicenet", |b| {
        let _ = spicenet::parse(b);
    }),
    ("spv", |b| {
        let _ = spv::parse(b);
    }),
    ("sqitchconf", |b| {
        let _ = sqitchconf::parse(b);
    }),
    ("sqlite", |b| {
        let _ = sqlite::parse(b);
    }),
    ("srm", |b| {
        let _ = srm::parse(b);
    }),
    ("sshkey", |b| {
        let _ = sshkey::parse(b);
    }),
    ("sssdconf", |b| {
        let _ = sssdconf::parse(b);
    }),
    ("sst", |b| {
        let _ = sst::parse(b);
    }),
    ("stardict", |b| {
        let _ = stardict::parse(b);
    }),
    ("staticcheckconf", |b| {
        let _ = staticcheckconf::parse(b);
    }),
    ("statsd", |b| {
        let _ = statsd::parse(b);
    }),
    ("step", |b| {
        let _ = step::parse(b);
    }),
    ("stix", |b| {
        let _ = stix::parse(b);
    }),
    ("stl", |b| {
        let _ = stl::parse(b);
    }),
    ("stm", |b| {
        let _ = stm::parse(b);
    }),
    ("stockholm", |b| {
        let _ = stockholm::parse(b);
    }),
    ("stp", |b| {
        let _ = stp::parse(b);
    }),
    ("strings", |b| {
        let _ = strings::parse(b);
    }),
    ("studio3", |b| {
        let _ = studio3::parse(b);
    }),
    ("stun", |b| {
        let _ = stun::parse(b);
    }),
    ("su", |b| {
        let _ = su::parse(b);
    }),
    ("su2", |b| {
        let _ = su2::parse(b);
    }),
    ("sudoku", |b| {
        let _ = sudoku::parse(b);
    }),
    ("suiconf", |b| {
        let _ = suiconf::parse(b);
    }),
    ("suricata", |b| {
        let _ = suricata::parse(b);
    }),
    ("svf", |b| {
        let _ = svf::parse(b);
    }),
    ("svg", |b| {
        let _ = svg::parse(b);
    }),
    ("svndump", |b| {
        let _ = svndump::parse(b);
    }),
    ("swf", |b| {
        let _ = swf::parse(b);
    }),
    ("swiftmt", |b| {
        let _ = swiftmt::parse(b);
    }),
    ("sylk", |b| {
        let _ = sylk::parse(b);
    }),
    ("synapse", |b| {
        let _ = synapse::parse(b);
    }),
    ("sysctlconf", |b| {
        let _ = sysctlconf::parse(b);
    }),
    ("syslog", |b| {
        let _ = syslog::parse(b);
    }),
    ("sysmonconf", |b| {
        let _ = sysmonconf::parse(b);
    }),
    ("systemd", |b| {
        let _ = systemd::parse(b);
    }),
    ("systemdboot", |b| {
        let _ = systemdboot::parse(b);
    }),
    ("sysv", |b| {
        let _ = sysv::parse(b);
    }),
    ("syx", |b| {
        let _ = syx::parse(b);
    }),
    ("t3d", |b| {
        let _ = t3d::parse(b);
    }),
    ("tabbyconf", |b| {
        let _ = tabbyconf::parse(b);
    }),
    ("tacacs", |b| {
        let _ = tacacs::parse(b);
    }),
    ("terrariaconf", |b| {
        let _ = terrariaconf::parse(b);
    }),
    ("tilestacheconf", |b| {
        let _ = tilestacheconf::parse(b);
    }),
    ("tarantool", |b| {
        let _ = tarantool::parse(b);
    }),
    ("taskfile", |b| {
        let _ = taskfile::parse(b);
    }),
    ("tcp", |b| {
        let _ = tcp::parse(b);
    }),
    ("tcx", |b| {
        let _ = tcx::parse(b);
    }),
    ("td0", |b| {
        let _ = td0::parse(b);
    }),
    ("tekton", |b| {
        let _ = tekton::parse(b);
    }),
    ("torrc", |b| {
        let _ = torrc::parse(b);
    }),
    ("tdm", |b| {
        let _ = tdm::parse(b);
    }),
    ("trojanconf", |b| {
        let _ = trojanconf::parse(b);
    }),
    ("tripwireconf", |b| {
        let _ = tripwireconf::parse(b);
    }),
    ("tds", |b| {
        let _ = tds::parse(b);
    }),
    ("telnet", |b| {
        let _ = telnet::parse(b);
    }),
    ("tempoconf", |b| {
        let _ = tempoconf::parse(b);
    }),
    ("terminfo", |b| {
        let _ = terminfo::parse(b);
    }),
    ("texinfo", |b| {
        let _ = texinfo::parse(b);
    }),
    ("textile", |b| {
        let _ = textile::parse(b);
    }),
    ("ts3serverini", |b| {
        let _ = ts3serverini::parse(b);
    }),
    ("tuicconf", |b| {
        let _ = tuicconf::parse(b);
    }),
    ("textlint", |b| {
        let _ = textlint::parse(b);
    }),
    ("tflite", |b| {
        let _ = tflite::parse(b);
    }),
    ("tfm", |b| {
        let _ = tfm::parse(b);
    }),
    ("tftp", |b| {
        let _ = tftp::parse(b);
    }),
    ("thanosconf", |b| {
        let _ = thanosconf::parse(b);
    }),
    ("threemf", |b| {
        let _ = threemf::parse(b);
    }),
    ("thrift", |b| {
        let _ = thrift::parse(b);
    }),
    ("tiff", |b| {
        let _ = tiff::parse(b);
    }),
    ("tileservergl", |b| {
        let _ = tileservergl::parse(b);
    }),
    ("timesyncd", |b| {
        let _ = timesyncd::parse(b);
    }),
    ("tlp", |b| {
        let _ = tlp::parse(b);
    }),
    ("tmpfilesd", |b| {
        let _ = tmpfilesd::parse(b);
    }),
    ("tmx", |b| {
        let _ = tmx::parse(b);
    }),
    ("topojson", |b| {
        let _ = topojson::parse(b);
    }),
    ("torrent", |b| {
        let _ = torrent::parse(b);
    }),
    ("tptp", |b| {
        let _ = tptp::parse(b);
    }),
    ("travisci", |b| {
        let _ = travisci::parse(b);
    }),
    ("trx", |b| {
        let _ = trx::parse(b);
    }),
    ("ts", |b| {
        let _ = ts::parse(b);
    }),
    ("tsx", |b| {
        let _ = tsx::parse(b);
    }),
    ("tta", |b| {
        let _ = tta::parse(b);
    }),
    ("ttc", |b| {
        let _ = ttc::parse(b);
    }),
    ("ttf", |b| {
        let _ = ttf::parse(b);
    }),
    ("ttml", |b| {
        let _ = ttml::parse(b);
    }),
    ("ttyrec", |b| {
        let _ = ttyrec::parse(b);
    }),
    ("txt2tags", |b| {
        let _ = txt2tags::parse(b);
    }),
    ("typeid", |b| {
        let _ = typeid::parse(b);
    }),
    ("typeormconf", |b| {
        let _ = typeormconf::parse(b);
    }),
    ("tzif", |b| {
        let _ = tzif::parse(b);
    }),
    ("tzx", |b| {
        let _ = tzx::parse(b);
    }),
    ("uasset", |b| {
        let _ = uasset::parse(b);
    }),
    ("ubi", |b| {
        let _ = ubi::parse(b);
    }),
    ("ubootenv", |b| {
        let _ = ubootenv::parse(b);
    }),
    ("v2rayconf", |b| {
        let _ = v2rayconf::parse(b);
    }),
    ("ubx", |b| {
        let _ = ubx::parse(b);
    }),
    ("ucf", |b| {
        let _ = ucf::parse(b);
    }),
    ("udevrules", |b| {
        let _ = udevrules::parse(b);
    }),
    ("udf", |b| {
        let _ = udf::parse(b);
    }),
    ("udp", |b| {
        let _ = udp::parse(b);
    }),
    ("ufs", |b| {
        let _ = ufs::parse(b);
    }),
    ("ufwrules", |b| {
        let _ = ufwrules::parse(b);
    }),
    ("uimage", |b| {
        let _ = uimage::parse(b);
    }),
    ("ult", |b| {
        let _ = ult::parse(b);
    }),
    ("unityfs", |b| {
        let _ = unityfs::parse(b);
    }),
    ("unitymanifest", |b| {
        let _ = unitymanifest::parse(b);
    }),
    ("unitysettings", |b| {
        let _ = unitysettings::parse(b);
    }),
    ("unrealircd", |b| {
        let _ = unrealircd::parse(b);
    }),
    ("unv", |b| {
        let _ = unv::parse(b);
    }),
    ("upc", |b| {
        let _ = upc::parse(b);
    }),
    ("upf", |b| {
        let _ = upf::parse(b);
    }),
    ("ups", |b| {
        let _ = ups::parse(b);
    }),
    ("urdf", |b| {
        let _ = urdf::parse(b);
    }),
    ("urlencode", |b| {
        let _ = urlencode::parse(b);
    }),
    ("vrtgdal", |b| {
        let _ = vrtgdal::parse(b);
    }),
    ("usercss", |b| {
        let _ = usercss::parse(b);
    }),
    ("userscript", |b| {
        let _ = userscript::parse(b);
    }),
    ("usf", |b| {
        let _ = usf::parse(b);
    }),
    ("usi", |b| {
        let _ = usi::parse(b);
    }),
    ("utmp", |b| {
        let _ = utmp::parse(b);
    }),
    ("velero", |b| {
        let _ = velero::parse(b);
    }),
    ("uue", |b| {
        let _ = uue::parse(b);
    }),
    ("vale", |b| {
        let _ = vale::parse(b);
    }),
    ("valkeyconf", |b| {
        let _ = valkeyconf::parse(b);
    }),
    ("vcard", |b| {
        let _ = vcard::parse(b);
    }),
    ("vcd", |b| {
        let _ = vcd::parse(b);
    }),
    ("vdi", |b| {
        let _ = vdi::parse(b);
    }),
    ("vercelconf", |b| {
        let _ = vercelconf::parse(b);
    }),
    ("verilog", |b| {
        let _ = verilog::parse(b);
    }),
    ("vernemq", |b| {
        let _ = vernemq::parse(b);
    }),
    ("vf", |b| {
        let _ = vf::parse(b);
    }),
    ("vgm", |b| {
        let _ = vgm::parse(b);
    }),
    ("vhd", |b| {
        let _ = vhd::parse(b);
    }),
    ("vhdx", |b| {
        let _ = vhdx::parse(b);
    }),
    ("vip", |b| {
        let _ = vip::parse(b);
    }),
    ("vlt", |b| {
        let _ = vlt::parse(b);
    }),
    ("vmagentconf", |b| {
        let _ = vmagentconf::parse(b);
    }),
    ("vms", |b| {
        let _ = vms::parse(b);
    }),
    ("voc", |b| {
        let _ = voc::parse(b);
    }),
    ("vox", |b| {
        let _ = vox::parse(b);
    }),
    ("vp3", |b| {
        let _ = vp3::parse(b);
    }),
    ("wktproj", |b| {
        let _ = wktproj::parse(b);
    }),
    ("vpk", |b| {
        let _ = vpk::parse(b);
    }),
    ("vrrp", |b| {
        let _ = vrrp::parse(b);
    }),
    ("vsdx", |b| {
        let _ = vsdx::parse(b);
    }),
    ("vtf", |b| {
        let _ = vtf::parse(b);
    }),
    ("vtk", |b| {
        let _ = vtk::parse(b);
    }),
    ("wsjtxconf", |b| {
        let _ = wsjtxconf::parse(b);
    }),
    ("vtu", |b| {
        let _ = vtu::parse(b);
    }),
    ("vxlan", |b| {
        let _ = vxlan::parse(b);
    }),
    ("w64", |b| {
        let _ = w64::parse(b);
    }),
    ("wad", |b| {
        let _ = wad::parse(b);
    }),
    ("werf", |b| {
        let _ = werf::parse(b);
    }),
    ("wasm", |b| {
        let _ = wasm::parse(b);
    }),
    ("wav", |b| {
        let _ = wav::parse(b);
    }),
    ("webfinger", |b| {
        let _ = webfinger::parse(b);
    }),
    ("webloc", |b| {
        let _ = webloc::parse(b);
    }),
    ("webmanifest", |b| {
        let _ = webmanifest::parse(b);
    }),
    ("webp", |b| {
        let _ = webp::parse(b);
    }),
    ("weechat", |b| {
        let _ = weechat::parse(b);
    }),
    ("westconf", |b| {
        let _ = westconf::parse(b);
    }),
    ("westonconf", |b| {
        let _ = westonconf::parse(b);
    }),
    ("weztermconf", |b| {
        let _ = weztermconf::parse(b);
    }),
    ("wfn", |b| {
        let _ = wfn::parse(b);
    }),
    ("wgsl", |b| {
        let _ = wgsl::parse(b);
    }),
    ("widgetxml", |b| {
        let _ = widgetxml::parse(b);
    }),
    ("wim", |b| {
        let _ = wim::parse(b);
    }),
    ("windowsterminal", |b| {
        let _ = windowsterminal::parse(b);
    }),
    ("winstonconf", |b| {
        let _ = winstonconf::parse(b);
    }),
    ("wiresharkpref", |b| {
        let _ = wiresharkpref::parse(b);
    }),
    ("wkb", |b| {
        let _ = wkb::parse(b);
    }),
    ("woff", |b| {
        let _ = woff::parse(b);
    }),
    ("woff2", |b| {
        let _ = woff2::parse(b);
    }),
    ("woodpecker", |b| {
        let _ = woodpecker::parse(b);
    }),
    ("woz", |b| {
        let _ = woz::parse(b);
    }),
    ("wpasupplicant", |b| {
        let _ = wpasupplicant::parse(b);
    }),
    ("wrl", |b| {
        let _ = wrl::parse(b);
    }),
    ("wsdl", |b| {
        let _ = wsdl::parse(b);
    }),
    ("wv", |b| {
        let _ = wv::parse(b);
    }),
    ("x3d", |b| {
        let _ = x3d::parse(b);
    }),
    ("x509", |b| {
        let _ = x509::parse(b);
    }),
    ("x7z", |b| {
        let _ = x7z::parse(b);
    }),
    ("xacro", |b| {
        let _ = xacro::parse(b);
    }),
    ("xapi", |b| {
        let _ = xapi::parse(b);
    }),
    ("xar", |b| {
        let _ = xar::parse(b);
    }),
    ("xbrl", |b| {
        let _ = xbrl::parse(b);
    }),
    ("xcf", |b| {
        let _ = xcf::parse(b);
    }),
    ("xdc", |b| {
        let _ = xdc::parse(b);
    }),
    ("xfs", |b| {
        let _ = xfs::parse(b);
    }),
    ("xi", |b| {
        let _ = xi::parse(b);
    }),
    ("xinetdconf", |b| {
        let _ = xinetdconf::parse(b);
    }),
    ("yggdrasil", |b| {
        let _ = yggdrasil::parse(b);
    }),
    ("xib", |b| {
        let _ = xib::parse(b);
    }),
    ("xid", |b| {
        let _ = xid::parse(b);
    }),
    ("xliff", |b| {
        let _ = xliff::parse(b);
    }),
    ("zathurarc", |b| {
        let _ = zathurarc::parse(b);
    }),
    ("zeekconf", |b| {
        let _ = zeekconf::parse(b);
    }),
    ("xlink", |b| {
        let _ = xlink::parse(b);
    }),
    ("xlsx", |b| {
        let _ = xlsx::parse(b);
    }),
    ("xm", |b| {
        let _ = xm::parse(b);
    }),
    ("xmodmap", |b| {
        let _ = xmodmap::parse(b);
    }),
    ("xmp", |b| {
        let _ = xmp::parse(b);
    }),
    ("xmpp", |b| {
        let _ = xmpp::parse(b);
    }),
    ("xnb", |b| {
        let _ = xnb::parse(b);
    }),
    ("xorgconf", |b| {
        let _ = xorgconf::parse(b);
    }),
    ("xpath", |b| {
        let _ = xpath::parse(b);
    }),
    ("xpm", |b| {
        let _ = xpm::parse(b);
    }),
    ("xps", |b| {
        let _ = xps::parse(b);
    }),
    ("xpt", |b| {
        let _ = xpt::parse(b);
    }),
    ("xq", |b| {
        let _ = xq::parse(b);
    }),
    ("xqf", |b| {
        let _ = xqf::parse(b);
    }),
    ("xrdpconf", |b| {
        let _ = xrdpconf::parse(b);
    }),
    ("xresources", |b| {
        let _ = xresources::parse(b);
    }),
    ("xsd", |b| {
        let _ = xsd::parse(b);
    }),
    ("xslt", |b| {
        let _ = xslt::parse(b);
    }),
    ("xspf", |b| {
        let _ = xspf::parse(b);
    }),
    ("xsvf", |b| {
        let _ = xsvf::parse(b);
    }),
    ("xyz", |b| {
        let _ = xyz::parse(b);
    }),
    ("xz", |b| {
        let _ = xz::parse(b);
    }),
    ("y4m", |b| {
        let _ = y4m::parse(b);
    }),
    ("yamllint", |b| {
        let _ = yamllint::parse(b);
    }),
    ("yara", |b| {
        let _ = yara::parse(b);
    }),
    ("yenc", |b| {
        let _ = yenc::parse(b);
    }),
    ("yosys", |b| {
        let _ = yosys::parse(b);
    }),
    ("yuzuconf", |b| {
        let _ = yuzuconf::parse(b);
    }),
    ("z64", |b| {
        let _ = z64::parse(b);
    }),
    ("zapconf", |b| {
        let _ = zapconf::parse(b);
    }),
    ("zeekctl", |b| {
        let _ = zeekctl::parse(b);
    }),
    ("zeekscript", |b| {
        let _ = zeekscript::parse(b);
    }),
    ("zfs", |b| {
        let _ = zfs::parse(b);
    }),
    ("zlib", |b| {
        let _ = zlib::parse(b);
    }),
    ("znc", |b| {
        let _ = znc::parse(b);
    }),
    ("zoo", |b| {
        let _ = zoo::parse(b);
    }),
    ("zpaq", |b| {
        let _ = zpaq::parse(b);
    }),
    ("zpl", |b| {
        let _ = zpl::parse(b);
    }),
    ("zstd", |b| {
        let _ = zstd::parse(b);
    }),
    ("zulipconf", |b| {
        let _ = zulipconf::parse(b);
    }),
];
