# ST59 —— 合言葉を画面から変えられる

change: `openspec/changes/st59-change-password-from-web/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D8（仮）. 「設定のファイルを指して起こしたサーバ」は、照らす形（`ASHIATO_SERVICE_MODE`）で `ASHIATO_SETTINGS_PATH` があるサーバ。Linux の試験は台本で起こす
- D9（仮）. 読み出しの記録は、口ごとに 1 求め 1 行。結果の語は今あるものを使い、移行を足さない（C1）
- D10（仮）. 口と本文の形
- D11（仮）. 画面の節と文言

全 Task の ledger の `Ruling:`（逐語）:
- 全 Task の ledger に `Ruling:` の行は無い。

## Task

9 Task のうち、埋めた Task は 9 本。
残した項目は無い。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st59_swap_ && test "$(cargo test -p ashiato-server st59_swap_ -- --list 2>/dev/null | grep -c ": test$")" -ge 4'` → rc=0（PASS・head fbb9895）
- 1.2 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; for p in web_session:16 st55_:47 st56_:7 ch15_:8; do n=${p%%:*}; m=${p##*:}; scripts/quiet-run ct -- cargo test -p ashiato-server "$n" || exit 1; test "$(cargo test -p ashiato-server "$n" -- --list 2>/dev/null | grep -c ": test$")" -ge "$m" || exit 1; done'` → rc=0（PASS・head fbb9895）
- 1.3 `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st56_runtime_unauthorized_keeps_pending_records && test "$(cargo test -p ashiato-collector-windows st56_runtime_unauthorized_keeps_pending_records -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` → rc=0（PASS・head fbb9895）
- 1.3 `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St56SetupNotifierTest*'` → rc=0（PASS・head fbb9895）
- 2.1 `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-server st59_settings_ && test "$(cargo test -p ashiato-server st59_settings_ -- --list 2>/dev/null | grep -c ": test$")" -ge 5'` → rc=0（PASS・head 1ccf94d）
- 2.1 `cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings` → rc=0（PASS・head 1ccf94d）
- 3.1 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st59_web_password_ && test "$(cargo test -p ashiato-server st59_web_password_ -- --list 2>/dev/null | grep -c ": test$")" -ge 10'` → rc=0（PASS・head 1d5ea48）
- 4.1 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st59_collector_token_ && test "$(cargo test -p ashiato-server st59_collector_token_ -- --list 2>/dev/null | grep -c ": test$")" -ge 7'` → rc=0（PASS・head e9d2264）
- 4.2 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st59_both_ && test "$(cargo test -p ashiato-server st59_both_ -- --list 2>/dev/null | grep -c ": test$")" -ge 8'` → rc=0（PASS・head e9d2264）
- 4.2 `./tools/check-openapi.sh` → rc=0（PASS・head e9d2264）
- 4.2 `tools/check-log-private.sh` → rc=0（PASS・head e9d2264）
- 5.1 `tools/settings-server.sh --self-test` → rc=0（PASS・head df8b319）
- 5.1 `tools/check-secrets.sh` → rc=0（PASS・head df8b319）
- 6.1 `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st59-change-secrets.spec.ts'` → rc=0（PASS・head 7bd493d）
- 6.1 `bash -c 'cd web && npx tsc --noEmit && npx vitest run && npm run lint'` → rc=0（PASS・head 04cfe42）
- 6.2 `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e'` → rc=0（PASS・head a036354）
- 7.1 `tools/release/install-check.sh --self-test` → rc=0（PASS・head 076cb8e）
- 7.1 `bash -c 'n=$(tools/release/install-check.sh --list | grep -cE "^(画面で変えた後も、インストーラで決め直せる|変えた後に起動し直しても、新しい合言葉でログインでき、前の合言葉ではできない|変えた後に起動し直しても、新しい収集アプリの値で動く)$"); test "$n" -eq 3'` → rc=0（PASS・head 076cb8e）
- 8.1 `bash -c 'grep -q "画面の合言葉を変える" docs/network.md && grep -q "収集アプリの値を変える" docs/network.md && grep -q "画面の合言葉を変える" docs/install.md && grep -q "収集アプリの値を変える" docs/collector-setup.md'` → rc=0（PASS・head f08f444）
- 8.2 `bash -c 'grep -q "st59-change-password-from-web" docs/handoff/ST60.md && grep -q "settings-server.sh" docs/handoff/ST60.md && grep -q "collector-token" docs/handoff/ST60.md'` → rc=0（PASS・head f08f444）
- 9.1 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p ashiato-setup -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings && scripts/quiet-run all -- cargo test --workspace'` → rc=0（PASS・head 04cfe42）
- 9.1 `bash -c 'tools/check-openapi.sh && tools/check-log-private.sh && tools/check-secrets.sh && python3 scripts/check_scenarios.py . st59-change-password-from-web'` → rc=0（PASS・head 04cfe42）

## レビュー（`review/code.md`）

- 指摘 31 件（escalated 1・fixed 30）
- 処置の re-review: 8 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
