# ST44 仮で進めた決定の確かめ

## D8
- 判定: 確か
- 根拠: 起動しない間の取りこぼし（`uncaptured`）は起きない。携帯は `collector-android/.../Retention.kt:16-18`（90 日・2 GB を越えるまで溜める）、PC は `crates/collector-windows/src/outbox.rs` に上限・捨てる処理が無い（`grep -n -i "retention\|purge\|max_bytes\|older"` で 0 件）。鍵は印の照合（`web_session.rs:98` `secret_tag`、`:208` の `secret_tag = $2`）にだけ使い、DB の行は書き直さない（置き場を替えたときに起きるのは全部のログインが 1 回切れることだけで、`rewrite-all` ではない）。費用: 起動の拒否は `check_web_password`（`web_session.rs:84-95`）と同じ形で、渡す側は `.env.example`・`tools/{env-init,record-env,ci-db-env,smoke,stack,dev,check-panic-log}.sh` の 8 つ（全部が実在。`grep -ln WEB_PASSWORD` の結果と同じ並びで、ほかに `WEB_PASSWORD` を渡す経路は `tools/agent-env.sh` が `.env` を読む口だけ。`.env` に行が足されれば一緒に渡る）。`tools/smoke.d/*` の 2 台目のサーバ（st13・st14・st08・st29）は `smoke.sh` の環境を受け継ぐので台本を足さなくてよい。範囲は越えず、要件の本文も変えない。`follows` は無い。土台の D7 は仮でない
- 注: 内部の `App::new(pool, "archive-store", INTERNAL_WEB_PASSWORD, 0)`（`lib.rs:1024`・`:3362`）も鍵の引数を受け取ることになる。値を出す口ではないので失うものは無いが、tasks.md の Task 4 に載っていない

## D11
- 判定: 確か
- 根拠: 何も格納しない（`authorize_quiet` の順は `lib.rs:664-688` の衛星 → `token_matches` → `session_valid` のまま）。両方を付ける呼び手は今は無い: 画面は `Authorization` を付けない（`grep -rn "Authorization\|Bearer" web/src` で 0 件）、収集アプリは cookie を持たない（`grep -rn -i cookie crates/collector-windows/src collector-android/app/src/main/kotlin` で 0 件）、e2e の `Bearer` の求めは cookie が `Secure` で http では付かない（`web/e2e/feel.spec.ts:16` の注記）。`follows`・`rests_on` は無い
- 注: tasks.md 6.1 で読む求めに見出しの `cookie` を付けると、`extraHTTPHeaders` に `Bearer` を持つ文脈（`web/e2e/places.spec.ts:17`・`st23-purge.spec.ts:15`・`ch08-nested.spec.ts:15`）では両方が付き、D11 のとおり 403 になる。失うものではなく、e2e が落ちて気づく実装上の落とし穴（D11 の「読む道具は印だけを付ける」を 6.1 で守る）

## D12
- 判定: 確か
- 根拠: 収集アプリは `GET /session` を呼ばない（`grep -rn '/session' collector-android/app/src/main crates/collector-windows/src` で 0 件。呼び手は画面の `web/src/session.tsx:48` と `tools/smoke.d/202610241500_st29-satellite-credentials.sh:17` の POST だけ）。返すのは呼び出し元の種類だけで、403 にしても格納・破棄するものは無い。`session_get`（`web_session.rs:427-462`）は衛星の 403 の行を自分で書く分岐を既に持ち、`api_token` を足すのは分岐 1 つ。`access_log` の CHECK は `api_token`・`forbidden` を既に持つ（design Context。移行は要らない）。費用は tasks.md の 61 行目（`web_session_endpoint_api_token_reads_and_writes` の断言の書き換え）と一致する。`follows` は無い。土台の D4・D5 は仮でない

## D10
- 判定: 確か
- 根拠: 印が有効なので読める範囲は本人の画面と同じ（`lib.rs:683-688`）。格納も破棄もしないので、401 の側へは分岐 1 つで移れる。ST29 の申し送り（`openspec/changes/archive/2026-10-27-st29-satellite-credentials/design.md:254`）を閉じるもので、`follows`・`rests_on` は無い
- 注: 反転条件の「一致しない `Bearer` が読み出しの記録に繰り返し現れる」は、選んだ側では観測できない。`AccessEntry`（`crates/server/src/access_log.rs:32-46`）には「`Bearer` を付けていたか」を持つ欄が無く、この組は `credential=web_session`・`ok` の行になって普通の画面の読みと見分けがつかない。ただし、古い値を試す側が有効な印を持つことはない（持っていれば最初から全部を読める）。印を持たない試しは、どちらを選んでも `none`・401 の行として残る。失われるのは意味の無い合図だけなので、判定は変えない。反転条件の書き方は「印の無い 401 の行が繰り返し現れる」に寄せるのが正しい

## D16
- 判定: 確か
- 根拠: 止まる場所は台本の頭の環境の検査（`tools/dev.sh:14`・`tools/stack.sh:37`。どちらも `docker compose`・`db-roles.sh`・`migrate` より前）なので、DB も移行の退避も触らずに止まる。起動しない間は D8 と同じで、収集アプリは溜めて待つ（携帯 `Retention.kt` の 90 日・2 GB、PC は上限なし。D8 の節）—— `uncaptured`・`discarded` は起きない。鍵は印の照合（`secret_tag`）にしか使わず DB に何も書かないので `rewrite-all` も `exported` も無い。費用: 鍵の無い `.env` は本人の main の checkout（`~/dev/ashiato2/.env` の `^WEB_SESSION_KEY=` 0 行）と他の worktree（`ashiato2-verify` ほか 8 本が 0 行）。ハーネスの検証の経路は止まらない —— `tools/verify-env.sh:38` が毎回無条件に `tools/env-init.sh` を呼び、`env-init.sh:49-52` が行の無い `.env` に乱数の 1 行を足す。確認バッチの `run.sh` は `tools/verify-prep.sh:35` が先に `env-init.sh` を呼ぶ。CI は `tools/ci-db-env.sh:6` が乱数で渡し、録画は `tools/record-env.sh:110`・`:127` が書く。本人が手で走らせる手順は `docs/briefs/ST44-pr.md:54`（merge 後の 1 番目）と design の Migration Plan 1 にある。範囲は越えず（台本 2 行）、要件の本文も変えない。`follows` は無い。土台の D8 は上の節で確か、D7 は仮でない
- 注: `stack.sh down` も検査の後で分岐する（`tools/stack.sh:41`）ので、鍵の無い `.env` では DB を止める口も止まる。`WEB_PASSWORD` の `:?` で前から同じ形で、失うものは無い（DB が動いたまま残るだけ）
