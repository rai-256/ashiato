# Task 2 レポート

## 実装したこと

- `tools/check-vulns-issues.py` に `report-unfiled <run URL>` を追加した。
  - ラベル無しで開いた issue を検索し、決まった題の issue を新規作成またはコメント追記する。
  - 本文は UTC 日付と run URL の 2 行だけにした。
  - `GITHUB_REPOSITORY_OWNER` があれば新規 issue に担当者を付け、失敗時は担当者無しで 1 回だけ再試行する。
- `tools/check-vulns.sh` の偽 `gh` と自己試験に、report-unfiled、担当者、再試行の Scenario を追加した。
- `.github/workflows/vulns-nightly.yml` に `file` step の ID と、file-issues 失敗時だけ走る report-unfiled step を追加した。
- `tools/check-vulns-wiring.sh` に配線の検査、Scenario の印、4 種の破壊自己試験を追加した。

## テスト結果

- `tools/check-vulns-wiring.sh --self-test`: PASS
- `scripts/quiet-run vulns-green3 -- tools/check-vulns.sh --self-test`: PASS
- `scripts/verify-run 2.1`: PASS（Scenario 名確認、self-test）
- `scripts/verify-run 2.2`: PASS（配線自己試験、Scenario、issues job より前の差分なしと `exit "$rc"` 2 件）
- `scripts/verify-run 2.3`: PASS（Scenario 名確認、self-test）
- `python3 -m py_compile tools/check-vulns-issues.py`: PASS
- `git diff --check`: PASS

## TDD Evidence

### RED

- `scripts/quiet-run vulns-red -- tools/check-vulns.sh --self-test`
- 新しい `report-unfiled` Scenario で `error: 検査できなかった（使い方）` となって終了した。自己試験が未実装の report-unfiled を期待して失敗したため、機能欠落を検出できている。

### GREEN

- `scripts/quiet-run vulns-green3 -- tools/check-vulns.sh --self-test`
- rc=0。report-unfiled の新規作成、ラベル非使用、本文、既存 issue への追記、作成失敗、引数エラー、および担当者の 4 Scenario が通った。

## 変更ファイル

- `.github/workflows/vulns-nightly.yml`
- `tools/check-vulns-issues.py`
- `tools/check-vulns.sh`
- `tools/check-vulns-wiring.sh`
- `openspec/changes/ch23-vulns-nightly-issue-fix/evidence.jsonl`

## 自己レビュー

- 仕様で指定された変更対象だけを変更した。
- 新規 issue 作成時だけ担当者を付け、既存 issue へのコメントには担当者やラベル処理を追加していない。
- 初回の並列 `verify-run` では Docker のコンテナ削除競合により準備が `BLOCKED_INFRA` になったが、直列で再実行して全項目 PASS を確認した。

## 懸念

なし。
