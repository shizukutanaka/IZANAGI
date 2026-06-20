# izanagi_kit 仕様書 (Specification)

> 本書は `izanagi_kit` の **API 契約・不変条件・完成度** を定義する仕様書。
> 改善点の調査は [`RESEARCH.md`](./RESEARCH.md)、変更履歴は [`CHANGELOG.md`](./CHANGELOG.md) を参照。
> 「不足部分」は §13 完成度チェックリストの ⬜ 項目で、本イテレーションで一部を実装する。

最終更新: 2026-06-06 / 対象ブランチ: `claude/deepresearch-ultrathink-improve-yq2th`

## 1. スコープと目的

`izanagi_kit` は **決定論的（replay-safe / lockstep-safe）なヘッドレス・ターミナル roguelike エンジンの
リファレンス・モジュール群**。ゲーム本体ではなくドロップイン部品の集合。

## 2. 全体不変条件 (Global invariants) — 全モジュール必須

| ID | 不変条件 | 根拠 |
|----|---------|------|
| G1 | **zero runtime dependencies**（`Cargo.toml` の `[dependencies]` 空） | 監査面の最小化 |
| G2 | **`#![forbid(unsafe_code)]`** | メモリ安全 |
| G3 | **シミュレーション経路で `f32`/`f64` を使わない**（fixed-point / 整数のみ） | クロスプラットフォーム決定論 |
| G4 | **wall-clock / thread-local をシミュレーション種にしない** | replay 再現性 |
| G5 | **状態 hash 列が同一入力で bit 一致**（`PINNED_FINAL_HASH` を CI で固定） | lockstep desync 検出 |
| G6 | コレクション走査は **canonical 順序**（昇順 index 等）で hash する | 順序由来の非決定性排除 |
| G7 | パニックしない公開 API（不正入力は飽和・None・no-op で処理） | 堅牢性 |
| G8 | MSRV **1.75** / edition 2021 | 互換性 |

## 3. `entity` — 世代付きエンティティ
- `Entity{index,generation}`（opaque）, `EntityAllocator{allocate, free, is_alive}`。
- 契約: free→再 allocate で index 再利用＋generation +1、stale handle は `is_alive`=false、double-free は no-op。
- 不変: 同一 allocate/free 列 → 同一ハンドル列（決定論）。

## 4. `sparse_set` — コンポーネント storage
- `SparseSet<T>{insert,get,get_mut,remove(swap-remove),contains,len,iter,iter_mut,iter_sorted}`。
- 契約: insert/get/remove は O(1)。`iter` は挿入履歴順（非 canonical）、`iter_sorted` は昇順 index（canonical, G6 用）。
- `det_hash(&Fnv1a)`（`T: DetHash`）— len と各 `(index,value)` を昇順 index で fold。
- **multi-component query（実装済）**: `join(&a,&b)->Vec<(Entity,&A,&B)>`（両在エンティティの inner join、昇順 index）、
  `join_mut(&mut a,&b)->Vec<(Entity,&mut A,&B)>`（A を B で更新する system 向け）。小さい方を走査して probe（O(min)）。

## 5. `fixed` — Q16.16 固定小数点
- `Fixed`：`from_int/from_ratio/raw/to_int_trunc/saturating_add/mul/div`、`Add/Sub`（飽和）。
- **transcendental（実装済）**: `sqrt`（整数 isqrt、負は 0 飽和）、`sin/cos/sin_cos/atan2`（CORDIC、整数定数）。
- 契約: 全演算は i64 中間で飽和（wrap 禁止, G3）。0 除算は符号方向飽和（G7）。
- **新規（本仕様）**: `impl DetHash`（raw i32 を fold）。

## 6. `rng` — SplitMix64
- `SplitMix64{new,next_u64,below(bound),state}`。`below` は **Lemire wide-multiply**（low-bias, modulo bias 無し）。
- 契約: `below(0)` は draw せず 0（debug/release 同一, G5）。`state()` を hash に折り込み stream 分岐検出。
- **新規（本仕様）**: `range(lo,hi)`（`[lo,hi)`、`lo>=hi` は `lo`）、`coin(num,den)`（確率 num/den の bool）。低 bias を維持し draw 数は決定的。

## 7. `world_hash` — 決定論 hashing
- `Fnv1a{write_bytes/u32/u64/i32,finish}`、`trait DetHash{det_hash}`。
- 契約: FNV-1a/64、順序依存。canonical 順序での fold は呼び手責務（G6）。
- 基本型（`u32,u64,i32,bool,char`）と kit 値型（`Fixed,Entity,Position,Render,Color,SplitMix64,Dungeon`）への `DetHash` 実装。
- `hash_state<T: DetHash>(&T) -> u64`（単一 checksum へ畳む便宜関数、`replay` が使用）。

## 8. `timestep` — 固定タイムステップ
- `FixedTimestep{new,sixty_hz,step_ns,total_steps,advance(frame_ns)->steps,alpha_ratio()->(num,den)}`。
- 契約: 整数ナノ秒、death-spiral ガード（`max_steps` 超過分は破棄）、`alpha_ratio` で補間（float-free, G3）。

## 9. `content`/`parser`/`serializer`/`validator`/`loader` — コンテンツパイプライン
- `parse` → `Content`（`BTreeMap` で canonical）→ `validate`（全件収集診断）→ `load_level` → ECS。
- `serialize` は canonical・idempotent、`content_eq` で round-trip 等価。診断は rustc 風 caret（column 付き）。
- 契約: パーサは panic-free・bounded（1024B 行 / 256×256 grid, G7）。

## 10. `fov` — 対称シャドウキャスティング（実装済）
- `compute_fov(origin,radius,is_opaque,mark_visible)`。整数有理数スロープ、4象限固定順、対称性保証、Euclidean radius。

## 11. `pathfinding` — グリッド経路探索（実装済）
- `astar(start,goal,is_blocked)->Option<Vec<(i32,i32)>>`（8方向、整数 octile 10/14、`(f,h,x,y)` 全順序 tie-break、corner-cut 無し）。
- `weighted_astar(start,goal,is_blocked,weight)`（ε-admissible、`f=g+weight×h`、cost ≤ weight×optimal、weight=1 で astar と一致）。
- **`jps(start,goal,is_blocked)->Option<Vec<(i32,i32)>>`（Jump Point Search）**: astar と同じ no-corner-cut モデルで
  対称領域を「ジャンプ」して探索を高速化。返すのは astar と同型の full path で **cost は astar と厳密一致**（近似ではない）。
  `is_blocked` は OOB=true を要求。決定論は astar と同一（`(f,h,x,y)` 全順序、固定コンパス順）。述語は `Fn`
  （jump 再帰が cell を reentrant に参照するため）。契約検証: 6000 ランダム盤面で astar と reachability/cost 一致 +
  corner-safe path を metamorphic に確認（astar が正解 oracle）。
- `dijkstra_map(sources,max_cost,is_blocked)->HashMap<cell,cost>`（多源距離場 / flow field）と
  `descend(&map,from,is_blocked)->Option<cell>`（最小コスト隣接へ決定的に降下、chase AI 用）。

## 11.5 `mapgen` — 手続き的ダンジョン生成（実装済）
- `generate_dungeon(width,height,&mut SplitMix64,GenParams) -> Dungeon`、`Rect`、`GenParams{max_rooms,min_room,max_room}`。
- `Dungeon{width,height,rooms,is_wall,is_floor}`、`impl DetHash`（wall bitmap を pack して fold）。
- 契約: 全乱択は渡された `SplitMix64` を固定順で消費（G3/G4）→ `(seed,params,size)` で byte 一致。room を rejection 配置（1セル境界）→ 直前 room と L 字回廊で接続 → **全 floor 連結**保証。小さすぎる盤面は all-wall（panic 無し, G7）。`is_wall` は OOB=wall で fov/pathfinding に直結。

## 11.7 `replay` — リプレイ／desync 検出／rollback（実装済）
- `record_trace(&mut S, inputs, step) -> Vec<u64>`（per-tick state hash 列）、`check_trace(...) -> Result<(),Divergence>`、
  `first_divergence(&[u64],&[u64]) -> Result<(),Divergence>`、`resimulate(&S, inputs, step) -> S`（snapshot を clone して再シミュ＝rollback 基盤）。`Divergence{tick,expected,actual}`。
- 契約: `S: DetHash`（状態 hash は `hash_state`）。`step` クロージャでエンジン非依存。同一 `(初期状態,inputs)` で trace bit 一致、最初の分岐 tick を特定（G5、desync 二分探索の起点）。

## 11.8 `terminal` — 表示層（セルバッファ）（実装済）
- `Cell{glyph,fg,bg}`（`DetHash`）、`Screen{new,width,height,get,put,set,clear,fill_rect,draw_str,diff,present,to_ansi}`（`DetHash`）。
- 契約: 描画は in-memory セル格子のみ（OS I/O 無し・ヘッドレス）。範囲外書込はクリップ（panic 無し, G7）。`to_ansi` は 24-bit truecolor の決定論文字列（行内で色が変わる時のみ SGR 再発行）。`diff`/`present` でダブルバッファ差分。フレームを world hash / snapshot test に畳める。

## 12. `gamec`（bin）— コンテンツゲート
- `.game` を検証、`--fmt` で canonical 整形、エラー時非ゼロ終了（CI gate）。

## 13. 完成度チェックリスト (Completeness checklist)

✅ 実装済 / 🔶 一部 / ⬜ 未実装（= 不足部分）

| 項目 | 状態 | 備考 |
|------|------|------|
| entity / sparse_set / fixed(基本) / rng(core) / world_hash(core) | ✅ | |
| timestep（accumulator + alpha + death-spiral） | ✅ | |
| content pipeline（parse/serialize/validate/load/gamec） | ✅ | |
| fixed: sqrt / CORDIC trig | ✅ | 本ループで実装 |
| fov: symmetric shadowcasting | ✅ | 本ループで実装 |
| pathfinding: A* | ✅ | 本ループで実装 |
| **D1 DetHash 実装（値型）＋ SparseSet 正準 hash** | ⬜→✅ | **本イテレーションで実装** |
| **P1 Dijkstra map（flow field）＋ descend** | ⬜→✅ | **本イテレーションで実装** |
| **R1 rng `range`/`coin` エルゴノミクス** | ⬜→✅ | **本イテレーションで実装** |
| procedural generation（seed 駆動ダンジョン）= `mapgen` | ✅ | 本イテレーションで実装（rooms + corridors, 連結保証, DetHash） |
| C1 multi-component query (`join`/`join_mut`) | ✅ | 本イテレーションで実装。archetype storage は ⬜ |
| C6 replay harness + snapshot/rollback + desync 検出 = `replay` | ✅ | 本イテレーションで実装（record/check/first_divergence/resimulate）|
| geometry: Bresenham line / LOS = `geometry` | ✅ | 本イテレーションで実装（`line`/`line_of_sight`）|
| weighted A*（ε-admissible） | ✅ | `pathfinding::weighted_astar`（`f=g+weight×h`、cost ≤ weight×optimal）|
| **JPS（Jump Point Search）** | ⬜→✅ | **本イテレーションで実装**（`pathfinding::jps`、no-corner-cut、A* と cost 一致、6000 ランダム盤面で metamorphic 検証）|
| 機械可読診断(JSON) | ✅ | `diag_json`（手書き JSON、CI/LSP 消費可能）|

### 本イテレーションで実装する不足部分
1. **D1**: `DetHash` を基本型と `Fixed/Entity/Position/Render/Color` に実装し、`SparseSet::det_hash` で
   canonical 順序の容器 hash を提供（G5/G6 を値型まで配線）。
2. **P1**: `pathfinding::dijkstra_map` と `descend`（決定的 flow field）。
3. **R1**: `SplitMix64::range` と `coin`（low-bias 維持、draw 数決定的）。
