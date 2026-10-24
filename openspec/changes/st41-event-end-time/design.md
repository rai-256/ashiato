# ST41 design — 幅のある出来事に終わりの時刻を持たせる

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q6 の推奨は `D<n>（仮）`、Q7（A）の推奨は仮でない D として書き、
> 本人の答えが推奨と違えば `spec --answers` がその D と Scenario だけを書き直す。
> 問いの HTML の「聞かずに決めること」(1)〜(6)（`deep-questions.json` の note。deep.md の C1〜C6）は、仮でない D1〜D3・D5・D6・D10・D11 に写した。
> spec レビュー（review/spec.md）で足した仮の決定は D17（仮）・D18（仮）。

## Context

動機は proposal.md の Why。いまの形:

- 記録（`core.event`）は出来事の時刻 `event_time` とそのずれ `tz_offset_min`・地域 `tz_id` を 1 組だけ持つ。前の版（`core.event_version`）も同じ 3 列を持つ
- 同じ記録かどうかの鍵 `content_hash` は論理ソース・出来事の時刻・原文の 3 欄だけ（`ingest.rs` の `content_hash_of`。`hash_is_pinned` が固定値で凍結）
- 外部の識別子のある記録の更新は `stored.content_hash != hash && external_id.is_some()` のときだけ `apply_external_update` に入る（`lib.rs` の `ingest_one`）。
  外部の識別子の無い記録は `ON CONFLICT (user_id, logical_source, content_hash) … DO NOTHING` で畳む
- 収集した記録の錠: 本表の門 `core.require_version_or_ledger`（`raw` / `payload` / `event_time` / `content_hash` の変化を見る・更新前の版の照合・消去の形）、
  前の版の錠 `core.reject_version_rewrite`（列を名指し）。始まりのずれと TZ は錠の外（`gates.sql` の注記）
- 読み口: `core.event_live`（`SELECT *` の展開が作った時点で固まる）・`core.event_version_live`（列を名指し）・`core.stay_erased`（終わりを `payload->>'end'` から読む）
- `GET /events` は `core.event_live` の全件を 6 つの鍵（`id` / `logical_source` / `event_time` / `tz_id` / `origin` / `raw`）で時刻順に返すだけ。
  browsing-views の Scenario「記録の読み出しの形は変わらない」（`day_records_tests.rs` の `st25_api_unchanged_events_and_stays_shapes`）がこの 6 つの鍵を固定している
- 書き出し（`export.rs` / `export/excluded.rs`）は幅の (a) を内容の `END_KEYS` からだけ読む。CH02 がその SQL を窓関数に作り替えている最中（proposal の Impact）
- 前提の確かめ（PostgreSQL 17 の範囲型・式の索引・120 万行の索引の作成時間）は `premises.md`（すべて確かさ 4）

## Goals / Non-Goals

**Goals**: 終わりの時刻を列で持ち、終わりにも FR-20 を課し、期間の重なりで引ける。終わりを足しても、収集アプリの送信・同じ記録の判定・削除済みの止め・
収集した記録の錠・書き出しの除く判定が今より弱くならない。

**Non-Goals**:
- 衛星の書き込み口・読み出しの門・台帳（ST39 / ST45）。期間の読み出しは今の本人の資格情報で読める範囲（D9）
- 1 日の並び（`day_records.rs` の許可リスト）・稼働状況の格子・画面。どれも終わりを列に持つソースをまだ持たない（D12）
- 始まりの側のずれと TZ の値の検査（既存の穴。ST01 の送信契約を変えるので ST39 へ申し送る。D2）
- 滞在と収集済みの記録の終わりを列へ写すこと（D13・D14）

## Decisions

### D1. 終わりは 3 列で持ち、3 つ揃って有るか揃って無いかを DB でも縛る

`core.event` と `core.event_version` に `end_time timestamptz NULL`・`end_tz_offset_min integer NULL`・`end_tz_id text NULL` を足す。
型は始まりの 3 列と同じ（失う桁は始まりと同じ。µs 未満）。CHECK を 2 つ:

- 揃い: `(end_time IS NULL) = (end_tz_offset_min IS NULL) AND (end_time IS NULL) = (end_tz_id IS NULL)`
- 値: `end_tz_offset_min BETWEEN -1439 AND 1439` と `end_tz_id <> ''`（NULL は CHECK を通る）

取り込み口で先に断る（D2）ので CHECK は取り込み口の外の書き込みへの止め（spec の Scenario「取り込み口を通さない書き込みでも…」で固定する）。終わりが始まりより前を断る CHECK は**置かない**（D4）。
列の名前は応答の欄と書き出しの列と同じ（`end_time` / `end_tz_offset_min` / `end_tz_id`）—— 名前を 1 つにしておけば、書き出しの「表の列と同じ名前」がそのまま成り立つ。

代案: 範囲型の列 1 つ（`tstzrange`）。ずれと TZ を別に持つ必要は変わらず、逆順の値を格納できない（`premises.md` 2 行目）。退けた。
代案: 長さ（続いた時間）で持つ。FR-91 は「終わりの時刻」と書き、終わりにも FR-20 を課すので時刻で持つ。

### D2. 取り込み口は任意の 3 欄を受け、外れた 1 件を `invalid_end_time` で断る（note (1)(2)・C1・C2）

`IngestRequest` に `#[serde(default)]` の `end_time: Option<DateTime<Utc>>`・`end_tz_offset_min: Option<i32>`・`end_tz_id: Option<String>` を足す。
`IngestRequest::validate` に検査を足し、`ingest::Invalid::EndTime` → `IngestError::InvalidEndTime`（`snake_case` で `invalid_end_time`）にする。断るのは:

- 3 つの一部だけがある
- `end_tz_offset_min` が −1,439〜1,439 の外（`chrono::FixedOffset` の上限。外れると読み出しで時刻に直せない）
- `end_tz_id` が空の文字列か U+0000 を含む（U+0000 は `text` に格納できず SQLSTATE 22021 → まとめ送り全体が 500 になる。`raw` と同じ理由）

TZ の名前を IANA の一覧と照らすのは**しない**（知らない名前の記録を断る＝失う側に倒れる）。
**始まりの側**（`tz_offset_min` / `tz_id`）にも同じ検査が無い（`lib.rs` の主張と場所の記録の検査の注記が書いている既存の穴）。
直すと ST01 の送信契約を変えるので、この Story では直さず `docs/handoff/ST39.md`（FR-99 の持ち主）へ申し送る（review/deep.md R5）。
`docs/openapi.json` を再生成し、`docs/collector-contract.md` に任意の 3 欄と `invalid_end_time` を足す（収集アプリは送らない、と書く）。

### D3. 収集アプリは変えない（note (2)・C2）

携帯（`collector-android`）と PC（`crates/collector-windows`）の送る形・送り方・試験に触らない（Story の「壊してはいけないもの」）。
サーバの側で、終わりの欄を持たない要求の結果と状態符号が今と同じであることを Scenario で固定する。

### D4（仮）. 終わりが始まりより前の記録は送られたまま入れ、読む側では使わない（Q1 の推奨）

取り込み口も DB も断らない。期間で引くとき（D8）は始まりの 1 点として扱う。書き出しの幅（D11）は、列の逆順の終わりを持たないものとして内容の終わりの項目から順に読む
（いまの書き出しが内容の逆順の終わりを扱うのと同じ。点に縮めるより除き過ぎの側）。
期間の索引の式は `greatest(event_time, end_time)` で始まりに寄せて、逆順の行でも INSERT が落ちないようにする（D10。`premises.md` 3 行目で確かめた）。

反転条件: 本人が Q1 で「その 1 件を断る」を選んだら、`validate` に `end_time < event_time` を足して `invalid_end_time` で断り、CHECK を足す。
読む側の扱い（点 / 次の順）は残してよい（断った後は逆順の行が来ない）。

### D5. 収集した記録の終わりの時刻は出来事の時刻と同じ錠にかける（note (3)・C3）

移行で 2 つの関数を `CREATE OR REPLACE` する:

- `core.require_version_or_ledger`:
  - `content_changed` に `NEW.end_time IS DISTINCT FROM OLD.end_time` を足す
  - 消去の形 `is_erasure` に `NEW.end_time IS NOT DISTINCT FROM OLD.end_time` を足す（消去の顔で終わりを書き換えさせない）
  - 更新前の版の照合に `v.end_time IS NOT DISTINCT FROM OLD.end_time` を足す（終わりだけ違う履歴の 1 行で通させない。R94 と同じ穴）
- `core.reject_version_rewrite`: 名指しの列に `end_time` / `end_tz_offset_min` / `end_tz_id` を足す

`end_tz_offset_min` / `end_tz_id` は本表の錠の外（始まりの `tz_offset_min` / `tz_id` と同じく、収集側の設定の誤りを直す余地）。
前の版では始まりの TZ も錠の中なので、終わりの TZ も錠の中。関数の本体はいまの版を写して足すだけにする（ほかの条件を変えない）。
`tools/check-immutable.d/<作成時刻>_st41-event-end-time.sh` に、終わりだけを履歴なしで変える・終わりだけ違う履歴で変える・前の版の終わりを変える、の 3 つが拒まれることを置く。

### D6. 進行中の出来事と、終わりが消える更新（note (4)・C4。D18（仮）で絞った）

- 進行中（眠っている途中など）は、外部の識別子を付けて終わりなしで送り、終わったら同じ識別子で終わりつきを送る。D7 の更新の経路で前の版を残して終わりが入る
- 更新で**内容が異なり**終わりを持たない版が届いたら、その版のとおり終わりを空にする（`apply_external_update` の UPDATE は 3 列を届いた値で置く。前の終わりは前の版に残る）。
  内容が同じで終わりを持たない到着は更新にしない（D18（仮））
- 終わりが届かないまま —— 終わりを持たない記録（点）として残る

`apply_external_update` の前の版の INSERT に 3 列を足し、`Stored` に 3 列を足す。

### D7. 同じ記録の鍵は変えず、外部の識別子のある記録だけ「終わりが違う」を更新にする（Q7 の推奨・答え待ち）

- `content_hash_of` の入力は 3 欄のまま。`hash_is_pinned` の固定値 `39d0…df1` を、終わりつきの要求でも同じ値になる試験で 1 本足す
- `ingest_one` の更新の判定を `(stored.content_hash != hash || (req.end_time.is_some() && stored.end_time != trunc_us(req.end_time))) && external_id.is_some()` にする。
  比べるのは `end_time` だけ（終わりのずれと TZ だけが違う到着は、始まりの側と同じく更新にしない。D5 の錠の外の列。spec の Scenario で固定）。
  届いた終わりは**マイクロ秒に切り捨ててから**比べ、格納する（DB の `timestamptz` は µs まで。切り捨てずに比べると、ns を持つ送り手の再送のたびに前の版が積む。spec レビュー R5）。
  `IngestRequest` を受けた直後に `end_time` を µs に切り捨てた値へ置き換える（比べる値と格納する値を 1 つにする）。
  内容が同じで終わりを持たない到着は更新にしない（D18（仮））
  古い更新時刻の版・削除済みの記録は今の `apply_external_update` の分岐のまま止まる
- 外部の識別子の無い記録は `DO NOTHING` のまま（終わりだけ違う到着は畳まれ、後の終わりは入らない —— Q7 の推奨の側で失うもの）
- 削除済みの止め（`event_dedup_hash`・`needs_deleted_check`）は鍵が変わらないのでそのまま効く

これは本人に問う A（`loss: discarded`）で、仮ではない。本人が「終わりも鍵に混ぜる」を選んだら、鍵の式・固定値の試験・削除済みの止め
（終わりを除いた鍵でも照らす仕組み）を spec から書き直す（`spec --answers Q7`）。

### D8（仮）. 期間の境界は両方とも「始まりを含み、終わりを含まない」（Q2 の推奨）

記録の幅は、`end_time > event_time` なら `[event_time, end_time)`、それ以外（終わりが無い・同じ・前）は点 `event_time`。
期間 `[from, to)` との重なりは、幅なら `event_time < to AND end_time > from`、点なら `from <= event_time AND event_time < to`。
長さ 0 を点にするのは、半開区間で始まりと終わりが同じだと空の範囲になり何にも重ならないため（`premises.md` 5 行目）。

反転条件: Q2 で「終わりのある記録だけを返す」なら点の条件を外す。「閉区間」なら幅を `event_time <= to AND end_time >= from`、点を `from <= event_time <= to` にする。

### D9（仮）. 期間で引く口は `GET /events` に足す（Q3 の推奨）

- 問い合わせの欄: `from` / `to`（RFC 3339。UTC からのずれか `Z` を必須にする —— ずれの無い時刻はどの地域の時刻か決まらない）と `source`（論理ソースの名前に一致）。
  `from` と `to` は両方そろって指定する。片方だけ・読めない・`from >= to` は 400（本文は `IngestError` と同じく種別の名前だけ）
- 応答の行 `EventRow` に `end_time` / `end_tz_offset_min` / `end_tz_id` を `#[serde(skip_serializing_if = "Option::is_none")]` で足す。
  終わりを持たない行の鍵は 6 つのまま —— browsing-views の Scenario「記録の読み出しの形は変わらない」は、終わりを持たない記録だけで書かれているので変わらず成り立つ
  （試験 `st25_api_unchanged_events_and_stays_shapes` は `events` の呼び方だけを直す。アサートは変えない）
- `events` は `Query<EventsQuery>` を受ける形になる。呼んでいる試験（`day_records_tests.rs` など）は `Query(EventsQuery::default())` を渡す
- 読み出しの記録（ST28 の `access_log::middleware`）はすべての口に掛かるので、手を足さずに残る（Scenario で確かめる）
- 件数の上限は置かない（いまの `GET /events` にも無い。D16（仮））

反転条件: Q3 で「新しい口を立てる」なら同じ検査と応答で別の経路（例 `GET /events/overlapping`）にし、`/events` は触らない。
「口は作らない」なら spec の「期間に重なる記録を引ける」を DB の読み方の Requirement に書き直し、試験は SQL を直に当てる。

### D10. 期間の重なりは、終わりのある行だけの式の部分索引で引く（note (5)・C5）

```sql
CREATE INDEX IF NOT EXISTS event_end_range ON core.event USING gist
  (tstzrange(event_time, greatest(event_time, end_time), '[]'))
  WHERE end_time IS NOT NULL;
```

- 範囲の列は足さない（NFR-24 は索引の追加を変更に数えない）。`greatest` で始まりに寄せるので逆順の行も入る（D4）
- 部分索引にするのは、いまは終わりを持つ行が 0 件で、移行の中の作成が一瞬で終わるため。全行の式の索引は 120 万行で 29.3 秒、その間 `core.event` への書き込みが待つ
  （`premises.md` 4 行目。`CONCURRENTLY` は移行のトランザクションの中で使えない）
- 引き方: 終わりのある行は `tstzrange(event_time, greatest(event_time, end_time), '[]') && tstzrange($from, $to, '[)')` で索引に当て、D8 の正確な条件で絞る。
  終わりの無い行は `end_time IS NULL AND event_time >= $from AND event_time < $to`。2 つを `UNION ALL` して時刻順に並べる。
  終わりの無い側に当たる索引は無い: `GET /events` は利用者で絞らず、`event_time` を先頭に持つ索引も無い（既存は `(user_id, event_time)` と `(logical_source, event_time)`）。
  `source` を付ければ `(logical_source, event_time)` に当たるが、`source` なしの期間の点の側は全走査になる。期間なしの今の `GET /events` も全件を読むので、今の挙動からの退行ではない。
  索引を足すか利用者で絞るかは、読み出しの量が決まる ST45 で決める（review/code.md R3）
- 下流は 1 万行の偽データ（うち終わりを持つ行 1 割）で `EXPLAIN` を取り、終わりのある側に `event_end_range` が当たることを試験で確かめる（Task 5）

### D11. 書き出しと消した滞在の時間は、終わりを列から先に読む（note (6)・C6・申し送り C7）

- 記録: 幅の (a) の前に列の終わりを置く —— `CASE WHEN r.end_time > r.event_time THEN r.end_time END` を `END_KEYS` の `coalesce` の先頭に足す（列の逆順と長さ 0 は NULL にして次の順へ。D4）
- 前の版: 前の版の行を作る副問い合わせ（いまは `v.payload` / `v.raw` だけを持つ）に `v.end_time` を足し、同じ式で読む
- 消した滞在の時間: `core.stay_erased` の `end_at` を `coalesce(end_time, core.try_timestamptz(payload->>'end'), event_time + interval '1 microsecond')` にし、
  消した滞在の前の版の時間（いまは `v.payload->>'end'`）も `coalesce(v.end_time, …)` にする。滞在は終わりを列に書かない（D13（仮））ので、いまの振る舞いは変わらない
- 列の一覧: `EVENT_COLUMNS` と `EVENT_VERSION_COLUMNS` に 3 列を `event_time` / `tz_id` の近くに足す（ST09 が末尾を触りうる）。説明書 `export/doc.rs` にも同じ説明
- CH02 との順: どちらが先に main に入っても、ST41 の下流は main にある `overlap_sql` に足す。CH02 の窓に載るのは鍵の列だけで（CH02 design D2）、
  終わりは窓の後に `id` で戻した行から読むので、窓の枠には触らない。ST41 が先なら `docs/handoff/CH02.md` に「`legacy` の写しと偽データに終わりの列の行が無い」を書く

### D12（仮）. 日の区切りは Asia/Tokyo のまま、件数は始まりの日に 1 件（Q4 の推奨）

稼働記録の件数は今のまま `event_time` の Asia/Tokyo の日に数える（コードは変えない。Scenario で固定する）。
「その日に何があったか」は、Asia/Tokyo の日の期間で `GET /events` を引けば重なった日すべてに出る（D8）。
1 日の並び（`day_records.rs`）は許可リストに終わりを列に持つソースが無いので、この Story では触らない。
衛星のソースを許可リストに足す Story が、その `Span` を列→内容の鍵の順で読む（`Span::EndKey` に列を足す）。宛先は衛星の記録を書く ST39 で、
`docs/handoff/ST39.md` に「1 日の並びに衛星のソースを足すなら、終わりを列から読み、重なった日すべてに出す（Q4）」を書いた（spec レビュー R9。`deferred ST39`）。

反転条件: Q4 で「記録ごとの TZ で切る」なら、稼働記録の日の計算を `event_time AT TIME ZONE tz_id` に変える（既存の行の日も数え直す）。主観の日（FR-41）は変えない。

### D13（仮）. 滞在の終わりは列に書かない（Q5 の推奨）

`stay_store.rs` は内容の `end` だけを書き続ける。列が NULL なので、前の版を書く `push_version` の列の並びは変えなくてよい（足した列は NULL で入る）。
反転条件: Q5 で「新しく作る滞在から列にも書く」なら、`stay_store.rs` の INSERT と `push_version` に 3 列（終わりの TZ は滞在の始まりの TZ と同じ）を足す。
次の作り直しで既存の滞在に版が 1 つずつ積む（内容の鍵は変わらないが D5 の門の「終わりが変わった」に当たる —— 滞在は派生なので門の外。版を積むのは `stay_store` の判定）。

### D14（仮）. 収集済みの記録の内容にある終わりは列へ写さない（Q6 の推奨）

移行で既存の行に触らない（3 列は NULL のまま）。携帯のアプリ利用の `end`・PC の `range_end` などは、読む側（1 日の並び・書き出し）が今のまま内容の鍵から読む。
反転条件: Q6 で「過去の分も写す」なら、ソースごとの内容の鍵 → 列の写しを移行の外の作業（`stay_store` の作り直しと同じく管理の口）で行い、
収集した記録 1 件ごとに前の版を書く（D5 の門が求める）。

### D15. 移行は 1 つ

`migrations/<作成時刻>_event_end_time.sql` と `.down.sql`、`migrations/ORDER` の末尾へ 1 行。中身の順:

1. `ALTER TABLE core.event ADD COLUMN IF NOT EXISTS …`（3 列）と `core.event_version` に同じ 3 列。CHECK（D1）を `ADD CONSTRAINT … NOT VALID` で足す
   （既存の行は足したばかりの 3 列がすべて NULL なので条件を満たすことが自明で、全行を読む検査は要らない。`NOT VALID` でも以後の INSERT / UPDATE には効く。
   `VALIDATE CONSTRAINT` は移行の中では打たない —— 120 万行を錠を取ったまま読む時間を測っていない。spec レビュー R14）。
   2 回目の当てで落ちないよう、制約の有無を `pg_constraint` で見てから足す
2. `CREATE OR REPLACE VIEW core.event_live AS SELECT * FROM core.event WHERE deleted_at IS NULL`（`SELECT *` の展開を作り直す。末尾に列が増えるだけなので置き換えられる）
3. `core.event_version_live` を列を名指しのまま末尾に 3 列を足して作り直す
4. `core.stay_erased` を D11 の式で作り直す（列の名前と並びは変えない）
5. 門と錠の 2 関数（D5）
6. 部分索引（D10）

`.down.sql` は逆順（索引 → 関数をいまの版に戻す → ビューをいまの形に戻す → CHECK → 列）。`IF NOT EXISTS` / `IF EXISTS` で 2 回当てても落ちない形にする
（`migrate()` は起動のたびに全版を当てる）。

### D16（仮）. 期間の求めに件数の上限を置かない

いまの `GET /events` にも上限が無く、本人の資格情報でしか読めない。反転条件: 衛星の読み出し（ST45）が同じ口を使うと決まったら、その Story で上限と続きの印を足す。

### D17（仮）. 書き出しの「列の終わりを先に読む」は ADDED のまま並べ、(a)〜(d) の文に畳むのは次の change に回す

同じ要件「書き出しは本人が消した場面を出さない」を CH02 が MODIFIED している。ST41 も MODIFIED にすると、後から archive した側が先の側の写しで上書きする。
ADDED の要件の本文に「(a) の前に置く読み方」と書いて、正典の 2 つの要件が食い違わないように読ませる。
反転条件: ST41 の spec を書き直す時点で CH02 が archive 済みなら、ADDED をやめて CH02 の後の正典を MODIFIED し、(a) の文に列の終わりを入れる（spec レビュー R7）。

### D18（仮）. 内容が同じで終わりを持たない到着は、保存済みの終わりを消さない

外部の識別子つきの進行中の版（終わりなし・更新時刻なし）を、応答を取り落とした送り手が再送すると（`lib.rs` の `id_reused` の注記が「正常な経路」と書く再送）、
「終わりが違う」を更新にする D7 だけでは入った終わりが空に戻り、前の版が 1 件積む。終わりの有る → 無いは、内容が違う版のときだけ更新にする。
失わない: 入った終わりも、再送で届いた版（内容は保存済みと同じ）も消えない。費用: 送り手が内容を変えずに終わりだけを取り消す手段が無い（内容の違う版を送る）。
反転条件: 衛星（ST39）が「終わりの取り消し」を内容を変えずに送る形を求めたら、`source_updated_at` を必須にしたうえで、更新時刻の新しい無い版だけを更新にする（spec レビュー R3）。

### D19（仮）. 主張・主観・場所の錠も終わりの 3 列を凍結する（review/code.md R2）

主張（`s01-attribute`）・主観（`s01-feeling`）・場所（`s01-place`）の即時の錠（`reject_claim_rewrite` / `reject_feeling_rewrite` / `reject_place_record_rewrite`）は列を名指しで凍結する。
終わりの 3 列（`end_time` / `end_tz_offset_min` / `end_tz_id`）を足したままにすると、本人が書いた記録の終わりが履歴なしで書き換えられる側に入る
（門 `require_version_or_ledger` は `origin <> 'collected'` を素通しし、取り込み口は origin を見ずに終わりを受ける）。
いまの本体を写し、始まりの `event_time` / `tz_offset_min` / `tz_id` と同じ扱いで 3 列を凍結に足す。ほかの条件は変えない（D5・D15）。`tools/check-immutable.sh` の点呼も 3 列を凍結の側に置く。
なぜ: 凍結する側は後で緩めても何も失わない。開けておくと、履歴なしの書き換えは戻らない。費用は錠の関数 3 つを写して 3 列を足すだけ。
反転条件: authored の記録の終わりを本人が書き直す口を作る Story が、版を残す経路と一緒に緩める。

## Risks / Trade-offs

- [Q7 の推奨の側で、外部の識別子を持たない送り手の後から来た終わりが捨てられる] → 衛星の更新は外部の識別子つきの形（FR-64）。外部の識別子の無い送り手には
  「終わってから送る」を `docs/handoff/ST39.md` で申し送る。捨てたときは `duplicate: true` の受理で返るので、送り手の側からは見えない —— ST39 の契約に書く
- [D9 で `/events` の応答に欄が増える] → 終わりを持つ行にだけ載るので、今の記録だけの読み手には見えない。OpenAPI は再生成して `tools/check-openapi.sh` で固定する
- [CH02 と同じ SQL を触る] → MODIFIED を重ねず ADDED にした。コードの衝突は gate の rebase が拾い、下流は main の形に足す（D11）
- [移行の CHECK が 120 万行を 1 回読む] → `NOT VALID` で足し、全行を読まない（D15。実測 1.4 ms、素の検査つきでも 357.7 ms —— premises.md）。列の追加は既定値が無いので書き直しも起きない
- [逆順の終わりが残る（D4）] → 読む側は使わず点として扱う。読み手が「終わりが始まりより前」を疑う手間は残る（Q1 の推奨が受け入れた側）
- [始まりの側の範囲外のずれ・空の TZ は今も通る] → 既存の穴。ST39 へ申し送り（D2）

## Migration Plan

1. 移行（D15）を当てる。終わりを持つ行が 0 件なので索引の作成は一瞬、CHECK の検査は 1 回の全行の読み
2. サーバを入れ替える。収集アプリは入れ替えない
3. 戻すとき: `.down.sql` を当てる。終わりを持つ行があれば、その終わりは失われる（衛星が書き始める前なら 0 件）
