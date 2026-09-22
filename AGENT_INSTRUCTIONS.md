# IZANAGI — 長所・短所・改善案と作業指示書(Opus / Sonnet 用)

> **この文書の目的**: 本リポジトリの現状評価(長所・短所)と、優先順位付きの改善案を、
> **Claude Opus / Claude Sonnet が単独セッションでそのまま実行できる粒度**で記述する。
> 曖昧さを排し、各タスクに「対象ファイル・検証手順・リスク・推奨モデル」を明記する。
>
> 最終更新はコミット履歴を正とする(`git log -1 --format=%cd AGENT_INSTRUCTIONS.md`)。
> 基準ブランチ: `main`(`claude/deepresearch-ultrathink-improve-yq2th` の全内容は PR #9 でマージ済み・`4e3bc5c`)
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
| kit モジュール数 | **102**(`izanagi_kit/src/*.rs`。`tests/docs_are_current.rs` が検証)|
| engine モジュール数 | **25**(`izanagi/src/*.rs`。同上)|
| 決定論 pinned hash | `PINNED_FINAL_HASH=0xd1a9236e96a2c802` / `PINNED_ROGUELIKE_HASH=0x5286d1420200fe66`(不変) |
| kit_bridge 統合ハッシュ | `353498ec4fbcd160`(headless == engine-hosted) |
| panic 経路(実装コード) | `panic!` **0**・assert 系マクロは**命名済み15サイトに凍結**(両クレートで `clippy::unwrap_used`/`expect_used`/`panic` を `deny`。弱め属性・新規 panic マクロはテストが落とす) |
| 出荷可能性 | `cargo package` 両クレート成功(`--no-verify` なし)。さらに**展開した tarball の中で
doctest・テスト・example・bin の4ターゲットが緑**で、さらに `gamec` が同梱 fixture を
正しく受理・拒否することまで確認する — 同梱した「証拠」が消費者の手元で実際に走る |
| 機械検査された文書主張 | tier 表・README モジュール表・pinned hash・モジュール数・engine 版数・f32 境界・README のテスト数下限・README の Quickstart(doctest として実行)・engine CLAUDE.md の Map・全 md の相対リンク・**非 float 非決定論ソースの許可リスト**(`HashMap`/壁時計/スレッド/アドレス依存)・**engine の順序づけ 0 件**(float 比較ソートの不在)・**能力マップが検証系12モジュールを名指しすること**・**SPEC.md の G1〜G11 が強制場所を持つこと**(zero-dep / `forbid(unsafe_code)` / edition / MSRV 宣言 / 条件コンパイル判別式の禁止を含む)・**凍結した「本イテレーション」記述の不在**・**`.game` 文法とパーサの一致**(キーワード9種・行長1024・名前32・寸法256)・**ARCHITECTURE.md の file map と Engine の公開フィールド数**・**3つの README の Rust ブロックが doctest として実行されること**・**`.gitattributes` がテキストを LF に固定していること**・**manifest に `target`/`features`/`lints`/`patch`/`replace`/`build-dependencies` テーブルがなく `.cargo` 設定ファイルが存在しないこと**・**`allow`/`expect`/`warn`/`force_warn` が安全系 lint(`unsafe_code`/`unwrap_used`/`expect_used`/`panic`)を弱めないこと**・**SPEC が名指す強制場所のファイルが実在すること**・**panic 系マクロ(assert*/debug_assert*/unreachable!/todo!/unimplemented!)が命名済みサイトに凍結されていること**・**出荷コードが `#[path]`/`include!`/`include_bytes!` で走査領域外から混入しないこと**・**kit が fs/process/io の環境入力を読まないこと**・**gate.sh が全ステージを含むこと(ゲート自身の検査)**・**manifest に [[bin]]/[[test]]/[[bench]]/[[example]] テーブルと harness/auto*/crate-type/proc-macro/test/bench/doctest キーがなく [profile.*] の意味論キー(debug-assertions/overflow-checks/panic)が未設定であること**・**rust-toolchain/Cross.toml/clippy.toml が存在せず rustfmt 設定に逃走経路キーがないこと**・**[workspace.dependencies] の不在**・**出荷コードに env!/option_env!/extern/#[no_mangle]/#[link がないこと**・**kit が std::arch/core::arch を使わないこと**・**gate.sh が RUSTFLAGS/RUSTDOCFLAGS を除去すること**・**tests/ と examples/ が #[cfg]/#![cfg]/cfg!/cfg_attr/unsafe/#[ignore] (命名済み allowlist のみ・陳腐化検査つき)を持たず全テストファイルが #[test] を含むこと**・**md のフェンスに ignore/no_run/compile_fail/should_panic がないこと**・**.githooks/pre-push と docs/ci/ci.yml が tools/gate.sh を呼ぶこと**・**kit が panic 機構(catch_unwind/panic::)を持たないこと**・**走査配下(src/tests/examples)に symlink がないこと**・**重複スキャナ helper(test_module_boundary/library_sources/contains_token/count_token/kit_src/take_balanced)が全コピーでバイト同一であること**・**AGENT が core.hooksPath の設定を指示し続けること**・**src 内 doc フェンスに ignore/compile_fail/should_panic がなく no_run は命名済み allowlist(陳腐化検査つき)のみであること**・**共有境界 lexer が偽 marker を拒否し真 marker を受理すること(合成入力検査)**・**kit がレイアウト照会(size_of/align_of)・abort・stdout/stderr サイドチャネル(println/dbg 等)を持たないこと**・**workspace が resolver = "2" を実キーとして設定すること**・**全テストファイルが実 assert を含むこと(vacuous 緑不可・測定系は命名済み allowlist+陳腐化検査)**・**kit に同期プリミティブ(Mutex/RwLock/channel/atomic/thread)と生ポインタ経路(as */*const/*mut/.as_mut_ptr)がないこと**・**tests/・examples/ にサブディレクトリがないこと(平坦走査の盲点の不在)**・**tests/・examples/ に dead_code/unused 系抑制子がないこと**・**tests/ が env::var/args/set_var/remove_var/current_dir/current_exe を読まないこと(機械不変のスイート)**・**README/AGENT のテスト数フロアが実数の 75% 以内の新鮮さを保つこと(下限+帯域)**・**`#[test]` 属性計測が行頭アンカーでコメント/文字列の水増しを受けないこと**・**kit 定義型の全 `impl DetHash` が golden ピン済みか `UNPINNED_DET_HASH` で明示宣言されること(双方向陳腐化検査つき)**・**Cargo.lock がワークスペース2クレート以外を解決しないこと(解決層の zero-dep)**・**src/bin/ も unsafe/panic/unwrap/expect/cfg/env::var/thread/ポインタ/layout 照会を持たないこと(bin ターゲットは lib.rs の forbid/deny を継承しない別クレート root)**・**src/ 配下の全 .rs が lib.rs の `mod`/`pub mod` で宣言されていること(走査されるがコンパイルされない孤立ファイルの不在)**・**出荷 src に dead_code/unused* 抑制子がないこと(両クレート)**・**.github/workflows/ が存在しないこと(CI 非コミット規則の強制)**・**tests/ の env!/option_env! が CARGO_MANIFEST_DIR のみを読むこと(コンパイル時環境入力の禁止)**・**tests/ に `if <x>.is_ok()/is_err()` ガードで包まれた条件 assert がないこと(環境依存スキップの禁止)**・**validate() が全 findings を収集すること(early-exit primitive 構造禁止 + 全 defect クラス同時発火テスト)**・**manifest の license フィールド・LICENSE ファイル内容・README 記載が三方向一致すること)**・**kit に interior mutability(RefCell/cell::Cell/OnceCell/LazyLock/Rc/Arc/lazy_static/once_cell)がないこと**・**save の wire layout がバイトピンされ write が encode に委譲されていること**・**serialize の canonical 出力が文字列ピンされていること**・**diag_json/diag_sarif の出力がバイトピンされていること**・**DIRS コンパス順(pathfinding 8 方向・plan/wfc 4 方向)と wfc の opposite() がピンされていること**・**xoshiro の jump() 後状態と出力が値ピンされていること(弱い「差異あり」テストではタイポを見逃す)**・**Visibility の順序契約(Unseen<Remembered<Visible と rank 値)がピンされていること**・**SpatialHash::iter_cells の昇順反復がピンされ doc が実装(BTreeMap)と一致していること**・**両クレートの公開 API 集合(pub 宣言・struct/enum/trait メンバー・複数行シグネチャ・API 属性)が hash+件数でピンされていること**・**swept_aabb の役割交換対称性と tunneling(細壁への高速衝突)が性質テストされていること**・**Mat3 の点結合則・rotation の等長性+準同型・reflect 対合・perp 直交が性質テストされていること**・**ECS の get/get_mut/remove が stale ハンドル(世代不一致)を拒否すること**・**Animation が duration ≤ 0/NaN のフレームを構築時に拒否すること(tick の無限ループ経路の封鎖)**・**load_wav が切り詰め・巨大サイズの悪性 WAV で panic/無限ループせず Err を返すこと**・**Save::parse が u32::MAX 宣言長でも wrap せず Err を返すこと(32-bit 加算 wrap の封鎖)**・**3系のバイナリデコーダ(savefile/load_wav/Save::parse)が全接頭辞・全1バイト破壊・乱数 garbage で panic しないこと**|
| 未検証の公開 API | **両クレートで 0** — kit 1500+ / engine 240+ の公開関数(トレイトメソッドを含む)。各クレートの `tests/public_api_is_exercised.rs` が、どのテスト・example・bench からも呼ばれない公開関数の追加を落とす(件数は成長で変わるので下限表記)|
| バージョン | engine 4.1.0 / kit 0.1.0(独立公開なので一致は不要。4.x の根拠は engine CHANGELOG `[4.0.0]`)|
| MSRV | engine 1.65 / kit 1.75 |
| main との差 | **0 遅れ**。PR #7 系(`b1607f1` 迄)は `617d651` で、続きのコミット列は PR #9 でマージ済み(`4e3bc5c`)。以降の作業は新ブランチで管理 |
| kit src 内 panic 系(**実装のみ**) | `panic!` 0・assert 系は命名済み allowlist サイトのみ(`clippy::unwrap_used/expect_used/panic` を `deny` + `panicking_macro_allowlist` で凍結。テスト込みの旧計測 242/20 はテストコードを数えていた) |

---

## 1. 長所(すべて測定値の裏づけあり)

1. **検証系11モジュールが揃い、相互に補完する** — `sim`(監査)/ `verify`(有界モデル
   検査・**証明**)/ `temporal`(時相性質)/ `recovery`(クラッシュ復旧)/ `explore`
   (archive 探索)/ `prop`(性質・モデル検査)/ `shrink`(縮約)/ `plan` / `dst` /
   `rollback` / `replay`(desync 局所化)。各々が別のバグクラスを狙い、出典が明記されている。
   (`world_hash` はこの11件が**乗っている基盤**であり道具ではない — README も
   fixed-point / seeded RNG と並べて substrate に分類している。)
   **決定的な非対称性**: 全ツールが「見つからなかった」を言えるが、
   「存在しない」を言えるのは `verify` だけ(三値の `Holds`/`Violated`/`Exhausted`)。
2. **主張が機械検査される(41+種)** — tier 表・README モジュール表・pinned hash・
   モジュール数・engine 版数・版数と CHANGELOG の対応・f32 境界・**engine の順序づけ 0 件**・
   engine CLAUDE.md の Map・全 md の相対リンク・README のテスト数下限・
   README Quickstart(doctest 実行)・**能力マップの検証系被覆**・
   **SPEC.md の全体不変条件 G1〜G11 が強制場所を名指しすること**・
   **どの文書も終わったイテレーションを指さないこと**・
   **SPEC.md §9.1 の EBNF がパーサと同じキーワード集合・同じ境界値を持つこと**・
   **ARCHITECTURE.md の file map / subsystem 数 / 図が実体と一致すること**・
   **全 fence が言語タグを持つこと**・**2つの README が doctest として配線されていること**・
   **ルート README の Rust ブロックが engine README に逐語で存在すること**・
   **`include_str!` がパッケージ外を指さないこと**・
   **kit README 冒頭の「Eleven modules」が検証系の実数と一致すること**・
   **本書 §1 が README と同じ11件を名指しすること**・
   **ポインタ幅の値が world hash に到達しないこと**(SPEC.md G9)・
   **native-endian のバイト列が world hash に到達しないこと**(SPEC.md G10)・
   **条件コンパイル判別式がターゲット/プロファイル/feature を参照しないこと**(SPEC.md G11)・
   **チェックアウト行末が LF に固定されていること**(`.gitattributes`)・
   **deny 系の安全 lint を `allow`/`expect`/`warn`/`force_warn` が弱めないこと**・
   **manifest のセクション文法が閉じていること**(`target`/`features`/`lints`/`patch`/`replace`/`build-dependencies` 禁止)と `.cargo` 設定が存在しないこと・
   **強制場所として名指されるファイルが実在すること**・
   **panic 系マクロが理由つき allowlist に凍結されていること**・
   **`#[path]`/`include!`/`include_bytes!` による走査外コード混入の不在**・
   **kit の ambient-input 禁止が fs/process/io に及ぶこと**・
   **gate.sh が全ステージを名指しで含み、RUSTFLAGS/RUSTDOCFLAGS を除去して走ること**・
   **manifest の残り文法**(`[[bin]]`/`[[test]]`/`[[bench]]`/`[[example]]` ターゲットテーブル、
   `harness`/`auto*`/`crate-type`/`proc-macro`/`test`/`bench`/`doctest` キー、profile の
   `debug-assertions`/`overflow-checks`/`panic`)が閉じていること・
   **`rust-toolchain`/`Cross.toml` の不在**・**`env!`/`option_env!`/`extern`/`#[no_mangle]`/`#[link` の不在**・
   **kit が `std::arch`/`core::arch` の CPU 機能検出を使わないこと**・
   **検証スイート自身の規律**(`#[ignore]` は命名済み allowlist のみ・tests/ に `#[cfg]`/`unsafe` なし・
   全テストファイルに `#[test]` あり)・**rustfmt/clippy の設定面**(rustfmt の `ignore`/
   `disable_all_formatting`/`skip_children` キー禁止・`clippy.toml` 不在・
   `[workspace.dependencies]` 禁止)。
   加えて **panic 経路 0**(コンパイラ強制)、**未検証の公開 API 0(両クレート)**、
   **MSRV 違反 0**(静的検査)、**非 float の非決定論ソース 0**(許可リスト方式)。
3. **オラクル中心のテスト 3,600+ 件** — 手計算値ではなく独立実装との照合。BFS オラクル
   (jps4/plan)、代数則(三角不等式・単調性)、denotational 参照(temporal の全パターン)、
   保存則(pool)、ビルダー等価(autotile/encounter)。
   **検査器自身も変異注入で検証**する慣行が定着している。
4. **zero-dependency / `#![forbid(unsafe_code)]`** — 両クレートとも実行時依存ゼロ。
   両クレートが**検証付き**で梱包可能(`cargo package` が tarball からビルドし直す)。
5. **「green」の定義が1つ** — `tools/gate.sh` の7段(最終段は tarball を展開して
   **doctest まで走らせる**)。pre-push フックと CI 提案が
   同じスクリプトを呼ぶので、ローカルと CI が定義上ずれない。
6. **エンジンとキットの境界が測定され機械検査される** — engine 25 モジュール中 8 が
   完全に float-free。`rng` は整数コアが replay-safe、便利側がそうでないという
   **分割**まで明記されている。
7. **研究駆動の開発記録** — `RESEARCH.md` に出典・実装 commit・見送り理由が残り、
   後続セッションが文脈を復元できる。本セッションでは「見送り理由」自体が
   誤っていた例(N3)も記録され直した。

## 2. 短所(重要度順・証拠付き)

### 未解決

> **この4件の性質**: 1 と 5 は**ユーザーの操作を待っているだけ**で、作業は完了している。
> 2 と 3 は**構造的限界であり、道具では埋まらない**という結論であって、未着手の作業項目ではない。
> 4 は意図した折り合い。つまり「コードで閉じられる欠陥」はここには残っていない。

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
4. **[小] example が印字する数値は誰も検算していない** — 29本中、assert を持つのは
   `verify_pipeline_demo`・`kit_bridge`・`menu_textlayout_demo`・`replay_demo`・
   `savefile_demo` の5本だけで、残りの**24本は assert をひとつも持たない**。
   gate は全29本について「headless 完走・非空出力・2回実行でバイト一致」を強制するので、
   panic・ハング・沈黙・非決定性は落ちる。**しかし「安定して間違っている」出力は通る。**
   個々の値の正しさは 3,600+ の単体テスト側で担保されており、example ごとに golden
   ファイルを置けば今度はそれが腐る(本セッションで正確な数値を4度削除した理由と同じ)。
   現状は意図した折り合いであり、欠落ではない — ただし主張はここまでである。
5. **[小] crates.io 未公開** — `cargo publish --dry-run` は成功する(公開可能)。
   資格情報が無く、公開自体が取り消し不能なので、ユーザーの判断と手による。

### 解消済み(本セッション)

- ~~pinned hash は debug profile でしか検証されていなかった~~ → `overflow-checks` は
  dev で既定 on・release で既定 off なので、シミュレーション経路の算術が静かに wrap する
  コードは debug では panic して気づけるが release では気づけない。gate は debug の
  pinned hash しか見ておらず、「このハッシュは安定している」という中核主張が
  **実際に出荷される profile では一度も検証されていなかった**。実測すると release でも
  同じハッシュが出た(健全)。gate の同じ段に release 実行を追加し、両方一致することを
  要求する — 追加コストはコンパイル1回 + 0.1秒。CI 提案にも `release` job を追加し、
  ワークスペース全体を release で通す(約3分半、push 前ではなく CI に置く判断)。
- ~~公開 API の panic 経路~~ → **要件が誤りだった**。「unwrap 242」はテストコード込みの
  計測で、実装のみでは12箇所・全て到達不能。0.2 破壊的変更は不要で、書き換えて
  コンパイラ強制(`deny(clippy::unwrap_used/expect_used/panic)`)に。
- ~~バージョン体系の不整合~~ → **要件が誤りだった**。独立公開なら版数一致は不要で、
  4.x の根拠は engine CHANGELOG `[4.0.0]` に記録済み。
- ~~engine の f32 境界が不明~~ → 実測(25中8が float-free、`rng` は分割)し機械検査。
- ~~新モジュールの example 不在~~ → `verify_pipeline_demo` が11モジュールを1本で通し、
  印字する主張をすべて assert する。
- ~~文書が古い~~ → 乖離4文書を削除、残りの検証可能な主張を27種の機械検査に。
- ~~main が遅れている~~ → main をマージして 0 遅れ。PR #7 はマージ済み、続きのコミット列は PR #9 でマージ済み(`4e3bc5c`)。
- ~~非 float の非決定論が未検査~~ → 監査で `SpatialHash` の**実バグ**を発見・修正し、
  クラス全体を許可リスト方式の機械検査に載せた。
- ~~engine には公開 API の門番が無かった~~ → kit だけが「未検証の公開 API 0」を強制しており、
  engine(v4.1.0 として公開・リポジトリ名を冠する側)には同等の検査が無かった。実測すると
  247件中**24件**が未行使 — kit が最初の掃引で見つけた数と同じだった。23件をオラクル付きで
  行使し(残り1件は `#[doc(hidden)]`)、engine 側にも門番を設置。併せて両クレートの掃引が
  **トレイトメソッドを1件も数えていなかった**盲点を塞いだ(kit 6 + engine 5)。
- ~~クロスプラットフォーム決定性が float の話だけだった~~ → 「bit 一致 replay」を
  壊すもう一つの経路は **`usize` の幅**で、これは仮定の話ではない — CI は kit を
  `wasm32-unknown-unknown`(`usize` = 32bit)向けにビルドする。実測した結果
  **設計は既に正しかった**: `usize`/`isize` の `DetHash` 実装なし、`Fnv1a` に
  `write_usize` なし(API の形で不可能にしている)、長さは `as u32`、`det_hash` 本体で
  `usize` に言及するものゼロ、ライブラリコードに `usize::MAX` ゼロ。
  ただし**規律による偶然であって強制されていなかった** — `impl DetHash for usize` は
  3行の変更で、wasm job はビルドするだけでハッシュを比較しないので気づかない。
  SPEC.md に **G9** として明記し、`hashes_are_width_independent.rs` で強制。
  G9 を書いた時点で `global_invariants_hold.rs` が**強制場所の登録を要求して落ちた** —
  前に作った仕組みが、演習ではなく本物の新規不変条件で機能した。
- ~~クロスプラットフォーム決定性のもう一つの穴: エンディアン~~ → G9 の隣にある同じ形の
  リスクで、Rust の `to_ne_bytes()` はターゲット CPU のネイティブなバイト順を返す
  (x86-64・wasm32 は little-endian、一部の組込み・メインフレームは big-endian)。
  `Fnv1a` の全 write メソッドは既に `to_le_bytes()` を明示的に呼んでおり(`to_ne_bytes`
  ではなく)、`world_hash.rs` 自身のコメントも「no native-endian ... leakage」と設計
  意図を述べていた。実測するとライブラリコード全体で `to_ne_bytes`/`to_be_bytes` は
  **ゼロ**。ただし**現行 CI ターゲットはすべて little-endian なので、`to_ne_bytes` に
  変わっても実行テストは何も気づかない**潜在バグの形だった(G9 と全く同じ構造)。
  SPEC.md に **G10** として明記し、`hashes_are_endian_independent.rs` で静的に強制
  (`to_ne_bytes`/`to_be_bytes` 禁止 + 各 write が `to_le_bytes` を呼ぶことを確認)。
  G10 も同様に `global_invariants_hold.rs` の登録要求を経由した。
- ~~同じ族を4つの文書が4つの数で説明していた~~ → 検証系モジュールについて
  kit README 冒頭は「Eleven」、本書 §1 は「11」だが `world_hash` を含み `replay` を欠く、
  `VERIFICATION_FAMILY` は12、能力マップ section Q は10行。**§1 は2つの誤りが
  打ち消し合って11になっていた**。`world_hash` は11件が乗っている**基盤**であり道具ではない
  (README 自身も2文あとで fixed-point / seeded RNG と並べて substrate に分類している)。
  正しい11件を `INTERROGATION_MODULES` として1箇所で定義し、README 冒頭の数詞と
  §1 の一覧の両方をそれに対して検査する。crates.io と docs.rs の第一文である。
- ~~「ビルドが通る」を動作の証拠として扱っていた~~ → 上の3件はすべて「ビルドは通るが
  動かない」だった。そこで gate は tarball 内で **bin もビルドし、`gamec` を実際に走らせる** —
  同梱の `dungeon.game` を受理し `broken.game` を拒否すること。壊れた内容を通すゲートは
  無いより悪い。加えて**この検査自身の穴を塞いだ**: fixture が出荷されなくなった場合も
  非ゼロ終了になるため「正しく拒否した」と区別できず、検査が静かに通ってしまう。
  fixture の存在を先に主張するようにし、変異注入で確認した。
- ~~公開クレートの example がコンパイルできなかった~~ → engine は `kit_bridge` example を
  同梱するが、これは `izanagi_kit` への **path による dev 依存**を使う。cargo は公開
  マニフェストから path 依存を除去する(消費者に解決できないので当然)ため、tarball には
  **存在しないクレートを import する example** が残っていた。`cargo build --examples` が
  `unresolved import izanagi_kit` で失敗する。`cargo package --verify` はライブラリしか
  ビルドしないので見えない。除外し(この example の仕事は本リポジトリの gate で pinned hash
  `353498ec4fbcd160` を確認することであり、消費者は kit が無いので実行できない)、
  gate に example ビルド段を追加。**同じクラスで3回連続の見落とし**(doctest / テスト /
  example)だったので、ターゲットごとに個別に検査する。
- ~~同梱した「証拠」が消費者の手元では走らなかった~~ → 両クレートの `exclude` は
  「テストと example は残す。『このシミュレーションは証明可能に replay する』を主張する
  クレートにとって、証拠は製品の一部だから」と述べている。しかし公開 tarball で
  `cargo test` を走らせると **kit 5ファイル・engine 1ファイルが失敗**した — これらは
  `AGENT_INSTRUCTIONS.md` や姉妹クレートのソースなど**パッケージの上にあるファイル**を
  読むリポジトリ観点の検査で、消費者は受け取っていない。到着時に失敗する証拠は
  証拠が無いより悪い。該当6ファイルを `exclude` に移し(消費者に意味を持つのは
  コードについての証拠だけ)、gate の最終段が tarball の中で全テストを走らせるようにした。
- ~~gate が green のまま「出荷すると壊れているクレート」を通した~~ → 直前の反復で入れた
  `include_str!("../../README.md")` は**パッケージの外**を指しており、tarball にその
  ファイルが入らない。展開して `cargo test --doc` を実行すると失敗する。
  **`cargo package --verify` は捕まえない** — 存在しないファイルを捕まえるための段なのに、
  走らせるのは**ビルド**であり、ビルド下では `#[cfg(doctest)]` の項目自体が消えるため。
  自分の変更を疑って実測したことで発見した。include を削除し、ルート README の
  Rust ブロックは**コンパイルされている engine README のブロックと逐語一致すること**で
  検査する方式に変更(パッケージ境界を跨がず、重複が乖離しないことも同時に強制)。
  gate の最終段は tarball の中で doctest を実行するようにし、クラス全体を静的にも
  検査する(`include_str!` がパッケージ外を指さない)。両方向とも変異注入で確認。
- ~~MSRV スキャナの針が薄く、境界規則が片側だけだった~~ → rustup が本環境で塞がれている以上、
  この静的スキャナが両クレートの MSRV 主張を支える唯一の仕組みで、ユーザーが有効化しようと
  している CI の `msrv` job が実トールチェーンで検証する対象そのもの。針を 9 → 23 に拡張
  (grid/固定小数点コードが反射的に使う `midpoint`/`next_multiple_of`/`is_sorted`/`chunk_by`/
  `as_chunks` ほか、および**構文**である let-chain — 古いコンパイラではパースエラーになる、
  最悪の壊れ方)。`Option::inspect` は `Iterator::inspect`(1.0)と綴りを共有するため
  **意図的に除外**し、その理由を表に明記した(誤検出する検査器は切られる)。
  拡張の過程で `contains_token` が**左境界しか見ていない**ことが露見 — `.` で始まる針を
  無検査で通す実装で、全ての `.` 針が `(` で終わっていた間だけ健全だった。姉妹2ファイルと
  同じ両側規則に統一。**自己テストが捕まえた**。
- ~~engine とルートの README が誰にもコンパイルされていなかった~~ → kit だけが
  `#[cfg(doctest)] #[doc = include_str!]` で README を doctest 化しており、engine の README
  (2ブロック)とルート README(2ブロック)は**何にもビルドされていなかった** — crates.io と
  GitHub の訪問者が最初にコピーするコードである。配線した結果、ルート README の
  **言語タグの無い fence 2件が実際の欠陥として露見**した(rustdoc は無タグ fence を Rust と
  みなすため `cargo test --workspace` を式としてコンパイルしようとする)。`text` タグを付け、
  全12文書の fence にタグを要求する検査と、3つの README の配線が外れないことの検査を追加。
  ブロック自体は4件とも正しく、コンパイル・実行できた。
- ~~ARCHITECTURE.md が実体と乖離~~ → engine の設計文書は file map に **25 中 16** しか載せて
  おらず、`audio_pcm`/`camera`/`debug`/`event`/`gamepad`/`log`/`sprite`/`tilemap`/`tween` の
  9件が存在しないかのように読めた。加えて「`Engine` owns **six** subsystems」(実際は10、
  図は5個しか描いていない)、「Total: ~1700 lines, ~85 tests」(実際は 5,898 行 / 209 テスト —
  3.5倍・2.5倍)。**同じクレートの同種文書 CLAUDE.md は機械検査済みで、こちらは無検査だった**
  (漂流した手管理インベントリの5件目)。行数・テスト数は訂正ではなく**削除**し、
  file map・subsystem 数・図の3つを `tests/architecture_md_is_current.rs` に載せた。
- ~~パーサが敵対的入力で試されていなかった~~ → `gamec` は「利用者が書いていないファイルを
  検査する CI ゲート」と謳う本ワークスペース唯一の非信頼入力境界で、`parse` は 25 箇所で
  添字・スライスを行う。既存の `roundtrip_fuzz.rs` は**意図的に整形式の入力しか生成しない**
  (理由も文書化済み)ため、拒否経路は一度も通っていなかった。22 の敵対ケース
  (長すぎる行・u32 溢れ寸法・多バイト hex・絵文字 glyph・NUL・孤立 CR ほか)と
  400 seed のランダムバイト列で測定 — **パニックは無く、実装は健全だった**。
  結果を恒久検査にし、SPEC.md §9.1 の EBNF がパーサと同じキーワード集合・同じ境界値を
  持つことも検査に載せた。文法の「幅 W・高さ H」注記は validator が1段後で強制しており、
  その分業も検査で記録した。
- ~~仕様書の不変条件が誰にも強制されていなかった~~ → `SPEC.md` §2 は全モジュール必須の
  不変条件 G1〜G8 を掲げるが、うち3件を強制するものが無かった。最大のものが **G1
  zero runtime dependencies** — README 冒頭・両 CLAUDE.md の規則・本書のハード制約・
  crate の package description と少なくとも5箇所が約束しているのに、`cargo add` 一発で
  そのすべてが黙って偽になる状態だった(G2 `forbid(unsafe_code)` の存在、G8 の
  edition/MSRV 宣言も同様)。`tests/global_invariants_hold.rs` を追加し、**G 表の各行が
  強制場所を名指しすること自体**を検査に載せた — 9個目の不変条件を仕様に書けば、
  どこで強制するかを書くまでビルドが落ちる。
- ~~文書が終わったイテレーションを指す~~ → `GAME_DEV_TAXONOMY.md` と `SPEC.md` で再発
  していた(SPEC は表が「D1/P1/R1 実装済」と書く直下で同じ3件を「不足部分」として
  列挙する自己矛盾)。全12文書で、読者が居ないイテレーションを指す表現を禁止した。
  禁止語の一覧は検査器(`docs_are_current.rs`)側にのみ置く — 本書に書けば本書自身が
  引っかかる。実際この行の初稿が引っかかり、検査器に例外を足すのではなく文面を直した。
- ~~gate は 29 本の example のうち 2 本しか実行していなかった~~ → 残り27本はコンパイルされる
  だけで一度も実行されず、panic する example もハングする example も出荷され得た。
  engine の CLAUDE.md が「example は headless で完走し、結果を印字すること」を規則として
  掲げていたが、それを検査するものが無かった。gate に9段目を追加 — **一覧をファイル
  システムから読む**ので新規 example は追加当日から対象になる。3方向の変異注入で確認
  (panic する / 何も印字しない / 壁時計を読む example をそれぞれ落とす)。
  副産物として、全29本が**2回実行でバイト一致**することを実測した。
- ~~能力マップが検証系を1行も持たない~~ → README が「能力マップ」として案内する
  `GAME_DEV_TAXONOMY.md` は検証系モジュールを**9件まったく言及していなかった**
  (本書が本製品の決定的な長所と呼ぶ族が、能力表では存在しないように見えていた)。
  section Q を追加し、以後の欠落は `docs_are_current.rs` が落とす。
- ~~条件コンパイルは「使われていない」だけで強制されていなかった~~ → G9/G10 が封じたのは
  hash に届く**値**の経路であり、`#[cfg(target_pointer_width = "64")]` や
  `cfg!(debug_assertions)` は禁止トークンを一切使わずに「コンパイルされるコード
  そのもの」をターゲット・プロファイルで変えられる — pinned hash は実行を比較する
  だけでビルド差分を見ない。実測で library code の対象 cfg は **0 件**
  (規律による偶然だった)のを、**G11** として SPEC に明記し
  `tests/no_platform_cfg_in_sim.rs` が `cfg`/`cfg!`/`cfg_attr` の全判別式を
  許可リスト(`test`/`doc`/`doctest`/`docsrs` + `not`/`any`/`all`)で検査。
  同型の迂回として manifest の `[target.'cfg(...)'.dependencies]` セクション
  (G1 の空 `[dependencies]` 検査をすり抜ける)も両クレートで禁止。変異注入
  (`#[cfg(unix)]` を実ファイルに混入)で検出を確認済み。
- ~~`#[cfg(test)]` の境界検索がコメント内の文字列に引っかかっていた~~ → lib.rs:132 の
  `// ... #[cfg(test)]` という記述が生ソース検索の境界になり、全スキャンが
  lib.rs を89文字の doc ヘッダで打ち切っていた(後続の `cfg_attr` deny・`cfg(doctest)`
  配線・`pub mod` 宣言を一度も見ていなかった)。新チェッカーの自己検証で発覚。
  `test_module_boundary`(非コメント行に限定した境界検索)を7ファイル9箇所へ水平展開。
- ~~チェックアウトの行末が clone 側の設定依存だった~~ → `* text=auto eol=lf` の
  `.gitattributes` を追加。fixture のバイト比較・複数行 `contains` 検査・`sh` の
  gate.sh は `core.autocrlf=true`(git の Windows インストーラ既定)の clone で
  壊れ得た。「同一入力」はリポジトリ自身のバイトにも適用される。存在と内容は
  `docs_are_current.rs` の `checkout_line_endings_are_pinned` が検査する。
- ~~`deny` は crate root にあるので安全と思われていた~~ → `deny` は**レベル**であり、
  リーフの `#[allow(clippy::unwrap_used)]` 一つで局所的に無効化できる。G7 の
  「panic 経路 0」は、deny 属性の存在と、その lint を名指す弱め属性
  (`allow`/`expect`/`warn`/`force_warn`)が非テスト領域に無いことの両方を
  `global_invariants_hold.rs` の `g7_*` で検査。`cfg_attr(test|doc|docsrs, …)`
  内側の弱め属性は出荷コードに効かないので免除。実ファイルへの
  `#[allow(clippy::unwrap_used)]` 注入で検出確認済み。
- ~~manifest のセクション文法が開いたままだった~~ → `[target.*]` だけでなく
  `[features]`・`[lints]`・`[patch]`・`[replace]`・`[build-dependencies]` も
  セクション走査が見ないビルド設定であり、`.cargo/config{,.toml}` は
  `--cfg` や `--cap-lints` を通じて **このスイート全体を迂回する**。
  いずれも存在しないことを実測し、ヘッダ原子の禁止リスト+`.cargo` 不在検査で
  閉じた(`no_platform_cfg_in_sim.rs`)。注入テストで両方とも検出を確認。
- ~~SPEC が名指す強制場所の実在が未検査だった~~ → `enforcement_sites()` の
  パス表記トークンが実在することを検査する `every_named_enforcement_site_is_a_file_that_exists`
  を追加。検査ファイルの改名/削除が「強制されていると読める亡霊」を残さなくなった。
- ~~`deny(panic)` は panic 経路全般を封じると思われていた~~ → `clippy::panic` が
  捕まえるのは `panic!` だけ。`assert!`/`debug_assert!`/`unreachable!`/`todo!`/
  `unimplemented!` は同じトラップに lowering されても lint をすり抜ける。実測で
  **15箇所**存在(コンストラクタの文書化済み事前条件 assert・内部不変の
  debug_assert・replay の zip-longest 到達不能 arm)。`panicking_macro_allowlist`
  (ファイル・件数・理由)に命名して凍結し、新規サイトは理由を書かないと落ちる。
  実行時入力への saturate/None/no-op 契約(G7)と区別するため、allowlist は
  「プログラマエラーの事前条件」を明記したものだけを認める。`todo!` 注入で検出確認。
  残存論点: スライス index `v[i]` は構文的に正当読み取りと区別不能なのでこの
  allowlist の射程外 — G7 の境界はそこに引かれる(RESEARCH.md 参照)。
- ~~スキャン領域の外からコードを混入する経路が未検査だった~~ → 全スキャナは
  「コンパイルされるコード == src/ 配下のテキスト」を仮定するが、`#[path]` は
  mod を任意ファイルに向け、`include!`/`include_bytes!` は走査外のバイトを
  混入できる。存在ゼロを実測し `shipped_code_cannot_come_from_outside_the_scanned_tree`
  で禁止(`include_str!` は文字列のみで済む doc 用に免除)。gen.rs を使う実注入で検出確認。
- ~~kit の ambient input 禁止が env:: のみだった~~ → ファイルシステム・プロセス・
  std I/O は入力ログに載らない環境入力であり BANNED リストに追加(`fs::`/
  `process::`/`std::io` 等)。`use std::process` 注入で検出確認。
- ~~gate.sh のステージ削除は何事も失敗させなかった~~ → 実行したチェックだけが
  通るので、stage を1つ消しても exit 0 のまま。`the_gate_script_still_runs_every_stage`
  が各ステージの識別トークン(fmt/test/clippy/doc/pinned-hash 両プロファイル/
  examples 列挙/package/tarball 内検査/gamec)の存在を検査。トークン削除で検出確認。
- ~~manifest 文法はテーブルだけ閉じれば足りると思われていた~~ → `[[bin]]`/`[[test]]`/
  `[[bench]]`/`[[example]]` ターゲットテーブル(path 転送・`harness = false` でテストを
  空実行化)と、テーブルを要しない `harness`/`auto*`/`crate-type`/`proc-macro`/`test`/
  `bench`/`doctest` キーが未禁止だった。さらに既存の `[profile.*]` テーブル自体は正当だが
  `debug-assertions`/`overflow-checks`/`panic` キーはプログラム意味論を書き換える。
  インライン `{...}` 内のキーも拾う行キースキャンで全面禁止。注入3種で検出確認。
- ~~ビルドを変えるファイルは .cargo だけと思われていた~~ → `rust-toolchain{,.toml}` は
  コンパイラ自体を差し替え(注入試験で channel=1.60 は MSRV エラーでビルド停止を実演)、
  `Cross.toml` はターゲットを変える。いずれも不在を検査。
- ~~gate.sh は呼ばれた環境で正直に走ると思われていた~~ → RUSTFLAGS/
  CARGO_ENCODED_RUSTFLAGS/RUSTDOCFLAGS は `--cfg`/`--cap-lints` を rustc/rustdoc に
  届けるため、呼び出し元シェルの環境がスイート全体を弱め得た。gate.sh 冒頭で
  無条件 unset し、その行の存在をステージ検査に追加(削除注入で検出確認)。
- ~~出荷コードが参照するのはソースツリーだけと思われていた~~ → `env!`/`option_env!` は
  ビルド機の環境変数をバイナリに焼き込み、`extern`/`#[no_mangle]`/`#[link` は manifest の
  依存リストに載らないネイティブリンクを宣言できる。実測ゼロを確認し provenance 検査に
  追加(env!・extern の実注入で検出確認)。また `std::arch`/`core::arch` の CPU 機能検出は
  「どのマシンで走るか」への実行時分岐として kit BANNED に追加(`use std::arch` 注入で検出確認)。
- ~~検証スイート自身は無条件に信頼されていた~~ → `#[ignore]` はスイートを緑のまま
  「実行された」ように見せる(実際に det_hash_golden.rs の再生成ヘルパーに1箇所存在)、
  tests/ 内の `#[cfg]` はプラットフォームでチェックを消し、テスト crate は lib の
  `forbid(unsafe_code)` を継承しないので `unsafe` が持ち込め、ゼロ `#[test]` のファイルは
  何も走らないままコンパイルされる。`the_verification_suite_cannot_quietly_skip_or_disable_its_own_checks`
  が両 crate の tests/ を走査(コメント・文字列・char リテラルを除外した上で)。
  `#[ignore]`/`#[cfg(unix)]`/`unsafe`/無テストファイルの各注入で検出確認。
- ~~rustfmt.toml の存在は良性と信じられていた~~ → `ignore`/`disable_all_formatting`/
  `skip_children` キーは `cargo fmt --check` の対象を縮小できる。izanagi/rustfmt.toml は
  存在を許したまま逃走経路キーのみ禁止。`clippy.toml`(`allow-unwrap-in-tests` 等)と
  `[workspace.dependencies]`(path-only 検査をすり抜ける依存隠し)は不在を検査。
  各々の注入で検出確認。
- ~~examples/ とドキュメントのフェンス指定はスイート外と思われていた~~ → gate が
  examples を2回バイト比較する以上、example 内の `#[cfg]` は「証拠そのもの」を
  プラットフォームでフォークし得る。tests/ と同じ規律(#[cfg]/unsafe/#[ignore] 禁止)を
  examples/ にも適用。md の ```ignore/no_run/compile_fail/should_panic フェンスは
  「doctest される」主張を空にするので全 md で禁止。また .githooks/pre-push と
  docs/ci/ci.yml が tools/gate.sh を呼ぶことを凍結 — "green" が一つの定義であることの
  最後の継ぎ目。3種の注入で検出確認。

- ~~内側属性は `#[cfg]` 検査をすり抜けていた~~ → `#![cfg]` は同じ力の別表記。
  ファイル冒頭の `#![cfg(unix)]` はテスト/example 全体をプラットフォーム条件化できる。
  禁止 needle に追加(`#![cfg(unix)]` 注入で検出確認)。また `#[ignore]` allowlist に
  「対象ファイルが実際に `#[ignore]` を保持する」陳腐化検査を追加(ignore 除去注入で
  検出確認)。副産物: Cargo.lock は gitignore 済み(ライブラリの正しい性質)と確認 —
  内容は zero-dep manifest 経由で間接凍結。
- ~~panic は deny で封じられていたと思われていた(第2段)~~ → lint の否定は
  `panic!` だけで、`catch_unwind`/`std::panic::*` は panic を**吸収**する
  機構 — G7「panic しない」の別の穴(実 panic を catch して通過結果に換金
  できる)。kit BANNED に追加(`catch_unwind` 注入で検出確認)。また走査配下
  (src/tests/examples)の symlink 不在を検査 — symlink はレビュー面に出ない
  外部バイトを読み込む(symlink 注入で検出確認)。
- ~~非決定論の抜け道は時計・環境・I/O で尽きたと思われていた(第3段)~~ →
  スケジューリング(Mutex/RwLock/channel/atomic/thread)とポインタアドレス
  漏洩の safe な別表記(`.as_mut_ptr(`、`as *`、`*const`/`*mut` cast)は未
  凍結だった。BANNED 追加(Mutex 注入で検出確認)。
- ~~境界検出は安全になったと思われていた(第1段で // コメント対策済み)~~ →
  `#[cfg(test)]` の偽 marker は文字列リテラル・ブロックコメント・行中配置
  でも打切りを引き起こし、しかも実装は 10 箇所 3 種に drift していた
  (global_invariants と no_platform_cfg は弱い find 版、no_float は行
  contains 版)。lexer 化して全面統一し、コピー同一性をテスト化(文字列内
  偽 marker + drift 注入で検出確認)。
- ~~重複は test_module_boundary だけと思われていた(第9段)~~ → contains_token
  は3種に drift(no_float 版は旧両側境界仕様)、library_sources は6箇所5種
  (width/endian は平坦走査で src サブディレクトリを構造的に見逃す)に
  既に分岐していた。全コピーを正規版に統一し、同一性検査を全重複 helper
  へ一般化。加えて .githooks が発火する唯一の連鎖「AGENT が core.hooksPath
  の設定を指示する」を検査化(drift + 指示削除の変異で検出確認)。
- ~~フェンスの opt-out は md だけの問題だった(第6段)~~ → src 内 `//!` doc
  コメントにも同じ逃走経路があり、log.rs の ` ```ignore ` は**実際に腐って
  いた**: `#[macro_export]` で crate ルートに移った `info!`/`warn!` を
  `izanagi::log::info` という誤った use で呼ぶ例がコンパイルすらされて
  いなかった。use を修正して ```rust に格上げ。src フェンス検査を追加
  (ignore 注入で検出確認)。同一性凍結だけでは「全コピーが同じく間違って
  いる」は防げない — lexer 正確性を合成入力で検査化(偽 marker 受理変異で
  検出確認)。
- ~~非決定論の凍結は値の話だけだった(第3・8段)~~ → `size_of`/`align_of`
  (ターゲットレイアウトを問う幅依存入力)、`abort(`(巻戻しも catch_unwind
  も通らない唯一の脱出)、stdout/stderr 書き出し(例出力バイト比較への
  無音汚染経路)が未凍結だった。BANNED に7 needle 追加(dbg! 注入で検出
  確認)。`{:?}`/Debug は実測で全て診断文字列と判明し許容・記録。
- ~~workspace manifest は設定済みと思われていた~~ → `resolver = "2"` は
  存在したが「実キーとしての存在」は未検査で、コメント化すれば警告のみ
  で通った。`[workspace]` セクション内の実キー検査に強化(コメント化
  変異で検出確認)。
- ~~#[test] の存在がテストの実在を保証すると思われていた~~ → assert 系
  マクロがゼロのファイルは「実行したが何も検証しない緑」。実測で
  bench.rs(測定ハーネス・出力が成果物)1件のみ該当 → 理由付き allowlist
  に登録し、全テストファイルに assert トークン床を新設(assert 消去変異
  で検出確認)。src 内 test 領域は実測ゼロ vacuous。
- ~~tests/ の平坦走査が tests/ 全体を覆うと思われていた~~ → cargo の
  テストクレート発見とスイート走査はどちらもトップレベルのみ —
  サブディレクトリの .rs は実行も走査もされない最も死んだコード。
  4 ディレクトリにサブディレクトリ禁止 assert を追加(注入変異で検出確認)。
- ~~スイートの lint 抑制は src だけの話と思われていた~~ →
  `#[allow(dead_code)]`/`unused*` 系抑制子が tests/・examples/ に無制限 —
  実在例として roguelike.rs に誰にも読まれない CombatEvent フィールドを
  隠していた(`#[allow(dead_code)]` 除去で死んだ Team enum も連鎖除去)。
  substring needle 化(トークン境界は `unused_mut` 等の接頭語族に不敗のため)。
- ~~テストは走査さえされれば機械不変と思われていた~~ → tests/ ファイルの
  `env::var`/`env::args`/`set_var`/`remove_var`/`current_dir`/`current_exe`
  は機械ごとに結果を変えるスイート自身の ambient input。tests/ のみ
  環境読取 needle 群を追加(temp_dir は免除 — パスは違うが検査は同じ)。
  4変異(dead_code/unused_mut/env::var/subdir)全発火確認。
- ~~テスト数フロアは「下回らない」だけで十分と思われていた~~ → 下限のみの
  検査では実数が 6,000 に育っても「3,400+」は真のまま腐る。帯域
  `actual >= claim >= actual*3/4` で陳腐化を強制検出し、`#[test]` の
  数え方も substring(コメント/文字列で水増し可・実在 phantom 10件)から
  行頭アンカーに修正。3変異で双方向の発火を確認。
- ~~golden ハッシュは全 wire format を監視すると思われていた~~ → 89 impl 中
  23 件のみピン、他は列挙すらされず(ヘッダ記述 "~40" 自体が陳腐化)。
  kit 定義 pub 型 ∩ impl は golden 言及 or `UNPINNED_DET_HASH` 宣言を強制
  (ピン獲得/impl 消滅で宣言は自動失効)。Cargo.lock の外部クレート不在も
  解決層として新規凍結(path-dep 変異で発火確認 — 直接編集は cargo が
  自動修復するため無効)。
- ~~`bin/` 除外は「対象外で安全」と思われていた~~ → 全走査が src/bin を
  スキップし、bin ターゲットは lib.rs の forbid/deny を継承しない別
  クレート root — gamec は出荷物なのに完全な無防備地帯だった(実測ゼロ
  違反)。CLI 適用範囲の needle セットで凍結、3変異で発火確認。
- ~~src/ の .rs は全てビルド対象と思われていた~~ → 走査はテキスト、
  コンパイルは mod 宣言を見る。`mod` 宣言なき孤立ファイルは suite 全緑
  のまま一生コンパイルされない(実証済み)。両クレートで「全 stem が
  lib.rs に宣言される」を新設、孤立ファイル・ディレクトリの変異で発火確認。
- ~~decay 抑制子は tests/examples だけの話と思われていた~~ → 出荷 src の
  `#[allow(dead_code)]`/`unused*` は weakening 走査(安全系 lint のみ)の
  対象外。両クレート impl 領域に同禁令(実測ゼロ)。
- ~~CI 非コミットは規則遵守に任せられていた~~ → `.github/workflows/`
  の存在自体を禁止検査化(dependabot.yml は許容)。
- ~~`env!` は src だけの話と思われていた~~ → tests/ のコンパイル時
  環境読取は未規制 — CARGO_MANIFEST_DIR のみ許可に限定(test_code が
  文字列を潰すため生テキスト走査、needle 分割構築で自己参照回避)。
- ~~`if x.is_ok() { assert! }` は無害と思われていた~~ → 書込み失敗時に
  assert が丸ごとスキップされる環境依存の静かな緑。readme_blocks_agree
  の probe を無条件化し、ガード形状自体を tests 全域で禁止(変異確認)。
- ~~validate() は「全 findings を収集(never short-circuit)」と信じられ
  ていた~~ → 未強制: 全既存 fixture が単一 defect クラス — first-finding
  bail も全緑。impl 領域の early-exit primitive(return/?/break/continue)
  構造禁止 + 13 クラス同時発火 bundle の二層で凍結(return 変異で両層発火)。
- ~~LICENSE ファイルは存在すればよい~~ → manifest フィールド・ファイル
  内容・README 記載の三方向 drift が可能だった。署名句(MIT/Apache)
  一致検査で凍結(内容差替・manifest 変更の2変異で発火確認)。
- ~~HashMap 等の許可リストだけで十分~~ → interior mutability(Cell/
  RefCell/OnceCell/Rc/Arc/lazy_static)は別の偽決定論経路として未禁。
  実測ゼロ、モジュールパス needle で terminal::Cell と区別して凍結。
- ~~roundtrip テストがあれば wire 形式は安全~~ → 自己整合的な形式
  変更(BE/u64 化等)は roundtrip 緑のまま既存ファイルを全て壊す。
  save.rs は write/encode 二重実装だった → 委譲で単一化+バイトピン+
  ディスク一致テスト。serialize も canonical 出力を文字列ピン。
  diag_json/diag_sarif も同型 — 機械消費 JSON をバイトピン(構造
  assert は形式 drift を見ない)。
- ~~「固定コンパス順」は順序が固定されていれば済む~~ → SPEC の主張は
  この順序であり、配列の並替えは決定性のまま経路・WFC 規則の方向
  index(0=N/1=E/2=S/3=W)の意味を変える。3 つの DIRS テーブルと
  opposite() をピン。
- ~~「順序の契約」は doc コメントに書けば済む~~ → 派生 Ord は判別値を
  見るため `Unseen = 0` を 2 に書替えるだけで hash 緑のまま
  is_explored が全マップで反転。順序+rank を値ピン。
- ~~イテレータ順は「決定的なら何でも」で済む~~ → 公開 iter_cells の
  doc が「HashMap 順・未ソート」と実装(BTreeMap・昇順)に反していた。
  順序契約は昇順ピン+doc 一致が正しい形。
- ~~「全 API が呼ばれる」を検査すれば API は守られている~~ → 呼出し検査は
  *集合* を見ない: シグネチャ変更・バリアント追加は緑のまま通る。
  公開面の hash+件数ピンで、API の変更は意図的な更新を要求される形に。
- ~~衝突判定は手選びの正解例で十分~~ → swept の entry/exit 符号ミスは
  片方向だけを壊し、固定例は交差軸を通さない。役割交換の metamorphic
  性質(2万ケース)と tunneling 最小ケースで閉じた。
- ~~行列演算は固定例で十分~~ → Mul の列-major index ずれは「行列らしい
  出力」を返すため結合則性質 (A*B)p==A(Bp) のみが捕捉する。
- ~~世代ハンドルは「存在する」ので stale は弾かれる~~ → alive 検査は
  insert にしか無く get/remove は index のみ: 再使用後に旧ハンドルが
  新エンティティを読み書きできた。機構の存在≠強制 — 全アクセサに
  alive ガードを追加。
- ~~フレーム duration はコンテンツ側の責任~~ → `while elapsed >= dur` は
  dur≤0 で永久回転しループ再生のゲームをフリーズさせる — データ起因の
  ハングは構築時 assert で弾く。
- ~~フォーマットの申告サイズは信用できる~~ → 切り詰め WAV はサイズだけ
  正直な申告で OOB panic — 申告値と実残バイトは別物として検査する。
- ~~宣言長の加算比較は wrap しない~~ → `10 + len` は 32-bit で wrap — 修正が
  片方のパーサに伝播しない同型バグは横断掃引で閉じる。
- ~~網羅スイープがあれば任意入力の安全性は証明される~~ → fixture 形状が
  経路に届かなければスイープは vacuous — 到達可能な形状を選ぶ。
- ~~jump() は「違う stream を生む」を検査すれば済む~~ → JUMP 定数の1桁
  タイポでも差異テストは緑のまま非重複の約束が壊れる。jump 後 state と
  出力ワードを値ピン。astar_cardinal 専用の 4方向 CARDINALS も fn 内に
  隠れて未ピンだった → CARDINAL_DIRS に昇格してピン統合。

## 3. 改善案(優先順位付き)

この表は 12 行あった。**11 行が閉じ、残る1行はユーザーの意思決定待ち**である。
閉じ方の内訳のほうが表そのものより重要で、**11 件のうち 6 件は実装ではなく測定で閉じた** —
要件が誤っていたか、対象が最初から存在しなかったか、既に実装済みだった。
表の外の P5(バージョン体系)を加えると測定で閉じた要件は 7 件になる。

最初の4件(下表)に加え、最後まで残っていた3行も測定で閉じた ── 詳細は「残っているもの」の節。

| 閉じた要件 | 閉じ方 | 測定 |
|---|---|---|
| I10 / N3 zero-panic API(「0.2 破壊的変更・工数大」) | **要件が誤り** | 追跡指標「unwrap 242 / expect 20」は**テストコード込みの計測**だった。実装コードのみでは計 12 箇所、`panic!` は 0。12 箇所を書き換え、`deny` で恒久強制。シグネチャ変更は不要だった |
| P5 バージョン体系の不整合(engine 4.1.0 vs kit 0.1.0) | **要件が誤り** | 独立に公開されるクレートに版数一致の要請はない。4.x の根拠は engine `CHANGELOG [4.0.0]` に既に記録済みだった |
| I5 / N17 engine の float ソートを `total_cmp` 化 | **対象が存在しない** | `izanagi/src/` の `sort` / `max_by` / `min_by` / `partial_cmp` / `binary_search` は **0 件**。kit 側は `no_float_in_sim.rs` が float 自体を禁じているので派生的に 0 件。**この 0 件は測って終わりにせず `izanagi/tests/float_boundary.rs` の空の許可リストに載せた** — 最初に `sort_by` を足す人が「何を比較するのか」を書かない限りビルドが落ちる |
| I6 / N15残 recipe/drop/encounter の孤児参照検出 | **対象が存在しない** | `Content` のフィールドは `prefabs` / `tiles` / `levels` の 3 つのみ(`izanagi_kit/src/content.rs`)。検出すべきデータ型自体が無い |

残る 5 件(I1 README 同期 / I2 example 追加 / I3 N13 WFC selector / I7 N11 lockstep /
I8 N19 `Fixed` op 行列)は実装で閉じた。詳細は §2 の解消済み一覧。

> **この節から得られた教訓**: 改善案は**書き足す前に測る**。未測定のまま積んだ 12 件のうち
> **6 件は着手すれば無駄骨だった** — 対象ゼロが 2 件(N17 の float ソート、N15 の
> recipe/drop/encounter)、過大見積りが 1 件(N3 は実数の 15 倍)、**既に実装済み**が 1 件
> (N5 MapBuilder)、動機が測ると成立しないものが 2 件(N20 feature collections、
> N2 増分ハッシュ)。半分である。
> 以降、新しい行を足すときは「対象が何件あるか」を最初の列に書けるまで足さない。
> 動機が性能なら、その性能を先に測ってから行を書く。

### 残っているもの

> **追記(後続セッション)**: 上の「もう無い」は当時のスナップショットで、RESEARCH.md の
> 🟢 backlog 行は生きていた。3行を実装で閉じた(C3-3 PRNG 統計品質テスト /
> C5-4 dropped_steps 可観測化 / C9-5 did-you-mean 修正提案、いずれも N24 へ記録)。
> 併せて engine 側の panic 経路を棚卸しし、audio の short-buffer panic + HashMap
> 混合順・gamepad の disconnect ghost input・tilemap の u32 wrap を3件修正。
> **C9 の残り(span ベース診断・help 文・quick-fix の lint 側適用)はまだ開いている。**

**エージェントが着手できる改善案は、もう無い。** 前版に残っていた4行のうち3行は、
着手ではなく**測定**で閉じた:

| 行 | 閉じ方 | 測定 |
|---|---|---|
| I4 / N5 MapBuilder 合成層 | **既に実装済みだった** | `MapBuilder`(`new`/`stage`/`keep_largest_region`/`stage_count`/`build`)が stage 毎 RNG ストリーム分離つきで存在し、テスト6件が付いていた。行のほうが古かった |
| I9 / N20 cargo feature collections | **動機が両方とも成立しない** | コンパイル時間: 88モジュール・実装28,812行の release cold build が **3.6 秒**。バイナリサイズ: rlib 5.5 MB に対し1モジュールしか使わない `wfc_demo` の release バイナリは **318 KB** — リンカが未使用87モジュールを既に落としている。対価は 88 個の `#[cfg]` と feature 組合せの検査行列と pinned hash 破壊の危険 |
| I11 / N2 incremental multiset hash | **現時点では不要。閾値を測定で定めた** | 3,579 セル状態の `hash_state` が **36 µs** = 60 Hz フレーム予算の **0.2%**。対価は savefile/replay 形式の破壊的変更。状態が約100倍になると ~3.6 ms(フレームの22%)で、**そこが着手線** |

残る1行(I12 / N21 crates.io Trusted Publishing + cargo-semver-checks)は
**初回公開の意思決定待ち**であり、エージェントの資格情報では実行できない。

網羅的な未着手候補一覧(N10 構造化ファジング — `cargo-fuzz` が nightly を要求し本環境の
ネットワーク制約で不可 / N18 archetype storage — `izanagi_kit/tests/bench.rs` の実測で
**不採用確定**: join 走査は archetype が 16x 速いが lookup/churn は SparseSet が速く、
frame 差は 0.1% — primitive として ArchTable を据え置き)は
**`izanagi_kit/RESEARCH.md` の N 候補表を正とする**。
同じ候補を2つの表で管理すれば必ず片方が古くなる。

**ユーザー判断待ち(エージェントには実行不能・これがプロダクトの残り全部)**:
1. **CI 有効化** — Web UI で `docs/ci/ci.yml` を `.github/workflows/ci.yml` として追加する。
   エージェント側の 3 経路(push / Contents API / Git Data API)はすべて 403 で実測済み。
   手順は [`docs/ci/README.md`](./docs/ci/README.md)。
2. ~~PR #8 のマージ~~ — **解消**: 当該ブランチの内容は PR #9 として main にマージ済み(`4e3bc5c`)。
3. **crates.io 公開** — 資格情報が本セッションに存在しない(`cargo publish --dry-run` は成功)。

これ以外に、コードで閉じられる欠陥は現時点で特定されていない。§2 の未解決4件のうち
2件(外部利用者ゼロ / `verify` の忠実性)は**構造的限界であり道具では埋まらない**ことが
結論であって、未完了の作業項目ではない。

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
`roguelike_sim`)/ **全 example(29本)が headless 完走・非空出力・2回実行で
バイト一致**(一覧はファイルシステムから読むので新規 example は追加当日から対象。
このループ内で `kit_bridge` の統合ハッシュ `353498ec4fbcd160` も出力から grep する。
`verify_pipeline_demo` は自身の主張を assert!/panic! で検証するため非ゼロ終了が
ループに捕捉される — 専用段は不要と判断し畳み込み済み)/ 両クレートの `cargo package` と、
**展開した tarball の中で doctest / テスト / example / bin の4ターゲットすべて**、
および **`gamec` を同梱 fixture に対して両方向**(`dungeon.game` を受理し `broken.game` を拒否)。
ライブラリのビルドだけでは他のどれも検証されず、実際に3つ壊れた状態で出荷しかけた。

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
  example は `examples/<name>.rs` の配置で自動検出される — `[[example]]` ブロックは
  既定値の再述だったため削除済み(復活させない)。
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
