# izanagi_kit — 機能過不足の監査リスト (Feature Audit: Sufficiency / Excess / Deficiency)

> **この文書の目的**: izanagi_kit の全機能を「充足 / 過剰(却下済み) / 解消済みの不足 /
> 意図的な未実装 / 残存する未実装」の5分類に選別した、**自己完結のハンドオフ監査文書**。
> **対象は kit のみ** — エンジン本体（`izanagi_v4.0.2.zip`）を含む製品全体の監査は
> [`../PRODUCT_AUDIT.md`](../PRODUCT_AUDIT.md) を参照。
> 前提知識ゼロの読者（将来の Claude セッション、新規コントリビュータ）が、この1ファイルだけで
> 「何があり・何が無く・なぜ無いのか」を把握できるように書かれている。
>
> **執筆規則**（本文全体で遵守）:
> - 未定義の略号を使わない。過去文書のコード（G20, W8 等）は使わず、内容を書き下す。
> - 全項目に「**何が** / **どこに**（ファイルパス） / **なぜ**」を含める。
> - 「実装済み」「意図的未実装（理由付き）」「未実装（残課題）」を明確に区別する。
> - 決定論影響タグ: 🟢 replay-safe（追加しても既存リプレイを壊さない）/
>   🟡 gated（feature 隔離や新 API なら安全）/ 🔴 breaking（pinned hash の更新が必要）。
>
> **検証コマンド**（本文書の主張は全て再検証可能）:
> ```
> cargo test                 # 全 suite green（lib 2710+ / property 259+ tests）
> cargo test --test determinism   # PINNED_FINAL_HASH = 0xd1a9236e96a2c802 の bit-exact 検証
> grep -c "^pub mod " src/lib.rs  # モジュール数 = 77
> ```
>
> 最終更新: 2026-07-02 / ブランチ: `claude/deepresearch-ultrathink-improve-yq2th` /
> 詳細な経緯は [`STRENGTHS_WEAKNESSES.md`](./STRENGTHS_WEAKNESSES.md)（棚卸し履歴）・
> [`RESEARCH.md`](./RESEARCH.md)（外部出典調査）・[`GAME_DEV_TAXONOMY.md`](./GAME_DEV_TAXONOMY.md)（能力地図）を参照。

---

## 1. 製品概要 (What this product is)

izanagi_kit は **zero-dependency・`#![forbid(unsafe_code)]` の決定論的（lockstep-safe）
Rust ゲームエンジンキット**。ターミナル/ヘッドレスのローグライクを対象とし、77 の独立モジュール
（`src/lib.rs` の `pub mod` 一覧）で構成される。設計の中心は **bit-exact replay**:
シミュレーションは整数/固定小数点のみ（float 禁止）、乱数は単一の seeded SplitMix64、
状態は FNV-1a checksum（`world_hash`）で毎 tick 検証でき、`tests/determinism.rs` が
`PINNED_FINAL_HASH = 0xd1a9236e96a2c802`、`tests/roguelike_sim.rs` が
`PINNED_ROGUELIKE_HASH = 0x5286d1420200fe66` を固定して回帰を検出する。
この2つの pinned hash を壊さないことが、全変更の受け入れ条件である。

---

## 2. 充足 (Sufficient — implemented capabilities)

ゲーム開発に必要な能力を16カテゴリに分類し、それぞれの実装モジュールを示す。
全項目が実装済み・テスト済みである（各モジュールの契約は [`SPEC.md`](./SPEC.md)）。

| カテゴリ | 能力 | 実装モジュール（`src/` 直下） |
|---|---|---|
| A. 時間とループ | 固定タイムステップ（補間 alpha・death-spiral ガード付き）、タイマー/クールダウン、エネルギー制ターンスケジューラ（行動順の非破壊 forecast 付き） | `timestep`, `timer`, `turn` |
| B. 数学 | Q16.16 固定小数点（sqrt・CORDIC 三角関数・lerp/clamp/signum）、整数幾何（Bresenham 線分・LOS・距離）、固定小数点ベクトル、easing 曲線 | `fixed`, `geometry`, `vec`, `easing` |
| C. 状態とデータ (ECS) | 世代付きエンティティハンドル、sparse-set storage（多コンポーネント join）、archetype table、変更検知、親子関係（transform 伝播付き） | `entity`, `sparse_set`, `arch`, `change`, `relations` |
| D. 乱数 | 決定論 PRNG（SplitMix64、bias 無し range 抽出、退化入力は draw を消費しない契約）、ダイス記法、整数 value noise、重み付き抽選テーブル（入れ子対応）、非復元抽出バッグ | `rng`, `dice`, `noise`, `random_table`, `shufflebag` |
| E. 決定論・リプレイ | FNV-1a 状態チェックサム（`DetHash` trait）、リプレイ記録・desync 特定・snapshot 再シミュレート（rollback 基盤） | `world_hash`, `replay` |
| F. 表示・描画 | ヘッドレスセルバッファ（24-bit ANSI・差分描画）、ワールド↔スクリーンカメラ | `terminal`, `camera` |
| G. 入力 | キー→アクション対応表、決定論コマンドキュー、長押し/リピート検出バッファ | `keymap`, `cmdqueue`, `inputbuf` |
| H. コンテンツ・アセット | テキスト DSL（`.game` 形式）のパース→検証→ECS ロードのパイプライン、canonical 直列化（round-trip property 検証済み）、CLI ゲート（`--fmt`/`--check`/`--json`）、アセットハンドル | `parser`, `content`, `validator`, `loader`, `serializer`, `assets`, `bin/gamec` |
| I. ワールド・マップ | 手続き生成4種（rooms+corridors / cellular caves / BSP / drunkard's-walk、全て連結保証）、Wave Function Collapse（backtrack 付き）、多層タイルマップ、bitmask オートタイル、複数フロア | `mapgen`, `wfc`, `tilemap`, `autotile`, `multimap` |
| J. 視界・AI・ナビ | 対称 shadowcasting FOV、A* / weighted A* / JPS、Dijkstra map + flee map + auto-explore、influence map、flat/階層 FSM、behavior tree、fog-of-war | `fov`, `pathfinding`, `influence`, `fsm`, `hfsm`, `behavior`, `visibility` |
| K. 物理・衝突 | グリッド通行判定、AABB 重なり、空間ハッシュ broadphase | `passability`, `aabb`, `spatial_hash` |
| L. ゲームプレイ | 整数戦闘式、型付きダメージ+耐性、インベントリ、装備（呪い/ロック対応）、状態異常、スキル（mana/CD/射程統合）、脅威テーブル、資源プール、XP/レベル、クロスラン恒久進行、アイテム識別、通貨/ショップ、会話ツリー、条件→アクション trigger、クエスト、レシピ、派閥、暦、照明、encounter/affix 生成、tween（逐次チェーン付き） | `combat`, `damage`, `inventory`, `equipment`, `status`, `ability`, `threat`, `pool`, `progression`, `meta`, `identify`, `wallet`, `shop`, `dialogue`, `trigger`, `quest`, `recipe`, `faction`, `calendar`, `lightmap`, `encounter`, `affix`, `tween` |
| M. UI | リングバッファメッセージログ、メニュー、テキスト折返し/整形、HUD 部品 | `msglog`, `menu`, `textlayout`, `hud` |
| N. 永続化 | バージョン付き・checksum 付きバイナリセーブ枠（schema migration 対応） | `savefile` |
| O. ネットワーク（ロジック層） | マルチプレイヤー入力予測・誤予測検出（transport 非依存、rollback と接続） | `netinput` |
| P. ツール・デバッグ | コンテンツ検証 CLI、機械可読 JSON 診断、tick プロファイラ、イベントキュー | `bin/gamec`, `diag_json`, `profiler`, `eventqueue` |

---

## 3. 過剰の検討結果 (Excess — examined and rejected)

「一見冗長・重複に見える機能」を個別に検討し、いずれも**削除・統合すべきでない**と結論した。
理由ごと記録する（将来「これ重複では？」と再検討する際の先回りの回答）。

| 疑った組 | 疑問 | 却下理由（なぜ冗長でないか) |
|---|---|---|
| `fsm` / `hfsm` / `behavior` | AI 抽象化が3つは多すぎないか | 各々が異なる設計原理: flat な状態遷移表 / 親状態+ワイルドカード遷移の階層 FSM / 木構造の逐次評価（sequence/selector）。ゲーム AI 文献で標準的に区別される別ツールであり、統合すると各々の単純さが失われる |
| `dialogue` / `quest` / `trigger` | フロー制御が3つは重複では | 会話ナビゲーション（テキスト+選択肢のグラフ）/ タスク完了の監視（カウンタと閾値）/ 汎用の条件→アクション規則、と担う形が異なる。`trigger` の docstring 自体が他2者との関係を明示している |
| `meta::MetaProgress` と `identify::Identification` | どちらも「冪等フラグ集合」— 型を共有すべきでは | ライフサイクルが正反対: MetaProgress は**死んでも消えない**（permadeath を跨ぐ）、Identification は**毎ランでリセット**される。共有すると型がライフサイクルを表現できなくなる |
| `recipe` の逆用（分解/salvage の新モジュール） | アイテム→素材の「分解」に専用型が要るのでは | `Recipe<T,T>`（材料1種→産物1種）で一般的な分解は既に表現できる。多品目産出の分解が必要になるまで新設は過剰 |
| calendar 駆動 NPC スケジュール専用モジュール | NPC の日課（朝は店、夜は家）に専用型が要るのでは | `calendar`（時刻）+ `fsm`/`hfsm`（状態）の合成で content 層から表現可能。エンジン側の新型は過剰 |

**結論: 削除すべき過剰機能はゼロ。** 77 モジュールは多いが、各々が単一責務で直交している。

---

## 4. 不足として発見・解消済み (Deficiencies — found and fixed)

過去の棚卸しで「既存 API では表現できない」と特定し、実装済みのギャップ。
形式:「何が欠けていた → どこに実装 → テスト数（u=unit, p=property）」。

### ゲームプレイの軸

| 欠けていたもの | 実装先 | テスト |
|---|---|---|
| 型付きダメージと耐性/弱点（火耐性で被ダメ減等） | `src/damage.rs` | 19u |
| 状態異常を戦闘式へ畳む結線（buff/debuff → StatsModifier） | `src/status.rs` 拡張 | 9u+doc |
| 深度連動のグループ遭遇生成（goblin×3+shaman×1） | `src/encounter.rs` | 11u+doc |
| 手続き的アイテム接辞（"Rusty Sword of Dragonslaying"） | `src/affix.rs` | 14u+doc |
| スキル統合（mana/クールダウン/射程を1呼び出しで判定） | `src/ability.rs` | 26u |
| behavior tree（sequence/selector/invert/repeat） | `src/behavior.rs` | 30u |
| 「今誰を狙うか」の脅威/アグロテーブル（taunt 付き） | `src/threat.rs` | 20u+5p |
| 有界再生リソース（mana/stamina/空腹、負の regen=毒） | `src/pool.rs` | 19u+5p |
| XP→レベルの恒久成長曲線（閉形式・2分探索逆写像） | `src/progression.rs` | 14u+5p |
| **permadeath を跨ぐ**恒久アンロックと歴代最高記録 | `src/meta.rs` | 19u+5p |
| シード毎にシャッフルされる未識別アイテムの見た目 | `src/identify.rs` | 18u+5p |
| 外せない呪われた装備（is_locked 問い合わせ+冪等 curse） | `src/equipment.rs` 拡張 | +13u+5p |
| 代替可能通貨の残高と原子的送金 | `src/wallet.rs` | 16u+5p |
| 商品×価格表×店の資金（買い/売りが Wallet::transfer 1回に帰着） | `src/shop.rs` | 19u+5p |
| NPC 会話の分岐ツリー（RNG 無しの純カーソル） | `src/dialogue.rs` | 12u+5p |
| 汎用の条件→アクション規則（once/repeatable、発火は key 昇順） | `src/trigger.rs` | 18u+5p |
| 非復元抽出の自動補充バッグ（Tetris 7-bag 型） | `src/shufflebag.rs` | 11u+6p |

### アルゴリズム・基盤の軸

| 欠けていたもの | 実装先 | テスト |
|---|---|---|
| flee/safety map（負係数 rescan 付き、袋小路で死なない逃走） | `src/pathfinding.rs::flee_map` | 5u+2p |
| 自動探索（未踏フロンティアへの最短路、壁を偽検出しない） | `src/pathfinding.rs::auto_explore` | 5u+1p |
| drunkard's-walk 洞窟生成（単一連結を構成的に保証） | `src/mapgen.rs::generate_drunkard` | 7u+3p |
| 行動順タイムラインの非破壊プレビュー（initiative bar 用） | `src/turn.rs::Scheduler::forecast` | 5u+1p |
| 複数 tween の単一クロック逐次再生（tick 繰越付き） | `src/tween.rs::TweenSequence` | 15u+4p |
| **マルチプレイヤー**入力予測と誤予測検出（rollback のトリガー判定。到着順に依存しない max-fold で out-of-order 配送に安全） | `src/netinput.rs` | 21u+5p |
| 複数モジュール横断の E2E リプレイ検証（wallet+shop+dialogue の複合状態に record/check/resimulate を適用） | `tests/economy_integration.rs` | 4×150 trials |
| フォーマット検証の CI ゲート（`cargo fmt --check` 相当） | `src/bin/gamec.rs` `--check` | 動作確認済み |

**実装時に発見・修正した決定論バグ**（性質テストが実装ミスを検出した実例）:
`netinput` の初版は「最後に confirm された値」で予測元を無条件上書きしており、古い tick の
入力が遅延到着すると新しい値を巻き戻す到着順依存バグがあった。property test
（confirm 順序の非依存性）が即座に検出し、tick の max-fold に修正した。

---

## 5. 意図的な未実装 (Deliberate non-goals — 実装しない理由が確定しているもの)

以下は「不足」ではなく**方針として実装しない**もの。実装を検討する前にこの理由を確認すること。

| 機能 | 実装しない理由 |
|---|---|
| `DetHash` の derive macro | proc-macro は別クレート依存を追加するため zero-dependency 方針と衝突する。手動 `impl DetHash` で完全に代替可能（全77モジュールがそうしている）。方針変更時のみ再検討 |
| コンテンツのホットリロード | OS のファイル監視 I/O が必要で、ヘッドレス/zero-dep 方針の範囲外。`parser`→`loader` の再実行は呼び出し側でいつでも組める |
| ネットワーク transport（ソケット送受信） | ソケット I/O は同じく範囲外。**ただし** transport 非依存の予測/補正ロジックは `src/netinput.rs` で実装済み — 呼び出し側は受信バイトを `confirm()` に渡すだけでよい |

---

## 6. 残存する未実装 (Remaining open items — 着手候補の残課題)

コード上に存在しないことを grep で確認済みの、真の残課題。優先度順ではなく分野順。

| 残課題 | 現状 | 決定論影響 |
|---|---|---|
| ~~named RNG streams / jump-ahead~~（サブシステム毎に独立乱数列） | ✅ **実装済み** — `SplitMix64::split(stream_id)`（parent state と stream_id を avalanche mix で合成する純粋関数、parent を消費・変更しない。7 unit + 5 property tests） | 🟢 追加 API のみ、既存 draw 挙動は不変 |
| ~~xoshiro256++ 等の代替 PRNG~~ | ✅ **実装済み（隔離）** — `rng_xoshiro::Xoshiro256pp`（2²⁵⁶ 周期・高統計品質、SplitMix64 で seeding、`jump()` で並列ストリーム）。既定の `SplitMix64` は不変・どの draw path にも未結線なので pinned hash 不変。手計算リファレンスベクタ（state `[1,0,0,0]`→出力 `8388609`・遷移 `[1,1,1,0]`）で正確性を pin。13 unit + 4 property tests | 🟢 opt-in 新型のみ（既定・pinned hash 不変） |
| ~~order-independent set hashing（可換 combine で sort 不要化）~~ / xxHash 高速化オプション | ✅ **order-independent 側は実装済み** — `world_hash::hash_unordered`（各要素を `hash_state`→avalanche mix→`wrapping_add` で合成する permutation-invariant な multiset hash。ソート不要で HashMap 等の非正準順コンテナを安定 hash 化。7 unit + 4 property tests）。既存 `hash_state`/`DetHash` は不変の純粋追加。xxHash 高速化は別件で未着手 | 🟢 新 API のみ（既存 hash 値・pinned hash 不変） |
| クロス OS/arch CI マトリクスで `PINNED_FINAL_HASH` 一致検証 | CI workflow はリポジトリルート `.github/workflows/` に必要（GitHub の検出仕様）。determinism-matrix ジョブ（Linux/macOS/Windows で pinned hash テスト実行）を含む ci.yml を用意済みだが、実行環境の GitHub App トークンに `workflows` 権限が無く push 不可 — API 経由の配置を試行中、駄目なら手動配置が必要 | 🟢 テスト基盤のみ |
| coverage-guided fuzzing（cargo-fuzz、parser/savefile 対象） | ⚠️ **試行済み・環境制約で未達** — `cargo-fuzz` は nightly toolchain を要求するが、サンドボックスの `rustup toolchain install nightly` がネットワーク制限で失敗。nightly が使える環境での再挑戦が必要 | 🟢 dev-only 依存（未実装の理由は方針でなく環境） |
| ~~SARIF 診断出力~~（GitHub アノテーション連携） | ✅ **実装済み** — `src/diag_json.rs::diag_sarif`、`gamec --sarif`（14 unit tests、実 JSON parser で構造検証済み） | 🟢 ツール層 |
| ~~`Fixed` の丸めモード明文化~~ | ✅ **実装済み** — module doc に "Rounding" 節を追加し実挙動を明文化（`mul` は arithmetic `>>` で −∞ 方向 floor、`div`/`from_ratio` は整数除算で toward-zero truncate、負値でのみ相違）。`to_int_trunc` が名前に反して −∞ floor である misnomer を発見・docstring で明示。4 regression tests で pin（挙動不変） | 🟢 文書 + テストのみ（挙動不変・pinned hash 不変） |
| ~~`.game` 形式の BNF 形式仕様~~・incremental/partial parse | ✅ **BNF 実装済み** — `SPEC.md` §9.1 に実装と 1:1 対応する形式 EBNF 文法＋語彙/境界規則を追加、`parser.rs` module doc から相互参照。incremental/partial parse は引き続き未実装（全体再パースで用途は充足） | 🟢 文書のみ（挙動不変） |

---

## 7. 横断サマリ (Cross-reference summary — 1行索引)

| 分類 | 件数 | 内容 |
|---|---|---|
| **充足** | 77 modules / 16 カテゴリ | 第2節の表。全カテゴリ（時間/数学/ECS/乱数/決定論/描画/入力/コンテンツ/マップ/AI/衝突/ゲームプレイ/UI/永続化/ネット予測/ツール）をカバー |
| **過剰 → 却下** | 5組検討・削除ゼロ | fsm/hfsm/behavior・dialogue/quest/trigger・meta/identify・recipe逆用・NPCスケジュール。全て「冗長でない」と理由付きで確認済み（第3節） |
| **不足 → 解消済み** | 25+ 項目 | 第4節の2表。全て実装+テスト済み、pinned hash 不変 |
| **意図的未実装** | 3件 | derive macro（依存追加）/ ホットリロード（OS I/O）/ transport（ソケット I/O）。理由確定済み（第5節） |
| **残課題** | 8件 | 第6節の表。RNG streams・代替 PRNG・hash オプション・CI マトリクス・fuzzing・SARIF・丸め文書化・BNF。全て決定論影響タグ付き |

**読み取り方（次の一手の選び方）**: 🟢 の残課題（CI マトリクス・fuzzing・SARIF・文書化）は
いつでも安全に着手できる。🟡/🔴 は feature flag か新 API として隔離し、
`PINNED_FINAL_HASH = 0xd1a9236e96a2c802` / `PINNED_ROGUELIKE_HASH = 0x5286d1420200fe66` を
必ず保護すること。新モジュールを追加する場合の作法: BTreeMap/BTreeSet で canonical order、
`DetHash` 実装、退化入力で RNG draw を消費しない、unit + property tests、
`src/lib.rs` の doc list と `pub use` への登録、README のモジュール表更新。
