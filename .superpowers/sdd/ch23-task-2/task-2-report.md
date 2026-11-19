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

## Fix round 1 対応

### 修正内容

- F1: 「担当者を付けられなければ、担当者無しで立て直す」の自己試験を依存 2 件（異なる題）と通知題の 3 題に拡張し、各題について assignee 付き作成の直後に同じ題を assignee 無しで 1 回だけ作成する順序を `gh_log` で検査するようにした。偽 gh の本文改行で引数が分割されるため、assignee 再試行時だけログの改行をエスケープした。
- F2: 配線検査を `issues` job 全体の部分文字列照合から step 単位の検査へ変更し、`file` step より後の通知 step 自身に正しい `if:`、run URL、`GH_TOKEN` があることを検査するようにした。自己試験に通知 step の `GH_TOKEN` 欠落ケースを追加した。

### Fix round の検証

- `bash -n tools/check-vulns.sh tools/check-vulns-wiring.sh`: PASS
- `tools/check-vulns-wiring.sh --self-test`: PASS
- `scripts/verify-run 2.1`: PASS（self-test を含む）
- `scripts/verify-run 2.2`: PASS（配線自己試験、Scenario、既存 job 前半の不変検査）
- `scripts/verify-run 2.3`: PASS（self-test を含む）
- `scripts/quiet-run vulns-final -- tools/check-vulns.sh --self-test`: rc=0
- `git diff --check`: PASS

### Fix round の自己レビュー

- 変更はレビュー指摘 F1/F2 の自己試験・配線検査に限定した。
- 本番 workflow、依存処理、issue 作成本体の振る舞いは変更していない。
- Actions 実行環境での `failure() && steps.file.outcome == 'failure'` の評価は、レビュー記録どおりこの Task の自己試験では判定していない。
