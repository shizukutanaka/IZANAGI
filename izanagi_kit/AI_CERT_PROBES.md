# AI_CERT_PROBES — validator の意味検査は「読み込めるが壊れている」バンドルを捕まえるか

この文書は `gamec --verify` に意図的に壊したゲームバンドルを与え、validator
([`src/validator.rs`](./src/validator.rs))が警告を出すか出さないかを記録した**測定記録**である。
対象は PR #486 で入った空間検査(到達不能 spawn・孤立部屋)と、既存の spawn 重なり検査。
コードは変更していない。

- 測定対象: `main` の `efddae8`(PR #488 マージ時点。PR #486 の `check_spatial` を含む)
- 実行: `cargo build -p izanagi_kit --bin gamec` でビルドし、各バンドルを `gamec --verify <file.game>` に通した
- 判定: 欠陥に対応する diagnostic(warning / error)が出れば「検出」、何も出なければ「未検出」
- バンドルは全文を下に載せてある。テキストをファイルに保存すれば同じ結果を再現できる

## 結果一覧

| ID | 壊し方 | 結果 | warning | error | exit |
|---|---|---|---|---|---|
| P00 | 対照: 2部屋が扉でつながった正常バンドル | 警告なし(誤検出しない) | 0 | 0 | 0 |
| P01 | 到達できない spawn 領域(壁で閉じた宝物庫に `chest`) | 検出 | 1 | 0 | 0 |
| P02 | 孤立した部屋(spawn の無い閉じた床領域) | 検出 | 1 | 0 | 0 |
| P03 | spawn の重なり(`hero` と `goblin` が同一セル) | 検出 | 1 | 0 | 0 |
| P04 | 斜めの角でしか接していない領域(角抜け禁止規則の境界) | 検出 | 1 | 0 | 0 |
| P05 | 最初の spawn(入口扱い)自身が1マスの小部屋に閉じ込められている | 検出(ただし帰責が逆) | 1 | 0 | 0 |
| P06 | 壁セルの上に spawn(`goblin` を外周の壁 (0,0) に配置) | 間接的に検出 | 1 | 0 | 0 |
| P07 | 2部屋を隔てる壁セルの上に spawn(`goblin` が壁 (4,2) に埋まり、宝物庫は実際には閉じている) | 未検出 | 0 | 0 | 0 |
| P08 | p01 と同じ地形だが、壁タイルの名前が `stone` | 未検出 | 0 | 0 | 0 |
| P09 | 通行不能を意図したタイル `water` で宝物庫を隔てる | 未検出 | 0 | 0 | 0 |
| P10 | 外周の壁が欠け、床が地図の端に接している | 未検出 | 0 | 0 | 0 |
| P11 | 行に未宣言のグリフ `X` がある | 未検出 | 0 | 0 | 0 |
| P12 | spawn を1つも持たないレベルに閉じた部屋が2つ | 未検出 | 0 | 0 | 0 |
| P13 | グリッド上の `g` の位置に spawn が無く、`goblin` は別の床セルに spawn | 未検出 | 0 | 0 | 0 |
| P14 | p01 と同じ地形だが、壁タイルの名前が `Wall`(大文字) | 未検出 | 0 | 0 | 0 |
| P15 | 対照: 範囲外 spawn(既存の非空間ルール) | 検出(error) | 0 | 1 | 1 |

16 件中、対照2件(P00 正常・P15 既存 error)を除く**意味的欠陥 14 件のうち検出 5 件・間接検出 1 件・未検出 8 件**。
P00 で誤検出が無いことも確認した。

## 検出できたもの

- **到達できない spawn 領域(P01)** — 壁で閉じた宝物庫の `chest` を「`hero` から到達できない」と報告。
- **孤立した部屋(P02)** — spawn の無い閉じた 9 タイルの床領域を `isolated room` として報告。
- **spawn の重なり(P03)** — 同一セル (1,1) の2 spawn を報告。
- **斜めの角だけで接する領域(P04)** — 8方向・角抜け禁止(`ConnectivityMap`)の規則どおり、
  直交方向が両方とも壁の斜め接続を非連結として報告。
- **入口自身が閉じ込められている(P05)** — 警告は出る。ただし入口は「宣言順で最初の spawn」と
  決め打ちされるため、閉じ込められているのは `hero` なのに、警告は残り 17 タイルの領域の
  `goblin`・`chest` を「到達できない」側として名指す。**検出はするが原因の帰責が逆になる。**
- **壁の上の spawn(P06、間接的)** — 「壁に埋まっている」という警告は無い。spawn セルは常に
  通行可能とみなされるので、壁に囲まれた 1 タイルの領域として P01 と同じ文面で報告される。
  P07 のように壁セルが2つの領域に接していると、この間接検出も消える(下記)。

P01〜P06 のうち P06 は欠陥そのもの(壁の中の spawn)を名指さないので、上の「検出 5 件」には
P01〜P05 だけを数えている。

## 検出できなかったもの(validator の限界)

いずれも `gamec --verify` は exit 0 で、警告もエラーも出さない。

1. **壁セル上の spawn が壁を橋渡しする(P07)** — spawn セルを無条件に通行可能とする規則により、
   2部屋を隔てる壁の上に置いた `goblin` が「扉」になり、実際には閉じている宝物庫が
   連結とみなされる。壁上の spawn 自体も報告されない。**偽陰性を作る規則であり、P06 の間接検出も
   壁セルが孤立している場合に限られる。**
2. **`wall` 以外の名前の壁タイル(P08 `stone`・P14 `Wall`)** — 空間検査は名前がちょうど `wall`
   のタイルが無いと丸ごと沈黙する。大文字小文字も区別される。P01 と同一の地形が、名前だけで
   無検査になる。
3. **`wall` 以外の通行不能タイル(P09 `water`)** — `.game` 形式は通行可否を宣言できないので、
   `wall` 以外のグリフはすべて床として扱われる。水・溶岩・穴で隔てた領域は連結とみなされる。
4. **外周の欠け(P10)** — 床が地図の端に接していても報告しない。地図外は検査対象外。
5. **未宣言のグリフ(P11)** — どの `tile` にも対応しないグリフ `X` が行にあっても報告しない
   (床として扱われる)。未使用タイルの警告はあるが、その逆向き(使われているが未定義)は無い。
6. **spawn を持たないレベル(P12)** — 入口が無いレベルは空間検査ごとスキップされるので、
   閉じた部屋が何個あっても報告されない。レベルに spawn が1つも無いこと自体も報告されない。
7. **グリッドと spawn の不一致(P13)** — 行に描かれた `g` の位置に spawn が無く、`goblin` は
   別の床セルに spawn していても報告しない。グリッド上の prefab グリフと spawn 座標の照合は無い。
8. **警告は exit code に現れない** — 検出できた P01〜P06 も含め、警告だけのバンドルは
   `gamec --verify` が exit 0 を返す(error だけが exit 1)。CI の合否だけを見る利用者には、
   検出した空間欠陥も見えない。これは「警告にとどめる」という `check_spatial` の設計どおりの
   挙動であり、欠陥ではなく限界として記録する。

## 測定したバンドル

### P00 `p00_control_open_door.game`

対照: 2部屋が扉でつながった正常バンドル。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level vault 9x5
  row #########
  row #@..#...#
  row #.....$.#
  row #...#...#
  row #########
  spawn hero 1 1
  spawn chest 6 2
```

`gamec --verify` の出力(exit 0):

```text
level 'vault': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0xed6e1781fc0f6005
verify: 1/1 level(s) deterministic
```

### P01 `p01_sealed_vault.game`

到達できない spawn 領域(壁で閉じた宝物庫に `chest`)。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level vault 9x5
  row #########
  row #@..#...#
  row #...#.$.#
  row #...#...#
  row #########
  spawn hero 1 1
  spawn chest 6 2
```

`gamec --verify` の出力(exit 0):

```text
p01_sealed_vault.game: warning: level 'vault': spawn(s) 'chest' in a region of 9 tile(s) at (5,1) cannot be reached from spawn 'hero' at (1,1)
level 'vault': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0xed6e1781fc0f6005
verify: 1/1 level(s) deterministic
```

### P02 `p02_isolated_room.game`

孤立した部屋(spawn の無い閉じた床領域)。

```text
prefab hero
  glyph @
prefab goblin
  glyph g
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level rooms 9x5
  row #########
  row #@.g#...#
  row #...#...#
  row #...#...#
  row #########
  spawn hero 1 1
  spawn goblin 3 1
  spawn chest 2 2
```

`gamec --verify` の出力(exit 0):

```text
p02_isolated_room.game: warning: level 'rooms': isolated room of 9 tile(s) at (5,1): no spawn inside and no path to any spawn
level 'rooms': deterministic over 256 tick(s), 3 entit(y/ies); final hash 0x3e4714bcfc2a5896
verify: 1/1 level(s) deterministic
```

### P03 `p03_overlap.game`

spawn の重なり(`hero` と `goblin` が同一セル)。

```text
prefab hero
  glyph @
prefab goblin
  glyph g
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level room 6x4
  row ######
  row #@...#
  row #..$.#
  row ######
  spawn hero 1 1
  spawn goblin 1 1
  spawn chest 3 2
```

`gamec --verify` の出力(exit 0):

```text
p03_overlap.game: warning: level 'room': multiple spawns at (1,1) (e.g. 'goblin')
level 'room': deterministic over 256 tick(s), 3 entit(y/ies); final hash 0x17f8b42be4c1a926
verify: 1/1 level(s) deterministic
```

### P04 `p04_diagonal_corner.game`

斜めの角でしか接していない領域(角抜け禁止規則の境界)。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level corner 6x5
  row ######
  row #@.###
  row ###.$#
  row ###..#
  row ######
  spawn hero 1 1
  spawn chest 4 2
```

`gamec --verify` の出力(exit 0):

```text
p04_diagonal_corner.game: warning: level 'corner': spawn(s) 'chest' in a region of 4 tile(s) at (3,2) cannot be reached from spawn 'hero' at (1,1)
level 'corner': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0x22ec6f2ec031ef5b
verify: 1/1 level(s) deterministic
```

### P05 `p05_entrance_is_sealed.game`

最初の spawn(入口扱い)自身が1マスの小部屋に閉じ込められている。

```text
prefab hero
  glyph @
prefab goblin
  glyph g
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level closet 9x5
  row #########
  row #@#.....#
  row ###.g.$.#
  row #.......#
  row #########
  spawn hero 1 1
  spawn goblin 4 2
  spawn chest 6 2
```

`gamec --verify` の出力(exit 0):

```text
p05_entrance_is_sealed.game: warning: level 'closet': spawn(s) 'goblin', 'chest' in a region of 17 tile(s) at (3,1) cannot be reached from spawn 'hero' at (1,1)
level 'closet': deterministic over 256 tick(s), 3 entit(y/ies); final hash 0xa7777f0e47da9337
verify: 1/1 level(s) deterministic
```

### P06 `p06_spawn_in_wall.game`

壁セルの上に spawn(`goblin` を外周の壁 (0,0) に配置)。

```text
prefab hero
  glyph @
prefab goblin
  glyph g
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level room 6x4
  row ######
  row #@...#
  row #....#
  row ######
  spawn hero 1 1
  spawn goblin 0 0
  spawn chest 3 2
```

`gamec --verify` の出力(exit 0):

```text
p06_spawn_in_wall.game: warning: level 'room': spawn(s) 'goblin' in a region of 1 tile(s) at (0,0) cannot be reached from spawn 'hero' at (1,1)
level 'room': deterministic over 256 tick(s), 3 entit(y/ies); final hash 0x17f8b42be4c1a926
verify: 1/1 level(s) deterministic
```

### P07 `p07_spawn_bridges_wall.game`

2部屋を隔てる壁セルの上に spawn(`goblin` が壁 (4,2) に埋まり、宝物庫は実際には閉じている)。

```text
prefab hero
  glyph @
prefab goblin
  glyph g
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level bridge 9x5
  row #########
  row #@..#...#
  row #...#.$.#
  row #...#...#
  row #########
  spawn hero 1 1
  spawn goblin 4 2
  spawn chest 6 2
```

`gamec --verify` の出力(exit 0):

```text
level 'bridge': deterministic over 256 tick(s), 3 entit(y/ies); final hash 0xe792c3de577189d6
verify: 1/1 level(s) deterministic
```

### P08 `p08_no_wall_tile.game`

p01 と同じ地形だが、壁タイルの名前が `stone`。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile stone # #6E7681
level vault 9x5
  row #########
  row #@..#...#
  row #...#.$.#
  row #...#...#
  row #########
  spawn hero 1 1
  spawn chest 6 2
```

`gamec --verify` の出力(exit 0):

```text
level 'vault': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0xed6e1781fc0f6005
verify: 1/1 level(s) deterministic
```

### P09 `p09_other_solid_tile.game`

通行不能を意図したタイル `water` で宝物庫を隔てる。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
tile water ~ #1F6FEB
level moat 9x5
  row #########
  row #@..~...#
  row #...~.$.#
  row #...~...#
  row #########
  spawn hero 1 1
  spawn chest 6 2
```

`gamec --verify` の出力(exit 0):

```text
level 'moat': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0xd4efdc5cb42f784e
verify: 1/1 level(s) deterministic
```

### P10 `p10_open_border.game`

外周の壁が欠け、床が地図の端に接している。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level leak 6x4
  row ######
  row #@....
  row #..$.#
  row ######
  spawn hero 1 1
  spawn chest 3 2
```

`gamec --verify` の出力(exit 0):

```text
level 'leak': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0xe3da071b88f472ec
verify: 1/1 level(s) deterministic
```

### P11 `p11_undeclared_glyph.game`

行に未宣言のグリフ `X` がある。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level room 6x4
  row ######
  row #@.X.#
  row #..$.#
  row ######
  spawn hero 1 1
  spawn chest 3 2
```

`gamec --verify` の出力(exit 0):

```text
level 'room': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0x6055dd9915c87a46
verify: 1/1 level(s) deterministic
```

### P12 `p12_no_spawns_level.game`

spawn を1つも持たないレベルに閉じた部屋が2つ。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level empty 9x5
  row #########
  row #...#...#
  row #...#...#
  row #...#...#
  row #########
level used 4x3
  row ####
  row #@$#
  row ####
  spawn hero 1 1
  spawn chest 2 1
```

`gamec --verify` の出力(exit 0):

```text
level 'empty': deterministic over 256 tick(s), 0 entit(y/ies); final hash 0xe9bd022b9afd45cf
level 'used': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0xefac5f95fbca45c7
verify: 2/2 level(s) deterministic
```

### P13 `p13_grid_glyph_without_spawn.game`

グリッド上の `g` の位置に spawn が無く、`goblin` は別の床セルに spawn。

```text
prefab hero
  glyph @
prefab goblin
  glyph g
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level room 6x4
  row ######
  row #@.g.#
  row #.$..#
  row ######
  spawn hero 1 1
  spawn chest 2 2
  spawn goblin 4 2
```

`gamec --verify` の出力(exit 0):

```text
level 'room': deterministic over 256 tick(s), 3 entit(y/ies); final hash 0xf3d3eac44799bc22
verify: 1/1 level(s) deterministic
```

### P14 `p14_wall_case.game`

p01 と同じ地形だが、壁タイルの名前が `Wall`(大文字)。

```text
prefab hero
  glyph @
prefab chest
  glyph $
tile floor . #3A3A3A
tile Wall # #6E7681
level vault 9x5
  row #########
  row #@..#...#
  row #...#.$.#
  row #...#...#
  row #########
  spawn hero 1 1
  spawn chest 6 2
```

`gamec --verify` の出力(exit 0):

```text
level 'vault': deterministic over 256 tick(s), 2 entit(y/ies); final hash 0xed6e1781fc0f6005
verify: 1/1 level(s) deterministic
```

### P15 `p15_oob_spawn.game`

対照: 範囲外 spawn(既存の非空間ルール)。

```text
prefab hero
  glyph @
prefab goblin
  glyph g
prefab chest
  glyph $
tile floor . #3A3A3A
tile wall # #6E7681
level room 6x4
  row ######
  row #@...#
  row #..$.#
  row ######
  spawn hero 1 1
  spawn chest 3 2
  spawn goblin 6 1
```

`gamec --verify` の出力(exit 1):

```text
p15_oob_spawn.game: error: level 'room': spawn 'goblin' at (6,1) is outside 6x4
FAILED: 1 error(s), 0 warning(s)
```
