# Task 2 レポート

## 実装内容

- `ci.yml` の `build-stamp`・`android`・`android-instrumented`・`package-linux` に、既存の `actions/setup-java` を使った Gradle の取り置きと、指定された 3 種類の依存パスを追加した。
- `ci.yml` の `chain` の checkout 直後に、既存の `actions/cache` の SHA 固定段を追加し、`~/.gradle/caches` と `~/.gradle/wrapper` を指定された hash 鍵で取り置くようにした。
- `nightly.yml` と `release.yml` の `package-linux` に同じ `setup-java` の取り置きを追加した。

## テストと検証結果

- `scripts/verify-run 2.1`: PASS（rc=0、7 秒）
- `scripts/verify-run 2.2`: PASS（rc=0、116 秒）
- `scripts/verify-run 2.3`: PASS（rc=0、76 秒）
- 証跡は `openspec/changes/ch25-ci-flake-gradle-cache-clock-test/evidence.jsonl` に記録済み。

## TDD Evidence

- RED: `python3 openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py ci` は、7 job が Gradle の部品の取り置きを持たないとして rc=1 になった。これは実装前の期待された失敗。
- GREEN: 同じ検証は実装後に `OK: ci`、rc=0 になり、`scripts/verify-run 2.1` でも PASS を記録した。

## 変更ファイル

- `.github/workflows/ci.yml`
- `.github/workflows/nightly.yml`
- `.github/workflows/release.yml`
- `openspec/changes/ch25-ci-flake-gradle-cache-clock-test/evidence.jsonl`
- `.superpowers/sdd/ch25-task-2/task-2-report.md`

## 自己レビュー

- `git diff --check` は問題なし。
- `check-ch25.py ci` が job 集合、取り置き以外の差分、workflow 外の差分、SHA 固定、Android 側の未変更を確認して PASS した。
- 依存の版、ロック、Gradle を使わない job、job の条件と段の順序は変更していない。

## 懸念

なし。
