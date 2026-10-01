## Context

動機は `proposal.md` の Why。振る舞いの契約は `specs/data-sensitivity/spec.md`。本人の決定と聞かずに決めた既定（C1〜C7）は `deep.md`。
独立レビューの指摘と処置は `review/spec.md`。

いまの形（2026-09-29 に読んだもの）:

- サーバ（`crates/server/src/lib.rs` の `run()`）は `DATABASE_URL` の 1 本で接続し、**起動のたびに全移行を当てる**。
  開発・CI・確認バッチのどれも、その接続は `docker-compose.yml` の superuser（`ashiato` / 公開の固定値 `ashiato`）
- `authorize()` は `authorization: Bearer <API_TOKEN>` だけを見る。`/healthz` 以外の全 route がこれを通る
- 画面は React の SPA を vite（`tools/stack.sh` は `vite preview --host 127.0.0.1`、`tools/dev.sh` は `vite dev --host 127.0.0.1`）で配り、
  `/api/*` を proxy でサーバへ送る。**proxy が `API_TOKEN` を付ける**（`web/vite.config.ts`）。画面は `/api/ingest` にも書く（ST19 の主張の登録）
- 網越しの到達は、機械の上の `tailscale serve`（リポジトリの外の設定）が loopback の口へ中継している。ashiato の口は 5 つとも http（`review/deep.md`）
- Android の収集アプリは `ashiato.baseUrl` の host に平文を許す network security config を build で生成する（ST01 D26）。
  計測テストは端末の中の試験用サーバ（`http://127.0.0.1:18787`）へ送る（`tools/android-emulator.sh` / CI）
- C-02（`crates/collector-windows`）は `ASHIATO_BASE_URL` を検査せずに使う（既定 `http://127.0.0.1:8787`）
- 試験（`crates/server/src/testdb.rs`）は共有の開発 DB に superuser で接続し（既定の URL に固定の合言葉）、`stay_tests` / `attributes_tests` は `CREATE DATABASE` を使う。
  門の試験（`TRUNCATE` が拒まれる など）は**トリガが止めること**を確かめている
- `tools/smoke.sh` は `docker compose down -v` で DB を作り直し、`tools/stack.sh` は `STACK_RESET=1` で同じことをする（CI の e2e）。どちらも `.env` の全部を export してサーバを起動する
- `tools/check-immutable.sh` は `docker compose exec psql -U ashiato`（コンテナ内の socket。trust）で門を叩く
- DB の schema は `core` の 1 つだけ

## Goals / Non-Goals

**Goals**
- spec の 10 Requirement を、**並走中の Story（st06 / st08 / st12 / st22）の移行・route・e2e を書き換えずに**成り立たせる
- 起動の前提（待ち受けのアドレス・DB の接続先と役割・画面の合言葉）を、**起動時の検査で**落とす（規約や手順書に頼らない）

**Non-Goals**
- 呼び出し元ごとの資格情報・収集側の合言葉の範囲を絞ること（Q4。ST29）
- 感度（PERM-2〜6。ST24）
- 拠点外へのバックアップ（ST30 / ST31）
- 網の手段の設定そのもの（`tailscale serve` の口を作る・消す）を自動で行うこと。手順書と、状態を見る検査までを持つ（D11 / D12）

## Decisions

### D1. 資格情報は 2 種類。API の合言葉とログインの印は、同じ route で同じ重みを持つ

`authorize()` を「`Bearer <API_TOKEN>` が一致する」**または**「有効なログインの印の cookie がある」に広げる。どちらも全 route で通す。

- **理由**: 画面は `/api/ingest` にも書く（ST19）。route ごとに受ける資格情報を分けると、並走中の st12 / st22 が足す route の扱いを
  あちらの change に決めさせることになる（差し戻し）。範囲を分けるのは呼び出し元ごとの資格情報を決める ST29
- **採らなかった案**: 画面の印を読み出しの route だけに通す → ST19 の画面の登録が 401 になる
- 呼び出し元の種類（`web_session` / `api_token` / `none`）を `authorize()` が返し、D8 の読み出しの記録に書く
- 合言葉の比較はどちらも `token_matches`（`subtle`。record-envelope の「一致した長さから推測されない」をそのまま使う）

### D2. ログインの口は `POST /session`（ログイン）/ `DELETE /session`（ログアウト）/ `GET /session`（状態）

| 口 | 入力 | 出力 |
|---|---|---|
| `POST /session` | `{"password": "<画面の合言葉>"}` | 一致: `204` + `Set-Cookie`。不一致・欠落: `401`（本文 `unauthorized`。何が違ったかは言わない）。D17 の上限: `429` |
| `DELETE /session` | cookie | `204` + 印を消す `Set-Cookie`（`Max-Age=0`）。印が無くても `204` |
| `GET /session` | cookie か Bearer | `200 {"credential": "web_session" \| "api_token"}` / `401` |

- 画面は起動時に `GET /api/session` を叩き、401 なら合言葉の入力欄を出す（記録の読み出しを先に叩かない）
- `POST /session` は `authorize()` を通さないが、**画面の合言葉そのものを資格情報として要求する**（record-envelope の「すべての API 要求は資格情報を要求する」の例外ではない。spec の注記）
- 応答の形は OpenAPI（`ApiDoc`。コードから生成。製造準備 A-1）が契約。`docs/openapi.json` を再生成する（`tools/check-openapi.sh`）

### D3. ログインの印は乱数の cookie。DB には SHA-256 と、合言葉の世代の印だけを置く

- 印: 32 バイトの乱数を base64url にしたもの。cookie の名前は `ashiato_session`
- 属性: `HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=<D18>`。`Domain` は付けない（画面の host だけ）
- **別のサイトからの書き込み（CSRF）**: `SameSite=Strict` で印が付かない。加えて書き込みの口は JSON の本文（`content-type: application/json`）しか
  受けない（axum の `Json` が他を 415 で断る）ので、別サイトのフォームからは届かない。独自ヘッダを要求する案は、画面の全 `fetch`
  （並走中の change が足すものを含む）を書き換えることになるので採らない
- 表 `core.web_session`（D9 の移行）: `token_sha256 bytea PRIMARY KEY` / `secret_tag bytea NOT NULL` / `issued_at timestamptz NOT NULL` / `revoked_at timestamptz NULL`。
  アプリの役割は INSERT と `revoked_at` の UPDATE だけを使う
- **有効の条件**: 行がある ∧ `revoked_at IS NULL` ∧ `secret_tag = 今の世代の印` ∧（期限が設定されていれば `issued_at + 期限 > now`）
- **`secret_tag` = HMAC-SHA256（鍵 = `API_TOKEN`、本文 = `ashiato-web-session-v1` ‖ 画面の合言葉）**。
  画面の合言葉を変えると全行が条件から外れる（Q6 の締め出し）。`API_TOKEN` を変えても外れる（電話を落としたときの手順で両方変える。D12）。
  行は消さない（ログアウトも `revoked_at` を付けるだけ）
- **鍵を `API_TOKEN` にする理由**（review R22）: 鍵が DB に無いので、**DB の写し（バックアップを含む）だけでは合言葉の候補を確かめられない**。
  新しい秘密を増やさずに済む
- **採らなかった案**: 鍵を画面の合言葉にして固定の文字列を HMAC する（当初の案）→ 塩の無い速いハッシュが全行に入り、DB の写しから総当たりで合言葉を確かめられる。
  署名つきの自己完結の cookie（DB を引かない）→ ログアウトした印を持ち出されたら止められない

### D4. DB の役割は 3 つ。アプリは `ashiato_app`、移行は `ashiato_owner`、管理者は起動の足場だけ

| 役割 | 何をするか | 性質 |
|---|---|---|
| 管理者（`POSTGRES_USER`、既定 `ashiato`） | 役割を作る・所有を移す（`tools/db-roles.sh` だけ） | superuser。合言葉は `.env` の `POSTGRES_PASSWORD` |
| `ashiato_owner` | 移行を当てる・`core` の全オブジェクトを所有する・試験の `CREATE DATABASE` | `LOGIN CREATEDB NOSUPERUSER NOCREATEROLE`。合言葉は `.env` の `OWNER_DB_PASSWORD` |
| `ashiato_app` | サーバの実行時 | `LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE`、何も所有しない。合言葉は `.env` の `APP_DB_PASSWORD` |

- **付与の段**（`migrate()` の最後に、移行の配列とは別に毎回当てる。`crates/server/src/grants.sql`）:
  `GRANT USAGE ON SCHEMA core`、`core` の全表に `SELECT, INSERT, UPDATE, DELETE`（**TRUNCATE と REFERENCES・TRIGGER は付けない**）、
  全 sequence に `USAGE, SELECT`、全関数に `EXECUTE`、さらに `ALTER DEFAULT PRIVILEGES FOR ROLE ashiato_owner IN SCHEMA core` で同じものを既定にする。
  **配列の外に置く理由**: 並走中の st12 / st22 は配列の末尾に移行を足す。付与を配列の 1 本にすると、あちらの移行の後ろに来るとは限らない
- DELETE を付けるのは、門（トリガ）が「台帳が同じまとまりにあるときだけ」を判定する側だから。権限で塞ぐと ST23 の正当な消去も塞がる
- **`session_replication_role`**: PostgreSQL 15 以降は非 superuser に `GRANT SET ON PARAMETER` で渡せるが、渡さない
- **起動時の自己検査**（`run()` の DB 接続の直後）: `SELECT rolsuper FROM pg_roles WHERE rolname = current_user` が真なら拒否。
  `core` の表のいずれかの所有者が `current_user` なら拒否（`pg_tables.tableowner`）。理由は `kind = "db_role"` で出す（値は出さない）
- **雛形の合言葉を拒む**（review R13）: `tools/db-roles.sh` は `POSTGRES_PASSWORD` / `OWNER_DB_PASSWORD` / `APP_DB_PASSWORD` のどれかが空か `change-me` で始まれば、何も作らずに exit 1 して項目の名前だけを出す
- 所有者は自分の表のトリガを外せる。**所有者の合言葉は管理者と同じ扱い**（配布物に固定値を持たない・サーバの環境に渡さない。D5）

### D5. 移行は `ashiato-server migrate` に分ける。サーバの起動では当てない。サーバの環境から管理者と所有者の秘密を外す

- `ashiato-server migrate` は `DATABASE_OWNER_URL` で接続し、`MIGRATIONS` を順に当てて付与の段を当てる。`ashiato-server`（引数なし）は `DATABASE_URL`（アプリ）で起動し、移行を当てない
- **DB を作り直す台本も含めて**、起動の台本（`tools/stack.sh`（`STACK_RESET=1` を含む）/ `tools/dev.sh` / `tools/smoke.sh`（`down -v`）/ 確認バッチの `run.sh`（`stack.sh` を呼ぶ）/ CI）は
  **`db-roles` → `migrate` → サーバ**の順に呼ぶ（volume を消すと役割も消える。review R17）
- 台本はサーバを `env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL …` で起動する（`.env` の全部を export しているので。review R13）
- **理由**: アプリの役割は表を作れない（それが目的）。サーバの実行時のプロセスに所有者の合言葉を持たせない
- **採らなかった案**: サーバが 2 本の URL を持ち、起動時に所有者で移行してから落とす → 実行時の環境に所有者の合言葉が残る

### D6. 足す crate は `hmac` / `rand` / `axum-extra`（cookie）/ `tower-http`（応答ヘッダ）

`sha2` / `subtle` は既にある。どれも MIT か Apache-2.0。`tools/check-licenses.sh` が見る。
D14 の一覧（外部の宛先へ送る部品）に当たるものは足さない。

### D7（仮）. 待ち受けは loopback だけ。`ALLOWED_NETS` は置かない

- `BIND` をアドレスに解決し、すべてが loopback（`127.0.0.0/8` / `::1`）なら通す。未指定（`0.0.0.0` / `::`）は `bind_unspecified`、それ以外は `bind_not_loopback` で拒否し、終了コード 2
- **判定は DB への接続より前**（`API_TOKEN` / `WEB_PASSWORD` の検査の直後）。拒否の試験が DB を要らない形にし、拒否されるべき起動が DB に触れないようにする
- 判定は純関数 `bind_allowed(addr) -> Result<(), BindRefusal>` に置き、試験は関数と、実際に起動して終了コードを見る結合の 2 段
- 画面（vite）は `--host 127.0.0.1` を `tools/stack.sh` / `tools/dev.sh` が固定で渡す（据え置き）。その状態は D11 の検査が見る
- `tools/stack.sh` の案内「端末から届くには BIND を LAN / Tailscale の IP にする」を、網の手順書（D12）への案内に書き換える
- **なぜ C1 から変えたか**（review R1）: deep の C1 は「loopback 以外は `.env` で明示した網だけ許す」だったが、それは網のアドレスにサーバが平文で直に口を開ける経路を正しい振る舞いとして残し、
  **本人が Q2 で選んだ「サーバと画面は 127.0.0.1 から動かない」「網の内側から届く口は暗号化された接続だけ」と食い違う**。
  厳しい側（Q2）に揃えた。PERM-7 の「明示的に許可した」は、利用者が網の手段に書く中継の設定で成り立つ
- **反転条件**: loopback の口へ暗号化して中継できない網の手段を使うことになったら、「暗号化を自前で持つ口に限って、明示した網のアドレスでの待ち受けを許す」を足す（そのとき spec を MODIFIED にする）

### D8. 読み出しの記録（C5）は `core.access_log`。axum の middleware で 1 求め 1 行。書けなければ断る

- 列: `at timestamptz` / `via text`（`direct` | `forwarded`）/ `credential text`（`web_session` | `api_token` | `none`）/
  `route text`（axum の `MatchedPath`。`/stays` のような型。**クエリ文字列と path の値は入れない**）/ `method text` /
  `outcome text`（`ok` | `unauthorized` | `login_ok` | `login_failed` | `login_throttled` | `logout`）/ `status smallint`
- 書く求め: GET の全部（`/healthz` を除く）・`/session` の 3 本・401 になった全部。**資格情報が認められた取り込み（`/ingest` `/heartbeat` `/drops`）は書かない**（spec）。
  画面のログインで通った書き込み（`/stays/rebuild` など、取り込み以外の POST）は書く
- 書く時機: **ハンドラの前**に 1 行を書き（`outcome` と `status` は認可の判定の結果。通れば `ok` / 200）、書けなければハンドラを呼ばずに 500。
  ハンドラがその後 4xx / 5xx を返しても行は足さない・直さない（追記のみ・1 求め 1 行。「誰がいつ読もうとしたか」を残すのが目的で、応答の成否はサーバのログが持つ）
- `via`: `x-forwarded-for` があれば `forwarded`（`tailscale serve` が付ける。vite の proxy は既定で足さない）。**同じ PC のプロセスが偽れる**（spec の注記）
- 追記のみ: UPDATE / DELETE の行トリガと TRUNCATE の文トリガで拒む（**この移行専用の関数**。既存の台帳と同じ形）
- 書き込みの失敗の試験: 読み出しの記録を書く口を trait にし（`AccessSink`）、試験は常に `Err` を返す偽物を差す。**表の権限を剥がす・名前を変える形は使わない**（並走する試験を壊す）
- **採らなかった案**: 書けなくても読み出しは返す（当初の案）→ C5 の「取っていない」を黙って作る（review R4）。ログ（tracing）に出す → ログは回転して消え、追記のみの保証が無い

### D9. 移行は 1 本（`YYYYMMDDHHMM_access_control.sql`）

`core.web_session`（D3）と `core.access_log`（D8）とその錠。当て直せる形（`IF NOT EXISTS` / `CREATE OR REPLACE`）。`.down.sql` も置く。
役割の作成は移行に入れない（役割はクラスタの物で、合言葉が要る。D4 の `tools/db-roles.sh`）。

### D10. 画面の側: proxy は何も付けない。CSP と `no-store` は vite の `preview.headers`。e2e は既定でログイン済み

- `web/vite.config.ts` の proxy から `authorization` を外す。**cookie はそのまま通る**（http-proxy の既定）
- `preview.headers`: `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'` と `Cache-Control: no-store`
- `vite dev` は HMR が inline script を使うので CSP を付けない。**その代わり開発用の画面は網へ出さない**（D11 の検査が落とす。spec）。人間の確認と e2e は `preview` を通る
- サーバの全応答に `Cache-Control: no-store`（`tower-http` の `SetResponseHeaderLayer`）
- 画面: 起動時に `GET /api/session`。401 なら合言葉の入力欄（1 枠 + 「ログイン」。`ui-direction.md` の下限: 44 px の押す面・24 px の文字の下限）。
  ログイン後は元の画面へ。画面の頭に「ログアウト」。どの面でも読み出しが 401 を返したら入力欄へ戻す
- e2e: Playwright の `globalSetup` で 1 回ログインし、`storageState` を全 project の既定にする。**既定の `page` を使う既存・並走中の e2e は書き換えずに通る**。
  ログインそのものの e2e は `test.use({ storageState: { cookies: [], origins: [] } })` で未ログインから始める
- `WEB_PASSWORD` は `.env` に置き、`tools/stack.sh` が export する。e2e は同じ値を読む。CI は job の中で乱数から作る（D19）
- `Secure` の cookie は `http://127.0.0.1` でも Chromium が受ける（loopback は安全な文脈）。受けなかったら e2e の `baseURL` を `http://localhost` にする（Risks）

### D11. 網の外に開いていない検査（C2）は `tools/check-exposure.sh`

- (a) `ss -ltnH` の待ち受けのうち、`BIND` の port・`WEB_PORT`・開発用の画面の port（`DEV_WEB_PORT`、既定 5173）・DB の `55432` のアドレスがすべて loopback
- (b) `tailscale` があれば `tailscale serve status --json`: `AllowFunnel` に真が 1 つでもあれば落ちる。本システムの port へ中継する口が `HTTPS` でなく `HTTP` なら落ちる（平文）。
  開発用の画面の port へ中継する口があれば落ちる。**JSON を読めなければ落ちる**（読めないことを通ったことにしない）。`tailscale` が無ければ (b) は飛ばしたと出す（網の手段が別のもの）
- 入力を差し替えられる（`EXPOSURE_SS_OUTPUT` / `EXPOSURE_SERVE_JSON` にファイルを渡す）。`--self-test` が fixture で spec の 6 つを撃つ
- `tools/stack.sh` は起動の後に (a) を走らせ、落ちたら止める
- **自動で直さない**。網の手段の設定は本人の網のもので、ここで書き換えると手順書に無い状態が生まれる

### D12. 手順書は `docs/network.md`（新規）

- `tailscale serve --bg --https=<port> http://127.0.0.1:<port>` を画面と API の 2 本。既存の http の口を消す（`tailscale serve --http=<port> off`）。開発用の画面の口も消す
- **HTTPS を有効にすると、機械の名前と網の名前が公開の証明書ログに載り、消せない**（Q2。本人は受け入れた。配布先の利用者に先に見せる）
- 収集アプリの `ashiato.baseUrl` を `https://<ホスト名>:<port>` にして入れ直す
- **電話を落としたとき**（Q4 の context）: 網からノードを外す → `API_TOKEN` を変えて APK と C-02 を入れ直す → 画面の合言葉を変えてサーバを起動し直す（Q6。D3 により `API_TOKEN` を変えた時点でも全端末のログインが切れる）
- **ログインの印は同じ機械名の別の口にも送られる**（cookie は port で分かれない。RFC 6265 §8.5。review R21）。この機械は同じ機械名で操作盤の口も出している。
  手順書に「画面を出す機械名に、信頼できないサービスを同居させない」を書く
- **別のサイトから始まった書き込み（CSRF）への守りは、cookie の `SameSite=Strict` と本文の型だけに頼っている**（final review R14）。
  `POST /session` は `application/json` 以外を 415 で断るが、`POST /stays/rebuild` は本文を `Bytes` で受けて型を見ない
  （空の本文を既定の基準として受ける、先行 Story の約束。この change では変えない）。
  `SameSite` は**同じサイト**（同じ機械名の別の口を含む）からの求めには cookie を付けるので、この守りは
  上の「同じ機械名に信頼できないサービスを同居させない」が守られている間だけ成り立つ。
  **反転条件**: 同じ機械名に、画面の外の手が書ける口（操作盤以外の Web サービス）を出すことになったら、
  `cookie` で認める書き込みの route すべてに `Content-Type: application/json` の要求か CSRF の印を足す
- `docs/screens.md` の「`http://<手元の網のホスト名>:5180`」を `https://` に直し、ログインの面を 1 行足す
- 本人の機械での移行の順序（D20）

### D13. 収集側の平文の検査

- **Android**: `GenerateNetworkSecurityConfig` は `base-config cleartextTrafficPermitted="false"` と、`localhost` / `127.0.0.1` だけに平文を許す `domain-config` を常に出す
  （`ashiato.baseUrl` の host から例外を作らない）。`ashiato.baseUrl` が `http://` で host が `domain-config` の 2 つ（`localhost` / `127.0.0.1`）でなければ build を落とす（gradle の task で `GradleException`。`[::1]` も落とす —— 平文の許可に無い宛先を組み立てで通すと、送るときに断られる。final review R10）
- **変更前のコードで落ちる検査**（review R14）: `-Pashiato.baseUrl=https://example.invalid:1` で組み立て、生成された `network_security_config.xml` に `example.invalid` の平文の許可が無いこと
  （いまのコードはその host に `cleartextTrafficPermitted="true"` を出すので落ちる）
- 計測テスト: `NetworkSecurityPolicy.getInstance().isCleartextTrafficPermitted("127.0.0.1")` が真（loopback の Scenario）
- **C-02**: `Config::from_env` が `ASHIATO_BASE_URL` を見て、`http://` で host が loopback（`127.0.0.1` / `::1` / `localhost`）でなければ `Err`

### D14. 拠点外への口の検査（NFR-15）は `tools/check-offsite.sh` と、DB の接続先の起動時の検査

- `cargo tree -p ashiato-server -e normal --prefix none` の crate 名を、外部の宛先へ送る crate の一覧（`reqwest` / `ureq` / `isahc` / `surf` / `attohttpc` / `curl` / `lettre` / `aws-sdk-*` / `rusoto_*` / `google-cloud-*` / `azure_*` / `object_store` / `opendal`）と突き合わせ、1 件でもあれば落ちる。
  一覧は台本の中に持ち、**空の一覧では走らない**（一覧が 1 件以上であることを台本自身が確かめる。review R8）
- **route の一覧は見ない。** route の許可一覧を置くと、並走中の st12 / st22 が route を足すたびにこの検査が落ち、走っている Story へ差し戻すことになる
- **DB の接続先**（review R8）: サーバと `migrate` は、`DATABASE_URL` / `DATABASE_OWNER_URL` の host が loopback（`127.0.0.0/8` / `::1` / `localhost`）か unix socket でなければ、接続せずに `kind = "db_not_loopback"` で終了コード 2。
  sqlx は任意の宛先へ TLS で接続できる部品で、接続先を拠点外に向けると記録の写しがまるごと拠点外に置かれる
- **限界**: tokio の `net` と sqlx で外へ接続を張るコードは書ける。この検査が止めるのは「外への送り手の部品を足すこと」と「DB を拠点外に向けること」まで
- 暗号化されたバックアップの送り手を足す Story は、この一覧からその crate だけを外し、spec を MODIFIED にする

### D15. 手元の機械の短い名前も pre-commit で禁じる（C3 / R12）

`tools/check-private.sh --staged` は、`tailscale` があれば `tailscale status --json` の `Self.DNSName` の先頭のラベル（機械の短い名前）と
網の名前（2 つめのラベル）をその場で読み、禁止語に足す。**値はリポジトリにも出力にも書かない**（一致した行の位置だけ出す）。`tailscale` が無ければ足さない。

### D16（仮）. 画面の合言葉の下限は 16 文字。雛形の値（`change-me` で始まる）のままでは起動しない

API の合言葉と同じ下限。**加えて `change-me` で始まる値は長さに関係なく拒む**（`reason=placeholder`。値は出さない）——
`.env.example` の雛形 `change-me-web-password` は 22 文字で下限を通ってしまい、雛形を写しただけの `.env` で
公開されている合言葉のまま画面が開く（final review R1）。判定は `tools/db-roles.sh` が DB の合言葉に使う規則と同じ。
**反転条件**: 本人が電話で打つのが負担だと言ったら 12 文字に下げる（期限なし（Q6）なので打つのは端末ごとに 1 回）。
spec の Scenario（15 文字で起動しない）も同時に直す。

### D17（仮）. ログインの失敗が 1 分に 10 回を超えたら、その 1 分が過ぎるまで 429。失敗は 1 回ごとに 1 秒待たせる

総当たりを遅くする。網の内側からしか届かない（PERM-7）ので、長い締め出し（一定回数でロック）はしない —— 本人が締め出される。
数は機械全体で 1 つ（呼び出し元ごとに分けない。経路は偽れる）。読み出しの記録には `login_throttled` で残す。
**反転条件**: 読み出しの記録に 1 日 100 回を超える `login_failed` が出たら、ロックと通知を足す。本人が 429 に当たって困ったら上限を上げる。

### D18（仮）. ログインの期限の設定は `WEB_SESSION_MAX_AGE_DAYS`（既定 0 = なし）。cookie の寿命は 400 日で、使うたびに延ばす

Q6 の「期限なし」はサーバ側の判定。**ブラウザは cookie の寿命を最長 400 日に切り詰める**（Chromium の仕様）ので、
印で認めた応答のたびに `Set-Cookie`（`Max-Age=34560000`）を出し直して延ばす。400 日開かなかった端末はブラウザが印を捨て、もう 1 度ログインになる。
**反転条件**: 本人が「一定期間で切れてほしい」と言ったら `WEB_SESSION_MAX_AGE_DAYS` の既定を変える（Q6 を本人に問い直す）。

### D19. 試験の DB: 既存の試験は所有者で、役割の試験はアプリで。合言葉は環境からだけ読む

- `testdb::pool()` は `DATABASE_OWNER_URL` で接続する。**既定の URL（固定の合言葉入り）を消す** —— 無ければ「`.env` を読み込むか `DATABASE_OWNER_URL` を渡す」と出して落ちる（飛ばさない。review R13）。
  **既存の門の試験は、所有者の接続でもトリガが止めることを確かめている**（管理者でなくても、門は権限ではなくトリガで効く）ので、意味を変えない
- `testdb::app_pool()` を足し（`DATABASE_URL`）、役割の Scenario と、アプリの役割でのハンドラの結合（取り込みと読み出し）をそこで撃つ
- `CREATE DATABASE` は所有者（`CREATEDB`）で通る。`fresh_db` の URL は `DATABASE_OWNER_URL` から組む
- **役割は試験の前に `tools/db-roles.sh` が作る**（開発 DB と CI の両方）。役割が無ければ `testdb` はその旨を出して落ちる
- **CI の合言葉は job の中で乱数から作る**（`openssl rand -hex 24` を `$GITHUB_ENV` へ）。`services` の `POSTGRES_PASSWORD` も `${{ env.… }}` で渡し、ワークフローに字面の値を書かない

### D20（仮）. 本人の機械での移行は、手順書に置いて merge の後に本人が行う

実データの入った DB の所有の移し替え・`.env` の新しい秘密・`tailscale serve` の https 化・`ashiato.baseUrl` の書き換えと APK の入れ直しは、
本人の機械と網の上でしかできず、tasks の実装者は触らない（触ると手順書に無い状態が生まれる）。`docs/network.md` に次の順で置き、PR 本文にも写す:

1. `pg_dump` で退避する（`tools/db-roles.sh` は所有を移すだけで行を変えないが、実データの DB に対する初めての操作なので）
2. `.env` に 4 つの秘密を足し、`DATABASE_URL` をアプリの役割に、`DATABASE_OWNER_URL` を足す
3. `tools/db-roles.sh` → `ashiato-server migrate` → サーバ
4. `tailscale serve` の口を https に置き換え、http と開発用の画面の口を消す → `tools/check-exposure.sh`
5. `~/.gradle/gradle.properties` の `ashiato.baseUrl` を `https://` にして APK を入れ直す。C-02 の `ASHIATO_BASE_URL` は loopback のままでよい

確認バッチ（`tools/verify-prep.sh`）は、`ashiato.baseUrl` が平文で loopback でなければ APK を作らずに「D20 の 5 を済ませる」と出す（組み立てが落ちて確認バッチ全体が止まるのを避ける）。
**反転条件**: 確認バッチの `run.sh` が 2〜3 を自動で済ませられると分かったら、`run.sh` に取り込む（4〜5 は網と端末の上なので残る）。

### D21（仮）. PERM-10 の画面の経路は、画面の合言葉のログインで満たす

PERM-10 は「資格情報の配り方は ST29 の深掘りで決める」としていたが、Q1 で画面の合言葉を API の合言葉と別に持つことにしたので、配り方の一部を ST28 で決めたことになる。
`docs/requirements.md` の PERM-10 に ★ 2026-09-29 補足を入れ、`docs/stories/INDEX.md` に ST28 が PERM-10 を画面の経路で満たし直すことを注記した（review R19）。
**反転条件**: ST29 の深掘りで、画面も呼び出し元ごとの資格情報の 1 つとして配り直すと決めたら、この補足をそちらへ寄せる。

## Risks / Trade-offs

- [既存の開発 DB は superuser がすべてを所有している] → `tools/db-roles.sh` が `core` の schema と全オブジェクトの所有を `ashiato_owner` へ移し、
  `ashiato` DB の所有者も移す。当て直せる形（2 回走らせても同じ）。`REASSIGN OWNED BY` は bootstrap の superuser には使えない（システムのオブジェクトを持つ）ので、
  `pg_class` / `pg_proc` / `pg_type` を引いて 1 つずつ `ALTER … OWNER TO`
- [既存の volume の superuser の合言葉は `POSTGRES_PASSWORD` を変えても変わらない（初期化のときだけ読まれる）] → `tools/db-roles.sh` が `ALTER ROLE … PASSWORD` で `.env` の値に揃える
- [並走中の change の試験が superuser を前提にしている / `testdb.rs` を変えている（st12 はプールの持ち方を変え、`docker-compose.yml` に `max_connections` を足す）] →
  `testdb::pool()` は所有者で、所有者は `CREATE DATABASE` もトリガの外の DML も通る。superuser だけの操作（`session_replication_role`・役割の作成）を試験で使うものと、
  st12 のプールの持ち方は、その change が merge した後に ST28 が rebase で揃える（Task 10）。いま main にそれを使う試験は 0 本（`grep -rn session_replication_role crates` が 0 件）
- [`api_tests.rs` が `ALTER TABLE core.coverage ADD CONSTRAINT` を使う] → 所有者なら通る（D19）
- [Chromium が `http://127.0.0.1` で `Secure` の cookie を捨てる] → Task 6 の最初に確かめ、捨てるなら e2e の `baseURL` を `localhost` にする
- [CSP が React の style を止める] → React は `style` を CSSOM で書くので CSP の対象外。e2e で CSP 違反の報告が 0 件であることを見る（spec）
- [`tailscale serve status --json` の形が版で変わる] → D11 の (b) は `AllowFunnel` と `TCP.*.HTTPS` / `HTTP` だけを読む。読めなければ落とす（spec）
- [ログインの印が同じ機械名の別の口へ送られる] → 主張できない。手順書で同居を避ける（D12）
- [読み出しの記録（D8）が肥大する] → 1 行は 100 バイト程度。画面を 1 日 100 回開いて 1 回 10 求めでも 1 年 36 万行。消さない（追記のみ）
- [読み出しの記録が書けないと画面が全部 500 になる] → 厳しい側（C5）。DB が書けないならほかの読み出しも怪しい。`kind = "access_log_write_failed"` で原因が分かる

## Migration Plan

1. 本人の機械の移行は D20 の順（手順書 `docs/network.md`）
2. **戻し**: `.down.sql` で 2 表を消し、コードを revert する（`DATABASE_URL` を所有者に戻すと D4 の自己検査に当たる —— 戻すなら自己検査ごと戻す）。役割と所有の移し替えは残ってよい
