# Task 3 実装報告

## 実装内容

- `tools/check-vulns.sh --base <commit>` を追加し、base の commit の lockfile と例外表を `git show` で一時ディレクトリへ展開して、(ecosystem, package, advisory ID) の差分を検査するようにした。
- `新しく持ち込んだ: <n> 件` と、例外表の追加・期限変更・理由変更の一覧を標準出力へ出し、`GITHUB_STEP_SUMMARY` が設定されている場合は同じ一覧を追記するようにした。
- base に無い lockfile / 例外表を空として扱い、base commit の解決失敗は rc=2 とした。
- `tools/release/changes.sh deps` と、Task 3 の 9 Scenario（既存 Scenario の印を含む自己試験）を追加した。

## TDD Evidence

### RED

`scripts/quiet-run task3-red -- tools/check-vulns.sh --self-test`

rc=1。既存 16 Scenario の後、最初の新規 Scenario「勧告のある版を足した PR は落ちる」で停止した。未実装の `--base` を検出する期待どおりの失敗。

### GREEN

`scripts/quiet-run task3-final-verification -- tools/check-vulns.sh --self-test`

rc=0。Task 3 の PR 差分、例外表一覧、例外表検査、osv.dev 到達不能、`deps` 判定の Scenario が通過した。

## 検証

- `scripts/verify-run 3.1` — rc=0、evidence.jsonl に記録。
- `scripts/verify-run 3.2` — rc=0、evidence.jsonl に記録。
- `git diff --check` — rc=0。
- `HX_CHANGE=ch11-dependency-vulnerability-scan python3 scripts/check_scenarios.py .` — change 全体では、後続 Task の未実装 Scenario 25 件が未担保として FAIL。Task 3 追加分は `tools/check-vulns.sh --self-test` で担保済み。

## 変更ファイル

- `tools/check-vulns.py`
- `tools/check-vulns.sh`
- `tools/release/changes.sh`
- `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl`

## 自己レビュー

指定範囲外の workflow・tasks.md・spec は変更していない。base の比較は advisory の版を比較キーに含めず、同じ advisory が残る版更新を新規扱いしない。自己試験の一時ディレクトリは `mktemp` と `trap` で管理している。

## 懸念

Scenario checker の change 全体 FAIL は、この Task が担保する範囲外の未実装 Task によるもの。Task 3 の検証コマンド自体は両項目 rc=0。
