# IZANAGI 製品全体 — 機能過不足の監査リスト (Product-Level Feature Audit)

> **この文書の目的**: リポジトリ `IZANAGI` が含む**2つの成果物**（エンジン本体 + izanagi_kit）を
> 製品としてまとめて監査し、機能を「片側のみに存在 / 両側に重複 / 製品として不足」に選別した
> 自己完結のリスト。前提知識ゼロの読者（将来の Claude セッション、新規コントリビュータ）が
> この1ファイルで製品の全体像と欠落を把握できるように書かれている。
>
> **執筆規則**: 未定義の略号を使わない / 全項目に「何が・どこに・なぜ」を含める /
> 主張には検証コマンドを添える。
>
> **対象範囲の関係**: `izanagi_kit/FEATURE_AUDIT.md` は kit **内部**の77モジュールの過不足を
> 監査した姉妹文書。本書はその上位で、**エンジン↔kit の間**と**製品全体**を扱う。
> kit 内部の詳細は本書では繰り返さない。
>
> 最終更新: 2026-07-02 / ブランチ: `claude/deepresearch-ultrathink-improve-yq2th`

---

## 1. 製品の全体像 (What this product actually is)

リポジトリ直下には2つの成果物がある:

| 成果物 | 場所 | 規模 | 設計哲学 |
|---|---|---|---|
| **IZANAGI エンジン v4.0.2** | `izanagi_v4.0.2.zip`（**未展開**の完全な crate: Cargo.toml + src 24 modules + examples 6本 + tests） | ~6,500 LOC | 「One type, one method」— `Engine::new().run()` だけで動く使いやすさ最優先のリアルタイム層。f32 数学、immediate-mode 描画、audio/gamepad 付き |
| **izanagi_kit** | `izanagi_kit/`（通常のソースツリー） | 77 modules | 決定論最優先のシミュレーション層。整数/Q16.16 固定小数点のみ、bit-exact replay を pinned hash（`izanagi_kit/tests/determinism.rs`）で保証 |

kit の `src/lib.rs` 冒頭が示す通り、kit は「エンジンの design review から抽出された参照実装群」
であり、両者は**意図的に哲学が異なる**。したがって本書の「過剰（重複）」判定基準は:

- **並存が正当**: 同名/同概念でも、リアルタイム層（f32・速度優先）と決定論層（整数・再現性優先）
  という異なる制約に応えているもの。
- **統合検討**: 制約の違いで説明できない無自覚な二重実装。

検証コマンド:
```
unzip -l izanagi_v4.0.2.zip              # エンジンの全ファイル一覧
unzip -p izanagi_v4.0.2.zip src/lib.rs   # エンジンのモジュール宣言（展開せず読む）
grep -c "^pub mod " izanagi_kit/src/lib.rs   # kit のモジュール数 = 77
cd izanagi_kit && cargo test             # kit の全 suite（3113 tests）green
```

---

## 2. エンジンにあり、kit に無いもの (Engine-only capabilities)

各行に「kit へ移植すべきか」の判定を付す。kit の方針は zero-dependency・ヘッドレス・
OS I/O 非依存（`izanagi_kit/GAME_DEV_TAXONOMY.md` 冒頭に明記）。

| 機能 | エンジン側の場所 | kit へ移植すべきか |
|---|---|---|
| `Engine` facade + run loop（1型で全 subsystem を field 公開） | `src/lib.rs` | **不要** — kit は「エンジンではなく部品集」が方針。facade は利用側ゲームが組む |
| audio ミキサー + WAV/PCM ローダ + sine 生成 | `src/audio.rs`, `src/audio_pcm.rs` | **範囲外** — 音声出力は OS I/O。kit のヘッドレス方針で意図的に除外（FEATURE_AUDIT.md 第5節と同判定） |
| gamepad（4台・スティック・deadzone） | `src/gamepad.rs` | **範囲外** — 同上。ただし「読み取った入力を決定論 sim に流す」側は kit の `cmdqueue`/`inputbuf` が受け皿として実装済み |
| mouse 入力・edge イベント | `src/input.rs` | **範囲外**（OS I/O）。キー抽象は kit の `keymap` が対応済み |
| `Backend` trait 抽象（NullBackend / TerminalBackend / 将来の winit・wgpu） | `src/backend.rs` | **検討価値あり** — kit の `terminal` は具体実装のみで、描画先を差し替える trait が無い。ヘッドレス方針と両立する純抽象なので、kit 側の数少ない実装候補 |
| sprite / frame Animation | `src/sprite.rs` | **不要** — kit の `tween` + `timer` + `terminal` の合成で表現可能 |
| scene graph（親子 2D transform 合成） | `src/scene.rs` | **概ね対応済み** — kit の `relations`（親子 + propagate）が整数版に相当 |
| log モジュール | `src/log.rs` | **概ね対応済み** — kit は `msglog`（ゲーム内ログ）と `profiler::EventLog`（構造化イベント）で代替 |

## 3. kit にあり、エンジンに無いもの (Kit-only capabilities — 要約)

詳細は [`izanagi_kit/FEATURE_AUDIT.md`](./izanagi_kit/FEATURE_AUDIT.md) 第2節（16カテゴリ×77
モジュールの全表）を参照。エンジンに無い主要な塊だけ挙げる:

- **決定論スタック**: Q16.16 固定小数点（`fixed`）・状態チェックサム（`world_hash`）・
  リプレイ記録/desync 特定/rollback（`replay`）・マルチプレイヤー入力予測（`netinput`）
- **roguelike アルゴリズム**: 対称 FOV・A*/JPS/Dijkstra map/flee map/auto-explore・
  手続き生成4種・WFC・fog-of-war
- **コンテンツパイプライン**: テキスト DSL のパース→検証→ECS ロード + CLI ゲート（`gamec`）
- **ゲームプレイ系 24+ modules**: 戦闘・装備（呪い付き）・スキル・脅威・クエスト・会話・
  ショップ・アイテム識別・クロスラン進行 等

エンジンの README が例示する `roguelike.rs`（20KB の example）は、kit ならモジュール合成で
組める内容を1ファイルに手書きしている — kit の存在理由の実証でもある。

---

## 4. 概念の重複 (Duplicated concepts — 製品レベルの「過剰」候補の判定)

同一概念が両側に存在する13組。**数値モデル（f32 vs 整数/Q16.16）と決定論要件が非互換**のため、
大半は「並存が正当」だが、判断根拠を1行ずつ明文化する:

| 概念 | エンジン側 | kit 側 | 判定 |
|---|---|---|---|
| RNG | `rng.rs`（xorshift64、wall-clock seed の `from_entropy()` あり） | `rng.rs`（SplitMix64、退化入力で draw を消費しない契約、bias 無し抽出） | **並存が正当** — ただしエンジン側の `from_entropy` はリプレイ破壊 API（第5節参照） |
| 数学 | `math.rs`（Vec2/Vec3/Mat3/Rect、f32、「games, not science」と明記） | `fixed.rs` + `vec.rs`（Q16.16、CORDIC） | **並存が正当** — 層の目的が違う（描画補間 vs 再現可能 sim） |
| tween | `tween.rs`（f32 + Timer） | `tween.rs`（Fixed + TweenSequence） | **並存が正当** — 同上 |
| easing | `ease.rs`（f32） | `easing.rs`（Fixed、back/bounce/elastic 含む） | **並存が正当** |
| camera | `camera.rs`（f32、follow/zoom/rotation） | `camera.rs`（整数 viewport） | **並存が正当** |
| tilemap | `tilemap.rs`（culling 付き） | `tilemap.rs`（多層 + LayeredMap） | **並存が正当** |
| 衝突 | `collide.rs`（AABB/swept/ray/circle、f32） | `aabb.rs` + `spatial_hash.rs` + `passability.rs`（整数） | **並存が正当** — swept AABB は kit 未実装だが整数 sim ではグリッド衝突が主 |
| ECS | `ecs.rs`（`HashMap<Entity,T>` per component） | `entity.rs` + `sparse_set.rs` + `arch.rs`（Vec ベース） | **並存が正当だが要注意** — エンジン側 HashMap は iteration 順が非決定（第5節） |
| state 機械 | `state.rs`（pushdown automaton） | `fsm.rs` + `hfsm.rs`（遷移表 + 階層） | **並存が正当** — シーン遷移用 vs ゲーム AI 用 |
| event | `event.rs`（typed bus） | `eventqueue.rs`（intra-tick FIFO） | **並存が正当** |
| save | `save.rs`（magic+version+length） | `savefile.rs`（+checksum+migration） | **統合検討** — 形式がほぼ同型で、kit 版が上位互換。エンジンが kit 版を使えば二重実装が消える |
| assets | `assets.rs`（byte cache + fs loader） | `assets.rs`（世代付き typed handle） | **統合検討** — handle 安全性は kit 版が上。fs loader 部分のみエンジン固有 |
| debug 計測 | `debug.rs`（FPS/worst_ms） | `profiler.rs`（tick 区間 + EventLog) | **並存が正当** — フレーム計測 vs tick 計測 |

**結論**: 削除すべき重複は無いが、`save` と `assets` の2組は「kit 版を正とし、エンジンが薄い
ラッパで再利用する」統合を（両成果物を1リポジトリで保守し続けるなら）検討する価値がある。

---

## 5. 製品レベルの不足 (Product-level deficiencies — 本監査の主要な発見)

kit 内部は FEATURE_AUDIT.md の通り充足しているが、**製品として**は以下が欠けている:

| # | 不足 | 詳細（何が・どこに・なぜ問題か） |
|---|---|---|
| P1 | **エンジン↔kit の橋渡し層が存在しない** | kit の決定論 sim の結果をエンジンの `render`/`Backend` で描く統合例・アダプタが1つも無い。2成果物は同一リポジトリに同居しているだけで、コード上の接点がゼロ。「kit で sim を回し、エンジンで見せる」という製品の自然な完成形が示されていない |
| P2 | **エンジンが zip のまま（ビルド/テスト不能）** | `izanagi_v4.0.2.zip` は Cargo.toml・tests を含む完全な crate だが未展開。README の「159 tests」「cargo run --example pong」はこのリポジトリでは実行できない。展開して cargo workspace（P4）に含めるのが自然な解消 |
| P3 | ~~**CI 定義が無い**~~ ✅ **本監査での誤りを訂正・修正済み** — CI ワークフロー自体は `izanagi_kit/.github/workflows/ci.yml` に存在していたが、**GitHub Actions はリポジトリルートの `.github/workflows/` しか検出しない**ため、サブディレクトリに置かれたこのファイルは一度も実行されていなかった（`dependabot.yml` も同様に非機能、かつ `directory: "/"` が誤り）。`.github/workflows/ci.yml` と `.github/dependabot.yml` をリポジトリルートへ移設し `defaults.run.working-directory: izanagi_kit` を付与、`dependabot.yml` の cargo エントリを `directory: "/izanagi_kit"` に修正。さらに既存 `test` ジョブが Linux のみだった点（kit の中心的主張である `PINNED_FINAL_HASH`/`PINNED_ROGUELIKE_HASH` のクロス OS 一致が一度も CI 検証されていなかった）を突き止め、`determinism-matrix` ジョブ（Linux/macOS/Windows の3 OS で pinned hash テストを実行）を新設 | ルート README は「CI: Linux + macOS + Windows」を明記するが、ファイルが誤った場所にあり実行されていなかった。kit の pinned hash（クロス OS の bit-exact 検証が本来の狙い）にとって致命的な見落としだった |
| P4 | **cargo workspace になっていない** | ルートに workspace Cargo.toml が無く、`izanagi_kit` は独立 crate。エンジンを展開しても2 crate をまとめてビルド/テストする構成が無い |
| P5 | **バージョン系譜の欠落** | zip は v4.0.2、kit の `src/lib.rs` は「design review of the IZANAGI engine **(v4.4.0)**」を引用。v4.0.2→v4.4.0 の間の版がリポジトリに無く、レビュー対象と保存物が一致しない |
| P6 | **ルート README が kit に言及しない** | ルート README はエンジンのみを説明。リポジトリの大半（77 modules・3113 tests）を占める kit の存在・目的・エンジンとの関係が root からは見えない（本監査と同時に案内を追記して解消） |
| P7 | **エンジンの決定論主張と実装の乖離** | README は「**Deterministic.** Seed the RNG, replay the run」と謳うが: (a) `math.rs` は f32（クロスプラットフォームで非再現 — kit の `RESEARCH.md` C2 が文献付きで指摘）、(b) `ecs.rs` は `HashMap` storage（iteration 順が非決定）、(c) `rng.rs` に wall-clock seed の `from_entropy()`。同一 OS/バイナリの単機リプレイなら概ね成立するが、無条件の「Deterministic」表記は kit が保証する水準（bit-exact・クロスプラットフォーム）と混同を招く。README の限定表記への修正、または kit を「厳密決定論が要る場合の層」として README から参照するのが解消 |

**注**: P2〜P4 の解消（zip 展開・workspace 化・CI 追加)は本監査のスコープ外の実装作業として
リスト化のみ。P6 は本コミットで README への案内追記により解消済み。

---

## 6. ルート README の主張の検証結果 (Claims vs reality)

| README の主張 | 検証結果 |
|---|---|
| 「Zero dependencies. Only the standard library」 | ✅ 検証可（zip 内 Cargo.toml に依存なし。kit も同様） |
| 「159 tests — 121 unit + 19 integration + 10 benchmark + 9 doctest」 | ⚠️ **このリポジトリでは検証不能** — zip 未展開のため実行できない（不足 P2） |
| 「CI: Linux + macOS + Windows」 | ✅ **修正済み** — ワークフローは存在したがリポジトリルート外にあり非機能だった（不足 P3）。ルートへ移設し、Linux/macOS/Windows 全てで pinned hash を検証する `determinism-matrix` ジョブを追加して主張を実態化 |
| 「Deterministic. Seed the RNG, replay the run」 | ⚠️ **限定付きで成立** — 単機・同一バイナリなら概ね成立。クロスプラットフォームでは f32/HashMap/from_entropy が破る（不足 P7）。厳密決定論は kit が担う |
| 「Headless first. Tests run in CI environments unchanged」 | ✅ 設計は検証可（NullBackend 既定、`src/backend.rs`）— ただし CI 自体は無い |
| 「~6,500 LOC total」 | ✅ 概ね妥当（zip 内ファイルサイズ合計から整合） |

---

## 7. 横断サマリ (Cross-reference summary)

| 分類 | 件数 | 内容 |
|---|---|---|
| エンジン固有の充足 | 8 機能 | facade/run loop・audio・gamepad・mouse・Backend trait・sprite・scene・log（第2節。うち kit へ移植検討は Backend trait のみ、他は範囲外/対応済み） |
| kit 固有の充足 | 実質 60+ modules | 決定論スタック・roguelike アルゴリズム・コンテンツパイプライン・ゲームプレイ系（第3節、詳細は FEATURE_AUDIT.md） |
| 概念の重複 | 13 組 | 11 組は「並存が正当」（f32 リアルタイム層 vs 整数決定論層）、2 組（save・assets）のみ統合検討（第4節） |
| 製品レベルの不足 | 7 件中1件解消（P3）、6件残（P1,P2,P4〜P7） | 最重要な残課題は P1 橋渡し層の不在・P7 決定論主張の乖離（第5節） |
| README 主張の乖離 | 6 主張中 ✅4・⚠️2 | CI 主張は修正済み、テスト数は zip 未展開で検証不能、決定論は限定付き（第6節） |

**読み取り方（次の一手の選び方)**: P3（CI）は本セッションで解消済み
（ワークフローをリポジトリルートへ移設し、`determinism-matrix` ジョブで pinned hash の
クロス OS 一致を実際に検証するようにした）。次の最短経路は P2→P4 の順
（zip 展開 → workspace 化。これで「159 tests」の検証も同じ CI に乗せられる）。
その後 P1（エンジンの `Backend` に kit の `terminal` セルバッファを流す統合 example が
最小構成）。kit 内部の残課題は
[`izanagi_kit/FEATURE_AUDIT.md`](./izanagi_kit/FEATURE_AUDIT.md) 第6節を参照。
