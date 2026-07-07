# IZANAGI 製品全体 — SWOT分析 (Product-Level SWOT Analysis)

> **この文書の目的**: リポジトリ `IZANAGI`（エンジン `izanagi` + シミュレーションキット
> `izanagi_kit` の Cargo workspace）を製品として評価し、強み・弱み・機会・脅威を
> 前提知識ゼロの読者（将来の Claude セッション、新規コントリビュータ、意思決定者）が
> この1ファイルで把握できるように書いた自己完結の分析。
>
> **執筆規則**: 全ての数値主張には検証コマンドを添える／未定義の略号を使わない／
> 「主観的な期待」と「実測した事実」を明確に区別する。
>
> **関係する既存文書**: `PRODUCT_AUDIT.md`（製品レベルの機能過不足、P1〜P7）、
> `izanagi_kit/FEATURE_AUDIT.md`（kit 内部78モジュールの過不足）。本書はそれらの
> 上位に立ち、「機能の有無」ではなく「事業・技術上の強み/弱み/機会/脅威」を扱う。
>
> 最終更新: 2026-07-03 / 分析対象コミット: `203ea4a`（ブランチ
> `claude/deepresearch-ultrathink-improve-yq2th`）/ 分析ブランチ:
> `claude/product-swot-analysis-0gaz09`

---

## 0. 前提: 検証済みの現状 (Verified baseline)

分析の土台となる事実。全てこのセッション内で実行・確認済み。

| 項目 | 値 | 検証コマンド |
|---|---|---|
| workspace 構成 | `izanagi`（エンジン）+ `izanagi_kit`（キット）の2クレート、resolver "2" | `cat Cargo.toml` |
| エンジン規模 | 25 モジュール（`src/*.rs`）、6 examples、zero-dependency | `grep -c "^mod \|^pub mod " izanagi/src/lib.rs` |
| キット規模 | 78 モジュール、18 統合テストファイル、zero-dependency | `grep -c "^pub mod " izanagi_kit/src/lib.rs` |
| workspace 全体テスト数 | **3362 件、全 green**（キット 3174 + エンジン 188） | `cargo test --workspace` |
| Lint | `cargo clippy --workspace --all-targets` 警告 **0**、`cargo fmt --all --check` クリーン | 同左コマンド |
| 安全性 | 両クレートとも `#![forbid(unsafe_code)]` | `grep forbid izanagi*/src/lib.rs` |
| 決定論保証 | `PINNED_FINAL_HASH`/`PINNED_ROGUELIKE_HASH` による bit-exact replay pin、`no_float_in_sim.rs` によるソーススキャン | `cargo test --test determinism --test roguelike_sim --test no_float_in_sim -p izanagi_kit` |
| CI | ワークフロー定義済み（`.github/workflows/ci.yml`、cross-OS determinism matrix 含む）だが**この実行環境の GitHub App トークンに `workflows` 権限が無く push 未達** — GitHub 上で一度も実行されていない | `git log --all -- .github/workflows/ci.yml`（ローカルにのみ存在するコミットの有無で確認） |
| 公開状態 | 両クレートとも crates.io 未公開（kit は `version = "0.1.0"`、engine は `"4.1.0"` だが由来不明のバージョン番号） | `cargo search izanagi_kit`（レジストリに存在しないことを確認） |
| ルート README | 「159 tests」等、workspace 化前の古い主張が残存（本書執筆時点で未修正） | `grep "159\|tests" README.md` |

---

## 1. Strengths — 強み

### S1. 決定論スタックの深さと検証密度
`izanagi_kit` の中核価値は「同じ入力なら、どの OS・CPU でも bit-exact に同じ結果」という
保証であり、これを**多層のテストで多角的に検証**している一プロジェクトは同カテゴリで稀:
- pinned hash 回帰（`tests/determinism.rs`, `tests/roguelike_sim.rs`）
- property-based test（`tests/properties.rs`、多くが3000イテレーション）
- metamorphic / differential / golden hash / stateful / conservation 則テストまで用意
- ソースコードを静的スキャンして `f32`/`f64` の混入を機械的に禁止する `no_float_in_sim.rs`

これは「決定論を謳うが実は未検証」というこの分野にありがちな失敗を回避しており、
**主張と実装の一致度が高い**ことが最大の技術的資産。

### S2. Zero-dependency 哲学
両クレートとも外部クレートに一切依存しない。これにより:
- サプライチェーン攻撃面がゼロ（`cargo audit` は常に「advisories: 0」）
- ビルド時間が短く、CI/組込み/wasm 移植の障壁が低い
- API が枯れた標準ライブラリのみで完結し、長期メンテナンスコストが低い

### S3. Headless-first 設計
`NullBackend`（エンジン）・`terminal` モジュール（キット）がヘッドレス実行を1級市民として
扱っており、GUI/ウィンドウシステムなしで CI・自動テスト・サーバーサイドシミュレーションが
成立する。ゲームエンジンとして異色の強みで、決定論保証と相性が良い。

### S4. コンテンツパイプラインとツール連携
`.game` 独自フォーマットの parser/validator/loader が揃い、`gamec` CLI が
`--json`/`--sarif`/`--check` の複数出力モードを持つ。SARIF はGitHub Code Scanning への
統合を想定した実装で、「決定論エンジン」という枠を超えたツールチェーン品質がある。

### S5. 文書の厚さと自己完結性
`SPEC.md`（.game 形式の形式 EBNF 文法含む）、`ARCHITECTURE.md`、`CLAUDE.md`
（AI エージェント向け開発ガイド）、複数の監査文書（`PRODUCT_AUDIT.md`,
`FEATURE_AUDIT.md`, 本書）が揃い、前提知識なしで開発を引き継げる設計になっている。

---

## 2. Weaknesses — 弱み

### W1. CI が一度もリモートで実行されていない
ワークフロー定義（cross-OS determinism matrix 含む）は存在するが、実行環境のトークン権限
不足で GitHub にまだ push できていない。**「クロス OS で決定論が保たれる」という中心的な
製品主張が、ローカル1環境でしか検証されていない**（`cargo test` はこのセッションの
Linux 環境でのみ実行）。これは S1 の価値を裏付ける最後のピースが欠けている状態。

### W2. 実用バックエンドの不在
エンジンには `NullBackend`（ヘッドレス）と `TerminalBackend`（ANSI端末）しかなく、
実ウィンドウ（winit/SDL2 等）・実音声出力・実ゲームパッド入力のバックエンドが無い
（`gamepad.rs`/`audio.rs` はデータ構造のみで、OS ドライバに繋がっていない）。
「市販レベル」を名乗るには、実機で遊べる経路が無いことは大きな欠落。

### W3. エンジン(f32) とキット(整数) の間に手動の橋渡ししか無い
両クレートの型システムは意図的に断絶しており（リアルタイム層 vs 決定論層）、
橋渡しは呼び出し側が手書きする以外に手段がない。`izanagi/examples/kit_bridge.rs`
（本書執筆時点で作業中、未コミット）が最初の実例だが、汎用の変換ヘルパーやトレイトは
存在せず、統合のたびに同じ変換コードが再発明される可能性が高い。

### W4. リリース履歴・来歴の不透明さ
`izanagi_kit` は `0.1.0` で未公開。エンジンの `version = "4.1.0"`（`Cargo.toml`）と
リポジトリ名/README が示唆する `v4.0.2` の関係が不明瞭（zip ファイル名は
`izanagi_v4.0.2.zip` だったが中身の `Cargo.toml` は `4.1.0` — どちらが正か、
CHANGELOG との対応が取れていない）。バージョニングの一貫性が製品としての信頼性を弱める。

### W5. Bus factor が実質 1
コミット履歴・`CLAUDE.md` の存在が示す通り、開発のほぼ全てが単一の AI エージェント
セッション群によって行われている。人間のレビュアー/共同メンテナが確認できず、
設計判断の妥当性を独立に検証する主体がいない。

### W6. Fuzzing が未達（環境制約、方針ではない）
`cargo-fuzz` は nightly toolchain を要求するが、このセッションのサンドボックスは
ネットワーク制限で nightly をインストールできず、実施できていない
（`FEATURE_AUDIT.md` に記録済み）。parser/savefile のような外部入力を扱う経路の
coverage-guided fuzzing が無いことは、"panic-free" の主張を完全には裏付けきれていない。

### W7. crates.io 未公開
両クレートともレジストリに存在せず、`cargo add izanagi_kit` で誰も使えない。
どれだけ品質が高くても、配布経路が無ければ採用は起こらない。

---

## 3. Opportunities — 機会

### O1. 「決定論ロークライク基盤」というニッチはほぼ無風
Rust ゲームエコシステムには Bevy/macroquad/ggez のような汎用エンジンは多いが、
「ネットワーク同期・リプレイ・自動テストのために bit-exact 決定論を最初から設計する」
kit は競合が少ない（`RESEARCH.md` が示す通り、この設計思想自体はローグライク/lockstep
文献に根拠があるが、実装として揃えたクレートは希少）。crates.io 公開でこのニッチの
デファクトになれる可能性がある。

### O2. kit_bridge をショーケースにした「エンジン+キット」の統合デモ
W3 で述べた橋渡しの弱みは、裏を返せば**汎用ブリッジ層を切り出して製品化する機会**でもある。
`kit_bridge.rs` を完成させ、両クレートの強みを1つの動くデモ（決定論シミュレーション +
リアルタイム描画）として見せられれば、README の「24モジュールのエンジン」という
検証不能な主張よりずっと説得力のあるマーケティング資産になる。

### O3. `netinput` モジュールを核にした決定論マルチプレイヤーのデモ化
`izanagi_kit::netinput` は決定論的な複数プレイヤー入力予測・誤予測検出を既に実装済み
（`FEATURE_AUDIT.md` 確認済み）。ロールバックネットコードのデモは Rust ゲーム界隈で
常に注目されるトピックであり、既存資産を可視化するだけで技術訴求力が上がる。

### O4. SARIF/Code Scanning 連携を GitHub エコシステム訴求に使う
`gamec --sarif` は既に実装済み。CI が稼働すれば「コンテンツファイルの誤りが PR に
インラインで表示される」という体験を即座に提供でき、単なる「決定論エンジン」以上の
開発者体験の良さを訴求できる。

### O5. wasm / 組込みターゲットへの展開
zero-dependency・`std`-only（`no_std` ではないが依存ゼロ）という制約は、wasm32
ターゲットへのポーティング障壁が低いことを意味する。ブラウザで動く決定論シミュレーションの
デモは配布・体験のハードルを大きく下げる。

---

## 4. Threats — 脅威

### T1. エコシステム規模で劣る確立済みエンジンとの競合
Bevy・macroquad・ggez 等は圧倒的なコミュニティ・プラグイン・ドキュメント量を持つ。
「決定論」という差別化軸を明確に打ち出し続けない限り、汎用ゲームエンジンとして比較されると
埋没するリスクが高い。

### T2. Zero-dependency 哲学が機能追加・貢献者獲得の天井になる
`CLAUDE.md` の "Do not: Add anyhow/serde/glam/rand" という明文ルールは品質を守る一方、
実ウィンドウバックエンド（W2）のような機能を追加しようとすると外部クレート（winit等）が
事実上必須になる領域にぶつかる。方針を貫くほど機能面で見劣りするジレンマがある。

### T3. CI 不在が長引くほど「決定論」主張の信頼が既存ユーザーに対して目減りする
W1 の状態が続くと、たとえローカルテストが3362件 green でも、外部の開発者・評価者は
「本当にクロス OS で動くのか」を自分で確認するまで信用しない。これはニッチ市場（O1）を
獲得する上で最大の障害になりうる。

### T4. CI 配置が単一の外部権限（GitHub App の `workflows` scope）に依存している
現在の実行環境の構造上、CI ワークフローファイルの配置がその環境のトークン権限に
ブロックされ続けている。これは技術的負債というより**運用上の単一障害点**であり、
解消されない限り T3 が固定化する。

### T5. AI エージェント主導開発への外部からの信頼性懸念
W5（bus factor 1）と組み合わさると、「人間のレビューを経ていないコードベース」という
懸念が、特にセキュリティ意識の高い潜在採用者（決定論・ネットコードが重要な用途は
往々にしてこの層と重なる）からの採用障壁になりうる。

---

## 5. クロス戦略 (次の一手)

| 象限 | 戦略 | 具体的な次の一手 |
|---|---|---|
| **SO**（強みを機会に活かす） | S1(決定論の深さ) × O1(ニッチ市場) | `kit_bridge.rs` を完成させ crates.io に `izanagi_kit` を公開。README を実装済み機能のみで書き直し、決定論保証をトップメッセージにする |
| **SO** | S4(SARIF等ツール品質) × O4(GitHub連携訴求) | CI 稼働後、`upload-sarif` を README のスクリーンショット/GIF付きで訴求 |
| **WO**（弱みを機会で克服） | W1(CI不在) × O1/O3(市場機会) | **最優先**: `workflows` 権限問題を解決（権限付与 or 手動配置)し、determinism-matrix を実際に緑化する。これなしには O1 の「決定論を謳う」訴求が空証文のまま |
| **WO** | W3(橋渡し不在) × O2(統合デモ機会) | `kit_bridge.rs` の完成を、汎用ブリッジ層（型変換ヘルパー）切り出しの第一歩として設計する |
| **ST**（強みで脅威を防ぐ） | S1/S5(文書の厚さ) × T5(AI開発への懸念) | 監査文書群（本書含む）を「品質担保の代替エビデンス」として README から明示的にリンクし、透明性で信頼を補う |
| **WT**（弱みと脅威の複合リスクを最小化） | W1×T3×T4(CI依存の連鎖) | CI 配置を GitHub App 権限に依存しない代替経路（例: リポジトリ管理者による手動 push、別トークンでの一時配置）で早期に断ち切り、連鎖的な信頼毀損を止める |

**読み方の指針**: この分析における最大のテコは一貫して **W1/T3/T4（CI不在の連鎖）** に
集約される。技術的な実装（S1〜S4）は既に高水準にあり、残るボトルネックは実行環境の
権限問題という非技術的な1点である。これが解消されれば O1〜O4 の機会は即座に着手可能になる。
