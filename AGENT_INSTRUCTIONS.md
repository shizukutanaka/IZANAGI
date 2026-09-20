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
| kit モジュール数 | **89**(`izanagi_kit/src/*.rs`。`tests/docs_are_current.rs` が検証)|
| engine モジュール数 | **25**(`izanagi/src/*.rs`。同上)|
| 決定論 pinned hash | `PINNED_FINAL_HASH=0xd1a9236e96a2c802` / `PINNED_ROGUELIKE_HASH=0x5286d1420200fe66`(不変) |
| kit_bridge 統合ハッシュ | `353498ec4fbcd160`(headless == engine-hosted) |
| panic 経路(実装コード) | `panic!` **0**・assert 系マクロは**命名済み15サイトに凍結**(両クレートで `clippy::unwrap_used`/`expect_used`/`panic` を `deny`。弱め属性・新規 panic マクロはテストが落とす) |
| 出荷可能性 | `cargo package` 両クレート成功(`--no-verify` なし)。さらに**展開した tarball の中で
doctest・テスト・example・bin の4ターゲットが緑**で、さらに `gamec` が同梱 fixture を
正しく受理・拒否することまで確認する — 同梱した「証拠」が消費者の手元で実際に走る |
| 機械検査された文書主張 | tier 表・README モジュール表・pinned hash・モジュール数・engine 版数・f32 境界・README のテスト数下限・README の Quickstart(doctest として実行)・engine CLAUDE.md の Map・全 md の相対リンク・**非 float 非決定論ソースの許可リスト**(`HashMap`/壁時計/スレッド/アドレス依存)・**engine の順序づけ 0 件**(float 比較ソートの不在)・**能力マップが検証系12モジュールを名指しすること**・**SPEC.md の G1〜G12 が強制場所を持つこと**(zero-dep / `forbid(unsafe_code)` / edition / MSRV 宣言 / 条件コンパイル判別式の禁止を含む)・**凍結した「本イテレーション」記述の不在**・**`.game` 文法とパーサの一致**(キーワード9種・行長1024・名前32・寸法256)・**ARCHITECTURE.md の file map と Engine の公開フィールド数**・**3つの README の Rust ブロックが doctest として実行されること**・**`.gitattributes` がテキストを LF に固定していること**・**manifest に `target`/`features`/`lints`/`patch`/`replace`/`build-dependencies` テーブルがなく `.cargo` 設定ファイルが存在しないこと**・**`allow`/`expect`/`warn`/`force_warn` が安全系 lint(`unsafe_code`/`unwrap_used`/`expect_used`/`panic`)を弱めないこと**・**SPEC が名指す強制場所のファイルが実在すること**・**panic 系マクロ(assert*/debug_assert*/unreachable!/todo!/unimplemented!)が命名済みサイトに凍結されていること**・**出荷コードが `#[path]`/`include!`/`include_bytes!` で走査領域外から混入しないこと**・**kit が fs/process/io の環境入力を読まないこと**・**gate.sh が全ステージを含むこと(ゲート自身の検査)**・**manifest に [[bin]]/[[test]]/[[bench]]/[[example]] テーブルと harness/auto*/crate-type/proc-macro/test/bench/doctest キーがなく [profile.*] の意味論キー(debug-assertions/overflow-checks/panic)が未設定であること**・**rust-toolchain/Cross.toml/clippy.toml が存在せず rustfmt 設定に逃走経路キーがないこと**・**[workspace.dependencies] の不在**・**出荷コードに env!/option_env!/extern/#[no_mangle]/#[link がないこと**・**kit が std::arch/core::arch を使わないこと**・**gate.sh が RUSTFLAGS/RUSTDOCFLAGS を除去すること**・**tests/ と examples/ が #[cfg]/#![cfg]/cfg!/cfg_attr/unsafe/#[ignore] (命名済み allowlist のみ・陳腐化検査つき)を持たず全テストファイルが #[test] を含むこと**・**md のフェンスに ignore/no_run/compile_fail/should_panic がないこと**・**.githooks/pre-push と docs/ci/ci.yml が tools/gate.sh を呼ぶこと**・**kit が panic 機構(catch_unwind/panic::)を持たないこと**・**走査配下(src/tests/examples)に symlink がないこと**・**重複スキャナ helper(test_module_boundary/library_sources/contains_token/count_token/kit_src/take_balanced)が全コピーでバイト同一であること**・**AGENT が core.hooksPath の設定を指示し続けること**・**src 内 doc フェンスに ignore/compile_fail/should_panic がなく no_run は命名済み allowlist(陳腐化検査つき)のみであること**・**共有境界 lexer が偽 marker を拒否し真 marker を受理すること(合成入力検査)**・**kit に同期プリミティブ(Mutex/RwLock/channel/atomic/thread)と生ポインタ経路(as */*const/*mut/.as_mut_ptr)がないこと**・**gate.sh が CARGO*/RUST*/RUSTUP* 系環境変数を全除去し CARGO_HOME を target/ 内に固定すること**・**`cfg_attr` の適用側にネストした cfg/cfg!/cfg_attr も同じ許可リストで走査されること**・**`[lib]`/`[bin]`/`[test]`/`[bench]`/`[example]` の path 差し替えがなく `links` キーがないこと**・**src の doc フェンスと `include_str!` が辿る md の Rust フェンス本体が禁則トークン(unsafe/cfg/env!/include! 等)を持たないこと**・**`allow`/`expect`/`warn`/`force_warn` が lint グループ名(warnings/all/clippy::restriction 等19種)でも安全系 lint を弱めないこと**・**engine の src に unordered container(HashMap/HashSet/hashers)が一切ないこと(G12)**・**`#[rustfmt::skip]` が命名済み allowlist(1箇所・陳腐化検査つき)に凍結されていること**・**library code に `usize::BITS`/`isize::BITS` がないこと**|
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

- ~~doc コメントの ` ``` ` フェンスは走査が効く(フェンス検査テストがある)~~ →
  `BANNED_IN_FENCE` はコンパイル系(unsafe/cfg/include/env!/extern/asm)のみで、
  **実行時の ambient input 族が抜けていた**: doctest は `cargo test` の検証済み
  ビルド内で走る別クレートなので、フェンス内の `std::net::TcpStream::connect` や
  `std::process::Command` は「全スキャナーが見ないが確実に実行される」コードだった。
  lib 側の BANNED と同族を追加: net/process/Command/env::var・set_var・remove_var/
  fs::・File::・OpenOptions/io/Instant/SystemTime/thread/sync/catch_unwind/panic::/
  set_hook・take_hook/should_panic/arch/TypeId/type_name/layout(offset_of・addr_of・
  size_of・align_of)/ポインタ全族/NonNull/ptr_eq/HashMap・HashSet/RandomState/
  DefaultHasher/`#[allow`・`#[warn`・`#[expect`(doctest クレート内での lint 弱化)、
  および `#[path`/`mod `(doctest から任意ファイルへの splice — `#[path` 抜けていた)。
  注入で実証: doc フェンス内の `TcpStream` と `#[path] mod` がともに失敗。
- ~~example の2回実行バイト一致を通るなら出力は「バイナリ自身の計算」~~ →
  **FS は gate 実行間の暗黙チャネル**だった: run 1 で書いたバイトを run 2 が読めば
  比較は一致しつつ、実際には走査されない機器上のバイトを読んでいる。examples では
  `fs::`/`File::`/`OpenOptions`/`read_to_string`/`read_dir`/`canonicalize` を禁止
  (tests は据え置き — リポジトリを読むのが scanner の本筋)。併せて examples の
  `#[allow]`/`#[warn]`/`#[expect]`(および `#![...]` 内側綴り)を (file,count,reason)
  の `EXAMPLE_ALLOW_ALLOWLIST` で等価一致に凍結 — 新規追加も既存削除(=古い許可の
  亡霊化)も失敗する。注入で実証: example に `fs::read_to_string` を入れると失敗、
  `#[allow]` 追加・`#![warn]` 追加・platformer の既存 `#[allow]` 削除の3方向とも失敗。
- ~~ambient input 拒否リストが時計・FS・I/O・process・arch・ポインタを網羅しているなら
  「機器に依存する入力は全部塞がれている」~~ → ネットワークスタックだけ抜けていた:
  sim コードが `std::net::TcpStream::connect` でソケットを開けば、入力ログも replay も
  再構成できないバイトが直接 sim に入り、二度の正直な実行は別のバイトを読む。
  `BANNED` に `std::net`/`net::`/`TcpStream`/`TcpListener`/`UdpSocket`/`ToSocketAddrs` を追加。
  同じ「検査対象の計算を走査外へ委譲する」族が tests/examples にもあった —
  `Command::new`/`process::Command` で任意のサブプロセス、`std::net` 系でソケット、
  `env::set_var`/`env::remove_var` で他ニードルが取り締まる ambient input 自体の書き換え
  (全対象で実使用ゼロ)。注入で実証: kit src の `TcpStream` は境界より前の実コード位置で
  `library_code_has_no_unstable_inputs` を落とし、tests の `Command::new`・
  examples の `UdpSocket`・tests の `env::set_var` はいずれも
  `the_verification_suite_cannot_quietly_skip_or_disable_its_own_checks` を落とす。
- ~~`library_sources` が `src/` を再帰的に歩くなら出荷バイナリも走査されている~~ →
  `src/bin/` は意図的に除外されている(「CLI はその機械に答える」)が、
  `src/bin/*.rs` は**それ自身がクレートルート**なので lib の
  `#![forbid(unsafe_code)]` は `gamec` に届かず、ファイルは誰にも読まれていなかった:
  `unsafe`、走査外の `src/bin/<name>/mod.rs` への `mod`(cargo が bin に自動昇格するのは
  `src/bin/*.rs` と `src/bin/*/main.rs` だけで `mod.rs` は拾わない)、`env!("SECRET")`
  がすべて無検査で入れた。`g7_the_safety_denies_are_present_and_nothing_weakens_them`
  は両クレートの `src/bin/*.rs` を読むようになり: 各ファイル自身の
  `#![forbid(unsafe_code)]` を要求し(1回剥がしてから検査 — 属性自身の `unsafe_code`
  テキストがニードルを満たさないように)、`unsafe`/`include!`/`include_bytes!`/`#[path`/`mod `
  を拒否し、`env!` の引数を `CARGO_*` に限定し、`izanagi_kit/src/bin` が空でないことを
  主張する(`gamec` の削除が真空で通らないように)。注入で実証: `forbid` 下の unsafe は
  そもそもコンパイルエラー、`mod sub` + `src/bin/sub/mod.rs` はコンパイルするが
  `mod ` ニードルが検出、`env!("HOME")` と forbid 行の削除もともに失敗。
  さらに同じ段で **`src/bin/*/main.rs` 経路**: cargo は `src/bin/probe/main.rs` を
  第2の bin ターゲットとして自動発見する — 直下 `*.rs` だけ読む初版はこれを
  逃がしたので再帰 walk に直し(ネストした `mod` 到達ファイルも一緒に読む)、
  `probe/main.rs` 無禁止注入が失敗することを実証。
- ~~`#[cfg(test)]` 以降はテスト領域なのでスキャナーが切り捨ててよい~~ →
  切り捨てが正しいのは「境界がファイル最後の item を開く時」だけで、`mod tests`
  の *後* に書かれた `pub fn`/`impl` はコンパイルも clippy も通るが全スキャナーが
  読まない(global_invariants_hold::nothing_compiles_after_the_test_module_boundary が
  境界後の構造テキストを `mod <ident> {…}` のみと機械強制 — `mod` 後にコードを
  置く注入と、マーカーが `mod` 以外を開く注入はともに失敗することを実証)。
- ~~`env::var` 等の「パス形」ニードルは banned path をそのまま綴ったコードを
  捕捉する~~ → `use std::env::{var}`(ブレース輸入)や `use std::env as e`
  (エイリアス輸入)は banned path を一度も綴らずに同じ呼び出しをコンパイルする
  — `env::var` ニードルを完全に素通りする(subprocess の `Command` も
  `use std::process::{Command as C}` で同型)。flattened_use_paths が `use`
  文のブレースを展開して `a::b` 群を走査対象に復元し(3ファイルに byte 同一
  複製+GROUPS 登録)、`std::env`/`std::panic` のモジュール根ニードルと
  `env as`/`thread as`/`process as` エイリアスニードルで閉塞 — src 注入
  (env/panic の alias)、fence 注入、suite 注入(brace/alias/入れ子)は
  すべて失敗することを実証。
- ~~`env!` 系だけが環境混入経路~~ → `file!`/`line!`/`column!`/`module_path!`
  はビルドマシンのチェックアウトパスと編集位置をバイナリに焼き込むのに
  一切 unscanned — 同一マシン2回実行のバイト一致では絶対に見えない
  (src と example への注入で実証)。src 側の ambient 針と tests/examples
  の suite 針の両方に4マクロを追加し、全6注入形態で検知を実証。
- ~~`#[cfg(test)]` 境界以降はテスト領域なのでスキャン不要~~ → 全スキャナーが
  境界で切り捨てるため `mod tests` 内の `#[cfg(unix)] fn 検査()` は
  プラットフォームで黙って消えてもゲート緑のまま(注入実証)。
  tests/examples ディレクトリにあった `#[cfg`/`cfg!`/`cfg_attr`/`#[ignore`
  禁則を境界以降の構造テキストにも適用し、`#[ignore]` は
  SRC_IGNORE_ALLOWLIST(savefile.rs の print_golden_save)に名指しで
  許可 — 4形態の注入で双方の挙動を実証。
- ~~panic 系マクロは「ファイル毎の個数」を凍結すれば十分~~ → `debug_assert!` を
  `assert!` に入れ替えるだけで「debug限定の監査 assert」が「出荷パニック」に
  昇格するのに個数は変わらずゲートを通る(注入実証)。凍結対象を個数から
  「マクロ名+呼び出し先頭40文字」のサイトキー多重集合に強化 —
  昇格・メッセージ変更・削除・追加の4形態で検知を実証
  (g7_panicking_macros_are_frozen_at_named_sites)。
- ~~`cfg_attr` の適用対象は条件コンパイルだけ~~ → `push_applied_cfg` が
  cfg 系のみ再帰表面化していたため、`#[cfg_attr(test, allow(dead_code))]`
  は `#[allow` needle を一度も綴らずに「テストビルドだけ lint が緩む」
  非対称を作れた(注入実証)。非 cfg 適用項目を `cfg_attr_apply`
  疑似述語(外側述語\0項目)として通知し、許可は `not(test)` 下の
  `deny`/`forbid`(crate root の clippy ゲート)と `doc` 項目のみ —
  `test`/`doc`/`docsrs` 下の `allow`/`deny`/`repr`/`no_mangle` 等は
  全て offender(4形態の注入で双方の挙動を実証)。合成テストは
  疑似エントリを除外してカウント。
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
- ~~gate.sh の環境除去は RUSTFLAGS/RUSTDOCFLAGS の2変数で十分と思われていた~~ →
  cargo/rustc が読む同等の注入経路は他にも存在する: `RUSTC_BOOTSTRAP=1` は stable で
  `#![feature]` を通し、`CARGO_ENCODED_RUSTFLAGS`/`CARGO_BUILD_RUSTFLAGS`/`RUSTDOCFLAGS` は
  `--cap-lints=allow` を全ステージに差し込め、`RUSTUP_TOOLCHAIN` は toolchain 自体を
  差し替え、`CARGO_HOME` は呼び出し側のキャッシュを指す。**`RUSTC_BOOTSTRAP=1` で
  `#![feature]` が実際にコンパイルされること、`--cap-lints` で警告数が 0 になることを
  注入で確認**。gate は冒頭で `CARGO*`/`RUST*`/`RUSTUP*` 系の全環境変数を除去し
  `CARGO_HOME` をリポジトリ内 `target/` に固定する。
- ~~`cfg_attr` は判別式だけ見れば足りると思われていた~~ → `#[cfg_attr(test, cfg(unix))]` の
  ように**適用側にネストした `cfg`** はスキャナを完全にすり抜けた(初回注入は
  `#[cfg(test)]` 境界の後に置かれ「発見なし」は偽陰性 — 境界の前に置き直して本物の
  盲点と確認)。適用側アイテムを再帰走査する `push_applied_cfg` を追加し
  `cfg_attr(any(), cfg!(...))` まで捕捉する単体テストで固定。
- ~~manifest 文法はテーブルとキーを閉じれば足りると思われていた~~ → **`[lib]` の
  `path` 差し替え**が残っており、kit がスキャン済み `src/` だけを見せる再エクスポート
  shim でライブラリ本体を走査外に置けることを実注入で確認(スキャナは src/ のみ読む)。
  `path` を `src/lib.rs` 固定に、`links` キーを禁止に。
- ~~doctest フェンスはタグだけ見れば足りると思われていた~~ → フェンスの**本体**は
  一度も走査されておらず、`//! ``` ` の内側に `unsafe`/`env!`/`include!` 等を書けば
  出荷される doctest としてコンパイルされるのに全ソース検査が素通りしていた。
  `docs_are_current.rs` に `rust_fence_bodies` を追加し、`include_str!` で辿れる md の
  Rust フェンスも含め本体の禁則トークンを検査(注入で検出確認)。
- ~~安全系 lint の弱め属性は名前指しで塞いだと思われていた~~ → `#[allow(clippy::restriction)]`
  等の **lint グループ** allow は個別名を含まないため検査を素通りし、内側スコープの
  group allow が outer `deny(clippy::unwrap_used)` を無効化することを注入で確認。
  19 の lint グループ名を `weakening_lint_args` の検査対象に追加。
- ~~engine の「順序づけ 0 件」で unordered container は尽きたと思われていた~~ →
  `audio.rs` の `voices: HashMap<Voice, Playing>` を `mix_into` が反復し **f32 `+=` fold**
  に掛けていた。f32 加算は非結合なので hasher seed 由来の反復順がプロセス毎に出力を
  bit ずらし得る。ecs.rs が自明視した「despawn でしか反復しない例外」は
  `clips`/`bytes`/`names`/`HashSet<Button>`/`HashSet<Key>` でも残っていた。全て
  `BTreeMap`/`BTreeSet` に置換(`Voice`/`Handle`/`Button`/`Key` に `Ord`)、
  **`izanagi/tests/no_unordered_containers.rs` が型の不在を強制** — 例外管理から
  「存在しないので書けない」へ。SPEC.md に **G12** として明記(注入で検出確認)。
- ~~`cargo fmt --check` があれば全コードは整形済みと信じられていた~~ → `#[rustfmt::skip]` は
  fmt の検査を通ったまま任意の未整形ブロックを置ける escape hatch(実測で正当利用は
  autotile_demo.rs の ASCII マップ1箇所のみ)。`test_code` で文字列・コメントを除去し
  全6ディレクトリを走査、命名済み allowlist に凍結(陳腐化検査つき・注入で検出確認)。
- ~~usize 系の漏れは幅の値だけと思われていた~~ → `usize::BITS`/`isize::BITS` は `usize`
  への言及を含まない定数形式の同じ漏洩。`hashes_are_width_independent.rs` の
  センチネル表に追加(注入で検出確認)。
- ~~スイートは自前のファイルを全て走査していると信じられていた~~ → `mod`/
  `include!`/`#[path]`/`include_bytes!` でフラットな `read_dir` が届かない
  コードをコンパイルでき、`env!`/`option_env!` でビルドマシンをバイト出力に
  焼き込めた。4ディレクトリ(tests+examples)で `include!`/`include_bytes!`/
  `#[path`/`mod ` を禁止、env 引数は文字列認識付きの `env_macro_args` で読み
  テスト側は `CARGO_MANIFEST_DIR` のみ・example は全面禁止、さらに直下以外の
  `.rs` を検査してフラットさを凍結(6種注入全て検出確認)。
- ~~コンパイル時の逃げ道を塞げばスイート改竄は尽きたと思われていた~~ →
  実行時の逃げ道が残っていた: `env::var` でマシン依存スキップ、
  `catch_unwind`/`set_hook`/`take_hook` で panic を飲み込む/再定義、
  detach された `thread::spawn` で panic を JoinHandle ごと捨てる、
  `should_panic` で失敗を緑化、テスト内 `process::exit` でハーネスを
  途中終了、`#[allow]`/`#[warn]`/`#[expect]` で lint を局所ダウングレード。
  共通6ニードル + テスト限定4ニードルを追加(example 側の正当利用 —
  `env::args` の `--terminal` 処理、`process::exit` のエラーパス、
  `#[allow]` 2件 — は温存)。7種注入全て検出確認。
- ~~gate の環境除去は CARGO*/RUST*/RUSTUP* の3族で完璧と思われていた~~ →
  `LD_PRELOAD`/`DYLD_INSERT_LIBRARIES` はフラグではなく rustc への共有
  ライブラリ混入経路で、除去表に無かった。`LD_*`/`DYLD_*` 族を追加
  (注入変数を立てた実行で除去確認)。あわせて「gate の実行がツリーを
  汚さない」を担保するものが無かった → 開始時の `git status --porcelain`
  を記録し最終段で一致を要求(変更混入の注入で検出確認)。
- ~~走査対象は列挙できれば読めると思われていた~~ → スイート内25箇所の
  `fs::read_to_string(...).unwrap_or_default()` は読取失敗を空文字に潰し、
  「〜を含まない」系検査がvacuousに緑化する経路だった。全箇所を
  `unwrap_or_else(|e| panic!(...))` に変換 — 列挙されたファイルが読めな
  いことは対象の不在ではなくスキャンの破損(chmod 000 注入で検出確認)。
- ~~rustfmt.toml の逃げ道は `ignore`/`disable_all_formatting`/`skip_children`
  の3キーで尽きたと思われていた~~ → `format_strings = false` のような
  `format_*` キーは「カテゴリ丸ごと fmt 検査を外す」同じ逃げ道だった。
  `format_macro_bodies`/`format_macro_matchers`/`format_strings`/
  `format_code_in_doc_comments`/`reorder_imports`/`reorder_modules`/
  `reorder_impl_items`/`reorder_use_trees`/`skip_macro_invocations` を
  禁止表に追加(format_strings 注入で検出確認)。
- ~~manifest の逃げ道はキーとセクションを閉じれば尽きたと思われていた~~ →
  `[package] exclude`/`include` は正当利用されているが、エントリを1つ
  増やすだけで tarball 検証の対象(src/tests/examples)を縮小できた。
  許可リストを双方向照合で凍結 — 追加も除去も失敗(3種注入で検出確認)。
- ~~`#[allow]` を禁じれば lint 弱化は尽きたと思われていた~~ → 内側属性
  `#![allow]`/`#![warn]`/`#![expect]`/`#![feature` は `#[allow` のニードル
  をすり抜けてファイル全体の lint を弱められた(注入で再現→検出確認)。

- ~~doc コメントは `///` と `//!` で尽きたと思われていた~~ → `#[doc = "..."]`
  属性リテラルはフェンス走査の視界外で、3行の `#[doc]` で ```rust フェンスを
  綴り `std::env::var` を走らせても docs_are_current は緑だった(注入実証)。
  `#[doc]` の許可形を `= include_str!(`(README 配線)と `(...)`(フラグ形)に
  限定。`#[doc = concat!(...)]` の分割合成も拒否(検出確認)。


- ~~tests/examples の環境読みは `env::var` を禁じれば尽きたと思われていた~~ →
  ident 境界により `env::var_os` が `env::var` ニードルをすり抜け、
  `env::vars`/`current_dir`/`temp_dir`/`current_exe`/`home_dir` はそもそも
  未掲載(注入で var_os/current_dir の両方が緑のまま通ることを実証)。
  `env::` の後続 ident をホワイトリスト化 — `args`/`args_os`(argv は
  gate ラン間で同一)と `temp_dir`(スクラッチ保存・値非観測)のみ許可。


- ~~tests/examples のポインタ禁止は src と同じ表が効いていると思われていた~~ →
  src のポインタ表(no_nondeterminism_in_sim.rs)は library_sources にだけ
  適用され、tests/examples は `as *`/`*const`/`*mut`/`.as_ptr(`/
  `into_raw(`/ポインタ書式フラグを全て通した(`&x as *const u8 as usize` を
  テストと example の両方に注入して緑を実証)。スイート走査にもポインタ
  ニードルを追加 — 書式フラグは文字列内に潜むため raw ソース走査で。
- ~~`#[link]` 系の宣言は `#[link` ニードルで尽きると思われていた~~ →
  `#[export_name]`/`#[link_section]`/`#[used]` は検査をすり抜けるが
  `forbid(unsafe_code)` が unsafe 属性としてコンパイルを拒否(注入で
  実証→コンパイル段で失敗)。検査不要と確認 — コンパイラが塞いでいた。


- ~~`as *`/`*const` を禁じればポインタ混入は尽きたと思われていた~~ →
  `std::ptr::addr_of!` マクロは `*const` を本文に持たず生ポインタを作れ、
  `Arc::ptr_eq` はアドレスを直接比較し、`x as fn()` 変換した fn ポインタを
  `{:?}` で出せばアドレスが印字できる(全注入で緑を実証)。
  `std::ptr`/`ptr::`/`addr_of`/`ptr_eq`/`as fn` をスイート走査に追加。


- ~~workspace 形状は [dependencies] と target テーブルを閉じれば尽きたと
  思われていた~~ → `[workspace]` の `members` キーが未凍結で、新規メンバー
  クレートを列挙に足すだけで全スキャンが読まないコード束が
  `cargo test --workspace` の対象に加わった(`_sneak` クレートを作成し
  `env::var` を持たせて注入→全検査緑を実証)。`[workspace]` セクションの
  キーは `members`/`resolver` の2つに限定、`members` の値自体を
  `["izanagi","izanagi_kit"]` に固定(追加・除去・並べ替え全て検出確認)。


- ~~dev-dependency は `path =` の形があれば安全と思われていた~~ → `path` の
  値は未検証で、別名パッケージ+同名 `[lib]` のコピークレートへ差し替え
  (`package = "_fakekit"` 指定)がコンパイルも全検査も緑で通過 — kit_bridge は
  走査外クレートをビルドしていた(注入実証)。dev-dependency 行を
  `izanagi_kit = { path = "../izanagi_kit" }` の文字列に逐字固定
  (リネーム・package キー・パス書き換え全て検出確認)。


- ~~tests/examples の環境混入は env::・ポインタ系を閉じれば尽きたと思われて
  いた~~ → `std::backtrace::Backtrace::capture`/`force_capture` はシンボル名・
  ビルドパス・RUST_BACKTRACE 状態を読む ambient 混入で未禁止だった
  (test・example の両方に注入→緑を実証)。`backtrace`/`Backtrace` をスイート
  ニードルに追加。


- ~~examples の ambient 混入は fs:: を禁じれば尽きたと思われていた~~ →
  壁時計(`Instant`/`SystemTime`/`UNIX_EPOCH`/`std::time`)は未禁止で、
  example が `Instant::now()` の elapsed を印字すれば pin 出力が実行毎に
  変わる(十分速い2ランで誤って byte-match し得る) — 注入で緑を実証。
  examples-only ニードルに追加(tests は bench.rs の timing 検査が正当利用
  のため保持)。


- ~~examples の非決定性は時計・ファイル系を閉じれば尽きたと思われて
  いた~~ → `HashMap`/`HashSet`/`hash_map`/`RandomState`/`DefaultHasher`/
  `SipHasher` は反復順がプロセス乱数シードで変わるのに example で未禁止
  (3形とも注入→緑を実証; pin 出力2連比較は運で一致し得る)。examples-only
  ニードルに追加(tests は自身の HashMap が自分を flake させるだけなので
  保持 — 注入で緑のままを確認)。


- ~~examples/tests の入力混入は環境変数・ファイル系を閉じれば尽きたと
  思われていた~~ → `stdin` は未禁止: cargo test は標準入力を継承するため
  test/example が端末入力という ambient を読める(gate の閉じた stdin では
  空文字を速攻返すので pin 比較は気付けない)。両方に注入→緑を実証し
  shared ニードルに追加。


- ~~ソケット混入は `std::net`/`TcpStream`/`UdpSocket` を禁じれば尽きたと
  思われていた~~ → `std::os::unix::net::UnixStream` 等の unix ドメイン
  ソケットと `os::fd`/`os::unix` の ext trait は `net`/`fs` ニードルに
  引っかからず開ける(example・test 両方に注入→緑を実証)。`std::os`/
  `os::unix`/`os::windows`/`os::fd`/`UnixStream`/`UnixListener`/
  `UnixDatagram` を shared ニードルに追加。


- ~~テストはファイル独立・順序非依存と思われていた~~ → `static` の
  `OnceLock`/`LazyLock`/`OnceCell`/`LazyCell`/`Mutex`/`RwLock`/`atomic::`/
  `Atomic*`/`thread_local` は同一テストバイナリ内でテスト関数を跨ぐ
  隠れ状態で未禁止だった(3形注入→緑を実証)。shared ニードルに追加 —
  「テスト群は独立」の仮定を機械的に担保する層。


- ~~テストの assert! は標準 prelude に忠実と思われていた~~ → **重大**:
  `macro_rules! assert { … => {} }` が prelude マクロをテキストで
  シャドウし、以降の `assert!` が全部空展開される — `assert!(false)` が
  `ok` で通ることを実証(test・example 両方に注入→緑)。`macro_rules`/
  `macro_export`/`macro_use`/`proc_macro`/`proc-macro` を shared ニードル
  に追加 — 「検査条件を述べるのではなく検査器自体を書き換える」経路の
  封鎖。
- 観察記録: gate の packaged-izanagi `cargo test --tests` が一度だけ
  exit 1 で失敗したが、同一条件で 4 連続緑(手動3回+gate1回)。再現せず
  原因不明 — 再発時は tail-30 より前のバイナリ出力を見る必要あり。


- ~~同期系は `Mutex`/`RwLock`/atomic を禁じれば尽きたと思われていた~~ →
  `Once`/`mpsc`/`Condvar`/`Barrier` はスルーで、単体でも跨り状態に使える
  (Once::call_once と mpsc::channel を注入→緑を実証)。`sync::` 一括 +
  個別ニードル(`Once`/`Condvar`/`Barrier`/`mpsc`/`WaitTimeoutResult`)で
  std::sync 全体を封鎖。


- ~~環境読み取りは env::/stdin/std::os を閉じれば尽きたと思われていた~~ →
  `is_terminal()`/`IsTerminal` は「走らせているのが端末かパイプか」を読む
  ambient 分岐で未禁止だった(test・example 両方に注入→緑を実証)。
  shared ニードルに追加。


- ~~MSRV・edition の主張は msrv_is_respected.rs の構文走査が担保すると
  思われていた~~ → 同テストは「コードが post-MSRV 構文を使わない」だけで、
  マニフェスト側の `rust-version`/`edition`/`name` 値は無ピンだった。
  `rust-version = "1.60"` でコンパイル可・スイート緑を実証(静かな主張の
  弱化)。`edition = "2024"`/`2018` は cargo/rustc がロード・借用検査で
  弾く(compiler-closed)、`name` 改名は dev-dep エッジが即壊す
  (cargo-closed)。`[package]` の三鍵をクレート別に値ピンし、
  `default-run` を BANNED_KEYS に追加。


- ~~fs 混入は `fs::`/`File::`/`read_to_string` 等のニードルで尽きたと
  思われていた~~ → ニードルは API 名だけで、絶対パスリテラル自体は
  未走査だった: `"/dev/urandom"` はカーネル entropy、`"/dev/stdin"` は
  stdin ニードルの迂回、`"/etc/…"`/`"/proc/…"` はホスト状態を読む
  (3形注入→緑を実証)。raw 走査で「`"` の直後が `/`+英数字」を禁止。
  正当ヒット `"/src"`(サフィックス整理用リテラル)は記法を変更して解消。


- ~~絶対パス禁止は「`"` の直後 `/`+英数字」の raw 走査で尽きたと
  思われていた~~ → `concat!` はコンパイル時に引数を連結するため、
  `concat!("/", "dev/", "urandom")` のように断片化すればどの
  リテラル単体も先頭スラッシュを綴らず probe をすり抜けた
  (注入→緑を実証)。`"/x"` 形の引数は主 probe が既に拾うため、
  抜けは `"/"` 単独リテラルのみ — `concat!(...)` 内部に限り禁止
  (`join("/")` のような正当使用は温存)。パーサは平衡括弧で
  呼び出し本体を切り出す。


- ~~`build.rs` は禁止済みとコメントに書かれていた~~ → 実際は
  BANNED_KEYS に `build` が無く、しかも cargo は各クレートルートの
  `build.rs` をマニフェスト記述無しで自動発見する。センチネルを書く
  裸の build.rs を注入 → 走査を一切通らず実行された(2形とも実証)。
  `build` を BANNED_KEYS に追加し、さらに `.`/`izanagi`/`izanagi_kit`
  直下の `build.rs` 非存在をアサート — ファイルシステム側も塞ぐ。


- ~~パス禁止は絶対パスで尽きたと思われていた~~ → 先頭 `/` probe は
  相対脱出を見ない: `"../../../../etc/hostname"` は `fs::` を使う正当な
  テスト内でホストファイルを読める(注入→緑を実証)。`..` 2連続セグメント
  を raw 走査で禁止(1段の `../README.md` は文書慣習として温存)。
  副産物: readme_blocks_agree の `starts_with` チェック自体が `"../.."`
  裸リテラルを含んでいたため成分判定に書き換え — ついでに `"../.."`
  トレイリング無しの形も拾うよう強化。


- ~~パス脱出は「`..` 2連続」で尽きたと思われていた~~ →
  `Path::new("..").join("..")` のように裸 `".."` 断片を join で重ねれば
  どのリテラルにも2連続は出ない(注入→緑)。裸 `".."` リテラル自体を
  4ディレクトリで禁止し、検査側の `== ".."` 比較は
  `s.bytes().all(|b| b == b'.')` に書き換え(セルフエスケープ不要に)。
  残差: char 配列等で実行時に `..` を組み立てる形は字句走査では
  原理的に不可避 — eval 不在の Rust でも文字列組立は残る受容残差。


- ~~`process::` は `Command`/`exit` を個別に止めれば足りると思われていた~~ →
  `process::id()` は毎回新しい値を返す skip スイッチであり、
  `abort`/`set_exit_code` は結果自体を書き換える(注入→緑を実証)。
  `process::exit(` 以外の `process::` を全面禁止(2例の正当な
  エラー終了は温存するため部分置換でスコープ)。


- ~~`std::arch` の機能検出は src 側スキャンで塞がれていると思われていた~~ →
  スイート側には無く、`is_aarch64_feature_detected!` で分岐するチェックが
  ホスト依存の skip スイッチになる(注入→緑を実証)。`arch::` と
  `feature_detected` を needle に追加。


- ~~`env::temp_dir` を白名単に残していた(スクラッチファイル用)~~ →
  temp 配下のフラグファイルは gate 実行をまたいで残存し、
  `join(flag).exists()` が「前回実行が仕込む skip スイッチ」になる
  (注入→両回とも緑、TMPDIR に実存を確認)。scratch はワークスペース
  `target/`(gitignore 済み、ツリー内)へ移し、whitelist から除去。


- ~~`".."`リテラルを塞げばパスは tree 内に閉じると思われていた~~ →
  `parent().parent()` はリテラル無しで2段登り(manifest→root→機械)。
  注入→緑を実証。正当使用は全て1段のみなので「式(statement)毎に
  `.parent()` 高々1回」制限で閉塞。複数文に分ける遠回し回避は残存
  として記録。


- ~~concat! 内だけ裸 `"/"` を禁じれば十分と思われていた~~ →
  `PathBuf::from("/")` のような concat! 外の裸スラッシュリテラルで
  絶対パスが組める(注入→緑)。`"`/`/`"` を byte-scan で全面禁止に
  昇格(正当な `join("/")` 区切りは `char::from(b'/')` 構築へ置換)。


- ~~tests は fs:: を保持してよい(リポジトリ読取に必要)と思われていた~~ →
  `fs::metadata(p).modified()/created()/accessed()` はホストFSの
  タイムスタンプを検査へ流し込む(注入→緑)。時計を保持する正当性は
  bench のみ — `metadata(` を共有 needle へ追加(examples は fs:: 全
  禁止で二重防護)。


- ~~`env::args`/`args_os` は全4dirで正当と思われていた~~ → 正当使用は
  examples の `--terminal` のみで、test binary の argv[0] はハーネス
  自身の機械パス(注入→緑)。whitelist を per-dir 化: tests 側は
  `env::` 一切不許可、examples のみ argv 許可。


- ~~`canonicalize` は examples 禁止で十分と思われていた~~ → tests の
  `fs::canonicalize(p)` は symlink 解決後の機械絶対パスを返す
  (/tmp vs /private/tmp)(注入→緑)。共有 needle へ昇格。
- 残存(記録のみ): `read_dir` の返却順・`target/` 配下の gitignore 書込・
  複数文に分けた parent() 登り — トークン走査では到達不能なデータフロー
  依存の回避路。


- ~~fs の機械情報は metadata/clock で尽きたと思われていた~~ →
  `fs::permissions(p).readonly()` は umask/checkout 依存の mode bits
  (注入→緑)。`permissions(` を追加(`set_permissions` は書込側なので
  porcelain sentinel が捕捉)。


- ~~`debug_assert` は src の allowlist 管理で済むと思われていた~~ →
  suite dir では非 allowlist なので pinned-hash テストの release 走査で
  検査が消える(注入→緑)。tests/examples では `debug_assert` 全面禁止
  (src 側は理由付き allowlist を維持)。


- ~~env scrub は CARGO/RUST/LD/DYLD 族で十分と思われていた~~ →
  `GIT_DIR`/`GIT_WORK_TREE` は porcelain sentinel の `git status` を別
  repo へ向け、実ツリーへの書込が不可視になる(decoy 実証: sentinel
  before==after)。GIT[A-Z_]* を scrub 族に追加 — 正規 git 操作に env
  不要のため無害。


- ~~porcelain sentinel は git が正直に読む限り正しいと思われていた~~ →
  ambient ~/.gitconfig の `status.showUntrackedFiles=no` (実在する設定)や
  untracked cache が落とされたファイルを `git status` から隠す(偽HOME
  で実証: `?? file` 消失)。`tree_status()` に `-c` 強制フラグを集約。


- ~~porcelain sentinel は untracked/modified を尽くすと思われていた~~ →
  .gitignore の glob に合う drop (`*.swp`, `.DS_Store`) は porcelain に
  一切出ない。`--ignored=matching` を追加(ignored-dir は `!! dir/` に
  畳むので target churn は不発、glob 一致ファイルは個別に検出)。
  clean clone で gate が作る `!! target/`/`!! Cargo.lock` は除外。
  残存: 既存 ignored-dir 内部への書込 (target/, .temp/, .claude/)。


- ~~`.git` は 2段登りが要るので1段 `..` 許可で届かないと思われていた~~ →
  tests の cwd は package dir、`.` `..` 1つで repo root → `.git` へ即
  到達。`info/exclude` 編集は status に出ない書込で後続の drop を不可視
  化(機序実証済み)。パス成分 `.git` (両端が `"` か `/`)を byte-scan で
  禁止 — `.gitignore`/`.gitattributes` は成分境界で安全に通過。


- ~~ツリー内に secrets を置く経路は ignore と sentinel で見えると思われていた~~ →
  `.env`/`*.secret` は *読み* が問題 — gitignored 機密を test が読み
  assert メッセージ経由で流出可。成分境界(`.env`=左 `"`/`/`、
  `.secret`=サフィックス)で両者をスキャン禁止に。同機構で `.git` も
  既に禁止済み(info/exclude 編集→後続 drop 不可視化を塞ぐ)。
- ~~gate の緑は toolchain 非依存だと思われていた~~ → rustup の
  directory override や別 toolchain で `cargo` が解決されると fmt/
  clippy/`#![feature]` 受理が変わり、検証前提自体が揺れる。gate.sh
  冒頭で `rustc`/`cargo`/`rustfmt` が stable かつ >=1.75(workspace
  MSRV)であることを assert(偽 nightly で拒否確認)。


- ~~`include_str!` の exempt は package-boundary チェックが守ると思われ
  ていた~~ → そのチェックは `..` 相対しか見ず、`"/etc/shells"` を
  kit lib に焼き込む注入が全走査を緑で通過(実証)。両 src で
  `include_str!` を `include_str!("../README.md")` の2箇所にピン —
  その他の include_str!/include_bytes!/絶対・相対の新規 include は
  コンパイルが通っても走査が落とす。


- ~~bin ルートの裸 `#[test]` は `mod` 禁止で実質塞がれていると思われて
  いた~~ → `mod` 無しでコンパイル・実行される(実証: gamec.rs 末尾の
  `#[test]` が `cargo test` で走り、中身の `env::var`/`process::id` は
  bin スコープで合法=未走査の suite コード)。bin 走査に
  `#[test`/`#[cfg(test`/`#![cfg(test` を禁止追加。


- ~~bin ルートは `src/bin/` のみだと思われていた~~ → cargo は
  `<crate>/src/main.rs` も bin target として発見 — そのファイルは
  `src/bin` walk の外で `forbid(unsafe_code)` 必須も `#[test]` 禁止も
  掛からず、`unsafe` を含む main.rs が全走査緑で通った(実証)。
  bin ルート列挙をファイル収集型に変更し `src/main.rs` を両クレートで
  同じ規則で検査。


- ~~src の `#[cfg(test)] mod tests` 内は lib 走査の切り捨てで守られている
  と思われていた~~ → tail 検査は cfg/ignore のみで、モジュール内の
  `env::var`/`fs::read`/`process::id` が注入で全走査緑を通過。tail に
  ambient 系ニードル(env/fs/process/thread/net/std::os/pointer/panic
  機構/atomic/Instant/SystemTime/arch/metadata/canonicalize/
  permissions/include!/env! 等)を適用 — 正当使用(Mutex/channel/
  is_terminal/std::io/thread_local/should_panic)は除外、
  `include_str!("../README.md")` と `env!("CARGO_MANIFEST_DIR")` のみ
  ピン許可。`src/main.rs` を bin ルート列挙に追加する件も同コミット群。

- ~~テストファイルに `#[test]` さえあれば意味ある検査~~ → `fn t() { let _ = setup(); }` は assert 系ゼロで緑を通過する vacuous check を注入実証。require_tests に assert!/assert_eq!/expect(/unwrap(/panic!/matches!/unreachable!/todo! いずれかのトークン≥1 を要求(機構上 assert 不在のファイルは全件失敗し得る)。
- ~~テスト/例ファイルは assert さえ書けば環境側機構は触れない~~ → `#[global_allocator]` がスイートプロセスのアロケータを全走査緑で差替(実証)。global_allocator/no_mangle/export_name/link_section/`#[link`/`#[used]` を共有ニードルへ(リンカ結線・計測経路のすり替えは走査対象コードとして呼出に現れない)。
- ~~浮動小数点の超越関数は「検査コード側」でも全禁止できると思われていた~~ →
  `differential.rs` が Fixed 実装の truth として f64 libm(`theta.sin()` 等)を
  TRIG_TOL 内の近似比較で故意に使っており、`.sin()` メソッド(決定的 Fixed)と
  f64 libm の区別は字句走査では原理的に不能。厳密一致で libm 出力をピンする
  書き方は機種間で緑を偽造するのでなく red flake を起こす向きの残差 —
  受容残差として記録(塞ぐなら意味解析レイヤが必要)。
- ~~`git status` が失敗すれば sentinel が赤になると思われていた~~ →
  `tree_status` の出力が空同士で一致し「unchanged」を空告する面と、
  全行が `!!` フィルタに落ちる pristine tree で `grep -vE` rc1 が
  set -e を踏んで gate 自殺する面を分離して実証(/tmp probe: rc=1 fail-closed /
  rc=0 empty)。status を変数に退避して rc を伝播、フィルタは `|| true` で
  終了ステータスを手放す。
- ~~bin ルートの cfg 系は test-cfg の綴りだけ止めれば足りると思われていた~~ →
  `#[cfg(unix)]`/`#[cfg(not(unix))]`/`cfg!(unix)` が gamec.rs で全て緑で通過
  (host 条件で出荷ツールの振る舞いを分岐できる)。`#[test` の脇のニードルを
  `#[cfg`/`#![cfg`/`cfg!(`/`cfg_attr(` 全面禁止へ拡張。
- ~~bin ルートのスプライス系は `include!`/`include_bytes!`/`#[path]` で尽きたと思われていた~~ →
  `include_str!` が gamec.rs で親ディレクトリ越えのファイルを緑のまま焼込めた
  (注入実証)。bin ニードルに `include_str!` を追加(lib 側の README doc 埋込は
  bin には存在しないため全面禁止)。

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
