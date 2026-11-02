# Task 3 実装レポート

## 実装内容

- `crates/server/src/contract_version.rs` の `check_header` と `ItemGate::new` にある呼び出し元判定を、`Caller::ApiToken` と `Caller::WebSession | Caller::Satellite { .. }` を明示する `_` 無しの `match` に変更した。
- 振る舞いは変更していない。Task で指定された grep 検査と `verify-run 3.1` の証跡を更新した。

## テスト

- `scripts/verify-run 3.1`: PASS（focused command rc=0、構造検査 rc=0）。
- `cargo test -p ashiato-server st50_ -- --list 2>/dev/null | grep -c ': test$'`: 42（要求の20以上）。
- `scripts/quiet-run ct-direct -- cargo test -p ashiato-server`: 42 passed, 0 failed。
- `scripts/quiet-run server-full -- cargo test -p ashiato-server`: 1872 passed, 3 ignored, 0 failed; `server_startup` 37 passed, 0 failed。
- `git diff --check`: PASS。

## TDD Evidence

- RED: `bash -c '! grep -n "matches!(caller, Caller::ApiToken)" crates/server/src/contract_version.rs'` は実装前に rc=1。対象の2箇所が残っているため、構造要件を正しく検出する失敗だった。
- GREEN: 実装後に `scripts/verify-run 3.1` が PASS。focused test は最終的に42本が実行され全て PASS、grep 検査も PASS。

## 変更ファイル

- `crates/server/src/contract_version.rs`
- `openspec/changes/st44-collector-credential-scope/evidence.jsonl`
- `.superpowers/sdd/st44-task-3/task-3-report.md`

## 自己レビュー

- `Caller` の現在の全種類を列挙し、`contract_get` や他 Story の範囲には変更を広げていない。
- Task に新規 Scenario が指定されていないため、テストコードは追加していない。
- 懸念なし。
