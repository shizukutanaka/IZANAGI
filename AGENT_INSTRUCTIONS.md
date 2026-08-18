# IZANAGI — 長所・短所・改善案と作業指示書(Opus / Sonnet 用)

> **この文書の目的**: 本リポジトリの現状評価(長所・短所)と、優先順位付きの改善案を、
> **Claude Opus / Claude Sonnet が単独セッションでそのまま実行できる粒度**で記述する。
> 曖昧さを排し、各タスクに「対象ファイル・検証手順・リスク・推奨モデル」を明記する。
>
> 最終更新: 2026-07-21 / 基準ブランチ: `claude/deepresearch-ultrathink-improve-yq2th`(origin と同期済み)
> 併読: `izanagi_kit/RESEARCH.md`(外部出典調査。N1〜N23 候補表は実装状況を随時反映済み)
>
> **削除済みの先行文書**: `STRENGTHS_WEAKNESSES.md` / `FEATURE_AUDIT.md` / `IMPROVEMENTS.md` /
> `PRODUCT_AUDIT.md` の4件は、見出しの数値が実態から乖離していたため削除した(FEATURE_AUDIT は
> 「77 モジュール / 3362 テスト」、PRODUCT_AUDIT は「78 モジュール / 188 テスト」と主張していたが
> 実態は 89 / 3699)。古い数値は無い数値より悪く、どれが最新か読者に判別できなくなる。
> 内容は git 履歴から復元可能。**現行の真実の source は本書と `RESEARCH.md` の2つだけ**であり、
> 本書の検証可能な主張(モジュール数・pinned hash)は `izanagi_kit/tests/docs_are_current.rs` が
> ビルド時に検査するので、黙って古くなることはない。

---

## 0. 現状スナップショット(2026-07-21 実測)

| 指標 | 値 |
|---|---|
| workspace テスト | **3539 passed / 0 failed** |
| clippy 警告(`--workspace --all-targets`) | 0 |
| rustfmt | clean |
| kit モジュール数 | **88**(`izanagi_kit/src/*.rs`。`tests/docs_are_current.rs` が検証)|
| engine モジュール数 | **25**(`izanagi/src/*.rs`。同上)|
| 決定論 pinned hash | `PINNED_FINAL_HASH=0xd1a9236e96a2c802` / `PINNED_ROGUELIKE_HASH=0x5286d1420200fe66`(不変) |
| kit_bridge 統合ハッシュ | `353498ec4fbcd160`(headless == engine-hosted) |
| バージョン | engine 4.1.0 / kit 0.1.0 |
| MSRV | engine 1.65 / kit 1.75 |
| main との差 | feature ブランチが 352 コミット先行(PR 未作成) |
| kit src 内 panic 系(**実装のみ**) | **0**(`clippy::unwrap_used/expect_used/panic` を `deny` で強制。テスト込みの旧計測 242/20 はテストコードを数えていた) |

---

## 1. 長所(証拠付き)

1. **決定論パイプラインの完結性** — 業界でも稀な end-to-end 構成が完成している:
   `DetHash`/FNV-1a world hash → `LabeledDigest`(subsystem 粒度 desync 局所化)→
   `DesyncReport`(本番再現バンドル)→ `dst`(seed 掃引 + 二重実行検査)→
   `rollback::sync_test`(毎フレーム rollback 自己検査)→ `plan`(goal→入力列合成)。
   検証系がそれ自体テストされている(例: sync_test の隠れ状態注入テスト)。
2. **テスト密度と質** — 3492 テスト。単なる例示ではなく BFS オラクル(jps4/plan)、
   `is_reachable` オラクル 2400 ペア(ConnectivityMap)、bit-exact 等価テスト(WFC 重み)、
   メタモルフィック則(三角不等式・壁単調性)など**機械検証可能なオラクル**中心。
3. **zero-dependency / `#![forbid(unsafe_code)]`** — 両クレートとも実行時依存ゼロ。
   供給網リスクなし。wasm32 にツールチェーン追加だけでコンパイル可能(CI job 定義済み)。
4. **文書品質が deny レベルで固定** — `#![deny(missing_docs)]` +
   `#![deny(rustdoc::broken_intra_doc_links)]`。全公開 API に出典付き doc(GDC 講演・arXiv 論文を明記)。
5. **アルゴリズムの正しさへの投資** — JPS4 は交換論法の証明スケッチ付き、
   実装初稿のバグを BFS オラクルが実際に検出した記録が commit message に残る。
6. **コンテンツパイプライン** — text → parser → validator(SARIF 出力)→ loader。
   LLM 生成コンテンツの検証ゲートとして機能する(未使用 prefab/tile 検出まで実装)。
7. **エンジンとキットの実証済み統合** — `kit_bridge` example が headless と engine-hosted の
   world-hash 一致をアサートし、単一ハッシュ値で回帰を検出。
8. **研究駆動の開発記録** — `RESEARCH.md` に出典・実装 commit・見送り理由が全て残り、
   後続セッションが文脈を完全に復元できる。

## 2. 短所(重要度順・証拠付き)

1. **[重大] CI が未稼働** — `.github/workflows/ci.yml` は GitHub App トークンの
   `workflows` 権限不足で push 不能(履歴から除外済み)。8 job の定義はユーザーへ送付済みだが、
   **ユーザーが GitHub Web UI で追加するまで、3492 テストも決定論 matrix も GitHub 上では一切走らない**。
2. **[重大] main が 342 コミット遅れ** — 成果は feature ブランチにのみ存在。PR 未作成
   (ユーザー明示指示待ち)。main を見た訪問者には改善が一切見えない。
3. ~~**[中] 公開 API に panic 経路が残る**~~ — **解消済み**: 旧記載の「unwrap 242 / expect 20」は
   **テストコードを含む計測**だった。実装コードのみを数え直すと kit は unwrap 6 / expect 4 /
   `panic!` 0、engine は unwrap 1 / expect 1 で、**合計 12 箇所**(すべて到達不能かガード済み)。
   0.2 の破壊的変更は不要で、12 箇所を書き換えて両クレートに
   `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]`
   を追加し、コンパイラが強制する不変条件にした。添字アクセス(約700箇所)とコンストラクタの
   `assert!` は意図的に対象外 — 前者を `get().ok_or()` に置換するとコードが悪化し、
   後者は「不正な設定を作った瞬間に報告する」ための正しい振る舞い。
4. ~~**[中] README が新機能に追随していない**~~ — **解消済み (04d472b)**: kit README と
   crate doc に 4 層のモジュール地図を追加し、`sim` / `rollback` / `dst` / `plan` /
   `AdaptiveDelay` を掲載。残る細部として `combine_maps` / `farthest_cell` /
   `ConnectivityMap` / `jps4` は tier 表の `pathfinding` 行に含まれるが個別記載はまだない。
5. **[中] バージョン体系の不整合** — engine 4.1.0 / kit 0.1.0。crates.io 未公開なのに
   engine が 4.x を名乗る根拠がリポジトリ内に記録されていない(P5 — ユーザーへの確認事項)。
6. ~~**[小] エンジン側の f32 シミュレーション**~~ — **解消済み**: 境界を実測して
   engine の crate doc に「Determinism boundary」節を追加し、`izanagi/tests/float_boundary.rs`
   で機械検査に載せた。実測では 25 モジュール中 **8 個が完全に float-free**
   (`assets`/`ecs`/`error`/`event`/`log`/`save`/`scene`/`state`)で、これらから組んだ状態は
   replay に参加できる。`rng` は**分割している**のが要点 — 整数側(`u64`/`u32`/`int_range`/
   `choose`)は replay-safe だが便利側(`f32`/`range`/`chance`)は違い、これが事故で desync する
   最短経路。テストは両方向(float-free だったものが汚れた / 汚れていたものが綺麗になった)を
   検出し、doc の記載と定数の一致も検査する。
7. **[小] 新モジュールの example 不在** — `dst`/`plan`/`rollback` は doc とテストのみで、
   `examples/` に使用例がない(既存 example 群は充実)。
8. ~~**[小] 文書が古い**~~ — **解消済み**: 乖離した4文書を削除し、残る文書の検証可能な主張を
   `tests/docs_are_current.rs` の機械検査に載せた(tier 表・README のモジュール表が存在しない
   モジュールを挙げていないこと、pinned hash の一致、モジュール数の一致)。

## 3. 改善案(優先順位付き)

| # | 改善案 | 効果 | 工数 | 推奨モデル | 依存 |
|---|---|---|---|---|---|
| I1 | kit README / engine README を現状に同期(新モジュール8件・オラクル検証の訴求) | 高(公開物の顔) | 小 | **Sonnet** | なし |
| I2 | `dst`/`plan`/`rollback` の実行可能 example 追加(`examples/dst_demo.rs` 等、既存 example の書式踏襲) | 中 | 小 | **Sonnet** | なし |
| I3 | N13: WFC selector フック(collapse 順を外部制御、既定挙動 bit 不変) | 中 | 中 | **Opus** | なし |
| I4 | N5 残り: MapBuilder 合成層(`Dungeon` に可変 API を足さず、`Vec<bool>` グリッド変換の合成で設計) | 中 | 中〜大 | **Opus** | なし |
| I5 | N17: engine の float ソート箇所棚卸し → `total_cmp`+index tie-break 化 | 中(決定論) | 小〜中 | **Sonnet**(棚卸し)→ **Opus**(判断) | なし |
| I6 | N15 残り: recipe/drop/encounter の孤児参照検出(validator 拡張) | 中 | 小 | **Sonnet** | なし |
| I7 | N11 残り: t+delay lockstep ヘルパ(1500 Archers 型、`AdaptiveDelay` の推奨値を消費) | 中 | 中 | **Opus** | なし |
| I8 | N19 残り: `Fixed::mul/div` の i128 中間化検討 — **注意: 既存の丸め挙動を 1 bit も変えてはならない**(pinned hash が壊れる)。現行 i64 で十分か検証が先 | 低 | 小 | **Opus** | なし |
| I9 | N20: cargo feature collections(`default=full` 必須、pinned hash テストを全 feature 組合せで確認) | 低 | 中 | **Opus** | なし |
| I10 | N3: zero-panic API(0.2 破壊的変更)— 着手前に `clippy::unwrap_used` を **warn** で入れ実態を層別(pub API 到達可能なものだけが対象) | 高(長期) | 大 | **Opus**(設計)+ **Sonnet**(機械的変換) | ユーザーの 0.2 合意 |
| I11 | N2: incremental multiset hash — `savefile`/`replay` ヘッダの algo バージョニング設計が先。**単独で着手しないこと** | 低 | 大 | **Opus** | 設計合意 |
| I12 | N21: crates.io Trusted Publishing + cargo-semver-checks — 初回公開の意思決定待ち | 中 | 小 | ユーザー判断 | P5 解決 |

**ユーザー判断待ち(エージェントは着手禁止)**: CI 有効化(Web UI で ci.yml 追加)/ main への PR 作成 /
P5 バージョン体系 / 0.2 破壊的変更の承認 / crates.io 公開。

---

## 4. 指示書 — 共通プロトコル(Opus・Sonnet 両方が厳守)

### 4.1 絶対に壊してはならないもの(ハード制約)

1. **pinned hash**: `izanagi_kit/tests/determinism.rs` の `PINNED_FINAL_HASH` と
   `tests/roguelike_sim.rs` の `PINNED_ROGUELIKE_HASH`。これが変わる変更は
   「既定パラメータでの RNG 消費・演算順序・丸めが変わった」ことを意味する。
   新機能は **opt-in パラメータの既定値で従来経路と bit 一致**させること
   (実例: `wfc` の重み、`mapgen` の `extra_loops=0`)。
2. **zero runtime dependency**: 両 Cargo.toml の `[dependencies]` は空のまま。
   dev-dependencies も原則追加しない(kit_bridge の path 依存のみ例外)。
3. **`#![forbid(unsafe_code)]`** / **`#![deny(missing_docs)]`** /
   **`#![deny(rustdoc::broken_intra_doc_links)]`**: 全公開 API に doc comment 必須。
4. **MSRV**: engine 1.65 / kit 1.75。`is_some_and`(1.70)や `is_none_or`(1.82)、
   `u64::isqrt`(1.84)等の新しめ API は使用禁止。clippy --fix が導入してくることがあるので注意。
5. **`.github/workflows/` 配下を変更するコミットを作らない**。push が全拒否される
   (GitHub App トークンに workflows 権限がない)。CI 定義の変更提案はファイルとして
   ユーザーに渡すこと。
6. push 先は現行 feature ブランチのみ。**main へ push しない。PR は明示指示があるまで作らない。**

### 4.2 検証パイプライン(1 機能ごと・コミット前に全て実行)

```bash
cargo fmt --all
cargo test -p izanagi_kit --lib <対象module>      # 対象テスト
cargo test --workspace                             # 全 3492+ green
cargo clippy --workspace --all-targets             # 警告 0
cargo fmt --all -- --check
cd izanagi_kit && cargo test --test determinism --test roguelike_sim   # pinned hash 不変
cargo run -p izanagi --example kit_bridge          # hash 353498ec4fbcd160 不変
```

コミットは **1 機能 = 1 コミット**、メッセージ末尾に検証結果を記載
(先例: `git log --oneline -20` の各コミットを参照)。コミット後は毎回
`git push -u origin claude/deepresearch-ultrathink-improve-yq2th`。
push が classifier エラーで拒否された場合は 1〜2 回リトライすれば通る(一過性)。

### 4.3 環境の既知の罠

- **stop-hook「Unverified commits」警告は無視**: 原因は環境の署名鍵不在(修復不能)。
  提案される `--reset-author`/rebase を実行しないこと(公開済み履歴の書き換えになる)。
- **worktree エージェントの分岐元**: `isolation: worktree` は現在のブランチではなく
  古い ref から分岐することがある。並列実装を任せる場合は分岐元 SHA の確認を
  プロンプトに含め、成果は diff として受け取り本ブランチへ手動移植する。
- ネットワーク制約: rustup の toolchain/target 追加、crates.io fetch は不可。
  cargo-fuzz(nightly)や cargo-audit のローカル実行は不能 — CI 定義に留めること。
- 完了済みの実装を `RESEARCH.md` の N 候補表に反映すること(commit hash 付き)。

### 4.4 テスト設計の要求水準(このリポジトリの流儀)

- 可能な限り**機械オラクル**を使う: 自明に正しい別実装(BFS)、既存の信頼済み実装(astar)、
  代数則(三角不等式・単調性・可換性)、bit 一致(既定パラメータ)。
- 新機能 1 件につき最低: 正常系 / 境界(空・ゼロ・満杯)/ 決定論(同入力 2 回)/
  統合(既存 API との橋渡し)の 4 観点。
- 「テストがバグを見つけた」場合はその経緯を commit message に書く(検証手法の価値の記録)。

---

## 5. 指示書 — モデル別の作業割当て

### 5.1 Sonnet に任せるべきタスク(強いオラクルがあり、設計判断が少ない)

- **I1 README 同期**: `izanagi_kit/README.md` のモジュール表に
  `dst`・`plan`・`rollback` の 3 行を追加し、`pathfinding` 行に jps4/combine_maps/
  farthest_cell/ConnectivityMap を追記。lib.rs のモジュール一覧 doc(既に最新)を正とする。
  engine README には TerminalBackend の cell-diff 描画を 1 行追記。
- **I2 examples**: `izanagi_kit/examples/` の既存ファイル(例: `replay_demo.rs`)の
  構成(ヘッダ doc・main・最後に成否 print・非ゼロ exit)を踏襲。
  `dst_demo`(seed 掃引→意図的バグ注入→1行再現)、`plan_demo`(迷路→入力列合成→再生検証)。
  Cargo.toml への `[[example]]` 登録を忘れない。
- **I5 の棚卸しフェーズ**: `grep -rn "sort\|partial_cmp\|f32" izanagi/src/` で
  float 比較・ソート箇所を列挙し、各箇所に「決定論影響あり/なし」の所見を付けた
  一覧を作る(変更はまだしない)。
- **I6 validator 拡張**: 既存の unused-prefab / unused-tile と同じパターン
  (`HashSet` 構築 → 突合 → `Diagnostic::warning`)を recipe/drop/encounter 参照に適用。
  ※ content.rs に該当フィールドが存在するかを最初に確認し、なければ「対象なし」と報告して終了。

### 5.2 Opus に任せるべきタスク(設計判断・正しさ論証が必要)

- **I3 WFC selector**: `wfc.rs` の collapse 地点選択を trait/closure フックで注入可能にする。
  既定フックは現行ロジックと **bit 一致必須**(uniform 等価テストの流儀で検証)。
- **I4 MapBuilder**: `Dungeon` の内部 `tiles: Vec<bool>` を直接可変化せず、
  `fn(Dungeon, &mut SplitMix64) -> Dungeon` の合成としてビルダーを設計する案を推奨。
  既存 4 ジェネレータ + `extra_loops` + `farthest_cell`(階段)+
  `ConnectivityMap`(非連結ポケットの除去)を段として繋ぐ。全段で RNG 消費順を固定。
- **I7 t+delay lockstep**: `AdaptiveDelay::recommended_delay()` の値を消費し、
  「tick t の入力を t+delay で実行する」スケジューラを `netinput` に追加。
  world hash への含め方(delay 自体は含めない、実行された入力列は含まれる)を doc で明確化。
- **I8/I9/I10/I11**: 各項の注意書き(§3)を先に読み、着手判断そのものを成果物にする
  (「やらない理由」の文書化も成果)。

### 5.3 判断に迷ったときの規則

1. pinned hash・既定挙動・公開 API シグネチャに影響し得る → **変更せず、選択肢と推奨を報告**。
2. ユーザーの明示指示が要る事項(§3 の判断待ちリスト)に踏み込まない。
3. 実装が 2 通りあり優劣が非自明 → 小さい方・可逆な方・opt-in の方を選ぶ。
4. このファイルと `RESEARCH.md` の候補表を更新してから終了する。
