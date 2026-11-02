# ST44 実装タスク — 収集アプリと画面の資格情報が読める範囲を締める

読む順: `deep.md`（**本人の答え Q1〜Q3**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/data-sensitivity/spec.md` → `specs/record-envelope/spec.md` →
`specs/subjective-log/spec.md` → `specs/satellite-platform/spec.md` → `design.md` → `premises.md` → `docs/collector-contract.md` → `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: API の合言葉で途絶の判定を読める`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario のうち、新しく印を置くのは 23 本**（`data-sensitivity` 20・`record-envelope` 1・`subjective-log` 2）。MODIFIED と置き直した Requirement の残りの Scenario は既存の印（ST17・ST28・ST29 の試験）のまま通す。
  **既存の印「API の合言葉での読み書きは変わらない」（`web_session_tests.rs`）は消す** —— その Scenario は spec から外した（`data-sensitivity` の REMOVED）
- **件数つき検証**: このファイルで
  - **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
    `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` がその項目に書いた本数以上
  - **`CS <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run cs -- cargo test -p ashiato-server --test server_startup <名前>'` rc=0 かつ
    `cargo test -p ashiato-server --test server_startup <名前> -- --list 2>/dev/null | grep -c ': test$'` がその項目に書いた本数以上
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので本数の下限を必ず見る）
- **試験の名前の接頭辞は `st44_`**。サーバの DB の試験は新しい `crates/server/src/collector_scope_tests.rs`（Task 1〜3）に置き、`lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;` を足す。
  起動の試験は `crates/server/tests/server_startup.rs` の `st44_` の関数（Task 4）
- **許可の一覧の試験は必ず `router()` を通す**（`tower::ServiceExt::oneshot`）。ハンドラを直に呼ぶと middleware を通らず、一覧が掛からない（design D4）
- **本人の決定（下流は変えない）**: Q1 いまの API の合言葉（`API_TOKEN`）を送るだけに締める（新しい資格情報を作らない）/ Q2 配り方は共有の 1 本のまま（端末ごとには発行しない。どの資格情報で届いたかの欄も作らない）/ Q3 画面のログインで通る範囲は変えない。
  仮: D8（印の鍵は環境変数 `WEB_SESSION_KEY`・無ければ起動しない）・D11（API の合言葉と印の両方を付けた求めは API の合言葉）・D12（`GET /session` は API の合言葉で 403）。
  （第 1 回は推奨で先に書いた。本人の答えは Q2 だけが推奨と違い、上流が書き直した）
- **名前は統一する**: 許可の一覧と判定の関数は `crates/server/src/collector_scope.rs` の `COLLECTOR_ALLOWED: &[(Method, &str)]` と `collector_allows(method: &Method, matched_path: &str) -> bool`。
  環境変数 `WEB_SESSION_KEY`、起動の拒否 `kind=web_session_key reason=missing|too_short|same_as_api_token|same_as_web_password`。ログインの道具 `tools/web-login.sh`
- **許可の一覧は 9 つ**（design D4）: `POST /ingest`・`POST /heartbeat`・`POST /drops`・`POST /media/missing`・`PUT /media/originals/{sha256}`・`GET /contract`・`HEAD /media/originals/{sha256}`・`GET /feelings/reminder`・`GET /outages`。
  足さない・減らさない（足すのは、その口を収集アプリが使う Story）
- **呼び出し元の種類を見る判定は `_` の無い `match`**（design D6）。新しく書く分岐に `matches!(caller, Caller::…)` と `_ =>` を使わない
- **移行は無い**（読み出しの記録の CHECK は既に `api_token` と `forbidden` を持つ）。`migrations/` に足さない
- **配った収集アプリ（`collector-android`・`crates/collector-windows`）を変えない**（呼ぶ口はすべて一覧の中。design D1）
- 合言葉・印の鍵・ログインの印の値をログ・読み出しの記録・応答・試験の失敗の文に出さない（製造準備 A-2）
- 共有の台本（`tools/smoke.sh`・`tools/check-immutable.sh`）の本体に**新しい節を書き足さない**。ST44 の節は `tools/smoke.d/` の新しいファイルに置く（読む求めの見出しを替える行の書き換えは本体の中でしてよい）
- 試験と台本は一時ファイルを `/tmp` の固定名に置かない（`$TMPDIR` か `mktemp`）

## Task 1: 許可の一覧と、一覧の外の 403（サーバ）

- [x] 1.1 新しい `crates/server/src/collector_scope.rs` に `COLLECTOR_ALLOWED`（9 つ）と `collector_allows` を置く（一覧に**ある**ものだけ真。design D4）。
  `access_log::middleware` で、判定（`decided`）の直後・衛星の 403 と同じ並びに `match caller` を書き、`Some(Caller::ApiToken)` で `collector_allows(method, route)` が偽なら
  `AccessEntry::for_caller(caller, via, route, method, "forbidden", 403)` を書いてハンドラを呼ばずに `403 forbidden` を返す（D5。本文を読まない）。一覧の中の取り込みは今どおり行を書かない（`is_ingest` を変えない）。
  あわせて middleware で `None`（資格情報が無いか一致しない）なら、行を書いた直後にハンドラを呼ばず `401 unauthorized` を返す（引数や本文を読む前に断る。`authorize` の `kind=unauthorized` の warn も同じ場所で出す。design D9）。
  `web_session::session_get` は middleware を通らないので、`Caller::ApiToken` なら同じ行を書いて 403 を返す（D12（仮））。`collector_scope_tests.rs`:
  - `st44_scope_sends_pass` — `// Scenario: API の合言葉で記録・生存信号・破棄の報告・原本の欠けを送れる`
  - `st44_scope_original_put_head` — `// Scenario: API の合言葉で写真の原本を置き、受け取り済みかを確かめられる`（D-02 の置き場は `$TMPDIR` の下）
  - `st44_scope_contract` — `// Scenario: API の合言葉で送信契約の版を確かめられる`
  - `st44_scope_feelings_reminder` — `// Scenario: API の合言葉で気分の通知の判断を読める` と `// Scenario: 収集アプリの資格情報で通知の判断を読める`（ひとことと尺度を持つ記録がある日。本文に文字列も値も無い）
  - `st44_scope_feelings_reminder_post_403` — `// Scenario: 収集アプリの資格情報では通知の時刻を変えられない`（`POST /feelings/reminder` が 403・時刻は変わらない）
  - `st44_scope_outages` — `// Scenario: API の合言葉で途絶の判定を読める`（判定を待つ時間は試験の時計で過ぎた状態にする）
  - `st44_scope_feelings_403` — `// Scenario: API の合言葉では気分の中身は読めない`
  - `st44_scope_events_403_logged` — `// Scenario: API の合言葉では記録を送れるが読めない` と `// Scenario: API の合言葉で断った読み出しは、API の合言葉・403 の行で残る`
  - `st44_scope_session_get_403` — `// Scenario: API の合言葉ではログインの状態も読めない`
  - `st44_scope_both_headers_403` — `// Scenario: API の合言葉とログインの印を両方付けた読み出しは断られる`
  - `st44_scope_cookie_only_reads` — `// Scenario: ログインの印だけを付けた読み出しは通る`（`cookie:` の見出しだけ。画面を配る側を通さない）
  - `st44_scope_web_session_unchanged` — `// Scenario: 画面のログインは許可の一覧の外の口も通る`（記録・滞在・日ごとの気分の読み出しと `/ingest`）
  - `st44_scope_allowlist_is_nine`（`COLLECTOR_ALLOWED` がちょうど 9 つで、上の一覧と一致すること。足し忘れ・足しすぎを止める）
  - `st44_scope_wrong_bearer_with_cookie` — `// Scenario: 一致しない Bearer とログインの印を両方付けた読み出しは印で通る`（読み出しの記録の行の種類が画面のログイン。D10（仮））
  - `st44_scope_allowed_reads_logged` — `// Scenario: API の合言葉で通知に要る読みをすると、読み出しの記録に 1 行残る`（`/contract`・`/feelings/reminder`・`/outages` で `api_token`・200 の行が 1 行ずつ。D5）
  検証: `CT st44_scope_` rc=0（`-- --list` で 15 本以上）
- [x] 1.2 router を通して API の合言葉で記録を読んでいる既存の試験を、読む求めだけログインの印に替える（`grep -ln 'Bearer' crates/server/src/*_tests.rs` のうち `router(` か `send(` を使うもの。少なくとも `web_session_tests.rs`・`access_log_tests.rs`・`purge_api_tests.rs`・`version_change_tests.rs`・`satellite_*_tests.rs`）。
  `web_session_tests.rs` の `web_session_endpoint_api_token_reads_and_writes`（印「API の合言葉での読み書きは変わらない」）は消し、`GET /session` を API の合言葉で `{"credential":"api_token"}` と読む断言を 403 に替える（D12（仮））。
  ハンドラを直に呼ぶ試験は変えない（middleware を通らない）。期待を 200 から 403 に替えた試験は、その理由（ST44 の許可の一覧）を 1 行のコメントで残す。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server'` rc=0 / `bash -c '! grep -rn "Scenario: API の合言葉での読み書きは変わらない" crates web tools'` rc=0

## Task 2: 資格情報なしと API の合言葉だけで全部の口を呼ぶ試験

- [x] 2.1 `collector_scope_tests.rs` に、口の一覧を `ApiDoc::openapi()` の `paths` と方法から作り `/healthz` を足す関数を置く（`satellite_routes_tests.rs` の `contract_operations` と同じ取り方。手で並べない。design D9）。道の `{sha256}`・`{id}`・`{cid}` は形の合う値で埋める。
  - `st44_routes_without_credentials` — `// Scenario: 資格情報を求めない口は生存確認・ログイン・ログアウトの 3 つだけ`（引数も本文も付けない。`/healthz` 200・`DELETE /session` 204・`POST /session` は合言葉の欄の無い JSON で 401・ほかの全部が 401。D9 の middleware の 401 が前提）
  - `st44_routes_api_token_outside_403` — `// Scenario: API の合言葉では許可の一覧の外の口はすべて断られる`（一覧の外の全部が 403、行が口ごとに 1 行増える。呼ぶ前と後で `core.event`・滞在・場所・属性・衛星・書き出しの行の数が同じ）
  - `st44_routes_allowlist_in_contract`（`COLLECTOR_ALLOWED` の 9 つが全部、契約に載る口と方法であること。名前の打ち間違いで一覧が黙って空振りしない）
  `satellite_routes_tests.rs` の `is_exempt` と同じ 3 つになることも確かめる（2 つの試験が別の例外を持たない）。
  検証: `CT st44_routes_` rc=0（`-- --list` で 3 本以上）/ `CT every_route_refuses_satellite` rc=0（`-- --list` で 1 本以上）

## Task 3: 呼び出し元の種類を見る判定を `_` の無い `match` に

- [x] 3.1 `crates/server/src/contract_version.rs` の `matches!(caller, Caller::ApiToken)` 2 か所（見出しの門・知らない欄の検査の `strict`）を、`Caller::ApiToken => 掛ける`・`Caller::WebSession | Caller::Satellite { .. } => 掛けない` の `match` に替える（design D6。振る舞いは変えない）。
  `crates/server/src` で `Caller` の種類を `matches!` で見ている箇所が、試験と `access_log.rs` の衛星の判定のほかに残らないこと。
  検証: `CT st50_` rc=0（`-- --list` で 20 本以上）/ `bash -c '! grep -n "matches!(caller, Caller::ApiToken)" crates/server/src/contract_version.rs'` rc=0

## Task 4: 画面のログインの印の鍵を `WEB_SESSION_KEY` に替える

- [ ] 4.1 `web_session::secret_tag` の鍵を `API_TOKEN` から印の鍵に替え（HMAC の内容は変えない。design D7）、`WebLogin::new` と `App::new` に印の鍵の引数を足す（書庫の中の `App`（`lib.rs` の `INTERNAL_WEB_PASSWORD` の近く）は使われない固定の値でよい）。
  `web_session::check_web_session_key(key, api_token, web_password) -> Result<(), WebSessionKeyRefusal>`（無い・16 文字未満・API の合言葉と同じ・画面の合言葉と同じ）を `check_web_password` と同じ形で置き、`run` で画面の合言葉の検査の直後に呼ぶ
  （拒否なら `kind=web_session_key reason=<種別>` を出して `exit 2`。値は出さない。D8（仮））。試験の `App` を作る補助（各 `*_tests.rs` の `app_with` など）に試験用の鍵を渡す。
  - `crates/server/tests/server_startup.rs`: `st44_session_key_missing_refuses` — `// Scenario: ログインの印の鍵が無いと起動しない` / `st44_session_key_short_or_same_refuses` — `// Scenario: ログインの印の鍵が短いか、ほかの合言葉と同じだと起動しない`（3 つの値。出力に鍵・API の合言葉・画面の合言葉の値が無い）。
    既存の起動の試験（`start_with_web_password` など）は `WEB_SESSION_KEY` を渡す
  - `collector_scope_tests.rs`（DB を使う）: `st44_api_token_change_keeps_login` — `// Scenario: API の合言葉を変えてもログインは切れない`（同じ DB・同じ画面の合言葉と鍵で、`API_TOKEN` だけ違う `App` を作り直して印で読む）/
    `st44_session_key_change_revokes` — `// Scenario: ログインの印の鍵を変えると、それまでのログインはすべて使えなくなる`（2 つの印。鍵だけ違う `App` で両方 401）/
    `st44_secret_tag_not_from_api_token` — `// Scenario: API の合言葉と DB の写しだけでは画面の合言葉を確かめられない`（ログインした後の `core.web_session.secret_tag` と、`API_TOKEN` を鍵にして作った印が一致しない）
  検証: `CS st44_session_key_` rc=0（`-- --list` で 2 本以上）/ `CS server_startup` rc=0（`-- --list` で 9 本以上）/ `CT st44_api_token_change_keeps_login` rc=0（`-- --list` で 1 本以上）/
  `CT st44_session_key_change_revokes` rc=0（`-- --list` で 1 本以上）/ `CT st44_secret_tag_not_from_api_token` rc=0（`-- --list` で 1 本以上）/ `CT web_session` rc=0（`-- --list` で 15 本以上）
- [ ] 4.2 `WEB_SESSION_KEY` を渡す側を直す（design D8（仮））: `.env.example`（雛形 `change-me-…` と説明。16 文字以上・ほかの 2 つと違う値・変えると全部のログインが切れる）、
  `tools/env-init.sh`（新しい `.env` に乱数で作る。**既にある `.env` に `WEB_SESSION_KEY=` の行が無ければ乱数の 1 行を足す**。ほかの行は 1 文字も変えない）、`tools/record-env.sh`（録画の `.env` の 2 か所）、
  `tools/ci-db-env.sh`（乱数の名前の列に足す）、`tools/smoke.sh`・`tools/stack.sh`・`tools/dev.sh`・`tools/check-panic-log.sh`（`API_TOKEN` と同じく、無ければ台本の中の既定の値で export）。
  `tools/env-init.sh` の行を足す振る舞いは、使い捨ての木で確かめる台本 `tools/check-env-init-session-key.sh` に置く（行の無い `.env` に足す・ある行を変えない・2 回走らせても 1 行）。
  検証: `tools/check-env-init-session-key.sh` rc=0 / `bash -c 'grep -q "^WEB_SESSION_KEY=" .env.example && grep -q WEB_SESSION_KEY tools/ci-db-env.sh && grep -q WEB_SESSION_KEY tools/record-env.sh'` rc=0

## Task 5: 縦串は画面の合言葉でログインして読む

- [ ] 5.1 新しい `tools/web-login.sh <base_url>`: `POST <base>/session` に `{"password": "$WEB_PASSWORD"}` を送り、応答の `Set-Cookie` から印を取り出して `cookie: <名前>=<値>` の 1 行を標準出力に出す。失敗なら何も出さず rc 1（合言葉も印もログに出さない。design D13）。
  `--self-test` で、手元の小さな http の応答（`Set-Cookie` を返すもの）から値を取り出せること・204 以外で rc 1 になることを確かめる。
  検証: `tools/web-login.sh --self-test` rc=0
- [ ] 5.2 `tools/smoke.sh` で、サーバが待ち受けた後に `READ=(-H "$(tools/web-login.sh "http://$BIND")")` を 1 回作り、**読む**求め（`/events`・`/stays*`・`/coverage*`・`/day*`・`/feelings`・`/export*`・`/places*`・`/attributes*`・`/sources`・`/archives/status`・`/media/summary` など）の `"${AUTH[@]}"` を `"${READ[@]}"` に替える。
  規則は許可の一覧の側で決める: 一覧の 9 つ（送る求めと `/contract`・`/outages`・`/feelings/reminder`（GET）・`HEAD /media/originals`）だけ `AUTH`、それ以外（読む・書く・作り直す。`POST /stays/*`・`POST /places`・`POST /attributes/kinds` など）は `READ`。
  `tools/smoke.d/*.sh` の既存の節、縦串と `tools/stack.sh` が呼ぶ偽データ（`tools/seed.sh`・`tools/seed.d/*.sh`）、`tools/record-env.sh`（`/archives/status`）も同じ規則で替える（行の書き換えだけ。節を足さない。design D13）。
  新しい `tools/smoke.d/<作成時刻>_st44-collector-scope.sh` に、外から `curl` で: API の合言葉で `/events` が 403・`/outages` と `/feelings/reminder?date=…` が 200・印だけで `/events` が 200・資格情報なしで `/healthz` 200 を見る節を置く（`echo "== ST44: …"`）。
  検証: `scripts/quiet-run smoke -- ./tools/smoke.sh` rc=0 / `tools/check-shared-appends.sh` rc=0 / `bash -n tools/seed.sh` rc=0 / `bash -n tools/record-env.sh` rc=0

## Task 6: e2e と録画の読む求めをログインの印にする

- [ ] 6.1 `web/e2e/*.spec.ts` のうち `Authorization: Bearer` で**読む**求め（`ch08-nested`・`st49-client-version`・`feel-ch01`・`feel`・`places`・`feel-parts`・`settings-satellites`・`st23-purge` から `grep -n Bearer` で拾う）を、
  `global-setup.ts` が残す `storageState`（`e2e/.auth/state.json`）の印を読んで見出し `cookie` で付ける形に替える（共通の補助を `web/e2e/auth.ts` に 1 つ置く。`page.request` に任せない。design D13）。送る求めは `Bearer` のまま。
  `web/e2e/feel.spec.ts:16` の注記（「画面の外の要求には付かない」ので Bearer）を、送る求めだけ Bearer・読む求めは見出しの印、に直す。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && npm run test:e2e'` rc=0 / `bash -c 'cd web && npm run typecheck'` rc=0
- [ ] 6.2 `tools/recording` の録画（`st17/st17-feel.rec.ts`・`ch01/ch01-feel.rec.ts`・`ch08/ch08-nested.rec.ts`・`st23/st23-purge.rec.ts`・`playwright.recording.config.ts`）の読む求めを同じ形に替える（`web/e2e/auth.ts` の補助を使う）。
  送る求めの `BEARER` は残す。注記（「画面の外から `/api` を直に叩く要求は API の合言葉で通す」）を送る求めだけに直す。
  あわせて新しい `tools/check-api-token-reads.sh <ファイルかフォルダ…>` を置く: 渡したファイルの中で API の合言葉を付ける求め（`Bearer`・`BEARER`・`AUTH[@]` と同じ行か同じ呼び出し）の口が、許可の一覧の 9 つの口でなければ、ファイルと行を出して rc 1。
  `--self-test` で、許可の一覧の口（`/ingest`）は通し・外の口（`/events`）は落とすことを確かめる（台本を読み違えて全部を通す形を止める）。
  検証: `tools/check-api-token-reads.sh --self-test` rc=0 / `tools/check-api-token-reads.sh tools/recording web/e2e` rc=0

## Task 7: 確認の手順書・道具・文書を締めた後の形に

- [ ] 7.1 `tools/verify-prep.sh`（確認の手順書の `curl`。`/stays/rebuild` など読む・作り直す求め）と `tools/st25-seed-check.sh`（`/day` の読み）を、`tools/web-login.sh` で印を取って付ける形に替える。
  手順書の文に「読む・作り直すのは画面の合言葉でログインして（`tools/web-login.sh`）。API の合言葉は送るだけ」を足す。
  検証: `tools/check-api-token-reads.sh tools/verify-prep.sh tools/st25-seed-check.sh tools/smoke.sh tools/smoke.d tools/seed.sh tools/seed.d tools/record-env.sh` rc=0 / `bash -n tools/verify-prep.sh` rc=0 / `bash -n tools/st25-seed-check.sh` rc=0
- [ ] 7.2 `docs/network.md` §4（電話を落としたとき）を design D14 の形に書き直す: 落とした電話の値でできること（送る・記録の中身を返さない 4 つの読み）とできないこと、締め出す順、`API_TOKEN` を変えてもログインは切れないこと、
  画面のログインを締め出すのは `WEB_PASSWORD` か `WEB_SESSION_KEY` を変えること。§4 の冒頭の「ST29 まで全読みのまま」の注記を ST44 で締めた、に直す。§6 の秘密の数に `WEB_SESSION_KEY` を足す。
  `docs/collector-contract.md` に「## API の合言葉で通る口（ST44）」の節（9 つの一覧・一覧の外は 403（携帯は `server_403`・PC は `send_unauthorized`）・新しい口は一覧に足すまで断る）を足す。
  `bin/openapi.rs` で許可の一覧の外の口に 403 の応答を足し、`docs/openapi.json` を作り直す（design D15）。
  検証: `./tools/check-openapi.sh` rc=0 / `bash -c 'grep -q "WEB_SESSION_KEY" docs/network.md && grep -q "ST44" docs/collector-contract.md'` rc=0

## Task 8: 全体を通す

- [ ] 8.1 Scenario の担保・書き換え禁止・縦串を通して確かめる。
  検証: `python3 scripts/check_scenarios.py . st44-collector-credential-scope` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server'` rc=0 /
  `./tools/check-immutable.sh` rc=0 / `./tools/check-panic-log.sh` rc=0
