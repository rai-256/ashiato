# ST44 code review

## final review（615fc4e..e5d82bb）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 2 / Minor 13。判定は「With fixes」。
reviewer が手元で走らせたもの: `check_scenarios.py`（OK）、`tools/check-api-token-reads.sh`（web/e2e・tools/recording・smoke(.d)・seed(.d)・stack・verify-prep・record-env に rc=0、`--self-test` rc=0）、
`cargo test -- --list` の本数（`st44_scope_` 15・`st44_routes_` 3・`web_session` 27・`st44_` 25・`server_startup st44_session_key_` 2。どれも tasks.md の下限以上）。
申し送り（`docs/handoff/ST44.md`）の 2 件 —— `GET /feelings/reminder` と `GET /outages` を収集アプリの資格情報で読める —— は許可の一覧の 9 つに入り、`st44_scope_feelings_reminder`・`st44_scope_outages` が置かれている（試験の強さは R1）。
Task ごとの ⚠️ は final reviewer が全体で確かめた。解消: T1 の D9（`st44_routes_without_credentials`）と HEAD（lib.rs の明示の `.head()`）、T2 の印の名前、T4 の network.md §4/§6 と本数、T5 の seed の型と D13 の置き換え先、T6 の他の Bearer の読み。
残るもの: T1 F10 → R1、T4 F7 → R2、T6 の auth.ts → R8、T7 の st25-seed-check → R11。

## R1. Scenario の印を持つ試験の 4 本が Scenario の THEN を確かめていない
- 成果物: crates/server/src/collector_scope_tests.rs
- 根拠: `st44_scope_outages` は `App::for_test` の warmup が `OUTAGE_WARMUP` のまま（lib.rs:486）で応答はいつも `warming_up`、`judged_at` は WarmingUp にもあるので Ready の道を通らずに真（tasks 1.1「試験の時計で過ぎた状態にする」とずれる）/
  `st44_scope_feelings_reminder` は記録を入れず 200 だけ見る（THEN「その日の記録があることを示し、ひとことの文字列も尺度の値も含まない」）/
  `st44_scope_sends_pass` は `{}` を送って `!= 403` だけ（THEN「どの求めも 200 で、送った 1 件ずつが受け付けられた結果」）/
  `st44_scope_feelings_reminder_post_403` は通知の時刻が変わらないことを DB で見ていない（Task 1 F10）
- kind: technical
- 提案: `/outages` は warmup を 0 にして `status == "ready"` と `outages` の配列を断言。通知の判断はひとことと尺度を持つ記録を入れて `recorded == true`・鍵が `["at","recorded"]` だけ・ひとことの文字列が無いを断言。送る口は形の正しい 1 件で 200 と受け付けを断言。POST 403 は前後の時刻の一致を断言

## R2. `.env.example` の雛形の `WEB_SESSION_KEY=change-me-web-session-key` が起動の検査を通る
- 成果物: .env.example / crates/server/src/web_session.rs（`check_web_session_key`）
- 根拠: `.env.example:23`。`check_web_password` には `change-me` の拒否（Placeholder）があるが鍵の検査には無い。雛形を写した `.env` では HMAC の鍵が公開値になり、D7 が塞いだ「API の合言葉と DB の写しだけで画面の合言葉を確かめられる」が戻る（Task 4 F7）。拒否の理由は Global Constraints が 4 つに固定（D8 は仮）
- kind: technical
- 提案: 計画を変えずに、雛形の値を空にする（`missing` で起動しない）。`env-init.sh` は乱数で作るので、それを使う経路は変わらない

## R3. 新しく書いた分岐に `if let Caller::ApiToken = caller` が 1 か所残る（D6）
- 成果物: crates/server/src/web_session.rs
- 根拠: web_session.rs:517。D6 は新しい分岐を `_` の無い全種類の match にして、種類が増えたらコンパイラが止める形にする（Task 1 F8）
- kind: technical
- 提案: 全種類を並べた match にする

## R4. middleware の 2 つ目の match が 1 つ目の判定を文字列の比較で読み直している
- 成果物: crates/server/src/access_log.rs
- 根拠: `Some(Caller::ApiToken) if outcome == "forbidden"`（Task 1 F6）。判定の値と文字列が食い違うと黙って外れる
- kind: technical
- 提案: 1 つ目の判定の結果（bool か enum）を持ち回る

## R5. `credential_present` と `authorize` で「資格情報がある」の意味が違う
- 成果物: crates/server/src/access_log.rs
- 根拠: `credential_present` は authorization か cookie の見出しの有無、`authorize` は空でない Bearer の有無（Task 1 F7）。同じ warn が 2 つの道で違う条件で出る
- kind: technical
- 提案: 同じ条件にそろえるか、違いの理由をコメントに書く

## R6. `/session` の GET の期待を 403 に替えた断言に ST44 の理由のコメントが無い
- 成果物: crates/server/src/access_log_tests.rs
- 根拠: tasks 1.2 は 1 行のコメントを求める。`grep ST44 crates/server/src/access_log_tests.rs` が 0 件（Task 1 F9）
- kind: technical
- 提案: D12 を指す 1 行のコメントを足す

## R7. `st44_scope_web_session_unchanged` が `/ingest` を叩いていない
- 成果物: crates/server/src/collector_scope_tests.rs
- 根拠: tasks の指定は「記録・滞在・日ごとの気分の読み出しと `/ingest`」
- kind: technical
- 提案: 画面のログインの印で `/ingest` に 1 件送り、範囲が変わらないことを断言する

## R8. `web/e2e/auth.ts` がモジュールの読み込み時に `state.json` を読む
- 成果物: web/e2e/auth.ts
- 根拠: `npm run test:e2e` は globalSetup の後に試験を読むので CI は落ちない（Playwright runner の順を確かめた）。`--list`・UI・watch の型は globalSetup の前に読むので、新しい checkout で throw する（Task 6 ⚠️）
- kind: technical
- 提案: 初めて呼ばれたときに読み、結果を持つ

## R9. 録画が `../../../web/e2e/auth.js` を相対の深さで読む
- 成果物: tools/recording/scenarios.d/（ST44 で書き換えた録画の spec）
- 根拠: 写し先 `web/e2e-recording/<id>/` からも同じ深さで解決する（確かめた）が、深さの偶然の一致に乗っている。シナリオの `requires` に `web/e2e/auth.ts` が無い
- kind: technical
- 提案: 該当シナリオの `requires` に `web/e2e/auth.ts` を足す

## R10. `tools/record-env.sh` の注記が古く、ログインの失敗を受けていない
- 成果物: tools/record-env.sh
- 根拠: :229 の注記「録画の spec が画面の外から `/api` を直に読む…ときの Bearer」は締める前の形。`archive_prepare` の `read_header="$(tools/web-login.sh …)"` は `|| return 1` が無く、`||` の中なので errexit も効かず、後の curl の 401 で落ちる（Task 5 F1）
- kind: technical
- 提案: 注記を直し、`|| return 1` を足す

## R11. `tools/st25-seed-check.sh` はログインの失敗で原因の分からない落ち方をする
- 成果物: tools/st25-seed-check.sh
- 根拠: `set -e` が無く、ログインに失敗すると空の見出しで 401 → jq の失敗として落ちる（Task 7 F4）。cookie で `/day` を読む流れを走らせた証跡も無い（Task 7 ⚠️）
- kind: technical
- 提案: ログインの失敗を明示して exit する

## R12. `tools/seed.sh` の seed.d の注記に `READ` が無い
- 成果物: tools/seed.sh
- 根拠: 末尾の注記「AUTH・BIND・work はそのまま使える」。seed.d の台本が読むときに使う `READ` が抜けている
- kind: technical
- 提案: 注記に `READ` を足す

## R13. `lib.rs` と起動の試験の細部
- 成果物: crates/server/src/lib.rs / crates/server/tests/server_startup.rs
- 根拠: `with_session_key` が期限を 0 に固定、`"archive-internal-session-key"` が 2 か所に字面で書かれている、`start_with_session_key` に不要な `let mut c = c;`（Task 4 F3・F5・F6）
- kind: technical
- 提案: 字面を定数に、不要な束ね直しを消す。期限の 0 は理由をコメントにするか引数で渡す

## R14. `bin/openapi.rs` の `collector_allowed` がサーバの判定の写し
- 成果物: crates/server/src/bin/openapi.rs
- 根拠: `collector_allows` の文字列版を手で写し、除外（`/healthz`・`/session` の POST と DELETE）も手書き。サーバの判定と食い違っても `check-openapi.sh` は気付けない（Task 7 F2・F3）
- kind: technical
- 提案: `ashiato_server::collector_scope` の `COLLECTOR_ALLOWED` / `collector_allows` を直に使う

## R15. 本人の機械での移行の手順を読む場所が無い
- 成果物: docs/briefs/ST44-pr.md（finish で書く）
- 根拠: design の Migration Plan の 1（本人の `.env` に `WEB_SESSION_KEY` を足してから入れ替える）は、利用者向けには `docs/network.md` §6 の ST28 の手順の中にしか無い
- kind: technical
- 提案: PR 本文に「merge 後に本人が行うこと」（`WEB_SESSION_KEY` を足してから入れ替える・ログインが 1 回切れる・手で打つ読みの curl は `tools/web-login.sh` から）を書く
