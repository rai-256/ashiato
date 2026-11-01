# Task 1 実装報告

## 実装したもの

- `tools/release/version.sh` を追加し、`source`・`check`・`code`・`order`・`plan`・`nightly-plan`・`--self-test` を実装した。
- versionCode の版番号計算、beta/正式/dev の順序、MINOR/PATCH の上限、出所の版一致を実装した。
- タグの形・main の祖先・修正の枝の移行/封筒差分・浅い履歴を判定する `plan` を実装した。
- 日本時間の日付、nightly と main の差分、公開設定、正式版公開後の出所版更新要求を判定する `nightly-plan` を実装した。
- `tools/release/version-table.tsv` と、自己テスト用の `version_test.sh` を追加した。
- `CLAUDE.md` の tools 構成に `release/` を追加した。

## テスト結果

- `scripts/verify-run 1.1`: PASS（`version.sh --self-test`、指定 versionCode 検証）
- `scripts/verify-run 1.2`: PASS
- `bash tools/release/version_test.sh`: PASS
- `bash -n tools/release/version.sh tools/release/version_test.sh`: PASS
- `python3 scripts/check_scenarios.py .`: FAIL。後続 Task が担保する ST52 全体の未実装 Scenario 63 件が残るためで、Task 1 の対象 Scenario は自己テストで印を付けている。

## TDD Evidence

### RED

`bash tools/release/version_test.sh` を実装前に実行し、`tools/release/version.sh: No such file or directory` で失敗した。対象台本が無いことによる期待どおりの失敗だった。

### GREEN

実装後に `bash tools/release/version_test.sh` と `scripts/verify-run 1.1` を実行し、いずれも PASS。`--self-test` は versionCode、タグ判定、浅い木、nightly 判定の Scenario を実行した。

## 変更ファイル

- `tools/release/version.sh`
- `tools/release/version-table.tsv`
- `tools/release/version_test.sh`
- `CLAUDE.md`
- `openspec/changes/st52-release-channels/evidence.jsonl`（`verify-run` の証跡）

## 自己レビュー

版番号の出所一致、浅い履歴の拒否、nightly の日付と公開設定を確認した。Task 1 の範囲外である workflow、Gradle、配布物検査は変更していない。

## 懸念

`check_scenarios.py` 全体は後続 Task の Scenario が未実装のため FAIL する。Task 1 の指定検証はすべて PASS。
