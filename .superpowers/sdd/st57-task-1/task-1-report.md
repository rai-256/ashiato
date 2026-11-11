# Task 1 実装報告

## 実装内容

- `ashiato-update-check` crate を workspace に追加。
- GitHub release JSON（64 KiB 制限）の読み取り、3 部品の配布物名からの SemVer 選択、正式版/beta/nightly のチャンネル規則、下書き・対象外配布物の除外を実装。
- `due`（24 時間）、`stale`（7 日）、`updater_not_running`（48 時間）を実装。未記録と未来の時刻は期限到来として扱う。
- RFC 3339 入力、記録 1 行、`status.json` の serde 形、問い合わせ道・見出し・ファイル名の定数を追加。
- `docs/contracts/update-check-vectors.json` に選択 30 件、時期 10 件を追加。

## TDD Evidence

- RED: `cargo test -p ashiato-update-check st57_choose_`。crate の API（`Release`、`Component`、`choose`）未実装によるコンパイル失敗を確認。
- GREEN: `scripts/quiet-run ct -- cargo test -p ashiato-update-check st57_choose_`。16/16 passing。

## 検証

- 指定検証（focused test、`-- --list` 14 本以上、vectors 条件）: PASS。
- `cargo clippy -p ashiato-update-check --all-targets -- -D warnings`: PASS。
- `cargo test --workspace`: FAIL。既存 server の DB 依存試験が testdb 作成失敗（`crates/server/src/testdb.rs:346`）で大量に失敗。今回の crate の試験は通過。

## 変更ファイル

- `Cargo.toml`, `Cargo.lock`
- `crates/update-check/Cargo.toml`
- `crates/update-check/src/lib.rs`
- `docs/contracts/update-check-vectors.json`

## 自己レビュー

- Task 1 の HTTP・TLS・DB を crate に追加していない。
- Scenario コメントは Task 1 が担保する選択・結果の Scenario 名に一致させた。
- workspace 全体テストの失敗は今回の変更箇所ではなく、DB 環境依存の既存 server 試験によるもの。
