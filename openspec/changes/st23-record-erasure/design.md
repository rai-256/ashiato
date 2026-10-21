## Context

動機は `proposal.md` の Why。**深掘り第 1 回の答えはまだ無く、推奨の答えで書いた**（`deep.md` の `（答え待ち・推奨: …）`）。推奨と違う答えが返った問いは `spec --answers` で直す。
聞かずに決めたもの C1〜C16 は `defaults.md`、前提は `premises.md`。いまのコードの形（深掘りの手順 5 と `review/deep.md`）:

- **本文を消す操作は無い。** `core.erasure_ledger`（ST03。記録ごとの行・追記のみ・`scope ∈ {event, version}`）に書くコードは無く、`export.rs` の除外の表に名前があるだけ
- 収集した記録の門（`core.require_version_or_ledger`。COMMIT 時の constraint trigger）は**消去の形**（`raw = ''`・`payload = '{}'`・`event_time` と `content_hash` は不変）を台帳の行と前の版の全消去を条件に通す。
  **派生（`origin = 'derived'`）は素通し**（`migrations/202609120944_gates.sql:83-85`）。前の版の門（`core.reject_version_rewrite` / `core.require_ledger_for_erasure`）は origin を問わず、消去の形だけを台帳の行（`event_id` = 親）つきで通す
- ST22 の「消した」: `deletion::erase` が滞在に `deleted_by = 'user'`、その時間の位置・書庫の位置・位置を持つ書庫の項目に `user:cascade` を付け、`core.deletion_ledger`（`cause_event_id` = 滞在）に 1 行ずつ。
  後着は `user:late`。作り直しは `rebuild:absorbed` / `rebuild:erased-range` を付ける
- **消した時間帯を読む所が 7 つあり、どれも滞在の本文から終わりを読む**: `core.stay_erased`（`payload->>'end'`）/ `stay_store::day_view`（ビュー越し）/ `mark_late_arrivals` と `mark_late_for_user`（`s.payload->>'end'` を直読み）/
  `load_existing`（`payload->>'end'` で絞り `span_of`）/ `export/excluded.rs`（前の版の `payload->>'end'` も）/ `deletion.rs`（`span_of`）。本文を消すと 1µs に縮む（premises.md の実測）
- 後着の印は**作り直しの中**（`rebuild_day` の先頭の `mark_late_arrivals`）と書き出しの前（`mark_late_for_user`）と書庫の格納の後（`mark_archive_arrivals`）だけ。`/ingest` は 1 件 1 トランザクション（`ingest_one`）で、その後に作り直す
- サーバは表の所有者でない `ashiato_app` で動く（ST28）。所有者の接続は移行だけ。定期の仕事は書庫の取り込み器と書き出しの job だけで、汎用の定期処理は無い

## Goals / Non-Goals

**Goals**: 「消した」滞在を本文ごと消す口と確認、1 操作 1 行の台帳、本文を消した後も消した時間帯が保たれること、後着の本文も消えること、DB の門が派生にも及ぶこと、ファイルの痕跡を夜に追い出すこと。

**Non-Goals**:
- 主張・場所・気分・PC・ブラウザ・写真を本文ごと消す口（Q1 推奨。新しい Story）。写真の 4 点（ST09 review R8）は写真を見て 1 件消す Story へ（C8）
- 書庫の写しの丸ごとの削除・書き換え（Q4 推奨）。書き出したファイルの削除（C10）。バックアップへの及び方（C9。ST30 へ申し送る）
- 内容の鍵に塩を足す・鍵を消す（C1）。`core.event` への直の INSERT を DB で締める（C11）。ブラウザで消えた訪問を消す（C12）
- WAL の痕跡（外へ写していない。premises.md）。ディスクの空き領域の上書き
- 消去の台帳を見る画面（C3。一覧の行が事実を出す）

## 決定の一覧

| D | 何を | 出所 | 仮か |
|---|---|---|---|
| D1 | 口は確認（`GET /stays/purge/preview`）と消す（`POST /stays/purge`）の 2 つ。消す口は確認で見せた件数を運ぶ | Q1 / Q5 推奨・C7・FR-52 | — |
| D2 | 本文ごと消す範囲の集め方（重なる滞在は作り直しの印のものだけ） | Q2 推奨・C8・C14・spec review R2 | 仮 |
| D3 | 消去の形と、1 つのまとまりの中の順 | C1・C4・FR-51 | — |
| D4 | 消去の操作の台帳と範囲の表。`core.erasure_ledger` に操作の列を足す | C3・R6 | — |
| D5 | 消した時間帯は `core.stay_erased` 1 か所で台帳から読み、読み手を全部そこへ寄せる | C13・R1 | — |
| D6 | 本文を消した滞在の印と、作り直しから外す形・戻す口の断り方 | C2・C5・R18・spec review R3 | 仮 |
| D7 | 派生の門 | C5・R14 | — |
| D8 | 後着の印と消去を、取り込みの口（HTTP）の 1 件のまとまりの中へ移す | Q3 推奨・申し送り R8・FR-50 ★ 2026-09-29・spec review R1 | 仮 |
| D9 | 書庫の後着は格納の後の印付けのまとまりで消し、落ちたら既存の読み直しで消し直す | Q3 推奨・R8・spec review R1 / R17 | 仮 |
| D10 | 確認に出す書庫の写しの数は、ソースで数える（少なく数えない） | Q4 推奨・R9 | 仮 |
| D11 | 夜の書き直しの仕事・時刻・権限・確かめ方 | Q6 推奨・R15 | 仮 |
| D12 | 「本文ごと消した」の区間の形と行の時間 | Q5 推奨・Q3 推奨 | 仮 |
| D13 | 本文ごと消した時間帯は、指定した滞在の時間だけ（一緒に本文を消した滞在の時間で広げない） | Q2 / Q3 推奨の帰結・spec review R4 | 仮 |
| D14 | 読めない範囲の倒し方 | C3・R6 | 仮 |
| D15 | ソースの組の分け方（確認と台帳の件数） | Q5 推奨 | 仮 |
| D16 | 画面の確認の形と文面 | Q5 推奨・C16 | 仮 |
| D17 | 重なる滞在がまだ一覧に出ている（作り直しが落ちた）ときは範囲に入れない | C2・spec review R5 | 仮 |
| D18 | 稼働状況と「後に消した記録」の数え方は変えずに試験で固定する | C6・C15・申し送り R11 | — |

## D1. 口は確認と消すの 2 つ（Q1 / Q5 推奨・C7・FR-52）

- `GET /stays/purge/preview?stay_id=<uuid>[&user_id=<uuid>]` → 200 `PurgePreview { counts: PurgeCounts, range: {start, end}, copies: u32, remains: [...], purged: bool }`。何も書かない。
  `range` は指定した滞在の時間（D13）。すでに本文ごと消した滞在には `counts` をすべて 0・`purged: true` で返す
- `POST /stays/purge { stay_id, user_id?, expected: PurgeCounts }` → 200 `PurgeOutcome { counts: PurgeCounts, operation_id }` / 409 `{ reason: "not_erased" | "counts_changed" | "unknown_source", counts? }`
- `PurgeCounts = { stays, locations, archive_locations, archive_items, versions }`（D15）
- 断り方は ST22 の `POST /stays/erase` と同じ（滞在でない・無い・利用者違い → 404、資格情報なし → 401）。**消えていない滞在・作り直しの印（`rebuild:`）だけの滞在 → 409**（`not_erased`）
- 消す口の順: 錠（`stay_store::LOCK_KEY` × 利用者。作り直し・消す・戻す・取り込みの口の後着の印と同じ）→ 指定した滞在を `FOR UPDATE` で読む →
  **すでに本文ごと消した滞在（`raw = ''`）なら `expected` を比べずに 200・件数 0・台帳に行を足さない**（2 度押しは成功として返す。spec review R9）→
  消えていない・作り直しの印だけなら 409 `not_erased` → 範囲を数え直し、`expected` と 1 つでも違えば何も書かずに 409 `counts_changed` と数え直した件数（C7）
- 範囲にどの組（D15）にも入らない論理ソースがあれば 409 `unknown_source`（spec「組に入らない記録が範囲にあると断られる」）

**なぜ確認を別の口にするか**: FR-52 の「実行前に件数を提示」は、消す口が自分で数えて返すだけでは満たせない（押した後に数が出る）。確認の口と消す口が**同じ関数**で範囲を数える（`deletion::purge_scope`）ので、見せた数と消す数の数え方がずれない。

## D2（仮）. 本文ごと消す範囲の集め方（Q2 推奨・C8・C14）

錠の中で、次の和集合を集める（`deletion::purge_scope(tx, stay_id)`）:

1. **指定した滞在**（`deleted_by = 'user'`・削除の台帳の最新が `erase`）
2. **その滞在を原因とする記録**: `core.deletion_ledger` に `cause_event_id = 指定した滞在` の `erase` の行があり、その記録の最新の台帳の行が `erase` で、いまも削除の印を持つもの
   （`user:cascade` と `user:late`。別の滞在の操作で先に消えていて、ST22 が原因を追記した位置も入る —— その場面の座標なので消す）
3. **その時間に重なる滞在**: `s01-stay` の派生の行のうち、削除の印が**作り直しの印**（`rebuild:absorbed`・`rebuild:erased-range`）で、時間が指定した滞在の時間と**重なる**もの
   （`a.start < b.end AND b.start < a.end`。端が触れるだけは重なりにしない）。本人が別の操作で消した滞在（`user`）と、印の無い滞在（D17）は入れない
4. 1〜3 の**前の版すべて**（`core.event_version` の `raw <> ''`）

入らないもの: 写真と画像のソース・PC・ブラウザ・気分（どれも 2 の連鎖に入らず、3 は滞在だけ）。**集めた中に写真・画像のソース（どの組にも入らないソース）が混ざっていたら、確認の口も消す口も 409 `unknown_source` で何も消さずに断る**（C8・D15。連鎖の規則が将来変わったときに黙って写真を消さない）。
本人が別の滞在で消した位置は、指定した滞在の台帳の行を持たないので 2 に入らない（Scenario「別の操作で消した位置の本文は消えない」）。

**（仮）3 を作り直しの印の滞在に絞る**（spec review R2）: 広く取る側（印を問わず端でも重なる）は、本人が別に消した滞在 B を巻き込んで `raw = ''` にし、
B を原因とする位置は本文を持ったまま「戻せない（B は 409）・本文ごと消せない（B はすでに本文ごと消した）」になる。絞る側は何も失わない ——
B は B の「本文ごと消す」で後から消せる。反転条件: 本人の Q2 の答えが「重なる滞在は印を問わず」を選んだら、B の連鎖（原因 = B の記録）も範囲に入れる形で広げる。

**なぜ台帳の原因で引くか**（時間帯で引き直さない）: 確認で見せる件数は「そのとき『消した』にした記録」で、時間帯で引き直すと、基準のソースが変わった・書庫の位置が後から入った、で数がずれる。
後着は ST22 が原因つきで台帳に積んでいるので、原因で引けば漏れない。

## D3. 消去の形と、1 つのまとまりの中の順（C1・C4・FR-51）

1 つのトランザクションで:

1. 錠（`pg_advisory_xact_lock(LOCK_KEY, hashtext(user_id))`）→ 指定した滞在の行を `FOR UPDATE` で読み、D1 の条件 → D2 で範囲を集めて数え、`expected` と比べる
2. 範囲の滞在の時間（消す前の本文から `span_of`。前の版は前の版の本文から）を控える
3. `core.erasure_operation` に 1 行（D4）
4. 記録ごとに `core.erasure_ledger`（`scope = 'event'`、`operation_id`）、前の版ごとに `scope = 'version'`（`event_id` は親。既存の版の門がこの形で通す）
5. 前の版を消去の形に（`UPDATE core.event_version SET raw = '', payload = '{}'`）→ 記録を消去の形に（`UPDATE core.event SET raw = '', payload = '{}'`）。`event_time`・`content_hash`・`device_id`・`received_at`・削除の印と時刻は書き換えない
6. 重なる滞在で印が本人の印でないもの（`rebuild:*`・印なし）を `user:purged` に付け替え、`core.deletion_ledger` に `erase`・原因 = 指定した滞在・`mark = 'user:purged'` を 1 行ずつ（D6）
7. `core.erasure_span` に、本文を消した滞在それぞれの時間（`scope = 'event'`）と前の版の時間（`scope = 'version'`）を 1 行ずつ（D4）
8. commit。COMMIT 時の門（D7 と既存）が台帳の行と前の版の全消去を確かめる

落ちたら何も残らない（C4）。commit の後に作り直しは**しない** —— 範囲の記録はすでに削除の印を持ち、作り直しの入力（`event_live`）に入っていない。

**原文の空は `''`・解析済みの空は `{}`**: 既存の門と `ingest.rs` が「空の原文は受け口で断る」を前提にこれを消去の印にしている（ST03 R51 / R95）。別の値にすると全部の門と読み手を書き直すことになる。

## D4. 消去の操作の台帳と範囲の表（C3・R6）

移行 1 本（`migrations/YYYYMMDDHHMM_record_purge.sql` と `.down.sql`）:

| 表 | 列 | 何か |
|---|---|---|
| `core.erasure_operation` | `id uuid PK`・`user_id`・`kind text CHECK IN ('user','late')`・`parent_id uuid NULL`（`late` のとき元の操作）・`stay_id uuid`（指定した滞在）・`start_at timestamptz`・`end_at timestamptz`（D13 の和の外側）・`stays`・`locations`・`archive_locations`・`archive_items`・`versions`（`integer NOT NULL CHECK >= 0`）・`at timestamptz DEFAULT now()`・`txid xid8 DEFAULT pg_current_xact_id()` | 1 操作 1 行 |
| `core.erasure_span` | `operation_id uuid`・`event_id uuid`・`scope text CHECK IN ('event','version')`・`version_no integer NULL`・`start_at`・`end_at`（`CHECK end_at >= start_at`） | 本文を消した滞在と前の版の時間 |
| `core.erasure_ledger` | `operation_id uuid NULL` を足す（**既存の行は NULL のまま**。ST19 / ST21 / ST17 の門は `operation_id` を見ない） | 記録ごとの行を操作に結ぶ |

- 3 つとも**追記のみ**: 新しい 2 表に `BEFORE UPDATE OR DELETE` の行トリガと `BEFORE TRUNCATE` の文トリガ（関数は表ごとに別。`deletion_ledger` の移行の注記どおり、共用すると古い down が当たらなくなる）。`erasure_ledger` の既存のトリガはそのまま効く
- 時刻は `timestamptz` のまま（丸めない。C3）。**行の形を固定の値で試験する**（R6: 列名・型・CHECK を `information_schema` から読み、期待の表と一致すること）
- 後着の消去（D8 / D9）は `kind = 'late'`・`parent_id` = 元の操作・件数はその 1 回分の行を足す（元の行は書き換えない。凍結した件数は増やせない）。「後から届いた N 件」は `kind = 'late' AND parent_id = <元>` の件数の和
- 書き出しの表の分類（`export.rs` の表）に **3 表**を `Exclude` で足す: `core.erasure_operation` と `core.erasure_span`（「消したことの台帳（痕跡を残さない）」）、
  `core.purge_compaction`（D11。「書き直しの運用の記録（記録の値を持たない）」）。分類に無い表があると既存の試験が落ちる

**なぜ操作の行を別の表にするか**: 記録ごとの台帳は記録の数だけ行があり、件数と範囲を持たせると 1 操作の事実が N 行に複製される（どれが正か分からない）。FR-51 の「事実・時刻・件数・範囲」は操作の単位。

## D5. 消した時間帯は `core.stay_erased` 1 か所で読む（C13・R1）

`core.stay_erased` を作り直す: `start_at` / `end_at` を **`core.erasure_span`（`scope = 'event'`・`event_id` = その滞在）があればそこから、無ければ本文から**読み、列 `purged boolean`（`raw = ''`）と `operation_id` を足す。
**`deleted_by = 'user:purged'`（重なって一緒に本文を消した滞在。D6）はビューに出さない** —— 本文ごと消した時間帯は指定した滞在の時間だけ（D13）で、
吸収された滞在の時間を窓にすると、はみ出しに届いた位置に戻せない後着の印が付く（spec review R4）。範囲の行が無い本文を消した滞在は D14 の読み方で出す。
読み手を全部このビュー（か、ビューと同じ式を持つ SQL 関数 `core.stay_span(id)`）へ寄せる:

| 読み手 | いま | 後 |
|---|---|---|
| `stay_store::day_view` | ビュー | ビュー（`purged` で区間を分ける。D12） |
| `mark_late_arrivals` / `mark_late_for_user` | `s.payload->>'end'` を直読み | ビューの `end_at`（`user_erased_stay_sql` をビュー越しに） |
| `load_existing` | `payload->>'end'` で絞り `span_of` | 本文を消した滞在は候補から外し、その時間（ビュー）を `erased` の列に足す（D6） |
| `export/excluded.rs` | ビュー＋前の版の `payload->>'end'` | ビュー＋前の版は `core.erasure_span`（`scope = 'version'`）か本文 |
| `deletion::erase` / `restore` | `span_of`（本文） | 本文を消した滞在には来ない（erase は削除済みを 0 件で返す・restore は D6 で断る） |
| `lib.rs` の `stays_detail_get` | `event_live` 越しの `payload->>'end'` | **変えない**（生きた滞在だけを読む。本文を消した滞在は `event_live` に入らない） |
| `load_existing` / `day_view` の粗い絞り | 生きた滞在を索引で絞る `payload->>'end'` | **変えない**（生きた滞在の本文。消した時間帯の読みではない） |

**機械で確かめる**（spec review R6）: 消した時間帯を読む SQL —— `core.stay_erased` を読む式と、`deleted_at IS NOT NULL` の滞在を読む SQL —— に `payload->>'end'` が無いこと。
`payload->>'end'` の出現は許可の一覧（`stay_store.rs` の `load_existing` と `day_view` の生きた滞在の絞り・`lib.rs` の `stays_detail_get`・`export/excluded.rs` の前の版の本文の補い・コメント）と
`file:関数` で突き合わせる試験（`purge_range_reader_allowlist`）にする。`grep -v 'fn span_of'` の形は何も除かない（`span_of` は `payload.get("end")`）ので使わない。

## D6（仮）. 本文を消した滞在の印と、作り直しから外す形・戻す口の断り方（C2・C5・R18・spec review R3）

- 指定した滞在の印は `user` のまま。重なって一緒に本文を消した滞在（D2 の 3。作り直しの印）は **`user:purged`** に付け替え、削除の台帳に `erase` の行（原因 = 指定した滞在・`mark = 'user:purged'`）を積む
  （正典「消す・戻すは追記のみの台帳に 1 操作 1 行で残る」の「印を付ける」に当たる）。
  付け替えるのは、`rebuild:` の印のままだと、作り直しが「もう吸収されていない」と読んで印を外し、本文の空な滞在が一覧に戻るため。
  `core.event.deleted_by` と `core.deletion_ledger.mark` は値を縛る CHECK を持たない（`migrations/202609081618_envelope.sql:34`・`migrations/202609271716_deletion_ledger.sql:12`）ので、移行で許す値を足す必要は無い。
  `user:purged` は ST33 の D17（後に消した記録の数）には削除の印として入る。**消した時間帯の窓（`core.stay_erased`・`user_erased_stay_sql`）には入れない**（D5 / D13）
- `load_existing` は **`raw = ''` の滞在を一致の候補に入れない**。その時間はビューから `erased` の列に足すので、重なる新しい滞在は `rebuild:erased-range` で隠れる（FR-50 ★ 2026-09-13）
- `deletion::restore` は、戻すことになる記録（指定した滞在と、それを原因に消えた記録のうち、**別の原因で消えたまま残るもの（いまの `blocked_by_other_cause`）を除いたもの**）に
  `raw = ''` が 1 件でもあれば、何も書かずに 409（`purged_restore`）。HTTP の口は 409 を返す（spec review R3。境界の位置を共有する隣の「消した」を戻せなくしない）

**（仮）の中身は戻す口の断り方**: 409 の数から別の原因で消えたまま残る記録を除くのは、ST22 の「別の操作で消えた記録は戻さない」と同じ読み方で、境界の 1 件のために隣の「消した」の「戻す」が効かなくなる（行は戻すための識別子を出し続ける）のを避けるため。
反転条件: 本人が「本文を消した記録に触れる戻す操作は、境界の 1 件でもすべて断る」を選んだら、409 の数に戻し、1 日の行は境界を共有する「消した」から戻すための識別子を外す。

## D7. 派生の門（C5・R14）

新しい COMMIT 時の constraint trigger `event_requires_derived_ledger`（関数 `core.require_derived_erasure_ledger()`）を `core.event` の `AFTER UPDATE` に置く。`OLD.origin = 'derived'` の行だけを見る:

- `OLD.raw = '' AND NEW.raw <> ''` → 拒む（本文の書き戻し）
- 消去の形（収集した記録の門と同じ式）→ その行の `core.erasure_ledger`（`event_id = NEW.id AND scope = 'event' AND txid = 現在`）が無ければ拒む。その行の前の版に `raw <> ''` が残っていれば拒む
- `OLD.raw <> '' AND NEW.raw = ''` で消去の形でないもの（`payload` を残す・`event_time` や `content_hash` を変える）→ 拒む（spec review R10。`raw = ''` を「本文を消した」の印として読む D5 / D6 を守る。収集した記録の門の R95 と同じ形）
- それ以外（作り直しの書き換え・削除の印・感度）→ 通す

**既存の `core.require_version_or_ledger` は変えない**（`record-envelope` の門。名前と範囲が「収集した記録」）。派生の門を別の関数にするのは、収集した記録の門の試験（ST03）を動かさないため。
門は `tools/check-immutable.d/<時刻>_st23-derived-erasure.sh` で、取り込み口の外（psql）から確かめる。

## D8（仮）. 後着の印と消去を、取り込みの口（HTTP）の 1 件のまとまりの中へ（Q3 推奨・申し送り R8・FR-50 ★ 2026-09-29）

**取り込みの口（HTTP の `/ingest`）から入った記録だけ**、記録を入れた直後（commit の前）に `stay_store::settle_arrival(tx, &stored)` を呼ぶ。
トランザクションを開くのは `ingest_request`（`lib.rs` の `.begin()`）で、書庫の取り込み器も `PgSink::store → store_with → ingest_request` で同じ関数を通る（spec review R1）。
そのため `ingest_request` に「後着をここで片付けるか」の引数（`Settle::InTx` / `Settle::Later`）を足し、`ingest_one`（HTTP）は `InTx`、`store_with`（書庫・内部の格納）は `Later` を渡す。
書庫の 1 件ごと（百万件級）に錠を取らないため。書庫の後着は D9:

1. 記録が位置（基準のソース ∪ 書庫の位置）か、位置を持つ書庫の項目でなければ何もしない
2. 錠を取る（作り直し・消す操作と直列にする。取らないと、消す操作の commit と取り込みの commit が互いに見えずに印が漏れる）
3. その時刻が本人の消した滞在の時間帯（D5）に入れば、`user:late` を付けて削除の台帳に 1 行（原因 = その滞在）。**ここで ST22 の印が作り直しの成否から外れる**
4. さらにその滞在が本文を消した滞在（`purged`）なら、同じまとまりで本文を消し、`core.erasure_operation`（`kind = 'late'`・`parent_id`）と `core.erasure_ledger` の行を積む

`rebuild_day` の先頭の `mark_late_arrivals` は残す（取り込み以外の経路と、この変更より前に入った行の拾い直し）。応答は変えない。
**錠の費用**: 位置（と位置を持つ書庫の項目）の取り込み 1 件ごとに錠を取る（1 で外れる記録は取らない）。作り直しの最中の取り込みは待つ（いまも取り込みの後に同じ錠で作り直している）。
**（仮）の反転条件**: `stay.rebuild_slow` のログで取り込みの待ちが目立てば「本人が消した滞在が無い利用者は錠を取らない」に絞る。書庫も同じまとまりで消すべきと分かれば（本人の Q3 の答え）、`store_with` も `InTx` にして書庫の取り込みの錠の費用を測る。

## D9（仮）. 書庫の後着は格納の後の印付けのまとまりで消し、落ちたら既存の読み直しで消し直す（Q3 推奨・R8）

- `stay_store::mark_archive_arrivals`（書庫の格納の後。別のまとまり。錠を取る）が、印に加えて、本文を消した時間帯に入る行の本文を消す（D8 の 4 と同じ関数を使う）。
  `archive/worker.rs` は**変えない** —— 呼び出しは 2 か所（明示の取り込みと見回り）あり、どちらも既存の呼び出しがこの関数を通る
- 落ちたら、既存の仕組みがそのまま消し直す: `mark_archive_arrivals` が Err を返すと、取り込み器は `read` の台帳の行を書かず書庫を置き場に残し、次の見回りが書庫を読み直して
  （格納は内容の鍵で増えない）同じ時間の範囲で `mark_archive_arrivals` を呼び直す（ST12 final review R72・code-verify R80）
- **ST13 の直す取り込み器の台帳のまとまり（R25）と `archive/worker.rs` には触らない**（spec review R17）

**（仮）**: 見回りごとの別の掃除（本文を消した時間帯に本文が残る行を探す）は置かない。反転条件: 読み直しで印付けが呼び直されない経路が見つかったら（試験「書庫の後着の消去に失敗しても次の見回りで消える」が落ちる）、
`archive/worker.rs` の 2 か所の呼び出しの隣に `settle_purged_ranges(pool, user)` を足す（ST13 の merge の後に rebase で）。

## D10（仮）. 確認に出す書庫の写しの数は、ソースで数える（Q4 推奨・R9）

記録 1 件から写しへの結びは無い（`core.event` は写しを指さない。写し 1 つ = 製品のファイル 1 つ）。範囲の書庫の記録の論理ソースを読む写し（`core.archive_file` のうち、置き場にファイルが残っているもの）を数える。
**多く数える側**: 範囲の記録を含まない写しも数えうるが、「0 個」と出して実は残っていた、は起きない。
反転条件: 記録に写しの識別子を持たせる Story が来たら、記録から引いて正確に数える。

## D11（仮）. 夜の書き直し（Q6 推奨・R15）

- **仕事**: `crates/server/src/purge_compaction.rs`。`run()` で取り込み器の隣に `tokio::spawn` する。毎日 03:30（Asia/Tokyo）に 1 回。時刻は試験で差し込める（壁時計を直に読まない）
- **対象**: `core.event` と `core.event_version` のうち、その表の最後の成功の後に `core.erasure_operation` の行がある表だけ
- **書き出しを待つ**: 書き出しの job が走っている間は始めず、1 分ごとに見直す（書き出しの長い読み出しの後ろで `ACCESS EXCLUSIVE` を待つと、後続の読み書きがすべて止まる）
- **権限**: 移行で `GRANT MAINTAIN ON core.event, core.event_version TO ashiato_app`（PostgreSQL 17）。`.down.sql` で `REVOKE`。`grants.sql`（ST28 の共有ファイル）には足さない —— 表ごとの付与は移行の中で済み、`ALL TABLES` に広げない
- **確かめ方**: `pg_class.relfilenode` を前後で比べ、**替わらなければ失敗**（権限が無いと `VACUUM FULL` は警告だけで成功として返る。premises.md）。`SET lock_timeout = '5s'` を超えたら失敗
- **残し方**: `core.purge_compaction`（表の名前・始まり・終わり・`outcome ∈ {rewritten, not_rewritten, lock_timeout, failed}`・SQLSTATE）に 1 行。値を含めない。失敗した表は「最後の成功」が進まないので次の晩にもう一度
- **確認の文面**の「今夜の書き直しで消える」はこの時刻に合わせる（D16）

反転条件: 実物の量（seed の最大）で書き直しが 5 分を超える・夜中にも取り込みが詰まる → 対象を部分ごとに分ける（表の分割は別の Story）か、Q6 を本人に聞き直す。待つ時間は tasks で測って PR に書く。

## D12（仮）. 「本文ごと消した」の区間の形と行の時間（Q5 / Q3 推奨）

- `DayEntry` の `kind = Erased` に `purged: true`・`purge_stay_id`・`late_purged: u32` を足し、`stay_ids` を出さない。既存の項目は変えない（`stays.ts` の型に足す）
- 区間の時間 = 指定した滞在の時間（D5 / D13）。**本文を消していない「消した」とはまとめない**（まとめると「戻す」が 409 になる）。重なる部分は「本文ごと消した」が持ち、「消した」の区間はその分を差し引く
- 「後から届いた N 件も本文ごと消した」は `late_purged > 0` のときだけ行に添える

## D13（仮）. 本文ごと消した時間帯は、指定した滞在の時間だけ（Q2 / Q3 推奨の帰結・spec review R4）

本文ごと消した時間帯（後着の本文を消す窓・1 日の行・書き出しの除外・作り直しが滞在を作らない窓）は、**指定した滞在の、本文を消した時点の時間だけ**にする。
重なって一緒に本文を消した滞在（`user:purged`）の時間で広げない（D5 のビューに出さない）。

- 広げる側（和）は、指定した滞在の時間の外へはみ出した分に後から届いた位置の本文を、本人に見せた範囲の外で消す。さらに次の作り直しで、はみ出しの時間にかかる生きた滞在も隠れる
- 広げない側の費用: はみ出しの時間に後から届いた位置は生きた記録として入り、一覧に出る（失うものは無い。本人は出てきた滞在を ST22 の口で消し、本文ごと消せる）
- spec の Scenario「一緒に本文を消した滞在のはみ出しは時間帯に入らない」で固定する。確認の `range` も指定した滞在の時間で出す（D1）

反転条件: 本人の答え（Q2・Q3）が「吸収された滞在の時間も含めて消す」を選んだら、窓を `core.erasure_span` の `scope = 'event'` の和に広げ、確認の文面に和の範囲を出す。

## D14（仮）. 読めない範囲の倒し方（C3・R6）

本文を消した滞在（`raw = ''`）に `core.erasure_span` の行が無い（移行の前の手作業・壊れた行）ときは、その滞在の出来事の時刻の日（Asia/Tokyo）の 00:00〜24:00 を範囲とみなす（書き出しから除く・作り直しで隠す・1 日の一覧の行）。
**後着の本文は消さず、印だけ付ける**（消す側に倒すと、見せていない記録の本文が戻らない）。行が無いことは起動時に件数だけログに出す。
観測できる振る舞いは spec の「本文ごと消した時間帯は、台帳の範囲から読む」の Requirement と Scenario 2 本（「範囲の行が無い…」）が持つ（spec review R7）。
反転条件: 範囲の行が無い滞在が実際に見つかり、その日の生きた記録が書き出しから落ちて困ったら、範囲を本文の出来事の時刻の前後の位置の記録から推す形にする。

## D15（仮）. ソースの組の分け方

| 組 | 論理ソース |
|---|---|
| 滞在 | `s01-stay` |
| 位置 | 基準のソース（`c01-location` など。書庫の位置を除く） |
| 書庫の位置 | `archive::LOCATION_SOURCES` |
| 書庫の項目 | `stay_store::item_sources`（マイアクティビティ・YouTube・Chrome） |
| 前の版 | 上の全部の `core.event_version`（本文が空でないもの） |

どの組にも入らない論理ソースが範囲に入ったら、確認の口も消す口も 409 `unknown_source` で断る（D2 の写真の断言と同じ。黙って数えない。spec「組に入らない記録が範囲にあると断られる」）。

## D16（仮）. 画面の確認の形と文面（Q5 推奨・C16）

- 「消した」行の操作は「戻す」「本文ごと消す」の順。どちらも 44×44 px 以上。押すと行の下に `role="alertdialog"` で確認を開き、確認の口（D1）を読む
- 文面: 「10:05 – 11:48 の本文を消します。戻せません。消えるもの: 滞在 N 件・位置 N 件・書庫の位置 N 件・書庫の項目 N 件（前の版 N 件）。消えずに残るもの: 書庫の写し N 個・本人のフォルダの書庫・書き出したファイル。ディスクの痕跡は次の書き直し（毎晩 3:30）で消えます。」
  （時刻は確認の口の `range`。「今夜」は 00:00〜03:30 に消すとずれるので使わない。spec review R20）
  0 件の組も出す（数え方が見える）。ボタンは「本文ごと消す」「やめる」
- 409 `counts_changed` のときは文面を数え直した件数で出し直し、「件数が変わりました」を添える
- ST22 の「この滞在を消す」の確認は変えない（C16）。並ぶ理由: こちらは戻せない操作で、内訳と残るものを見せる。戻せる削除の確認は ST22 で本人が選んだ形
- 文字列は `web/src/stays.ts` に置く（`messages` の流儀に合わせる）

## D17（仮）. 作り直しが落ちて重なる滞在が一覧に出ているときは範囲に入れない（C2・spec review R5）

ST22 は位置を消した後に作り直すが、作り直しが落ちると、消した位置から作られた滞在が印なしで残る（spec「作り直しが失敗しても消したことは残る」）。
その滞在は一覧に出ている（本人が「消した」にしていない）ので、D2 の 3 に入れない。入れる側は、本人が見えている滞在の本文を確認の「滞在 N 件」に混ぜて戻せなくする。
入れない側の費用: その滞在は消した場面の座標を持ったまま残るが、本人は一覧でそれを見て ST22 の口で消し、本文ごと消せる（失うものは無い）。spec の Scenario「一覧に出ている滞在の本文は消えない」で固定する。
反転条件: 本人が Q2 で「印の無い重なる滞在も一緒に消す」を選んだら入れる。

## D18. 稼働状況と「後に消した記録」の数え方は変えない（C6・C15・申し送り R11）

稼働状況（`coverage`）は `core.event` の行を削除の印も本文も見ずに数える。書き出しの「後に消した記録」（ST33 D17）は削除の印で数える。どちらも本文を消しても行と印が残るので、**コードは変えず試験で固定する**。
ST22 の申し送り R11（記録を消した日の隣の記録の無い日の状態）も同じ試験で固める。`collection-coverage` / `data-durability` の正典には書かない（proposal の Impact）。

## Risks / Trade-offs

- [位置の取り込み 1 件ごとに錠を取る（D8）] → 作り直しの最中の取り込みが待つ。`stay.rebuild_slow` のログで見張り、遅ければ「本人が消した滞在が無い利用者は錠を取らない」に絞る。書庫の取り込みは取らない（D8 / D9）
- [夜の書き直しの間、表への読み書きが止まる（D11）] → 夜中に寄せ、書き出しの後に回し、`lock_timeout` で長く待たない。実物の量の時間は tasks で測る（premises.md の確かさ 1）
- [`MAINTAIN` を足すと、サーバの役割が表を書き直せる] → 付与は 2 表だけ。`VACUUM FULL` は中身を変えない。ST28 の「TRUNCATE / REFERENCES / TRIGGER は付けない」は守る（`app_role_tests.rs` に `MAINTAIN` の 2 表だけを足す）
- [確認の文面（D16）が proto の確認と違う（範囲の時刻・「次の書き直し（毎晩 3:30）」）] → proto は Q5 の相談の形。reading の段の GO view が spec に合わせる
- [書庫の後着は格納から消去までの間、本文が DB にある（D9）] → Q3 の推奨の detail に書いてある（本人が読んで選ぶ）。見回り（既定 120 秒）ごとに消し直す
- [WAL に本文が残る] → 外へ写していない。再利用で上書きされる。ST30（バックアップ）へ申し送る（C9）
- [内容の鍵（塩なし SHA-256）が残り、候補の原文を持つ人は照らせる] → C1。鍵に塩を足すのは全行の書き直しで、この Story の範囲を越える

## Migration Plan

- 移行 1 本: 2 表・`erasure_ledger.operation_id`・追記のみのトリガ・`core.stay_erased` の作り直し・派生の門・`core.purge_compaction`・`MAINTAIN` の付与。`migrations/ORDER` の末尾へ 1 行。`.down.sql` は逆順に落とす（`erasure_ledger` の列は落とす）
- 既存のデータは書き換えない（本文を消した滞在はまだ 1 件も無い）。`core.stay_erased` の範囲は span が無ければ本文から読むので、移行の前後で消した時間帯は変わらない
- 戻すときは `.down.sql`。`.down.sql` は `core.stay_erased` を元の定義（終わりは `payload->>'end'`、無ければ +1µs）に戻すので、**本文を消した滞在が 1 件でもあれば範囲が 1µs に縮む**（premises.md の実測と同じ形。D14 では読まれない）。
  本文を消した滞在があるときは down しない（`.down.sql` の先頭で `core.erasure_operation` に行があれば `RAISE EXCEPTION` で止める）
