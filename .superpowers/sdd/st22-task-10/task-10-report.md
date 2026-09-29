# Task 10: 仕上げ — 実施報告

## 実装内容

- `cargo run -p ashiato-server --bin openapi` から `docs/openapi.json` を再生成し、既存実装の 3 経路 `/stays/detail`、`/stays/erase`、`/stays/restore` と関連スキーマを契約へ反映した。
- `stay_store::day_view` の削除済み行取得を名前付き `HiddenRow` に変更した。挙動は変えず、Clippy の `type_complexity` を解消した。
- `evidence.jsonl` に 10.1〜10.3 の現行木での検証結果を記録した。

## 検証結果

- 10.1 `scripts/verify-run 10.1`: PASS
- 10.2 `scripts/verify-run 10.2`: 3 検査すべて PASS
- 10.3: fmt / clippy / Web 一式 / immutable / smoke は PASS。
- 10.3 の `cargo test --workspace` は 2 回とも `pool timed out while waiting for an open connection` で `BLOCKED_INFRA`。コードのコンパイル後テスト失敗ではなく、DB 接続プール枯渇としてハーネスが判定した。
- 10.4: `grep -c st22-record-deletion docs/handoff/ST23.md docs/handoff/ST33.md` 相当を確認し、ST23=6、ST33=3。指定の handoff を再読した。

## TDD Evidence

Task 10.1 は新しい API 挙動の実装ではなく、コードから生成される契約ファイルの更新であるため、追加の挙動テストは不要と判断した。既存の `utoipa` 注釈から生成した JSON を `tools/check-openapi.sh` で検証した。Clippy 対応も型表現だけの変更で、既存の workspace test と smoke の対象範囲で確認した。

## 変更ファイル

- `docs/openapi.json`
- `crates/server/src/stay_store.rs`
- `openspec/changes/st22-record-deletion/evidence.jsonl`
- `.superpowers/sdd/st22-task-10/task-10-report.md`

## 自己レビュー

- `git diff --check` は問題なし。
- OpenAPI は生成元との差分検査を通過した。
- `HiddenRow` は SQL の列と後続処理の対応を明示するだけで、削除・復元・一覧の意味は変更していない。

## 懸念

- `cargo test --workspace` の PASS 証跡を取得できていない。DB 接続プール枯渇が解消した環境で、Task gate が 10.3 を再実行する必要がある。
