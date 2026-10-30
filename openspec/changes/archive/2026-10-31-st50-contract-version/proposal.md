# ST50 — 収集アプリとサーバの版が食い違っても記録が失われない

## Why

いまの S-01 は送信契約の版を確かめず、収集アプリが送った知らない欄を黙って読み飛ばして 200 で受け付ける（premises P1）。受け付けた記録は収集アプリの未送信から消えるので、
新しい収集アプリが古い S-01 に送ると、新しい欄（ST41 の終わりの時刻・ST49 の版など）だけが二度と戻らない。逆に断りを 400 の恒久の種別にすると、携帯の収集アプリは記録ごと捨てる（FR-10）。
S-01 の更新は手動（FR-114）なので、収集アプリだけが新しい組み合わせは日常に起きる。最初の版を配る（ST52）前に、送信契約の版と、版が食い違ったときに記録を手元に残す振る舞いを入れる（FR-124 / FR-125 / FR-126 / FR-130）。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q4・Q7〜Q9 は A（失われるものがある）、Q5・Q6 は B で、本人の答えが推奨と違えば `spec --answers` が書き直す。

- **送信契約の版**（FR-124 / FR-130）: スキーマ版（`schema_version`）と別の整数 1 つ。今の封筒の全部と、最初の配った印（タグ `v*`。ベータ版を含む。Q8 の推奨）までに足す欄（ST41 の終わりの時刻・ST49 の `client_version`）は**版 1**。
  S-01 は自分の版以下の全部を受け付け続ける（design D10・D12）
- **確かめる口** `GET /contract`（Q4 の推奨）: S-01 が受け付ける一番新しい契約の版を `{"contract_version": <整数>}` で返す。API の合言葉で守る（design D4・D15）
- **要求が版を名乗る**（Q4 の推奨）: 収集アプリの要求は見出し `Ashiato-Contract-Version: <整数>` を持つ。S-01 は自分の版より新しい版を **426**（`{"error": "contract_unsupported", "contract_version": <S-01 の版>}`）で断り、何も格納しない。
  見出しの無い要求（ST50 より前の収集アプリ）は版 1 とみなす（design D4・D10）。対象は収集アプリが送るもの全部 —— 記録・生存信号・破棄の報告・原本の欠け・原本（Q3 の推奨。design D3）
- **知らない欄の断り**（Q2 の推奨）: S-01 は、自分が知らない最上位の欄を持つ 1 件を、収集アプリが捨てない断りの種別 `unknown_field` で 1 件ごとに断る（`malformed` にしない。design D2）
- **記録は作った時の契約の版を持つ**（Q1 の推奨）: 収集アプリ 2 本は、溜めに積む時点で契約の版を 1 行ごとに焼き込み、記録の 1 件に `contract_version` を載せる。S-01 は記録の表と前の版の表の列
  `contract_version` に残し、書き出しにも説明つきで出す。収集アプリ以外が書いた記録（画面・S-01 の中の書庫・滞在）は空（design D1・D20）
- **収集アプリは送る前に確かめ、受け付けられない分を手元に残す**（FR-125。Q9 の推奨）: 送る契機ごとに `GET /contract` を引き、S-01 が受け付ける版で作った行だけを送る。
  受け付けない版の行・口を持たない S-01（404・読めない答え）に当たった行は送らずに手元に残す。携帯の上限（90 日・2 GB）は今どおり掛かり、PC には上限を足さない（design D9・D11・D13（仮））
- **「サーバを更新してください」**（FR-126）: 契約の版で手元に残している行がある間、携帯は画面と通知（止まった時に 1 回・その後 1 日 1 回）に、PC は通知（同じ頻度・何日分を残しているか）に出す（Q5・Q6 の推奨。design D5・D6）
- **版の上げ忘れを CI が落とす**（FR-130。Q7 の推奨）: 新しい `tools/check-contract-version.sh` が、S-01 が受ける型（`docs/openapi.json`）と収集アプリ 2 本が実際に直列化する欄から封筒の形の写し
  （`docs/contract/envelope.json`）を作り、最後の配った印 `v*` の写しと比べて、欄の追加・削除・型・必須の変化があるのに版が上がっていなければ落とす。3 つの版の定数の食い違いも落とす（design D7・D8・D12）
- **配った版の封筒を固定する材料**: 版 1 の封筒の例（記録・生存信号・破棄の報告・原本の欠け）を S-01 の試験の材料に置き、受け付け続けることを試験で固定する（FR-124。design D10）

**BREAKING ではない**（今配っている収集アプリとの組み合わせ）: 見出しの無い要求は版 1 として今どおり受け付ける。ST50 より前の収集アプリが送る欄は全部 S-01 が知っているので `unknown_field` にならない。
426 と `unknown_field` は、今の携帯の収集アプリも PC の収集アプリも捨てない（premises P3・P4・P5・P9）。
**新しい収集アプリと ST50 より前の S-01 の組み合わせでは、収集アプリは何も送らなくなる**（FR-125 の逐語。design の Risks と Migration Plan）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `record-envelope`: 送信契約の版と確かめる口・要求が版を名乗り新しすぎる版を 426 で断る・知らない欄を `unknown_field` で断る・記録が作った時の契約の版を持つ・配った版を受け付け続ける・版の上げ忘れを CI が落とす（ADDED 6 つ）
- `device-collection`: 携帯の収集アプリは送る前に確かめ、受け付けられない行を手元に残す・「サーバを更新してください」を画面と通知に出す（ADDED 2 つ）
- `desktop-collection`: PC の収集アプリは送る前に確かめ、受け付けられない行を手元に残す・「サーバを更新してください」を通知に出す（ADDED 2 つ）
- `data-durability`: 書き出しの記録と前の版の表が契約の版の列を説明つきで持つ（ADDED 1 つ）

**capability の照合**: INDEX.md は ST50 を `record-envelope`・`device-collection`・`desktop-collection` に割り当てていた。契約の版と S-01 の断りは API 契約（`record-envelope`）、
送る前に確かめて手元に残すのは収集アプリ（`device-collection` / `desktop-collection`）で合っている。生存信号と破棄の報告の受け口（`collection-coverage`）も止めるが（Q3）、
止める規則は「収集アプリの資格情報の要求すべて」に掛かる API 契約の 1 つの Requirement として `record-envelope` に置き、`collection-coverage` の Requirement は変えない。
記録の契約の版を書き出しの列に出す（Q1）ので `data-durability` に ST50 を足し、INDEX.md の訂正の節に理由を書いた。確かめる口は今の読み出しの記録の規則のまま行を残す（design D23（仮））ので `data-sensitivity` は変えない。ADDED で足すだけで、後続の振る舞いの前倒しではない。

**requires は変えない**（ST04 のまま）。ST49（下流が走っている）の `client_version` は版 1 に含む欄として扱うだけで、ST49 の実装に積まない（後から main に入る側が rebase で写す。下の Impact）。

## Impact

- コード（S-01）: `crates/server/src/lib.rs`（`router` に `GET /contract`・収集アプリの資格情報の要求の見出しの検査・`ingest_one` の知らない欄と記録の版・2 つの INSERT・`apply_external_update`・前の版の INSERT）、
  新しい `crates/server/src/contract_version.rs`（版の定数・見出しの読み・知らない欄の検査・426 の応答）、`crates/server/src/ingest.rs`（`contract_version` の欄）、`heartbeat.rs`・`drops.rs`・`media_missing.rs`（知らない欄・`contract_version` の欄）、
  `media.rs`（原本の `PUT` / `HEAD` の見出しの検査）、`export.rs`（`EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS`）、`bin/openapi.rs` と `docs/openapi.json`（再生成）
- 移行: 新しい移行 1 つ（`core.event` と `core.event_version` に `contract_version integer`・収集アプリの記録の埋め戻し・`core.event_live` / `core.event_version_live` の作り直し・門と前の版の錠に列）と `.down.sql`、`migrations/ORDER` の末尾へ 1 行
- 収集アプリ: `crates/collector-windows`（`contract.rs` の版の定数と欄・`outbox.rs` の積む時の焼き込み・`sender.rs` の確かめと 426・`runtime.rs` の契機・`toast.rs` の別の札の通知）、
  `collector-android`（`IngestRequest.kt` / `Heartbeat.kt` / `DropReport.kt` / `OriginalLedger.kt` の版・`Sender.kt` の確かめと 426・`OriginalUploader.kt` の 426・`LocationService.kt` の契機・新しい通知と `MainActivity.kt` の行・`Retention.kt` の 7 日前の知らせの文言）
- 道具と文書: 新しい `tools/check-contract-version.sh`（`--self-test` つき。CI の job に足す）、`docs/contract/envelope.json`（封筒の形の写し）、`docs/collector-contract.md`（版の節・見出し・426・`unknown_field`・`GET /contract`・版の履歴）、
  新しい `tools/check-immutable.d/<作成時刻>_st50-contract-version.sh`
- **走っている change との重なり**（差し戻さない。後から main に入る側が rebase で解く）:
  - **ST49**（`st49-record-component-version`。下流が走っている）: 同じ `IngestRequest`（S-01 と PC）・`ingest_one` の 2 つの INSERT・`apply_external_update`・前の版の INSERT・`core.event_live` / `core.event_version_live`・
    `core.reject_version_rewrite` と収集した記録の門・`EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS`・収集アプリの溜めの焼き込み（D13）に列と欄を足す。ST50 の移行は**移行を当てる時点の**本体を写して版の列を足すだけにする（design D20）。
    ST49 の `client_version`（`/ingest`・`/drops`・`/media/missing`）は版 1 の欄で、ST50 の知らない欄の検査は型の欄の一覧から作るので、後から入る側が型を合わせれば `unknown_field` にならない。
    ST49 の D3（仮）の反転条件（「知らない欄を送り返したことで S-01 が要求を断る事例が出たら、送る前に契約の版で欄を落とす」）は ST50 の Q2 の推奨で条件の形が立つが、欄を落とす必要は無い —— `docs/handoff/ST49.md` に申し送る（review/spec.md の followup）
  - **ST23**（`st23-record-erasure`。下流が走っている）: ST50 は `ingest_one` / `ingest_request` に「収集アプリの 1 件か」の印の引数を足し（design D1）、ST23 は同じ `ingest_request` に `Settle` の引数を足す（ST23 design D7・D8）。後から main に入る側が rebase で両方の引数を合わせる（review/spec.md R4）
  - **ST11**（`st11-health-history-permission`）・CH03 / CH05: 触る表と関数が重ならない
- 申し送り: `docs/handoff/ST49.md`（D3（仮）の反転条件の扱い）、`docs/handoff/ST52.md`（配った印の型 `v*` を ST50 の検査も使う）、ST51（生存信号に欄を足すときは `tools/check-contract-version.sh --write` で封筒の形の写しを作り直し、最初の配った印の後なら契約の版を上げる。印の前は版 1 のまま。tasks がまだ無いので `deferred ST51`。review/spec.md R14）
