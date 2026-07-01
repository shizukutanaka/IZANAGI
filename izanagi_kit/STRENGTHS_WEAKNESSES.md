# izanagi_kit — 長所・短所・不足機能の戦略的棚卸し (Strengths / Weaknesses / Gaps)

> 目的: `/goal このプロダクトの長所短所不足してる機能を洗い出し、実装` に応えるための
> **製品レベル（module 横断）の戦略評価**。`RESEARCH.md`（カテゴリ別の外部出典調査）と
> `IMPROVEMENTS.md`（確定バグ修正ログ）の上位に位置する索引で、
> 「次に実装すべき機能」を効果と工数で優先順位づけする。
>
> 比較対象: bracket-lib, bevy_ecs, rot.js, ratatui, EnTT/FLECS。
> 制約（全項目共通）: zero runtime dependency / `#![forbid(unsafe_code)]` /
> no float in sim / `PINNED_FINAL_HASH` と `PINNED_ROGUELIKE_HASH` を壊さない /
> 新規コードは各機能 3 テスト以上。
>
> 最終更新: 2026-07-01 / ブランチ: `claude/deepresearch-ultrathink-improve-yq2th`

---

## 1. 長所 (Strengths) — 同種ツールに対する差別化点

| # | 長所 | 代表ファイル | なぜ強いか |
|---|------|-------------|-----------|
| S1 | **決定論的 state hashing** (FNV-1a/64, little-endian 固定) | `src/world_hash.rs` | `hash_state(&world)` が全プラットフォームで bit 一致。bracket-lib / rot.js には無い replay checksum を標準提供。 |
| S2 | **replay / rollback ハーネス** | `src/replay.rs` | `record_trace` / `check_trace` / `resimulate` で desync を tick + hash 単位で特定。bevy_ecs には replay API が無い。 |
| S3 | **エッジケースまで決定論な PRNG** | `src/rng.rs` | `below(0)` / `range(lo>=hi)` / `coin(0,_)` が *draw を消費せず* 確定値を返す → debug/release で replay がずれない。 |
| S4 | **death-spiral ガード付き fixed-timestep** | `src/timestep.rs` | Fiedler パターン（accumulate → clamp catch-up）を標準化。1 秒スタックが 60 frame の暴走 catch-up にならない。 |
| S5 | **テキスト DSL → 検証 → ECS のコンテンツパイプライン** | `src/parser.rs`, `content.rs`, `validator.rs`, `loader.rs` | panic-free・全エラー一括報告・BTreeMap/Vec の確定順序。rot.js/ratatui に asset pipeline は無い。 |
| S6 | **zero-dependency + forbid(unsafe)** | `src/lib.rs:56`, `Cargo.toml` | ~50 module が std のみ・unsafe ゼロ。determinism-critical 層に外部コードの版差が混入しない。 |
| S7 | **sparse-set / archetype の 2 系統 ECS** | `src/sparse_set.rs`, `src/arch.rs` | 用途で storage を選べる（疎な component は sparse-set、多 component 反復は cache-friendly な `ArchTable<Row>`）。 |
| S8 | **A* / weighted A* / JPS の 3 段経路探索** | `src/pathfinding.rs` | 同一 no-corner-cut モデルで最適（A*）・近似高速（weighted）・厳密高速（JPS）を選択可能。JPS は A* と cost 厳密一致を 6000 ランダム盤面で metamorphic 検証済み（rot.js/bracket-lib は JPS 非標準）。 |

## 2. 短所 (Weaknesses) — 設計上の制約・欠落

| # | 短所 | 位置 | 工数 |
|---|------|------|------|
| W1 | ~~**多 component クエリ API が無い**~~（呼び出し側が手で N 重ループ） ✅ **実装済み**（`join3` / `join3_mut` で3コンポーネント同時クエリ、6 tests + doc test） | `src/sparse_set.rs` | Medium |
| W2 | ~~**generation overflow 検出が弱い**~~（`wrapping_add(1)`、2³² 再利用で stale handle 復活の理論リスク） ✅ **実装済み**（`EntityAllocator::generation_wrap_count()` で wrap 回数を追跡、saturating カウンタ） | `src/entity.rs` | Small |
| W3 | ~~**terminal の入力抽象が無い**~~ ✅ **実装済み**（`KeySource` trait + `ListKeySource` + `InputBuffer::pump_from`、6 tests + doc test） | `src/inputbuf.rs` | Medium |
| W4 | ~~**save file の schema migration 基盤が無い**~~ ✅ **実装済み**（`Migrator` trait + `load_bytes_migrated` + `LoadError::MigrationFailed`、5 tests + doc test） | `src/savefile.rs` | Medium |
| W5 | ~~**WFC の contradiction からの部分解抽出/backtrack が最小限**~~ ✅ **実装済み**（`wfc_solve_backtrack` / `wfc_solve_partial`） | `src/wfc.rs` | Small |
| W6 | ~~**relations が transform 伝播しない**~~（親移動で子が追従しない） ✅ **実装済み**（`Relations::propagate` BFS 位相順 + `root_entities`、7 tests） | `src/relations.rs` | Medium |
| W7 | ~~**FSM が flat**~~（階層状態・遷移ペイロード無し） ✅ **実装済み**（`HFsm<S,E>`: `with_parent` / `on` / `on_any` builder、`fire` が ancestor chain + wildcard 順に検索、`is_in` で substate 判定、25 tests） | `src/hfsm.rs` | Large |

## 3. 不足機能 (Missing Features) — peer 比較での欠落と工数

| # | 機能 | 現状 | 工数 | 状態 |
|---|------|------|------|------|
| G1 | **damage type / resistance**（火耐性・弱点で被ダメ増減） | `combat` は scalar + flat `apply_resistance` のみ | Small | ✅ **実装済み**（`src/damage.rs`, 本コミット） |
| G2 | **status effect ↔ combat 統合**（時限 buff/debuff を戦闘式に反映） | `status.rs` 単独、combat 非連携 | Small | ✅ **実装済み**（`StatTarget` + `StatusSet::stats_modifier` / `dot_total`） |
| G3 | **nested / tiered loot table**（「種別 → その種別の loot」入れ子） | `random_table` は flat | Small | ✅ **実装済み**（`RandomTable::roll_nested` / `roll_nested_owned`） |
| G4 | **encounter pack 生成**（深度/難度で「goblin×3 + shaman×1」） | 個体 pick のみ | Small | ✅ **実装済み**（`src/encounter.rs`: `EncounterPack` / `EncounterSlot`） |
| G5 | **multi-floor 遷移パス探索**（floor A→B を stairs 経由で） | `multimap` は connector lookup のみ | Medium | ✅ **実装済み**（`MultiMap::find_floor_path` / `floor_distance` / `is_floor_reachable`） |
| G6 | **stairs 連結の自動検出/チェイン** | 手動 Connector 追加 | Small | ✅ **実装済み**（`MultiMap::link_floors` 双方向ペア追加） |
| G7 | **item affix / enchantment 生成** | `random_table` は値のみ | Medium | ✅ **実装済み**（`src/affix.rs`: `Affix` / `AffixedItem` / `AffixGenerator`） |
| G8 | **behavior tree / GOAP / utility AI** | `fsm` は flat | Large | ✅ **実装済み**（`src/behavior.rs`: `BehaviorTree<A>` / `BehaviorNode<A>` / `BehaviorStatus`、sequence/selector/invert/repeat/succeed/fail + action/condition leaves、DetHash、30 tests） |
| G9 | **unified ability/skill system**（mana/cooldown/range/effect 結線） | `timer`+`fsm`+`combat` を手結線 | Large | ✅ **実装済み**（`src/ability.rs`: `AbilitySet<K,E>` + `Ability<E>` + `AbilityResult<E>`、cooldown/mana/range 統合、DetHash、26 tests） |
| G10 | **per-combatant threat / aggro table**（敵対中に「今誰を狙うか」） | encounter 個別には持たない | Medium | ✅ **実装済み**（`src/threat.rs`: `ThreatTable<K>` — BTreeMap backed、add/reduce/decay/taunt、最小キー tie-break、DetHash、20u + 5p tests） |
| G11 | **bounded regenerating resource pool**（mana/stamina/hunger） | `Pool` 抽象化なし | Small | ✅ **実装済み**（`src/pool.rs`: 有界 i32、add/drain/restore、符号付き regen（正=再生、負=減衰）、DetHash、19u + 5p tests） |
| G12 | **eased time-driven value interpolation**（D ティック区間での補間） | easing 曲線のみ・状態なし | Medium | ✅ **実装済み**（`src/tween.rs`: `Tween` — Q16.16 Fixed、advance/reset/reverse、curve 非保持（fn pointer 決定論不可）、DetHash、15u + 6p tests） |
| G13 | **fungible currency / shop wallet**（gold/gem の残高） | inventory は discrete items のみ | Small | ✅ **実装済み**（`src/wallet.rs`: `Wallet<C>` — u64 balances in BTreeMap、withdraw all-or-nothing、transfer atomic、DetHash、16u + 5p tests） |
| G14 | **branching dialogue tree**（NPC 会話の分岐） | fsm/quest は general state/task | Medium | ✅ **実装済み**（`src/dialogue.rs`: `Dialogue` + `DialogueNode` + `Choice` — 無 RNG・純 cursor navigation、terminal node 判定、out-of-range safe、DetHash、12u + 5p tests） |

## 4. 本イテレーションの実装 (Implemented this pass)

**G1 — typed damage & resistance profile** → `src/damage.rs`（新規 module, 19 tests）

- `DamageType` enum: `Physical / Fire / Cold / Lightning / Poison / Arcane / True`
  （`#[repr(u8)]`、`ALL` 固定順、`index`/`from_index` ラウンドトリップ、`DetHash`）。
- `ResistanceProfile`: 型ごとの耐性%を固定長 `[i32; 7]` で保持（HashMap 不使用＝順序非決定性なし）。
  - `new / uniform / with`（builder）/ `get / set / add(saturating)` / `is_immune / is_vulnerable`。
  - `apply(damage, ty)`: `True` は素通し；それ以外は `max(0, dmg×(100−resist)/100)`、
    resist は上限 100 で clamp（過剰免疫が回復にならない）、負値は弱点として被ダメ増幅。
    中間計算は `i64` で overflow なし。
  - `DetHash` 実装＝creature の耐性が replay checksum に畳み込まれる。
- **互換性検証**: resist ∈ [0,100] で `combat::apply_resistance` と一致するテストを同梱
  （2 システムが予測可能に合成できることを保証）。

決定論影響: 🟢 replay-safe（整数のみ・固定順・float なし）。既存 sim は本 module を未使用のため
`PINNED_FINAL_HASH = 0xd1a9_236e_96a2_c802` / `PINNED_ROGUELIKE_HASH = 0x5286_d142_0200_fe66` 不変。

## 6. 本セッションの実装（G10-G14）

**G10 — per-combatant threat / aggro table** → `src/threat.rs`（新規 module, 20u + 5p tests）

- `ThreatTable<K>`: BTreeMap<K, i32> backed、zero-threat entries pruned。
- `add / reduce / set / remove`、flat decay + per-mille decay（cool-off）。
- `top_target()`: 最大威脅値の source を返す。**タイ解決は最小キーで決定論的**（挿入順非依存）。
- `taunt(src, margin)`: src を top へ強制（tank pull）。
- **ソクラテス的ギャップ**: `faction` は「集団同士が敵対か」、`influence` は「危険はどこか」を答えるが、
  「敵対中に**今誰を狙うか**」の軸が欠落 → 同値タイを最小キー優先で解決して replay-safe に。

**G11 — bounded regenerating resource pool** → `src/pool.rs`（新規 module, 19u + 5p tests）

- `Pool`: u32 max, i32 current & regen_per_tick。current は常に [0, max]。
- `spend()` all-or-nothing、`drain()/restore()` は報告 delta。
- 符号付き regen: 正=再生、負=減衰（毒・飢え）。
- **ソクラテス的ギャップ**: `combat::Stats` は HP のみ、`ability` はマナ管理を呼び出し側に委譲
  → 有界・毎ティック再生（減衰含む）の汎用リソース。

**G12 — eased time-driven value interpolation** → `src/tween.rs`（新規 module, 15u + 6p tests）

- `Tween`: Fixed start/end、u32 duration/elapsed。`value(curve_fn)` で easing 曲線を sample。
- curve は **非保持**（fn pointer は deterministic hash 不可）→ `recipe` 同様に呼び出し側で supply。
- `value()` は overshooting curve を [0,1] に clamp、`value_overshoot()` は preservation。
- `advance() / reset() / reverse()`（ping-pong）。
- **ソクラテス的ギャップ**: `easing` は曲線、`Fixed::lerp` は端点補間を提供するが、
  「D ティック中の**今**の緩急値」を保持する状態が無い。

**G13 — fungible currency / shop wallet** → `src/wallet.rs`（新規 module, 16u + 5p tests）

- `Wallet<C>`: BTreeMap<C, u64> backed、zero-balance entries pruned。
- `deposit() / withdraw()` all-or-nothing、`transfer()` atomic（両側同時更新 or 両側無変更）。
- **ソクラテス的ギャップ**: item層は完備（inventory store、equipment wear、recipe transform、affix enchant）
  だが、**代替可能な通貨**（gold/gem/token）の軸が無い → 原子的 transfer・通貨保存則。

**G14 — branching dialogue tree** → `src/dialogue.rs`（新規 module, 12u + 5p tests）

- `Dialogue + DialogueNode + Choice`: ノードは text + choices の vec。choice は label + target index。
- 実行時状態は単一 cursor `Option<usize>`（None 時は ended）。**RNG 不使用・完全に replay-safe**。
- `choose(i)` / `goto(node)` / `end()`、terminal node（choice 0 個）は自動判定。
- **範囲外は安全に拒否**: choice index out-of-range、target out-of-range → state 不変で false を返す（panic しない）。
- **ソクラテス的ギャップ**: `fsm`/`hfsm` は汎用 AI state machine、`quest` はタスク完了を追う
  だが、「ノードがテキスト+選択肢を持ち各選択肢が次ノードへ遷移する」という**会話特有の形**を専用には扱えない。

---

決定論影響: 🟢 全て replay-safe（整数 / fixed-point のみ、float 無し、RNG 無し or 決定論的、canonical order）。
プロパティテスト: **218件**（G1-G9: 193件 → +25件で新 G10-G14）。
`PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変。

## 5. 次の推奨着手順（効果 × 低工数）

1. ~~**G2 status↔combat 統合**~~ ✅ 実装済み（`StatTarget` enum +
   `StatusSet::stats_modifier(target_of)` で `combat::StatsModifier` に畳み込み、
   `dot_total(is_dot)` で poison/burn の per-tick ダメージ合算。9 tests + doc test）
2. ~~**G3 nested loot table**~~ ✅ 実装済み（`RandomTable<RandomTable<T>>` に
   `roll_nested` / `roll_nested_owned`。draw 数決定論: 空 outer は draw なし、
   非空 outer は 1 draw + inner 1 draw。6 tests + doc test）
3. ~~**G4 encounter pack**~~ ✅ 実装済み（新規 `src/encounter.rs`:
   `EncounterPack<T>` — slot 毎の count range + 出現確率%、挿入順 roll、
   draw 数決定論（degenerate chance / 固定 count は draw なし）、`DetHash`、
   `min_spawns`/`max_spawns` 境界、`roll`/`roll_counts`。11 tests + doc test）
4. ~~**G5 multi-floor pathfinding**~~ ✅ 実装済み（`MultiMap::find_floor_path`:
   connector graph 上の BFS、挿入順展開で同長経路のタイブレークも決定論。
   `floor_distance` / `is_floor_reachable` 付随。out-of-range / dangling
   connector は安全にスキップ。10 tests）
5. ~~**G6 stairs 連結チェイン**~~ ✅ 実装済み（`MultiMap::link_floors`:
   下り階段＋帰り階段の双方向 Connector ペアを 1 呼び出しで追加）

6. ~~**G7 item affix**~~ ✅ 実装済み（新規 `src/affix.rs`: `Affix<M>`
   （prefix/suffix + modifier payload）、`AffixedItem<T, M>`（`display_name`
   → "Rusty Sword of Dragonslaying"、`combined_modifier` で `StatsModifier`
   飽和合算 → `Stats::modified` に直結）、`AffixGenerator<M>`（weighted pool
   + 付与確率%、固定 draw 順序: prefix coin → prefix roll → suffix coin →
   suffix roll、degenerate は draw なし）。`DetHash` 完備。14 tests + doc test）

## 7. 現段階の弱点と次の改善候補

### 7.1 短所（現段階）

| # | 短所 | 影響 | 工数 |
|---|------|------|------|
| W8 | ~~**Integration test 欠落**~~（複数 module 横断の E2E テストが無い） ✅ **実装済み**（`tests/economy_integration.rs`: wallet+dialogue+shop の合成セッションで record_trace/check_trace/resimulate を通貨・会話状態の複合ハッシュに適用、4 tests × 150 trials） | wallet + dialogue + shop pricing の連携が未検証 | Medium |
| W9 | ~~**複数 tween の同時再生管理がない**~~（animation sequencer） ✅ **実装済み**（`src/tween.rs`: `TweenSequence` — 単一クロックで複数 `Tween` を順序再生、tick 繰越、15u + 4p tests） | UI/visual FX は複数並行アニメを必要とするが、個別管理が煩雑 | Medium |
| W10 | ~~**shop pricing model**~~（buy/sell markup、NPC ごとの価格設定） ✅ **実装済み**（`src/shop.rs`: `Shop<K,C>` + `Listing` — wallet-backed till、`buy`/`sell` all-or-nothing、19u + 5p tests） | wallet の基盤は整備されたが、実際のショップ仕組みが無い | Small |
| W11 | ~~**trigger / event script**~~（条件→アクション チェーン）✅ **実装済み**（`src/trigger.rs`: `TriggerSet<K,C,A>` + `Trigger<C,A>` — 条件はデータとして保持し評価関数を呼び出し側から供給、once/repeatable、18u + 5p tests） | dialogue の結果（choice）を quest や world state 変化に紐づける基盤が無い | Large |
| W12 | ~~**README.md が実装に追いついていない**~~ ✅ **実装済み**（module 表に `ability`/`behavior`/`hfsm`/`aabb`/`spatial_hash`/`passability` を追加、`gamec --check`/`--json` を CLI 節に追記、戦略文書4本への「Project documents」導線を新設） | G1-G14 の機能一覧が記載されていない | Small |

**W10 — shop pricing model** → `src/shop.rs`（新規 module, 19 unit + 5 property tests）

- `Listing { buy_price, sell_price }` + `Shop<K,C>`: `BTreeMap<K, Listing>` の価格表 + `Wallet<C>` の till。
- `buy(buyer, item)` / `sell(seller, item)`: いずれも `Wallet::transfer` を1回呼ぶだけの all-or-nothing トランザクション
  （未リスト商品・買い手の残高不足・till の資金不足はいずれも両ウォレット無変更で `false`）。
- `stock() / drain_till() / till_balance()`: 既存 `Wallet` の deposit/withdraw をそのまま再利用（重複実装なし）。
- **ソクラテス的ギャップ**: `wallet` は atomic transfer を提供するが「商品と価格を結びつける」層が無かった
  → 価格表を挟むだけで買い/売りの両方向が同一 primitive（`transfer`）に帰着することを確認。
- 検証した性質: buy/sell は `can_buy`/`can_sell` と succeed/fail が一致、成功時は buyer+till（または
  seller+till）の合計通貨が保存、買って売る往復でも合計保存、未リスト商品は残高に関わらず常に失敗、
  `DetHash` はリスト挿入順に非依存・価格や till 残高の変化に敏感。

決定論影響: 🟢 replay-safe（整数のみ・`Wallet::transfer` の atomic 性を再利用・float/RNG 無し）。
`PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変（`tests/determinism.rs` 確認済み）。

**W8 — wallet + dialogue + shop integration test** → `tests/economy_integration.rs`（4 tests × 150 trials）

- `ShopSession { wallet, shop, talk }`: 3 module を game-loop 側で結線（module 間の直接依存は無し）。
  `dialogue` の選択（買う/売る/立ち去る）を受けて `apply()` が `shop.buy`/`shop.sell` を呼び、
  結果（成功/失敗）に応じて `Dialogue::goto` で応答ノードへ遷移する——`replay_integration.rs` と
  同型の「複数 module の相互作用が真の replay リスクを生む」ケースを実モジュールで再現。
- `record_trace` / `check_trace` / `resimulate`（`src/replay.rs`）をそのまま複合状態（`Wallet`+`Shop`+
  `Dialogue` を連結 `DetHash`）に適用: (1) 同一 seed+選択列 → bit-identical trace、(2) tick K の選択を
  変えると divergence が tick K 以前で検出、(3) 中間 snapshot からの `resimulate` が非中断実行と同一
  最終 hash に到達し、snapshot 自体は不変。
- 4本目のテストは3 module の**不変条件の複合**を検証: どの分岐を通っても wallet+till の合計通貨は
  保存され、dialogue カーソルは常に greeting（次ラウンド待ち）か ended のいずれかに落ち着く
  （3 module が互いを不整合な状態に置き去りにしない）。
- **ソクラテス的ギャップ**: 各 module 単体のユニットテストは「自分の契約」しか検証できず、
  「A の成功/失敗が B の分岐を決め、その複合状態が replay-safe か」という**結線点**は無検証だった。

決定論影響: 🟢 replay-safe（新規テストのみ・実装コード変更なし）。
`PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変（`tests/determinism.rs` 確認済み）。

**W9 — animation sequencer** → `src/tween.rs::TweenSequence`（新規 type, 15 unit + 4 property tests）

- `TweenSequence { steps: Vec<Tween>, current: usize }`: 単一クロックで複数 `Tween` を順序再生。
  `advance(ticks)` は現在ステップに ticks を投入し、完了したら**余った tick を同じ呼び出し内で
  次ステップへ繰り越す**（1回の大きな advance が短いステップを複数またいで正しく早送りされる）。
- **並行再生（同時に複数アニメ）は新型不要**——`Vec<Tween>` + `iter_mut().for_each(|t| t.advance(dt))`
  で既に表現可能（W9 の元記述が明記）。今回のギャップは**単一クロックの逐次連結**のみだった
  （歩行アニメの複数フレーム、カットシーンの複数区間、スライドイン→ホールド→フェードアウト）。
- `value(easing)` は現在ステップの eased 値、完了後は最終ステップの終端値で安定
  （カーソルは配列末尾を超えて進まない）。`total_duration()` / `elapsed_total()` / `progress()` で
  チェーン全体の進捗をバー表示等に提供。`reset()` は全ステップ+カーソルを初期状態へ。
- ゼロ duration の中間ステップは無限ループにならず1回のループでスキップされることをテストで確認。
- **ソクラテス的ギャップ**: `Tween` 単体は「1区間の今の値」を持つが、「複数区間を順に、tick を
  跨いで繰り越しながら」再生する状態機械が無かった。
- 検証した性質: 分割 advance（x then y）== 一括 advance（x+y）の結果一致（`Tween::advance` の
  加法性をチェーン境界を跨いでも保証）、カーソルは常に `[0,n)` に収まり `is_done` は
  `elapsed_total >= total_duration` と一致、`reset()` は新規構築と完全一致、`DetHash` は
  カーソル位置の変化に敏感。

決定論影響: 🟢 replay-safe（整数のみ・既存 `Tween`/`Fixed` の再利用・float/RNG 無し）。
`PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変（`tests/determinism.rs` 確認済み）。

**W12 — README.md 更新** → `README.md`

- module 表に `ability`/`behavior`/`hfsm`（skill system・behavior tree・hierarchical FSM の3大systemが
  完全に未記載だったギャップ）と `aabb`/`spatial_hash`/`passability`（衝突判定層）を追加。
- `gamec` CLI 節に既存実装済みだが未記載だった `--check`/`--json` フラグを追記。
- 「Project documents」節を新設し、`STRENGTHS_WEAKNESSES.md`/`RESEARCH.md`/`IMPROVEMENTS.md`/
  `CHANGELOG.md` への導線を追加（従来 README から一切リンクされていなかった）。
- 21 examples の一覧・モジュール表の整合性を `lib.rs` の `pub mod` 一覧と突き合わせて検証済み。

**W11 — trigger / event script** → `src/trigger.rs`（新規 module, 18 unit + 5 property tests）

- `Trigger<C, A> { condition: C, actions: Vec<A>, once: bool }` + `TriggerSet<K, C, A>`:
  `BTreeMap<K, Trigger<C,A>>` の canonical ルール集合 + 発火済み one-shot key の `BTreeSet`。
- `tween`/`recipe` と同じ脱結合方針: 条件・アクションは**データ**として保持し、
  評価関数は `check<F: Fn(&C) -> bool>` に**呼び出し時に**供給する（関数ポインタは非保存＝
  `DetHash` 安全）。`TriggerSet` はどのルールが発火したかだけを返し、アクションの実行（dispatch）は
  呼び出し側の責務のまま——「発火判定」と「発火の意味」の境界を明示的に保つ。
- `check()` は armed なルールを昇順 key で評価し、発火した `(key, actions)` を返す。once ルールは
  発火後に disarm され、`reset`/`reset_all` で再武装可能。
- **ソクラテス的ギャップ**: `dialogue` は選択、`quest` はタスク完了、`status`/`eventqueue` は時限/即時
  効果を扱うが、「任意の条件が成立したらアクション列を実行する」という汎用ルール層が無かった。
- 検証した性質: repeatable trigger は条件が真の間毎回発火・偽なら決して発火しない、once trigger は
  何度 check しても厳密に1回だけ発火、`reset_all` は全 once ルールを同時再武装、発火順は挿入順に
  非依存で常に key 昇順、`DetHash` は挿入順に非依存・発火状態とルール内容（アクション列）の変化に敏感。

決定論影響: 🟢 replay-safe（整数のみ・条件/アクションはデータ・関数ポインタ非保存・float/RNG 無し）。
`PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変（`tests/determinism.rs` 確認済み）。

**W8–W12 の全 weakness items は本ブランチで解消済み。**

## 8. 外部知見（Qiita / Zenn）に基づく改善

Qiita / Zenn の調査で、ローグライク AI の古典技法として **Dijkstra マップ（脅威マップ / 誘導マップ）** が
繰り返し言及されていた（"Game AI: Dijkstra's algorithm is used to create threat maps and item-targeting
maps"）。本 kit には `dijkstra_map` / `descend` は既存だが、**flee map（safety map）** が欠けていた。

**G15 — flee/safety map（rescan 付き）** → `src/pathfinding.rs::flee_map`（5u + 2p tests）

- **問題**: `descend` の docstring は「flee by descending its negation」と述べていたが、
  RogueBasin "The Incredible Power of Dijkstra Maps" が指摘する通り、Dijkstra マップを単純に
  負化して降下すると、**行き止まりに追い込まれて停止**する愚かな逃走になる（local minimum 問題）。
- **解法**: desire マップに負係数（`coeff_num/coeff_den`、例 `12/10` = 1.2）を掛け、
  **再スキャン(rescan)** = octile コスト・no-corner-cut で fixpoint まで緩和。これにより
  「ソースから遠ざかりつつ、壁を迂回して開けた空間へ逃げる」勾配が再生成される。
- **決定論**: cell を `(x,y)` 昇順で緩和、値は単調減少で下界あり → `|cells|` パス以内で収束。
  整数のみ（i64 中間計算で overflow なし）。replay-safe。
- **検証**: 行き止まり脱出テスト（naive 負化なら失敗するケース）、局所一貫性
  (`value <= neighbour + step`)、降下の cycle-free 終了をプロパティテストで保証。
- 出典: RogueBasin（古典技法）+ Qiita/Zenn のダイクストラ法ゲーム AI 記事群で再確認。

決定論影響: 🟢 replay-safe。`PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変。
プロパティテスト: **220件**。

**G16 — drunkard's walk（穴掘り法）ダンジョン生成** → `src/mapgen.rs::generate_drunkard`（7u + 3p tests）

- **調査根拠**: Qiita/Zenn のローグライク生成記事群で「穴掘り法」が古典手法として繰り返し言及
  （gis 氏の C++ シリーズ、Python/Unity 実装等）。本 kit は room-placement / cellular cave / BSP の
  3 種を持つが、**単一エージェントが乱歩で掘る** drunkard's walk が欠けていた。
- **特徴**: digger が中心から開始し毎ステップ 1 cardinal 方向を引いて移動・床化。連続移動なので
  **常に単一 4-connected 領域**を保証（`generate_cave` のような cull 後処理が不要）。
- **決定論**: 固定中心開始、1 step = 1 draw、interior `[1,w-2]×[1,h-2]` に clamp（壁沿いに滑る）。
  `max_steps` で必ず終了。replay-safe。
- **検証**: 全 20 seed で full-connectivity、fill 上限不超過、border 維持、tiny-map 安全をテスト。
- 出典: RogueBasin "drunkard's walk" / bracket-lib + Qiita/Zenn 穴掘り法記事群。

決定論影響: 🟢 replay-safe。`PINNED_*_HASH` 不変。プロパティテスト: **223件**。

**G17 — turn-order timeline（行動順プレビュー）** → `src/turn.rs::Scheduler::forecast`（5u + 1p tests）

- **調査根拠**: Qiita/Zenn のターン制バトル設計記事群で「行動順タイムライン / ATB / 速度順」が頻出
  （RPGツクールMZ の agility order、ポケモン型ターン処理、ビヘイビアグラフ AI 等）。
  既存 `Scheduler` は `peek_next_turn`（直近1体）のみで、**次のN体の順序プレビュー**が無かった。
- **機能**: `forecast(n)` が次の n ターンを非破壊シミュレートし、行動する actor id を順序付きで返す。
  Final Fantasy Tactics / Into the Breach 型の initiative-bar UI に直結。速い actor は window 内で複数回登場。
- **決定論**: `next_turn` と同一の time-advance ルール・smallest-id tie-break を private コピー上で再現。
  scheduler 状態は不変。`forecast(n)` == `next_turn` を n 回呼んだ結果と byte 一致（プロパティで保証）。
- 出典: RPGツクール/ATB 設計 + Qiita/Zenn ターン制バトル記事群。

決定論影響: 🟢 replay-safe。`PINNED_*_HASH` 不変。プロパティテスト: **224件**。

**G18 — auto-explore（自動探索）** → `src/pathfinding.rs::auto_explore`（5u + 1p tests）

- **調査根拠**: Qiita/Zenn のローグライク開発記事群でグリッド管理・探索済みマップ・敵の経路探索が頻出。
  ローグライク定番の「自動探索（NetHack travel / DCSS explore）」コマンドは既存 `nearest_reachable` +
  `VisibilityMap.is_explored` で部分的に組めるが、**フロンティア検出を含む専用ヘルパー**が無かった。
- **機能**: `auto_explore(start, is_blocked, is_explored)` が、**探索済み・通行可能セルのみを通って**
  最も近い「未踏破フロンティア」（探索済み通行可能セルで、**未探索かつ通行可能**な隣接セルを持つもの）
  までの最短経路を返す。`None` は完全探索済み（探索完了）。recipe 同様にクロージャで `VisibilityMap` から脱結合。
- **設計の肝**: フロンティア判定が「未探索 *かつ通行可能*」隣接を要求するため、壁・盤外を誤検出しない
  （これを誤ると盤端セルが全て偽フロンティアになる — 実装中に発見・修正）。
- **決定論**: frontier を `(cost,x,y)` 順、parent は厳密改善時のみ設定 → 経路が安定。octile コスト・no-corner-cut。
- 出典: NetHack/DCSS auto-explore + Qiita/Zenn ローグライク開発記事群。

決定論影響: 🟢 replay-safe。`PINNED_*_HASH` 不変。プロパティテスト: **225件**。
G8 は本ブランチで `src/behavior.rs` として実装済み。
Small/Medium の全ギャップ（G1–G8）は本ブランチで解消済み。

G9（ability system）は Large・別スコープ確認の上で着手。
**W1–W7 の全 weakness items および G1–G9 の全 missing features は本ブランチで解消済み。**
全ての追加コードは `#![forbid(unsafe_code)]`・zero runtime dependency・no float・`PINNED_FINAL_HASH`/`PINNED_ROGUELIKE_HASH` 不変 の制約を満たす。

---

## 6. 第2次棚卸し (2026-06-22) — randomness と item 層の隙間

G1–G9・W1–W7 を埋めた後、ソクラテス式問答で残る **層間の隙間** を再走査した。
「既存 API では表現できないこと」を起点に2つの欠落を特定・実装。

| # | 問い（既存 API で表現できないこと） | 隙間 | 状態 |
|---|------------------------------------|------|------|
| G10 | 「補充されるバッグ」抽選——`random_table` は復元抽出（drought 発生）、`sample_n` は一回限り。Tetris の 7-bag やドラフ無し loot を表す型が無い。 | 非復元・自動補充の bag randomizer | ✅ **実装済み**（`src/shufflebag.rs`: `ShuffleBag<T>`） |
| G11 | 「装備中の防具一式の合計ステータス補正」——`Inventory<T>` は *保管* のみ、`StatsModifier`/`affix` は *単品の補正* を記述するが、「スロット別に着用→合計 modifier を `combat::Stats` に畳む」層が無い。 | body slot 別 loadout + 合計補正 | ✅ **実装済み**（`src/equipment.rs`: `Equipment<T>` / `EquipSlot`） |

**G10 — ShuffleBag** → `src/shufflebag.rs`（新規 module, 11 unit + 6 property tests）
- 1 cycle = template の置換（重複は多重度で保持）、空になると自動補充。
- size-1 bag は RNG draw を消費しない（`SplitMix64::below` の退化契約に整合）→ replay state が draw 回数の決定論的関数のまま。
- `DetHash` を template と live bag の双方に実装。

**G11 — Equipment loadout** → `src/equipment.rs`（新規 module, 13 unit + 5 property tests）
- `EquipSlot` 固定 enum 9 スロット（MainHand/OffHand/Head/Body/Hands/Feet/Ring1/Ring2/Amulet）、`[Option<T>; 9]` 固定長配列（HashMap 不使用＝順序非決定性なし）。
- `equip`（occupied なら旧装備を返す swap）/ `unequip` / `aggregate(modifier_of)`（全スロットを canonical 順に `StatsModifier::combine` で飽和合算→`Stats::modified` に直結）。
- 付随改善: `combat::StatsModifier` に `combine`（飽和 field-wise 和、単位元 = `default`）と `DetHash` を追加（装備・affix・buff のスタックを replay checksum に畳む）。
- `DetHash` は占有フラグ + 各装備で実装、スロット配置の違いも hash に反映。

決定論影響: 🟢 両 module とも replay-safe（整数のみ・固定順・float なし）。既存 sim は未使用のため
`PINNED_FINAL_HASH = 0xd1a9_236e_96a2_c802` / `PINNED_ROGUELIKE_HASH = 0x5286_d142_0200_fe66` 不変（`tests/determinism.rs` で確認済み）。

---

## 7. 第3次棚卸し (2026-06-23) — 「恒久成長」という新しい軸

ソクラテス式問答で **既存モジュールが扱っていない概念軸** を探索した。
既存は *空間*（map/fov/path）・*時間の刻み*（timestep/turn/timer）・*瞬間の状態*（combat/status/equipment）
を扱うが、「**時間をかけたキャラクターの恒久成長**（experience / leveling）」という軸が完全に欠落していた。

| # | 問い（既存 API で表現できないこと） | 隙間 | 状態 |
|---|------------------------------------|------|------|
| G12 | 「モンスターを倒して経験値を蓄積し、閾値でレベルアップ」——`combat::Stats` は瞬間値、`StatsModifier` は一時補正のみ。XP→level の写像が無い。 | 経験値曲線とレベル算出 | ✅ **実装済み**（`src/progression.rs`: `Progression` / `LevelCurve`） |

**G12 — Progression / leveling** → `src/progression.rs`（新規 module, 14 unit + 5 property tests）
- `LevelCurve { base, step, max_level }`: 等差の per-level コスト（`L→L+1` = `base + step·(L-1)`）。
  累積 XP は閉形式 `xp_to_reach(L) = (L-1)·base + step·(L-1)·(L-2)/2`（`u128` で計算し `u64` に飽和、テーブル不要）。
  `level_at(total_xp)` は単調性を使った2分探索で閾値の逆写像。
- `Progression`: XP 蓄積・レベル算出。`add_xp`（飽和加算、複数レベル同時上昇に対応、獲得レベル数を返す）、
  `xp_into_level` / `xp_to_next` / `is_max_level` / `with_xp`。
- 検証した代数法則（property tests）: 閾値ラウンドトリップ `level_at(xp_to_reach(L))==L`、
  境界 `level_at(t-1)==L-1`、XP 単調 ⟹ level 単調、`add_xp` の XP 保存と level の純関数性、
  level 内会計 `xp_into_level + xp_to_next == cost_of_level_up`。
- `DetHash` 実装＝キャラクター成長を replay checksum に畳む。

決定論影響: 🟢 replay-safe（整数のみ・`u128` 中間計算で overflow なし・float なし）。
既存 sim 未使用のため PINNED hashes 不変（`tests/determinism.rs` 確認済み）。

---

## 9. 第4次棚卸し (2026-07-01) — マルチプレイヤー決定論という新しい軸

W8–W12（節7）を埋めた後、`GAME_DEV_TAXONOMY.md` を再走査して残存ギャップを確認した。全カテゴリ中、
実質的な未実装は3件のみ: **E5**（`DetHash` derive macro — zero-dep 方針では手実装維持も可、と
明記済みの任意項目）、**H6**（ホットリロード — OS I/O でヘッドレス方針上**意図的に範囲外**）、
**O2/O3**（ネットワーク入力同期）。O2（transport 自体）はソケット I/O で同じく範囲外だが、
**O3（予測/補正のロジック）は transport 非依存に実装可能**で、既存の `replay::resimulate`
（rollback 基盤）と組み合わせて初めて意味を持つ——「シングルプレイヤーの決定論は完備だが、
マルチプレイヤーの決定論（複数プレイヤーの入力が非同期に届く状況での予測と誤り訂正）」という
軸が完全に欠落していた。

| # | 問い（既存 API で表現できないこと） | 隙間 | 状態 |
|---|------------------------------------|------|------|
| G20 | 「プレイヤー B の tick 5 の入力がまだ届いていない——それでも tick 5 を進めるには？届いた後、予測が外れていたら？」——`replay::resimulate` は snapshot から再生する *手段* を持つが、「いつ・どのプレイヤーの・どの tick で」再生すべきかを判定する層が無い。 | 決定論的マルチプレイヤー入力予測・誤予測検出 | ✅ **実装済み**（`src/netinput.rs`: `NetInputBuffer<P,I>`） |

**G20 — 決定論的マルチプレイヤー入力予測** → `src/netinput.rs`（新規 module, 21 unit + 5 property tests）

- `NetInputBuffer<P,I>`: `confirmed: BTreeMap<(tick,P), I>`（確定入力）+ `predicted`（`input_for` が
  行った予測の memo）+ `last_known: BTreeMap<P,I>`（各プレイヤーの最新確定入力＝予測の元）。
- `input_for(tick, player)`: 確定済みならそれを返し、未確定なら `last_known` から予測して memo 化
  （以後同じ `(tick,player)` への呼び出しは同じ予測を返す）。
- `confirm(tick, player, input)`: 本物の入力が届いた時に呼ぶ。既に予測されていた値と食い違えば
  `true`（誤予測——呼び出し側は `replay::resimulate` で `tick` から再シミュレートする）を返す。
- **決定論バグを発見・修正**: 実装直後の property test（confirm 順序が最終状態に影響しないこと）が
  失敗。原因は `last_known` を「直近の `confirm` 呼び出し」で無条件上書きしていたため、**古い tick
  の入力が遅れて届くと新しい tick の入力を巻き戻してしまう**（実ネットワークの reorder で普通に起こる）
  という真正のバグだった。`last_known_tick: BTreeMap<P,u32>` で各プレイヤーの最高確定 tick を追跡し、
  「`confirm` された tick が既知の最高値以上の時のみ `last_known` を更新」という **max-fold**（`threat`
  の decay や他モジュールの可換演算と同じ「到着順に依存しない集約」パターン）に修正して解消。
- **`DetHash` は `predicted` を意図的に除外**: 2ピアが同じ確定履歴を持っていても、ネットワーク
  タイミング差で「今この瞬間に何を予測中か」は異なりうる——これをハッシュに含めると、実際には
  同期が壊れていないのに偽の divergence を報告してしまう。`confirmed` + `last_known`（＝同じ確定
  情報を得れば必ず収束する部分）のみを正準状態とした。
- **ソクラテス的ギャップ**: `replay` は「巻き戻して再生する」手段を提供するが、「いつ再生すべきか」
  （どの tick でどのプレイヤーの予測が外れたか）を判定する層が無かった。`cmdqueue` は単一ローカル
  プレイヤーの決定論的入力供給を扱うが、複数プレイヤー・非同期到着は範囲外だった。
- 検証した性質: `confirm` の呼び出し順序が最終状態（`confirmed`・`last_known`・`DetHash`）に影響
  しない、`input_for` は確定値があれば必ずそれを返す（予測が先にあっても）、誤予測フラグは
  「予測値 ≠ 確定値」と厳密に一致、`prune_before` は境界を跨がず・`last_known` を保護、`DetHash` は
  `predicted` のみの差では変化せず `confirmed`/`last_known` の差には敏感。

決定論影響: 🟢 replay-safe（整数/データのみ・`predicted` を意図的にハッシュ対象外・float 無し）。
transport 自体は範囲外のまま——受信バイトから `confirm()` を呼ぶのは呼び出し側の責務。
`PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変（`tests/determinism.rs` 確認済み）。

これで `GAME_DEV_TAXONOMY.md` の実装可能な項目（zero-dep・ヘッドレス方針と両立するもの）は
**すべて解消**。残る ⬜ は E5（任意）・H6（範囲外）・O2（範囲外、transport のみ）の3件で、
いずれも明示的に「今は実装しない」理由が文書化されている。

---

## 10. 第5次棚卸し (2026-07-01) — ソクラテス式問答による過不足の再点検

`GAME_DEV_TAXONOMY.md` の枠外（タクソノミー自体の死角）を探すため、改めてソクラテス式問答で
「過剰（redundant）」と「不足（missing）」の両面を問い直した。

**過剰の検討**: 「`fsm`/`hfsm`/`behavior` の3つの AI 抽象化は冗長か？」「`dialogue`/`quest`/`trigger`
の3つの『フロー制御』は重複しているか？」——いずれも **否**。前者は状態遷移（FSM）・階層状態
（HFSM）・木構造の逐次評価（behavior tree）という異なる設計原理を持つゲームAI文献の標準的な
区別であり、後者も「会話ナビゲーション」「タスク完了追跡」「汎用条件→アクション」と役割が
明確に異なる（`trigger` の docstring 自体が他2者との関係を明示）。→ 冗長性は見つからなかった。

**不足の検討**: 「`progression`（XP→レベル）は**1回の生存内**の成長を扱うが、ローグライクの
ジャンル的核心である **permadeath（死んでも続く要素）** の裏側を表現できるか？」

| # | 問い（既存 API で表現できないこと） | 隙間 | 状態 |
|---|------------------------------------|------|------|
| G21 | 「死んでも失われない、恒久的なアンロックと歴代最高記録」——`progression` はキャラクター1体の生存内成長、`savefile` は汎用バイナリ永続化の*手段*だが、「idempotent なアンロックフラグ集合」「到着順に依存しない歴代最高値」という*データ構造*が無い。`wallet`/`quest` も単一 run スコープ。 | クロスラン meta-progression（恒久アンロック・歴代記録） | ✅ **実装済み**（`src/meta.rs`: `MetaProgress<K,R>`） |

**G21 — MetaProgress（クロスラン meta-progression）** → `src/meta.rs`（新規 module, 19 unit + 5 property tests）

- `unlocked: BTreeSet<K>`（恒久アンロックフラグ）+ `records: BTreeMap<R, i64>`（統計名ごとの歴代最高値）。
  Rogue Legacy の継承強化・Hades の Mirror of Night・Dead Cells の細胞通貨・NetHack のハイスコア表と
  同じ形——「死んでもリセットされない少数の idempotent な状態」を1つの型に集約。
- `unlock(feature)`: 冪等（2回目以降は `false` を返すだけで状態不変）。
- `record_best(stat, value)`: 「大きい方が良い」の max-fold（`netinput` の `last_known_tick` と同じ
  到着順非依存パターン）。「小さい方が良い」記録（最速クリア等）は呼び出し側が値を negate して渡す
  ことで単一メソッドのまま両対応——2つ目の easily-misused メソッドを増やさない設計判断。
- **意図的にライフサイクル非関与**: 「run とは何か」「いつ始まり終わるか」は一切知らない。呼び出し側が
  1つの `MetaProgress` インスタンスをセッション全体で保持し、死亡時は per-run 状態（キャラ・所持品・
  ダンジョン）だけを破棄・再構築する——という運用を、型を汚さずに支える。
- 検証した性質: unlock 順序は最終集合/ハッシュに非依存、`record_best` は「与えられた値集合の真の
  最大値」に順序非依存で収束、`record_best` の戻り値は「厳密に前回のベストを上回ったか」と正確に
  一致、unlock の繰り返しは1回と等価（冪等性）、`DetHash` は却下された（記録を更新しない）
  `record_best` 呼び出しでは変化せず、実際の内容変化にのみ敏感。

決定論影響: 🟢 replay-safe（整数のみ・`BTreeSet`/`BTreeMap` で canonical・float/RNG 無し）。
既存 sim 未使用のため `PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変（`tests/determinism.rs` 確認済み）。

---

## 11. 第6次棚卸し (2026-07-01) — ソクラテス式問答・第2ラウンド

前回に続き「過剰」と「不足」の両面を再度問い直した。

**過剰の再検討**: 「G21 の `MetaProgress::unlock`（冪等フラグ集合）と類似のフラグ管理が別モジュールに
必要になったとき、新しい型を作るべきか、`MetaProgress` を使い回すべきか？」——今回の不足調査で
「アイテム識別（下記 G22）」が同じ「冪等フラグ集合」の形を必要とすることが判明したが、
`MetaProgress` は「死んでも消えない」ことをモジュール doc で明言しており、
「毎回のランでリセットされる」識別状態と混ぜるとライフサイクルの意味が汚染される。
→ **意図的に型を分けたままにする**のが正しい判断と結論（過剰ではなく、責務分離の保持）。

**不足の検討**: 「Rogue/NetHack/Angband に共通する『未識別ポーション/巻物』——真の種類と
見た目ラベルの対応がシード毎にシャッフルされ、識別するまで隠される仕組み——を、既存 API で
表現できるか？」`random_table` は重み付き**抽選**の道具であって、2つの固定集合間の
**scrambled bijection**（一度きりの置換対応）を構築する道具ではない。

| # | 問い（既存 API で表現できないこと） | 隙間 | 状態 |
|---|------------------------------------|------|------|
| G22 | 「このポーションの本当の効果は分からないが、『渦巻き模様』というラベルは今回のプレイで一貫している」——`random_table` は毎回抽選する道具、`shufflebag` は非復元抽出の道具だが、「2つの固定集合を一度だけシャッフルして対応付け、種類ごとに開示フラグを持つ」構造が無い。 | シード毎のアイテム識別（scrambled 対応 + 開示フラグ） | ✅ **実装済み**（`src/identify.rs`: `Identification<T,L>`） |

**G22 — Identification（アイテム識別）** → `src/identify.rs`（新規 module, 18 unit + 5 property tests）

- `appearance: BTreeMap<T,L>`（`kinds` をソート後 dedup → `SplitMix64::shuffle` した `labels` と zip、
  入力順不変・シード決定論的）+ `identified: BTreeSet<T>`（`MetaProgress::unlock` と同じ冪等パターン、
  ただし意図的に別型——ライフサイクルが異なるため）。
- `appearance(kind)`: 識別済みかどうかに関わらず常にスクランブルラベルを返す——未識別時の表示に使う。
  ラベル自体は構築時に固定され、`identify()` では一切変化しない。
- `identify(kind)`: 冪等（既知でない `kind` や既に識別済みの `kind` への呼び出しは状態不変で `false`）。
- **ソクラテス的ギャップ**: `random_table` は「毎回抽選」、`shufflebag` は「非復元抽出のバッグ」だが、
  「2つの固定集合間の一度きりの scrambled 対応付け + 種類ごとの開示状態」という形は既存のどれとも
  一致しなかった。
- 検証した性質: 割り当ては真の全単射（重複ラベル無し）、`kinds` の入力順序はマッピングに非影響
  （内部でソートしてからシャッフル列と zip）、`identify` の呼び出し順序は最終状態/ハッシュに非依存、
  `appearance` は `identify` 呼び出し回数に関わらず不変、`DetHash` は真に新しい識別でのみ変化し
  重複呼び出しでは不変。

決定論影響: 🟢 replay-safe（整数のみ・`SplitMix64::shuffle` 経由で決定論的・float 無し）。
既存 sim 未使用のため `PINNED_FINAL_HASH` / `PINNED_ROGUELIKE_HASH` 不変（`tests/determinism.rs` 確認済み）。
