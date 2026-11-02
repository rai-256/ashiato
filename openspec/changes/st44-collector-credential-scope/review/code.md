# ST44 code review

## final review（615fc4e..e5d82bb）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 2 / Minor 13。判定は「With fixes」。
reviewer が手元で走らせたもの: `check_scenarios.py`（OK）、`tools/check-api-token-reads.sh`（web/e2e・tools/recording・smoke(.d)・seed(.d)・stack・verify-prep・record-env に rc=0、`--self-test` rc=0）、
`cargo test -- --list` の本数（`st44_scope_` 15・`st44_routes_` 3・`web_session` 27・`st44_` 25・`server_startup st44_session_key_` 2。どれも tasks.md の下限以上）。
申し送り（`docs/handoff/ST44.md`）の 2 件 —— `GET /feelings/reminder` と `GET /outages` を収集アプリの資格情報で読める —— は許可の一覧の 9 つに入り、`st44_scope_feelings_reminder`・`st44_scope_outages` が置かれている（試験の強さは R1）。
Task ごとの ⚠️ は final reviewer が全体で確かめた。解消: T1 の D9（`st44_routes_without_credentials`）と HEAD（lib.rs の明示の `.head()`）、T2 の印の名前、T4 の network.md §4/§6 と本数、T5 の seed の型と D13 の置き換え先、T6 の他の Bearer の読み。
残るもの: T1 F10 → R1、T4 F7 → R2、T6 の auth.ts → R8、T7 の st25-seed-check → R11。
fix は 1 回（1f3039c..3797ac7、12 commit）。指摘の外に 1 つ: clippy `unnested_or_patterns`（3797ac74。振る舞いは変えない）。fix の担当が走らせたもの: `cargo test` の `st44_`（25）・`access_log`・`web_session`・`server_startup st44_` rc=0、clippy -D warnings・fmt・check-openapi・check-api-token-reads（--self-test 含む）・check_scenarios・tsc rc=0、触った e2e 7 本 104 passed（全 243 本は走らせていない）、`st25-seed-check.sh` 成功。
scoped re-review 1 回で R1〜R14 は全部 ADDRESSED、新しい破損なし。R15 は PR 本文で処置する（finish）。

## R1. Scenario の印を持つ試験の 4 本が Scenario の THEN を確かめていない
- 成果物: crates/server/src/collector_scope_tests.rs
- 根拠: `st44_scope_outages` は `App::for_test` の warmup が `OUTAGE_WARMUP` のまま（lib.rs:486）で応答はいつも `warming_up`、`judged_at` は WarmingUp にもあるので Ready の道を通らずに真（tasks 1.1「試験の時計で過ぎた状態にする」とずれる）/
  `st44_scope_feelings_reminder` は記録を入れず 200 だけ見る（THEN「その日の記録があることを示し、ひとことの文字列も尺度の値も含まない」）/
  `st44_scope_sends_pass` は `{}` を送って `!= 403` だけ（THEN「どの求めも 200 で、送った 1 件ずつが受け付けられた結果」）/
  `st44_scope_feelings_reminder_post_403` は通知の時刻が変わらないことを DB で見ていない（Task 1 F10）
- kind: technical
- 提案: `/outages` は warmup を 0 にして `status == "ready"` と `outages` の配列を断言。通知の判断はひとことと尺度を持つ記録を入れて `recorded == true`・鍵が `["at","recorded"]` だけ・ひとことの文字列が無いを断言。送る口は形の正しい 1 件で 200 と受け付けを断言。POST 403 は前後の時刻の一致を断言
- 処置: fixed 1.1 —— 4 本を THEN まで: 送る口は形の正しい 1 件で 200・accepted・duplicate=false、通知の判断はひとことと尺度を持つ記録を置き鍵が ["at","recorded"]・recorded=true・本文を含まない、POST 403 は台帳の行数と時刻が前後で同じ、/outages は試験の時計で待ちを過ぎ status=ready と outages 配列（b744cee9）

## R2. `.env.example` の雛形の `WEB_SESSION_KEY=change-me-web-session-key` が起動の検査を通る
- 成果物: .env.example / crates/server/src/web_session.rs（`check_web_session_key`）
- 根拠: `.env.example:23`。`check_web_password` には `change-me` の拒否（Placeholder）があるが鍵の検査には無い。雛形を写した `.env` では HMAC の鍵が公開値になり、D7 が塞いだ「API の合言葉と DB の写しだけで画面の合言葉を確かめられる」が戻る（Task 4 F7）。拒否の理由は Global Constraints が 4 つに固定（D8 は仮）
- kind: technical
- 提案: 計画を変えずに、雛形の値を空にする（`missing` で起動しない）。`env-init.sh` は乱数で作るので、それを使う経路は変わらない
- 処置: fixed 4.2 —— `.env.example` の `WEB_SESSION_KEY` を空にし乱数で作る注記（`missing` で起動しない。拒否の理由は 4 つのまま）。dev/stack/smoke/check-panic-log は `${WEB_SESSION_KEY:-…}` で影響なし（ee583ac0）

## R3. 新しく書いた分岐に `if let Caller::ApiToken = caller` が 1 か所残る（D6）
- 成果物: crates/server/src/web_session.rs
- 根拠: web_session.rs:517。D6 は新しい分岐を `_` の無い全種類の match にして、種類が増えたらコンパイラが止める形にする（Task 1 F8）
- kind: technical
- 提案: 全種類を並べた match にする
- 処置: fixed 3.1 —— 全種類を並べた `_` の無い match に。衛星は 403 の腕（1c38ea0c）

## R4. middleware の 2 つ目の match が 1 つ目の判定を文字列の比較で読み直している
- 成果物: crates/server/src/access_log.rs
- 根拠: `Some(Caller::ApiToken) if outcome == "forbidden"`（Task 1 F6）。判定の値と文字列が食い違うと黙って外れる
- kind: technical
- 提案: 1 つ目の判定の結果（bool か enum）を持ち回る
- 処置: fixed 1.1 —— 判定を enum `Verdict` で持ち回り、文字列の読み直しを消した（3b17abb7）

## R5. `credential_present` と `authorize` で「資格情報がある」の意味が違う
- 成果物: crates/server/src/access_log.rs
- 根拠: `credential_present` は authorization か cookie の見出しの有無、`authorize` は空でない Bearer の有無（Task 1 F7）。同じ warn が 2 つの道で違う条件で出る
- kind: technical
- 提案: 同じ条件にそろえるか、違いの理由をコメントに書く
- 処置: fixed 1.1 —— `lib.rs` の `credential_present` 1 本を `authorize` と middleware が使う（3b17abb7）

## R6. `/session` の GET の期待を 403 に替えた断言に ST44 の理由のコメントが無い
- 成果物: crates/server/src/access_log_tests.rs
- 根拠: tasks 1.2 は 1 行のコメントを求める。`grep ST44 crates/server/src/access_log_tests.rs` が 0 件（Task 1 F9）
- kind: technical
- 提案: D12 を指す 1 行のコメントを足す
- 処置: fixed 1.2 —— ST44 D12 を指す 1 行のコメント（bc2212f1）

## R7. `st44_scope_web_session_unchanged` が `/ingest` を叩いていない
- 成果物: crates/server/src/collector_scope_tests.rs
- 根拠: tasks の指定は「記録・滞在・日ごとの気分の読み出しと `/ingest`」
- kind: technical
- 提案: 画面のログインの印で `/ingest` に 1 件送り、範囲が変わらないことを断言する
- 処置: fixed 1.1 —— 読みを 2xx の断言に強め、印で `/ingest` に 1 件送り受け付けを断言。隠れていた `/stays` の 400 は `?date=` で直した（b744cee9）

## R8. `web/e2e/auth.ts` がモジュールの読み込み時に `state.json` を読む
- 成果物: web/e2e/auth.ts
- 根拠: `npm run test:e2e` は globalSetup の後に試験を読むので CI は落ちない（Playwright runner の順を確かめた）。`--list`・UI・watch の型は globalSetup の前に読むので、新しい checkout で throw する（Task 6 ⚠️）
- kind: technical
- 提案: 初めて呼ばれたときに読み、結果を持つ
- 処置: fixed 6.1 —— auth.ts は初回呼び出しで読んで保持。e2e 7 本・録画 4 本のモジュール頭の呼び出しを使う場所へ。state.json を退けて `playwright test --list` rc=0（d9a18a00）

## R9. 録画が `../../../web/e2e/auth.js` を相対の深さで読む
- 成果物: tools/recording/scenarios.d/（ST44 で書き換えた録画の spec）
- 根拠: 写し先 `web/e2e-recording/<id>/` からも同じ深さで解決する（確かめた）が、深さの偶然の一致に乗っている。シナリオの `requires` に `web/e2e/auth.ts` が無い
- kind: technical
- 提案: 該当シナリオの `requires` に `web/e2e/auth.ts` を足す
- 処置: fixed 6.2 —— 録画シナリオ 4 つ（st17・ch01・st23・ch08）の requires に `web/e2e/auth.ts`（301b07d6）

## R10. `tools/record-env.sh` の注記が古く、ログインの失敗を受けていない
- 成果物: tools/record-env.sh
- 根拠: :229 の注記「録画の spec が画面の外から `/api` を直に読む…ときの Bearer」は締める前の形。`archive_prepare` の `read_header="$(tools/web-login.sh …)"` は `|| return 1` が無く、`||` の中なので errexit も効かず、後の curl の 401 で落ちる（Task 5 F1）
- kind: technical
- 提案: 注記を直し、`|| return 1` を足す
- 処置: fixed 5.2 —— `archive_prepare` がログインの失敗と空の見出しで return 1、注記を送る用に直した（f2df5061）

## R11. `tools/st25-seed-check.sh` はログインの失敗で原因の分からない落ち方をする
- 成果物: tools/st25-seed-check.sh
- 根拠: `set -e` が無く、ログインに失敗すると空の見出しで 401 → jq の失敗として落ちる（Task 7 F4）。cookie で `/day` を読む流れを走らせた証跡も無い（Task 7 ⚠️）
- kind: technical
- 提案: ログインの失敗を明示して exit する
- 処置: fixed 7.1 —— ログインの失敗・空の見出しで理由を出して exit 1。fix の担当がスタックを立てて 1 回走らせ成功（f2df5061）

## R12. `tools/seed.sh` の seed.d の注記に `READ` が無い
- 成果物: tools/seed.sh
- 根拠: 末尾の注記「AUTH・BIND・work はそのまま使える」。seed.d の台本が読むときに使う `READ` が抜けている
- kind: technical
- 提案: 注記に `READ` を足す
- 処置: fixed 5.2 —— 注記に READ を足した（5b19189f）

## R13. `lib.rs` と起動の試験の細部
- 成果物: crates/server/src/lib.rs / crates/server/tests/server_startup.rs
- 根拠: `with_session_key` が期限を 0 に固定、`"archive-internal-session-key"` が 2 か所に字面で書かれている、`start_with_session_key` に不要な `let mut c = c;`（Task 4 F3・F5・F6）
- kind: technical
- 提案: 字面を定数に、不要な束ね直しを消す。期限の 0 は理由をコメントにするか引数で渡す
- 処置: fixed 4.1 —— `INTERNAL_WEB_SESSION_KEY` 定数、`with_session_key` は `WebLogin::with_session_key_for_test` 経由で期限を保つ、不要な束ね直しを削除（64a8c85e）

## R14. `bin/openapi.rs` の `collector_allowed` がサーバの判定の写し
- 成果物: crates/server/src/bin/openapi.rs
- 根拠: `collector_allows` の文字列版を手で写し、除外（`/healthz`・`/session` の POST と DELETE）も手書き。サーバの判定と食い違っても `check-openapi.sh` は気付けない（Task 7 F2・F3）
- kind: technical
- 提案: `ashiato_server::collector_scope` の `COLLECTOR_ALLOWED` / `collector_allows` を直に使う
- 処置: fixed 7.2 —— `bin/openapi.rs` が `collector_scope::collector_allows` を直に呼び、除外は `is_exempt` 1 つに。`docs/openapi.json` は不変（7a57f004）

## R15. 本人の機械での移行の手順を読む場所が無い
- 成果物: docs/briefs/ST44-pr.md（finish で書く）
- 根拠: design の Migration Plan の 1（本人の `.env` に `WEB_SESSION_KEY` を足してから入れ替える）は、利用者向けには `docs/network.md` §6 の ST28 の手順の中にしか無い
- kind: technical
- 提案: PR 本文に「merge 後に本人が行うこと」（`WEB_SESSION_KEY` を足してから入れ替える・ログインが 1 回切れる・手で打つ読みの curl は `tools/web-login.sh` から）を書く
- 処置: fixed 7.2 —— PR 本文（`docs/briefs/ST44-pr.md`）の「merge 後に本人が行うこと」に書いた（`WEB_SESSION_KEY` を `tools/env-init.sh` か `openssl rand -hex 24` で足してから入れ替える・足さないとサーバと `dev.sh`・`stack.sh` が理由を出して止まる・ログインが 1 回切れる・手で打つ読みの curl は `tools/web-login.sh` の印で）

## code_verify（HEAD bc74b73e。独立検証 2026-11-03）

席: code_verify（手 1〜7）。上の R1〜R15（final review）とは別に、実行して確かめたことだけを書く。作業ツリーは clean（`git status --porcelain` が空）の HEAD `bc74b73e` で走らせた。
変異はどれも作業ツリーで 1 か所ずつ当て、走らせた直後に `git checkout --` で戻し、前後の `git status --porcelain` と `git diff` が同じ（どちらも空）であることを毎回 `diff` で確かめた（10 回とも `restored=yes`）。結果は `/tmp/st44cv/`。
DB を使う検証の 1 回目（smoke rc=1・e2e rc=1・`check-immutable.sh` の `NG 202610241200_satellite_credentials が当たらない`）は、こちらが 3 本を同時に走らせて互いの `docker compose down -v` で DB を落としたためで（smoke のログに `terminating connection due to administrator command`）、実装の欠陥ではない。1 本ずつ順に走らせ直した結果だけを表に書く。

### 申告: 13/13（`- [x]` 13・`- [ ]` 0。1.1・1.2・2.1・3.1・4.1・4.2・5.1・5.2・6.1・6.2・7.1・7.2・8.1）・検証コマンド一覧と実測

`CT`・`CS` の rc は、全体の `cargo test -p ashiato-server`（1916 passed・0 failed・3 ignored、308 秒、rc=0。`tests/server_startup.rs` を含む）に含まれる。本数は `-- --list | grep -c ': test$'` で数えた。

| 項目 | 検証コマンド（tasks.md の本文どおり） | 実測（HEAD bc74b73e） | 一致 |
|---|---|---|---|
| 1.1 | `CT st44_scope_`（≥15） | rc=0・15 本 | 一致 |
| 1.2 | `cargo test -p ashiato-server` / `! grep -rn "Scenario: API の合言葉での読み書きは変わらない" crates web tools` | rc=0（1916 passed）/ rc=0 | 一致 |
| 2.1 | `CT st44_routes_`（≥3）/ `CT every_route_refuses_satellite`（≥1） | rc=0・3 本 / rc=0・1 本 | 一致 |
| 3.1 | `CT st50_`（≥20）/ `! grep -n "matches!(caller, Caller::ApiToken)" …contract_version.rs` | rc=0・42 本 / rc=0 | 一致 |
| 4.1 | `CS st44_session_key_`（≥2）/ `CS server_startup`（≥9）/ `CT st44_api_token_change_keeps_login`・`st44_session_key_change_revokes`・`st44_secret_tag_not_from_api_token`（各 ≥1）/ `CT web_session`（≥15） | rc=0・2 本 / rc=0・9 本 / rc=0・1・1・1 本 / rc=0・27 本 | 一致（ただし R18） |
| 4.2 | `tools/check-env-init-session-key.sh` / `.env.example`・`ci-db-env.sh`・`record-env.sh` の grep | rc=0（`PASS: WEB_SESSION_KEY を一度だけ追加し、既存行を保持した`）/ rc=0 | 一致（ただし R17） |
| 5.1 | `tools/web-login.sh --self-test` | rc=0 | 一致 |
| 5.2 | `scripts/quiet-run smoke -- ./tools/smoke.sh` / `check-shared-appends.sh` / `bash -n seed.sh`・`record-env.sh` | rc=0（80 秒。最後の節が `== ST44: 画面の合言葉で読む口の資格情報の範囲`）/ rc=0 / rc=0・rc=0 | 一致 |
| 6.1 | `cd web && npm run test:e2e` / `npm run typecheck` | rc=0（243 passed、4.4 分）/ rc=0 | 一致 |
| 6.2 | `check-api-token-reads.sh --self-test` / `check-api-token-reads.sh tools/recording web/e2e` | rc=0 / rc=0 | 一致 |
| 7.1 | `check-api-token-reads.sh tools/verify-prep.sh tools/st25-seed-check.sh tools/smoke.sh tools/smoke.d tools/seed.sh tools/seed.d tools/record-env.sh` / `bash -n` 2 本 | rc=0 / rc=0・rc=0 | **rc は一致、検査は空振り（R16）** |
| 7.2 | `./tools/check-openapi.sh` / `network.md`・`collector-contract.md` の grep | rc=0（`API の契約 OK`）/ rc=0 | 一致 |
| 8.1 | `check_scenarios.py . st44-collector-credential-scope` / `cargo test -p ashiato-server` / `check-immutable.sh` / `check-panic-log.sh` | rc=0（`scenarios: OK`）/ rc=0 / rc=0（`書き換え禁止 OK`）/ rc=0 | 一致 |

`cargo test --test server_startup` の `tests/server_startup.rs` は実在する。

**手 1（固定値の再計算）: 該当なし（一致）。** ST44 の試験が直書きする値は許可の一覧の 9 つ（`st44_scope_allowlist_is_nine` の `expected`）。これを実装（`collector_scope.rs`）からでなく、収集アプリのソースから拾い直した —— 携帯: `LocationService.kt:937/949/957/962`（`newTransport("/ingest"|"/heartbeat"|"/drops"|"/media/missing")`、`HttpTransport` は POST）・`:254`（GET `/outages`）・`:258`（GET `/contract`）・`FeelingReminderSource.kt:48-49`（GET `/feelings/reminder`）・`OriginalAndroid.kt:57/63/79`（HEAD・PUT `/media/originals/{sha256}`）。PC: `sender.rs:120/130`（POST `/ingest`・`/heartbeat`）・`sender.rs:764`・`runtime.rs:815`（GET `/contract`）・`sender.rs:441`（GET `/outages`）。和集合は 9 つにちょうど一致し、一覧の外の口を呼ぶ箇所は無い。HMAC の印（`secret_tag`）を直書きの値で固定する試験は無い（比べる側も実装の関数を呼ぶ）。

**手 2（ガードを壊す）:**
- 許可の判定: `collector_allows` を常に真（`collector_scope.rs:20`）→ `CT st44_` rc=101（`st44_scope_feelings_403`・`events_403_logged`・`both_headers_403`・`feelings_reminder_post_403` ほか）/ middleware の `Some(Caller::ApiToken) if !collector_allows` の腕を殺す（`access_log.rs:179` に `false &&`）→ rc=101（同じ 4 本ほか。試験が `router()` を通って middleware に掛かっている）
- 資格情報なしの 401（D9）: `access_log.rs:213` の `return 401` を外す → `CT st44_routes_` rc=101（`st44_routes_without_credentials` が `POST /attributes/kinds` で 401 でない）
- `GET /session` の 403（D12）: `web_session.rs` の `Caller::ApiToken` を `Caller::WebSession` の腕へ移す → `CT st44_scope_session_get` rc=101
- 印の鍵: `secret_tag` の HMAC の鍵を空の固定値にする → `CT st44_` rc=101（`st44_secret_tag_uses_session_key`・`st44_secret_tag_not_from_api_token`・`st44_session_key_change_revokes`）
- `env-init.sh` の行を足す 4 行を消す → `check-env-init-session-key.sh` rc=1
- `check-api-token-reads.sh`: 録画の `ch08-nested.rec.ts` の `readAuthHeaders()` を `BEARER` に戻した複製 → rc=1（4 か所を挙げる）/ e2e の型（`request.get("/api/events", { headers: { Authorization: \`Bearer ${process.env.API_TOKEN …}\` } })`）→ rc=1。**bash の台本の側は空振り**（**R16**）
- 鍵の長さの下限 16 を 14・15・20 にする → どれも rc=0（**R18**）

**手 3（Scenario と test）:** `check_scenarios.py` rc=0（この change の 63 本の Scenario は全部に印がある。印の先は `collector_scope_tests.rs`（印 21 個）・`web_session_tests.rs`・`server_startup.rs`・既存の `access_log_tests.rs`・`login.spec.ts`・`smoke.sh` など）。
主張の階層との突き合わせ: 許可の一覧の外の全口（契約から機械で作る）・資格情報なしの全口・読み出しの記録の行（Tee で 1 行ずつ）・通知の判断の鍵が `["at","recorded"]` だけ・POST の 403 の前後で台帳の行数と時刻が同じ、は THEN を観測している（上の変異で落ちることも確かめた）。
ずれは 1 か所: 「ログインの印の鍵が短いか、ほかの合言葉と同じだと起動しない」の WHEN は「**15 文字の値**」だが、試験の短い値は 13 文字（`"short-session"`）と 5 文字（`"short"`）（**R18**）。
（観測が弱いが指摘にしないもの: `st44_scope_cookie_only_reads`・`st44_scope_wrong_bearer_with_cookie` の THEN は「記録を返す」だが、試験は記録を置かずに status と行の種類だけを見る。読みのハンドラは ST44 で変わらないので失うものは無い）

**手 4（本人の決定）:**
- Q1（`API_TOKEN` を送るだけに締める・9 つ）: 一覧の数と中身は `st44_scope_allowlist_is_nine`・`st44_routes_allowlist_in_contract` が、外の口の 403 は手 2 の変異が落とす。固定されている
- Q2（共有の 1 本・どの資格情報で届いたかの欄を作らない）: `git diff --stat 615fc4e7 HEAD` に `migrations/` は 0 件、`collector-android`・`crates/collector-windows` の変更も 0 件。欄を作らないことを試験は持たないが、作らない決定なので試験で固定する対象が無い
- Q3（画面のログインの範囲は変えない）: `st44_scope_web_session_unchanged`（読み 3 つと `/ingest`）
- 仮の D8 の数値「16 文字以上」: **値を 14・15・20 に変えても全部通る**（`MIN_WEB_SESSION_KEY_LEN`。`web_session.rs:33`。**R18**）。拒否の理由の 4 つは `server_startup.rs` の 2 本と `st44_session_key_validation_rejects_missing_short_and_duplicates` が固定
- 仮の D11・D12・D10: `st44_scope_both_headers_403`・`st44_scope_session_get_403`（変異で落ちる）・`st44_scope_wrong_bearer_with_cookie`

**手 5（tasks の `[x]` と実体）:** 13 項目とも検証コマンドが実在し rc=0、本数も下限以上（上の表）。7.1 の `check-api-token-reads.sh` は rc=0 だが bash の台本では 1 件も検査していない（**R16**）。

**手 6（隙間）:** 「捨てたものは復元できない」型は見つからなかった。確かめた範囲: 収集アプリが一覧の外を呼んで 403 を受け、溜めた記録を捨てる経路 —— 手 1 のとおり両アプリとも一覧の外を呼ばない / 移行なし（記録の形は変わらない）/ `WEB_SESSION_KEY` を足し忘れたサーバは起動しないが、収集アプリは溜めて待つ（design Risks の 1 番目。携帯 90 日・2 GB）/ 入れ替えの最初の起動でログインが 1 回切れる（記録は失わない）。
日々の起動の入口で D8 の「無ければ起動しない」が働かない経路が 1 つある（**R17**）。

**手 7: 該当なし**（`python3 scripts/plan_fix.py pending openspec/changes/st44-collector-credential-scope` → `[]`。`correction-review.md` は作らない）。

## R16. `check-api-token-reads.sh` は URL に `/api/` が付く求めしか見ないので、bash の台本（`tools/smoke.sh` ほか）の API の合言葉の読みを 1 件も検査していない
- 成果物: tools/check-api-token-reads.sh / tasks.md 7.1（と 5.2 の規則）
- 根拠: 口を拾う正規表現は `URL = [\"'\`](/api/…)` と curl の fallback `https?://[^\s\"']+(/api/…)` の 2 つだけで、どちらも `/api/` を要る。`grep -c '/api/' tools/smoke.sh` → 0（台本は `http://$BIND/events` の形）。
  実測: `smoke.sh`・`seed.sh`・`smoke.d/202610141000_st17-daily-feeling.sh` の `"${READ[@]}"` を全部 `"${AUTH[@]}"` に戻した作業ツリー（`AUTH[@]` が smoke.sh 30・seed.sh 14・st17 4 か所）で 7.1 の検証コマンドが **rc=0**（戻した後 `restored`）。
  単発の複製でも `curl -sf "${AUTH[@]}" "http://$BIND/events?…"` → rc=0、同じ行に `/api` を付けると rc=1。curl の枝は `AUTH[@]` しか見ないので `-H "authorization: Bearer $API_TOKEN" "http://$BIND/api/events"` と `-H "$AUTH_HEADER" …/api/events` も rc=0。
  `--self-test` の例は全部 `/api/` 付きなので、この空振りを見抜けない。TS（e2e・録画）は `"/api/…"` の直書きなので検出する（手 2）
- kind: technical
- 提案: curl の枝で `/api` の無い `http://…/<口>` と `$BIND/<口>` も口として拾い、Bearer の見出し（`authorization: Bearer`・`$API_TOKEN`）も資格情報として見る。`--self-test` に `/api` の無い bash の例（`"${AUTH[@]}" "http://$BIND/events"` が落ちる・`/ingest` が通る）を足す
- 処置: fixed 7.1 —— curl の枝が `/api` の無い `http://$BIND/<口>`・`$BIND/<口>`・`http://host:port/<口>` を拾い、`/api` を剥がして比べる。資格情報は `AUTH[@]` のほか `$API_TOKEN`・`$AUTH_HEADER`（衛星の `Bearer $SAT_VALUE` は見ない）。`-d`/`-I`/`-T` から方法を決める。`--self-test` に `/api` の無い bash の例（通る 6・落ちる 6）を足した。smoke.sh の `READ[@]` を `AUTH[@]` に戻した複製で rc=1（`GET /events` ほか）、7.1・6.2 の verify-run は PASS

## R17. `dev.sh`・`stack.sh` は `WEB_SESSION_KEY` が無いと公開の固定値で起動し、D8 の「無ければ起動しない」と R2 で塞いだ公開の鍵が日々の起動の入口で戻る
- 成果物: tools/dev.sh:13 / tools/stack.sh:36（tools/smoke.sh:15・tools/check-panic-log.sh も同じ形）/ design D8（仮）/ tasks.md 4.2
- 根拠: `env -u WEB_SESSION_KEY` で各行を評価 → `dev.sh: dev-session-key-0123456789abcdef`・`stack.sh: dev-session-key-0123456789abcdef`。1 行上の `WEB_PASSWORD` は `:?` で止まる（`env -u WEB_PASSWORD` で rc=127・`.env を読み込む…か WEB_PASSWORD を渡す`）。
  本体の作業ツリー `~/dev/ashiato2/.env` の `^WEB_SESSION_KEY=` は 0 行。`dev.sh`・`stack.sh` は `env-init.sh` を呼ばない（`grep -n env-init tools/dev.sh tools/stack.sh` が 0 件）。
  よって merge の後に本人が `./tools/dev.sh`（`docs/production-prep.md:149` の起動の入口）で立てると、拒否されずに repo に載った鍵で印を作る —— R2 が `.env.example` から消した「公開された鍵と DB の写しで画面の合言葉の候補を確かめられる」状態が、別のファイルから戻る。
  実装は tasks 4.2・D8 の「`API_TOKEN` と同じく、無ければ台本の中の既定の値」のとおりで逸脱ではない（だから premise）
- kind: conflict
- 分類の直し: code_verify は premise。finish が conflict に直した: 崩れたのは本人の答え（deep.md の Q）でなく AI の仮決め D8 の中の食い違い —— 同じ D8 の「無ければ起動しない」と「渡す側は無ければ台本の既定値」。どちらの側も記録を失わず、仮の台帳で別の席が確かめる
- 提案: `dev.sh`・`stack.sh` は `WEB_PASSWORD` と同じく `:?` で止める（使い捨ての DB を作る `smoke.sh`・`check-panic-log.sh` は既定値のままでよい）か、既定値を使ったときは起動前に警告を出す。D8 の「渡す側」の段落を直す
- 処置: fixed D16 仮 —— P（失わない側: 公開の鍵で起動しない。止まるだけで記録は失わない）。`dev.sh`・`stack.sh` は `WEB_PASSWORD` と同じ `:?` で「tools/env-init.sh で足す」と出して止まる。使い捨ての DB の `smoke.sh`・`check-panic-log.sh` は既定値のまま。CI は ci-db-env.sh、verify-prep は stack.sh の前に env-init.sh、録画は record-env.sh が毎回鍵つきで書く。design D16（仮）と D8 の渡す側の注記、`provisional.py add --id D16 --rests-on D8`

## R18. 鍵の長さの下限 16 は、14・15・20 に変えても全部の試験が通る（spec の「15 文字の値」を試していない）
- 成果物: crates/server/src/web_session.rs:33（`MIN_WEB_SESSION_KEY_LEN`）/ crates/server/tests/server_startup.rs（`st44_session_key_short_or_same_refuses`）/ crates/server/src/web_session_tests.rs（`st44_session_key_validation_rejects_missing_short_and_duplicates`）
- 根拠: `MIN_WEB_SESSION_KEY_LEN` を 15・14・20 にした作業ツリーで `cargo test -p ashiato-server st44_session_key` → 3 回とも rc=0（lib 2 passed・server_startup 2 passed。戻した後 `restored`）。`check_web_session_key` を呼ぶのは `run()` とこの単体試験だけ（grep）。
  試験の「短い」値は `"short-session"`（13 文字）と `"short"`（5 文字）、通る値の最短は `"test-session-key-0123456789"`（27 文字）なので、下限は 14〜27 のどこでも緑。spec の WHEN は「ログインの印の鍵を **15 文字の値**…にして起動する」で、下限 15 なら 15 文字の鍵で起動してしまう
- kind: technical
- 提案: `server_startup.rs` の短い値を 15 文字にし、単体試験に 16 文字ちょうどが `Ok` になる断言を足す（境界の両側）
- 処置: fixed 4.1 —— `server_startup.rs` の短い値を 15 文字（`short-session15`、長さを断言）に、単体試験に 15 文字が TooShort・16 文字ちょうどが Ok の断言を足した。下限を 15 にすると lib・startup とも rc=101、17 にすると lib が rc=101（境界の両側で落ちる）

## security review（615fc4e..bc74b73）

見た経路:
- API の合言葉 → `strip_api_prefix`（lib.rs:4262）→ `refresh_cookie`（判定 1 回）→ `access_log::middleware`（`route_layer`。lib.rs:4358。`api_router` の口は全部この前に `.route` で並ぶ）→ `collector_allows(method, MatchedPath)` で一覧の外は行を書いて 403、ハンドラも本文の読みも呼ばない（access_log.rs:179）（4）
- 口の一覧の漏れ: `api_router` の `.route` の道と `docs/openapi.json` の `paths` を突き合わせ → 差は `/healthz` だけ（`diff` で確認）。よって契約から作る `st44_routes_*` の 2 本が router の全部の口を覆う（4）
- 変種: 契約の全 44 口 × {`/api` 付き・末尾の `/`・`//` 始まり・`/api/api`・HEAD・OPTIONS} の 264 求めを API の合言葉で `router()` に送る一時の試験 → 一覧の外で 2xx は `DELETE /api/session`（資格情報を求めない口）だけ。GET の口に axum が足す HEAD も 403（`HEAD /events`）。`/api/api/…` と末尾 `/` は 404 で fallback（画面の静的ファイル）へ落ち、記録は返らない（4）
- 資格情報なし → middleware が引数より先に 401（access_log.rs:205-214。`HEAD /events` も 401）（4）
- `GET /session` を API の合言葉で → `session_get` の `_` の無い match で 403 と `forbidden` の行（web_session.rs:525-541。実行して 403 `forbidden`）（4）
- API の合言葉から印を得る経路: `refresh_cookie` は `Authn::Decided(Some(WebSession))` のときだけ `Set-Cookie` を足す（web_session.rs:290）。`POST /session` は合言葉の本文だけを見る（3）
- 印の鍵: `WEB_SESSION_KEY` → `check_web_session_key`（拒否は `reason()` の固定語だけを tracing と stderr に出す。lib.rs:4422-4430）→ `WebLogin::new` が HMAC の tag だけを持ち、鍵の写しは `#[cfg(test)]` の欄だけ（web_session.rs:199-216）。DB の行は tag のまま（3）
- 401 のログ: `credential_present` は見出しの有無の真偽だけで値を載せない（lib.rs:791-794）（2）
- 道具: `tools/web-login.sh` は合言葉を `mktemp`（0600）の本文ファイルで curl に渡し argv に出さず、印は標準出力の 1 行だけ。失敗時は値を出さず rc 1。`seed.sh`・`smoke.sh`・`record-env.sh`・`st25-seed-check.sh` は印を変数で持ち、echo しない（3）
- 秘密の固定値: 追加行の `PASSWORD=`/`WEB_SESSION_KEY=` の実値と `ashiato_session=<値>` を `git diff | grep` → `.superpowers`・`evidence.jsonl` を含めて 0 件。`.env.example` は空、CI は `openssl rand` + `::add-mask::`。台本の既定値（`dev.sh`・`stack.sh`）は R17 が既に挙げている（3）

安全の決め手: **`api_router` の口は全部 `route_layer(access_log::middleware)` の内側にあり、`Caller::ApiToken` は `collector_allows`（(方法, `MatchedPath`) の 9 つの許可の一覧）が偽なら行を書いた後にハンドラを呼ばず 403 で返る** —— 確かさ 4。
`collector_scope_tests.rs` の末尾に一時の試験 `zz_secprobe_variants`（上の 264 求め）と `zz_secprobe_session_get_cookie_plus_token` を足して `cargo test -p ashiato-server --lib zz_secprobe -- --nocapture` → rc=0、`SUCCESS-OUTSIDE` は `DELETE /api/session -> 204` の 1 行だけ、`GET /session`・`HEAD /session` は 403、資格情報なしの `HEAD /events` は 401。試験はファイルを戻して消した。
あわせて既存の `cargo test -p ashiato-server --lib st44_routes_` → 3 passed。

指摘なし（この段で新しく足す R は無い。日々の起動の入口で公開の鍵が使われる経路は R17 が既に挙げている）

範囲外: 道具（`smoke.sh`・`verify-prep.sh` の手順の 1 行）は印を `curl -H` の引数で渡すので、同じ機械のほかの利用者が `/proc/<pid>/cmdline` で見られる。差分の前の `Bearer $API_TOKEN` の渡し方と同じ形。

## /code-review（615fc4e..bc74b73）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R19. stack.sh, dev.sh and smoke.sh fill WEB_SESSION_KEY with a public literal (`${WEB_SESSION_KEY:-dev-session-key-0123456789abcdef}`) whenever it is empty or unset. That silently skips the server's `Missing` refusal, and check_web_session_key does not reject these known values.
- 成果物: tools/stack.sh:36
- 根拠: Someone copies .env.example (which now has `WEB_SESSION_KEY=` empty and says the server will not start) or keeps an older .env without the line, then runs `tools/stack.sh up`. The `:-` fallback swaps in `dev-session-key-0123456789abcdef`, the server starts, and secret_tag is an HMAC under a key published in the repo. Anyone holding a DB copy can then test WEB_PASSWORD guesses offline, which is the thing D7 is meant to prevent.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D16 仮 —— R17 と同じ処置（`dev.sh`・`stack.sh` は既定値を持たず止まる）。`smoke.sh` は使い捨ての DB（走らせた後に `down -v` で消え、本人の記録を持たない）なので既定値のまま

## R20. The new step that appends WEB_SESSION_KEY sits after the early `exit 0` for a .env that already has COMPOSE_PROJECT_NAME, so those .env files never get a key.
- 成果物: tools/env-init.sh:14
- 根拠: On a tree whose existing .env has a COMPOSE_PROJECT_NAME= line (such as a verify or recording tree created before ST44), env-init.sh exits before reaching the append. Without the stack.sh fallback the server then stops with reason=missing; with it, the server silently runs on the public dev key.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `COMPOSE_PROJECT_NAME` の行を持つ `.env` は録画の `.env` だけで、`tools/record-env.sh` の `up()` が毎回 `cat >.env` で鍵つきに書き直す（:95-128）—— 古い録画の `.env` が残って使われる経路は無い。さらに `check-verify-env-reset.sh` の既存の Scenario「compose project の名前を持つ .env には触らない」（:435-440、`cmp` で同じことを見る）が、ここで行を足すことを禁じている。万一鍵が無くても、D16（仮）で `stack.sh` は理由を出して止まり公開の鍵では起動しない

## R21. The checker only finds URLs that contain a literal `/api/` prefix, but smoke.sh, seed.sh and smoke.d call `http://$BIND/<path>` with no `/api`. Any `curl "${AUTH[@]}"` read in those scripts therefore passes unflagged.
- 成果物: tools/check-api-token-reads.sh:66
- 根拠: Probed: a file containing `curl -sf "${AUTH[@]}" "http://$BIND/events"` gives rc=0. The task-6/7 check `tools/check-api-token-reads.sh tools/smoke.sh tools/smoke.d tools/seed.sh ...` passes no matter what those scripts contain, so a future Story that reads with the API token in smoke.d only shows up as a 403 at run time.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— R16 と同じ処置。`curl -sf "${AUTH[@]}" "http://$BIND/events"` は rc=1 になった（`--self-test` の `curl-bind.sh`）

## R22. `READ=(-H "$(tools/web-login.sh ...)")` stops the script under errexit when login fails, but web-login.sh prints nothing on failure, so smoke and seed exit 1 with no explanation. Commit f2df5061 added an error message for this in record-env.sh and st25-seed-check.sh but not here.
- 成果物: tools/smoke.sh:53
- 根拠: WEB_PASSWORD does not match the running server, or POST /session returns 429 or 500. `tools/smoke.sh` and `tools/seed.sh` exit 1 right after healthz with no output, which looks like a hang or crash. seed.sh also logs in before its `case "$MODE"`, so `seed.sh foo` with no server running exits 1 silently instead of printing the usage line.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.2 —— `tools/web-login.sh` が失敗の理由（繋がらない・`POST /session が <番号>`・印が無い。合言葉と印は出さない）を stderr に出し、`--self-test` が 401 の理由が出ることと合言葉が出ないことを確かめる。`smoke.sh`・`seed.sh` はログインの失敗と空の見出しで `error: 画面にログインできない…` を出して exit 1。`seed.sh` のログインは使い方の判定の後へ移した。smoke rc=0

## R23. credential_present is now true for any `Cookie` or `Authorization` header, including cookies unrelated to `ashiato_session` and non-Bearer schemes. The earlier check meant 'a Bearer value was sent'.
- 成果物: crates/server/src/lib.rs:793
- 根拠: A browser with only some unrelated cookie for the host (tailscale, the vite dev server) calls a read route with no ashiato login and gets 401. The log says `credential_present=true`, the same as an expired login or a wrong token. The R27/R5 log was meant to tell 'nothing sent' apart from 'wrong credential', so this misleads triage. Checking `cookie_token(headers).is_some()` and a Bearer prefix would keep the meaning.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 1.1 —— 再現: 旧の `credential_present` は `cookie: ts_session=abc` だけで真。空でない `Bearer` か `ashiato_session` の cookie（`web_session::cookie_token`）だけを数える形にし、`st44_credential_present_counts_only_bearer_and_session_cookie`（ほかの cookie・Basic・空の Bearer は偽、ずれた Bearer・期限切れの印は真）を足した

## R24. The collector scope rule is enforced inside the access-log middleware, which skips /session and /healthz. session_get and contract_get each repeat their own caller match, and openapi.rs has a third copy of the exemptions in is_exempt.
- 成果物: crates/server/src/access_log.rs:179
- 根拠: The middleware's string list (`/healthz`/`/session`), openapi.rs `is_exempt` and session_get's own 403 branch have to be edited together by hand. If a new route is added outside route_layer (as `/selftest/panic` already is), or a new exempt path is added in one place only, the API token reaches it without a scope check or the contract shows the wrong 403. A dedicated scope layer (or one shared exemption constant) applied to every route would remove these copies.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.2 —— 再現: 資格情報を求めない口の判定が `bin/openapi.rs` の `is_exempt` に手で写されていた。`collector_scope::credential_exempt` に置き、`openapi.rs` はそれを直に呼ぶ（`docs/openapi.json` の 403 は不変、check-openapi rc=0）。middleware が `/session` を丸ごと通すのは「ハンドラが自分で行を書く」ためで免除とは別の意味なので、その旨を `credential_exempt` の注記に書いた。専用の層への作り直しは採らない: security review が `api_router` の全口が `route_layer` の内側にあること（契約との diff は `/healthz` だけ）と 264 求めの変種を確かめており、`/selftest/panic` は既定で生えない（design Risks）。試験の側の免除の一覧（`collector_scope_tests.rs` の `is_exempt`）は実装から独立に持つ（試験が実装の定義を引くと食い違いを見つけられない）

## R25. The checker keeps its own copy of the 9 allowed routes (ALLOWED) instead of reading `collector_scope::COLLECTOR_ALLOWED`. The openapi.rs commit fixed the same duplication (R14) and this one remains.
- 成果物: tools/check-api-token-reads.sh:58
- 根拠: Adding or removing an entry in COLLECTOR_ALLOWED leaves the Python list unchanged. The checker then rejects a call the server allows, or lets through one the server rejects. It could derive the list from docs/openapi.json (a route without 403) or from a small bin that prints COLLECTOR_ALLOWED.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— `check-api-token-reads.sh` は許可の一覧を `crates/server/src/collector_scope.rs` の `COLLECTOR_ALLOWED` から読む（写しを消した。読めなければ rc=2）。`{sha256}` は 1 区切りに当てる

## R26. No CI job or check runs the new checks (`check-api-token-reads.sh`, `check-env-init-session-key.sh`). They are only called from the task verification lines in tasks.md.
- 成果物: tools/check-api-token-reads.sh:1
- 根拠: Rule in CLAUDE.md: 「機械が判定できることは機械に決めさせる。規約に書く・人間に聞くで代替しない」. A later Story that adds `{ headers: BEARER }` to a read in web/e2e or tools/recording passes CI, and the problem only shows as a 403 at e2e run time.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— `.github/workflows/ci.yml` の検査の job に `check-api-token-reads.sh --self-test`・同じ台本の全部（web/e2e・tools/recording・smoke(.d)・seed(.d)・verify-prep・st25-seed-check・record-env・stack）・`check-env-init-session-key.sh` を足した。手元で 3 本とも rc=0、`check-shared-appends.sh` OK

## R27. session_get's doc comment still says the API token is accepted (「印か API の合言葉で認められれば、その種類を返す」), and its utoipa responses list only 200 and 401, but the handler now returns 403 for ApiToken.
- 成果物: crates/server/src/web_session.rs:484
- 根拠: Readers and the generated contract describe GET /session wrongly. The 403 appears only because openapi.rs adds it to every route, under the generic text 「API の合言葉では通らない」. Anyone reading the handler annotation expects API-token callers to get 200 with credential=api_token.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 —— `session_get` の注記を「画面の印で認められれば…。API の合言葉と衛星の資格情報では 403」に、utoipa の `responses` に `403`（`API の合言葉では通らない`。openapi.rs が足す文と同じ）を足した。`docs/openapi.json` は summary の 1 行だけ変わり、check-openapi・check-contract-version rc=0

## R28. Each call also passes `headers: readAuthHeaders()`, even though the `request` fixture already sets `extraHTTPHeaders: readAuthHeaders()`. The same repeat appears in st23-purge.spec.ts and ch08-nested.spec.ts (erase and purge calls).
- 成果物: web/e2e/places.spec.ts:166
- 根拠: The cookie is sent twice by two mechanisms, which makes it look as if the fixture does not authenticate. When the auth method changes, both places need edits. The per-request reads can drop the header and rely on the fixture.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 6.1 —— 再現: 3 本の `request` は fixture の `extraHTTPHeaders: readAuthHeaders()` を持ち、`registerViaApi`・`erase`・`purge` の呼び出し元は全部その fixture の `request`。求めごとの `headers: readAuthHeaders()` 5 か所を消した。tsc rc=0、`npm run test:e2e -- places st23-purge ch08-nested` 59 passed

## finish の fix（R16〜R28。1 回）

fix の担当（finish）が走らせたもの: `cargo clippy -p ashiato-server --all-targets -D warnings`・`cargo fmt`・`cargo test -p ashiato-server --lib st44_`・`access_log`・`web_session`・`--test server_startup`（39 passed）rc=0、
`tools/smoke.sh` rc=0、`npm run test:e2e -- places st23-purge ch08-nested`（59 passed）rc=0、`npm run typecheck` rc=0、
`scripts/verify-run` 4.2・5.1・6.2・7.1・7.2 PASS、`check_scenarios.py` OK、`check-openapi.sh`・`check-contract-version.sh`・`check-panic-log.sh`・`check-shared-appends.sh` rc=0。
変異: 鍵の下限を 15 / 17 → 試験が落ちる（R18）、smoke.sh の `READ[@]` を `AUTH[@]` に戻す → `check-api-token-reads.sh` rc=1（R16）。全体の `cargo test -p ashiato-server` と全 e2e は走らせていない（CI が持つ）。
