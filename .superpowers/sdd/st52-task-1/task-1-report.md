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

## Fix round 1 report

### 対応内容

- F1: 否定シナリオを `if (...)` で終了コードとして検証し、main非祖先・浅い木では期待する出力も独立に検証した。nightly浅い木では `build=` が無いことも検証するようにした。
- F2: beta番号を 1〜98 に制限し、`beta.99` の `code` と `plan` を拒否する自己テストを追加した。
- F3: nightly浅い木の `build=` 行が無いことを自己テストで検証するようにした。
- F4: version order のbeta番号をゼロ埋めした数値キーにし、beta.2・beta.9・beta.10の順序を自己テストに追加した。
- F5: `origin/main` と `main` のどちらも無い場合に `HEAD` へフォールバックせず失敗するようにし、その自己テストを追加した。

### Fix round tests

- RED: `bash tools/release/version.sh --self-test` — beta.99拒否の追加ケースで、修正前は rc=1（期待どおりの失敗）。
- GREEN: `bash tools/release/version.sh --self-test` — rc=0、全Scenario出力を確認。
- `bash -n tools/release/version.sh` — PASS。
- `bash tools/release/version_test.sh` — PASS。
- `scripts/verify-run 1.2` — PASS。
- `scripts/verify-run 1.1` — BLOCKED_INFRA。`tools/verify-env.sh` が rc=1 で検証準備に失敗し、self-test本体は実行されなかった。証跡は `openspec/changes/st52-release-channels/evidence.jsonl` に記録済み。独立に実行した `bash tools/release/version.sh --self-test` はPASS。

### Fix round files

- `tools/release/version.sh`
- `openspec/changes/st52-release-channels/evidence.jsonl`

### 自己レビュー

F1〜F5の指摘範囲だけを変更した。レビューで確認対象外とされたCI job配線とTask 8のD6引き継ぎは変更していない。

## Fix round 2 report

### 対応内容

F1〜F5は前回の修正コミット `ed7e0e80` で既に対応済みだったため、コードの追加変更は行わず、現在の木で再検証した。否定シナリオは `if ...; then return 1; fi` と出力の独立検査になっており、beta.99拒否、beta順序、main参照欠如、nightly浅い木での `build=` 不在も確認できる。

### Fix round 2 tests

- `bash -n tools/release/version.sh && bash tools/release/version.sh --self-test` — PASS。17個のScenarioを出力。
- `bash tools/release/version_test.sh` — PASS。
- `bash -c 'test "$(tools/release/version.sh code 0.3.0-beta.2)" = 30002 && test "$(tools/release/version.sh code 1.0.0)" = 1000099'` — PASS。
- `scripts/verify-run 1.1` — `tools/verify-env.sh` が rc=1 で `BLOCKED_INFRA`（self-test本体は未実行）、後段のversionCode検査は PASS。証跡に記録済み。
- `scripts/verify-run 1.2` — PASS。

### Fix round 2 files

- `openspec/changes/st52-release-channels/evidence.jsonl`
- `.superpowers/sdd/st52-task-1/task-1-report.md`

### 自己レビュー

F1〜F5の指摘に対する実装は既存の修正コミットにあり、今回の作業で対象外のコード変更は加えていない。CI job配線とTask 8のD6引き継ぎはレビュー記載どおり変更していない。
