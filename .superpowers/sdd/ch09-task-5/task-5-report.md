# Task 5 レポート

## 実装

- Rust の条件待ちを、回数上限から実時間 30 秒の期限へ変更した。
- scheduled 5 箇所、outage のロック待ち、履歴収集 2 箇所、runtime の履歴完了・生存信号待ち 3 箇所について、期限切れ時に待ち時間と最後に観測した値を表示するようにした。
- `ch04_statement_timeout_cancels_a_long_query` は `pg_sleep(60)` と「30 秒未満」へ、`time_sync_timeout_kills_the_child` は `sleep 60` と「30 秒未満」へ変更した。
- `tick_until_read` 系と `stay_tests.rs:2389` は変更せず、候補一覧を `inventory.md` に記録した。

## テスト

- `scripts/verify-run 5.1`: PASS（history 128 件、scheduled 2 件、outage の指定コマンドは rc=0）
- `scripts/verify-run 5.2`: PASS（指定コマンドは rc=0）
- 件数確認: scheduled 5 件、outage 5 件、打ち切り 1 件以上を `--list` で確認。
- `scripts/quiet-run full -- cargo test --workspace`: rc=0
- `cargo fmt --all -- --check` と `git diff --check`: PASS

## TDD Evidence

- RED: 既存試験の補助待ちを期限化する変更で、新規の製品コードはなく、変更前に独立した新規テストを追加して失敗を観測する対象がなかったため、独立した RED 記録は取得していない。
- GREEN: `scripts/verify-run 5.1` / `5.2`、focused test、workspace 全体テストが PASS した。

## 変更ファイル

- `crates/server/src/scheduled_tests.rs`
- `crates/server/src/outage_api_tests.rs`
- `crates/server/src/testdb_tests.rs`
- `crates/collector-windows/src/history/collect.rs`
- `crates/collector-windows/src/runtime.rs`
- `crates/collector-windows/src/time_sync.rs`
- `openspec/changes/ch09-load-tolerant-timing-tests/inventory.md`
- `openspec/changes/ch09-load-tolerant-timing-tests/evidence.jsonl`

## Self-review

- 30 秒期限、間隔、Scenario 名、試験名、本数を確認した。
- 製品コード（`#[cfg(test)]` 外）は変更していない。
- TDD の RED 記録が不足している点はプロセス上の懸念だが、実装後の focused / workspace 検証はすべて PASS している。

## Fix report（review 1）

- F1: `tick_until_read` と `tick_until_read_again` に、時刻を兼ねる回数ループを維持する理由の `// timing-ok:` 注記を追加した。
- F2: `collect.rs` の `last_seen`、`runtime.rs` の `last_reading` / `last_beats` をループ内の不変束縛へ移し、`unused_assignments` 警告を解消した。
- テスト: `scripts/quiet-run cw-fix -- cargo test -p ashiato-collector-windows --lib history` — `128 passed; 0 failed`。
- 検証: `scripts/verify-run 5.1` — 3 件すべて `PASS`（collector history 128 件、scheduled 2 件、outage コマンド rc=0）。
- 追加確認: `cargo fmt --all -- --check`、`git diff --check` — PASS。
