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
3. **PRNG 品質の自動テスト**（小規模 chi-square / 既知ベクタ回帰）。出典: PractRand/TestU01 文献。🟢 replay-safe。
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
4. **death-spiral ガードの可観測化**（dropped tick 数のメトリクス）。🟢 replay-safe。
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
5. **修正提案（quick-fix）**（未定義参照に近傍候補を提示）。出典: rustc suggestions。🟢 replay-safe。
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
| H48 | **`#[should_panic]` の `expected=` 必須化を凍結** — 全6サイトは既に `expected=` 付きだが、無印の混入を拒否する検査がなく wrong-panic で緑になる弱テストを新規追加できた。全 .rs(src+tests+examples、テスト領域を含む全文)で `#[should_panic` 行に `expected` を要求。自己言及は test_code で潰す(初版は自コメントが needle に一致して失敗 — 走査自身が検査対象の中に住む問題) |
| H49 | **DetHash フィールド感度を property 化** — ゴールデンピンは1インスタンスの定点検査のみ: フィールドを読まなくなった impl はピンの fixture が変わらなければ緑のまま。相補契約「各フィールドを個別に摂動すればハッシュが動く」を Vec2/Vec3/Aabb/Dice/Camera/Entity(index+generation)/Cooldown/Fixed に凍結。変異(Vec2::y 除去)で発火確認 — ピンと両層で検出 |
