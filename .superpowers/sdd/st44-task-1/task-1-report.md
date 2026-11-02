# Task 1 report

## 実装

- `collector_scope.rs` に `COLLECTOR_ALLOWED` の9組と `collector_allows` を追加した。
- `access_log::middleware` で API トークンの一覧外を403（`forbidden` の記録付き）にし、未認証をハンドラ到達前に401（`unauthorized` の記録と warn 付き）にした。
- `GET /session` の API トークンを403（`api_token`・`forbidden`・403 の記録付き）にした。
- 既存の `/session` APIトークン試験を新しい仕様に更新し、旧Scenarioを削除した。

## テスト

- RED: `scripts/quiet-run st44-red -- cargo test -p ashiato-server st44_scope_` — `collector_scope` モジュール未実装で `E0583`。
- GREEN: `cargo test -p ashiato-server collector_scope_tests::st44_scope_allowlist_is_nine -- --exact` — 1 passed。
- `cargo test -p ashiato-server st44_scope_ -- --list ... | grep -c ': test$'` — 15。
- `cargo fmt --all`、`git diff --check` — 成功。
- 全体: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct-full -- cargo test -p ashiato-server'` — rc=101。`DATABASE_OWNER_URL` 不在でDB依存試験が失敗（コードコンパイルとDB非依存テストは通過）。
- Task gate 検証 1.1: `scripts/verify-run 1.1` — PASS、rc=0、証跡を `openspec/changes/st44-collector-credential-scope/evidence.jsonl` に記録。
- Task gate 検証 1.2（初回）: `scripts/verify-run 1.2` — 全体テストはrc=101でFAIL（APIトークンで一覧外の読み出しを行う既存router試験が多数残存）、旧ScenarioのgrepはPASS。
- 追加修正: 既存router試験の読み出し認証を画面ログインの印へ移し、APIトークンを維持すべきアクセスログ試験は直接routerへ送るテスト補助を追加した。
- Task gate 検証 1.2（再実行）: `scripts/verify-run 1.2` — 全体テストPASS、rc=0、418秒。旧ScenarioのgrepもPASS。
- Task gate 検証 1.1（再実行）: `scripts/verify-run 1.1` — PASS、rc=0。

## 変更ファイル

- `crates/server/src/collector_scope.rs`
- `crates/server/src/collector_scope_tests.rs`
- `crates/server/src/access_log.rs`
- `crates/server/src/web_session.rs`
- `crates/server/src/web_session_tests.rs`
- `crates/server/src/lib.rs`

## Self-review

許可判定は方法と `MatchedPath` の組で行い、HEADをGETへ拡張していない。取り込み3口の成功時に記録を書かない既存挙動も維持した。DB接続環境が無いため、router経由のDB統合シナリオは実行できていない。

## Fix report (review 1)

- F1/F2: `collector_scope_tests.rs` の 15 本を `router(...).oneshot(...)` 経由へ変更し、403/200、ハンドラ応答、AccessEntry の credential・route・method・outcome・status を実測するようにした。原本は `media_tests::Tmp` の自動削除される `$TMPDIR` 配下で PUT/HEAD と配置を確認する。
- F4: `COLLECTOR_ALLOWED` の長さと、要求された 9 組との集合一致を断言するようにした。
- F5: Bearer から画面ログインの印へ変換する処理を `testdb::as_web_session` に集約し、対象の既存 router 試験から重複実装を除いた。
- F3: `cargo test -p ashiato-server --lib st44_scope_ -- --list 2>/dev/null | grep -c ': test$'` は `15`。focused 実行は `scripts/quiet-run st44-focused-final -- cargo test -p ashiato-server --lib st44_scope_` で 15/15 PASS。
- `scripts/verify-run 1.1` — PASS、rc=0、最新コードの証跡を `evidence.jsonl` に追記。
- `scripts/verify-run 1.2` — PASS、rc=0（全体 377 秒）、旧 Scenario の grep も PASS。
- 追加の全体確認は 1868 passed / 1 failed。失敗は既存の `archive_scan_interval_tests::archive_scan_interval_change_takes_effect_without_restart`（期待 7 回、実測 6 回）で、今回の変更箇所外。

### Fix review self-review

15 本すべてが純粋な `collector_allows` のみを見ず router を通ること、一覧集合が 9 組と一致すること、共通の Cookie 変換 helper が使われることを確認した。Task 1 の範囲外の open finding（D9 の欠けた引数、原本 HEAD の axum依存確認）は変更していない。

`python3 scripts/check_scenarios.py .` は Task 2〜4 が担保する 7 Scenario の未担保で FAIL。Task 1 の Scenario 印は追加済みで、この失敗は今回の open finding ではない。
