# CH15 —— 配った形のサーバで、収集アプリの設定が正しい合言葉を断る

change: `openspec/changes/ch15-collector-setup-verifier-password/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）: なし

全 Task の ledger の `Ruling:`（逐語）:
- 全 Task の ledger に `Ruling:` の行は無い。

## Task

1 Task のうち、埋めた Task は 1 本。

残した項目:
- 9.1（人間の確認待ち（確認バッチが持つ））: 9.1 配った形の確かめ（確認バッチ）: **この change を含む組み立ての配る道具（`tools/release/`）で上げた**本人の PC のサーバで（いま入っているのは不具合の起きた版。上げた版の commit を確認バッチの手順書で示す）、画面にログインし、設定の「収集アプリの設定」で同じ合言葉を入れて「表示」を押すと、設定の文字列とコピーのボタンが出る（「合言葉が違います」にならない）。違う合言葉では「合言葉が違います」と出ることも見る

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server ch15_'` → rc=0（PASS・head ead53c0）
- 1.2 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server ch15_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` → rc=0（PASS・head ccd8f75）
- 1.2 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; test "$(cargo test -p ashiato-server st56_ -- --list 2>/dev/null | grep -c ": test$")" -ge 7 && scripts/quiet-run ct -- cargo test -p ashiato-server st56_'` → rc=0（PASS・head ccd8f75）
- 1.2 `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st55_ && scripts/quiet-run ct -- cargo test -p ashiato-server web_session'` → rc=0（PASS・head ccd8f75）
- 1.2 `bash -c '! grep -n "login\.password\b" crates/server/src/web_session.rs'` → rc=0（PASS・head ccd8f75）
- 1.2 `bash -c 'code=$(grep -vE "^\s*//" crates/server/src/web_session.rs); test "$(printf "%s\n" "$code" | grep -c "\.accepts(")" -eq 1 && test "$(printf "%s\n" "$code" | grep -c "LOGIN_CHECK")" -eq 2'` → rc=0（PASS・head ccd8f75）
- 1.2 `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st56-collector-setup.spec.ts'` → rc=0（PASS・head ccd8f75）
- 1.2 `scripts/quiet-run clippy -- cargo clippy -p ashiato-server --all-targets -- -D warnings` → rc=0（PASS・head ccd8f75）
- 1.2 `./tools/check-openapi.sh` → rc=0（PASS・head ccd8f75）

## レビュー（`review/code.md`）

- 指摘 19 件（fixed 18・rejected 1）
- 処置の re-review: 5 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
