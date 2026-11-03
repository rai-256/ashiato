# Task 4 実装報告

## 実装内容

- `ashiatoDependencyPoms` を追加し、`releaseRuntimeClasspath` のモジュールごとに `@pom` を detached configuration で解決するようにした。
- POM の `<licenses>` を読み、無ければ親を最大 5 段辿り、座標・名前・URL・出所を TSV に書き出すようにした。POM 解決失敗は部品を飛ばさず task を失敗させる。
- `ashiato.extraRepo` を優先する Maven repository として追加した。
- POM 許諾名の対応表と Android の許諾検査を追加した。名指し外の `PROPRIETARY`、未知名、許諾なし、AND 条件違反を報告して失敗させる。
- Android の自己試験に、名指し・未知名・二重許諾・正本変更・親 POM・POM 取得失敗・空対象の Scenario を追加した。

## TDD Evidence

- RED: `tools/check-licenses.sh --self-test`。Android 節が未実装だったため、最初の `名指しの外の Google のライブラリを足すと、許諾の検査が落ちる` Scenario で失敗した。
- GREEN: `tools/check-licenses.sh --self-test`。全 Scenario が実行され `self-test OK`。
- GREEN: `scripts/verify-run 4.1`、`4.2`、`4.3` がすべて `PASS`。

## 変更ファイル

- `collector-android/app/build.gradle.kts`
- `collector-android/settings.gradle.kts`
- `tools/licenses/pom-license-names.tsv`
- `tools/check-licenses.sh`
- `openspec/changes/st53-license-bundle/evidence.jsonl`

## 自己レビュー

- online/offline の POM 出力は一致し、55 部品を確認できた。
- Android の実依存で `PROPRIETARY` として通る部品は、正本に名指しされた Google の 4 座標だけであることを確認した。
- 検証コマンドの指定により `$TMPDIR` に POM 出力が残る注意が `verify-run` に記録された。実装側の自己試験用一時ディレクトリは `mktemp -d` と終了時 trap で管理している。

## Fix report（review 1）

### 対応

- F1: Android 節に 4.3 の 2 Scenario の印を追加した。
- F2: 親 POM の self-test を、実 Gradle task・使い捨て Maven repository・`-Pashiato.extraRepo`・コピーした Android 木で実行し、出力の `parent` と Apache-2.0 判定を検証するようにした。
- F3: `.module` と jar（および Android variant の artifact）だけを置いた部品を実 Gradle で cache した後、`--offline` の task で POM 欠落を検証し、座標と POM 欠落を確認するようにした。偽の gradlew は削除した。
- F4: Android 節は `gradlew` を直接実行し、失敗抽出は `grep` にした。`scripts/quiet-run` と `rg` への依存を除いた。
- F5: TSV の列数が 4 でない行を即時失敗にし、POM の空白列を Kotlin 側で trim・連続空白の単一空白化するようにした。

### 検証

- RED: review 前の self-test は欠落 POM Scenario が偽の gradlew で成功扱いになった。実 Gradle の欠落 POM 構成へ置き換え後、途中の `releaseRuntimeClasspath` 解決が成立し、`--offline` では `Could not find <座標>.pom` で失敗することを確認した。
- GREEN: `TMPDIR=/tmp/ashiato-task4-tmp scripts/quiet-run task4-self-test-pass -- tools/check-licenses.sh --self-test` — rc=0、`self-test OK`。
- GREEN: `scripts/verify-run 4.3` — 2 項目とも PASS（Android 実依存、self-test）。実依存の確認は 55 件・不許可 0 件、検証コマンドの proprietary 件数は 4。
- `python3 scripts/check_scenarios.py . st53-license-bundle` は、この Task 外の未実装 Scenario 27 件で FAIL。F1 の Android 2 Scenario の印不足は解消済みで、他の Task の不足は変更していない。

### 変更ファイル

- `collector-android/app/build.gradle.kts`
- `tools/check-licenses.sh`
- `openspec/changes/st53-license-bundle/evidence.jsonl`

### 自己レビュー

- 指摘された F1〜F5 以外の仕様・Task は変更していない。
- `verify-run 4.3` の self-test では一時フォルダ 2 件の残留注意が記録された。self-test の親ディレクトリ自体は `mktemp -d` と `RETURN` trap で管理しているが、Gradle が生成する内部一時物の残留は既存の Gradle 道具由来であり、今回の範囲では変更していない。
