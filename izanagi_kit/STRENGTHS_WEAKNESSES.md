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
> 最終更新: 2026-06-11 / ブランチ: `claude/deepresearch-ultrathink-improve-yq2th`

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

## 2. 短所 (Weaknesses) — 設計上の制約・欠落

| # | 短所 | 位置 | 工数 |
|---|------|------|------|
| W1 | **多 component クエリ API が無い**（呼び出し側が手で N 重ループ） | `src/sparse_set.rs` | Medium（`Query<(A,B,C)>` ビルダー or マクロ） |
| W2 | **generation overflow 検出が弱い**（`wrapping_add(1)`、2³² 再利用で stale handle 復活の理論リスク） | `src/entity.rs` | Small（warn-on-wrap 診断） |
| W3 | **terminal の入力抽象が無い**（`inputbuf`/`keymap` はあるが端末 I/O 非接続） | `src/terminal.rs` | Medium（`TerminalInput` trait） |
| W4 | **save file の schema migration 基盤が無い** | `src/savefile.rs` | Medium（`Migrator<T>` / 版ディスパッチ） |
| W5 | **WFC の contradiction からの部分解抽出/backtrack が最小限** | `src/wfc.rs` | Small（`PartialSolution` / backtrack 上限） |
| W6 | **relations が transform 伝播しない**（親移動で子が追従しない） | `src/relations.rs` | Medium（`propagate_transforms`） |
| W7 | **FSM が flat**（階層状態・遷移ペイロード無し） | `src/fsm.rs` | Large（`Fsm<S,E,Payload>` 再設計） |

## 3. 不足機能 (Missing Features) — peer 比較での欠落と工数

| # | 機能 | 現状 | 工数 | 状態 |
|---|------|------|------|------|
| G1 | **damage type / resistance**（火耐性・弱点で被ダメ増減） | `combat` は scalar + flat `apply_resistance` のみ | Small | ✅ **実装済み**（`src/damage.rs`, 本コミット） |
| G2 | **status effect ↔ combat 統合**（時限 buff/debuff を戦闘式に反映） | `status.rs` 単独、combat 非連携 | Small | 未 |
| G3 | **nested / tiered loot table**（「種別 → その種別の loot」入れ子） | `random_table` は flat | Small | 未 |
| G4 | **encounter pack 生成**（深度/難度で「goblin×3 + shaman×1」） | 個体 pick のみ | Small | 未 |
| G5 | **multi-floor 遷移パス探索**（floor A→B を stairs 経由で） | `multimap` は connector lookup のみ | Medium | 未 |
| G6 | **stairs 連結の自動検出/チェイン** | 手動 Connector 追加 | Small | 未 |
| G7 | **item affix / enchantment 生成** | `random_table` は値のみ | Medium | 未 |
| G8 | **behavior tree / GOAP / utility AI** | `fsm` は flat | Large | 未 |
| G9 | **unified ability/skill system**（mana/cooldown/range/effect 結線） | `timer`+`fsm`+`combat` を手結線 | Large | 未 |

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

## 5. 次の推奨着手順（効果 × 低工数）

1. **G2 status↔combat 統合**（Small, `status.rs` を combat helper に結線）
2. **G3 nested loot table**（Small, `random_table` 上に `NestedTable`）
3. **G4 encounter pack**（Small, 深度パラメタ付き spawn roller）
4. **G5 multi-floor pathfinding**（Medium, connector graph 上の A*）

G8/G9（behavior tree, ability system）は Large・別スコープ確認の上で着手。
