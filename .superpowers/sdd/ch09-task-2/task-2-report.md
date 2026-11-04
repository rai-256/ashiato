# Task 2 実装報告

## 実装内容

- `tools/break-check.sh` を追加した。
- `--patch` で HEAD から detached worktree を作り、差分を適用して `--` 以降の命令を実行する。
- `--expect` の複数文言、既定 900 秒の timeout、`CARGO_TARGET_DIR` の既定値（元の木の `target/`）、失敗時の判定を実装した。
- `--self-test` で指定された 4 つの Scenario（成功条件、通過・文言違い・不適用差分、後始末、timeout）を確認する。

## テスト結果

- `bash -n tools/break-check.sh` — PASS
- `tools/break-check.sh --self-test` — PASS（rc=0）
- `scripts/verify-run 2.1` — PASS（rc=0、evidence.jsonl に記録）
- `git diff --check` — PASS

## TDD Evidence

### RED

- `tools/break-check.sh --self-test` — rc=127、対象スクリプトが未実装のため失敗した。

### GREEN

- `tools/break-check.sh --self-test` — rc=0。4 Scenario の出力と、期待どおり失敗する命令の検出を確認した。

## 変更ファイル

- `tools/break-check.sh`
- `openspec/changes/ch09-load-tolerant-timing-tests/evidence.jsonl`

## 自己レビュー

- 製品コード、`tasks.md`、既存 Scenario のテストは変更していない。
- 一時 directory と worktree は `trap` で必ず削除・prune する。
- 懸念なし。

## Fix report（review 1: F1 / F2）

### 修正内容

- `--self-test` の「通った」「文言が違う」「差分が当たらない」「上限で殺した」の各ケースで、rc=1 と対応する診断文言を個別に検証するようにした。
- timeout ケースでは実行経過時間も確認し、短絡的な別エラーを timeout 成功と誤認しないようにした。
- worktree と一時ディレクトリの cleanup を `RETURN` trap から `EXIT` trap に変更し、INT/TERM は終了経由で cleanup するようにした。関数終了後も変数が残るよう cleanup 用の共有変数を使った。

### 検証

- `bash -n tools/break-check.sh` — PASS
- `tools/break-check.sh --self-test` — PASS（rc=0、4 Scenario と cleanup を確認）
- `scripts/verify-run 2.1` — PASS（rc=0、evidence.jsonl に記録）
- `git diff --check` — PASS

### Self-review

- F1/F2 以外の機能・ファイルは変更していない。
- review で後続 Task に委ねられている実差分の空走確認は実施していない。
