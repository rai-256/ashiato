# Task 4 実装報告

## 実装したこと

- `crates/installer/src/db.rs` に、配る PostgreSQL 用の `initdb` 引数、`postgresql.conf` 末尾設定、pwfile の冪等な後始末を追加した。
- `tools/check-pg-major.sh` を追加し、`pgsql.lock` と `docker-compose.yml` の PostgreSQL メジャー版一致、および不一致 self-test を実装した。
- `tools/release/install-check.sh` に同梱 `postgres --version` の 17 検査と Task 4 の Scenario 印を追加した。

## テスト結果

- `scripts/quiet-run ct-final -- cargo test -p ashiato-setup st55_db_` — PASS、10 tests passed。
- `cargo test -p ashiato-setup st55_db_ -- --list` — 10 tests listed（3 以上）。
- `cargo test --workspace` — PASS。
- `cargo fmt --all -- --check` — PASS。
- `cargo clippy -p ashiato-setup --all-targets -- -D warnings` — PASS。
- `tools/check-pg-major.sh` — PASS。
- `tools/check-pg-major.sh --self-test` — PASS（不一致を拒否）。
- `tools/release/install-check.sh --self-test` — PASS。

## TDD Evidence

### RED

`cargo test -p ashiato-setup st55_db_` を実装ファイル未追加・`db` module 未登録の状態で実行し、`unresolved import super::db` で失敗した。これは新しい DB helper API がまだ存在しないための期待した失敗だった。

### GREEN

実装後に `scripts/quiet-run ct-final -- cargo test -p ashiato-setup st55_db_` を実行し、10/10 passed。`--list` でも 10 本を確認した。

## 変更ファイル

- `crates/installer/src/db.rs`
- `crates/installer/src/lib.rs`
- `tools/check-pg-major.sh`
- `tools/release/install-check.sh`

## 自己レビュー

要求された範囲に限定し、秘密値をログや期待値に出していない。全体テストが生成した未追跡の `crates/server/logs/` は検査後に削除した。追加の懸念はない。

## Fix round 1

### 対応

- F1: `tools/release/install-check.sh` の実サービス検査に、`psql` による無認証拒否・SHOW 値・`ashiato` の管理者役割、`pg_hba.conf` の `trust` 不在、`netstat` の loopback 限定、DB ログの曜日ファイルと SQL 目印の不在、`icacls` ACL、`wevtutil` イベントログ、pwfile 残留を追加した。self-test の偽コマンドと偽状態ディレクトリもこれらを実際に検査する出力へ更新した。
- F2: Rust の純粋関数テストから、実サービスの振る舞いを検証していない Scenario 印を削除し、Scenario の担保を `install-check.sh` の実検査へ一本化した。

### 検証

- `scripts/verify-run 4.1` — PASS。4.1 の全 4 項目を現在の木で再実行。
- `tools/release/install-check.sh --self-test` — PASS。追加した DB/ログ/ACL/イベントログ等の偽実サービス検査を通過。
- `cargo test -p ashiato-setup st55_db_` — 10/10 passed。`--list` は 10 本。
- `cargo test --workspace` — 今回の変更外の既存不安定テスト `archive_scan_interval_tests::archive_scan_interval_change_takes_effect_without_restart` が 1 件失敗（1887 passed、走査 7 回要求に対し 6 回）。
- `bash -n tools/release/install-check.sh`、`git diff --check` — PASS。

### Fix round の懸念

workspace 全体の失敗は `crates/server/src/archive_scan_interval_tests.rs:164` の既存テストで、今回変更したファイルには関係しないため修正していない。
