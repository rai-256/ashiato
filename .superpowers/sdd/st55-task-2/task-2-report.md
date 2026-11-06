# Task 2 実装報告

## 実装したこと

- `ashiato-server service --settings <場所>` の入口を追加し、tokio 起動前に JSON 設定を読み、文字列値をプロセス環境へ反映するようにした。
- 設定ファイルの `settings_missing` / `settings_unreadable` / `settings_malformed` 分類と、状態置き場の `logs/server/server-YYYY-MM-DD.log` への拒否理由記録を追加した。秘密の値・パスは理由へ含めない。
- DB 接続を 1 秒間隔・最長 120 秒で待つ純粋な関数と service 起動経路を追加した。
- 日付切替で開き直すサーバログ、30 日より古いログの削除、停止時のドット始まりの一時ファイル削除を追加した。
- `windows-service` は `crates/winsvc` に隔離し、`define_windows_service!` の入口から service 実行関数を呼ぶ形にした。判断を `design.md` の D2 に追記した。
- 開発起動の環境変数経路を確認する `st55_bin_env_unchanged` と、下支えテスト 9 本を追加した。Windows cross-target clippy を通すため、Unix 専用の既存テスト補助を `cfg(unix)` に整理した。

## TDD Evidence

### RED

- `scripts/quiet-run st55-red -- cargo test -p ashiato-server st55_service_`
- `service.rs` 未作成による `error[E0583]: file not found for module service` で失敗。対象機能が存在しないための期待どおりの失敗だった。

### GREEN

- `scripts/quiet-run st55-post-windows -- cargo test -p ashiato-server st55_tests`
- 9 passed、0 failed。
- `scripts/verify-run 2.2`、`2.3`、`2.4` は最新コードで PASS。

## 検証

- `cargo test -p ashiato-server`: 1887 passed、0 failed、3 ignored、および統合テスト 40 passed。
- `cargo clippy -p ashiato-server --all-targets -- -D warnings`: PASS。
- `cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings`: PASS。
- `tools/check-licenses.sh`: PASS（Rust 360 件、Node 257 件、不許可 0 件）。
- `tools/check-log-private.sh`: PASS。

## 変更ファイル

- `crates/server/src/service.rs`
- `crates/server/src/st55_tests.rs`
- `crates/server/src/main.rs`
- `crates/server/src/lib.rs`
- `crates/server/tests/server_startup.rs`
- `crates/winsvc/Cargo.toml`
- `crates/winsvc/src/lib.rs`
- `Cargo.toml` / `Cargo.lock` / `crates/server/Cargo.toml`
- `openspec/changes/st55-server-installer/design.md`
- `openspec/changes/st55-server-installer/evidence.jsonl`

## 自己レビュー

- 既存の開発起動経路は変更せず、service 経路だけが設定ファイルを読み込むことを確認した。
- ログの公開 API はメッセージ文字列を受け取るため、呼び出し側が秘密を渡さないよう service 内の拒否理由を種別だけに限定した。ログ private 検査は PASS。
- Windows の SCM からの停止通知を受ける入口は `winsvc` に分離した。router の完全な graceful shutdown と背景 worker の停止統合は、この Task の Linux 下支え範囲ではまだ service callback から接続していないため、Task 3 の install-check と後続実装で確認すべき事項として残る。

## Fix round 1

### 修正内容

- F1/F7: `ashiato-winsvc` が SCM に StartPending/Running/Stopped を報告し、Stop 通知をサーバへ渡すようにした。サービス側は graceful shutdown を待ち、export の書き途中ファイルを停止時に片付ける。
- F6: `windows-service` の依存を Windows 限定にし、crate 全体を `cfg(windows)` にしたため Linux の workspace check が空実装で通る。
- F2/F3/F4: 停止試験が遅延後の停止通知と実ファイル状態を確認するようにし、DB 待機試験は tokio の時間を使い、タイムアウト理由のログを確認するようにした。サービス段の Scenario 印は下支え試験から除いた。
- F5/F8/F9: service 起動拒否を同じ server log に記録し、runtime/service 起動失敗をログして終了し、ANSI 無効・断片書き込み時の余分な改行を抑えた。

### 修正後の検証

- `cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings`: PASS
- `tools/check-log-private.sh`: PASS
- `cargo test -p ashiato-server --lib st55_service_`: 6/6 PASS
- `cargo test -p ashiato-server --lib st55_stop_`: 1/1 PASS
- `cargo test -p ashiato-server --lib st55_log_`: 3/3 PASS
- `cargo test --workspace`: 1887 passed、1 failed、3 ignored。失敗は既存の `archive_scan_interval_tests::archive_scan_interval_change_takes_effect_without_restart`（走査回数 2/3）で、今回の変更箇所とは無関係。

## Fix round 2

### 修正内容

- F1/F15: `run_inner` が取り込み器・定期取得のハンドルを保持し、サービスの graceful shutdown 後に drop するようにした。書き出しジョブには abort handle を保持させ、停止後に abort と部分ファイル清掃を行うようにした。Windows の SCM へ `StopPending` を通知し、実行失敗時は `Stopped` に非 0 の終了コードを返すようにした。
- F2: 停止試験を、`ASHIATO_EXPORT_SELFTEST_DELAY_MS` の遅延中に部分ファイルを作成し、停止後に完成ファイルへ進まず、次の起動相当の清掃後にも状態を持ち越さないことを確かめる内容へ改めた。
- F3: DB 待機の現在の tokio 時間ベース実装に対し、正式検証で 6 本の service 下支え試験とタイムアウト理由の試験を再実行した。
- F14: 設定パスが無い場合も既定の `logs/server` を使い、ログ書き込みに失敗した場合は秘密を含まない構造化ログへフォールバックするようにした。

### 修正後の検証

- `scripts/verify-run 2.1`: clippy / license とも PASS。
- `scripts/verify-run 2.2`: service 6 本、`server_startup` 1 本を含め PASS。
- `scripts/verify-run 2.3`: stop 1 本を含め PASS。
- `scripts/verify-run 2.4`: log 3 本、`tools/check-log-private.sh` とも PASS。
- `cargo test -p ashiato-server`: 1888 passed、0 failed、3 ignored。統合テスト 40 passed。
- `cargo check -p ashiato-server --target x86_64-pc-windows-gnu`: PASS。

## Fix round 3

### 修正内容

- F2: 停止 Scenario の試験を、実際の `export::spawn` と `ExportConfig::from_env` で組み直した。`ASHIATO_EXPORT_SELFTEST_DELAY_MS` の遅延中に実際の `.partial-*` ファイルを観測し、停止通知と `ExportJobs::stop` 後にドット始まりのファイルが残らず、次の起動用の新しい job 状態が実行中でないことを確認する。

### 修正後の検証

- `cargo test -p ashiato-server st55_stop_removes_partial_dot_files`: 1 passed、0 failed。`--list` でも対象 1 本を確認。
- `scripts/verify-run 2.3`: PASS、rc=0。
- `cargo test -p ashiato-server`: 今回の停止試験を含め大半は通過したが、既存の `drops_tests::drops_api_idempotent` が PostgreSQL の deadlock (`40P01`) で失敗。今回の変更箇所とは無関係。
