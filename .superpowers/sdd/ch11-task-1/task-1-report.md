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

- ネットワーク障害のため、実 osv-scanner による 1.2 の PASS 証跡がない。controller はネットワーク復旧後に 1.2 を再実行する必要がある。
