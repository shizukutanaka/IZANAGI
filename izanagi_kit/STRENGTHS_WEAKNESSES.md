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
> 最終更新: 2026-06-24 / ブランチ: `claude/deepresearch-ultrathink-improve-yq2th`

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
| W8 | **Integration test 欠落**（複数 module 横断の E2E テストが無い） | wallet + dialogue + shop pricing の連携が未検証 | Medium |
| W9 | **複数 tween の同時再生管理がない**（animation sequencer） | UI/visual FX は複数並行アニメを必要とするが、個別管理が煩雑 | Medium |
| W10 | **shop pricing model**（buy/sell markup、NPC ごとの価格設定） | wallet の基盤は整備されたが、実際のショップ仕組みが無い | Small |
| W11 | **trigger / event script**（条件→アクション チェーン）| dialogue の結果（choice）を quest や world state 変化に紐づける基盤が無い | Large |
| W12 | **README.md が実装に追いついていない** | G1-G14 の機能一覧が記載されていない | Small |

### 7.2 次の推奨着手順（効果 × 工数、内部依存度）

1. **W12 — README.md 更新** ← **即実装推奨**（手付かず、Small 工数）
   - G1-G14 の機能一覧を追加
   - 各モジュールの Socratic gap（なぜこれが必要だったか）を簡潔に記述
   - property test 数を記載（218 件）

2. **W10 — shop pricing model** ← **次推奨**（Small、wallet 直結）
   - `Shop<K>` struct: `Wallet` + 各 item に buy/sell markup
   - `can_buy / buy / can_sell / sell` トランザクション
   - wallet 実装済みなので low-hanging fruit

3. **W8 — wallet + dialogue integration test** ← **その次**（Medium、E2E validation）
   - NPC が商品提示（dialogue）→ player が購入（wallet withdraw）
   - property test 1-2 件で wallet/dialogue の組み合わせが safe であることを示す

4. **W9 — animation sequencer** ← **中期**（Medium、tween の拡張）
   - `TweenSequence<T>` or `TweenChain`: `Vec<Tween>` を順序付で管理
   - 並行実行は `Vec<Tween>` → `iter_mut / advance_all`
   - UI bar fill（1 tween）+ sound fade（別 tween）同時再生

5. **W11 — trigger / event script** ← **大型フェーズ**（Large、新型の検討が必要）
   - condition predicate （「quest active? 」「player in zone? 」）
   - action lambda （「show dialogue」「grant item」「start encounter」）
   - chain: `if condition then actions` の linked list か DAG
   - **現段階では未実装でよし**（NG ギャップリスト化が目的）
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
