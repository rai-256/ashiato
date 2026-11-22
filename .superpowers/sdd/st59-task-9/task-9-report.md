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
