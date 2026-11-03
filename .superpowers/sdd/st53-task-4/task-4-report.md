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
