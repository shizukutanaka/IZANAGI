# ゲーム開発 能力タクソノミー (Game-Dev Capability Taxonomy)

> 「ゲームを作成するのに必要なもの」を**徹底的にカテゴライズ**し、各々を**細分化**して、
> izanagi_kit の現状被覆（✅ 実装済 / 🔶 一部 / ⬜ 未実装＝強化対象）を対応付ける。
> これがエンジン高機能化のロードマップ兼ギャップ表。詳細契約は [`SPEC.md`](./SPEC.md)、
> 調査根拠は [`RESEARCH.md`](./RESEARCH.md)。

スコープ前提: zero-dep / `#![forbid(unsafe_code)]` / シミュレーションは整数・決定論（→ [`SPEC.md`](./SPEC.md) G1–G8）。
音声・GPU・OS 依存 I/O は本キットのヘッドレス方針では**意図的に範囲外**（呼び手側で実装）。

## A. 時間とループ (Time & Loop)
- A1 固定タイムステップ ✅ `timestep` / A2 補間 alpha ✅ / A3 death-spiral ガード ✅
- A4 スケジューラ（タイマー・クールダウン・遅延イベント）✅ `timer` / A5 ターン制エネルギー系（speed-based turn order）✅ `turn`

## B. 数学 (Math)
- B1 fixed-point Q16.16 ✅ `fixed` / B2 sqrt・CORDIC trig ✅ / B3 整数幾何（line/LOS）✅ `geometry`
- B4 fixed ベクトル（vec2/vec3, dot/len/normalize）✅ `vec` / B5 easing・tween（整数）✅ `easing` / B6 補間（lerp/clamp/sign）✅ `Fixed::lerp/clamp/sign/abs`

## C. 状態とデータ (State & Data / ECS)
- C1 generational entity ✅ `entity` / C2 sparse-set storage ✅ / C3 多コンポーネント join ✅
- C4 archetype storage ⬜ / C5 変更検知（dirty/changed）✅ `change` / C6 エンティティ関係（parent/child, relations）✅ `relations`

## D. 乱数 (Randomness)
- D1 決定論 PRNG ✅ `rng` / D2 range・coin ✅ / D3 stream の DetHash ✅
- D4 重み付き抽選（weighted choice / loot table）✅ `weighted_index` / D5 ダイス（NdM）✅ `dice` / D6 value/Perlin noise（整数）✅ `noise`

## E. 決定論・リプレイ (Determinism & Replay)
- E1 state hashing FNV-1a ✅ `world_hash` / E2 DetHash（値型＋容器）✅ / E3 replay trace・desync 検出 ✅ `replay`
- E4 snapshot/rollback 基盤 ✅ `resimulate` / E5 `DetHash` derive macro ⬜（zero-dep 方針では手実装維持も可）

## F. 表示・描画 (Presentation / Rendering)
- F1 セル画面バッファ（glyph + fg/bg）✅ `terminal` / F2 ANSI 24-bit 出力 ✅ `to_ansi` / F3 ダブルバッファ差分 ✅ `diff`/`present`
- F4 ヘッドレス検査（snapshot test 用 cell アクセス）✅ `get`/`DetHash` / F5 カメラ/ビューポート（world→screen）✅ `camera` / F6 描画プリミティブ（fill/box/text）✅ `fill_rect`/`draw_str`
- **F1–F6 すべて実装済み**（`terminal` + `camera`）。

## G. 入力 (Input)
- G1 キー→アクションのマッピング ✅ `keymap` / G2 コマンドキュー（決定論 input feed、replay と直結）✅ `cmdqueue` / G3 入力バッファ/長押し ✅ `inputbuf`

## H. コンテンツ・アセット (Content & Assets)
- H1 DSL パーサ ✅ `parser` / H2 シリアライズ往復 ✅ `serializer` / H3 意味検証 ✅ `validator` / H4 ECS ロード ✅ `loader` / H5 CLI ゲート ✅ `gamec`
- H6 ホットリロード ⬜ / H7 アセット ID/ハンドル管理 ✅ `assets`

## I. ワールド・マップ (World & Map)
- I1 手続き生成（rooms+corridors）✅ `mapgen` / I2 連結保証 ✅
- I3 タイルマップ層（複数レイヤ）✅ `tilemap` / I4 オートタイル ✅ `autotile` / I5 WFC 生成 ⬜ / I6 マルチレベル/階層 ✅ `multimap`

## J. 視界・AI・ナビ (Visibility / AI / Navigation)
- J1 対称 FOV ✅ `fov` / J2 A* 経路 ✅ `pathfinding` / J3 Dijkstra map（flow field）✅ / J4 descend（chase/flee）✅ / J5 LOS ✅ `geometry`
- J6 ステアリング/influence map ✅ `influence` / J7 FSM / behavior tree ✅ `fsm` / J8 JPS / weighted A* ⬜

## K. 物理・衝突 (Physics / Collision)
- K1 グリッド衝突（passability）🔶（pathfinding/mapgen の is_blocked）/ K2 AABB 重なり ✅ `aabb` / K3 空間ハッシュ broadphase ✅ `spatial_hash`

## L. ゲームプレイ系 (Gameplay systems)
- L1 ターンスケジューラ（energy system）✅ `turn` / L2 ステータス/戦闘式 ✅ `combat` / L3 インベントリ/アイテム ✅ `inventory` / L4 状態異常（buff/debuff の期限管理）✅ `status`

## M. UI
- M1 メッセージログ ✅ `msglog` / M2 メニュー/ウィジェット ✅ `menu` / M3 テキストレイアウト/折返し ✅ `textlayout` / M4 HUD ✅ `hud`

## N. 永続化・セーブ (Persistence)
- N1 コンテンツ serialize ✅ / N2 ワールド save/load ⬜（replay の seed+inputs で代替可）/ N3 バージョニング ⬜

## O. ネットワーク (Networking)
- O1 rollback/replay 基盤 ✅ `replay` / O2 input 同期 transport ⬜ / O3 予測/補正 ⬜

## P. ツール・デバッグ (Tooling / Debug)
- P1 コンテンツ検証 CLI ✅ `gamec` / P2 desync 二分探索 ✅ `replay::first_divergence` / P3 ロギング/プロファイル ✅ `profiler` / P4 機械可読診断(JSON/SARIF) ⬜

---

## 実装優先度（高機能化ロードマップ）

| 順 | カテゴリ | 理由 | 状態 |
|----|---------|------|------|
| 1 | **F 表示・描画**（cell buffer + ANSI + diff） | ゲームを**表示できない**最大の欠落。terminal-first 宣言と乖離 | ✅ 実装済み（`terminal`、F5 カメラのみ残） |
| 2 | L1 ターンスケジューラ（energy system） | roguelike のコア進行 | ✅ 実装済み（`turn`） |
| 3 | D4 weighted choice / loot table | コンテンツ/戦闘に必須 | ✅ 実装済み（`weighted_index`, `dice`） |
| 4 | B4 fixed ベクトル | 移動・物理の土台 | ✅ 実装済み（`vec` — Vec2/Vec3） |
| 5 | M1 メッセージログ / G2 コマンドキュー | UI・入力決定論 | ✅ 実装済み（`msglog`, `cmdqueue`） |

**本イテレーションの着手**: F（表示層）= `terminal` モジュール（セルバッファ・ANSI 24-bit・差分・ヘッドレス検査・DetHash）。
決定論（純粋セルバッファ＋差分、float なし）・zero-dep を維持し、`Color`/`Render`/`mapgen`/`fov` と接続する。
