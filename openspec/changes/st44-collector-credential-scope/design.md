# ST44 design — 収集アプリと画面の資格情報が読める範囲を締める

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q3 の推奨は仮でない D として書き（Q1 → D1、Q2 → D2、Q3 → D3）、
> 本人の答えが推奨と違えば `spec --answers` がその D と Scenario だけを書き直す。
> 深掘りの Q1 の context に書いた「どちらの案でも AI が既定で決める」9 つは D に写した: 許可の一覧 → D4、403 と読み出しの記録 → D5、`_` の無い `match` → D6、印の鍵 → D7、
> 資格情報を求めない 3 つの口 → D9、違う Bearer と印の組 → D10、正典の書き直し → specs（proposal の Capabilities）、FR-92 → Non-Goals、落とした電話の手順 → D14。
> 仮の決定 D8・D10・D11・D12 は `provisional.jsonl` に載せた（D10 は深掘りの独立検査 review/deep.md R8、ほかは spec の段で下流が実装中に決めそうな判断を先に出したもの）。

## Context

動機は proposal.md の Why。いまの形（確かめた場所は `premises.md` の「問いの context に書いた repo の中の事実」）:

- 資格情報の判定は `authorize_quiet`（`crates/server/src/lib.rs`）の 1 か所。`Bearer` が衛星の形（`ashsat_`）なら衛星の資格情報を引き、`API_TOKEN` に一致すれば `Caller::ApiToken`、どちらでもなく有効なログインの印があれば `Caller::WebSession`。
  `Bearer` を印より先に見る。判定は最も外側の層（`web_session::refresh_cookie`）が 1 回だけ下し、内側（`access_log::middleware` とハンドラの `authorize`）はそれを読む
- 衛星の資格情報は `access_log::middleware` が全部の口で 403 にし（行を書いてハンドラを呼ばない）、`authorize` も 403 を返す。`/healthz` と `/session` は middleware を通らない（`/session` のハンドラが自分で行を書く）
- `Caller::ApiToken` はいま**全部の口**を通る。`Caller` の種類を見る判定は `Writer::of`（`_` の無い `match`）と、`contract_version.rs` の 2 か所の `matches!(caller, Caller::ApiToken)`（`:100` 契約の版の見出しの門・`:226` 知らない欄の検査）
- 画面のログインの印の行（`core.web_session`）は `secret_tag = HMAC-SHA256(key = API_TOKEN, "ashiato-web-session-v1" || WEB_PASSWORD)` を持ち、起動時の値と一致する行だけが有効（`web_session.rs` の `secret_tag`）
- 読み出しの記録（`core.access_log`）の CHECK は既に `credential` に `api_token`、`outcome` に `forbidden` を持つ（`migrations/202610241200_satellite_credentials.sql`）
- `router` の口は全部 `docs/openapi.json` に載る（`/healthz` を除く）。`/selftest/panic` は環境変数 `ASHIATO_SELFTEST_PANIC=1` のときだけ `router()` の外で足す

## Goals / Non-Goals

**Goals**: API の合言葉（携帯の APK と PC の台本が持つ値）で通る口を、送る口と通知に要る読みの許可の一覧だけにする。全部を読めるのは画面のログインだけにする。
収集アプリに残る値から、画面の合言葉を総当たりする材料を無くす。資格情報を求めない口を 3 つに固定し、試験で確かめる。配った収集アプリは変えない。

**Non-Goals**:
- 端末ごとの資格情報の発行・取り消しと、どの資格情報で届いたかを記録と読み出しの記録に書く欄（作らない。後の Story にも回さない。D2）
- 利用者識別子を送られた内容から取っている今の形（FR-92）。書き手を資格情報から決める列（ST37）
- PC の収集アプリのログで 401 と 403 を別の種別に分けること（いまは `send_unauthorized` に一緒に出す。`sender.rs:254`）。PC の収集アプリは許可の一覧の外を呼ばないので 403 を受けない
- 締める前に読まれた分を取り戻すこと（外に出たものは消せない）
- 衛星の資格情報の範囲（ST29・ST38・ST45）。画面のログインの範囲（D3）

## Decisions

### D1. いまの API の合言葉そのものを「送るだけ」に締める（深掘り Q1 の推奨）

- 新しい資格情報を作らず、`Caller::ApiToken` の通る口を D4 の許可の一覧に限る。携帯の APK・PC の台本に残った値も、サーバを入れ替えた時点で同時に読めなくなる。収集アプリは入れ直さない
- 全部を読む経路は画面のログイン（画面の合言葉）だけになる。道具は画面の合言葉でログインして読む（D13）
- 名前は `API_TOKEN` のまま（ST52 の「公開する APK の `BuildConfig` の `API_TOKEN` が空」の検査を変えない）
- 代替: 収集アプリ用の合言葉を新しく作り、`API_TOKEN` は全部読めるまま残す（全部を読む合言葉が外に 1 つ残る。全部の収集アプリの入れ直しと `API_TOKEN` の値の変更が要る。要件の資格情報の種類に 4 つ目が足る）—— 深掘り Q1 の選択肢

### D2. 配り方は共有の 1 本のまま締める（深掘り Q2 の本人の答え）

- ST44 は共有の 1 本（携帯と PC が同じ `API_TOKEN`）の範囲を締めるだけ。端末ごとの発行・取り消しの画面も、どの資格情報で届いたかの欄も作らない。
  後の Story にも回さない（FR-128 ★ 2026-11-02「端末ごとには発行しない」。ST56 は共有の値を入れる口だけを作る）
- どの端末の資格情報で届いたかは、ずっと持たない（端末は PERM-10 ★ 2026-10-11 のとおり送り手の申告のまま）。落とした電話の値で偽の記録を送られても、後から見分けられない（深掘り Q2 の選んだ側の irreversible）
- 落とした電話を締め出すには、`API_TOKEN` を変えて携帯と PC の収集アプリを全部入れ直す（ST56 の後は値を設定し直す。D14・`docs/network.md` §4）。その間、携帯は送れず溜め続け、90 日・2 GB を越えた分は捨てる
- 要件: PERM-10 の書き手の粒度は変えない（書き手の粒度の★は足さない。review/spec.md R9。PERM-10 ★ 2026-11-02 補足は配り方の宛先を FR-128 へ閉じるだけ）。FR-128 ★ 2026-11-02 が配り方を持つ
- 代替: 端末ごとに発行し ST56 が作る（推奨だった）・ST44 で端末ごとに作る（画面を持つ Story になる）—— 深掘り Q2 の選択肢

### D3. 画面のログインで通る範囲は変えない（深掘り Q3 の推奨）

- 画面のログインの印で通る口は今どおり（送信契約の版の口 `GET /contract` は今どおり 403。ST50 の D15。ほかの口は全部通る）。画面は気分や場所を `/ingest` で送るので、送る口も通す
- 収集アプリだけが使う送る口（生存信号・破棄の報告・原本の欠け・原本を置く）を画面のログインで断るかは、後から変えても何も失われない（Q3 は B）

### D4. 許可の一覧は (方法, 口) の 9 つで、1 つの関数が 2 か所から引かれる

- 一覧（`crates/server/src/collector_scope.rs` の定数。口は `MatchedPath` の型どおり）:
  `POST /ingest`・`POST /heartbeat`・`POST /drops`・`POST /media/missing`・`PUT /media/originals/{sha256}`・`GET /contract`・`HEAD /media/originals/{sha256}`・`GET /feelings/reminder`・`GET /outages`
- 判定の関数 `collector_allows(method, matched_path) -> bool` は一覧に**あるもの**だけを真にする（無いものは全部偽 —— 新しい口は足すまで断る）
- 引くのは 2 か所: `access_log::middleware`（`/healthz` と `/session` 以外の全部の口。`Caller::ApiToken` で偽なら D5 の行を書いてハンドラを呼ばない）と、
  middleware を通らない `session_get`（D12（仮））。衛星の資格情報の 403 と同じ場所・同じ順に置く
- `HEAD` は `MatchedPath` と方法の組で引く（`GET` の口に `HEAD` を足す axum の振る舞いに乗らない。一覧の `HEAD` は原本の口だけ）
- `POST /feelings/reminder`（通知の時刻を変える）・`GET /media/summary` は収集アプリが呼ばない（premises.md）ので一覧に入れない
- ハンドラの `authorize` は変えない。ハンドラを直に呼ぶ試験は middleware を通らないので、許可の一覧の試験は必ず `router()` を通す（Task 2 の Global Constraints）
- 代替: ハンドラごとに `authorize_collector` を呼ぶ（新しい口で呼び忘れると黙って通る。既定が緩い側）・拒否の一覧（新しい口が黙って通る）

### D5. 一覧の外は 403。読み出しの記録に「API の合言葉・forbidden・403」の 1 行を残す。移行は無し

- 応答は `403 forbidden`（衛星と同じ本文）。401 にしない —— 401 は「資格情報が一致しない」で、携帯の収集アプリは 401 を `unauthorized`、403 を `server_403` に分けてログに出す。読み出しの記録でも「合言葉は合っているが範囲の外」が分かる
- 行は `AccessEntry::for_caller(Some(Caller::ApiToken), …, "forbidden", 403)`。`credential = 'api_token'`・`outcome = 'forbidden'` は今の CHECK のまま入る（移行は要らない）
- 許可の一覧の中の取り込み（`/ingest`・`/heartbeat`・`/drops`）は今どおり行を書かない（`is_ingest` を変えない。`/media/*` を「書かない」へ広げない）。一覧の中の読み（`/contract` など）は今どおり 1 回 1 行
- 落とした電話の値で読もうとした跡が `api_token`・`forbidden` の行として残る

### D6. 呼び出し元の種類を見る判定は `_` の無い `match`

- `contract_version.rs:100`（見出しの門）と `:226`（知らない欄の検査の `strict`）の `matches!(caller, Caller::ApiToken)` を、全部の種類を書き並べた `match` に替える（`ApiToken => 掛ける`・`WebSession | Satellite { .. } => 掛けない`）
- D4 の middleware の分岐も `match caller { Some(ApiToken) => …, Some(WebSession) => …, Some(Satellite { .. }) => …, None => … }` と書く
- 種類が増えると（後の Story が資格情報の種類を足したとき）コンパイルが落ち、振り分けを決めるまで先へ進めない。新しい種類で契約の門が黙って外れ、新しい欄が捨てられる（review/deep.md R1）のを防ぐ

### D7. 画面のログインの印の鍵を、API の合言葉からサーバだけが持つ値に替える

- `secret_tag(key, web_password)` の鍵を `API_TOKEN` から `WEB_SESSION_KEY`（D8（仮））に替える。HMAC の内容（`"ashiato-web-session-v1" || WEB_PASSWORD`）は変えない
- 替えた後の最初の起動で、それまでの行の `secret_tag` は今の値と合わなくなり、全部のログインが 1 回切れる（記録は失わない。画面は合言葉の入力欄を出す）
- `API_TOKEN` を変えてもログインは切れなくなる（`docs/network.md` §4 の「`API_TOKEN` を変えた時点でも全端末のログインは切れる」を直す。D14）
- 印の鍵は `WebLogin` の中にだけ持ち、ログ・読み出しの記録・応答・DB に出さない（今の `password` と同じ扱い）

### D8（仮）. 印の鍵は環境変数 `WEB_SESSION_KEY` で渡し、無い・短い・ほかと同じなら起動しない

- 16 文字以上・`API_TOKEN` とも `WEB_PASSWORD` とも違う値。どれかに当たれば `kind=web_session_key reason=<missing|too_short|same_as_api_token|same_as_web_password>` を出して `exit 2`（画面の合言葉の検査 `check_web_password` と同じ形）
- 渡す側: `.env.example`（雛形と説明）・`tools/env-init.sh`（新しい `.env` に乱数で作る。**既にある `.env` に行が無ければ乱数の 1 行を足す** —— 合言葉の行を変えない今の約束のまま、無い行を足すだけ）・
  `tools/record-env.sh`（録画の `.env`）・`tools/ci-db-env.sh`（CI の乱数）・`tools/smoke.sh`・`tools/stack.sh`・`tools/dev.sh`・`tools/check-panic-log.sh`（`API_TOKEN` と同じく、無ければ台本の中の既定の値）
- 本人の機械は、新しいサーバに入れ替える前に `.env` へ 1 行足す（Migration Plan。`docs/network.md` §6 の秘密の数を 5 つに）
- 反転条件: 更新の経路（ST53〜ST58 の自動の更新）が `.env` に行を足せず、入れ替えの後にサーバが起動しない事例が出たら、サーバが最初の起動で乱数の鍵を作り、DB の外の置き場（権限 0600 のファイル）に残す形へ移る（どちらでも DB の写しと収集アプリの値だけでは確かめられない）
- 代替: 鍵を DB に置く（DB の写しだけで確かめられ、意味が無い）・`APP_DB_PASSWORD` を鍵に流用する（DB の合言葉を替えるとログインが切れ、2 つの用途が結び付く）

### D9. 資格情報を求めない口は `GET /healthz`・`POST /session`・`DELETE /session` の 3 つ。資格情報の無い求めは middleware が引数より先に 401 で返す。全部の口を資格情報なしで呼ぶ試験を置く

- PERM-10 の本文が除く 3 つ。`POST /session` は資格情報が画面の合言葉そのもの（合言葉が無ければ 401）。`DELETE /session` は印が無くても 204（無効にする印が無いだけ）
- いまの `access_log::middleware` は、資格情報の無い求めにも 401 の行を書いた後でハンドラを呼ぶ（`access_log.rs:167-187`）。ハンドラの引数の取り出し（`Query<StaysQuery>` の必須の `date`・`Json<PlaceCreateRequest>` の必須の `id` など）が `authorize` より先に走るので、引数や本文の欠けた求めは 400 / 422 になり 401 に届かない（review/spec.md R6）。
  middleware で `None`（資格情報が無いか一致しない）なら、行を書いた直後にハンドラを呼ばず 401 を返す（衛星の 403 と同じ形。応答の本文は `authorize` と同じ `unauthorized`。`authorize` の warn のログも同じ場所で出す）。
  引数や本文を見る前に断るので、資格情報の無い求めに引数の誤りを教えない
- 試験は口の一覧を `docs/openapi.json` の `paths` と方法から作り、`/healthz` を足す（手で並べない —— 次の Story が足した口が黙って抜けない）。各口を資格情報なしで呼び、生存確認 200・ログアウト 204・ほか 401 を見る
- 同じ一覧で、許可の一覧の外の全部の口を API の合言葉だけで呼んで 403 を見る試験も置く（D4。道の `{id}` などは形の合う値で埋める）
- `/selftest/panic` は既定では生えず（`ASHIATO_SELFTEST_PANIC=1` のときだけ）、`docs/openapi.json` に載らないので対象の外（Risks）

### D10（仮）. 衛星の形でない違う Bearer と有効なログインの印の組は、いまのまま印で通す

- `authorize_quiet` の順（衛星の形 → `API_TOKEN` → 印）を変えない。読めるのは本人の画面と同じ範囲で、失うものは無い
- ST29 の design（`ashsat_` で始まらない `Bearer` の扱いは ST44 の範囲・「厳しくするなら ST44 が収集アプリの合言葉と一緒に決める」）の申し送りをこれで閉じる
- 反転条件: 一致しない `Bearer` を付けた求めが読み出しの記録に繰り返し現れる（漏れた古い値を試している跡）など、印があっても違う `Bearer` を 401 にすべき事例が出たら、ST29 の D19 と同じく 401 にする（判定の分岐 1 つで、記録は失わない）

### D11（仮）. API の合言葉とログインの印を両方付けた求めは API の合言葉として扱う（読みは 403）

- `authorize_quiet` が `Bearer` を印より先に見る今の順を変えない。両方を付けた読みは許可の一覧の外として 403
- 読む道具（D13）は印だけを付け、`Bearer` を付けない（読む求めと送る求めで見出しを分ける）
- 反転条件: 両方を付ける正当な呼び手（画面を配る側が API の合言葉を足す形など。data-sensitivity は禁じている）が出たら、印を先に見る順へ替える（判定の順を替えるだけで、記録は失わない）

### D12（仮）. `GET /session`（ログインの状態）は API の合言葉では 403

- 許可の一覧に入れない（収集アプリは呼ばない。premises.md）。`session_get` は middleware を通らないので、衛星の 403 と同じ場所で `collector_allows` を引き、`api_token`・`forbidden`・403 の行を書いて返す
- 返すのは呼び出し元の種類だけで記録の中身ではないが、既定は厳しい側（D4）に揃える
- 反転条件: 収集アプリが合言葉の正否を確かめる用途で `GET /session` を呼ぶ必要が出たら、一覧に足す（足すだけで失うものは無い）

### D13. 道具は画面の合言葉でログインし、印を見出しで付けて読む

- 新しい `tools/web-login.sh <base>`: `POST /session` に `{"password": "$WEB_PASSWORD"}` を送り、応答の `Set-Cookie` から印の値を取り出して標準出力に `cookie: <名前>=<値>` の 1 行を出す（失敗なら rc 1。合言葉も印もログに出さない）。
  印は `Secure` だが、道具は `Set-Cookie` の値を自分で見出しに付けるので、http の `127.0.0.1` でも送れる（premises.md の 3 行目）
- `tools/smoke.sh`: `AUTH`（送る求め。`Bearer $API_TOKEN`）のほかに `READ=(-H "$(tools/web-login.sh …)")` を 1 回作り、読む求め（`/events`・`/stays`・`/coverage`・`/day*`・`/feelings`・`/export*`・`/places*` など）を `READ` に替える。
  `smoke.d/*.sh` も同じ（送る求めと `/contract`・`/outages`・`/feelings/reminder` は `AUTH` のまま）
- `web/e2e`: 読む求めの `Authorization: Bearer` を、`global-setup.ts` が残す `storageState` の印を見出し `cookie` で付ける形に替える（`page.request` は `Secure` の印を http の求めに付けないことがあるので、値を読んで自分で付ける）。送る求めは `Bearer` のまま
- `tools/recording`・`tools/verify-prep.sh`（確認の手順書の `curl`）・`tools/st25-seed-check.sh` も同じ
- 偽データ（`tools/seed.sh`・`tools/seed.d/*.sh`。縦串の `tools/smoke.sh` と `tools/stack.sh` が呼ぶ）と `tools/record-env.sh`（`/archives/status` の読み）も同じ（review/spec.md R7）。
  規則は「許可の一覧の 9 つだけ API の合言葉、それ以外（読む・書く・作り直す。`POST /attributes/kinds`・`POST /places`・`POST /stays/*` など）は印」と一覧の側で決める。`tools/check-api-token-reads.sh` がこの規則で台本を見る
- 本人が手で打つ読みの `curl` は、手順書に `tools/web-login.sh` を使う形で書く

### D14. 落とした電話の手順（`docs/network.md` §4）を締めた後の形に書き直す

- 落とした電話の値で**できること**: 送ること（偽の記録を混ぜる）と、記録の中身を返さない 4 つの読み。**できないこと**: 記録・滞在・気分の読み出し・書き出し・消去
- 締め出す順: 網からノードを外す → `API_TOKEN` を変えて APK と PC の収集アプリを入れ直す（入れ直すまでの間、携帯は溜め続け、90 日・2 GB を越えた分は捨てる。端末は 401 を本人に知らせない）→ 画面のログインが落とした電話のブラウザに残っていれば、`WEB_PASSWORD` か `WEB_SESSION_KEY` を変えて起動し直す
- `API_TOKEN` を変えても画面のログインは切れない（D7）
- `docs/collector-contract.md` に、API の合言葉で通る口の一覧と、一覧の外が 403 になること（携帯は `server_403`）を書く

### D15. API の説明に 403 を書く

- `docs/openapi.json` の、許可の一覧の外の口に `403`（API の合言葉では通らない）の応答を足す（`bin/openapi.rs` の `responses`）。`tools/check-openapi.sh` が通ること

## Risks / Trade-offs

- [道具・手順の読む求めが 403 になる（BREAKING）] → Task 4〜5 で全部を替え、`tools/smoke.sh`・e2e・録画の型検査で落ちないことを確かめる。手で打つ `curl` は `docs/network.md` と確認の手順書を書き直す
- [本人の機械で `WEB_SESSION_KEY` を足し忘れると、新しいサーバが起動しない] → 起動の拒否が理由を出す（D8（仮））。収集アプリは溜めて待つ（携帯 90 日・2 GB。PC は上限なし）ので、数日のうちに足せば記録は失わない。Migration Plan の 1 番目に置く
- [入れ替えた最初の起動で、全部のブラウザのログインが 1 回切れる] → 合言葉を入れ直せば戻る。記録は失わない（D7）
- [ハンドラを直に呼ぶ試験は middleware を通らず、許可の一覧が掛からない] → 許可の一覧の試験は `router()` を通す（D4。Task 2）。本番の経路は `router()` だけ
- [`/selftest/panic` は資格情報なしで呼べる] → 既定では生えない（環境変数で明示的に開けたときだけ）。`docs/openapi.json` に載らず D9 の試験の対象の外。今の振る舞いを変えない
- [共有の 1 本の間は、落とした電話の値で偽の記録を送られても後から見分けられない] → 深掘り Q2 で本人が受け入れた（D2）。締めた後にその値でできるのは「送ること」と通知に要る 4 つの読みだけで、記録の中身は読めない。締め出すには値を変えて全部を設定し直す
- [収集アプリが許可の一覧の外を呼んでいたら、締めた時点で黙って止まる] → premises.md で携帯・PC のソースを辿って 9 つに収まることを確かめた。送る口・読む 4 つの口を API の合言葉で呼んで 200 を見る試験を置く（Task 2）。
  携帯の計測テスト・PC の実行時テストは router を変えないので今どおり通る
- [ST52 が同じ `router` に `/api` の剥がしを足す] → 許可の一覧は剥がした後の `MatchedPath` で引く。後から main に入る側が rebase で合わせる

## Migration Plan

1. 本人の機械の `.env` に `WEB_SESSION_KEY=$(openssl rand -hex 24)` を 1 行足す（新しいサーバに入れ替える**前**。足さないと起動しない）
2. サーバを入れ替えて起動する（移行は無い）。全部のブラウザのログインが 1 回切れるので、画面で合言葉を入れ直す
3. 収集アプリは入れ直さない（呼ぶ口はすべて許可の一覧の中）
4. 手で打っていた読みの `curl` は `tools/web-login.sh` で印を取ってから打つ

戻し: 前のサーバに戻すと、`API_TOKEN` は再び全部の口を通り、ログインの印の鍵も `API_TOKEN` に戻る（ログインがもう 1 回切れる）。`.env` の `WEB_SESSION_KEY` の行は前のサーバには無視される。DB は変えていないので戻しに手順は要らない。
