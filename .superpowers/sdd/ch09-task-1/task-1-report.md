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

## Fix round 1

### 対応

- `tools/load-repeat.sh` の DB 負荷を `exec -a` から、`ch09-load-<pid>` を argv に渡す busy loop へ変更した。DB 内の印を待ってから実行を開始し、終了時は印で停止する。
- self-test の中断 Scenario は、印が 1 本以上立ったことを確認してから `INT` を送り、host と DB の印が 0 本になったことを確認するようにした。通常の Scenario 後も印で残留を検査する。
- `load-proof.sh` は各試験の全出力を `load-before.txt` に保存し、失敗時の panic 行を後から確認できるようにした。

### TDD Evidence

- RED: 印の残留検査を self-test に追加した状態で `tools/load-repeat.sh --self-test` を実行し、DB 環境変数が無い状態では `DB が要る: self-test の db 負荷を確認できない` で rc=1 となった。DB 負荷の検査を実行できない前提条件の失敗であることを確認した。
- GREEN: `scripts/verify-run 1.1` — PASS、rc=0。
- GREEN: `scripts/verify-run 1.2` — PASS、rc=0。`load-before.txt` に各回の rc と cargo test の全出力、scan の panic 行を保存した。

### Fix report

- 変更: DB 負荷の argv 印、印の開始待ち、INT 前後の残留検査、試験出力の保存。
- 実行: `bash -n tools/load-repeat.sh`; `bash -n openspec/changes/ch09-load-tolerant-timing-tests/load-proof.sh`; `git diff --check`; `scripts/verify-run 1.1`; `scripts/verify-run 1.2`。
- 結果: shell 構文・差分検査は rc=0、項目 1.1/1.2 はいずれも PASS。

### Self-review correction

今回の再現では history は `yes`（db）、scan は `yes`（host）となった。`load-before.txt` はこの結果に更新し、history の panic 行と scan の失敗理由 `走査が 4 回に届かない` / `走査が 5 回に届かない` を含む出力を残した。
