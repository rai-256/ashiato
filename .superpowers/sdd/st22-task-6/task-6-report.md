# Task 6 実装報告

## 実装内容

- `GET /stays/detail?stay_id=&user_id=` を追加した。
- `core.event_live` の滞在を対象に時間範囲を取得し、同じ利用者の読み出し中の記録を `logical_source` ごとに集計した。
- 滞在自身を除外し、`core.source.display_name` を添えて返すようにした。
- 資格情報なしは 401、未知または利用者が一致しない滞在は 404 とした。
- 指定された 2 Scenario の印を付けた PostgreSQL 統合テストを追加した。

## TDD Evidence

### RED

`scripts/quiet-run task6-red -- cargo test -p ashiato-server stays_detail_counts` を実行。
実装前のため `stays_detail_get` と `StaysDetailQuery` が crate root に無いというコンパイルエラーで rc=101 になった。対象 API が未実装であることによる期待どおりの失敗。

### GREEN

`scripts/verify-run 6.1` が PASS（rc=0）。証跡は `openspec/changes/st22-record-deletion/evidence.jsonl` に記録済み。
対象テスト `deletion_tests::stays_detail_counts` は 1 passed。

## テスト結果

- 指定検証: PASS。
- サーバの deletion_tests 直列実行: 17 passed, 0 failed。
- 全ワークスペース直列実行（`cargo test --workspace -- --test-threads=1`）: rc=0。
- 通常の全ワークスペース並列実行は、既存の `restore_is_scoped` が共有 `core.deletion_ledger` 全体件数を検証するため、並走テストの台帳追加 1 件を拾って 2 回とも失敗した。対象テスト単独では PASS しており、本変更の詳細件数テストも PASS している。既存テストは変更していない。

## 変更ファイル

- `crates/server/src/lib.rs`
- `crates/server/src/deletion_tests.rs`
- `openspec/changes/st22-record-deletion/evidence.jsonl`
- `.superpowers/sdd/st22-task-6/task-6-report.md`

## Self-review

- 集計元は削除済みを除く `core.event_live` で、滞在自身を論理ソースで除外している。
- `core.source` と結合して登録簿の表示名を返している。
- API の認証・未知識別子のエラー経路と、削除済み件数除外をテストしている。

## 懸念

通常の並列全体テストには既存の共有台帳件数競合があり、直列化が必要。Task 6 の指定検証と直列全体テストは PASS。

## Fix report（review 1 / F1）

- `core.event` に `event_by_user_time_live (user_id, event_time) WHERE deleted_at IS NULL` を追加する末尾移行
  `202609291151_detail_counts_index.sql` を作成し、`MIGRATIONS` に登録した。
- 詳細集計と同じ SQL の `EXPLAIN` が `event_by_user_time_live` を使うことを検証するテストを追加した。
- 新索引がより適切に選ばれることで旧索引名だけを期待して失敗する既存の計画検査を、旧索引または新索引を受け入れるよう更新した。

### Fix verification

- RED: `scripts/quiet-run task6-f1-red -- cargo test -p ashiato-server stays_detail_query_uses_user_time_index` — rc=101。
  計画に `event_by_user_time_live` が無く、追加した索引利用検査が失敗した。
- GREEN: `scripts/quiet-run task6-f1-green -- cargo test -p ashiato-server stays_detail_query_uses_user_time_index` — rc=0、対象 1 passed。
- 指定検証: `scripts/verify-run 6.1` — PASS（rc=0、対象 1 passed）。
- 全体検証: `scripts/quiet-run task6-f1-full-2 -- cargo test --workspace -- --test-threads=1` — rc=0。
- テスト名確認: `cargo test -p ashiato-server --lib -- --list | rg 'stays_detail'` — `stays_detail_counts` と
  `stays_detail_query_uses_user_time_index` を確認。

## Fix report（review 2 / F1 の再確認）

- 現ツリー（`eb08c0f`）に F1 の修正が存在することを確認した。末尾 migration
  `202609291151_detail_counts_index.sql` が `core.event` に `(user_id, event_time)` の
  `WHERE deleted_at IS NULL` 索引を追加し、`MIGRATIONS` に登録されている。
- `scripts/verify-run 6.1` — PASS（rc=0）。現ツリーの `eb08c0f` に対する証跡が
  `openspec/changes/st22-record-deletion/evidence.jsonl` に追加された。
- `scripts/quiet-run task6-f1-round2-focused -- cargo test -p ashiato-server stays_detail_query_uses_user_time_index`
  — rc=0、`1 passed; 0 failed`。
- `scripts/quiet-run task6-f1-round2-full -- cargo test --workspace -- --test-threads=1`
  — rc=0。
