## Context

動機は `proposal.md` の Why。観測できる振る舞いは `specs/`（`satellite-platform`・`data-sensitivity`・`data-durability`）が持つ。
ここは「どう作るか」と、下流が実装中に迷いそうな判断を先に決めたもの。

いまの資格情報の判定（`premises.md` P4）:

- `crates/server/src/lib.rs` の `authorize_quiet` が `Authorization: Bearer` を `API_TOKEN` と比べ（`token_matches`）、合わなければ画面のログインの印（cookie）を見る。
  結果は `web_session::Caller`（`WebSession` / `ApiToken`）
- 判定は最も外側の層 `web_session::refresh_cookie` が `decide` で 1 回だけ下し、task-local の `AUTHN` に置く。内側（読み出しの記録の middleware・ハンドラの `authorize`）は `decided` で読む
- 読み出しの記録の middleware（`access_log::middleware`、`route_layer`）はハンドラの前に 1 行書く。`/healthz` と `/session` は通らない（`/session` はハンドラが自分で書く）。
  資格情報が認められた取り込み（`/ingest` `/heartbeat` `/drops`）は書かない
- `authorize` は 33 か所から呼ばれ、`Caller` が何であっても `Ok` を返す
- `core.access_log` は追記のみ（UPDATE / DELETE / TRUNCATE を拒むトリガ）。`credential` と `outcome` に CHECK がある
- `migrate()` は起動のたびに全版を当て直す（各版は当て直せる形で書く）。アプリの役割への付与は `grants.sql` が全表にまとめて当てる
- 書き出しは `export::CATALOG` に分類の無い表があると始まらない

## Goals / Non-Goals

**Goals**

- 衛星の資格情報を、**足し忘れても閉じている**形で入れる —— 衛星の判定は最も外側で 1 回下し、どの口のハンドラも衛星を通さない
- 全 route の試験を、route を列挙せず router から取る（後から足した口も拾う）
- 衛星の識別子と名前の形の検査を 1 か所に置き、ST38 / ST39 がそれを呼ぶ

**Non-Goals**

- 承認（ST38）・書き手の列（ST37）・衛星の書き込み（ST39）・衛星の読み出しの門と台帳（ST45）・版の付いた契約（ST42）・AI の MCP（ST27）
- 収集アプリの合言葉の範囲（ST44。`authorize` が `ApiToken` を通すのは変えない）
- 正典 `record-envelope` の「すべての API 要求は資格情報を要求する」に例外の口を書く MODIFIED（ST44。`docs/audit/2026-10-11-consistency.md` K5）
- 衛星の名前（画面に出す名前）を変える操作（登録し直すと名前を変えられる。ふだんの改名の口は要る Story が足す）

## Decisions

### D1（仮）. 資格情報は 1 本ずつに番号を持ち、取り消した 1 本の行は消さずに取り消しの時刻を付ける

`core.satellite_credential` の行は消さない。取り消しは `revoked_at` を付けるだけ。衛星を外したときも、その衛星の資格情報すべてに `revoked_at` を付ける。
読み出しの記録には衛星の番号と 1 本の番号を残す（D9）。
- 台帳: `provisional.jsonl` の D1（`review/deep.md` R1）。失わない側（細かい粒度で残す。後から衛星の粒度に粗くできるが、逆はできない）
- **反転条件**: 1 本の粒度の手がかりが使われないまま、資格情報の表の行が増えて一覧や判定が遅くなったとき（衛星の粒度に畳む）。
  書き手（ST37）と衛星の読み出しの台帳（ST45）の粒度は、それぞれの上流が決める（`docs/handoff/ST37.md` `ST45.md`）

### D2（仮）. 本体の前置きの形で始まる識別子は登録の時に断る

`^[a-z][0-9]{2}-` で始まる識別子を 400 で断る。衛星の名前は `<識別子>:<…>` なので、識別子を断れば名前も本体の前置きで始まらない。
本体には前置きで意味を持たせている箇所がある（`lib.rs` の `LIKE 'c03-%'`・`202610141130_outage_alert.sql:31`・`web/src/App.tsx:54`・`archives.ts:103`）。
- 台帳: D2（`review/deep.md` R2）。rests_on Q1。断る側は後から緩められ、緩めても付け替えは起きない
- **反転条件**: 本体の前置きの判定をすべて登録簿の欄（ソースの種別）に移し終えたとき（形の検査は不要になる）

### D3（仮）. モノの型（FR-86）の名前に、ST29 の名前の形を当てない

ST29 はモノの台帳を作らない。モノの型の名前の形は ST40 の深掘りで、最初のモノが書かれる前に問う（`docs/handoff/ST40.md`）。
- 台帳: D3（`review/deep.md` R3）
- **反転条件**: ST40 の深掘りで本人が「衛星の名前と同じ形にする」を選んだとき（`satellite-platform` の名前の形の Requirement にモノの型を足す）

### D4（仮）. 読み出しの記録の資格情報の種類に「衛星」を足し、衛星と 1 本の番号の欄を足す

正典 `data-sensitivity` の読み出しの記録の Requirement を MODIFIED にする。PERM-13 の ★ C6 が禁じたのは記録と利用者の識別子で、衛星と資格情報の番号は呼び出し元を指すもの。
欄に入れるのは**本体の中の番号（uuid）**で、識別子の文字列（`dev.example.booklog`）ではない —— 書き換えられない表に識別子の形を焼き込まない
（Q1 の答えが推奨と違っても、読み出しの記録の行は書き直さずに済む）。
- 台帳: D4（`review/deep.md` R4）。rests_on D1
- **反転条件**: 衛星の読み出しの台帳（ST45）が、断った求めも含めて衛星の粒度で残すようになり、読み出しの記録の衛星の行が誰にも読まれなくなったとき（欄は残し、書くのを止める）

### D5. 識別子と名前の形の検査は `satellite.rs` の純粋関数 1 か所

- `check_identifier(&str) -> Result<(), IdentifierRefusal>`: `^[a-z0-9-]+(\.[a-z0-9-]+)+$` かつ 64 文字以下（形）・`ashiato.` で始まらない（本体が使う）・`^[a-z][0-9]{2}-` で始まらない（D2）。
  拒む理由は種別（`malformed` / `reserved` / `body_prefix`）で返し、応答の本文はこの種別の固定の語だけにする
- `name_owner(&str) -> Option<&str>`: `<識別子>:<名前の部分>` を分け、両方が形に合えば識別子を返す。名前の部分は `^[a-z0-9-]{1,64}$`（D15（仮））。
  ST29 では呼ぶ口が無い。ST38 / ST39 が論理ソースと種類の登録で呼ぶ（呼び出し側の約束は `satellite-platform` の「衛星が登録する名前は…」）
- 本体の名前に `:` が無いことは、登録簿を読む試験（`本体の論理ソースの名前は区切りを含まない`）で固定する。DB の CHECK にはしない（ST39 が衛星の名前を同じ登録簿に入れるとき、CHECK を外す移行が要る）
- 識別子の正規化（大文字を小文字にするなど）はしない。形に合わなければ断る（何が保存されたかを利用者が推測しなくてよい）

### D6. 資格情報の値は `ashsat_<識別子>_<乱数>`、保存は値全体の SHA-256

- 乱数は `rand::rngs::OsRng` の 32 バイトを `base64::URL_SAFE_NO_PAD`（43 文字）。値は 7 + 識別子 + 1 + 43 文字
- 保存は値全体の SHA-256（`token_sha256 bytea UNIQUE`）。値は 256 bit の乱数を含むので、遅いハッシュ（argon2 など）は要らない。画面のログインの印（`web_session`）と同じ形
- 照合は「`Bearer ` の値が `ashsat_` で始まる」ときだけ行い、SHA-256 で行を引く（行の有無で時間が変わっても、漏れるのは 256 bit の乱数のハッシュの一致だけ）
- 値は `POST /satellites/{id}/credentials` の応答に 1 回だけ載せ、ログにも読み出しの記録にも出さない（`tools/check-log-private.sh` の対象）

### D7. 表は 2 つ。行は消さない。最初の 2 つは移行が入れる

```sql
CREATE TABLE IF NOT EXISTS core.satellite (
  id            uuid        PRIMARY KEY,
  identifier    text        NOT NULL UNIQUE CHECK (identifier ~ '^[a-z0-9-]+(\.[a-z0-9-]+)+$' AND length(identifier) <= 64),
  name          text        NOT NULL CHECK (name = btrim(name) AND length(name) BETWEEN 1 AND 40),
  builtin       boolean     NOT NULL DEFAULT false,
  registered_at timestamptz NOT NULL,
  removed_at    timestamptz,
  CHECK (NOT (builtin AND removed_at IS NOT NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS satellite_live_name ON core.satellite (name) WHERE removed_at IS NULL;

CREATE TABLE IF NOT EXISTS core.satellite_credential (
  id           uuid        PRIMARY KEY,
  satellite_id uuid        NOT NULL REFERENCES core.satellite(id),
  label        text        NOT NULL DEFAULT '' CHECK (length(label) <= 40),
  token_sha256 bytea       NOT NULL UNIQUE,
  issued_at    timestamptz NOT NULL,
  last_used_at timestamptz,
  revoked_at   timestamptz
);
```

- 最初の 2 つは移行が `INSERT … ON CONFLICT (identifier) DO NOTHING` で入れる（固定の uuid。当て直しても増えない）。`builtin = true` は外せない（CHECK と口の 409 の両方）
- **行を消させない**: 2 つの表に DELETE と TRUNCATE を拒むトリガを付ける（この移行専用の関数。他の版の関数を使わない —— 戻し手順が依存で当たらなくなる）。
  UPDATE は許す（`removed_at` `revoked_at` `last_used_at` `name` を書く）
- 登録し直し（Q2）: 外した衛星と同じ識別子の登録は `UPDATE … SET removed_at = NULL, name = $2` で同じ行を戻す（`registered_at` は最初の登録の時刻のまま。番号が同じことが「続き」）。
  資格情報は外したときに全部 `revoked_at` が付いているので戻らない
- 名前は口で前後の空白を除いてから保存する（`name` の CHECK は `name = btrim(name)` も見る）。名前の重なり（D16（仮））は外していない衛星どうしの部分一意索引で見る（保存した名前で比べるので、前後の空白だけ違う名前も重なる。同時の登録の競合も DB が断る → 409）
- 移行の名前は作成時刻 `YYYYMMDDHHMM_satellite_credentials.sql`（と `.down.sql`）。`migrations/ORDER` の末尾へ 1 行

### D8. 判定: `Caller::Satellite` を足し、最も外側の判定と `authorize` の両方で閉じる

- `web_session::Caller` に `Satellite { satellite: Uuid, credential: Uuid }` を足す（`Copy` のまま）。`as_str()` は `satellite`
- `authorize_quiet`: `Bearer ` の値が `ashsat_` で始まれば D6 で引く。引けて、資格情報が取り消されておらず衛星が外されていなければ、
  同じ文で `last_used_at` を求めの時刻（`app.now()`）にして `Satellite` を返す（`UPDATE … FROM core.satellite … RETURNING`。1 求め 1 回）。
  `ashsat_` で始まる値が引けなければ（取り消した・外した衛星・知らない値）、印を見ずに 401（D19（仮））。`ashsat_` で始まらない `Bearer` は今までどおり
- `Bearer` が衛星の値で、かつ有効な印も持つ求めは `Satellite`（`Bearer` を先に見るので自然にそうなる。D18（仮））。`refresh_cookie` は `WebSession` のときだけ印を出し直すので、衛星の求めには出さない
- `authorize` は `Satellite` に `Err(403, "forbidden")` を返す（33 か所の呼び出し元は変えない）。middleware を通らない口（`GET /session`）もこれで閉じる
- `access_log::middleware`: 判定が `Satellite` なら、route が何であれ（取り込みの口も）403 の行を書き、**ハンドラを呼ばずに** 403 を返す。
  ハンドラを呼ばないので、ハンドラが判定より先に本文を読む口（取り込み）でも衛星の本文は処理されない
- `GET /session`（`session_get`）: `authorize` の 403 を受けたら、資格情報の種類 `satellite`・結果 `forbidden`・403 の行を書いて 403 を返す
- `POST /session`（ログイン）と `DELETE /session`（ログアウト）は `Bearer` を見ないので変えない（資格情報の要らない口は衛星の資格情報の有無で同じ応答）。`/healthz` も変えない
- 衛星の管理の口（D11）は `authorize` の結果が `WebSession` のときだけ通し、`ApiToken` は 403

**なぜ middleware で止めるか**（`authorize` だけでなく）: ハンドラの中で `authorize` より前に DB を読む口や、`authorize` を呼び忘れた口があっても閉じている。
`authorize` でも止めるのは、`router()` を通らない呼び出し（ハンドラを直に呼ぶ試験）と、middleware を通らない `/session` のため。

### D9. 読み出しの記録: `satellite` と `forbidden`、2 つの番号の欄

```sql
ALTER TABLE core.access_log ADD COLUMN IF NOT EXISTS satellite_id uuid;
ALTER TABLE core.access_log ADD COLUMN IF NOT EXISTS satellite_credential_id uuid;
DO $do$ BEGIN
  -- 列挙の CHECK は、この版の語を含む定義がまだ無いときだけ付け替える（review/code.md R3 / R7）
  IF NOT EXISTS (… conname = 'access_log_credential_check' AND pg_get_constraintdef(oid) LIKE '%''satellite''%') THEN
    DROP / ADD access_log_credential_check CHECK (credential IN ('web_session', 'api_token', 'satellite', 'none'));
  END IF;
  IF NOT EXISTS (… conname = 'access_log_outcome_check' AND pg_get_constraintdef(oid) LIKE '%''forbidden''%') THEN
    DROP / ADD access_log_outcome_check
      CHECK (outcome IN ('ok', 'unauthorized', 'forbidden', 'login_ok', 'login_failed', 'login_throttled', 'logout'));
  END IF;
  -- 2 つの番号は衛星の行ではそろって在り、ほかの行ではそろって無い（review/code.md R8）
  ALTER TABLE core.access_log DROP CONSTRAINT IF EXISTS access_log_satellite_ids;
  IF NOT EXISTS (… conname = 'access_log_satellite_pair') THEN
    ADD access_log_satellite_pair CHECK (CASE WHEN credential = 'satellite'
      THEN satellite_id IS NOT NULL AND satellite_credential_id IS NOT NULL
      ELSE satellite_id IS NULL AND satellite_credential_id IS NULL END);
  END IF;
END $do$;
```

- **起動のたびに付け直さない**（review/code.md R3 / R7）: `migrate()` は全版を当て直すので、無条件の DROP / ADD だと、後の版（ST38 / ST44 / ST45）が
  種類か結果を足して行を書いた後にこの版の CHECK が当たらず起動が止まり、追記のみの表を毎回 ACCESS EXCLUSIVE で全走査する

- 制約の名前は `202609290900_access_control.sql` が付けた既定の名前（`<表>_<列>_check`）。移行の前に `\d core.access_log` で名前を確かめ、違えば合わせる（tasks 1.1）
- 外部キーは付けない（追記のみの表が登録簿の行に縛られると、登録簿の戻し手順が当たらなくなる。登録簿の行は消さないので指す先は残る）
- `AccessEntry` に `satellite: Option<(Uuid, Uuid)>` を足す。`credential` が `satellite` のときだけ `Some`（DB の CHECK と同じ約束を型の作り方で守る: `AccessEntry::for_caller(caller, …)` で作る）
- 既存の行は新しい CHECK を満たす（`satellite` も `forbidden` も無く、欄は NULL）
- **書き換えられない表の行の形を、固定した値で試す**（過去の取りこぼし「書き直せない記録の形」）: 衛星の行を 1 行書き、全列を読んで期待の値と比べる試験を置く。
  知らない値（将来の資格情報の種類）を読む側は無い（読み出しの記録を読む口は無い）

### D10. FR-77 の検査は `tools/check-satellite-boundary.sh`

- `cargo tree -p ashiato-server -e normal --target all --prefix none` の名前を、禁じる部品の一覧
  （`libloading` `dlopen` `dlopen2` `wasmtime` `wasmer` `wasmi` `extism` `mlua` `rlua` `rhai` `pyo3` `deno_core` `v8` `boa_engine` `abi_stable`）と突き合わせ、1 つでもあれば名前を出して rc=1
- 突き合わせは「依存の名前の一覧を標準入力から読む」関数に分け、`--self-test` で固定の一覧（`libloading` と `wasmtime` を含む）を食わせて落ちることを確かめる（Scenario `読み込む部品が依存に入ると検査が落ちる`）
- ソースに `dlopen` / `LoadLibrary` / `Library::new` の字面が無いことも grep で見る（依存を通さずに FFI で呼ぶ抜け道）
- CI の既存の検査の job に足す（`docs/testing.md` の表に 1 行）
- `Cargo.lock` の字面では見ない —— `libloading` は `clang-sys`（ビルドの道具）経由で既に lock にある（`premises.md` P7）

### D11. 衛星の管理の口は `/satellites` の下に 5 本

| 口 | 何を | 成功 | 断り |
|---|---|---|---|
| `GET /satellites` | 外していない衛星（資格情報の取り消していないものを入れ子で）と外した衛星の一覧 | 200 | 401 / 403 |
| `POST /satellites` `{name, identifier}` | 登録（外した衛星と同じ識別子なら登録し直し） | 201（登録し直しは 200） | 400 形 / 409 重なり |
| `DELETE /satellites/{id}` | 外す | 204 | 404 / 409 最初の 2 つ・外した衛星 |
| `POST /satellites/{id}/credentials` `{label}` | 発行。応答に値を 1 回だけ | 201 | 404 / 409 外した衛星 / 400 名前 |
| `DELETE /satellites/{id}/credentials/{cid}` | 取り消す | 204 | 404 |

- どれも `WebSession` だけ（D8）。`{id}` は衛星の番号（uuid）。一覧の応答に資格情報の値もハッシュも載せない
- 一覧の資格情報の行: `id` `label` `issued_at` `last_used_at`（null 可）。衛星: `id` `identifier` `name` `builtin` `registered_at` `removed_at`
- 400 / 409 の本文は種別の固定の語（`malformed` / `reserved` / `body_prefix` / `name_invalid` / `identifier_taken` / `name_taken` / `builtin` / `removed`）。画面が日本語の理由に引く（`messages.ts`）
- utoipa で契約に載せ、`docs/openapi.json` を作り直す（`tools/check-openapi.sh`）
- 読み出しの記録には、ほかの口と同じく middleware が 1 行書く（口の名前は route の型 `/satellites/{id}`）

### D12. 画面は `SettingsView.tsx` に節を足し、部品は `SatellitesPanel.tsx`

- 書き出しの節（ST33）の前に「衛星と資格情報」の節を置く（proto の並び）。API の型と文字列は `web/src/satellites.ts`、文言は `messages.ts`
- 発行した値は React の state にだけ持ち、閉じたら捨てる（`localStorage` などに置かない。ST28 の「端末に記録の写しを置かない」と同じ）。コピーは `navigator.clipboard.writeText`
- 外すの確認・発行の欄・登録の欄は小節の中にその場で開く（Q8 の推奨の形）。同時に開くのは 1 つ
- e2e は新しい `web/e2e/settings-satellites.spec.ts`（`settings-export.spec.ts` と同じ立て方）。衛星 5 つ・資格情報 0〜2 本ずつを API で積んでから測る

### D13. 書き出し: 登録簿は `Include`、資格情報は `Exclude`

- `export::CATALOG` に `satellite`（`Include`。全列。ファイル名 `satellites.jsonl`）と `satellite_credential`（`Exclude { reason: "衛星の資格情報（値のハッシュ）" }`）を足す
- `export_file_tests.rs` の `ENTRIES` に `satellites.jsonl` を足す（数も 15 → 16）。説明書の列の意味は `CATALOG` から引く

### D14. 試験の立て方

- **全 route の試験は router から route を取る**: `router()` が持つ route の一覧を手で書かない。`ApiDoc::openapi()` の paths と method を全部取り、
  path の `{…}` を形の合う値で埋めて、衛星の資格情報を付けて呼ぶ。資格情報の要らない口（`/healthz`・`POST /session`・`DELETE /session`）は除いて 403 を確かめる
  - 契約に載らない route（`/selftest/panic` は既定の構成に無い）が漏れないよう、`lib.rs` から `\.route\(\s*"([^"]+)"`（複数行にまたがる形。rustfmt が path を次の行に送る口がある）で取った path の集合と、契約の path の集合が一致することも同じ試験で確かめる（差があれば、どの path かを出して落ちる）
  - `lib.rs` に `.merge(` / `.nest(` / `.nest_service(` が現れたら同じ試験が落ちる（字面から見えない route ができる。足す Story は、この試験の抽出をその Router まで広げる）
  - 応答の番号と、読み出しの記録の行（資格情報の種類 `satellite`・結果 `forbidden`・403）の両方を見る
- 衛星の試験は利用者でなく衛星で隔離する（識別子に試験ごとの接尾を付ける: `dev.test<乱数>.booklog`）。最初の 2 つを外す試験は 409 だけを見る（状態を変えない）。最初の 2 つに発行する試験は、同じ試験の中で取り消して閉じる（行は残るが一覧と判定には効かない）
- 「移行は最初の 2 つに資格情報を入れない」は、試験の DB ではなく、その試験の中で作った空の schema に全版を当てて確かめる（共有の試験の DB には他の試験の行が残る）
- 時刻を進める試験（400 日後）は `App` の時計の差し替え（ST28 の試験と同じ口）を使う

### D15（仮）. 名前と識別子の長さ: 識別子は 64 文字、名前の部分は 1〜64 文字、衛星の名前は 1〜40 文字、資格情報の名前は 40 文字まで

- 長さは表示と入力の都合で、記録の形には効かない（名前の部分の形は Q1 で決まり、長さは後から広げても既存の名前は変わらない）
- **反転条件**: 実在する衛星の識別子か名前の部分が上限に当たったとき（広げる。狭めるのは既存の名前を断ることになるので本人に返す）

### D16（仮）. 外していない衛星どうしで名前の重なりを断る

- PERM-14 の「取り違えにくい名前」。外した衛星の名前は再利用できる
- **反転条件**: 同じ名前の衛星を 2 つ置きたい場面が出たとき（識別子を名前と並べて出す形にして、重なりを許す）

### D17（仮）. 取り消しは確認をはさまず、取り消した 1 本は一覧に出さない。外した衛星は名前と識別子だけを並べる

- 取り消しは発行し直せば戻る（値は変わる）。確認をはさむのは外す（その衛星の資格情報が全部止まる）だけ
- **反転条件**: 押し間違いの取り消しで衛星が止まった違和感が出たとき（取り消しにも確認をはさむ）・取り消した 1 本の履歴を画面で見たい要望が出たとき（行は DB に残っている）

### D18（仮）. 衛星の資格情報とログインの印を両方持つ求めは、衛星の求めとして 403

- 厳しい側。衛星のプロセスが本人のブラウザの印を持つことは無いはずで、両方あるのは取り違えか細工
- **反転条件**: 本人の画面が衛星の資格情報を `Authorization` に付けて送る正当な場面ができたとき（ST27 の MCP の設定画面など）

### D19（仮）. `ashsat_` で始まる値が引けない求めは、ログインの印を見ずに 401

- 取り消した 1 本・外した衛星の 1 本・知らない値のどれでも、衛星の値を名乗る求めは印で通さない（厳しい側。有効な値 + 印を 403 にする D18 と揃える）
- `ashsat_` で始まらない `Bearer` の扱いは変えない（ST44 の範囲）
- **反転条件**: 本人の画面が、取り消した衛星の値を `Authorization` に付けたまま正当に動く場面ができたとき（D18 の反転条件と同じ場面）

### D20（仮）. 衛星の登録と資格情報の発行は別の操作。登録しただけでは資格情報は 0 本

- PERM-14 は「利用者が衛星の資格情報を発行する」を登録と別の操作として書いている。登録で 1 本を自動で発行すると、値を 1 回だけ見せる場面が登録の応答にも要り、名前の付け方も決まらない
- Story の完了の判定「衛星を登録すると、その衛星の名前の付いた資格情報が発行される」は、登録した衛星を選んで発行でき、発行した 1 本にその衛星の名前が付くことで満たす（proposal の「完了の判定の読み方」。`review/spec.md` R15）
- **反転条件**: 本人が確認バッチで「登録したら 1 本出てほしい」と返したとき（登録の応答に 1 本目の値を載せる。行の形は変わらない）

### D21（仮）. 正典の書き出しのファイルの一覧の書き漏れ（ST17 / ST21 のファイル）を、同じ Scenario の MODIFIED で一緒に直す

- ST29 は「台帳と導出の表は書き出しに入らない」の一覧（「それ以外のファイルは無い」）に衛星の登録簿を足すので、同じ Scenario を写す。正典の一覧が今の実装（`export_file_tests.rs` の `ENTRIES`）より少ないまま写すと、
  archive の後も正典と実装が食い違う
- ST29 の決定（Q / C）ではない訂正なので proposal の What Changes に 1 行書いた（`review/spec.md` R17）
- **反転条件**: 走っている別の change が同じ Scenario を MODIFIED して、一覧の直し方がぶつかったとき（後から archive する側が、先の一覧に衛星の登録簿だけを足す形に戻す）

## Risks / Trade-offs

- [衛星の求めごとに `last_used_at` を UPDATE する] → 衛星はまだ何も読めないので頻度は低い。ST45 で読み始めて重くなれば、1 分単位に丸めて同じ値なら書かない形にする
- [`ashsat_` で始まらない違う `Bearer` + 有効な印は、今までどおり印で通る] → 衛星の値でなければ ST29 は触らない。厳しくするなら ST44 が収集アプリの合言葉と一緒に決める
- [全 route の試験が契約（OpenAPI）に依る] → 契約に載らない route は数の突き合わせで落ちる（D14）。契約に載せ忘れた口は `tools/check-openapi.sh` とこの試験の両方で見つかる
- [Q1 が推奨と違う答えになる] → 識別子の形（D5・D7 の CHECK）・資格情報の値の形（D6）・登録の画面の欄の意味（D12）を書き直す。読み出しの記録には識別子を入れない（D4）ので、書き換えられない表は書き直さずに済む
- [`ashsat_` の値がログに出る] → 値は応答に 1 回載るだけ。`tools/check-log-private.sh` は書き方の静的な検査で実行時のログを見ないので、縦串（tasks 10.2）の終わりにサーバのログをパターン `ashsat_[a-z0-9.-]+_[A-Za-z0-9_-]{43}` で探して 0 件を確かめる

## Migration Plan

- 移行 1 本（D7 の 2 表 + 最初の 2 つ + D9 の `ALTER`）。当て直せる形。`.down.sql` は 2 表を落とし、`access_log` の欄を落とし CHECK を元の値に戻す
  （`satellite` / `forbidden` の行が既にあると元の CHECK を付けられないので、戻しはその行が無いときだけ当たる。戻し手順に注記する）
- 既存の利用者には何も要らない（最初の 2 つは移行が入れる。資格情報は本人が画面で発行する）

## Open Questions

- 無い（Q1〜Q8 の本人の答えで推奨と違うものが出たら、その問いだけを `spec --answers` で写す）
