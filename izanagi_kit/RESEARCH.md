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
