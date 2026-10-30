# ST50 実装タスク — 収集アプリとサーバの版が食い違っても記録が失われない

読む順: `deep.md`（**本人の答え Q1〜Q9**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/record-envelope/spec.md` → `specs/device-collection/spec.md` →
`specs/desktop-collection/spec.md` → `specs/data-durability/spec.md` → `design.md` → `premises.md` → `docs/collector-contract.md` → `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin はコメント（`// Scenario: 記録は 1 件ごとの契約の版を持つ`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 80 本**（`record-envelope` 42・`device-collection` 21・`desktop-collection` 15・`data-durability` 2）。**全部 ST50 で新しく足した ADDED で、全部このファイルのどこかの Task が名指しする**
- **件数つき検証**: このファイルで
  - **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
    `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` がその項目に書いた本数以上
  - **`CW <名前>`** は `scripts/quiet-run cw -- cargo test -p ashiato-collector-windows --lib <名前>` rc=0 かつ
    `cargo test -p ashiato-collector-windows --lib <名前> -- --list 2>/dev/null | grep -c ': test$'` がその項目に書いた本数以上（ubuntu で走る `#[cfg(test)]`）
  - **`GT <クラス>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*<クラス>*'` rc=0 かつ
    `cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*<クラス>*.xml | grep -c '<testcase '` がその本数以上
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので本数の下限を必ず見る）
- **試験の名前の接頭辞は `st50_`**。S-01 の DB の試験は `crates/server/src/contract_version_tests.rs`（Task 1〜5）と `crates/server/src/export_contract_version_tests.rs`（Task 6）に置き、
  `lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;` を足す。C-02 の試験は各モジュールの `#[cfg(test)] mod tests` の `st50_` の関数、C-01 は `collector-android/app/src/test/kotlin/dev/ashiato/collector/St50*Test.kt`
- **本人の決定（下流は変えない）**: Q1 記録は作った時の契約の版を溜めの 1 行ごとに持ち、記録に残す / Q2 S-01 が知らない欄の 1 件は `unknown_field` / Q3 収集アプリが送るもの全部（6 つの口）を同じ版で止める /
  Q4 `GET /contract` で確かめ、要求の見出しにも版を載せ、426 で断る / Q5 PC の通知は止まった時に 1 回・1 日 1 回 / Q6 携帯は画面と通知（同じ頻度）/ Q7 欄の追加・削除・型・必須を S-01 の型と収集アプリ 2 本で落とす /
  Q8 ベータ版の印も配った印 / Q9 記録ごとに止める。仮: D13（PC に上限を足さない）・D18（確かめは契機ごと・覚えない）・D19（外部の更新で版が動く）。
  （第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- **名前は統一する**: 見出し `Ashiato-Contract-Version`、確かめる口 `GET /contract` の答え `{"contract_version": N}`、426 の本文 `{"error": "contract_unsupported", "contract_version": N}`、
  1 件の欄と DB の列と書き出しの列 `contract_version`、断りの種別 `unknown_field` / `contract_unsupported`、定数 `CONTRACT_VERSION`（S-01 `crates/server/src/contract_version.rs`・C-02 `crates/collector-windows/src/contract.rs`・C-01 `IngestRequest.kt`）。
  今の版は **1**（D8。最初の配った印までは版を上げない）
- **426 と `unknown_field` と `contract_unsupported` は捨てない側**（D17）。更新を促すのは「S-01 の更新を待っている」間だけで、200 の読めない答えは 24 時間続いてから促す（D9 の（e）・D11・D22・D24（仮）。数え始めは再起動をまたいで持ち、読める答えでだけ数え直す）。携帯の `PERMANENT_ERRORS` の 6 種を変えない。`deny_unknown_fields` を使わない（D2）
- **画面のログイン・衛星の要求に見出しの検査・知らない欄の検査・記録の版を掛けない**（D16）。`GET /outages` など読み出しの口は止めない（D3）
- **内容の鍵（`content_hash_of`）・原文の形を変えない**（契約の版は鍵に入れない）。`hash_is_pinned` の固定値を変える変更は落とす
- **移行は 1 つ・名前は作成時刻**（`migrations/<YYYYMMDDHHMM>_contract_version.sql` と `.down.sql`、`migrations/ORDER` の末尾へ 1 行）。名前の時刻は `202610250000` 以後で ST48 の規則に従う（D20）。
  門と錠の関数・読み口は**移行を当てる時点の本体を写して列を足すだけ**にし、ほかの条件を変えない（ST49 が先に main に入っていれば版の列を含む本体を写す）
- **走っている ST49 と同じ場所を触る**（proposal の Impact）。後から main に入る側が rebase で前の側の欄と列を写す。ST49 の `client_version` は版 1 の欄（知らない欄の一覧は型から作るので、型が持てば `unknown_field` にならない）
- 記録の値（原文・時刻・TZ）をログに出さない。版の値と断りの種別はログに出してよい（私的データではない）
- 共有の台本（`tools/smoke.sh`・`tools/check-immutable.sh`）の本体に書き足さない。ST50 の節は `tools/smoke.d/`・`tools/check-immutable.d/` の新しいファイルに置く

## Task 1: S-01 の契約の版と確かめる口 `GET /contract`

- [x] 1.1 `crates/server/src/contract_version.rs` を作る: `pub const CONTRACT_VERSION: u32 = 1;`、`App` に `contract_version: u32`（既定は定数。試験だけが別の値で作れる。設定や環境変数で変えられる口は作らない。D4）、
  見出し `Ashiato-Contract-Version` を読む関数（無ければ 1、正の整数でなければ「読めない」）。`router` に `GET /contract` を足し、`Caller::ApiToken` だけに `200 {"contract_version": N}` を返す（合言葉なし・不一致は 401、画面のログインと衛星は 403。D15）。
  `bin/openapi.rs` に載せて `docs/openapi.json` を作り直す。読み出しの記録は今の規則のまま 1 回 1 行（D23（仮）。`access_log.rs` を変えない）。`contract_version_tests.rs`:
  - `st50_contract_answers_version` — `// Scenario: 問い合わせの口は受け付ける一番新しい契約の版を答える`（試験の App を版 3 で作る）
  - `st50_contract_requires_token` — `// Scenario: 合言葉の無い問い合わせは断られる`
  - `st50_contract_web_session_403` — `// Scenario: 画面のログインで問い合わせると 403 で断られる`（衛星の資格情報でも 403）
  - `st50_contract_access_logged`（合言葉が認められた問い合わせで読み出しの記録が 1 行増えること。D23（仮）の固定）
  既存の `access_log_tests` が通ること。
  検証: `CT st50_contract_` rc=0（`-- --list` で 4 本以上）/ `CT access_log` rc=0（`-- --list` で 9 本以上）/ `./tools/check-openapi.sh` rc=0

## Task 2: 見出しの検査と 426（6 つの口）

- [x] 2.1 `POST /ingest`・`/heartbeat`・`/drops`・`/media/missing`、`PUT` / `HEAD /media/originals/{sha256}` の入口で、資格情報の判定の直後・本文の解釈の前に、`Caller::ApiToken` の要求の見出しを読む。
  版 > `app.contract_version` か読めない → `426` と `{"error": "contract_unsupported", "contract_version": N}`、何も格納しない（D4）。画面のログインには掛けない（D16）。`contract_version_tests.rs`:
  - `st50_header_ingest_426` — `// Scenario: S-01 より新しい版を名乗る取り込みは 426 で断られ、何も入らない`
  - `st50_header_other_mouths_426` — `// Scenario: 生存信号・破棄の報告・原本の欠けも新しすぎる版では 426 で断られる`
  - `st50_header_original_426` — `// Scenario: 新しすぎる版を名乗る原本は置かれない`（D-02 の置き場を `$TMPDIR` の下に。`PUT`）
  - `st50_header_original_head_426` — `// Scenario: 新しすぎる版を名乗る原本の有無の問い合わせは 426 だけを返す`（`HEAD`。本文が空）
  - `st50_header_absent_is_v1` — `// Scenario: 見出しの無い要求は版 1 として今どおり受け付けられる`（試験の App を版 2 で作る）
  - `st50_header_at_or_below_accepted` — `// Scenario: S-01 以下の版を名乗る要求は受け付けられる`
  - `st50_header_unreadable_426` — `// Scenario: 読めない版の見出しは 426 で断られる`（`abc`・`0`・`-1`・空）
  - `st50_header_web_session_exempt` — `// Scenario: 画面のログインの要求には版の検査が掛からない`
  検証: `CT st50_header_` rc=0（`-- --list` で 8 本以上）

## Task 3: 知らない欄 `unknown_field` と 1 件の `contract_version`（4 つの型）

- [x] 3.1 `IngestRequest`・`HeartbeatRequest`・`DropReportRequest`・`MediaMissingRequest` に任意の欄 `contract_version: Option<i64>` を足す。`contract_version.rs` に、型が知る最上位の欄の一覧（型から作る。`utoipa` の schema か `serde` の field の名前。手で並べない）と、
  1 件の JSON の最上位の鍵を比べる関数を置く。`Caller::ApiToken` の要求だけ、1 件ごとの解釈（`serde_json::from_value`）の**前**に見て、知らない鍵があれば `unknown_field`（D2）。
  1 件の `contract_version` が `app.contract_version` より大きいか正の整数でなければ `contract_unsupported`（D4）。`IngestError` / 生存信号・破棄の報告・原本の欠けの断りの列挙に 2 つを足す。`docs/openapi.json` を作り直す。`contract_version_tests.rs`:
  - `st50_unknown_field_ingest` — `// Scenario: 知らない欄を持つ記録は unknown_field で断られ、ほかの記録は入る`（応答 200・3 件の結果）
  - `st50_unknown_field_other_types` — `// Scenario: 生存信号・破棄の報告・原本の欠けの知らない欄も unknown_field で断られる`
  - `st50_unknown_field_by_known_fields` — `// Scenario: 知らない欄の判定は S-01 が知る欄で行う`（試験の App を版 2・見出し 1・終わりの 3 欄つき）
  - `st50_unknown_field_payload_inner` — `// Scenario: payload の中の欄は知らない欄として断られない`
  - `st50_unknown_field_web_session` — `// Scenario: 画面のログインで届く知らない欄は今どおり読み飛ばされる`
  - `st50_item_version_other_types` — `// Scenario: S-01 より新しい版の生存信号・破棄の報告・原本の欠けの 1 件は contract_unsupported で断られる`（3 つの断りの列挙 `HeartbeatError`・`DropError`・`MediaMissingError` のそれぞれ）
  - `st50_unknown_field_not_malformed`（知らない欄の 1 件の `error` が `malformed` でないこと・`deny_unknown_fields` が型に無いことの固定）
  検証: `CT st50_unknown_field_` rc=0（`-- --list` で 6 本以上）/ `CT st50_item_version_` rc=0（`-- --list` で 1 本以上）/ `./tools/check-openapi.sh` rc=0
- [x] 3.2 `docs/collector-contract.md` に「## 送信契約の版（ST50）」の節を足す: 版の意味（スキーマ版と別・S-01 は自分以下を全部受け付ける）・見出し・`GET /contract`・426・`unknown_field` / `contract_unsupported` の行（返る形の表）・
  1 件の `contract_version`・入れ替えの順（design の Migration Plan）・「## 送信契約の版の履歴」の表（版 1 の行: 最初の印 未定・今の封筒の全部と最初の配った印までに足した欄・理由「最初の版」）。
  `PUT` の応答の表に `426` の行を足す。
  検証: `bash -c 'grep -q "^## 送信契約の版の履歴" docs/collector-contract.md && grep -q "contract_unsupported" docs/collector-contract.md && grep -q "unknown_field" docs/collector-contract.md && grep -q "Ashiato-Contract-Version" docs/collector-contract.md'` rc=0

## Task 4: 移行・記録の契約の版・前の版・門と錠

- [x] 4.1 `migrations/<作成時刻>_contract_version.sql` を design D20 のとおりに書く（`core.event` と `core.event_version` に `contract_version integer` と CHECK・`c01-*` / `c02-*` の埋め戻し・
  `core.event_live` と `core.event_version_live` の作り直し・`core.reject_version_rewrite` と `core.require_version_or_ledger` に列）。埋め戻しは D20 の手順（`event_version_immutable` と `event_requires_version` を `DISABLE TRIGGER` → `UPDATE` → `ENABLE TRIGGER` → 本体の置き換え）で行う。本体を置き換える前に、`202610231200_event_end_time.sql` と同じ形で置き換える前の本体の md5 を確かめる。
  `.down.sql` は逆順で今の形に戻し、`DELETE FROM migration_ledger.applied WHERE name = '<名前>'` を持つ。`migrations/ORDER` の末尾へ 1 行（前の行に `# ST50: 送信契約の版（記録と前の版の列・埋め戻し・読み口・門と前の版の錠）` の注記）。
  検証: `./tools/check-migrations.sh` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run build -- cargo build -p ashiato-server'` rc=0
- [x] 4.2 `ingest` が `authorize` の戻り値（`Caller`）を捨てずに、`ingest_one` / `ingest_request` へ「収集アプリの 1 件か」の印を渡す引数を足す（書庫の取り込み `store_with` と滞在・画面は偽。D1。ST23 の `Settle` と同じ署名に触るので、後から入る側が合わせる）。
  `ingest_one` の 2 つの INSERT に `contract_version`（収集アプリの 1 件は欄の値か 1、それ以外は NULL。D1・D10）、`apply_external_update` の本表の UPDATE と前の版の INSERT に `contract_version`（D19（仮））を足す。
  内容の鍵に入れない。`contract_version_tests.rs`:
  - `st50_record_version_per_item` — `// Scenario: 記録は 1 件ごとの契約の版を持つ`
  - `st50_record_version_default_one` — `// Scenario: 契約の版の欄が無い記録は版 1 になる`
  - `st50_record_version_too_new_item` — `// Scenario: S-01 より新しい版の 1 件は contract_unsupported で断られ、ほかは入る`
  - `st50_record_version_null_for_others` — `// Scenario: 収集アプリ以外が書いた記録の契約の版は空`（画面のログインの取り込み・`store_one` の書庫の取り込み・滞在の作り直し）
  - `st50_record_version_dedup` — `// Scenario: 契約の版だけが違う同じ記録は 1 件に畳まれる`
  - `st50_record_version_external_update` — `// Scenario: 外部サービス由来の更新で契約の版が動き、前の値は前の版に残る`
  - `st50_record_version_schema_independent` — `// Scenario: 契約の版はスキーマ版と別に動く`（試験の App を版 2）
  - `st50_record_version_not_from_header` — `// Scenario: 要求の見出しの版は記録の契約の版を決めない`
  検証: `CT st50_record_version_` rc=0（`-- --list` で 8 本以上）/ `CT hash_is_pinned` rc=0（`-- --list` で 1 本以上）
- [x] 4.3 移行と錠の試験（`contract_version_tests.rs`。移行の前後は `media_migration_tests.rs` の使い捨ての DB の形で、ST50 の前の版まで当てて行を置いてから ST50 の移行を当てる）:
  - `st50_migration_backfill` — `// Scenario: ST50 より前に収集アプリから入った記録は版 1 になる`（`c01-location`・`c02-window` とその前の版・`s01-feeling`）
  - `st50_lock_version_row` — `// Scenario: 前の版の契約の版は書き換えられない`
  - `st50_lock_collected_rewrite` — `// Scenario: 収集した記録の契約の版だけを前の版なしで書き換えると拒まれる`
  - `st50_migration_down_up`（`.down.sql` を当てて列が消え、版の記録の行が消え、もう一度当てても落ちない）
  `tools/check-immutable.d/<作成時刻>_st50-contract-version.sh` を `202610231200_st41-event-end-time.sh` の頭の形（schema を作り直し、付与を当て直す）で作り、
  `echo "== Scenario: 前の版の契約の版は書き換えられない"` と `echo "== Scenario: 収集した記録の契約の版だけを前の版なしで書き換えると拒まれる"` を、わざと UPDATE を投げて拒まれることで見る（変数は `ST50_`、関数は `st50_` を頭に）。
  埋め戻しで外した 2 つの錠が張り直されていること（`pg_trigger.tgenabled = 'O'`）も同じ節で見る。
  検証: `CT st50_migration_` rc=0（`-- --list` で 2 本以上）/ `CT st50_lock_` rc=0（`-- --list` で 2 本以上）/ `scripts/quiet-run immutable -- tools/check-immutable.sh` rc=0

## Task 5: 配った版の封筒の例（版 1）を受け付け続ける

- [x] 5.1 `crates/server/tests/fixtures/contract/v1/` に、今の収集アプリ 2 本が送る形の例を置く（`ingest-c01-location.json`・`ingest-c02-window.json`・`ingest-end-time.json`（終わりの 3 欄つき）・`heartbeat-c01.json`・`heartbeat-c02.json`・`drops.json`・`media-missing.json`。
  値は架空。`docs/collector-contract.md` の例と `CollectorContractFixture.kt` の形から作る）と、口の形の例 `wire.json`（`/contract` の道と答えの鍵・見出しの名前 `Ashiato-Contract-Version`・426 と本文の鍵。D4・D8）。`contract_version_tests.rs`:
  - `st50_released_v1_accepted` — `// Scenario: 版 1 の封筒の例は受け付けられる`（例を全部、見出しなしと見出し 1 で送り、版の検査と知らない欄の検査で断られないこと）
  - `st50_released_v1_end_time` — `// Scenario: ST41 の終わりの時刻を持つ記録は版 1 として受け付けられる`
  - `st50_released_v1_wire` — `// Scenario: S-01 は版 1 の口の形の例に従う`（`wire.json` を読んで、道・見出しの名前・答えの鍵・426 と本文の鍵を名指しせずに例から引いて当てる）
  検証: `CT st50_released_` rc=0（`-- --list` で 3 本以上）

## Task 6: 書き出しの列

- [ ] 6.1 `export.rs` の `EVENT_COLUMNS` と `EVENT_VERSION_COLUMNS` に `("contract_version", "収集アプリが記録を積んだ時の送信契約の版（スキーマ版とは別の番号。収集アプリ以外が書いた記録は空）")` を `schema_version` の次に足す（D20）。
  `export_contract_version_tests.rs`:
  - `st50_export_contract_version` — `// Scenario: 書き出しの記録と前の版の行は契約の版を持つ`
  - `st50_export_manual_describes` — `// Scenario: 説明書は契約の版の列を説明する`
  既存の「記録の行は記録の表のすべての列を持つ」の試験が通ること（列の一覧と表が一致する）。
  検証: `CT st50_export_` rc=0（`-- --list` で 2 本以上）/ `CT export_` rc=0（`-- --list` で 100 本以上）

## Task 7: 封筒の形の写しと版の上げ忘れの検査（CI）

- [ ] 7.1 収集アプリの直列化から欄を書き出す試験を足す（D7・D21）:
  C-01 `St50EnvelopeShapeTest.kt`（記録・生存信号・破棄の報告・原本の欠けの 4 つ。全部の欄を埋めた 1 件と任意の欄を空にした 1 件を本物の直列化で JSON にし、最上位の鍵・JSON の型・必須を出す。
  `ASHIATO_ENVELOPE_OUT` があればそのファイルへ書き、無ければ `docs/contract/envelope.json` の `android` と比べる）、C-02 `contract::tests::st50_envelope_shape`（記録と生存信号。同じ形で `windows` と比べる）。
  比べる関数は、写しと違う形を渡されたら差分（欄の名前）を出して落ちることを PC の `st50_envelope_shape_detects_drift`（`// Scenario: 写しがコードとずれていると CI が落ちる`。写しの `windows.ingest` から欄を 1 つ消した形を渡す）で固定する。
  PC と携帯の定数 `CONTRACT_VERSION`（D12。値 1）はここで足す（S-01 の定数は 1.1。8.1・10.1 はこの定数を使い、作らない）。
  この 2 つの試験の検証は、写しを作る 7.2 の後に走らせる（7.2 の検証に入れてある）
- [ ] 7.2 `tools/check-contract-version.sh` を作る（D7 の 1〜5。`--write` で `docs/contract/envelope.json` を作り直す。S-01 の側は `docs/openapi.json` の 4 つの型の `properties` と `required` から）。
  `--self-test` は `$TMPDIR` の使い捨ての git の木で次を作り、落ちる / 通るを確かめる:
  - `echo "== Scenario: 収集アプリだけが欄を足して版を上げないと CI が落ちる"`（印 `v0.1.0-beta.1` を打った後に `android.ingest` に欄を足す）
  - `echo "== Scenario: 既存の欄の型や必須を変えて版を上げないと CI が落ちる"`
  - `echo "== Scenario: 版を上げれば欄を足しても通る"`
  - `echo "== Scenario: 配った印が無い間は欄を足しても通る"`
  - `echo "== Scenario: 3 つの版の値が食い違うと CI が落ちる"`
  - 写しの S-01 の部分が `docs/openapi.json` とずれていると落ちること（収集アプリの部分のずれは 7.1 の試験が見る）
  - `echo "== Scenario: 配った版の封筒の例を書き換えると CI が落ちる"`
  - 印はあるのに写しを取り出せない・版の履歴の行が無い、の 2 つでも落ちること
  - `echo "== Scenario: 欄を消して版を上げないと CI が落ちる"`
  - `echo "== Scenario: ベータ版の印も配った印として比べる"`（印は `v0.1.0-beta.1` だけ）
  - `echo "== Scenario: 版の履歴に今の版の行が無いと CI が落ちる"`
  - `echo "== Scenario: 配った印の写しを取り出せないと CI が落ちる"`
  `docs/contract/envelope.json` を `--write` で作って commit する。CI の置き場は **rust の job の `./tools/check-released-migrations.sh` の 2 行の直後**（`.github/workflows/ci.yml:91-92`。その job が印を取り寄せている。D7）に
  `./tools/check-contract-version.sh --self-test` と `./tools/check-contract-version.sh` の 2 行。検査の本体は写しの S-01 の部分だけを `docs/openapi.json` から作り直して比べ（jq だけ）、収集アプリの部分のずれは 7.1 の試験がそれぞれの job で落とす。
  `docs/testing.md` の検査の表に 1 行足す。
  検証: `scripts/quiet-run ccv-self -- tools/check-contract-version.sh --self-test` rc=0 / `scripts/quiet-run ccv -- tools/check-contract-version.sh` rc=0 /
  `GT St50EnvelopeShapeTest` rc=0（1 本以上）/ `CW st50_envelope_shape` rc=0（`-- --list` で 2 本以上）

## Task 8: PC の収集アプリ —— 焼き込み・確かめ・版で選んで送る・426

- [ ] 8.1 `contract.rs` の `IngestRequest` と `HeartbeatRequest` に `contract_version: u32`（`#[serde(default = "one")]` で、版を持たない溜めの行を読み戻すと 1）を足し、溜めに積む 1 か所で 7.1 の `CONTRACT_VERSION` を焼き込む（D1・D10）。欄を足したら `tools/check-contract-version.sh --write` で写しを作り直す。
  `sender.rs` に `GET /contract` を引く口（`Transport` に `get` を足す）と、答えを「版 A / 受け付けない / 一時的な失敗」に読む関数（D11）、`flush` に「版 ≤ A の行だけ・見出しは載せた行の最大」（D9）と 426 の分岐（何も取り除かない・止まった状態）を足す。
  `runtime.rs` の送る契機の頭で 1 回確かめ、記録と生存信号の両方の `flush` に同じ答えを渡す（D18（仮））。「S-01 の更新を待っている」状態（D9 の a〜e。e は 200 の読めない答えが 24 時間続いた —— 数え始めの時刻を再起動をまたぐ手元のファイルに持つ。置き場はこの Task で作り、9.1 が同じファイルに通知の時刻を足す。D22・D24（仮））を持ち、抜けた契機で「飛ばす」印を捨てる（D14）。`sender.rs` の試験:
  - `st50_sender_sends_accepted_versions_only` — `// Scenario: PC は S-01 が受け付ける版の行だけを送り、新しい版の行を残す`（見出しの値も見る）
  - `st50_sender_burned_version_kept` — `// Scenario: PC は積んだ時の版を 1 行ごとに持ち、更新の後も変えない`（積んだ後に送る版の定数を差し替えられる形で）
  - `st50_sender_unversioned_row_is_v1` — `// Scenario: PC の版を持たない古い行は版 1 として送られる`（ST50 より前の形の JSONL の行を読み戻す）
  - `st50_sender_header_is_batch_max` — `// Scenario: PC の要求の見出しは載せた行の版の最大になる`
  - `st50_sender_no_contract_endpoint` — `// Scenario: 問い合わせの口が無い S-01 には PC も何も送らない`（404・本文が読めない・版が無い）
  - `st50_sender_426_removes_nothing` — `// Scenario: 426 で断られた PC の要求の行は 1 件も取り除かれない`
  - `st50_sender_never_drops_when_held` — `// Scenario: 契約の版で止まっている間も PC は行を捨てない`（100 日分の行を作り、何度 `flush` しても件数が減らない）
  - `st50_sender_resumes_after_update` — `// Scenario: S-01 を更新すると PC に残っていた記録がそのまま入る`（答えを 1 → 2 に変え、飛ばす印が捨てられることも見る）
  - `st50_sender_transient_is_not_held`（到達できない・401・503 で更新を待っている状態にならない）
  既存の `sender::` の試験（P5 の 11 本）が通ること。
  検証: `CW st50_sender_` rc=0（`-- --list` で 9 本以上）/ `CW sender::` rc=0（`-- --list` で 20 本以上）/ `scripts/quiet-run ccv -- tools/check-contract-version.sh` rc=0 / `CW st50_envelope_shape` rc=0（`-- --list` で 1 本以上）

## Task 9: PC の「サーバを更新してください」の通知

- [ ] 9.1 OS に依らない判定（止まった状態に入った・24 時間経った・抜けた → 出す / 出し直す / 消す。文言に日数。前に出した時刻を手元のファイルに持つ。D5。200 の読めない答えの数え始めの時刻も同じファイルに持ち、24 時間で（e）に入る・読める答えでだけ捨てる・（e）だけのときは網の確認を添えた文言。D22・D24（仮））を新しい `crates/collector-windows/src/contract_notice.rs` に置き、
  `toast.rs` に札 `contract` の通知と消す口を足す（`outage` の札に触らない）。試験（`contract_notice::tests`）:
  - `st50_notice_first_time` — `// Scenario: 契約の版で止まると PC に通知が 1 回出る`
  - `st50_notice_daily` — `// Scenario: PC の通知は止まっている間 1 日に 1 回出直す`（時計を差し替え、5 分ごとに 2 日進める。出したのは 3 回・日数 0・1・2）
  - `st50_notice_separate_tag` — `// Scenario: 更新を促す通知は途切れの通知を置き換えない`（偽の `Toaster` が受けた札が `contract` と `outage` で別）
  - `st50_notice_cleared` — `// Scenario: S-01 を更新すると PC の更新を促す通知が消える`
  - `st50_notice_not_for_transient` — `// Scenario: PC が到達できないだけでは更新を促さない`
  - `st50_notice_not_for_unreadable_200_under_24h` — `// Scenario: PC は読めない 200 の答えが 24 時間続くまでは更新を促さない`
  - `st50_notice_unreadable_200_after_24h` — `// Scenario: PC は読めない 200 の答えが 24 時間続くと網の確認を添えて更新を促す`（文言が「サーバを更新するか、つながっている網を確かめてください」で日数を含む）
  - `st50_notice_unreadable_200_survives_restart_and_transient` — `// Scenario: PC の再起動と到達できない間が挟まっても読めない 200 の数え始めは変わらない`（手元のファイルを残したまま判定を作り直す。到達できない区間と 503 の区間を挟む）
  - `st50_notice_unreadable_200_reset_by_readable` — `// Scenario: PC は読める答えを 1 回得ると読めない 200 の数え始めをやり直す`
  - `st50_notice_survives_restart` — `// Scenario: PC が再起動しても 24 時間経つまで通知は出直さない`（手元のファイルを残したまま判定を作り直す）
  検証: `CW st50_notice_` rc=0（`-- --list` で 10 本以上）/ `scripts/quiet-run clippy -- cargo clippy -p ashiato-collector-windows --all-targets -- -D warnings` rc=0
- [ ] 9.2 本物の Windows で札が 2 つ並ぶことを確かめる試験を `crates/collector-windows/tests/runtime_windows.rs` に足す（`st50_contract_toast_coexists_with_outage`。両方の札を出して `ToastNotificationManager::History` に 2 通あること、`contract` を消すと `outage` だけが残ること）。
  検証: `bash -c 'grep -c "fn st50_contract_toast_coexists_with_outage" crates/collector-windows/tests/runtime_windows.rs | grep -qx 1'` rc=0。
  **終了条件は CI の `collector-windows-runtime` job（本物の Windows）が緑であること**（Linux では走らない。grep は置いたことだけを見る）

## Task 10: 携帯の収集アプリ —— 焼き込み・確かめ・版で選んで送る・426・原本

- [ ] 10.1 `IngestRequest.kt`・`Heartbeat.kt`・`DropReport.kt`・原本の欠けの 1 件に `contractVersion`（直列化の名前 `contract_version`・既定 1。版を持たない溜めの行を読み戻すと 1）を足す（定数は 7.1 の `CONTRACT_VERSION`）。欄を足したら `tools/check-contract-version.sh --write` で写しを作り直す。
  未送信に積む 1 か所（記録・生存信号・破棄の報告・原本の欠け）と、原本の送り待ち（`originals.jsonl` の 1 行。記録を積んだ時の版を写す）で焼き込む（D1）。
  `Sender.kt` に「版 A / 受け付けない / 一時的な失敗」の答えを受け取り、版 ≤ A の行だけを載せ、見出しに載せた行の最大を入れる形を足す。426 は 1 件も取り除かず止まった状態、`unknown_field` / `contract_unsupported` は残す（`PERMANENT_ERRORS` を変えない。D17）。
  `OriginalUploader.kt` は版 > A の行を送らず、426 を「試し直す」に入れ、`record_missing` の 7 日の数えを始めない。`LocationService.kt` の送る契機の頭で 1 回 `GET /contract` を引き（`HttpTransport` に GET を足す）、5 つの送り手に同じ答えを渡す（D18（仮））。
  「S-01 の更新を待っている」状態（D9 の a〜e。e は 200 の読めない答えが 24 時間続いた —— 数え始めの時刻を `SharedPreferences` に持ち、再起動をまたぐ。置き場はこの Task で作り、11.1 が同じ置き場に通知の時刻を足す。D22・D24（仮））を 1 つの値で持ち、抜けた契機で `Sender` の飛ばす印を捨てる（D14）。試験 `St50SenderTest.kt`（記録の送り手と原本の送り手を偽の `Transport` で）:
  - `// Scenario: 携帯は S-01 が受け付ける版の行だけを送り、新しい版の行を残す`
  - `// Scenario: 携帯の要求の見出しは載せた行の版の最大になる`
  - `// Scenario: 携帯は積んだ時の版を 1 行ごとに持ち、更新の後も変えない`（積んだ後に定数を差し替えられる形で）
  - `// Scenario: 版を持たない古い行は版 1 として送られる`（ST50 より前の形の行を区切りの置き場に置いて読み戻す）
  - `// Scenario: 問い合わせの口が無い S-01 には何も送らない`（5 つの送り手）
  - `// Scenario: 読めない答えの S-01 には何も送らない`
  - `// Scenario: 到達できないときは今どおり一時的な失敗として扱う`
  - `// Scenario: 426 で断られた要求の行は 1 件も取り除かれない`
  - `// Scenario: unknown_field と contract_unsupported で断られた記録は捨てずに残す`
  - `// Scenario: 契約の版で残した記録も 90 日を超えると捨てられ、破棄として報告される`（`Retention` を通す）
  - `// Scenario: S-01 を更新すると手元に残っていた記録がそのまま入る`
  - `// Scenario: S-01 を更新すると、止まっていた間に飛ばす印の付いた記録も先に送られる`
  - `// Scenario: 原本は記録を積んだ時の版で止まる`（426 と版 > A のどちらでも欠けにならない）
  既存の `SenderTest`（恒久の 6 種・一時的な失敗の 401〜503）と `OriginalUploaderTest` が通ること。
  検証: `GT St50SenderTest` rc=0（13 本以上）/ `GT OriginalUploaderTest` rc=0（30 本以上）/ `scripts/quiet-run ccv -- tools/check-contract-version.sh` rc=0 / `GT St50EnvelopeShapeTest` rc=0（1 本以上）/
  既存の `SenderTest` だけを名指しして `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests 'dev.ashiato.collector.SenderTest'` rc=0 かつ
  `grep -c '<testcase ' collector-android/app/build/test-results/testDebugUnitTest/TEST-dev.ashiato.collector.SenderTest.xml` が 14 以上（`*SenderTest*` は `St50SenderTest` にも当たるので使わない）

## Task 11: 携帯の「サーバを更新してください」—— 画面と通知と 7 日前の知らせ

- [ ] 11.1 新しい `ContractNotifier.kt`（チャンネル `contract`・通知 id・止まった状態に入った時に 1 回・24 時間ごとに出し直し・抜けたら消す。前に出した時刻と 200 の読めない答えの数え始めの時刻は `SharedPreferences`。D6・D22。（e）だけのときは網の確認を添えた文言）、
  `MainActivity.kt` に止まった状態の間だけ出る 1 行（文言と日数）、`Retention.kt` の 7 日前の知らせの文言に止まった状態のとき「サーバを更新してください」を添える（（e）だけのときは「サーバを更新するか、つながっている網を確かめてください」。D24（仮））。試験（Robolectric）`St50ContractNoticeTest.kt`:
  - `// Scenario: 契約の版で止まると画面と通知に出る`（`MainActivity` の行と `ShadowNotificationManager` の 1 通）
  - `// Scenario: 画面には手元に残している日数が出る`
  - `// Scenario: 再起動しても 24 時間経つまで通知は出直さない`（`SharedPreferences` を残したまま作り直す）
  - `// Scenario: 止まっている間は 1 日に 1 回だけ通知が出直す`（時計を差し替え、5 分ごとに 2 日進める。出したのは 3 回・残っているのは 1 通）
  - `// Scenario: S-01 を更新すると通知と画面の行が消える`
  - `// Scenario: 到達できないだけでは更新を促さない`
  - `// Scenario: 読めない 200 の答えが 24 時間続くまでは更新を促さない`
  - `// Scenario: 読めない 200 の答えが 24 時間続くと網の確認を添えて更新を促す`（画面の行と通知の文言が「サーバを更新するか、つながっている網を確かめてください」）
  - `// Scenario: 再起動と圏外が挟まっても読めない 200 の数え始めは変わらない`（`SharedPreferences` を残したまま作り直す。圏外と 503 の区間を挟む）
  - `// Scenario: 読める答えを 1 回得ると読めない 200 の数え始めをやり直す`
  - `// Scenario: 契約の版で止まっている間の 7 日前の知らせは更新を促す`
  - `// Scenario: 読めない 200 の答えが続いて止まっている間の 7 日前の知らせは網の確認を添える`
  既存の `RetentionNotifierTest`・`MainActivityTest`・`OutageNotifierTest` が通ること。
  検証: `GT St50ContractNoticeTest` rc=0（12 本以上）/ `GT RetentionNotifierTest` rc=0（10 本以上）/ `GT MainActivityTest` rc=0（13 本以上）/
  `scripts/quiet-run lint -- ./collector-android/gradlew -p collector-android detekt` rc=0

## Task 12: 縦串（本物の S-01 と DB）と申し送りの確かめ

- [ ] 12.1 `tools/smoke.d/<作成時刻>_st50-contract-version.sh` を作る（`tools/smoke.sh` の本体には書き足さない）: 立ち上げた S-01 に curl で
  `GET /contract` が `{"contract_version":1}`・見出し `Ashiato-Contract-Version: 2` の `/ingest` が 426 で行が増えない・見出しなしの今の形の 1 件が入って `contract_version` が 1・知らない最上位の欄の 1 件が `unknown_field` で行が増えない、を psql で行を見て確かめる
  （Scenario の印は Task 1〜3 の試験が持つ。この節は本物の起動・資格情報・DB を通した確かめ）。
  検証: `scripts/quiet-run smoke -- tools/smoke.sh` rc=0
- [ ] 12.2 PR の前に、全 Scenario に印があり、仮の決定の台帳が通ることを見る。
  検証: `python3 scripts/check_scenarios.py . st50-contract-version` rc=0 / `python3 scripts/provisional.py check openspec/changes/st50-contract-version` rc=0
