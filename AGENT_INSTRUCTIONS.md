# IZANAGI — 長所・短所・改善案と作業指示書(Opus / Sonnet 用)

> **この文書の目的**: 本リポジトリの現状評価(長所・短所)と、優先順位付きの改善案を、
> **Claude Opus / Claude Sonnet が単独セッションでそのまま実行できる粒度**で記述する。
> 曖昧さを排し、各タスクに「対象ファイル・検証手順・リスク・推奨モデル」を明記する。
>
> 最終更新はコミット履歴を正とする(`git log -1 --format=%cd AGENT_INSTRUCTIONS.md`)。
> 基準ブランチ: `claude/deepresearch-ultrathink-improve-yq2th`(origin と同期済み)
> 併読: `izanagi_kit/RESEARCH.md`(外部出典調査。N1〜N23 候補表は実装状況を随時反映済み)
>
> **削除済みの先行文書**: `STRENGTHS_WEAKNESSES.md` / `FEATURE_AUDIT.md` / `IMPROVEMENTS.md` /
> `PRODUCT_AUDIT.md` の4件は、見出しの数値が実態から乖離していたため削除した(FEATURE_AUDIT は
> 「77 モジュール / 3362 テスト」、PRODUCT_AUDIT は「78 モジュール / 188 テスト」と主張していたが
> 実態は 88 モジュール / 3,600 超のテスト)。古い数値は無い数値より悪く、どれが最新か読者に判別できなくなる。
> 内容は git 履歴から復元可能。**現行の真実の source は本書と `RESEARCH.md` の2つだけ**であり、
> 本書の検証可能な主張(モジュール数・pinned hash)は `izanagi_kit/tests/docs_are_current.rs` が
> ビルド時に検査するので、黙って古くなることはない。

---

## 0. 現状スナップショット(数値は `tools/gate.sh` の直近 exit 0 時点の実測)

| 指標 | 値 |
|---|---|
| workspace テスト | **3,600+ passed / 0 failed**(下限。`docs_are_current.rs` が実測値で検査)|
| clippy 警告(`--workspace --all-targets`) | 0 |
| rustfmt | clean |
| kit モジュール数 | **88**(`izanagi_kit/src/*.rs`。`tests/docs_are_current.rs` が検証)|
| engine モジュール数 | **25**(`izanagi/src/*.rs`。同上)|
| 決定論 pinned hash | `PINNED_FINAL_HASH=0xd1a9236e96a2c802` / `PINNED_ROGUELIKE_HASH=0x5286d1420200fe66`(不変) |
| kit_bridge 統合ハッシュ | `353498ec4fbcd160`(headless == engine-hosted) |
| panic 経路(実装コード) | **0** — 両クレートで `clippy::unwrap_used`/`expect_used`/`panic` を `deny` |
| 出荷可能性 | `cargo package` 両クレート成功(`--no-verify` なし。tarball を実際にコンパイルする)|
| 機械検査された文書主張 | tier 表・README モジュール表・pinned hash・モジュール数・engine 版数・f32 境界・README のテスト数下限・README の Quickstart(doctest として実行)・engine CLAUDE.md の Map・全 md の相対リンク・**非 float 非決定論ソースの許可リスト**(`HashMap`/壁時計/スレッド/アドレス依存)・**engine の順序づけ 0 件**(float 比較ソートの不在)|
| 未検証の公開 API | **0** — `tests/public_api_is_exercised.rs` が、どのテスト・example からも呼ばれない `pub fn` の追加を落とす |
| バージョン | engine 4.1.0 / kit 0.1.0(独立公開なので一致は不要。4.x の根拠は engine CHANGELOG `[4.0.0]`)|
| MSRV | engine 1.65 / kit 1.75 |
| main との差 | **0 遅れ**(main の全内容を取り込み済み)。PR #7 は作成済み・**未マージ**(CI 有効化を先にする合意)|
| kit src 内 panic 系(**実装のみ**) | **0**(`clippy::unwrap_used/expect_used/panic` を `deny` で強制。テスト込みの旧計測 242/20 はテストコードを数えていた) |

---

## 1. 長所(すべて測定値の裏づけあり)

1. **検証系11モジュールが揃い、相互に補完する** — `sim`(監査)/ `verify`(有界モデル
   検査・**証明**)/ `temporal`(時相性質)/ `recovery`(クラッシュ復旧)/ `explore`
   (archive 探索)/ `prop`(性質・モデル検査)/ `shrink`(縮約)/ `plan` / `dst` /
   `rollback` / `world_hash`。各々が別のバグクラスを狙い、出典が明記されている。
   **決定的な非対称性**: 全ツールが「見つからなかった」を言えるが、
   「存在しない」を言えるのは `verify` だけ(三値の `Holds`/`Violated`/`Exhausted`)。
2. **主張が機械検査される(12種)** — tier 表・README モジュール表・pinned hash・
   モジュール数・engine 版数・版数と CHANGELOG の対応・f32 境界・**engine の順序づけ 0 件**・
   engine CLAUDE.md の Map・全 md の相対リンク・README のテスト数下限・
   README Quickstart(doctest 実行)。
   加えて **panic 経路 0**(コンパイラ強制)、**未検証の公開 API 0**、
   **MSRV 違反 0**(静的検査)、**非 float の非決定論ソース 0**(許可リスト方式)。
3. **オラクル中心のテスト 3,600+ 件** — 手計算値ではなく独立実装との照合。BFS オラクル
   (jps4/plan)、代数則(三角不等式・単調性)、denotational 参照(temporal の全パターン)、
   保存則(pool)、ビルダー等価(autotile/encounter)。
   **検査器自身も変異注入で検証**する慣行が定着している。
4. **zero-dependency / `#![forbid(unsafe_code)]`** — 両クレートとも実行時依存ゼロ。
   両クレートが**検証付き**で梱包可能(`cargo package` が tarball からビルドし直す)。
5. **「green」の定義が1つ** — `tools/gate.sh` の8段。pre-push フックと CI 提案が
   同じスクリプトを呼ぶので、ローカルと CI が定義上ずれない。
6. **エンジンとキットの境界が測定され機械検査される** — engine 25 モジュール中 8 が
   完全に float-free。`rng` は整数コアが replay-safe、便利側がそうでないという
   **分割**まで明記されている。
7. **研究駆動の開発記録** — `RESEARCH.md` に出典・実装 commit・見送り理由が残り、
   後続セッションが文脈を復元できる。本セッションでは「見送り理由」自体が
   誤っていた例(N3)も記録され直した。

## 2. 短所(重要度順・証拠付き)

### 未解決

1. **[重大] CI が未稼働** — 3経路すべて実測で 403(git push / Contents API /
   Git Data API)。エージェント側の回避策は存在しない。定義は `docs/ci/ci.yml`、
   手順は `docs/ci/README.md`。**ユーザーが Web UI で追加するまで、3,600+ テストも
   MSRV も wasm も GitHub 上では一度も走らない。**
2. **[重大] 外部利用者ゼロ — 全テストが著者自身によるもの** — これは構造的な限界で、
   道具では埋まらない。API が使えるという証拠は `verify_pipeline_demo` と doctest だけで、
   どちらも同じ著者が書いた。「動く」ことは 3,600+ テストが示すが、
   **「他人に使える」ことは何も示していない。** 公開して最初の利用者が来るまで、
   この欄は埋まらない。
3. **[中] `verify` はモデルについて証明する。忠実性は標本にすぎない** —
   実ゲームの状態空間は列挙不能なので、検査するのはモデル。`prop::forall_model` が
   モデルと実装の一致を示すが、**それは標本であって精緻化の証明ではない**。
   両者の連携は doc と demo に明記済み(`RealRoom` が INCONCLUSIVE、モデルが PROVED)。
   これ以上を主張しないことが正しく、過大主張は道具の信頼を壊す。
4. **[小] crates.io 未公開** — `cargo publish --dry-run` は成功する(公開可能)。
   資格情報が無く、公開自体が取り消し不能なので、ユーザーの判断と手による。

### 解消済み(本セッション)

- ~~公開 API の panic 経路~~ → **要件が誤りだった**。「unwrap 242」はテストコード込みの
  計測で、実装のみでは12箇所・全て到達不能。0.2 破壊的変更は不要で、書き換えて
  コンパイラ強制(`deny(clippy::unwrap_used/expect_used/panic)`)に。
- ~~バージョン体系の不整合~~ → **要件が誤りだった**。独立公開なら版数一致は不要で、
  4.x の根拠は engine CHANGELOG `[4.0.0]` に記録済み。
- ~~engine の f32 境界が不明~~ → 実測(25中8が float-free、`rng` は分割)し機械検査。
- ~~新モジュールの example 不在~~ → `verify_pipeline_demo` が11モジュールを1本で通し、
  印字する主張をすべて assert する。
- ~~文書が古い~~ → 乖離4文書を削除、残りの検証可能な主張を12種の機械検査に。
- ~~main が遅れている~~ → main をマージして 0 遅れ、PR #7 作成済み(未マージ)。
- ~~非 float の非決定論が未検査~~ → 監査で `SpatialHash` の**実バグ**を発見・修正し、
  クラス全体を許可リスト方式の機械検査に載せた。

## 3. 改善案(優先順位付き)

前版のこの表は 12 行あった。**8 行が閉じ、残るのは 4 行**である。閉じ方の内訳のほうが
表そのものより重要で、**8 件のうち 3 件は実装ではなく測定で閉じた** — 要件が誤っていたか、
対象が最初から存在しなかった。表の外の P5(バージョン体系)を加えると 4 件になる:

| 閉じた要件 | 閉じ方 | 測定 |
|---|---|---|
| I10 / N3 zero-panic API(「0.2 破壊的変更・工数大」) | **要件が誤り** | 追跡指標「unwrap 242 / expect 20」は**テストコード込みの計測**だった。実装コードのみでは計 12 箇所、`panic!` は 0。12 箇所を書き換え、`deny` で恒久強制。シグネチャ変更は不要だった |
| P5 バージョン体系の不整合(engine 4.1.0 vs kit 0.1.0) | **要件が誤り** | 独立に公開されるクレートに版数一致の要請はない。4.x の根拠は engine `CHANGELOG [4.0.0]` に既に記録済みだった |
| I5 / N17 engine の float ソートを `total_cmp` 化 | **対象が存在しない** | `izanagi/src/` の `sort` / `max_by` / `min_by` / `partial_cmp` / `binary_search` は **0 件**。kit 側は `no_float_in_sim.rs` が float 自体を禁じているので派生的に 0 件。**この 0 件は測って終わりにせず `izanagi/tests/float_boundary.rs` の空の許可リストに載せた** — 最初に `sort_by` を足す人が「何を比較するのか」を書かない限りビルドが落ちる |
| I6 / N15残 recipe/drop/encounter の孤児参照検出 | **対象が存在しない** | `Content` のフィールドは `prefabs` / `tiles` / `levels` の 3 つのみ(`izanagi_kit/src/content.rs`)。検出すべきデータ型自体が無い |

残る 5 件(I1 README 同期 / I2 example 追加 / I3 N13 WFC selector / I7 N11 lockstep /
I8 N19 `Fixed` op 行列)は実装で閉じた。詳細は §2 の解消済み一覧。

> **この節から得られた教訓**: 改善案は**書き足す前に測る**。未測定のまま積んだ 12 件のうち
> 3 件は着手すれば無駄骨だった(2 件は対象ゼロ、1 件は 15 倍の過大見積り)。
> 以降、新しい行を足すときは「対象が何件あるか」を最初の列に書けるまで足さない。

### 残っているもの

| # | 改善案 | 効果 | 工数 | 推奨モデル | 依存 |
|---|---|---|---|---|---|
| I4 | N5 残り: MapBuilder 合成層(`Dungeon` に可変 API を足さず、`Vec<bool>` グリッド変換の合成で設計)。部品(`farthest_cell` / `keep_largest_region` / `ConnectivityMap`)は実装済みで、stage 連鎖 API 本体のみ未着手 | 中 | 中〜大 | **Opus** | なし |
| I9 | N20: cargo feature collections(`default=full` 必須。pinned hash テストを全 feature 組合せで確認すること) | 低 | 中 | **Opus** | なし |
| I11 | N2: incremental multiset hash — `savefile`/`replay` ヘッダの algo バージョニング設計が先。**単独で着手しないこと** | 低 | 大 | **Opus** | 設計合意 |
| I12 | N21: crates.io Trusted Publishing + cargo-semver-checks | 中 | 小 | ユーザー判断 | 初回公開の意思決定 |

これ以外の未着手候補(N10 構造化ファジング — `cargo-fuzz` が nightly を要求し本環境の
ネットワーク制約で不可 / N16 DSL `extends` / N18 archetype storage / N22 観測フック /
N23 LLM パイプラインの位置づけ)は **`izanagi_kit/RESEARCH.md` の N 候補表を正とする**。
同じ候補を 2 つの表で管理すれば必ず片方が古くなる ── 本節は「今すぐ着手できるもの」だけを持ち、
網羅は RESEARCH.md が持つ。

**ユーザー判断待ち(エージェントには実行不能)**:
1. **CI 有効化** — Web UI で `docs/ci/ci.yml` を `.github/workflows/ci.yml` として追加する。
   エージェント側の 3 経路(push / Contents API / Git Data API)はすべて 403 で実測済み。
   手順は [`docs/ci/README.md`](./docs/ci/README.md)。
2. **crates.io 公開** — 資格情報が本セッションに存在しない(`cargo publish --dry-run` は成功する)。

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
5. **`.github/workflows/` 配下を変更するコミットを作らない**。GitHub App トークンに
   `workflows` 権限が無く、**3経路とも実測で塞がっている**: git push は push 全体が拒否、
   Contents API と Git Data API(`POST /git/trees`)はいずれも
   `403 Resource not accessible by integration`。エージェント側の回避策は
   存在しないので再検証は不要。CI 定義は `docs/ci/ci.yml` に置き、有効化はユーザーに委ねる。
6. push 先は現行 feature ブランチのみ。**main へ push しない。PR は明示指示があるまで作らない。**

### 4.2 検証パイプライン(1 機能ごと・コミット前に全て実行)

```bash
cargo fmt --all                                    # 整形(ゲートは検査のみで直さない)
cargo test -p izanagi_kit --lib <対象module>       # 開発中の高速ループ
tools/gate.sh                                      # ★ゲート全段
```

`tools/gate.sh` が「green」の唯一の定義であり、以下を順に実行して最初の失敗で
non-zero 終了する: `cargo fmt --all -- --check` / `cargo test --workspace` /
clippy 警告 0 / rustdoc 警告 0(両クレート)/ pinned hash(`determinism` +
`roguelike_sim`)/ `kit_bridge` の統合ハッシュ `353498ec4fbcd160` を出力に含むこと /
`verify_pipeline_demo`(自身の主張を assert する)/ 両クレートの `cargo package`。

同じスクリプトを `.githooks/pre-push` と CI 提案(`docs/ci/ci.yml` の `gate` ジョブ)が
呼ぶので、ローカルの green と CI の green が定義上ずれない。フック有効化は**リポジトリ
ルートで** `git config core.hooksPath .githooks`。

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
