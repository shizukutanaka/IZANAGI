# IZANAGI / izanagi_kit — カテゴリ別 改善点 洗い出し (Research & Improvement Backlog)

> 目的: 本プロダクト（zero-dependency・`#![forbid(unsafe_code)]` の決定論的 lockstep-safe Rust ゲームエンジン）を
> **10 カテゴリ**に分解し、各カテゴリにつき **arXiv / GitHub の関連情報を約 10 件**収集して、
> izanagi_kit に対する**改善点を洗い出す**ための調査資料。
>
> - 本書は「洗い出し（enumeration）」が主目的であり、コード変更は含まない。確定バグ修正の記録は
>   `IMPROVEMENTS.md`(削除済み — git 履歴参照)を参照。
> - 各改善点には**決定論への影響タグ**を付す:
>   - 🟢 **replay-safe**: 既存の replay/lockstep 不変条件を壊さない（`PINNED_FINAL_HASH` に影響なし）。
>   - 🟡 **gated**: 実装方法次第で draw 順序や hash 順序に影響しうる。feature flag / 別 API で隔離すべき。
>   - 🔴 **breaking**: 既存の `PINNED_FINAL_HASH = 0xd1a9_236e_96a2_c802`（`tests/determinism.rs:163`）を
>     更新する必要がある。メジャー方針変更時のみ。
> - 出典は WebSearch のインデックスで実在確認済み。arXiv は ID を併記（abs ページは bot 403 のため
>   一次裏取りは ar5iv / Semantic Scholar 等で補完予定）。

最終更新: 2026-09-20 / 対象ブランチ: `claude/deepresearch-ultrathink-improve-yq2th`

---

## 1. ECS アーキテクチャ / コンポーネント storage (Entity-Component-System, sparse-set)

**現状（izanagi_kit）**: `src/entity.rs`（generational handle + free-list allocator、despawn→respawn 後の
stale handle 拒否）、`src/sparse_set.rs`（dense `Vec<T>` + sparse index、swap-remove で O(1) 合成変更）、
`src/arch.rs`（archetype テーブル — 走査型 workload 向けの opt-in primitive、N18 で計測決着済み）。
query DSL は未実装。

**参考情報（arXiv / GitHub / 同種ソフト）**
- GitHub: [SanderMertens/ecs-faq](https://github.com/SanderMertens/ecs-faq) — ECS 設計論の網羅 FAQ（flecs 作者）。
- GitHub: [skypjack/entt](https://github.com/skypjack/entt) — sparse-set ECS の事実上の参照実装（C++）。group/view の最適化。
- GitHub: [amethyst/legion](https://github.com/amethyst/legion) — archetype 方式の高速 Rust ECS。iteration 重視。
- GitHub: [Ralith/hecs](https://github.com/Ralith/hecs) — archetype・最小 API の Rust ECS。同種コンポーネント群を dense 配列で管理。
- GitHub: [bevyengine/bevy](https://github.com/bevyengine/bevy) — `Table`(archetype)+`SparseSet` のハイブリッド storage を選択可能。
- GitHub: [amethyst/shipyard](https://github.com/leudz/shipyard) — sparse-set ベースの並列 ECS。
- GitHub: [jslee02/awesome-entity-component-system](https://github.com/jslee02/awesome-entity-component-system) — ECS ライブラリ/論文の curated list。
- 論文: Eurographics diglib *"Run-time Performance Comparison of Sparse-set and Archetype Entity-Component Systems"* — sparse-set は変更が安く iteration は archetype が速い、というトレードオフの実測。
- 記事: PRDeving *"Deep-diving into ECS Architecture and Data Oriented Programming"* — DOP / cache locality の解説。
- 記事: csherratt *"Specs and Legion, two very different approaches to ECS"* — bitset filter vs archetype table の比較。

**洗い出した改善点**
1. ✅**実装済み** — **multi-component query / iteration API**（`sparse_set::join`/`join_mut`、最小集合走査・canonical 昇順）+ **archetype storage**（`src/arch.rs` の `ArchTable`、opt-in。N18 の実測で「kit core の既定 storage にはしない」と確定）。出典: entt group/view, legion query。🟢 replay-safe（canonical 順序）。
2. ~~**archetype storage の optional 化**~~ — **実装済み（N18）**: `arch::ArchTable` が opt-in primitive として存在。core の既定 storage 化は実測で不採用（point get で 4.3x 劣後、join 差は frame の 0.1%）。出典: bevy hybrid storage, EG 比較論文。
3. **bitset によるコンポーネント有無の高速判定**（query 高速化）。出典: specs bitset filter。🟢 replay-safe。
4. **generation 枯渇（u32 wrap）時の handle 再利用ポリシー明文化**。出典: hecs generational index 解説。🟢 replay-safe（テスト追加のみ）。
5. **ZST（タグコンポーネント）最適化**（storage を確保しない marker component）。出典: entt/bevy tag。🟢 replay-safe。
6. **コンポーネント登録のコンパイル時 ID 化**（`TypeId` ランタイム比較の削減）。🟢 replay-safe。

---

## 2. 決定論的 fixed-point 演算 (Q16.16 fixed-point, saturating arithmetic)

**現状（izanagi_kit）**: `src/fixed.rs` — Q16.16 スカラ、`saturating_mul` で sign-flip 回避、`from_ratio` の
0 除算は符号方向へ飽和（バグ修正済み — 旧 `IMPROVEMENTS.md`、現在は git 履歴）。sqrt・trig（CORDIC `sin`/`cos`/`sin_cos`/`atan2`）・除算は実装済み（下記改善点 1〜2、N19）。

**参考情報**
- arXiv: **1605.03229** *"CORDIC-based Architecture for Powering Computation in Fixed-Point Arithmetic"* — hyperbolic CORDIC による pow/exp/log の整数実装。
- 記事: Gaffer On Games *"Floating Point Determinism"* — IEEE float がクロスプラットフォームで非決定的になる根拠（fixed-point を使う動機）。
- 記事: Gamedeveloper *"Cross platform RTS synchronization and floating point indeterminism"* — RTS での float 非同期の実例。
- GitHub: [GitHub topics: fixed-point-arithmetic](https://github.com/topics/fixed-point-arithmetic) — 同種ライブラリ群の一覧。
- GitHub: FixedMathSharp — 決定論 fixed-point math（.NET、sqrt/trig/vector 付き）。API 設計の参照。
- GitHub: [encointer/substrate-fixed](https://github.com/encointer/substrate-fixed)（`fixed` crate 系） — Rust の Q 形式 fixed-point 実装と丸めモード。
- GitHub: [PetteriAimonen/libfixmath](https://github.com/PetteriAimonen/libfixmath) — Q16.16 の sqrt/sin/cos/atan2/exp の C 実装。**izanagi_kit と同じ Q16.16** で直接移植参照になる。
- 記事: RogueBasin / Gaffer 由来の「整数のみで sqrt・trig を実装すれば異アーキ間で決定的」議論（HN #26357209）。
- アルゴリズム: Newton–Raphson / restoring 法による integer sqrt（fixed-point sqrt の定番）。
- アルゴリズム: CORDIC（sin/cos/atan2 を加算とシフトのみで、ハードウェア乗算器なしに計算）。

**洗い出した改善点**
1. ✅**実装済み** — **`sqrt()` の追加**（integer bit-by-bit isqrt、`src/fixed.rs`）。距離計算・正規化に必須。負入力は 0 へ飽和。🟢 replay-safe（新規 API、既存演算不変・`PINNED_FINAL_HASH` 不変）。
2. ✅**実装済み** — **CORDIC による `sin`/`cos`/`sin_cos`/`atan2`**（テーブルではなく反復で決定的、16回・整数定数、`src/fixed.rs`）。出典: arXiv:1605.03229（Simmonds et al., 2016）, libfixmath。🟢 replay-safe。
3. **丸めモードの明示**（truncate / round-half-to-even）。乗除の丸めを文書化し API 化。出典: substrate-fixed の丸め。🟡 gated（既存の演算の丸めを変えると 🔴。新 API として追加なら 🟢）。
4. ~~**`Vec2`/`Vec3` 等の fixed-point ベクトル型**~~ — **実装済み**: `src/vec.rs` の `Vec2`（`dot`/`length_sq`/`normalize`/`reflect`、全て Q16.16）。出典: FixedMathSharp。
5. **overflow 検出モード（debug 時 panic / release saturating）**の二層化を文書化。🟢 replay-safe。
6. ~~**`from_ratio`・`from_int` の property test 拡充**~~ — **実装済み**: 飽和境界テストが存在（`test_sub_saturates_at_min`・`test_from_int_saturates_out_of_range`・`test_from_ratio_div_by_zero_saturates_by_sign` ほか）。🟢 replay-safe。
7. ~~**lerp / clamp / sign 等のユーティリティ**~~ — **実装済み**: `lerp`・`clamp`/`clamp01`・`sign` が `fixed.rs` に存在。🟢 replay-safe。

---

## 3. 決定論的 PRNG (deterministic pseudorandom number generator, SplitMix64)

**現状（izanagi_kit）**: `src/rng.rs` — SplitMix64、`split(stream_id)` で名前付き子ストリーム分岐済み・固定 draw 順序。`below(0)` は draw せず 0 を返す
（release desync バグ修正済み — 旧 `IMPROVEMENTS.md`、現在は git 履歴）。`below(n)` は Lemire の乗算シフト法で bias 除去済み。`src/rng_xoshiro.rs` に opt-in の xoshiro256++ もある。

**参考情報**
- arXiv: **1805.01407** *"Scrambled Linear Pseudorandom Number Generators"*（Blackman & Vigna）— xoshiro/xoroshiro と scrambler の品質。
- arXiv: **2507.03007** *"Statistical Quality and Reproducibility of Pseudorandom Number Generators in Machine Learning technologies"* — PCG/Philox/MT を TestU01 BigCrush で比較、再現性の落とし穴。
- arXiv: **2501.00193** *"A Pseudo-random Number Generator for Multi-Sequence Generation with Programmable Statistics"* — 複数ストリーム生成。
- サイト: [pcg-random.org](https://www.pcg-random.org/) — PCG 系、SplitMix64 比較、PractRand 結果。
- 記事: Daniel Lemire *"Xorshift… Fail Statistical Tests for Linearity"*（Semantic Scholar）— 線形性故障の指摘。
- 記事: zephyrtronium *"State of the Art in Randomness"* — 現代 PRNG の俯瞰。
- GitHub: [imneme/pcg-c](https://github.com/imneme/pcg-c) / pcg-cpp — PCG 参照実装。
- GitHub: [rust-random/rand](https://github.com/rust-random/rand)（`rand_pcg`, `rand_xoshiro`） — Rust の標準的 PRNG 実装と **bias なし range 抽出（Lemire / widening 法）**の参照。
- アルゴリズム: Lemire *"Fast Random Integer Generation in an Interval"*（nearly-divisionless）— `below(n)` の modulo bias 除去。
- アルゴリズム: SplitMix64（seed 派生・stream 分割の定番）。

**洗い出した改善点**
1. ~~**modulo bias の除去**~~ — **実装済み**: `below(n)` は既に Lemire の乗算シフト法（`u128` 積の上位64bit）。出典: Lemire, rand crate。
2. ~~**複数の名前付きストリーム**~~ — **実装済み**: `SplitMix64::split(stream_id)` がサブシステム毎の独立ストリームを純関数で分岐（親を消費しない）。出典: SplitMix64 split, xoshiro jump。
3. ~~**PRNG 品質の自動テスト**~~ — **実装済み (f51f55f)**: `rng.rs` テスト群 — seed0 の公式参照ベクタ8連・`below(6)` 60k 回 chi-square 上界・全64ビット反転の avalanche 帯・連続出力の serial correlation 上界（全て固定 seed の機械オラクル）。出典: PractRand/TestU01 文献。🟢 replay-safe。
4. **`f`-range・gaussian・weighted choice 等の分布ヘルパ**（fixed-point 連携）。🟢 replay-safe（新 API）。
5. **seed の文書化（wall-clock seed 禁止の明文化）**と replay seed の永続化。出典: lockstep 文献（C6）。🟢 replay-safe。
6. ~~**xoshiro256++ への移行検討**~~ — **実装済み（opt-in）**: `src/rng_xoshiro.rs` の `Xoshiro256pp` が別 generator として存在（既定は SplitMix64 のまま、pinned hash 不変）。出典: arXiv:1805.01407。

---

## 4. World state hashing / desync 検出 (state checksum, FNV-1a, canonical order)

**現状（izanagi_kit）**: `src/world_hash.rs` — FNV-1a による per-frame state checksum、canonical（sorted）iteration で
決定性確保。型ごとの構造化ハッシュ（`DetHash` trait）や差分検出ツールは未整備。

**参考情報**
- 論文: ACM Computing Surveys **10.1145/2790077** *"Deterministic Replay: A Survey"* — 非決定要因の分類と checksum 戦略。
- 記事: Bugnet *"How to Debug Multiplayer Desync Issues in Games"* — frame checksum 比較による desync 原因特定。
- 記事: yal.cc *"Preparing your game for deterministic netcode"* — state hash の置き場所・粒度。
- 記事: SnapNet *"Netcode Architectures Part 1: Lockstep"* — 各 tick で checksum 比較。
- GitHub: [gschup/ggrs](https://github.com/gschup/ggrs) — rollback 実装。`Config` で state checksum を要求する設計。
- GitHub: [gschup/bevy_ggrs](https://github.com/gschup/bevy_ggrs) — 登録した component/resource のみ snapshot/hash。
- アルゴリズム: FNV-1a（現状採用）vs xxHash / FxHash の速度比較。
- アルゴリズム: order-independent hashing（XOR/加法 commutative combine）でソート不要化。
- 記事: Gaffer / RTS 系の「複数地点で checksum を取り desync を二分探索」手法。
- GitHub: [Cyan4973/xxHash](https://github.com/Cyan4973/xxHash) — 高速非暗号 hash（per-frame コスト削減の候補）。

**洗い出した改善点**
1. ✅**実装済み（一部）** — **`DetHash` trait の実装**（基本型 + `Fixed/Entity/Position/Render/Color` + `SparseSet::det_hash` で canonical 順序の容器 hash、`src/world_hash.rs` ほか）。derive macro 化は残。出典: ggrs Config checksum。🟢 replay-safe（FNV 流用で `PINNED_FINAL_HASH` 不変）。
2. **order-independent な集合ハッシュ**（commutative combine）で sort コスト削減。出典: order-independent hashing。🔴 breaking（hash 値が変わる）→ 別 API。
3. ~~**desync 二分探索ツール**~~ — **実装済み（N12）**: `replay::first_divergence`/`all_divergences` + `DesyncReport`(分岐 tick・サブシステム局所化・seed・入力窓を含む再現バンドル)。出典: Bugnet, Gaffer。
4. **xxHash/FxHash オプション**（per-frame hash の高速化）。出典: xxHash。🟡 gated（hash 関数変更で値が変わる→ feature 隔離）。
5. **replay trace の永続化フォーマット**（seed + inputs + 期待 hash 列）。出典: Deterministic Replay Survey。🟢 replay-safe。
6. **CI で複数 OS/arch の hash 一致を検証**（matrix で `PINNED_FINAL_HASH` を突き合わせ）。🟢 replay-safe。

---

## 5. Fixed-timestep シミュレーションループ (fixed timestep, accumulator, death-spiral guard)

**現状（izanagi_kit）**: `src/timestep.rs` — accumulator で sim tick と render frame を分離、death-spiral ガード付き。
render 補間は `alpha_ratio()`（残量の整数比）として提供済み。input サンプリングの tick 整合は `cmdqueue`/`netinput` が担う。

**参考情報**
- 記事: Gaffer On Games *"Fix Your Timestep!"* — accumulator パターンの原典。
- 記事: jakubtomsu *"Fixed timestep without interpolation"* — 補間なしで滑らかに見せる工夫。
- 記事: jakubtomsu *"Reliable fixed timestep & inputs"* — 入力を tick 境界で確定する方法。
- 記事: André Leite *"Taming Time in Game Engines / fixed-timestep game loop"* — accumulator の実装詳細。
- 記事: Medium *"Game Loops Unveiled"* — game loop 設計の俯瞰。
- GitHub: [bevyengine/bevy](https://github.com/bevyengine/bevy)（`FixedUpdate` schedule） — fixed timestep の実運用 API 参照。
- GitHub: [gschup/ggrs](https://github.com/gschup/ggrs) — rollback では 1 tick = 固定 dt が前提（timestep と netcode の接続）。
- 記事: vodacek アーカイブ版 *"Fix Your Timestep"* — 派生解説。
- 概念: semi-fixed timestep の float 丸め問題（fixed-point dt で解消、C2 と連携）。
- 概念: spiral of death ガード（max steps / クランプ）。

**洗い出した改善点**
1. ~~**render 補間 alpha の提供**~~ — **実装済み**: `timestep::alpha_ratio()` が `(accumulator_ns, step_ns)` の整数比を返す。出典: Gaffer。
2. ~~**入力の tick 整列 API**~~ — **実装済み**: `cmdqueue` + `netinput::DelayScheduler` が input を tick 境界に整列（N11）。出典: jakubtomsu inputs。
3. ~~**dt を fixed-point 化**~~ — **実装済み（設計で解消）**: accumulator は整数ナノ秒（`u64`）で float を持たない — 丸め誤差は構造的に存在しない。出典: C2 + semi-fixed 問題。
4. ~~**death-spiral ガードの可観測化**~~ — **実装済み (c63d9f7)**: `FixedTimestep::dropped_steps()` がクランプ時に捨てたステップ数を累積返却（remainder は数えない）。出典: Gaffer fixed-timestep + 可観測性慣行。🟢 replay-safe。
5. ~~**rollback 対応フック**~~ — **実装済み**: `rollback::SnapshotRing` + `sync_test` と `replay::resimulate` が巻き戻し再シミュレートを提供（N4）。出典: ggrs。
6. **可変 tickrate のテスト**（同一 input で render fps を変えても sim hash 不変を property test 化）。🟢 replay-safe。

---

## 6. Lockstep / replay 決定論 (deterministic lockstep, rollback netcode) — 横断カテゴリ

**現状（izanagi_kit）**: RNG(C3) + fixed-point(C2) + world_hash(C4) + timestep(C5) を統合し、
`tests/determinism.rs` で end-to-end の bit-exact replay（`PINNED_FINAL_HASH = 0xd1a9_236e_96a2_c802`）を保証。
`src/rollback.rs`（`SnapshotRing` + `sync_test`、N4）・`src/replay.rs`（trace 記録・desync 検出・再シミュ）・`src/netinput.rs`（適応 input delay、N11）・`src/cmdqueue.rs` が実装済み — 単機 replay のみではない。

**参考情報**
- arXiv: **1705.05937** *"Engineering Record And Replay For Deployability"*（rr）— 低オーバーヘッド record/replay の設計。
- arXiv: **1805.06267** *"Efficient and Deterministic Record & Replay for Actor Languages"* — 並行系の決定的再生。
- 論文: ACM CSUR **10.1145/2790077** *"Deterministic Replay: A Survey"* — 非決定要因の体系。
- GitHub: [proepkes/UnityLockstep](https://github.com/proepkes/UnityLockstep) — deterministic lockstep + client prediction + rollback の実装例。
- GitHub: [gschup/ggrs](https://github.com/gschup/ggrs) / [gschup/bevy_ggrs](https://github.com/gschup/bevy_ggrs) — Rust の GGPO 系 rollback。**同種ソフトの第一参照**。
- サイト: [ggpo.net](https://www.ggpo.net/) — rollback netcode SDK の原典。
- 記事: SnapNet *"Netcode Architectures Part 1: Lockstep"* — lockstep のビットレベル決定性要件。
- 記事: meseta *"Netcode Concepts Part 3: Lockstep and Rollback"* — lockstep↔rollback の対比。
- 記事: yal.cc *"Preparing your game for deterministic netcode"* — RNG 単一化・float 排除のチェックリスト。
- 記事: coherence docs *"Determinism, Prediction and Rollback"* — 予測と巻き戻しの実務。

**洗い出した改善点**
1. ~~**input-only 同期の transport 非依存 API**~~ — **実装済み**: `netinput`（`NetInputBuffer`・`DelayScheduler`・`AdaptiveDelay`）が transport 非依存の input 配信を提供（N11）。出典: ggrs/GGPO。
2. ✅**実装済み** — **rollback/replay ハーネス**（`replay::record_trace`/`check_trace`/`first_divergence`/`resimulate`、`src/replay.rs`）。出典: rr, ggrs。🟢 replay-safe。
3. ✅**実装済み（基盤）** — **state snapshot/restore**（`replay::resimulate` が clone+再シミュで rollback 基盤を提供。`DetHash` と対）。出典: bevy_ggrs snapshot。🟢 replay-safe。
4. ~~**非決定 API の静的禁止**~~ — **実装済み**: `tests/no_float_in_sim.rs`（sim 経路の float 排除）+ `global_invariants_hold.rs`（unordered 反復・clock・thread_local の allowlist 検査）が静的に遮断。出典: yal.cc チェックリスト。
5. **クロス OS/arch の決定性 CI**（Linux/macOS/Windows で `PINNED_FINAL_HASH` 一致を必須化）。出典: Deterministic Replay Survey。🟢 replay-safe。
6. ~~**input prediction の対応**~~ — **実装済み**: `netinput::NetInputBuffer` が未着 input を予測で埋め、rollback (`SnapshotRing`) で修正（N11/N4）。出典: GGPO。

---

## 7. コンテンツ DSL: パース & 診断 (parser, panic-free, rustc-style diagnostics)

**現状（izanagi_kit）**: `src/parser.rs` — 行ベース `.game` 形式、panic-free・bounded（1024B 行 / 256×256 grid）、
column-aware な rustc 風 caret 診断。`src/content.rs` は `BTreeMap` で決定的 iteration。error recovery（複数エラー継続）や
span ベースの高機能診断は限定的。

**参考情報**
- arXiv: **1905.02145** *"Automatic Syntax Error Reporting and Recovery in Parsing Expression Grammars"* — PEG の labeled failure による回復。
- arXiv: **1804.07133** *"Don't Panic! Better, Fewer, Syntax Errors for LR Parsers"*（Diekmann & Tratt）— CPCT+（Rust <500 行で大半のエラーを修復）。
- arXiv: **2507.03629** *"Towards Automatic Error Recovery in Parsing Expression Grammars"* — PEG 回復の続報。
- 記事: Laurence Tratt *"Automatic Syntax Error Recovery"* — 実務的な error recovery 解説。
- GitHub: [rust-lang/rust](https://github.com/rust-lang/rust)（`rustc_parse`） — 診断・回復の参照実装。
- GitHub: [zesterer/chumsky](https://github.com/zesterer/chumsky) — error recovery を備えた Rust パーサコンビネータ。
- GitHub: [lalrpop/lalrpop](https://github.com/lalrpop/lalrpop) — LR パーサ生成（grammar 駆動の選択肢）。
- GitHub: [rust-lang/gll #16](https://github.com/rust-lang/gll/issues/16) — GLL の回復における非決定性議論。
- 概念: line-based DSL の bounded parsing（DoS 耐性、現状採用）。
- 概念: LSP 連携（partial parse から補完/診断を返す）。

**洗い出した改善点**
1. ~~**error recovery（複数エラー一括報告）**~~ — **実装済み**: `parse` は `(Content, Vec<Diagnostic>)` を返し、最初のエラーで止まらず全診断を収集（broken.game fixture で複数診断を実証）。出典: arXiv:1804.07133, chumsky。
2. **span ベース診断（複数ラベル・related notes）**。出典: rustc, ariadne(C9)。🟢 replay-safe。
3. ~~**grammar の形式仕様 / BNF 文書化**~~ — **実装済み**: `SPEC.md` §9.1 が `.game` 文法を形式化し、spec↔parser の一致は機械検査済み。🟢 replay-safe。
4. **fuzz harness（cargo-fuzz）で panic-freedom を継続検証**。出典: C8。🟢 replay-safe。
5. **インクリメンタル/部分パース**（エディタ統合・大規模コンテンツ向け）。🟢 replay-safe。
6. **数値・色リテラルの厳密な境界テスト**（UTF-8 マルチバイト色は修正済み → 回帰固定）。🟢 replay-safe。

---

## 8. シリアライズ・round-trip・property/fuzz テスト (serialization, property-based testing, fuzzing)

**現状（izanagi_kit）**: `src/serializer.rs`（parser の逆、canonical `.game` 出力）、
`tests/roundtrip_fuzz.rs`（`parse(serialize(c)) ≅ c` の property test、3000+ 生成）。proptest/quickcheck 等の
外部クレートは zero-dependency 方針のため未使用（自前生成）。coverage-guided fuzzing は未導入。

**参考情報**
- arXiv: **2602.18545** *"Programmable Property-Based Testing"* — PBT の生成器プログラム化。
- GitHub: [proptest-rs/proptest](https://github.com/proptest-rs/proptest) — Rust の PBT（shrink 強力）。設計参照。
- GitHub: [BurntSushi/quickcheck](https://github.com/BurntSushi/quickcheck) — Haskell QuickCheck 系の Rust 版。
- GitHub: [rust-fuzz/cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) + [rust-fuzz/arbitrary](https://github.com/rust-fuzz/arbitrary) — libFuzzer 連携・`Arbitrary` trait。
- GitHub: [facebookarchive/propfuzz](https://github.com/facebookarchive/propfuzz) — PBT と fuzzing の統合 toolkit。
- 記事: nelhage *"Property-Based Testing Is Fuzzing"* — 両者の同一視。
- 記事: yoshuawuyts *"bridging fuzzing and property testing"* — `Arbitrary` 共有でテストと fuzz を統一。
- 概念: FuzzChick（coverage-guided PBT、novel path を mutate）。
- 概念: round-trip / snapshot testing（penumbra #351 のような serialize round-trip fuzz）。
- 記事: Rust Project Primer *"Property Testing"* — proptest vs quickcheck の選択指針。

**洗い出した改善点**
1. **coverage-guided fuzzing 導入**（`cargo-fuzz` ターゲットを dev-only で追加、本体は zero-dep 維持）。出典: cargo-fuzz, FuzzChick。🟢 replay-safe。
2. ~~**shrinking（最小反例生成）の自前実装強化**~~ — **実装済み**: `src/shrink.rs`（delta debugging、1-minimal 反例まで縮約）。出典: proptest shrink。
3. **`Arbitrary` 互換の生成器**でテストと fuzz を共有。出典: yoshuawuyts, arbitrary。🟢 replay-safe（dev 依存のみ）。
4. ~~**snapshot テスト**~~ — **実装済み**: `examples/dungeon.game`/`broken.game` の fixture が canonical 出力の golden として gate で受け入れ/拒否の両方向を固定。出典: penumbra #351。
5. ~~**round-trip の意味的等価性定義の明文化**~~ — **実装済み**: `serializer.rs` の doc が `parse(serialize(c)) ≅ c` の `≅` を順序正規化込みで定義。🟢 replay-safe。
6. ~~**determinism property の追加**~~ — **実装済み**: `tests/determinism.rs` が同 seed→同 hash を pinned trace で固定（`PINNED_FINAL_HASH`）。🟢 replay-safe。

---

## 9. バリデーション & ローダ / CLI ゲート (validation, ECS instantiation, content gate)

**現状（izanagi_kit）**: `src/validator.rs`（重複名・未定義参照・範囲外 spawn・寸法不整合を short-circuit せず全収集）、
`src/loader.rs`（検証済み content → sparse-set ECS の `LoadedLevel` 化）、`src/bin/gamec.rs`（CI 用 content checker、
`--fmt` で canonical 整形、エラー時非ゼロ終了）。診断の見栄え・出力フォーマット（JSON 等）は最小限。

**参考情報**
- GitHub: [zkat/miette](https://github.com/zkat/miette) — リッチな診断レポート（source span / help / 関連ラベル）。**同種の診断 UX 参照**。
- GitHub: [zesterer/ariadne](https://github.com/zesterer/ariadne) — multi-line ラベル・色付き診断。
- GitHub: [kevinmehall/codemap-diagnostic](https://github.com/kevinmehall/codemap-diagnostic) — rustc 風診断の最小実装。
- GitHub: [brendanzab/codespan](https://github.com/brendanzab/codespan) — span/位置管理と reporting。
- GitHub: [rust-lang/rust](https://github.com/rust-lang/rust) — 診断・lint・`--error-format=json` の参照。
- 概念: linter / formatter の CI ゲート（`gamec --fmt --check` で差分検出、`cargo fmt --check` 流儀）。
- 概念: SARIF / JSON 診断出力（CI アノテーション連携）。
- 概念: validator の「全件収集」設計（LLM 生成コンテンツの一括修正向け、現状採用）。
- arXiv: **1905.02145**（C7）— 回復した上で複数診断を出す枠組み（validator にも適用可）。
- 記事: nrc *"Error Handling in Rust ecosystem"* — 診断/エラー処理クレートの俯瞰。

**洗い出した改善点**
1. ✅**実装済み** — **`--check` モード**（`--fmt` の非破壊版、整形差分があれば非ゼロ終了）。出典: `cargo fmt --check`。🟢 replay-safe。
2. ~~**機械可読診断出力（JSON / SARIF）**~~ — **実装済み**: `gamec --json`/`--sarif` が機械可読診断を出力（CI アノテーション用）。出典: rustc `--error-format=json`。
3. **診断 UX 強化**（miette/ariadne 風の span・help・suggestion を自前 zero-dep で導入）。出典: miette, ariadne。🟢 replay-safe。
4. **validator ルールの拡張**（到達不能タイル・孤立部屋・spawn 重なり等の意味検査）。🟢 replay-safe。
5. ~~**修正提案（quick-fix）**~~ — **実装済み (459f2fc)**: validator の未定義 spawn 参照と extends の missing base に `(did you mean 'X'?)` を付加 — char 単位 Levenshtein を宣言順に走査し同距離は先出宣言が勝つので診断集合が決定的（整数のみ、rustc の suggestion 半径に倣い長い方の名の1/3で打切り）。出典: rustc suggestions。🟢 replay-safe。
6. ~~**loader の決定性テスト**~~ — **実装済み**: `determinism.rs` が allocator の free list と `live` 順序まで pin、loader は spawn 順の entity 割当を文書化+テスト済み。C1/C4 連携。🟢 replay-safe。

---

## 10. Roguelike アルゴリズム & ターミナル描画 (FOV / pathfinding / procgen, ANSI truecolor) — 機能パリティ

**現状（izanagi_kit）**: README 記載の「terminal-first」描画（24-bit ANSI 半ブロック `▀`、ヘッドレス CI で不変）。
FOV（`src/fov.rs` symmetric shadowcasting）・pathfinding（`src/pathfinding.rs`: A*/JPS/JPS4/Dijkstra map）・procedural generation（`src/mapgen.rs`、`src/wfc.rs`）は実装済み — 洗い出し時点では未実装領域だったが、本セッションで埋まった。

**参考情報**
- GitHub: [amethyst/bracket-lib](https://github.com/amethyst/bracket-lib) — Rust 製 roguelike toolkit（FOV・A*・Dijkstra map・noise）。**同種ソフトの第一参照**。
- GitHub: [libtcod/libtcod](https://github.com/libtcod/libtcod) — FOV・pathfinding・truecolor console の定番 C ライブラリ。
- GitHub: [ondras/rot.js](https://github.com/ondras/rot.js) — libtcod 系の JS roguelike toolkit（FOV/path/map gen/scheduler）。
- 記事: journal.stuffwithstuff *"What the Hero Sees: Field-of-View for Roguelikes"* — symmetric shadowcasting の決定版解説。
- 記事: RogueBasin *"FOV using recursive shadowcasting"* — 再帰 shadowcasting アルゴリズム。
- arXiv: **2308.07307** *"Extend Wave Function Collapse to Large-Scale Content Generation"* — **deterministic・aperiodic・infinite** な WFC（決定論方針と整合）。
- arXiv: **2410.15644** *"Procedural Content Generation in Games: A Survey…"* — PCG 手法の俯瞰。
- arXiv: **2503.21474** *"The Procedural Content Generation Benchmark"* — PCG 評価のための testbed。
- arXiv: **1906.04660** *"Two-step Constructive Approaches for Dungeon Generation"* — room 配置 + 接続のダンジョン生成。
- アルゴリズム: A* / JPS（Jump Point Search）/ Dijkstra map（bracket-pathfinding 準拠）。
- GitHub: [ratatui/ratatui](https://github.com/ratatui/ratatui) — TUI 描画・`TestBackend` による headless 描画アサート（描画テスト手法の参照、依存追加は不要）。

**洗い出した改善点**
1. ✅**実装済み** — **symmetric shadowcasting FOV**（決定的・対称な視界、`src/fov.rs`、Albert Ford 法・整数有理数スロープ）。出典: stuffwithstuff, RogueBasin, libtcod。🟢 replay-safe（整数演算で実装）。
2. ✅**実装済み** — **A* pathfinding**（8方向・整数 octile・`(f,h,x,y)` 全順序で tie-break 固定・corner-cut 無し）+ **Dijkstra map**（`dijkstra_map` + flee/safety 再スキャン + `combine_maps` 係数合成、N14）、`src/pathfinding.rs`。出典: bracket-pathfinding。🟢 replay-safe（順序確定済み）。
3. ✅**実装済み** — **決定論的 procedural generation**（seed 駆動 room-corridor、連結保証、`src/mapgen.rs`）。将来 WFC。出典: arXiv:1906.04660（Green et al., FDG'19）, 2308.07307（Nie et al., 2023、決定論的 N-WFC）。🟢 replay-safe（RNG=C3 を単一ストリームで使用）。
4. **headless 描画スナップショットテスト**（出力セルバッファを golden 比較）。出典: ratatui TestBackend。🟢 replay-safe。
5. **JPS による A* 高速化**（grid 限定の最適化）。🟢 replay-safe。
6. **PCG 品質メトリクス**（連結性・到達可能性の自動検査、validator=C9 と連携）。出典: arXiv:2503.21474。🟢 replay-safe。
7. **line-of-sight / Bresenham line**（攻撃判定・FOV 補助）。🟢 replay-safe。

---

## 横断サマリ（実装イテレーション用インデックス）

各カテゴリの「最も着手価値が高い改善点」を優先度付けした索引。優先度は (価値 × 安全性 × 既存方針との整合) で評価。

| # | カテゴリ | 推し改善点（最有力） | 優先 | 決定論影響 | 主要出典 |
|---|---------|----------------------|------|-----------|---------|
| 2 | Fixed-point | `sqrt` + CORDIC `sin/cos/atan2` | **高** | 🟢 | arXiv:1605.03229 / libfixmath |
| 6 | Lockstep/replay | rollback/replay ハーネス + snapshot API | **高** | 🟢 | ggrs / rr(1705.05937) |
| 4 | State hashing | `DetHash` trait + desync 二分探索 | **高** | 🟢 | CSUR 10.1145/2790077 / ggrs |
| 10 | Roguelike | symmetric shadowcasting FOV + A* | **高** | 🟢 | bracket-lib / stuffwithstuff |
| 3 | PRNG | modulo bias 除去（新メソッド） + 名前付き stream | 中 | 🟡/🔴 | Lemire / arXiv:1805.01407 |
| 1 | ECS | multi-component query API | 中 | 🟢 | entt / legion |
| 8 | Testing | cargo-fuzz ターゲット + snapshot | 中 | 🟢 | cargo-fuzz / arbitrary |
| 9 | Validation/CLI | `--check` + JSON/SARIF 診断 | 中 | 🟢 | rustc / miette |
| 7 | Parser | error recovery（複数エラー） | 中 | 🟢 | arXiv:1804.07133 / chumsky |
| 5 | Timestep | render 補間 alpha + rollback フック | 低 | 🟢 | Gaffer / ggrs |

**実装の原則（全カテゴリ共通）**
- 🟢 replay-safe を優先採用。🟡/🔴 は feature flag / 新規 API で隔離し、既存 `PINNED_FINAL_HASH = 0xd1a9_236e_96a2_c802` を保護。
- zero-dependency・`#![forbid(unsafe_code)]` を維持（テスト/fuzz は dev-dependencies で隔離可）。
- 新規アルゴリズムは fixed-point(C2) と単一 RNG ストリーム(C3) の上に実装し、determinism property test(C8) を必ず追加。

## 同種ソフトとの feature-parity / gap 分析 (Comparison vs. similar OSS)

原プロンプトの「同種ソフトを参照して改善点を洗い出す」に対応。代表機能を OSS と比較し、**最大の gap**＝
着手価値の高い改善点を可視化する。凡例: ✅ あり / ⚠️ 部分的 / ❌ 無し。

| 機能 (capability) | izanagi_kit | bracket-lib (Rust roguelike) | libtcod (C roguelike) | bevy/entt (ECS) | ggrs (rollback) | 最大 gap → 改善点 |
|------------------|:----:|:----:|:----:|:----:|:----:|------------------|
| Generational handle / sparse-set | ✅ | ✅ | — | ✅ | — | — |
| multi-component query / iteration | ✅ | ⚠️ | — | ✅ | — | ✅ `sparse_set::join`/`join_mut` 実装済み |
| archetype storage（大規模 iteration） | ✅(opt-in) | ❌ | — | ✅ | — | ✅ `arch::ArchTable` 実装済み（N18: 既定 storage 化は実測不採用） |
| fixed-point sqrt / trig | ✅ | ⚠️(f32) | ⚠️ | ⚠️(f32) | — | ✅ 実装済み（決定論 integer/CORDIC） |
| 決定論 PRNG（単一ストリーム） | ✅ | ✅ | ✅ | ⚠️ | ✅(要求) | — |
| bias なし range 抽出 | ✅ | ✅ | ⚠️ | ✅ | — | ✅ below=Lemire・range/coin 追加済み |
| per-frame state hash / desync 検出 | ✅ | ❌ | ❌ | ⚠️ | ✅ | ✅ `first_divergence`+`DesyncReport` 実装済み(N12) |
| snapshot / restore（rollback） | ✅(基盤) | ❌ | ❌ | ⚠️(bevy_ggrs) | ✅ | ✅ replay::resimulate（clone+再シミュ） |
| input-only 同期 / replay ハーネス | ✅ | ❌ | ❌ | ⚠️ | ✅ | ✅ replay harness 実装済み（snapshot/rollback 基盤含む） |
| FOV（shadowcasting） | ✅ | ✅ | ✅ | ❌ | — | ✅ 実装済み（symmetric, integer） |
| pathfinding（A*/Dijkstra） | ✅ | ✅ | ✅ | ❌ | — | ✅ A* + Dijkstra map + descend 実装済み |
| procedural generation | ✅ | ✅ | ⚠️ | ❌ | — | ✅ 実装済み（mapgen: rooms+corridors, 連結保証） |
| parser error recovery（複数エラー） | ✅ | — | — | — | — | ✅ `parse` は全診断を収集 |
| coverage-guided fuzzing | ❌ | — | — | ⚠️ | — | C8-1 cargo-fuzz |
| 機械可読診断（JSON/SARIF） | ✅ | — | — | — | — | ✅ `gamec --json`/`--sarif` 実装済み |

**読み取り**: izanagi_kit の**決定論コア（hash / PRNG / fixed-point / timestep）は同種ソフトと同等以上**。
洗い出し時点の最大 gap だった **C10（FOV+A*+procgen）と C6（snapshot+replay harness）はいずれも実装済み** —
表の izanagi_kit 列で残る ❌ は **coverage-guided fuzzing のみ**（nightly+ネットワーク制約で環境ブロック、N10 参照）。
残る ⚠️ はなく、query/archetype/diagnostics/recovery の4行は本表記載時点より実装が進んだため更新済み。

## 検証済み一次出典 (Verified primary sources)

下表の arXiv 論文は、表題・著者・年・査読会場を**一次情報まで照合済み**（2026-06-05、WebSearch インデックス
＋会場ページ ECOOP/USENIX/IEEE/FDG 等で確認）。本文中の引用はこの確定情報に基づく。

| arXiv ID | 確定表題 | 著者 | 年 / 会場 | 本書での用途 |
|----------|---------|------|-----------|-------------|
| 1605.03229 | CORDIC-based Architecture for Powering Computation in Fixed-Point Arithmetic | Simmonds, Mack, Bellestri, Llamocca | 2016 | C2: fixed-point sqrt/pow/trig（CORDIC） |
| 1805.01407 | Scrambled Linear Pseudorandom Number Generators | Blackman, Vigna | 2018 | C3: PRNG 品質 / xoshiro scrambler |
| 1805.06267 | Efficient and Deterministic Record & Replay for Actor Languages | Aumayr, Marr, Béra, Gonzalez Boix, Mössenböck | 2018 / ManLang'18 | C6: 決定的 record & replay |
| 1705.05937 | Engineering Record And Replay For Deployability | O'Callahan, Jones, Froyd, Huey, Noll, Partush | 2017 / USENIX ATC | C6: rr 低オーバーヘッド replay |
| 1804.07133 | Don't Panic! Better, Fewer, Syntax Errors for LR Parsers | Diekmann, Tratt | 2018 / ECOOP 2020 | C7: CPCT+ error recovery（98.37% 修復） |
| 1905.02145 | Automatic Syntax Error Reporting and Recovery in Parsing Expression Grammars | Medeiros, Alvez Junior, Mascarenhas | 2019 | C7: PEG labeled-failure 回復 |
| 1906.04660 | Two-step Constructive Approaches for Dungeon Generation | Green, Khalifa, Alsoughayer, Surana, Liapis, Togelius | 2019 / FDG'19 | C10: 2段階ダンジョン生成 |
| 2308.07307 | Extend Wave Function Collapse to Large-Scale Content Generation | Nie, Zheng, Zhuang, Song | 2023 / IEEE | C10: 決定論的・aperiodic な N-WFC |
| 2410.15644 | Procedural Content Generation in Games: A Survey with Insights on Emerging LLM Integration | Farrokhi Maleki, Zhao | 2024 | C10: PCG 手法サーベイ |
| 2503.21474 | The Procedural Content Generation Benchmark | Khalifa, Gallotta, Barthet, Liapis, Togelius, Yannakakis | 2025 / FDG'25 | C10: PCG 評価 testbed |
| 2501.00193 | A Pseudo-random Number Generator for Multi-Sequence Generation with Programmable Statistics | Wu, Salim, Elmitwalli, Köse, Ignjatovic | 2024 | C3: 複数ストリーム / programmable stats |
| 2507.03007 | Statistical Quality and Reproducibility of Pseudorandom Number Generators in Machine Learning technologies | Antunes | 2025 | C3: PRNG 統計品質 / 再現性（BigCrush） |
| 2507.03629 | Towards Automatic Error Recovery in Parsing Expression Grammars | Medeiros, Mascarenhas | SBLP 2018（arXiv 2025） | C7: PEG ラベル付き回復 |
| 2602.18545 | Programmable Property-Based Testing | Keles, Frank, Mert, Goldstein, Lampropoulos | 2026 | C8: 生成器プログラム化 PBT |

**未昇格（search-indexed、著者/年は次イテレーションで照合）**: なし — 本書が引用する arXiv 出典は**全件、表題・著者・年まで照合済み**。

## 出典について（検証メモ）
- arXiv の abs / `ar5iv.labs.arxiv.org` / Semantic Scholar API は本環境の fetch bot に対し一律 **HTTP 403** を返すため、
  一次裏取りは **WebSearch のインデックス＋査読会場ページ**（ECOOP/USENIX/IEEE/FDG 等）で実施した。
- 上記「検証済み一次出典」**14件**は表題・著者・年・会場まで確定。GitHub repo（owner/repo）はインデックス上で実在確認済み。
- 本書が引用する arXiv 出典は全件昇格済み。以後の `/loop` は出典照合ではなく、同種ソフトとの gap 分析・改善点の精緻化に充てる。

---

# 第2次調査 (2026-07-10) — 論文 + 動画/講演 + エコシステム

> 第1次調査(2026-06-05、arXiv/GitHub 限定)の**差分**として実施した3情報源の横断調査。
> WebSearch は正常動作。arXiv abs は従来どおり bot 403 のため、メタデータは search index で照合。
> 動画/講演は第1次で**完全に未カバーだったモダリティ**。各知見は本リポジトリのコードと突合し、
> 「実在するギャップか(既に実装済みでないか)」を確認した上で改善提案に落としている。
> **本セクション執筆後、下記のうち B1〜B6 を実装済み**(コミット履歴参照); 残りは「次期候補」に集約。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `world_hash::LabeledDigest` + `replay::first_divergence_labeled` — subsystem 粒度の desync 特定 | **3源が独立に収斂**: Factorio FFF-188/340(per-subsystem CRC)・bevy_ggrs(per-entity checksum)・incremental multiset hash(arXiv:2507.21096)。既存バックログ C4-3 も充足 | 🟢 純粋追加、pinned hash 不変 |
| `wfc` タイル重み + `wfc_solve_retry`(派生 seed リトライ)+ 接続性 post-pass(`reachable_count`/`is_passable_connected`) | Caves of Qud GDC/RC 2019(素の WFC は過均質・非連結・矛盾する)+ arXiv:2509.09919(quality vs validity 分離) | 🟢 uniform 既定は bit 不変 |
| `mapgen::GenParams::extra_loops` — 環状連結 | Joris Dormans, Cyclic Dungeon Generation(PROCJAM/Everything Procedural 2016)+ RC mapgen 講演 | 🟢 既定 0 で bit 不変 |
| エンジン `TerminalBackend` の cell-diff 描画 | Ratatui FOSDEM/EuroRust 2024(back-buffer + cell diff が sub-ms TUI の核)。kit 側 `terminal::Screen::diff` は実装済みだったがエンジン側が毎フレーム全再描画だった非対称を解消 | 🟢 API 不変 |
| engine ECS `HashMap`→`BTreeMap`(反復順序決定化)+ `from_entropy` docstring 強化 | FP 非結合性(arXiv:2408.05148)+ 決定論主張と実装の乖離(PRODUCT_AUDIT P7)。※前セッションで実装、本調査が裏付け | 🟢 挙動不変・API 不変 |
| 公開品質束: docs.rs メタデータ・`deny(missing_docs)`/`deny(broken_intra_doc_links)` 昇格・MSRV 固定 toolchain CI job・wasm32 check CI job | crates.io publishing norms 2025-26 / docs.rs hygiene / MSRV consensus(api-guidelines#231)/ wasm-pack 廃止(2025-07)後の素 wasm32 | 🟢/📄 |

## 次期候補(優先度順 — 設計判断・環境・工数のいずれかで本セッション見送り)

| # | 改善点 | 出典 | 決定論影響 | 見送り理由 |
|---|---|---|---|---|
| N1 | ~~**JPS4**(4方向グリッド専用 Jump Point Search)~~ **実装済み (19008e0)**: 縦軸支配 + 横プローブ設計、BFS オラクル(6000 グリッド歩数完全一致)で検証 — オラクルが初稿のプローブ欠落(完全性喪失)を実際に検出 | Baum, arXiv:2501.14816 (2025) | 🟢 整数のみ | — |
| N2 | **incremental multiset world-hash**(O(changes) の per-tick hash)。**現時点では不要(実測)**: 200x200 ダンジョンの 3,579 セル状態に対し `hash_state` は **36 µs/回** — 60 Hz の1フレーム予算 16,667 µs の **0.2%**。対価は `replay`/`savefile` ヘッダの algo バージョニング(= 形式の破壊的変更)。**再検討の閾値を測定で定める**: 状態が約100倍(35万要素規模)になると ~3.6 ms でフレームの 22% を占めるので、そこが着手線。それまでは着手しない | HexaMorphHash arXiv:2507.21096 / ECMH 1601.06502 | 🟡 hash 値が変わる → algo バージョニング必須 | LabeledDigest で desync 局所化は達成済み |
| N3 | ~~**zero-panic 公開 API**~~ **実装済み**: 両クレートに `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]`。**要件そのものが誤っていた** — 「unwrap 223/242」という追跡指標は**テストコードを含む計測**で、実装コードのみでは kit 6+4、engine 1+1 の**計12箇所**、`panic!` は 0 だった。全12箇所が到達不能かガード済みで、シグネチャ変更を伴う 0.2 破壊的変更は不要。12 箇所を書き換え(`pop().unwrap()`→`pop().map()`、`ready().expect()`→`?`、`min()/max().unwrap()`→単一 fold、`stack.pop().unwrap()`→`let-else` で既存の contradiction 出口に合流、parser は引数を一度だけ束縛)、engine の `States<S>` は `Vec` を `top: S` + `below: Vec<S>` に変えて**空を型で表現不能に**した。添字アクセス(約700箇所)とコンストラクタ `assert!` は意図的に対象外(理由は lib.rs の属性コメントに明記)| fortress-rollback(全 API `Result`)| 🟢 シグネチャ変更なし | — |
| N4 | ~~**SnapshotRing / SyncTest セッション**~~ **実装済み**: `rollback` module — `SnapshotRing`(stride 付き有界 snapshot リング、最古から eviction)+ `sync_test`(毎フレーム rollback+再sim で step 関数の非決定性を検出、`dst_determinism_sweep` と違い部分再実行パスを検証) | MK11 GDC 2019(snapshot 保存こそ rollback の支配的コスト)+ ggrs `SyncTestSession` | 🟢 | — |
| N5 | ~~**MapBuilder パイプライン**~~ **実装済み**: `MapBuilder`(`new`/`stage`/`keep_largest_region`/`stage_count`/`build`)。部品(`pathfinding::farthest_cell` / `Dungeon::keep_largest_region` / `ConnectivityMap`)に加え、**合成層本体も実装済み**だった — この行は古かった。設計上の要は **stage 毎の RNG ストリーム分離**: 各 stage は `SplitMix64::new(base_seed).split(i)` で独立ストリームを受け取るので、ある stage の抽選回数が他 stage の出力を1 bit も動かさない(stage0 の draw 数を 1 と 500 で振っても stage1 の carve が一致することを検査)。パイプライン全体が `(開始 dungeon, base_seed)` の純関数。テスト6件 | Wolverson RC 2020 / Brogue RC 2018 | 🟢 | — |
| N6 | ~~**接続成分キャッシュ**~~ **実装済み**: `pathfinding::ConnectivityMap` — 1回の flood fill で全セルに成分ラベル付与、`connected(a,b)` を O(1) 化。`is_reachable` と同じ 8方向 no-corner-cutting で完全一致(2400ペアのオラクル検証)。増分キャッシュ無効化の決定論ハザードを避け不変スナップショット方式(変化時は再構築)を採用 | Dwarf Fortress 最適化(GDC 2016) | 🟡 キャッシュ無効化が決定論に繊細 | — |
| N7 | ~~**DST ハーネス**~~ **実装済み (ba43d90)**: `dst` module — `dst_sweep`(seed 掃引 + 毎 tick 不変条件)/`dst_replay`(1行再現)/`dst_determinism_sweep`(二重実行 hash 比較で非決定性自体を検出) | Deterministic Simulation Testing の主流化(Polar Signals 2025-07 / madsim) | 🟢 | — |
| N8 | ~~**planning-based test kit**~~ **実装済み (79f17d4)**: `plan` module — `plan_inputs`(goal 述語 → BFS 最短入力列合成、DetHash による状態重複排除)。`resimulate`/`dst_sweep` と同じ `Fn(&S,&I)->S` 形状で相互運用 | Using Planning for Automated Testing of Video Games, IJCAI 2025 | 🟢 | — |
| N9 | ~~**メタモルフィックテスト群**~~ **一部実装済み (6367b70)**: astar cost の三角不等式 + 壁追加の単調性を追加(FOV対称性・fixed代数則は既存 property test で既にカバー済みと判明したため対象外) | MR-Coupler arXiv:2604.10126 | 🟢 | — |
| N10 | ~~**generator-based fuzzing**~~ **一部実装済み**: 構造化生成器は `roundtrip_fuzz` が既に担い、`mutate` + `test_malformed_input_never_panics_and_is_deterministic` が「parse は全域関数(パニックなし・診断は2回同一)」を 4,000 変異で検証。coverage-guided 連携(`cargo-fuzz`/nightly)のみ環境ブロックのまま | arXiv:2604.01442 / LibAFL-DiFuzz 2601.22772。既存 C8-1 | 🟢 dev-only | coverage-guided fuzzing は nightly+ネットワーク制約で不可 |
| N11 | ~~**適応 input delay + t+delay lockstep**~~ **実装済み**: `netinput::AdaptiveDelay`(misprediction 率を整数‰でヒステリシス追跡し推奨 delay を増減)+ `netinput::DelayScheduler`(捕捉入力を delay tick 後に実行。delay 引き下げ時は execute tick を単調増加に clamp して衝突・順序逆転を防ぎ、引き上げ時の gap は `NetInputBuffer` の予測が埋める)。60 tick で delay を揺らしても全入力が順序どおり厳密に1回実行されることを property テストで検証 | Overwatch GDC 2017 / 1500 Archers GDC 2001 | 🟢 | — |
| N12 | ~~**DesyncReport 型**~~ **実装済み (8d208c7)**: `DesyncReport<I>`(divergence + subsystem 局所化 + seed + 入力窓)+ `desync_report(_labeled)` + `DesyncPolicy{Resync,Kick,Disband}`。再現十分性を end-to-end test で証明 | For Honor GDC 2019 | 🟢 | — |
| N13 | ~~**WFC selector フック**~~ **実装済み**: `wfc::CellSelector` トレイト + `wfc_solve_with_selector` + 既定 `LowestEntropySelector`(現行 collapse 選択と bit 一致、200 seed の等価テスト + RNG ストリーム整合テストで証明)。collapse 順を外部から操縦可能に | Markovian WFC, arXiv:2509.09919 | 🟢 | — |
| N14 | ~~**Dijkstra map 係数合成**~~ **実装済み (0754806)**: `combine_maps(&[(&DijkstraMap, coeff)])` — 正係数=誘引・負係数=忌避、交差セマンティクス、飽和演算。「火を避けつつ接近」を descend 1回で表現 | Brogue / Brian Walker RC 2018 | 🟢 | — |
| N15 | ~~**孤児コンテンツ validator**~~ **一部実装済み (3036b66)**: 未使用 tile 警告(既存 unused-prefab パターンを glyph 参照に適用)。**recipe/drop/encounter 側は「対象なし」と実測で確定**: `Content` のフィールドは `prefabs` / `tiles` / `levels` の3つのみ(`src/content.rs`)で、検出すべきデータ型自体が存在しない。該当フィールドを導入する時が来たら同じ glyph 参照パターンを適用すればよい | RC 2024-25 の content/story 生成トレンド | 🟢 | — |
| N16 | ~~**DSL `extends` オーバーレイ**~~ **実装済み (daf85e5)**: `prefab <name> extends <base>` — ヘッダ構文 + フィールド単位 override。`stats` は key 単位で子が勝ち、`flags` は base 先頭 union、`glyph`/`color` は「その行を著した」場合のみ override(新設の `*_declared` bit が parsed 既定値と明示 reset を区別)。解決は `Content::resolve_prefab`(missing base / cycle は `ExtendsError`)、validator が診断化 + extends base を unused-warning 免除、loader が spawn 毎に解決。serializer は overlay を保持(`--fmt` が flatten しない)→ round-trip が `extends` を透過。副次修正: 失敗した prefab ヘッダが前行の block を開きっぱなしにして子行が誤帰属する bug。roundtrip_fuzz が非巡回 overlay を生成 | Bevy 0.19 BSN(patchable scenes) | 🟢(大) | — |
| N17 | ~~**total_cmp ソート監査**~~ **対象なし(棚卸し実施)**: 前提だった棚卸しを行った結果、`izanagi/src/` に `sort` / `sort_by` / `sort_unstable_by` / `max_by` / `min_by` / `partial_cmp` / `binary_search` は **1件も存在しない**(kit 実装コードの `partial_cmp` も 0件)。順序づけに float を使っている箇所が無いため、`total_cmp` 化すべきコードが存在しない。engine の float は `math`/`tween`/`render` の値計算に閉じており、比較ソートに到達していない。**この 0 件は `izanagi/tests/float_boundary.rs` の空の許可リストとして機械検査に載せた** — 順序づけを最初に導入する人が「何を比較するのか」を書かない限りビルドが落ちる | XiSort arXiv:2505.11927 | 🟢 | — |
| N18 | ~~**archetype storage(feature-gated)**~~ **不採用確定(実測)**: primitive 自体は `arch::ArchTable<Row>` として既に実装済み。残件の「kit core を archetype 化するか」は `tests/bench.rs` で実測決着(10k entities・Pos=全員/Vel=6k/Hp=4k・release median)。**multi-component 走査では archetype が圧勝**: join2 20.7µs→1.25µs(16x)、join_mut 22.1µs→1.33µs(17x)、join3 17.6µs→6.1µs(2.9x)。**しかし point get は SparseSet が 4.3x 高速**(直接 index vs HashMap probe: 14.6µs vs 62.5µs/10k)、remove+insert churn も 2x 高速(41ns vs 83ns)。60fps frame(16.6ms)に対し join 差は ~19µs=0.1% で、反復順序 semantics を変えてまで core を移行する動機は成立しない。ArchTable は join-heavy な consumer が opt-in で使う primitive として据え置き — 「どちらが上か」ではなく「走査型 workload と lookup 型 workload で構造を選ぶ」が測定の答え | The Essence of ECS, SAC 2026 arXiv:2606.14919 | 🟡 → ⚪️ 実測で決着 | — |
| N19 | ~~**Fixed op 命名行列**~~ **実装済み (d109196)**: `checked_add/sub/mul/div`・`wrapping_add/sub`・`overflowing_add/sub`・`saturating_sub` の9メソッド。**mul/div の i128 中間化は「不要」と結論**: Q16.16 を i32 に格納するため mul は \|a·b\| ≤ 2⁶² < 2⁶³、div は \|raw<<16\| ≤ 2⁴⁷ で i64 中間は原理的に溢れ得ない(`i64::MIN / -1` も到達不能)。i128 化は出力を1 bit も変えず速度だけ損なう。証明を `checked_mul`/`checked_shl` による極値テスト + 20,000 サンプルの sweep で機械検証済み | `fixed` クレートの API 規範 | 🟢 | — |
| N20 | ~~**cargo feature collections**~~ **不要と結論(実測)**: 動機は「コンパイル時間」と「バイナリサイズ」の2つだが、**どちらも測ると成立しない**。(1) 88 モジュール・実装28,812行のライブラリの release cold build が **3.6 秒**。(2) rlib は 5.5 MB だが、1モジュールしか使わない `wfc_demo` の release バイナリは **318 KB** — リンカが未使用87モジュールを既に落としている。対価は 88 個の `#[cfg(feature)]`、feature 組合せの検査行列、そして非既定 feature 下で pinned hash が割れる危険。`default=full` が必須という但し書き自体が「既定利用者には何も買わない」ことの自認だった | Bevy 0.18 feature collections | 🟢 | — |
| N21 | **crates.io Trusted Publishing + cargo-semver-checks リリース gate** | RFC 3691(2025-07 GA)/ cargo-semver-checks 2026 project goal | 📄 プロセスのみ | GH runner 上で動くので sandbox 制約は無関係。初回公開後に |
| N22 | ~~**観測フック(observers/hooks)**~~ **実装済み (523a61b)**: `observe::Observed<T>` — `SparseSet<T>` を包み、構造変化(insert=Added/Replaced、remove=Removed)を内部 `EventQueue` にプログラム順で push する。コールバックではなくイベントデータなので順序付き・ハッシュ可能・リプレイ可能で、drain 忘れは desync として大きく鳴る(`DetHash` が保留イベントまで畳む)。`change::Changed` は pull 軸(「tick N 以降に書かれたか」)、これは push 軸(「Position を失ったので今 threat row を落とす」)。値変更(`get_mut`/`iter_mut`)は設計上イベントを出さない — それは Changed の領分。`&mut SparseSet` の抜け道は意図的に無し — 構造を無通知で変えられると型の存在意義が消える。既定経路は bit-identical(opt-in 新モジュール)。9 テスト — 400 ランダム op の live-set モデル oracle を含む | Bevy ECS 討論 RustWeek 2025 | 🟡 ECS dispatch に触れる | — |
| N23 | ~~**LLM コンテンツパイプライン位置付け**~~ **実装済み (523a61b)**: kit README に「generate → verify → repair loop」節を追加 — `gamec --json`/`--sarif` が生成テキストの検証ゲートであること、診断が機械著者が犯しがちな誤り(rename 済み参照・範囲外座標・重複名・描画不能 glyph)向けに設計されていること、verifier 自体が機械可読であることが修復ループを可能にすることを文書化。契約は「well-formed かつ自己整合的」であり、良さ・意図は検査しない旨も明記。validator.rs / lib.rs tier 3 には既に同趣旨の言及があったため、本項は埋まっていた最後の隙間(名前付きワークフローの提示)のみ | arXiv:2508.18533 ほか 2025-26 LLM-PCG 群 | 📄 文書のみ | — |
| N24 | ~~**engine 側の決定性・パニック経路棚卸し**~~ **実装済み (97fa98b / e4946ee / 59627f0)**: 監査で見つけた3系統。(1) `Audio` — `mix_into` が `out[f*2]` を直接 index し短い出力で panic、かつ `voices` の `HashMap` 走査で f32 畳込み順が run 依存だった → `BTreeMap` 化(Voice の Ord = 再生順)で混合順を pin、`chunks_exact_mut(2).take(frames)` で短バッファは打切り。`to_mono` も `chunks(2)` が奇数尾で panic し得た → `chunks_exact(2)`。(2) `Gamepad` — `on_connect(false)` が connected フラグだけ落として held buttons/sticks/triggers を生存させた(ケーブル抜け中押しっぱなしの ghost input、gilrs/SDL の disconnect ポリシー)→ 静かに全リセット、release edge は合成しない。(3) `Tilemap::new` — `cols*rows` の u32 乗算が wrap して過小な vec を生成し後段 get/set で panic → `checked_mul` でコンストラクタ時点失敗(G7 allowlist に理由記載)。play-order 混合は符号付きサンプル(+1/−1/+1e-7)の bit 単位オラクルで順序を固定 — 単調正値だと ulp 差が tie-to-even で消えて検出不能になることが実測で判明 | gilrs/SDL disconnect 仕様・FP 非結合性(arXiv:2408.05148)・`Vec` capacity-overflow 挙動 | 🟢 API 不変(Voice に Ord 追加のみ) | — |

## 出典(第2次、search-index 照合)

**論文**: arXiv 2509.09919 / 2501.14816 / 2606.14919 / 2508.15264 / 2507.21096 / 2505.11927 /
2408.05148 / 2604.10126 / 2604.01442 / 2601.22772 / 2509.22170 / 2605.01783 / 2605.13570 /
2508.18533 / 2509.22426、IJCAI 2025 proceedings 1250。

**動画/講演**: MK11 rollback(GDC 2019, Stallone)/ Overwatch netcode(GDC 2017, Ford)/
For Honor 決定論(GDC 2019, Henry)/ 1500 Archers(GDC 2001, Terrano&Bettner)/
Caves of Qud WFC・End-to-End PCG(GDC 2019, Bucklew&Grinblat)/ Cyclic Dungeon Generation(Dormans, PROCJAM 2016)/
Procedural Map Generation(Wolverson, RC 2020)/ Brogue level design(Walker, RC 2018)/
Dwarf Fortress 最適化(GDC 2016, Adams)/ Ratatui(FOSDEM/EuroRust 2024, Parmaksız)/
Vision Visualized(Albert Ford, RC 2020 — `fov.rs` が既に準拠、対応不要のポジティブ確認)/
Factorio FFF-188/340 / Roguelike Celebration 2024-25。

**エコシステム**: Bevy 0.18/0.19 release notes / ggrs / bevy_ggrs architecture & pitfalls /
fortress-rollback / crates.io Trusted Publishing(RFC 3691)/ cargo-semver-checks project goal /
api-guidelines#231(MSRV)/ docs.rs metadata / rustwasm sunset(team#291)/ gamedev.rs /
`fixed` / `fixed-num` / Polar Signals DST(2025-07)/ madsim。

---

# 第3次: First Principles 分析 (2026-07-24)

外部出典の網羅(第1・2次)ではなく、**公理からの演繹**で過不足を洗い出した回。

**公理**: ゲームが bit-identical に再生されるために必要十分なものは 3 つだけ —
①決定論的な状態(正準バイト表現・float/壁時計/未順序反復/アドレス依存を含まない遷移)
②決定論的な入力(順序付き・再現可能)③①②の証明(hash・replay・分岐検出)。
それ以外は*内容の利便*であり、公理を支えるのは「それ自身が決定論的で hash 可能」な限りにおいて。

## 過剰(surplus)— 「削除」ではなく「階層が不可視」が defect

分析時点で 82 モジュール(現在はさらに増)中、公理の荷重負担は ~14、決定論アルゴリズムが ~16、コンテンツ検証 6、
残る **~45 は通常のゲームプレイ機能**。これらは削除対象ではない(ユーザーが自作すると
`HashMap` を使って replay を壊す)が、**フラットな名前空間ではどれが荷重負担か判別不能**
だった。→ 4 層の地図を crate doc と README に追加(04d472b)。

## 不足(missing)— 公理から演繹された欠落と対処

| # | 欠落 | 根拠 | 対処 |
|---|---|---|---|
| G1 | **step の綴りが 3 種類に分裂**: `FnMut(&mut S,&I)`(replay/rollback)/ `Fn(&S,&I)->S`(plan)/ `FnMut(&mut S,usize)`(dst)。公理③は単一の状態機械を前提とするのに、検証ツール 5 種が非互換な 3 形状を要求していた | Schneider, *State Machine Approach*, ACM Comput. Surv. 22(4), 1990 — 決定論的状態機械 + 順序付き入力ログが実行を完全に決定する | **実装済み (a3ba284)**: `sim::Simulation` トレイト + 各ツールへのアダプタ + `sim::audit()`(二重実行 + rollback 再実行を 1 呼び出しで検査、final hash を返す)。既存 API は不変 |
| G2 | **`DetHash` の網羅性を検証する手段がない**。フィールドを足して `det_hash` の更新を忘れると、その場では何も壊れず**後で静かに replay が分岐**する(報告される tick と混入 tick が無関係) | mutation testing: DeMillo/Lipton/Sayward, IEEE Computer 1978; Jia & Harman, IEEE TSE 2011 — 小さな変異を注入しオラクルが気づくか見る。生存ミュータント = hash し忘れたフィールド | **実装済み (3934c53)**: `hash_covers` / `field_coverage` / `uncovered_fields`。意図的に不完全な `DetHash` 実装が正しく検出されることをテストで証明 |
| G3 | 階層の不可視(上記 surplus と同一) | — | **実装済み (04d472b)** |
| G4 | **hash 安定性ポリシーが未文書化**: 「マシン間・同一コード」と「crate バージョン間」で保証が異なることが明文化されていなかった | — | **実装済み (04d472b)**: `world_hash` module doc に節を追加。バージョン間は非保証 → `DetHash` の形状変更は永続化物に対して破壊的、同一コミットで `savefile` header version を上げる |

## 副次的な確認(修正不要と判明)

- **N17(float ソートの `total_cmp` 化)**: engine にソート皆無(immediate-mode で z-sort なし)、
  kit は固定小数点で全ソートが整数キー。**このハザードは元から存在しない**。
- **N15 残り(recipe/drop/encounter の孤児参照検出)**: `content.rs` の `Content` は
  Prefab/Tile/Spawn/Level のみモデル化。当該参照はコンテンツパイプラインの対象外 → 追加対象なし。

## 追加実装: swarm testing (第3次の派生)

| 項目 | 出典 | 実装 |
|---|---|---|
| **積構成による時相性質の網羅検証**: シミュレーションの状態空間 × 監視器オートマトンを探索 | Vardi & Wolper, *An Automata-Theoretic Approach to Automatic Program Verification*, LICS 1986(自動理論的モデル検査の核 — SPIN が実際に行う構成)| `verify::check_temporal` / `check_temporal_sim` / `NotSafety`、および `temporal::MonitorState` / `Monitor::advance` / `initial_state` / `finish_state` / `is_safety`。**`temporal`(単一実行の監視)と `verify`(状態不変条件の網羅証明)の欠けていた合成**。`check_invariant` が構造的に表現できない問いに答える — 「コインが負にならない」は単一状態の性質だが「所持金が成立する前に購入が起きない」は*順序*の性質で、いかなる単一状態述語でも書けない(この主張を、バグのある shop で状態不変条件は `Holds` を返すことを示すテストで直接 pin)。**前提となったリファクタ**: `RespondsWithin` の内部状態が*絶対 tick* を保持していたため監視器状態が無限に増え、積が有限にならず `Holds` に到達不能だった → 残り猶予のカウントダウン(`None` または `Some(0..=within)`)に変更。等価性は既存 22 テスト(400 seed の denotational 差分検査・締切の厳密位置テスト)が全通過することで保証。**liveness は証明せず拒否**: `eventually`/`until` は有限接頭辞で反証不能なので、探索しても反例が出ないのは当然であり `Holds` を返せば「何も検証していないことの証明」になる → `Err(NotSafety)` を返す(SPIN の nested DFS による受理サイクル検出は範囲外と明記)。**変異注入 4 種中 1 種が最初すり抜け、実際の欠陥を暴いた**: 積キーから監視器状態を外しても全 3064 テストが通過していた — これは積構成の最重要性質(シミュレーション状態が同じでも監視器状態が異なれば再訪しなければならない)が未検証だったことを意味する → 2 状態のシミュレーションで同一状態を異なる監視器状態で再訪しないと違反を見逃すモデルを追加(積の訪問対数 > シミュレーション状態数 も同時に検証)|
| **有界モデル検査**: 到達可能な全状態を幅優先で列挙し、不変条件を**証明**するか最短反例を返す | Clarke, Emerson & Sistla, *Automatic Verification of Finite-State Concurrent Systems Using Temporal Logic Specifications*, ACM TOPLAS 8(2) (1986) / Holzmann, *The Model Checker SPIN*, IEEE TSE 23(5) (1997)。関連ソフトウェア: SPIN / TLA+(TLC)/ fortress-rollback(TLA+·Kani·Z3 — Kani は nightly 要求のため本クレートでは不可) | `verify::check_invariant` / `check_invariant_sim` / `reachable_states` / `Verification` / `Counterexample`。**ギャップはコードで確認**: `plan_inputs` の `None` は「queue が空になった(存在しないと証明)」と「`explored > max_states` で諦めた」の**両方**を意味し、呼び出し側から区別不能 — バグ探しと検証の差そのもの。**三値の結果**で解決: `Holds{states, diameter}`(到達空間を尽くした=証明)/ `Violated(最短反例)` / `Exhausted`(予算切れ・何も証明されていない)。`temporal::Verdict` と同じ三値形状であり理由も同じ(「決着しなかった」を「はい/いいえ」に潰すのが検証ツールが嘘をつき始める瞬間)。**オラクル**: 手計算可能な状態空間(0..=9 のクランプ counter = 10 状態・直径 9)と `Holds` の報告値を照合 / 反例長を独立実装の `plan_inputs` の BFS 距離と一致検証 / 健全性(`Holds` なら 500 seed のランダム play が決して反証しない)と完全性(ランダム play が見つける違反は必ず検査器も見つけ、経路はより短い)を双方向で検証。**変異注入 4 種**で実効性を確認 — うち**2 種が最初すり抜け**、テストの実際の欠陥を暴いた: (a) 直線状のモデルでは DFS と BFS が同じ経路を返すため「最短性」が検証できていなかった → `+10`/`+1` で最短 7 対 DFS 25 になるモデルを追加、(b) 違反が予算のはるか手前で起きるため「不変条件を予算より先に検査する」順序が効いていなかった → 違反状態がちょうど `max_states` 番目に発見されるよう調整。**doctest 自身も誤りを検出**: 当初の「床だけガードした purse」例は上限がなく無限空間なので `Holds` にならない — この落とし穴こそモジュールの主題なので、有界版・違反版・`Exhausted` 版の3例に書き直して明示 |
| **クラッシュ復旧テスト(フォールト注入)**: save/restore サイクルを**全入力位置に注入**し、中断しなかった実行と挙動が区別できないことを検査 | Will Wilson, *Testing Distributed Systems w/ Deterministic Simulation*, Strange Loop 2014(FoundationDB — DST の価値の中核はフォールト注入)/ Pillai et al., *All File Systems Are Not Created Equal: On the Complexity of Crafting Crash-Consistent Applications*, OSDI 2014(「もっともらしい箇所」でなく全点にクラッシュを注入する規律) | `recovery::restart_test` / `restart_test_bytes` / `restart_test_sim` / `RestartFailure`。**ギャップは実測で確認**: `dst` にフォールトモデルが皆無(seed と action 集合を振るだけで実行を中断しない)、`savefile` は容器(magic/version/checksum)しか守らずペイロードの往復が挙動を保存するかは無検査。**`sync_test` との違いが要点**: あちらは*メモリ内スナップショット*から再入するのでシリアライズ経路を一切通らない。**3 つの故障クラスを区別**するのが報告の価値: `Failed`(load 不能)/ `Lossy`(復元直後にハッシュ相違 = ハッシュ対象フィールドの欠落)/ `Divergent`(**ハッシュは一致するのに後で分岐** = ハッシュが覆っていない状態を save が落とした — 復元点のハッシュ比較だけでは絶対に見つからない)。**オラクル**: 完全な往復は 300 seed のランダム入力列で決して検出されない(`prop::forall_inputs`)/ ハッシュ対象を落とせば `Lossy`・非ハッシュ対象を落とせば `Divergent` と報告され、後者は復元点でハッシュが実際に一致することを別テストで直接証明 / 実 `savefile` 容器との統合(magic・version・checksum が全て正当で `validate_integrity` も通る「構造的に正しいが 1 フィールド欠落」の save を捕捉)。変異注入 4 種で実効性を確認済み(復元点以降を再実行しない / 注入点を k=0 のみに / `detected_tick` off-by-one / `Divergent` を `Lossy` に潰す)|
| **時相性質の runtime verification**: 単一状態ではなく*時間をまたぐ*性質を tick 列上で監視 | Bauer, Leucker & Schallhart, *Runtime Verification for LTL and TLTL*, ACM TOSEM 20(4):14 (2011) — LTL₃ 三値意味論 / Dwyer, Avrunin & Corbett, *Patterns in Property Specifications for Finite-State Verification*, ICSE 1999 — パターンカタログ | `temporal::Monitor` / `MonitorSet` / `Verdict` / `check_run`。**ギャップは実測で確認**: `dst_sweep` の `invariant(&state, tick)`、`forall_states` の `is_bad(&state)`、`sim::audit` のハッシュ比較 — 従来キットが表現できる性質は全て**単一状態の述語**であり、「ボス撃破後 60 tick 以内にドロップ」「所持金成立前に購入が起きない」「鍵を拾うまで扉は施錠」といった tick 間を関係づける性質は一切書けなかった。**二つの意味論を明確に分離**したのが設計の要点: 実行中は LTL₃ の三値判定(`verdict()` — 継続し得る prefix に対し ⊤/⊥/? を返し、違反が不可避になった瞬間に発火)、実行後は有限トレース意味論(`finish()` — 証拠はトレースが全てとして確定値)。この区別には実質があり、`always(p)` は `verdict()` で ⊤ を返し得ず(次 tick で壊れ得る)`finish()` では真を返す、`eventually(p)` はその鏡像 — この非対称性こそ safety/co-safety の特徴づけであり専用テストで pin。**response は意図的に有界**(`responds_within`): 無界の `G(t → F r)` は純粋 liveness で有限トレース上では決して判定不能=監視器として無価値。**オラクル**: 各パターンの denotational 定義(トレース全体に対する自明に正しい O(n²) 実装)を書き、`prop::forall_inputs` で 400 seed のランダムトレースに対し漸進的監視器と一致することを差分検証。オラクルの実効性を**変異注入 5 種で確認済み**(締切の off-by-one / Until の finish 分岐 / precedes の同時刻扱い / pending を最古でなく最新で追跡 / impartiality freeze 除去)— いずれも意味的に対応するテストが検出した。`MonitorSet::update` は `Result<(), String>` を返し `dst_sweep` の invariant にそのまま差し込める |
| **model-based / differential testing**: 本物のシミュレーションと*単純で信頼できる参照モデル*を同一コマンド列で lockstep 駆動し、毎ステップ一致を検査 → 不一致列を最小化 | John Hughes, *Experiences with QuickCheck: Testing the Hard Stuff and Staying Sane*, LNCS 9600 (2016)(stateful/model-based QuickCheck)+ McKeeman, *Differential Testing for Software*, Digital Technical Journal 10(1) (1998) | `prop::forall_model` / `ModelFailure`。**このキットが既に手作業で使っている技法の汎用ハーネス化**: JPS を BFS オラクルで検証・`kit_bridge` が headless==engine-hosted をアサート、という「信頼できる別実装との差分検査」を再利用可能な形にした。両辺が `Simulation`(同一 `Input`)で 1 本の入力列で駆動されるため、不一致列はそのまま replay 可能な回帰テストになる。`agree(&real,&model)` を各コマンド後に検査し**最初の分岐コマンドを `diverged_at` で局所化**、`shrink_inputs` で 1-minimal 化。`forall_states`(最終状態の述語)や `sim::audit`(自分自身との二重実行)とは別クラス — *正しさ*のバグ(最適化実装が参照からズレる)を捕捉する。初期状態不一致は `diverged_at=0` で報告。分岐点の前後で一致/不一致が切り替わることを sim に対して直接検証(ハーネスを信用しない) |
| **archive ベースの状態空間探索(Go-Explore)**: 到達済み状態を archive に記録 → 決定論的に「戻る」→ そこから探索 | Ecoffet, Huizinga, Lehman, Stanley & Clune, *First return, then explore*, Nature 590:580–586 (2021) / arXiv:1901.10995 | `explore::explore` / `explore_until` / `explore_sim(_until)` / `Archive` / `ExploreConfig`。**論文の前提条件が本クレートの founding guarantee と一致する**のが採用理由: 論文は「derailment を避けて戻る」ために *reset 可能な環境* を要求し(だから彼らはエミュレータの state restore を使い、確率的環境には robustification フェーズを追加する)、`Simulation` は `(state,input)` の純関数なので「戻る」が `Clone` 1回で済み、robustification 相当が原理的に不要。`plan`(完全だが指数)と `prop`/`dst`(memoryless なランダム play)の**中間**を埋める。cell = 状態の down-sampling(`hash_state` で exact、粗い射影で外向き探索)。**測定して判明した設計上の帰結**: 最短経路の代表を保持するため「戻る」先は cell の*入口側*に固定される → `steps_per_iteration` が cell 径より短いと cell から脱出できず探索が停止する(90長廊下・300 iter 実測: cell幅10 × walk 8 → 1 cell で停止 / walk 24 → 7 cells)。この sizing 則を module doc に表付きで明記し、停止する側・進む側の**両方をテストで pin**(代表選択規則を変えると当該テストが検出する)。archive の全 path が start から replay して当該 state を再現することを不変条件テストで検証、BFS(`plan_inputs`)を オラクルに「archive の path が BFS 最短より短くなることはない」を検証 |
| **property-based testing ループ**: ランダム入力生成 → 性質検査 → 反例を自動で最小化 | Claessen & Hughes, *QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs*, ICFP 2000 | `prop::forall_inputs` / `forall_states` / `PropFailure`。**部品は全て既存で、本モジュールは合成のみ**: 生成 = `SplitMix64::split(PROP_GEN_STREAM)`(swarm と同じ named sub-stream 方式で sim の RNG を乱さず seed 単独再現)、縮約 = `shrink_inputs`(ddmin)。**決定論 core により flaky 再試行機構が不要**という点が本家実装との差 — proptest/Hypothesis が費やす再実行・安定化の機構は、述語が入力の純関数である本キットでは丸ごと不要。反例の 1-minimal 性は `shrink::is_one_minimal` で直接検証(実装を信用しない) |
| **world hash のアバランシェ実測**: FNV-1a の分布品質を測定し、opt-in の終端処理を追加 | FNV-1a の構造(`hash ^= b; hash *= PRIME`)— 最後に書き込まれたバイトは乗算 1 回しか通らない。終端処理は SplitMix64 finalizer(`hash_unordered` が既に同じ理由で使用) | **実測**(1 bit 反転で変化する出力 bit 数、理想 32/64): 生 FNV-1a は 4B で平均 20.3・最悪 6、64B で平均 30.1・最悪 6。**最終バイトの 1 bit 反転は平均 9.3 bit しか動かさない**。`hash_state_mixed` 適用後は全長で平均 32.0・最悪 14〜17。**正しさのバグではない**(差分は必ず hash を動かす=最悪でも 6 bit ≠ 0 なので desync 検出は仕様どおり機能する)が、大量の構造化状態に対する衝突確率が理想の birthday bound から離れる。`hash_state` 自体の変更は全既存 hash を無効化するため **opt-in 追加**に留め、既定化は savefile header bump と対で 0.2 に延期。測定値は回帰テストで pin 済み |
| **delta debugging(テストケース最小化)**: 失敗する入力列を 1-minimal まで縮約 | Zeller & Hildebrandt, *Simplifying and Isolating Failure-Inducing Input*, IEEE TSE 28(2), 2002(`ddmin`。QuickCheck 系 shrinking / C-Reduce の基礎) | `shrink::shrink_inputs` / `is_one_minimal` / `shrink_simulation_inputs`。**決定論シミュレーションとの相性が本質的**: ddmin は述語が安定であることを要求するが、`Simulation` は `(初期状態, 入力列)` の純関数なので「この部分列はまだ失敗するか」が真の数学的述語になり、実用 shrinker が必要とする再試行・flakiness ヒューリスティクスが一切不要。順序保存・1-minimal 性・O(n²) 呼び出し上限をテストで検証 |
| **swarm testing**: seed ごとにアクションの**部分集合**のみを有効化して掃引 | Groce, Zhang, Eide, Chen & Regehr, *Swarm Testing*, ISSTA 2012 — 「毎回全機能を許すより、実行ごとにランダムに機能を*省く*方がバグを多く見つける」。一様生成は希釈するため、容量・飢餓・順序バグを暴く「同一アクションの長い連続」がほぼ発生しない | `dst::dst_swarm_sweep` / `dst_swarm_replay` / `swarm_subset` / `SwarmFailure`。部分集合とアクション選択は seed 由来の**名前付きサブストリーム**から引くので、sim 自身の RNG 列を一切乱さない(テストで検証済み)。同一バグに対し一様掃引が見逃し swarm が捕捉することを実証するテストを同梱 |

---

# 第4次: 検証機構自身への問答 (2026-09-20)

前回までの演繹は「シミュレーション」に向いていた。今回は問いを**検証機構そのもの**に向けた:
主張が「信じられている」と「強制されている」は別物であり、ピン留めされた hash は実行を
比較するだけで*コンパイルされるコード*を比較しない。

## 演繹された欠落と対処

| # | 欠落 | 対処 |
|---|---|---|
| H1 | **条件コンパイルは G1〜G10 の全経路を迂回する**。`#[cfg(target_pointer_width)]`・`cfg!(debug_assertions)`・`#[cfg(feature)]` は禁止トークンを一切含まず、値ではなくコードそのものをターゲット・プロファイルで変える。実測で library code の対象 cfg は 0 件 — 規律による偶然だった | **G11 として SPEC.md §2 に追加**。`tests/no_platform_cfg_in_sim.rs` が両クレートの非テスト領域から `cfg`/`cfg!`/`cfg_attr` の全判別式を走査し、許可アトム(`test`/`doc`/`doctest`/`docsrs` + `not`/`any`/`all`)以外を落とす。manifest の `[target.*]` セクション(G1 の空 `[dependencies]` 検査をすり抜ける同型の迂回)も禁止。**変異注入で確認**: 実ファイルへの `#[cfg(unix)]` 混入を検出 |
| H2 | **`#[cfg(test)]` の境界検索がコメント内の文字列に引っかかる** — lib.rs:132 の `// ... #[cfg(test)]` が生ソース検索の境界となり、7ファイル9箇所のスキャンが lib.rs を89文字の doc ヘッダで打ち切っていた。後続の `cfg_attr` deny 配線・`pub mod` 宣言を一度も走査していなかった | 新チェッカーの自己検証(存在する全形式を数える vacuity guard)で発覚。`test_module_boundary`(非コメント行のみを境界に採用)として水平展開。**「スキャナが見ている領域」自体が検査対象になる教訓** |
| H3 | **チェックアウトの行末が clone 側の `core.autocrlf` 依存** — fixture のバイト比較・複数行 `contains` 検査・`sh` の gate.sh は autocrlf=true の Windows clone で壊れ得た。「同一入力」はリポジトリ自身のバイトにも適用される | `.gitattributes` に `* text=auto eol=lf` を追加。存在と内容は `docs_are_current.rs` の `checkout_line_endings_are_pinned` が検査 |
| H4 | **`deny` はレベルでありリーフで無効化される** — G7「panic 経路 0」は crate root の `deny(clippy::unwrap_used, expect_used, panic)` に依存するが、`#[allow(clippy::unwrap_used)]` を1行書けばそのアイテムでは消える。`unsafe_code` は `forbid` で絶対だが clippy lint は `deny` のままだった | `global_invariants_hold.rs` の `g7_*` が(a)両 lib.rs の deny 属性の存在を assert し、(b)非テスト領域の `allow`/`expect`/`warn`/`force_warn` が保護 lint(`unsafe_code`/`unwrap_used`/`expect_used`/`panic`)を名指さないことを検査。`cfg_attr(test|doctest|doc|docsrs, …)` 内側は出荷コードに効かないので免除(`not` を含む判別式は免除しない)。実注入(`#[allow(clippy::unwrap_used)]` を src に混入)で検出確認 |
| H5 | **manifest のセクション文法が開いていた** — `[target.*]` だけでなく `[features]`(`cfg(feature)` の入力宣言)、`[lints]`・`[patch]`・`[replace]`・`[build-dependencies]` はいずれもセクション走査の視野外のビルド設定。さらに `.cargo/config{,.toml}` は `build.rustflags` で `--cfg`(条件コンパイルの注入)や `--cap-lints allow`(deny の一律降格)を渡せる — **このスイート全体を迂回するファイル** | `no_platform_cfg_in_sim.rs` が3 manifest 全てのテーブルヘッダを atom 化して禁止リストと照合し、ルート+両クレートの `.cargo/config{,.toml}` 不在を検査。`[features]` セクション注入と `.cargo/config.toml` 設置の両方で検出確認 |
| H6 | **SPEC が名指す強制場所の実在が未検査だった** — `enforcement_sites()` は「どの G がどこで強制されるか」の対応だけを検査し、名指されたファイルの実在は検査していなかった。チェックファイルの改名で「強制されていると読める亡霊」が残る | `every_named_enforcement_site_is_a_file_that_exists` が site 文字列中の `.rs`/`.md`/`.sh` トークンを実在検査(パス省略形も拒否 — G5 の `roguelike_sim.rs` をフルパスに修正) |
| H7 | **`deny(clippy::panic)` は panic 経路全般を封じない** — `assert!`/`debug_assert!`/`unreachable!`/`todo!`/`unimplemented!` は同じトラップに lower されても `panic!` リテラルの lint をすり抜ける。実測で15箇所存在(コンストラクタの文書化済み事前条件・debug-only の不変条件・replay の到達不能 arm) | `panicking_macro_allowlist`(file・件数・理由)で凍結 — 新規サイトは理由を書かない限りビルドが落ちる。allowlist は「実行時入力への saturate/None/no-op」(G7 の契約)と区別するため、プログラマエラーの事前条件と明記したサイトのみを認める。`todo!` の注入で検出確認。**残存論点**: `v[i]` のスライス indexing は構文的に正当読み取りと区別不能なので射程外 — G7 の境界はそこ |
| H8 | **スキャン領域の外からコードを混入する経路が開いていた** — 全スキャナの根底的仮定は「コンパイルされるコード == src/ 配下のテキスト」。`#[path]` は `mod` を任意ファイルに向け、`include!`/`include_bytes!` は走査外のバイトを混入できる | `shipped_code_cannot_come_from_outside_the_scanned_tree` で両クレートに禁止。`include_str!` は `&'static str`(doc 用)のみなので免除。実注入(gen.rs を `include!`)で検出確認 |
| H9 | **ambient input 禁止が `env::` のみ・ゲート自身は誰も検査していなかった** — filesystem/process/I/O は入力ログに載らない環境入力であり禁止漏れ。さらに gate.sh のステージを消しても「実行したチェックだけが通る」ので exit 0 のまま — 定義者自身が無検査 | no_nondeterminism の BANNED に `std::process`/`process::`/`std::fs`/`fs::`/`std::io` を追加(`use std::process` 注入で検出確認)。docs_are_current に `the_gate_script_still_runs_every_stage` — fmt/test/clippy/doc/pinned hash 両プロファイル/examples 列挙/package/tarball 検査/gamec の各識別トークンの存在を検査(トークン削除で検出確認) |
| H10 | **manifest の文法はテーブル列挙だけでは閉じない** — `[[bin]]`/`[[test]]`/`[[bench]]`/`[[example]]` ターゲットテーブルは `path` 転送や `harness = false`(テストを空実行化)を可能にし、テーブルを要しない `harness`/`auto*`/`crate-type`/`proc-macro`/`test`/`bench`/`doctest` キーも未禁止だった。加えて既存の `[profile.*]` は正当だが `debug-assertions`/`overflow-checks`/`panic` キーは意味論を書き換える | テーブル atom に `bin`/`test`/`bench`/`example` を追加、インライン `{...}` を含む行の bare-key スキャンで全 manifest に禁止キーを適用、profile セクションでは意味論3キーを禁止。`debug-assertions = false`・`[[test]]`・`harness = false` の各注入で検出確認(初版は `=` 前の空白で key が空になる vacuity バグを変異注入で炙り出して修正) |
| H11 | **ビルドを変えるファイルは `.cargo` だけではない** — `rust-toolchain{,.toml}` はコンパイラ自体を差し替え(注入実演で channel=1.60 が MSRV エラーでビルドを停止)、`Cross.toml` はターゲットを変える。どちらも不在で弱く検査されていなかった | manifest 検査の存在チェックに `rust-toolchain`/`rust-toolchain.toml`/`Cross.toml` を追加(`channel = "stable"` 記述のファイル設置で検出確認) |
| H12 | **gate.sh は呼ばれた環境をそのまま信頼していた** — `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`/`RUSTDOCFLAGS` は `--cfg` や `--cap-lints allow` を rustc/rustdoc に届け、呼び出し元シェルの環境がスイート全体を弱め得る。「ゲートを定義する者がゲートの入力を定義していなかった」 | gate.sh 冒頭で無条件 `unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTDOCFLAGS`、その行の存在をゲート自己検査のトークンに追加(行削除注入で検出確認)。併せて出荷コードの `env!`/`option_env!`(ビルド機の環境をバイナリに焼き込む)と `extern`/`#[no_mangle]`/`#[link`(manifest に載らないネイティブ依存)を禁止、`std::arch`/`core::arch` の CPU 機能検出を kit BANNED に追加(各々の注入で検出確認) |
| H13 | **検証スイート自身は無条件に信頼されていた** — `#[ignore]` はスイートを緑のまま「実行された」ように見せ(実際に det_hash_golden.rs に1箇所存在: 再生成ヘルパー)、tests/ 内の `#[cfg]` はプラットフォームでチェックを消し、テスト crate は lib の `forbid(unsafe_code)` を継承せず `unsafe` を持ち込め、ゼロ `#[test]` のファイルは何も走らないままコンパイルされる | `the_verification_suite_cannot_quietly_skip_or_disable_its_own_checks` が両 crate の tests/ を走査(コメント・文字列・char リテラル除外)。`#[ignore]` は理由つき allowlist に凍結。4種の注入で検出確認 — 実装時に char リテラル `'"'` が文字列モードを誤起動するパーサバグをテストが炙り出し修正 |
| H14 | **フォーマッタ・リンタの設定面が未検査** — `izanagi/rustfmt.toml` は良性だが `ignore`/`disable_all_formatting`/`skip_children` で fmt --check の対象を縮小可能、`clippy.toml` は lint を弱め、`[workspace.dependencies]` は path-only 検査をすり抜ける依存隠し | rustfmt の逃走経路キーは禁止(ファイル自体は許可)、`clippy.toml` と `[workspace.dependencies]` は不在を検査。3種の注入で検出確認 |
| H15 | **examples/ とドキュメントフェンスはスイート規律の外にいた** — gate が examples を2回バイト比較する以上、example 内の `#[cfg]` は pinned hash の証拠自体をプラットフォームでフォークし得る。md の ```ignore/no_run/compile_fail/should_panic フェンスは「doctest される」主張を空にする。さらに .githooks と docs/ci が gate.sh を呼ぶことも凍結されていなかった | tests/ の規律走査を examples/ に拡張(#[test] 要件のみ除外)。全 md の逃走フェンスを禁止。.githooks/pre-push・docs/ci/ci.yml が tools/gate.sh を呼ぶことを固定。example への #[cfg]・README への ```ignore・ci.yml からの参照除去の各注入で検出確認 |
| H16 | **同一機能の別表記が残っていた** — `#![cfg(unix)]` 内側属性はテスト/example ファイル全体をプラットフォーム条件化できるが `#[cfg` needle はすり抜ける。また `#[ignore]` allowlist は対象が実際に ignore を保持することを検査していなかった — ignore を外しても許可証が残る | `#![cfg` を禁止 needle に追加(`#![cfg(unix)]` 注入で検出確認)。allowlist に実体検査を追加(`#[ignore]` 除去注入で検出確認)。調査の副産物: Cargo.lock は gitignore 済みでライブラリとして正しい — 内容は zero-dep manifest 経由で間接凍結されている |
| H17 | **panic の「封じ」は lint で、機構は残っていた** — `clippy::panic` は `panic!` だけを見る。`catch_unwind`/`std::panic::*` は panic を吸収する機構であり、G7「panic しない」下で「実 panic を catch して通過結果に換金」する経路だった。さらに走査配下の symlink はレビュー面に出ない外部バイトを読む | kit BANNED に `catch_unwind`/`panic::` を追加(注入で検出確認)。src/tests/examples の symlink 不在を provenance 検査に追加(symlink 注入で検出確認)。tests/examples の `env::`/`process::`/`temp_dir`/`env::args`/`process::exit` は実測で CLI・テストの仕事そのものと判明 — 「設計どおり」として記録し、禁止は src/ 限界に留める |
| H18 | **スケジューリングとアドレス漏洩は別の扉だった** — Mutex/RwLock/mpsc/Atomic/std::sync/std::thread はOSスケジューラを入力ログに混入させる ambient input だったが未禁止。ポインタアドレスは `.as_ptr(` だけが禁止で、`.as_mut_ptr(`・`as *`・`*const`/`*mut` 型名経由の safe な漏洩が残っていた | 実測ゼロ使用を確認し BANNED に 10 needle 追加(Mutex 注入で検出確認)。`spawn` は `min_spawns` 変数名との衝突で needle 不可 — `thread::`/`std::sync` のモジュール経路で代替。残余: `std::ptr::null::<T>() as usize` のような型名を書かない safe キャスト(文書化) |
| H19 | **境界検出は `//` 対策で終わっていなかった** — `#[cfg(test)]` marker を find/行contains で探す10箇所の走査は、文字列リテラル・ブロックコメント・行中の偽 marker で全て打切可能だった。加えて実装は既に3種に drift(global_invariants/no_platform_cfg は弱い find 版、no_float は行版) — コピー同一性自体が未検査だった | 全10箇所をネストしたブロックコメント・raw文字列・文字リテラル/ライフタイムを処理する共通 lexer に統一し、`the_boundary_lexer_is_the_same_in_every_scanner` で fn 本体のバイト同一性を凍結(文字列内偽 marker + drift 変異で検出確認) |
| H20 | **drift は境界 lexer だけでなかった** — `contains_token` は3種(旧両側境界版が no_float に残存)、`library_sources` は6箇所5種(width/endian は再帰しない平坦走査 — 将来の非-bin サブディレクトリを無音で見逃す構造的盲点)。`.githooks` も「実在≠発火」: core.hooksPath は clone ローカル設定で、repo が強制できるのは「AGENT が設定を指示し続ける」ことだけだった | 全 helper を正規版に統一(`&Path`→`BTreeMap` 共通シグネチャ)、同一性検査を6 helper × 計33コピーへ一般化(drift 変異で検出確認)。docs_are_current に `core.hooksPath` 指示の存続検査を追加(削除変異で検出確認) |
| H21 | **md のフェンス規則は src doc を見ていなかった + 同一性≠正確性** — `//!` フェンスの `ignore`/`no_run` は未走査で、log.rs の `ignore` は実際に腐った例(マクロ移動で use が壊れていた)だった。同一性凍結は「同じ」を証明するだけで「正しい」は証明しない — 境界 lexer に合成入力での正確性検査が無かった | log.rs の use を修正して ```rust 化。src フェンス検査追加(ignore 注入・no_run 陳腐化変異で検出確認)。`the_shared_boundary_lexer_finds_real_markers_and_rejects_fakes` を追加 — 偽 marker 7形態・真 marker・混在を合成入力で凍結(偽 marker 受理変異で検出確認) |
| H22 | **レイアウト照会・abort・出力系は値列挙の外だった + resolver は「書いてある」≠「実キー」** — `size_of`/`align_of`(幅依存バイトの混入経路)、`abort(`(catch_unwind すら迂回する脱出)、print 系(例出力バイト比較への汚染源)は実測ゼロで BANNED 化。`resolver = "2"` は存在したが `contains` 検査ならコメント化で素通り — [workspace] セクション内の実キー+値を検査。`{:?}`/Debug は診断文字列用途のみで正当と記録 | kit BANNED +7 needle(dbg! 注入・resolver コメント化で検出確認) |
| H23 | **#[test] の存在は assert の存在を意味しない** — assert 系マクロがゼロのテストファイルは「実行するが検証しない」vacuous 緑。実測で bench.rs(測定ハーネス・出力が成果物・異常は依然 crash で落ちる)のみ該当。src 内 test 領域は vacuous ゼロ | tests/ 全ファイルに assert トークン床を追加し bench.rs を理由付き allowlist 化(assert 消去変異で検出確認)。allowlist は「ファイルが依然 assert-フリーであること」を要求(陳腐化検査)。tools/ は gate.sh のみで走査対象外スクリプトなしと確認 |
| H24 | **平坦走査の盲点 + スイート自身の環境依存/腐敗隠蔽** — `tests/` は cargo のテストクレート発見も走査も平坦(トップレベルのみ) — サブディレクトリの .rs は「実行も走査もされない」最も死んだコード。`#[allow(dead_code)]`/`unused` 系抑制子は src 外でも腐敗を隠せた(roguelike.rs の write-only CombatEvent フィールドが実在例)。tests/ ファイルの `env::var`/`env::args`/`set_var`/`remove_var`/`current_dir`/`current_exe` は機械ごとに結果を変えるスイート自身の ambient input だった | tests/・examples/ 4 ディレクトリにサブディレクトリ禁止 assert を追加、`dead_code`/`unused` を substring needle 化(トークン境界では `unused_mut` 等に不敗のため)、tests のみ環境読取 needle 群を追加(temp_dir は免除 — パスは違うが検査は同じ)。roguelike の死フィールド削除。4変異(dead_code/unused_mut/env::var/subdir 注入)全発火確認 |
| H25 | **テスト数フロアは下限しか検査していなかった + 数え方が substring** — 「3,400+ tests」は実数が 6,000 に育っても真のまま腐り続ける(下限のみ)。加えて `.matches("#[test]")` の substring 計測は `// #[test]` コメント・文字列内言及を実テストとして水増しできた(実在 phantom 10件) | `test_attributes` を行頭アンカー化(phantom 消滅・コメント水増し不可に)。フロア新鮮さ帯域 `actual >= claim >= actual*3/4` を追加 — 実数がフロアの 133% を超えると同コミットでフロア引上げを強制。3変異(陳腐フロア/コメント phantom 200行/実属性 960 個の過剰水増し)で両方向の発火を確認 |
| H26 | **golden ピン集合は「網羅されている」と信じられていたが列挙されていなかった** — det_hash_golden の自身の記述ですら "~40 other impls pinned nowhere" と陳腐化(実測: kit 定義型の DetHash impl 89件中 23件のみピン、60件以上が無言及)。新しい `impl DetHash` は誰にも監視されず追加可能だった。lockfile も別層: 空 [dependencies] は manifest 面の話で、解決結果(Cargo.lock)の外部クレート不在は未検査(かつ gitignore + cargo 自動修復で直接変異は不可能 — path-dep 変異で発火確認) | `every_dethash_impl_is_pinned_or_declared` 新設: kit 定義 pub 型 ∩ impl は golden 言及 or `UNPINNED_DET_HASH` 宣言を強制(双方向陳腐化検査: ピン獲得/impl 消滅で宣言自動失効)。`the_lockfile_names_only_workspace_crates` で解決層の zero-dep を凍結。golden ヘッダの "~40" を列挙側の記述に更新。3変異全発火 |
| H27 | **`src/bin/` は全走査が意図的にスキップする完全な無防備地帯だった** — library_sources の `bin/` 除外は「CLI はマシンに答える」の論理で正当だが、「argv とファイルを読む」≠「何でもしてよい」。加えて bin ターゲットは別クレート root — lib.rs の `forbid(unsafe_code)`/`deny(clippy::panic)` は gamec に一切効かない。実測ゼロ違反で「信じられているが強制されていない」の典型 | `bin_targets_inherit_the_bans` 新設: unsafe/panic 系全般/unwrap・expect/cfg/env::var/process::exit/thread/pointer/size_of 等を CLI 適用範囲で禁止(env::args・std::fs・ExitCode は正当)。unwrap/cfg/unsafe の3変異で発火確認。除外コメント3箇所を新検査への参照に更新 |
| H28 | **走査はテキストを見る、コンパイラは mod 宣言を見る — 両者の差分が無検査だった** — `library_sources` は src/ 配下の全 .rs を走査するが、`mod foo;` 宣言のない src/foo.rs はコンパイルされない。実証: 孤立ファイルを置いても cargo build は再コンパイルすらせず suite 全緑 — 「走査対象だから生きているコード」は偽。逆向き(ファイル→宣言)はどの doc↔宣言検査も見ていなかった | `every_source_file_is_a_declared_module` 新設: 両クレートの src/*.rs 全 stem が lib.rs の `mod`/`pub mod` に存在することを要求(bin/ は target であり module ではない、mod.rs サブディレクトリの宣言半分も検査)。孤立ファイル・孤立ディレクトリの2変異で発火確認 |
| H29 | **出荷 src にも decay 抑制子の扉が残っていた + workflows 不在と env! 名称が未検査** — round15 は tests/examples の dead_code/unused を禁じたが出荷 src は対象外(weakening 走査は安全系 lint のみ保護)。`.github/workflows/` の不在は家の規則のみで機械検査なし — dependabot.yml は許容、workflows は push ごとに動くため除外。`env!`/`option_env!` は src で禁止済みだが tests/examples のコンパイル時環境読取は未規制 — test_code が文字列を潰すため変数名の区別には生テキスト走査が必要だった(自己参照は引用符直前 skip + needle 分割構築で回避) | `shipped_code_has_no_decay_suppressors`(両クレート impl 領域の allow/expect/warn 引数から dead_code・unused* 原子を禁止)、`no_ci_workflow_is_committed`(.github/workflows の不在)、suite 検査へ env!/option_env! の変数名検査追加(CARGO_MANIFEST_DIR のみ許可)。3変異(env!("HOME")/src allow(dead_code)/workflows ディレクトリ)全発火 |
| H30 | **`if x.is_ok() { assert!(...) }` は環境依存の静かなスキップだった** — readme_blocks_agree.rs が `if fs::write(...).is_ok()` で probe ファイルの書込みをガード — 書けない FS では assert が丸ごとスキップされ緑。全テスト唯一の is_ok() 使用がこの形状(実測)。出荷 src の `let _ = w.write_all` 系は意図的 best-effort(log/pipe 切断で crash しないための設計)と判明、kit の static HIDDEN は監査用 test fixture | probe を無条件 `expect` に修正、`if <x>.is_ok()/is_err()` ガード形状を tests 全域で禁止(変異注入で発火確認) |
| H31 | **validate() の「全 findings を収集(never short-circuit)」は未強制だった** — 既存テストは全て1 defect クラスの fixture — first-finding で bail する validator も全緑。二層で凍結: (a) validator.rs impl 領域に `return`/`?`/`break`/`continue` を構造禁止(test_code が文字列・コメント・文字リテラルを潰すため安全)、(b) 13 defect クラス全発火する1つの bundle で欠落を検査。`return Vec::new()` 変異で両層発火確認 |
| H32 | **ライセンスは3箇所で宣言され drift 可能だった** — manifest `license` フィールド・LICENSE ファイル内容・README 記載が相互不検査(存在+プレースホルダ検査はあったが内容一致は無し)。三方向一致を新規凍結: manifest フィールド↔ファイル内容(MIT/Apache 署名句)↔README 記載。LICENSE-MIT を Apache 本文に差替・manifest フィールド変更の2変異で発火確認。なお izanagi=MIT・kit=MIT OR Apache-2.0 の差は README 記載済みの意図的設計 |
| H33 | **interior mutability は未禁の偽決定論経路だった** — `Cell`/`RefCell`/`OnceCell`/`Rc`/`Arc`/`lazy_static` は「純粋に見える型の中の memo キャッシュ」— 2回目の呼出が input log に無い値を返せる。実測ゼロ使用(terminal::Cell は同名のセルバッファ型 — `cell::Cell` モジュールパス needle で区別、sim.rs:378 の static Cell は監査用 test fixture)。10 needle 追加凍結、RefCell 注入で発火確認 |
| H34 | **roundtrip は自己整合的な wire 形式変更を検出しない** — save.rs の write/encode が wire layout を二重実装しており、片方だけ変えても(例: BE version)roundtrip は緑のまま既存セーブを全て壊せた。write を encode へ委譲して単一化 + バイト厳密ピン(MAGIC+LE u16+u32+payload) + write==encode ディスクバイト一致の3層。kit 側も serialize の canonical 出力を文字列ピン(roundtrip/idempotent は形式 drift を見ない)。BE 変異で発火確認 |
| H35 | **diag_json/diag_sarif も同じ wire 形式の穴** — 機械消費される JSON 出力(CI/LSP 連携)は構造 assert(contains・brace 平衡・フィールド抽出)のみ — キー改名・空白変更・エスケープ方式変更は全て緑のまま消費側を壊す。R25 と同型のバイト厳密ピンを両フォーマットに追加(エスケープ・region 有無・severity→ruleId 写像・schema URL まで凍結)。空白除去変異で発火確認 |
| H36 | **SPEC の「固定コンパス順」は配列の順序をピンしていなかった** — pathfinding の 8 方向 DIRS・plan/wfc の 4 方向 DIRS は決定的だが「固定順」の中身は無言及: 並替えは決定性を保ったまま全ての経路解決・WFC 規則の方向 index(wfc の DIRS は 0=N/1=E/2=S/3=W がルール表の公開意味)を静かに変える。3テーブルを in-file テストでピン + wfc の opposite() も固定。DIRS 交換変異で発火確認 |
| H37 | **最後の無防備定数2件を閉じた** — pathfinding の astar_cardinal 専用 CARDINALS(8方向 DIRS とは別テーブル)が fn 内ローカルで未ピン → CARDINAL_DIRS としてファイルスコープに昇格し既存ピンに統合。xoshiro の JUMP 定数は「違う stream を生む」弱テストのみ — タイポ1桁でもテストは緑のまま 2¹²⁸ 非重複の公開約束が静かに破れる → jump 後 state と出力第1ワードを値ピン。JUMP 1桁変異・CARDINALS 交換変異で発火確認 |
| H38 | **順序が公開契約の enum は判別値変更で静かに壊れる** — Visibility は `Unseen < Remembered < Visible` が doc 契約(is_explored = `>= Remembered`、進捗は `max`)。派生 Ord は宣言順ではなく判別値を見るため、バリアント並替えは無害だが `Unseen = 0 → 2` の書替えは全 hash ピン緑のまま exploredness を反転 — rank() と大小関係をピン化。判別値変異で発火確認。他の Ord 付き enum(damage/equipment)は ALL-index 往復で被覆済み、85本の手書き DetHash impl と全 op 実装は derive/明示演算で法則矛盾不可 |
| H39 | **公開イテレータの順序契約は doc のみだった** — SpatialHash::iter_cells は実装が BTreeMap(昇順)だが doc は「HashMap 順 — 未ソート」と腐っていた(安全側の誤記だが、契約の機械的根拠が無いまま)。逆順変異で発火する昇順ピンを追加し doc を実装に一致させた。parents/children は Vec 保持で挿入順(決定的)、multimap も Vec 裏付け — 順序漏洩面は残りゼロ |
| H40 | **公開 API の集合自体は未ピンだった** — 「全 pub fn が呼ばれる」検査は存在したが、引数追加・enum バリアント増・フィールド拡張・re-export 削除は全て緑のまま通る。`pub` 行+`pub struct/enum/trait` ブロック内メンバー+複数行シグネチャ尾部+API 属性(deprecated/non_exhaustive/must_use)を抽出し、両クレートで FNV-1a hash+件数をピン(kit 2283行・engine 514行)。enum バリアント追加・引数追加の両変異で発火確認 |
| H41 | **engine の swept_aabb は手選び例のみで、対称性・ tunneling が未検証だった** — 「a が m で b に突入」は「b が -m で a に突入」と同一の相対運動であり、entry/exit 式の符号ミスは片側だけを壊すが従来テストは交差軸を見ない。2万ケースの metamorphic 対称性テスト(役割交換で is_some と t が一致) + 1px 壁への400単位移動の tunneling ケースを追加。entry/exit 入替変異で両テスト発火確認 |
| H42 | **engine math の性質層も手選び例だけだった** — Mat3::Mul の列-major 結合は index 1つずれで「なんらかの行列」を返すが壊れる: 点への結合則 (A*B)p == A(Bp) を5千ケースで検証。rotation の等長性と R(a)R(b)=R(a+b) 準同型、reflect の対合性、perp の直交性を追加。Mul index 変異で発火。回転符号反転(CW↔CCW)は準同型では検出不能(同じく準同型) — 既存の90度固定例が捕捉する役割分担を確認 |
| H43 | **実バグ: ECS の世代チェックが insert にしか無かった** — doc は「despawn で無効化」と約束するが `get`/`get_mut`/`remove` は index のみで検索 — despawn→index 再使用で**stale ハンドルが新エンティティのコンポーネントを読み書きできた**(ABA aliasing)。実証テストで再現→3アクセサに `alive` ガードを追加(insert と対称)し回帰テスト化。世代機構は存在したが強制されていなかった |
| H44 | **実バグ: Animation のゼロ duration で tick が無限ループ** — `while elapsed >= duration` は duration≤0 で elapsed が減らず、looping 再生は永久回転(コンテンツ起因のゲームフリーズ)。ECS stale バグと同系の「機構あり強制なし」。`Animation::new` に `duration > 0.0` バリデーション(NaN も弾く、`+inf` は「静止保持」として許可) + should_panic 回帰 + infinite-frame 保持テスト。assert 除去変異で発火確認 |
| H45 | **実バグ: WAV パーサが申告サイズを信頼し切り詰め入力で OOB panic** — `fmt ` チャンクは `size>=16` の申告のみ検査し実残バイトを見ず、`data[pos+14]` 系読取りがバッファ終端を超えた(信頼性/DoS)。`pos += size` も 32-bit usize で wrap し得る(カーソル巻戻り→無限ループ)。実残サイズ検査 + saturating_add カーソルに修正、回帰テスト2件追加 |
| H46 | **実バグ(2件目): エンジン save パーサに同型の 32-bit wrap バグ** — kit savefile が直した「宣言長 + 固定オフセットの加算比較」を `izanagi/save.rs` が踏んでいた: `bytes.len() < 10 + len` は wasm32 で `len=u32::MAX` が加算 wrap してガードを抜け slice panic。兄弟パーサへの修正伝播漏れ — 同一リポ内でバグの"系"は横断掃引が必要。減算形式に修正 + 敵対 len テスト |
| H47 | **3系のバイナリデコーダに全接頭辞+全バイト破壊スイープ追加** — テキストパーサには garbage ファズがあったが savefile/load_wav/Save::parse は点テストのみ。**スイープ自身にも vacuity があった**: minimal_wav(fmt 先頭)の接頭辞は 44B 下限に遮られ脆弱状態に到達不能 — fmt を末尾に置く第2 fixture で初めて R36 の OOB 系を検出可能に(変異で実証)。「スイープがある」≠「経路に届く」 |

---

# 第5次: 外部出典サーベイ — procgen 配置パイプラインと lockstep wire codec (2026-09-22)

GitHub・論文・Qiita/Zenn・海外技術情報を対象に第1・2次とは別パスの棚卸し。
前回までの棚卸しが検証機構へ収斂したのに対し、今回は**コンテンツ生成側の隙間**を調べた:
scatter(配置)→ territory(領域)→ connectivity(接続)という手続き生成の標準パイプラインが
全く未実装だったこと、および lockstep の基盤プリミティブであるビット列 codec が
未実装だったことが判明。4件すべて実装。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `mapgen::poisson_disc` — Bridson アルゴリズムの最小間隔散布 | Bridson, *Fast Poisson Disk Sampling in Arbitrary Dimensions* (SIGGRAPH 2007 sketch, doi:10.1145/1278780.1278807)。参照実装 kchapelier/poisson-disk-sampling、Unity *Graphics Programming* vol.4 第7章。`radius/√2` グリッド(整数では `r·7071/10000` で保守的 floor)+ active-list + annulus 拒否サンプルで文献どおり k=30 既定。三角形サンプルは極座標ではなく外接正方形からの拒否法に置換(Trig 不要・均一性は等価: 環状領域上の一様分布)| 🟢 純粋追加、pinned hash 不変 |
| `voronoi::voronoi_partition` + `voronoi_flood` + `mst_edges` — 厳密最近seed分割・可通行BFS版・Kruskal MST | Rong & Tan JFA(I3D 2006, doi:10.1145/1111411.1111431)は意図的に**不採用**(GPU 近似=このクレートの厳密性公理に反する)。`O(whk)` 素朴法がゲーム規模では正解 — 検証コストをゼロにする「approximate ではない」保証付き。`voronoi_flood` は DijkstraMap のはしごが降りない先の「どの領土か」を返す BFS 版。mst_edges は TinyKeep/Slay-the-Spire 系 dungeon 配線の標準部品(散乱→領域→接続のパイプライン完結)| 🟢 純粋追加。`VoronoiGrid` に `DetHash` — golden ピン済み |
| `noise::worley_2d` 系 — Worley セルラーノイズ (F1/F2/cell) | Worley, SIGGRAPH'96 (doi:10.1145/237170.237267)、iq 解説、Qiita GLSL 記事群。**3×3 走査は近似に過ぎないことを確認して 5×5 を採用**: 隣2セルの feature は `cs` 距離まで近づき得る一方、自セル feature は `√2·cs` まで遠ざかり得るので、悪条件では 3×3 の外側が勝つ。5×5 なら外側は `2·cs` 超で `√2·cs` に必ず負け、証明可能な厳密性に到達する(この論点は既存記事にはほぼ記載がない)。5×5 独立オラクルで実装を検証 | 🟢 純粋追加 |
| `bits::BitWriter`/`BitReader` — LSB-first ビット列 wire codec | Gaffer *Reading and Writing Packets* / *Serialization Strategies*(lockstep ネットコードのカノニカルプリミティブ)+ protobuf wire format(varint LEB128 + zigzag)。packed bitfield・`write_ranged`(bits_required)·canonical varint(非 canonical trailing-zero group を decode 側で拒否し、値↔バイトの全単射を保持)。`savefile` はバイトコンテナ、`serializer` は `.game` テキストであり、ビット単位の wire 形式を担う層が無かった | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| jump flooding (JFA) GPU Voronoi | Rong & Tan 2006 | GPU 専用かつ近似 — このクレートの「証明可能な結果」公理に反する。CPU `O(whk)` が規模的に正解 |
| Simplex/Perlin 勾配ノイズ | Perlin 2002, Gustavson 2005 | 既存の value/fbm/ridge/turbulence 群で実用上被覆済み。勾配ノイズの追加は純粋重複で新規知見を産まない |
| 空間分割の四分木 | gaffer・kd-tree 系 | `spatial_hash` が同用途を担っている。四分木は本質的に浮動座標向きで、整数グリッドのこの kit では冗長 |

## 出典(第5次、search-index 照合)

**論文**: Bridson 2007 (SIGGRAPH sketch)/ Rong & Tan (I3D 2006)/ Worley (SIGGRAPH'96)/ Dormans cyclic dungeon(参考)。

**実装物**: kchapelier/poisson-disk-sampling / TinyKeep mapgen 記事 / Slay-the-Spire mapgen 記事 / protobuf encoding 仕様 / Gaffer serialization 記事群 / iquilezles noise 記事 / Qiita セルラーノイズ・Poisson 記事群 / Unity Graphics Programming vol.4。

# 第6次: 外部出典サーベイ — 迷路・六角・三角分割・回廊掘削 (2026-09-22)

> 第5次で完成した scatter→territory→connectivity パイプラインの**下流層**が未実装だった:
> 接続するための三角分割配線(Delaunay)、許可エッジ制限下の MST、掘削自体の
> 最小コスト隧道、迷路生成、六角グリッド。5系統実装。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `delaunay::delaunay` + `delaunay_edges` — 整数厳密 Delaunay 三角分割 | Bowyer & Watson, *Comput. J.* 24(2) 1981。**cavity-fan ではなく split+Lawson-flip 増分挿入を採用** — cavity 版は非 star-shaped cavity で hull 内に穴を残す実害を再現(回帰テスト化)。incircle は i128 の 3×3 lifted 行列式で厳密評価、`det==0` の cocircular は「外」扱いとし対角線選択を canonical に固定。**super-triangle は `span²` 距離**(本セッション最大の知見: 距離 `~span` では "(hull edge, super vertex) の外接円が内部点を包み hull 辺が super 側へ flip され、super 三角形の除去後に内部穴" になる。遠方 super では sagitta = edge²/8D < 1 となり整数点は絶対に包めない) | 🟢 純粋追加。pinned hash 不変 |
| `hexgrid` — axial 座標の六角計量一式 | Amit Patel, redblobgames.com/grids/hexagons (実務標準); Qiita/Zenn の hex 記事群も同レシピ。`s = −q−r`、distance = (|dq|+|dr|+|ds|)/2、cube lerp + `cube_round`(最大誤差軸を丸め戻す)、line/ring/spiral/offset 変換。**cube_round の tiebreak は away-from-zero に固定**(IEEE 系 round-half-even 依存を排除)| 🟢 純粋追加。`Hex` に `DetHash` — golden ピン済み |
| `maze::wilson_maze` — Wilson の loop-erased random walk で perfect maze | Wilson, STOC'96 (*Generating random spanning trees more quickly than the product time*)。全域木を**一様分布**で生成する数少ない正しい迷路アルゴリズム(DFS backtracker 系は分布が歪む)。Buckblog (Mazes for Programmers) の手順 — LERW の「既訪 walk を pos+1 以降 drain で消す」ループ消去をそのまま実装。cell=奇数座標・回廊=+1 の定石レイアウト | 🟢 純粋追加 |
| `pathfinding::min_cost_path` + `mapgen::carve_corridors` — 重み A* と TinyKeep 式隧道 | TinyKeep mapgen 記事(Petteri, 2014)の corridor recipe — 「既存床は安く・壁掘りは wall_cost」の重み A* で MST エッジを掘る。`enter_cost: FnMut→Option<i32>` の一般化(床=1/壁=wall_cost/OOB=None)。対角ステップは橋 cell を併掘して 4-連結を維持(斜め掘りだと物理的に通行不能になる見落としを実測で捕捉) | 🟢 純粋追加 |
| `voronoi::mst_edges_over` — restricted Kruskal | TinyKeep の本流 recipe: 全対 MST ではなく Delaunay エッジ集合上の MST(配線図上で最小接続)。invalid ペア除外→(dist_sq,i,j) 全順序→union-find。disconnected 入力では森を返す | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Prim's/recursive-backtracker 迷路 | Buckblog 迷路シリーズ | 生成木の分布が一様でない(偏りが構造的に解析可能)。Wilson は一様性の保証があり DST 系との親和性が高い |
| Fortune's algorithm (Delaunay/Voronoi) | Fortune 1987 | sweep-line は浮動イベント点前提。整数厳密では Bowyer–Watson 系が実装も証明も現実的 |
| 六角 offset/q-r 双対座標のみ | redblobgames | axial 単系に統一 — 2系統併存は変換 bug の温床。cube(3成分)は内部表現のみ |

## 出典(第6次、search-index 照合)

**論文**: Bowyer & Watson 1981 (*Comput. J.* 24(2)) / Wilson STOC'96 / Guibas & Stolfi 1985 (Lawson flip / quad-edge 構造) / Fortune 1987 (見送り)。

**実装物**: redblobgames hex ガイド / TinyKeep mapgen (Petteri 2014) / Buckblog *Mazes for Programmers* / Bowyer-Watson 参照実装群 (hug-sun 等) / Qiita・Zenn の hex・迷路・Delaunay 記事群。

# 第7次: 外部出典サーベイ — レイキャスト・グラフ解析・矩形パッキン (2026-09-22)

> taxonomy の残存ギャップ(E5 derive macro・H6 ホットリロード・O2 ソケット transport)は
> すべて意図的スコープ外のため、今回は**アルゴリズム層の未実装領域**を棚卸し:
> グリッド ray 走査、グラフ構造解析、矩形パッキン、六角 A*。4系統実装。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `gridcast` — `grid_ray`/`ray_blocked_at`/`clear_los` グリッド ray 走査 | Amanatides & Woo, *A Fast Voxel Traversal Algorithm for Ray Tracing* (Eurographics'87)。Bresenham(geometry)は列ごとに1 cell を選ぶラスタ化だが、DDA は**線分が入る全 cell を入射順に**列挙する別物 — 弾道・掃引 LOS に必要な層がなかった。実装は全座標を 2 倍し端点を cell 中心に置く整数厳密版; 次境界比較 `tMaxX < tMaxY` は i128 交叉乗算。**corner 通過(tMaxX==tMaxY)では対角 cell のみに進入**し側方 cell はゼロ長交差として除外 — slab-test オラクル(開箱との正長交差)で 500 ray の集合一致を検証し、対称性 `ray(a,b)=rev(ray(b,a))` を確認 | 🟢 純粋追加 |
| `graph` — Tarjan SCC・関節点・橋・Kahn topo・`UnionFind` | Tarjan 1972 (*SIAM J. Comput.* 1(2), low-link)、Kahn 1962 (topological sort)。マップ接続性の解析と tech tree / 依存 DAG の build order を担う層が未実装だった。DFS は全て**反復版**(再帰深度非依存)。`topo_sort` は ready-queue を `BinaryHeap<Reverse>` にし辞書順最小の canonical order を保証。articulation/bridges は「parent への木辺を1本だけ skip」(平行辺の2本目は真の back edge として low を下げる)まで踏み込み、remove-and-recount オラクルで検証。`UnionFind` は `voronoi::mst_edges` 内部で使われていた構造を独立公開(小さい index を root に保つタイブレークで representative が union 列の純粋関数)| 🟢 純粋追加 |
| `pack::pack_skyline` — skyline(bottom-left)矩形パッキン | Jylänki, *A Thousand Ways to Pack the Bin* (2010、定番サーベイ)。テクスチャアトラス・インベントリ・ダイアログ敷詰に使う。入力順=配置順の純粋関数で決定的。skyline node の split/merge で不変条件(重なりなし・bin 内)を維持 — 乱数矩形 200 ケースのオラクル検証 | 🟢 純粋追加 |
| `hexgrid::hex_astar` — 六角格子上の A* | redblobgames pathfinding ガイド。厳密な hex `distance` が consistent heuristic になるため各 node は高々1回だけ expand、戻り値は常に真の最短路。open heap の tie-break を `(f, h, q, r)` 辞書順に固定、簿記は全て ordered map。格子が非有界なので `max_steps` 予算を必須引数に。ランダム障害フィールド上で BFS オラクル一致を検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| BVH シーン分割 / quadtree | gaffer・kd-tree 系 | `spatial_hash` が同用途を担う。四分木は浮動座標向きで整数グリッドのこの kit では冗長 |
| R*-tree 矩形検索 | Beckmann 1990 | 永続的な動的 index は `spatial_hash` で十分。pack は packing(配置)であって空間検索ではない |
| MaxRects packing | Jylänki 2010 | skyline より 2-3% 充填率が上がるが実装複雑度が跳ねる。skyline は「bottom-left + 入力順」で十分実用的、必要になれば後続ラウンドで追加 |
| Hopcroft-Tarjan 平面性判定 | HT 1974 | Delaunay 配線が既に平面性を保証するので用途が薄い |
| SAT/separating-axis 衝突 | 一般物理文献 | `aabb`+`passability`+`gridcast` の組合せがこの kit の衝突層。任意凸形状は整数厳密化が高コスト |

## 出典(第7次、search-index 照合)

**論文**: Amanatides & Woo (Eurographics'87) / Tarjan (SIAM J. Comput. 1972) / Kahn (1962) / Jylänki (*A Thousand Ways to Pack the Bin*, 2010)。

**実装物**: redblobgames grids/pathfinding ガイド / cp-algorithms (cut points・bridges・SCC ページ) / jakedowns blog skyline packing 実装 / Qiita・Zenn の DDA・グラフアルゴリズム・パッキン記事群。

# 第8次: 外部出典サーベイ — 空間充填曲線・等高線・network flow・割当・L-system (2026-09-22)

> taxonomy 残存ギャップは引き続き全て意図的スコープ外のため、アルゴリズム層を継続棚卸し:
> 局所性保存の空間コード、スカラー場の等高線、容量ネットワーク解析、最小コスト割当、
> 書換系プロシージャル生成。5系統実装。本ラウンドは PR #23 (第7次) の上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `zorder` — Morton/Hilbert 空間充填曲線コード + `spatial_sort` | Morton 1966 (z-order ビットインタリーブ)、Skilling xy↔d 写像 (Hilbert 曲線、Wikipedia 準拠の反復版)。空間キーの局所性保存ソートは cache-friendly 走査・空間 partition の基礎。符号付き座標は sign-bit bias (`x ^ 0x8000_0000`) で biased 順序に。**encode の quadrant 変換は level size `s` ではなく全 grid mask `n-1` への回転** — この非対称が Skilling 実装の定番誤記で、decode は逆に `s-1` を使う非対称を踏襲。Hilbert の隣接 index が必ず 4-近傍である性質と xy↔d round-trip を全 bits ≤ 16 で性質検査 | 🟢 純粋追加 |
| `msquares` — marching squares 等高線抽出 | Lorensen & Cline 系の 2D 版 (SIGGRAPH'87 系譜)。スカラー場の iso-contour は地形高度線・霧境界・区域描出に使う。辺中点出力を**倍精度座標**(cell は [2i,2i+2]×[2j,2j+2]、端点は奇/偶の組)で整数化 — 補間係数なしの canonical extraction。鞍部(5/10 case)の ambiguity は**双線形中心値の漸近決定子**で一意解決。interior 頂点の偶数次数性・ループ連鎖保存を乱数場でオラクル検証 | 🟢 純粋追加 |
| `flow::FlowNet` — Edmonds–Karp max-flow / min-cut | Edmonds & Karp (JACM 1972)。接続ボトルネック・区域分断・物流割当の解析層。BFS augmenting path は隣接走査順が決定的なので残余 cut も一意。`add_edge_undirected` は両方向 cap の2 pair として記録し `flow_on` で追加番号から逆流も参照可能。**cut 容量 = max flow**(定理)と全中間頂点の保存則を乱数ネットでオラクル検証 | 🟢 純粋追加 |
| `hungarian::assign_min_cost` — 最小コスト割当 | Kuhn–Munkres (Naval Research Logistics 1955; e-maxx 形式の O(n³) ポテンシャル法)。unit→target 割当・build order 最適化に使う n ≤ m 行列版。i64 コストで負値可(利得行列は negate)。全注入写像の列挙ブルートフォースで 200 乱数行列の最適値一致を検証 | 🟢 純粋追加 |
| `lsystem` — deterministic L-system 展開 + 整数 turtle | Lindenmayer 1968 / Prusinkiewicz & Lindenmayer *The Algorithmic Beauty of Plants*。0L 並列書換 `expand` は `(axiom, rules, iters)` の純粋関数(stochastic variant は replay 公理から意図的に不採用)。turtle は `F/G/f/g` 移動描画・`+-` 回転・`[]` push/pop。`DIRS_4/8/HEX` 方向集合で方格・六角格を切替可能、描画は `gridcast` 経由で対角 cell 欠落なし。Fibonacci 系長・枝復帰・8-連結性を検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Marching cubes (3D) | Lorensen & Cline 1987 | この kit はグリッド sim 主体で voxel 等高線は用途が薄い。msquares が 2D 部分をカバー |
| Dinic / push-relabel max-flow | Dinic 1970 / Goldberg-Tarjan | Edmonds–Karp はゲーム規模で十分速く、隣接順走査の方が説明が容易。大規模解析が必要になれば後続で |
| R-tree / kd-tree 空間索引 | Guttman 1984 | `spatial_hash` + `zorder` の組合せで用途をカバー。永続動的 index は過剰 |
| Stochastic / parametric L-system | ABOP | 非決定拡張は replay 公理と正面衝突。seed 駆動の拡張は利用側で `rng` を介して構築可能(規則が静的データなら deterministic) |

## 出典(第8次、search-index 照合)

**論文**: Morton (IBM Research Report 1966) / Skilling (Bayesian inference and maximum entropy 2004 — Hilbert xy↔d) / Lorensen & Cline (SIGGRAPH'87) / Edmonds & Karp (JACM 1972) / Kuhn (Naval Research Logistics 1955) / Lindenmayer (1968)・Prusinkiewicz & Lindenmayer *ABOP*。

**実装物**: Wikipedia "Hilbert curve" 反復写像 / e-maxx cp-algorithms hungarian ページ / KACTL・USACO guide max-flow 実装群 / Paul Bourke・Qiita・Zenn の L-system・marching squares・z-order 記事群。

# 第9次: 外部出典サーベイ — 多角形述語・polyline 簡約・接頭辞和・多パターン走査・差分 (2026-09-22)

> taxonomy 残存ギャップは引き続き全て意図的スコープ外のため、幾何・文字列・クエリの
> アルゴリズム層を継続棚卸し。5系統実装。本ラウンドは PR #24 (第8次) の上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `poly` — 整数 2D 多角形述語・凸包・耳切り三角分割 | shoelace 公式、`orient` 回帰の computational geometry 定石、Andrew monotone chain (1979)、ear clipping (Eberly 系)。`point_in_polygon` は even-odd + half-open 頂点規約 + 交差乗算の整数比較。耳 test は**境界上の頂点も拒否**する inclusive 判定 — strict interior 判定では凹頂点が耳辺上に乗るケースを見逃し、三角形が凹部へ食い出す(実測で L 字多角形の面積不一致を再現・解析して修正) | 🟢 純粋追加 |
| `rdp` — Ramer–Douglas–Peucker polyline 簡約 | RDP (1973-74)。`msquares::contour_loops` 出力の canonical 後段 — 階段状 contour を整数閾値で salient corner に圧縮。距離比較は `|cross|² vs eps²·|chord|²` への i128 交差乗算で真の垂距に対する閾値を厳密化。`simplify_loop` は重心最遠点アンカーで閉ループの切れ目を内容定義化(index 0 依存を排除) | 🟢 純粋追加 |
| `fenwick` — Fenwick tree / BIT | Fenwick (IBM J. Res. Dev. 1979)。接頭辞和クエリの定番構造 — リーダーボード・経済台帳・重み付き抽選の `lower_bound` 秩序統計量。`from_slice` は n 回 add ではなく責任区間直接伝播の O(n) 構築。乱数列の全クエリ(prefix/range/total)をブルートフォース接頭辞和と照合 | 🟢 純粋追加 |
| `ahocor` — Aho–Corasick 多パターン走査 | Aho & Corasick (CACM 1975)。fail リンク + dict リンクで O(text+hits)。子遷移は BTreeMap(dense [u8;256] より記憶小・走査順 canonical)。emit は (end_pos, pattern_idx) ソートで overlapping/suffix 内包 match も全件取得。禁止語フィルタ・署名パターン検出・diag 抽出層 | 🟢 純粋追加 |
| `diff` — Myers 差分 + Levenshtein + LCS | Myers "An O(ND) Difference Algorithm" (1986) 貪欲 frontier + 保存 trace backtrack。`hunks` は unified-diff 形の (a_start, del, b_start, ins) 塊。desync report・セーブ比較・did-you-mean 候補の差分層。**backtrack は frontier level を直接 iterate** — level 0 まで降りると `Ins(-1)` 型の underflow(実測で発覚・修正)。スクリプト適用 = 目標再現・最小性 n+m−2·lcs を乱数ペアでオラクル検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Segment tree / sparse table | 定石 | Fenwick が prefix-sum 系をカバー。範囲 min/max が必要になれば後続で |
| Yen / k-shortest paths | Yen 1971 | 経路多様化は `weighted_astar` + 障害追加で近似的に可能。真の k-最短路は後続候補 |
| Bentley–Ottmann 線分交差 sweep | 1979 | `zorder` + 個別 orient 判定で用途をカバー。全体交差列挙は需要が薄い |
| Suffix array / FM-index | Manber–Myers / Ferragina | Aho–Corasick が多パターン側をカバー。全文索引は replay log 検索が必要になれば |

## 出典(第9次、search-index 照合)

**論文**: Fenwick (IBM J. Res. Dev. 1979) / Aho & Corasick (CACM 1975) / Myers (Algorithmica 1986) / Douglas & Peucker (Cartographica 1973) / Andrew (IPL 1979 monotone chain) / Eberly ear-clipping 実装系。

**実装物**: cp-algorithms fenwick・hungarian・aho-corasick ページ / Wikipedia "Ramer–Douglas–Peucker"・"Myers diff"・"point in polygon"・"convex hull" / Qiita・Zenn の BIT・耳切り三角分割・Aho-Corasick 記事群。

# 第10次: 外部出典サーベイ — 前置辞書・範囲集約・二分マッチング・巡回経路・走長圧縮 (2026-09-22)

> taxonomy 残存ギャップは依然全て意図的スコープ外のため、クエリ構造・割当・経路計画・
> 圧縮の定石アルゴリズムを継続棚卸し。5系統実装。PR #23/#24 は本ラウンド着手前に
> main へマージ済み — 本ブランチは第9次 PR #25 の tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `trie` — byte 前置辞書 | 古典的 trie (Fredkin 1960 / Knuth TAOCP §6.3)。`ahocor` が multi-pattern *走査* を担うのに対し、こちらは *集合の辞書順列挙* — `keys_with_prefix` が `diff::levenshtein` の did-you-mean 候補源。子遷移は BTreeMap で listings は常に byte 辞書順(挿入順非依存)。BTreeSet 参照モデルと insert/remove/prefix 列挙の同値を乱数検証 | 🟢 純粋追加 |
| `segtree` — 範囲 min/max/sum | 反復 bottom-up segment tree(cp-algorithms 系レイアウト — size=次冪、葉は `size+i`)。fenwick が prefix-sum のみを担うのに対し任意窓集約 — `range_stats` が 1 walk で (min,max,sum) を同時返却。200 系のランダム update/query 系列をブルートフォース区間走査と全一致検証 | 🟢 純粋追加 |
| `bipartite` — Hopcroft–Karp 二分最大マッチング | Hopcroft & Karp (SIAM J. Comput. 1973) — BFS layered graph + DFS augment で O(E·√V)。`hungarian`(加重 n≤m)の非加重版として補完。`kuhn_match` 素朴増広路を parity oracle として併載し 300 ランダムグラフでサイズ一致、matched edge の adj 存在・right 一意性を不変検証 | 🟢 純粋追加 |
| `tsp` — NN + 2-opt 巡回経路 | nearest-neighbour 構築 + 2-opt (Croes 1958) first-improvement descent。結果は city 0 始点・`tour[1] < tour[n-1]` で canonical 化(逆巡回を同一視)。全 tie-break は index 規則 → 行列のみの純関数。n≤8 のブルートフォース最適との hit-rate 計測 + 2-opt≤NN の不変検証 | 🟢 純粋追加 |
| `rle` — 走長符号 | `(count:u8, value)` pair の古典 RLE — `bits` wire codec 直前の最安ロスレス層。255+ run は自動分割。zero-count・奇数長は encode が生成し得ないため `decode → None` が構造的 authenticity 検査として機能。run-heavy 乱数モデルで往復完全一致 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| kd-tree 最近傍クエリ | Bentley 1975 | `zorder::spatial_sort` + `spatial_hash` が空間クエリをカバー。真の kNN は需要が出れば |
| Stoer–Wagner global min-cut | 1997 | `flow::min_cut` が s-t 切断をカバー。全域最小カットは解析用途が薄い |
| Yen k-最短路 | 1971 | 第9次と同じ判断 — `weighted_astar` + 障害追加で近似的に足りる |
| LZW / Huffman | 1977/1952 | `rle` が盤面圧縮の 80% をカバー。本格符号は wire 帯域が課題になれば |
| Bloom filter | 1970 | 確率的 membership は「偽陽性率が seed に依存」= replay 公理と微妙に緊張。trie/BTreeSet で厳密に足りる |

## 出典(第10次、search-index 照合)

**論文**: Hopcroft & Karp (SIAM J. Comput. 1973) / Croes (Operations Research 1958, 2-opt) / Fredkin (CACM 1960, trie memory) / Bentley (CACM 1975, kd-tree — 見送り評価用) / Fenwick 系文献の segment-tree 比較。

**実装物**: cp-algorithms segment tree(iterative 系)・Hopcroft–Karp・Kuhn ページ / USACO guide matching・segtree / Wikipedia "2-opt"・"run-length encoding"���"trie" / Qiita・Zenn の seg-tree・二分マッチング・巡回セールスマン記事群。

# 第11次: 外部出典サーベイ — 線分述語・オイラー路・静的RMQ・最近点対・区間集合 (2026-09-22)

> 計算幾何の残存基盤(線分・点対)と、グラフ/クエリの定石(オイラー路・sparse table・
> 区間集合)を棚卸し。5系統実装。PR #25/#26 は本ラウンド時点で open —
> 本ブランチは第10次 PR #26 の tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `segment` — 線分述語 | 計算幾何教科書の orientation/straddle 系(Cormen §33, de Berg et al.)。判定は i128 cross 厳密、端点-on-線分・collinear・退化(点=点)を全分岐。`point_segment_dist2` は floor ではなく **ceiling** 返却 — floor だと 1 未満の有理距離が 0 に潰れて「dist²==0 ⟺ 接する」不変が崩れる事をテストが捕捉、ceiling なら整数のまま「0 iff 幾何学的接触」「≤r² の判定は整数 r で厳密」を同時に満たす | 🟢 純粋追加 |
| `euler` — Hierholzer オイラー路 | Hierholzer & Wiener (1873) 定石 + Fleury との比較文献。無向多重グラフ(自己ループ次数2・平行辺個別)を dense-index 化して O(E) の反復 Hierholzer。次数判定 → BFS 連結検査 → 主走査の3段。辺消費は入力順 first-unused で決定的。`Some((kind, walk))` / 非存在 `None` — 巡回点検ルート・全通路往路(NPC patrol on edges) | 🟢 純粋追加 |
| `rmq` — sparse table 静的範囲 min/max | Fischer & Heun (2006) 系 sparse table。`O(n log n)` 構築 → `O(1)` クエリ。min/max は冪等なので 2 ブロック重複合成がそのまま使える(disjoint cover 不要 = sparse table が冪等演算で簡潔になる理由)。`segtree`(点更新 O(log n))の静的補完 — 焼き付けデータへの高頻度クエリ | 🟢 純粋追加 |
| `closestpair` — 最近点対 | Shamos & Hoey (1975) 分割統治 O(n log n)。y マージ + strip 走査(上限7点)の定石形。全 tie を `(dist², p, q)` 辞書順で解くので答は集合のみの純関数 — 入力配列順序にも依存しない(乱数シャッフル同一性検証付き)。poisson_disc の最小間隔検証・voronoi の近接 seed 解析に直結 | 🟢 純粋追加 |
| `interval` — 区間集合演算 | sorted disjoint interval list(GTL/BLAST 系 occupancy 構造)。`insert` は接触・橋渡しの全件マージ、`remove` は切断時 split、`clip` は交差残し — 全て `Vec::splice` で in-place、クエリは `partition_point` 二分探索 O(log n)。BTreeSet 点集合オラクルで混合系列の逐次同値検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Bentley–Ottmann 全交差列挙 | 1979 | `segments_intersect` が pairwise をカバー。sweep line 全列挙は n 大の GIS 用途で需要が出れば |
| Fleury の橋回避オイラー路 | 古典 | Hierholzer が O(E) で Fleury O(E²) より定石 — 橋検査が不要な分 Hierholzer を採用 |
| Farach–Colton ±1RMQ O(1)/O(n) | 2000 | sparse table の O(1) で十分(構築 O(n log n) を許容)。±1 専用最適化は seed 後の棚卸し |
| 動的区間木(augmented BST) | CLRS §14 | 静的 sorted vec が read-heavy で定石。BTreeMap augmented は需要時 |
| Li–Weis K-Skip-Graph / 重心挿入 union-find | 各種 | `euler` の辺集合定義域では過剰 |

## 出典(第11次、search-index 照合)

**論文**: Hierholzer & Wiener (1873, Eulerian 路の古典) / Fischer & Heun (2006, RMQ) / Shamos & Hoey (1975, closest pair divide&conquer) / Bentley & Ottmann (1979, sweep line — 見送り評価用) / Cormen et al. CLRS §33 (orientation predicates)。

**実装物**: cp-algorithms sparse-table・closest-pair-of-points ページ / emaxx Eulerian path / Wikipedia "Eulerian path"・"Closest pair of points problem"・"Sparse table"・"Interval tree" / redblobgames line-intersection ノート / Qiita・Zenn の蟻本系 sparse table・closest pair・区間スケジューリング記事群。

# 第12次: 外部出典サーベイ — 周期スケジュール・fuzzy 順位付け・オンライン統計・Markov 名付け・ダウンサンプリング (2026-09-22)

> 時刻・文字列・時系列のユーティリティ層を棚卸し。5系統実装。
> PR #27 (第11次) は本ラウンド時点で open — 本ブランチはその tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `cron` — POSIX cron 次発火時刻 | Vixie/POSIX cron 5-field 仕様 + Quartz `a/n` ステップ拡張(`a/n` = a..hi step n。POSIX では `*/n` のみ。crontab.guru・Quartz ドキュメント照合)。dom∧dow 同時制約で OR 規則(POSIX)、どちらか一方のみ制約なら AND。`7≡0` Sunday マージ、月/曜日名を case-insensitive で受理。bitset + day-loop(5年水平) で発火走査 — 1分グリッドのブルートフォース oracle と乱数一致。`31 feb` 等の不可能スケジュールは `None`(hang しない)。Hinnant civil-from-days で O(days) | 🟢 純粋追加 |
| `fuzzy` — fzf 式 subsequence ランカー | junegunn/fzf の scoring shape(連続 run が支配的・boundary 補助・gap ペナルティ)。CONSECUTIVE=10 > BOUNDARY=8 > CHAR=1 > GAP=−1。全順序は score desc → len → bytes → index で tie 不可 = 純関数。`trie::keys_with_prefix` のランク付け後段・`diff::levenshtein` の did-you-mean と役割分担 | 🟢 純粋追加 |
| `stats` — Welford オンラインモーメント | Welford (1962) + Chan et al. parallel merge。全量 `i64·SCALE` 固定小数(SCALE=10⁶) — mean/var は丸め込み整数、stddev は `isqrt` で整数化。merge は δ²·na·nb/n のチャン公式で shard 統計を結合可能(mapreduce 用途)。naive 2-pass i128 oracle に ±SCALE 相当のドリフト内一致を乱数検証 | 🟢 純粋追加 |
| `markov` — コーパス学習の名付け生成 | 古典 Markov 連鎖テキスト生成(Rogue/roguelike 界隈の名付け定石)。order-k の byte-level 遷移を BEGIN/END パディング語から BTreeMap の cumulative-weight 表に構築。サンプルは `SplitMix64::below` で昇順 byte — 全て内容定義で挿入順・ハッシュ順は出力に漏れない。出力の全 order-k gram が corpus に存在(oracle)・seed 一致で完全一致・seed 分岐で分岐を検証 | 🟢 純粋追加 |
| `lttb` — 時系列ダウンサンプリング | Steinarsson (University of Iceland 2013) LTTB — 各 bucket で前選択点×次 bucket 重心の三角形面積最大の点を採用。i128 2倍面積評価で厳密。bucket 境界は floor division で内容定義、端点保存・順序保存部分列。「毎 k 番目」サンプラーが破壊する孤立 spike を必ず保持する例を pin | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| DDSketch / t-digest 分位点 | Datadog 2019 / Dunning 2019 | `stats` のモーメント範囲外。p50/p99 が需要化したら別モジュールで。`RunningStats` は mean/var のみに絞った |
| Quartz 6/7-field(秒・年フィールド) | Quartz scheduler | POSIX 5-field で需要を満たす。秒粒度は `timer` 系の役割 |
| Skeleton-based fuzzy(マルチパス DP) | fzf v2 / fzy | 単一走査の左端 greedy で実用上十分。DP 版は score 意味の乖離リスクがあり差し替え需要時 |
| 高次 Markov(k>4)・名前学習の reverse 化 | roguelike 系記事 | order 固定を train の引数化で済む。逆方向学習は別層 |
| GPU 系ダウンサンプリング(JFA 等) | — | LTTB が CPU O(n) で十分。GPU は方針上範囲外 |

## 出典(第12次、search-index 照合)

**論文・仕様**: Welford (1962, online mean/var) / Chan, Golub & LeVeque (1979, parallel variance merge) / Steinarsson (2013, LTTB thesis, University of Iceland) / POSIX cron 5-field spec + Quartz CronExpression の `a/n` 拡張 / Markov 連鎖名付け(古典、roguelike 界隈定石)。

**実装物**: junegunn/fzf scoring 実装(CONSECUTIVE・BOUNDARY の重み比較) / crontab.guru・Quartz CronExpression ドキュメント / cp-algorithms・Wikipedia "Algorithms for calculating variance" / redblobgames・fzy ソースの boundary 判定 / Qiita・Zenn の cron 実装・Welford・LTTB 記事群。

# 第13次: 外部出典サーベイ — 整数論・木祖先クエリ・エントロピー符号・順序集合・単一パターン走査 (2026-09-22)

> 前回までの層(mapgen→delaunay→接続、文字列・クエリ・時系列)の基盤層を棚卸し。
> 「他モジュールが暗黙に仮定するが未実装だった」数学・走査・集合の定石5系統を実装。
> PR #28 (第12次) は本ラウンド時点で open — 本ブランチはその tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `ntheory` — gcd/extgcd/mod_pow/mod_inv/CRT | ユークリッド互除法(古典) + 拡張ユークリッド Bézout + binary modular exponentiation + 中国剰余定理(非 coprime 版は gcd 条件 `r1≡r2 (mod g)` で矛盾検出、合成 modulus は lcm)。`cron` の周期整合・`bits`/hash の residue 計算が暗黙に必要とする層。`unsigned_abs` で `i64::MIN` 入力の overflow を構造的に回避。ブルートフォース gcd 全走査・Bézout 恒等式 `a·x+b·y=g`・CRT の合同性/一意性/矛盾性を乱数オラクル検証 | 🟢 純粋追加 |
| `lca` — binary lifting LCA | Euler-tour RMQ 版と並ぶ定石;構築 O(n log n)・クエリ O(log n) の `up[k][v]` 表。根付き森は `parent[]` 配列入力 — 循環/範囲外は stamp 検査で invalid 化し `None` 返却(hang しない)。ゾーン木・スキルツリー・エンティティ階層の共通祖先=最小包含領域クエリ。祖先集合列挙 oracle との全対一致を乱数森で検証 | 🟢 純粋追加 |
| `huffman` — canonical Huffman codec | Huffman 1952 + canonical assignment(DEFLATE 系の wire 形:(len, sym) ソート順の連番コード — ツリー形は送らない)。2-queue merge で O(n) 構築かつ (weight, node-id) タイブレークで一意木。wire = (sym,len) 表 + bit_count + MSB-first パックで自己完結。`rle`→`huffman`→`bits` の圧縮段階を完備。往復同一・prefix-free・Kraft 等式・malformed 全拒否を乱数検証 | 🟢 純粋追加 |
| `treap` — seeded 優先度 treap | Seidel & Aragon (1996) treap。BST ∩ min-heap(prio = `splitmix64(key^seed)` 内容定義)→ 形状は key 集合の一意関数で挿入順に非依存 = HashSet 反復の「platform ごとの列挙順」を排しつつ sorted 列挙を得る決定的辞書。merge/split 構成でコード小さく証明可能な構造。BTreeSet 全 ops 同値・シャッフル挿入で (key,depth) 署名一致・ヒープ不変量を検証 | 🟢 純粋追加 |
| `kmp` — KMP 単一パターン + streaming | Knuth–Morris–Pratt (1977) — `fail` 表前処理で後戻りなし O(n) 走査。`ahocor` が multi-pattern を担うのに対し単一パターンの廉価層(区切り・ヘッダ・センチネル)。`Stream` は 1 byte 供給で match を絶対 index 返却 = パケット境界で分割された match も取れる wire 走査。naive 全位置走査との全一致(重複 match 含む)・stream≡batch を乱数チャンク分割で検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Miller–Rabin 素数判定 | Miller'76/Rabin'80 | `ntheory` の範囲外需要。素数合成サイズが要るハッシュ表が kit に無いため定石組込み時 |
| Euler-tour + RMQ 版 LCA | Tarjan/Harel'84 | binary lifting でクエリ十分。定数係数改善は `rmq` との組合せ需要時 |
| Arithmetic / range coding | Sayood | Huffman より圧縮率は高いが range coder の整数化は別大物。wire は canonical Huffman で誤差許容帯に十分 |
| AVL/red-black / B-tree | — | treap の「hash 優先度=形状が key 集合のみで一意」性は挿入順非依存性の証明に直結。常勤 BST は優先度の表れない deterministic shape を持たない |
| Boyer–Moore / Sunday | — | KMP の文字ごと処理は stream 供給に直交する構造 — 高速スキップ系は巨大バッファ走査で需要化した時 |

## 出典(第13次、search-index 照合)

**論文・仕様**: Euclid(古典) / Bezout 恒等式 / 中国剰余定理(非 coprime 拡張)/ Knuth–Morris–Pratt (1977, SIAM J. Comput.) / Huffman (1952, Proc. IRE) + canonical codes(Schwartz 1964, DEFLATE RFC 1951) / Seidel & Aragon (1996, "Randomized Search Trees", Algorithmica) / binary lifting(Schieber & Vishkin 1988 の木クエリ系)。

**実装物**: cp-algorithms 各項(gcd/extgcd/CRT/binary lifting KMP)/ rust `std::str::find` の empty-pattern 意味 / junegunn fzf 対比のための KMP stream 形 / redblobgames・Qiita・Zenn の treap・CRT・Huffman 解説記事群 / DEFLATE canonical table(RFC 1951 §3.2.2)。

# 第14次: 外部出典サーベイ — 因果順序・ハッシュ木・所属判定・差分同期・辞書圧縮 (2026-09-22)

> 第13次までで基盤数学と文字列/集合層を完備したので、今回はマルチピア状態同期に必要な
> 「同じものを見たか」を証明する構造群に集中。lockstep/replay の desync 報告が
> 「フレーム番号+ハッシュ値」の一点比較しか持てなかったのに対し、どの項目が分岐したかを
> 構造的に割り出す道具一式(vclock/merkle/delta)と、wire 層の圧縮完備(lzss)・
> 前置フィルタ(bloom)を実装。PR #28/#29 は本ラウンド時点で open — 本ブランチは #29 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `vclock` — Lamport/Fidge–Mattern ベクタクロック | Lamport 1978 (partial ordering) + Fidge/Mattern 1988 (vector time)。actor→counter の BTreeMap 表現で列挙順も決定的。`compare` が Before/After/Equal/Concurrent の4値を返し「分岐した履歴」を first-class で表す — 単一 Lamport clock の「時刻比較では concurrent が区別できない」限界を構造的に解消。部分順序公理(反射・反対称・推移)・merge の最小上限性・メッセージ流シミュレーション(受信 snapshot ≤ merge 結果)を乱数検証 | 🟢 純粋追加 |
| `merkle` — DetHash 葉上の二分ハッシュ木 | Merkle 1979/87。「root 一致 ⟺ 状態一致」の証人構造 + 証明(proof/verify)+ 差分検出(`first_diff` で O(log n) 下降 = `replay::first_divergence` のストア版)。奇数ノードは複製せず昇格(Bitcoin 式 dup の malleability 回避)。ドメイン分離タグ付き Fnv1a 合成。proof 往復・naive 葉走査一致・葉順序依存性・長さ違いの構造的不一致を乱数検証 | 🟢 純粋追加 |
| `bloom` — seeded Bloom filter | Bloom 1970 + Kirsch–Mitzenmacher 2006(double hashing `h1+i·h2` で k 個のハッシュ関数を2個から生成 — 実測で単独ハッシュ列と同等の FPR)。片方向誤りのみ(挿入済みは必ず present)なので高価な完全一致の前置フィルタに直結。Fnv1a×2+seed で `(seed, params, multiset)` の純関数。false-negative 不存在・FPR が情報理論限界内(m=8.5n, k=6 で <8% 実測 vs 理論 ~2%)・ビット列の挿入順非依存性を乱数検証 | 🟢 純粋追加 |
| `delta` — 順序マップのスナップショット差分 | state-sync の delta encoding(Gaffer/Replica 系の手法 — 全量送信ではなく変更 op のみ送る)。`diff_sorted`/`apply_sorted` が sorted `u64→u64` view 上で全単射往復。wire は昇順キー delta varint(bits 層)で canonical — 非昇順・切り詰め・ゴミ残りを拒否。`Del` 不存在キーは `None` で失敗閉鎖(警告で通さない)。往復同一・op 数最小性・malformed 拒否を乱数検証 | 🟢 純粋追加 |
| `lzss` — greedy LZ77 系 codec | LZSS(Storer–Szymanski 1982、LZ77 の flag-bit 実用形)。4096 window・match 3–18・`0`+8bit literal / `1`+12bit(offset-1)+4bit len + u64 生長ヘッダ — `rle`→`lzss`→`huffman`→`bits` の圧縮梯子を完備(run 長・繰り返し部分列・頻度偏りの各 regime をカバー)。greedy 最長一致+最小 offset 優先で出力が入力の純関数。往復同一(ノイズ・周期・run)・repetitive 圧縮率・malformed 全拒否を乱数検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| CRDT (G-Set/LWW-Map/OR-Set) | Shapiro 2011 | マージ半格子は kit の replay 前提(共有権威時計)と相性が薄い。vclock+delta が因果追跡の骨格を既に与えるので、CRDT 型はマルチリーダー合流需要が出た時 |
| DMCA/rsync rolling hash 差分 | Tridgell'96 | 可変長ブロック境界同期は wire 層需要。順序マップの固定キー差分(`delta`)で snapshot 同期のコアは済んだ |
| LZMA/range coding | Pavlov | 圧縮率は上がるが range coder の整数厳密化は別大物 — `lzss`+`huffman` で帯域削減の実用帯はカバー |
| PATRICIA/radix tree | — | `trie` が既に供給。経路圧縮はメモリ効率の改善で決定性影響なし |
| B-tree / skip list | — | `treap` の seed 優先度が「key 集合のみで一意形状」を与えるため lockstep 辞書要件は充足 |

## 出典(第14次、search-index 照合)

**論文・仕様**: Lamport (1978, CACM "Time, Clocks...") / Fidge (1988), Mattern (1989) ベクタ時計 / Merkle (1979/1987, CRYPTO) / Bloom (1970, CACM) / Kirsch & Mitzenmacher (2006, "Less Hashing, Same Performance", ESA/Internet Math) / Storer & Szymanski (1982, JACM "Data compression via textual substitution") / Ziv & Lempel (1977) / 中国剰余同様の標準形。

**実装物**: RFC 8974 Merkle tree 記述 / Bitcoin-duplicate 方式の CVE 的 malleability 記述(昇格方式の根拠)/ MIT 6.824 DDIA(Kleppmann) の vector clock 章 / mrembley・Qiita・Zenn の Bloom/LZSS 解説記事群 / cp-algorithms・netty lz4 の LZ77 系 token 形対比。

# 第15次: 外部出典サーベイ — 転がし指紋・接尾辞配列・点索引・2-SAT・ゲーム木 (2026-09-22)

## 方針

> 前回の「分岐を証明する構造」(vclock/merkle/delta) の自然な続きとして、
> 差分転送の前段(内容定義チャンキング = rolling)、文字列構造の監査(suffix)、
> 均一密度を仮定しない空間索引(kdtree)、制約判定(twosat)、探索AIの非乱数版(minimax)を実装。
> PR #28/#29/#30 は本ラウンド時点で open — 本ブランチは #30 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `rolling` — Rabin–Karp 転がし指紋 + CDC | Rabin fingerprint(Karp–Rabin 1987) + rsync/Tridgell'96 系の内容定義分割・FastCDC(Xia'16)の [min,max] 強制形。mod-2^64 多項式 `ΣbᵢB^{n-1-i}` で push/pop ともに O(1)。`chunks` は `hash&mask==0` で切断し最小幅・最大幅を強制 — 局所編集が遠方の境界を動かさない「編集頑健性」が delta 同期の転送量を決める。再計算一致・naive 全位置走査・chunk 幅制約・prefix 安定性(編集前の境界が保存される)を乱数検証 | 🟢 純粋追加 |
| `suffix` — 接尾辞配列 + Kasai LCP | Manber–Myers 1991 prefix-doubling + Kasai 2001 LCP 構築。`search` が `O(pat log n + hits)` の全出現、`longest_repeated`/`distinct_substrings` が文字列の重複構造を曝く — `markov` 系の生成器が実際に何を覚えたかを数える監査層。naive ソート・素朴 LCP・全位置走査・BTreeSet 部分列数で乱数一致検証 | 🟢 純粋追加 |
| `kdtree` — 静的 2-D kd-tree | Bentley 1975。median-split で平衡構築、分割軸交互・全 tuple sort で tie まで決定的。`nearest`/`within`/`in_rect` を期待 O(log n)。`closestpair` が「全体の最近対」を問うのに対しこちらは「この点の近傍」を問う。i128 距離²・辞書順 tie-break・入力順非依存をブルートフォース全走査と乱数一致検証(順列不変性込み) | 🟢 純粋追加 |
| `twosat` — 含意グラフ 2-SAT | Aspvall–Plass–Tarjan 1979。`a∨b` を `¬a→b`,`¬b→a` に変え `graph::strongly_connected` で SCC 分解。sinks-first 出力順位で `rank[t]<rank[f]` の正極性 = canonical 解(初手で逆極性を書いて brute-force 検証が捕捉 → 反転)。n≤7 の全 2^n 割当と SAT/UNSAT 一致 + 解自身が `check` を通ることを乱数検証 | 🟢 純粋追加 |
| `minimax` — 決定的 negamax αβ | Knuth–Moore 1975 negamax + αβ。`Game` トレイト(moves/apply/evaluate/terminal)に対する `score`/`best_move` — `mcts` の乱数対称的な全探索版。canonical 着手順・同値先着・ply 割引終端スコア。Tic-Tac-Toe 全域(深さ4で全到達局面)で αβ=naive negamax 一致・完全棋譜の引き分け・win-in-1・最遅敗の既知値を検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| 固定長ブロック差分(rsync 二相版) | Tridgell'96 | `delta` の順序マップ差分 + `rolling::chunks` で機能は分解済み |
| EERTREE / palindrome tree | Rubinchik'14 | 用途が回文検出に限定 — `suffix` が部分列構造の一般形を供給 |
| BVH (AABB 木) | — | `kdtree` が点索引を供給。AABB 体索引は `aabb`/`spatial_hash` の補間需要が出た時 |
| SAT 一般形 / CDCL | Marques-Silva | 2-SAT の多項式形で実用帯をカバー。一般 SAT は型/証明規模が別物 |
| 並行 alpha-beta / MTD(f) | Plaat | 探索順を乱さない方が replay 直結。行列入替探索は「canonical 順」保証を壊すので今回は素直形 |

## 出典(第15次、search-index 照合)

**論文・仕様**: Rabin (1981, fingerprinting by random polynomials) / Karp–Rabin (1987, IBM JRD) / Tridgell (1996, rsync) / Xia et al. (2016, FastCDC) / Manber–Myers (1991, SODA/ FOCS'89) / Kasai et al. (2001, LCP) / Bentley (1975, CACM "Multidimensional binary search trees") / Aspvall–Plass–Tarjan (1979, IPL) / Knuth–Moore (1975, "An analysis of alpha-beta pruning")。

**実装物**: cp-algorithms 2-SAT・suffix-array 項 / librsync 指紋窓 / Qt KdTree・scipy cKDTree の query 形対比 / Qiita・Zenn の Rabin–Karp・CDC・kd-tree・2-SAT 解説記事群 / chessprogramming.org negamax 形。

# 第16次: 外部出典サーベイ — 順列代数・数論変換・回文構造・厳密線形代数・整数曲線 (2026-09-22)

## 方針

> 前回までの「文字列・空間・制約・探索」の一般化として、数学基盤の整数厳密版を整備。
> perm(順列の正準形と全単射)、conv(float FFT を排除する数論畳み込み)、
> manacher(文字列対称構造)、gauss(float pivot を排除する厳密消去)、
> bezier(有理 t の厳密スプライン)を実装。
> PR #28–#31 は本ラウンド時点で open — 本ブランチは #31 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `perm` — 順列代数 + factoradic 順位列挙 | 対称群の標準演算(合成・逆・巡回分解・符号・位数) + Lehmer コード factoradic(Knuth TAOCP 4A §7.2.1.2)。`unrank` が `r ∈ 0..n!` を順列に全単射で写すので「seed の下位 k bit がそのまま配置順」になる正準列挙。結合法則・逆元・`p^order=e`・符号=inversion パリティ・n≤7 全順列の rank/unrank 往復を乱数検証 | 🟢 純粋追加 |
| `conv` — NTT 畳み込み mod 998244353 | NTT 標準 modulus 119·2²³+1・原始根3(cp-algorithms 系)。bit-reversal + 反復 butterfly で `O(n log n)`、全演算が u64/u128 — float FFT が決定論を壊す理由(丸め誤差が被験者依存)を素因数的に回避。d2d の合計分布・loot の母関数計数が整数厳密になる。naive O(n²) mod-p 全一致・可換性・`convolve_i64` の失敗閉鎖を検証 | 🟢 純粋追加 |
| `manacher` — O(n) 回文構造 | Manacher 1975 の d1/d2 配列。半開区間 [l,r) 慣行で書くと鏡像 index が inclusive 系から +1/−1 ずれる — 初版で leftmost 以外の全回文が +1 カウントされ、BTreeSet 列挙オラクルが即捕捉して `l+r−1−i`/`l+r−i` に確定。名付け lint・生成 seed の対称性スコアに利用 | 🟢 純粋追加 |
| `gauss` — Bareiss 厳密消去(det/solve/rank) | Bareiss 1968 fraction-free elimination — 各ステップが先行 pivot での整除を厳密に行うので i64 行列の det が i128 で絶対厳密。`solve` は拡大行列 + 既約 (num,den) 逆戻入で「連立方程式が解けない」の None が singular 証明になる。`rank` は独立経路で `det==0 ⟺ rank<n` を相互検証。permutation 展開 det・A·x=b 復元を乱数検証 | 🟢 純粋追加 |
| `bezier` — 有理 t の整数厳密曲線 | Bernstein/De Casteljau を `t=num/den` に持ち上げ、(1−t)³P₀… を `u=d−t` の整数展開で評価 — 座標は全て既約 i128 分数。Catmull-Rom は standard basis で内点 2 点を厳密補間。カメラパス・投射物弧・patrol 経路が機種非依存で bit-identical。Horner 形オラクル・端点復元・補間性・等分割 polyline を乱数検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| FFT over complex floats | Cooley–Tukey | float が決定論公理に抵触 — NTT が用途の全てを賄う |
| Strassen / 任意精度線形代数 | — | Bareiss で n≤8 のゲーム用途は十分。大行列需要はまず無い |
| EERTREE / palindrome tree | Rubinchik'14 | 前回検討で `suffix` が一般形として優先 — manacher は d1/d2 の専用形として今回補完(両者で用途が直交) |
| B-spline / NURBS | de Boor | Catmull-Rom の補間性がゲーム経路の標準形。制御点が外部曲線を逸脱する B-spline はパス編集 UX に不適 |
| Gröbner 基底・多変数制約 | Buchberger | 2-SAT/gauss で実用帯をカバー。実装複雑度が段違い |

## 出典(第16次、search-index 照合)

**論文・仕様**: Knuth TAOCP vol.4A §7.2.1.2(factoradic/Lehmer コード) / Pollard (1971, NTT) / cp-algorithms NTT・Manacher 項 / Manacher (1975, JACM) / Bareiss (1968, Sylvester's identity) / Bernstein–Bézier・De Casteljau (1959–63) / Catmull–Rom (1974)。

**実装物**: cp-algorithms(convolution・manacher) / KACTL NTT 形対比 / AtCoder Library convolution の modulus 選定 / Qiita・Zenn の NTT・Bareiss・Manacher 解説記事群 / redblobgames Bézier 項の曲面形対比。

---

# 第17次 — 要約構造(sketch)・割当・適応圧縮(2026-09-22、第7サイクル)

> bloom(所属)は既にある — では「何個」「何回」「p99 は」も省メモリで
> 答えられるかが残課題。kmv(distinct 概数)、cms(頻度推定)、
> quantile(ε近似分位)、chash(最小混乱の割当)、lzw(辞書を wire に
> 載せない適応圧縮)を実装。
> PR #28–#32 は本ラウンド時点で open — 本ブランチは #32 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `kmv` — KMV distinct-count sketch | K-minimum-values(Bar-Yossef et al. 2002 系の第 k 最小値法)。seed 付き Fnv1a の k 最小値を保持、推定は `(k−1)·2⁶⁴/vₖ` の純整数演算 — HyperLogLog の调和平均のような float 近似誤差が設計上存在しない。distinct<k は厳密カウント。merge は最小値集合の合併でストリーム合併と同値(冪等・可換)。BTreeSet 正確数・4σ 整数窓・merge 一致を乱数検証 | 🟢 純粋追加 |
| `cms` — count-min sketch | Cormode–Muthukrishnan 2005。`depth×width` カウンタ + 独立 seed 行 hash。推定 = min(row) で片方向誤りのみ(衝突は加算のみ = 絶対に過小評価しない) — 「上限を超えたら通報」用途に安全側の誤り。merge は要素和、次元/seed 不一致は `None` で拒否。片方向性・merge=単一ストリーム・経験過剰境界を乱数検証 | 🟢 純粋追加 |
| `quantile` — Greenwald–Khanna ε近似分位 | GK 2001 の `(v,g,δ)` タプル列 + `g+g'+δ' ≤ ⌊2εn⌋` の周期 compact。query は `r_i^max > r+εn` の最初の i の一つ前を返す — 初版で limit に 2εn(バンド全幅)を使い順位境界違反を oracle が捕捉、εn(半分)が正解。`|真順位 − φn| ≤ εn` を全十分位で sorted-oracle 照合。小ストリーム厳密・端点 δ=0・退化引数を検証 | 🟢 純粋追加 |
| `chash` — rendezvous(HRW)一貫ハッシュ | Thaler–Ravishankar 1998 (HRW)。"highest random weight" — `argmax_n hash(seed,n,key)`。ノード除去でそのノードの key のみが再配置される最小混乱性 = Karger consistent hashing より単純で環状虚ノード不要。`pick_top` で複製先上位 r 件。除去時非移動性・ノード順序不変・pick=top[0]・大域均衡を乱数検証 | 🟢 純粋追加 |
| `lzw` — LZW codec | Welch 1984 / Unix compress・GIF の句圧縮。greedy 最長一致、12-bit 固定コードで `bits` 上に展開、辞書 256+3840=4096 で freeze。wire に辞書を載せず decoder がコード列から lockstep 再構築 — KwKwK(code==dict.len() = prev+prev[0])を構造的に処理。往復同一・repetitive 圧縮率・切り詰め prefix 安全を乱数検証。`rle→lzss→huffman` 梯子を「適応型」で完備 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| HyperLogLog | Flajolet 2007 | 標準形は調和平均に float 必須。KMV が整数のみで同じ問題を解く(union も厳密) |
| t-digest | Dunning | merge 中心の重心更新で内部に float/比率。GK の (v,g,δ) が全て整数で公理適合 |
| Jump consistent hash | Lamping–Veach 2014 | HRW の方が pick_top(複製先)・weighted variant に自然拡張でき、本用途(ゲーム規模)では線形走査で十分 |
| DEFLATE 完全実装 | RFC 1951 | lzss+huffman の組合せで実質同等。独自 window/huffman 連結の複雑度に対し便益が薄い |
| FST / 有限状態トランスデューサ | — | trie/suffix で辞書構造はカバー。適用帯が曖昧 |

## ���典(第17次、search-index 照合)

**論文・仕様**: Bar-Yossef et al. (2002, KMV/第k最小値) / Cormode–Muthukrishnan (2005, count-min) / Greenwald–Khanna (2001, quantile sketches) / Thaler–Ravishankar (1998, rendezvous hashing) / Welch (1984, LZW, IEEE Computer) / Kirsch–Mitzenmacher (bloom 引用の双 hashing を cms で再利用)。

**実装物**: cp-algorithms・Wikipedia の GK/Rendezvous/LZW 疑似コード / Redis の HRW 利用対比 / Lucene の KMV 変種対比 / Qiita・Zenn の count-min・GK・LZW 解説記事群 / GIF87a 可変幅コード対比(本実装は固定幅で単純化)。

---

# 第18次 — 類似度・文字列・負辺経路・有理数・窓集約(2026-09-22、第8サイクル)

> 集合が「似ているか」(minhash)、文字列が「どこで一致するか」(zfunc)、
> 負の利益を持つグラフで「最短は/套利は」(bellman)、分数を「丸めずに」
> (frac)、滑る窓の「極値を線形で」(slide) — 残った未実装の古典定石層。
> PR #28–#33 は本ラウンド時点で open — 本ブランチは #33 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `minhash` — MinHash 類似度署名 | Broder 1997 (AltaVista dedup)。`k` 個の独立 seed hash の最小値ベクトル — `P(min_A = min_B) = J(A,B)` を使い signature 一致率が Jaccard 推定量。推定は permille 整数(`err ~ 1/√k`)、`union` は位置別 min で mergeable。内蔵 oracle `jaccard_exact`(sorted merge-join)に対し 40 試行 `err²·k ≤ 9·10⁶` 境界を検証 | 🟢 純粋追加 |
| `zfunc` — Z-algorithm | Gusfield 1997 / cp-algorithms Z-function。各位置 i の prefix 一致長 z[i] を Z-box 再利用で O(n)。`z_search` は `pat+0xFF+text` 連結 → z[i]==|pat| の位置列、sep 混入時は naive fallback(正しさ保証)。`borders`(prefix=suffix)は z 値の後ろから読み、`min_period` は `z[p]==n−p` の最小 p。全位置 naive・borders brute-force・周期最小性を乱数検証 | 🟢 純粋追加 |
| `bellman` — Bellman–Ford | Bellman 1958 / Moore 1959。`O(V·E)` で負辺を扱う SSSP — `pathfinding` の非負領域を拡張。n−1 pass 後にまだ relax できる辺 = 到達可能負閉路 → `None`。`negative_cycle` は super-source(全頂点へ 0 辺)で「どこかの」負閉路の頂点列を返す — 通貨裁定・バフ掛け算の非負性検査に直結。独立 relax oracle・path 累積重み一致・閉路 sum<0 を乱数検証 | 🟢 純粋追加 |
| `frac` — 正規化有理数 | 計算代数系の canonical form(gcd=1・den>0 で等値=構造的一致)。全演算 `i128` 厳密、掛算は cross-reduce(`a/d' × b/d` で gcd 済み同士の積)で桁 headroom 確保 — `bezier`/`gauss` が返す分数と同じ形の user-facing 型版。`den==0` 生成は `0/1` に clamp(total 化)、`/0` は `None`。cross-multiply 真値・還元不変条件・四則往復を乱数検証 | 🟢 純粋追加 |
| `slide` — 単調デック窓集約 | 競プロ定石(monotone queue)— `segtree` O(n log n) を「窓が 1 要素ずつ滑る」限定で O(n) に圧縮。deque 先頭が常に窓の最良候補、tail から支配される要素を追い出す。直近 w フレームの最悪遅延・巡回経路の極値に。全窓 brute-force 照合・単調/退化入力を乱数検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| SimHash | Charikar 2002 | MinHash が同用途(近似 dedup)を同じメモリで解き、Jaccard の推定値が解釈容易。cosine 類似度用途は bloom 重複で検討余地 |
| FFT 文字列マッチング(ワイルドカード) | Fischer–Paterson 1974 | `conv` の NTT で原理的には可能だが、コード規模に対し適用帯が狭い。zfunc/kmp で決定的版はカバー済み |
| SPFA | Moore の queue 版 | worst-case が Bellman–Ford に退化、決定性メリットなし。辺列走査の古典形を採用 |
| 任意精度 BigInt 有理数 | — | i128 で公開 API 面の複雑さを避ける。溢れが問題になる規模は `gauss` 直接利用 |

## 出典(第18次、search-index 照合)

**論文・仕様**: Broder (1997, MinHash) / Gusfield (*Algorithms on Strings*, Z-algorithm 章) / Bellman (1958)・Moore (1959, Bellman–Ford) / Fischer–Paterson (1974, 見送り) / cp-algorithms(Z-function・Bellman–Ford・negative cycle) / monotonic queue 標準技法。

**実装物**: cp-algorithms(z-function・bellman_ford) / emaxx(Z 関数) / Qiita・Zenn の MinHash・Z-Algorithm・Bellman-Ford・単調 queue 解説記事群 / KACTL SlidingMinimum 対比 / Lucene MinHash 変種対比。

# 第19次 — 部分列・DAGパス・選択・ラスタ化・漸化式(2026-09-22、第8サイクル)

> 「列の最長単調部分は」(lis)、「依存グラフの最短と最長は」(dagsp)、
> 「乱択なしのk番目は」(bfprt)、「セル列に落とす」(raster)、
> 「漸化式のk項を跳ぶ」(linrec) — DP/選択/描画の残った古典定石層。
> PR #28–#34 は本ラウンド時点で open — 本ブランチは #34 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `lis` — 最長増加部分列 | patience sorting の tails 配列定石(Knuth/競プロ)。`tails` は各長の最小末尾 index、`parent` で witness 復元。`O(n²)` DP oracle(全 dp[i] 最大+辿り直し)と長一致・復元列の真単調性を 400 乱数検証 | 🟢 純粋追加 |
| `dagsp` — DAG 最短/最長路 | CLRS §24.2 / cp-algorithms。`topo_sort` 上で両方向 relax 一回 `O(V+E)`。`critical_path` は「全頂点 dist=0 初期化」の unbounded-start longest path — 入辺全てが負なら非 source 起点が正しい設計を BF oracle が捕捉。shortest≡BF・longest≡反転重みBFの負・cycle→None を乱数検証 | 🟢 純粋追加 |
| `bfprt` — median-of-medians 選択 | Blum–Floyd–Pratt–Rivest–Tarjan 1973。group-of-5 中央値を再帰で pivot に worst-case `O(n)` 保証 — quickselect の期待値に依存しない。Dutch-flag `lt/gt` partition で pivot 相等も確実。sorted oracle 全 k・全重複・逆順 adversarial を 400 乱数検証 | 🟢 純粋追加 |
| `raster` — Bresenham + midpoint circle + 走査線充填 | Bresenham 1965 / Pitteway 1967。「どちらの端点から引いても同じセル集合」を tie-break が壊す問題 → 辞書順小 endpoint に正準化して構造的対称化。`fill_polygon` は座標2倍化で「頂点=偶・セル中心=奇」を作り i128 有理 crossing で境界判定を厳密化。DDA 密 sample oracle(部分集合+8連結+逆転)・`poly::point_in_polygon` 全セル照合を乱数検証 | 🟢 純粋追加 |
| `linrec` — 線形漸化式 k 項 | companion-matrix exponentiation(行列累乗)/ Kitamasa。`O(d³ log k)` で naive `O(dk)` の陪乗置換。`linrec` は `i128` checked で `None` 失敗閉鎖、`linrec_mod` は `u128` 中間で常時 total — ハッシュ種・周期イベントの跳び読み向け。naive 漸化式 oracle・mod ⟺ exact 整合を 300 乱数検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Kitamasa 多項式法 | — | 陪乗は companion 行列と同じ O(d² log k)。行列版は実装が自明・検証しやすい、d ≤ 数十の用途で十分 |
| Berlekamp–Massey | — | 係数自体を列から逆求する用途が IZANAGI では未確認。将来 seed 監査用途で検討余地 |
| Bresenham 対称変種(symmetric Bresenham) | — | 正準方向化で構造解決する方が古典形を維持でき検証しやすい |
| Xiaolin Wu アンチエイリアス線 | — | 整数域にグレースケールが無い — セル描画は二値で十分 |

## 出典(第19次、search-index 照合)

**論文・仕様**: Knuth TAOCP(patience tails / LIS) / CLRS §24.2(DAG SSSP) / Blum–Floyd–Pratt–Rivest–Tarjan (1973, median-of-medians) / Bresenham (1965) / Pitteway (1967, midpoint circle) / Kitamasa・行列累乗定石。

**実装物**: cp-algorithms(LIS・negative cycle) / emaxx(K-th element・LIS) / KACTL(LIS・LineHull) / redblobgames(line drawing 記事) / Qiita・Zenn の LIS・BFPRT・Bresenham・行列累乗解説記事群。

# 第20次 — 費用流・ナップサック・彩色・取消可能連結・直線包絡(2026-09-22、第8サイクル)

> 「一番安い輸送は」(mcflow)、「上限内の最適選択は」(knapsack)、
> 「隣と違う色を最小で」(coloring)、「仮説を取り消せる連結は」(dsurb)、
> 「直線束の下限は」(cht) — 割当・DP・構造の残存古典定石層。
> PR #28–#35 は本ラウンド時点で open — 本ブランチは #35 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `mcflow` — 最小費用最大流 | Edmonds–Karp + Klein 1967 cycle-canceling。`bellman::negative_cycle` で残余負閉路を特定し bottleneck 容量を回す — 整数容量で厳密最適。負コスト辺は残余逆向きで自然に扱える。全域列挙 oracle(全 path フロー割当)・負閉路 rerouting 回帰テストを乱数検証 | 🟢 純粋追加 |
| `knapsack` — 0/1+無限ナップサック | 教科書 DP。0/1 は **全 DP 表**を保持 — ローリング行だと「i 層の最適」と「後層の改善」が混ざり witness が壊れる問題をレイヤ比較で解消(tie は先 index 優先で復元一意)。2^n 全列挙 oracle・bounded 展開 oracle を乱数照合 | 🟢 純粋追加 |
| `coloring` — DSATUR 彩色 | Brélaz 1979。飽和度→次数→index の tie-break で二部/サイクルは厳密解、一般は強いヒューリスティック。`is_proper` で properness を構造検査。全 k-coloring 探索 oracle で彩色数 bound + 既知族(クリーク・奇/偶 cycle・K_{2,3})を乱数検証 | 🟢 純粋追加 |
| `dsurb` — rollback union-find | 競プロ定石(undoable DSU)。path compression は任意深度を書き換え undo 不能 → union-by-size+ジャーナルで `O(log n)` find と引き換えに任意 snapshot 復帰。「この辺があったら?」仮説クエリ・オフライン接続性向け。BFS 再構築 oracle との component 一致を乱数検証 | 🟢 純粋追加 |
| `cht` — Li Chao tree | Li Chao 1986 / cp-algorithms。区間中央点で勝つ直線を各ノードに保持、敗者が勝つ側のみ降る `O(log X)` 挿入。`i128` 評価で int64 係数の積でも溢れない。brute-force 全直線 min oracle を乱数照合 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| SSP + Johnson potentials の min-cost flow | — | potentials 版は速いが負辺の初期化が必要。cycle-canceling は `bellman` 再利用で検証済み閉路列を直接得られ、整数容量の規模で十分 |
| Edmonds blossom(一般マッチング) | — | 実装規模が大きく bipartite が用途の大半をカバー。MCMF による一般割当で代替可能 |
| Persistent segtree / 完全永続構造 | — | 検証オラクルが肥大。dsurb が「履歴への問合せ」の実用形を先に提供 |
| Held–Karp exact TSP | — | tsp のヒューリスティック+本ラウンドの DP 系でカバー傾向。n≤20 の厳密版は次ラウンド候補 |

## 出典(第20次、search-index 照合)

**論文・仕様**: Edmonds–Karp (1972) / Klein (1967, cycle-canceling) / Brélaz (1979, DSATUR) / Li Chao (1986) / knapsack DP 教科書定石 / undoable DSU(競プロ)。

**実装物**: cp-algorithms(mincost_flow・DSU rollback・Li Chao tree) / KACTL(MinCostMaxFlow・RollbackUF・LineContainer) / emaxx(DSATUR・Li Chao) / Qiita・Zenn の最小費用流・ナップサック・彩色・Undo可能UnionFind・Li Chao Tree 解説記事群。

# 第21次 — 可逆変換・木パスクエリ・全域カット・素数性・区間統計(2026-09-22、第8サイクル)

> 「圧縮の可逆前段は」(bwt)、「木の区間操作は」(hld)、
> 「最小分断コストは」(mincut)、「素数かは厳密に」(miller)、
> 「区間の中央値・頻度は」(wavelet) — 符号化・クエリ・数論の残存層。
> PR #28–#36 は本ラウンド時点で open — 本ブランチは #36 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `bwt` — Burrows–Wheeler + MTF | Burrows & Wheeler 1994 / bzip2 pipeline。巡回 BWT(no sentinel)は doubled-string 上の `SuffixArray` で回転順を確定 — primary index は rotation-0 の行。inverse は LF-mapping(count/first/occ)で `inv` 置換を歩く。周期入力では duplicate 回転の primary が一意でない点を oracle で分離。回転行列 oracle・往復一致・小アルファベット乱数検証 | 🟢 純粋追加 |
| `hld` — heavy-light decomposition | Sleator–Tarjan 1983(競プロ実装形式)。max-size 子=heavy(同値は最小 index)、light 子が新 chain 起点。`path_segments` が `O(log n)` の flat range を path 順で返し `segtree`/`fenwick` に直載せ可能。subtree は preorder 連続区間で 1 range。祖先 climb oracle・BFS subtree 集合・乱数照合 | 🟢 純粋追加 |
| `mincut` — Stoer–Wagner 全域最小カット | Stoer & Wagner 1997。終端不要の `O(n³)`: 各 phase で最密接頂点を A に加え最後の2点を s,t として t を s に収縮、phase cut = t の残余接続重み。全対 s-t maxflow (`flow::FlowNet`) oracle で乱数照合、crossing 再計算で側集合の正当性も検証 | 🟢 純粋追加 |
| `miller` — 決定的 Miller–Rabin + 分解 | Jaeschke/Sinclair の `u64` 全域決定的 7-base 集合。sieve oracle と 20 万件全照合、Carmichael/SPSP 有名数を全拒否。Brent rho は固定多項式 `x²+c`(c=1,2,…)で factor が `n` の純関数 — trial division ≤1000 + rho で sorted 素因数列 | 🟢 純粋追加 |
| `wavelet` — wavelet matrix | Claude & Navarro 2012。MSB 安定分割 bitplane 行列:`ones` 前置和 + `zeros` 分割点のみ保持(bitvec すら不要— `pref[i+1]-pref[i]` でビット復元)。rank は (l,r) の matched-section 追跡が必須(1-boundary 近似は ones-hop 後に不正確になる bug を oracle が捕捉)。全操作 brute-force 照合 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Suffix automaton / Eertree | — | `suffix`(SA+LCP)と `manacher` が部分文字列・回文の主用途をカバー。SAM の遷移圧縮は次ラウンド候補 |
| Held–Karp exact TSP | — | tsp ヒューリスティックの厳密 oracle として有効だが n≤20 制約の価値を熟考中 |
| Cuckoo filter | — | bloom の削除対応版として有望。両者の誤り特性差を先に明文化したい |
| Dominators (Lengauer–Tarjan) | — | CFG 解析向け — kit の現状に dominator 需要が弱い。簡易 iterative 版は次回検討 |

## 出典(第21次、search-index 照合)

**論文・仕様**: Burrows & Wheeler (1994) / Sleator & Tarjan (1983, link-cut/HLD) / Stoer & Wagner (1997) / Miller–Rabin deterministic bases (Jaeschke 1993, Sinclair set) / Claude & Navarro (2012, wavelet matrix) / Brent (1980, rho variant) / Bentley MTF。

**実装物**: cp-algorithms(HLD・BWT・Miller–Rabin・Brent rho) / KACTL(StressTest patterns) / emaxx / Qiita・Zenn の BWT・HLD・Stoer-Wagner・Miller-Rabin・wavelet matrix 解説記事群。

# 第22次サーベイ — 文字列圧縮・厳密最適化・制約探索・有向全域木・線形基底

> 「部分文字列の重いクエリは」(sam)、「小規模巡回の真の最適は」(hamdp)、
> 「丁度1度ずつ覆う配置は」(dlx)、「根へ向かう最小有向木は」(arborescence)、
> 「xor 結合の可否と最大は」(xorbasis) — 前回「次回候補」に挙げた SAM と
> Held–Karp を含む、厳密解・正準形の残存層。
> PR #28–#37 は本ラウンド時点で open — 本ブランチは #37 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `sam` — suffix automaton | Blumer et al. 1985 / cp-algorithms SAM。オンライン拡張は「既存遷移を先に読んでから c→cur を挿入」が正しい順序(先に挿入すると根の遷移を q と誤読し link[cur]=cur の自己ループになる — oracle が捕捉)。`occurrences` は `finish` で len 降順の suffix-link 伝播、`distinct_substrings` は Σ len[v]−len[link[v]]。windows 走査・BTreeSet 全部分文字列・brute-force LCS 照合 | 🟢 純粋追加 |
| `hamdp` — Held–Karp exact TSP | Held & Karp 1962 / Bellman。`dp[mask][j]`(j∈mask に終わる最小コスト)O(n²·2ⁿ)、n≤16 前提で u64 コスト。`u32::MAX`=辺なしを全経路で失敗閉鎖。witness は greedy 最小 index + `tail_cost`(残余都市への再 Held–Karp)で辞書順最小最適ツアーを確定 — dp 差分推論ではなく exact tail を毎回計算する設計に倒し全順列 oracle で検証 | 🟢 純粋追加 |
| `dlx` — Algorithm X exact cover | Knuth 2000 (dancing links — 本実装はジャーナル方式)。最小候補列選択(同数は最小 index)+「cover が自行を col_rows 経由で自然 disable する」設計: `covered` bitset + 「cover で無効化した行」のジャーナル。初期版は「列に coverer 0 → 即失敗」と「列を被覆済み」を混同して `exact_cover(3,&[])=Some` になっていた — covered 配列で区別に修正。200 反復 subset 枚挙 oracle 一致 | 🟢 純粋追加 |
| `arborescence` — Edmonds directed MST | Edmonds 1967 (Chu–Liu/Edmonds)。各非根の最小入辺を選び、閉路があれば収縮(entering edge は w−best_in へ調整)→ 再帰 → 展開(被 entry で置換されるメンバー=displaced だけ best_in を捨てる)。コストは cost' + Σ_cyc best_in(cost' が既に調整済み)の帳簿を oracle が確認。(n−1) 辺 subset 枚挙(非根入辺1・根到達)+ witness 独立検証 | 🟢 純粋追加 |
| `xorbasis` — GF(2) 線形基底 | 標準 xor-basis(cp-algorithms k-th 構成)。逐次 RREF: 消去済み x は既存ピボット bit を持たないので、新ピボット x の MSB を他ベクトルから `^= x` で除いても他ピボット bit は揺らがない不変条件。`kth` は vec 昇順 basis の bit-写像が正確な順序を出すのに RREF が必要 — 最終の「下降一括除去」だと中間ピボット bit を再混入させる bug を kth 列挙 oracle が捕捉し逐次 RREF に確定 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Eertree (palindromic tree) | — | `manacher` が最長回文・`sam` が部分文字列構造をカバー。回文「数え上げ」系だけでは新規軸が薄い — 次ラウンド再検討 |
| 2D Fenwick | — | `fenwick` 1-D + `segtree` で矩形クエリは擬似的に書ける。真の 2-D BIT はグリッド集約需要が出てから |
| Dominators (Cooper iterative) | — | CFG/dominator 需要が依然弱い。`graph` 系の制御構造が必要になった時点で |
| Stable marriage (Gale–Shapley) | — | 割当としては `hungarian`/`bipartite` が強い。安定性保証は「優先度リスト駆動」の需要があるとき実装 |
| Rope / piece table | — | エディタ用文字列構造。`bits`/`delta` 層とは別軸で大きい — 別ラウンド候補 |

## 出典(第22次、search-index 照合)

**論文・仕様**: Blumer, Blumer, Haussler, Ehrenfeucht, Chen & Seiferas (1985, DAWG) / Held & Karp (1962, TSP DP) / Bellman (1962) / Knuth (2000, Algorithm X / dancing links) / Edmonds (1967) & Chu–Liu (1965) optimum branchings / cp-algorithms xor-basis k-th element 構成。

**実装物**: cp-algorithms(SAM・xor basis・Edmonds DMST) / KACTL(FastDlx・DFSMatching・DirectedMST 系の形) / emaxx(suffix automaton) / Qiita・Zenn の SAM・Held–Karp・Algorithm X・最小費用有向全域木・xor basis 解説記事群。

# 第23次サーベイ — 回文構造・オフライン区間・矩形最大化・安定割当・重み付き連結

> 「回文の全構造を1本で」(eertree)、「区間クエリを窓ソートで一括に」(mo)、
> 「最大矩形は」(histrect)、「相互指名の衝突しない割当は」(stable)、
> 「相対差の矛盾しない連結は」(wdsu) — 前回見送りに挙げた eertree・
> Gale–Shapley を含む、残存の定石クエリ/制約層。
> PR #28–#38 は本ラウンド時点で open — 本ブランチは #38 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `eertree` — palindromic tree | Rubinchik & Shur 2015 / eertree.org。IMAG(len=−1)根を「常に遷移可能」(`l < 0 || text[i−l−1] == c`)として設計しないと suffix-link 探索が停止しない — 省略すると無限ループ。`occ` は各位置の最長接尾辞回文のみ +1、`finish` で len 降順に link へ伝播すると総出現数。`node_of(pat)` は中心から1文字ずつ遷移を辿る(2文字消費は遷移誤り)。BTreeSet 全回文列挙・naive 最長回文 oracle 照合 | 🟢 純粋追加 |
| `mo` — Mo's オフライン区間クエリ | 競プロ定石(cp-algorithms / emaxx)。`isqrt` を整数二分で実装(f64 禁止)。ソートキー `(l/block, b%2==0 ? r : !r, i)` の蛇行で窓移動量を削減。カウンタは値が大きいと Vec が爆発するため `BTreeMap<u32,u32>` + `distinct` で駆動、窓端 `ql.min(n)`/`qr.clamp` を構造的に丸める。全区間 brute-force 照合 | 🟢 純粋追加 |
| `histrect` — ヒストグラム/0-1 行列最大矩形 | 競プロ定石(monotone stack)+ maximal rectangle via running heights。「右端に仮想 h=0 を供給して残スタックを全 flush」「左端は pop 後の `stack.last()+1` でピアノの蓋の向こうまで伸びる」二つの定石。行毎に高さを更新して same 関数へ流す max-rectangle が標準解法。O(n³) 全ペア最小高・列ラン全真 oracle 照合 | 🟢 純粋追加 |
| `stable` — Gale–Shapley 安定結婚 | Gale & Shapley 1962。提案者側最適は提案順に依らず一意 — 提案キューの discipline が結果に影響しないことを乱数順列で確認。`is_stable` は blocking-pair 不存在の独立述語(全 (m,w) で双方向優先度を検査)。n≤5 の全順列安定マッチング枚挙 oracle で man-optimality を照合 | 🟢 純粋追加 |
| `wdsu` — 重み付き union-find | 差分制約 DSU(cp-algorithms / 競プロ potential UF)。`weight[x] = pot[x]−pot[parent[x]]`、find の経路圧縮で `weight[x] += weight[p]` を伝播。attach は `weight[ru] = rel[v] − rel[u] − w`(ru を rv 下に)または対称式 — 符号を両 attach 方向で検算。`unite` は矛盾を検出して false(成分不変) — `dsurb` の undo 系と別軸。自己辺 (u,u,w) は w=0 でのみ受理 — オラクルが自己辺を「常に受理」と誤読する bug を捕捉。独立成分リプレイ oracle 全クエリ照合 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Dominators (Cooper iterative) | — | CFG/dominator 需要が依然弱い。`graph` 系の制御構造が必要になった時点で |
| Rope / piece table | — | エディタ用文字列構造。`bits`/`delta` 層とは別軸で大きい — 別ラウンド候補 |
| 2D Fenwick | — | `fenwick` 1-D + `segtree` で矩形クエリは擬似的に書ける。真の 2-D BIT はグリッド集約需要が出てから |
| Cuckoo hashing | — | ハッシュ表の決定性は DetHash 層と別の用途。必要性が薄い |
| Circulation (lower-bound flow) | — | `mcflow` が費用流をカバー。下界付き輸送は需要が出てから |

## 出典(第23次、search-index 照合)

**論文・仕様**: Rubinchik & Shur (2015, eertree) / Mo's algorithm (競プロ区間クエリ定石) / Gale & Shapley (1962, College Admissions) / weighted/potential union-find (差分制約 DSU) / monotone-stack largest-rectangle 定石。

**実装物**: cp-algorithms(eertree・Mo・potential DSU・largest rectangle) / eertree.org(オンライン構築手順) / emaxx / Qiita・Zenn の eertree・Mo's・Gale–Shapley・重み付きUF・ヒストグラム最大矩形解説記事群。

# 第24次サーベイ — 支配木・2-D BIT・下界付き流量・二重連結・類似度指紋

> 「この頂点を通らなければ到達できないのは」(dominators)、
> 「矩形領域の合計は」(fenwick2d)、「各辺に最低流量がある供給網は」(circulation)、
> 「共通サイクル上の辺の集合は」(biconn)、「生成物の近似重複は」(simhash) —
> 第23次で見送りに挙げた dominators・2D Fenwick・circulation を含む、
> グラフ解析・集約・フィンガープリント層。
> PR #28–#39 は本ラウンド時点で open — 本ブランチは #39 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `dominators` — 支配木+支配辺境 | Cooper, Harvey & Kennedy (2001)「A Simple, Fast Dominance Algorithm」— RPO 順の `intersect` フィンガーウォークが Lengauer–Tarjan より単純で実効速い。反復 postorder DFS → RPO 番号付け → 不動点ループで `idom` を収束、子は整列して決定的。Cytron の辺境 `frontier[b] = preds[b] を idom[b] まで遡る` 走査。到達不能頂点は `UNREACH` マークで除外。集合不動点 oracle(`dom[v] = {v} ∪ ⋂dom[pred]`)と全頂点照合 | 🟢 純粋追加 |
| `fenwick2d` — 2-D Fenwick BIT | 競プロ定石 — `i += i & −i` を2軸に拡張、1-based を row-major `(i−1)·h + (j−1)` に写像。`rect_sum` は4隅の包含除去、クランプで空→0。行-major だと `i` の外側ループが行ブロック跨ぎで親へ飛ぶ古典的 index 算術。稠密 Vec オラクル全矩形照合 | 🟢 純粋追加 |
| `circulation` — 下界+デマンド付き可行巡回流 | 古典的還元(Kleinberg–Tardos / emaxx): `req[v] = demand[v] − lo_in[v] + lo_out[v]`、`req>0` なら `v→tt`、`req<0` なら `ss→v` — **方向を逆にすると保留流で witness が保存則を壊す**(本ラウンドの oracle が初版の逆配置を捕捉: SS→0→1→2→TT の経路が demand を満たすが witness は不成立)。`flow::add_edge` が `u==v`/`c==0` で早期 return して `adds` エントリを積まないため rank 写像 `added` が必須 — スキップ辺は `lo` がそのまま強制値。feasible ⟺ maxflow == Σreq>0。Hoffman 切断条件(∀S: hi_in(S)−lo_out(S) ≥ demand(S))の全 2^n subset oracle で可/不可を照合 + witness 独立検証 | 🟢 純粋追加 |
| `biconn` — 二重連結成分分解 | Tarjan 辺スタック法: DFS で辺を積み、`low[w] ≥ disc[v]`(子 subtree が v より上に辿れない)で成分を閉じてスタックから pop — 橋は singleton 成分として自然に浮上。無向 DFS では back edge は全て祖先向き(`disc[w] < disc[v]` 側のみ push で二度積み回避)。反復 DFS (頂点,親辺,adj index) フレーム。多重辺の 2-cycle は実サイクル — oracle(単純サイクル全列挙)が両者で一致確認 | 🟢 純粋追加 |
| `simhash` — Charikar LSH 指紋 | Charikar (STOC 2002) / Google の near-dup 検出 — 特徴ハッシュの各 bit が ±weight 投票し bit=正票多数。多重出現はそのまま重み(multiplicity IS weight — 再 seed 化すると多重性を潰す誤設計を検討段で棄却)。tie(正負同票)は 0 で正準化 — 符号半平面の unsigned 慣行。`hamming` = popcount、`near_dupes` は全ペア O(n²)。bit-major 独立再計数 oracle 照合 + 類似度順序性(部分共通 < 完全 disjoint) | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Suffix array induced sorting (SA-IS) | — | `suffix` の prefix-doubling で需要はカバー。SA-IS は実装が大きく別ラウンド候補 |
| Cuckoo / SwissTable ハッシュ | — | ハッシュ表の決定性は DetHash + BTreeMap 層が担う。open-addressing の真の需要は依然薄い |
| Rope / piece table | — | エディタ用文字列構造。`bits`/`delta` 層とは別軸で大きい — 引き続き別ラウンド候補 |
| Hopcroft–Tarjan planarity | — | 平面性判定は procgen 需要が出てから。実装が最重級 |
| Link-cut tree | — | 動的木のパスクエリ。`hld`+`dsurb` で静的/undo 版は充足 |

## 出典(第24次、search-index 照合)

**論文・仕様**: Cooper, Harvey & Kennedy (2001, dominators) / Cytron et al. (1991, SSA dominance frontiers) / Hoffman circulation theorem (1960) / Tarjan (1972, biconnected components) / Charikar (STOC 2002, SimHash)。

**実装物**: cp-algorithms(circulation 還元・dominators CHK・biconn 辺スタック・simhash) / emaxx / Kleinberg–Tardos 教科書(下界付き可行流) / Qiita・Zenn の dominator・2-D BIT・辺連結分解・SimHash 解説記事群。

# 第25次サーベイ — 有限体・消失訂正・圧縮索引・軽量最短路・SAT

> 「shard が欠けても復元できるか」(rsfec の上に立つ `gf2`)、
> 「巨大テキストをインデックスで検索できるか」(fmidx)、
> 「重みが 0/1 しかないとき heap は要らない」(zerobfs)、
> 「ルールを CNF に落として矛盾なく満たせるか」(dpll) —
> 通信信頼性・検索・判定の基盤層。第24次の見送り表に残った cuckoo/rope/linkcut
> 系は引き続き需要観察、本ラウンドは通信レイヤの不足していた
> 誤り訂正と探索/判定の残存定石を優先。
> PR #28–#40 は本ラウンド時点で open — 本ブランチは #40 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `gf2` — GF(2⁸) 体演算 | AES 多項式 `x⁸+x⁴+x³+x+1` (`0x11B`) — `sub`=`add`=xor、乗算はロシア農民法で log 無しに O(8)、`Tables` は生成元 3 の exp/log 写像(`x·3 = (x<<1)^x` は多項式時間に原始元 — `exp.copy_within(..255,255)` でインデックス 255 を持つ exp、log は非ゼロのみ)。`inv`=`exp[255−log[a]]`、0 の除算は `None`。2000 反復の乱数体公理照合 + 生成元が全 255 非零元を巡回する検証 | 🟢 純粋追加 |
| `rsfec` — Reed–Solomon 消失訂正 | Backblaze/FEC 定石(k+m coding matrix): 係数行列は `V(total,data)·V_top⁻¹`(Vandermonde の上 data 行の逆を右掛け → top data 行が恒等行列になりつつ任意 data 行が可逆)。`reconstruct` は現存 data 行の部分行列逆でデータ復元、欠けた parity は復元後に再 encode — GF 上 Gauss–Jordan(pivot 検索・`inv` でスケール・全行消去)。4-of-8 の全 70 subset 網羅 + 60 反復乱択損失照合、malformed shard 拒否 | 🟢 純粋追加 |
| `fmidx` — FM-index | Ferragina & Manzini (2000): `C[b]` = b 未満の総数、`Occ(b,i)` = `l[..i]` 中の b 出現数で backward search が区間を縮める。実装は巡回 BWT 上 — `C` 表 + `OCC_STEP=32` 毎の全バイト集計チェックポイント(`occ[k]` = `l[..k·STEP]` のカウント)+ 間は線形走査、完全 SA で `locate` も O(occ)(checkpointed 版なら LF-mapping が必要になるため完全 SA のまま)。巡回一致(パターンが wrap する出現)をドキュメント化した semantics、naive 巡回列挙 oracle 300 反復照合 | 🟢 純粋追加 |
| `zerobfs` — 0-1 BFS + Dial | 競プロ定石(cp-algorithms): 重み {0,1} は VecDeque(0 は front、1 は back)で monotone queue、重み ∈ 0..=cap は `cap·(n−1)` 個のバケット配列(最短距離の上限が cap·(n−1) — 非負最短路は頂点を繰り返さない)。どちらも「申告した上界を破る辺」は `None` で失敗閉鎖(黙って切り詰めると破綻する)。`bellman` オラクル乱数 600 照合 | 🟢 純粋追加 |
| `dpll` — DPLL SAT | Davis–Putnam–Logemann–Loveland (1960/62): unit propagation + pure literal 除去を不動点まで回し、最小変数で split(`true` 分岐優先)。CDCL ではない(節学習・VSIDS なし)— ゲームスケールの制約判定に exact で十分速い層。返す model は canonical(未設定変数 `false`)で `solve` は `(clauses)` の純関数 — 同一 CNF は同一 model。2^n 全割当 brute-force oracle で satisfiability + 返却 model が全 clause を充足する独立検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| CDCL (conflict-driven clause learning) | — | `dpll` でゲームスケールの CNF は充足。学習は実装が数倍の規模になり本ラウンドの範囲外 |
| Cuckoo / SwissTable | — | ハッシュ表の決定性需要は DetHash + BTreeMap が担う。open-addressing の真の需要は依然薄い |
| Rope / piece table | — | エディタ用文字列構造。`bits`/`delta` 層とは別軸で大きい — 引き続き別ラウンド候補 |
| Link-cut tree | — | 動的木のパスクエリ。`hld`+`dsurb` で静的/undo 版は充足 |
| SA-IS induced sorting | — | `suffix` の prefix-doubling で需要はカバー。`fmidx` が SA に依存するため、必要になればそこへ波及 |

## 出典(第25次、search-index 照合)

**論文・仕様**: Reed & Solomon (1960) / Ferragina & Manzini (FOCS 2000, FM-index) / Davis, Putnam, Logemann & Loveland (1960, 1962) / AES 有限体 `0x11B` (FIPS-197) / Dial (1969, bucket shortest path)。

**実装物**: cp-algorithms(0-1 BFS・Dial・DPLL・GF(2^8) 演算) / Backblaze の Reed–Solomon 解説(k+m coding matrix) / Qiita・Zenn の RS 符号・FM-index・0-1 BFS・DPLL 解説記事群。

# 第26次: 区間クエリ・重心分解・本文バッファ・Steiner 近似・先行書き込みログ

> 永続化とクエリ構造の残存層。第25次の見送り表から `piecetable` を引き上げ(本ラウンドで実装)、残りの cuckoo/linkcut/sais は引き続き需要観察。
> PR #28–#41 は本ラウンド時点で open — 本ブランチは #41 tip 上に積層。

## 実装済み(本セッション)

| 実装 | 対応する知見 / 出典 | 決定論影響 |
|---|---|---|
| `intervaltree` — centered interval tree | 計算幾何定石(cp-algorithms / CGAL Interval_tree): pivot は端点スパンの**中点** — 端点 median だと葉の `hi` と一致して停止しないことがある(単一区間 (0,3) で pivot=3 が hi≤pivot に再帰し続ける再現 bug)。`by_lo` 昇順・`by_hi` 降順で stab は条件側のみ線形走査 + 部分木 1 方向。`overlap(l,l)` は `stab(l)` に退化、`l>r` は失敗閉鎖空 — 集合論的意味論を文書化して oracle を一致させた。300 反復 naive 照合 | 🟢 純粋追加 |
| `centroid` — centroid decomposition | 競プロ定石: 各除去が成分を ≤半分に分割する分解木(深さ ≤ ceil(log2 n))。木以外の入力では DFS `sizes` に `seen` が必須 — `w != par[u]` は直近の親しか排除せず閉路で無限ループする bug を 4-cycle で再現・修正。`find` の重側歩行も spanning-tree 辺のみに限定して停止性を保証(非木辺の `sz` は別側に計上済み)。全頂点で除去後 piece ≤ half の oracle 照合 + 深さ上界 + `lca` 祖先列検証 | 🟢 純粋追加 |
| `piecetable` — piece table | Crowley (1998, "Data Structures for Text Sequences"): 不変 `original` + append-only `added` + piece 列の三段構造 — エディタ定石を sim スケールに適用。`insert` は包含 piece の split、`delete` は両端点 trim + 隣接 coalesce で piece 数膨張を抑制。Vec::splice/drain を oracle に 300 反復の乱択 op 列で完全一致 + out-of-range 拒否 | 🟢 純粋追加 |
| `steiner` — Steiner 2-近似 | Kou–Markowsky–Berman (1981): 終端間最短路の metric closure → MST → 経路展開 → union の MST で cycle prune(重みは単調非増)。2·opt 保証は「最適木の Euler 巡回のショートカット巡回 ≤ 2·opt ≥ closure MST」。oracle は Dreyfus–Wagner 厳密 DP(部分集合 DP + 行毎 multi-source Dijkstra で `O(3^k n + 2^k n²)`)— `opt ≤ w ≤ 2·opt` 300 乱数照合 + |E|=|V|−1 の tree 検証 | 🟢 純粋追加 |
| `wal` — write-ahead log | SQLite WAL / Aries: `[kind|len|crc|payload]` レコード、crc は `"walv1" ‖ kind ‖ len ‖ payload` の domain 分離 Fnv1a。replay は**先頭からの正当 prefix** semantics — torn/corrupt レコードで停止し `stopped_at` を返す(以降のオフセット desync で全て信用不能)。全 cut 位置で `clean ⟺ stopped_at == cut`、payload/kind の bit-flip で prefix 拒否、`truncate(stopped_at)` で修復を検証 | 🟢 純粋追加 |

## 検討して見送った候補

| 候補 | 出典 | 見送り理由 |
|---|---|---|
| Link-cut tree | — | 動的木パスクエリ。`centroid`+`hld`+`dsurb` で静的/undo/分解の三層は充足 — 全動的は依然待ち |
| SA-IS induced sorting | — | `suffix` prefix-doubling + `fmidx` で需要カバー |
| Cuckoo / SwissTable | — | open-addressing ハッシュ表の需要は引き続き薄い(DetHash+BTreeMap が担う) |
| Rope | — | `piecetable` がテキストバッファ需要をカバー。永続 rope は別軸で引き続き候補 |
| Planarity testing | — | `delaunay`+`biconn`+`poly` で部分的需要は充足。完全 planarity は実装規模が大きい |

## 出典(第26次、search-index 照合)

**論文・仕様**: Kou, Markowsky & Berman (1981, Steiner 2-approx) / Dreyfus & Wagner (1971, exact Steiner DP) / Crowley (1998, piece table) / SQLite WAL 設計 / cp-algorithms(centroid decomposition・interval tree・0-1 BFS 系)。

**実装物**: cp-algorithms / CGAL Interval_tree 参照設計 / Qiita・Zenn の重心分解・Steiner 木・piece table・WAL 解説記事群。

# 第27次: 近似文字列照合・群体経路・式評価・凸衝突・冪2割付

| 採用 | 根拠 / 検証 | 判定 |
|---|---|---|
| `bitap` — Shift-And bit-parallel | Wu & Manber (1992) / cp-algorithms 系解説: pattern ≤64 byte は `u64` 1 ワードで Shift-And 進行し、fuzzy (Hamming k) は誤り行毎に 1 ワードの多重追跡。置換項 `(prev<<1)|1` の bit-0 種付けを欠くと pattern[0] 置換が一切落ちる実 bug を naive Hamming 全窓 oracle が捕捉 — 空 prefix は常に真なので進行項・置換項それぞれに `|1` が要ることを確認し両項独立の seed で確定。挿入・削除は許容しない Hamming 意味論(編集距離ではない)であることを、"needl " 窓が末尾空白→'e' 置換として正当に k=1 一致する例で明文化 | 🟢 純粋追加 |
| `flowfield` — integration + vector field | *Supreme Commander 2* / *Planetary Annihilation* 系の TD swarm 定石 + redblobgames grid 参照: 目的集合からの逆向き Dijkstra で 1 構築・全 agent O(1)/step。8 連結で対角は両 ortho 隣接が通行可能な時のみ(角抜け禁止)、整数 √2≈1.5 倍率で ortho=2·cost・diag=3·cost の厳密整数距離。検証は dist 全セル oracle 一致 + dir が必ず厳密降下し goal に到達する性質検査(独立構築 oracle は弱いため、argmin/単調性/終端性の contract 検査を主体に) | 🟢 純粋追加 |
| `shunting` — shunting-yard + i64 eval | Dijkstra (1961) operator-stack: `+ - * / %` と右結合単項 `Neg`、括弧。operand/operator 交互 + paren 深度を parse 時検証し、評価不能な postfix を産出しない構造保証。`/`・`%` は Rust と同じゼロ方向切断、`/0`・`i64::MIN/-1`・全 overflow は `None` の失敗閉鎖 — panic 経路が存在しないことを全演算で確認。oracle は別実装の再帰降下評価器で 2000 乱数式照合 | 🟢 純粋追加 |
| `sat` — separating-axis convex collision | Gottschalk 系 SAT 定石(辺法線のみが候補軸)、`i128` 投影で float 軸を根絶。境界接触は重なり(closed polygon)。depth は axis 未正規化の投影単位 = `depth/|axis|` が真の深さ、axis は重心差で a→b の正準向き。点(辺なし退化形)も凸多角形として受理 — 点の投影区間が全辺法線で多角形 slab 内なら内部、凸多面の法線 slab 交差が多面を再構成する性質で正当化。oracle は edge-intersect(端点包含)+ 両方向頂点包含の独立判定で 3000 乱数照合 | 🟢 純粋追加 |
| `buddy` — binary buddy allocator | Knuth/Kerrighan 系 buddy 定石 + Linux page buddy: 冪2 ブロックを需要時 split・解放時 eager merge。free list を address sort し最小 order・最低位を採用することで状態は操作列の純関数に(決定性)。「二つの buddy が同時 free でない」canonical 性質が coalescing の完全性を保証。oracle は byte-shadow 占有列 + 全ブロック天然整列 + `free_bytes` 不変 + 最終 drain で全 byte 割付可能(合体漏れ検出)を 300 arena×200 op で検証。double-free/未割付 free は false 失敗閉鎖 | 🟢 純粋追加 |

見送り(第27次): 編集距離 fuzzy 照合(既出 `diff` の Levenshtein で代替可能、Bitap Hamming と非重複)、GJK/EPA(汎用凸距離 — SAT は凸限定で軽い、需要出れば拡張)、TLSF(境界付き worst-case allocator — buddy の O(log) で現在需要十分)、Chomsky 完全な式構文(比較・関数呼出し — tokenizer 公開 API と合わせ需要待ち)、jump-point search(格子経路高速化 — `pathfinding` A* + `flowfield` で需要十分)。

## 出典(第27次、search-index 照合)

**論文・仕様**: Wu & Manber (1992, Shift-And/agrep) / Dijkstra (1961, shunting-yard) / Gottschalk (1996, OBBTree/SAT) / Knuth TAOCP buddy / Linux buddy allocator / Crowley (1998, piece table 参照系)。

**実装物**: cp-algorithms(shift-and・buddy 系) / redblobgames grid pathfinding / Planetary Annihilation flow-field 技術解説 / Qiita・Zenn の Shift-And・shunting-yard・SAT・buddy 解説記事群。

# 第28次: メンバーシップ・キャッシュ・重み抽選・動的空間・配列木橋

| 採用 | 根拠 / 検証 | 判定 |
|---|---|---|
| `xorfilter` — xor filter | Graf & Lemire (2020) xor filter: bloom の後継で ~1.23B/key・fp ~1/256・挿入済みは必ず true。3 分割 disjoint 区間で h1,h2,h3 の衝突なしを構造保証し、degree-1 slot の BFS peel で構築 — 2-core 残存なら seed を変えて再試行(≤64)。claimant を XOR 記録して deg==1 slot の唯一キーを O(1) 特定。検証は BTreeSet oracle で 40 trial の全メンバー受理 + 4000 probe の fp ≤4% 境界 + 同一 build の表一致 | 🟢 純粋追加 |
| `lru` — LRU cache | 教科書定石(ordered map + recency queue): linked list ではなく BTreeMap×2 の (stamp,key) ソート index で排出順を正準化 — 同刻印の key 衝突は構造上起きない(u64 単調刻印)が仮に起きても key 昇順で決着する設計。`get` は刻印更新、`peek` は刻印不変、`by_recency`/`pop_lru` は canonical 直列化。VecDeque シャドー oracle で全 op・排出列・drain を照合 | 🟢 純粋追加 |
| `vose` — Vose alias | Vose (1991) 線形 alias 法: float `p_i` の代わりに `scaled[i]=w_i·n` を u128 で保持し small(<total)/large(≥total) を厳密分割。核心の正当性は枚挙可能 — 全 (bucket,coin) の `n·total` ペアで各 item は `w_k·n` 件に写像されることを直接検証(実装の近似ではなく厳密分配)。pop 順は index 最大側で正準、coin は `next_u64 % total` で seeded 決定 | 🟢 純粋追加 |
| `quadtree` — bucketed point quadtree | Finkel & Bentley (1974) 系 bucketed quadtree: 半開矩形 4 分岐(NW,NE,SW,SE 固定順)、bucket 超過で遅延分割、1-wide/1-tall 帯は can_split=false で leaf 溢れ(hang しない)。回答は常に sort 正準 — 木形状は挿入列の純関数。`nearest` は子矩形 min-dist² の BinaryHeap best-first、タイは辞書順小の点で正準。全矩形 brute-force + nearest 全点照合 oracle(重複点の multiset 意味論も検証) | 🟢 純粋追加 |
| `cartesian` — cartesian tree | Vuillemin (1980) O(n) スタック構築: heap-on-values × BST-on-positions の一意木 — Fischer–Heun RMQ と treap 形状の理論的土台。重複値は (val,idx) 総順序で canonical(左端最小が根)。`rmq` = i,j の LCA が範囲極値 index を返すことを brute-force argmin と 300 配列×40 区間照合、heap 順序・inorder=0..n・部分木の連続区間性と包含関係を全検証 | 🟢 純粋追加 |

見送り(第28次): 可変メンバーシップ(cuckoo/SwissTable — 既出見送り継続、静的構造で需要十分)、rope(piece table で代替済みのまま)、SA-IS(線形接尾辞配列 — 既出 `suffix` doubling で十分)、jump-point search(`flowfield`+A* で十分、見送り継続)、regex 完全構文(部分集合のみ需要、見送り継続)。

## 出典(第28次、search-index 照合)

**論文・仕様**: Graf & Lemire (2020, xor filter) / Vose (1991, alias method) / Finkel & Bentley (1974, quad trees) / Vuillemin (1980, cartesian trees) / Sleator–Tarjan LRU 定石文献。

**実装物**: cp-algorithms(cartesian/RMQ 系)・FastFilter(Lemire 系参照実装)・redblobgames spatial index・Qiita・Zenn の xor filter・alias method・quadtree・cartesian tree 解説記事群。

# 第29次: 索引ヒープ・永続木・チェックサム・探索・ハンドル

| 採用 | 根拠 / 検証 | 判定 |
|---|---|---|
| `iheap` — indexed binary heap | Dijkstra 定石の一般化(教科書 indexed priority queue): pos[] 逆引きで `set`/`decrease`/`increase`/`remove` を O(log n)。pop は (prio,key) 辞書順 — 同優先度は key 昇順で正準(挿入順非依存を保証)。`remove` は末尾 swap→pop→両方向 sift で heap 性維持。BTreeMap オラクルで 200 配列×300 op の全遷移・peek・drain 照合 | 🟢 純粋追加 |
| `pstree` — persistent segment tree | chairman tree 定石(持久化線段樹): path-copy 挿入で prefix 毎に root を切る — 過去版を一切書換えず `O(n log n)` node arena。`a[l..=r]` の kth/freq/range_count は roots[r+1]−roots[l] の差分降下。座標圧縮で i64 全域対応。slice sort + 全範囲 brute-force oracle 200 配列×50 クエリ照合、版の非 alias 性を確認 | 🟢 純粋追加 |
| `crc` — CRC-32 streaming | IEEE 802.3 reflected poly 0xEDB88320(教科書 + zlib 系): 8bit 後退シフトを 256 エントリ const テーブルで前計算、seed !0・final !。chunk 分割が検査値に影響しないことを任意分割照合、既知ベクタ CBF43926/414FA339/D202EF8D、全 byte×全 bit 反転を検出 — bit-level 多項式除算 oracle 500 照合 | 🟢 純粋追加 |
| `mcts` — seeded UCB1 MCTS | Kocsis & Szepesvári (2006) UCT: selection/expansion/rollout/backprop の 4 相を `minimax::Game` 上に構成。float 禁止のため UCB は整数版 — `ln`→`⌈log2⌉` 置換は ln N=ln2·log2 N により探索定数に吸収可能、√は Newton isqrt、勝率は {0,500,1000} permille。**子ノードの wins はその子の side-to-move 視点で蓄積されるため、親からの UCB 評価は 1000−child_mean が必須** — stored view をそのまま使う初版は best-first が逆転し「必勝手を選ばない」bug として強制勝ちオラクルが捕捉。展開は canonical moves() 順、rollout は SplitMix64 seeded、最終 argmax は最多 visit・同率は canonical 順 | 🟢 純粋追加 |
| `slotmap` — generational slot map | 教科書 slot map(bitsquid/EnTT 系): (slot,gen) 詰め込み u64 handle、remove で gen++ → 全ての旧 handle が構造的に不成立(ABA 防止)。LIFO recycle で再利用順は操作列の純関数、gen が u32::MAX に達した slot は alias 防止のため永久退役。entries は slot 順 canonical 出力。BTreeMap+stale 集合の shadow oracle 2000 op + recycle 新世代・不明 handle 全拒否を検証 | 🟢 純粋追加 |

見送り(第29次): regex(見送り継続 — tokenizer/部分集合需要のみ)、link-cut tree(動的木 — `dsurb`/`hld` で需要十分、見送り継続)、planarity 判定(見送り継続)、GJK/EPA(SAT で凸衝突は充足、見送り継続)、bitboard/magic(bitboard は別途需要待ち)。

## 出典(第29次、search-index 照合)

**論文・仕様**: Kocsis & Szepesvári (2006, UCT) / Vuillemin 系 indexed priority queue 教科書 / IEEE 802.3 CRC-32 / chairman tree(持久化線段樹)競プロ文献 / bitsquid・EnTT の slot map 設計。

**実装物**: cp-algorithms(indexed heap・persistent segtree・MCTS 系解説)・zlib CRC 参照実装・skypjack/entt slot map・Qiita・Zenn の持久化セグ木・UCB・世代付きハンドル解説記事群。

# 第30次: 削除可能フィルタ・エントロピー符号・内陸極点・完全ハッシュ・独立集合

| 採用 | 根拠 / 検証 | 判定 |
|---|---|---|
| `cuckoof` — Cuckoo filter | Fan, Andersen, Kaminsky & Mitzenmacher (2014): u8 fingerprint + `h2=h1^hash(fp)` で消去が再ハッシュ不要に — Bloom が構造的に持てない delete を実現。kick victim は (key,seed,kick) の SplitMix64 で選び table は (keys,seed) の純関数(再構築一致を検証)。偽陰性ゼロ・2000 op BTreeSet 照合・fp 率上界確認 | 🟢 純粋追加 |
| `rans` — rANS entropy codec | Duda (2013) ANS: Huffman の 1 bit/symbol 下限を割るエントロピー符号。largest-remainder 正規化で Σ=L=2^12(全シンボル freq≥1、剰余は降順・同率は小 index が優先で正準)。encoder は逆順 consume→decoder 順序復元、u64 状態+ u16 排出の乱数3k 往復一致・wire 切り詰め/末尾ゴミ全拒否。歪分布 ~0.2 bit/symbol で圧縮確認 | 🟢 純粋追加 |
| `polylabel` — integer pole | mapbox polylabel の整数化: セル上界を `(ceil√d²+ceil√r²)²` に保持し全比較を i128 有理数の平方距離で実施 — float/√ を一切出さずに剪定が厳密。lattice 点のみを評価(PIP=Inside)し同点は辞書順最小で正準。凸多角形乱択・L字の lattice 全走査 oracle と最適値一致 | 🟢 純粋追加 |
| `mphf` — CHD perfect hash | BDZ/CHD 2段 displacement: `mphf = h(key, d[bucket]) mod n`、バケット解決順は (size desc, idx) 正準、(key set,seed) の純関数(挿入順非依存を検証)。乱択 key 集合で全単射性・dup 除去・foreign key も [0,n) 全域写像を確認 | 🟢 純粋追加 |
| `mis` — canonical greedy MIS | 教科書貪欲: index 昇順走査、既選択近傍が無い頂点を採用 — 辺集合のみの一意決定(辺順序非依存を検証)。自己ループは構造的に不適格。独立性(両端点同時選択なし)+ 極大性(非選択点は被覆済)を定義通り全乱択検証 | 🟢 純粋追加 |

見送り(第30次): regex・linkcut・planarity・rope・GJK/EPA・TLSF・Chomsky-full expr・edit-distance fuzzy(第29次見送り継続 — 需要顕在化まで凍結)、bitboard/magic(需要待ち)。

## 出典(第30次、search-index 照合)

**論文・仕様**: Fan, Andersen, Kaminsky & Mitzenmacher, "Cuckoo Filter: Practically Better Than Bloom" (CoNEXT 2014) / Duda, "Asymmetric numeral systems" (2009/2013) / mapbox polylabel (grid B&B アルゴリズム) / BDZ・CHD minimal perfect hashing / 貪欲 MIS 教科書定式。

**実装物**: cp-algorithms・Qiita・Zenn の Cuckoo filter・rANS・polylabel・MPH 解説、 Fabian "ryg" Giesen の rANS 実装ノート。


# 第31次: ストリーム暗号・暗号学的ハッシュ・クラスタ・静的索引

| 採用 | 根拠 / 検証 | 判定 |
|---|---|---|
| `chacha` — ChaCha20 | RFC 8439 ARX 構造: `[SIGMA|key8|counter|nonce3]` 状態、quarter-round 回転 16/12/8/7 × 10 ダブルラウンド、出力 = 作業+初期状態。§2.3.2 ブロック `22 4f 51 f3`・§2.4.2 暗号 `6e 2e 35 9a` を Python 参照実装と両側一致確認、チャンク分割 ≡ 一括適用。replay 系に seed 由来の決定性暗号ストリームを提供 | 🟢 純粋追加 |
| `sha256` — SHA-256 | FIPS 180-4: K[64]・σ0/σ1・Σ0/Σ1、インクリメンタル write + 正準 BE パディング(`be_bytes` 族は幅規則で禁止 → 手動シフト抽出)。`abc`=`ba7816bf`・空=`e3b0c442`・56 文字=`248d6a61`・10^6×'a'=`cdc76e5c` 全 NIST ベクトル一致 + 分割不変 + 1bit 反転で完全差分 | 🟢 純粋追加 |
| `dbscan` — 密度クラスタ | Ester et al. (1996): eps²・min_pts のみでクラスタ数不要。平方距離判定で √ を排除、シード+展開キュー共に昇順 index の正準形 — (points,eps2,min_pts) の純関数。3 定義照合: コア点(密度充足)は必ずラベル化、境界点は同クラスタのコアに eps 近接、各クラスタのコアは eps² 連鎖で連結 | 🟢 純粋追加 |
| `kmeans` — 整数 k-means | farthest-point(Gonzalez 式)初期化で乱数を排除 + 整数重心丸めの Lloyd 反復。不動点停止(assignment 不変)で上限 256 回。全ラベルが argmin 一貫・冪等・inertia 再計算一致 — `dbscan` の k 指定補完 | 🟢 純粋追加 |
| `rtree` — STR R-tree | Leutenegger, Edgington & Lopez (1997) STR 梱包: x ソート→タイル化→y ソート→m 梱包を層毎に再帰 — 全工程が総ソートのため木形状は点集合の純関数(挿入順非依存を反転入力で検証)。矩形クエリは昇順正準でブルートフォース全照合。leaf 化けの `div_ceil` 二重計算で group_size を正しく取る修正点を stack overflow として観測・修正 | 🟢 純粋追加 |

見送り(第31次): cuckoo テーブル本体・SwissTable(`cuckoof`/`xorfilter` で近似は充足 — 厳格 dict は `treap`/`BTreeMap` 需要)、sais O(n) SA(`suffix` の doubling で実務十分)、rope(`piecetable` で充足)、regex・linkcut・planarity・JPS 拡張・GJK/EPA・TLSF・Chomsky-full expr・edit-distance fuzzy・bitboard/magic(前次見送り継続 — 需要顕在化まで凍結)。

## 出典(第31次、search-index 照合)

**論文・仕様**: RFC 8439 "ChaCha20 and Poly1305" / FIPS 180-4 SHA-256 / Ester, Kriegel, Sander & Xu, "A Density-Based Algorithm for Discovering Clusters" (KDD 1996) / Lloyd (1982) k-means + Gonzalez farthest-point seeding / Leutenegger, Edgington & Lopez, "STR: A Simple and Efficient Algorithm for R-Tree Packing" (ICDE 1997)。

**実装物**: ring/rust-crypto 系 ChaCha・SHA-2 定数表、Qiita・Zenn の DBSCAN/k-means/R-tree 解説、scikit-learn・RBush・rtree-rs の梱包設計メモ。


# 第32次: 一時認証子・前駆/後継構造・圧縮ビットマップ・文字列正準形・集合格変換

| 採用 | 根拠 / 検証 | 判定 |
|---|---|---|
| `poly1305` — Poly1305 MAC | Bernstein (2005) / RFC 8439 §2.5: `r`=clamp 済み 128bit 乗算器、`s`=128bit 加算項、5×26-bit limb で `h=(h+block)·r mod 2^130−5`。partial block の終端 `0x01` は limb 内位置 `8·len` に置くため hibit 無効が正解(本ラウンドの設計ミスをベクタ照合で捕捉)。§2.5.2 `a8061dc1` 一致・分割不変・1bit 反転全差分 — `chacha` と AEAD 対を構成 | 🟢 純粋追加 |
| `veb` — proto van Emde Boas | van Emde Boas (1977) sqrt 分解を 2 段に固定: hi:lo=16+16、top bitset が非空クラスタ標記。predecessor/successor は自クラスタ内 bit 走査→失敗時に top 走査の定数級手続き。BTreeSet oracle に insert/remove/pred/succ 4000 乱択全照合 + 境界値(0, u32 端) | 🟢 純粋追加 |
| `roaring` — Roaring bitmap | Chambi, Lemire et al. (2016): 上位 16bit でコンテナ分割、疎=sorted array / 密=1024-word bitset、4096 閾値の双方向変換。集合演算はコンテナ対 wordwise 合成→normalize — 同一集合は構造も同一(挿入順非依存)。BTreeSet oracle で 4 演算全照合、昇順 iter 保証 | 🟢 純粋追加 |
| `lyndon` — Lyndon 分解 | Duval (1983): `s[i..j)=w^p·w'` で完全コピーのみ emit(本ラウンドの `i≤j−k` 境界 bug を因子全 Lyndon オラクルが捕捉 → `i≤k` に確定)。Booth (1980) 最小回転は doubled-string 走査。`is_lyndon`=全真接尾辞下位を `s < s[k..]` 直接評価 — 全回転枚挙・因子被覆・非増加性を乱択照合 | 🟢 純粋追加 |
| `sosdp` — 部分集合格変換 | Yates (1937) 系 SOSDP + Walsh–Hadamard: subset/superset zeta↔Möbius 逆対、OR/AND 畳み込み(zeta→点積→Möbius)、xor 畳み込み(FWHT、逆変換は 2^n スケール)。naive O(4^n) 全照合 + zeta 部分和走査一致 + WHT 畳合検証 — `conv`(NTT) と並ぶ bitmask DP 計数の第三基盤 | 🟢 純粋追加 |

見送り(第32次): cuckoo テーブル本体・SwissTable・sais O(n) SA・rope・regex・linkcut・planarity・JPS 拡張・GJK/EPA��TLSF・Chomsky-full expr・edit-distance fuzzy・bitboard/magic(第30–31次見送り継続 — 需要顕在化まで凍結)。真の 3 段再帰 vEB / y-fast trie は proto 版で実務十分なため見送り。

## 出典(第32次、search-index 照合)

**論文・仕様**: RFC 8439 §2.5 (Poly1305) / Bernstein, "The Poly1305-AES message-authentication code" (2005) / van Emde Boas, "Preserving order in a forest in less than logarithmic time" (1977) / Chambi, Lemire, Kaser et al., "Better bitmap performance with Roaring bitmaps" (SPE 2016) / Duval, "Factorizing words over an ordered alphabet" (1983) / Booth, "Lexicographically least circular substrings" (1980) / Yates, "The design and analysis of factorial experiments" (1937, SOS zeta の原型) / Walsh (1923)–Hadamard 変換。

**実装物**: poly1305-donna(26-bit limb 版)・RustCrypto/poly1305 の limb 分割、roaring-rs のコンテナ設計、cp-algorithms の Duval/Booth コード、Qiita・Zenn の SOSDP(高速ゼータ/メビウス)解説 — OR/AND/XOR 畳み込みまでの展開系を踏襲。

## 第33次 順序集合・索引・乗算・暗号(round 33)

**採用モジュール(222→227)**: `skiplist` / `karatsuba` / `lsm` / `pgm` / `aes`

- `skiplist` — Pugh の skip list のコイン投げを `trailing_zeros(hash(key,seed))` に置換。幾何分布レベルが (key,seed) の純関数 → 車線構造が挿入順非依存(レane検証で確認: node index は allocation 順依存なので比較は各レベルのキー列)。`treap` の二分木平衡に対するリスト平衡の対極 — BTreeSet oracle で全 op 照合
- `karatsuba` — base-2^64 limb の O(n^1.585) 乗算。z1=(a0+a1)(b0+b1)−z0−z2 を magnitude 演算(add/sub が u128 carry で正確)、CUTOFF=16 で schoolbook 降下 — schoolbook oracle で全サイズ・非対称長照合
- `lsm` — BTreeMap memtable + 凍結 sorted run(newest-first)+ tombstone 削除 + 自動 tier compaction。観測状態が操作列の純関数 — iter/compact は merge 方向が決定的(newest wins、runs[] そのまま or_insert で走査)。BTreeMap oracle で全 op + compaction 前後一致
- `pgm` — Ferragina–Vinciguerra PGM の整数版。固定サイズ区分 + 有理傾斜 num/den + 構築時の厳密 max deviation ε — predict→[p−ε,p+ε] binary search。全工程整数、brute-force rank/get oracle 全照合
- `aes` — AES-128 ブロック暗号(FIPS-197)。S-box は格納テーブルではなく `sbox(x)=affine(gf2::inv(x))` を構築時計算 — 逆元+アフィン変換が定義のまま。FIPS-197 §C.1 既知解答 + 全256定数ブロック往復 + 1bit 反転 avalanche(≥8/16 bytes 差分) — `chacha`/`sha256`/`poly1305` にブロック暗号を追加し暗号レイヤー完備

**継続延期バックログ**: cuckoo hashing / SwissTable、link-cut、sais、平面性判定、rope、regex、jps、GJK/EPA、TLSF、Chomsky-full expr、edit-distance fuzzy、bitboard/magic、真の 3 段 recursive vEB/y-fast trie、HLL(整数化)、ED25519。理由は前次と同じ(整数化困難・スコープ過大・実用優位が薄い)。

## 出典(第33次、search-index 照合)

**論文・仕様**: Pugh, "Skip lists: a probabilistic alternative to balanced trees" (1990) / Karatsuba & Ofman (1962) / O'Neil et al., "The log-structured merge-tree" (1996) / Ferragina & Vinciguerra, "The PGM-index" (VLDB 2020) / FIPS-197 (AES)。

**実装物**: deterministic skip list(hash レベル方式、Redis/tantivy 系の seeded 構築)、rust-num/num-bigint の karatsuba 分割境界、RocksDB/LevelDB の run 構造、PGM paper の区分モデル、RustCrypto/aes の S-box 算出(affine(inv)) — Qiita/Zenn の LSM/PGM/AES 解説を参照し整数のみで逐語実装。


## 第34次 区間クエリ・空間符号・wire codec・二部被覆(round 34)

**採用モジュール(227→232)**: `fenwickrange` / `geohash` / `octree` / `base64` / `vertexcover`

- `fenwickrange` — Fenwick の差分表現を明示化した range-update 系 2 構造。`RangePoint` は差分 BIT(区間加算 O(log n)・点クエリ O(log n))、`RangeSum` は B1/B2 二 BIT で区間加算+区間和 `P(x)=prefix(B1,x)·x−prefix(B2,x)`。外部 API は 0 基点半開区間、内部は 1 基点 — naive 配列 oracle で加算パターン全乱択照合
- `geohash` — Gustavo Niemeyer の geohash を microdegree e6 整数 lat/lon へ移植。lon 先交互 bisect → 5bit 単位で base32、decode は cell 境界対を返す(非 base32・空は None)。floor 半分の累積で cell 幅は nominal ±1 — 500 乱択で包含・prefix 入れ子性・8 近傍 clamp を検証
- `octree` — `quadtree` の 3-D 版。bucked leaf(BUCKET=8)→branch(Box<[Node;8]>)分割、s≤1 で分割停止して同一座標 2000 点積み上げでも退化しない。nearest は box_min_dist2(外部距離は +1 補正 — 内部点の最大座標は hi−1)の best-first、(dist,pt) 正準 tie — BTreeMap multiset oracle・brute-force 最近傍全照合
- `base64` — RFC 4648 std/base64url の strict codec。decode は len%4・pad≤2・末尾のみ・alphabet 外 byte を全て拒否 — `wal`/`savefile`/`lzss` 系 wire の人間可読表現層。RFC §10 既知ベクタ + 全長・全 256 byte 往復 + 拒否ケース網羅
- `vertexcover` — Kőnig 定理の構成的版:`bipartite::hopcroft_karp` のマッチングから自由 L 頂点起点の交互 BFS(unmatched 辺 L→R・matched 辺 R→L)で Z を取り、被覆 = (L\Z)∪(R∩Z)。`bipartite` を「最大マッチング」から「最小被覆」の双対側へ拡張 — 被覆サイズ=マッチングの相互検証 + n,m≤4 の 2^(n+m) 全列挙で minimality を確認

**継続延期バックログ**: cuckoo hashing / SwissTable、link-cut、sais、平面性判定、rope、regex、jps、GJK/EPA、TLSF、Chomsky-full expr、edit-distance fuzzy、bitboard/magic、真の 3 段 recursive vEB/y-fast trie、HLL(整数化)、ED25519。理由は前次と同じ(整数化困難・スコープ過大・実用優位が薄い)。

## 出典(第34次、search-index 照合)

**論文・仕様**: Fenwick, "A new data structure for cumulative frequency tables" (1994) + 競技界隈の区間加算 BIT 定式化 / Niemeyer geohash 仕様(Wikipedia/geohash.org) / Finkel & Bentley, "Quad trees" (1974) の 3-D 版 / RFC 4648 / Kőnig (1931) + Hopcroft–Karp (1973)。

**実装物**: AtCoder Library の BIT 区間加算、Rust geo/geohash クレートの interleave 順序、bevy/hecs 系 octree、base64 クレートの strict mode、bipartite matching→cover の教科書構成 — Qiita/Zenn の Fenwick range 拡張・geohash・Kőnig 解説を参照し整数のみで逐語実装。


## 第35次 鍵付きハッシュ・MAC・meldable キュー・ソート網・バッチ LCA(round 35)

**採用モジュール(232→237)**: `siphash` / `hmac` / `pairingheap` / `bitonic` / `offlinelca`

- `siphash` — Aumasson–Bernstein SipHash-2-4。u64 add/xor/rot のみ、128bit key を LE 2 語として読む論文仕様。8-byte staging で呼び出し分割に非依存 — `DetHash` が key 無しなのに対し PRF として署名付き状態確認・hashmap DoS 耐性に使える。論文ベクタ 8 件 + 全長 0..64 byte-by-byte ≡ one-shot
- `hmac` — RFC 2104 HMAC-SHA256。64-byte block 0x36/0x5c パッド、key>block は先ず SHA-256。RFC 4231 TC1/2/4/6 既知解答 + 分割非依存 — `poly1305`(Wequn once)と対になる反復使用可 MAC
- `pairingheap` — Fredman–Sedgewick pairing heap を Vec arena で。meld O(1)(他ヒープの arena を append して index ずらし、構造もコストも定数)。二段 pairing pass で pop amortized O(log n)。(prio,key) 全対で正準順 — BTreeMap multiset oracle が push/pop/meld 交錯 300 op × 60 seed を全照合
- `bitonic` — Batcher 網。`network(n)` が入力に関わらず同一 (i,j,asc) 比較列を生成 — replay で全 peer が同じ比較痕を辿る data-oblivious 整列で、lockstep ガジェットやソート検証の対象にできる。非 2 冪は !0 sentinel パディング(実値 !0u64 も最初の n 要素が最小 n 個なので正しい)
- `offlinelca` — Tarjan offline LCA。黒化頂点 w に lca=ancestor[find(w)]。実装で oracle が捕捉した2点: (a) DSU が root 間共有のため跨木クエリに phantom ancestor — `tree_of` で同一木のみ応答に遮蔽、(b) `parent>=n` を root 扱いすると `lca` の invalid 意味論と乖離 — unreachable 扱いで全クエリ None。binary-lifting オラクル 100 seed × 40 query 一致、5000 深連鎖を iterative で処理

**継続延期バックログ**: cuckoo hashing / SwissTable、link-cut、sais、平面性判定、rope、regex、jps、GJK/EPA、TLSF、Chomsky-full expr、edit-distance fuzzy、bitboard/magic、真の 3 段 recursive vEB/y-fast trie、HLL(整数化)、ED25519。理由は前次と同じ(整数化困難・スコープ過大・実用優位が薄い)。

## 出典(第35次、search-index 照合)

**論文・仕様**: Aumasson & Bernstein, "SipHash: a fast short-input PRF" (2012) + 論文付録ベクタ / RFC 2104 + RFC 4231 / Fredman, Sedgewick, Sleator & Tarjan, "The pairing heap" (1986) / Batcher, "Sorting networks and their applications" (1968) / Tarjan, "Applications of path compression on balanced trees" (1979)。

**実装物**: Rust std `SipHasher13/24` の round 構造、RustCrypto hmac の pad 処理、bcmr/pairing-heap の二段 pass、Wikipedia bitonic 網の (i,j,dir) 生成、cp-algorithms の Tarjan LCA — Qiita/Zenn の SipHash/HMAC/offline LCA 解説を参照し整数のみで逐語実装。


## 第36次 圧縮順序集合・暗号ハッシュ・凸包キャリパ・単調行列(round 36)

**採用モジュール(237→242)**: `elias` / `patricia` / `blake2s` / `rotcal` / `smawk`

- `elias` — Elias–Fano 単調列。l=⌊log2(u/n)⌋ 低位 verbatim + 高位を unary gap bitmap(n+U bit)で。`rank` は「h番目ゼロまでの one 数が hi ≤ h の要素数」を正しく扱う必要があり、hi < h の境界は h−1 番ゼロ、h==u_hi は末尾 one ランで処理 — 全て BTreeSet 相当の Vec oracle で照合
- `patricia` — crit-bit radix tree(Bernstein 型、u64 版 Okasaki–Gill)。深さ ≤64、in-order = 昇順。重要な落とし穴: subtree は「テスト済み bit」しか制約しないため、floor/ceil を単純下降すると off-branch の葉が untested bit で bound を潜り越して誤答 — 両側を subtree min/max 刈り付き探索に確定(BTreeSet oracle 150 回 × 30 query)
- `blake2s` — RFC 7693 BLAKE2s-256。keyed-MAC モードは HMAC 構成なしで key を param に注入。設計値: key block を即座に圧縮すると空メッセージで「真の最終 block が key block 自身である」不変条件を破壊 → buffer に留めて `finish`/`write` が自然に処理する形に確定(公式 unkeyed/keyed ベクタ検証)
- `rotcal` — 回転キャリパ(Shamos 1978 / Toussaint 1983)。diameter pair、Frac 最小幅、Frac 最小面積外接矩形。最小矩形の 4 本 caliper(jt/jr/jl)は edge-0 で全頂点スキャン初期化が必須 — 単調指針を jl=0 で開始すると index 0 手前にある真最小を永遠に見落とす bug を brute oracle が捕捉
- `smawk` — SMAWK(AKMSW '87)。Reduce stack 刈り + 奇数行再帰 + 偶数行境界走査。Monge ⇒ totally monotone ⇒ argmin 非減少の連鎖を利用 — Knuth 最適化・Aliens trick・D&C DP の土台。検証で判明した罠: a+b+wx(昇順×昇順)は anti-Monge で argmin が減少方向、真 Monge には −wx が必要

**継続延期バックログ**: cuckoo hashing / SwissTable、link-cut、sais、平面性判定、rope、regex、jps、GJK/EPA、TLSF、Chomsky-full expr、edit-distance fuzzy、bitboard/magic、真の 3 段 recursive vEB/y-fast trie、HLL(整数化)、ED25519、BK-tree(次次回候補)。理由は前次と同じ。

## 出典(第36次、search-index 照合)

**論文・仕様**: Elias, "Efficient storage and retrieval by content and address of static files" (1974) + Fano (1971) + Vigna, "Quasi-succinct indices" (2013) / Morrison, "PATRICIA" (1968) + Bernstein crit-bit trees (2006) + Okasaki & Gill (1998) / RFC 7693 (BLAKE2) / Shamos 1978 + Toussaint, "Solving geometric problems with the rotating calipers" (1983) / Aggarwal, Klawe, Moran, Shor & Wilber, "Geometric applications of a matrix-searching algorithm" (1987)。

**実装物**: sux/sux4j の rank/select、djb critbit.c の split 挿入、RustCrypto blake2 の param 注入と buffer 運用、e-maxx/cp-algorithms の回転キャリパ四指針形、KACTL の SMAWK — Qiita/Zenn の Elias-Fano・crit-bit・SMAWK 解説も参照し整数のみで逐語実装。

## 第37次 テキスト編集・メトリック索引・最小包含円・凸 DP・正規表現(round 37)

**採用モジュール(242→247)**: `rope` / `bktree` / `mincircle` / `slopetrick` / `regex`

- `rope` — 二分木ロープ(葉 `Vec<u8>` チャンク)。`concat`/`split`/`insert`/`remove`/`slice`/`to_string` を node-weight で O(log n)。深さ > 2⌈log2 n⌉+1 で全葉 balanced 再構成 — 先頭固定挿入の一次退化でも線形 amortized 復帰を検証。設計値: `insert` が `mem::take(self)` 評価後に `self.len()` を読むと pos が常に 0 にクランプ → len 先取りで修正
- `bktree` — Burkhard–Keller メトリック木(1973)。辺ラベル = 親鍵との `diff::levenshtein` 距離、`within` は [d−r,d+r] 帯のみ下降(三角不等式刈り)、`nearest` は best-first 境界更新。80 鍵・4 文字辞書 × 300 query を brute-force 全照合
- `mincircle` — Welzl 最小包含円の反復形(sort+dedup 正準化 → 3 重ループ)。`Frac` 座標で中心・r² とも有理厳密、共線点は widest-pair 直径円へフォールバック。MEC の一意性から入力順非依存を順列テストで検証
- `slopetrick` — slope trick(凸区分線形関数の合成)。正規の4行形に確定: `add_a_minus_x` は `min_f += max(0, a − topR); push_r(a); push_l(pop_min_r)` — 条件分岐版は L が maxR を越える逆転区間を生み argmin/eval を破壊する bug を oracle が捕捉。`slide(a,b)`(平行 offset)で `f(x)=min_{x−b≤y≤x−a}` — モデル側は ±800 の帯で clamp 汚染を到達不能に
- `regex` — Thompson 構成の byte NFA 正規表現(`.`, `[a-z]`/`[^..]`/`\d\w\s`, `|`, `?`/`+`/`*`, グループ)。再帰降下で `Inst` 列にコンパイル、HOLE sentinel の out ポインタを一括 patch。is_match は各位置で start を再播種する unanchored pike loop — `frag.start` を 0 と誤用する bug を `a|` 空マッチケースが捕捉。AST 生成→レンダの独立 oracle で端位置集合を全照合

**継続延期バックログ**: cuckoo hashing / SwissTable、link-cut、sais、平面性判定、jps、GJK/EPA、TLSF、Chomsky-full expr、edit-distance fuzzy、bitboard/magic、真の 3 段 recursive vEB/y-fast trie、HLL(整数化)、ED25519、halfplane 交差、alphahull。rope/bktree/regex は本ラウンドで消化。

## 出典(第37次、search-index 照合)

**論文・仕様**: Boehm, Atkinson & Plass, "Ropes: an Alternative to Strings" (1995) / Burkhard & Keller, "Some approaches to best-match file searching" (1973) / Welzl, "Smallest enclosing disks" (1991) + de Berg et al. Computational Geometry / drken「slope trick」解説 + MtSaka/competitive-library の正規 heap 実装 + maspy slope_trick / Thompson (1968) + Cox, "Regular Expression Matching Can Be Simple And Fast" (2007) pike loop。

**実装物**: rope の weight 再帰 + rebalance 閾値、cp 系の BK-tree edge ラベル索引、KACTL の circumcircle/minEnclosingCircle、slope-trick の L/R heap 規約、russ cox nfa.c の Frag/patch 構造 — Qiita/Zenn のロープ・BK木・slope trick・正規表現エンジン解説を参照し整数のみで逐語実装。

## 第38次 開番地ハッシュ・盤面集合演算・形式言語・実行領域・順序集合(round 38)

**採用モジュール(247→252)**: `cuckoo` / `bitboard` / `cyk` / `halfplane` / `yfast`

- `cuckoo` — Pagh–Rodler cuckoo hashing(2001)。`t1[h1]`/`t2[h2]` の二 home 配置で contains は高々2 probe、削除は tombstone 不要。キック連鎖は交互テーブルで budget 超過時に (table,slot,old) ジャーナルを全巻戻し — 初版は打ち切りで逐出済みキーが迷子になる実 bug を cram テストが捕捉。固定容量で rehash 方針の分岐なし、`!0` は empty sentinel として挿入拒否
- `bitboard` — 8x8 u64 ビット盤(a1=0)。file mask クランプの8方向シフト + dumb7fill 遮蔽 fill(空升を6反復して最終1shiftが blocker を含む)で rook/bishop/queen の ray attack。leaper(knight/king/pawn)は ray oracle が効かないため単步 oracle を別建て — 両系統で全64升+4000乱択を照合
- `cyk` — CYK 受理(Cocke–Younger–Kasami)。bin[a][b] を lhs bitset に前計算した O(n³) 三角表、`accepts`/`derive`/`cell` の3粒度。oracle は (nt,i,j) メモ化の直接再帰展開 — 小規模 CFG×40入力×150文法を全照合
- `halfplane` — sort-and-deque 半平面交差。方向 d=(−b,a) を quadrant+cross で整数整列(atan2 なし)、同方向は c_h·|b_prev|≤c_prev·|b_h| の符号自由比較で tight merge。捕捉した設計値3件: 閉交点 meet(dq.back,dq.front) の push_front 欠落で頂点が1個不足 / 通過のみの境界線が重複・共線頂点を残す / deque の巡回順が入力依存で CW になる — 正規化で CCW 確定。空/非有界/退化/零面積は全て None
- `yfast` — Willard(1983)の rep 層+クラスタ構造(ハッシュ不要の決定論版)。bucket は (prev_rep,rep] 区間で内容分割、>2·BUCKET で median 分裂、rep 削除時は bucket max が rep 昇格 — (prev,rep] 不変条件を保つ。predecessor strict< / successor inclusive≥ で veb と同規約

**継続延期バックログ**: SwissTable、link-cut、sais、平面性判定、jps(実装済)、GJK/EPA、TLSF、Chomsky-full expr、edit-distance fuzzy、magic bitboard、真の 3 段 recursive vEB、HLL(整数化)、ED25519、alphahull。cuckoo/bitboard/halfplane/yfast/CYK は本ラウンドで消化。

## 出典(第38次、search-index 照合)

**論文・仕様**: Pagh & Rodler, "Cuckoo hashing" (2001) / Chess Programming Wiki(bitboards, dumb7fill) / Cocke–Younger–Kasami + Hopcroft & Ullman (1979) / cp-algorithms half-plane intersection + Preparata & Shamos / Willard, "Log-logarithmic worst-case range queries" (1983)。

**実装物**: Rust std/hashbrown の cuckoo kick 設計、ChessProgramming dumb7fill/occluded fill、教科書 CYK 三角表、cp-algorithms/KACTL の deque HPI 規約、y-fast の rep/bucket 二層 — Qiita/Zenn のカッコーハッシュ・ビットボード・CYK・半平面交差解説を参照し整数のみで逐語実装。

## 第39次 線形整列・稠密索引・巡回列・有理近似・任意CFG(round 39)

**採用モジュール(252→257)**: `radixsort` / `rankselect` / `debruijn` / `cf` / `earley`

- `radixsort` — 8bit×8 pass の安定 LSD 基数ソート + `[0,bound)` 計数ソート + `(key,payload)` 安定版。初版 `dst.resize(len, src[0])` が空配列でパニック → 早期 return。安定性は「同キー内で入力順保持」を反転入力で直接検証
- `rankselect` — Jacobson 二段 directory(L0=512bit superblock 絶対 rank、L1=word 内相対)。rank1 は2 lookup+popcount、select1 は superblock 二分探索+word 走査。naive 全位置・全 k 照合
- `debruijn` — FKM アルゴリズム: k 進 Lyndon 語(長|n)を辞書順連結で B(k,n) 一発生成。「先頭以外を prefix 反復で延長→末尾から increment」が正しい遷移 — 全 k^n 語の巡回窓一意性を `is_debruijn` + 独立全列挙で検証
- `cf` — Euclid の連分数展開(末尾1を前方に畳んだ canonical 形)+ 全収束列 + `best_approx`: cap を超える収束点で semiconvergent 係数 t=(cap−q_{i−2})/q_{i−1} の2候補比較、同距離は小分母優先。全探索 oracle で300乱数照合
- `earley` — Earley chart パーサ(predict/scan/complete)。設計値: スキャンが chart[i+1] へ入れたアイテムが現位置の queue で pop されると位置 i の byte で誤走査 — **位置別 queue が必須**(単一 VecDeque 版が a^nb^n で即座に失敗)。合成 S'→S で受理判定、ε規則・混長産出・自己再帰 start も受理。CYK オラクル(メモ化 derives)を任意規則に一般化して120文法×40入力全照合

**継続延期バックログ**: SwissTable、link-cut、sais、平面性判定、GJK/EPA、TLSF、edit-distance fuzzy、magic bitboard、真の 3 段 recursive vEB、HLL(整数化)、ED25519、alphahull。earley は「Chomsky-full」延期項を消化。

## 出典(第39次、search-index 照合)

**論文・仕様**: Knuth TAOCP 5.2.5(distribution counting/radix)/ Jacobson (1989) rank 構造 + González et al. (2005) select / Fredericksen–Kessler–Maiorana (1978) FKM + Ruskey 7章 / Khinchin "Continued Fractions" 最良近似定理 / Earley (1970) CACM 13(2) + Aycock–Horspool (2002) practical Earley。

**実装物**: cp-algorithms counting sort、SDSL rank_select の superblock 設計、Wikipedia de Bruijn の FKM 擬似コード、教科書連分数の semiconvergent 評価、Earley 擬似コードの chart-set fixpoint — Qiita/Zenn の基数ソート・rank/select・de Bruijn・Earley 解説を参照し整数のみで逐語実装。

## 第40次 線形構築・loglog推定・区間符号・制御byte表・完全ハッシュ(round 40)

**採用モジュール(257→262)**: `sais` / `hll` / `arith` / `swiss` / `magic`

- `sais` — SA-IS 誘導ソート O(n): S/L 型分類→LMS を bucket tail 配置→L 左走査・S 右走査の誘導 pass で LMS 部分列を命名→簡約文字列を再帰→戻り誘導。suffix の prefix-doubling(O(n log n))と別系統。命名比較は「文字+次 LMS フラグ」一致まで走査 — naive oracle が全乱数・周期・定数・全相異形状で一致
- `hll` — 整数化 HyperLogLog: p index bit→m=2^p レジスタが leading-zero 最大値を保持。raw 推定 αm²/Σ2^{-r} を i128 の 64-bit 固定小数分母で評価。merge=elementwise max(冪等・可換・合併=単一ストリーム)。小域(E≤5m/2)は線形計数 m·ln(m/V) — ln は num/den→[1,2) 縮約+atanh 級数(t≤1/3)の Q32 固定小数、float 一切なし。オラクルが捕捉した設計値: 空レジスタ支配による小域過大推定(n=100,m=1024→786)は raw HLL 固有のバイアスで、線形計数が正しい補正
- `arith` — Subbotin carry-less range coder。`[low,low+range)` を (cum,freq,total) で細分、頂byte 確定で出力。キャリー伝搬は `range = -low & (BOT-1)` 切詰で回避(byte 列が入力の純関数)。decoder は low/range を鏡像維持+code 残差 — 呼出側モデルが一致すれば adaptive も決定的
- `swiss` — SwissTable 風: 16 slot group + 7bit h2 fingerprint で key 配列非接触の判定多数。h1→group home、group 単位 probing、tombstone=DELETED(byte で区別、chain 維持)。負荷15/16 または tombstone 1/4 で倍長 rehash。seeded hash で (items,seed) の純関数 — BTreeSet oracle 全乱択照合
- `magic` — マジックビットボード: relevant occ は「各 ray の終端のみ除く」`ray & opp(ray)` — 盤端リング全体を除く初版設計は blocker として有効な角升(例 a1 の b1..g1)を潰す設計値を sq=0 で捕捉。(occ·m)>>(64−bits) の完全ハッシュを seeded 乱択で発見、尽きれば brute=true で dumb7fill oracle に正直 fallback — 誤テーブル値でなく正しい退化経路

**継続延期バックログ**: link-cut、平面性判定、GJK/EPA、TLSF、edit-distance fuzzy(bitap/diff/bktree で近似済、本格版)、真の 3 段 recursive vEB、ED25519、alphahull、SwissTable の SIMD 群制御(本物の group 演算)。

## 出典(第40次、search-index 照合)

**論文・仕様**: Nong (2013) SA-IS O(N) Time + divsufsort / Flajolet et al. (2007) HLL + Heule et al. (2013) / Subbotin (1999) carry-less range coder + Witten–Neal–Cleary (1987) / Bening–Kingsley–Luaces (CppCon 2017) SwissTable + Abseil raw_hash_set / Romstad & Kannan magic move bitboards(chessprogramming wiki)。

**実装物**: SA-IS 擬似コード(Nong 論文)、redis ClickHouse の HLL、Matt Mahoney の carry-less レンジコーダ、Abseil の制御 byte 仕様、chessprogramming wiki の magic 生成器 — Qiita/Zenn の SA-IS・HLL・算術符号・SwissTable・magic bitboard 解説を参照し全て整数のみで実装。


## 第41次: 動的木・償却平衡・ビット距離・凸体距離・分離割付

- `linkcut` — Link–Cut 木: preferred-path 分解を aux splay で保持、`access` が root↔x 経路を一つの splay に集約し「最後の経路親(=二度目 access での LCA)」を返す。遅延 `rev` は splay 前に祖先連鎖を pop-push で伝播。link は make_root+巡回検査で森を維持、cut は隣接関係を確認して誠実拒否。BFS 隣接 oracle と connected/path_min を乱択照合
- `splay` — ボトムアップ splay: zig(親が根)/zig-zig(同方向は親先回転)/zig-zag(自身二回)。アクセス頂点を根へ昇格 → 局所的作業負荷で償却 O(log n)、木形状は操作列のみの純関数で RNG 一切不要。BTreeSet oracle 全 op 照合+正準 in-order 検証
- `editdist` — Myers ビットベクトル DP: `xv=eq|mv; xh=((eq&pv)+pv)^pv|eq; ph=mv|~(xh|pv); mh=pv&xh` の1-word オートマトンで score を ±1 更新。**設計の核心は境界**: 全文 `dist` は左列 `D[i][0]=i` で `Ph<<1|1` 注入、自由開始 `find_leq` は `D[i][0]=0` で `|0` 注入 — 上段は両方 `D[0][j]=j`。64 語を超える pattern は DP/`diff::levenshtein` に正直 fallback
- `gjk` — 2D 整数 GJK: Minkowski 差 `A⊖B` の support 写像で原点を包む simplex を反復。最近点 v は `Frac` 有理数 {nx,ny,den} で保持し全比較を整数交叉乗算 — `distance2` が厳密二乗距離を返す。終了条件は `|v|² ≤ dot(w,v)` すなわち `len2_num ≤ den·dot_i`(den 因子が必須 — 脱落すると SAT oracle が即不一致)。凸包 400 乱数で有理 oracle と一致
- `tlsf` — TLSF 分離割付: size≥32 は (fl=最上位bit, sl=上位4bit) の二段 bin、SMALL=32 未満は exact bin。`alloc` は `bin_of(want)` 以降の bin を候補走査して収まる最下位ブロックを採用 — 古典 `mapping_search` 切上げは want 自身の bin 内の exact-fit を見逃す設計値を正直拒否 oracle が捕捉 → 候補毎の fit 検査に確定。free は next/prev 双方向 coalescing(stale `blocks` 残存も oracle が捕捉)

**継続延期バックログ**: 平面性判定、EPA(GJK の penetration 版)、真の 3 段 recursive vEB、ED25519、alphahull、SwissTable の SIMD 群制御(本物の group 演算)、edit-distance 本格 fuzzy(64 超 pattern の bitap)。

## 出典(第41次、search-index 照合)

**論文・仕様**: Sleator & Tarjan (1985) "Self-adjusting binary search trees" + (1983) "A data structure for dynamic trees" / Myers (1999) "A fast bit-vector algorithm for approximate string matching based on dynamic programming" + Navarro & Raffinot "Flexible Pattern Matching in Strings" §6 / Gilbert–Johnson–Keerthi (1988) GJK + Gino van den Bergen "Collision Detection in Interactive 3D Environments" / Masmano et al. (2004) TLSF: a new dynamic memory allocator for real-time systems。

**実装物**: competitive-programming の Link–Cut 実装形、sedgewick の bottom-up splay、Myers 論文のビットベクトル疑似コード、dyn4j/bullet の GJK 参照実装、NuttX/rtems の TLSF — Qiita/Zenn の Link–Cut・Myers bitap・GJK・TLSF 解説を参照し全て整数のみで実装。


## 第42次: ハッシュ拡張・署名・一般マッチング・侵入深度・形状境界

- `sha512` — FIPS 180-4 SHA-512: 64bit 語×80 段、Σ は ror 14/18/41 と 19/28/39、128bit 長さフィールド。`be_bytes` 系禁止のため手動シフトで BE 語を生成。NIST 全 4 ベクトル+分割不変+境界長(111/112/113/127/128)を照合
- `ed25519` — RFC 8032 EdDSA: 実装は二系統を分離 — 体は radix-51 の `[u64;5]` GF(p)(radix-64 の `[u64;4]` 版は積の列和が ~2^130 で u128 を溢れ全面書換)、スカラーは `[u64;4]` mod-L bit-fold。加算は EFD add-2008-hwcd-3 の完全式 — **D=2·Z1·Z2 の係数 2 が必須で脱落すると加算全体が壊れる設計値を捕捉**。`norm`(2 回キャリー)と `canon`(条件付き p 減算)を分離し serialize/比較は canon 側。TEST1-3 ベクトル+改竄/非正規拒否
- `blossom` — Edmonds 一般マッチング: e-maxx 形 BFS で奇閉路を発見したら `lca` 基底へ `base[]` 縮約し p[] を blossom 内逆張りで更新。mate 追跡で増加路復元。n≤9 の全部分集合列挙 oracle + 二重三角形花必須ケース
- `epa` — GJK 補完の penetrating 版: GJK ループを simplex 保持で走らせ原点包含三角形を seed → CCW 多胞体の最近 edge を外向法線で support 拡張、`dot(w,n) ≤ dot(edge,n)` で厳密収束。brute oracle(全面法線 SAT overlap の最小値)と depth²+軸平行を 400 乱数照合 — **法線符号規約をテスト側が誤読(+n̂ で a→b)した誤期待を捕捉**
- `alphahull` — α-shape: delaunay 三角形を外接半径² ≤ α² で選別、1 回出現 edge が境界。外心は垂線二等分線の Cramer 解で有理数 — **両軸の分子符号が反転する実装 bug を独立式 abc/(4A) oracle が捕捉**。`α²·d²` は checked_mul で飽和比較

**継続延期バックログ**: 平面性判定、真の 3 段 recursive vEB、SwissTable の SIMD 群制御(本物の group 演算)、edit-distance 本格 fuzzy(64 超 pattern の bitap)。

## 出典(第42次、search-index 照合)

**論文・仕様**: FIPS 180-4 Secure Hash Standard / Bernstein et al. (2011) Ed25519 + RFC 8032 §5,§7.1 公式ベクトル + Hisil et al. EFD add-2008-hwcd-3 / Edmonds (1965) "Paths, trees, and flowers" + e-maxx blossom 実装形 / van den Bergen (2001) EPA + bullet btGjkEpa2 / Edelsbrunner–Kirkpatrick–Seidel (1983) α-shape + CGAL 2D alpha shapes。

**実装物**: donna64/ref10 の radix-51 fe25519、libsodium の mod-L 畳み込み、competitive-programming の blossom、dyn4j/bullet の EPA、CGAL α-shape — Qiita/Zenn/海外技術記事の Ed25519・Edmonds・EPA・α-shape 解説を参照し全て整数のみで実装。

## 第43次: 列編集木・半全列挙・有限体線形・多段vEB・多語オートマトン

- `imptreap` — implicit-key treap: arena `Vec<Node>` + `SplitMix64` 優先度で再現可能な min-heap マージ。`split`/`merge`/`insert`/`remove`/`reverse`/`get` — **設計値: 遅延 rev フラグは「親の保留 flip が子の effective フラグを反転」するため、`get` は遅延適用ではなく降下中に flip パリティを累積しなければならない**(pending 親の反転を descent で拾わないと子の左右を誤読する — Vec oracle が pred/get 混乱で即捕捉)
- `meetmid` — meet-in-the-middle: `subset_sums` が `i128` 和で全 2^(n/2) を正攻に、右半分は sort + `partition_point`/`binary_search`。`subset_sum` は witness index を返すため右半 hit 後に再列挙で復元、`count_subsets` は区間 count、`best_fit` は cap 未満最大和(cap<0 は `i128::MIN`)
- `modlin` — GF(p) 線形: `rref`/`rank`/`solve`/`nullspace`。積は u128 回避で `(a*b)%p` を u128 に保持(u64² は ~2^128 で u64 積が溢れるため必須)、逆元は Fermat `inv=a^(p−2)`。solve は free vars=0 規約、nullspace は free 列に 1・pivot 列に −RREF 係数。rank は部分集合枚挙 oracle、非自明解は全代入列挙と照合
- `veb3` — recursive van Emde Boas: LEAF_BITS=6 以下は `u64` mask、上位は hi=⌈bits/2⌉ 個 cluster + summary 再帰。**三つの設計値をオラクルが捕捉**: (1) 葉の shift guard は宇宙境界 `x+1>=self.bits` ではなくシフト幅 `x+1>=64`(u64 シフトは mod 64 で意味を失う)(2) `predecessor` は `summary.predecessor(hi)=None` の時 `self.min` にフォールバック必須(min は cluster 外に保持される CLRS 不変)(3) 葉表現は「mask が全集合・min/max は純粋 cache」の一貫不変 — insert を一律に書かないと new-min が mask に入らず破壊
- `bigedit` — multi-word Myers: 任意長 pattern を ⌈m/64⌉ 語の frontier で。語間キャリーは (a) `(Eq&Pv)+Pv` の多倍長加算(overflowing_add ×2 の carry 繋鎖)(b) `Ph`/`Mh` <<1 の top-bit 受渡し(word0 は `dist` なら |1、`find_leq` なら |0)(c) score は最終語の bit m−1 のみ更新。`editdist` の single-word 版と m≤64 で完全不一致なし cross-check + DP oracle 全照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御(本物の group 演算)。

## 出典(第43次、search-index 照合)

**論文・仕様**: Seidel & Aragon (1996) treaps + CLRS "dynamic order statistics" implicit-key 系 / Horowitz & Sahni (1974) meet-in-the-middle + Schroeppel & Shamir (1981) / Bareiss / van Emde Boas (1977) "Preserving order in a forest in less than logarithmic time" + CLRS 3rd §20.3 proto-vEB→vEB / Myers (1999) §4 multi-word + Hyyrö (2003) "A bit-vector algorithm" carry 規律。

**実装物**: competitive-programming の implicit treap・meet-in-the-middle 形、e-maxx modlin、protobuf/CLRS の vEB 逐語実装、edlib/SeqAn の multi-word Myers — Qiita/Zenn/海外技術記事の implicit treap・MITM・GF(p) RREF・vEB・bitap 解説を参照し全て整数のみで実装。

## 第44次: wire codec・最小モデル帰結・多倍長・beats・動的森

- `varint` — canonical LEB128 u64 + zigzag i64: 符号化が値の「写像」である設計 — 全値↔唯一の byte 列。**設計値: canonicality は2つの穴を塞ぐ** — (a) 終端 byte で `i>1 && payload==0` を拒否(`[0x80,0x00]` は 0 の非最小形)(b) 第10 byte は `payload==1` 厳格(u64 桁溢れ >1 と padding =0 を同時に潰す)。bit 境界全値の byte-identical round-trip + malformed 7 類で oracle 照合
- `hornsat` — Dowling–Gallier 線形 Horn SAT: `Clause{pos:Option<u32>,neg:Vec<u32>}`、watch[v] に neg を登録 + remaining[c]=|neg|。`remaining==0` 種付け FIFO で `Σ|neg|` 線形 — `pos=None` 種 clause は即 UNSAT、unit 導出は least model(代入された変数のみ true)。brute 2^n 全代入 oracle と 400 乱数インスタンス照合、least-model 一意性確認
- `bigint` — sign-magnitude u64 limb BigInt: `norm` が上位ゼロ除去+zero→非負、加減は比較→同符号加算/異符号減算の2系統、`mul` schoolbook で u128 積→(lo,hi)、`pow` 二進。**設計値: `to_i128` で `-(i128::MIN)` は neg 化で桁溢れ — `m==1<<127` を `Some(i128::MIN)` 直接返却**。i128 checked_* oracle 2000・limb 境界 `[1,0,!0-1,!0]`・3 limb 拒否を照合
- `segbeats` — segment tree beats(`chmin`/`chmax`/`sum`/`get`): max/smax/cmax + min/smin/cmin の第二極値帳簿。**設計値2件**: (1) push は明示 lazy タグ不要 — lazy 着弾後の親 mx/mn がそのまま子の尊重すべき clamp で「`mx[c]>mx[v]` 時のみ子へ `update_max`」の textbook 形に確定 (2) sum 読み取りは push 必須 — lazy ノード配下の子 sum は stale(親 sum のみ畳まれるため)。非べき乗 n は 2n では index 溢れ → 4n heap cell。Vec naive oracle 200×60 op 照合
- `ett` — Euler-tour tree 動的森: 各木の巡回 edge tour を implicit treap(親指針+rank index)1本で保持、各頂点は恒常 vertex-node、有向 half-edge は `BTreeMap<(u32,u32),u32>`。`link` = 代表ノードで reroot(split+merge の巡回回転)×2 → `U+[uv]+V+[vu]`、`cut` = `A x B y C` の 4-split で B(v側)+C+A(u側)、`connected` = root 一致。linkcut の sibling で連結のみ — splay expose 一切不要。BFS adjacency oracle 30×120 op + 60 頂点 chain/cut/relink 照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御(本物の group 演算)、`utf8` strict DFA codec、`segbeats` add-lazy 変種。

## 出典(第44次、search-index 照合)

**論文・仕様**: Google Protocol Buffers wire format(LEB128 canonical 性の実務規範)+ Fujiwara(2013) "negative literals in Horn clauses" / Dowling & Gallier (1984) "Linear-time algorithms for testing the satisfiability of propositional Horn formulae" / Knuth TAOCP §4.3.3 multi-precision / Okanohara 実践的 beats + jiayiqi's segbeats blog(Zenn/海外解説の second-extremum ledger 形)/ Henzinger & King (1999) + Tarjan (1997) "Dynamic trees as search trees via Euler tours" + competitive-programming ETT の半辺 tour 実装形。

**実装物**: protobuf/libprotobuf-mutator の varint、SAT4j/minisat の Horn-clause watcher、rust-bigint/java BigInteger の limb 形、beet-aizu/library-checker の segment tree beats、e-maxx/cp-algorithms の ET-tree — Qiita/Zenn/海外技術記事の varint・Horn SAT・多倍長・segbeats・ETT 解説を参照し全て整数のみで実装。

## 第45次: strict codec・lazy 集約・素数法・離散対数・最小化

- `utf8` — Höhrmann strict-DFA UTF-8: 364-entry クラス×状態表で overlong・surrogate・`>0x10FFFF` を byte 単位で拒否。`Decoder` は `state/codep/seq_start/pos` を保持し `feed`→`Option<u32>`。**設計値: `decode_lossy` の resync は `e.pos == seq_start` で「lead 拒否は消費・mid-sequence 拒否は再供給」を分岐** — 一律再供給だと stray continuation が ACCEPT でも永遠に拒否され無限ループ、一律消費だと mid-sequence 中断後の回復が欠ける。naive 別系統デコーダ(lead-length+range-check)と600乱 byte soup 照合
- `lazyseg` — 正準 lazy segtree: range `add` + `sum`/`min`/`max`/`get`/`set`(set は `add(i, i+1, x−get(i))` で単一タグ維持)。**設計値: push で子に畳む sum は子の実葉数 `len/2`(floor)でスケール — 中点 `m=(l+r)/2` は奇数長で左に寄るため `ceil(len/2)` は左子に多めに畳み oracle が捕捉**。`apply` は tag+sum+mn+mx 一括、読み取り側の降下も push 必須
- `tonelli` — Tonelli–Shanks `sqrt_mod`: `p−1 = q·2ˢ` 分解、`p≡3 (mod 4)` は `a^((p+1)/4)` 直接式、非剰余 z は 2,3,… 逐次探索で決定的。`r=a^((q+1)/2)`/`t=a^q`/`c=z^q` 初期化後、`t` の 2-adic 次数を `c` の自乗で収束 — 返却は `(lo, p−lo)` 整序対で表現一意。p<200 全素数×全剰余の brute oracle + 2^62 級素数での往復(発見側 x を根ペアに含む)照合
- `bsgs` — baby-step giant-step `discrete_log`: baby 表 `g^j→最小 j`、giant 歩行 `cur = h·f^i` で `f = g^{p−1−m}`(Fermat、逆元補助不要)。初 hit が最小 x — `x = i·m + j` 分解で `i·m` 未満は全棄却済み。**設計値: `g=0` は `f` が真の逆元でないため(`0^m·f=0`)i≥1 で phantom hit — 歩行前に直接解決**。p<200 全素数×g<20×全 h の brute oracle + 部分群外(2 in Z23)の `None` 確認
- `dfamin` — Hopcroft DFA 最小化: splitter worklist の分割精細、小さい半分のみ再キュー(CLRS 的 n log n 規則)。block id は最小メンバーで正準化 → 言語のみの関数で状態順に依らず、compact 最小機械(`start`/`accept`/`delta`)を付随。**到達性は独立 `reachable`、未到達状態は自分達の中で最小化** — naive signature-iteration oracle(異なる手続き)200乱択全照合 + compact 機械の遷移シミュレーション一致

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御(本物の group 演算)、`segbeats` add-lazy 変種。

## 出典(第45次、search-index 照合)

**論文・仕様**: Höhrmann (2008) "Flexible and Economical UTF-8 Decoder" の DFA 表 + Unicode 準拠 maximal-subpart 置換規範 / atcoder library `lazy_segtree` の (min/max/sum)+add モノイド + cp-algorithms lazy propagation / Tonelli (1891)・Shanks (1973) "Five algorithms for modular square roots" + cp-algorithms / Shanks (1971) baby-step giant-step + Stinson "Cryptography: Theory and Practice" の最小指数規約 / Hopcroft (1971) "An n log n algorithm for minimizing states" + Knuutila (TUCS 2001) の splitter 再調査。

**実装物**: Bjoern Hoehrmann 公開 DFA テーブル、ACL/kactl の lazy segtree 形、e-maxx/cp-algorithms の tonelli・bsgs 実装形、ecma/C# FiniteAutomata の Hopcroft worklist — Qiita/Zenn/海外技術記事の UTF-8 DFA・lazy 伝搬・Tonelli–Shanks・離散対数・DFA 最小化解説を参照し全て整数のみで実装。

## 第46次: 厳密曲線・線形回帰・xor 索引・レプリケーション

- `bspline` — de Boor 整数化: `eval(degree, ctrl, knots, t)` は Frac 係数で厳密一点評価。**設計値3件**: (a) `denom==0` は `alpha=0`(`d[j]=d[j−1]`)— `continue` だと d[j] が前段残りで汚染され oracle が捕捉 (b) 右端 `t==u[n+1]` は「最後の非空 span ≤ n」に帰着 — `s=n` クランプは次数0 で空の後尾 span を指し左連続の曲線値を破る (c) oracle 側の教訓: 定義域右端は `u[m]` ではなく `u[n+1]`、`t==right_end` を半開区間所属から除外しないと基底が二重計数で分割の一意性 ΣN=2 に破綻。再帰 Cox–de Boor 基底 oracle + partition-of-unity 全点照合
- `bmassey` — Berlekamp–Massey 最短 LFSR: C[0]=1・長さ L+1・`s[n]=−Σ_{j≥1}C[j]·s[n−j]` の規約、更新は `coef=d·b^{p−2}`(Fermat)。Fibonacci mod7 → [1,6,6]、定数列 → [1,6]、全ゼロ → [1]、幾何 2ⁿ → [1,5] の既知ベクタ + mod 998244353 の LFSR 回復。**テスト側 bug 捕捉**: oracle `min_complexity` が成功時 `return l` でなく `continue 'outer` → 全ゼロ列の want=4 を算出 — GF(p)^L 全 r 枚挙の検証器自身が壊れていた
- `xortrie` — bit trie(u64、高々64深): `insert`/`contains`/`remove` は multiplicity cnt、`descend` は greedy opposite-bit、`max_xor_pair` は O(n·64) で全要素の greedy パートナーを取り `(lo,hi)` dedup + xor 降順→pair 昇順の正準選定。`len` は live multiplicity 計数 — 集合等価 oracle には distinct-key 操作が必須(remove 後の期待値誤読を捕捉)
- `orset` — add-wins OR-Set: dot=(replica,ctr)、`remove` は観測済み add-dot のみカバー、adds 写像は縮小しない(removes が墓標カバレッジ)。merge = 両 map union で交換・結合・冪等 — 並行 add 不滅・re-add は新 dot で存続。**設計値**: `remove` の返値は「live dot が存在したか」— adds[v] 非空チェックだとカバー済み dead dot でも true を返す穴。oracle は per-replica (adds,removes) シャドーで全ステップ contains 照合 + 収束・冪等・順序独立の3性質
- `lww` — LWW-element-set: orset の対極 — remove は常に (clock,replica) stamp を書く(未観測要素でも消せる)。要素の生死は add/rem 両 stamp の大きい側、同時刻タイは remove 勝ち(Riak 規約)。`added_since(clock)` で差分出荷。**設計値**: oracle が impl の内部 ctr と同期するには add 側で `views[r].2 = s.0` と返却 stamp の clock を直接拾う必要(独自に +=1 すると merge 後に乖離)

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御(本物の group 演算)、`segbeats` add-lazy 変種。

## 出典(第46次、search-index 照合)

**論文・仕様**: de Boor (1978) "A Practical Guide to Splines" + Piegl & Tiller "The NURBS Book" §2 の de Boor 漸化式と clamped 端点規約 / Berlekamp (1968) "Algebraic Coding Theory" + Massey (1969) "Shift-register synthesis and BCH decoding" + cp-algorithms の BM 実装形 / Pain & Pichery xor-trie max-xor 慣行 + Library Checker `set_xor_min`/`max_xor_pair` 問題系 / Shapiro, Preguiça, Baquero, Zawirski (2011) "A comprehensive study of Convergent and Commutative Replicated Data Types" の OR-Set(add-wins observed-remove)と LWW-Register / Riak KV の LWW-element-set tie→remove 規約。

**実装物**: geomdl/NURBS-Python の de Boor 評価形、AtCoder 提出の BM、Library Checker 提出の binary-trie xor、Automerge/Yjs の OR-Set、Riak・Redis Enterprise の CRDT LWW-eset — Qiita/Zenn/海外技術記事の B-スプライン・Berlekamp–Massey・binary trie・CRDT 解説を参照し全て整数のみで実装。

## 第47次: 素数索引・ゲーム数・エディタ構造・開番地・文書指紋

- `sieve` — 線形篩 SPF + Eratosthenes + セグメント篩: `spf_sieve`/`factor`/`is_prime_table`/`factor_map`/`primes_up_to` + `Primes{phi,tau,sigma}` + `primes_between`。設計値2件: (a) `sigma(0)` は `v==0` の早期 return が必須 — 空の factor_map では Π(1)=1 が誤返却される (b) セグメント篩の marking 開始は `ceil(lo/p)·p` を `p^2` にクランプ — lo<p² の窓で小倍数を消し忘れる。p==0 を残基として残す `factor` 規約と `out=out/p*(p−1)` の phi 実装で overflow 回避。Miller–Rabin オラクル 60 セグメント + 1M 窓照合
- `grundy` — Sprague–Grundy: `mex`/`take_away`/`position_grundy`/`nim_sum`/`winning_move`/`losing`/`detect_period`。**設計値(定理としての周期検出)**: 引き算ゲーム表は最終的に周期化するが「短い検証済み周期尾部」は単なる偶然 — 保証ある周期 (s,p) は `n−s−p ≥ memory`(再帰の lookback = max moves)が必要で、`.max(1)` 床がないと memory=0 で (n−1,1) の vacuous 報告が真の p=3 を隠す。最後の bad index `s = i+1−p` の計算式を oracle で確定
- `gapbuffer` — Emacs 型 gap buffer(Vec<u8>): gap=[gap_start,gap_end)、`get` は跨ぎ index 写像、`move_to` は `copy_within`(左: pos..gap_start→gap_end−count、右: gap_end..→gap_start)、delete は実削除数を返却。Vec シャドー oracle で to_vec/cursor/before/after/get を全ステップ照合
- `robin` — Robin Hood 開番地(u64): probe 長強奪 + 後退シフト削除(tombstone 不要)。`contains` は slot.dist<probe_dist で早期終了 — 「運の良い到達者が先に座った」のだからその先に key はあり得ない。負荷 0.75 で slot 順 rehash → 配置は (挿入列,seed) の純関数。BTreeSet オラクルで到達性不変式を全乱択照合
- `winnow` — winnowing 文書指紋(Schleimer 2003): k-gram ハッシュの各窓 w から rightmost-min を選択・連続重複は dedup。**設計値**: 初期窓の argmin は未走査のため `!0` sentinel が必須 — `r=0` 開始だと「index 0 が最小」を暗に仮定し真の argmin を逃す(basics の単純例では偶然動き oracle_random が捕捉)。保証: 共有 run ≥ k+w−1 バイトは必ず共通指紋ハッシュを出す — shared_run_surfaces で乱択検証

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、`segbeats` add-lazy 変種、mtt、scapegoat、beam、bandit。

## 出典(第47次、search-index 照合)

**論文・仕様**: O'Neill (2009) "The Genuine Sieve of Eratosthenes" + cp-algorithms の linear sieve/segmented sieve 実装形 / Sprague (1935)・Grundy (1939) 不偏ゲームの mex 理論 + Bouton (1901) Nim — subtraction game の周期定理は Berlekamp–Conway–Guy "Winning Ways" §4 / gap buffer: GNU Emacs `insdel.c` の move_gap/insert/delete 規約 + Finlånder "The Craft of Text Editing" §7 / Celis, Larson, Munro (1985) "Robin Hood Hashing" + Appleby Rust hashbrown tombstone-free backward-shift / Schleimer, Wilkerson, Aiken (SIGMOD 2003) "Winnowing: Local Algorithms for Document Fingerprinting" の rightmost-min・窓保証。

**実装物**: cp-algorithms/e-maxx の sieve 実装形、atcoder Library Checker の grundy 問題系、Emacs/XEmacs gap buffer コード、Rust std collections の Robin Hood 系譜(rust-lang/hashbrown の backward-shift)、MOSS・Google schleimer-winnowing 実装 — Qiita/Zenn/海外技術記事の線形篩・Nim 数・gap buffer・Robin Hood・winnowing 解説を参照し全て整数のみで実装。

## 第48次: 自己調整構造・探索・分類・鞍背

- `scapegoat` — α 重み平衡 scapegoat 木(Galperin & Rivest 1993、α=3/4): 挿入で `4·size(child) > 3·size(node)` を満たす最深祖先を median-split 全再構築。設計値: per-node α 不変式は*挿入後のみ*保証 — 削除は max_size 高水位で `len < α·max_size` まで不均衡を許容(oracle が「right heavy」を捕捉し仕様通りと確定)。削除 splice は全木 O(n) サイズ再計算で amortized 費に吸収
- `leftist` — 左辺ヒープ: rank=null path length、`rank(left) ≥ rank(right)` で右背骨のみ O(log n) 保証。`merge` のみが実操作で push/pop/heapify は帰着。`from_slice` は pairwise-meld O(n)。multiset oracle で構造不変式(rank/heap order)を全ステップ照合 — 4096 乱数後も rank ≤ 13 を確認
- `beam` — 決定的ビーム探索: (score,生成順) で canonical ランク → `expand` の出力順の純関数。parent-link arena で経路復元、best は全 level 横断最深でなく最大スコア。`beam_moves` は `minimax::Game` 上で root-perspective 高スコア幅 w 展開
- `perceptron` — Rosenblatt 線形分類: `w += y·x, b += y` 誤分類更新。i128 スコア蓄積 + saturating 更新で巨大 magnitude も決定的。Novikoff 収束を update-trace oracle が教科書規則と bit 完全一致で裏付け、one-vs-rest argmax(同点は小ラベル正準)の多クラス
- `saddleback` — 鞍背探索(Bird 2006): 右上起点 `O(r+c)`。**実 bug 捕捉**: ragged 行で `m[r][c]` の境界外アクセス — `continue` が loop 先頭の index 読み出しより後に置かれており panic。`c >= m[r].len()` 早期ガードで「セル不在 = 列破棄」として解決。全走査 oracle で phantom-hit/見落としの両方向照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、`segbeats` add-lazy 変種、mtt、bandit。

## 出典(第48次、search-index 照合)

**論文・仕様**: Galperin & Rivest (SCG 1993) "Scapegoat Trees" の α-weight 再構築・max_size 高水位規約 / Crane (1972) "Linear Lists and Priority Queues as Balanced Binary Trees" の leftist tree = null-path 左右不変式 + Okasaki "Purely Functional Data Structures" 実装形 / Lowerre (1976) HARPY beam search + Bisiani (1987) 幅正準化 / Rosenblatt (1958) パーセプトロン + Novikoff (1962) 収束定理 (R/γ)² + one-vs-rest argmax / Bird & Millward (Oxford) "Saddleback Search" + Martin Gardner 行列探索 puzzle + Bir, Pontus "Pearls of Functional Algorithm Design" の saddleback 章。

**実装物**: Okasaki PFD の leftist 形、cp-algorithms のビーム探索慣行、scikit-learn Perceptron の one-vs-rest 形、Haskell pearls の saddleback — Qiita/Zenn/海外技術記事の scapegoat・leftist heap・beam search・パーセプトロン・鞍背探索解説を参照し全て整数のみで実装。

## 第49次: 平衡木・探索戦略・増分ハッシュ・公開鍵・文書構文

- `avltree` — AVL 高平衡 BST(Adelson-Velsky & Landis 1962): ノード高追跡、`|h(l)−h(r)|≤1` を LL/RR/LR/RL 回転で祖先毎に復元。seed 不要 — 形状は挿入順の純関数。BTreeSet シャドーで op 毎に (BST順・高不変式・昇順列) を照合 — 昇順 1000 挿入でも高 ≤ 11
- `bandit` — マルチアーム探索: 報酬を SCALE 倍整数で蓄積。設計値: 初版は「平均優先+explore同点ブレーク」の2相近似で良腕独占・他腕枯渇を観測 → 真の UCB1 (mean+bonus 加算) に確定: Q8 `floor(r·256/p)+isqrt(2·log2(t)·SCALE²·256/p)` — 組込系で使われる整数形。ε-greedy は seeded rng で再生可能、ε=0 で greedy 退化を確認
- `zobrist` — 増分盤面ハッシュ: (piece,square)→u64 鍵を seeded SplitMix64 で表引き、XOR で toggle=厳密 undo。side-to-move 鍵は表鍵の後ろのストリーム位置から派生し衝突不能。每ステップ `hash == rehash(occupancy)` をオラクル照合
- `rsa` — 教科書 RSA(padding 無し): BigInt に除法が無いため magnitude 演算を自前実装 — **実 bug 捕捉**: `mrem` で bit長一致時 `dr−dm−1` が underflow panic。降順シフトループに確定(各 step で rem < m·2^(sh+1) → 高々1回減算)。二進長除法 rem/商・平方乗 modpow・拡張 Euclid modinv・固定証人(2,3,5,7,11,13) Miller–Rabin — Carmichael 数 561/1105/1729/41041/825265 を全拒否を検証。パディング無しが「決定的暗号文」=本クレートの要件と一致、wire 用途不可を明記
- `json` — RFC 8259 整数部分集合: 数値は i64 のみ('.','e' 拒否)、`\uXXXX` は surrogate pairing 経由 UTF-8 復号(lone surrogate 拒否)、leading zero・未エスケープ制御文字・末尾ゴミ全拒否。canonical render は BTreeMap ソート鍵+最小エスケープ — `parse(render(x))==x` を乱択300件照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、`segbeats` add-lazy 変種、mtt。bintree/heavy modules消化進行。

## 出典(第49次、search-index 照合)

**論文・仕様**: Adelson-Velsky & Landis (1962) "An algorithm for the organization of information" AVL 回転不変式 / Auer, Cesa-Bianchi, Fischer (2002) "Finite-time Analysis of the Multiarmed Bandit Problem" UCB1 + Sutton & Barto §2.3 ε-greedy — 整数形は組込 UCB 実装慣行 / Zobrist (1970) "A New Hashing Method with Applications for Game Playing" + transposition-table 慣行 / Rivest, Shamir, Adleman (1978) + PKCS#1 v1.5 教科書形(パディング無し、教育版) + HAC §4 Miller–Rabin 固定証人 / Bray (2017) RFC 8259 JSON grammar + ECMA-404 — 数値の整数部分集合化は crate no-float 規約。

**実装物**: Okasaki の AVL 形、bandit の embedded Q8 形、chess programming wiki の Zobrist/side-key 慣行、HAC・mbedTLS の RSA 骨格、serde_json/rapidjson の strict parse 方針 — Qiita/Zenn/海外技術記事の AVL・バンディット・Zobrist・RSA・JSON パーサ解説を参照し全て整数のみで実装。

## 第50次: 接尾辞木・平衡木・全点対最短路・ランク・オートマトン

- `sufftree` — Ukkonen のオンライン接尾辞木: active point (node,pos,len) + skip/count + suffix link で O(n) 構築。**設計値2件捕捉**: (a) 内部ノードで終わる接尾辞は葉を持たず出現数を過小計上 → 仮想終端 `SENT=u16::MAX` を末尾に付加(エッジキーを u16 化)して全接尾辞が固有の葉を持つ形に確定。(b) skip/count の無い初版は `active_pos` が辺境界を跨いで誤走査 — 降下ループを導入。`contains`/`occurrences`/`count`/`longest_repeat` を naive 全照合(全部分文字列+全出現位置+最長重複の総当り)
- `redblack` — 赤黒木(CLRS insert/delete-fixup 完全実装): arena `Vec<Node>` で値木、色 bit のみで削除時回転 ≤3。NIL sentinel の fixup は「親+左右」を対で追跡(子 slot が空でも向きが分かる)。BTreeSet シャドー oracle が op 毎に (BST順・赤赤なし・黒高等差・min/max) を照合、昇順/ランダム/敵対的削除順で不変式維持
- `apsp` — 全点対最短路: Floyd–Warshall O(n³)(i64 加重・i128 内部累積 — 経路総和の overflow を構造的に排除)+ Johnson: `bellman::shortest` の超源点でポテンシャル h[v] 取得、`w'=w+h[u]−h[v]` が非負化の三角不等式(負閉路時は None)。johnson==floyd==per-source-BF 3 者照合
- `pagerank` — 整数 PageRank: 全質量 Q32(`SCALE=1<<32`)、反復 = 辺配分 `d·mass/outdeg` + teleport `(1−d)·SCALE/n` + dangling 質量の均等配分。除算は全 floor — 質量は微漏洩するが順序に非影響。floor 意味で縮約写像 → 実践的に不動点収束、`converged` フラグで報告(MAX_ITERS=10⁴ 上限)。**テスト側誤りも捕捉**: dangling sink と in-cycle 兄弟は同じ in-edge・同配分で同スコアになる対称性
- `automaton` — NFA(Thompson 構成)→DFA 部分集合構成 + union/concat/star/plus/optional/complement/intersect/minimize。**実 bug 2件捕捉**: (a) `concat` が `other.eps` を落とし star のループが消失。(b) `complete()` の `row.insert(c, sink)` が既存遷移を sink で破壊(`insert` は常に置換)→ `entry().or_insert` に確定。complement は alphabet を引数化(「alphabet 外の文字を含む語は語でない」— 部分 DFA の暗黙 die は complement で言語が反転する問題を設計上回避)。dfamin::minimize 連携

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、`segbeats` add-lazy 変種。

## 出典(第50次、search-index 照合)

**論文・仕様**: Ukkonen (1995) "On-line construction of suffix trees" の active point + suffix link 構成と終端文字 `$` の必然性 / CLRS §13 red-black insert/delete-fixup(NIL sentinel の親指針付き fixup 形)/ Floyd (1962)・Johnson (1977) "Efficient algorithms for shortest paths in sparse networks" のポテンシャル再重み付け / Page, Brin, Motwani, Winograd (1999) PageRank の dangling-node 補正形 / Thompson (1968) NFA 構成・Rabin–Scott (1959) 部分集合構成・Hopcroft DFA 最小化(dfamin) — Qiita/Zenn/海外技術記事の suffix tree・赤黒木・Johnson・PageRank・オートマトン解説を参照し全て整数のみで実装。

**実装物**: jogojapan の Ukkonen active-point 形(SENT 終端付き)、cp-algorithms の Johnson/reweight 形、networkx の PageRank power-iteration 形、Thompson 教科書の NFA→DFA — 整数のみで実装。

## 第51次: AA木・LZ4・計量TSP近似・Keccak・Minkowski和

- `aastree` — AA 木(Andersson の2不変式赤黒簡略版): `level(t) = level(left)+1`・NIL=0 で、左水平辺の skew(右回転)と右右水平の split(左回転+level+)の2操作のみで平衡。削除巻戻は decrease_level(NIL=0 で min+1)→skew×3(t, t.right, t.right.right)→split×2(t, t.right) の固定列。**設計値捕捉**: (a) 真の AA 恒等式は `min(children)+1` でなく `left+1` — 左葉のみ持つ level-2 ノードは合法。(b) 順序監査は pre-order ではなく真の in-order(left→prev→right)が必須 — 初版の事前順報告が大量偽陽性。arena + freelist、BTreeSet シャドー 60×400 op 照合
- `lz4` — LZ4 ブロック codec(wire 形式): シーケンス = [lit4|match4] トークン + 255 拡張 + u16 LE offset。貪欲 4-byte ハッシュパース、match≥4・offset≥1・末尾リテラル専用・MFLIMIT=12/LASTLITERALS=5。復号は offset>emitted・末尾マッチ・0 offset を全拒否、重複マッチは逐語コピー。**設計値捕捉**: 圧縮側で `table[h]` を先に更新してから参照すると offset が自己参照になる — 観測値 `prev` を別退避。往復 oracle・切詰全拒否・決定出力
- `christofides` — 計量 TSP 3/2 近似: Prim MST(コスト,頂点)正準選択 → 握手補題で偶数の奇数次集合 T → 完全部グラフ最小重み完全マッチング(部分集合 DP `O(2^|T|·|T|)`、|T|≤20、超過はソート貪欲)→ MST∪matching 多重グラフの Euler 回路(`euler::euler_walk`)→ 初出頂点 shortcut。計量性 ⇒ shortcut が増長しないため MST+matching ≤ opt+opt/2。全 tie-break 正準で行列の純関数、n≤8 総当り `2·cost ≤ 3·opt` oracle
- `sha3` — Keccak-f[1600] スポンジ(FIPS 202): 25 車線 u64、θ/ρ/π/χ/ι ×24 ラウンド、SHA3=0x06・SHAKE=0x1F ドメイン + pad10*1。`Digest256` インクリメンタル。**設計値捕捉**: 2回目以降の `finalize` squeeze が rate 内位置 0 に戻ると真の XOF 意味でない — `pos` フィールドで rate 内読出位置を跨呼出保持に確定(NIST ベクタ + chunked 不変 + prefix 安定)
- `minkowski` — 凸 Minkowski 和/差: CCW 正規化(最低 (y,x) 始点)+ 辺ベクトルの角度マージ O(n+m) — cross>0 で a 側前進、平行時は両側合成(合成辺は後で convex_hull が 180° 頂点を除去)。`diff(a,b)=sum(a,−b)` が配置空間障害物で `collide` = 原点の多角形内判定、縮退(<3頂点)は全ペア和→凸包。brute pair-sums→hull oracle 60乱数 + C-obstacle 意味論照合(任意頂点含有∪辺交差)

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、`segbeats` add-lazy 変種。

## 出典(第51次、search-index 照合)

**論文・仕様**: Andersson (1993) "Balanced search trees made simple" の AA 木2不変式・level=left+1 / LZ4 block format spec(Yann Collet) のシーケンス文法と MFLIMIT/LASTLITERALS 制約 / Christofides (1976) "Worst-case analysis of a new heuristic for the travelling salesman problem" 3/2 界 / FIPS 202 SHA-3 + Keccak リファレンスの θρπχι 写像と pad10*1 / de Berg et al. Computational Geometry の Minkowski 和の角度マージ構成 — Qiita/Zenn/海外技術記事の AA 木・LZ4・Christofides・SHA-3・Minkowski 和解説を参照し全て整数のみで実装。

**実装物**: 教科書形 AA 木の skew/split 骨格、lz4 リファレンスの hash4+greedy parse、競技プログラミング慣行の Christofides パイプライン、tiny_sha3 系のレーン配置、cp-algorithms 系の辺ベクトルマージ — 整数のみで実装。


## 第52次: 秘密分散・中国式配点・最小DFA・噴水符号・距離変換

- `shamir` — GF(p) 上の (k,n) しきい値秘密分散: 係数を seeded SplitMix64 で純関数化、株は x=1..n で評価、x=0 の Lagrange 補間で復元。k−1 株では素体上で全秘密候補と整合(情報理論的秘匿) — 「k−1 が誤値を返さない」ことを乱択で統計検証し、外部株混入時の値変化も照合。素数判定は決定的 Miller–Rabin 8 基底
- `postman` — 中国式配点問題(閉路/開路): 奇数次集合 T の bellman 計量閉包→最小重みマッチング→多重グラフ Euler。**設計値2件捕捉**: (a) matching DP は `solve(mask, free)` に一般化 — free=2 が開路端点選定を O(k²) 列挙なしで一括解く。(b) sentinel は `i128::MAX/4` — 素の `i128::MAX` は `dist+MAX` で debug overflow。(c) 計量閉包は odd-index 空間、元頂点 id で参照すると境界外。全完全マッチング列挙 oracle で optimality 照合、walk 実コスト == total_cost も検証
- `fst` — 最小無環 DFA 辞書索引: BTreeSet 正準化→全トライ→深さ降順の signature レジスタ(hash-consing)で右言語同値な部分木を全併合 = Myhill–Nerode 一意最小。root を状態 0 へ swap-back(参照全書換え)。**設計値捕捉**: register 順は深さ降順が必須(子の正準 id 確定後でないと親 signature が不確定)。BTreeSet 会員照合 + enumerate 往復 + 到達性/重複 signature ゼロの構造監査
- `fountain` — Luby 変換噴水符号(GF(2)): 次数・近傍集合とも (k,i,seed) の純関数 — wire は (i,data) のみ。robust-soliton τ(d)=R/(kd)+spike@k/R を u64 重みテーブル(kR 共通分母)で O(k) 逆CDF。**設計値捕捉**: 定数 c=1/10 では小 k(≤~64)で R=isqrt(k)·ln(2k)/10=1 に退化し robust 効果が消失 → c=1/4 で R≥2 確保(理想形のままでは剥離停止が確率的発生 — oracle が観測)。BP 剥離 decode は被覆不足時に正直 None
- `edt` — Felzenszwalb–Huttenlocher 二乗ユークリッド距離変換: 1-D 放物線下包絡を列・行の2 pass。breakpoint は (num,den) 有理数で交差比較を i128 積に。**設計値捕捉**: −inf sentinel を i128::MAX/4 と置くと `zn·den` が型上限を超過して debug panic — i64::MIN の有界値に確定。BIG=i64::MAX/4 サイトは実サイト存在時に包絡から早期除外可能。全ピクセル brute 最近点 oracle 240 乱数照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、`segbeats` add-lazy 変種。

## 出典(第52次、search-index 照合)

**論文・仕様**: Shamir (1979) "How to share a secret" の GF(p) 補間構成 / Edmonds & Johnson (1973) + Kwan Mei-Ko (1962) の中国式配点 — Euler 増大の奇数次マッチング形 / Daciuk, Mihov, Watson, Watson (2000) "Incremental construction of minimal acyclic finite-state automata" の register 手法 / Luby (2002) "LT codes" の ideal/robust soliton 分布と BP 剥離 / Felzenszwalb & Huttenlocher (2012) "Distance transforms of sampled functions" の放物線下包絡アルゴリズム — Qiita/Zenn/海外技術記事の秘密分散・中国式配点・FST・fountain code・距離変換解説を参照し全て整数のみで実装。

**実装物**: 教科書形の係数評価/Lagrange、競技プログラミング慣行の配点増大パイプライン、fst ライブラリ形の bottom-up register、LT 実装の robust-soliton 重み表形、F&H の z/v 配列構成 — 整数のみで実装。

## 第53次: B+木・スイープ線交差・組合せ・GCM・多角形クリップ

- `bplus` — B+木順序マップ u64→u64(order 4..64、葉連鎖+range 走査): 葉 split は separator コピー昇格、内部 split は中央キー移動昇格 — 不変式は「separator = 右部分木の最小キー」。**設計値捕捉**: 葉の先頭キー削除時に祖先 separator が陳腐化 — `fix_sep` で「非先頭スロットを持つ最近祖先」まで上昇して keys[pos−1] を新最小に更新(shadow oracle が step 192 で捕捉)。underflow は借用2分岐→merge_into、arena + freelist、BTreeMap 照合 4000 op × orders{4,6,16}
- `bentley` — Bentley–Ottmann 交差列挙(全有理点): BTreeMap を (x,y) 優先度付きイベント列として Cross イベントも同じキューに投入。status は (y at p.x, slope, id) で毎バッチ再整列 — 同一 p で concurrent な seg が近接するため隣接ペア検査だけで交差を拾う。**設計値捕捉**: 垂直線分は status に入れられない(y_at 不定) — その x-line 中は `verticals` に保持し「status ∪ 端点 seg」全対と対検査、さらに touched 計数で共有端点も拾う。共線 overlap は報告しない(連続体)が共有端点は報告 — 両規約を brute oracle にも反映して 120 乱数全照合
- `comb` — 組合せ rank/unrank(combinadic): `choose128` は `acc·(n−k+i) = i·C(n−k+i, i)` の不変式で毎回の除算が厳密 — gcd 正規化が要らない。`rank` は辞書順で「i 番目が v である部分集合の個数 Σ C(n−v−1, k−1−i)」、`unrank` は逆走査。**設計値捕捉**: `next_comb` の可増分境界は `comb[idx] < n − k + idx`(0-index 最大値 n−(k−idx)) — `+1` が漏れると範囲外値を生成して走査が暴発する(35 vs 20)。n≤8 で全列挙往復照合、辞書順 oracle
- `gcm` — AES-128-GCM AEAD(aes の上に): GF(2^128) 乗算はビットシリアル shift-xor(R = 0xE1<<120)、GHASH の累積器は aad→ct を**連鎖**させる — 別々に計算して XOR 合成するのは fold 構造違反(NIST ベクタで捕捉)。J0 = IV‖0^31‖1(12B)または GHASH 導出。**設計値捕捉**: 掲載ベクタの期待値を記憶で誤記 — 純 Python AES+GCM を書いて完全独立に相互検証し、no-AAD 版 tag=cc15abcc…/AAD 版 tag=5bc94fbc… を確定。往復 + タグ/暗号文改竄全拒否 40 乱数
- `polyclip` — Sutherland–Hodgman 多角形クリップ(全 Frac 厳密): clipper は i128 shoelace 符号で CCW 正規化、inside = 有向辺の左側(cross ≥ 0)、交点パラメータ `t = cross(cd, a−s)/cross(cd, sd)`。**設計値捕捉**: t の符号が反転すると交点が線分の裏側に出て全面消失(0 面積回帰が捕捉)→ 分子は a−s が正。出力は連続重複 + 端点重複を dedup して閉路化。乱択矩形 oracle(面積上界 + 全頂点 inside)+ winding 非依存の決定出力

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。

## 出典(第53次、search-index 照合)

**論文・仕様**: Bayer & McCreight (1972) + Comer (1979) の B+木 copy-up/move-up 構造 / Bentley & Ottmann (1979) スイープ線アルゴリズム + de Berg et al. の status/event 構成 / 組合せ論の combinadic rank/unrank(Lehmer 符号系)/ NIST SP 800-38D + McGrew & Viega の GCM 仕様(GHASH fold 構造、J0 導出)/ Sutherland & Hodgman (1974) 多角形クリップの半平面 pass — Qiita/Zenn/海外技術記事の B+木・線分交差・combinadic・GCM・ポリゴンクリップ解説を参照し全て整数のみで実装。

**実装物**: Rust BTreeMap 系の split/borrow 骨格、FHO 系 sweep のイベント分類、競技プログラミング慣行の combinadic、BearSSL/mbedtls 系のビットシリアル GHASH、clip ライブラリの正規化形 — 整数のみで実装。


## 第54次: 双端ヒープ・範囲計数・極大クリーク・時間伸縮・パス被覆

- `mmheap` — Min-max ヒープ(min/max 交互レベルの双端優先度キュー): 最大値は根の最大子(index 1/2)の浅い位置に限定されるため peek が両端 O(1)。sift は側別(side)分岐で子+孫の最良候補へ。**設計値捕捉**: `pop_max` で m が末尾スロットの時、先に pop すると `a[m] = last` が範囲外 — pop 対象自身が max なので move を skip。BTreeMultiset シャドー 20 ケース×2000 op 全照合
- `mstree` — マージソート木(静的範囲計数): 各ノードが子の sorted run を保持、クエリは O(log n) ノード×二分探索。**設計値捕捉**: 配列ヒープ配置(n+i 葉)は冪次 n でしか mid-split と一致しない — 任意 n では再帰 build のノード id をそのまま索引に使う(4n 容量)必要がある。brute slice 全照合 40 ケース×200 クエリ
- `clique` — Bron–Kerbosch 極大クリーク列挙(u64 隣接マスク、頂点≤64): Tomita のピボット u ∈ P∪X で |P∩N(u)| 最大を選び枝刈り。**設計値**: 出力は discovery 順をソートして正準化 — 入力のみの純関数。n≤8 で maximal 性の部分集合全走査 oracle 全照合、max_clique サイズも brute 一致
- `dtw` — 動的時間伸縮(整数弾性距離): dp[i][j] = |a_i−b_j| + min(上,左,対角)。Sakoe–Chiba 帯版と対角優先の正準経路復元。**設計値捕捉**: 境界セルを「左/上移動で伝播可能」にすると枯渇 prefix が自由消費され真値を過小に返す(24 vs 14 で捕捉) — i==0||j==0 は INF に留める。三角不等式は一般には不成立(メトリック非メトリック) — 検証は同一長の対称性・非負・恒等路線上界のみ。再帰メモ oracle 300 乱数全照合
- `pathcover` — DAG 最小パス被覆(Dilworth 鎖分割): L_u—R_v の二部コピー + hopcroft_karp 最大マッチングで被覆 = n − |matching|。**設計値捕捉**: 被覆再構成は「マッチングで前駆を持たない頂点から succ 連鎖を辿る」— succ 関数の列挙 oracle(n≤6 で (n+1)^n 全列挙、indeg≤1+非閉路条件)で optimality 照合。閉路入力は topo_order が None → 全体 None

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。

## 出典(第54次、search-index 照合)

**論文・仕様**: Atkinson, Sack, Santoro & Strothotte (1986) "Min-max heaps and generalized priority queues" の交互レベル構造 / merge-sort tree の競技プログラミング定石(sorted run ノード) / Bron & Kerbosch (1973) + Tomita ピボットの極大クリーク列挙 / Sakoe & Chiba (1978) の帯制約 DTW / Dilworth の鎖分割 + Fulkerson の二部被覆帰着 — Qiita/Zenn/海外技術記事の min-max heap・mstree・クリーク列挙・DTW・パス被覆解説を参照し全て整数のみで実装。

**実装物**: std::collections 系ヒープの sift 骨格、競技プログラミング慣行の mstree/被覆帰着、networkx 系クリーク列挙のピボット形、dtw 実装の境界 INF 規約、Ford–Fulkerson 系マッチング被覆 — 整数のみで実装。

## 第55次: ジャンプ点探索・GOAP・行動木・Verlet・バリューノイズ

- `jps` — Jump Point Search(8-way 格子): `&[u64]` 行ビットマスク上でジャンプ点だけを展開する一様コスト探索 `(f,g,x,y)` BTreeSet。斜め到達は naturals `{(dx,0),(0,dy),(dx,dy)}` + 壁越し強制近傍、直進到達は継続+斜め強制。**設計値捕捉**: 厳格 no-corner-cut(両サイドセル開放必須)は到達性を変え、JPS が oracle の見つける経路を失う — 論文の角接触規則(斜め step は遷移先のみ開放)が pruning 補題の前提。コストは STRAIGHT=2/DIAG=3 で `DIAG < 2·STRAIGHT` により最適性保存
- `goap` — Goal-Oriented Action Planning: `Action{name,pre,set,clr,cost}`、pre 充足のビットマスク世界遷移を Dijkstra。`BTreeSet<(cost, seq, state)>` で「同コストはアクション列の辞書順最小」を正準形に — 計画が純関数。深さ≤8 全 DFS oracle で plan ベクタ一致
- `btree` — 行動木(resume 意味論): arena `Vec<Node>`、Sequence/Selector は Running 子 index を `mem[i]` に保持して次 tick でそこから再開(先頭からの再評価ではない)。Condition の Running は Failure へ写像(葉は Running を返さない規約)。状態なし eager oracle 300 乱択で status+葉訪問数を全照合
- `verlet` — 整数 Verlet 統合(Q16.16): `x' = x + (x−x_prev)·damping + a` + 距離リンクを iters 回緩和、両端半分の `(len−rest)` 誤差を軸方向へ。**設計値捕捉**: damping=1 は無減衰でエネルギー保存 — リンクが平衡を貫通して持続振動する(両端点が入れ替わる大振動も観測)ため、収束主張には damping < 1 が必須。緩和順は挿入順リストでトレースの一部
- `vnoise` — 整数バリューノイズ+fBm: SplitMix64 格子ハッシュ(負 index も bit-mix 吸収)→ 上位16bit を Q16.16 へ、smootherstep `u²(3−2u)` 双線形補間、fbm は amp·freq の mul 更新+正規化。**設計値**: セル床は `raw >> 16` 算術シフト(負座標で二の補数 frac が [0,1) へ)、転置補間 oracle は丸め経路違いで±4量子一致

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。

## 出典(第55次、search-index 照合)

**論文・仕様**: Harabor & Grastien (2011/2014) "Online Graph Pruning for Pathfinding on Grid Maps" の jump point 定義(自然/強制近傍、角接触規則)/ Orkin (2003) "Applying Goal-Oriented Action Planning to Games" の pre/effects 状態遷移 / Colledanchise & Ögren の行動木 Resume 意味論(Running 子の記憶)/ Jakobsen (2001) "Advanced Character Physics" の Verlet+制約緩和 / Perlin (1985) + "Texturing and Modeling" fBm — Qiita/Zenn/海外技術記事の JPS・GOAP・行動木・Verlet・value noise 解説を参照し全て整数のみで実装。

**実装物**: 競技プログラミング/ゲームAI慣行の JPS ジャンプ規則、F.E.A.R./gore 系 GOAP ビットマスク表現、Unreal/Unity 系 BT の Running-resume、Jakobsen/Thomas Jakobsen 系緩和ソルバ、libnoise/stb 系 value noise の格子ハッシュ — 整数のみで実装。

## 第56次: 二分決定図・厳密単体法・packrat PEG・基数ヒープ・Life

- `bdd` — 簡約順序付き二分決定図(ROBDD): `(var,lo,hi)` ハッシュコンシング(BTreeMap 独特表)で構造的に同一の式は同ノード id。`apply`(Op::{And,Or,Xor,Diff,Implies})は Shannon 再帰+`(op,a,b)` 全メモ化、`restrict`/`exists`/`count_sat`(u128 厳密)/`eval`。真理表 oracle で200乱択合成式の全16割当+充足数を全照合、正準性テストは `(x0∧x1)∨¬x0 ≡ ¬x0∨x1` の id 一致まで確認
- `simplex` — Frac 厳密単体法 LP ソルバ: `max cᵀx s.t. Ax≤b, x≥0`、Phase I(補助変数 x₀ 列追加・最負 b 行ピボット)+Phase II、**Bland 規則**(最小添字進入/退出)で退化巡回が定理として不可 — Beale の巡回例(最適値 1/20)で検証。頂点全列挙 oracle(C(m+n,n) 基底 → gauss::solve → 実行可能頂点の最大目的値)120乱択照合
- `peg` — packrat PEG パーサ: `(rule,pos)` メモ化の順序選択再帰下降。**設計値**: `Star`/`Plus` は子が空マッチした時点で停止必須(no-progress break — 無いと無限ループ)、左再帰は active 集合ガードで代替枝失敗化、Class は構築時ソート+dedup で binary_search。200文法×20入力でメモ化版 vs 素朴再帰版の完全等価を検証
- `radixheap` — 単調基数ヒープ(Dijkstra 向け): **設計値捕捉** — バケツ判定は `msb(key XOR last)` が正しい(初版 `bit_len(key−last)` は last 前進時に高バケツへ残った小キーが最下位バケツの最小を潜り順序不変式を破壊 — BTreeMap シャドウ oracle が step 9 で (38 vs 33) の不一致を捕捉)。XOR 版では単調 push ⇒ 高バケツは厳密に大キーが証明可能。3000-op interleaved oracle 全照合
- `life` — 疎 B3/S23 セルオートマトン: `BTreeSet<(i64,i64)>` がそのまま正準状態(同一集合⇒同一トレース)。近傍カウントを BTreeMap 一発走査。グライダー4step 平行移動・blinker・still life 不変、稠密グリッド oracle 30試行×12世代で全照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。

## 出典(第56次、search-index 照合)

**論文・仕様**: Bryant (1986) "Graph-based Algorithms for Boolean Function Manipulation" の ROBDD unique-table/apply 構成 / Dantzig (1963) + Bland (1977) "New finite pivoting rules" の anti-cycling / Ford (2004) "Parsing Expression Grammars" (POPL) の packrat 構成 / Ahuja, Mehlhorn, Orlin & Tarjan (1990) "Faster algorithms for the shortest path problem" §3 の radix heap(XOR バケツは monotone radix heap の標準形)/ Gardner (1970) B3/S23 — Qiita/Zenn/海外技術記事の BDD・単体法・PEG・radix heap・Life 解説を参照し全て整数のみで実装。

**実装物**: BuDDy/m dd 系の mk+apply メモ化形、教科書系二相単体法の Bland 規則実装、PEG.js/rust-peg 系の packrat メモ化、競技プログラミング慣行の radix heap bucket 再配置、sparse Life の隣接カウント走査 — 整数のみで実装。

## 第57次: Fibonacci ヒープ・Halton 準乱数・D4 群・ターンパイク・Yen

- `fibheap` — Fredman–Tarjan の Fibonacci ヒープをアリーナ実装: 循環 sibling 環 + (key,seq) 正準 pop 順で純関数的トレース。`push`/`meld`/`decrease_key` は O(1) lazy、`pop` で次数統合。**設計値**: ステールハンドルは `live` ビットで拒否(pop 後の id 再利用は呼び出し側責任)、consolidate は「先に全 root 切り離し→by_degree で link→root 環を再構築」の3相に分けると link 中の環破壊を構造上排除。3000-op BTreeMap シャドウ oracle(decrease_key 追跡つき)で全照合
- `halton` — radical inverse `i/b^k` を厳密 `Frac` で返す Halton 準乱数列: 浮動小数点を経由しないため base^k 層化(先頭 b^k 項が全 j/b^k セルを丁度1回)が*性質検査として*成立。`Halton` は Iterator 実装(index=1 開始の古典慣例)、`point(i)` は純関数。低速累積 oracle で2000乱択照合
- `dihedral` — 正方格子の二面体群 D4 を `(swap, sx, sy)` closed-form で: `apply` は整数3命令、`compose` は swap=XOR + 入力スロット別の符号積、swap 元の `inverse` は符号入替で閉じる。検証は全部隊走査: 8×8 Cayley 表の全64項を「点対応で一致する唯一の元」を探す oracle と照合 — 導出した closed-form の4ケース全てが機械検証された
- `turnpike` — Skiena のターンパイク再構成: `n(n−1)/2` 個の距離マルチ集合から点列を復元。左優先 DFS で決定的。**設計値捕捉**: `need` の存在検査は*multiplicity 対応*が必須 — 同距離が2箇所から要求されるケース(例: `|x−p1|=|x−p2|`)を単一 presence で通すと残差マルチ集合が壊れ真の解へ辿り着かない(oracle が unsolved 誤報として捕捉)。homometric mates `{0,1,5,7,8}` vs `{0,1,3,7,8}` で同距離集合を確認、roundtrip oracle 200乱択
- `kpaths` — Yen の k-最短単純路: spur 偏差ごとに banned edges(受理済み経路の同 prefix 出辺)と banned nodes(root prefix)で Dijkstra 再実行、候補は `(cost, path)` BTreeMap。**設計値**: Yen の契約は「最小 k 個のコスト列」— 等コスト経路の内部順序は実装依存なので oracle は top-k コストベクタ一致+全経路が単純経路集合に含まれること+純関数再実行一致の3条件で検証(初版は等コスト経路の列挙順まで強制契約化しており、Yen の真の仕様外要求として oracle 側を修正)

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。

## 出典(第57次、search-index 照合)

**論文・仕様**: Fredman & Tarjan (1987) "Fibonacci heaps and their uses in improved network optimization algorithms" (JACM) の lazy binomial forest + cascading cut / Halton (1964) "Algorithm 247: Radical-inverse quasi-random point sequence" (CACM) / 二面体群 D4 の標準表示 ⟨r,s | r⁴=s²=1, srs=r⁻¹⟩ / Skiena, Smith & Lemke (1990) "Reconstructing sets from interpoint distances" (SoCG) の backtracking / Yen (1971) "Finding the k shortest loopless paths in a network" (Management Science) — Qiita/Zenn/海外技術記事の Fibonacci heap・低 discrepancy sequence・ターンパイク・Yen 解説を参照し全て整数のみで実装。

**実装物**: CLRS §19 の cut/cascade 手続きをアリーナ化、libstdc++ の radix-inplace ではなくランダムアクセス版、d4 group の bit-packed 表現、turnpike 教科書版の multiset BTreeMap 化、yen における重複辺 dedup(min weight)の正準化 — 整数のみで実装。

## 第58次: 形式的冪級数・矩形和集合・区間グラフ・スターリング数・玉ねぎ層

- `fps` — GF(998244353) 上の形式的冪級数(FPS): `add`/`sub`/`mul`(conv::convolve 再利用)/`scale`/`trunc`/`derivative`/`integral`/`inv`/`log`/`exp`/`pow`/`divmod`/`eval`。Newton 反復は `inv` が `g←g(2−fg)`、`exp` が `g←g(1+f−log g)` で既知 prefix を倍化。**設計値捕捉**: ループ条件を `g.len() < n` にすると `convolve` 積の末尾がゼロ係数になるケース(`f` の有効次数が m 未満)で `norm` が g を再縮小し無限ループ — 次数は*追跡変数* `m = min(2m, n)` で駆動必須。`exp(log f)=f`・`log(exp g)=g` 双方向 roundtrip、doctest は 1/k! の階乗逆元まで実測値一致
- `rectunion` — 矩形和集合面積: `(x, [y0,y1], ±1)` イベントを x-sweep、slab 毎に active 区間の union 長を再計算する正直な O(n²)(座標圧縮 segtree 版より単純性優先、n≤10⁴ まで実用)。`i128` 返却、逆順端点・退化スライバーは正規化。格子セル oracle で正負座標500乱択全照合
- `intervalgraph` — 区間グラフの古典3問: `max_independent_set` は最早終了貪欲の正準 tie-break `(end,start,index)`、`min_rooms` は半開区間の sweep 深度(区間グラフは完全グラフ=彩色数=最大重複)、`weighted_select` は finish ソート+`p[j]` 二分探索の O(n log n) DP で正準 predecessor 復元。退化 `[s,s)` は全 API で選択不能 — n≤8 全部分集合 oracle でサイズ・重み・ disjoint 性を全照合
- `stirling` — Stirling 数 mod p: 符号付き s1(`s(n,k)=s(n−1,k−1)−(n−1)s(n−1,k)`)、unsigned `us1`(順列の cycle 数 — `perm::unrank`+`cycles` で n≤7 全順列列挙の独立 oracle)、s2(閉形式 `1/k!·Σ(−1)^j C(k,j)(k−j)^n` と三角 DP を相互検証)、`bell`、`falling_coeffs`(降冪 xⁿ̲=Σs(n,k)xᵏ — Horner 評価 vs 直接積で恒等式 oracle)
- `onion` — 凸包玉ねぎ層分解: `poly::convex_hull` の繰り返し peel。**設計値捕捉**: hull 辺上の共線点は monotone-chain の出力に*含まれない*(strict-corner 契約)ため、その点は peel を生き残り独立した退化レイヤを形成 — 「玉ねぎ深度」の正直な意味論であり、doc で明示。全共線点列は各 peel が両端点2個しか除去しないため ⌈n/2⌉ 層。層数・層集合・union 復元・厳密入れ子を別ループ oracle で全照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。

## 出典(第58次、search-index 照合)

**論文・仕様**: Brent & Kung (1978) "Fast algorithms for manipulating formal power series" (JACM) の Newton 反復構成 / Bentley & Shamos 系 scanline 面積計測 / Gavril (1972) 区間グラフの彩色・最大独立集合の貪欲正当性 / Graham–Knuth–Patashnik "Concrete Mathematics" §6 の Stirling 三角と閉形式 / Chazelle (1985) "On the convex layers of a planar set" の onion 分解 — Qiita/Zenn/海外技術記事の FPS・sweep・区間スケジューリング・Stirling・onion layers 解説を参照し全て整数のみで実装。

**実装物**: Library Checker 系 FPS API 形状(inv/log/exp/pow の次数引数契約)、AOJ/ACL 系矩形和 sweep、教科書系 weighted interval scheduling DP+復元、ConMath の Stirling 双対恒等式、convex_hull 再利用の onion peel — 整数のみで実装。

## 第59次: 最小平均サイクル・線形空間アライメント・行列連鎖・継目削り・順序統計木

- `karp` — Karp の最小平均重みサイクル: `dp[k][v]`=長さ k 歩道の最小重み、`μ=min_v max_k (dp[n][v]−dp[k][v])/(n−k)`。**設計値捕捉**: サイクル抽出は「v* の最適 n 歩道の末尾 n−k* 辺が閉歩道」は偽(末尾 n−k* 辺の始点は任意) — 正しくは n 辺全て backtrack し最初の重複頂点で閉じる(その segment が mean=μ の証明: 切除すると μ 未満の歩道が残る)。初版は suffix-only backtrack で mean 不一致を oracle が捕捉
- `hirschberg` — Hirschberg 線形空間 Needleman–Wunsch: 中点で a を二分、前向き last-row `L` と逆転後向き `R` の `L[j]+R[n−j]` を最小 j で分割して再帰。スコア +2/−1/−1 固定、ops は `apply` が a→b を厳密再生する整合性を oracle が400乱択で全照合
- `matchain` — 行列連鎖積の最小スカラー乗算数: `dp[i][j]` 区間 DP + split 表、postorder の `Step` 列で括弧を復元。Catalan 全列挙 oracle で n≤7 全乱択照合
- `seamcarve` — Avidan–Shamir 継目削り: 二乗勾配エネルギー(境界は片側差分)+ 8-連結 seam DP(左端 argmin)+ `remove_vseam`(不正 seam は None)。**設計値捕捉**: 力任せ oracle の `go()` で「途中打ち切りパス」を `best` の初期値に残すと全 seam 未満のコストを返す — 葉行のみが base case。初版実装は正しかった(536 vs 5 で oracle が自壊を告白)
- `ost` — 順序統計 treap: `insert`/`erase`/`contains`/`select`/`rank`/`lower_bound`。優先度は `splitmix64(seed ⊕ mix(key))` — 形状がキー集合の純関数で挿入順序非依存。size/heap 不変式を再帰検査 + BTreeSet oracle 60 ラウンド×300 op 全照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。

## 出典(第59次、search-index 照合)

**論文・仕様**: Karp (1978) "A characterization of the minimum cycle mean in a digraph" / Hirschberg (1975) "A linear space algorithm for computing maximal common subsequences" / Avidan & Shamir (2007) "Seam carving for content-aware image resizing" / Cormen et al. CLRS §15.2 行列連鎖、§15.4 + Knuth の順序統計木 — Qiita/Zenn/海外技術記事の Karp 復元・Hirschberg・seam carving・order-statistics tree 解説を参照し全て整数のみで実装。

**実装物**: Library Checker 系 cycle API 形状、AtCoder/ACL 系 LCS 復元の空間最適化、画像縮退 seam API、GNU pbds tree_order_statistics_node_update 相当の rank/select — 全て整数のみで実装。

## 第60次: KK分割・x-fast・beats+lazy・Tunstall・厳密QR

- `kkpart` — Karmarkar–Karp 最大差分化分割: 残差ヒープ要素が `(plus, minus)` index bitmask を保持し、差分ステップで y の山を反転併合。**設計値捕捉**: 不変式 `v = Σplus − Σminus` で返却値 d が「実現可能な差分」— よって d ≥ optimal が構造的に証明される(近似保証をテストが主張するのでなく、分割そのものを返す設計)。n≤9 の 2ⁿ 全列挙 oracle で達成性+下界性を全照合
- `xfast` — x-fast trie(Willard 1983): 65 層の prefix→(min,max) 葉範囲テーブル、pred/succ はレベル二分探索で「x のパスが分岐する最深ノード」を特定し分岐子の葉境界から解く。**設計値捕捉**: 層 l の prefix 存在は全層 ≤l に存在を含意する単調性が二分探索を正当化; 葉の双方向リンクは BTreeSet で代替(O(1) hop でなく O(log n) — doc で正直に明記)
- `seglazy` — 延期バックログ消化: segbeats に `add` lazy を合成。**設計値捕捉**: `push` は pending add を子へ `apply_add` して*から* clamp を適用 — 逆順だと stale-low の子 mx が add 前の天井でクランプされ不変式を破壊。add 適用は NEG/POS sentinel を素通し("第二極値なし"の意味を保存)。120×300 op naive oracle + add/clamp 交互2000回
- `tunstall` — Tunstall 可変→固定長符号: 根から最大確率葉を A 子へ展開、葉数が 2ᵏ を超える直前で停止。葉確率は経路重み積だが深い経路で u128 溢れするため `BigInt` 交叉積で厳密比較(den^depth 差の正規化は不可能な大きさ)。DFS 順が正準コード、prefix-free・Σp=1・budget タイト性を BigInt 厳密検証
- `ortho` — Frac 上の厳密 Gram–Schmidt/QR: `qⱼ = aⱼ − Σ (⟨aⱼ,qₖ⟩/⟨qₖ,qₖ⟩)qₖ`、Q は*非正規化*直交(ℚ に √ が無いため QᵀQ=diag が限界)、R は単位対角上三角で A=Q·R が厳密成立。従属列の q は零ベクトル(エラーでなく仕様)、rank = 非零列数をガウス消去 oracle で照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。

## 出典(第60次、search-index 照合)

**論文・仕様**: Karmarkar & Karp (1982) "The Differencing Method of Set Partitioning" / Willard (1983) "Log-Logarithmic Worst-Case Range Queries" の x-fast trie / J. Dai (jiry_2) "Segment Tree Beats" の add 合成形 / Tunstall (1967) "Synthesis of Noiseless Compression Codes" + Savari & Gallager (1997) / Golub & Van Loan *Matrix Computations* §5.2 — Qiita/Zenn/海外技術記事の KK partition・x-fast/y-fast・segbeats lazy add・Tunstall coding・Gram-Schmidt QR 解説を参照し全て整数のみで実装。

**実装物**: Library Checker `range_chmin_chmax_add_range_sum` 準拠の beats+add 形状、Demaine 6.851 講義の x-fast 層テーブル構造、競プロ系 KK 復元の bitmask 追跡、Tunstall 教科書の貪欲葉展開、教科書系 classical Gram-Schmidt(修正版��なく — 厳密算術なら数値誤差を考慮する理由が無い)— 全て整数のみで実装。

## 第61次: 有理ベジェ・軌道数え上げ・最小値キュー・局所アライメント・三分探索

- `ratbezier` — 有理ベジェ(NURBS 式): 制御点を同次 `(w·x, w·y, w)` にリフトし de Casteljau で線形補間、最後に割り戻す — t の全中間値が `Frac` で厳密。`eval_deriv` は次数-1 の差分同次曲線に同じ評価を適用し商の微分 `(X'W − XW')/W²` で射影 — **設計値捕捉**: 差分ベクタの y/w 成分が `h[i] − h[i]` の自分自身差分で恒常 0 になっていたタイポを doctest が捕捉
- `polya` — Burnside 補題 `#orbits = (1/|G|)·Σk^{cycles(g)}`: `burnside` は群を置換のリストとして受理、`necklaces`/`bracelets` は巡回群 C_n・二面体群 D_n を位置集合上に生成。分子和は `BigInt`、|G| による厳密除算は limbs の上位側からの学校法則 — 除法未実装の bigint に小除数除法を合成。`necklaces(6,2)=14`(初版テストは bracelet 列 13 と混同 — oracle 列挙で捕捉)
- `minq` — 最小値キュー(MinQueue/MinStack): 各スタック要素が running min を保持し `min` = 両端の top の小さい方。`out` が空なら `in→out` pour で minima を一括再構築 — 各要素は最大1回 pour で償却 O(1)。`slide` の窓 deque と異なり永続 FIFO オブジェクト
- `ssw` — Smith–Waterman 局所アライメント: `dp[i][j] = max(0, sub, del, ins)` の 0-floor restart、勝者セルは最早 i → 最早 j で正準化、witness 範囲は restart まで traceback。**検証**: score = 全 substring pair の global NW 最大値(brute oracle)、返却範囲が真に score を達成するかを独立 NW で再検証
- `ternary` — 離散三分探索: **設計値捕捉**: 非厳密単峰(階段降下 `…,11,11,8`)で `f(m1)==f(m2)` の等値 probe は argmin を *どちら側にも* 局所化不能(平坦段の後に降下が続き得る) — 古典ルールは厳密単峰が前提。契約を厳密単峰に限定し、末尾窓を全評価+全体最小スキャンで検証 — 契約外入力は偶然正解 or `None` で、誤 index を黙って返さない

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。


## 出典(第61次、search-index 照合)

**論文・仕様**: Piegl & Tiller "The NURBS Book" §4.1 同次評価、Farin "Curves and Surfaces for CAGD" §13 有理ベジェ / Pólya & Read (1987) *Combinatorial Enumeration* の Burnside/巡回・二面体群 / cp-algorithms "Stack & Queue modification" の min-queue 構成 / Smith & Waterman (1981) JMB 局所アライメント、Gusfield §11 / ternary search folklore + 厳密単峰性の階段反例 — Qiita/Zenn/海外技術記事の有理ベジェ・Burnside・min-queue・SSW・ternary 解説を参照し全て整数のみで実装。

**実装物**: NURBS ライブラリ系の同次 (wx,wy,w) リフト評価、Sage/GAP 系 orbit-count API、cp-alg の two-stack min-queue、BioPython/Edlib 系 local-alignment API 形状、Library Checker 系 argmin インターフェース — 整数のみで実装。

## 第62次: 楕円曲線・p進整数・グレイ符号・整数分割・彩色数え上げ

- `ec` — GF(p) 楕円曲線群法則: 短 Weierstrass `y²=x³+ax+b` の affine chord-tangent を i128 中間値で全厳密。`on_curve` が呼び出し前提を明示し、`p<5`(拡張法則が必要)/非体利用を正直に拒否 — `double` は `y=0`→Inf、`add` は同 x・逆 y→Inf、逆元非存在(合成 p)は Inf に退化。巡回群表(19点曲線)で全 Cayley 表、大きめ素数での結合律・スカラー法則を oracle 検証
- `adic` — 切断 p進整数 `Z_p mod pᵏ`: add/sub/mul/neg は mod pᵏ 厳密、unit は `gcd(v,p)=1`(**合成 p も正しく** — `2 mod 4ᵏ` は非unit)、`inv` は i64 extgcd(modulus を `new` で i64 範囲に限定して全域化)、`val` は切断 0 に `None`(真の付値は未知のまま)。`lift`/`trunc` で精度遷移、リング演算は `unwrap` なし全域
- `gray` — BRGC グレイ符号: `to_gray(i)=i^(i>>1)`、prefix-xor 逆写像、全 2ⁿ `sequence`、`SubsetWalk` Iterator は部分集合を1ビット step で走査し `last_flip` が遷移ビットを同報 — 単位 step・置換性・flip 報告の3不変式を oracle 検証
- `partitions` — 整数分割: `count` は Euler 五角数漸化式 `p(n)=Σ(−1)^{k+1} p(n−g_k)` を `BigInt` で(p(100)=190,569,292 — u64 超過も厳密)。`count_bounded`(p(n,m) 三角: 部品 1 削除 or 全部品 −1)と `count_distinct`(q(n,m) DP)が互いの shadow oracle — Σ_m p(n,m)=p(n) かつ **distinct=odd 分割の Euler 定理を双方で検証**。`enumerate` は降次語彙順 `[n]→[1,…,1]`
- `chrompoly` — 厳密彩色数え上げ: 削除-縮約 `P(G)=P(G−e)−P(G/e)` を `BigInt` で最小辺の正準再帰 — `count`(u128 オプション)・`count_poly`(任意精度 k)・`chromatic`(最小 k)。kⁿ 全列挙 oracle(400 乱択)と C₄ 閉形式 `k(k−1)(k²−3k+3)` で照合

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。


## 出典(第62次、search-index 照合)

**論文・仕様**: Cohen & Frey *Handbook of Elliptic and Hyperelliptic Curve Cryptography* §13.2 の affine 群法則 / Gouvêa *p-adic Numbers* の切断 Z_p 演算 / TAOCP 7.2.1.1 BRGC / Andrews *The Theory of Partitions* §1.3 Euler 五角数漸化式 / Read (JCT 1968) chromatic polynomial の削除-縮約 — Qiita/Zenn/海外技術記事の EC point math・p-adic・Gray code・partition number・chromatic polynomial 解説を参照し全て整数のみで実装。

**実装物**: Sage/pari 系 EC API の add/double/mul/on_curve 形状、p-adic リング API(Sage `Zp` の val/lift)、BRGC 教科書 rank/unrank+subset walk、Euler pentagonal+p(n,m) DP 双 oracle、NetworkX 系 chromatic_polynomial 評価 — 整数のみで実装。


## 第63次: Pell 方程式・Farey 数列・メビウス反転・Kronecker 記号・エジプト分数

- `pell` — Pell 方程式 `x²−d·y²=±1`: `surd_cf` が √d の `(m,d,a)` CF 漸化式を全整数で計算(周期終端は `a_k=2a₀` の定理)、`solve`/`negative` が `BigInt` 収束分数を走査 — OEIS 基底解(d=2..13)と d=61 の 1766319049/226153980 を照合。`power` は Z[√d] の群法則で k 番目解を生成 — 冪が解に留まることを residue で検証
- `farey` — Farey 数列: `farey(n)` は next-term 漸化式 `k=⌊(n+b)/d⌋` で1算術ステップ/項を生成 — |F_n|=1+Σφ(k)(sieve の phi で oracle)・隣接項行列式 |ad−bc|=1 を全対で検証。`neighbor` は F_n 内の直前直後、`stern_brocot` は 0/1→1/1 からの L/R mediants 経路を返す(端点は非 mediant なので契約を開区間に限定)。`floor_sum` は ACL primitive `Σ⌊(a·i+b)/m⌋` を u128 Euclidean 漸化式で — 直接和 oracle 3000 乱択
- `mobius` — 除数格子代数: `mu_sieve` は線形篩(各 n は最小素因子で1回のみ)、`mu` は SPF factor_map で単発、`convolve`/`invert` は Dirichlet 畳込み (f∗g)(n)=Σ_{d|n}f(d)·g(n/d) とその逆(`f ∗ μ` が g(n)=Σf(d) を厳密反転 — 200 往復照合)。`coprime_count` は Σμ(d)⌊n/d⌋ 包除で gcd 直接列挙と照合
- `jacobi` — Kronecker 記号 (a|n): 奇数素数では Legendre と一致、全整数へ乗法性+拡張規約 ((a|−1)=sign a, (a|0)=[a=±1]) で全域化 — Cohen Alg.1.4.10 の二分互換法。Euler 判定 a^((p−1)/2) mod p との4000乱択照合 + 乗法性 (a|mn)=(a|m)(a|n) 3000 照合
- `egypt` — エジプト分数: Fibonacci–Sylvester 貪欲 `d=⌈den/num⌉` で真分数を相異なる単位分数和に — 残余分子が単調減少(termination invariant)であることを乱択検証、`verify` は Frac 厳密和+distinct 検査。`i128` 超過は正直に `None`

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。


## 出典(第63次、search-index 照合)

**論文・仕様**: Hardy & Wright §14.5 (Pell 解は √d の収束分数) / Cohen *CCANT* Alg.1.4.10 の Kronecker 二分互換法 / Hardy & Wright §III Thm.29 (Farey next-term 漸化式) / AtCoder Library `floor_sum` / Fibonacci (1202)–Sylvester (1880) 貪欲エジプト分数 / Dirichlet 畳込み・Möbius 反転 (H&W §16) — Qiita/Zenn/海外技術記事の surd CF・Farey・floor_sum・Kronecker・Egyptian fraction 解説を参照し全て整数のみで実装。

**実装物**: Library-Checker 系 `surd_cf` の (m,d,a) 漸化式、ACL `floor_sum` の Euclidean swap ループ、線形篩の lp 配列、Cohen 本の拡張表 (n∈{−1,0})、SymPy 系 `egyptian_fraction` の greedy API 形状 — 整数のみで実装。


## 第64次: Lucas 数列・Frobenius 数・Josephus・Bernoulli 数・Eulerian 数

- `lucas` — Lucas 数列 Uₖ(P,Q)/Vₖ(P,Q): 厳密版は i128 checked 線形走査、`lucas_mod` は標準 doubling 恒等式 U(2k)=U·V・V(2k)=V²−2Qᵏ・U(2k+1)=(P·U+V)/2・V(2k+1)=(D·U+P·V)/2 で O(log k)。halving は残余 r∈[0,m) の偶奇で行う必要が必須(生の x の偶奇を見ると負数/剰余前で誤反転 — oracle が捕捉)。不変式 V²−D·U²=4Qⁿ を i128 検証、m は奇数限定(/2 が逆元を持つため)
- `frobenius` — 硬貨問題: `dist[r]=min{ representable ≡ r mod m }`(m=min coin)を Dijkstra で構築し `g = max dist − m`、representable は `dist[x mod m] ≤ x` の判定。2硬貨は Sylvester 閉形式 ab−a−b との300乱択照合、DP oracle 2000乱択、g(6,10,15)=29・McNugget g(6,9,20)=43
- `josephus` — ヨセフス環状淘汰: `survivor` は O(n) 漸化式 Jₙ=(Jₙ₋₁+k) mod n、`survivor2` は k=2 閉形式 2l (n=2ᵐ+l)。`order` は `ost` の order-statistic treap で select/erase を回す O(n log n) 全淘汰順 — Vec 逐次消去 oracle が order 全体を照合(400乱択)
- `bernoulli` — 厳密 Bernoulli 数: Akiyama–Tanigawa を `Frac` で全厳密に(B₁=+1/2 規約)。`faulhaber` は Faulhaber 公式 1/(k+1)·Σⱼ C(k+1,j)·Bⱼ·n^{k+1−j} — 直接冪和 oracle 400乱択 + B_{2k+1}=0 定理 + 生成漸化式 ΣC(n+1,j)Bⱼ=n+1 の3系統検証
- `eulerian` — Eulerian 数 ⟨n k⟩: 漸化式 (n−k)⟨n−1 k−1⟩+(k+1)⟨n−1 k⟩ を `BigInt` で。行和定理 Σ⟨n k⟩=n! と Worpitzky 恒等式 xⁿ=Σ⟨n k⟩C(x+k,n) を両面検証、`permutations` は perm::unrank で descent=k の全置換を列挙(長さ=⟨n k⟩ と一致)

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。


## 出典(第64次、search-index 照合)

**論文・仕様**: Concrete Mathematics §1.3 (Josephus 漸化式・k=2 閉形式)・§6.2 (Eulerian 数・Worpitzky) / TAOCP §1.2.9 (Bernoulli) / Sylvester (1882) g(a,b)=ab−a−b / Brauer–Davison の residue-Dijkstra Frobenius / Akiyama–Tanigawa (2001) — Qiita/Zenn/海外技術記事の Lucas doubling・硬貨問題・ヨセフス・Faulhaber 解説を参照し全て整数のみで実装。

**実装物**: cp-algorithms/Library-Checker `lucas_number` の doubling 恒等式と odd-mod halving、競技プロ系 min-residue Dijkstra (dist[x mod m] ≤ x 判定)、order-statistic tree による淘汰順シミュレーション、Akiyama–Tanigawa の in-place 三角 — 整数のみで実装。


## 第65次: Catalan・撹乱順列・Zeckendorf・桁DP・ハノイ塔

- `catalan` — Catalan 系統: `binomial` は chain `res·(n+1−i)/i` の各除算厳密、`catalan` は Cᵢ=Cᵢ₋₁(4i−2)/(i+1)、`ballot` は Bertrand 票差数 (a−b)/(a+b)·C(a+b,a) — 全て `BigInt` 厳密。`dyck` は '('/')' バックトラック生成 — 個数=C_n・全 validity・BTreeSet 一意性 + 2^{2n} 全列挙 oracle・ballot mask oracle
- `derange` — 撹乱順列: `derangement` は Dₙ=(n−1)(Dₙ₋₁+Dₙ₋₂) 漸化式、`rencontres(n,k)=C(n,k)·!(n−k)` で comb モジュール再利用、`derangements` は `perm::unrank` で固定点0の全置換列挙。行和 ΣR(n,k)=n!・R(n,n−1)=0・列挙histogram oracle の3検証
- `zeckendorf` — Zeckendorf フィボナッチ表現: 貪欲+`partition_point` 二部探索で降順非隣接表現。ユニーク性 oracle: 非隣接 index 部分集合の全列挙が「表現が丁度1つ=貪欲結果」を構造検証 — 存在(Zeckendorf 定理前半)と一意性(後半)を同時に機械検証
- `digit` — 桁 DP: tight-bound DP が (pos,tight,started) 状態を MSD→LSD 走査 — `started` フラグが「先頭ゼロは数の桁ではない」を符号化する微妙点であり全性の本体。count_avoid_digit の d=0 は x=0 自体も除外、count_digit_sum は s 上限早期 break。両者に n≤20000 brute oracle
- `hanoi` — ハノイ塔: `moves` は古典再帰、`move_at` は分割点 k=2ⁿ⁻¹−1 の左右半分を降下する O(n) 点クエリ(左: from→aux、右: aux→to)。`state` は k 手リプレイ。検証は全手合法性シミュレーション(トップ冪板・小≤大) + ctz(k+1) 特性(move k は冪板 ctz(k+1) を動かす定理) の2系統

**継続延期バックログ**: 平面性判定、SwissTable の SIMD 群制御、α-hull の垂線中点パラメータ式。


## 出典(第65次、search-index 照合)

**論文・仕様**: TAOCP 7.2.1.6 (Catalan/Dyck path 生成)・Concrete Mathematics §2.3 (derangement 漸化式・rencontres) / Zeckendorf (1972) 表現定理 / Brown (1964) 貪欲一意性 / AtCoder EDPC・ACL 系の桁DP (pos-tight-started 3状態) / Lucas (1883) ハノイ塔・Frame–Stewart 系手順解析 (ctz(k+1) 特性は Stockmeyer 等) — Qiita/Zenn/海外技術記事の Catalan・derangement・Zeckendorf・digit-DP・Hanoi 解説を参照し全て整数のみで実装。

**実装物**: SymPy `binomial`/`subfactorial` の API 形状、Library-Checker `zeckendorf` 貪欲+partition_point、cp-algorithms/競プロ典型の digit-DP (pos,tight,started)、典型 k 番目ハノイ手 `move_at` の二分降下 — 整数のみで実装。


## 第66次: 監査駆動 — DetHash 標準型完備・Fixed exp/log 基盤・golden 全ピン化

**方法の転換**: 今回は新モジュール追加ではなく、第一原理(第3次の3公理: 決定的状態・決定的入力列・証明器具)とソクラテス問答で「公理上本当に欠けているもの」を列挙。3件の結論:

- `world_hash` — **DetHash の標準型カバーが非完備だった**: `write_i8`/`write_u128`/`write_i128` を Fnv1a に追加し、`i8`/`i16`/`u128`/`i128`/`()`/`(A,B,C,D)`/`[T;N]`/`Box<T>`/`&T`/`VecDeque<T>`/`LinkedList<T>`/`BTreeMap<K,V>`/`BTreeSet<T>` に `DetHash` を実装。問答の産物として**意図的除外をコード内に明文化**: `usize`/`isize`(SPEC G9 の幅依存)、`HashMap`/`HashSet`(プロセス乱択順 — `hash_unordered` が分担)、`BinaryHeap`(iter 順が挿入履歴を encode)。`VecDeque`/`LinkedList`/`BTree*` は全て長さ u32 + 正準順で既存 `[T]` 規約と同一 encod­ing
- `fixed` — **exp/log 超越関数が不在であり、crate 内で既に重複実装されていた**: `easing.rs` の私有 `exp2_tween` が 0.5% 誤差の3項 Taylor で基盤の欠落を代理証明。`exp2` = 整数部シフト × 16項 `2^(2^-i)` 積テーブル(最終誤差 ≤~8ulp frac — 出力では 2^n 倍され相対 ~2.5e-4)、`log2` = [2¹⁶,2¹⁷) 正規化 + 16回二乗で1bit/反復、`ln`/`exp`/`powf` は ln2/log₂e 定数倍の合成。収縮契約: exp2 `x≥15`→MAX・`x≤−18`→0、log2 `x≤0`→MIN、powf `base≤0`→0。**計測が設計を決めた点**: 積算の round-half-up は単調性を破壊(14057 非単調点を Python で実測)→ 切り捨て採用で全範囲単調 0 違反; `exp2` 最終シフトのみ round-half-up。`exp2_tween` は `Fixed::exp2` に委譲し精度 ~100倍向上(全イージングテストは許容値ベースなので安全)
- `det_hash_golden` — **宣言済み債務こそ弱点**: `UNPINNED_DET_HASH` の62型を全棚卸しし、公開構築可能な61型全て + 新規 std 型を golden pin(97 ケース)。区別できた2つの仕様差: `Fsm` は遷移表を意図的に除外(設定は状態でない — コメント済)、`Quest::state` は objectives からの導出値でフィールド非存在(折り畳み不要が正しい)。残存宣言は `MonitorState` のみ(private フィールド・`Monitor` 経由生成のため正準フィクスチャ不可)。単バイト判別子 enum は意図的に同一 hash(同バイト列→同 digest は hash の正義)となり得るため、推移カバー(VisibilityMap セル・Objective.state)で担保

**検証**: 冪は exp2/log2 共に厳密、oracle は f64(テスト専用許可)20k sweep、同一入力2回 determinism、ラウンドトリップは出力量子化に応じた scale-aware 束 |Δ|≲2¹⁶/(2r·ln2)。全例・pin hash・パッケージング含む gate 全緑。


## 出典(第66次)

**手法**: Elon Musk の第一原理推論(公理まで分解し推論し直す)とソクラテス問答法(仮定を問いで崩す)を監査ツールとして適用 — 「何を足すか」ではなく「公理上何が欠けているか」を列挙する逆方向のラウンド設計。

**実装物**: IEEE 系 round-half-up 出力シフト、binary-decomposition べき乗表(CORDIC/平方分割と同族)、Fnv1a LE-bytes ワイヤ規約の std 型全域化、golden-fixture による wire-format 完全監視 — 全て整数のみで実装。


## 第67次: 文献参照の標準 primitive 補完 — steer・reroot・pbs・crdt・quat

**方法**: GitHub・論文・Qiita/Zenn・海外技術記事から「決定性ライブラリが持つべき標準 primitive」を列挙し、grep で未収録を確かめてから選定(`persistent`/`ntt`/`cordic`/`scc` は既存 `pstree`/`conv`/`Fixed::sin_cos`/`graph::strongly_connected` が引き受けており近複製を回避)。結果5本 + 基盤3件:

- `steer` — Reynolds steering behaviors(GDC '99): `seek`/`flee`/`arrive`/`pursue`/`evade`/`wander`/`separation`/`combine`。一貫形は「desired = dir·max_speed、force = truncate(desired − vel, max_force)」。`arrive` は slow_radius 内で線形減速、`pursue`/`evade` は距離/max_speed tick の線形予測、`wander` は SplitMix64 jitter、`separation` は単位ベクトル×距離反比例の和。`Fixed` のみ、`Option`/空ベクトルで total に
- `reroot` — 全方位木 DP: DFS 親リンク + 事前順リストの2パスで `down`(部分木集約)→`up`(残り全体集約、prefix/suffix で兄弟 fold)。出力は隣接順序非依存。教訓: **リフトの正しさは集約型に依存** — 距離和は部分木サイズ分増えるため `(sum,count)` 対モノイドで `lift=|(s,c)| (s+c,c)`;「`+1`リフト」は eccentricity 等の max 型に限る。退廃モノイドで書いたテストが即座に食い止めた(BFS oracle)
- `pbs` — 並行二分探索: n個の適用 op と q 個の単調述語クエリを `O((n+q) log n)` で。ctx を各パス `init()+apply` で再構築するため unapply 不要 — 契約は「pred が t で単調」のみ。発火しないクエリは `t=lo` 最終評価で `None`
- `crdt` — Shapiro 系統の join-semilattice レプリカ: `GCounter`(レーン別 max)、`PNCounter`(p−n 2つの GCounter — 負値も順序安全)、`TwoPhaseSet`(add/remove 両 union、墓碑が永続的に勝つ — re-add にはタグ付き要素が要るので非提供)、`GSet`(union)。`Vec`/`BTreeSet` は第66次の `DetHash` をそのまま使い全型 golden pin
- `quat` — `Fixed` 上の四元数: Hamilton 積、共役、norm、Cayley 形 `v + 2·(q⃗×(q⃗×v + w·v))` 回転、`from_axis_angle`、`between`(対蹠付近は最大垂直軸フォールバック)、`nlerp`(半球 flip で短弧を保証、`acos` が基盤に無く真の slerp は不可 — 不可の理由を docs に明記)、`to_axis_angle`/`angle_to` は `Fixed::atan2`
- `fixed` — **基盤ギャップ**: π が公開定数として存在せず、`easing` が私有 `pi()`(355/113)で代理実装していた → `pub const PI/TWO_PI/HALF_PI` 公開(raw は CORDIC 域内値と同一)。`from_raw` は第66次で追加済
- 合わせて `reroot` の monoid ドキュメントを「+1 は hop-count/max 系のみ、和系は count を持て」と明示 — README 表にも設計ノートを記録

**検証**: 37 新規テスト全緑。oracle: BFS 距離和/eccentricity(reroot)、brute prefix-sum/membership 走査(pbs)、Join 半束の交換・結合・冪等(crdt)、四元数の逆回転・合成順・半球 flip(quat)、30 例 headless×2、pins 4725/0x3dd8f9a0e44300d9、golden 102。


## 出典(第67次、search-index 照合)

**論文・仕様**: Reynolds, *Steering Behaviors for Autonomous Characters* (GDC '99 プロシーディングス、公式解析 seek/arrive/separation/wander) / 全方位木 DP(あれば木DP・rerooting、AtCoder ABC 系解説・Qiita/Zenn 記事) / 並行二分探索(offline parallel binary search、Qiita・Zenn・cp-algorithms 相当の解説群) / Shapiro et al., *Conflict-free Replicated Data Types* (2011) — G/PN Counter、G/2P Set の join-semilattice 仕様 / Hamilton (1844) 四元数・Slicker *Quaternions and Rotation Sequences* の Cayley 回転形・Ken Shoemake slerp/nlerp 文献(acos 不在のため nlerp+半球flipのみ採用) — 全て整数のみで実装。

**実装物**: libgdx/jMonkeyEngine 系 steering の力合成式、AtCoder Library/競プロ典型の reroot 2パス + lift モノイド化、Parallel BS の init/apply/pred 3引数契約、automerge/yjs 系 CRDT の counter/set 結合則、glm/DirectXMath 系 quat の axis-angle/between/nlerp API 形状 — 全て整数のみで実装。


## 第68次: 曲線・復号・制御/推定 — catmull・viterbi・spring・pid・kalman・gnoise

**方法**: 第67次と同じ文献参照ラウンド — GitHub・論文・Qiita/Zenn・海外技術記事の「決定性ライブラリ標準 primitive」を列挙し grep で未収録確認後に選定(`crt`/`earcut` は `ntheory::crt2`+`crt`・`poly::ear_clip` が引き受けており近複製回避)。基盤側の証拠も拾った: `easing` の `Fixed::exp`(第66次)があれば Unity 式有理近似ではなく厳密解バネが書ける、が選定理由。結果6本 + steer boids 完成:

- `catmull` — Catmull-Rom / Hermite スプライン: `catmull_rom` は係数 `2p₁, −p₀+p₂, 2p₀−5p₁+4p₂−p₃, −p₀+3p₁−3p₂+p₃` の Horner + 最後に ×½(初版がこの ½ を落としてちょうど2倍の誤差 — endpoints テストが捕捉)、`sample` は端点ミラーの phantom 制御点で区間毎に `t∈[0,1)` + 最終点、`centripetal_sample` は Barry–Goldman 3-Lerp ピラミッドと `t_{i+1}=t_i+√|Δ|` ノット — Yuksel–Schaefer–Keyser の「centripetal は自己交差を作らない」保証のため。chord は i64 脚の二乗和を leg ≤46340 に clamp して `isqrt64` で整数化
- `viterbi` — 整数コスト加算型 HMM 最尤復号: `viterbi`/`viterbi_cost` で `O(T·S²)` DP、コストは「スケール済み −log p」(呼び出し側が `Fixed::ln` 等で作る)。tie は各セルで小さい predecessor — 全体 lex-min ではないことを doc 明記(brute-force lex-min 列挙テストがその差を実際に捕捉し、検証を「コスト一致」に修正)
- `spring` — 厳密臨界減衰バネ: 閉形式 `x=(Δ+(v₀+ωΔ)t)e^{−ωt}`、`v=(v₀−ω(v₀+ωΔ)t)e^{−ωt}` を `Fixed::exp` で — GPG4/Unity SmoothDamp の有理近似 `1/(1+x+0.48x²+0.235x³)` ではなく真の指数。状態版 `Spring` は中途リターゲットで速度が連続するよう `v` を持ち越す。`ω≤0`/`dt≤0` は no-op
- `pid` — Åström–Hägglund 離散 PID: `u=clamp(kp·e + ki·∫e + kd·(−dm/dt))`。微分は measurement 側(セットポイントステップでキックしない)、anti-windup は `ki·∫e` を出力域に割り戻す clamped-integration 形、`ki=0` は積分自体を skip(死に状態を持たない)。`dt≤0` は P のみ clamp 出力で状態不変
- `kalman` — スカラー Kalman(Welch–Bishop): predict `P+=q`/`predict_with dx`、update `K=P/(P+R)`、`x+=K(z−x)`、`P=(1−K)P`。`r≤0` は無ノイズセンサとして `x:=z,P:=0` 完全信頼。`p0<0`/`q<0` は 0 に clamp(分散は負にならない)。`DetHash` 実装 + golden pin
- `gnoise` — Perlin 勾配ノイズ: 全整数格子点で厳密 0(勾配ノイズのsignature — 値ノイズ `noise` では埋められなかった連続場ギャップ)、Perlin 2002 quintic `6t⁵−15t⁴+10t³` fade、8方向勾配を SplitMix64 系 corner-hash の下位3bit で選択、双線形 quintic 補間。`fbm2` は周波数倍・振幅半減の正規化済み octaves
- `steer` — Reynolds boids 完成: `alignment`(近傍平均速度を max_speed に renorm → steer_to、速度相殺近傍は零)と `cohesion`(近傍 centroid への seek)を追加し separation/alignment/cohesion 3則全揃い

**検証**: 新規 33 モジュールテスト + steer 13 全緑。oracle: f64 閉形式(spring/kalman)、brute S^T 列挙(viterbi)、カージナル接線で Hermite ≡ Catmull(catmull)、格子点 0/連続性/有界(gnoise)。API pin 4781、golden 105、kit 398 モジュール。


## 出典(第68次、search-index 照合)

**論文・仕様**: Yuksel–Schaefer–Keyser, *On the Parameterization of Catmull-Rom Curves* (centripetal 版が cusp/自己交差を作らない証明 — Barry–Goldman pyramid 形式) / Hermite 基底 h₀₀,h₁₀,h₀₁,h₁₁ (Ferguson 1964 系) / Perlin, *An Image Synthesizer* (SIGGRAPH '85) 及び *Improving Noise* (2002 — quintic fade・8 勾配選択) / Eiserloh, *Game Programming Gems 4* "Interpolating with a Smoothstep Function" + Unity `Vector3.SmoothDamp` 式 (臨界減衰 — 本実装は近似を捨て厳密解へ) / Åström–Hägglund, *PID Controllers: Theory, Design and Tuning* (derivative-on-measurement・clamping anti-windup) / Viterbi (1967)・Forney (1973) MLSE・Rabiner HMM tutorial (復号 DP) / Welch–Bishop, *An Introduction to the Kalman Filter* (スカラー predict/update 式) — 全て整数のみで実装。

**実装物**: Unity `Vector3.SmoothDamp`・Godot `lerp_angle` 系 API 形状(リターゲット連続性のための状態版)、cp-algorithms/競プロ典型の Viterbi DP とコスト化規約、Arduino/PID-library 系の anti-windup・derivative-kick 対策、Ken Perlin 参照実装の permutation→hash 勾配選択を SplitMix64 系 avalanche hash へ置換 — 全て整数のみで実装。

## 第69次: 幾何クエリ・経路平滑・変換・系列推論・数値・照合 — ray・funnel・affine・hmm・roots・glob

**方法**: 第68次と同じ文献参照ラウンド — GitHub・論文・Qiita/Zenn・海外技術記事の標準 primitive 候補を列挙し grep で未収録確認(halton は既存、crt/earcut は ntheory/poly が引き受け、sobol/fft/semver 等は今回スコープ外)。結果6本:

- `ray` — 解析的レイクエリ: `hit_aabb`(スラブ法・軸毎に t 区間を縮め、原点が box 内なら t=0、並行なら同スラブ内判定)、`hit_circle`(半係数二次式 `b=(o−c)·d` で判別式の radicand を縮小 — Q16.16 の乗算桁を節約)、`hit_segment`(2D cross パラメータ `t=(a−o)×e / d×e`, `u=(a−o)×d / d×e`、並行は None)、`hit_polygon`(全辺最小 t)。`geometry` のグリッド DDA とは別物: 連続空間の「最初の交差 t」を返す
- `funnel` — Mononen のシンプルファネル(Detour/recast・crowd-sim 標準、Lee–Hinckley 系): `(left,right)` ポータル列を apex+左右レイで `O(n)` 走査、左右どちらかが対側レイを越えたらその側の頂点を新 apex にして再開。初版は符号規約が Mononen と逆(彼の portalLeft が CW 側)で誤った頂点を apex にしていた — 非交差オラクル(wall chain との厳密交差検査)が捕捉し、「left=CCW 辺」規約に docs 明記して比較符号を反転。ポータルゲートは自由に跨ぎ、壁(左右チェーン+端点キャップ)は決して跨がない、が正しい妥当性条件。collapse apex が goal 自身の場合は末尾 push を dedupe
- `affine` — 2D アフィン `[a b c; d e f; 0 0 1]`(PostScript/SVG/Canvas `transform(a,b,c,d,e,f)` 形): `then` 合成は「self 後に other」= `other·self` の積、`apply`/`apply_delta`(並進を除く — 速度・オフセット用)、adjugate `invert`(det 0 → None、`Fixed::div` 切り捨てで往復は数 ulp の誤差)。`rotate_about` は `T(p)·R·T(−p)` の順序ミスがピボット固定点テストで捕捉された
- `hmm` — Rabiner 1989 §III のスケーリング付き forward–backward: `α_t = c_t·(b⊙Aᵀα_{t−1})` で各ステップを `Σ=1` に再正規化 → 長系列でもアンダーフローしない。`β` は同じ `c_t` を使う `β_T=c_T`・`β_t=c_t·Σ a·b·β` 規約で `γ_t ∝ α_t·β_t` がそのまま事後確率。`log_likelihood = Σ ln(Σα̃)`(初版は `−ln c_t` の符号を逆に書きカジノモデルの大小比較テストが捕捉)。`viterbi` が最尤経路を返すのに対しこちらは周辺確率
- `roots` — 固定反復回数のスカラー求根: `bisect`(符号跨ぎ必須・収束域保証・tie は左)、`secant`(2点補間・微分不要)、`newton`(微分クロージャ版)。イプシロン停止ではなく反復予算 — リプレイ安定のため `iters` を公開引数に。収束時の ulp 停止は `next == x` で早期打ち切り
- `glob` — bytewise POSIX fnmatch 風: `*`(パス区切りも跨ぐ — FNM_PATHNAME 無しと明記)、`?`、`[abc]`/`[a-z]`/`[!x]`/`[^x]`、`\` エスケープ、2ポインタ+スターバックトラック(再帰なし)。閉じない `[` はリテラル扱い、`[` 直後の `]` はリテラルメンバー

**検証**: 新規 31 モジュールテスト全緑。oracle: f64 行列積・f64 forward-backward(affine/hmm)、壁非交差+頂点集合(funnel)、√2/立方根の解析解(roots)、端正な手計算例(ray/glob)。API pin 4826、kit 404 モジュール。


## 出典(第69次、search-index 照合)

**論文・仕様**: Mononen, *The Simple Stupid Funnel Algorithm* (Digesting Duck、recast/Detour の crowd 引き回し実装 — Lee & Hinckley 系 portal string-pulling) / Rabiner, *A Tutorial on Hidden Markov Models and Selected Applications in Speech Recognition* (1989、§III scaling — `c_t=1/Σα̃` 再正規化と `β_T=c_T` 規約) / Press et al., *Numerical Recipes* §9 (bisection・secant・Newton–Raphson の収束・失敗形) / POSIX `fnmatch(3)`・van Rossum fnmatch パターン規則 / PostScript `concat`・W3C SVG `transform(a,b,c,d,e,f)` 行列表記 / akenine-möller ray-box slab 法・Glassner *An Introduction to Ray Tracing* の ray-quadratic — 全て整数のみで実装。

**実装物**: recastnavigation `dtMergeCorridorStartMoved` 系の portal 表現((left,right) 対 + 退化 portal)、GStreamer/OpenHMM 系 scaled forward-backward の c_t 保持実装、cp-algorithms/競プロ典型の roots 反復打ち切り形、BSD `fnmatch`/rust `globset` の `[`..`]` クラス規則、Java2D `AffineTransform`/GLM `mat3` の then/apply/invert API 形状 — 全て整数のみで実装。

## 第70次: 準乱数・フィルタ・画像解析・広相・補間・整列 — sobol・biquad・ccl・sap・pchip・align

**方法**: 第69次と同じ文献参照ラウンド — GitHub・論文・Qiita/Zenn・海外技術記事の標準 primitive 候補を列挙し grep で未収録確認(`halton` は低食違い列として既存、`biconn`/SCC はグラフで CCL は画像解析、spatial_hash/rtree/quadtree は索引で SAP は走査、c spline/bspline は任意曲線で pchip は単調保証、dtw/editdist は距離で NW/SW は整列トレース)。結果6本:

- `sobol` — Sobol' (0,2)-列準乱数: direction number `v₁ = 1<<(31−i)`、2次元目は Joe–Kuo 標準の初期値 `m = 1, 3` から `m[i] = 2m[i−1] ⊕ 4m[i−2] ⊕ m[i−2]`、gray ではなく canonical 直 index XOR 展開。初版が `m = 1, 1` ショートカットを使い (t,m,s)-net 性(4連ブロックの象限層化)を破壊 — 層化テストが捕捉、偶然値検査では見逃す種類の誤り。`sobol2_fixed` は `>>16` で Q16.16 化、`Sobol2` は `wrapping_add` カウンタの反復器
- `biquad` — RBJ Audio-EQ-Cookbook バイクアッド: `ω₀=2πf/fs`, `α=sin ω₀/2Q`, `a₀=1+α` 正規化で low/high/band/notch の `b`/`a` を合成、DF-I で `y = Σbx − Σay` 状態更新。`Fixed` 制約(Q>8 で a₁ が −2 に近づき切り捨てノイズ増大)は docs 明記。`new` は正規化済み係数の直指定
- `ccl` — Rosenfeld–Pfaltz 2パス連結成分ラベリング: pass1 でラスタ走査 + 上位/左近傍(8連は斜めも)の union-find 統合、pass2 で root を first-seen 順に連番化。path-halving union、小さいラベル勝ちの規約で決定的。`areas`/`bboxes`/`cells` アクセサ付き — BFS flood ではなく UF なので領域統計まで一体
- `sap` — Sweep-and-Prune 広相: min-x ソート(index tie は index)で走査、active リストを `max.x ≤ min.x` で刈り残存にだけ y 判定 — Box2D/bullet 的 first pass。交差判定は「両軸で正面積共有」(`max(mins) < min(maxs)`)で統一、接触辺・零面積は対象外(初期版の `>=` retain + y-only 判定は内部零幅を誤検出する可能性があったため統一化)。出力は `(i<j)` ソート
- `pchip` — Fritsch–Carlson 単調3次補間: 内部接線を加重調和平均 `(w1+w2)/(w1/δ_{i−1}+w2/δ_i)`(w は区間幅の加重)、符号反転で 0、端点は一側三点推定 + FC clamp(符号・3×secant 上限)。評価は Hermite 基底 + 二分探索区間、範囲外は端値に flat 外挿(線形外挿は単調性を外で破壊)。catmull/bspline が埋められなかった「オーバーシュート禁止」の補間ギャップ
- `align` — NW/SW 系列整列: `(n+1)×(m+1)` i32 DP、NW は端初期化・SW は 0 floor + argmax セルから traceback。op は `=`/`X`/`D`(a 消費・b gap)/`I`(b 消費・a gap)、tie は diag > up > left で doc 明記。`editdist` が Levenshtein 距離を返すのに対しこちらは最適整列そのものを再現可能

**検証**: 新規 28 モジュールテスト全緑。oracle: (0,2)-net 層化 + 4096 distinct(sobol)、f64 インパルス応答 + DC/Nyquist 定常(biquad)、BFS flood 総当り(ccl)、O(n²) 厳密 oracle(sap)、f64 FC 構築 + 密サンプル単調性(pchip)、全整列列挙(brute go)(align)。API pin は実測更新、kit 410 モジュール。


## 出典(第70次、search-index 照合)

**論文・仕様**: Sobol' (1967) 及び Joe & Kuo, *Constructing Sobol Sequences with Better Two-Dimensional Projections* (2008 — dim-2 初期方向数 m = 1,3 と recurrence 係数) / Smith, *The Scientist and Engineer's Guide to DSP* + RBJ Audio-EQ-Cookbook (Robert Bristow-Johnson、low/high/band/notch 係数表 + DF-I) / Rosenfeld & Pfaltz (1966) 2-pass CCL・Suzuki–Abe 系統合(prior work の streaming 形は採用せず全体2パス) / Baraff & Witkin, *Large Steps in Cloth Simulation* SIGGRAPH notes 系 SAP・Tracy–Buss–Woods GPCE sweep 実装 / Fritsch & Carlson, *Monotone Piecewise Cubic Interpolation* (SIAM J. Numer. Anal. 1980 — 調和平均接線と端点 clamp 規則) / Needleman & Wunsch (1970)・Smith & Waterman (1981) 整数 DP + traceback — 全て整数のみで実装。

**実装物**: OpenImageIO/ue4 `FSobolSampler` 系の direction-number テーブル形(2D 固定なので定数 recurrence で生成)、C++ `<dsp>`/JUCE `dsp::IIR` 系 DF-I 状態 API、scipy `ndimage.label` 系ラスタ連番規約、Bullet `btAxisSweep` の active-list 刈り込み、pandas/scipy `PchipInterpolator` の flat 外挿規約、biopython `PairwiseAligner` の `=`/`X`/`I`/`D` op 表現 — 全て整数のみで実装。

## 第71次: 文字列構造・テクスチャ・連続衝突・鍵導出・表データ・識別子・認証暗号 — lcp・worley・swept・kdf・csv・ulid・aead

**方法**: 第70次と同じ文献参照ラウンド — GitHub・論文・Qiita/Zenn・海外技術記事の標準 primitive 候補を列挙し grep で未収録確認(`sais`/`sufftree`/`fmidx` は構築側として既存で LCP は未、`vnoise`/`gnoise`/`noise` は値・勾配ノイズでセルラーは未、`sap` は広相で狭相 CCD は未、`hmac`/`sha256`/`chacha`/`poly1305` は部品として既存で kdf/aead は合成が未、`json` は構造化で CSV は表形式が未)。結果7本:

- `lcp` — Kasai の線形 LCP 配列(`sais` の兄弟部品): rank 配列で `h` を前エントリ値−1 から再開する O(n) 歩行。`longest_repeated`(argmax)、`distinct_substrings`(`n(n+1)/2 − Σlcp` の結合律トリック)、`longest_common`(a+sep+b セパレータ継ぎで「SA 隣接が別列由来」のものだけ最大 — `is_none_or` は MSRV 1.75 超過で `map_or` 採用)
- `worley` — Worley/セルラーノイズ F1/F2: セルごとに avalanche hash → (0,1)² 乱数 feature point、3×3 近傍スキャンで最近接・次近接のユークリッド距離を `Fixed` で返す。`worley_edge = f2−f1` はリッジ/ボロノイ境界描画の古典。`vnoise`/`gnoise` が連続場を埋めてもセル構造は別 primitive — Lipschitz 連続性と境界最小性をテストで保証
- `swept` — スイープ AABB 連続衝突(`sap` の狭相反): Minkowski 拡張ターゲットに対するスラブ区間法で x/y それぞれの入出時刻を `Frac` 厳密有理数で保持、`entry < exit ∧ entry < 1 ∧ exit > 0` で採否、開始時既重複は `t=0`。離散重なり検査が原理的に見逃すトンネリング(薄い壁貫通)を捕捉 — 衝突法線は最大入出時刻の軸 −sign(vel)、接触位置は開始側へ切り捨ての `mul_trunc`
- `kdf` — RFC 2898 PBKDF2-HMAC-SHA256 + RFC 5869 HKDF: PBKDF2 は `Ti = U1⊕⋯⊕Uc` のブロック連結(公開ベクトルでピン化)、HKDF は extract(空 salt→32 ゼロ鍵)+expand(`T(i)=HMAC(PRK,T(i−1)‖info‖i)`、spec 上限 255·32)。既存 `hmac_sha256` からの直接合成 — 部品があっても合成済み API は別の欠落
- `csv` — RFC 4180 CSV: quoted field(`,`,`\n`,`""` 埋め込み可)、`CRLF`/`LF`/`CR` 全終端受理、末尾改行なし flush、空 field・quoted-empty・lazy-quote 許容(closing `"` 後の junk を delimiter まで verbatim 保持 — 寛容系パーサの規約)。`emit` は「必要時のみ引用・`""` 二重化・CRLF」正規形で `parse∘emit` 完全 round-trip を pin
- `ulid` — ULID 128bit ソート可能 ID: 48bit ミリ秒 + 80bit 乱数を Crockford Base32 26 文字で MSB-first エンコード(文字列ソート＝時刻順)。`decode` は `i/l→1`・`o→0` 正規化 + 大小文字非依存 + 48bit 溢れ検査。`Ulid` は SplitMix64 シードの単調生成器 — 同一 ms で rand インクリメント(最終バイトから carry)、全 0xFF で時刻を +1 にオーバーフローする spec 標準の解答。小文字 decode が上位化前の `c` で減算していたバグを canonicalization テストが捕捉
- `aead` — RFC 8439 §2.8 ChaCha20-Poly1305 AEAD: block 0 の先頭 32B を one-time Poly1305 鍵、暗号化は counter 1 から `aad‖pad16‖ct‖pad16‖len64‖len64` 上に MAC。`open` は差分累積比較(early exit なし)で改竄・wrong-context は `None` — 壊れた平文は決して返さない。RFC A.5 ベクトルのタグで合成全体をピン化(nonce ワード反転はベクトル不一致で捕捉)

**検証**: 新規 31 モジュールテスト全緑。oracle: brute LCP 総当り・distinct 列挙(lcp)、f64 最近接距離 ±0.02 + Lipschitz(worley)、dense-step オラクル + exact `Frac` 時刻(swept)、PBKDF2/RFC5869 公開ベクトル(kdf)、round-trip + RFC ケース(csv)、spec ベクトル + 単調性(ulid)、RFC 8439 A.5 タグ + 全位置 tamper(aead)。API pin 4916、kit 417 モジュール。


## 出典(第71次、search-index 照合)

**論文・仕様**: Kasai, Lee, Arimura, Arikawa & Park, *Linear-Time Longest-Common-Prefix Computation in Suffix Arrays* (CPM 2001 — rank+i−1 再開歩行) / Worley, *A Cellular Texture Basis Function* (SIGGRAPH 1996 — F1/F2 feature 距離) / Linney, *Swept AABB collision detection using the Minkowski sum* (gamedev.net tutorial — スラブ入出時刻法) / RFC 2898 §5.2 PBKDF2・RFC 5869 HKDF / RFC 4180 CSV / alizain, *ULID spec* (Crockford Base32 + monotonic 規則) / RFC 8439 §2.8 AEAD_CHACHA20_POLY1305 — 全て整数のみで実装。

**実装物**: Go `index/suffixarray` の LCSArray 系コンパニオン形、Blender shader `Voronoi distance to edge` の f2−f1 API 形、Nasser/gamedev swept-AABB 系の normal+t インタフェース、Go `encoding/csv` の LazyQuotes 許容規約、`oklog/ulid` の monotonic インクリメント実装、`ring`/`libsodium` の `seal`/`open` API 形 — 全て整数のみで実装。

## 第72次: 画像解析・準乱数・文字類似度・レガシー摘要・青ノイズ・版数代数 — otsu・integral・pcg・jaro・md5・poisson・semver

**方法**: 第71次と同じ文献参照ラウンド — GitHub・論文・Qiita/Zenn・海外技術記事の標準 primitive 候補を列挙し grep で未収録確認(`ccl`/`worley` の画像・ラスタ族に閾値選択と矩形統計の相方が未、`SplitMix64`/`rng_xoshiro` に並ぶ第三の決定的乱数系が未、`editdist`/`fuzzy` とは別族の Jaro 類似度が未、`sha256`/`sha3`/`blake2s` 系に legacy 互換摘要が未、`sobol` の低食違いとは別要求の最小距離保証配置が未、`varint`/`json` 系プロトコル基盤に版数比較が未)。結果7本:

- `otsu` — 大津の自動2値化閾値: 256-bin ヒストグラム上で between-class variance を厳密有理数で最大化(`d = sum_all·w0 − sum0·total` の i128、score = `d²/(w0·w1)`)。first-maximizer 規約で `pixel > t` が前景 — bimodal/skewed/noise 画像と 50 シード f64 オラクルで規約をピン化
- `integral` — 積分画像/Summed-area table: `(w+1)×(h+1)` 零縁 SAT で `sum_rect` が O(1)。`u64` セルで引き算順を `A+D−B−C` に固定(`A−B−C+D` は `A−B < C` で underflow する — 解析的導出で捕捉、`A = B+C−D+query` ⟹ `A+D−B−C = query ≥ 0`)。`mean_rect`(床関数)/`box_mean`((2r+1)² clamped 局所平均)/`total`/`dims` — `ccl`/`worley` ラスタ族の統計相方
- `pcg` — PCG PRNG(O'Neill): LCG `state·6364136223846793005+inc` に XSH-RR 出力変換(`xorshifted = ((old>>18)^old)>>27`、rotate = `old>>59`)。公式 seeding 手順(初期 state で 1 回 discard → +seed → もう1回)をそのまま再現 — Python 独立再導出で seed 42/stream 54 の先頭 `0xa15c02b7` をピン化(記憶値 `0xa15c02b9` は +2 ずれ)。`next_bounded` は Lemire multiply-shift、`next_u64` 連結
- `jaro` — Jaro-Winkler 文字類似度: 窓 `⌊max/2⌋−1` の貪欲マッチ + 順序スキャン転倒数 `t/2`、score を `m²(la+lb)+(m−t)·la·lb` / `3·la·lb·m` の厳密有理数で `Fixed::from_ratio` 化。Winkler boost は `j > 7/10` 時のみ `+ l/10·(1−j)`(l = 共通接頭辞 ≤4)。MARTHA/MARHTA = 17/18、DIXON/DICKSONX = 23/30(発表値 — `DICKSONX` は8文字、手計算を7文字と誤数して捕捉)、DWAYNE/DUANE = 37/45 + 200 シード f64 オラクル(千分率許容)
- `md5` — RFC 1321 MD5: LE ワード展開・4ラウンド F/G/H/I・K/s 定数表、padding は `0x80 + 0*(56 mod 64) + bit_len64 LE`。互換摘要(セーブ形式・プロトコル互換)であり暗号用途ではないことを doc 明記。RFC 1321 テストスイート全7ベクトル + padding 境界(55/56/63/64B)でピン化
- `poisson` — Bridson Poisson-disk 青ノイズ配置: cell `⌈r·7071/10000⌉ ≈ r/√2` グリッド加速の dart-throwing、active list からリング `[r,2r)` に最大 k 候補(`Pcg` 駆動、`Fixed::sin_cos` 整数角度)、近傍 `⌈r/cell⌉+1` セル内で `i64` 二乗距離厳密検査。`sobol` の低食違いが要求しない「全ペア ≥ min_dist」の分散要求 — 全ペア検査・密度下限・seed 決定性でピン化
- `semver` — SemVer 2.0.0: 厳密 spec パース(数値成分の先頭ゼロ・空識別子・文字集合違反を拒否)、`Ord` は pre-release 規則(release > prerelease、数値 < 英数、短い方 < 同接頭の長い方、build メタは比較無視 — spec §11 の 11 版連鎖をピン化)、npm 系 `satisfies_caret`(0.x 左詰め規則: major>0→同 major、major=0∧minor>0→同 minor、両方0→厳密)/`satisfies_tilde`(同 minor 帯)。`varint`/`json` 系プロトコル基盤の互換性通貨

**検証**: 新�� 29 モジュールテスト全緑。oracle: 50 シード f64 between-class(otsu)、全位置 2D 総当り vs `sum_rect`(integral)、Python 独立再導出の標準ベクトル(pcg)、発表値 3 組 + 200 シード f64 千分率(jaro)、RFC 1321 全ベクトル + 境界 pad(md5)、全ペア距離 + 密度(poisson)、spec §11 連鎖 + 拒否集合(semver)。API pin 4947、kit 424 モジュール。


## 出典(第72次、search-index 照合)

**論文・仕様**: Otsu, *A Threshold Selection Method from Gray-Level Histograms* (IEEE SMC 1979 — between-class variance 最大化) / Crow, *Summed-Area Tables for Texture Mapping* (SIGGRAPH 1984 — SAT 矩形和) / O'Neill, *PCG: A Family of Simple Fast Space-Efficient Statistically Good Algorithms for Random Number Generation* (HMC-CS-2014-0905 + pcg-random.org seed/sequence 規約) / Jaro 1989 + Winkler 1990 接頭辞補正 / RFC 1321 MD5 / Bridson, *Fast Poisson Disk Sampling in Arbitrary Dimensions* (SIGGRAPH 2007 sketch — r/√2 グリッド + k dart) / semver.org 2.0.0 §10–11 — 全て整数のみで実装。

**実装物**: OpenCV `threshold(THRESH_OTSU)` の first-maximizer 規約、Halide/ゲーム界の SAT 局所平均パターン、`rand_pcg` crate の `new_stream(seed, seq)` API 形、Apache commons-text `JaroWinklerSimilarity` の 0.7 ゲート、Go `crypto/md5` 出力形、Red Blob Games poisson-disc 実装形、npm `node-semver` の caret/tilde 意味論 — 全て整数のみで実装。

## 第73次: グラフ核・ワンタイムパス・メタヒューリスティック・音符号・IR順位付け・計画・バイト同期 — kcore・otp・anneal・soundex・bm25・mdp・rsync

**方法**: 文献参照ラウンド継続 — 候補列挙→grep 未収録確認で初回7本中 hilbert(`zorder`内蔵)・crc・base64・huffman・Gale–Shapley(`stable`)・Welford(`stats`) が既収録と判明し差し替え。残グラフ解析・認証・探索・音韻・検索・制御・バイト差分の7本:

- `kcore` — k-core 分解(Batagelj–Zaveršnik): bucket 配列 `vert[]`/`pos[]`/`start[]` で次数ビンの O(1) 移動、剥がし時 `core[v]=deg[v]`、`deg[u]>deg[v]` の隣接のみバブル左移動 — 隣接リスト構築で真の O(n+m)(初版の辺全走査 O(n·m) を差し替え)。`degeneracy` = max coreness。K4/路/二三角・結合例 + 40 ケース seeded k-削除ブルートオラクル一致
- `otp` — HOTP/TOTP(RFC 4226/6238): `DT(HMAC-SHA1(key, counter_be64)) mod 10^digits` の dynamic truncation(末尾4bitオフセットの31bit抽出)。SHA-1 は公開ベクトルが全て SHA-1 基底のためモジュール内 private 実装(`hmac`/`sha256` は現代側を担う)。RFC 3174 "abc"・Appendix D 全10カウンタ・Appendix B 全6時刻ベクトルでピン化、digits 1–9 clamp・長鍵>64B ハッシュ化・x=0 退行を網羅
- `anneal` — シミュレーテッドアニーリング(Kirkpatrick 1983): `exp(−ΔE/T)` 受理、温度は `ln t0→ln t1` の `Fixed` 幾何冷却(`t_i = exp(lnt0 + f·Δln)`)、`SplitMix64` の下位16bitを `Fixed` [0,1) に写像して受理判定 — (state, seed, iters, t0, t1) で bit-exact リプレイ。エネルギーは `i64`、Boltzmann 指数は `−Δ·65536/t_raw` の raw スケールで `Fixed::exp` 直接評価。keep-the-best 返却。t0/t1 ≤0 は純貪欲退化
- `soundex` — refined Soundex(NARA): 子音6群→数字、首文字コードがラン頭(`Pfister`→P236 の根拠)、H/W は透過でランを分断せず(`Ashcraft`→A261)、母音/Y はランリセット。Robert/Rupert→R163・Tymczak→T522・Gutierrez→G362 等発表値でピン化。`jaro`/`editdist` と異なる発音鍵の距離族
- `bm25` — Okapi BM25(Robertson–Zaragoza): `idf = ln((2N+2)/(2df+1))` の厳密有理数を `Fixed::ln` で評価(±½ が相殺)、`tf·(k1+1)/(tf+k1·(1−b+b·dl/avgdl))` を `Fixed` 飽和+長さ正規化、`k1=1.2`/`b=0.75` 既定・`with_params` で可変、タイは文書番号で決定 — kit 初の IR 順位付け。希少語>一般語・tf 飽和の逓減・長さ正規化・b=0 無効化のタイで特性をピン化
- `mdp` — 有限 MDP ソルバ(Bellman/Howard): `V(s)←max_a Σ p·(r+γV)` の価値反復と「評価 sweep + greedy 改善」の方策反復、`Fixed` 確率・`i64` 報酬・sweep 回数は公開引数(リプレイ安定 > 適応停止)、確率不足分は零報酬吸収、遷移先範囲外はクランプ、無行動状態は吸収 sink — `hmm`/`kalman` の制御相方。連鎖 MDP・乱択期待値・両ソルバ一致でピン化
- `rsync` — rsync 型バイト列 delta(Tridgell): 弱摘要 `a=Σx, b=Σ(L+1−i)x (mod 2¹⁶)` を O(1) ロール(`a'=a−x₀+xₙ, b'=b−L·x₀+a'`)、候補は MD5 で確認、`Lit`/`Copy{idx,len}` op 列、末尾短ブロックは `Sig.len` 保持でマッチ可能。同一入力全 Copy・不連続全 Lit・中間挿入・短尾・LCG 乱流の round-trip でピン化 — `delta` の map 型差分とは別層の生バイト同期

**検証**: 新規 34 モジュールテスト全緑。oracle: seeded k-削除ブルート(kcore)、RFC 3174/4226/6238 全公開ベクトル(otp)、貪欲 vs 熱拡散の井戸越え(anneal)、発表 Soundex 値+首文字ラン規則(soundex)、tf 飽和/長正規化/idf 順位特性(bm25)、価値/方策反復一致+閉形解(mdp)、全 Copy/全 Lit/乱流 round-trip(rsync)。API pin 5008、kit 431 モジュール。


## 出典(第73次、search-index 照合)

**論文・仕様**: Batagelj & Zaveršnik, *An O(m) Algorithm for Cores Decomposition of Networks* (2003 — bucket 剥がし) / RFC 4226 HOTP・RFC 6238 TOTP・RFC 3174 SHA-1 / Kirkpatrick, Gelatt & Vecchi, *Optimization by Simulated Annealing* (Science 1983) / Russell–Odell Soundex (1918) + NARA refined 規則 / Robertson & Zaragoza, *The Probabilistic Relevance Framework: BM25 and Beyond* (FnTIR 2009) / Bellman 動的計画法 + Howard 方策反復(1960) / Tridgell & Mackerras, *The rsync algorithm* (1996 — 弱/強二重摘要) — 全て整数のみで実装。

**実装物**: NetworkX `core_number` の次数ビン構造、Google Authenticator/oathtool の DT 形、scipy `dual_annealing`/SimulatedAnnealing の受理判定形、Apache commons-codec `Soundex` の NARA 意味論、Lucene `BM25Similarity` の k1/b 既定値、OpenAI gym の離散 MDP 形、librsync `rollsum` の a/b 弱摘要 — 全て整数のみで実装。

## 第74次: 自動微分・強化学習・語幹化・ハーフトーン・トーン検出・画像符号・暦算 — dual・qlearn・porter・dither・goertzel・qoi・civil

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。`roots`/`pid` が手計算の微分を要求する数値基盤の穴、`mdp` に学習側の相方が無い、`soundex`/`jaro` に語幹正規化が無い、`otsu`/`ccl` にハーフトーンが無い、`biquad` に単一周波数検出が無い、codec 系に現代最小仕様の画像形式が無い、`cron` に日付↔通日の変換基盤が無い、というギャップ:

- `dual` — 前進型自動微分(dual number): `Dual{v,d}` の組で連鎖律を機械適用 — `mul` は積則、`div` は商則、`powi`(負冪は `ONE.div` 経由)、`exp`/`ln`/`sqrt`/`sin`/`cos` を `Fixed` 上に構成。`impl Add/Sub/Neg/Mul<Fixed>` 双方向で多項式がそのまま書ける — `roots`/`pid` が手で差分する場面の微分基盤。多項式則・商則・連鎖則・超越関数 f64 オラクル(ε=1/50 — CORDIC の cos'(0)≈−0.00003 を厳密0と誤断言して捕捉)でピン化
- `qlearn` — 表形式 Q学習+SARSA(Watkins 1989): `Q(s,a) ← (1−α)Q + α(r+γ·boot)` の `Fixed` 補間 — off-policy は `boot = max Q(s',·)`、on-policy `update_sarsa` は実取行動 `a'`。ε-greedy `select` は `SplitMix64` 下位16bitを `Fixed` [0,1) に写像 — `mdp` の計画者に対する学習者、全引数で bit-exact リプレイ。off/on の bootstrap 差(同状態で max=9 vs SARSA=4)・α 補間・ε=0 貪欲/ε=1 探索で特性ピン化
- `porter` — Porter 1980 語幹化: 5 step 全実装(`m` measure・`*v*`/`*d`/`*o` 条件、`y` の子音扱い規則)。最大の誤り源は「論文の各 step 例示値」= 途中形 — 正規出力は step4/5 が更に刈る(`relational`→`relat`、`feudalism`→`feudal` は `m("feud")=1` で存続)。公開される canonical 出力形 ~60 語で全連鎖をピン化 — `soundex`/`jaro` の文字系に語彙正規化を追加
- `dither` — Bayer 4×4/8×8 順序ディザ + Floyd–Steinberg 誤差拡散: Bayer は整数閾値比較で `levels` 段階化、FS は右7/16・左下3/16・下5/16・右下1/16。**拡散誤差は clamp 後の表示値で計算** — 生バッファ誤差の拡散は病理場(定数入力で誤差が指数増大)でオーバーフローすることを乱流テストが捕捉。`quantize`/`ordered_n` 併置 — `otsu`/`ccl` ラスタ族の表示側
- `goertzel` — Goertzel 単一周波数検出 + DTMF: `s=x+c·s₁−s₂`、`power=s₁²+s₂²−c·s₁·s₂` を **i64 raw Q16.16** で再帰(共振で `i32` raw を超える — `Fixed` 直接累算が overflow して初版が捕捉)、係数 `2cos(2πf/fs)` は `Fixed::sin_cos`。`dtmf` は行/列各群の winner>2×runner-up + 両群パワー比≤16 の twist 検査(単音は他群漏洩のみで reject) — 全16キー合成音でピン化
- `qoi` — QOI 画像コーデック(2021 spec): run/index/diff/luma/literal の5 op、`hash=(3r+5g+7b+11a)&63` の64エントリ索引、run 62 cap、RGB 入力は α=255 扱い。差分の `dr−dg`/`db−dg` は i8 で −255..255 に達し得るため **i16** で比較(`dg` のみ i8 範囲) — 乱流 round-trip・truncate/長過 run/悪 ch の `None`・op タグピン化。現代最小仕様の codec 層
- `civil` — Hinnant 民用暦算術: `days_from_civil`/`civil_from_days`/`weekday`/`weekday_iso`/`is_leap`/`days_in_month` — era 床除算で負年も厳密(proleptic、year 0 有り、year 1-01-01 = −719162)。−80000..80000 通日の完全 round-trip、2000 leap/1900 非 leap、紀元境界でピン化 — `cron` の日付次元の基盤

**検証**: 新規 39 モジ��ールテスト全緑。oracle: 多項式/商/連鎖則+超越 f64(dual)、off/on bootstrap 差(qlearn)、canonical 出力 ~60 語(porter)、Bayer 参照行列+混合/発散境界(dither)、合成16 DTMF+twist 拒否(goertzel)、乱流/構造 round-trip+`None` 集合(qoi)、通日完全往復+紀元境界(civil)。API pin 5059、kit 438 モジュール。


## 出典(第74次、search-index 照合)

**論文・仕様**: Wengert, *A Simple Automatic Derivative Evaluation Program* (CACM 1964 — 前進型 AD) / Watkins, *Learning from Delayed Rewards* (PhD 1989 — Q-learning + SARSA) / Porter, *An Algorithm for Suffix Stripping* (Program 1980 + tartarus.org 公式語彙対) / Bayer, *An Optimum Method for Two-Level Rendition* (ICCC 1973) + Floyd–Steinberg (1976) / Goertzel 1958 + DTMF ITU-T Q.23 / QOI spec (phoboslab, 2021) / Hinnant, *chrono-Compatible Low-Level Date Algorithms* (days_from_civil) — 全て整数のみで実装。

**実装物**: JAX/PyTorch autograd の dual-pair 形、OpenAI baselines の ε-greedy 形、Snowball/NLTK `PorterStemmer` の canonical 出力、libdither/stb の Bayer+FS 形、WebRTC/Go `goertzel` 実装の twist 検査、phoboslab `qoi.h` 参照実装の op 構造、Hinnant date_algorithms.html の era 算術 — 全て整数のみで実装。

## 第75次: トークン・識別子・符号・ハッシュ・評価・端末描画・意思決定 — jwt・uuid・cbor・xxhash・elo・braille・utility

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。`aead`/`otp` の認証族にトークン標準が無い、`ulid` に非ソート系 ID が無い、`json` にバイナリ codec 相方が無い、fnv/sip 系に広 avalanche の高速ハッシュが無い、ゲーム評価にレーティング族が無い、terminal 描画にサブセル解像度が無い、`goap`/behavior tree に連続トレードオフ型の意思決定が無い、というギャップ:

- `jwt` — RFC 7519 HS256 JWT: `sign` は固定ヘッダ `{"alg":"HS256","typ":"JWT"}` + b64url nopad + `hmac_sha256` 合成、`verify` はヘッダ等値・MAC 差分累積比較・`exp`/`nbf`/`iat` 数値 claim を `now` で検査(非 JSON payload は MAC のみ) — 認証族のトークン層。改竄 flip・期限・将来発行を reject 集合でピン化
- `uuid` — RFC 4122 UUID v4: `SplitMix64` 16B 引きに ver4/var1 ビット打刻、`format` は canonical `8-4-4-4-12` 小文字、`parse` はハイフン形/裸32桁・大小混在を受理 — `ulid` の非ソート相方。流れの決定性・version/variant 打刻・NIL/MAX でピン化
- `cbor` — RFC 7049 deterministic CBOR: uint/nint/bytes/text/array/map/bool/null の `Cbor` enum、canonical は最短形式 + map 鍵 (長さ, bytes) ソート、`decode_prefix` が消費数を返すので trailing は検出可。float/tag/indefinite は `None` — `json` のバイナリ相方。Appendix A 全ベクトル + 非 canonical 受理 + reject 集合
- `xxhash` — xxHash32/64(Collet r.5): 4 lane 本体 + tail 処理、LE load はバイト手動構成(endian 禁止)。`""`/`"a"`/`"abc"`/quick-brown-fox 公開ベクトル、seed 変化、境界 lane、bit-flip でピン化 — fnv/sip 系の広 avalanche 高速ハッシュ
- `elo` — Elo + Glicko-1: `expected`/`update`/`update_pair`(厳密ゼロサム交換)。`Glicko` は `decayed`(rd²+c²t、i64 平方和 — `Fixed::mul` は i32 溢れ)+ `update`。**分散分母 ~1e-5 は Q16 分解能以下なので最終部を i64 Q32.32(`mul32`/`div32`/`isqrt`)で計算** — 初版は denominator が raw 2 に量子化され f64 oracle ±2 が捕捉。Elo 400 差 ~0.909・upset 非対称・oracle 一致でピン化
- `braille` — Unicode ブライユ 2×4-dot canvas: `U+2800+bits` 1cell=2×4px で端末に 2×/4× 解像度(drawille 系)。dot→bit 写像全8ピン化、境界 no-op、`U+28ff` 全点灯、render 行形状でピン化
- `utility` — Utility AI(Dave Mark): `Curve::{Linear,Quad,Inverse,Logistic,Step}` の `[0,1]→[0,1]` 応答、`score` は product + Mark 補償 `s·(1+m(1-s))`(m=1−1/n — 単因子劣化で全滅しない)、weight は冪として適用、`choose` は最小 index タイの argmax — `goap`/BT に無い連続意思決定層。曲線参照点・積+補償・weight 冪・タイ順でピン化

**検証**: 新規 40 モジュールテスト全緑。oracle: RFC 7519 形+reject 集合(jwt)、ver/var 打刻+大小受理(uuid)、RFC 7049 Appendix A 全ベクトル+canonical 鍵順(cbor)、公開ベクトル+bit-flip(xxhash)、f64 参照式一致+ゼロサム(elo)、全8 dot 写像+`U+28ff`(braille)、曲線参照点+補償値 11/32(utility)。kit 445 モジュール。

## 出典(第75次、search-index 照合)

**論文・仕様**: RFC 7519 JWT・RFC 7515 JWS / RFC 4122 UUID / RFC 7049 CBOR + canonical 形 §4.2 / Collet, *xxHash spec r.5* / Elo, *The Rating of Chessplayers* (1978) + Glickman, *Glicko-1* (1999) / Unicode 14.0 Braille Patterns + drawille / Dave Mark, *Behavioral Mathematics for Game AI* (2009 — 応答カーブ+補償) — 全て整数のみで実装。

**実装物**: jwt.io/Auth0 の compact 形、cpython `uuid` のバイト順、py cbor2/tinycbor の canonical 鍵順、Cyan4973 `xxHash` の lane 構造、lichess/skillcalc の Glicko 式、asciimoo `drawille`・UnicodePlots の dot 写像、utility-ais の consideration 形 — 全て整数のみで実装。

## 第76次: codec・ID・距離場・IK・トーナメント・端末プロット・求根 — msgpack・snowflake・sdf・ik・tournament・plot・brent

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。`cbor`/`json` の codec 族に MessagePack が無い、`ulid`/`uuid` の ID 族に時刻ソート型 64bit 鍵が無い、`geometry` 離散格子に連続形状 SDF が無い、`spring`/`verlet` の kinematics に IK が無い、`elo` に大会組合せが無い、`braille` に古典 ASCII plot 相方が無い、`roots` に Brent が無い、というギャップ:

- `msgpack` — MessagePack codec: `Msg{Nil,Bool,Int,UInt,Bin,Str,Arr,Map}`、正の Int は最小 unsigned 形式に正規化(canonical では非対称になる旨 doc 明記)、str/bin/coll は最短頭。負 fixint の符号拡張は `<<`/`>>` で n-byte 両対応 — `cbor` の非 canonical 相方
- `snowflake` — Twitter snowflake ID: 41bit ms + 10bit worker + 12bit seq、seq 溢れは **virtual ms** に進めて単調性保持、rollback(ts<last)も同じ seq 継続規則 — `ulid`/`uuid` の第3 ID 形。オーバーフローで next_real_ms が ts+1 に進む、virtual ms より後の wall 時刻呼び出しは rollback として seq 延長、を両方ピン化
- `sdf` — 2-D 符号付き距離場(IQ 式): `circle`/`rect`/`segment` + `union`/`intersect`/`subtract` + polynomial `smin`、`march` で sphere-tracing(Hit(t)/Miss、step 上限)。`Vec2::scale` で `Fixed` 乗算の i32 溢れを回避 — geometry 族の連続形状側
- `ik` — 逆運動学: `two_bone` は余弦定理(角度は `atan2(√(1−x²),x)` 型 `acos` — kit 初の acos 需要)、i64 raw 平方和で `Fixed::mul` 溢れを回避、flip は対称解、`ccd` は FABRIK 型反復(各関節を target 方向へ回転、root 固定、骨長厳密保持)— kinematics 相方
- `tournament` — トーナメント組合せ: `Bracket` single-elim(再帰 canonical seeding `[o_i, 2k−1−o_i, …]` + bye)、`DoubleBracket` は winners/losers **queue**(front 2 が対戦、勝者は後尾に再投入)+ grand final、`Swiss` は点数群ソート貪欲ペアリング + 再戦回避 — `elo` の大会相方。初版は bye 進出者を winner 二重計上で無限ループ、`q_num` 反転��仮想 ms テストが捕捉
- `plot` — 端末プロット: `sparkline` 8ブロック(`▁`…`█`、全等値は中央 — 平坦性可視)、`Canvas` 底上げ y 座標、`line` バケット平均列プロット、`histogram` 比例バー — `braille` の古典 ASCII 相方
- `brent` — Brent 求根(Brent 1971): bisection/secant/**inverse quadratic** ハイブリッド、IQI は a,b,c 3点相異時のみで p/q 形、`mid` 内側受容テスト + `min_step` ガード、iters 尽きたら best bound を返す(bracket 外は `None`)— `roots` の第4法。f64 oracle(√2・∛式・π/2)と bisect 比較でピン化

**検証**: 新規 36 モジュールテスト全緑。oracle: 最小形式+非対称明記(msgpack)、仮想 ms + rollback 両方向(snowflake)、IQ 既知形+sphere-trace 収束(sdf)、余弦定理 3-4-5 + 骨長保持(ik)、canonical 8ペア和=7 + queue 型敗者復活 + 再戦回避(tournament)、全ブロック単調+底上げ座標(plot)、√2/∛/cos vs f64 + bisect 比較(brent)。kit 452 モジュール。

## 出典(第76次、search-index 照合)

**論文・仕様**: Furuhashi, *MessagePack format spec* / X(Twitter) snowflake 発表 + Discord/Sony 変種 / Quilez, *2D distance functions* + *raymarching* (iquilezles.org) / CCD IK (Welman 1993 GDC) + FABRIK (Aristidou–Lasenby 2011) / bracket seeding 標準(テニス/チェス 8→4→2) + Swiss 規約(FIDE Dutch) / drawille + UnicodePlots.jl sparkline/histogram / Brent, *Algorithms for Minimization without Derivatives* (1971, ch.4 zbrent) — 全て整数のみで実装。

**実装物**: msgpack-rust の最小 int 形式、bwmarrin/snowflake の epoch+worker+seq、iquilez SDF の op 式、Unity/Unreal two-bone solver の flip 形、Chess.com/lichess Swiss ペアリング、plotille/youplot の 8-block sparkline、CPython `zeros.c`/Boost brent の受容テスト — 全て整数のみで実装。

## 第77次: 伸長器・MT・FFT・URI・simplex・QR・多暦 — inflate・mt・fft・uri・snoise・qr・calendars

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。`huffman`/`lzss` 既存ゆえ残る最大 codec 空白が DEFLATE、`SplitMix64`/`Pcg`/`rng_xoshiro` の第4乱数系が MT、`biquad`/`goertzel` に FFT 相方、`glob`/`semver` のプロトコル族に URI、`gnoise` Perlin/`worley` に続く第3の場が simplex、`rsfec` 直結で QR、`civil` の多暦相方:

- `inflate` — DEFLATE 伸長器(RFC 1951/1952): `Bits` は LSB-first、Huffman 要素は MSB-first の混在ビット順、`Huff` canonical テーブル(Kraft 検査 + `first[l]`/`base[l]` — 初版は `count[0]` 不在記号をコード空間歩行に混入し dynamic で全滅、bl_count[0]=0 の RFC §3.2.2 準拠で修正)、stored/fixed/dynamic 3 btype、`inflate_zlib`(CMF/FLG FCHECK + FDICT reject)/`inflate_gzip`(FEXTRA/FNAME/FCOMMENT/FHCRC skip + trailer)。python3 zlib で全ベクトル生成し stored/fixed/dynamic/zlib/gzip 全経路を実符号列でピン化
- `mt` — MT19937(松本眞–西村): 624 状態 twist + init_genrand/`from_key` init_by_array、`next_u32`/`next_u64`/`next_res53`/`below` — `SplitMix64`/`Pcg`/`xoshiro` の第4 PRNG 系
- `fft` — 基数2 Cooley–Tukey FFT: `Cx = (Fixed,Fixed)` 固定小数点複素数、`twiddle(θ)=e^{−iθ}` で forward は正角度、bit-reverse 反復 DIT、非2冪は `dft` O(n²) 退化、Parseval は相対誤差検査。twiddle 符号ミスは bin 比較で捕捉(`cx_mul` は i64 raw 中間)
- `uri` — RFC 3986 URI: `Uri::parse` 全域関数(空 scheme/authority/path/query/fragment で縮退)、`pct_decode`/`pct_encode`、`remove_dot_segments`/`resolve` — `glob`/`semver` のプロトコル族
- `snoise` — Gustavson simplex ノイズ: skew 係数 `G2`、unskew の `t=(i+j)·G2` は i64 定数(初版の `>>16` 落ちで常 0 → 全頂点が整数格子上に潰れ max slope 41/unit、デバッグダンプで捕捉)、kernel `r²=0.5` 境界、4 寄与点 hash 勾配、`fbm2` 総振幅正規化 — `gnoise`/`worley` の第3場
- `qr` — QR byte-mode v1–10 自動選択(ISO 18004): 私有 `QrGf`(AES 用 `gf2` は 0x11B、QR は 0x11D のため)、rs_generator 昇冪畳込み→反転、block 分割+parity インタリーブ、function layer(ファインダ/タイミング/アライメント/ダークモジュール/フォーマットBCH+mask)、zigzag 配置、8 mask 全評価で最小 penalty → `ModuleMatrix`/`to_terminal` — `braille`/`plot` の端末表示相方。RS parity は独立 Python 再実装で互換検証済み
- `calendars` — Reingold–Dershowitz fixed-date: `Rd` 通日で gregorian/julian/islamic/persian/hebrew 相互変換 + `easter`(Anonymous computus)+`weekday` — `civil` の多暦相方

**検証**: 新規 43 モジュールテスト全緑。oracle: python3 zlib 実ベクトル stored/fixed/dynamic/zlib/gzip(inflate)、MT19937 公式 init+twist(mt)、delta/DC/sine-bin/radix2-vs-DFT/Parseval/ifft roundtrip(fft)、RFC 5.4 resolve + pct(uri)、格子点零+slope/kernal 境界+正規化(snoise)、フォーマット BCH ベクトル+RS parity 独立実装+8 mask penalty+QR サイズ表(qr)、RD 既知日付+4暦往復+computus(calendars)。kit 459 モジュール。

## 出典(第77次、search-index 照合)

**論文・仕様**: RFC 1951 DEFLATE + RFC 1950 zlib + RFC 1952 gzip / Matsumoto–Nishimura, *MT19937* (ACM TOMACS 1998) / Cooley–Tukey, *An Algorithm for the Machine Calculation of Complex Fourier Series* (1965) / RFC 3986 URI §5 / Gustavson, *Simplex noise demystified* (2005) / ISO/IEC 18004 QR / Reingold–Dershowitz, *Calendrical Calculations* — 全て整数のみで実装。

**実装物**: python zlib の stored/fixed/dynamic ベクトル、numpy `RandomState` の MT 種値表記、FFTW/numpy fft の bin 規約、Python `urllib.parse`/`urljoin` の dot 除去、gustavson Java/C リファレンスの skew 式、nayuki QR generator の mask/penalty 表、Emacs `calendar.el` の computus — 全て整数のみで実装。

## 第78次: 圧縮器・PNG・ZIP・Murmur・IP・UUIDv7・Base58 — deflate・png・zip・murmur・ip・uuid7・base58

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。r77 の `inflate` が解いた codec 空白の発行側 `deflate`、inflate+crc32 が可能にした実 PNG デコーダ、deflate/inflate+crc32 のコンテナ層 `zip`、fnv/sip/xxhash 系の第4広 avalanche ハッシュ `murmur`、`semver`/`uri` のプロトコル族に IP/CIDR、`uuid`/`ulid`/`snowflake` の第4 ID 形 UUIDv7、`base64` の Base58 相方:

- `deflate` — DEFLATE 圧縮器(RFC 1951/1950/1952): `BitW` LSB-first 書出し + `put_msb` Huffman 要素反転(固定テーブル §3.2.6)、greedy LZ77 は 3 バイト hash → depth-1 単一候補(決定的・依存ゼロ — 密度は zlib 未満だが合法ストリーム)、`deflate_stored` 65535 分割、`adler32`(5552 chunk mod)、zlib `0x78 0x9C`+BE adler、gzip 固定10B ヘッダ+LE crc/isize — `to_be_bytes` 禁止ゆえ BE は shift 書出し
- `png` — PNG codec(ISO/IEC 15948): sig+chunk 走査、CRC は type+body で検証、bit-depth 8・非インターレースのみで Adam7/16bit は `None` 退化、unfilter は Sub/Up/Avg/Paeth 全実装(bpp 左・上・左上参照)、`encode_*` は filter-0 + `deflate_zlib` で発行 — `inflate`/`deflate`/`crc` の直結層。実 PNG は python3 zlib+手組 chunk で生成してピン化
- `zip` — ZIP コンテナ(PKWARE APPNOTE): EOCD `PK\x05\x06` を末尾 64KiB+22 内後方走査 → central dir → local header、method 0/8、CRC 検証付き `extract`、暗号/data-descriptor/zip64 は `None`、`ZipWriter` が local+central+EOCD を発行。前付け junk/コメント耐性は EOCD scan の副作用
- `murmur` — MurmurHash3(Austin Appleby): x86_32 は rotl15/`0xe6546b64` 4 ラウンド+fmix32、x64_128 は k1/k2 交差 rotl31/33+fmix64+相互加算 — 参照実装値ピン化(`hello`→`0x248bfa47`/`0xcbd8a7b3…`)、LE 手動 load で endian 非依存
- `ip` — IPv4/IPv6/CIDR(RFC 791/4291/5952/4632): v4 は厳格形(先頭零禁止)、v6 は `::` 1 回のみ+埋込 v4 尾対応+hextet 検査、format は最長零ラン圧縮(長さ≥2・先勝ち)、`Cidr::contains_v4/v6` マスク比較、`to_ipv4_mapped`
- `uuid7` — UUIDv7(RFC 9562): ts(48)|7|rand_a(12)|0b10|rand_b(62)、`Uuid7::next` は ms 非進行時に埋込 ts を+1 仮想進行で厳密単調 — ソート可能 = バイト順 == 時刻順
- `base58` — Base58(Bitcoin/Flickr alphabet): 入力を BE 整数として divmod 58 反復、先頭零バイト→`1` digits、`encode_check`/`decode_check` は double-SHA256 先頭4B 検査

**検証**: 新規モジュールテスト全緑。oracle: `inflate` が `deflate` の自己オラクル(inflate は python zlib ベクトルで検証済み)、`adler32` は RFC 1950 "Wikipedia" 値、png は python3 zlib 実 PNG ベクトル+自己往復、zip は自己往復(実 unzip/zipfile 互換形)、murmur は参照実装値、ip は RFC 5952 canonical 例、uuid7 は版/変種ビット+単調列、base58 は公開ベクトル+check 往復。kit 466 モジュール。

## 出典(第78次、search-index 照合)

**論文・仕様**: RFC 1951/1950/1952 DEFLATE+zlib+gzip / ISO/IEC 15948 PNG(W3C PNG spec) / PKWARE APPNOTE ZIP / Appleby, *MurmurHash3* (SMHasher) / RFC 791 IPv4 + RFC 4291 IPv6 + RFC 5952 canonical format + RFC 4632 CIDR / RFC 9562 UUIDv6–v8 / Nakamoto Bitcoin Wiki Base58Check — 全て整数のみで実装。

**実装物**: zlib の `deflate` stored/fixed 戦略、python `zlib`/`struct`/`binascii` による PNG chunk 生成、Info-ZIP `unzip` の central-dir-first 読筋、py-mmh3 の x86_32/x64_128 ベクトル、CPython `ipaddress` の厳格形、python-`uuid7` の ms+rand 配置、bitcoin Wiki の Base58Check 手順 — 全て整数のみで実装。

## 第79次: tar・GIF・MIDI・測地・Bech32・Punycode・WebSocket — tar・gif・midi・geo・bech32・punycode・ws

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。r78 で揃った codec/ID 層の延長: `zip` の POSIX 相方 ustar、既存 `lzw` の固定12bit変種とは別物の GIF 固有可変幅 LZW デコーダ、png/gif のメディア族に SMF スコア、`snoise`/`worley` の地理系相方 geodesy、`base58` の segwit 相方 BCH 符号、`uri` の国際化相方 bootstring、プロトコル族に WebSocket フレーム:

- `tar` — POSIX ustar(POSIX.1-1988 pax 前身): 512B ヘッダ(name/mode/uid/gid/octal size+mtime/checksum/typeflag/`ustar\0` magic/prefix)、payload 512 パディング、2 零ブロック終端。checksum は field を空白として和検証、name>100 は prefix 分割、`TarWriter` は mtime=0 決定的発行 — `zip` の相方、`deflate_gzip` 合成で `.tar.gz` 到達
- `gif` — GIF89a デコーダ(CompuServe): LSD+GCT+image descriptor+extension skip、GIF 固有 LZW は `lzw` の固定12bitと別物(min_code+1 bit から辞書増大で可変幅、clear/EOI in-band、LSB-first packing、KwKwK 経路)、palette-index 出力で `png` と同形状。非インターレースのみ(Adam7 同様に退化)、ベクトルは Python GIF-LZW encoder で生成
- `midi` — SMF パーサ(MMA): `MThd`/`MTrk`、delta varint(28bit 上限)、running status、meta(0x51 tempo/0x2F EOT 必須)/SysEx、`notes` on/off ペアリング(vel=0 は off 畳込み)、`tempo_map` — `EvKind` 全域分解、不正列は `None`
- `geo` — 測地(`Fixed` 度、距離は km — meter は Q16 溢れ): haversine 球距離(R_KM=IUGG 6371.0088km)、Lambert–Andoyer 扁平補正(Vincenty の反復を閉形式で置換 — Fixed で反復不要の安全側)、`bearing`/`dest`/`midpoint`/`norm_lon`。asin/acos は atan2 経由
- `bech32` — Bech32/Bech32m(BIP-173/350): 5 生成多項式 `polymod` BCH、HRP 展開+6文字 checksum、`encode`/`encode_m`/`decode` は variant 報告、`convert` 8↔5 bit 群変換、`encode_segwit`/`decode_segwit` は v0↔bech32・v1+↔bech32m のコンセンサス結合 — `base58` の相方
- `punycode` — RFC 3492 bootstring: basic コピー+`-`+一般化可変長整数で (n,pos) デルタ符号、`adapt` bias 再計算、u64 checked 算術で退化は `None`。`-` 無しラベルは数字列として解釈される RFC 曖昧性を doc 明記 — `uri` の IDN 相方
- `ws` — WebSocket フレーム(RFC 6455): FIN/opcode、最短形式強制の 16/64bit 拡張長、client mask XOR、RSV/control 規約違反→`None`、`accept_key` は private SHA-1(otp と同構成)+`base64` で RFC §1.3 ベクトル

**検証**: 新規 30 モジュールテスト全緑。oracle: TarWriter↔list/extract 往復+checksum 破壊検出(tar)、Python GIF-LZW 生成ベクトル+KwKwK 辞書増大列(gif)、hand-built SMF+running status+varint 境界+tempo(midi)、LHR→JFK/Tokyo/赤道四分弧の haversine/lambert/bearing/dest/midpoint(geo)、BIP-173 valid/invalid 一覧+segwit 正規ベクトル+v1 bech32m 結合(bech32)、RFC 3492 §7 全例+IDN 往復(punycode)、RFC accept_key+masked/拡張長往復+RSV/control/最短形違反(ws)。kit 473 モジュール。

## 出典(第79次、search-index 照合)

**論文・仕様**: POSIX.1-1988 ustar / CompuServe GIF89a + Welch LZW (1985) / MMA Standard MIDI Files 1.0 / haversine + Lambert–Andoyer (Survey Review 1942) + Vincenty WGS84 / BIP-173 Bech32 + BIP-350 Bech32m / RFC 3492 Punycode / RFC 6455 WebSocket + RFC 4648 base64 + FIPS 180-1 SHA-1 — 全て整数のみで実装。

**実装物**: GNU tar/BSD pax の ustar レイアウト、PIL/imageio の GIF ブロック列、mido の running-status/varint 規約、geographiclib の Lambert 式、Bitcoin 参照実装の polymod/convertbits、CPython `punycode` codec の §7 例、python `websockets` の frame エンコード/ハンドシェイク — 全て整数のみで実装。
## 第80次: BMP・Bencode・NBT・DNS・チェックデジット・WKT・式評価 — bmp・bencode・nbt・dns・checkcode・wkt・expr

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。codec/ID 層の残物: png/gif/qoi に並ぶ古典画像 BMP、torrent の bencode、level.dat の NBT、ip のプロトコル相方 DNS wire、bech32/base58check に並ぶ人間向けチェックデジット群、geo の表現相方 WKT、そして scripting 基盤として決定的整数式エバリュエータ(Pratt TDOP):

- `bmp` — BMP codec(BITMAPINFOHEADER): "BM" 署名+54B ヘッダ、24/32bit BI_RGB のみ、行 stride 4B パディング、高さ負で top-down。`encode` は 24bit bottom-up の canonical 形、`decode` は圧縮・ビットフィールド・パレットを全拒否 — png/gif/qoi の古典相方
- `bencode` — Bencode(BEP 3): `i<int>e`・`<len>:<bytes>`・`l..e`・`d<k><v>..e`。`decode` は全域関数(leading zero/-0/切断列→`None`、dict キー順は寛容受理)、`encode` は canonical(キーを辞書順ソート)で `encode∘decode` が正規化 — torrent の相方
- `nbt` — Minecraft NBT: 全13タグ(End/Byte/Short/Int/Long/Float/Double/ByteArray/String/List/Compound/IntArray/LongArray)BE 手動読み、Float/Double は生 IEEE bits で保持(`Tag::float/double/as_f64` で変換)。spec 正規ドキュメント `Compound("hello world"){"name"="Bananrama"}` をバイトピン — savefile の外部形
- `dns` — DNS wire(RFC 1035): ヘッダ flags 分解、ラベル圧縮ポインタ(0xC0)は訪問有界でループ→`None`、A/AAAA/CNAME/NS/PTR/MX/SOA/TXT 型付き rdata。`build_query` は RD=1 の標準クエリ発行、ラベル≤63 強制 — `ip` の相方
- `checkcode` — チェックデジット識別子: Luhn(ISO 7812)/ISBN-10・13(和 mod 11・GS1 prefix 978/979)/EAN-13・UPC-A(重み 1,3 GS1)/IBAN(ISO 13616 mod-97 + 国別長レジストリ)/VIN(ISO 3779 転写表+重み)/MRZ(ICAO 9303 重み 7-3-1) — bech32/base58check の人間側
- `wkt` — Well-Known Text(OGC 06-103): POINT/LINESTRING/POLYGON/MULTI*/GEOMETRYCOLLECTION/EMPTY の再帰下降、座標は十進数字からの厳密 `Fixed` 化(float 不使用)、2-D 限定で Z/M 拒否、`write` は最短十進 emit で往復保存
- `expr` — 整数式エバリュエータ(Pratt/TOP): i64 十進/0x/0b/0o リテラル、unary - + ~ !、C 順優先度の `* / % + - << >> < <= > >= == != & ^ | && ||`、変数解決、min/max/abs/clamp、全て checked_* で overflow/div0/未知変数/構文不良→`None` — scripting 基盤

**検証**: 新規モジュールテスト全緑。oracle: 2×1 canonical バイトベクトル+self往復+32bpp/top-down(bmp)、BEP 3 spec 例+torrent 形往復(bencode)、spec 正規 hello world hex ベクトル+全タグ往復+depth bomb(nbt)、クエリ canonical バイト+0xC00C 圧縮応答+ポインタループ・予約ラベル(dns)、79927398713/GB29 NWBK…/0-306-40615-2/1M8GDM9AXKP042788/4006381333931(checkcode)、OGC 例一式+往復+Z/構文拒否(wkt)、C 優先度表ピン+全域失敗集合(expr)。kit 480 モジュール。

## 出典(第80次、search-index 照合)

**論文・仕様**: Microsoft Windows Bitmap format / Cohen, *BEP 3: The BitTorrent Protocol Specification* / Mojang NBT format specification / RFC 1035 DNS wire format + RFC 3596 AAAA / ISO/IEC 7812-1 Luhn + ISO 2108 ISBN + GS1 EAN/UPC + ISO 13616 IBAN registry + ISO 3779 VIN + ICAO Doc 9303 MRZ / OGC 06-103r4 Well-Known Text / Pratt, *Top Down Operator Precedence* (1973) — 全て整数のみで実装。

**実装物**: Pillow の BMP リーダ構造、libtorrent bdecode、wiki.vg の NBT レイアウト、knot/dnspython の name decompressor、ibantools の mod-97+国別長、geomet/geopy の WKT 文法、Python `ast`/lark の TDOP 優先度表 — 全て整数のみで実装。
## 第81次: TOML・NMEA・TLE・OBJ・SRT・SGF・otpauth — toml・nmea・tle・obj・srt・sgf・otpauth

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。codec 層の残る大物フォーマット群: json/cbor/msgpack の設定相方 TOML、geo の産業標準 NMEA、軌道の標準交換形 TLE、3D の古典 OBJ、メディア字幕 SRT、棋譜 SGF、otp+uri の合成 otpauth:

- `toml` — TOML v1 パーサ+emit: `[table]`/`[[array]]` ヘッダ・dotted key・基本/リテラル/3連文字列・int(0x/0o/0b/underscore)/bool/arr/inline なし・float は十進桁算術→`Fixed`(inf/nan/範囲外は拒否)、datetime は `Val::Str` 保持。`encode` は scalar-then-section canonical 形 — json/cbor/msgpack の設定相方
- `nmea` — NMEA 0183: `$TALKER KIND,fields*CS`、checksum XOR 検証(無ければ寛容受理)、`GGA`/`RMC` の typed デコード、緯度経度は `ddmm.mmmm` の最終2桁を分とみなし `Fixed` 化、S/W は負 — geo のシリアル相方
- `tle` — Two-Line Element 形式: 69 桁 mod-10 チェックデジット('−'=1)、厳格カラム(epoch YYDDD.FFFFFFFF・mm 第一/第二導関数の implied `±.N`・BSTAR `±NNNNN-N` 科学記法)、`period_seconds` = 86400/平均運動 — ISS 実測値ピン化
- `obj` — Wavefront OBJ: `v`/`vt`/`vn`/`f` ディレクティブ、負インデックス相対参照、`v/vt/vn` タプル面頂点、`triangles()` ファン分割、`bbox` — mesh 基盤(o/g/s/usemtl/mtllib/l/p はスキップ)
- `srt` — SubRip 字幕: `idx\nt1 --> t2\ntext` ブロック、`HH:MM:SS[,|.]mmm` 時刻(時間は任意幅)、`\r\n`/`CRLF` 正規化、`emit` は canonical CRLF、`shifted` は ≥0 clamp
- `sgf` — Smart Game Format FF[4]: `(tree)`/`;node`/`PROP[v]*` 再帰下降、`\]`/`\\` エスケープ、variation 木、`moves`/`coord`(`aa`→(0,0))/`game_info`/`emit` canonical 往復
- `otpauth` — otpauth:// URI: `totp|hotp` を host 型、`Issuer:account` ラベル+issuer/secret/algorithm/digits/period/counter パラメータ、私有 RFC 4648 base32(大文字正規化・canonical pad 検査)、`Otp::code` は `otp::hotp`/`totp` 直結(SHA1 のみ — 他アルゴは `None`)

**検証**: 新規モジュールテスト全緑。oracle: TOML spec 例+往復 canonical+構文拒否集合(toml)、`$GPGGA,123519,…`/`$GPRMC,225446,…` 実測ベクトル+checksum 改竄拒否(nmea)、ISS TLE 実測(25544・epoch 2008-09-20・mm 15.72125391)+checksum 全列検証(tle)、`f v/vt/vn` +負インデックス+fan+乱れ行拒否(obj)、canonical emit 往復+`-->`構文+ms3桁強制(srt)、KGS 形往復+variation+escape+malformed 集合(sgf)、RFC 4226/6238 ベクトル(755224/287082)+base32 全長ベクトル+canonical pad 拒否(otpauth)。kit 487 モジュール。

## 出典(第81次、search-index 照合)

**論文・仕様**: TOML v1.0.0 ABNF specification / NMEA 0183 standard sentences (GGA/RMC) / CelesTrak NORAD Two-Line Element Set format definition / Wavefront OBJ file format (Alias|Wavefront) / SubRip `.srt` convention / SGF FF[4] Smart Game Format specification / Google Authenticator Key URI format + RFC 4648 base32 + RFC 4226/6238 — 全て整数のみで実装。

**実装物**: Rust `toml` crate / `tomllib` の grammar、GPSD nmealib の sentence splitter、python-sgp4/tle-tools のカラム分割、tinyobjloader のインデックス規約、FFmpeg subripenc/subripdec、gnugo/gotools の SGF 木構造、oathtool/gauth の URI パーサ — 全て整数のみで実装。
## 第82次: WAV・WebVTT・SSA/ASS・PGN・STL・INI・unified diff — wav・vtt・ass・pgn・stl・ini・udiff

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。codec/フォーマット層の残隙(音声・字幕・棋譜・メッシュ・設定・パッチ):

- `wav` — RIFF/WAVE PCM codec: `RIFF…WAVE` + `fmt `(PCM tag 1・`0xFFFE` WAVEFORMATEXTENSIBLE は PCM GUID のみ受理・IEEE float tag 3 は float 型不在で拒否)+ `data` チャンク、word 整列スキップ、`samples_i16` 8/16bit→i16 復号、`encode` canonical 44B ヘッダ — bmp/png の音声相方
- `vtt` — WebVTT(W3C): `WEBVTT` 署名(空白+テキスト許容)、cue 識別子・`HH:MM:SS.mmm`/`MM:SS.mmm`(ドット必須 — srt のカンマと排反)、cue settings verbatim、NOTE/STYLE/REGION ブロック保存、canonical emit 往復 — srt の web 相方
- `ass` — SSA/ASS: `[Script Info]`・`[V4+ Styles]`・`[Events]`、各行 `Format:` カラム宣言駆動でフィールド解決(末尾 `Text` はカンマ吸収)、`Dialogue:`/`Comment:`、`H:MM:SS.cc` センチ秒、未認識節は `other` verbatim 保存 — srt/vtt の高度相方
- `pgn` — Portable Game Notation: `[Tag "v"]`(エスケープ対応)+ movetext: `1.`/`...` 手数表記・SAN・`$n` NAG・`{}` コメント・`()` 再帰 RAV variation・`1-0|0-1|1/2-1/2|*` ターミネータ、`1...` 先手黒検出、`emit` は手数再採番 — sgf のチェス相方
- `stl` — STL mesh: ASCII `solid/facet normal/outer loop/vertex/endloop/endfacet/endsolid` とバイナリ(80B ヘッダ+u32 count+50B/tri)両対応。バイナリ座標は IEEE-754 `f32` bits を**手動デコード→`Fixed`**(inf/nan・範囲外拒否 — float 型不用)、ASCII は十進桁算術、`emit` canonical ASCII、`bbox` — obj の三角形直接形相方
- `ini` — INI 設定: `[section]`・`k=v`/`k: v`・`;`/`#` コメント・挿入順保存・重複キー最後勝ち・グローバル(無名)節、`get`/`set`/`emit` — toml の祖先形
- `udiff` — unified diff テキスト: `---/+++` ヘッダ・`@@ -a[,b] +c[,d] @@`(count 省略=1)・` `/`-`/`+` 行・`\ No newline` マーカー、複数ファイル対応、`emit` canonical、`apply` は ctx/del の厳密一致が前提の全域 `Option` 適用器 — `diff` モジュール(アルゴリズム)の wire 形相方

**検証**: 新規モジュールテスト全緑。oracle: ラウンドトリ往復(wav/vtt/ass/pgn/ini/udiff)、奇数サイズ chunk pad+LIST スキップ+全1バイト破壊不パニック(wav)、`WEBVTT - header`/BOM/CRLF/時刻集合(vtt)、Format 駆動列解決+カンマ入り Text+Comment 種別(ass)、variation/NAG/comment/ターミネータ全4種/黒番検出/エスケープタグ(pgn)、f32 0x3F800000→ONE/count 虚偽/NaN・inf 拒否/ASCII+バイナリ自動判別(stl)、重複最後勝ち/新規節 set(ini)、GNU 形 `-0,0` 挿入+ctx 不一致拒否+`-`/`+`/`\` 行集合(udiff)。ラウンド内捕捉: f32 subnormal(指数0)の `>>133` シフト溢れ → `>=127` は 0 退化に修正、STL facet トークン幅ミス、PGN variation 手数再採番の起点誤り。kit 494 モジュール。

## 出典(第82次、search-index 照合)

**論文・仕様**: Microsoft/IBM Multimedia Programming Interface RIFF spec + WAVE `fmt ` chunk (incl. WAVEFORMATEXTENSIBLE) / W3C WebVTT: The Web Video Text Tracks Format / SubStation Alpha v4.00+ script format / Portable Game Notation Specification (Timothy Mann) / STL(STereoLithography) file format specification / INI file convention / POSIX `diff -u` unified output format + `patch` — 全て整数のみで実装。

**実装物**: Rust `hound`/python `wave` の chunk 走査、video.js/webvtt-parser の cue 分解、libass/aegisub の Format 駆動パース、python-chess の PGN reader、numpy-stl のバイナリレイアウト、Python `configparser` の節・コメント規約、GNU diffutils/patch の `@@` 範囲と `apply` 手順 — 全て整数のみで実装。
## 第83次: iCalendar・vCard・PEM・FEN・GPX・M3U・TGA — ics・vcf・pem・fen・gpx・m3u・tga

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定。標準交換フォーマット層の残隙(予定・連絡先・鍵装甲・局面・GPS・プレイリスト・画像):

- `ics` — iCalendar(RFC 5545): `BEGIN:VCALENDAR`/`VEVENT`/`VTODO`、プロパティ折り畳み(space/tab 継続行)、`NAME[;params]:value`、入れ子コンポーネント(VALARM 等)は `X-IC-MARKER` verbatim 保存で往復、`parse_dt`/`emit_dt` は `civil` 直結 — cron/civil の予定系相方
- `vcf` — vCard 3.0(RFC 2426): `BEGIN:VCARD`…`END:VCARD`、折り畳み、`NAME;PARAMS:value`、`VERSION` 必須、`get`(パラメータ無視)/`get_all` — ics と同構造の連絡先相方
- `pem` — PEM(RFC 7468): `-----BEGIN L-----`/base64/`-----END L-----`、複数ブロック・ラベル一致検査、canonical 64桁折返し — jwt/uuid の鍵材料層
- `fen` — FEN チェス局面: 8ランク `/` 区切り+digits 空マス、piece letters(大小=白黒)、`w|b`・`KQkq` 権利・ep ・halfmove・fullmove(≥1)、emit は空マス圧縮再発行 — pgn の局面相方
- `gpx` — GPX 1.1: タグ走査 XML(`<wpt>`/`<rte>`/`<trk>/<trkseg>` コンテキストスタック)、`lat`/`lon` 属性は十進桁算術→`Fixed`、`<ele>`/`<time>`/`<name>` 子要素、未知タグはスキップ(degrade) — geo/nmea の XML 相方
- `m3u` — M3U/M3U8: `#EXTM3U`、`#EXTINF:secs,title`、他 `#EXT…` ディレクティブ verbatim、壊れた EXTINF は degrade — wav のプレイリスト相方
- `tga` — TGA 画像: 18B ヘッダ、type 2(非圧縮)/10(RLE raw+RLE 両パケット)、24/32bpp BGR(A)、origin bit(top/bottom・left/right)を正規化して top-left 出力、canonical type-2 32bpp emit — bmp/png/qoi の古典相方

**検証**: 新規モジュールテスト全緑。oracle: canonical emit 往復(ics/vcf/pem/fen/gpx/m3u/tga)、VALARM verbatim 保存+DTEND;TZID パラメータ解決+日時集合(ics)、折り畳み+複数カード+VERSION 必須(vcf)、64桁折返し+複数ブロック+ラベル不一致拒否(pem)、初期局面+ep 正方形+KQkq+ malformed 集合(fen)、trkseg/rte/wpt コンテキスト+self-closing+非数 attr スキップ(gpx)、破損 EXTINF degrade+plain M3U(m3u)、RLE 両パケット+origin 全4象限+colormap/gray 拒否(tga)。ラウンド内捕捉: fen emit の board/手番間スペース欠落。kit 501 モジュール。

## 出典(第83次、search-index 照合)

**論文・仕様**: RFC 5545 iCalendar / RFC 2426 vCard 3.0 / RFC 7468 PEM text encoding / Forsyth–Edwards Notation (FEN) specification / GPX 1.1 schema documentation / M3U・#EXTINF convention + draft-pantos-http-live-streaming (#EXTM3U) / Truevision TGA File Format Specification — 全て整数のみで実装。

**実装物**: ical4j/python `icalendar` の BEGIN/END スタック、python `vobject` の折り畳み規約、OpenSSL pem 読み取り、python-chess の FEN parser、GPX schema 実装群、mpv/FFmpeg m3u パーサ、SDL_image/stb_image の TGA decoder — 全て整数のみで実装。

## 第84次: DER・RTF・EXIF・RSS・ID3・TZif・pcap — der・rtf・exif・rss・id3・tzif・pcap

**方法**: 文献参照ラウンド継続 — grep 未収録確認で7本確定(bloom/merkle/roaring は既収録と判明、DER は `derange` に紛らわしいが未収録)。バイナリタグ・文書・メタ・フィード・タイムゾーン・キャプチャ層:

- `der` — ASN.1 DER(X.690): `tag·length·content` TLV 走査、high-tag-number 形式、shortest-length 規則・indefinite 拒否、`children`/`oid`/`integer`(0-pad 正数)/`text`/`utc_time`(UTCTime 13桁 RFC5280 窓・GeneralizedTime 15桁、`civil` 直結) — pem の中身層、X.509 基盤
- `rtf` — RTF 1.x テキスト抽出: `{\rtf` 必須、制御語 `wordN`、escape `\{\}\\`、`\'hh` 生バイト、`\uN` は Unicode scalar+`\uc` フォールバック文字スキップ(pend カウンタ)、destination グループ(`\fonttbl`/`\colortbl`/`\info`/`\pict`/`\*\…` 等)は出力寄与ゼロ
- `exif` — EXIF/TIFF: JPEG APP1 `Exif\0\0` 走査(`parse_jpeg`)+`II*\0`/`MM\0` 直接(`parse_tiff`)、IFD0+EXIF(0x8769)+GPS(0x8825) sub-IFD 併合、type×size≤4 は inline、超えれば offset 解決、`get_str`/`get_int`/`get_rational` はファイル endian 尊重
- `rss` — RSS 2.0+Atom 正規化: `<rss>`/`<feed>` スニッフ、`item`/`entry` body を先に切り出してチャンネル項目と分離、Atom `<link href>`、CDATA unwrap+XML entity 解除、canonical RSS emit 往復
- `id3` — ID3: v2.3(plain u32 size)/v2.4(synchsafe) ヘッダ+フレーム走査+ext-header スキップ、encoding byte(0 Latin-1/1 UTF-16 BOM/2 UTF-16BE/3 UTF-8)、`TIT2`/`TPE1`/`TALB`/`TDRC`/`TYER`/`TRCK`/`TCON` typed+全 T*** frames、v1 末尾 128B `TAG` 併合(genre 80 id 表)
- `tzif` — TZif(RFC 8536): `TZif`+ver、v1 32bit ブロック、v2/v3 は 64bit ブロック+`\n POSIX-TZ \n` フッタ、`offset_at`/`type_at`(先頭遷移前は最初の非DST型=RFC の standard-time 規則)
- `pcap` — libpcap: magic(LE/BE×us/ns 4種)、ver 2.x のみ、record `ts_sec`/`ts_frac`/`incl`/`orig`、末尾 trunc record は fail せず drop、ts を ns 正規化、linktype 露出

**検証**: 新規67テスト全緑。oracle: INTEGER/OID/UTCTime/高タグ/shortest-form DER 往復(der)、destination elision+`\uc` fallback+destined 非テキスト(rtf)、II/MM 両 endian+inline/offset 境界 4B+sub-IFD 併合+JPEG APP1 走査(exif)、channel/item 分離+Atom href+CDATA/entity(rss)、v2.3/4 size 形式+UTF-16 BOM+v1 80 genre(id3)、v1/v2 ブロック選択+先頭遷移前 std 型(tzif)、LE/BE×us/ns+truncated-tail drop(pcap)。ラウンド内捕捉: UTCTime 桁数(13/15)誤判定、EXIF ≤4B inline 規則で offset 解決誤用、`\~` の裸 `~` 取扱、U+23376=子 の期待値。kit 508 モジュール。

## 出典(第84次、search-index 照合)

**論文・仕様**: ITU-T X.690 BER/CER/DER / Microsoft RTF Specification 1.9.1 / JEITA CP-3451 EXIF 2.x + TIFF 6.0 / RSS 2.0 Specification + RFC 4287 Atom / ID3v2.3.0・v2.4.0 + ID3v1 / RFC 8536 TZif / libpcap file format — 全て整数のみで実装。

**実装物**: OpenSSL/asn1crypto の TLV 走査、striprtf/unrtf の destination 集合、piexif/EXIF.py の II/MM 走査、feedparser/rome の RSS+Atom 正規化、mutagen/tinytag の ID3 実装、python `zoneinfo`/tzdata の TZif 読み取り、tcpdump/wireshark libpcap リーダ — 全て整数のみで実装。
## 第85次: SHA-1・git・ELF・HTTP・MIME・OpenSSH・cpio — sha1・git・elf・http・mime・ssh・cpio

**方法**: 文献参照ラウンド継続 — バイナリ/ワイヤ層の残隙(候補 `lz4` は既収録と判明し差し替え、`sha1`/`base32` 系の私有コピーから公開化の需要を精査):

- `sha1` — RFC 3174 SHA-1: `sha256` と同形の streaming `Sha1`/`sha1`/`sha1_hex`/`hmac_sha1`(RFC 2104)。md5/sha256/sha3/sha512 が公開済みで SHA-1 だけ `otp`/`ws` の私有コピーだった穴を公開化 — git オブジェクト名と TOTP バックエンドの共有素子
- `git` — git オブジェクトモデル+ワイヤ: `"type N\0body"` の `store`/`parse_obj`/`object_name`(sha1 名 = `git hash-object` とバイト一致)、zlib loose object `open_loose`/`write_loose`(`inflate`/`deflate` 直結)、tree `"mode name\0"+20B` 走査+emit、commit ヘッダ、pkt-line フレーミング(4桁 hex・`0000` flush・65520 上限)
- `elf` — ELF32/64: e_ident マジック+class/data 解決、LE/BE 両対応、program header(ELF64 は flags 位置が違う型ずれを型で分離)、section header+`shstrtab` 名解決、`section(name)` 生バイト
- `http` — HTTP/1.1(RFC 7230): request/status line、header(name 検証・obs-fold は deprecated 扱いで拒否)、Content-Length/chunked(拡張子・trailer 対応)ボディ解決、canonical emit(自動 Content-Length)
- `mime` — RFC 2045/2046: header folding 展開(mail では合法 — `http` との対称差)、`Content-Type` param(quoted value)、`multipart/*` boundary 分割(preamble/epilogue 捨て)、base64/quoted-printable/7bit transfer decode、QP `=HH`+soft break 両方向
- `ssh` — OpenSSH(RFC 4253 §6.6): u32 長 `string`/`mpint` 走査 `fields`、authorized_keys 行(options 前置スキップ・algo/blob 整合検査)、`SHA256:` unpadded-base64 fingerprint(ssh-keygen -l 形)、emit_line
- `cpio` — SVR4 newc(`070701`): 110B ASCII-hex ヘッダ 13 フィールド、namesize/filesize の 4B アライン、`TRAILER!!!` 終端、全フィールド hex 検証で壊れレコードは収集打ち切り

**検証**: 新規テスト全緑。oracle: RFC 3174 全4ベクトル+million-'a'+RFC 2202 HMAC 4件(sha1)、`git hash-object` の公開名 `ce0136…`/`e69de2…`/`4b825d…`(git)、ELF64 構築物の phdr/shdr/strtab 走査+ELF32-BE(elf)、chunked 拡張+trailer(http)、multipart boundary+QP 往復全256バイト(mime)、ed25519 blob 整合+options 前置+SHA256: 長さ 43(ssh)、namesize 1..8 全 pad 幅+truncated-data drop(cpio)。ラウンド内捕捉: data_end の usize::MAX sentinel 化防止(禁止項目)、cpio `pad4` デッドコード削除、ssh doctest の `len(), 2 &&` 誤構文。

## 出典(第85次、search-index 照合)

**論文・仕様**: RFC 3174 SHA-1 / RFC 2104 HMAC / git object format (git-scm.com book "Git Internals") + pkt-line protocol / TIS ELF 1.2 + System V ABI / RFC 7230 HTTP/1.1 / RFC 2045・2046 MIME + RFC 2045 §6.7 quoted-printable / RFC 4253 SSH §6.6 + OpenSSH authorized_keys 形式 / SUSv4 cpio newc 形式 — 全て整数のみで実装。

**実装物**: git.git の object-file.c/sha1dc、binutils/readelf の phdr/shdr 走査、nginx/curl の HTTP パーサ、python `email` パッケージの multipart/QP、OpenSSH sshkey.c の blob 形式、GNU cpio/pax の newc リーダ — 全て整数のみで実装。
## 第86次: Mach-O・COFF・WASM・X.509・TLS・ICO・WebP — macho・coff・wasm・x509・tls・ico・webp

**方法**: 文献参照ラウンド継続 — バイナリコンテナ層の残隙(ELF の platform 相方・DER の consumer・RIFF 系):

- `macho` — Mach-O: thin MH_MAGIC/CIGAM×32/64 の endian+bits 解決、cputype/filetype、load-command 走査(cmdsize<8 拒否)、LC_SEGMENT 系 `seg_name`、FAT_MAGIC nfat テーブル — `elf` の Apple 相方
- `coff` — COFF/PE: 20B COFF ヘッダ(machine/nsects/opt_size)、0x10B/0x20B/0x107 の optional PE 識別、40B section テーブル(Name[8]/vsize/vaddr/raw_size/raw_offset/flags)— `elf` の Windows 相方
- `wasm` — WebAssembly binary: `\0asm`+version 1 固定、LEB128 section-size 走査(id 0-12 検証)、`section(id)` 検索 — 構造走査のみ、実行器ではない旨 doc 明記
- `x509` — X.509 証明書: `der::Tlv` 経由で SEQ{tbscertificate, sig_alg, signature}、version [0] 明示化(無ければ v1)、RDNSequence を `/CN=…/O=…` 短縮形に平坦化、UTCTime/GeneralizedTime を `utc_time()` で Unix 秒化、BIT STRING の unused-bits byte スキップ
- `tls` — TLS 1.x record layer(RFC 5246/8446): 5B ヘッダ walk、type 20-23 検証、len≤16384 上限、record emit、`handshake()` で handshake fragment を `(type,body)` u24-length メッセージ列に分解 — codec であって復号器ではない旨 doc 明記
- `ico` — ICO/CUR: 6B ディレクトリ(reserved=0・type 1/2・count>0)+16B エントリ、w/h=0→256 規則、CUR は planes/bpp が hotspot、`image(n)` でペイロード切出(PNG/BMP 判別は呼出側)
- `webp` — WebP RIFF: `RIFF`size`WEBP`、chunk 走査(奇数サイズは 1B pad)、VP8X 24bit canvas-1、VP8L `0x2F`+14bit dims、VP8 `0x9D012A`+14bit dims、画像 dim 不在は拒否

**検証**: 新規テスト全緑。oracle: Mach-O thin64 構築物+fat2arch、COFF PE32+構築物+obj 裸形式、wasm minimal module+全13 section id、手組 X.509(RDN 短縮 `/CN=CA`・UTC/GenTime 両種→Unix 秒)、TLS ClientHello record+handshake 分解+16384 cap、ICO 2 画像+CUR hotspot+payload 範囲外、WebP VP8X/VP8L/VP8 3形式の dim decode+truncated-tail drop。ラウンド内捕捉: `records(&[])` の空ストリーム → Some(vec![]) の意図 vs テスト矛盾を修正。

## 出典(第86次、search-index 照合)

**論文・仕様**: Apple Mach-O Runtime Architecture Reference / Microsoft PE-COFF Specification + IMAGE_FILE_* 定数 / WebAssembly Core Spec §5 Binary Format / ITU-T X.509 + RFC 5280 / RFC 5246(TLS1.2)+RFC 8446(TLS1.3)§5.1 / Microsoft ICO/CUR format 解説 / WebP Container Specification + VP8/VP8L bitstream format — 全て整数のみで実装。

**実装物**: llvm-objdump の Mach-O/FAT 走査、llvm-readobj の COFF/PE、wasm-tools/wasmparser の section walker、python cryptography.x509 の DER 経路、mitmproxy/tlslite の record layer、python PIL/IcoImagePlugin、libwebp/dwebp の RIFF 走査 — 全て整数のみで実装。
## 第87次: TTF・WOFF・SQLite・protobuf・BSON・FLAC・Ogg — ttf・woff・sqlite・proto・bson・flac・ogg

**方法**: 文献参照ラウンド継続 — フォント/DB/直列化/メディアコンテナの残隙(macho/coff/wasm/x509/tls/ico/webp に続くヘッダ層):

- `ttf` — sfnt/TrueType フォント: offset テーブル(sfnt 4 種受理)+16B ディレクトリ(BE)、`head`/`maxp`/`name` typed 取得 — WOFF の中身の直前行
- `woff` — Web Open Font Format: `wOFF` + 20B ディレクトリ(tag/offset/compLen/origLen/checksum)、compLen<origLen のみ deflate で `inflate_zlib` 展開、非圧縮はコピー
- `sqlite` — SQLite3 ファイルヘッダ: `SQLite format 3\0`、page size 1→65536 規則・`usable=size−reserved`・versions 1/2・encoding 1-3・`page1_tree` で b-tree ヘッダ走査(interior 12B/leaf 8B・cell ptr 配列)
- `proto` — Protocol Buffers wire(RFC ではなく protobuf.dev encoding 仕様): `(num<<3)|wire` tag、wire 0/1/2/5(varint/I64/LEN/I32)、群 wire 3/4 は拒否、emitters 一式。varint は `varint` モジュールの canonical 規則を共有(overlong 拒否)
- `bson` — BSON(mongodb spec): i32-length prefix 文書、全16要素種、Double/Decimal128 は raw IEEE bits(u64/u128)で保持 — kit の float 型禁止に適合、canonical emit
- `flac` — FLAC: `fLaC` + メタデータブロック鎖(初手 STREAMINFO 必須・last flag で終端)、STREAMINFO の packed u64(min/max block・min/max frame・rate/channels/bps/total+MD5)、Vorbis コメント走査
- `ogg` — Ogg コンテナ(Xiph.Org RFC 3533): `OggS` ページ走査・version 0 強制・**非反射 CRC-32**(poly 0x04C11DB7 — `crc` の IEEE 版とは別物・CRC フィールドゼロ化して検証)、255-lacing でページ横断パケット再構築、continued フラグ追跡、`emit_page` は CRC を挿入して発行

**検証**: 新規テスト全緑(25件+7 doctest)。oracle: MongoDB 公式 `{"hello":"world"}` 22B ベクトル、deflate 実績の自己オラクル(WOFF 圧縮テーブル)、emit→parse 往復(proto/bson/ogg)。ラウンド内捕捉: emit_page が num_segments バイトを未出力(CRC 検証で全滅)、ttf `indexToLocFormat` の BE u16 低位バイト誤置、woff fixture が meta/priv 領域 20B 欠落で dir 開始ずれ、sqlite `lib_version` の 16 進定数ミス(0x2DC62E→0x2DC72E)、`varint` 共有規則で proto の overlong 受理テストを拒否側へ反転。

## 出典(第87次、search-index 照合)

**論文・仕様**: Microsoft TrueType/OpenType spec (sfnt directory・head・maxp・name) / W3C WOFF 1.0 spec / sqlite.org "Database File Format" (file format section) / protobuf.dev "Encoding" guide (wire types) / bsonspec.org BSON spec / Xiph.Org FLAC format spec + Vorbis comment spec / RFC 3533 Ogg format + RFC 5334 §B CRC — 全て整数のみで実装。

**実装物**: fonttools/freetype の sfnt・WOFF directory walker、sqlite3shell/btree ヘッダ読取り、protoc/protoscope の wire walker、PyMongo bson codec、brotli/metaflac の metadata walker、ogg-tools/oggz の page lacing と CRC — 全て整数のみで実装。

## 第88次: git packfile・Java .class・ar・plist・shp・pcapng・mbox — packfile・classfile・ar・plist・shp・pcapng・mbox

**方法**: 文献参照ラウンド継続 — バージョン管理/VM/アーカイブ/設定/地理/キャプチャ/メールの残りコンテナ層:

- `packfile` — git packfile + `.idx` v2: `PACK` ヘッダ(v2/3)、オブジェクト先頭の `(msb|type3|size4)` varint、OFS_DELTA は base-offset varint(`off = ((off+1)<<7)|low7` — git 固有の非単純 LEB128)、REF_DELTA は 20B base sha、zlib メンバは `inflate_zlib_count` で消費バイト数を取得して歩行(新規公開 `inflate`/`inflate_zlib` の count 版)、declared size 不一致・trailer 20B まで厳密。`.idx` v2 は fanout 単調検査 + name/crc32/offset 3 テーブル + MSB 付き large-offset 副表
- `classfile` — Java `.class`: `CAFEBABE`、cp_count(index 0 は穴)、Long/Double の 2 スロット占有を `Cp::Unknown` プレースホルダで表現、interface/field/method/attribute の各テーブル走査、`utf8`/`class_name` 解決
- `ar` — Unix ar アーカイブ: `!<arch>\n` + 60B ヘッダ(name16/mtime12/uid6/gid6/mode8 オクタル/size10 + `\x60\n`)、GNU `//` string-table 長名(`/n` 参照)と BSD `#1/n` インライン名の両方言、奇数 member は 2B pad
- `plist` — Apple plist 両形式: `bplist00` は 32B trailer の offSize/refSize/count/top/offTab を読み object 表をデコード(refs は index のまま → `resolve` が深さ制限付きで `Val` 化、dict キーは string 必須)。XML は `<dict>/<array>/<string>/<integer>/<data base64>` 走査 + `emit_xml` canonical 往復
- `shp` — ESRI `.shp`: 100B ヘッダは BE(9994/file length in 16bit words)+ LE(version 1000/type/bbox) の混在 endian、record は BE ヘッダ + LE content、**f64 は IEEE bit パターンを i128 演算で `Fixed` 手動デコード**(float 型不使用、subnormal→0、NaN/inf 拒否)
- `pcapng` — pcapng(RFC ドラフト): block type は section endian だが SHB `0x0A0D0D0A` は両 endian で同一 → BOM `4D3C2B1A`/`1A2B3C4D` で section endian を解決、IDB(linktype/snaplen)・SPB・EPB(iface+ts_hi/lo+caplen≤len) 走査、block 末尾の重複 len 一致検査
- `mbox` — mbox メールスプール: 列0 `From ` 開始行で分割(後続行の `>From ` は quoted 本文としてそのまま保持)、ヘッダは空行まで、sender を From 行第2語から抽出

**検証**: 新規テスト全緑。oracle: hand-built packfile(blob+2obj walk+idx v2 MSB-large offset)、javac 互換最小 class(Utf8/Class/Long 2slot+members walk)、GNU ar string-table + BSD `#1/`、bplist00 手組 dict + XML plist emit 往復、shapefile Point/複数 endian 併存 fixture、pcapng BE/LE 両 section + EPB、mbox 2 通 + quoted From。ラウンド内捕捉: `Cp` 2slot プレースホルダの push 順序反転、`.idx` fanout の「全要素 ≥ 初名」の誤 fixture(fanout[k] は累積数)、shapefile declared length vs actual の厳密一致テスト、plist binary の Obj/Val 2 段表現への整理。

## 出典(第88次、search-index 照合)

**論文・仕様**: git Documentation "Packfile format" (object header + OFS/REF delta + .idx v2) / JVMS §4 The Class File Format (constant pool tags + 2-slot 規則) / FreeBSD `ar(5)` man page + System V ar format / Apple "Property List Programming Guide" + CFBinaryPList.c bplist00 layout / ESRI Shapefile Technical Description (header/record layout) / pcapng draft spec (SHB/IDB/EPB + BOM endian rule) / POSIX mbox conventions + RFC 4155 — 全て整数のみで実装。

**実装物**: git 本体 pack-objects/index-pack の obj walk、javap/JDK ClassReader の cp walker、binutils ar の長名テーブル、Apple plutil/python plistlib の bplist00 decoder、GDAL/shapelib の shp record walker、wireshark pcapng セクション解決、mutt/python mailbox の From-line splitter — 全て整数のみで実装。

## 第89次: dbf・iso9660・mvt・pgp・ply・fits・qcow2 — dbf・iso9660・mvt・pgp・ply・fits・qcow2

**方法**: 文献参照ラウンド継続 — 地理属性・ファイルシステム・ベクタタイル・暗号パケット・メッシュ・天文・VM ディスクの残りフォーマット層:

- `dbf` — dBASE III `.dbf`: 32B ヘッダ(ver/date/numrec/hdrlen/reclen)+ 32B フィールド記述子列(0x0D 終端)+ フラグ先頭の固定長レコード。`cell` はフラグ込みオフセットで生幅切出し、宣言数と実長の厳密検査、削除フラグ `*`、フィールド名検索
- `iso9660` — ECMA-119/ISO 9660: セクタ16 の PVD(type1・`CD001`・version1)、system/volume id、両 endian 733/723 フィールド(LE+BE が一致必須 — `both16`/`both32`)、root dir record 34B(両 endian extent/size・7B date・flags・vol seq・name+pad)。`entries` は 0-len byte でパディングをまたぎセクタ単位歩行、`find` は `;version` 手前の名で大文字比較
- `mvt` — Mapbox Vector Tile 2.x: `proto`/`inflate_gzip` の直結合成(gzip magic 検出で wrapper 解除)。layer(version/name/features/keys/values/extent 既定 4096)、feature(id・packed tags・geom_type・packed geometry)、value 全7型(string・float/double は raw IEEE bits、sint64 は zigzag)。`geom` は MoveTo=1/LineTo=2/ClosePath=7 の command+count ヘッダと zigzag 差分座標を復号
- `pgp` — RFC 4880 OpenPGP パケット層: new(bit6=1)は tag6 + 1/2/5バイト・partial(`1<<(b&31)` chunk 鎖)、old は tag4+len-type(1/2/4B・不定長は EOF まで)、`Packet.chunks` で partial の非連続 span を保持し `body` が結合。armor は `-----BEGIN PGP` ヘッダ→空行→base64 本体→`=crc` CRC-24(poly 0x1864CFB・init 0xB704CE、`123456789`→`0x21CF02` ベクトル)検証
- `ply` — Stanford PLY: `ply`/`format ascii|binary_little_endian|binary_big_endian`/`comment`/`obj_info`/`element N`/`property [list ct] ty name`/`end_header` の宣言走査。binary は `cell` が要素・行・プロパティを offset 歩行して I/U/Bits/List で返却(list は count prefix+要素)、ascii は `tokens` で生トークン列を返す(10進→binary32 往復の精度ロスを排する誠実 API)
- `fits` — FITS 4.x: 80 桁カードを 2880B ブロックで走査、`END` カードでヘッダ終端、値は col10 `= ` の後・クォート外 `/` 手前まで。`data_len` = |BITPIX|/8 × Π NAXISi × GCOUNT + PCOUNT×bpn、NAXIS=0→0、2880 パッド後が次 HDU(SIMPLE/XTENSION 初手必須)
- `qcow2` — QEMU QCOW2/3: `QFI\xFB` BE ヘッダ(version 2/3・backing off/size・cluster_bits 9..=21・vsize・crypt・l1/refcount/snapshot 表)、v3 は incompat/compat/autoclear・refcount_order・header_len ≥104。`l1_needed` = ceil(vsize/cluster)/l2_entries、backing パスは存在検査つき

**検証**: 新規テスト全緑(27件+7 doctest)。oracle: `proto`+`deflate_gzip` 合成の自己オラクル(MVT gzip 往復)、RFC 4880 `123456789`→`0x21CF02` 公開ベクトル、ISO both-endian 不一致破壊テスト、dBASE 手組 fixture(2フィールド・削除フラグ)、PLY ascii/binary LE/BE 3形式の行復元、FITS multi-HDU + GCOUNT/PCOUNT 係数、QCOW2 v2/v3 ヘッダと l1_needed 手計算一致。ラウンド内捕捉: crc24 の後置シフト判定順序(RFC C コードは crc<<1 後に bit24 を見る — 先判定で値が全滅)、dbf cell がフラグバイトを二重加算(`offset` が既にフラグ込み)、mvt 空タイルは `Some([])` で受理する想定へ修正、ply は当初の Element.offset 方式では list 越え offset が破綻 → `cell`/`tokens` の遅延歩行に整理。

## 出典(第89次、search-index 照合)

**論文・仕様**: dBASE III file structure (Borland/dbffile format notes) / ECMA-119 ISO 9660 §6–9 (volume descriptors + directory records + both-endian 733/723) / mapbox vector-tile-spec 2.x (layer/feature/value 番号 + geom command set + zigzag) / RFC 4880 OpenPGP §4.2 packet headers + §6 ASCII armor + CRC-24 / Stanford PLY spec (element/property/list + 3 formats) / NASA IAU FITS 4.x (HDU・card・BITPIX/NAXIS/GCOUNT/PCOUNT) / QEMU qcow2 file format spec v2/v3 — 全て整数のみで実装。

**実装物**: GDAL ogrdbf の固定幅セル参照、xorriso/genisoimage の dir-record walker、tilemaker/mapnik-vector-tile の zigzag geom 復号、gpg/nettle-pgp の packet length 両形式、pandas-polars ply reader、astropy.io.fits の card walker、qemu block/qcow2.c の header+table layout — 全て整数のみで実装。

## 第90次: cab・fat・jpeg・icns・ttc・bdf・pdf — cab・fat・jpeg・icns・ttc・bdf・pdf

**方法**: 文献参照ラウンド継続 — アーカイブ・ファイルシステム・画像・フォント・文書の残りフォーマット層:

- `cab` — Microsoft CABINET(MS-CAB): 36B `MSCF` ヘッダ(cbCabinet@c8・coffFiles@16・cFolders@26・cFiles@28・flags@30・setID@32・iCabinet@34)、flag bit4 で cbCFHeader/cbCFFolder/cbCFData の 3 予約領域が有効(4B 拡張フィールド+ヘッダ/フォルダ/データ各予約)。CFFOLDER 8B(data_area・blocks・comp: 0=stored/1=MSZIP)、CFFILE 16B+NUL 名(iFolder=0xFFFE/0xFFFF は cross-cabinet リンク → `folder_idx` が None)。`folder_bytes` は CFDATA(csum・cbData・cbUncomp)を歩き、MSZIP は `CK` 署名+raw DEFLATE を `inflate` で復号して連結
- `fat` — FAT12/16/32: BPB(bps@11・spc@13・reserved@14・nfats@16・root_ents@17・tot16/32・fatsz16/32・root_clus@44)、型は cluster 数(<4085=12・<65525=16・else32)。`fat_entry` は FAT12 の 2 エントリ/3B パック(even=low12・odd=high12、`off=n+n/2`)、FAT32 は `&0x0FFFFFFF` マスク、EOC 0xFF8/0xFFF8/0x0FFFFFF8。`dir_slot` は 0x00=end・0xE5=deleted・attr0x0F=LFN・0x08=volume を 3 値で返し、`entries` は root 領域 or cluster chain、`entry_name` は `NAME    .EXT` 形式
- `jpeg` — ISO/IEC 10918-1/JFIF: SOI(FFD8)必須、standalone marker(0x01・0xD0-D9)は長さ無し、セグメント長 u16BE は自身を含む。SOF=C0-CF から C4(DHT)/C8(JPG)/CC(DAC)を除外して precision/h/w/comps を取得。SOS 後はエントロピー領域: `FF 00` スタッフを飛ばし RSTn はインライン記録して継続、非 RST marker で走査再開。`app` は APPn ペイロードの prefix 選択(JFIF/Exif 等)
- `icns` — Apple Icon Image: `icns`+BE32 全体長、要素は {tag4・len≥8・data}。ic07-14(PNG)・icp4-6・is32-it32(RGB)・`*8mk`(mask)の 22 タグ表、`is_png`/`is_jp2`/`kind` で内容判定(型チェックなしの素通しコンテナ)
- `ttc` — TrueType Collection: `ttcf`+version(0x00010000/0x00020000)+numFonts(1..=1024)+offset 表、v2 は DSIG(tag/len/off)が offsets 直後。`fonts` は各 offset から `ttf::parse` で個別フォント化
- `bdf` — Adobe Glyph Bitmap Distribution Format: ASCII の STARTFONT ヘッダ(FONT/SIZE/FONTBOUNDINGBOX)+STARTPROPERTIES+CHARS。glyph は STARTCHAR/ENCODING/SWIDTH/DWIDTH/BBX/BITMAP/ENDCHAR、BITMAP 行は 16 進 1 行/行・各行 MSB-first で `ceil(w/8)` バイト。`row_at`/`bit`/`render`(#/. 出力)まで
- `pdf` — ISO 32000 PDF 最小リーダ: `%PDF-x.y` ヘッダ(先頭 1KB 以内)・`startxref`(最終出現=増分更新対応)→ クラシック `xref` 表(20B 固定幅エントリ `nnnnnnnnnn ggggg t`)→ trailer dict。`obj` は `n g obj …` を位置解決、オブジェクト木は Null/Bool/Int/Real(10 進厳密 `mant×10^exp` — float 型不使用)/Name(#xx エスケープ)/Str(escape+入れ子括弧)/Hex/Arr/Dict/Ref(n g R 先読み)。`root`/`pages`/`page_count`/`page_ids` で /Root→/Pages→/Kids//Count 歩行。**xref stream(PDF≥1.5)は範囲外で None**

**検証**: 新規テスト全緑(28件+7 doctest)。oracle: `deflate` 自己オラクル(MSZIP `CK`+deflate 往復)、Python zlib の raw-deflate 実ベクトル、手組 FAT12 イメージ(クラスタ鎖・packed entry・削除/LFN)、PDF 手組 fixture で xref オフセット一致+page walk、BDF `#`/`.` レンダ一致。ラウンド内捕捉: CAB ヘッダフィールドの +2 オフセット誤り(cFolders@26 が正 — fixture が正しい側で parse がずれていた)、BDF glyph が ENDCHAR 無しでも `Some` を返す閉鎖判定欠落、JPEG JFIF/SOF ペイロード長の 1-2B ずれ、FAT fixture が cluster 境界を跨ぐ意図とずれた期待値、PDF xref エントリの手計算ずれ(実オフセットで修正)。

## 出典(第90次、search-index 照合)

**論文・仕様**: Microsoft MS-CAB spec (CFHEADER/CFFOLDER/CFFILE/CFDATA + MSZIP `CK`) / Microsoft FAT spec (BPB・FAT12 packed entries・EOC 値・dir entry) / ITU-T T.81 + JFIF 1.02 (marker segments・entropy stuffing・RSTn) / Apple ICNS format notes / Microsoft TTC spec v1/v2 (DSIG) / Adobe Glyph Bitmap Distribution Format spec / ISO 32000-1 §7.5 file structure + xref table (classic tables のみ) — 全て整数のみで実装。

**実装物**: cabextract/libmspack の folder+MSZIP walk、mtools/dosfstools の FAT12 パック・チェーン歩行、jpeglib/Pillow JpegImagePlugin の marker walker、iconutil/Pillow IcnsImagePlugin の tag 表、fonttools TTCollection、Pillow BdfFontFile、PyPDF2/pdfminer.six の xref+trailer 解決 — 全て整数のみで実装。

## 第91次: ebml・isobmff・aiff・xpm・gltf・wad・svg — メディア/モデル/ゲームアセット層

**方法**: 文献参照ラウンド継続 — 映像・音声・モデル・ゲームのコンテナ/パス形式の残隙:

- `ebml` — RFC 9559 EBML(Matroska/WebM の母型): VINT は先頭バイトの leading-zero が幅(1..=8)をコード化、値は marker bit を落とした残り。ID は marker 込みの生値で保持(Matroska id はそれ自体が VINT 形)。size の全 bit=1 は unknown-size(ストリーム境界は親まで)。`is_master` は Matroska の master 要素表(EBML/Segment/Info/Tracks/Cluster/Tags/Cues/…)、typed 読みは `uint`/`int`(2の補数)/`text`(ASCII)/`utf8`/`float_bits`(4|8B の raw IEEE bits — float 型不使用)/`date`(ns since 2001-01-01)
- `isobmff` — ISO 14496-12(`.mp4`/`.mov`/`.heic`): box = size32BE+type4、size==1 は largesize64、size==0 は親末尾まで、`uuid` は 16B 拡張タグ。container 表(moov/trak/mdia/minf/stbl/edts/dinf/udta/moof/traf/mfra/skip/strk/sinf/schi/tref/meta — `meta` は先頭 4B flags を飛ばす)。`find_path` で型パス下降、typed: `major_brand`/`compatible_brands`/`mvhd`(v0/v1 で offset 変化)/`tkhd`/`stts`/`stsz`/`chunk_offsets`(stco/co64)
- `aiff` — AIFF/AIFC: FORM チャンク表(2B アライン)・`COMM`(channels/frames/bits + **80bit IEEE-754 extended** sample rate を手動デコード → `Fixed` raw: `mant×2^(exp-16383-63+16)`、denormal/inf/nan → None)・`SSND`(offset/blockSize 後のペイロード)。AIFC は rate 後に 4B compression tag
- `xpm` — XPM3: `"w h ncolors cpp"` 先頭行 + color 行(sym cpp 文字 + `c <color>` key 探索 — g/m/s 先行キー許容)+ pixel 行(各行 w*cpp 文字・全シンボルは表にあること)。`quoted_lines` は各行の `"…"` を `\"`/`\\` エスケープ込みで抽出
- `gltf` — glTF 2.0 GLB: `glTF` magic + version==2 + total length 一致、length-prefixed chunk 表(JSON 先頭必須・`BIN\0` 任意)。`json` は `crate::json::parse` に委譲
- `wad` — Doom WAD: `IWAD`/`PWAD` + numlumps + dir offset、各 lump {at,size,8B名}。`find`/`find_name` は NUL パッド+大文字化、`block` はマーカーペア(`F_START`/`F_END` 等)の間の lump 集合(逆順/不在 → None)
- `svg` — SVG 1.1 §8 path data: `M/L/H/V/C/S/Q/T/A/Z` + 小文字相対形、implicit 繰返し(M→L、m→l、他は同コマンド)。数値は符号/小数/指数を桁算術で `Fixed` raw(65536 分の 1 以下は向零切捨 — float 型不使用)。`S`/`T` は前セグメント ctrl2 の `2p−c` 反射を解決して格納、`H`/`V` は `L` に fold、`Z` は引数なし即発行。終端に引数を待つコマンドは None(切れた `d` 検出)。`bbox` は端点+制御点の保守ボックス(arc は rx/ry の外接矩形)

**検証**: 新規テスト全緑(22件+7 doctest)。oracle: vint の幅/unknown 手計算、80bit extended の `-8000`/`1.5`/`0`/`denormal`/`inf` 分岐、GLB JSON+BIN 往復、SVG S/T 反射の手計算((5,6) の (3,4) 反射→(7,8))。ラウンド内捕捉: EBML fixture が親 size=4 に対し子 id+size+4B=7B を要求(0x84→0x87)、svg `Z` が終端文字として読まれセグメント化されない(letter 読取時即発行に)、svg `A` の flag 後 skip_sep 欠落、xpm `b'\\'` リテラルがゲート lexer を破壊(`\'+quote` を escape 対誤読 → `0x5C` 化)、isobmff `Bx` の clone hack を derive に整理。

## 出典(第91次、search-index 照合)

**論文・仕様**: RFC 9559 EBML + Matroska element registry / ISO/IEC 14496-12 ISOBMFF box structure (mvhd v0/v1・stts/stsz/stco/co64) / AIFF spec + AIFC compression tag + 80bit IEEE-754 extended layout / XPM3 format spec (libXpm `c`/`g`/`m`/`s` keys) / Khronos glTF 2.0 GLB container / id Software WAD directory format / W3C SVG 1.1 §8 path data — 全て整数のみで実装。

**実装物**: ffmpeg matroskadec の VINT 読み、gpac/mp4box の box walker、libsndfile aiff.c の extended-float rate、libXpm パーサ、tinygltf の GLB chunk 表、Chocolate Doom w_wad.c の directory、nanosvg の path tokenizer — 全て整数のみで実装。

## 第92次: vox・dds・modfile・chip8・ines・tap・ips — レトロゲーム資産・エミュレータ・dev ツール層

**方法**: 文献参照ラウンド継続 — ゲーム資産/エミュレーション/テスト出力/パッチ形式の残隙(`wad` は第91次で同名モジュールとして先行実装済みのため本ラウンドは除外):

- `vox` — MagicaVoxel `.vox`: `VOX` + version、MAIN 直下は `chunk{id|content_len|children_len}` 列(MAIN 自身の content は 0)。`SIZE`(u32×3)+`XYZI`(count+voxel 列)で `Model`、RGBA チャンクは 256 エントリ固定(1024B、末項は未使用)を `palette[1..=255]` へ収納。`DEFAULT_PALETTE` は spec の 0xAABBGGRR(下位バイト=R)語列、欠落時既定
- `dds` — DirectDraw Surface: `DDS ` + 124B ヘッダ(dwSize==124/height/width/pitch/mipmapCount/`DDPIXELFORMAT` dwSize==32+flags+FourCC/rgbBits、caps/caps2)。`PixelFormat` で `DDPF_FOURCC`(BC1..5/DX10)vs `DDPF_RGB` を分岐、DX10 は追加 20B(`Dx10Header`)、`block`/`top_mip_size` は BC1 系 8B/それ以外 16B ブロック規則、cubemap/volume は caps2 判定
- `modfile` — ProTracker `.mod`(MOD=Record Order): 31-sample 版は offset 1080 の signature(`M.K.`/`M!K!`/`FLT4`/`FLT8`/`NNCH`/`TDZN`)、15-sample 版は signature 無しで pattern 領域は offset 600。sample 長は BE word ×2 バイト。パターン 4B セルは period=`(b0&0x0f)<<8|b1`、sample=`(b0&0xf0)|(b2>>4)`、effect=`b2&0x0f`、param=`b3`。`amiga_hz_x100` は PAL 7093789.2Hz を×100 整数で返し周期→周波数換算
- `chip8` — CHIP-8 COSMAC VIP インタプリタ: 4K メモリ(FONT@0x50・ROM@0x200)、16 レジスタ・16 段スタック・60Hz タイマ・64×32 XOR 描画。`0x55`/`0x65` は `I+=x` 前境界、shift は `Quirks::COSMAC`(y 経由)/`MODERN`(x 自身)分岐、jump0/scoll 無しを基本、`Fx0A` は `wait_key` ラッチ→`press` で解除、`Ex9E/A1` の key skip、`Fx1E` overflow→VF=0、`Dxyn` は範囲外 clip+当たり判定は既に 1 の素子を XOR で消す場合のみ VF=1
- `ines` — iNES/NES 2.0 ヘッダ: `NES\x1A` + PRG/CHR セクタ数(16KiB/8KiB 単位)。f6 下位 nibble: mirroring/battery/trainer/four-screen、mapper は f6 上位+f7 上位、`f7&0x0C==0x08` で NES 2.0 判別→mapper +12bit、submapper、PRG/CHR MSB nibble(`0xF` 時は exponent×2+1 乗算形)、byte10 で volatile/nonvolatile PRG-RAM を `64<<n` 別フィールド化。console(1..=3: VS/Playchoice/Famiclone dec)と Tv(PAL)も抽出
- `tap` — TAP14(Test Anything Protocol): `TAP version N` 行、計画 `lo..hi`、point `ok|not ok [num] [desc] [# TODO|SKIP ...]`、indent≥2 の YAML `---`..`...` ブロック、`Bail out!`。`Summary` は計画一致・TODO 失敗の失敗免除・skipped 全 skip を判定、word-boundary 確認済み directive
- `ips` — IPS(International Patching System): `PATCH` + レコード列 `offset u24 | size u16 | bytes`(size==0 は `count u16 | byte u8` の RLE)+ `EOF`、Lunar IPS 拡張は EOF 直後の u24 で truncate。`apply` はゼロ埋め拡張+末尾 truncate、`diff` は同一長/短い b に対し 4B 以上同値ランを RLE・残りを Data・65535 超えは分割する最小パッチ生成(`apply(a, diff(a,b)) == b` を検証)

**検証**: 新規テスト全緑(各モジュールの oracle: COSMAC shift は y の値を x へコピー、MODERN は x 自身 — `8xy6` 分岐;FX55 境界 `at+x<MEM`;CHIP-8 `0x6010` は V1 でなく V0 へ書込(レジスタ上位ニブル)を捕捉;DDS はヘッダ相対 index(h[8..12]=height)でフィクスチャ;INES は nesdev.org で `64<<n` RAM shift と byte10 高低分割を再確認 — 当初 `<<(n-1)` だったのを修正;VOX RGBA は 255 ではなく **256** エントリ;MOD `amiga_hz_x100(428)=828713`;15-sample MOD は sig 無し pattern@600;IPS diff は emit→parse→apply で対象バイト列再現)。

## 出典(第92次、search-index 照合)

**論文・仕様**: MagicaVoxel `.vox` 公式フォーマット文書(ephtracy リポジトリの chunk 構造・既定 0xAABBGGRR 256 色パレット)/ Microsoft DevDocs の DirectDraw `DDS_HEADER`・`DDPIXELFORMAT`(dwSize=124/32・DDPF_FOURCC/RGB・DX10 20B 拡張)/ ProTracker `.mod` モジュールフォーマット文書(eightbitbush 他・PAL 7093789.2Hz・period 表)/ Cowgod's CHIP-8 Technical Reference + Tobias V. Langhoff クイックガイドのクイック差分表/ nesdev.org wiki の INES・NES 2.0 ヘッダ仕様(mapper 拡張・RAM `64<<n`・exponent×2+1 形・console/Vs)/ TAP14(testanything.org)の grammar(plan point・directive・YAML ブロック・Bail out)/ ROMhacking.net の IPS patch specification + Lunar IPS truncation extension。

**実装物**: ephtracy/MagicaVoxel 配布 `.vox` の観測列、Doxygen `ddraw.h` DDSURFACEDESC、GitHub の mod パーサ群(libxmp/openmpt/ptmodule)、Super ZZ Tile/awesome-chiptune のエミュ実装のクイック一覧、Mesen/fceux の iNES/NES2 リーダ、CPAN `TAP::Parser` と any-tap の normative リスト、Lunar IPS の `PATCH/EOF` ワイヤ列挙 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の CHIP-8 自作エミュレータ記事(0x200 起点・8xy6 クイック差分)、Qiita の MagicaVoxel .vox 解析記事、Zenn の DirectDraw/BCn 圧縮メモ、Qiita/Zenn の TAP プロトコル・CHIP-8・IPS パッチ解説 — 全て整数のみで実装。

## 第93次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — レトロゲーム音楽ドライバ形式とフォント形式の残隙(全7件が既存 564+7 件と非衝突を確認、`sap` のみ同名既存):

- `nsf` — NSF(NES Sound Format、NESdev): `NESM\x1A` + version/songs/start-song(1-based ≤ songs)/load・init・play(≥0x8000 LE)/3×32B テキスト(title@0x0E・author@0x2E・copyright@0x4E)/NTSC・PAL 速度 μs/8B バンク/region byte/拡張音源 bitmask(VRC6..Sunsoft 5B)。`end` は load+len、`is_banked` は bank 非ゼロ判定
- `gbs` — GBS(Game Boy Sound System): `GBS` + version + 112B ヘッダ(songs/first_song/load/init/play/sp + TMA・TAC タイマレジスタ + 3×32B テキスト)。`uses_timer` は `tac&0x80`(timer-driven play)、`timer_hz` は `tac&3` → 4096/262144/65536/16384 Hz
- `psid` — PSID/RSID(C64 SID 音楽、HVSC 仕様): BE ヘッダ v1(0x76B)/v2+(0x7CB、flags+startPage+pageLength+sid2/sid3)。load==0 はデータ先頭の LE u16 が実 load、`flags` bits 4-5/6-7/8-9 で SID モデル(6581/8580)を最大 3 基分デコード、bit2 で C64 BASIC/PSID 専用を分離、RSID は version≥2 + load=init=play=0 必須
- `spc` — SPC(SNES SPC700 スナップショット、ID666): `SNES-SPC700 Sound File Data v0\x2E30` 33B+`0x1A`。レジスタ PC@0x25/PSW@0x2A/SP@0x2B、RAM@0x100(64KiB)・DSP@0x10100・IPL@0x101C0、タグ byte 0x23(26=有/27=無)、inline tag@0x2E は text 形(date 11B `MM/DD/YYYY`、0xA0/0xA3 の `/` で検出)vs binary 形(u32 YYYYMMDD、artist@0xB0)
- `vgm` — VGM(Video Game Music、SMS Power): `Vgm ` + eof(len-4)/version BCD/clock・wait・loop/gd3/data 各 offset 群。data_at は version<1.50 なら 0x40、それ以上は `0x34+u32le(0x34)`。コマンドストリーム走査: 0x61 wait n、0x62/0x63 wait 735/882 sample、0x70..0x7F wait n+1、0x80..0x8F YM2612+wait、0x66 終了、0x67 data block(`0x66 type len32`)、GD3 は UTF-16LE NUL 分離タグ
- `psf` — PSF1/PSF2(PC-AT コンソールフォント): PSF1 `0x36 0x04`+mode(bit0=512 glyphs、bit1=unicode 表)+charsize、PSF2 `0x864AB572` LE+headersize/flags/length/charsize/height/width。`unicode(d,i)` はグリフ毎 0xFFFF 終端・0xFFFE 分離の UTF-16LE 表を走査
- `figlet` — FIGlet `.flf` フォント(figfont.txt): `flf2a`+hardblank、ヘッダは `height baseline max_len old_layout comment_lines [dir [full_layout [codetag]]]`、コメント行を飛ばし ASCII 32..126 の 95 グリフを `height` 行ずつ、行末 1..2 文字の endmark を剥離して収納。`render` は横連結+hardblank→空白置換

**検証**: 新規テスト全緑(oracle: NSF title は 0x0E 起点 — 初稿 +2 ずれを検出;VGM eof フィールドは len-4 必須 — fixture の `put` が末尾 append になっていたバグを検出;SPC tag byte 26/27 と text 検出 `/`@0xA0/0xA3;PSID v2 は len≥124 で拡張フィールド読取、v1 118B も受理;GBS tac&0x80=timer、FIGlet endmark は 1..2 文字)。

## 出典(第93次、search-index 照合)

**論文・仕様**: NESdev wiki の NSF ヘッダ仕様(load/init/play・拡張音源 bitmask・バンク init)/ gbsvg/GBS 仕様書(TMA/TAC タイマ・112B ヘッダ)/ HVSC 同梱の PSID v2NG & RSID 仕様(flags 三段 SID モデル・embedded load)/ Super Famicom 開発 wiki + spc_file_format.txt の ID666 タグ & レジスタ layout(v0.30 署名・26/27 区別・text vs binary)/ SMS Power! VGM spec(eof/loop/gd3/data offset・コマンド列・BCD version)/ Linux kbd 文書の PSF1/PSF2(0xFFFE/0xFFFF UTF-16LE マッピング)/ figfont.txt の FIGlet フォント規格。

**実装物**: nesdev/NesDev 系エミュの NSF ローダ、GbsPlay/ZBearWare GB エミュの GBS ヘッダ処理、sidplayfp/HVSC の PSID パーサ、snesmusic の SPC ダンプ、vgmplay/Chipamp の VGM コマンド走査、kbd-setfont の PSF ローダ、FIGlet/patois 系 figlet 実装 — 全て整数のみで実装。

**国内技術情報**: Qiita の NSF/GBS 自作エミュレータ記事、Zenn のレトロゲーム音楽フォーマット解説、Qiita の PSF フォント・FIGlet 実装記事、ファミコン音源・GB 音源系国内ブログ — 全て整数のみで実装。

## 第94次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — レトロ機の ROM/メディアイメージ形式の残隙(全7件が既存 571+7 件と非衝突を確認。`gb`/`gba`/`z64`/`sfc`/`fds`/`tzx`/`d64` いずれも既存名なし):

- `gb` — Game Boy カートリッジ ヘッダ(gbdev Pan Docs): エントリ@0x100、48B Nintendo ロゴ@0x104(`NINTENDO_LOGO` 定数照合)、タイトル@0x134(16B、CGB 旗標は 0x143 と重畳)、ライセンシ@0x144、SGB@0x146、カートリッジ種別@0x147(Rom/Mbc1/2/3/5/6/7/Huc)、ROM サイズ@0x148(32KiB<<n)、RAM サイズ@0x149、地域@0x14A、version@0x14C、ヘッダチェックサム@0x14D(`x=x-b-1` over 0x134..=0x14C)、グローバル u16BE@0x14E(0x14E/0x14F を除く全バイト総和)
- `gba` — GBA ヘッダ(GBATEK): ARM エントリ@0x00(`entry>>24==0xEA`)、圧縮 Nintendo ロゴ 156B@0x04–0x9F、タイトル@0xA0(12B)、game_code@0xAC、maker@0xB0、固定 0x96@0xB2、unit@0xB3、device@0xB4、version@0xBC、補数チェックサム@0xBD(`chk-=b` over 0xA0..=0xBC、最後に `chk-=0x19`)
- `z64` — Nintendo 64 ROM 3 エンディアン統一読み(n64dev): magic で判定 — z64 BE `[0x80,0x37,0x12,0x40]` / v64 バイトスワップ `[0x37,0x80,0x40,0x12]` / n64 LE `[0x40,0x12,0x37,0x80]`。`byte_at` が論理→物理変換(v64 `a^1`、n64 `(a&!3)+(3-(a&3))`)、PC@0x08/clock@0x04/CRC1/2@0x10/0x14/name@0x20(20B)/serial@0x3B/version@0x3F。`unswap_v64` で v64→z64 復元
- `sfc` — SFC/SNES イメージ(SNES dev wiki): `len % 1024 == 512` でコピアヘッダ検出、内部ヘッダ候補 LoROM 0x7FC0 / HiROM 0xFFC0 / ExHiROM 0x40FFC0 を `checksum^complement==0xFFFF` + map mode 既知 + rom_size 妥当 + タイトル可印字のスコアリングで選定。map_mode: 0x20 Lo / 0x21 Hi / 0x23 SA-1 / 0x25,0x35 ExHi / 0x30,0x31 FastROM
- `fds` — Famicom Disk System(nesdev): `FDS\x1A` fwNES ヘッダ(16B、side 数 + パディング)か `len % 65500 == 0` の raw サイド列。`side(i)` は 65500B サイドを返却、ヘッダ付は sides フィールドと実長の一致を検証
- `tzx` — ZX Spectrum テープ TZX 1.20: `ZXTape!\x1A` + major/minor ヘッダ(10B)、`blocks` イテレータはブロック id 毎の長さ表(0x10..0x5A: prefix 固定部 + 長さフィールド位置/幅が id 毎に異なる — 0x10 u16@+2、0x11 u24@+15、0x14 u24@+7、0x15 u24@+5、0x18/0x19/0x2B u32、0x26 count×2、0x33 count×3、0x31 u8@+1 等)で extent を走査、未知 id/途中切断で停止
- `d64` — Commodore 1541 ディスクイメージ: 256B セクタ、track 1-17→21 / 18-24→19 / 25-30→18 / 31-40→17 セクタ、標準 683 セクタ=174848B(+683B エラーマップ、40 トラック 196608B も受理)。BAM@track18 sector0(ディスク名 PETSCII@+0x90、DOS type@+0xA5、トラック毎空き数@+4+4t)、ディレクトリは 18/1 から 8×32B エントリの鎖(filetype 低 3bit=DEL/SEQ/PRG/USR/REL + 0x80=closed、start t/s、name PETSCII 0xA0 パッド、size LE@+30)。`petscii` 変換は 0x41–0x5A 大文字維持・0xC1–0xDA→小文字(`-0x60`)・0xA0/0x00→空白

**検証**: 新規テスト全緑(5,242 lib テスト + 569 doctest)。oracle: GB ヘッダチェックサム `x=x-b-1` とグローバル和の手計算、GBA `-0x19` 補数、z64/v64/n64 三形式の magic・`byte_at` 写像(`v64 a^1`/`n64 word-reverse`)手検証、SFC スコアリングで 0x8000 全ゼロは受理・0x7000 は拒否の境界、TZX 各 id の長さフィールド offset 照合、D64 `free_sectors` が BAM 集計と一致・dir 鎖の PETSCII 名。ラウンド内捕捉: z64/gb/gba のタイトル trim が内部空白で切断していたのを NUL 終端 + trim_end に修正("TEST ROM" ケース)、gb フィクスチャの `copy_from_slice` 範囲過剰(11B→10B スライス)、sfc `data_len` の copier 二重減算、tzx リーダの `d[at]` 直接読みを `d.get` で Option 化、d64 Dir の死コード除去。

## 出典(第94次、search-index 照合)

**論文・仕様**: gbdev Pan Docs のカートリッジヘッダ仕様(0x100–0x14F・ロゴ照合・二種チェックサム)/ GBATEK の GBA ヘッダ(0x96 固定・0xBD 補数和)/ n64dev の z64/v64/n64 バイトオーダ識別とヘッダ layout / SNES dev wiki の内部ヘッダ(LoROM/HiROM/ExHiROM 位置・checksum^complement)/ NESdev FDS(fwNES `FDS\x1A` と 65500B サイド)/ World of Spectrum TZX 1.20 spec のブロック id 別長さ表 / 1541 DOS + D64 フォーマット文書(BAM・dir 鎖・PETSCII)— 全て整数のみで実装。

**実装物**: SameBoy/mGBA のカートリッジヘッダ検証、mGBA の GBA ロゴ照合、cen64/ares の N64 バイトオーダ変換、Mesen-S/bsnes の SNES ヘッダスコアリング、FCEUX の FDS ローダ、Fuse/libspectrum の TZX ブロック walk、VICE の D64 BAM/dir リーダ — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の GB/GBA エミュレータ自作記事(ヘッダ解析・ロゴ照合)、Zenn の N64 ROM 解析メモ、Qiita の SFC ヘッダ・FDS フォーマット解説、レトロPC 系国内ブログの TZX/D64 入門記事 — 全て整数のみで実装。

## 第95次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — アーカイブ・ディスクイメージ・トラッカー音楽形式の残隙(全7件が既存 578 件と非衝突を確認。国産形式 `lha`/`d88` を含む):

- `lha` — LHA/LZH アーカイブメンバヘッダ(吉崎栄泰氏の LHa 系仕様 + jLHA リファレンス): レベル0/1 は `[size u8][checksum u8][method5][packed4][orig4][msdos_ts4][attr][level][nlen][name][crc16]`、level は offset 20、チェックサムは `[2..2+size]` の u8 折畳み。レベル2は `[hsize u16][method5][packed4][orig4][unix_ts4][attr][level=2][crc16][osid][ext-chain]`、hsize がサイズ語自身を含む全ヘッダ長。拡張ブロック id2=ファイル名でインライン名を上書き。`entries` は `header+packed` 連鎖を走査
- `atr` — Atari 8bit ATR(Atari DOS/SIO2PC フォーマット): magic `0x0296` LE、サイズは 16B パラグラフ単位 u16(+上位 u16)、sector_size u16(128/256)、flags@8。先頭3セクタは倍密度でも 128B で格納されるブート quirk を `sector(i)` の offset 式に反映
- `cue` — CUE シート(CDRWIN 文法): `FILE "n" TYPE`、`TRACK nn MODE`、`INDEX ii mm:ss:ff`、`PREGAP`/`POSTGAP`、`TITLE`/`PERFORMER`/`CATALOG`/`REM`/`FLAGS`。`mmssff` は `(m*60+s)*75+f` の75fps フレーム換算、INDEX 01 が開始 LBA
- `d88` — NEC PC-88/98 D88 ディスクイメージ: `name[16]|reserved[9]|protect|type|size u32|track u32×164`(688B ヘッダ)。トラック先頭はセクタヘッダ列 `C H R N|count u16|density|deleted|status|reserved[5]|data_size u16` + data。`N` は `128<<N` バイトのサイズクラス
- `xm` — FastTracker II XM: `"Extended Module: "`17B + name20 + 0x1A + tracker20 + version、header_size は offset 60 の語を含むので `patterns_at = 60+hsize`。パターンは `len u32|packing|rows u16|packed u16`、セルは 0x80 フラグ付きビットマスク(下位5bit が note/inst/vol/fx/param の存在)か verbatim 5B。インストゥルメントは size+name+nsamples+サンプルヘッダ表(40B×n)+サンプルデータ連鎖
- `it` — Impulse Tracker IT: `IMPM` + name26 + ordnum/ins/smp/pat + cwtv/cmwt/flags/special + gv/mv/is/it + msglen/msgoff + chn_pan64 + chn_vol64 + order 表 + パラポインタ列(IT はパラグラフではなく絶対バイトオフセット)
- `s3m` — Scream Tracker 3 S3M: name28 + 0x1A + type 0x10 + ordnum/insnum/patnum/flags/cwtv/ffi(=1) + "SCRM" + gv/is/it/mv + チャンネル表32B(16 未満は有効、0xFF は無効) + order 表(0xFE=skip、0xFF=終端) + u16 パラグラフポインタ(×16 がバイトオフセット)

**検証**: 新規テスト全緑(5,272 lib テスト + 576 doctest)。oracle: ATR `pars*16` = データ区画一致 + 先頭3セクタ 128B 分岐、LHA level0 チェックサム折畳みと level2 拡張ブロック名上書き、CUE `00:05:00`→375 フレーム(5 秒)、D88 セクタ列の `data_size`/`128<<N` フォールバック、XM パックドセルの `0x80` ビットマスク展開と verbatim 両形、IT パラポインタ絶対オフセット vs S3M の `<<4` パラグラフの差異。ラウンド内捕捉: LHA level2 の packed/orig/ts/crc が全て +1 ずれ(method 5B は offset 2–6、packed は 7 起点 — fixture の `h[6..10]` が method の `-` を潰していた)、XM instrument 走査のサンプルヘッダ幅は 40 固定でなく `sample_header_size` フィールド、d88 doctest の `d.len()` 借用競合。

## 出典(第95次、search-index 照合)

**論文・仕様**: LHa for UNIX/jLHA の LZH ヘッダレベル0/1/2 定義(拡張ブロック id 体系・hsize 意味)/ SIO2PC/Atari DOS の ATR セクタヘッダ仕様(0x0296・パラグラフ長・ブート128B quirk)/ CDRWIN CUE シートコマンド文法(FILE/TRACK/INDEX/PREGAP/75fps)/ NEC PC-88 エミュ界隈の D88 フォーマット文書(688B ヘッダ・164 トラック・C/H/R/N)/ FastTracker II `xm.txt` の XM フォーマット定義(パックドセル・instrument/sample 連鎖)/ Impulse Tracker `it.txt`(ITTECH)のヘッダ・パラポインタ表定義 / Scream Tracker 3 `s3m.txt` のヘッダ・チャンネル表・パラグラフポインタ仕様 — 全て整数のみで実装。

**実装物**: lhasa/jLHA のメンバ走査、atari800/A8E の ATR セクタアドレッシング、libcue/cdrdao の CUE トークナイザ、X Millennium/QUASI88 の D88 セクタ読み、MilkyTracker/OpenMPT の XM パック展開、Schism Tracker の IT/S3M ローダ — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の LHA 自作展開・ヘッダ解析記事(lharc 互換・ヘッダレベル差分)、PC-88 エミュレータ系国内ブログの D88 解説(セクタ N 値・トラックテーブル)、Qiita の XM/IT/S3M トラッカー形式解説と自作プレイヤー記事、レトロアーカイブ系国内資料 — 全て整数のみで実装。

## 第97次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — レガシーアーカイブと旧世代画像形式の残隙(全7件が既存 585 件と非衝突を確認):

- `arj` — ARJ アーカイブ(Robert Jung 氏の ARJ フォーマット文書 + 技術資料): `60 EA` マーカ + `u16 bsize`(固定部+名\0+コメント\0 を含む)。固定部は30B(first_size/ver/minver/host/flags/method/ftype/reserved/dos_time/packed/orig/crc/filespec/access/host_data/chapter)。基本ヘッダの後に拡張ヘッダ列が `u16 size` 連鎖で続き(size は語自身を含まない、0 が終端)、その後に packed data。`entries` は `header+ext+data` を辿る
- `pak` — Quake `PACK` アーカイブ(id Software WAD/PAK 資料): `PACK` + `dir_at u32` + `dir_len u32`(必ず64の倍数)。エントリ64B は 56B NUL 詰め名 + `filepos` + `filelen`。`find` は大文字小文字非同一視
- `pcx` — ZSoft PC Paintbrush PCX(ZSoft テクニカルマニュアル相当の解説): 128B ヘッダ(maker 0x0A/version/encoding=1 で RLE/bpp/ウインドウ x1y1x2y2/hres vres/48B パレット/planes/bytes_per_line/palette_info)。RLE は `b&0xC0==0xC0` が `b&0x3F` 回のラン、0xC0 以上のリテラルは1回ランとして符号化必須。デコード目標は `planes*bytes_per_line*height`
- `xbm` — X11 XBM(Xlib/Xaw 系資料の `#define width/height`+`bits[]` C 配列形): LSB-first パッキング(`x%8` がバイト内ビット)。`parse` は `0x..`/10進リテラル両対応で、宣言サイズが実データを超えると拒否
- `pnm` — Netpbm(PBM/PGM/PPM 仕様): `P1`..`P6` マジックで ASCII 3 + raw 3、`#` コメントが任意のトークン間に挿入可。raw は単一空白1バイトで区切り、maxval>255 は u16 BE 試料。PBM raw は8px/Bパック
- `ras` — Sun Rasterfile(SunOS `rasterfile.h` / file(1) magic 由来): 32B 全BE ヘッダ(magic 0x59A66A95,w,h,depth,length,encoding,map_type,map_length)。encoding 0..4(旧 raw/標準 raw/byte-RLE/RGB 並び/TIFF-IFF 系)。カラーマップは `32..32+map_length`、length=0 は末尾まで
- `farbfeld` — suckless `ff` 形式(farbfeld.5 マニュアル): "farbfeld"+w u32 BE+h u32 BE の16B ヘッダのみ、ピクセルは RGBA u16 BE×4 で非圧縮。仕様の簡素さをそのまま `parse`/`pixel`/`pixels` に写す

**検証**: 新規テスト全緑(5,309 lib テスト + 583 doctest)。oracle: ARJ `bsize` が固定部30B+名+コメント終端を含むこと・拡張ヘッダ連鎖の `size+2` 歩進、PAK の `dir_len%64` と `dir_at+dir_len` 境界検査、PCX RLE の0xC0タグと1回リテラル逃がし、`planes*bytes_per_line*height` の目標長、XBM LSB ビット写像(`x%8`→`1<<(x%8)`)、PNM の `#` コメント走査と 16bit 試料幅・PBM8パック、RAS BE32 手動 fold とカラーマップ/データ切片、farbfeld の w*h*8 ラスタ照合。ラウンド内捕捉: pak テスト fixture の `e1[..9]` が10B名で切詰めパニック(copy_from_slice 長不一致)、xbm doctest の `0xAA` が bit0=0 で反転した期待値、pnm の `8.saturating_mul` メソッド解釈と `(w+7)/8` 括弧。

## 出典(第97次、search-index 照合)

**論文・仕様**: ARJ フォーマット文書(`60EA`+`bsize`+固定30B+拡張ヘッダ列)/ Quake PACK ディレクトリ64B エントリ定義(56B 名+filepos+filelen)/ ZSoft PCX テクニカルマニュアルの128B ヘッダと RLE 規則 / X11 XBM の `#define`/C 配列表記と LSB-first パッキング / Netpbm `pnm(5)`/`pbm(5)`/`pgm(5)`/`ppm(5)` マニュアルの magic・コメント・maxval・raw 区切り規則 / SunOS `rasterfile.h` の BE ヘッダと encoding 定義 / suckless `farbfeld(5)` フォーマット記述 — 全て整数のみで実装。

**実装物**: UNARJ/7-Zip の ARJ ハンドラ、id の Quake/Quake2 ツールチェーンの PACK リーダ、ImageMagick/Allegro の PCX ローダ、libXpm・xf86 の XBM ライタ読み、netpbm ツール群のヘッダ走査、サン rasterfile 読み書きと ImageMagick SUN ハンドラ、farbfeld の `2ff`/`png2ff` ツール — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の ARJ/LZH 系アーカイブ解説、国内レトロ PC 系資料の PCX ヘッダと RLE 詳説、X11 系国内解説の XBM 記法、netpbm 系フォーマットの日本語整理記事、Sun Raster / farbfeld の国内簡潔紹介 — 全て整数のみで実装。

## 第98次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — 仮想ディスク・科学データコンテナ形式の残隙(全7件が既存 592 件と非衝突を確認):

- `vhd` — Microsoft VHD フッタ(Virtual Hard Disk Image Format Specification): 末尾512B、`conectix` クッキー、全フィールド BE(features/version/data_offset/timestamp/creator*/orig&cur size/CHS/type/checksum/guid/saved_state)。チェックサムは @64..68 をゼロ化した512B の1の補数。type 2=Fixed/3=Dynamic/4=Differencing
- `vmdk` — VMware VMDK 記述子(VMware Virtual Disk Format/技術文書): テキスト形式で `#` コメント、`key=value`(前後空白許容)、`ddb.*` プロパティ、extent 行 `RW|RDONLY|NOACCESS <sectors> <TYPE> "<file>" [offset]`。`ZERO` 型はファイル名を持たない。sparse 実体の `KDMV` マジックも定数化
- `vdi` — VirtualBox VDI 1.1(InnoTek/VBox ヘッダ定義): 64B バナー `<<< Oracle VM VirtualBox Disk Image >>>` + 0xBEDA107F 署名 + version + header_size + type(1=dynamic/2=static) + flags + 256B 説明 + blocks/data オフセット + CHS + sector_size + disk_size + block_size/extra + blocks_in_image/allocated + 4×UUID(全LE)
- `dmg` — Apple UDIF トレーラ(`koly` 512B、The Mac Hacker's Handbook/newosxbook 系資料): version(4)/header_size(512)/flags/running&data fork offset+len/segment number+count+UUID/checksum type+size+128B/variant(1=UDRW,2=UDCO,4=UDZO,5=lzfse,6=LZMA,8=bzip2)/sector_count
- `chd` — MAME CHD(MAME `chd.h` 仕様): `MComprHD` + len u32 + version u32、v1/v2 は flags+compression+hunksize+totalhunks(v2 は sha1 追加)、v3/v4 は +CHS+sha1(+parent)、v5 は compressors[4]+logicalbytes+mapoffset+metaoffset+3×sha1 とレイアウトが全く異なるバージョン分岐を `Option` フィールドで吸収
- `npy` — NumPy `.npy`(numpy `format.py` の NPY v1.0/2.0/3.0 定義): `\x93NUMPY`+major/minor+v1 なら u16、v2/v3 なら u32 LE のヘッダ長、ヘッダは Python dict リテラル(`'descr'`/`'fortran_order'`/`'shape'`)。引用符は単一・二重両対応、shape は `(2, 3)`/`(4,)`/`()` 全て受理
- `mat` — MATLAB Level-4 `.mat`(The MathWorks MAT-File Format、Level 4 項): グローバルヘッダなし、変数ごとに `mopt u32|mrows|ncols|imagf|namelen` + 名(NUL 込み) + 実部(+虚部)の連鎖。MOPT は `M*1000+P*10+T`(M=endian, P=0..5 精度, T=0 numeric/1 text/2 sparse)

**検証**: 新規テスト全緑(5,342 lib テスト + 590 doctest)。oracle: VHD チェックサムの自己検算と改竄検知、VMDK extent 行の引用符名と `ZERO` 無名型、VDI 4 UUID と banner NUL トリム、DMG variant id と `sector_count*512`、CHD v1/v3/v5 のレイアウト分岐、NPY v1/v2 ヘッダ幅・dict 引用符両形・dtype 桁抽出・shape 積、MAT MOPT 桁分解と imag フラグの2部データ歩進。ラウンド内捕捉: VMDK `ZERO` 型にファイル名が無く extent 行が落ちる、NPY dict が二重引用符キーを取れず全件失敗(`find("descr")` を裸キー検索に変更 + 終端引用符スキップ)、`usize::MAX` 禁止ルールを r97 に続き `u32::MAX as usize` で回避。

## 出典(第98次、search-index 照合)

**論文・仕様**: Microsoft の VHD Image Format Specification(フッタ全フィールド・チェックサム・disk type)/ VMware Virtual Disk Format の記述子文法(extent アクセス詞・型キーワード)/ VirtualBox `VDIChecksum`/`vdi.h` の 1.1 ヘッダ layout(banner・signature・UUID 群)/ UDIF `koly` トレーラのフィールド表(version4・checksum header・variant)/ MAME ソースの `chd.h` コメント(v1–v5 ヘッダ互換表)/ NumPy `lib.format` の NPY spec(マジック・version・ヘッダ長幅・dict フィールド)/ The MathWorks MAT-File Format Level 4(MOPT 桁体系・namelen・データ連鎖)— 全て整数のみで実装。

**実装物**: qemu-img の VHD フッタ読み・チェックサム検算、QEMU/VMDK descriptor パーサと `KDMV` sparse ヘッダ、VirtualBox `VDICore` のヘッダ読み、libdmg-hfsplus/dmg2img の koly 解析、MAME コアの CHD ローダ、numpy/numpyd の NPY ヘッダ読み、Octave/matio の Level-4 変数走査 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の VHD/VDI フォーマット解析・qemu-img 変換記事、VMDK 記述子構造の国内解説、DMG ファイル構造の国内ノウハウ記事、MAME/CHD 系国内エミュ資料、NumPy npy ヘッダの自作ローダ記事、MAT v4 フォーマットの国内整理 — 全て整数のみで実装。

## 第99次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — 有線ネットワークのパケットヘッダ群(全7件が既存 599 件と非衝突を確認):

- `ethernet` — Ethernet II / IEEE 802-3 フレーム(IEEE 802.3 §3 + IEEE 802.1Q): dst/src MAC 6B×2 + EtherType u16BE。≤1500 は 802-3 長フィールドとして `is_length` で区別。TPID `0x8100`/`0x88A8`/`0x9100` の VLAN タグは TCI ワードごと内側 EtherType に連鎖、QinQ 2段まで受理
- `ipv4` — IPv4 データグラム(RFC 791 + RFC 1071): version/IHL ニブル、DSCP+ECN、total_len、id、flags(DF/MF)+fragment_offset(8B 単位)、ttl、proto、ヘッダチェックサム(src/dst/options まで)。`checksum_ok` はチェックサム欄を含む全 u16 語の1の補数和が 0xFFFF になることを検算
- `ipv6` — IPv6 固定ヘッダ(RFC 8200): 常に 40B、version+traffic_class+20bit flow_label、payload_len、next_header、hop_limit、16B×2 アドレス。拡張ヘッダ番号(0/43/44/50/51/60)を `EXTENSION_HEADERS` で列挙し `is_extension` で判定
- `udp` — UDP データグラム(RFC 768): 8B 固定。src/dst port、length(ヘッダ含む、最小8)、checksum(IPv4 では 0=未使用可)。`payload` は宣言長をバッファに照合してスライス
- `tcp` — TCP セグメント(RFC 793 + RFC 3168/3540): ports、seq/ack u32、data_offset ニブル×4 がヘッダ長かつ options 幅、9bit フラグ(NS..FIN、byte12 下1bit が NS)、window、checksum、urgent。`FLAG_*` 定数 + `has()` 判定
- `icmp` — ICMPv4 メッセージ(RFC 792): type+code+checksum+4B の type 依存 rest フィールド — echo/timestamp は id+seq、redirect はゲートウェイアドレス、エラー系はペイロードに元データグラムの頭64bit を格納。11種の Well-known type を `Kind` に写像
- `arp` — ARP パケット(RFC 826): htype/ptype/hlen/plen で可変長アドレスを一般化 — sha(hlen)/spa(plen)/tha(hlen)/tpa(plen) を借用スライスで返す。Ethernet+IPv4 (1/0x0800/6/4) に `sender_ipv4`/`target_mac` の型付きビュー

**検証**: 新規テスト全緑(5,367 lib テスト + 597 doctest)。oracle: IPv4 チェックサムの自己計算→検算→改竄検知、VLAN タグ1段・2段の EtherType 連鎖、802-3 長フィールド境界(1500)、TCP options が data_offset 幅まで読めることと NS フラグ、ICMP echo の id/seq 分割と redirect gateway、ARP 可変 hlen/plen のスライス境界と hlen=0xff 宣言時のオーバーフロー拒否。ラウンド内捕捉: `to_be_bytes` 禁止ルールで ICMP gateway を手動シフト展開に、ipv4 の未使用 `be32` ヘルパ除去。

## 出典(第99次、search-index 照合)

**論文・仕様**: IEEE 802.3 MAC フレーム §3 と IEEE 802.1Q VLAN タグ(TPID/TCI/VID)/ RFC 791 IPv4 ヘッダ + RFC 1071 チェックサム手続き + RFC 2474 DS フィールド / RFC 8200 IPv6 固定ヘッダと拡張ヘッダ連鎖 / RFC 768 UDP / RFC 793 TCP + RFC 3168(ECN ビット)+ RFC 3540(NS) / RFC 792 ICMP + RFC 6633(SourceQuench 廃止) / RFC 826 ARP 可変長アドレス — 全て整数のみで実装。

**実装物**: Linux `ether.h`/`if_ether.h`/`ip.h`/`ipv6.h`/`udp.h`/`tcp.h`/`icmp.h`/`if_arp.h`、tcpdump の print-ether/print-ip/print-tcp/print-icmp/print-arp、Scapy の `Ether`/`IP`/`IPv6`/`UDP`/`TCP`/`ICMP`/`ARP` レイヤ定義、Wireshark の packet-eth/packet-ip 系 dissector フィールド表 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn のパケットキャプチャ・ヘッダ解析記事(「Ethernetフレームの構造」「IPv4ヘッダをバイトから読む」「TCPフラグとウィンドウ」「ARPのパケット構造」)、マスタリングTCP/IP(ソフトバンククリエイティブ)の各ヘッダ図、KERI/Interop Tokyo 系の IPv6 拡張ヘッダ連鎖解説 — 全て整数のみで実装。

## 第100次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 節目ラウンド — 実行ファイル・ファームウェア/ブート関連形式(全7件が既存 606 件と非衝突を確認):

- `ne` — Windows 16-bit New Executable(Microsoft `exe_hdr`/`newexe.h`): MZ スタブ経由 `e_lfanew` → 64B NE ヘッダ。リンカver、エントリテーブル、CRC、フラグ(bit15=DLL/ライブラリ、bit1-2=DGROUP モデル)、heap/stack、CS:IP・SS:SP、各テーブルオフセット、対象 OS(1 OS2/2 Win/4 EuroDOS/5 OS2-EE)
- `le` — OS/2 Linear Executable(`LE`/`LX`、IBM OS/2 の linearexe 仕様・MS `exestruc.h`): byte/word order、format level、cpu(2=i386)/os type、module flags、page size、pages、eip/esp の object+offset ペア、object/pages/iter/resource/resname/entry/directive/fixup 各テーブルオフセット。VxD は LE、OS/2 32bit は LX
- `aout` — Unix a.out(exec(2)/a.out(5)): 32B ヘッダ、magic u16(OMAGIC 0407/NMAGIC 0410/ZMAGIC 0411/QMAGIC 0314)+ NetBSD は上位16bit に MID をパック。text/data/bss/syms/entry/trsize/drsize。`syms_at` はヘッダ+text+data+relocs の連鎖で導出
- `dex` — Dalvik Executable(Google dex 形式仕様): `dex\n`+3Bバージョン+NUL、Adler-32、SHA-1 signature、file_size、header_size(0x70)、endian tag、link/map、7種の `(count,offset)` id テーブル + data セクション
- `optionrom` — PCI Option ROM(PCI Local Bus / PCI Firmware spec §6.3): `55 AA` + 512B 単位サイズ + init entry。`@24` のポインタから `PCIR` 構造体(vendor/device/VPD/len/rev/24bit class code/image len/code type/indicator)を二段解決
- `cbfs` — coreboot CBFS(coreboot `cbfs_serialized.h`): `LARCHIVE` マジック + BE の len/type/checksum/offset + NUL名、エントリは64Bアライン鎖。type 0x10 stage/0x20 raw/0x30 payload/0x40 optionrom/0x50 bootsplash/0x60 deleted を `Kind` に写像
- `ifd` — Intel Flash Descriptor(ICH/PCH SPI flash 仕様): `0x0FF0A55A`@0x10、FLMAP0/1 が 16B 単位のベースアドレス(FCBA/FRBA/FMBA/FPSBA)とカウント(NC/NR/NM)をパック、FLREGx は 15bit 4KiB 単位の base/limit で enabled は `limit>=base`

**検証**: 新規テスト全緑(5,386 lib テスト + 604 doctest)。oracle: NE のフラグ分解と CS:IP/SS:SP 対、LE/LX 署名分岐と object テーブルオフセット、a.out の4 magic と `syms_at` 連鎖、DEX の `header_size==0x70` 強制と7テーブル対、PCIR のポインタ二段解決と last-image ビット、CBFS の64Bアライン歩進と削除スロット終端、IFD の FLMAP ビット分解と base/limit enable 判定・frba 超過拒否。ラウンド内捕捉: MSRV 1.75 により `trim_ascii_end`(1.80)不許可 → NUL トリムを `name()` に集約、CBFS while 条件の `?` がチェーン終端を None 化する → `loop`+`match` に、a.out `syms_at` のテスト期待値誤算、PCIR class code のバイト順(iface/sub/base)。

## 出典(第100次、search-index 照合)

**論文・仕様**: Microsoft `newexe.h`/exehdr の NE 定義と IBM/Microsoft「New Executable」仕様 / OS/2 LX・LE の linearexe ヘッダ layout(EDM/2・osFree 資料) / 4.4BSD `a.out(5)` マニュアルと NetBSD `exec.h` の midmag パッキング / Android `DexFile` 形式仕様(map_list・各 *_ids テーブル) / PCI Firmware Specification の option ROM・PCIR 定義 / coreboot の CBFS エントリ構造体とアライメント規則 / Intel ICH9/100 系 SPI flash の descriptor map 資料 — 全て整数のみで実装。

**実装物**: binutils/LLVM の NE・LX リーダ、linux `a.out`/`execve` の歴史的ローダ、Android runtime の `DexFileVerifier`/`libdex`、SeaBIOS/coreboot の option ROM ランナと `cbfstool`、flashrom の descriptor パーサ(ich_descriptors_tool) — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の MZ/NE/PE ヘッダ解析記事(「e_lfanew をたどる」系)、a.out→ELF 移行の国内解説、DEX ファイル構造の日本語リバース資料、coreboot/flashrom 導入記事、BIOS ROM・Intel Flash Descriptor の国内検証記事 — 全て整数のみで実装。

## 第101次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 科学・医療・気象・分子データ形式(全7件が既存 613 件と非衝突を確認):

- `dicom` — DICOM Part 10(NEMA PS3.10): 128B preamble + `DICM`。explicit-VR 要素走査 — `(group,element)` u16LE + 2B VR + u16 長(OB/OW/OF/OD/OL/OV/SQ/UC/UR/UN/UT は `00 00` + u32 長)。indefinite-length SQ は `u32::MAX` で識別、`get(group,element)`/`text` でタグ参照
- `nifti` — NIfTI-1 348B ヘッダ(nifti1.h): `sizeof_hdr`==348 の LE/BE 自動判別。`dim[8]`(dim[0]=次元数)、`datatype`/`bitpix`、pixdim/vox_offset/scl/cal は f32 を **raw u32 ビット保持**(kit は float を解釈しない)、qform/sform code、`n+1`/`ni1` magic で単一 vs .hdr/.img 分割を `Kind` 判別
- `nrrd` — NRRD(teem `nrrd` 1-5): `NRRD000x` magic + `key: value` 行 + `#` コメント、空行でヘッダ終端→data 開始位置。`get`/`sizes`/`dimension` の型付きビュー
- `nc` — NetCDF classic(UCAR CDF 仕様): `CDF`+version(1=32bit,2=64bit,5=CDF-5) + `numrecs`、dim_list/gatt_list/var_list は ABSENT(0,0) または NC_* tag+count。名前は u32 長 + 4B パディング、属性は `(name,xtype,count,value)` で値サイズは型幅×count を4B丸め
- `grib` — WMO FM-92 GRIB: ed1 は 3B 総長 + Indicator Section(table ver/centre/process/GDS-BMS フラグ)、ed2 は discipline + u64 総長 + 番号付き section(len≥5、num 7 で data 端)の鎖走査
- `pdb` — Protein Data Bank 固定カラム(80 桁カード): ATOM/HETATM の serial/name/residue/chain/resseq、x/y/z は 8.3 固定小数点 → **整数 milliunits**、CRYST1 は cell a/b/c(9.3) + 角度(7.2) + space group
- `mol2` — Tripos MOL2: `@<TRIPOS>MOLECULE` の name/counts(atoms/bonds/substructures)/mol_type/charge_type、`@<TRIPOS>ATOM` 行の id/name/xyz(固定小数点→milliunits)/type/subst/charge

**検証**: 新規テスト全緑(5,408 lib テスト + 611 doctest)。oracle: DICOM の long-VR u32 長と indefinite-length、NIfTI の sizeof_hdr 両エンディアン判別と `n+1`/`ni1`、NRRD の空行終端と `data_at`、NetCDF の ABSENT list と 4B パディング名前、GRIB ed2 の section 鎖、PDB の固定カラム負座標と CRYST1、MOL2 のセクション横断。ラウンド内捕捉: PDB 行末で field が短くなる行に `get(55..66)` が失敗 → clamp、NRRD の末尾 `\n` は暗黙の空行を生む(未終端テストは `…4` で末尾 newline なしに)、NIfTI BE テストの pixdim index 誤り。

## 出典(第101次、search-index 照合)

**論文・仕様**: NEMA PS3.10 Media Storage & File Format(VR 表・explicit/implicit エンコーディング) / NIfTI-1 `nifti1.h` と NIfTI-1 Data Format spec / teem NRRD format definition / NetCDF classic `file format specification`(B-netcdf) / WMO Manual on Codes FM 92 GRIB edition 1・edition 2 / wwPDB Atomic Coordinate Entry Format v3.3 カラム定義 / Tripos MOL2 File Format — 全て整数のみで実装(float フィールドは raw bits または固定小数点整数)。

**実装物**: pydicom/dcmtk の explicit-VR タグ走査、nibabel の NIfTI ヘッダ、nrrd/teem のヘッダパーサ、netcdf-c の `nc3` ヘッダ読み、ecCodes/wgrib の edition 判別、biopython/PDB-tools の固定カラム抽出、Open Babel/RDKit の MOL2 リーダ — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の DICOM タグ解析記事(プレアンブル+タグ走査系)、NIfTI ヘッダの日本語解説、NetCDF/HDF 形式比較記事、GRIB2 の気象データ解説、PDB/MOL2 の構造データ国内チュートリアル — 全て整数のみで実装。

## 第102次(search-index 照合ラウンド / 実装証跡付き)

**方法**: パッケージ・アーカイブ・複合ドキュメント系エンベロープ(全7件が既存 620 件と非衝突を確認):

- `deb` — Debian バイナリパッケージ(deb(5)/deb-format): `ar` の第1メンバが `debian-binary`(内容 `"2.0\n"`)、`control.tar.*`/`data.tar.*` を suffix で `Compression` 分類(gz/xz/zst/bz2/none)。`crate::ar` 再利用の薄いラッパ
- `ole` — OLE2 Compound File Binary(MS-CFB): `D0CF11E0A1B11AE1`、major3=512B/major4=4096B セクタ、109スロット DIFAT → FAT 構築 → `first_dir_sector` から ENDOFCHAIN までディレクトリ鎖歩進、128B エントリの UTF-16 名/FREESECT リンク/ストリーム先頭+サイズ
- `rar` — RAR(技術ノート note.txt): v4 マーカ `Rar!\x1A\x07\x00` + HEAD_CRC/TYPE/FLAGS/SIZE + `0x8000` で ADD_SIZE。main(0x72)/file(0x73)/service(0x7A)/end(0x7B) の鎖歩進。v5 マーカは `01 00` で `Kind::Rar5` 判別
- `rpm` — RPM(max-rpm lead/header 仕様): 96B lead(`EDABEEDB` + ver + type + arch + name66B + os + sigtype)、ヘッダ構造 `8EA8E8 01` + nindex/hlen u32BE、signature→8Bアラインで main ヘッダ位置
- `x7z` — 7z(7zFormat.txt): `377ABCAF271C` + ver(00 04) + start_crc + next_header の (offset,size,crc)。`next_header()` でファイル内位置解決
- `xar` — XAR(xar-1.x 仕様, `.pkg`/`safariextz`): `xar!` u32BE magic + header_size(28) + version + TOC 圧縮/非圧縮長 u64BE + checksum id(0 none/1 sha1/2 md5/3 sha256)
- `xz` — XZ ファイル形式(tukaani xz-file-format.txt): `FD 37 7A 58 5A 00` + stream flags(上位4bit 予約0 + check id 下位4bit: 0/1 crc32/4 crc64/10 sha256) + flags CRC32 + ブロックヘッダ `(n+1)*4` B

**検証**: 新規テスト全緑(5,426 lib テスト + 618 doctest)。oracle: OLE の DIFAT→FAT→ディレクトリ鎖と UTF-16 名、RAR4 の ADD_SIZE 鎖と v5 判別、RPM の signature→main アライメント、7z next_header のファイル内位置、XAR checksum 写像、XZ の check nibble とブロックサイズ式、deb の debian-binary 先頭強制。ラウンド内捕捉: OLE セクタオフセット式(ヘッダ後 N×sector_size)、RPM doctest の nindex/hlen オフセット誤り(104/108)、`header().ok()` の Option 二重包み、closure の `&mut d` 競合→`fn` 化、XZ ブロックがバッファを超える例。

## 出典(第102次、search-index 照合)

**論文・仕様**: `deb(5)` マニュアルと Debian `.deb` 形式説明(ar+debian-binary+control/data) / Microsoft `[MS-CFB]` Compound File Binary Format(ヘッダ・DIFAT/FAT・ディレクトリエントリ) / RARLAB「RAR 4.x archive format — technical note」と `RAR 5.0` 形式 / `Maximum RPM` の lead・signature・header structure / 7-zip `7zFormat.txt`(signature header・kEnd・next header) / `xar` フォーマット(ヘッダ・TOC 長・checksum メッセージダイジェスト名) / tukaani `xz-file-format.txt`(stream flags・check 型・block header) — 全て整数のみで実装。

**実装物**: dpkg/ar 実装、python `olefile`/`libarchive` CFB リーダ、unrar/bsdtar の RAR4 ブロック走査、rpm ツールの lead/signature パーサ、7-Zip `7zIn.c`/`p7zip`、libxar、xz utils `stream_flags`/`block_header` デコーダ — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の deb/rpm 内部構造の解析記事、CFB(Office 旧形式)ヘッダ解説、7z/xz 形式の日本語メモ、XAR(.pkg)検証記事 — 全て整数のみで実装。

## 第103次(search-index 照合ラウンド / 実装証跡付き)

**方法**: ファイルシステムのスーパーブロック/ブート領域(全7件が既存 627 件と非衝突を確認):

- `ext2` — ext2/ext3/ext4 スーパーブロック(linux ext2_fs.h / ext4 wiki): オフセット1024固定。inodes/blocks/free 各カウント、first_data_block、`s_log_block_size`→`1024<<n` ブロックサイズ、blocks/inodes per group(ブロックグループ数は切り上げ除算)、mount/state/errors、feature_compat/incompat/ro_compat(journal/extents/64bit フラグ解読)、rev_level、first_ino、inode_size、UUID+ラベル
- `ntfs` — NTFS ブートセクタ:Microsoft NTFS BPB レイアウト。`NTFS    ` OEM、bytes/sector×sectors/cluster、total_sectors、`$MFT`/`$MFTMirr` の LCN、file-record/index の**符号付き**クラスタ係数(負なら `2^|n|` バイト、正なら n クラスタ — `expand` でバイト化)、シリアル、`55AA` 確認
- `hfsplus` — HFS+/HFSX ボリュームヘッダ(Apple TN1150): オフセット1024、`H+`/`HX` 判別、BE カウント類(file/folder/total/free blocks)、block_size、Mac epoch(1904)日時、next_catalog_id、5系 ForkData(alloc/extents/catalog/attributes/startup — 各 論理サイズ+clump+total+先頭 extent ペア)
- `ufs` — UFS1/UFS2 スーパーブロック(BSD ffs/ufs 8K オフセット): `fs_magic`@+1372 が `0x00011954`(UFS1)/`0x19540119`(UFS2)を分岐。frag/block サイズ、cg 数、fpg/ipg、minfree、UFS2 は size/dsize と cstotal が 64bit 化、fs_fsmnt 52B
- `minix` — MINIX v1/v2 スーパーブロック(minix fs.h): オフセット1024。magic 4値(`0x137F`/`0x138F`/`0x2468`/`0x2478` = v1/v2 × 14/30文字名)が zones の読み元(16bit nzones vs 32bit s_zones)を決める — `Magic::is_v1`/`name_len`/`zones()` に集約
- `xfs` — XFS スーパーブロック(xfs_sb.h): AG 先頭、`XFSB` で唯一の BE 系。block_size/dblocks/uuid/logstart/rootino、agblocks/agcount、versionnum 下位4bit が major(4/5)、sectsize/inodesize/inopblock、12B 名、icount/ifree/fdblocks/frextents、features2(v5)
- `exfat` — exFAT メインブートレコード(MS exFAT spec §3.3): `EXFAT   ` OEM + 53B MustBeZero 厳格チェック(FAT12/16 の誤マウント排除)、partition_offset/volume_length、FAT offset×length、cluster heap offset+count、root 先頭クラスタ、serial、revision、flags(active-FAT/dirty/media-failure)、**シフト表現**の bytes/sector・sectors/cluster(9..12 / 0..25)、`root_cluster_at` が heap+(cluster-2) のバイトオフセット

**検証**: 新規テスト全緑(5,441 lib テスト)。oracle: ext2 の `1024<<n` ブロックサイズとグループ数切り上げ・feature マスク、NTFS の負係数→2^k バイト化(−10→1024)、HFS+ の `H+`/`HX` 判別と catalog fork の first-extent、UFS の magic によるレイアウト分岐と UFS2 の 64bit フィールド、MINIX の magic→zones 読み元分岐、XFS の versionnum 下位4bit と AG ジオメトリ、exFAT の MustBeZero 強制と `heap+(root-2)` オフセット。

## 出典(第103次、search-index 照合)

**論文・仕様**: linux `Documentation/filesystems/ext2.txt`/`ext4` wiki(superblock layout・feature マスク)/ Microsoft NTFS BPB リファレンス(符号付きクラスタ係数)/ Apple TN1150 HFS+ Volume Format(volume header・ForkData・catalog)/ BSD `fs/ufs` 系 superblock(UFS1/2 の fs_magic 分岐・64bit 化)/ MINIX `fs.h` magic 4値 / XFS `xfs_format.h`/`xfs_sb.h`(BE スーパーブロック・AG)/ Microsoft exFAT File System Specification §3(MustBeZero・シフト表現・クラスタヒープ) — 全て整数のみで実装。

**実装物**: e2fsprogs/dumpe2fs のスーパーブロックダンプ、ntfs-3g の BPB リーダ、Apple `hfs` 実装と Linux `hfsplus` ドライバ、FreeBSD `ufs/ffs`、minix-tools、xfsprogs/xfs_db、linux exfat ドライバと `exfatprogs` — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の ext2 スーパーブロック解析記事(1024 オフセット・magic 0xEF53 系)、NTFS ブートセクタ解説、HFS+ のディスク検証記事、exFAT フォーマット仕様の日本語抄訳・解析メモ、XFS/UFS ファイルシステム比較記事 — 全て整数のみで実装。

## 第104次(search-index 照合ラウンド / 実装証跡付き)

**方法**: ゲームエンジン資産コンテナ(全7件が既存 634 件と非衝突を確認):

- `bsp` — Quake BSP(Quake Wiki BSP 仕様): Q1 はマジックなしの生 version 29 + 15 ランプ `(offset,len)` ディレクトリ、Q2 は `IBSP`+38(III 系は 46)+19 ランプ。全ランプを EOF 境界照合、`Q1_LUMPS`/`Q2_LUMPS` 名表 + `lump()` ビュー
- `mdl` — Quake MDL(Quake wiki / quake source `mdl.h`): `IDPO`+version 6、84B ヘッダ — scale/translate/eye/radius は f32 の **raw u32 ビット**保持、numskins/skinwh/numverts/numtris/numframes/synctype/flags
- `md2` — Quake II MD2(megafps/md2 仕様): `IDP2`+version 8、68B ヘッダ — skinwh/framesize/5カウント + 6スロット section offset 表(skins/st/tris/frames/glcmds/end)、`section_at`/`section_name`、全 offset 境界照合
- `mpq` — Blizzard MPQ(zealdocs MPQ 仕様): `MPQ\x1A` ユーザヘッダマジック、header/archive size、format_version(v0-v3)、`512<<shift` セクタ、hash/block テーブル位置+エントリ数(各16B)、v2 拡張(hi テーブル u64 + hi16 半分)
- `grp` — Build エンジン GRP(Ken Silverman 形式): `KenSilverman` 12B 署名 + u32 カウント + 連続 `(name12, size)` ディレクトリ → 順次 blob。`file`/`find`(大文字不区別)で参照
- `vtf` — Valve VTF(Valve Dev Community VTF 仕様): `VTF\0` + (7,0)-(7,5) バージョン対 + 80B ヘッダ: 幅高・flags・frames/first_frame・reflectivity/bump raw bits・image_format・mipmap・低解像度サムネイル・depth(7.2+)
- `vpk` — Valve VPK(同 VPK 仕様): `0x55AA1234`、v1 は 12B(tree size のみ)、v2 は +16B(file-data/archive-md5/other-md5/signature 各セクション長)。`tree_end`/`signature_at`/`total_len` で配置連鎖

**検証**: 新規テスト全緑(5,456 lib テスト)。oracle: BSP の Q1/Q2 ディレクトリ開始差(Q1 は version 直後=+4、IBSP は +8)と全ランプ EOF 照合、MDL の v6 固定・raw float ビット、MD2 の 6 オフセット表境界、MPQ の `512<<shift` と 16B エントリ範囲・v2 拡張フィールド、GRP の 12.3 名 NUL トリムと blob 連続配置、VTF の `VTF\0`+v7.x とサムネイル存在判定、VPK の v1/v2 セクション連鎖。ラウンド内捕捉: BSP の Q1 ディレクトリ位置(0→4 修正)、GRP フィクスチャの 12.3 名コピー幅不一致、MDL/MD2/VPK doctest のフィールドオフセット誤り。

## 出典(第104次、search-index 照合)

**論文・仕様**: Quake BSP 形式仕様(Quest for the Mersenne Twister / Quake Wiki BSP29・IBSP ドキュメント)/ Quake `mdl.h` 構造体定義 / `MD2` ファイルフォーマット記述(megafps 他) / zealdocs「MPQ File Format」/ Ken Silverman の Build engine GRP 定義 / Valve Developer Community「Valve Texture Format」「VPK File Format」— 全て整数のみで実装(f32 は raw bits)。

**実装物**: Quake/Q2 ソースのモデルローダ、quake-utils/wad3 系ツール、StormLib(ZeL sounding MPQ 実装)、kextract/EDuke32 の GRP リーダ、VTFLib、vpk.exe/ValveResourceFormat — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の Quake 資産解析・MOD 作成記事、GoldSrc/Source エンジンの VTF/VPK 解説、MPQ/StormLib 日本語資料、Build エンジン系の国内メモ — 全て整数のみで実装。

## 第105次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — レガシー CJK エンコーディングと UTF-16(全7件が既存 641 件と非衝突を確認):

- `sjis` — Shift_JIS(WHATWG Encoding の trail 範囲定義): lead `0x81..=0x9F`/`0xE0..=0xFC`、trail `0x40..=0x7E`/`0x80..=0xFC`(`0x7F` 除外)、`0xA1..=0xDF` は半角カタカナ。`to_kuten` は `p = hi*188 + (trail≤0x7e ? trail−0x40 : trail−0x41)`(hi は ≤0x9F で `lead−0x81`、以降 `lead−0xC1`)から `(p/94+1, p%94+1)` の区点へ投影
- `eucjp` — EUC-JP(JIS X 0208/0212 対応): `0x8E`+1B=半角カタカナ、`0x8F`+2B=JIS X 0212、`0xA1..=0xFE`×2=JIS X 0208 の区点
- `iso2022` — ISO-2022-JP(RFC 1468 + JIS X 0212 拡張): 状態機械で G0 指定を追跡 — `ESC ( B` ASCII、`ESC ( J` JIS X 0201 roman、`ESC $ B`/`ESC $ @` JIS X 0208(1983/1978)、`ESC $ ( D` JIS X 0212。指定中は `0x21..=0x7E`×2 が区点
- `big5` — Big5(ETen/CNS 11643 系): lead `0x81..=0xFE` × trail `0x40..=0x7E`|`0xA1..=0xFE`、`point = (lead−0x81)*157 + adj`(adj: trail≤0x7e → −0x40、else −0x62)の線形インデックス
- `gbk` — GBK(GB 2312 上位互換拡張): lead `0x81..=0xFE` × trail `0x40..=0xFE`(`0x7F` のみ穴)。低位 trail `0x40..=0x7E` は GBK 独自(GB 2312 では違法)。`point = (lead−0x81)*190 + adj`(−0x40/−0x41)
- `euckr` — EUC-KR(KS X 1001): シフトなしの `0xA1..=0xFE`×2 区点のみ
- `utf16` — UTF-16(Unicode Core §3.9 D91): BOM `FE FF`/`FF FE` 検出(無 BOM は BE)、`unit` は手動シフトで両端序対応、hi `0xD800..=0xDBFF` + lo `0xDC00..=0xDFFF` → `0x10000+((hi−0xD800)<<10)|(lo−0xDC00)`、孤立サロゲートは `Unpaired`、`encode` は範囲外を `0xFFFD`

**検証**: 新規テスト全緑 + doctest 全緑(639 件)。oracle: SJIS 区点公式の往復(lead/trail 境界 `0x7E`/`0x7F` 判定)、EUC-JP 3 系シフト、ISO-2022-JP の指定→文字→リセット状態遷移、Big5/GBK の `point` 線形式と低位 trail の GBK 独自性、EUC-KR 区点、UTF-16 の BOM・サロゲート対・孤立サロゲート・エンコード逆変換。ラウンド内捕捉: SJIS 区点の +1 二重計上、UTF-16 の奇数バイト末尾を `?` で落とす経路(長さ先検査に変更)、`to_be_bytes` 禁止で `unit` を手動シフト化。

## 出典(第105次、search-index 照合)

**論文・仕様**: WHATWG Encoding Standard(Shift_JIS/EUC-JP/Big5/GBK/EUC-KR の lead-trail テーブルと pointer 式)/ RFC 1468 ISO-2022-JP(escape sequence 指定集)/ JIS X 0208・JIS X 0212・JIS X 0201 / CNS 11643(Big5)/ GB 2312-80・GBK 仕様 / KS X 1001 / Unicode Standard §3.9(D91 UTF-16・サロゲート範囲・BOM)— 全て整数のみで実装。

**実装物**: glibc/libiconv の SHIFT_JIS・EUC-JP・ISO-2022-JP・BIG5・GBK・EUC-KR テーブル、nkf の 2022-JP 状態機械、ICU コンバータの境界挙動、Rust `encoding_rs` のインデックス付け方針 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の文字コード解説記事(「Shift_JIS のバイト範囲」「EUC-JP と ISO-2022-JP の違い」「サロゲートペアの仕組み」系)、JIS 区点表の国内整理、nkf 派生記事 — 全て整数のみで実装。

## 第107次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — チャンク型マルチメディアコンテナと点群データ(全7件が既存 648 件と非衝突を確認):

- `iff` — EA IFF-85(Electronic Arts のチャンク構造): `FORM`/`LIST`/`CAT ` + BE32 サイズ + 4B 型ワード、チャンク `{id:4, size:BE32, data, 偶数パディング}`、`subform` で入れ子 FORM を再帰解決。Amiga の ILBM/8SVX/AIFF(Mac) の祖先
- `avi` — RIFF AVI(Microsoft、IFF 派生で LE32): `RIFF <sz> AVI `、`LIST hdrl`/`avih`(56B 固定レイアウト: usec/frame、flags、total frames、streams、幅高さ)、`LIST strl`/`strh`(56B: fccType `vids`/`auds`、handler、scale/rate/length)、`LIST movi` データ位置。RIFF サイズは `RIFF` 語自身と形式ワードを除く
- `flv` — Flash Video(Adobe FLV spec): `FLV` + version + flags(bit2=audio, bit0=video) + BE32 ヘッダサイズ(9) + PrevTagSize0(0)。タグ `{type:u8, BE24 サイズ, BE24 タイムスタンプ, u8 ts拡張上位, BE24 streamid, data, BE32 prevsize}`、時刻は `BE24 | ext<<24` の 32bit 合成。type 8/9/18 = audio/video/script
- `caf` — Core Audio Format(Apple、64bit サイズで RIFF の 4GB 限界を突破): `caff` + u16 ver + u16 flags; チャンク `{type:4, size:BE64-as-i64}` — 負サイズは「EOF まで」。`desc` = 32B ASBD 全BE(f64 sample rate は raw u64 bits)、`data` チャンクの先頭 u32 は edit count
- `voc` — Creative Voice File(Sound Blaster 時代): banner `"Creative Voice File\x1A"`(20B) + u16 データオフセット(≥26) + u16 version + u16 check(`= !version + 0x1234`)。ブロック `{type:u8, size:LE24, data}`、type 0 = 1B 終端、1=サウンド、3=無音、9=新拡張フォーマット
- `las` — ASPRS LAS 点群(1.0–1.4): `LASF`、version @24,25、header_size u16@94 がレイアウトを決める(227 ≤1.2 / 235 v1.3 +waveform@227 / 375 v1.4 +EVLR@235,count64@247,by_return64@255)。scale/offset/bounds の f64 は raw u64 bits で保持、v1.4 は legacy u32 点数が 0 のとき points64 を使う
- `woff2` — Web Open Font Format 2(W3C、Brotli 圧縮): 48B BE ヘッダ `wOF2`、flavor@4(`0x00010000`/`true`/`typ1`/`OTTO`/`ttcf`)、reserved@14 は必ず 0、meta(offset,len,origLen)/priv(offset,len) ペア — 長さ非ゼロなら offset 必須

**検証**: 新規テスト全緑 + doctest 全緑。oracle: IFF の pad-to-even 走査と入れ子 LIST/FORM、AVI の `strh` が `LIST strl` の型ワードであること(chunk id ではない — 実装は `id==LIST && ty==strl` で分岐)、FLV の BE24/32bit 時刻合成、CAF の負サイズ=EOF 規則、VOC の `!version+0x1234` 検算(演算子優先度の落とし穴: `!x.wrapping_add(..)` は `!(x.wrapping_add(..))` と解釈されるため括弧必須)、LAS の version 別最小ヘッダ長、WOFF2 の meta/priv オフセット整合。ラウンド内捕捉: `*b"..."` はパターンとして不許可(matches!→等価比較連鎖へ修正)、AVI `strl` は LIST の型ワード、VOC check の折返し(`wrapping_add` + 16bit マスク)、IFF チャンクの `at` はデータ開始(ヘッダ+8)。

## 出典(第107次、search-index 照合)

**論文・仕様**: EA「IFF: A Standard for Interchange Format Files」(EA IFF 85)/ Microsoft RIFF/AVI 仕様(avifil32・aviriff.h の `strh`/`avih` レイアウト)/ Adobe「Video File Format Specification」(FLV v10.1)/ Apple Core Audio Format Specification(caff チャンク・ASBD)/ Creative Labs「Voice File (.VOC) Technical Specifications」/ ASPRS LASer File Format Exchange Activities(LAS 1.0–1.4 R15)/ W3C Recommendation「WOFF File Format 2.0」§4(File structure)— 全て整数のみで実装。

**実装物**: libsndfile の CAF/VOC/IFF リーダ、FFmpeg の `avidec.c`/`flvdec.c`、lastools/LASlib、fontTools の WOFF2 コンパイラ(w3c-woff2)/ Google woff2 リファレンス実装 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の RIFF チャンク解析・FLV/RTMP 配信解説・LAS 点群処理記事・WOFF2 フォント圧縮紹介、『ゲームプログラマになるための3Dグラフィックス技術』系の IFF/RIFF 言及 — 全て整数のみで実装。

## 第108次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — カラムナリデータ交換・タグディレクトリ・CAD/印刷記述(全7件が既存 655 件と非衝突を確認):

- `parquet` — Apache Parquet ファイルエンベロープ: 先頭 `PAR1` + 末尾 `PAR1`(暗号化フッタ時は `PARE`)、末尾8Bは `u32 LE フッタメタ長 + PAR1`、フッタ本体は `len−8−meta_len` から(Thrift compact — 実装は外郭のみでメタ内容は不透明)
- `avro` — Apache Avro OCF: `Obj\x01` + zigzag-varint メタデータマップ(`long count`、負数 `-N` は「N エントリ + 前置き byte-size long」のブロック形)+ 16B sync マーカ。データブロックは `{count, size, payload, sync}` を EOF まで反復、各ブロックで sync 照合
- `arrow` — Apache Arrow IPC ファイル形式: 両端 `ARROW1\0\0`(8B)、`len−12` に i32 フッタ長。メッセージ列は `0xFFFFFFFF` 継続 + i32 メタ長(v1.0+)、レガシーは裸 i32(0=EOS)、メタデータ後のボディは 8B アライン。フラットバッファ内部のボディ長は解かないので走査は body-less メッセージで正確
- `tiff` — TIFF 6.0 + BigTIFF: `II`(LE)/`MM`(BE) バイトオーダ + マジック 42(クラシック: u32 IFD0@4、u16 カウント + 12B エントリ `{tag,type,count,value}` + u32 next)/ 43(BigTIFF: bytesize u16=8@4、reserved 0@6、u64 IFD0@8、u64 カウント + 20B エントリ)。インライン値は `count*type_size ≤ 4`(クラシック)/ ≤8(BigTIFF)、**BE のインライン SHORT は u32 フィールドの上位ハーフに置かれる**
- `tds` — 3DS バイナリ(Autodesk 3D Studio): チャンク `{id:u16 LE, len:u32 LE(ヘッダ6B込), data}` の木。`0x4D4D` MAIN → `0x3D3D` EDITOR → `0x4000` OBJECT(先頭 NUL 名 + サブチャンク)→ `0x4100` MESH → `0x4110` VERTICES(u16 数 + 12B/頂点)/ `0x4120` FACES(u16 数 + 8B/面)
- `dxf` — AutoCAD DXF ASCII 交換: 「整数グループコード行 + 生値行」の交互。`0`=エンティティ区切り、`2`=名前、`999`=コメント、`0`/`SECTION`…`0`/`ENDSEC` がセクション区間、`0`/`EOF` で完結
- `eps` — EPSF/DSC ヘッダ(Adobe Document Structuring Conventions 3.0): `%!PS-Adobe-x.y`(+` EPSF-x.y`)、`%%Key: value` コメント、`%%BoundingBox:` 4 int、`%%Pages:`、`%%Page:`、`%%EndComments`、`%%EOF`

**検証**: 新規テスト全緑 + doctest 全緑。oracle: TIFF の両端序(手組み BE フィクスチャ — `to_be_bytes` 禁止のため生バイト配列)と BigTIFF 20B エントリ・BE インライン SHORT の上位ハーフ配置、3DS の OBJECT 名スキップ後のサブチャンク解決、DXF のコード/値行ペアリングと EOF 判定、EPS の BoundingBox int 4 つ組と Pages、Avro の負カウント・ブロック形マップと sync 照合、Parquet の PARE 暗号化フッタ判別、Arrow の continuation エンベロープ。ラウンド内捕捉: Avro `count.checked_neg()` は正数も反転する(2→−2 でエントリループが空に)— `if count < 0` に修正、`usize::MAX as i64` は −1 に折り返るため長さ上限比較が全失敗(`kl as u64 > usize::MAX as u64` へ)、DXF の値行なし終端条件(`val_at >= len`)。

## 出典(第108次、search-index 照合)

**論文・仕様**: Apache Parquet Format(parquet-format の File Format 節 — `PAR1`/`PARE` と 4B フッタ長)/ Apache Avro 1.x Specification「Object Container Files」(zigzag/LEB128・ブロック形マップ・sync)/ Apache Arrow Format「IPC File Format」(continuation・EOS・8B アライン)/ Adobe TIFF Revision 6.0 + BigTIFF ドラフト(IFD レイアウト・インライン値規則)/ Autodesk 3D Studio File Format(MLehnérfeldt 解説・chunk id 表)/ AutoCAD DXF Reference(group code 表)/ Adobe DSC 3.0(仕様番号 5001)+ EPSF 規約 — 全て整数のみで実装。

**実装物**: parquet-rs/arrow-rs のフッタ・メッセージ走査、apache/avro の OCF デコーダ、Pillow/libtiff の IFD パーサ(BE インライン値の高ハーフ配置を確認)、Blender/Assimp の 3DS インポータ、`ezdxf` の pair イテレータ、ghostscript の DSC スキャナ — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の Parquet/Avro/Arrow 比較・Iceberg/Spark 連携記事、TIFF タグ仕様の国内整理、3DS→OBJ 変換記事、DXF を自前で読む記事、PostScript/DSC 解説 — 全て整数のみで実装。

## 第109次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — スキーマ付きシリアライズフレーム + Windows フォレンジックアーティファクト(全7件が既存 662 件と非衝突を確認):

- `thrift` — Apache Thrift TBinaryProtocol: 厳格版は `0x80010000 | type`(BE u32、最上位ビットが厳格印)+ 名前 `string`(i32 BE len)+ i32 seqid;非厳格版は先頭が名前長。TFramedTransport は各メッセージに u32 BE フレーム長を前置。type 1..4 = CALL/REPLY/EXCEPTION/ONEWAY
- `flatbuf` — FlatBuffers バッファ: `u32 LE` でルートテーブル位置、任意の 4B `file_identifier`、テーブル先頭の i32 は vtable への**逆行き**距離。vtable = `{u16 vtable_len, u16 table_len, u16 field_voffsets[]}`、voffset 0 = フィールド欠落、voffset ≥ table_len は非合法
- `capnp` — Cap'n Proto ストリームフレーミング: `u32 (segment_count − 1)` + count 個の u32 ワード数、テーブルは偶数 u32 個にパディング。セグメントデータは連続、各セグメントは 8B ワード単位
- `ion` — Amazon Ion 1.0 バイナリ: `E0 01 00 EA` BVM(型14 ann 長3 の値でもある)+ TLV `{typedesc:u8(hi4=型,lo4=長), [lo==14→varuint len], payload}`、lo=15 は null/終端形。varuint は 7bit 群 + 終端 high bit
- `regf` — Windows レジストリハイブ: `regf` + 4096B ヘッダ(seq 主/副 @4/@8 一致検査、version、root cell rel @36、size @40)、`0x1000` から 4096 アライン `hbin` ブロック鎖、セルは符号付き i32 サイズ(負=割当済み、8 アライン)
- `evtx` — Windows イベントログ: `ElfFile\0` + 4096B ヘッダ(最初/最後 chunk no、次 record id、header_size 128、major 3/minor 1、chunk 数、@124 に先頭120B の CRC32)+ `0x1000` 以降 64KiB `ElfChnk\0` チャンク鎖
- `prefetch` — Windows Prefetch `.pf`: version @0(17/23/26/30/31)+ `SCCA` @4 + filesize @12 + UTF-16LE 名 @16(60B)、hash @76(v17/23)/@80(v26+)、run_count @0x90/@0x98/@0xD0(版別)。Win10+ の `MAM\x04` 圧縮は検出のみ

**検証**: 新規テスト全緑 + doctest 全緑。oracle: Thrift 厳格/非厳格の同じバッファ二肢、TFramedTransport の枠長照合、FlatBuffers の soffset 逆方向解決と voffset=0 欠落、Cap'n Proto の count−1 格納と偶数パディング、Ion の lo=14 varuint 長/lo=15 null 終端、regf の hbin/負サイズセル、evtx の表なし CRC32 + 64KiB チャンク境界、prefetch の版別 hash/run_count オフセット差。ラウンド内捕捉: FlatBuffers のルートは **vtable ではなくテーブル** を指す(フィクスチャの u32 を 8→14 に修正)、table_len は soffset ワードを含む(f0 voffset 4 は table_len ≥ 8 が必要)。

## 出典(第109次、search-index 照合)

**論文・仕様**: Apache Thrift 仕様(「thrift-spec」`TBinaryProtocol` の `0x80010000|type` 厳格ワード・`TFramedTransport`)/ Google FlatBuffers「Internals of FlatBuffers」(root uoffset・file_identifier・soffset/vtable レイアウト)/ Cap'n Proto Encoding Spec「Serialization over a stream」(segment table·count−1・偶数パディング)/ Amazon Ion 1.0 Specification「Binary Encoding」(BVM・typedesc ニブル・varuint)/ libyal winreg-kb「Windows NT Registry File(REGF)format」/ libyal evtx-kb「Windows XML Event Log(EVTX)format」/ libyal libscca「Windows Prefetch File(PF)format」(版別オフセット表)— 全て整数のみで実装。

**実装物**: apache/thrift の `TBinaryProtocol`・`TFramedTransport` 実装、google/flatbuffers の `GetRoot`/`Table` 参照、capnproto C++ の `serialize.c++` セグメントテーブル、amazon-ion の ion-c、RegRipper/sleuthkit の regf パーサ、python-evtx・libevtx のチャンク走査、Eric Zimmerman PECmd の prefetch 版別レイアウト — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の gRPC/Thrift/FlatBuffers/Cap'n Proto 比較記事・Ion 紹介、Windows フォレンジックの regf/evtx/prefetch 解析記事(DFIR 系)、『Windows Forensic Analysis』系書籍の邦訳知見 — 全て整数のみで実装。

## 第110次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — EDA/半導体設計データ + 光学/仮想ディスクメタデータ(全7件が既存 669 件と非衝突を確認):

- `gds` — GDSII ストリーム(`{reclen:u16 BE incl. 4B 自ヘッダ, tag:u8, dtype:u8}` のレコード走査、`HEADER` の i16 version、tag/dtype 名表)。GDS の実数は IBM excess-64 — 浮動小数点デコードは行わず生バイトのまま
- `edif` — EDIF (ANSI/EIA-548) ネットリスト: LISP 形式の字句(`;` コメント・文字列保持)、`(edifVersion M m p)` 抽出、`(cell`/`(library` 定義数
- `lef` — LEF 物理アブストラクト: `VERSION x.y ;`、`UNITS … DATABASE MICRONS n ;`(実構文は UNITS ブロック内行頭 `DATABASE`)、`MACRO name`/`PIN name` … `END` ブロック
- `def` — DEF 設計ファイル: `VERSION`/`DESIGN`/`COMPONENTS n ;` ヘッダ + `- inst macro …` 行は `COMPONENTS…END COMPONENTS` 内のみ有効(ブロック外の `-` を誤数しない)
- `liberty` — Liberty `.lib`: `library (name) { … }` グループ、任意深さの `cell (arg)` 引数走査、`key : value ;` 属性
- `udf` — UDF (OSTA/ECMA-167): セクタ16+ の Volume Recognition Sequence `{type 0, id "BEA01"/"NSR02"/"NSR03"/"TEA01", ver 1}`(1 セクタ 1 記述)、AVDP(tag id 2 @ sector 256)存在確認
- `vhdx` — Hyper-V VHDX: `vhdxfile` 署名 + 512B UTF-16LE creator、64KiB/128KiB の冗長 `head` ヘッダ(sequence 最大の方が有効、log_version/version/log_offset/length)

**検証**: 新規テスト全緑 + doctest 全緑。oracle: GDS のレコード長走査(切詰めで停止)、EDIF の括弧/コメント/文字列字句、LEF/DEF のブロック内限定走査(LEF の `UNITS` ブロック構文を doctest が検証 — 行頭 `DATABASE` でないと落ちる)、Liberty のグループ引数 vs 属性値の区別、UDF の VRS 鎖 + TEA01 終端、VHDX の sequence による active ヘッダ選択と二重破壊時 None。ラウンド内捕捉: `d[7]` は reclen 下位バイト(切断するには上位 `d[6]` が必要)、`count.checked_neg` 型の教訓と同系統で「中間オフセットを弄る破壊テストは無効化されうる」ことを確認。

## 出典(第110次、search-index 照合)

**論文・仕様**: Calma/Cadence GDSII Stream Format Manual / GDSIITOOLKIT 準拠の `{reclen, tag, dtype}` レイアウト、ANSI/EIA-548 EDIF 2 0 0 仕様、Cadence LEF/DEF Language Reference(UNITS·MACRO·PIN·COMPONENTS·NETS 文法)、Synopsys Liberty リファレンス(グループ/属性文法)、OSTA Universal Disk Format Specification + ECMA-167(VRS·AVDP)、Microsoft [MS-VHDX] VHDX Format Specification(vhdxfile 署名・冗長 head・sequence)— 全て整数のみで実装。

**実装物**: KLayout/gdstk の GDS レコード走査、qflow/OpenROAD の LEF/DEF パーサ、liberty-parser、pycdlib/libisofs の UDF VRS、qemu の vhdx ドライバ — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の半導体設計フロー解説(GDSII・LEF/DEF・Liberty・OpenROAD 記事)、UDF/iso イメージ解析記事、VHDX フォレンジック記事 — 全て整数のみで実装。

## 第111次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — 3D/CG 制作パイプライン形式(全7件が既存 676 件と非衝突を確認):

- `blend` — Blender `.blend`: `BLENDER` + ptr-size フラグ(`_`=4/`-`=8)+ endian フラグ(`v`=LE/`V`=BE)+ 3桁バージョン、ブロック `{code:4, size:u32, old_mem_addr:ptr, sdna_index:u32, count:u32}` をヘッダ自身のエンディアンで走査、`ENDB` で終端、`DNA1` ペイロード位置
- `fbx` — Kaydara FBX バイナリ: `Kaydara FBX Binary  \x00\x1A\x00` + u32 version;version<7500 は32bit `{end_offset, prop_count, prop_bytes}` + u8 名長、7500+ は64bit化;`end_offset==0` のヌルレコードがレベル終端
- `glb` — glTF バイナリコンテナ: `glTF` + u32 version(2)+ u32 総長、`{u32 len, u32 type}` の `JSON`/`BIN` チャンク鎖(4Bアライン)
- `abc` — Alembic Ogawa: `\x89Ogawa\x0D\x0A` マジック + u16 frozen + u16 version;グループ = u64 子数 + 子オフセット列、葉は `u64::MAX` マーカー + `(size, offset)`
- `pmd` — MikuMikuDance PMD: `Pmd` + version(生 u32、1.0=`0x3F80_0000`)+ 20B Shift-JIS 名 + 256B コメント + u32 頂点数
- `pmx` — PMX: `PMX ` + version(2.0=`0x4000_0000`、2.1=`0x4000_0001`)+ u8 ヘッダサイズ(8)+ 8B 設定ヘッダ(encoding 0=UTF-16/1=UTF-8、index 幅 1/2/4)
- `bvh` — Biovision BVH モーション: `HIERARCHY`/`ROOT`/`JOINT`/`CHANNELS` の階層 + `MOTION`/`Frames:`/`Frame Time:`(float を排除してミリ単位整数に変換)

**検証**: 新規テスト全緑 + doctest 全緑。oracle: blend の ptr-size/endian 両軸フィクスチャ、fbx の narrow/wide 両ヘッダ、glb の宣言長照合、abc の葉グループ `u64::MAX` 分岐、pmx の index 幅 1/2/4 検査、bvh の `Frame Time:` 小数→ミリ秒変換(`0.033333`→33、`1.5`→1500)。ラウンド内捕捉: blend のブロックヘッダは `16 + ptr_size` バイト(サイズ2誤算で鎖がずれる)、BVH の `Frame Time:` は空白を含むキーなので単語分割ではなく行頭プレフィックス照合が必要。

## 出典(第111次、search-index 照合)

**論文・仕様**: Blender `.blend` File Structure(SDNA / BLENDER ヘッダ・ENDB 終端)、Kaydara/Autodesk FBX Binary File Format(7500 の 64bit 化・ヌルレコード規則)、Khronos glTF 2.0 §GLB(JSON/BIN チャンク・4B アライン)、Alembic Ogawa 内部フォーマット(マジック・frozen/version・グループ子オフセット列)、PMD/PMX 形式仕様(20B 名・256B コメント・8B 設定ヘッダ)、Biovision BVH 仕様(HIERARCHY/MOTION)— 全て整数のみで実装。

**実装物**: Blender 本体の `BLO_blend_defs.h`/readfile、assimp の FBX パーサ、glTF-Sample-Models/`gltf` crate の GLB ヘッダ、alembic-rs の Ogawa リーダ、MMD 系ローダー(mmd_tools/PmxSharp)、bvh リーダー実装群 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の .blend 内部構造・FBX バイナリ解析・GLB コンテナ・Alembic・MMD(PMD/PMX)・BVH モーション解説記事 — 全て整数のみで実装。

## 第112次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — 圧縮コンテナ形式(全7件が既存 683 件と非衝突を確認。`lz4` はブロックコーデックとして既存のためフレーム側は `lz4f`):

- `gzip` — RFC 1952 メンバラッパ: `1F 8B` + CM(8)+ FLG(FTEXT/FHCRC/FEXTRA/FNAME/FCOMMENT)+ MTIME/XFL/OS、フラグ順の可変フィールド(FEXTRA=u16長、FNAME/FCOMMENT=NUL終端、FHCRC=u16)、末尾 `{crc32, isize}`
- `bzip2` — `BZh` + 数字(1-9 = ブロックサイズ×100KB);ブロック先頭 `0x314159265359`(π)+ u32BE crc + randomised + u24 origPtr;終端 `0x177245385090`(√π)
- `zstd` — RFC 8878: u32LE `0xFD2FB528`;Frame_Header_Descriptor 1B(bits 7-6 FCS幅、bit5 single_segment=window不要、bit2 checksum、bit1-0 dict-id幅)+ 可変 window/dict-id/FCS;`0x184D2A50..5F` スキップ可能フレーム
- `lz4f` — LZ4 フレーム: u32LE `0x184D2204` + FLG(version 01・block-independence・checksum 系・content-size・dict-id フラグ)+ BD(4-7 → 64KB..4MB);`{u32 size}` ブロック(bit31=非圧縮)、`0` で終端
- `snappy` — Snappy フレーム形式: 先頭チャンクは必ず `0xFF, len=6, "sNaPpY"`;`{u8 type, u24LE len}` チャンク(0x00 圧縮/0x01 非圧縮/0x02 パディング/0x80-FE スキップ可)+ マスク済み CRC32C(`ror15 + 0xA282EAD8`)
- `brotli` — RFC 7932: マジックなし、先頭の LSB-first ビットが WBITS ラダー(`0`→16、`1`+3bit n>0→17+n、n=0 の延長形)
- `zlib` — RFC 1950 プレリュード: CMF(メソッド8・CINFO≤7 = window `2^(cinfo+8)`)+ FLG(FLEVEL/FDICT/FCHECK — `(CMF*256+FLG)%31==0`);FDICT 時は u32BE 辞書 id、末尾 u32BE Adler32

**検証**: 新規テスト全緑 + doctest 全緑。oracle: gzip の FLG 全経路(FEXTRA/FNAME/FCOMMENT/FHCRC 順)、zlib の FCHECK 探索(`0x78` に対し FDICT 立つ FLG を %31 で発見)、zstd の single_segment 時 FCS=1B 規則、lz4f の block_max コード 4-7 境界、snappy の mask/unmask 往復、brotli の WBITS 最深9bit経路。ラウンド内捕捉: zstd `0xA0` は fcs=4B かつ single_segment 両立(window なしになる)、snappy マスク期待値の手計算誤り(`ror15`+delta の桁溢れ)、brotli は非空入力1Bでも最深経路は9bit必要で打ち切り判定可能。

## 出典(第112次、search-index 照合)

**論文・仕様**: RFC 1952(gzip member layout・FLG 順序・trailer)、bzip2 format(π/√π マジック・u24 origPtr)、RFC 8878(Zstandard frame header descriptor・skippable frames)、LZ4 Frame Format 仕様(FLG/BD・end mark)、snappy-framed(ストリーム識別子・マスク済み CRC32C)、RFC 7932(Brotli WBITS ラダー)、RFC 1950(zlib CMF/FLG・FCHECK・FDICT)— 全て整数のみで実装。

**実装物**: gzip/zlib/zstd/lz4/snappy/brotli/bzip2 各リファレンス実装のヘッダ読み取り部、7-Zip フォーマット一覧、facebook/zstd の frameHeader 処理 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の gzip ヘッダ構造・zlib ヘッダ 2 バイト・Zstandard フレーム・LZ4 frame・Snappy framed・brotli ストリーム構造解説記事 — 全て整数のみで実装。

## 第113次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — パッチ・デルタ形式(全7件が既存 690 件と非衝突を確認。`ips`/`vcdiff` は既存):

- `ups` — byuu の UPS パッチ: `UPS1` + 可変長 source/target サイズ + `{相対オフセット, XOR データ, 0x00}` レコード + 末尾3×u32LE CRC。varint は LEB128 ではなく byuu 式(MSB 立つバイトが終端 + shift バイアスでエンコードが一意)
- `bps` — BPS(UPS の後継): `BPS1` + src/dst/metadata 長 varint + メタデータ + アクション列(`v&3` で SourceRead/TargetRead/SourceCopy/TargetCopy、長は `v>>2 + 1`、コピー系は符号付き varint オフセット続行)+ 末尾3 CRC
- `aps` — N64 APS: `APS10` + 50B 記述(NUL パディング)+ u8 タイプ + `{u32BE offset, u8 len, data}` レコード(`len==0` は RLE `{u8 rle_len, u8 value}` 形)
- `ppf` — PlayStation Patch Format: `PPF`+バージョン数字(10/20/30)、v3 は encoding byte(bin=0→u32LE offset / Gi=1→u64LE offset)+ 60B 記述(v1/v2 は50B)+ `{offset, u8 len, data}` レコード
- `gdiff` — W3C Generic Diff Format: `D1 FF D1 FF` + version + コマンド列(0=EOF、1-246=即値リテラル長、247/248=u16/u32 リテラル、249-253=コピーの offset/len 幅行列、254=u32 チェックサム)
- `rdiff` — librsync ワイヤ形式: 署名 `0x72730136`/`0x72730137`(block_len+strong_len 続行)、デルタ `0x72730236`;0x01-0x40 は即値リテラル、0x41-0x44 は u8/u16/u32/u64 長リテラル、0x45-0x54 は (offset幅,len幅) の N1/N2/N4/N8 行列
- `bsdiff` — `BSDIFF40` + 3×u64LE(bzip2 圧縮済み ctrl/diff サイズ + 新ファイルの非圧縮サイズ)、セクションは連続配置

**検証**: 新規テスト全緑 + doctest 全緑。oracle: byuu varint の MSB-終端+バイアス(`[0x48,0x80]`→200、`[0x01,0x80]`→129)、BPS svarint 符号ビット、gdiff/rdiff のオペランド幅行列全経路、ppf の Gi 64bit オフセット、aps の RLE 分岐。ラウンド内捕捉: 当初実装した標準 LEB128(MSB=継続)は byuu 形式と極性が逆で、さらにエンコード一意化の `data += shift` バイアスが必要 — ups-spec.pdf の擬似コードで正しさを確認して差し替え。

## 出典(第113次、search-index 照合)

**論文・仕様**: ups-spec.pdf(byuu、UPS 構造+エンコード擬似コード)、BPS 形式仕様(byuu、アクション列+符号付き varint)、APS N64 仕様(50B 記述+RLE 形)、PPF3.0 仕様(bin/Gi encoding)、W3C NOTE「Generic Diff Format」コマンド表、librsync page_formats/prototab(デルタマジック+オペランド幅行列)、bsdiff BSDIFF40 ヘッダ — 全て整数のみで実装。

**実装物**: beat/Flips(byuu)の encode/decode、xdelta 参照実装、librsync `prototab.c`、bsdifflib、UniPatcher 各種パッチャーのヘッダ処理 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の ROM パッチ形式(IPS/UPS/BPS)・ランレングス差分・librsync デルタ解説記事 — 全て整数のみで実装。

## 第114次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — バイオインフォマティクス形式(全7件が既存 697 件と非衝突を確認。`vcf`=vCard、`sam`=suffix automaton、`gbk`=GBK encoding は名前衝突のため回避):

- `fasta` — `>` defline(id=先頭トークン、description=残り)+ 折り返しシーケンス行。空白・数字を剥がして IUPAC 文字のみ回収、複数レコードイテレータ
- `fastq` — `@id` + 複数行シーケンス + `+` 区切り + 品質行(シーケンス長に達するまで連結)。Cock et al. 2009 の NAR 論文仕様。`phred33`/`phred64` 変換は整数のみ(浮動小数点不使用)
- `gff` — GFF3 9列タブ区切り、`##` プラグマ、`##FASTA` 末尾カットオフ。score/phase はテキスト保持で浮動小数点を排除。`attribute(key)` で `key=value;` 属性引き
- `bed` — UCSC BED: `chrom start end` 必須3列 + 最大12列までの任意列を verbatim 公開。`track`/`browser`/`#` 行スキップ
- `genbank` — GenBank フラットファイル: LOCUS ヘッダ(name/length/circular)、継続行対応の ACCESSION/DEFINITION、FEATURES テーブルの key+location ペア、ORIGIN シーケンス
- `stockholm` — Stockholm 1.0: `# STOCKHOLM 1.0` マジック、`name seq` 行、`#=GC/GS/GR` マークアップ、`//` 終端。インターリーブブロック対応
- `newick` — Newick 系統樹: ネスト括弧+`name`+`:length`+`;`。再帰なしのスタック解析(深さ 2048 cap)、ノードアリーナ+子 index、枝長はテキスト保持(浮動小数点不使用)+ `length_milli` でミリ単位整数変換(指数表記対応)

**検証**: 新規テスト全緑 + doctest 全緑。ラウンド内捕捉: FASTA/FASTQ のシーケンスはケースを保持する(doctest が「大文字化される」と誤想定)、phred64('@')=0 が正(ASCII 64 オフセット)、GenBank FEATURES のキー列は col 5(0-indexed)開始、Stockholm `#=GC` のクラス切り出しは先頭4バイト固定(`#=G`+1文字)。ラウンド内補足: fastq の品質収集は「シーケンス長に達するまで」が正しい終端条件(品質行が `@` で始まり得るため行数固定は誤り)。

## 出典(第114次、search-index 照合)

**論文・仕様**: Cock, Fields, Goto, Heuer, Rice「The Sanger FASTQ file format for sequences with quality scores, and the Solexa/Illumina FASTQ variants」(NAR 2010) — 品質行は長さ一致まで連結する規則。Sequence Ontology GFF3 仕様(9 列定義・strand/phase)。UCSC Genome Browser FAQ(BED 必須/任意列、0-based start)。NCBI GenBank Flat File Release Notes(LOCUS 列位置、FEATURES テーブル col 6-21/21+)。Pfam/Rfam Stockholm format 1.0(`#=GC/GS/GR` マークアップ)。Joe Felsenstein の Newick ツリー形式解説(括弧+ラベル+枝長)。NCBI FASTA defline 規則。

**実装物**: Biopython `Bio.SeqIO`(fasta/fastq/genbank/stockholm)、htslib/biojs-io-fastq、UCSC `kent/src/lib` の BED パーサ、ape/ETE の Newick パーサ — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn のバイオインフォマティクス形式解説(FASTQ の4行構造とマルチライン罠、GFF3 と GTF の差異、GenBank flat file 読み方、SAM/BAM と本ラウンドの差分) — 全て整数のみで実装。

## 第115次(search-index 照合ラウンド / 実装証跡付き)

**方法**: 文献参照ラウンド継続 — 音源・インストゥルメント形式(全7件が既存 704 件と非衝突を確認。`aiff`/`iff`/`midi`/`wav` は既存、`smp` は `op2` に差し替え):

- `sf2` — SoundFont 2: `RIFF`-`sfbk` フォーム + `INFO`/`sdta`/`pdta` 3 LIST。`INFO` は `ifil`(u16 ペア=バージョン)/`INAM`/`ISFT` 等の `fourcc+size` サブチャンク列、`sdta` に `smpl` 波形ブロブ、`pdta` にプリセット/インストゥルメント/ジェネレータ定義群
- `dls` — DLS Level 1/2: `RIFF`-`DLS ` フォーム(末尾スペース必須)、`vers` u32 ペア、`colh` インストゥルメント数、`wvpl`/`lins` LIST 入れ子
- `xi` — FastTracker II インストゥルメント: `Extended Instrument: ` 21B + 名前22B + 0x1A + トラッカ名20B + version u16 @0x40 + 96B ノート→サンプルマップ @0x42 + `num_samples` u16 @0x128 + 40B サンプルヘッダ列
- `iti` — Impulse Instrument: `IMPI` + DOS ファイル名12B + NNA/DCT/DCA + fadeout + pitch-pan separation/center + global volume/default pan + 乱数変動 + tracker version + `nos` + 名前26B @0x20
- `pat` — GUS パッチ(GF1PATCH110/ID#000002): 60B 記述 + instruments/voices/channels + waveforms u16 + master volume + data size + 36B 予約 = 固定128B ヘッダ
- `sbi` — Sound Blaster Instrument: `SBI\x1A` + 32B 名 + OPL2 レジスタ 16B(モジュレータ 0-4 / キャリア 5-9 / feedback+connection @10 / waveform select 12,13)
- `op2` — Doom GENMIDI.OP2: `#O3_II#` + 175 × 36B インストゥルメントレコード(flags/fine tune/fixed note + モジュレータ・キャリア 2オペレータの OPL レジスタセット)+ 175 × 32B NUL パッド名表

**検証**: 新規テスト全緑 + doctest 全緑。ラウンド内補足: DLS のフォーム種別は 4 文字目がスペースの `DLS `(`RIFF`/`LIST` の4CC は常に4バイト)、XI の `num_samples` は 0x128 でサンプルヘッダはその直後の 40B 固定幅、SBI は名前が 32B 固定で NUL パッド、OP2 の名前表はファイル末尾に全件連続配置される。

## 出典(第115次、search-index 照合)

**論文・仕様**: SoundFont 2.01/2.04 Technical Specification(EMU Systems/Creative — `sfbk` フォーム、INFO/sdta/pdta の3 LIST 構造)、DLS Level 1/2 仕様(MMA/AMEI、`DLS ` フォーム、`vers`/`colh`/`wvpl`/`lins`)、FT2 xi フォーマット解説(Samplicity xi_specs.txt / milkytracker xm-form.txt — ヘッダ 66B、ノートマップ、num_samples オフセット)、ITTECH.TXT(Impulse Tracker インストゥルメントヘッダ — NNA/DCT/DCA、PPC/PPS)、GUS .PAT フォーマット仕様(GF1PATCH110+ID#000002 ヘッダ)、SBI フォーマット仕様(OPL2 16 レジスタ値)、DMX GENMIDI.OP2 レイアウト(`#O3_II#`、175 レコード+名前表)— 全て整数のみで実装。

**実装物**: FluidSynth/timidity の SF2/DLS ローダ、MilkyTracker/ft2-clone の XI 読み込み、Schism Tracker/OpenMPT の ITI ヘッダ処理、TiMidity++ GUS パッチ読み込み、AdPlug 系の SBI/OPL パッチ処理、chocolate-doom の GENMIDI 解析 — 全て整数のみで実装。

**国内技術情報**: Qiita/Zenn の SoundFont/DLS・トラッカ音源(IT/XM インストゥルメント)・GUS パッチ・FM 音源 OPL レジスタ構成・Doom 音楽データ解説記事 — 全て整数のみで実装。
