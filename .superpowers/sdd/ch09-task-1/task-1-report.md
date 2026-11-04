# Task 1 レポート

## 実装

- `tools/load-repeat.sh` を追加した。`--times`、`--load host|db|none`、`--expect-fail`、`--self-test` を実装し、各回の rc・所要時間・`pass k/N` を出す。
- host の `nproc` 本の busy loop と DB コンテナ内の `ch09-load-<pid>` 負荷を `EXIT` / `INT` / `TERM` で停止する。
- 3 つの負荷台本 Scenario を self-test に追加した。DB が無い場合は `DB が要る` と rc=1 で停止する。
- `openspec/changes/ch09-load-tolerant-timing-tests/load-proof.sh` と `load-before.txt` を追加し、指定された 2 試験を host 負荷下で 10 回実行して結果を記録した。

## TDD Evidence

### RED

`tools/load-repeat.sh --self-test` を実装前に実行し、`rc=127`（`No such file or directory`）を確認した。

### GREEN

- `tools/load-repeat.sh --self-test` — rc=0
- `scripts/verify-run 1.1` — PASS、rc=0
- `scripts/verify-run 1.2` — PASS、rc=0

## 変更ファイル

- `tools/load-repeat.sh`
- `openspec/changes/ch09-load-tolerant-timing-tests/load-proof.sh`
- `openspec/changes/ch09-load-tolerant-timing-tests/load-before.txt`
- `openspec/changes/ch09-load-tolerant-timing-tests/evidence.jsonl`

## Self-review

- 一時ファイルは `mktemp -d` で作成し、trap で削除する。
- `/tmp` 固定名への書き込みは行っていない。
- 製品コード、既存試験、`tasks.md` は変更していない。
- `load-before.txt` の履歴・走査はいずれも host 負荷で再現を記録した。各回の rc も同ファイルに残している。

## 注意事項

初期の検証では `.env` 不在による BLOCKED_INFRA と、実装途中の FAIL が記録された。最新の正式検証では実値の一時 `.env` を用意して 1.1/1.2 とも PASS を確認し、検証後に `.env` は削除した。
