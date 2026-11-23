# Task 1 の制約（原典からの逐語抜粋。`scripts/task_context.py` が機械で集めた写し）

対象: **Task 1: 版番号の規則と versionCode の式（bash の正本と対応表）** / change: `st61-auto-version-bump`

> **これは仕様の正本ではない。** それぞれの `Source:` が正本で、ここにあるのは見出しと行の逐語コピー。
> 入っているのは **Task 1 の本文が名指ししたものだけ**（`D<n>` `C<n>` `Q<n>` `R<n>`・Scenario 名・パス）。
> 足りなければ**原典のその見出しだけ**を読む（下の表）。読むことは禁じていない —— 広く探すのをやめるだけ。

## 原典（足りなければ、ここの見出しを名指しで読む）

| 何 | 正本 | 見出しの形 |
|---|---|---|
| Task の本文（brief と同じ） | `openspec/changes/st61-auto-version-bump/tasks.md` | `## Task 1:` |
| 本人が決めたこと | `openspec/changes/st61-auto-version-bump/deep.md` | `### Q<n>` / 既定 `C<n>` の表 / 答えの逐語ブロック |
| 設計の判断 | `openspec/changes/st61-auto-version-bump/design.md` | `### D<n>.` |
| Scenario と受け入れ条件 | `openspec/changes/st61-auto-version-bump/specs/**/spec.md`・`openspec/specs/**/spec.md` | `### Requirement:` / `#### Scenario:` |
| 独立レビューの指摘 | `openspec/changes/st61-auto-version-bump/review/*.md` | `## R<n>.` |

## tasks.md の前置きと Global Constraints（逐語）

Source: `openspec/changes/st61-auto-version-bump/tasks.md`

```md
# ST61 実装タスク — main に merge するたびに版番号が自動で進む

読む順: `deep.md`（Q1〜Q4。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST52・ST54 は archive 済みで main にある。版の検査（`tools/release/version.sh`・`crates/build-stamp/src/lib.rs`・`collector-android/app/build.gradle.kts`・`web/build-stamp.ts`・`tools/check-version.sh`・`tools/check-build-stamp.sh`）・
配る組み立て（`tools/release/{build,package,check-artifacts,draft,publish-nightly,check-guards}.sh`）・workflow（`ci.yml`・`release.yml`・`nightly.yml`）・更新の試験（`tools/release/update-apply-install.sh`）がそこにある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。件数つきの検証は、試験を走らせた後に `-- --list` の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント、bash は `echo "== Scenario: …"`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と一字一句合わせる
- **この change の Scenario は 128 本**（すべて `distribution`）。新しい名前の Scenario は下の Task が全部名指しする。残りは今の台本・試験が印を持つ。そのうち中身（GIVEN の版・期待する値）を変えた Scenario は、印の場所の試験の値を直す（Task が名指しする）。印を消さない
- **本人の決定（第 1 回は推奨で書いた。答えが違えば上流が写し直してから下流へ渡す）**: Q1 番号は CI が通り前の番号から製品が変わった commit に付ける（D4（仮））/ Q2 versionCode は `左×10,000,000 + 真ん中×10,000 + 右`（D3（仮））/ Q3 間に確認バッチの merge があれば真ん中を上げる（D5）/ Q4 番号を持たない組み立ては 0.0.x（D6）。
  仮: D1（番号の組み立ては全部、下書きのリリースに残す）・D2（署名していない APK も残す）。D3・D4・D6 も（仮）で、第 1 回の答えが推奨と違えば `spec --answers` の段が design を直す
- **名前は統一する**: 左の数字のファイル `tools/release/MAJOR`・番号の印 `build-<番号>`・下書きの説明文の行 `ashiato-build: <番号> <commit の 40 桁>`・署名していない APK `ashiato-collector-<番号>-unsigned.apk`・番号付けの workflow `.github/workflows/number.yml`・
  台本の口 `tools/release/version.sh number <commit>`（番号の判定）・`tools/release/number.sh`（印を作り下書きに置く）。環境変数は今の `ASHIATO_BUILD_VERSION`・`ASHIATO_APK_UNSIGNED`・`ASHIATO_RELEASE`
- **workflow の YAML に判定の論理を書かない**（ST52 D8 の型）。判定は `tools/release/*.sh` に置き、`--self-test` が使い捨ての git の木・偽の `gh`（`PATH` の先に置く台本）で確かめる。workflow の形は `tools/release/check-guards.sh workflows` が読む
- **下流の実装の中では配布元に何も書かない**: 印 `build-*`・`v*`・`nightly` を push しない・下書きを作らない。配布元での確かめは末尾の「人間の確認待ち」で行う
- **版の検査の 4 か所（bash・Rust・gradle・TypeScript）は同じ規則**（D9）。versionCode の表（`tools/release/version-table.tsv`）と、受ける・断る版番号の表（`tools/release/version-check-table.tsv`）を 4 か所の試験が読む
- **main の HEAD を組まない**（番号付けは run の commit、開発版は最新の番号の commit を組む。D10・D11）
- **新しい Rust・Android・npm の依存を足さない**
- **長いコマンドは `scripts/quiet-run`**（cargo test・gradle）。Rust の試験の名前の接頭辞は `st61_`

```

## Task 1 が名指しする決定（逐語）

### D3 — 設計の判断（`design.md` の `### D<n>.`）

Source: `openspec/changes/st61-auto-version-bump/design.md:52` — ### D3（仮）. 携帯の versionCode は `左×10,000,000 + 真ん中×10,000 + 右`（deep.md Q2）

```md
### D3（仮）. 携帯の versionCode は `左×10,000,000 + 真ん中×10,000 + 右`（deep.md Q2）

後ろの `-beta.<N>`・`-dev.<日付>` は値に入れない。真ん中 999・右 9,999・左 209 を上限にし、超えれば組み立てを落とす（上限の `209.999.9999` は 2,099,999,999 で、Android の上限 2,100,000,000 の内側。P7）。

- 最初の番号 0.2.0 は 20,000 で、前の式の 0.1.1 の 10,199 より大きい（D13）
- 同じ番号の `-beta.N` と正式は同じ値になる。札の形では同じ番号は同じ組み立てなので、同じ値でよい。移る間に同じ番号のベータ版と正式版を両方出すと、Android は同じ値の上書きとして入れる
- 開発版は別のアプリ（識別子 `.dev`）なので、正式版と値を比べない
- 対応表 `tools/release/version-table.tsv` を新しい式で書き直し、`version.sh` と gradle の `verifyVersionCodeTable` が同じ表で確かめる（今の型のまま）
- 反転条件: Q2 の答えが「今の式のまま」→ 式を戻し、D5 の規則を「右が 99 を超えるときだけ真ん中を上げる」にする（Q2 と Q3 は結び付いている。今の式で確認バッチごとに真ん中を上げると約 5 か月で 99 を超える）。「通し番号」→ 台帳（印 `build-*`）の数から値を決める

```

### D9 — 設計の判断（`design.md` の `### D<n>.`）

Source: `openspec/changes/st61-auto-version-bump/design.md:124` — ### D9. 左の数字はファイル `tools/release/MAJOR`。`Cargo.toml` の版は `0.0.1`（premises の C4）

```md
### D9. 左の数字はファイル `tools/release/MAJOR`。`Cargo.toml` の版は `0.0.1`（premises の C4）

- `tools/release/MAJOR` は数字 1 つ（今は `0`）。本人が PR で変える（NFR-25。0.x → 1.0）。変えた PR が main に入って最初の番号が `<左>.0.0`（D5 の前に効く）
- 左を下げたら番号付けは止まる（番号は 1 列に並び、下がらない）
- 根の `Cargo.toml` の `[workspace.package] version` は `0.0.1` にし、番号を持たない組み立ての版にする（D6）。版の検査は「出所の版は `0.0.<N>`（N≥1）」を見る（旧い「MAJOR が 0」の検査を置き換える）
- API の契約の `info.version` は出所の版のまま（`0.0.1`）。番号にすると main に commit を積むことになる（FR-134）。契約の版の比べは封筒の写し（`docs/contract/envelope.json`）が持ち、`info.version` を読む口は `tools/check-openapi.sh` だけ
- 版の検査の 4 か所（bash・Rust・gradle・TypeScript）を同じ規則にする。受ける・断る版番号の表 `tools/release/version-check-table.tsv`（`<版番号>\t<出所の版>\t<左>\tok|ng`。`0.4.0-beta.2` ok・`1.4.0` ng・`0.0.2`（出所 `0.0.1`）ng・`0.0.0` ng など）を 4 か所の試験が読み、そろいを固定する（versionCode の表と同じ型。review/spec.md R17）。規則: 渡した版が無ければ出所の版。渡した版の 3 つの数字が出所の版と同じなら「番号を持たない版」、左がファイルの値で `<左>.<真ん中>` が `0.0` でなければ「番号」。後ろの形（`-beta.<1〜98>`・`-dev.<YYYYMMDD>`）は今のまま
- gradle と TypeScript と Rust は `tools/release/MAJOR` を読む（`Cargo.toml` を読むのと同じ方法で）
- 反転条件: 左の数字を変える PR を本人以外が出す運用になった → `MAJOR` の変更に本人の承認の規則を足す

```

### R17 — 独立レビューの指摘（`review/*.md`。**見出しだけ。**本文は必要な 1 件を読む）

- `openspec/changes/st61-auto-version-bump/review/spec.md:193` — R17. 版の検査の 4 つの実装（bash・Rust・gradle・TypeScript）が同じ D9 の規則を持つことを、gradle と TypeScript で確かめる試験が無い

（本文は引いていない。要る 1 件だけを上の `path:行` から読む）

## Task 1 が担保する Scenario（spec の逐語。test の印は一字一句これに合わせる）

### Scenario: versionCode は版番号の 3 つの数字から決まる

Source: `openspec/changes/st61-auto-version-bump/specs/distribution/spec.md:284` — ### Requirement: 携帯の versionCode は、番号の 3 つの数字から計算する

```md
#### Scenario: versionCode は版番号の 3 つの数字から決まる
- **WHEN** 版番号 `0.0.1`・`0.2.0`・`0.2.7`・`0.3.0`・`0.250.9999`・`1.0.0` で携帯の versionCode を計算する
- **THEN** それぞれ 1・20000・20007・30000・2509999・10000000 になる

```
既にある test の印: `tools/release/version.sh`

### Scenario: 後ろに -beta や -dev が付いても、同じ番号は同じ versionCode になる

Source: `openspec/changes/st61-auto-version-bump/specs/distribution/spec.md:288` — ### Requirement: 携帯の versionCode は、番号の 3 つの数字から計算する

```md
#### Scenario: 後ろに -beta や -dev が付いても、同じ番号は同じ versionCode になる
- **WHEN** 版番号 `0.4.0`・`0.4.0-beta.2`・`0.4.0-dev.20261101`・`0.4.0-dev.20261102` で携帯の versionCode を計算する
- **THEN** どれも 40000

```
既にある test の印: `tools/release/version.sh`

### Scenario: 最初の番号は、前の式の 0.1.x の APK より大きい

Source: `openspec/changes/st61-auto-version-bump/specs/distribution/spec.md:292` — ### Requirement: 携帯の versionCode は、番号の 3 つの数字から計算する

```md
#### Scenario: 最初の番号は、前の式の 0.1.x の APK より大きい
- **WHEN** 版番号 `0.2.0` で携帯の versionCode を計算する
- **THEN** 値は 20000 で、前の式の `0.1.1` の値 10199 より大きい

```
既にある test の印: `tools/release/version.sh`

### Scenario: 桁を超える版番号では、versionCode の計算が失敗する

Source: `openspec/changes/st61-auto-version-bump/specs/distribution/spec.md:296` — ### Requirement: 携帯の versionCode は、番号の 3 つの数字から計算する

```md
#### Scenario: 桁を超える版番号では、versionCode の計算が失敗する
- **WHEN** 版番号 `0.1000.0`・`0.1.10000`・`210.0.0` のどれかで携帯の versionCode を計算する
- **THEN** 計算は 0 でない終了コードで終わり、超えた部分が示される

```
既にある test の印: `tools/release/version.sh`

### Scenario: 出所の版が 0.0.x でないと、版の検査が落ちる

Source: `openspec/changes/st61-auto-version-bump/specs/distribution/spec.md:538` — ### Requirement: 3 つの部品の版番号は、番号か、番号を持たない出所の版から決まる

```md
#### Scenario: 出所の版が 0.0.x でないと、版の検査が落ちる
- **WHEN** 出所の版を `0.1.0`・`1.0.0`・`0.0.0` のどれかにして版の検査を走らせる
- **THEN** どれも検査は 0 でない終了コードで終わる

```
既にある test の印: `tools/check-version.sh`

### Scenario: 左の数字のファイルが数字 1 つでないと、版の検査が落ちる

Source: `openspec/changes/st61-auto-version-bump/specs/distribution/spec.md:546` — ### Requirement: 3 つの部品の版番号は、番号か、番号を持たない出所の版から決まる

```md
#### Scenario: 左の数字のファイルが数字 1 つでないと、版の検査が落ちる
- **WHEN** 左の数字のファイルを `0.1` か空にして版の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わる

```
既にある test の印: `tools/check-version.sh`

### Scenario: 出所の版が MAJOR.MINOR.PATCH の形でないと、版の検査が落ちる

Source: `openspec/changes/st61-auto-version-bump/specs/distribution/spec.md:542` — ### Requirement: 3 つの部品の版番号は、番号か、番号を持たない出所の版から決まる

```md
#### Scenario: 出所の版が MAJOR.MINOR.PATCH の形でないと、版の検査が落ちる
- **WHEN** 出所の版を `0.0.1-beta.1` にして版の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わる

```
既にある test の印: `tools/check-version.sh`

### Scenario: 出所の版の MAJOR が 0 でないと、版の検査が落ちる

Source: `openspec/specs/distribution/spec.md:596` — ### Requirement: 3 つの部品の版番号は、1 つの出所から決まる

```md
#### Scenario: 出所の版の MAJOR が 0 でないと、版の検査が落ちる
- **WHEN** 出所の版を `1.0.0` にして、検査の値を変えずに版の検査を走らせる
- **THEN** 検査は 0 でない終了コードで終わる

```
**test の印はまだ無い**（`scripts/check_scenarios.py` が FAIL にする）

## Task 1 の項目と検証（`scripts/verify-run <項目>` が本文の「検証:」をそのまま走らせる）

| 項目 | 済 | 工程 | 検証コマンド |
|---|---|---|---|
| 1.1 | [ ] | impl | `tools/release/version.sh --self-test` / `tools/release/version_test.sh` / `bash -c 'test "$(tools/release/version.sh code 0.2.0)" = 20000 && test "$(tools/release/version.sh code 1.0.0)" = 10000000 && ! tools/release/version.sh code 0.1.10000 2>/dev/null'` |
| 1.2 | [ ] | impl | `tools/check-version.sh --self-test` / `tools/check-version.sh` |

`impl` 以外（`pr` / `ci` / `human`）は `[ ]` のまま残るのが正常。FAIL / BLOCKED_INFRA を過去の値や別のコマンドで代えない。

## Task 1 の本文が名指しするパス

| パス | いま |
|---|---|
| `tools/release/MAJOR` | あり |
| `tools/release/version.sh` | あり |
| `tools/release/version-table.tsv` | あり |
| `tools/release/version-check-table.tsv` | あり |
| `tools/release/version_test.sh` | あり |
| `tools/check-version.sh` | あり |

読むのはここまで。**足りない分は原典の見出しを名指しで読む**（上の表）。コードの理解のための `rg` / `find` / 読みは制限しない —— 仕様を探すための広い探索だけをやめる。
