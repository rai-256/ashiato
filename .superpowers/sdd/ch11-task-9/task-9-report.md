# Task 9: 毎晩の検査と issue

## 実装

- `tools/check-vulns.sh nightly <置き場>` を追加し、検査結果を依存ごとの JSON 材料として保存する。検査不能 (`rc=2`) では材料を作らない。
- `tools/check-vulns.sh file-issues <置き場>` を追加し、ラベルの作成、依存ごとの重複判定、未記録の勧告だけの追記を行う。既存 issue の自動 close は行わない。
- 例外表の問題と PostgreSQL minor の材料も同じ入口で issue 化する。
- `.github/workflows/vulns-nightly.yml` を追加し、検査 2 job と `always()` の issue job を UTC 15:30 に配線した。`nightly.yml` は変更していない。

## TDD Evidence

### RED

- コマンド: `scripts/quiet-run task9-red -- tools/check-vulns.sh --self-test`
- 結果: `rc=1`。新しい Scenario の開始後に、未実装の `nightly` 入口が「使い方」で失敗した。

### GREEN

- コマンド: `scripts/verify-run 9.1`
- 結果: `PASS rc=0`。Task 9.1 の自己試験全体が成功した。

## 検証

- `scripts/verify-run 9.1`: PASS (`rc=0`)
- `scripts/verify-run 9.2`: PASS (`rc=0`)
- `python3 -m py_compile tools/check-vulns-issues.py`: PASS
- `git diff --check`: PASS

## 変更ファイル

- `tools/check-vulns.sh`
- `tools/check-vulns-issues.py`
- `.github/workflows/vulns-nightly.yml`
- `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl`（verify-run の証跡）

## 自己レビュー

- 検査不能時に issue 材料を作らないこと、既存 issue の本文・コメントを重複判定に使うこと、依存ごとの集約、例外表・PostgreSQL 材料の個別処理を確認した。
- `nightly.yml` への変更は無く、issue job だけが `issues: write` を持つ。
- 追加の懸念はない。
