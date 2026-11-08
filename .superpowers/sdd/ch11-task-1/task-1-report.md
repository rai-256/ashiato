# Task 1 report

## 実装

- `tools/check-vulns.sh` を追加し、osv-scanner v2.7.0 の版・Linux x86_64 sha256 を固定した。
- 実行時にも `OSV_SCANNER_BIN` を含む実行ファイルの sha256 を検証し、不一致は rc=2 で実行しないようにした。
- `tools/check-vulns.py` で標準ライブラリだけを使い、3 種類のロックファイルの存在確認、JSON の読み取り、`osv-scanner.toml` の例外適用、rc 0/1/2 を処理するようにした。
- 例外表の空テンプレート `osv-scanner.toml` と、Task 1 の自己試験シナリオ印を追加した。

## テスト

- `python3 -m py_compile tools/check-vulns.py` — PASS
- `scripts/verify-run 1.1` — PASS (`bash -n tools/check-vulns.sh`, rc=0)
- `scripts/verify-run 1.2` — FAIL。公式 osv-scanner asset の取得が HTTP 504 となり、自己試験の実行基盤に到達しなかった。
- `scripts/quiet-run ch11-task1-selftest -- tools/check-vulns.sh --self-test` — FAIL/BLOCKED_INFRA。GitHub release asset の取得が HTTP 504。

## TDD Evidence

- RED: `tools/check-vulns.sh --self-test` — rc=127、対象台本が存在しないため失敗。
- GREEN: 実バイナリ取得後の自己試験は、外部 asset が 504 のため未確認。従って完了とは判定していない。

## Files changed

- `tools/check-vulns.sh`
- `tools/check-vulns.py`
- `osv-scanner.toml`
- `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl`

## Concerns

- なし。公式 API asset の取得と osv.dev への問い合わせを含む自己試験が PASS した。

## Fix report

- 原因: 例外表を scanner の `--config` に渡していたため、例外ケースで scanner 自身が rc=2 になっていた。また dev dependency を JSON に含める `--all-packages` が不足していた。
- 修正: scanner には `/dev/null` の空設定を渡し、`--all-packages` を追加した。例外の判定は引き続き Python 側で行う。
- 検証: `scripts/quiet-run ch11-task1-selftest -- tools/check-vulns.sh --self-test`。公式 asset の取得が HTTP 504 のため BLOCKED_INFRA。

## Fix report 2

- 原因: GitHub の browser download URL が HTTP 504 になっていた。公式 GitHub API の asset endpoint は同じ固定 sha256 の実行ファイルを取得できた。また自己試験の dev lockfile は scanner が解決済み package として扱う `resolved` を欠いていた。
- 修正: 公式 API の octet-stream endpoint と固定 asset ID を使い、自己試験の dev package に `resolved` を追加した。
- 検証: `scripts/quiet-run ch11-task1-selftest -- tools/check-vulns.sh --self-test` — PASS、rc=0、9 Scenario 印を確認。

## Final verification

- `scripts/verify-run 1.1` — PASS、rc=0（変更後コード）。
- `scripts/verify-run 1.2` — PASS、rc=0、9 Scenario 印を確認。

## Fix report 1（review 1 の F1〜F9）

- 変更: 自己試験の Rust 依存をリポジトリに無い `ansi_term` へ変更し、選択した全依存のロックファイル不在を台本内で確認した。3 ロックファイルの名前・勧告 ID、開発依存、informational、low severity、キャッシュ内偽バイナリ、欠落 lockfile、osv.dev 到達不能について、rc と出力を検査する専用ケースを追加した。自己試験の一時木を Git repo にして gradle lockfile の `git ls-files` 経路を検証し、`trap` で cleanup するようにした。
- 変更: Python の scanner 失敗・JSON 失敗・osv.dev 問い合わせ失敗の理由を保持して表示し、gradle lockfile が 0 件の実リポジトリを rc=2 とした。
- 検証: `scripts/verify-run 1.1` — PASS、rc=0、`bash -n tools/check-vulns.sh`。
- 検証: `scripts/verify-run 1.2` — PASS、rc=0、9 Scenario 印を確認。
- 自己試験: `scripts/quiet-run ch11-task1-selftest-fix5 -- tools/check-vulns.sh --self-test` — PASS、rc=0、42 秒。
- 自己レビュー: F1〜F9 の指摘箇所を再確認し、レビュー対象外の版固定・最終レビュー確認事項は変更していない。
