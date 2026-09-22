# ゲーム開発 能力タクソノミー (Game-Dev Capability Taxonomy)

> 「ゲームを作成するのに必要なもの」を**徹底的にカテゴライズ**し、各々を**細分化**して、
> izanagi_kit の現状被覆（✅ 実装済 / 🔶 一部 / ⬜ 未実装＝強化対象）を対応付ける。
> これがエンジン高機能化のロードマップ兼ギャップ表。詳細契約は [`SPEC.md`](./SPEC.md)、
> 調査根拠は [`RESEARCH.md`](./RESEARCH.md)。

スコープ前提: zero-dep / `#![forbid(unsafe_code)]` / シミュレーションは整数・決定論（→ [`SPEC.md`](./SPEC.md) G1–G10）。
音声・GPU・OS 依存 I/O は本キットのヘッドレス方針では**意図的に範囲外**（呼び手側で実装）。

## A. 時間とループ (Time & Loop)
- A1 固定タイムステップ ✅ `timestep` / A2 補間 alpha ✅ / A3 death-spiral ガード ✅
- A4 スケジューラ（タイマー・クールダウン・遅延イベント）✅ `timer` / A5 ターン制エネルギー系（speed-based turn order）✅ `turn`

## B. 数学 (Math)
- B1 fixed-point Q16.16 ✅ `fixed` / B2 sqrt・CORDIC trig ✅ / B3 整数幾何（line/LOS）✅ `geometry`
- B4 fixed ベクトル（vec2/vec3, dot/len/normalize）✅ `vec` / B5 easing・tween（整数）✅ `easing` / B6 補間（lerp/clamp/sign）✅ `Fixed::lerp/clamp/sign/abs`

## C. 状態とデータ (State & Data / ECS)
- C1 generational entity ✅ `entity` / C2 sparse-set storage ✅ / C3 多コンポーネント join ✅
- C4 archetype storage ✅ `arch` / C5 変更検知（dirty/changed + 構造変化イベント）✅ `change` + `observe` / C6 エンティティ関係（parent/child, relations）✅ `relations`

## D. 乱数 (Randomness)
- D1 決定論 PRNG ✅ `rng` / D2 range・coin ✅ / D3 stream の DetHash ✅
- D4 重み付き抽選（weighted choice / loot table）✅ `weighted_index` / D5 ダイス（NdM）✅ `dice` / D6 value/Perlin noise（整数）✅ `noise`
- D7 named 独立ストリーム（サブシステム毎に非干渉な乱数列）✅ `SplitMix64::split(stream_id)` / D8 長周期・高統計品質な代替 PRNG（opt-in・既定不変）✅ `rng_xoshiro::Xoshiro256pp`（2²⁵⁶ 周期、`jump()` で並列ストリーム）
- D9 セルラー/Worley ノイズ ✅ `noise::worley_2d`（Worley SIGGRAPH'96 — 正確な F1/F2 距離とセル ID。`worley_2d_f2_minus_f1` で火口・静脈状リッジ。5×5 走査で厳密性を証明、GPU JFA 近似は意図的に不採用）

## E. 決定論・リプレイ (Determinism & Replay)
- E1 state hashing FNV-1a ✅ `world_hash` / E2 DetHash（値型＋容器）✅ / E3 replay trace・desync 検出 ✅ `replay`
- E4 snapshot/rollback 基盤 ✅ `rollback`（`SnapshotRing`: stride 付き有界リング・最古から eviction、`sync_test`: 毎フレーム rollback+再sim で step 関数の非決定性を検出）+ `replay::resimulate`（部分再実行） / E5 `DetHash` derive macro ⬜（zero-dep 方針では手実装維持も可）
- E6 順序非依存（permutation-invariant）multiset hashing ✅ `world_hash::hash_unordered`（`HashMap` 等の非正準順コンテナをソート不要で安定 hash 化）

## F. 表示・描画 (Presentation / Rendering)
- F1 セル画面バッファ（glyph + fg/bg）✅ `terminal` / F2 ANSI 24-bit 出力 ✅ `to_ansi` / F3 ダブルバッファ差分 ✅ `diff`/`present`
- F4 ヘッドレス検査（snapshot test 用 cell アクセス）✅ `get`/`DetHash` / F5 カメラ/ビューポート（world→screen）✅ `camera` / F6 描画プリミティブ（fill/box/text）✅ `fill_rect`/`draw_str`
- **F1–F6 すべて実装済み**（`terminal` + `camera`）。

## G. 入力 (Input)
- G1 キー→アクションのマッピング ✅ `keymap` / G2 コマンドキュー（決定論 input feed、replay と直結）✅ `cmdqueue` / G3 入力バッファ/長押し ✅ `inputbuf`

## H. コンテンツ・アセット (Content & Assets)
- H1 DSL パーサ ✅ `parser`（`extends` フィールド単位 override 含む）/ H2 シリアライズ往復 ✅ `serializer` / H3 意味検証 ✅ `validator` / H4 ECS ロード ✅ `loader` / H5 CLI ゲート ✅ `gamec`
- H6 ホットリロード ⬜ / H7 アセット ID/ハンドル管理 ✅ `assets`

## I. ワールド・マップ (World & Map)
- I1 手続き生成（rooms+corridors）✅ `mapgen` / I2 連結保証 ✅
- I3 タイルマップ層（複数レイヤ）✅ `tilemap` / I4 オートタイル ✅ `autotile` / I5 WFC 生成 ✅ `wfc` / I6 マルチレベル/階層 ✅ `multimap`
- I7 Voronoi 領域分割（生物群系・領土割当）✅ `voronoi::voronoi_partition`（選択距離で厳密な最近seed、タイは最小index。`voronoi_flood` は可通行地形を介した BFS 版 — 壁が領土を分断）
- I8 Poisson-disc 散布配置（最小間隔の blue-noise 配置）✅ `mapgen::poisson_disc`（Bridson 2007、`radius/√2` グリッド + active list + annulus サンプル。ルーム・資源・シードの非凝集散布）
- I9 散布点の最小接続（MST）✅ `voronoi::mst_edges`（完全グラフ上の Kruskal、`(距離,i,j)` 全順序で決定的 — TinyKeep 式の scatter → 領域 → 接続パイプラインを構成）/ `mst_edges_over`（許可エッジ集合に制限した restricted Kruskal — Delaunay 配線上の MST = TinyKeep 本流の recipe）
- I10 Delaunay 三角分割（配線網・隣接判定）✅ `delaunay::delaunay` / `delaunay_edges`（Bowyer–Watson 系の split+Lawson-flip 増分挿入。incircle を i128 厳密行列式で評価し cocircular は対角線選択を canonical に固定。super-triangle を `span²` 距離に置く設計 — 緩い距離では「(hull edge, super vertex) の外接円が内部点を包み hull 辺が flip される」穴あき bug を生むと実測で確認）
- I11 六角グリッド（戦略マップ・距離計量）✅ `hexgrid`（redblobgames axial 座標一式 — `Hex`/`DIRECTIONS`/`distance`/`line`/`ring`/`spiral`/odd,even-r offset 変換/`random_in_range`。cube_round のタイブレークを away-from-zero に固定し決定的）
- I12 迷路・回廊掘削（perfect maze・最小コスト隧道）✅ `maze::wilson_maze`（Wilson の loop-erased random walk — 全域木を一様分布で生成、STOC'96。cell 奇数座標・回廊は+1）/ `mapgen::carve_corridors`（TinyKeep recipe の掘削器 — `pathfinding::min_cost_path` で床=1・壁=wall_cost の重み A*、対角ステップは橋 cell を併掘し 4-連結を保証）
- I13 グラフ構造解析（マップ接続性・依存グラフ）✅ `graph`（Tarjan SCC = `strongly_connected`・関節点・橋・Kahn 辞書順最小 `topo_sort`・`UnionFind`。全て反復版で再帰深度制約なし。平行辺は bridge にならない等、multigraph 境界条件まで remove-and-recount オラクルで検証）
- I14 空間充填曲線コード（局所性保存の空間キー・cache-friendly 走査）✅ `zorder`（Morton 1966 z-order + Skilling Hilbert 写像 — `morton_encode`/`decode` 2D/3D/64-bit、`hilbert_encode`/`decode`、`spatial_sort`、`morton_key`。符号付き座標は sign-bit bias で順序保持。Hilbert は隣接 index が必ず 4-近傍であることを性質検査、round-trip を全域検証。encode の quadrant 変換が全 grid mask への回転である点を謬説と区別して実装）
- I15 等高線抽出（スカラー場の iso-contour）✅ `msquares`（marching squares — 16 case で辺中点へ segment を出力。鞍部 ambiguity は双線形中心値の漸近決定子で canonical 解決。倍精度座標で端点は必ず辺中点、interior 頂点の偶数次数性・ループ連鎖を不変条件オラクルで検証）
- I16 容量ネットワーク解析（ボトルネック・分断・物流割当）✅ `flow::FlowNet`（Edmonds–Karp max-flow — `max_flow`/`min_cut`/`flow_on`/`add_edge_undirected`。残余グラフの到達側 = min-cut 証人分割。cut 容量 = flow 値の定理・各頂点保存則を乱数ネットでオラクル検証）
- I17 書換系プロシージャル生成（植物・分岐構造）✅ `lsystem`（Lindenmayer 1968 の deterministic 0L 並列書換 `expand` + 整数 turtle `turtle_cells` — `F`/`G` 描画・`f`/`g` 移動・`+`/`-` 回転・`[`/`]` push/pop。`DIRS_4`/`DIRS_8`/`DIRS_HEX` で方格・六角格の植物。描画は `gridcast` 経由で対角欠落なし。Fibonacci 系・枝復帰・8-連結性を検証）

## J. 視界・AI・ナビ (Visibility / AI / Navigation)
- J1 対称 FOV ✅ `fov` / J2 A* 経路 ✅ `pathfinding`（正方格）/ `hexgrid::hex_astar`（六角格 — 厳密 hex distance を consistent heuristic として最短路保証、`(f,h,q,r)` 辞書順で決定的）/ J3 Dijkstra map（flow field）✅ / J4 descend（chase/flee）✅ / J5 LOS ✅ `geometry`
- J6 ステアリング/influence map ✅ `influence` / J7 FSM / behavior tree ✅ `fsm` / J8 JPS / weighted A* ✅ `weighted_astar`
- J9 グリッド raycast（投射物・掃引 LOS）✅ `gridcast`（Amanatides–Woo 1987 — 線分が*入る全 cell* を入射順に列挙。倍精度座標で整数厳密、corner 通過は対角 cell のみに進入し traverse が対称 = `grid_ray(a,b)` は `grid_ray(b,a)` の逆。`ray_blocked_at`/`clear_los` で LOS クエリ化。Bresenham の列1選択ではなく slab test オラクルで集合一致を検証）
- J10 最小コスト割当（unit→target・build order の割当て）✅ `hungarian::assign_min_cost`（Kuhn–Munkres O(n³) ポテンシャル法、n ≤ m 行列・i64 コストで負値も可 = 利得行列は negate で済む。全行が distinct 列を得る。固定走査順で複数最適解間も決定的 — 全注入マップ列挙ブルートフォースで最適性をオラクル検証）

## K. 物理・衝突 (Physics / Collision)
- K1 グリッド衝突（passability）✅ `passability` / K2 AABB 重なり ✅ `aabb` / K3 空間ハッシュ broadphase ✅ `spatial_hash`

## L. ゲームプレイ系 (Gameplay systems)
- L1 ターンスケジューラ（energy system）✅ `turn` / L2 ステータス/戦闘式 ✅ `combat` / L3 インベントリ/アイテム ✅ `inventory` / L4 状態異常（buff/debuff の期限管理）✅ `status`

## M. UI
- M1 メッセージログ ✅ `msglog` / M2 メニュー/ウィジェット ✅ `menu` / M3 テキストレイアウト/折返し ✅ `textlayout` / M4 HUD ✅ `hud`
- M5 矩形 bin packing（アトラス・インベントリ・ダイアログ敷詰）✅ `pack::pack_skyline`（Jylänki *A Thousand Ways to Pack the Bin* 2010 の skyline/bottom-left 法 — 入力順がそのまま配置順の純粋関数。重なり・境界逸脱を不変条件オラクルで検証）

## N. 永続化・セーブ (Persistence)
- N1 コンテンツ serialize ✅ / N2 ワールド save/load ✅ `savefile` / N3 バージョニング ✅ `savefile::SaveHeader::version`

## O. ネットワーク (Networking)
- O1 rollback/replay 基盤 ✅ `replay` / O2 input 同期 transport ⬜（ソケット I/O はヘッドレス方針で意図的に範囲外）/ O3 予測/補正 ✅ `netinput`（`NetInputBuffer`: 決定論的 input 予測・誤予測検出。transport 非依存＝呼び手が受信バイトを供給）
- O4 ビットレベル wire codec ✅ `bits`（`BitWriter`/`BitReader` — Gaffer 式のパックドビットフィールド・範囲整数・protobuf 型 canonical varint+zigzag。lockstep パケットをバイト境界無駄なく直列化、canonical 性を decode 側で強制）

## P. ツール・デバッグ (Tooling / Debug)
- P1 コンテンツ検証 CLI ✅ `gamec` / P2 desync 二分探索 ✅ `replay::first_divergence` / P3 ロギング/プロファイル ✅ `profiler` / P4 機械可読診断(JSON/SARIF) ✅ `diag_json` / `gamec --json`

## Q. 検証・テスト (Verification & Testing)
- Q1 統一シミュレーション契約 ✅ `sim`（`Simulation` トレイト + `audit()` = 二重実行 + rollback 再実行 + 最終 hash を1呼び出しで）
- Q2 決定論シミュレーションテスト ✅ `dst`（seed 掃引 + 毎 tick 不変条件、`(seed, tick)` の1行再現、二重実行 hash 比較で非決定性自体を検出）
- Q3 property-based testing ✅ `prop`（QuickCheck 式のランダム列 + 縮小済み反例）
- Q4 model-based / differential testing ✅ `prop::forall_model`（実 sim と信頼できる参照モデルを歩調合わせで走らせ、乖離した命令列を縮小）
- Q5 デルタデバッグ ✅ `shrink`（`ddmin` — 失敗入力列を 1-minimal まで削減）
- Q6 計画ベースのテスト合成 ✅ `plan`（goal 述語 → BFS 最短入力列。「X に到達できるか」が実行可能な replay になる）
- Q7 アーカイブ探索 ✅ `explore`（Go-Explore — 到達した状態を記憶し決定論的に復帰して先へ。BFS が停止する規模の先まで届き、状態ごとに再生可能な経路を返す）
- Q8 時相性質のランタイム検証 ✅ `temporal`（`always`/`eventually`/`until`/`precedes`/`responds_within`。LTL₃ の三値 anytime 判定 + 有限トレースの確定判定。`dst_sweep` の不変条件にそのまま挿せる）
- Q9 クラッシュ復旧テスト ✅ `recovery`（入力ごとに save/restore を挟み、実行が同一に継続することを証明。状態 hash が覆わないフィールドを落とすセーブ形式も捕まえる）
- Q10 有界モデル検査 ✅ `verify`（到達可能状態を全列挙し、不変条件を**証明**するか最短反例を返す。証明と「予算切れ」を別の値で報告する三値。`check_temporal` は `temporal` モニタと積を取り、単一状態述語で書けない順序性質を検証）

> **Q10 の射程**: `verify` が証明するのは**与えられたモデルについて**であり、実シミュレーションについてではない。
> モデルの忠実性は標本抽出（`forall_model` の差分テスト）で裏づけられるが、精密化証明ではない。
> この非対称性は `examples/verify_pipeline_demo.rs` の step 8 が実演する（実 sim では INCONCLUSIVE、モデルでは PROVED）。

---

## 実装優先度（高機能化ロードマップ）

| 順 | カテゴリ | 理由 | 状態 |
|----|---------|------|------|
| 1 | **F 表示・描画**（cell buffer + ANSI + diff） | ゲームを**表示できない**最大の欠落。terminal-first 宣言と乖離 | ✅ 実装済み（`terminal`、F5 カメラのみ残） |
| 2 | L1 ターンスケジューラ（energy system） | roguelike のコア進行 | ✅ 実装済み（`turn`） |
| 3 | D4 weighted choice / loot table | コンテンツ/戦闘に必須 | ✅ 実装済み（`weighted_index`, `dice`） |
| 4 | B4 fixed ベクトル | 移動・物理の土台 | ✅ 実装済み（`vec` — Vec2/Vec3） |
| 5 | M1 メッセージログ / G2 コマンドキュー | UI・入力決定論 | ✅ 実装済み（`msglog`, `cmdqueue`） |

上表の5件はいずれも完了している。**現在の未着手候補は本書ではなく [`RESEARCH.md`](./RESEARCH.md) の N 候補表を正とする** — 同じ候補を2つの表で管理すれば必ず片方が古くなる。
本書が持つのは「何が実装されているか」であり、「次に何をするか」ではない。
