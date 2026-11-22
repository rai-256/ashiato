# Task 9: 全体

## 実装内容

- ST59 のサーバテストを rustfmt 準拠に整えた。
- clippy で検出された未使用のテスト補助メソッドを削除した。
- 環境変数をテスト間で保護する mutex が await をまたぐ2つのテストに、意図を明示する clippy 許可を付けた。
- 真偽値リテラルとの `assert_eq!` を `assert!` に直した。
- 書き込み失敗 Scenario の e2e 印を spec の両方の Scenario 名と一致させた。

## テスト

`scripts/verify-run 9.1` を実行し、次の2系統が PASS（rc=0）になった。

- `cargo fmt --all -- --check`、workspace clippy、Windows target の2 crate clippy、`cargo test --workspace`: PASS（668秒）
- `tools/check-openapi.sh`、`tools/check-log-private.sh`、`tools/check-secrets.sh`、`python3 scripts/check_scenarios.py . st59-change-password-from-web`: PASS（37秒、`scenarios: OK`）

## TDD Evidence

- RED: 初回 `scripts/verify-run 9.1` は rustfmt 差分と Scenario 1件不足で FAIL（rc=1）。修正後の再実行では clippy が未使用メソッド、mutex の await 跨ぎ、bool 比較を検出して FAIL（rc=101）した。
- GREEN: 指摘を修正後、同じ `scripts/verify-run 9.1` が全項目 PASS（rc=0）。

## 変更ファイル

- `crates/server/src/lib.rs`
- `crates/server/src/st59_tests.rs`
- `web/e2e/st59-change-secrets.spec.ts`
- `openspec/changes/st59-change-password-from-web/evidence.jsonl`
- `.superpowers/sdd/st59-task-9/task-9-report.md`

## 自己レビュー

仕様や本番動作は変更せず、全変更はテスト補助・整形・Scenario の証跡に限定した。検証後にコード変更は行っていない。

## 懸念

Scenario 検査は既存ツール由来の「spec に無い Scenario」警告を一部出すが、検査結果は `scenarios: OK` であり、今回の担保不足はない。

## Fix report（review 1）

- F1: e2e の `設定のファイルに書けなければ、何も変わらない` の印を削除し、実際に設定ファイル・両 API の応答・旧値での動作・書きかけの不在を検証する Rust テストへ移した。
- F2: `st59_both_write_failure_returns_not_saved` で、両 API について `ok`（204/200）と `forbidden`（500）の access_log 件数がそれぞれ 1 件増えることを、要求前後の DB 件数比較で検証するようにした。

### Fix verification

- `scripts/quiet-run st59-write-failure-lib -- cargo test -p ashiato-server --lib st59_both_write_failure_returns_not_saved`: 1 passed, 0 failed。
- `scripts/verify-run 9.1`: 2 系統とも PASS（rc=0）。全体 cargo 検証と Scenario 突き合わせ（`scenarios: OK`）を実行。
- `scripts/quiet-run st59-e2e -- npm --prefix web run test:e2e -- e2e/st59-change-secrets.spec.ts`: 9 passed, 0 failed。

自己レビューでは、レビュー指摘の F1/F2 以外のコード変更は行っておらず、検証後のコード変更もない。
