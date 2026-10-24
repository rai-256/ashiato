# ST29 実装タスク — 衛星を登録すると、衛星ごとの資格情報が発行される（まだ何も読み書きできない）

読む順: `deep.md`（**最優先。第 1 回の Q1〜Q8 と、聞かずに決めた C1〜C15**）→ このファイル →
`specs/satellite-platform/spec.md` → `specs/data-sensitivity/spec.md` → `specs/data-durability/spec.md` → `design.md` →
`docs/stories/ST29.md` → `docs/handoff/ST29.md`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D7 / D9）。名前は作成時刻 `YYYYMMDDHHMM_satellite_credentials.sql`（連番にしない）で、`migrations/ORDER` の末尾へ 1 行。
**`authorize` が `ApiToken` と `WebSession` を通す振る舞い・既存の口の応答・`record-envelope` の要件には触らない**（ST44 の範囲。design の Non-Goals）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
`cargo test` は数十秒以上かかるので、repo の根から `scripts/quiet-run` で走らせ、1 回で待つ（`AGENTS.md`）。

## Global Constraints（規律。**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 取り消した資格情報は次の求めから使えない`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。印の名前は spec の `#### Scenario:` と一字一句合わせる
- **この change が持つ Scenario は 61 本**（`satellite-platform` 44 本 —— サーバ 36 本は Rust と bash、画面 8 本は `web/e2e`。`data-sensitivity` 9 本・`data-durability` 8 本は Rust）。
  MODIFIED の Requirement のうち既にある Scenario（`記録の読み出しで読み出しの記録に 1 行増える` など）は既存のテストの印のまま。中身を変えた Scenario（`読み出しの記録には中身も合言葉も残らない`・`台帳と導出の表は書き出しに入らない`）は既存の印で緑になるので、**直したことが落ちる形の終了条件**を 3.2・7.1 に置いた
- **件数つき検証**: このファイルで **`CT <絞り込み> <N>`** と書いたものは、repo の根で次が rc=0 になることを指す（`<N>` はテストの本数の下限）:
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && n=$(grep -Eo 'test result: ok\. [0-9]+ passed' /tmp/ct.log | awk '{s+=$4} END{print s+0}') && [ "$n" -ge <N> ]`
  （`cargo test` の絞り込みは 1 つずつ。**Scenario 1 本にテスト 1 本以上**。走らせるときは `scripts/quiet-run unit -- bash -o pipefail -c '…'` で包む）
- **e2e も件数で確かめる**: `ET <grep> <N>` は `cd web && npx playwright test settings-satellites --grep '<grep>' --list` の件数が `<N>` 以上で、かつ `npm run test:e2e -- settings-satellites --grep '<grep>'` が rc=0
- **衛星の判定は最も外側の層で 1 回だけ**（design D8）。`access_log::middleware` が衛星をハンドラの前で 403 にし、`authorize` も `Satellite` を 403 にする。
  **新しい口のハンドラに衛星の分岐を書かない**（足し忘れても閉じている形を崩さない）
- **全 route の試験に route を手で列挙しない**（design D14）。契約（`ApiDoc::openapi()`）から取り、`lib.rs` から複数行にまたがる `.route(` の path の集合と突き合わせる
- **資格情報の値は応答に 1 回だけ**（D6）。DB・ログ・読み出しの記録・画面の保存領域に置かない。ハッシュは値全体の SHA-256
- **識別子と名前の形の検査は `satellite.rs` の 1 か所**（D5）。口・画面・移行の CHECK で同じ正規表現を書くときは、`satellite.rs` の定数を正本とし、移行の CHECK と画面の文言はそれに合わせたことをテストで確かめる（2.2）
- **読み出しの記録に入れるのは衛星と資格情報の番号（uuid）**で、識別子の文字列ではない（D4）
- **登録簿と資格情報の行は消さない**（D1 / D7）。取り消しと外すは時刻を付けるだけ
- **衛星の試験は識別子に試験ごとの接尾を付けて隔離する**（`dev.t<乱数>.booklog`）。最初の 2 つを外す試験は 409 だけを見て状態を変えない（D14）
- **D1〜D4 と D15〜D21 は（仮）決め。** 反転条件は `design.md`。D1〜D4 は台帳（`provisional.jsonl`）に載っている —— 変えたら見出しと台帳を合わせる
- **Q1〜Q8 は第 1 回の深掘りの問い。この文書は推奨の答えで先に書いてある。** 本人の答えが推奨と違えば、上流がこのファイルごと直してから下流に渡す。下流は変えない
- 資格情報の値・合言葉・印を**ログに出さない**（製造準備 A-2）。出すのは種別・衛星の番号・件数だけ

## Task 1: 移行 —— 衛星の登録簿と資格情報、読み出しの記録の欄（design D7 / D9）

- [x] 1.1 移行 `migrations/YYYYMMDDHHMM_satellite_credentials.sql` と `.down.sql` を足し、`migrations/ORDER` の末尾へ 1 行 ——
  `core.satellite` と `core.satellite_credential`（D7 の列と CHECK・部分一意索引）、最初の 2 つ（`ashiato.ai-external`「外部 AI」・`ashiato.ai-local`「ローカル AI」。固定の uuid・`builtin = true`・`ON CONFLICT (identifier) DO NOTHING`）、
  2 表の DELETE と TRUNCATE を拒むトリガ（**この移行専用の関数** `core.reject_satellite_delete()`）、`core.access_log` の欄 2 つと CHECK 3 つ（D9。制約の既定の名前は `\d core.access_log` で確かめてから書く）。
  すべて当て直せる形（`IF NOT EXISTS` / `DROP … IF EXISTS` → `ADD`）。`.down.sql` には「衛星の行が読み出しの記録にあると元の CHECK を付けられない」注記を書く。
  Scenario: `移行を当て直しても最初の 2 つは増えない`・`移行は最初の 2 つに資格情報を入れない`（試験の中で作った空の schema に全版を当てて、資格情報の行が 0。design D14）。
  （`最初から外部 AI とローカル AI が登録されている` の印は口から確かめる 4.3 にだけ置く）
  検証: `tools/check-migrations.sh` rc=0、`CT satellite_migration 2`
- [x] 1.2 `tools/check-immutable.d/<作成時刻>_st29-satellite-credentials.sh` を足す（`tools/check-immutable.sh` の本体に書き足さない）——
  (a) `core.satellite` と `core.satellite_credential` への DELETE / TRUNCATE が psql から拒まれ、UPDATE は通ること、
  (b) 衛星の値を持つ `core.access_log` の行への UPDATE / DELETE が拒まれること、
  (c) `…_satellite_credentials.down.sql` を戻しの逆順の先頭で当て（衛星の行が無い DB で）、当て直せること。
  Scenario: `読み出しの記録の行は書き換えも削除もできない`（既存の印に加えて、衛星の行で）・`外した衛星と取り消した資格情報の行は消えずに残る`（(a) を、衛星を外し資格情報を取り消した後の行で試し、時刻の列が埋まっていることも見る）。
  検証: `bash -o pipefail -c 'tools/check-immutable.sh | tee /tmp/ci.log' && grep -q satellite /tmp/ci.log`

## Task 2: 識別子と名前の形（design D2 / D5 / D15。`crates/server/src/satellite.rs`）

- [x] 2.1 `satellite.rs` に `check_identifier`（拒む種別 `malformed` / `reserved` / `body_prefix`）・`name_owner`・名前（前後の空白を除いてから）と資格情報の名前の長さの検査を純粋関数で置く。
  単体テストは Scenario `形に合わない識別子の衛星は登録されない`・`本体が使う識別子の衛星は登録されない`・`本体の前置きの形で始まる識別子の衛星は登録されない` の値をそのまま表で持つ
  （`booklog`・`Dev.example.booklog`・`dev..booklog`・`dev.example.`・`dev_example.booklog`・`dev.example:booklog`・65 文字・`ashiato.booklog`・`ashiato.ai-external`・`c03-takeout.example`・`s01-x.example` を断り、`dev.example.booklog` を通す）。
  `name_owner` は Scenario `名前の持ち主は区切りより前の識別子で決まる` の値を表で持つ（`dev.example.booklog:reading` → `Some("dev.example.booklog")`、`dev.example.booklog:`・名前の部分 65 文字・`dev.example.booklog:Reading`・`c03-takeout`・`x:reading` → `None`）。
  Scenario: `名前の持ち主は区切りより前の識別子で決まる`。
  検証: `CT satellite::tests 5`
- [x] 2.2 形の正本の突き合わせ: 移行の CHECK の正規表現と `satellite.rs` の定数が同じであることを、`MIGRATIONS` の SQL の字面から取り出して比べるテストを置く（片方だけ直すと落ちる）。
  あわせて論理ソースの登録簿を全部読み、`:` を含む名前が 0 個であることを確かめる。Scenario: `本体の論理ソースの名前は区切りを含まない`。
  検証: `CT satellite_form 2`

## Task 3: 判定と読み出しの記録（design D4 / D6 / D8 / D9 / D18。`lib.rs`・`web_session.rs`・`access_log.rs`）

- [x] 3.1 `Caller::Satellite { satellite, credential }` を足し、`authorize_quiet` が `ashsat_` で始まる `Bearer` の値を SHA-256 で引く。
  有効なら同じ文で `last_used_at` を `app.now()` にして返す（D8 の `UPDATE … FROM … RETURNING`）。`ashsat_` で始まる値が引けなければ（取り消した 1 本・外した衛星の 1 本）印を見ずに 401（D19（仮））。
  **この段では Task 4 / 5 の口がまだ無いので、衛星と資格情報の行は試験の中で SQL で直に入れ、`last_used_at` も表を読んで確かめる**（一覧の口からの確かめは 5.1 で足す）。
  `authorize` は `Satellite` を 403（本文 `forbidden`）にする。`refresh_cookie` は衛星の求めに印を出し直さない。
  Scenario: `ログインの印を併せて持っていても衛星の求めとして断る`（`Set-Cookie` が無いことも見る）・`取り消した資格情報はログインの印を併せて持っていても通らない`・`衛星の資格情報は日が経っても切れない`（時計を 400 日進める）。
  検証: `CT satellite_authn 3`
- [x] 3.2 `access_log::middleware`: 判定が `Satellite` なら route にかかわらず（取り込みの口も）資格情報の種類 `satellite`・衛星と 1 本の番号・結果 `forbidden`・403 の行を書き、ハンドラを呼ばずに 403 を返す。
  `AccessEntry` は `AccessEntry::for_caller` で作り、`satellite` の欄は `Caller::Satellite` からだけ埋まる形にする。`GET /session` も 403 と同じ行（D8）。
  書き換えられない表の行の形を固定した値で試す（衛星の行を 1 行書いて全列を読み、期待の値と比べる。D9）。
  衛星と資格情報の行は 3.1 と同じく SQL で直に入れる（2 本の資格情報も直に 2 行）。
  Scenario: `衛星の資格情報で断った求めは、衛星と 1 本の番号つきで残る`（行の全列に識別子の文字列も値も無いことも見る）・`衛星の資格情報で取り込みの口に送った求めも残る`・`読み出しの記録には中身も合言葉も残らない`（既存のテストに衛星の求めを足す）・
  `衛星の資格情報で送った記録は取り込まれない`・`衛星の資格情報ではログインの状態も読めない`。
  検証: `CT access_log 9`（既存の読み出しの記録のテストが通る）・`CT satellite_access 5`（衛星の 5 本は名前に `satellite_access` を含める）・
  `CT access_log_secrets_satellite 1`（`読み出しの記録には中身も合言葉も残らない` に衛星の求めを足したテストの名前。直したことが落ちる形）

## Task 4: 衛星の登録と外す（design D7 / D11 / D16。`GET` `POST /satellites`・`DELETE /satellites/{id}`）

- [x] 4.1 3 本の口を足し、utoipa で契約に載せる。どれも `WebSession` だけ（`ApiToken` と `Satellite` は 403、無しは 401）。
  登録: 名前の前後の空白を除く → `check_identifier` → 外していない同じ識別子は 409・外した同じ識別子は同じ行を戻す（名前を求めの名前に。`registered_at` は変えない。200）・名前の重なりは部分一意索引の違反を 409 に。断りの本文は D11 の種別。
  外す: 最初の 2 つと外した衛星は 409。外すと `removed_at` と、その衛星の取り消していない資格情報すべてに `revoked_at` を同じトランザクションで付ける。
  Scenario: `形に合う識別子で衛星を登録できる`・`形に合わない識別子の衛星は登録されない`・`本体が使う識別子の衛星は登録されない`・`本体の前置きの形で始まる識別子の衛星は登録されない`・
  `64 文字の識別子の衛星は登録できる`・`登録している衛星と同じ識別子の衛星は登録されない`・`登録している衛星と同じ名前の衛星は登録されない`・`名前が空白だけか 41 文字の衛星は登録されない`・`最初の AI の 2 つは外せない`。
  検証: `CT satellite_register 9`
- [x] 4.2 権限の口ごとのテスト。Scenario: `API の合言葉では衛星を登録できない`（一覧・登録・発行・取り消し・外すの 5 つ。発行と取り消しは Task 5 の口が入ってから足す）・`資格情報が無いと衛星の一覧も読めない`。
  検証: `CT satellite_admin_only 2`
- [x] 4.3 口から最初の 2 つが見えることを確かめる。Scenario: `最初から外部 AI とローカル AI が登録されている`（画面のログインで `GET /satellites`）。
  検証: `CT satellite_builtin 1`

## Task 5: 資格情報の発行と取り消し（design D1 / D6 / D11。`POST /satellites/{id}/credentials`・`DELETE /satellites/{id}/credentials/{cid}`）

- [x] 5.1 発行: `OsRng` の 32 バイト → `URL_SAFE_NO_PAD` → `ashsat_<識別子>_<43 文字>`。SHA-256 だけを保存し、値は 201 の応答に 1 回だけ載せる。外した衛星は 409。名前は 40 文字まで（空は可）。
  一覧（`GET /satellites`）の資格情報の行は `id` `label` `issued_at` `last_used_at` だけ。
  Scenario: `発行すると、衛星の識別子の入った値が 1 回だけ返る`・`発行した値は二度と読み出せない`（一覧と表の全列に値も乱数の部分も無い）・`1 つの衛星に 2 本の資格情報を発行できる`・`外した衛星には資格情報を発行できない`・
  `41 文字の名前の資格情報は発行されない`・`使われた資格情報には最後に使われた時刻が残る`（発行した値で記録の読み出しを求め、一覧で確かめる）。
  検証: `CT satellite_credential_issue 6`
- [x] 5.2 取り消し: `revoked_at` を付け、一覧から外す。取り消した 1 本は 401（次の求めから）、同じ衛星のほかの 1 本は 403 のまま。
  Scenario: `取り消した資格情報は次の求めから使えない`・`衛星を外すとその衛星の資格情報が全部止まる`・`外した衛星を同じ識別子で登録し直すと前の衛星の続きになる`・`最初の AI にも資格情報を発行して取り消せる`（同じ試験の中で取り消して閉じる）。
  4.2 のテストに発行と取り消しを足す。
  検証: `CT satellite_credential_revoke 4`・`CT satellite_admin_only 2`
- [x] 5.3 資格情報の値が実行時のログに出ないことは、縦串（10.2）の終わりで確かめる（`tools/check-log-private.sh` は書き方の静的な検査で、実行時のログを見ない。design の Risks）。
  この段では、発行・取り消しの口と `authorize_quiet` の `tracing` の呼び出しに値・ハッシュを渡していないことを `tools/check-log-private.sh` が通ることで見る。
  検証: `tools/check-log-private.sh` rc=0

## Task 6: 全 route の試験（design D14。完了の判定「衛星の資格情報では、まだどの口も記録を返さず受け取らない」）

- [x] 6.1 `ApiDoc::openapi()` の paths と method を全部取り、path の `{…}` を形の合う値で埋め、有効な衛星の資格情報を付けて 1 回ずつ呼ぶ。
  `/healthz`・`POST /session`・`DELETE /session` を除くすべてが 403・本文が固定の語で、読み出しの記録にその route の衛星の 403 の行が 1 行ずつ増えることを確かめる。
  `lib.rs` から `\.route\(\s*"([^"]+)"`（複数行にまたがる形。今の `lib.rs` では 27 の path が取れる —— `/feelings/reminder`・`/attributes/kinds/{id}/names`・`/session` は path が次の行にある）で取った path の集合と契約の path の集合が一致することも同じテストで確かめる
  （`/selftest/panic` は既定の構成に無いので除く。差があれば path を出して落ちる）。`lib.rs` に `.merge(` / `.nest(` / `.nest_service(` があれば落ちる（design D14）。
  Scenario: `衛星の資格情報では、資格情報の要らない口のほかに 2xx を返す口が無い`。
  検証: `CT every_route_refuses_satellite 1`。あわせて変異で落ちることを確かめる: middleware の衛星の 403 の分岐を外し `authorize` の `Satellite` を `Ok` にした変更を一時 commit（`git stash` は使わない）に置き、
  `scripts/quiet-run unit -- cargo test -p ashiato-server every_route_refuses_satellite` が rc≠0 になることを見てから `git reset --hard HEAD~1` で戻す。落ちたテストの名前を PR 本文に書く
- [x] 6.2 資格情報の要らない口の比較。`/healthz`・違う合言葉の `POST /session`・`DELETE /session` を、衛星の資格情報の有無でそれぞれ呼び、状態の番号と本文が同じ。
  Scenario: `資格情報の要らない口は衛星の資格情報の有無で応答が変わらない`。
  検証: `CT exempt_routes_ignore_satellite 1`

## Task 7: 書き出しの分類（design D13。`export.rs`）

- [x] 7.1 `export::CATALOG` に `satellite`（`Include`。全列。`satellites.jsonl`）と `satellite_credential`（`Exclude`）を足し、`export_file_tests.rs` の `ENTRIES` に `satellites.jsonl` を足す（15 → 16）。
  Scenario: `衛星の登録簿は外した衛星も含めて書き出しに出る`・`衛星の資格情報は書き出しに入らない`・`ST29 が足した表はすべて分類されている`・`台帳と導出の表は書き出しに入らない`（ファイルの一覧に衛星の登録簿）・
  `いまの記録の schema の表と列はすべて分類されている`（既存のテストが新しい 2 表を含めて通る）。
  検証: `CT export_satellite 3`・`CT export_ 60`（書き出しの既存のテストが全部通る。書き出しのテストは `EXPORT_SERIAL` を取る）・
  `grep -c '"satellites.jsonl"' crates/server/src/export_file_tests.rs` が 1 以上（`台帳と導出の表は書き出しに入らない` の一覧を直したことが落ちる形）

## Task 8: 本体が衛星のコードを読み込まない検査（design D10。FR-77）

- [x] 8.1 `tools/check-satellite-boundary.sh` を足す —— `cargo tree -p ashiato-server -e normal --target all --prefix none` の名前を禁じる部品の一覧と突き合わせ、
  `crates/server/src` に `dlopen` / `LoadLibrary` / `Library::new` の字面が無いことを grep で見る。`--self-test` は `libloading` と `wasmtime` を含む固定の一覧を食わせ、rc≠0 で 2 つの名前を出すことを確かめる。
  CI の検査の job に足し、`docs/testing.md` の表に 1 行足す。
  Scenario: `本体の依存に衛星のコードを読み込む部品は無い`・`読み込む部品が依存に入ると検査が落ちる`（bash の `echo` で印）。
  検証: `tools/check-satellite-boundary.sh` rc=0、`tools/check-satellite-boundary.sh --self-test` rc=0（中で落ちることを確かめる）

## Task 9: 画面（design D12。`web/src/SatellitesPanel.tsx`・`satellites.ts`・`messages.ts`・`SettingsView.tsx`）と e2e

- [x] 9.1 `SatellitesPanel.tsx` を書き、`SettingsView.tsx` の書き出しの節の前に置く。API の型は `satellites.ts`、文言と拒む種別の日本語は `messages.ts`。
  発行した値は state にだけ持ち、閉じたら捨てる。発行の欄・外すの確認・登録の欄は小節の中にその場で開く（同時に 1 つ）。
  jsdom では指定と勘定だけを見る（拒む種別の全部に日本語の理由があること・値が `localStorage` / `sessionStorage` に書かれないこと）。
  検証: `cd web && npm run lint && npm run test -- satellites` rc=0
- [x] 9.2 `web/e2e/settings-satellites.spec.ts`（本物の Chromium）。衛星 5 つ（最初の 2 つ + 3 つ）・資格情報 0〜2 本ずつを API で積んでから操作する。
  Scenario: `設定画面に衛星と資格情報の節が出る`・`画面から資格情報を発行すると値がその場に 1 回だけ出る`・`画面から資格情報を取り消すと行が消える`・
  `画面から衛星を外すと確認をはさんでから外れる`・`確認で「やめる」を押すと衛星は外れない`・`画面から衛星を登録できる`・`画面で形に合わない識別子を入れると理由が出て登録されない`・
  `衛星と資格情報の節は触れる大きさと幅の下限を守る`（360 px の幅で `getBoundingClientRect` の幅と高さ・`scrollWidth <= clientWidth`・Tab で移ったフォーカスの `outline` の幅）。
  アサートは数値と経路（実寸・可視・入力欄の下の文言・行の有無・値の有無）。スクリーンショット比較は使わない。
  取り消しと外すは、画面の操作の後に同じ値で `GET /events` を呼んで 401 を確かめる。
  検証: `ET 'Scenario' 8`（`settings-satellites` の 8 本以上）

## Task 10: OpenAPI・縦串・仕上げ

- [ ] 10.1 `cargo run -p ashiato-server --bin openapi > docs/openapi.json` で契約を作り直す。検証: `tools/check-openapi.sh` rc=0
- [ ] 10.2 縦串 `tools/smoke.d/<作成時刻>_st29-satellite-credentials.sh`（`tools/smoke.sh` の本体に書き足さない）——
  立てたスタックで画面のログイン → 衛星を登録 → 資格情報を発行 → その値で `GET /events` と `POST /ingest` が 403 → 取り消し → 401 を `echo` で印を付けて確かめる。
  終わりにサーバのログ（`docker compose logs` かスタックのログのファイル）を `ashsat_[a-z0-9.-]+_[A-Za-z0-9_-]{43}` と発行した値そのもので grep し、0 件でなければ落とす。
  検証: `tools/smoke.sh` rc=0
- [ ] 10.3 検査 3 本。検証: `python3 scripts/check_scenarios.py . st29-satellite-credentials` rc=0、`python3 scripts/review_triage.py . st29-satellite-credentials` rc=0、`python3 scripts/check_chain.py .` rc=0
- [ ] 10.4 まとめて緑。検証: `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` /
  `tools/db.sh down -v && cd web && npm run lint && npm run test && npm run build && STACK_RESET=1 npm run test:e2e` /
  `tools/check-immutable.sh` / `tools/check-log-private.sh` / `tools/check-satellite-boundary.sh` / `tools/check-openapi.sh` / `tools/smoke.sh` が全部 rc=0
- [ ] 10.5 `docs/handoff/ST29.md` を PR の前にもう一度読む（開始時と合わせて 2 回）。st17 C2 と st33 C4 が 6.1 の全 route の試験で閉じていること、
  st28 Q4 と st17 R5 が ST44 へ移っていること（ST29 では `ApiToken` の範囲を変えていない）を確かめる。
  検証: `grep -c 'Scenario: 衛星の資格情報では、資格情報の要らない口のほかに 2xx を返す口が無い' crates/server/src/*.rs` の合計が 1 以上、かつ `git diff main -- crates/server/src/lib.rs | grep -c '^-[^-].*Caller::ApiToken'` が 0

## 人間の確認待ち

**無し。** 画面の Scenario は 9.2 の e2e（本物のブラウザ）が担保し、サーバの Scenario は Rust のテストと bash の検査が担保する。
機械が再現できない物理（ロック・電池・本物の GPS・時間そのもの・実機）に当たるものが、この Story には無い。
AI の MCP クライアントに資格情報を設定できるか（`premises.md` P6）は ST27 が確かめる。
確認バッチの手順書が Story ごとに 1 問聞く「触ってみて違和感は無かったか」だけが人間に渡る。
