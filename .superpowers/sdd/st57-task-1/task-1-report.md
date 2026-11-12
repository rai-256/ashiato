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

## Task gate 証跡追補

- `scripts/verify-run 1.1`: 指定された focused test/list 件数の検証 PASS（16/16）。
- `scripts/verify-run 1.1`: `update-check-vectors.json` の cases/timing 件数検証 PASS。
- 証跡: `openspec/changes/st57-update-notice/evidence.jsonl`。

## Fix round 1 report

### 対応

- F1: Rust が `include_str!` で共有 vectors の cases と timing を全件検証する Scenario テストを追加。
- F2: 正式版用・nightly 用の問い合わせ道と、許可する 5 見出しおよび `User-Agent` / `Accept` 定数を D2 に合わせた。
- F3: 保存済み `Status.latest` を読む `update_available` を追加し、インストール後 Scenario を状態読み取り経由に変更。
- F4: `parse_status` / `parse_record_line` に 64 KiB、SemVer、serde による RFC 3339 検証を追加し、`stale_notified` に既定値を付与。
- F5: timing 材料を符号付き経過時間にし、未来の時刻と未記録の `stale` / `updater_not_running` を追加。
- F6: nightly の選択 Scenario に正式版 `v0.5.0` を同時に含め、正式版を数えないことを Rust と材料で検証。
- F7: `chrono` は RFC 3339 の型安全な時刻処理に必要な純粋 crate の依存であることを design に明記。

### 検証

- focused: `scripts/quiet-run st57-fix3 -- cargo test -p ashiato-update-check st57_choose_` — 17/17 passing。
- workspace: `scripts/quiet-run workspace -- cargo test --workspace` — rc=0、失敗なし。
- Task gate: `scripts/verify-run 1.1` — 指定 focused test/list 件数 PASS（17 tests）、vectors 件数 PASS。
- 証跡: `openspec/changes/st57-update-notice/evidence.jsonl`。

### 自己レビュー

- 変更範囲は F1〜F7 とその証跡に限定した。
- 保存状態の Scenario は `parse_status` で読み込んだ `Status.latest` を `update_available` に渡している。
- workspace 全体テストは今回の修正後に PASS した。
