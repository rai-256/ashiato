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

## Fix round 1

- F1: 例外表の各行を個別に検証し、不正な行だけを `exception_table` 材料にして、依存のスキャンを継続するよう修正した。表の TOML 不正やロックファイル欠落など別の `ValueError` は表の issue に変換せず `rc=2` とした。
- F2: 自己試験を、期限切れ行と理由無し行を実際の `nightly` から `file-issues` へ渡す形に変更し、例外表 issue が 1 件で両行の ID・問題種別を含むことを確認するようにした。
- F3: 到達不能夜について `nightly` と `file-issues` の両方が `rc=2` で、材料ファイルが無く、偽の `gh` が create/comment を受けないことを確認するようにした。材料無しの `file-issues` は `rc=2` にした。
- F4: 偽の `gh` が create/comment の本文を呼び出しごとに保存し、依存・版・勧告 ID・重大度・URL の固定形式だけと完全一致することを自己試験で確認するようにした。

### Fix round 1 の検証

- `scripts/quiet-run task9-fix-green4 -- tools/check-vulns.sh --self-test`: PASS (`rc=0`, 40 行)
- `scripts/verify-run 9.1`: PASS (`rc=0`)
- `scripts/verify-run 9.2`: PASS (`rc=0`)
- `python3 -m py_compile tools/check-vulns.py tools/check-vulns-issues.py`: PASS
- `git diff --check`: PASS

### Fix round 1 の自己レビュー

- F1〜F4 の未解決指摘だけを対象に、実際の nightly→file-issues 経路と偽の `gh` の本文を検査している。
- `tasks.md`、workflow、既存の期待値は変更していない。
- 追加の懸念はない。
