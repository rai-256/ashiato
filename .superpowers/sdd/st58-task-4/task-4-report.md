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

## 自己レビュー・懸念

Task 4 の純粋関数とテスト範囲に限定しており、実際のインストーラ実行フローへの接続は後続 Task の範囲である。その他の懸念はない。
