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
