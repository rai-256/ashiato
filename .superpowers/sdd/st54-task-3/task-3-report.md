# Task 3 レポート

## 実装内容

- `migration_ledger` の最初の適用行から DB の系統を判定する処理を追加した。`BeforeLedger` は正式版、空 DB と適用行 0 件は系統なしとした。
- `MigrateError::ChannelMismatch { db, me }`、`error: kind=channel_mismatch` の標準エラー、migrate / サーバ起動の終了コード 4 を追加した。
- 系統不一致は退避、BOOTSTRAP、版適用、付与、更新履歴より前に返し、更新履歴の早期返却にも追加した。
- 開発版・正式版の migrate / 起動、空 DB、旧 DB、知らない版併存、同系統起動の Scenario テストを追加した。
- `docs/handoff/ST58.md` の末尾に D3 の申し送りを追加した。

## TDD Evidence

- RED: `scripts/quiet-run red -- cargo test -p ashiato-server st54_channel_mismatch_report_is_distinct_from_db_ahead -- --exact`。`MigrateError` に `ChannelMismatch` が無いためコンパイル失敗し、未実装を検出した。
- GREEN: `scripts/quiet-run green4 -- cargo test -p ashiato-server --lib st54_channel_mismatch_report_is_distinct_from_db_ahead`。1 passed。
- DB を使う bin Scenario は `scripts/quiet-run st54-bin-serial -- cargo test -p ashiato-server --test server_startup st54_bin_ -- --test-threads=1` で 9 passed。

## 検証

- `scripts/verify-run 3.1`: PASS（4 コマンド、Scenario 件数条件を含む）。
- `scripts/verify-run 3.2`: PASS。
- `scripts/quiet-run drops-retry -- cargo test -p ashiato-server --lib drops_api_idempotent`: 1 passed。
- 全サーバテストは 1860 passed / 1 failed。失敗は並列実行時の既存 `drops_api_idempotent` の PostgreSQL deadlock（40P01）で、単体再実行は PASS。

## 変更ファイル

- `crates/server/src/migration_ledger.rs`
- `crates/server/src/migration_ledger_tests.rs`
- `crates/server/src/lib.rs`
- `crates/server/tests/server_startup.rs`
- `docs/handoff/ST58.md`
- `openspec/changes/st54-dev-side-by-side/evidence.jsonl`

## 自己レビュー

- 系統判定は既存の起動検査と migrate の共通経路に入り、系統不一致時に DB への書込み経路へ進まないことを確認した。
- `cargo fmt` と `git diff --check` は通過した。

## 懸念

- 全サーバテストの並列 deadlock は今回の変更外の既存テスト競合であり、単体再実行では再現しなかった。
