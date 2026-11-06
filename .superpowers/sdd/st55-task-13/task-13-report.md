# Task 13 実装報告

## 実装内容

- `configure_service_commands` の後段に、`ashiato-db` の SDDL を読み、`NT SERVICE\\ashiato-server` の SID へ `(A;;LCRP;;;<SID>)` だけを重複なく追加する PowerShell/sc.exe 命令を追加した。
- サーバのサービス実行中に 5 秒間隔で DB サービス状態を問い合わせ、`STOPPED` のときだけ開始する見張りを追加した。再起動成功は `kind=db_service_restarted`、問い合わせ失敗は理由付きでログに出す。Windows API 呼び出しは `crates/winsvc` に隔離した。
- `install-check.sh` の D35 シナリオを `postmaster.pid` の先頭行にある PostgreSQL 主 PID だけを `taskkill /F` する形へ修正し、サーバ SID の `LCRP` と `WP`/`DC` 不在も検査するようにした。

## TDD Evidence

### RED

- `scripts/quiet-run red-installer -- cargo test -p ashiato-setup st55_db_watch_`
  - 2 tests failed。DACL の `sdshow`/`sdset` 命令が未実装だったため。
- `scripts/quiet-run red-server -- cargo test -p ashiato-server st55_db_watch_`
  - `DbServiceState`、`DbWatchDecision`、`decide_db_watch` が未実装でコンパイル失敗。期待した RED。

### GREEN

- `scripts/verify-run 13.1` — PASS（installer の新規テスト 2 本以上）。
- `scripts/verify-run 13.2` — PASS（server の新規テスト 3 本以上、Windows target clippy も PASS）。
- `scripts/verify-run 13.3` — PASS（`install-check.sh --self-test`）。

## 変更ファイル

- `crates/installer/src/commands.rs`
- `crates/installer/src/lib.rs`
- `crates/server/src/service.rs`
- `crates/server/src/st55_tests.rs`
- `crates/server/tests/server_startup.rs`
- `crates/winsvc/src/lib.rs`
- `tools/release/install-check.sh`
- `openspec/changes/st55-server-installer/evidence.jsonl`

## 自己レビュー・懸念

- `cargo clippy -p ashiato-server -p ashiato-winsvc --all-targets --target x86_64-pc-windows-gnu -- -D warnings` は PASS。
- `cargo test --workspace` は、テスト用 PostgreSQL が `Connection refused` で利用できず、DB依存テストが失敗した。Task 13 の正式検証は `verify-run` の環境準備後に PASS している。
- Windows 実地のサービス再起動検証は仕様どおり PR CI の `install-windows` に委ねる。

## Fix report: review 1 findings F1-F5

- F1: SDDL の ACE 挿入を `S:` の前へ移す純粋関数 `add_db_watch_ace` とテストを追加し、既存 ACE の重複を防いだ。
- F2: DB サービス名を `crates/winsvc` の API 引数へ渡し、設定ファイルの置き場から release / development を選ぶようにした。`ashiato-db-dev` のテストを追加した。
- F3: 問い合わせ・開始失敗の実際の理由を `String` としてログへ渡すようにした。
- F4: 同一の問い合わせ失敗理由は連続してログへ出さず、理由が変わったときだけ記録する純粋関数とテストを追加した。
- F5: `assert_db_watch_acl` が DACL 部分だけを検査し、`LCRPWP` / `LCRPDC` のような複合権限も拒む self-test を追加した。

### Fix verification

- `cargo test -p ashiato-setup st55_db_watch_ -- --nocapture`: 3 passed.
- `cargo test -p ashiato-server st55_db_watch_ -- --nocapture`: 5 passed.
- `scripts/verify-run 13.1`: PASS、`scripts/verify-run 13.2`: PASS（clippy を含む）、`scripts/verify-run 13.3`: PASS。
- `cargo test --workspace`: PASS（失敗 0、ignored 3）。
- `tools/release/install-check.sh --self-test`: PASS。
