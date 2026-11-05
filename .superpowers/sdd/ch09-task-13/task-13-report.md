# Task 13 レポート

## 実装したもの

- `check-product-unchanged.sh` を追加。`origin/main` との分岐点から、製品コードの試験モジュール外の差分を検出し、`--self-test` で製品差分と試験モジュール内差分を検査する。
- `check-test-names.sh` を追加。Rust / Kotlin / 画面単体 / e2e / Windows 実行時試験の試験数と名前、および台本の `Scenario:` 印が分岐点から減っていないことを検査する。
- `load-proof.sh after` を追加。`load-before.txt` の負荷モードで history と scan の試験を各 10 回実行し、`load-after.txt` に全出力を保存する。

## テスト結果

- `check-product-unchanged.sh --self-test`: PASS
- `check-product-unchanged.sh`: PASS（製品コード変更なし）
- `check-test-names.sh`: PASS（Rust 2274→2274、Kotlin 777→777、画面単体 491→492、e2e 228→229、Windows 15→15）
- `load-proof.sh after`: PASS（history=db 10/10、scan=host 10/10）
- `cargo test --workspace`: PASS
- `python3 scripts/check_scenarios.py . ch09-load-tolerant-timing-tests`: PASS
- `python3 scripts/check_agreed_tests.py . --change ch09-load-tolerant-timing-tests`: PASS

## TDD Evidence

- RED: 未実装時に 13.1 / 13.2 の台本が存在せず、両方とも rc=127 で失敗した。
- GREEN: 実装後、13.1 と 13.3、13.4 を `scripts/verify-run` で実行し、すべて PASS の証跡を `evidence.jsonl` に記録した。13.2 は本文に検証コマンドがないため、台本を直接実行して PASS を確認した。

## 変更ファイル

- `openspec/changes/ch09-load-tolerant-timing-tests/check-product-unchanged.sh`
- `openspec/changes/ch09-load-tolerant-timing-tests/check-test-names.sh`
- `openspec/changes/ch09-load-tolerant-timing-tests/load-proof.sh`
- `openspec/changes/ch09-load-tolerant-timing-tests/load-after.txt`
- `openspec/changes/ch09-load-tolerant-timing-tests/evidence.jsonl`
- `.superpowers/sdd/ch09-task-13/task-13-report.md`

## 自己レビュー

製品コード、既存試験、`tasks.md` は変更していない。台本の一時ディレクトリは `mktemp -d` と `trap` で後始末する。Scenario 検査には既存の警告があるが rc=0 で、今回の Scenario は担保されている。

懸念事項はない。

## Fix report（review 1 / F1〜F4）

- `check-product-unchanged.sh` は、HEAD と分岐点それぞれの Rust ファイルで、直後に `mod tests` が来る `#[cfg(test)]` を境界として使うよう修正した。差分の旧行・新行を各版の境界で判定し、ファイルごとに行番号を初期化し、diff ヘッダーを正しく除外した。self-test に非境界の `#[cfg(test)]`、複数ファイル、試験モジュール内変更のケースを追加した。
- `load-proof.sh` は `before` の `--expect-fail` / `load-before.txt` 作成処理を復元し、`after` と併存させた。
- 検証: `scripts/verify-run 13.1` PASS（rc=0）、`scripts/verify-run 13.3` PASS（rc=0、両試験 pass 10/10）、`scripts/verify-run 13.4` PASS（workspace / Scenario / agreed tests）、`check-test-names.sh` PASS。
- 変更ファイル: `check-product-unchanged.sh`、`load-proof.sh`、`load-after.txt`、`evidence.jsonl`。
