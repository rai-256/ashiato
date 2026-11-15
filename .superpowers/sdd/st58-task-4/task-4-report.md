# Task 4 レポート

## 実装内容

- `crates/installer/src/update_plan.rs` に、版の系統・版差分の判定、更新途中の段と終了コード・退避有無による復旧計画、段ごとの復旧手順を追加した。
- 退避の保持選択、`PG_VERSION` の大きい版比較、SQLSTATE の級と理由説明、更新履歴 JSONL の 1 行生成を追加した。
- `sc.exe`、`pg_restore`、`psql` の純粋な命令生成関数を追加し、戻しの DB 接続は `ashiato_owner` を使うようにした。
- 開発版と正式版のレイアウト選択を `update_layout` にまとめた。

## テスト結果

- `scripts/verify-run 4.1`: PASS (rc=0)。`st58_plan_` 23 本、`--list` で 20 本以上を確認。
- `scripts/quiet-run full -- cargo test -p ashiato-setup`: PASS。125 + 4 tests passed。
- `cargo fmt --all` 実行済み、`git diff --check` PASS。

## TDD Evidence

- RED: `cargo test -p ashiato-setup st58_plan_` を実装前に実行し、`update_plan` モジュールと判定関数が無いためコンパイル失敗した。
- GREEN: `scripts/verify-run 4.1` で 23 本が PASS し、検証コマンド全体が rc=0 になった。

## 変更ファイル

- `crates/installer/src/update_plan.rs`
- `crates/installer/src/lib.rs`
- `openspec/changes/st58-update-apply-rollback/evidence.jsonl`

## Fix report: review 1 follow-up

- F1/F6: `recovery_plan` の段階ごとの復旧手順を整理し、`Started` / `Stopped` はまだ移動前なので `StartPrevious` のみ、移動後だけ `RestoreProgram` を含めるよう修正した。`Starting` の終了コード 0 は待ち受け失敗として、完了済み退避があれば DB 復旧を含める既存判定と手順を一致させた。
- F2: 退避エントリを完了フラグではなく `BackupEntryKind`（完了 `.dump` / 書きかけ `.dump.part` / `ashiato_failed_*` DB）で分類し、状態判定と保持処理が同じ分類を使うよう修正した。

### Fix verification

- `scripts/quiet-run st58-focused -- cargo test -p ashiato-setup --lib st58_plan_`: 25 passed、0 failed。
- `scripts/verify-run 4.1`: PASS、rc=0。指定検証の `--list` は 20 本以上。
- `scripts/quiet-run installer-full-lib -- cargo test -p ashiato-setup --lib`: 127 passed、0 failed。

### Fix changed files

- `crates/installer/src/update_plan.rs`
- `crates/installer/src/lib.rs`
- `openspec/changes/st58-update-apply-rollback/evidence.jsonl`

## 自己レビュー・懸念

Task 4 の純粋関数とテスト範囲に限定しており、実際のインストーラ実行フローへの接続は後続 Task の範囲である。その他の懸念はない。

## Fix report: review 1

- F1/F6: 段・終了コード・退避状態を受け取る `recovery_plan` に復旧手順と DB 復旧判定を統合し、`Starting` の待ち受け失敗（終了コード 0）だけを含め、`Started`〜`Replaced` ではプログラム復旧だけにした。
- F2: `BackupState`（`Completed` / `PartialOnly` / `None`）と `classify_backup_state` を追加し、終了コード 6 種 × 退避状態 3 種の 18 通りをテストした。
- F3: 保持対象・削除対象に run folder を含め、`ashiato_failed_*` を `drop_databases` として選ぶようにした。
- F4: `version_action` が `ashiato_build_stamp::channel` で系統を導出するようにした。
- F5: `pg_restore_command` に loopback host と指定 port（release 55400 / dev 55401 の呼び出し側想定）を追加した。

### Fix verification

- RED: `scripts/quiet-run red2 -- cargo test -p ashiato-setup st58_plan_` — 新 API（`BackupState`、統合後の `recovery_plan`、port 付き `pg_restore_command`）が無く、コンパイルエラーで失敗。
- GREEN: `scripts/quiet-run focused2 -- cargo test -p ashiato-setup st58_plan_` — 25 passed、0 failed。
- `scripts/verify-run 4.1` — PASS (rc=0)。指定検証の `--list` は 20 本以上。
- `scripts/quiet-run full-fix -- cargo test -p ashiato-setup` — 127 + 4 passed、0 failed。

### Fix changed files

- `crates/installer/src/update_plan.rs`
- `crates/installer/src/lib.rs`
- `openspec/changes/st58-update-apply-rollback/evidence.jsonl`
