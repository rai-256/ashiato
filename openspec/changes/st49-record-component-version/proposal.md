# ST49 — 記録ごとに、作った部品の版が残る

## Why

記録はスキーマ版と端末を持つが、それを作った収集アプリや S-01 の版を持たない。配った後に「この版の収集だけ取り違えていた」と分かっても、
どの記録がその版で作られたかを後から知る手段が無く、残し始める前の記録には版を足せない（扉 32）。本人の環境を配布の経路に切り替える日（ST52）の前に、
記録 1 件ごとに作った部品の版を焼き込み始める（FR-122 / FR-123）。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q4 は A（失われるものがある）で、本人の答えが推奨と違えば `spec --answers` が書き直す。

- **版の中身**（Q1 の推奨）: 3 つの部品（S-01・C-01・C-02）と画面（V-01）が、組み立てのたびに作り直す**組み立ての印**を持つ。形は
  `<版番号>+<commit 40 桁>[.dirty][.debug]`（その部品のソースに未 commit の変更があれば `.dirty`、release でない組み立てなら `.debug`。commit が取れなければ `unknown`）。
  commit の 40 桁は ST48 の台帳（`build_commit`）と同じ値（design D5・D6）
- **記録の表に 3 つの版の列**: 収集アプリの版（`collector_version`）・画面の版（`web_version`）・記録を D-01 に書いた S-01 の版（`server_version`）。
  前の版の表にも同じ 3 列を持たせ、前の版の読み口と書き出しの前の版の表に載せる（design D1（仮）・D7）
- **取り込み口に任意の欄 `client_version` を 1 つ足す**。S-01 は資格情報で振り分ける: API の合言葉（C-01 / C-02）の要求なら収集アプリの版、
  画面のログインの要求なら画面の版（Q4 の推奨）、それ以外（S-01 の中の書庫の取り込み・衛星）は使わない。値は形を検査せず・断らず、空は「不明」（design D8・D9）
- **S-01 の版はすべての記録に S-01 が自分で入れる**（Q2 の推奨。収集アプリと衛星の記録も、受け取って書いた S-01 の版を持つ。衛星自身の版は付けない）。
  書庫の取り込み（C-03）・滞在（派生）・画面から書いた記録は、それが FR-123 の版そのものになる（design D10）
- **版は今の中身を作った部品の版**: 外部サービス由来の更新で中身が入れ替わると版も一緒に動き、前の値は前の版に残る（design D1（仮））。滞在の作り直しで原文が変わったときは
  作り直した S-01 の版に変え、前の値は前の版に写す（design D2（仮））。同じ内容の再到着・印だけの書き換えでは版は変わらない
- **版は同じ記録の判定に効かない**: 内容の鍵・破棄の報告の鍵・原文に版を入れない（design D11）。版を持たない記録（ST49 より前の記録・更新の前に溜まった未送信）は「不明」のまま、
  後から推定で埋めない（design D12）
- **収集アプリ 2 本は、記録を溜めに積む時点で版を焼き込む**（未送信を更新の後に送っても作った時の版のまま）。溜めた行を読み戻すとき、版の欄が無ければ今の版を入れず、
  知らない欄を捨てずにそのまま送る（design D3（仮）・D13）
- **記録の表の外の追記のみの行にも版を持たせる**（Q3 の推奨）: 破棄の報告・原本の欠け（収集アプリの版と S-01 の版）、画面の操作の台帳 —— 削除・気分の通知・滞在の基準・個人属性の種類の名前（S-01 の版）。
  破棄の報告の下書きは、収集アプリの版が変わったら閉じて次の報告にする（design D14・D15）
- **書き出し**: 記録の表・前の版の表・破棄の報告・気分の通知の台帳・滞在の基準・個人属性の種類の名前のファイルに版の列を説明つきで足す。説明は「部品の申告（S-01 は確かめていない）」と「S-01 が自分で入れた」に書き分ける（design D4）。今の列は消さず・名前を変えない
- 収集した記録の版を書き換えるのは、前の版を同じまとまりに積むときだけ（書き換えの門（FR-30）の対象にする。前の版の表の版は凍結する。design D16・D21（仮））。選び出す口（画面・API の絞り込み）・版の変化の検知（ST51）・送信契約の版（ST50）は作らない

**BREAKING ではない**: 足す欄はどれも任意で、送らない収集アプリ・画面の要求と応答は今と同じ。今の S-01 は知らない欄を読み飛ばすので、新しい収集アプリが
ST49 より前の S-01 に送っても記録は入る（版だけが落ちる。代償は design の Risks）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `record-envelope`: 記録が作った部品の版を持つ・組み立ての印の形と作り直し・資格情報で版を振り分ける・版を同じ記録の判定に使わない・中身と一緒に版が動く・版は前の版を積むときだけ書き換えられる・画面が送る 1 件に画面の印を載せる（ADDED 7 つ）
- `device-collection`: 携帯の収集アプリが記録を積む時点で版を焼き込み、溜めた行の欄を落とさない・破棄の報告と原本の欠けが版を持つ（ADDED 2 つ）
- `desktop-collection`: PC の収集アプリが記録を積む時点で版を焼き込み、溜めた行の欄を落とさない（ADDED 1 つ）
- `derived-records`: 滞在は書いた S-01 の版を持ち、作り直しで原文が変わったら版も動く・滞在の基準の台帳の行が S-01 の版を持つ（ADDED 2 つ）
- `data-durability`: 書き出しの記録・前の版・破棄の報告・画面の操作の台帳のファイルに版の列を説明つきで持つ（ADDED 1 つ）
- `record-deletion`: 削除の台帳の行が書いた S-01 の版を持つ（ADDED 1 つ）
- `subjective-log`: 気分の通知の台帳の行が書いた S-01 の版を持つ（ADDED 1 つ）
- `personal-entities`: 個人属性の種類の名前の台帳の行が書いた S-01 の版を持つ（ADDED 1 つ）

**capability の照合**: INDEX.md の capability 表は ST49 を `record-envelope`・`device-collection`・`desktop-collection` に割り当てていた。記録の版の骨格はこの 3 つで合っているが、
版を持たせる先が滞在の作り直しと滞在の基準（D2（仮）・Q3）・書き出しの列（C6）・削除の台帳・気分の通知の台帳・個人属性の種類の名前（Q3 の推奨）に及ぶので、`derived-records`・`data-durability`・`record-deletion`・`subjective-log`・`personal-entities` に
ST49 を足し、INDEX.md の訂正の節に理由を書いた。どれも新しい関心を ADDED で足すだけで、後続の振る舞いの前倒しではない。走っている change が MODIFIED している Requirement
（ST41 の `record-envelope` 3 つ、ST23 の `record-deletion` 3 つ、ST29 の `data-durability` 1 つ）には MODIFIED を重ねない。

**requires の変更**: ST49 の requires に **ST48** を足した（layer 1 → 2。INDEX.md の訂正の節）。S-01 の組み立ての印は ST48 の `crates/server/build.rs`（design D8（仮））に積み、
印の形を ST48 の台帳とそろえる（C10 / C11）。ST49 の移行は名前の時刻が `202610250000` 以後なので、ST48 の D10（仮）の規則（前進側に取引の文を持たない・`.down.sql` が版の記録を消す）に従う。
ST50（送信契約の版）は requires に入れない —— 期限の前の版の脱落は入れ替えの順（S-01 が先）で防ぎ、期限の後は ST50 が止める（design の Migration Plan・`docs/handoff/ST50.md`）。

## Impact

- コード（S-01）: `crates/server/build.rs`（ST48 の印に `.dirty` / `.debug` と `.git/HEAD` の作り直しを足す）、新しい `crates/build-stamp`（3 つの組み立てが共有する印の作り方。build-dependency）、
  `crates/server/src/ingest.rs`（`client_version`）、`crates/server/src/lib.rs`（`ingest_request` の呼び出し元の区別・`ingest_one` の 2 つの INSERT・`Stored`・`apply_external_update`・前の版の INSERT）、
  `crates/server/src/stay_store.rs`（`insert_stay` / `update_stay` / `push_version`）、`crates/server/src/drops.rs`・`media_missing.rs`・`deletion.rs`・`feelings.rs`・`attributes_store.rs`（版の列）、
  `crates/server/src/export.rs`（`EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS` / `DROP_REPORT_COLUMNS` / `FEELING_REMINDER_COLUMNS` / `STAY_CRITERIA_COLUMNS` / `ATTRIBUTE_KIND_NAME_COLUMNS`）、`docs/openapi.json`（再生成）、`docs/collector-contract.md`
- 移行: 新しい移行 1 つ（記録の表・前の版の表に 3 列、破棄の報告・原本の欠けに 2 列、削除の台帳・気分の通知の台帳・滞在の基準・個人属性の種類の名前に 1 列、本表の門と前の版の錠に版の列、`core.event_live` / `core.event_version_live` の作り直し）と `.down.sql`、`migrations/ORDER` の末尾へ 1 行
- 収集アプリ: `crates/collector-windows`（新しい `build.rs`・`contract.rs` の `client_version` と知らない欄の保持・`outbox.rs` の積む時の焼き込み）、
  `collector-android`（`app/build.gradle.kts` の印・`IngestRequest.kt` / `Outbox.kt` / `SegmentStore.kt` の焼き込みと欄の保持・`DropReport.kt` の下書きの版・原本の欠けの送り出し）
- 画面: `web/build-stamp.ts`・`web/vite.config.ts`・`web/vitest.config.ts`（印の埋め込み）、`web/src/feelings.ts`・`places.ts`・`attributes.ts`・`MasterView.tsx`（送る 1 件に `client_version`）、`web/e2e/st49-client-version.spec.ts`。画面の表示は変えない
- **走っている change との重なり**（差し戻さない。後から main に入る側が rebase で解く）:
  - **ST48**（`st48-db-version-guard`。下流が走っている）: requires に入れた。`crates/server/build.rs` の `build_commit()` の上に積む。ST48 の D8（仮）が否になったら印の作り方だけを ST49 が持ち、形は変えない（C11）
  - **ST41**（`st41-event-end-time`。下流が走っている）: 同じ `IngestRequest`・`ingest_one` の 2 つの INSERT・`apply_external_update`・前の版の INSERT・`push_version`・
    `core.reject_version_rewrite`・`core.event_live` / `core.event_version_live`・`EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS` に列を足す。ST49 の移行は**移行を当てる時点の**
    `core.event_live`（`SELECT *` の作り直し）・`core.event_version_live`（列を名指し）・`core.reject_version_rewrite` の本体を写して版の列を足すだけにする（ST41 が先に main に入っていれば終わりの列を含む本体を写す）。
    書き出しの列は ST41 と同じく末尾でなく `schema_version` の次に足す（衝突の幅を減らす）
  - **ST23**（`st23-record-erasure`。下流が走っている）: `ingest_request` に「HTTP か内部の格納か」の引数を足す（ST23 design D7 / D8）。ST49 の呼び出し元の区別（D9）は同じ場所なので、
    後から入る側が 1 つの引数（呼び出し元の種類）に畳む。ST23 が足す削除の台帳の INSERT は `server_version` を書く（D10 の機械の止めが rebase の後に落とす。`docs/handoff/ST23.md`）。本文の消去は版の列に触らない（版は私的な中身ではない。D16）
  - **ST29**（`st29-satellite-credentials`。下流が走っている）: 衛星の資格情報が `Caller` に足す種類は「API の合言葉でも画面のログインでもない」側に入り、`client_version` を使わない（D9）。`docs/handoff/ST29.md` で確かめを申し送る
  - CH03 / CH04 / CH06: 触る capability と表が重ならない
- 申し送り: `docs/handoff/ST23.md`（削除の台帳の INSERT に `server_version`。review/spec.md R5）、`docs/handoff/ST50.md`（期限の前の版の脱落・入れ替えの順）、`docs/handoff/ST29.md`（衛星の要求で `client_version` を使わない）、`docs/handoff/ST52.md`（版番号の出所を 1 つにしたら印の版番号をそこから読む。commit は ST48 と同じ値）
