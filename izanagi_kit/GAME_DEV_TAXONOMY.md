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
- A6 暦定周期イベント（定期スポーン・サーバイベント窓・日次リセット）✅ `cron`（POSIX 5-field cron — `Cron::parse` + `next_after`/`next_n_after` で ms 時間軸の次発火時刻。dom ∧ dow 同時制約時の POSIX OR 規則、`7 ≡ 0` Sunday、Quartz 式 `a/n` ステップ。bitset day-loop + Howard Hinnant civil-from-days で O(days) スキャン。分数走査ブルートフォース oracle と乱数検証）

## B. 数学 (Math)
- B1 fixed-point Q16.16 ✅ `fixed` / B2 sqrt・CORDIC trig ✅ / B3 整数幾何（line/LOS）✅ `geometry`
- B4 fixed ベクトル（vec2/vec3, dot/len/normalize）✅ `vec` / B5 easing・tween（整数）✅ `easing` / B6 補間（lerp/clamp/sign）✅ `Fixed::lerp/clamp/sign/abs`
- B7 整数論・モジュラ演算（周期イベント整合・剰余アドレッシング・ハッシュ素性）✅ `ntheory`（`gcd`/`lcm`/`extgcd`/`mod_inv`/`mod_pow`/`crt2`/`crt` — unsigned-abs ユークリッド、Bézout 逆元、u128 中間の binary modpow、中国剰余合成は非 coprime も矛盾検出付き。ブルートフォース gcd・Bézout 恒等式・CRT 合同性/一意性/矛盾性を乱数オラクル検証）
- B8 順列代数・順位列挙（spawn 順シャッフルの正準形・perm→seed 全単射）✅ `perm`（`identity`/`is_valid`/`compose`/`inverse`/`cycles`/`sign`/`order`/`apply`/`rank`/`unrank` — 群演算は入力検証付き、巡回分解は最小要素先頭の canonical 形、位数はサイクル長 lcm。Lehmer コードの factoradic で n≤20 の `rank`/`unrank` 全単射 = 順列がそのままシード値になる。結合法則・逆元・`p^order=e`・符号=inversion パリティ・rank/unrank 全単射(n≤7 全網羅)を乱数検証）
- B9 数論変換畳み込み（ダイス合計分布・loot 母関数計数・音程畳み込み）✅ `conv`（NTT mod 998244353 = 119·2²³+1 原始根3 — ビット反転置換 + 反復 butterfly の `O(n log n)` 多項式積、全中間は u64/u128 のみで float FFT の丸めを根本回避。`convolve_i64` は係数上限が modulus を超えると包んでしまうため `None` を返す失敗閉鎖設計。naive O(n²) mod-p と全一致・可換性・負数復元を乱数検証）
- B10 厳密線形代数（制約連立解・透過率・体積保存判定）✅ `gauss`（Bareiss fraction-free 消去 — `det` は i128 中間で float pivot の「小さすぎる」曖昧さが存在しない絶対値、`solve` は拡大行列上の Bareiss + 既約 (num,den) 逆戻入、`rank` は独立した交差乗算経路。`det==0` ⟺ `rank<n` の同値・permutation 展開 det・A·x=b 復元を乱数検証）
- B11 整数厳密パラメトリック曲線（カメラ経路・投射物弧・patrol ルート）✅ `bezier`（Bernstein 形を `u=d−t` の整数展開で評価 — `cubic_pos`/`catmull_pos` は `t=num/den` の有理 t で座標が既約 i128 分数、共通 gcd で縮約。`flatten_cubic`/`flatten_catmull` は端点保存の等分割 polyline、Catmull-Rom は両内点を厳密補間。Horner 展開オラクル・端点復元・補間性を乱数検証）

## C. 状態とデータ (State & Data / ECS)
- C1 generational entity ✅ `entity` / C2 sparse-set storage ✅ / C3 多コンポーネント join ✅
- C4 archetype storage ✅ `arch` / C5 変更検知（dirty/changed + 構造変化イベント）✅ `change` + `observe` / C6 エンティティ関係（parent/child, relations）✅ `relations`
- C7 決定論的順序集合（ソート済み列挙が要する状態集合 — スポーン表・ロックステップ辞書）✅ `treap`（BST + min-heap treap、優先度は `splitmix64(key^seed)` で内容定義 = 挿入順に非依存な唯一形状。arena 格納 + merge/split 構成、`insert`/`remove`/`contains`/`min`/`max`/`rank`/`select`/`iter`。BTreeSet オラクル同値・シャッフル挿入の形状一致・ヒープ性を検証）

## D. 乱数 (Randomness)
- D1 決定論 PRNG ✅ `rng` / D2 range・coin ✅ / D3 stream の DetHash ✅
- D4 重み付き抽選（weighted choice / loot table）✅ `weighted_index` / D5 ダイス（NdM）✅ `dice` / D6 value/Perlin noise（整数）✅ `noise`
- D7 named 独立ストリーム（サブシステム毎に非干渉な乱数列）✅ `SplitMix64::split(stream_id)` / D8 長周期・高統計品質な代替 PRNG（opt-in・既定不変）✅ `rng_xoshiro::Xoshiro256pp`（2²⁵⁶ 周期、`jump()` で並列ストリーム）
- D9 セルラー/Worley ノイズ ✅ `noise::worley_2d`（Worley SIGGRAPH'96 — 正確な F1/F2 距離とセル ID。`worley_2d_f2_minus_f1` で火口・静脈状リッジ。5×5 走査で厳密性を証明、GPU JFA 近似は意図的に不採用）
- D10 コーパス学習の名付け生成（NPC・地名・アイテム名）✅ `markov::NameGen`（order-k byte-level Markov 連鎖 — BEGIN/END パディング語から cumulative-weight 表を BTreeMap で構築、遷移は `SplitMix64::below` で昇順 byte 走査。同じ (corpus, k, seed) で同じ名が全プラットフォームで再現 = rng stream の純関数。gram 全包含・seed 一致/分岐を検証）

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
- I18 多角形述語・三角分割（領域外形・分割・検査）✅ `poly`（shoelace `area2`、even-odd `point_in_polygon` + 境界判定、Andrew monotone chain `convex_hull`、`ear_clip` 三角分割。全判定は `orient` で i128 厳密。ear の頂点-on-境界ケース — 凹頂点が辺上に乗ると三角が凹部へ食い出る — は inclusive 判定で拒否。面積分割一致・凸包全点包含・非凸 L 字をオラクル検証）
- I19 ポリライン簡約（等高線の頂点数削減）✅ `rdp`（Ramer–Douglas–Peucker。距離比較は |cross|² vs eps²|chord|² の i128 交差乗算で真の垂距に対する整数閾値。`simplify_loop` は重心最遠点をアンカーに回転して閉ループの切れ目を内容定義化。端点保存・次数順保存・冪等性を検証）
- I20 ランキング・重み付き抽選の接頭辞和（リーダーボード・経済台帳）✅ `fenwick`（Fenwick 1979 BIT — `add`/`prefix_sum`/`range_sum`/`total`/`lower_bound` 秩序統計量。`from_slice` は O(n) 直接構築。乱数ブルートフォース接頭辞和と全クエリ一致を検証）
- I21 範囲集約クエリ（min/max/sum on 任意窓 — スライド視界・地形帯統計）✅ `segtree::SegTree`（bottom-up 反復 segment tree、`set` 点更新 + `range_min`/`range_max`/`range_sum`/`range_stats` を O(log n)。size は次冪で node index が内容定義。200 回 × ランダム n ≤ 40 の全クエリをブルートフォース区間走査と一致検証）
- I22 前置辞書・オートコンプリート（did-you-mean 候補・識別子表）✅ `trie`（byte trie — `insert`/`remove`/`contains`/`starts_with`/`keys`/`keys_with_prefix`。BTreeMap 子で listings は必ず byte 辞書順 = 挿入順非依存。`diff::levenshtein` の候補源として did-you-mean を構成。BTreeSet オラクルと挿入・削除・prefix 列挙の同値を乱数検証）
- I23 静的範囲最小/最大クエリ（焼き付けコスト場・波形エンベロープ — 更新不要でクエリ熱い場合）✅ `rmq::SparseTable`（sparse table — `O(n log n)` 構築で `range_min`/`range_max` を O(1)。冪等演算なので重複ブロック合成がそのまま使える = disjoint cover の管理不要。`segtree`（点更新あり）の静的補完。ランダム n≤50 の全範囲をブルートフォース min/max と一致検証）
- I24 区間集合（占有時間帯・予約窓・scanline 壁区間）✅ `interval::IntervalSet`（sorted disjoint 半開 `[lo,hi)` — `insert` は接続・橋渡し全件マージ、`remove` は切断で split、`clip` は交差残し。`contains`/`overlaps` は二分探索 O(log n)。BTreeSet 点集合オラクルと insert/remove/clip 混合系列の同値 + 不変条件（ソート・非隣接）を乱数検証）
- I25 ストリーミング統計・集約（テレメトリ・負荷統計・バランス監視）✅ `stats::RunningStats`（Welford 1982 online moments — `push`/`push_all`/`merge`/`mean`/`variance`/`sample_variance`/`stddev`/`min`/`max`。全量 i64·SCALE 固定小数、Chan parallel-merge で分割済み shard を結合可能 = mapreduce 用途。naive 2-pass oracle と ±SCALE 範囲の一致を乱数検証）

## J. 視界・AI・ナビ (Visibility / AI / Navigation)
- J1 対称 FOV ✅ `fov` / J2 A* 経路 ✅ `pathfinding`（正方格）/ `hexgrid::hex_astar`（六角格 — 厳密 hex distance を consistent heuristic として最短路保証、`(f,h,q,r)` 辞書順で決定的）/ J3 Dijkstra map（flow field）✅ / J4 descend（chase/flee）✅ / J5 LOS ✅ `geometry`
- J6 ステアリング/influence map ✅ `influence` / J7 FSM / behavior tree ✅ `fsm` / J8 JPS / weighted A* ✅ `weighted_astar`
- J9 グリッド raycast（投射物・掃引 LOS）✅ `gridcast`（Amanatides–Woo 1987 — 線分が*入る全 cell* を入射順に列挙。倍精度座標で整数厳密、corner 通過は対角 cell のみに進入し traverse が対称 = `grid_ray(a,b)` は `grid_ray(b,a)` の逆。`ray_blocked_at`/`clear_los` で LOS クエリ化。Bresenham の列1選択ではなく slab test オラクルで集合一致を検証）
- J10 最小コスト割当（unit→target・build order の割当て）✅ `hungarian::assign_min_cost`（Kuhn–Munkres O(n³) ポテンシャル法、n ≤ m 行列・i64 コストで負値も可 = 利得行列は negate で済む。全行が distinct 列を得る。固定走査順で複数最適解間も決定的 — 全注入マップ列挙ブルートフォースで最適性をオラクル検証）
- J11 複数パターン走査（禁止語・署名パターン・diag 抽出）✅ `ahocor`（Aho–Corasick CACM'75 — trie + fail リンクで O(text+hits)。overlap・suffix 内包 match を全件 (end_pos, pattern) で走査順に出力。BTreeMap 子遷移 + BFS fail 構築で決定的。ブルートフォース部分文字列走査との全一致を乱数検証）
- J12 差分・編集距離（desync 報告・did-you-mean・セーブ比較）✅ `diff`（Myers O(ND) 1986 貪欲法 — `diff` は最小 Keep/Del/Ins スクリプト、`hunks` は unified-diff 形の塊、`apply` で検証可能なラウンドトリップ。`levenshtein`/`lcs_len` 付き。スクリプト適用 = 目標再現・編集数最小性 n+m-2·lcs・位置順整合を乱数ペアでオラクル検証）
- J13 二分マッチング（unit↔job 非加重割当・チーム編成）✅ `bipartite::hopcroft_karp`（Hopcroft–Karp O(E·√V) — BFS layered graph + DFS augment、走査順固定で決定的。`kuhn_match` 素朴増広路とのサイズ一致・matched edge が adj に存在・right 再利用なしを乱数検証。`hungarian`（加重）の補完）
- J14 巡回経路計画（哨戒・visit-all ミッション）✅ `tsp`（NN 構築 + first-improvement 2-opt — `nn_tour`/`tsp_2opt`/`tour_cost`。結果は city 0 始点・次点最小 index で canonical 化、同一直線条件と探索順は index 規則のみ。2-opt ≤ NN 不変・n≤8 ブルートフォース最適との hit-rate 検証）
- J15 全辺走査ルート（巡回点検・ウォーター配給・全通路往路）✅ `euler::euler_walk`（Hierholzer 1973 — 無向多重グラフの Eulerian circuit/path を O(E)。次数奇数性で Circuit/Path を判定、非連結・odd>2 は `None`。自己ループは次数2・平行辺は個別辺として扱う。辺消費は入力順 first-unused で決定的。消費多重集合一致 oracle + ランダム Eulerian 多重グラフでの往復検証）
- J16 あいまい検索・優先順位付け（コマンドパレット・did-you-mean ランカー）✅ `fuzzy`（fzf 式 subsequence scoring — `score`/`rank`/`rank_str`、case-insensitive byte 走査。CONSECUTIVE(連続 run) が支配的重み、boundary(`_-. /`・camel hump) 補助、GAP ペナルティ。全順序は score desc → len → bytes → index で決定的、部分列 oracle で score↔subseq 同値を乱数検証）
- J17 根付き森の祖先/深さ/距離クエリ（ゾーン木・スキルツリー・エンティティ階層の共通祖先）✅ `lca`（binary lifting — O(n log n) 構築で `lca`/`ancestor`/`dist`/`depth` を O(log n)。循環・範囲外 parent は invalid マークで `None` を返し panic しない。祖先集合列挙 oracle と全対一致を乱数森で検証）
- J18 単一パターン走査（区切り・ヘッダ・プロトコルセンチネル）✅ `kmp`（Knuth–Morris–Pratt — O(text) 前処理 `fail` 表で後戻りなし走査。`find`/`find_all`/`count` + 1 byte ずつ供給する `Stream` でチャンク境界分割の match も絶対位置を報告。ブルートフォース全位置走査と全一致・ストリーム=バッチ同値を乱数検証。`ahocor` の単一パターン版）
- J19 確率的所属判定（高価な完全一致の前置フィルタ — replay checkpoint・entity 重複除去）✅ `bloom`（Kirsch–Mitzenmacher double hashing `h1+i·h2` — `Fnv1a` 対に seed を塩して `(seed, params, multiset)` の純関数。片方向誤りのみ: 挿入済みは必ず present、`definitely_absent` が安全方向。`sizing` で (bits, probes) 設計、`load_permille` が FPR 代理。false-negative 不存在・FPR 情報理論限界内・ビット列純関数性を乱数検証）
- J20 接尾辞配列・文字列構造解析（corpus 監査・`markov` が覚えた gram の検査・重複部分列）✅ `suffix`（prefix-doubling O(n log² n) 構築 + Kasai LCP — `search` が全出現を `O(pat log n + hits)`、`longest_repeated`/`distinct_substrings` が文字列の重複構造を曝く。naive ソート・素朴 LCP・全位置走査・BTreeSet 部分文字列数で乱数オラクル検証）
- J21 制約充足 2-SAT（key-and-lock・ペア排他・tech-tree ゲーティング）✅ `twosat`（Aspvall–Plass–Tarjan — `a∨b` を含意辺 `¬a→b`,`¬b→a` に変えて `graph::strongly_connected` で SCC 分解。変数とその否定が同 SCC で UNSAT。sinks-first 順位で `rank[t]<rank[f]` の正極性を採る canonical 解。n≤7 でブルートフォース SAT/UNSAT 判定一致 + 解が `check` を通ることを乱数検証）
- J22 ゲーム木完全探索（盤面 AI・戦術検証・後退解析）✅ `minimax`（deterministic negamax + αβ — `Game` トレイト（`moves`/`apply`/`evaluate`/`terminal`）に対し `score`/`best_move`。着手順は `moves` の canonical 順、同値は先着側を保持、終端スコアは ply 割引で最短勝ちを優先。Tic-Tac-Toe 全域で αβ=naive negamax 一致 + 完全棋譜引き分け・即勝ち・最遅敗を既知値検証）
- J23 回文構造クエリ（名付け lint・シード美観・対称 ID 生成）✅ `manacher`（Manacher 1975 — `odd_radii`/`even_radii` (d1/d2) を O(n) で構築、`longest_palindrome` は leftmost タイブレーク、`count_palindromes` は Σd1+Σd2 の個別 (start,len) 数。半開 [l,r) の鏡像 index を inclusive 慣行から正しく変換（初版のずれを BTreeSet 列挙オラクルが捕捉）。ブルートフォース全部分列検査・d1/d2 の真値性/最大性を乱数検証）
- J24 カーディナリティ推定（distinct カウントの省メモリ概算 — replay checkpoint dedup・entity 流出率監査）✅ `kmv`（K-minimum-values sketch — seed 付き Fnv1a hash の k 最小値を保持、distinct<k では厳密、`(k−1)·2⁶⁴/vₖ` の整数推定で浮動小数点を根本回避。merge は最小値集合の合併 = ストリーム合併と同値。BTreeSet 正確数・4σ 整数窓・merge 最小値一致を乱数検証）
- J25 頻度推定スケッチ（巨大 key 空間の到着回数 — packet rate・loot 履歴・hot-cell 検出）✅ `cms`（count-min sketch — `depth×width` カウンタ行列、行ごとに独立 seed hash、estimate=min で片方向誤りのみ（衝突は足すだけ = 決して過小評価しない）。merge は要素和、次元/seed 不一致は `None` で拒否。片方向性・合併=単一ストリーム一致・経験過剰境界を乱数検証）
- J26 ε近似分位数（レイテンシ・ダメージ分布の省メモリ要約 — p50/p99 監査）✅ `quantile`（Greenwald–Khanna 2001 — `(v,g,δ)` タプル列 + 周期的 compact で `O((1/ε)log εn)` メモリ、全て整数演算。query は `|真の順位 − φ·n| ≤ ε·n` を保証、端点は δ=0 で厳密。ソート済み真値との全十分位順位境界一致・小ストリーム厳密性・退化引数を乱数検証）
- J27 一貫ハッシュ割当（shard→peer の最小移動割当・レプリケーション群・coordinator 不要の決定的分割）✅ `chash`（rendezvous/HRW hashing — `argmax_n hash(seed,n,key)`、ノード除去でそのノードの key のみが再配置される最小混乱性、順序非依存の canonical tie-break。`pick_top` で上位 r ノード = 複製先。除去時の非移動性・順序不変・pick=top[0]・大域均衡を乱数検証）
- J28 集合類似度推定（spawn 重複監査・近似 dedup・corpus の近傍クラスタリング）✅ `minhash`（MinHash — `k` 個の独立 seed 最小値を保持、Jaccard 推定は permille で `O(1/√k)` 誤差、浮動小数点なし。`union` は位置別 min で mergeable。sorted merge-join の `jaccard_exact` を内蔵 oracle として公開し 40 試行で `err²·k` 境界を検証）
- J29 線形文字列走査・周期構造（パターン検索・prefix=suffix・繰返し最小周期）✅ `zfunc`（Z-algorithm — 各位置の「そこからの prefix 一致長」を O(n)。`z_search` は `pat+sep+text` 連結、sep が本文混入のとき naive へ退化する安全弁。`borders`/`min_period` で構造解析。全位置 naive 照合・borders brute-force・周期最小性を乱数検証）
- J30 負辺最短路・負閉路検出（通貨裁定・資源変換の net-cost・負ゲイン辺を含むグラフ）✅ `bellman`（Bellman–Ford — `O(V·E)` で `pathfinding` の非負辺領域を拡張。extra pass に relax できる辺が残れば到達可能な負閉路で `None`。`negative_cycle` は virtual-source 接続でグラフ全域の負閉路を頂点列で返す。独立 relax オラクル・path 累積重み一致・閉路 sum<0 を乱数検証）
- J31 正規化有理数算術（確率木・drop rate・分数が厳密のまま残るべきあらゆる場所）✅ `frac`（`Frac` — `(num,den)` を常に gcd=1・den>0 に正規化 = 等値は構造的一致。全演算 `i128` 厳密、掛算は cross-reduce で headroom 確保、`den==0` 生成は clamp・`/0` は `None`。cross-multiply 真値・還元不変条件・`+`/`-`/`*`/`/` 往復を乱数検証）
- J32 単調デック窓集約（tick 窓の極値 — 直近 w フレームの最悪遅延・巡回回廊の範囲値）✅ `slide`（monotonic deque — `segtree` の O(n log n) を「範囲が 1 ずつ滑る」限定で O(n) に圧縮。`slide_min`/`slide_max` が全窓を一巡で返す。brute-force 全窓照合・単調/退化入力を乱数検証）
- J33 最長増加部分列（上達曲線・連撃ボーナス・単調イベント列の最長 run）✅ `lis`（patience sorting 系 `O(n log n)` — `tails` は各長の最小末尾 index を保持、`parent` 鎖で実 witness を復元。O(n²) DP oracle との長一致+復元列の単調性を乱数検証）
- J34 DAG 両端パス（スキル木の最短/最長到達・クリティカルパス — 工順制約のボトルネック鎖）✅ `dagsp`（`topo_sort` 上の `O(V+E)` 両方向 relax — `dag_paths` は lo/hi を同時返却、`critical_path` は「どの点からでも始められる」全頂点0初期化の longest path で critical chain を頂点列で返す。Bellman–Ford oracle・負辺反転同値・閉路→None を乱数検証）
- J35 決定的選択・k番目・中央値（スコア上位k・中央ダメージ・順位報酬 — 乱択 quickselect を避けたい）✅ `bfprt`（median-of-medians — group-of-5 中央値の再帰中央値を pivot に `O(n)` 保証、Dutch-flag partition で `lt/gt` 帰還。sorted 配列 oracle・全重複・逆順 adversarial を乱数検証）
- J36 整数ラスタ化（LOS 光線・弾道セル・ポリゴン領域のセル判定）✅ `raster`（Bresenham — 辞書順小 endpoint 起点の正準方向で逆順対称を構造的保証。midpoint circle は8分円対称 push。`fill_polygon` は2倍座標で「セル中心は奇座標」を使い i128 有理 crossing で境界曖昧ゼロの偶奇判定。DDA oracle・`poly::point_in_polygon` 全セル照合を乱数検証）
- J37 線形漸化式の k 項（周期イベントの跳び読み・漸化式コストの n 週目・Fibonacci 系成長）✅ `linrec`（companion-matrix 冪乗で `O(d³ log k)` — naive `O(dk)` の陪乗置換。`linrec` は `i128` checked で `None` 失敗閉鎖、`linrec_mod` は `u128` 中間で常時 total。naive 漸化式 oracle・mod ⟺ exact 整合を乱数検証）
- J38 最小費用流（単位原価付き輸送・割当+移動コスト — flow の純量版を拡張）✅ `mcflow`（Edmonds–Karp で最大流 → `bellman::negative_cycle` で負閉路を見つけ bottleneck 分だけ回す cycle-canceling — 整数容量で厳密最適、負コスト辺も安全。全域列挙 oracle・負閉路 rerouting 回帰を乱数検証）
- J39 ナップサック DP（重量上限の最適荷物 — 戦利品選択・資源配分）✅ `knapsack`（`O(n·W)` — 0/1 は全 DP 表で witness 復元(tie は先 index 優先)、無限は last[c] 逆たどり。2^n 列挙 oracle・bounded 展開 oracle と乱数照合）
- J40 グラフ彩色（領域配色・チャネル/レジスタ割当 — 隣が違う最小色数）✅ `coloring`（DSATUR — 飽和度→次数→index の決定的 tie-break で二部/サイクルは厳密解。proper 性+小グラフの彩色数 bound を乱数検証）
- J41 取消可能連結判定（「この辺があったら?」仮説クエリ — 条件付き通行可否）✅ `dsurb`（rollback union-find — path compression を捨てて union-by-size+操作ジャーナルで任意 snapshot へ O(深度) 巻戻し。BFS 再構築 oracle との component 一致を乱数検証）
- J42 直線包絡クエリ（線型コスト選択・凸 DP 遷移 — min_j aⱼx+bⱼ）✅ `cht`（Li Chao tree — 区間中央の勝者を各ノードに保持し敗者だけが降りる `O(log X)` 挿入・クエリ。i128 評価、brute-force 包絡と乱数照合）

- J43 可逆変換符号（BWT+MTF — 圧縮前段の局所性集約：リプレイログ・セーブの可逆前処理）✅ `bwt`（巡回 BWT — doubled-string 上の suffix array で回転順を決定、primary index で `inverse` が厳密復元。MTF と合わせて bzip2 型パイプラインの前半を完備。回転行列 oracle・往復一致・周期入力を乱数検証）
- J44 木のパスクエリ（スキル木集計・親子集団の区間操作 — O(log n) 分割へ平坦化）✅ `hld`（heavy-light decomposition — max-size 子=heavy、light 子が新 chain。`path_vertices` で頂点列、`path_segments` で `segtree`/`fenwick` に直載せの `O(log n)` 連続区間、subtree は preorder 連続。祖先 oracle・BFS subtree 集合・乱数照合）
- J45 全域最小カット（ネットワーク脆弱点・クラスタ分割コスト — s-t 未定で最小分断）✅ `mincut`（Stoer–Wagner 最密接頂点収縮 `O(n³)` — 全対 s-t maxflow (`flow`) を oracle に乱数照合、非連結は 0、側集合も返却）
- J46 厳密素数判定・素因数分解（決定論的周期検証・Zobrist 類指数の因数監査 — 疑似乱数に依らない素数性）✅ `miller`（`u64` 全域で決定的 7-base Miller–Rabin — SPSP/Carmichael 全拒否、sieve oracle と 20 万件照合。Brent rho で合成数を固定多項式スケジュールで分解、出力 sorted）
- J47 区間統計クエリ（頻度・中央値・範囲個数 — wavelet matrix で O(bits)/クエリ）✅ `wavelet`（MSB 安定分割 bitplane 行列 — `access`/`rank`/`freq_less`/`range_freq`/`quantile` を整数のみで — 区間 k 番目や値頻度を `segtree` 系と別軸でカバー。brute-force 全操作照合）

- J48 部分文字列の状態数圧縮(全部分文字列の包含・出現数・最長共通 — SA では O(n) メモリを超える重いクエリ)✅ `sam`（suffix automaton — オンライン拡張 + `finish` の出現数伝播。`contains`/`occurrences`/`longest_common`/`distinct_substrings`(Σ len−link.len)/`longest_repeated`。windows 走査・BTreeSet 全部分文字列オラクル照合）
- J49 厳密巡回最適化(完全 TSP — ヒューリスティックの上限が要る小規模巡回経路)✅ `hamdp`（Held–Karp `O(n²·2ⁿ)` n≤16 — `u32::MAX`=辺なしの失敗閉鎖、witness は辞書順最小最適ツアーを貪欲+tail 再計算で復元。全順列 oracle と 120 乱数コスト+witness 照合）
- J50 厳密被覆探索(配置パズル・ポリオミノ敷詰 — 「各行が各列を丁度1度覆う」制約充足)✅ `dlx`（Algorithm X — 最小候補列選択 + disabled-rows ジャーナル undo、辞書順最初の解を返す。200 反復で部分集合枚挙 oracle と可解性一致 + 解の被覆正当性を独立検証）
- J51 有向最小全域木(単一司令系の最小コスト伝達網・根付き通信木)✅ `arborescence`（Edmonds 最小費用有向全域木 — 最小入辺選択→閉路検出→収縮(重み調整 w−best)→展開の再帰。(n−1) 辺 subset 枚挙 oracle + witness の「全非根に入辺1・根へ到達」独立検証）
- J52 xor 線形包監査(ビットマスク特性の結合可能性・最大 xor 選択 — GF(2) 基底の正準形)✅ `xorbasis`（逐次 RREF — 消去済み x が他ピボット bit を持たないため挿入時の MSB 除去が不変条件を保つ。`contains`/`max_xor`/`rank`/`kth` を全閉包列挙・独立 rank oracle・順列不変で照合）

- J53 回文の全構造(回文の種類数・出現数・最長回文を1本のオンライン構築で — `manacher` は半径配列のみ)✅ `eertree`（palindromic tree — IMAG(len=−1)根が常に遷移可能で suffix-link 探索の停止条件を担う。`occ` は位置毎の最長接尾辞回文のみ +1 し `finish` で len 降順伝播、BTreeSet 全回文列挙 oracle 照合）
- J54 オフライン区間クエリ一括処理(窓をソートして共有 — データ構造なしで区間集約)✅ `mo`（Mo's algorithm — `(l/block, r)` 偶奇蛇行ソートの `mos_order` + `add`/`remove` で駆動する `range_distinct`、窓は1要素ずつ滑らせる。全区間 brute-force 照合、`isqrt` は整数二分で f64 不使用）
- J55 矩形最大面積(ヒストグラム最大矩形・01 行列の最大真部分行列 — 建物 footprint・倉庫スロット)✅ `histrect`（`largest_rectangle` 単調スタック O(n) — 仮想 h=0 で残スタックを flush、左端は stack.last()+1。`maximal_rectangle` は行毎の高さ累計に流し込み。O(n³) 全ペア min・列ラン全真 oracle 照合）
- J56 安定割当(優先度リスト駆動のマッチング — どの割当にも「より良い相互指名」が存在しない)✅ `stable`（Gale–Shapley — 提案者最適が一意に定まるので提案順は結果に影響しない、`is_stable` で blocking-pair 不存在を独立検証。n≤5 全順列安定マッチング枚挙で男側最適性を照合）
- J57 ポテンシャル付き連結成分(相対高さ・オフセット制約 — `x − y = w` の差分制約の矛盾検出)✅ `wdsu`（重み付き DSU — `weight[x] = pot[x] − pot[parent[x]]`、find で経路圧縮+重み伝播、`unite` は矛盾で false。attach は小側を大側に `rel[v] − rel[u] − w` で接続 — 独立成分リプレイ oracle と全クエリ照合）

- J58 支配関係解析(単一入口領域・spawn gating・CFG 的支配木)✅ `dominators`（Cooper–Harvey–Kennedy 反復法 — RPO 番号付け + `intersect` フィンガーウォークの不動点で `idom`、子リスト整列済み支配木、Cytron 辺境 `frontier`。集合不動点 oracle(`dom[v] = {v} ∪ ⋂ dom[preds]`)と全域照合）
- J59 2次元矩形集約(稠密グリッドの頻度・熱量 — 1-D BIT では矩形和が書けない)✅ `fenwick2d`（2-D Fenwick — `i += i & −i` の2軸版で `O(log w·log h)`、`rect_sum` は包含除去4項。稠密 Vec オラクル全矩形照合）
- J60 下界付き輸送(供給ルート・維持パイプ — 各辺に最小流量がある輸送計画)✅ `circulation`（`feasible_circulation` — `req[v] = demand − lo_in + lo_out` の還元で超源点/沈点に接続、`req>0` は `v→tt`・`req<0` は `ss→v`(方向を誤ると保留流で witness が壊れる)。Hoffman 切断条件全 2^n subset oracle 照合）
- J61 故障分離解析(どの辺が共通サイクル上にあるか — 橋は単体成分として浮上)✅ `biconn`（Tarjan 辺スタック二重連結分解 — `low[w] ≥ disc[v]` で子 subtree の成分を閉じてスタックを pop。単純サイクル全列挙オラクル(2-辺平行サイクル含む)と成分集合一致照合）
- J62 近似重複検出(生成コンテンツの類似度監査 — ビット単位多数決の locality-sensitive 指紋)✅ `simhash`（Charikar — 特徴ハッシュの各 bit が ±weight 投票、正票が bit=1。多重出現=重みの multiset 意味論、tie は 0。bit-major 独立再計数 oracle 照合）

- J63 有限体演算(消去符号・乱択検証の算術核 — GF(2⁸) の乗除/べき/逆元)✅ `gf2`（AES 多項式 `0x11B` 上の農民乗算 + exp/log `Tables`(生成元 3)。`sub`=`add`=xor、`inv`=`exp[255−log]`。2000反復の体公理乱数照合 + 生成元が全非零を巡回する検証）
- J64 消失訂正(パケット損失・shard 再構成 — k/m 任意欠損からのデータ復元)✅ `rsfec`（Vandermonde Reed–Solomon over `gf2` — 係数行列 `V(total,data)·V_top⁻¹` で任意 data 行が可逆。`reconstruct` は現存 data 行の逆行列で復元し parity は再 encode。4-of-8 全 70 subset 網羅 + 60反復乱択損失照合）
- J65 副線形文字列検索(生成テキストの `count`/`locate` — O(m) で全出現)✅ `fmidx`（巡回 BWT 上の FM-index — C 表 + 32 行毎の Occ チェックポイント + 完全 SA。巡回一致(パターンが末尾→先頭へ wrap)を文書化した semantics。naive 巡回照合 oracle 全パターン照合）
- J66 微小重み最短路(0/1 通行コスト・小容量グリッド — Dijkstra の heap 不要領域)✅ `zerobfs`（`zero_one_bfs` は VecDeque 両端、`dial` は `cap·(n−1)` バケット配列 — 申告 `cap` を超える辺は失敗閉鎖 None。`bellman` 最短距離 oracle 600 乱数照合）
- J67 充足可能性判定(パズル規則・配置制約・desync 監査の CNF 判定)✅ `dpll`（DPLL — unit propagation + pure-literal 除去の不動点 + 最小変数 split で canonical model(未設定変数は false)。`solve` は `(clauses)` の純関数。2^n 全割当 brute-force oracle で satisfiability + model 検証を照合）
- J68 区間クエリ(当たり判定窓・時刻区間の stab/overlap — [lo,hi) 半開の中心点木)✅ `intervaltree`（centered interval tree — pivot は端点スパンの中点で必ず ≥1 区間を厳密に横断(終端保証)。`by_lo` 昇順/`by_hi` 降順の二方向リストで stab は 1 側のみ走査、overlap は straddle 時のみ両側下降。`l==r` は stab 退化 semantics、naive 全区間列挙 oracle 300 照合）
- J69 木の経路/距離クエリ前処理(重心分解 — 各除去が成分を半分以下に分割する O(log n) 深さ)✅ `centroid`（centroid decomposition — `parent`/`children`/`depth`/`order`/`roots` + 重心木 `lca`。`sizes` は seen 付き iterative DFS、`find` は spanning-tree 辺のみ歩く重側移動で停止保証。任意木で全除去点の分割サイズ ≤ half を oracle 検証 + 深さ ≤ ceil(log2 n)）
- J70 大きい本文バッファ編集(スクリプト・ログ・シリアライズ状態の insert/delete — piece table)✅ `piecetable`（Crowley 1998 — 不変 `original` + append-only `added` + 片持ちリストの三段構造。`insert` は added 追記 + 包含 piece の split、`delete` は端点 trim + 中間 drop + 隣接 coalesce。`O(#pieces)` locate で Vec splice oracle 300 反復照合）
- J71 多点接続木の近似(中立点 Steiner — ネットワーク配線・リソース接続の最小木)✅ `steiner`（Kou–Markowsky–Berman 2-近似 — 終端間最短路の metric closure → Kruskal MST → 経路展開 → 残 MST で cycle prune。Dreyfus–Wagner 厳密解 oracle(k≤5)で `opt ≤ w ≤ 2·opt` を 300 乱数照合 + tree 形状検証）
- J72 ジャーナル永続化(WAL — 書換え前の append-only 記録、torn tail 耐性)✅ `wal`（`[kind|len|crc|payload]` レコード、crc は domain 分離 Fnv1a。replay は truncated/corrupt レコードで停止し `stopped_at` オフセットを返す torn-tail 耐性 semantics — 全 cut で clean ⟺ 境界位置を検証、bit-flip で以降拒否、truncate で破損除去）

- J73 近似文字列照合(typo 許容の検索・did-you-mean — k 置換以内の一致列挙)✅ `bitap`(Shift-And ビット並列 — 1 文字毎の状態遷移を `u64` 1 ワードで進め、Hamming-fuzzy は error 行毎に 1 ワード追加。置換項の bit-0 種付け(空 prefix は常に真)を欠くと pattern[0] 置換が落ちる bug を naive Hamming oracle が捕捉 — `(prev<<1)|1` で確定。挿入/削除は許容しない Hamming 意味論、naive 全窓照合 400 反復)
- J74 群体経路誘導(1 目的〜全 agent の O(1)/step 誘導 — integration + vector field)✅ `flowfield`(goal set 逆向き Dijkstra の統合フィールド + 各セル最小 dist 隣接への方向場。8 連結・両 ortho 通行可能時のみ対角で角抜けなし、整数 √2≈3/2 倍率で 2·cost 単位。descent 単調 + 終端到達性を oracle 検証、壁セルは dir 無し)
- J75 式・ルール文字列の評価(ダメージ式・条件式 — 中置→後置→スタック評価)✅ `shunting`(Dijkstra shunting-yard — `+ - * / %` 二項と右結合単項 `Neg`、括弧。交互構造の parse 検証で eval 可能な postfix のみ産出、i64 切断 `/ %`・`/0`・overflow は全て None 失敗閉鎖。再帰降下オラクル 2000 乱数照合)
- J76 凸形状の衝突判定(SAT — 分離軸の非存在が重なりの証 + 最小重なり軸目撃者)✅ `sat`(両多角形の辺法線のみが候補軸、投影重なりで早期 exit。境界接触は重なり、`depth` は投影単位(|axis| 除算が真の深さ)、axis は重心差で a→b 正準向き。edge-intersect + 包含 oracle 3000 照合)
- J77 固定アリーナの冪2 割付(buddy allocator — split-on-alloc/merge-on-free)✅ `buddy`(log2 分離 free list、最小 order・最低位アドレス選択で状態は操作列の純関数。free は buddy 同時解放なら合体を繰返す eager coalescing —「二つの buddy が同時に free」は常に不成立の canonical 性質。byte-shadow oracle + 全 drain で全域使用可能を検証)

- J78 静的メンバーシップ判定(xor filter — 3 スロット XOR で bloom の片方向誤りのみ)✅ `xorfilter`(BFS peel の degree-1 キューで構築、残存 2-core は seed 交代で最大64試行。fingerprint は0以外の `u8` — 挿入済みキーは構築保証で必ず `true`、fp ~1/256 を 4000 probe で境界検証)
- J79 置換付き作業集合(LRU cache — 最古の最近使用を排出)✅ `lru`(BTreeMap×2 で (stamp,key) 辞書順の正準排出 — 挿入順・ポインタ非依存。`get` は使用刻印を更新、`peek` は不変。VecDeque シャドー oracle 全 op 照合)
- J80 O(1) 重み付き抽選(Vose alias — 構築 O(n)・抽選 1 コイン)✅ `vose`(`prob`/`alias` を `u128` 厳密に構成 — 全 (bucket,coin) ペア `n·total` 件を枚挙し各 item に `w_k·n` 件の厳密分配を oracle 照合。small/large の pop は index 最大側固定で正準)
- J81 動的点空間索引(bucketed quadtree — 挿入分割・矩形クエリ)✅ `quadtree`(半開矩形 4 分岐、bucket 超過で分割、1-wide 帯は分割不可で leaf 溢れ — hang しない。回答はソート正準、`nearest` は子矩形 min-dist² の best-first。全矩形 oracle + 最寄り brute-force 照合)
- J82 配列→木の正準橋(cartesian tree — heap on values + BST on positions)✅ `cartesian`(Vuillemin O(n) スタック構築、重複は (val,idx) 辞書順で一意 — 左端最小が根。`rmq` = i,j の LCA が範囲極値を答える。heap 順序・inorder=0..n・ブルートフォース argmin・部分木連続区間性を全検証)

- J83 索引付き優先度キュー(indexed heap — key でdecrease-key 可能)✅ `iheap`((prio,key) 辞書順 pop で正準、`set`/`decrease`/`increase`/`remove` を pos[] 逆引きで O(log n)。BTreeMap オラクル全 op・drain 順照合)
- J84 範囲順位統計の永続版(chairman persistent segment tree — path-copy で全 prefix に root)✅ `pstree`(座標圧縮 + 差分 root 対で `a[l..=r]` の `kth`/`freq`/`range_count` を O(log n)。slice sort oracle 全照合、版間非 alias を検証)
- J85 wire 完全性チェックサム(CRC-32 IEEE — チャンク不変)✅ `crc`(table-driven reflected 0xEDB88320、既知ベクタ CBF43926 + 全 1bit 反転を検出 + 任意チャンク分割で oneshot と一致 — bit-level 除算 oracle 照合)
- J86 確率的ゲーム木探索(UCB1 MCTS — 反復制の section 探索)✅ `mcts`(integer-only UCB — `ln`→`log2` で定数吸収、勝率は permille。ロールアウトは SplitMix64 seeded で (position,budget,seed) の純関数。**子の勝率は親視点 1000−mean で読む** のが UCB の要所 — 子 mover の stored view を直接使うと best-first が逆転し、oracle(必勝手選択)が捕捉。強制勝ち・必須ブロック・seed 一致を検証)
- J87 世代付き安定ハンドル(slot map — gen バンプで stale を構造拒否)✅ `slotmap`((slot:u32,gen:u32) 詰め込み u64、remove で gen+1 → 旧 handle は永久に不成立。LIFO recycle + gen 枯渇で永久退役。BTreeMap オラクル 2000 op + 世代分離・不明 handle 全拒否を検証)

- J88 削除可能な近似メンバーシップ(cuckoo filter — Bloom では消せない)✅ `cuckoof`(u8 fingerprint + `h2=h1^hash(fp)` の2候補バケット、bounded kick で seed 純関数。偽陰性ゼロ・片方向誤りのみを乱択照合、BTreeSet オラクル 2000 op で live 集合一致)
- J89 エントロピー限界への整数符号(rANS — Huffman の 1bit 下限を割る)✅ `rans`(largest-remainder 正規化で 2^12 台、u64 単状態+ u16 排出で往復一致。逆順 consume→decode で順序復元、wire=[len][state][rev(u16s)] で切り詰め・末尾ゴミを全拒否 — 圧縮梯子の最終段)
- J90 内陸極点(pole of inaccessibility — ラベル/湧き点の最深部)✅ `polylabel`(mapbox 系 B&B を整数化 — セル上界を `(ceil√d²+ceil√r²)²` に保持して平方距離の厳密比較のみで剪定。lattice 全点ブルートフォースと最適値一致、凸形状乱択・L字・退化全照合)
- J91 静的完全ハッシュ(CHD displacement — n keys→[0,n) 全単射)✅ `mphf`(2段 displacement、バケットは (size desc,idx) 正準順で解決、(key set,seed) の純関数で挿入順非依存を検証。dispatch 表・opcode 索引向け)
- J92 貪欲最大独立集合(greedy MIS — 昇順正準形)✅ `mis`(index 昇順走査・既選択近傍が無ければ採用 — 辺集合のみの一意決定、自己ループは構造的に不適格。独立性+極大性を定義通り全乱択検証)

- J93 対称ストリーム暗号(ChaCha20 — ARX 20 ラウンド)✅ `chacha`(RFC 8439: `[SIGMA|key8|counter|nonce3]` u32 状態、10 回の列+対角ダブルラウンド、キーストリーム = 作業状態+初期状態。§2.3.2 ブロックベクトル `22 4f 51 f3`・§2.4.2 暗号ベクトル `6e 2e 35 9a`・チャンク分割≡一括を既知値検証 — replay ワールドの per-tick 暗号ストリーム)
- J94 暗号学的ハッシュ(SHA-256 — FIPS 180-4)✅ `sha256`(K[64] 定数 + メッセージスケジュール σ0/σ1、インクリメンタル write + 正準 BE パディング。`abc`=`ba7816bf`・空=`e3b0c442`・56文字・10^6×'a'=`cdc76e5c` 全既知値一致、チャンク分割不変 — `merkle`/`crc` の暗号級ワイヤ検証補完)
- J95 密度クラスタリング(DBSCAN — eps²/minPts、クラスタ数不要)✅ `dbscan`(平方距離判定で √ 排除、昇順 index 正準のシード展開 + キューで (points,eps2,min_pts) の純関数。コア点は必ずラベル化・境界点はコア近接・クラスタコアは eps 連鎖の3条件を定義照合 — spawn 群検出・熱分布解析向け)
- J96 決定的クラスタリング(整数 k-means — seed 不要)✅ `kmeans`(farthest-point 初期化で乱数排除、Lloyd 反復を assignment 不動点まで。argmin 一貫・冪等・inertia 再計算一致を乱択照合 — `dbscan` の k 指定補完、勢力分割・拠点割当)
- J97 静的矩形索引(STR 梱包 R-tree — 点集合の純関数)✅ `rtree`(sort-tile-recursive 全ソート梱包で挿入順が構造的に漏れない packed R-tree。矩形クエリは昇順正準でブルートフォース全照合、形状は点集合のみの関数 — レベルロード時構築の読み取り専用索引)

- J98 一時認証子(Poly1305 — RFC 8439、chacha の AEAD 対)✅ `poly1305`(5×26-bit DJB limb 設計: r=clamp 済み乗算器 + s=128bit 加算項、ブロック毎 `h=(h+block)·r mod 2^130−5` を桁上げ連鎖で。partial block の `0x01` 終端・h−p 選択・LE pack を全て整数手続きに落とし込み — §2.5.2 既知タグ `a8061dc1` 一致 + 分割不変。replay ワイヤの per-tick 認証)
- J99 前駆/後継集合(proto van Emde Boas — sqrt 2 段分解)✅ `veb`(u32 を hi:lo=16+16 に分解、top bitset が非空クラスタを標記 — predecessor/successor/min/max が定数級 bitset 走査で応答。BTreeSet oracle 4000 乱択全照合 + クラスタ境界ケース — ソート順で近傍が欲しい entity-id/タイムライン集合)
- J100 圧縮ビットマップ集合(Roaring — コンテナ分割)✅ `roaring`(上位 16bit でコンテナ分割: 疎=sorted u16 array、密=1024-word bitset、4096 閾値で双方向変換。集合演算はコンテナ対を wordwise 合成 + normalize — (membership) の純関数で挿入順非依存を検証。BTreeSet oracle で union/intersect/difference/sym-diff 全照合 — entity フラグ・rank 索引の常駐表現)
- J101 Lyndon 分解・最小回転(Duval/Booth — 周期構造の正準形)✅ `lyndon`(Duval `O(n)` 因子分解: `s[i..j)=w^p·w'` で完全コピー `i≤k` 個のみ emit — 周期語混入 bug を因子全 Lyndon オラクルが捕捉。Booth 最小回転 index・`is_lyndon` を全回転枚挙で照合 — 巡回構造(necklace 盤面・回転対称ステート)の正準形化)
- J102 部分集合格変換(SOS zeta/Möbius + Walsh–Hadamard — 畳み込み基盤)✅ `sosdp`(subset/superset zeta↔Möbius 逆対 `O(n·2^n)`、OR/AND 畳み込み = zeta→点ごと積→Möbius、xor 畳み込み = FWHT 版。naive O(4^n) 全照合 + WHT 畳合 2^n スケール検証 — `conv`/NTT の多項式積と並ぶ bitmask DP の計数基盤)
- J103 決定性スキップリスト(hash レベル — 順序集合の確率的平衡)✅ `skiplist`(Pugh の skip list のコイン投げを `trailing_zeros(hash(key,seed))` に置換 — 幾何分布レベルが (key,seed) の純関数で車線構造が挿入順非依存。BTreeSet oracle で全 op 照合 + 全レベル車線キー列が 2 構築順で一致 — ヒープ非依存の順序マップとして `treap` の対極)
- J104 Karatsuba 多倍長乗算(u64 limb — サブ二次積)✅ `karatsuba`(base-2^64 limb の O(n^1.585) 積: CUTOFF=16 で schoolbook に降下、z1=(a0+a1)(b0+b1)−z0−z2 を magnitude 演算で。u128 中間のみ — RSA/将来の bigint 需要と多項式評価の乗算基盤、schoolbook oracle 全乱択照合)
- J105 LSM 索引(memtable + 凍結ソート実行 + 墓石 — 書込優先順序ストア)✅ `lsm`(BTreeMap memtable が容量で凍結 run 化、`insert(0)` で newest-first 積層、iter/compact は最新勝ち merge、tombstone 削除。観測状態が操作列の純関数 — `wal`/`delta` の永続裏付けとして BTreeMap oracle で全 op + compaction 前後一致を検証)
- J106 PGM 学習索引(区分線形モデル — 予測位置±ε 局所探索)✅ `pgm`(Ferragina–Vinciguerra の整数版: 固定サイズ区分に有理傾斜 + 構築時の厳密 max deviation ε を保持、predict→[p−ε,p+ε] binary search。全工程整数のみ — `veb`/`roaring` と並ぶ第三の u64 索引戦略で brute-force rank/get oracle 全照合)
- J107 AES-128 ブロック暗号(FIPS-197 — S-box は gf2 逆元+affine で算出)✅ `aes`(テーブル非格納: `sbox(x)=affine(gf2::inv(x))` を構築時計算、GF(2^8) mix_columns は `gf2::mul` の 9/11/13/14 行列。FIPS-197 §C.1 既知解答 + 全256定数ブロック往復 + 1bit 反転の avalanche ≥8/16 bytes — `chacha`/`sha256`/`poly1305` の暗号家族をブロック暗号で完備)

- J108 区間加算 Fenwick(range-update BIT — 差分配列/二段 BIT)✅ `fenwickrange`(`RangePoint`: 区間加算+点クエリの差分 BIT、`RangeSum`: 区間加算+区間和の二 BIT 構成 `P(x)=prefix(B1,x)·x−prefix(B2,x)`。0 基点半開区間→1 基点内部へ変換 — naive 配列 oracle で全乱択照合)
- J109 ジオハッシュ(整数 geohash — 緯経度の 5bit 交互 bisect)✅ `geohash`(microdegree e6 整数 lat/lon を lon 先交互 bisect で base32 化、decode が cell 境界を返す。cell_span・8 近傍 clamp — 500 乱択で decode∘encode 包含 + prefix 入れ子性を検証)
- J110 八分木(bucketed octree — 3-D 点索引)✅ `octree`(BUCKET=8 で leaf→branch 分割、s≤1 で分割停止して同一座標積み上げでも hang しない。ソート正準回答 + best-first nearest — BTreeMap multiset oracle・brute-force 最近傍照合)
- J111 Base64 符号(RFC 4648 — strict padding 検証)✅ `base64`(std + URL-safe 両 alphabet、decode は pad≤2・末尾のみ・alphabet 外 byte 拒否の厳格検査。RFC §10 既知ベクタ + 全長・全 256 byte 往復 — `wal`/`savefile` の wire 表現層)
- J112 König 最小頂点被覆(二部マッチング — 交互到達で被覆復元)✅ `vertexcover`(hopcroft_karp のマッチングから自由 L 頂点起点の交互 BFS → (L\Z)∪(R∩Z)。マッチングサイズ=被覆サイズの相互検証 + n,m≤4 全列挙 minimality — `bipartite` の双対定理実装)

- J113 SipHash 鍵付きハッシュ(Aumasson–Bernstein 2012 — 64bit PRF)✅ `siphash`(SipHash-2-4、u64 add/xor/rot のみ、8-byte staging で分割非依存の streaming。論文ベクタ 8 件検証 — `DetHash` の鍵付き対極で DoS 耐性 hashmap・署名付き seed 表現向け)
- J114 HMAC-SHA256(RFC 2104 — ipad/opad 二重ハッシュ MAC)✅ `hmac`(64-byte block 0x36/0x5c パッド、長 key は先に SHA-256 — RFC 4231 TC1/2/4/6 既知解答 + 分割非依存。`poly1305` と並ぶ MAC のハッシュ型版)
- J115 ペアリングヒープ(meldable 優先度キュー — O(1) meld)✅ `pairingheap`(Fredman–Sedgewick の pairing heap を Vec arena で。(prio,key) 全対の正準 pop 順 — 二段 pairing pass で根車線を浅く保つ。BTreeMap oracle で push/pop/meld 全乱択照合)
- J116 ビトニックソート網(Batcher 網 — 入力非依存比較列)✅ `bitonic`(`network(n)` が n のみの固定 (i,j,asc) 列を生成 — 全 peer が同一比較痕を replay する data-oblivious 整列、非 pow2 は !0 sentinel パディング。lockstep ガジェット/ソート検証器向け)
- J117 Tarjan オフライン LCA(DSU + 一 DFS — バッチ祖先クエリ)✅ `offlinelca`(黒化した w に対し lca=ancestor[find(w)]。DSU が root 間で共有されるため tree_of 番号で跨木クエリを None に遮蔽 — binary-lifting `lca` と全乱択一致、5000 深連鎖も iterative で安全)

- J118 Elias–Fano 単調整数列(succinct monotone sequence — unary gap + verbatim low bits)✅ `elias`(Elias 1974/Fano 1971。`access`/`rank`/`successor` が word popcount 走査のみ — `rank` は「h番目のゼロまでの one 数 = hi ≤ h の要素数」の境界ずれを補正して検証。順序集合・インデックスの圧縮層)
- J119 PATRICIA/crit-bit 木(radix trie over u64 — 高々64段)✅ `patricia`(Bernstein crit-bit 形式の arena 実装。in-order 走査 = 数値昇順。floor/ceil は「未テスト bit で off-branch が bound を潜り得る」性質から単純下降では誤答 — 両側を min/max 刈り付き探索に確定し BTreeSet 全照合)
- J120 BLAKE2s(RFC 7693 — ChaCha 型 G ミキサの暗号ハッシュ)✅ `blake2s`(IV は SHA-256 継承、param block で digest/key/fanout/depth を注入。keyed-MAC モードは key block を*バッファに留める*設計 — 先に圧縮すると空メッセージで phantom 最終 block になる bug を RFC ベクタが捕捉)
- J121 回転キャリパ(rotating calipers — 凸包の直径/幅/最小面積外接矩形)✅ `rotcal`(単調 advance の antipodal 走査、全演算 i128 cross product で Frac 面積・Frac 幅。最小矩形の 4 本 caliper は edge-0 で線形初期化が必須 — jl の単調指針が 0 手前の真最小を見落とす bug を brute-force oracle が捕捉)
- J122 SMAWK(AKMSW 1987 — 全単調暗黙行列の O(n+m) 行 argmin)✅ `smawk`(Reduce が列を stack 刈り → 奇数行に再帰 → 偶数行は境界内走査。Monge-DP/Knuth 最適化の土台 — テスト生成器は積項が anti-Monge になる罠を検証で確定、真 Monge = a+b−wx 昇順ペア)

- J123 ロープ(balanced rope — 大テキストの O(log n) 編集)✅ `rope`(葉チャンクの二分木。depth > 2⌈log2 n⌉+1 で引き金になる全葉再構成 rebalance — 先頭連続挿入の退化も線形復帰。`mem::take` 評価順の穴を len 先取りで修正)
- J124 BK 木(メトリック木 — Levenshtein 距離の索引)✅ `bktree`(辺ラベル=親鍵との編集距離、検索は [d−r,d+r] 帯だけ下降 — 三角不等式刈りで brute-force の一桁絞込み、結果は (dist,key) 正準)
- J125 最小包含円(Welzl — smallest enclosing circle)✅ `mincircle`(sort+dedup で正準入力 → 3重ループの反復 Welzl。共線/重複を widest-pair 直径フォールバックで処理、座標は全て Frac — MEC 一意性で入力順非依存を保証)
- J126 スロープトリック(凸区分線形関数 — DP 遷移の高速化)✅ `slopetrick`(L=max-heap/R=min-heap の正規4行形: `min_f += max(0,a−topR); pushr(a); pushl(popr)`。slide(a,b) の平行 offset で最小区間が [lo+a,hi+b] に — a>b(空窓)は規約外。密グリッド oracle が境界汚染の罠を検証)
- J127 Thompson NFA 正規表現(バックトラックなし byte 正規表現)✅ `regex`(再帰降下→命令列コンパイル、ε-closure を `seen` 集合で絞る pike loop。unanchored は各ステップで start を再播種、out ポインタを HOLE=!0 sentinel で後patch。AST レンダ oracle で端位置集合を全照合)

- J128 カッコーハッシュ(2 テーブル開番地 — 削除可能・tombstone 不要)✅ `cuckoo`(h1/h2 の二 home 配置、キック連鎖は交互テーブルで budget 超過時はジャーナル全巻戻し — 既存キー迷子を構造排除。固定容量で rehash 方針の分岐なし)
- J129 ビットボード(u64 盤面 — 8x8 集合演算)✅ `bitboard`(file mask クランプの8方向シフト + dumb7fill 遮蔽 fill で rook/bishop/queen の ray attack — leaper は単步 oracle で別系統検証)
- J130 CYK 構文認識(CNF 文法 — 動的計画受理)✅ `cyk`(bin[a][b] を lhs bitset に前計算した O(n³) 三角表。`accepts`/`derive`/`cell` — メモ化再帰 oracle で小規模 CFG を全照合)
- J131 半平面交差(実行領域の凸多角形 — deque 構築)✅ `halfplane`(方向角を quadrant+cross で整数整列、頂点は全て Frac。閉交点 push・連続重複/共線の正規化・shoelace で CCW 化 — 空/非有界/退化は None で失敗閉鎖)
- J132 y-fast トライ(rep 層+クラスタ — u32 前駆後継)✅ `yfast`(内容分割 bucket が median で分裂、rep 昇順 Vec。predecessor は strict<、successor は inclusive≥ で veb と意味合わせ — BTreeSet oracle で全演算照合)
- J133 基数/計数ソート(比較なし線形整列)✅ `radixsort`(8bit×8 pass の安定 LSD + `[0,bound)` 計数ソート + `(key,payload)` 安定版 — 挿入順を残したいイベント整列向け)
- J134 rank/select ビットベクトル(稠密索引の基底)✅ `rankselect`(512bit superblock の Jacobson 二段 directory、word 内 select は byte 走査 — wavelet/elias の下位プリミティブ版)
- J135 de Bruijn 列(全 k-mer を一巡する巡回列)✅ `debruijn`(FKM の Lyndon 語連結で B(k,n) を一発生成 + `is_debruijn` 検証器 + `window_hash` 巡回窓指紋)
- J136 連分数(有理数の正準展開)✅ `cf`(`to_cf`/`from_cf`/`convergents`/`best_approx` — cap 内分母の最良近似は semiconvergent 境界との2候補比較、同距離は小分母優先)
- J137 Earley 構文認識(任意 CFG — ε規則・混長産出可)✅ `earley`(位置別 queue の predict/scan/complete fixpoint、合成 S'→S で受理判定 — cyk の CNF 制約を解除、メモ化再帰 oracle 全照合)
- J138 誘導ソート接尾辞配列(線形 SA 構築)✅ `sais`(S/L 型分類→LMS 部分列ソート→簡約文字列再帰、suffix の prefix-doubling とは別系統の O(n) 構築 — naive 全位置照合)
- J139 HyperLogLog 基数推定(loglog 空間の distinct count)✅ `hll`(p bit レジスタ+max merge、raw αm²/Z を i128 固定小数で評価+小域は Q32 atanh 級数の整数 ln で線形計数補正)
- J140 算術符号化(区間細分 entropy 符号)✅ `arith`(Subbotin carry-less range coder — キャリー伝搬を range 切詰で回避した byte 出力のストリーミング符号、呼出側周波数モデル・往復完全一致)
- J141 SwissTable 型制御 byte 表(高密度単一テーブル)✅ `swiss`(16 slot probe group+7bit h2 fingerprint で key 配列非接触の判定多数、tombstone 削除、負荷15/16&墓石閾値で倍長 rehash — cuckoo との設計対極)
- J142 マジックビットボード(衝突なし掛算索引 attack 表)✅ `magic`(relevant occ = 各 ray 終端のみ除く `ray & opp(ray)`、seeded magic 探索で (occ·m)>>shift 完全ハッシュ、尽きれば dumb7fill oracle に正直 fallback)
- J143 Link–Cut 動的木(根付き森の link/cut/path 集約)✅ `linkcut`(aux splay+遅延 rev push の標準形 — access の戻り値が最後の経路親=LCA、make_root で reroot、BFS oracle と連結/経路 min を照合)
- J144 スプレー木(アクセス局所性による償却平衡 BST)✅ `splay`(bottom-up zig/zig-zig/zig-zag — 木形状は操作列のみの関数で RNG 不要、BTreeSet oracle と全 op 照合)
- J145 ビット並列編集距離(Myers 1-word automaton)✅ `editdist`(≤64 パターンの `dist` と `find_leq` 近似照合端位置 — 左列注入 bit が全文/自由開始の境界を切替、DP oracle 乱択照合)
- J146 GJK 凸体距離(Minkowski 差 support 写像)✅ `gjk`(2D 整数 GJK、最近点を Frac 有理数で保持し厳密二乗距離 — SAT oracle ブール+有理数 brute oracle と 400 乱数凸包照合)
- J147 TLSF アロケータ(segregated-fit O(1) 動的割付)✅ `tlsf`(fl/sl 二段 bin + free coalescing — 「収まるブロックがあるのに拒否しない」正直拒否を oracle が強制、最低アドレス採用で決定的)
- J148 SHA-512(64bit 語ハッシュ・長文標準)✅ `sha512`(FIPS 180-4、80 段 K 定数・128bit 長さフィールド・手動 BE 語 load — NIST 全 4 ベクトル+分割不変)
- J149 EdDSA 署名(twisted Edwards 上の Schnorr 系)✅ `ed25519`(RFC 8032 — radix-51 リム GF(p) + mod-L スカラー分離、EFD 完全加法、非正規 s/encoding 拒否、公式 TEST1-3 ベクトル)
- J150 一般グラフ最大マッチング(奇閉路花の収縮)✅ `blossom`(Edmonds — BFS 内 base[] 縮約の e-maxx 形 O(n³)、n≤9 全列挙 oracle + 決定性)
- J151 EPA 侵入深度(原点包含多胞体拡張)✅ `epa`(GJK 収束 simplex を seed、最近 edge を外向法線 support で拡張 — SAT witness を brute oracle にした Frac 厳密 MTV)
- J152 α-shape(スケール付き点集合境界)✅ `alphahull`(delaunay 三角形を外接半径² ≤ α² で選別、1 回出現 edge が境界 — 垂線二等分線外心と abc/4A の独立 2 式で半径照合)

- J153 implicit treap(ランダム不要な列編集木)✅ `imptreap`(seeded 優先度 min-heap マージ + 遅延 rev フラグ — 伝播中ノードの子側有効フラグ反転は「親の保留 flip が子の effective rev を反転」する不変式で確定、Vec オラクル全 op 照合)
- J154 meet-in-the-middle(半全列挙組合せ探索)✅ `meetmid`(subset_sums/subset_sum/count_subsets/best_fit — 右半分 binary search + 再列挙 witness 復元、i128 和でオーバーフロー安全)
- J155 GF(p) 線形連立(mod 素数の RREF)✅ `modlin`(u128 積 mod・Fermat 逆元・free vars=0 規約の solve/nullspace — 部分集合枚挙・全代入の両 oracle 照合)
- J156 recursive vEB(O(log log U) 前駆後継)✅ `veb3`(LEAF_BITS=6 の葉 mask + summary/cluster 再帰 — min をクラスタ外に保持する CLRS 式では「summary.predecessor が無い hi に min フォールバック」が必須、葉の shift guard は宇宙境界ではなく幅 64 で確定)
- J157 multi-word bit-parallel DP(64 語超オートマトン)✅ `bigedit`(⌈m/64⌉ 語の Myers frontier — `(Eq&Pv)+Pv` の多倍長キャリー連鎖と `Ph`/`Mh` 左シフトの語間繰上り、score は最終語の bit m−1 のみ読む)
- J158 canonical LEB128/zigzag wire codec(最小符号化を検査する写像符号化)✅ `varint`(encode/decode 往復一致 + 非 canonical 拒否 — 終端 byte の trailing-zero 群と第10 byte の `payload==1` 制約で「値→唯一の byte 列」な写像に確定)
- J159 Dowling–Gallier 線形 Horn SAT(最小モデル帰結)✅ `hornsat`(`Clause{pos,neg}` + watch[v] 逆引き + remaining カウンタの FIFO 単位伝播 — brute 2^n オラクル全照合、unit 導出で least model 一意)
- J160 sign-magnitude 多倍長整数(u64 limb BigInt)✅ `bigint`(add/sub/mul/pow + `to_i128`/`to_u64` 往復 — `−(i128::MIN)` の符号反転桁溢れは `m==1<<127` の直接返却で確定、i128 checked_* oracle 2000 照合)
- J161 segment tree beats(範囲 chmin/chmax/sum)✅ `segbeats`(max/smax/cmax + min/smin/cmin の第二極値帳簿で amortized O(log² n) — push は「親の mx/mn を子へ clamp」のみで明示 lazy タグ不要、読み取り側も stale 子 sum を避けるため push 必須)
- J162 Euler-tour tree(動的森連結性)✅ `ett`(各頂点の恒常 vertex-node + 有向 half-edge ノードの巡回列 implicit treap — link は代表ノードでの reroot + `U+[uv]+V+[vu]` 連結、cut は `A x B y C` の 4-split、connected は親指針の root 比較 — linkcut と同じ森だが連結のみなので splay expose が一切不要)

- J163 strict DFA UTF-8 codec ✅ `utf8`(Höhrmann 364-state transition table — `validate`/`check`/`decode`/`decode_lossy`/`encode` + streaming `Decoder`; lossy resync は「lead で reject された byte は消費・mid-sequence で reject は再供給」の `e.pos == seq_start` 判定で stray continuation の無限ループを回避)
- J164 lazy segment tree(range add + range 集約)✅ `lazyseg`(単一加算 lazy タグの正準形 — push は子の実葉数 `len/2` で sum をスケール、`ceil(len/2)` 誤用で add が多めに畳まれる bug を oracle が捕捉)
- J165 Tonelli–Shanks(素数法平方根)✅ `tonelli`(`p−1 = q·2ˢ` 分解 + 逐次非剰余探索で決定的、`sqrt_mod` は `(lo, p−lo)` 整序対、`p≡3 mod 4` は直接式)
- J166 baby-step giant-step(離散対数)✅ `bsgs`(baby 表 `g^j→j` + giant 歩行 `h·f^i`、`f = g^{p−1−m}` で逆元ヘルパ不要、最小 x 保証 — `g=0` は `f` が真逆元でないため先に解決)
- J167 Hopcroft DFA 最小化(分割精細)✅ `dfamin`(splitter worklist で小さい半分のみ再キュー、block id は最小メンバーで正準化 — naive signature-iteration oracle で200乱択全照合)

- J168 de Boor B-スプライン評価 ✅ `bspline`(Frac 厳密 — 退化 span は α=0 正規化で `continue` による前段残りを回避、右端は最後の非空 span ≤ n の左連続意味論)
- J169 Berlekamp–Massey 最短 LFSR ✅ `bmassey`(GF(p) — C[0]=1・C は L+1 長、coef は Fermat 逆元 `d·b^{p−2}`、holds 検証器併設)
- J170 bitwise xor trie(max/min-xor)✅ `xortrie`(multiplicity cnt、greedy opposite-bit descend、O(n·64) max pair は (lo,hi) 正準 dedup)
- J171 add-wins OR-Set CRDT ✅ `orset`((replica,ctr) dot、remove は観測済み dot のみ — 並行 add 不滅、merge は両 map union で交換・冪等・結合)
- J172 LWW-element-set CRDT ✅ `lww`((clock,replica) stamp の勝者側が生死を決定、同時刻タイは remove 勝ち — orset の対極で未観測要素も remove が stamp を書く)

- J173 線形篩 + 区間素数表 ✅ `sieve`(SPF で phi/tau/sigma/factor、segmented `primes_between` は `ceil(lo/p)·p` を `p^2` にクランプ — Miller–Rabin オラクル照合)
- J174 Sprague–Grundy 不偏ゲーム数 ✅ `grundy`(mex・take-away 表・多山合成・`detect_period` — 周期報告は末尾 `memory` 個の検証済み周期を定理として要求)
- J175 ギャップバッファ(Emacs 型)✅ `gapbuffer`(連続 gap、`copy_within` の両方向移動、delete は実削除数返却 — Vec シャドー oracle 全 op 照合)
- J176 Robin Hood 開番地集合 ✅ `robin`(probe 長の強奪挿入 + 後退シフト削除で tombstone 不要、`max_probe_len` 診断、0.75 負荷で slot 順 rehash)
- J177 winnowing 文書指紋 ✅ `winnow`(k-gram ハッシュ列の各窓から rightmost-min を選択 — k+w−1 バイトの共有走査で必ず共通指紋が出る保証)

- J178 scapegoat 木(α 重み平衡)✅ `scapegoat`(挿入時 `4·size(child) > 3·size(node)` の最深祖先を中央値再構築、削除後は `len < α·max_size` で全木再構築 — ノード毎 α 不変式は挿入後のみ保証)
- J179 左辺ヒープ(mergeable PQ)✅ `leftist`(rank=null path length、右背骨のみ O(log n)、`from_slice` 対 meld O(n) — multiset オラクルで構造不変式全照合)
- J180 ビーム探索 ✅ `beam`((score,生成順) 正準ランク、parent-link 経路復元、`minimax::Game` 上の `beam_moves`)
- J181 パーセプトロン線形分類 ✅ `perceptron`(i128 スコア・飽和 `w+=y·x` 更新、update-trace oracle で教科書規則と完全一致、one-vs-rest 多クラス)
- J182 鞍背探索(行+列ソート行列)✅ `saddleback`(右上起点で每步 row/col 破棄 O(r+c) — 全走査 oracle で phantom-hit/見落とし両方向照合)

- J183 AVL 木(高平衡)✅ `avltree`(全変更で `|h(l)-h(r)|≤1` を回転復元 — BTreeSet シャドーで op 毎に不変式照合、昇順挿入でも高 ~log n)
- J184 マルチアーム・バンディット ✅ `bandit`(UCB1 の Q8 整数形 `mean·256+isqrt(bonus)` + seeded ε-greedy — 全腕訪問→収束を検証、epsilon=0 で greedy 退化)
- J185 Zobrist ハッシュ ✅ `zobrist`((piece,square)→u64 鍵 XOR、toggle=厳密 undo、side-to-move 鍵は表末尾から派生 — transposition table 用増分ハッシュ)
- J186 教科書 RSA(パディング無し)✅ `rsa`(bigint 上で除法を自前実装: 二進長除法 rem/商、modpow、拡張 Euclid modinv、固定証人 Miller–Rabin — seed 決定的鍵生成、署名/暗号往復)
- J187 JSON パーサ(整数部分集合)✅ `json`(RFC 8259 − float、厳密拒否: leading zero・lone surrogate・非終端・末尾ゴミ、canonical render は BTreeMap ソート鍵 — `parse(render(x))==x`)
- J188 接尾辞木(Ukkonen オンライン構築)✅ `sufftree`(仮想終端 `SENT` で全接尾辞が固有葉を保有、active point + suffix link + skip/count 降下、`contains`/`occurrences`/`count`/`longest_repeat` — naive 全照合)
- J189 赤黒木 ✅ `redblack`(CLRS insert/delete-fixup の arena 実装、NIL sentinel は親+向きで追跡、`check()` が BST順・赤赤なし・黒高等差を監査 — BTreeSet シャドー照合)
- J190 全点対最短路(Floyd–Warshall + Johnson)✅ `apsp`(i64 辺・i128 内部、Johnson は `bellman::shortest` ポテンシャルで `w'≥0` 化、負閉路検出 — 3 者照合)
- J191 PageRank(整数化)✅ `pagerank`(Q32 質量 `SCALE=1<<32`、辺配分+teleport+dangling 均等分、全 floor 意味で縮約収束、`converged` フラグ報告)
- J192 有限オートマトン(NFA→DFA)✅ `automaton`(Thompson 構成 + 部分集合構成 + `complement(alphabet)`/`intersect`/`minimize` — dfamin 連携、`complete()` の or_insert 化で sink 上書き bug 修正)

- J193 AA 木 ✅ `aastree`(赤黒の2不変式簡略版 — `level = left+1`、NIL=0 計上、削除巻戻は decrease_level→skew×3→split×2、BTreeSet+真中順監査)
- J194 LZ4 ブロック codec ✅ `lz4`(4-byte ハッシュ貪欲パース、ニブル+255 拡張長、厳格復号 — offset≥1・末尾リテラル・重複マッチ逐語コピー)
- J195 Christofides TSP(1.5 近似)✅ `christofides`(MST→奇数次集合→部分集合 DP 最小重み完全マッチング(|T|≤20、超過は貪欲)→多重グラフ Euler→shortcut、全 tie-break 正準)
- J196 Keccak/SHA-3 ✅ `sha3`(Keccak-f[1600] 25 車線 ×24 ラウンド、SHA3-256/512=0x06・SHAKE128/256=0x1F ドメイン、pad10*1、`Digest256` は rate 位置を跨呼出で保持)
- J197 Minkowski 和差 ✅ `minkowski`(凸ポリゴンの辺ベクトル角度マージ O(n+m)、`diff` が配置空間障害物 — 全ペア和凸包 oracle)
- J198 Shamir 秘密分散 ✅ `shamir`(GF(p) 係数シード付き評価 + x=0 Lagrange 復元 — k−1 株は情報理論的に秘匿、偽株混入は値変化で検出可能)
- J199 中国式配点問題 ✅ `postman`(奇数次集合→bellman 計量閉包→free 引数化部分集合 DP マッチング — free=2 が開路端点を一括導出、Euler 増大化、全列挙最適照合)
- J200 最小無環 DFA 辞書 ✅ `fst`(ソート集合→トライ→下向上ハッシュコンス・レジスタ = Myhill–Nerode 一意最小、root を状態 0 へ swap-back)
- J201 LT 噴水符号 ✅ `fountain`(robust-soliton 次数 R=√k·ln(2k)/4 — c=1/10 では小 k で R=1 に退化し理想化するため c=1/4。近傍集合は (k,i,seed) の純関数、BP 剥離 decode)
- J202 Felzenszwalb 二乗距離変換 ✅ `edt`(放物線下包絡の2回1次元 pass、breakpoint は (num,den) 有理数 — −inf sentinel は i128::MAX/4 ではなく i64::MIN、zn·den が i128 溢れ)

- J203 B+木順序マップ ✅ `bplus`(copy-up 葉 split / move-up 内部 split — separator = 右部分木の最小キー、葉先頭キー削除は fix_sep 祖先 walk で伝播)
- J204 Bentley–Ottmann 交差列挙 ✅ `bentley`((x,y) BTreeMap イベント列 + (y at x, slope) 再整列 status; 垂直線分は status 不入・x-line 中は全関与 seg と対検査; 共線 overlap は共有端点のみ報告)
- J205 組合せ rank/unrank ✅ `comb`(choose128 の `acc = C(n−k+i, i)` 不変式で各除算が厳密 — gcd 正規化不要; 辞書順 combinadic)
- J206 AES-128-GCM AEAD ✅ `gcm`(GF(2^128) GHASH ビットシリアル、`R = 0xE1<<120`; aad→ct の累積器は連鎖必須 — 別計算の XOR 合成は誤り; J0 = IV‖1(12B) else GHASH 導出)
- J207 Sutherland–Hodgman ポリゴンクリップ ✅ `polyclip`(clipper は shoelace 符号で CCW 正規化、交点 t = cross(cd, a−s)/cross(cd, sd) — 符号反転は空クリップで検出)
- J208 Min-max ヒープ ✅ `mmheap`(min/max 交互レベルで両端 O(1) peek — pop_max は max が末尾スロット時 move を skip、pop 対象自身を書き戻さない)
- J209 マージソート木 ✅ `mstree`(静的範囲計数 O(log² n) — ノード配置は再帰 mid-split、2n ヒープ配置は冪次 n 限定)
- J210 極大クリーク列挙 ✅ `clique`(Bron–Kerbosch + Tomita ピボット P\N(u) — u64 隣接マスク、出力はソート正準で純関数)
- J211 動的時間伸縮 ✅ `dtw`(i64 厳密 Σ|差| DP + Sakoe–Chiba 帯 + 経路復元 — 境界セルは INF、枯渇 prefix は整列不能)
- J212 DAG 最小パス被覆 ✅ `pathcover`(二部 L_u—R_v マッチング帰着で被覆 = n − |matching| — 閉路入力は None、Kahn 最小優先で正準順)
- J213 Jump Point Search ✅ `jps`(自然+強制近傍のジャンプ点一様コスト探索 — 角接触許容は論文の剪定補題の一部、厳格 no-corner-cut は到達性を変えて最適性を壊す)
- J214 GOAP プランナ ✅ `goap`(ビットマスク世界の Dijkstra — `(cost, seq, state)` 順序付き BTreeSet で語彙最小正準計画を純関数化)
- J215 行動木 ✅ `btree`(Sequence/Selector が Running 子を per-node `mem` に記憶して resume — Condition の Running は Failure へ写像)
- J216 整数 Verlet 統合 ✅ `verlet`(Q16.16 Jakobsen 距離制約緩和 — 無減衰はエネルギー保存で平衡を貫通振動、収束には damping < 1 が必須)
- J217 整数バリューノイズ+fBm ✅ `vnoise`(SplitMix64 格子ハッシュ + smootherstep 双線形補間 — `raw >> 16` 算術シフトのセル床が負座標安全)
- J218 簡約順序付き二分決定図 ✅ `bdd`((var,lo,hi) ハッシュコンシング + apply/restrict/exists — 正準形はノード id 一致まで効く)
- J219 厳密有理 LP 単体法 ✅ `simplex`(Frac タブロー + Bland 最小添字規則 — 退化巡回が定理として不可)
- J220 packrat PEG パーサ ✅ `peg`((rule,pos) メモ化順序選択 — 左再帰ガードと Star/Plus 無進行停止で全性保持)
- J221 単調基数ヒープ ✅ `radixheap`(msb(key XOR last) バケツ — bit_len(key−last) は陳腐バケツに小キーを残して不変式破壊)
- J222 疎 Life セルオートマトン ✅ `life`(BTreeSet 生存集合がそのまま正準状態 — B3/S23、稠密グリッド oracle 全照合)
- J223 Fibonacci ヒープ ✅ `fibheap`(アリーナ+sibling 環、cut/cascade の decrease_key、pop で次数統合 — (key,seq) 正準順)
- J224 Halton 準乱数列 ✅ `halton`(radical inverse を厳密 Frac 化 — base^k 層化が性質検査として成り立つ)
- J225 二面体群 D4 変換 ✅ `dihedral`((swap,sx,sy) closed-form — 8×8 Cayley 表を点対応 oracle で全項検証)
- J226 ターンパイク再構成 ✅ `turnpike`(Skiena バックトラック — need の重複 multiplicity 検査が必須)
- J227 Yen の k-最短路 ✅ `kpaths`(spur 偏差 + (cost,path) 正準候補 — 契約は「最小 k 個のコスト」)
- J228 形式的冪級数 ✅ `fps`(GF(p) 上の inv/log/exp/pow — Newton 反復は*追跡次数*駆動が必須: norm 縮退で無限ループ)
- J229 矩形和集合面積 ✅ `rectunion`(x-sweep + slab 毎の y 区間 union — O(n²) で正直に)
- J230 区間グラフスケジューリング ✅ `intervalgraph`(最早終了貪欲 + 深さ sweep + 加重 DP — 退化区間は全 API で選択不能)
- J231 スターリング数・ベル数 ✅ `stirling`(signed s1/us1/s2/bell mod p — s2 は閉形式、s1 は順列 cycle 列挙で相互検証)
- J232 凸包玉ねぎ層 ✅ `onion`(hull 辺上の点は頂点でないため独自レイヤに残存 — hull 契約の正直な帰結)
- J233 最小平均重みサイクル ✅ `karp`(Karp の max-ratio DP — サイクル抽出は n 辺全 backtrack の初回重複、suffix のみでは不十分)
- J234 線形空間アライメント ✅ `hirschberg`(中点分割 L+R 復元 — 最小 j tie-break で正準スクリプト、apply 再生で整合)
- J235 行列連鎖積順序 ✅ `matchain`(O(n³) DP + postorder Step — 左端 argmin が tie を正準化)
- J236 継目削り ✅ `seamcarve`(二乗勾配エネルギー + 左端 argmin seam DP — 途中打切りパスは seam でない)
- J237 順序統計木 ✅ `ost`(subtree-size treap — priority=splitmix(seed⊕key) で形状が集合の純関数)
- J238 Karmarkar–Karp 分割 ✅ `kkpart`(残差ヒープに (plus,minus) mask を同梱 — 返す d が達成可能なため d ≥ optimal が構造的に保証)
- J239 x-fast trie ✅ `xfast`(65 層 prefix→(min,max) 表 — レベル二分探索で分岐点を特定、葉 hop は BTreeSet で代替)
- J240 beats+lazy add ✅ `seglazy`(延期バックログ消化 — push は add を*先に*伝搬してから clamp)
- J241 Tunstall 符号 ✅ `tunstall`(最大確率葉を 2ᵏ まで貪欲展開 — BigInt 交叉積で厳密比較、DFS 順で正準コード)
- J242 厳密 Gram–Schmidt ✅ `ortho`(Frac 上の非正規化 Q + 単位対角 R — A=Q·R が厳密成立、従属列は零ベクトル)
- J243 有理ベジェ ✅ `ratbezier`(同次 (w·x,w·y,w) リフトの de Casteljau — NURBS 式評価が全 Frac、導関数は差分曲線+商の微分)
- J244 Pólya/Burnside 数え上げ ✅ `polya`((1/|G|)·Σk^{cycles(g)} — BigInt 厳密除算、necklaces(4,3)=24/bracelets(6,2)=13)
- J245 最小値キュー ✅ `minq`(各スタックスロットが running min を保持 — pour 時に一括再構築で償却 O(1))
- J246 Smith–Waterman ✅ `ssw`(0-floor restart セル + (i,j) 最早勝者セルで正準化 — witness は restart まで traceback)
- J247 離散三分探索 ✅ `ternary`(契約は*厳密*単峰性 — 階段降下の等値 probe は argmin を局所化不能、末尾窓を全評価+全体最小検証)
- J248 GF(p) 楕円曲線 ✅ `ec`(chord-tangent 法則を i128 中間値で — `on_curve` 前提を明示、非体利用は正直に拒否)
- J249 切断 p進整数 ✅ `adic`(mod pᵏ の厳密環演算 + gcd-units の inv/div — 合成 p も正しく扱い、valuation/lift/trunc 完備)
- J250 グレイ符号 ✅ `gray`(BRGC rank/unrank + SubsetWalk — 1ビット step の部分集合走査で flip ビットを同報)
- J251 整数分割 ✅ `partitions`(Euler 五角数漸化式の p(n) を BigInt で — bounded/distinct DP が互いの oracle、distinct=odd の Euler 定理を双方検証)
- J252 彩色数え上げ ✅ `chrompoly`(削除-縮約 P(G)=P(G−e)−P(G/e) を BigInt で — kⁿ 全列挙 oracle と C₄ 閉形式で照合)
- J253 Pell 方程式 ✅ `pell`(√d の surd CF + BigInt 収束分数で x²−dy²=±1 を厳密に — Z[√d] 群法則の power、d=61 の巨大基底解も正確)
- J254 Farey 数列 ✅ `farey`(next-term 漸化式の F_n 生成 + Stern–Brocot 経路 + ACL `floor_sum` 格子計数を u128 で — |F_n|=1+Σφ(k) と隣接行列式 1 を全検証)
- J255 メビウス反転 ✅ `mobius`(線形篩 μ + Dirichlet 畳込み — `f ∗ μ` が除数和を厳密に反転、coprime_count の包除も照合)
- J256 Kronecker 記号 ✅ `jacobi`(全整数対の (a|n) を二分互換法で — Euler 判定基準 a^((p−1)/2) と乗法性の4000+3000乱択照合)
- J257 エジプト分数 ✅ `egypt`(Fibonacci–Sylvester 貪欲で真分数を相異なる単位分数和に — 分子降下不変式が終了性の証明)
- J258 Lucas 数列 ✅ `lucas`(Uₖ/Vₖ の i128 厳密 + doubling mod m — halving は残余の偶奇駆動が必須、V²−D·U²=4Qⁿ 不変式検証)
- J259 Frobenius 数 ✅ `frobenius`(min-residue Dijkstra の dist[r] — Sylvester 閉形式+DP oracle+McNugget 43)
- J260 ヨセフス問題 ✅ `josephus`(O(n) 漸化式+k=2 閉形式+ost で O(n log n) 全淘汰順 — Vec oracle 照合)
- J261 Bernoulli 数 ✅ `bernoulli`(Akiyama–Tanigawa を Frac 厳密で + Faulhaber 冪和 — 直接和 oracle・B_奇=0・生成漸化式の3検証)
- J262 Eulerian 数 ✅ `eulerian`(BigInt 漸化式+descent=k 置換列挙 — 行和 n!・Worpitzky 恒等式・列挙長=⟨n k⟩ の3定理)


## K. 物理・衝突 (Physics / Collision)
- K1 グリッド衝突（passability）✅ `passability` / K2 AABB 重なり ✅ `aabb` / K3 空間ハッシュ broadphase ✅ `spatial_hash`
- K4 線分述語（掃引衝突・壁判定・LOS 補助）✅ `segment`（`segments_intersect`/`point_on_segment`/`point_segment_dist2`/`segment_dist2` — i128 orientation 厳密判定。距離は `dist²` の ceiling 返却で `==0` ⟺ 幾何学的に接する、を整数のまま保証。端点-on-線分・collinear 退化を全分岐網羅 + 独立式オラクルと乱数検証）
- K5 最近点対（密度検証・近接 hotspot 解析）✅ `closestpair::closest_pair`（分割統治 O(n log n) — strip scan は y マージ済み、i128 距離²。同距は `(dist²,p,q)` 辞書順最小を返すので集合のみの純関数。O(n²) ブルートフォース argmin と同一 tie-break で 400 乱数集合一致検証）
- K6 静的点索引（spawn 点・POI の最近傍/範囲クエリ — 均一密度を仮定しない空間分割）✅ `kdtree`（median-split 2-D kd-tree — `nearest`/`within`/`in_rect` を期待 O(log n)、分割軸交互・全 tuple sort で tie-break まで決定的。入力順に依らず同一点集合 → 同一回答。i128 距離²・辞書順 tie-break をブルートフォース全走査と乱数一致検証）

## L. ゲームプレイ系 (Gameplay systems)
- L1 ターンスケジューラ（energy system）✅ `turn` / L2 ステータス/戦闘式 ✅ `combat` / L3 インベントリ/アイテム ✅ `inventory` / L4 状態異常（buff/debuff の期限管理）✅ `status`

## M. UI
- M1 メッセージログ ✅ `msglog` / M2 メニュー/ウィジェット ✅ `menu` / M3 テキストレイアウト/折返し ✅ `textlayout` / M4 HUD ✅ `hud`
- M5 矩形 bin packing（アトラス・インベントリ・ダイアログ敷詰）✅ `pack::pack_skyline`（Jylänki *A Thousand Ways to Pack the Bin* 2010 の skyline/bottom-left 法 — 入力順がそのまま配置順の純粋関数。重なり・境界逸脱を不変条件オラクルで検証）
- M6 時系列ダウンサンプリング（密な系列を小さな HUD グラフへ）✅ `lttb::lttb`（Steinarsson 2013 の Largest-Triangle-Three-Buckets — 各 bucket で prev選択点×次 bucket 重心との三角形面積最大の点を採用。i128 2倍面積で厳密、端点保存・順序保存部分列。nth-point 選択が形状を壊す spike 保存を検証）

## N. 永続化・セーブ (Persistence)
- N1 コンテンツ serialize ✅ / N2 ワールド save/load ✅ `savefile` / N3 バージョニング ✅ `savefile::SaveHeader::version` / N4 走長圧縮（スパースな盤面・連続値）✅ `rle`（`(count,byte)` pair — 255+ run は自動分割、bytes 版と `encode_u32`/`decode_u32` の素直 pair 版。zero-count・奇数長を reject する構造的 malformed 検査 = authenticity 側路。`decode(encode(x))==x` を乱数 run-heavy モデルで往復検証）
- N5 エントロピー圧縮（偏った頻度分布の wire/save 層 — `rle` と `bits` の間の帯域削減）✅ `huffman`（canonical Huffman — 2-queue マージで (weight, node-id) 決定的、コードは (長さ, symbol) 順 canonical 割当 = wire は `(symbol,length)` 表のみ。MSB-first パック + ビット総数ヘッダ。decode は非 prefix・長さ超過・ビット残しを全て `None` で拒否。往復同一・prefix-free・Kraft 等式・skewed 圧縮率を乱数検証）
- N6 辞書式圧縮（繰り返し部分列を持つ wire/save 層 — RLE の連続 run と Huffman の頻度偏りの中間領域）✅ `lzss`（greedy LZ77 系 — 4096 window、match 3–18、最小 offset 優先 tie-break で圧縮結果が入力の純関数。`bits` 上の `0`+8bit literal / `1`+12bit(offset-1)+4bit len トークン列 + u64 生長ヘッダ。decode は切り詰め・窓外 offset・長さ超過を全て `None` で拒否。往復同一・repetitive 圧縮率・手組 malformed 拒否を乱数検証）
- N7 内容定義チャンキング（編集位置に頑健な可変長分割 — delta sync・差分バックアップの前置層）✅ `rolling`（Rabin–Karp mod-2^64 多項式指紋 — `Rolling` 固定窓 + `find_all` 全出現 + `chunks` が `hash & mask == 0` の CDC 境界を [min,max] 内に強制。局所編集が遠方境界を動かさない prefix 安定性・chunk 幅制約・再計算一致を乱数検証。`delta` と組んで rsync 型の転送量削減を構成）
- N8 適応辞書圧縮（モデル表を持たない wire/save 層 — LZSS の窓探索が重い箇所の句表現）✅ `lzw`（LZW — greedy 最長一致、12-bit 固定コードで `bits` 上に展開、辞書は初期 256 literal + 3840 phrase で 4096 に達したら freeze。wire に辞書を載せず decoder が lockstep 再構築 = 写像はストリームの純関数。KwKwK（code==dict.len()）辺ケース・切り詰め・範囲外コードを `None` または厳密 prefix で処理。往復同一・repetitive 圧縮率を乱数検証）

## O. ネットワーク (Networking)
- O1 rollback/replay 基盤 ✅ `replay` / O2 input 同期 transport ⬜（ソケット I/O はヘッドレス方針で意図的に範囲外）/ O3 予測/補正 ✅ `netinput`（`NetInputBuffer`: 決定論的 input 予測・誤予測検出。transport 非依存＝呼び手が受信バイトを供給）
- O4 ビットレベル wire codec ✅ `bits`（`BitWriter`/`BitReader` — Gaffer 式のパックドビットフィールド・範囲整数・protobuf 型 canonical varint+zigzag。lockstep パケットをバイト境界無駄なく直列化、canonical 性を decode 側で強制）
- O5 因果順序・状態同期（マルチピア状態の合流・分岐・不一致検出）✅ `vclock`（Lamport/Fidge–Mattern ベクタクロック — `tick`/`merge`/`compare` が4値因果順序 Before/After/Equal/Concurrent を返す。BTreeMap 保持で列挙も決定的。公理系・最小上限性・メッセージ流シミュレーションを乱数検証）+ `merkle`（DetHash 葉上の二分ハッシュ木 — root 比較で発散有無、`first_diff` で O(log n) 下降して不一致葉 index を特定。`replay::first_divergence` のストア版。proof 往復・naive 葉走査一致・order 依存性を乱数検証）+ `delta`（順序 `u64→u64` マップのスナップショット差分 — `diff_sorted`/`apply_sorted` の全単射往復 + 昇順キー delta varint の canonical wire。`Del` 不存在キーは `None` で失敗閉鎖。乱数マップ対で round-trip・最小 op 数・malformed 拒否を検証）

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
