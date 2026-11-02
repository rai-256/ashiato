# ST44 — 収集アプリと画面の資格情報が読める範囲を締める

## Why

いまの API の合言葉（`API_TOKEN`）は、携帯の APK と PC の収集アプリの自動起動の台本が同じ値を持ち、その値でサーバの**全部の口**（記録・滞在・気分の読み出し、書き出し、消去、滞在の作り直し）が通る。
落とした電話や、入れ直す前の古い APK に残った値で全記録を読まれると、読まれた記録は戻らない（PERM-10。完了の判定「収集アプリの資格情報では、送ることだけができて記録は読めない」）。
加えて、`API_TOKEN` は画面のログインの印を無効にする鍵（`secret_tag` の HMAC の鍵）にも使われているので、収集アプリに残る値と DB の写しがそろうと、画面の合言葉を手元で総当たりできる。
衛星の資格情報は ST29 で締めた。残る全読みの合言葉はこれだけなので、最初の版を配る前に締める。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1・Q2 は A（失われるものがある）、Q3 は B。本人の答えが推奨と違えば `spec --answers` が書き直す。

- **いまの API の合言葉を「送るだけ」に締める**（Q1 の推奨。design D1）: `API_TOKEN` で通る口を、送る 5 つ（`POST /ingest`・`/heartbeat`・`/drops`・`/media/missing`、`PUT /media/originals/{sha256}`）と、
  通知と送信に要る読む 4 つ（`GET /contract`・`HEAD /media/originals/{sha256}`・`GET /feelings/reminder`・`GET /outages`）の**許可の一覧**だけにする。
  一覧の外の口は 403 で断り、読み出しの記録に「API の合言葉・断った・403」の 1 行を残す（design D4・D5）。後から足した口は、一覧に足すまで断る（既定は厳しい側）。
  携帯と PC の収集アプリは**入れ直さなくてよい**（呼ぶ口はすべて一覧の中。premises.md）
- **全部を読めるのは画面のログインだけ**（Q3 の推奨。design D3）: 画面のログインで通る範囲は変えない
- **画面のログインの印の鍵をサーバだけが持つ値に替える**（深掘りの AI の既定。design D7・D8（仮））: 新しい環境変数 `WEB_SESSION_KEY`（16 文字以上・API の合言葉とも画面の合言葉とも違う値）を鍵にする。
  無い・短い・ほかの 2 つと同じなら起動しない。**`API_TOKEN` を変えても画面のログインは切れなくなり**、`WEB_SESSION_KEY` か画面の合言葉を変えると全部が切れる。入れ替えた最初の起動で、それまでのログインは 1 回だけ全部切れる
- **資格情報を求めない口は 3 つだけ**（深掘りの AI の既定。design D9）: 生存確認（`GET /healthz`）・ログイン（`POST /session`）・ログアウト（`DELETE /session`）。正典の「すべての API 要求は資格情報を要求する」にこの 3 つを例外として書き、
  API の説明（`docs/openapi.json`）に載る全部の口を資格情報なしで呼んで確かめる試験を置く
- **呼び出し元の種類を見る判定は `_` の無い `match`**（深掘りの AI の既定。design D6）: 契約の版の門（`contract_version.rs` の 2 か所の `matches!`）と許可の一覧。種類が増えたら（ST56 の端末ごとの資格情報など）振り分けを決めないとコンパイルが落ちる
- **道具は画面の合言葉でログインして読む**（Q1 の推奨の費用。design D13）: 縦串（`tools/smoke.sh` と `smoke.d/`）・e2e（`web/e2e`）・録画（`tools/recording`）・確認の手順書（`tools/verify-prep.sh`）・`tools/st25-seed-check.sh` の**読む**求めを、
  ログインの印を見出しで付ける形に替える（読む求めには Bearer を付けない。design D11（仮））。送る求めは API の合言葉のまま
- **配り方は共有の 1 本のまま締める**（Q2 の推奨。design D2）: 端末ごとの発行・取り消しと、どの資格情報で届いたかを記録に書く欄は ST56 が作る（申し送り。書き手の列を作る ST37 にも置き場を申し送る）
- 文書: `docs/network.md` §4（電話を落としたとき）を締めた後の形に、`docs/collector-contract.md` に 403 と許可の一覧を、`.env.example` に `WEB_SESSION_KEY` を書く

**BREAKING**（道具と手順だけ。配った収集アプリは変わらない）: `API_TOKEN` で記録を読んでいた台本・手順（本人が手で打つ `curl` を含む）は 403 になる。画面の合言葉でログインして読む。
サーバの起動に `WEB_SESSION_KEY` が要る（無ければ起動しない）。更新の後に `.env` へ 1 行足す（design の Migration Plan）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `data-sensitivity`: API の合言葉で通る口を許可の一覧に限る（ADDED 1 つ）・画面のログインの印の鍵はサーバだけが持つ（ADDED 1 つ）・「画面から記録を読むには合言葉のログインが要る」を REMOVED にし、「API の合言葉による取り込み口と読み出し口の振る舞いを変えない」と Scenario「API の合言葉での読み書きは変わらない」を除いた同じ中身を「記録を読むには画面の合言葉のログインが要り、API の合言葉では読めない」として ADDED で置き直す（OpenSpec は MODIFIED で Scenario を落とせないため）・
  「画面のログインは期限で切れず、画面の合言葉を変えると全部が無効になる」に API の合言葉を変えても切れないこと・鍵を変えると切れることを足す（MODIFIED）・「誰がいつ読んだか」に API の合言葉で断った求めを足す（MODIFIED）
- `record-envelope`: 「すべての API 要求は資格情報を要求する」に例外の 3 つを書く（MODIFIED）
- `subjective-log`: 「通知の時刻を設定でき、通知の判断に要るものだけを読める」の導出元の「ST29 までは収集アプリの資格情報でも主観の中身は読める」を締めた後の形に直し、収集アプリの資格情報で通知の判断が読め、気分の中身が読めない Scenario を足す（MODIFIED）
- `satellite-platform`: 「衛星の登録と資格情報の操作は、画面のログインでだけできる」の導出元の「API の合言葉で通るいまある口の範囲は変えない（ST44）」を締めた後の形に直す（MODIFIED。振る舞いは変えない）

**capability の照合**: INDEX.md は ST44 を `device-collection`・`desktop-collection`・`data-sensitivity` に割り当てていた。資格情報で通る口の範囲は `data-sensitivity`（アクセス制御）で合っている。
**`device-collection` と `desktop-collection` は変えない** —— Q1 の推奨（いまの合言葉を締める）では、収集アプリが呼ぶ口はすべて許可の一覧の中にあり、収集アプリの振る舞いは何も変わらない（Q1 で案 2 が選ばれれば入れ直しの振る舞いが変わる）。
正典の「すべての API 要求」（`record-envelope`）・ST17 の通知の判断の導出元（`subjective-log`）・ST29 の衛星の操作の導出元（`satellite-platform`）が「ST44 で決める」「変えない（ST44）」と書いていたので、これらに ST44 を足す。
後続の振る舞いの前倒しではない（どれも ST44 の決定を写すだけ）。`docs/stories/INDEX.md` の表と訂正の節に書いた。
途絶の判定の口（`collection-coverage`「途切れの判定は問い合わせの口から読める」）は「収集アプリの資格情報でも読める」「ST44 はこの口を例外に残す」と既に書いていて整合する。Requirement は変えず、収集アプリの資格情報で 200 を見る Scenario は `data-sensitivity` の許可の一覧の側に置く。

**requires は変えない**（ST28 のまま。画面のログインと読み出しの記録の上に積む。ST29 の衛星の資格情報の判定も archive 済み）。

## Impact

- コード（サーバ）: `crates/server/src/access_log.rs`（middleware で API の合言葉の許可の一覧を引き、外なら 403 の行を書いてハンドラを呼ばない）、新しい `crates/server/src/collector_scope.rs`（許可の一覧と判定の関数）、
  `crates/server/src/web_session.rs`（`secret_tag` の鍵を `WEB_SESSION_KEY` に・起動の検査・`session_get` の API の合言葉の 403）、`crates/server/src/lib.rs`（`run` の起動の検査・`App::new` の引数）、
  `crates/server/src/contract_version.rs`（`matches!` を `match` に）、`crates/server/src/bin/openapi.rs` と `docs/openapi.json`（403 の応答）
- 試験: router を通して API の合言葉で記録を読んでいる既存の試験（`access_log_tests.rs`・`web_session_tests.rs`・`satellite_*_tests.rs`・`purge_api_tests.rs`・`version_change_tests.rs` など）は、読む求めをログインの印に替える。ハンドラを直に呼ぶ試験は middleware を通らないので変わらない
- 移行: **無し**（読み出しの記録の CHECK は既に `api_token` と `forbidden` を持つ。`migrations/202610241200_satellite_credentials.sql`）
- 道具と CI: 新しい `tools/web-login.sh`（画面の合言葉でログインして印を出す）、`tools/smoke.sh` と `smoke.d/*.sh`（読む求めの見出し）、`web/e2e/*.spec.ts`（8 ファイル）、`tools/recording`（5 ファイル）、`tools/verify-prep.sh`、`tools/st25-seed-check.sh`、偽データ（`tools/seed.sh`・`tools/seed.d/*.sh`）と `tools/record-env.sh` の一覧の外の求め、新しい `tools/check-api-token-reads.sh`（台本が API の合言葉で一覧の外を呼んでいないかを見る）、
  `WEB_SESSION_KEY` を渡す側（`.env.example`・`tools/env-init.sh`・`tools/record-env.sh`・`tools/ci-db-env.sh`・`tools/smoke.sh`・`tools/stack.sh`・`tools/dev.sh`・`tools/check-panic-log.sh`）、
  新しい `tools/smoke.d/<作成時刻>_st44-collector-scope.sh`
- 文書: `docs/network.md`（§4 電話を落としたとき・§6 の秘密の数）、`docs/collector-contract.md`（403 と許可の一覧）
- 申し送り（`deferred`。どちらも tasks.md がまだ無い）: ST56（端末ごとの資格情報の発行・取り消しと、どの資格情報で届いたかを記録と読み出しの記録に書く欄。種類を増やすときは許可の一覧と契約の版の門の `match` に振り分けを足す）、
  ST37（書き手の列を作るとき、資格情報の識別子の置き場を空けておく）。review/spec.md の処置に書く
- **走っている change との重なり**（差し戻さない。後から main に入る側が rebase で解く）:
  - **ST52**（`st52-release-channels`。上流）: 同じ `crates/server/src/lib.rs` の `router` に `/api` の剥がしと組み込んだ画面を足す。許可の一覧は剥がした後の道（`MatchedPath`）で引くので、画面を配る経路から届く求めにも同じ判定が掛かる。
    ST52 は公開する APK の `BuildConfig` の `API_TOKEN` が空であることを検査する —— 名前は変わらないので ST52 の検査は変えなくてよい。`.env.example`・`docs/network.md` は両方が書き足すので、後から入る側が写す
