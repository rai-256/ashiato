# ST41 — 幅のある出来事に終わりの時刻を持たせる

## Why

記録は「出来事が起きた時刻」を 1 つしか持たない。睡眠や読書のひと続きのように始まりと終わりのある出来事は、
終わりを中身（`payload`）に埋めるしかなく、期間の重なりで探せない（索引が効かない）。列は後から足しても過去の記録が
埋まらない（扉 #28）ので、幅のある出来事を書く衛星（ST39）が書き始める前に、記録の骨格に終わりの時刻を持たせる（FR-91）。

## What Changes

- 記録の表に「終わりの時刻」「その UTC からのずれ（分）」「そのタイムゾーン識別子」の 3 列を足す。3 つは揃って有るか揃って無いか。
  終わりにも FR-20（ずれと TZ の両方）を課す（design D1・D2）
- 取り込み口に任意の 3 欄を足す。片方だけ・ずれが範囲外・TZ が空の 1 件は、理由の種別つきで受け付けない。
  収集アプリ（携帯・PC）は送らないまま、送り方も変えない（D2・D3）
- 前の版の履歴にも同じ 3 列を持たせ、外部の識別子のある記録は「終わりだけが違う」到着も更新として前の版を残す。
  同じ記録かどうかの鍵（内容のハッシュ）は変えない（深掘り Q7 の推奨。D7）
- 収集した記録の終わりの時刻を、出来事の時刻と同じ錠にかける（門・前の版の錠・消去の形）。終わりのずれと TZ は始まりと同じく錠の外（D5）
- `GET /events` に期間（`from` / `to`）とソースの絞り込みを足し、期間に重なる記録を引けるようにする。終わりの 3 欄は値のある記録にだけ載る
  （深掘り Q2・Q3 の推奨。D8・D9）。期間の重なりは終わりのある行だけの部分索引（式の GiST）で引く（D10）
- 書き出しと「消した滞在の時間」の読み口は、終わりを列から先に、無ければ内容の鍵から読む（ST33 からの申し送り C7。D11）。
  書き出しの列の一覧に 3 列を足す
- 幅のある出来事を日で数えるときの区切りは Asia/Tokyo のまま、件数は始まりの日に 1 件（深掘り Q4 の推奨。D12（仮））
- 滞在（Q5）と収集済みの記録の中身にある終わり（Q6）は、この Story では列に写さない（D13（仮）・D14（仮））

**BREAKING ではない**: 終わりの 3 欄はどれも任意で、送らない収集側の要求と応答は今と同じ。`GET /events` は絞り込みを渡さなければ今と同じ行を返し、
終わりを持たない記録の行の鍵も今と同じ（browsing-views の「記録の読み出しの形は変わらない」が成り立つ。D9）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `record-envelope`: 記録の 2 つの時刻と地域に終わりの時刻を足す・外部サービス由来の更新の条件に終わりを足す・収集した記録の錠に終わりを足す
  （MODIFIED 3 つ）。同じ記録の判定と削除済みの止めに終わりが効かないこと・期間に重なる記録を引けること（ADDED 2 つ）。
  INDEX.md の capability 表は `record-envelope` に ST41（終わりの時刻）を割り当てている（照合: 合っている。前倒し無し）
- `data-durability`: 書き出しが幅を読むとき、終わりの列を内容の鍵より先に読む（ADDED 1 つ）。
  **MODIFIED にしない** —— 同じ要件「書き出しは本人が消した場面を出さない」を走っている CH02 が MODIFIED している。並べて MODIFIED にすると、
  後から archive した側が先の側を写しで上書きする。新しい関心（列の終わり）なので ADDED で足す
- `collection-coverage`: 幅のある出来事は、日ごとの件数で始まりの日に 1 件と数える（ADDED 1 つ。深掘り Q4）。
  INDEX.md の表では `collection-coverage` に ST41 は無いが、この Story は日ごとの件数の振る舞いを変えず、
  終わりのある記録での数え方を Scenario で固定するだけ（INDEX の C34「日の境界のタイムゾーンは ST41 の上流で決める」の置き場）
- INDEX.md の capability 表の `collection-coverage` と `data-durability` に ST41 を足し、訂正の節に理由を書いた（spec レビュー R13。どちらも ADDED で、後続の振る舞いの前倒しではない）

## Impact

- コード: `crates/server/src/ingest.rs`（`IngestRequest` の 3 欄・`validate`）、`crates/server/src/lib.rs`（`IngestError` の種別・`ingest_one` の INSERT・
  `Stored`・更新の判定・`apply_external_update`・`events`）、`crates/server/src/export.rs`（`EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS`）、
  `crates/server/src/export/excluded.rs`（幅の (a) の読み方）、`crates/server/src/export/doc.rs`（説明書）、`crates/server/src/stay_store.rs`（変えない。足した列は NULL で入る。D13（仮））、
  `docs/openapi.json`（再生成）、`docs/collector-contract.md`（任意の 3 欄）
- 移行: 新しい移行 1 つ（3 列 × 2 表・CHECK・門と錠の関数・`core.event_live` / `core.event_version_live` / `core.stay_erased` の作り直し・部分索引）と `.down.sql`、
  `migrations/ORDER` の末尾へ 1 行
- 収集アプリ（`collector-android` / `crates/collector-windows`）: 変えない（壊してはいけないもの）
- **走っている change との重なり**:
  - **CH02**（`ch02-export-next-record-window`。下流が走っている）: 同じ `excluded.rs` の `overlap_sql` / `excluded_sql` を窓関数に作り替え、
    いまの SQL を `export_window_tests.rs` の `legacy` に逐語で写して新旧を突き合わせる。ST41 の下流は **main にある形**（CH02 が先に入っていれば窓の形）の
    (a) の終わりの読み方に列を足す。窓に載る列（鍵の列）には触らない —— 終わりは窓の後に `id` で戻した行から読む（CH02 design D2 と同じ場所）。
    CH02 の要件には MODIFIED を重ねない（上の Capabilities）。ST41 が先に main に入ったら、CH02 の `legacy` の写しと偽データに終わりの列の行が無いことを
    `docs/handoff/CH02.md` で申し送る（下流の Task 7 が確かめる）
  - **ST09**（`st09-media-capture`。下流が走っている）: 同じ `excluded.rs` の `SPAN_RULES` / `NO_COORD_SOURCES` / `COORD_SOURCES` と `export.rs` の分類に行を足す。
    ST41 はそれらの一覧に触らない。`EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS` は ST09 も触りうるので、末尾ではなく `event_time` の次に足す（衝突の幅を減らす）
  - ST13 / CH01: 触る capability と表が重ならない
- 申し送り: `docs/handoff/ST39.md`（外部の識別子を持たない送り手は終わってから送る・始まりの側のずれと TZ の範囲の検査が無い既存の穴。review/deep.md R3・R5）
