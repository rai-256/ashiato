# ST28 実装タスク — 外からは許可した私設網の内側でしか届かない

読む順: `deep.md`（**最優先。本人が決めた 6 件と、聞かずに決めた既定 C1〜C7**）→ このファイル →
`specs/data-sensitivity/spec.md` → `design.md` → `review/spec.md`（処置の理由）→ `docs/stories/ST28.md` →
`docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D9）。**名前は作成時刻 `YYYYMMDDHHMM_access_control.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。**付与は配列に入れない**（design D4。移行の後に毎回当てる段）。
**`record-envelope` の要件、取り込みの口の応答の形、門のトリガには触らない。** 並走中の st06 / st08 / st12 / st22 の change のファイルも触らない。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `.env`（Task 1 の雛形から作る）と `docker compose up -d --wait db` と `tools/db-roles.sh`（Task 1 で作る）が前提。
`cargo test` は `.env` を読まないので、`set -a; . ./.env; set +a` の後で走らせる（`DATABASE_OWNER_URL` / `DATABASE_URL` が要る。design D19）。

## Global Constraints（規律。**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript / Kotlin はコメント（`// Scenario: 全インタフェースでは起動しない`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。
  印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **1 つの Scenario の印は 1 つの試験にだけ置く**（review R10）。単体と結合の 2 段で撃つもの（`bind_allowed` と起動の結合など）は、印を THEN を確かめる側（結合）にだけ置く
- **この change の Scenario は 59 本**（`data-sensitivity` の ADDED のみ。既存の capability の Scenario は変えない）
- **「人間の確認待ち」に逃がせる Scenario は 1 本も無い。** 網の外の到達は検査（Task 8）が状態で判定し、画面は e2e（Task 6）が数値と経路で見る。
  本人の機械での移行（design D20）は Scenario ではなく手順で、tasks の外（Task 9 の手順書と PR 本文）
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで
  **`CT <絞り込み>`** と書いたものは、次のコマンドが rc=0 になることを指す:
  `bash -o pipefail -c 'set -a; . ./.env; set +a; cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`
  **`ET <絞り込み>`** は `bash -o pipefail -c 'set -a; . ./.env; set +a; cd web && npx playwright test <絞り込み> 2>&1 | tee /tmp/et.log' && grep -Eq '[1-9][0-9]* passed' /tmp/et.log` が rc=0
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **試験の名前の接頭辞はこの change だけのもの**（review R15。既存の試験に部分一致させない）: `server_startup_` / `web_session_` / `access_log_` / `app_role_` / `response_no_store_` / `base_url_cleartext_`。
  作ったら `cargo test -p ashiato-server <接頭辞> -- --list` にこの change の試験しか出ないことを確かめる
- **本人の決定（下流は変えない）**: 画面は合言葉でログイン（Q1）/ 網の外へは `tailscale serve` の HTTPS、サーバと画面は 127.0.0.1（Q2）/
  NFR-15 は「本システムが記録の写しを拠点外に置くこと」（Q3）/ **`API_TOKEN` の読める範囲は変えない（Q4。ST29 まで）** /
  DB の役割分離と DB の合言葉の秘密化をこの Story で（Q5）/ **ログインは期限なし**（Q6）
- **D7 / D16 / D17 / D18 / D20 / D21 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号を書き直す
- **合言葉・ログインの印・DB の合言葉・網の名前・アドレスをログにも出力にもリポジトリにも出さない**（製造準備 A-2 / C3）。
  出すのは `kind`・件数・route の型・理由の種別だけ。試験の合言葉は試験の中で乱数から作る
- **試験は所有者の接続（`testdb::pool()`）で書き、役割の試験だけアプリの接続（`testdb::app_pool()`）で書く**（design D19）。
  既存の門の試験を書き換えない（トリガが所有者も止めることを確かめている）

## Task 1: DB の役割と、DB の合言葉を秘密へ（design D4 / D19）

- [x] 1.1 `tools/db-roles.sh` を足す。`.env`（`ENV_FILE` で差し替えられる）を読み、`docker compose exec -T db psql -U "$POSTGRES_USER"` で
  (a) 管理者の合言葉を `POSTGRES_PASSWORD` に揃える（`ALTER ROLE`）、(b) `ashiato_owner`（`LOGIN CREATEDB NOSUPERUSER NOCREATEROLE`）と
  `ashiato_app`（`LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE`）を作るか合言葉を揃える、(c) `ashiato` DB と `core` の schema・全表・sequence・関数・型の所有を
  `ashiato_owner` へ移す（`REASSIGN OWNED BY` は使わない。design Risks）。**2 回走らせても同じ結果**。合言葉は標準出力に出さない。
  3 つの合言葉のどれかが空か `change-me` で始まれば、**DB に触れずに** exit 1 して項目の名前だけを出す。
  検証: `bash -c 'tools/db-roles.sh && tools/db-roles.sh'`、`bash -c 'test "$(docker compose exec -T db psql -qtA -U ashiato -d ashiato -c "SELECT count(*) FROM pg_tables WHERE schemaname = '"'"'core'"'"' AND tableowner <> '"'"'ashiato_owner'"'"'")" = 0'`
- [x] 1.2 `docker-compose.yml` の `POSTGRES_PASSWORD` を `${POSTGRES_PASSWORD:?.env に POSTGRES_PASSWORD を置く}` にし、固定値を消す。
  `.env.example` に `POSTGRES_PASSWORD` / `OWNER_DB_PASSWORD` / `APP_DB_PASSWORD` / `DATABASE_OWNER_URL` / `WEB_PASSWORD` の雛形を足し、
  `DATABASE_URL` をアプリの役割に直す（値は `change-me-…` の雛形だけ）。`testdb.rs` の既定の URL（固定の合言葉入り）を消す（1.3）。
  `tools/check-db-secret.sh` を足す: (a) `git ls-files` の追跡ファイルのうち DB を立てる設定・試験・台本（`docker-compose.yml` / `crates/**/testdb.rs` / `tools/*.sh` / `.github/workflows/*.yml`）に
  `POSTGRES_PASSWORD: <字面の値>` と `postgres://<役割>:<字面の値>@` が無い（`${` で始まるもの・`.env.example` の `change-me-` は通す）、
  (b) 雛形の値だけを持つ fixture（`tools/fixtures/db-roles/placeholder.env`）で `ENV_FILE=… tools/db-roles.sh` が exit 1 し、DB の役割の一覧が変わらない。
  Scenario: `配布物に DB の合言葉の固定値が無い`（(a) の `echo`）/ `雛形の合言葉のままでは役割を作らない`（(b) の `echo`）。
  **この時点では (a) は `.github/workflows/ci.yml` の字面の値に当たって落ちる**ので、1.2 の検証は `docker-compose.yml` と `testdb.rs` だけを見る形（`--only compose,testdb`）で走らせ、全体は 1.4 で走らせる（review R16）。
  検証: `tools/check-db-secret.sh --only compose,testdb`
- [x] 1.3 `testdb.rs`: `pool()` は `DATABASE_OWNER_URL` で接続して移行と付与を当てる。無ければ「`.env` を読み込むか `DATABASE_OWNER_URL` を渡す」と出して落ちる（飛ばさない）。
  `app_pool()` を足す（`DATABASE_URL`）。役割が無いときは「`tools/db-roles.sh` を先に」と出して落ちる。
  `stay_tests.rs` / `attributes_tests.rs` の `fresh_db` の URL を `DATABASE_OWNER_URL` から組む。**既存の試験の本体は書き換えない。**
  検証: `bash -c 'set -a; . ./.env; set +a; cargo test -p ashiato-server'`、`bash -c 'git diff --exit-code origin/main -- crates/server/src/dedup_tests.rs crates/server/src/registry_tests.rs crates/server/src/drops_tests.rs'`
- [x] 1.4 `.github/workflows/ci.yml` の DB を使う job（rust / smoke / e2e のすべて）で、合言葉を job の中で乱数から作り（`openssl rand -hex 24` を `$GITHUB_ENV` へ）、
  `services` の `POSTGRES_PASSWORD` にも `${{ env.… }}` で渡し、役割を作ってから試験する。`DATABASE_URL`（アプリ）と `DATABASE_OWNER_URL` と `WEB_PASSWORD` を渡す。
  **ワークフローに字面の合言葉を書かない**（design D19）。`tools/check-db-secret.sh`（全体）を CI の検査の job に足す。
  検証: `tools/check-db-secret.sh`

## Task 2: 移行の分離と、アプリの接続（design D4 / D5）

- [x] 2.1 `crates/server/src/grants.sql` を足し、`migrate()` の最後（`MIGRATIONS` を当てた後）で毎回当てる（design D4 の付与と既定の権限。TRUNCATE / REFERENCES / TRIGGER を付けない）。
  Scenario: `移行が足したどの表にもアプリの役割が届き、切り詰めの権限は無い`（`core` の全表を `pg_tables` から引いて `has_table_privilege` と `tableowner` で見る。表の名前を試験に書かない —— 並走中の change が足す表も自動で入る）。
  検証: `CT app_role_privileges`
- [x] 2.2 `ashiato-server migrate` を足す（`DATABASE_OWNER_URL` で `migrate()`）。`run()` からは `migrate()` を外す。
  **DB を作り直す台本を含めて**（`tools/stack.sh`（`STACK_RESET=1`）/ `tools/dev.sh` / `tools/smoke.sh`（`down -v`）/ CI）、`tools/db-roles.sh` → `ashiato-server migrate` → サーバの順にする（review R17）。
  サーバは `env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL` で起動する（design D5）。
  `tools/smoke.sh` に、動いているサーバの `/proc/<pid>/environ` にその 3 つの名前と所有者・管理者の合言葉の値が無いことを見る段を足す。
  Scenario: `サーバの実行時の環境に所有者と管理者の合言葉が無い`（`tools/smoke.sh` の `echo`）。
  検証: `tools/smoke.sh`、`bash -c 'STACK_RESET=1 bash tools/stack.sh up --check-only'`（2.2 で `--check-only` を足す: 作り直し → 役割 → 移行 → サーバの `/healthz` まで通して止める）
- [x] 2.3 `run()` の DB 接続の直後に自己検査（`rolsuper` / `core` の表の所有者）。拒否は `kind = "db_role"` と理由の種別を出して終了コード 2。
  結合の試験は `crates/server/tests/server_startup.rs`（`env!("CARGO_BIN_EXE_ashiato-server")` を管理者・所有者の URL で起動し、終了コードと標準エラーの種別を見る）。
  Scenario: `管理者の接続ではサーバが起動しない` / `所有者の接続ではサーバが起動しない`。
  検証: `CT server_startup_refuses_privileged_role`
- [x] 2.4 アプリの接続で門を外せないことを撃つ（R103 の再現を逆向きに）。`testdb::app_pool()` で
  `SET session_replication_role = replica` が拒まれる / `ALTER TABLE core.event DISABLE TRIGGER ALL` が拒まれてトリガが有効のまま /
  `core` の**すべての表**（`pg_tables` から引く）で `TRUNCATE` が拒まれて行数が変わらない。
  Scenario: `アプリの接続からは門を外せない` / `アプリの接続からは表の定義を変えられない` / `アプリの接続からはどの表も切り詰められない`。
  検証: `CT app_role_cannot_bypass_gate`
- [x] 2.5 アプリの接続でハンドラを通す結合（`App::for_test(testdb::app_pool().await, …)` で `/ingest` → `/events`）。
  Scenario: `アプリの接続で取り込みと読み出しが通る`。
  検証: `CT app_role_ingest_and_read`
- [x] 2.6 `tools/check-immutable.sh` に、`psql -U ashiato_app`（コンテナ内の socket）から `session_replication_role` / `DISABLE TRIGGER` / `TRUNCATE` が**拒まれたときだけ**
  `  OK app-role <操作> を拒んだ` を出す 3 行を足す（通ってしまったら `NG` で exit 1）。
  検証: `bash -o pipefail -c 'tools/check-immutable.sh | tee /tmp/ci.log' && test "$(grep -c "OK app-role" /tmp/ci.log)" -eq 3`

## Task 3: 待ち受けと DB の接続先の検査（design D7 / D14）

- [x] 3.1 `bind_allowed(addr) -> Result<(), BindRefusal>` を純関数で置き、`run()` の DB 接続の前で `BIND` を解決したすべてのアドレスに掛ける。
  拒否は `bind_unspecified` / `bind_not_loopback` をログに出して終了コード 2。単体: `127.0.0.1` / `127.0.0.2` / `::1` / `0.0.0.0` / `::` / `192.0.2.1` / `localhost`（印は置かない。R10）。
  検証: `CT server_startup_bind_allowed`
- [x] 3.2 起動の結合（`crates/server/tests/server_startup.rs`）。サーバの実体を環境を変えて起動し、終了コードとログの `kind` を見る:
  `BIND` なし → 待ち受け、`ss -ltnpH` のそのプロセスの口がすべて loopback / `BIND=192.0.2.1:0`（TEST-NET）→ 2・`bind_not_loopback` / `BIND=0.0.0.0:0` → 2・`bind_unspecified`。
  Scenario: `既定では loopback でだけ待ち受ける` / `loopback 以外のアドレスでは起動しない` / `全インタフェースでは起動しない`。
  検証: `CT server_startup_bind`
- [x] 3.3 DB の接続先の検査（design D14）。サーバと `migrate` は `DATABASE_URL` / `DATABASE_OWNER_URL` の host が loopback か unix socket でなければ、接続せずに `kind = "db_not_loopback"` で終了コード 2。
  結合: `DATABASE_URL=postgres://x:y@192.0.2.1:5432/ashiato` のサーバと、同じ形の `DATABASE_OWNER_URL` の `migrate` がどちらも 2 で終わり、接続の試みがログに無い。
  Scenario: `DB の接続先が loopback でなければ起動しない`。
  検証: `CT server_startup_db_not_loopback`
- [x] 3.4 `tools/stack.sh` の案内「端末から届くには BIND を LAN / Tailscale の IP にする」を `docs/network.md`（Task 9）への案内に書き換える。
  検証: `bash -c '! grep -n "LAN / Tailscale の IP" tools/stack.sh tools/verify-prep.sh'`

## Task 4: 移行とログインの口（design D1 / D2 / D3 / D9 / D16 / D17 / D18）

- [x] 4.1 移行 `migrations/YYYYMMDDHHMM_access_control.sql` と `.down.sql` —— `core.web_session`（design D3）と `core.access_log`（design D8。UPDATE / DELETE を拒む行トリガと
  TRUNCATE を拒む文トリガ。**この移行専用の関数**）。当て直せる形。`MIGRATIONS` 配列の末尾に足す。
  検証: `tools/check-migrations.sh`、`CT access_log_migration_applies_twice`
- [x] 4.2 起動時の画面の合言葉の検査（`WEB_PASSWORD` が無い / 16 文字未満 / `API_TOKEN` と同じ → 拒否。`kind = "web_password"`。D16（仮））。
  Scenario: `画面の合言葉が API の合言葉と同じだと起動しない` / `画面の合言葉が無いか短いと起動しない`（15 文字で撃つ）。
  検証: `CT server_startup_web_password`
- [x] 4.3 `POST /session` / `DELETE /session` / `GET /session` と、`authorize()` がログインの印も受けて呼び出し元の種類を返す形（design D1 / D2 / D3）。
  比較は `token_matches`。`secret_tag` の鍵は `API_TOKEN`（design D3）。失敗は 1 秒待たせ、1 分 10 回を超えたら 429（D17（仮）。試験は待ちを差し替えられる形にする）。
  Scenario: `違う合言葉のログインの求めは断られ、印は発行されない` / `合言葉を付けないログインの求めは断られる` / `API の合言葉では画面にログインできない` /
  `ログインの印は暗号化された接続でだけ送られる` / `ログインの印は別のサイトから始まった求めには付かない` / `ログアウトした印を持ち出しても使えない` /
  `失敗を重ねたログインは一時的に断られる` / `API の合言葉での読み書きは変わらない`。
  検証: `CT web_session_endpoint`
- [x] 4.4 期限・寿命・合言葉の変更（試験は `App::at()` で時刻を差し込む。合言葉の変更は `App` を別の合言葉で組み直す）。
  Scenario: `既定では日が経ってもログインは切れない` / `ログインの印はブラウザを閉じても残り、使うたびに延びる`（ログインと読み出しの 2 つの応答の `Set-Cookie` の `Max-Age` が 86400 以上）/
  `画面の合言葉を変えると、それまでのログインはすべて使えなくなる` / `期限を設定したときは、過ぎた印は使えない`。
  検証: `CT web_session_lifetime`
- [x] 4.5 OpenAPI に 3 本を載せ、`docs/openapi.json` を再生成する。
  検証: `tools/check-openapi.sh`

## Task 5: 読み出しの記録と応答ヘッダ（design D8 / D10 / C4 / C5）

- [ ] 5.1 axum の middleware で `core.access_log` に 1 求め 1 行（design D8 の書く求め・列・時機）。書く口は `AccessSink` の trait で、書けなければハンドラを呼ばずに 500。
  Scenario: `記録の読み出しで読み出しの記録に 1 行増える` / `断られた求めも読み出しの記録に残る` / `取り込みは読み出しの記録に残らない` /
  `読み出しの記録には中身も合言葉も残らない`（全列を文字列にして、試験が送った引数の値・合言葉・印・利用者の識別子の部分文字列が 1 つも無い）/
  `読み出しの記録に書けないときは記録を返さない`（常に `Err` を返す偽の `AccessSink`）/ `読み出しの記録は記録の読み出しに出ない`。
  検証: `CT access_log_middleware`
- [ ] 5.2 読み出しの記録が追記のみであること（所有者の接続で UPDATE / DELETE / TRUNCATE がトリガで拒まれる）。`tools/check-immutable.sh` にも `OK access_log …` の 3 行を足す。
  Scenario: `読み出しの記録の行は書き換えも削除もできない`。
  検証: `CT access_log_is_append_only`、`bash -o pipefail -c 'tools/check-immutable.sh | tee /tmp/ci.log' && test "$(grep -c "OK access_log" /tmp/ci.log)" -eq 3`
- [ ] 5.3 サーバの全応答に `Cache-Control: no-store`（`tower-http`）。
  Scenario: `サーバの応答は写しを保存させない`（読み出し・取り込み・ログイン・ログアウト・401 の 5 つ）。
  検証: `CT response_no_store_server`

## Task 6: 画面（design D10）

- [ ] 6.1 `web/vite.config.ts` の proxy から `authorization` を外す。`preview.headers` に CSP（`default-src 'self'; frame-ancestors 'none'`）と `Cache-Control: no-store`。
  **最初に** `http://127.0.0.1` で `Secure` の cookie が Chromium に残るかを 1 本の e2e（`web/e2e/login-cookie.spec.ts`）で確かめ、残らなければ `playwright.config.ts` の `baseURL` を `http://localhost` にする（design Risks）。
  検証: `bash -c '! grep -n "authorization" web/vite.config.ts'`、`ET login-cookie`
- [ ] 6.2 画面のログイン: 起動時に `GET /api/session`、401 なら合言葉の入力欄（`ui-direction.md` の下限）。どの面でも読み出しが 401 なら入力欄へ。画面の頭に「ログアウト」。
  単体は `web/src/__tests__/session.test.ts`。
  検証: `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/session.test.ts 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log`、`bash -c 'cd web && npm run lint && npm run build'`
- [ ] 6.3 e2e の足場: `globalSetup` で 1 回ログインし、`storageState` を既定にする（既存の `coverage-year` / `day-stays` / `stack` は**書き換えずに**通る）。
  ログインの e2e（`web/e2e/login.spec.ts`）は未ログインの `storageState` から始める。
  Scenario: `ログインしていないブラウザには記録が 1 件も返らない`（画面が出した `/api/` の読み出しの応答がすべて 401・記録の行の要素が 0 個）/
  `ログインしていないブラウザには合言葉の入力欄が出る` / `合言葉でログインすると画面が記録を読める` / `違う合言葉を入れても画面は記録を出さない` /
  `ログアウトすると記録が読めなくなる` / `画面を配る側は合言葉を付け足さない`（cookie を持たない `request` で preview の `/api/events` を叩いて 401）/
  `ログインの印はスクリプトから読めない`（画面の中で `document.cookie` に `ashiato_session` が無い）。
  検証: `ET login.spec`、`ET coverage-year`、`ET day-stays`、`ET stack`
- [ ] 6.4 CSP・外部への要求・端末の写し（`web/e2e/offsite.spec.ts`）。`page.on('request')` で稼働状況・1 日を見る・マスタ管理を開いた間の要求を集め、preview の出所以外が 0 件。
  `securitypolicyviolation` の報告が 0 件。応答の `content-security-policy` と `cache-control` を見る。`navigator.serviceWorker.getRegistrations()` が 0 件、
  `localStorage` / `sessionStorage` / `indexedDB.databases()` に試験が入れた偽データの値が無い。
  Scenario: `画面を開いても外部への要求は 0 件である` / `画面は外部の資源の読み込みを禁じる指示を持つ` / `画面を開いても読み込みの指示に反した報告は出ない` /
  `画面の応答は写しを保存させない` / `画面は端末に記録の写しを置かない`。
  検証: `ET offsite`

## Task 7: 収集側は loopback 以外へ平文で送らない（design D13）

- [ ] 7.1 Android: `GenerateNetworkSecurityConfig` を、`base-config` の平文禁止と `localhost` / `127.0.0.1` だけの `domain-config` を常に出す形にする（`ashiato.baseUrl` から例外を作らない）。
  `ashiato.baseUrl` が `http://` で host が loopback でなければ build を落とす task を足す。
  `tools/check-apk-cleartext.sh` を足す: (a) `-Pashiato.baseUrl=https://example.invalid:1` で `:app:generateNetworkSecurityConfig` を走らせ、生成物に `example.invalid` の平文の許可も、loopback 以外の `domain-config` の平文の許可も無い、
  (b) `-Pashiato.baseUrl=http://example.invalid:1` の `assembleDebug` が失敗し、出力に理由がある。**(a) はいまのコードでは落ちる**（review R14）。
  Scenario: `収集アプリは接続先の宛先にも平文を許さない`（(a) の `echo`）/ `平文の接続先では収集アプリを組み立てられない`（(b) の `echo`）。
  検証: `tools/check-apk-cleartext.sh`
- [ ] 7.2 計測テスト `CleartextPolicyInstrumentedTest`（`NetworkSecurityPolicy.getInstance().isCleartextTrafficPermitted("127.0.0.1")` が真）。
  Scenario: `収集アプリは loopback への平文の接続を許す`。
  検証: `tools/android-emulator.sh`
- [ ] 7.3 C-02: `Config::from_env` が `ASHIATO_BASE_URL` の平文・非 loopback を `Err` にする（`127.0.0.1` / `::1` / `localhost` は通す）。試験の名前は `base_url_cleartext_` で始める。
  Scenario: `平文の接続先では PC の収集器が起動しない`。
  検証: `bash -o pipefail -c 'cargo test -p ashiato-collector-windows base_url_cleartext_ 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`

## Task 8: 網の外に開いていない・拠点外へ書き出さない、の検査（design D11 / D14 / D15）

- [ ] 8.1 `tools/check-exposure.sh`（design D11）。入力を `EXPOSURE_SS_OUTPUT` / `EXPOSURE_SERVE_JSON` で差し替えられる。fixture は `tools/fixtures/exposure/`（**網の名前とアドレスは雛形の値**。`tools/check-private.sh` が通ること）。
  `--self-test` が fixture で 6 つを撃つ（期待どおりに落ちる / 通るを見る）。`tools/stack.sh` が起動の後に (a) を走らせる。`tools/dev.sh` は vite dev に `--port "${DEV_WEB_PORT:-5173}"` を渡す。
  Scenario: `網の外への公開が有効だと検査が落ちる` / `loopback 以外で待ち受ける口があると検査が落ちる` / `網へ平文で出している口があると検査が落ちる` /
  `開発用の画面を網へ出していると検査が落ちる` / `網の手段の設定を読めないと検査が落ちる` / `loopback と暗号化された網の口だけなら検査は通る`。
  検証: `tools/check-exposure.sh --self-test`、`tools/check-private.sh`
- [ ] 8.2 `tools/check-offsite.sh`（design D14）。一覧は台本の中に持ち、空なら落ちる。`--self-test` は一覧の名前を 1 つ含む偽の `cargo tree` の出力で落ちること。CI の検査の job に足す。
  Scenario: `サーバは拠点外へ書き出す部品を持たない`。
  検証: `tools/check-offsite.sh`、`tools/check-offsite.sh --self-test`
- [ ] 8.3 `tools/check-private.sh --staged` が、`tailscale` があれば機械の短い名前と網の名前をその場で読んで禁止語に足す（design D15）。値は出力に出さない。
  `--self-test` は `CHECK_PRIVATE_EXTRA=<偽の名前>` で禁止語を差し込み、その語を含むファイルで落ち、出力にその語が出ないこと。
  検証: `tools/check-private.sh --self-test`

## Task 9: 手順書と確認バッチ（design D12 / D20）

- [ ] 9.1 `docs/network.md` を足す —— `tailscale serve` の HTTPS の口を画面と API の 2 本・http と開発用の画面の口を消す・**証明書ログに機械の名前と網の名前が載り消せない**（Q2）・
  収集アプリの `https://` の入れ直し・**電話を落としたとき**（Q4 / Q6。`API_TOKEN` と画面の合言葉を変える）・**画面を出す機械名に信頼できないサービスを同居させない**（design D12）・
  **本人の機械での移行の順序**（design D20 の 1〜5。退避を先に）・`tools/check-exposure.sh` で確かめる。網の名前は雛形（`<machine>.<tailnet>.ts.net`）だけ。
  検証: `tools/check-private.sh`、`bash -c 'for w in 証明書 落とした check-exposure pg_dump db-roles 同居; do grep -q "$w" docs/network.md || { echo "無い: $w"; exit 1; }; done'`
- [ ] 9.2 `docs/screens.md` の網越しの URL を `https://` に直し、ログインの面を 1 行足す。`tools/verify-prep.sh` の手順書（`run.sh` の説明）に
  ログイン（`.env` の `WEB_PASSWORD`）・`https://` を反映する。`curl` の例は `API_TOKEN` のまま（Q4）。
  `ashiato.baseUrl` が平文で loopback でなければ APK を作らずに「`docs/network.md` の移行の 5 を済ませる」と出して続ける（design D20）。
  検証: `bash -c '! grep -n "http://<手元の網のホスト名>" docs/screens.md tools/verify-prep.sh'`、`bash -c 'grep -q WEB_PASSWORD tools/verify-prep.sh && grep -q "network.md" tools/verify-prep.sh'`

## Task 10: 仕上げ

- [ ] 10.1 `python3 scripts/check_scenarios.py .` で、この change の 59 本すべてに印があることを確かめる。
  検証: `python3 scripts/check_scenarios.py .`
- [ ] 10.2 全部の検査を通す。
  検証: `bash -c 'set -a; . ./.env; set +a; cargo test --workspace'`、`cargo clippy --all-targets -- -D warnings`、`tools/smoke.sh`、`tools/check-immutable.sh`、`tools/check-licenses.sh`、`tools/check-db-secret.sh`、`bash -c 'set -a; . ./.env; set +a; cd web && npm run lint && npm test && npx playwright test'`
- [ ] 10.3 `docs/handoff/` を PR の前にもう 1 度読み、並走中の change が merge していたら rebase で追従する
  （足された表 → 2.1 / 2.4 の試験が自動で拾う / 足された e2e → 既定の `storageState` で通る / superuser の操作を使う試験 → 所有者で書き直す /
  st12 が変えた `testdb.rs` のプールの持ち方 → `app_pool()` を同じ持ち方に揃える / st12 の `docker-compose.yml` の `max_connections` → 残したまま合言葉だけ `${…}`）。
  検証: `python3 scripts/review_triage.py . st28-private-network-only`
