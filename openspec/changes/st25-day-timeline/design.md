# ST25 設計 — 1 日を時刻順に見る

## Context

動機は `proposal.md` の Why。振る舞いの契約は `specs/browsing-views/spec.md`。本人の決定（Q1〜Q6）と聞かずに決めた既定（C1〜C11）は `deep.md`。
独立レビューの指摘と処置は `review/spec.md`。**ここで決めるのは、それを実装に落とす技術判断だけ。**

いまの形（2026-10-01 に読んだもの）:

| 場所 | いまの形 |
|---|---|
| `GET /events`（`lib.rs`） | `core.event_live` の**全件**を絞らずに返す。`payload`・`device_id` を返さない。ST01 の読み出し経路で、変えない |
| `GET /stays`（`stay_store::day_view`） | 滞在・移動・記録なし（ST22 が「消した」を足す）。記録なしは位置の点の間隔だけから作り、稼働記録を読まない |
| `core.event_folded` | `event_live` を内容ハッシュで畳むビュー。**日付の条件が畳んだ後に掛かり、`device_id`・`tz_*`・`ingest_time` を持たない**。本番では未使用（試験だけ） |
| `c02-window`（ST07。main） | `payload.kind` が `foreground`（`app_name` / `process_name` / `title` / `url`）/ `idle`（`transition` enter・leave。**leave が区間を持つ**: `event_time` = 始まり、`range_end` = 終わり）/ `powered-off`（`event_time` = 始まり、`range_end`）/ `excluded`（`excluded_count`、`range_end`。本文なし。**送信の契機（5 分）ごとに区間を切り、数えが 0 の続きは記録にしない** —— `engine.rs` の `close_excluded`）/ `clock-skew`。前景が変わらない間は記録が出ない（離席から戻っても、前景が同じなら記録は無い） |
| `c02-browser-history`（ST08。下流） | `payload.kind` が `visit` / `vanished` / `excluded` / `profiles`。訪問は `at`・`browser`・`url`・`title`。**鍵名の一部が design と枝の実装で違う**（`profile_dir` と `profile` など）。どちらにも `url`・`title`・`browser`・`at` はある。**どこまで読んだかの印はサーバに無い**（PC の置き場にだけある） |
| `c01-app-usage`（ST06。下流） | 取得元のイベント 1 件 = 1 記録。`kind` の無い記録はイベント（`package`・`event_type`・`app_label`）、`kind: "gap"` は取りこぼした期間（`begin`・`end`・`reason` が `retention` か `clock_skew_abandoned`。**2 つを混ぜない**のが ST06 の契約。**`event_time` は終わり**） |
| `c03-*`（ST12。下流） | 書庫。**枝の移行が登録する名前**（`feat/st12-archive-ingestion` の `202609181600_archive_ingestion.sql`）で、活動は `c03-chrome-history` / `c03-youtube-watch` / `c03-youtube-search` / `c03-myactivity-*` / `c03-timeline-visit` / `c03-timeline-move` / `c03-legacy-visit` / `c03-legacy-activity`、位置の点は `c03-timeline-route` / `c03-timeline-signal` / `c03-legacy-location`。**ST12 の design.md は `-activity` / `-path` と書いていて、ST12 の中で design と実装が違う**（`docs/handoff/ST12.md`） |
| 稼働記録 | 収集を始めた日 `core.source.collection_started_on` / 止めていた時間 `core.coverage_span`（`kind='stopped'`。**書く経路は ST15 まで無い**）/ 端末の破棄 `core.drop_report` と `core.drop_report_hour`（ST04）と旧形の `coverage_span kind='dropped'` / 生存信号 `core.heartbeat` |
| 画面 | `Root.tsx` がルート = 稼働状況、`#/day/…` = 1 日の画面、`#/master` = マスタ管理。暦に無い日付の画面は `SCHEMES.dark` 固定。ST22 が `DayView.tsx` の滞在の行を「開ける行」にし、`GET /stays/detail` で件数を出す |

## Goals / Non-Goals

**Goals**

- spec の要件を、**ST22 が書き換えた `browsing-views` の要件に触れずに**成り立たせる（ADDED と、ST22 が触らない「日を移れる」の置き換えだけ）
- 要約（閉じた行）と中身（開いた行）を**同じ正規化から**作る —— 要約の件数と開いたときの件数がずれない
- まだ main に無いソース（ST06 / ST08 / ST12）を、許可リストの 1 か所を見れば追える形にする

**Non-Goals**（proposal の Non-Goals に足すもの）

- `/events` の形を変える・日付で絞る引数を足すこと（C3。新しい口を足す）
- `core.event_folded` を作り直すこと（D3。使わない）
- ブラウザ履歴の「どこまで読んだか」の印をサーバに足すこと（`desktop-collection` の振る舞いで ST08 の範囲。D10 は今ある材料で近似する）
- 止めていた時間を書く口（ST15）。読む側だけを作る（D11）

## Decisions

### D1. 口は 2 本足す。`GET /day`（1 日の並び＋要約＋取れていない時間＋事情）と `GET /day/records`（記録を 1 件ずつ）

| 口 | 引数 | 応答 |
|---|---|---|
| `GET /day` | `date`・`user_id?` | `{date, criteria, entries: [<day_view の行> + summary + reasons], gaps: [...], browser_through?}` |
| `GET /day/records` | `date`・`from?`・`to?`・`user_id?` | `[{id, kind, source, display_name, start, end?, tz_id, app?, title?, domain?, count?}]` |

- `GET /day` の `entries` は **`stay_store::day_view` の結果そのもの**（ST22 の「消した」を含む）に、行ごとの `summary`（D7）と、記録なしの行の `reasons`（D11）を足したもの。
  画面は `/stays` の代わりにこれを 1 回読む。**`/stays` は変えない**（ST22 の契約と試験がそのまま残る）
- `GET /day/records` は spec「日付を指定してその日の記録を 1 件ずつ読み出せる」の口。画面は行を開いたときに、その行の範囲を `from`/`to` で読む（D8）。
  ST26（検索）が同じ口を使える
- 認証・利用者の取り方・日付の解釈は `/stays` と同じ（`authorize()`、`user_id` を省くと既定、`invalid_date` で 400）。
  ST28 の読み出しの記録（middleware）にはそのまま乗る
- 応答の型は `utoipa` で OpenAPI に出す（`tools/check-openapi.sh`）

> 採らなかった案: 要約を画面で計算する（`/day/records` を 1 日ぶん読む）。1 日 3,000〜4,000 件（deep.md 手順 1）を毎回送ることになり、
> 開かれない行のぶんまで題名と URL を画面に運ぶ。採らなかった案 2: `/stays` の応答に要約を足す。ST22 の下流が同じ応答を書き換えている最中で、
> 2 本の Story が同じ型を同時に変えることになる。

### D2. 何を記録として出すかは**許可リスト 1 つ**で決める（C9）

`crates/server/src/day_records.rs`（新規）の 1 つの表に、論理ソースと payload の条件と、正規化した形への写し方を置く:

| 論理ソース | 条件 | 種類（`kind`） | `source` | アプリ / 題名 / ドメイン |
|---|---|---|---|---|
| `c02-window` | `kind = foreground` | `foreground` | `pc` | `app_name`（無ければ `process_name`）/ `title` / `url` のホスト |
| `c02-window` | `kind = idle` かつ `transition = leave` | `away` | `pc` | —（終わり = `range_end`） |
| `c02-window` | `kind = powered-off` | `stopped` | `pc` | —（終わり = `range_end`） |
| `c02-window` | `kind = excluded` | `excluded` | `pc` | **本文を写さない**。`excluded_count` と終わり |
| `c02-browser-history` | `kind = visit` | `visit` | `browser` | `browser` / `title` / `url` のホスト |
| `c01-app-usage` | `kind` が無く `event_type = 1`（前景に出た） | `foreground` | `phone` | `app_label`（無ければ `package`） |
| `c01-app-usage` | `kind` が無く `event_type` が 2 / 23（後ろに下がった） | `background`（**内部だけ**。D6 の区切りに使い、`/day/records` には出さない） | `phone` | `package` |
| `c01-app-usage` | `kind = gap` | `unavailable`（`reason` を写す） | `phone` | —（始まり = `begin`、終わり = `end`） |
| `c03-chrome-history` / `c03-youtube-watch` / `c03-youtube-search` / `c03-myactivity-*` / `c03-timeline-visit` / `c03-timeline-move` / `c03-legacy-visit` / `c03-legacy-activity` | なし | `archive` | `archive` | `product`（マイアクティビティだけ）/ `title`（無ければ `query`・`semantic_type`・`activity_type`）/ `url` のホスト |

- **表に無いものは出さない**（`c01-location`・`s01-stay`・`s01-attribute`・`c01-app-usage-rollup`・`c01-clock`・`clock-skew`・書庫の位置の点・ブラウザの `vanished` / `excluded` / `profiles`・スマホの前景と後ろに下がった以外のイベント）。
  **正規化は 2 段**: 1 段目（内部の形）は D6 が使う `background` を持ち、2 段目で `/day/records` に出すものを選ぶ。要約（D7）は 1 段目を読む
  既定は厳しい側 —— ST06 は種別をふるい落とさずに積むので、許可リストでないと新しい種別が黙って行に混ざる
- **離席は leave の記録だけを使う**（区間は leave が持つ。ST07 の契約）。enter だけがあって leave がまだ無い離席（今日のいま離席中）は、**今日に限り** enter の時刻からいままでの離席として出す
- 書庫の `display_name` は登録簿（`core.source.display_name`）から引き、画面は「書庫: <表示名>」と書く（C1）
- **ST06 / ST08 / ST12 の payload の鍵名は、それぞれの下流が main に入れた契約（`docs/collector-contract.md`）で確かめてから写す**。
  ST08 は design と枝の実装で鍵名が一部違う（`profile_dir` / `profile`）が、この表が読むのは両方にある `browser` / `url` / `title` だけ。
  鍵名が違えばこの表の 1 行を直す（試験はこの表の行ごとにある。Task 1）。
  **書庫の論理ソースの名前は収集の契約に載らない**（サーバ側の取り込み）ので、登録簿（`core.source`）に実在する `c03-%` がどれも表（D2 か D11 の位置の点）にあることを試験で確かめる（Task 1.1）
- ブラウザの `excluded`（シークレットなどの除外）は（仮）で**出さない**（spec の読み出しの要件に「訪問でない記録を返さない」と置いた）。
  **反転条件**: ST08 の除外の記録が時刻の区間を持つと分かったら、PC の除外と同じ行にする

### D3. 内容ハッシュが同じ記録は、**日付で先に絞ってから** SQL の中で畳む。移行は足さない（C6）

```sql
SELECT DISTINCT ON (e.logical_source, e.content_hash) e.id, e.logical_source, e.event_time, e.tz_id, e.device_id, e.payload, s.display_name
  FROM core.event_live e JOIN core.source s USING (logical_source)
 WHERE e.user_id = $1 AND e.logical_source = ANY($2) AND <日付の条件（D5）>
 ORDER BY e.logical_source, e.content_hash, e.ingest_time, e.id
```

- 代表は ST03 の D16（仮）と同じ（`ingest_time` の最も古い行）。感度は読まない（C5）
- **`core.event_folded` を使わない・作り直さない**: 日付の条件が畳んだ後に掛かる（1 日を読むたびに全件を畳む）うえ、`device_id` と `tz_id` を持たない。
  ビューを作り直すと ST03 の試験（`dedup_tests.rs`）の前提が動く。クエリの中で済むので**移行は足さない**
- 削除済みは `core.event_live` で落ちる（C7。製造準備 A-3 のビュー越し）

### D4（仮）. 2 台の PC から届いた同じ訪問は、**訪問の時刻と URL の組**で 1 件に畳む（Q2）

D3 の後で、`c02-browser-history` の訪問だけを `(event_time, url)` で畳み、`ingest_time` の最も古い行を代表にする。
ST08 の識別子（`external_id`）は PC ごとの値なので内容ハッシュでは畳まれない。ST08 は「URL のハッシュ」を payload に持たない（design D9 の文と違う）ので、`url` を直接比べる。

- 書庫と手元の組（Q2 の（1）〜（3））は**畳まない**。論理ソースが違うので D3・D4 のどちらにも掛からない
- **反転条件**: 同期で訪問の時刻が PC ごとにずれる（マイクロ秒が丸められる）と分かったら、秒で丸めて比べる

### D5（仮）. 日付の条件。点の記録は出来事の時刻、区間の記録は区間の重なりで引く

- 点の記録（`foreground` / `visit` / `archive`）: `event_time ∈ [d0, d1)`（`d0`・`d1` は Asia/Tokyo の日の端。`stay_store::day_bounds`）
- 区間の記録: **始まりと終わりを種類ごとに読み**（D2 の表）、`始まり < d1 かつ 終わり > d0`。
  `event_time` の置き方がソースで違う（`c02-window` は始まり、`c01-app-usage` の `gap` は終わり）ので、SQL では
  `event_time ∈ [d0 − 40 日, d1 + 40 日)` で索引（`event_by_source_time`）に当て、重なりは Rust 側で判定する
- `from` / `to` を指定したときは、その範囲で同じ判定をする
- **反転条件**: 40 日より長い PC の停止（長い旅行）が実際に出たら、`powered-off` だけ下限を外す（件数は少ない）

### D6（仮）. アプリを使った時間の数え方

- **PC**（spec の文のとおり）: 前景（`foreground`）の時刻から、**次の前景の変化**（次の `foreground`・除外の始まり）までを使った時間とし、
  その間に入る離席（10 分未満も）・除外・停止の区間を差し引く。次が無ければ、今日はいままで、過ぎた日はその日の終わりまで。
  離席から戻っても前景が同じなら記録が出ない（ST07）ので、「次の離席の始まり」で区切ると戻った後の時間が落ちる —— 区切りに離席を使わない（spec-review R1）
- **スマホ**: 前に出た時刻から、次に別のアプリが前に出た時刻か、同じアプリが後ろに下がった時刻（`event_type` 2 / 23。D2 の内部の `background`）までとし、**1 回 10 分で打ち切る**
  （proto の数え方。画面の消灯・ロックのイベントを読まないため、放置した時間を数えない。spec に値を置いた）
- 行の範囲（D7）の外にはみ出した分は、その行に数えない
- 数え方は記録から計算し直せる（deep.md Q1 の context）。**反転条件**: スマホの使った時間が本人の体感と大きく違うと分かったら、画面の入切（15 / 16）とロック（17 / 18）で区間を閉じる。10 分の打ち切りも同じ

### D7. 要約はサーバで作る。行の範囲は**行の実際の始まりから終わりまで**

`GET /day` が `day_view` の各行（滞在・移動・記録なし）について、その行の `[start, end)`（日付をまたぐ滞在は日の外まで）に入る記録で要約を作る:

```json
"summary": {
  "pc":     {"seconds": 9900, "top": [{"app": "VS Code", "seconds": 5400}, …最大 3]},
  "phone":  {"seconds": 1800, "top": [...]},
  "browser":{"count": 32, "top": [{"domain": "github.com", "count": 18}, …最大 3]},
  "archive":[{"display_name": "Chrome の履歴", "count": 12}, …]
}
```

- **上位 3 の定数は `pub const SUMMARY_TOP: usize = 3`**（Q1 の本人の決定。試験が名指しで固定する）
- 同点はアプリ名・ドメインの昇順、ドメインは小文字にして先頭の `www.` を外す（spec に置いた）。
  （仮）**反転条件**: 束ね方が粗い・細かいと分かったら、登録可能なドメイン（eTLD+1）で束ねる
- 記録が無いソースは鍵ごと省く（spec「記録の無いソースの要約は出ない」）
- 「消した」の行には要約を付けない（spec。ST22 の「消した行に詳細を出さない」に合わせる）。
- **どの行にも入らない時間**（消した時間・今日の最後の位置からいままで）に位置以外の記録があれば、`GET /day` が `kind: "other"` の行（「ほかの記録」）を足し、同じ形の要約を付ける。
  FR-56 ★「記録 1 件まで届く」を、ST22 の「消した行に詳細を出さない」を変えずに満たすため（spec-review R20）。消した時間の PC の記録は ST22 の Q1 で生きている。
  `stay_store::day_view` は変えず、`day.rs` が `day_view` の行の隙間から作る。
  （仮）**反転条件**: 「ほかの記録」の行が消した行のすぐ後に出るのが、本人に「消したのに出ている」と読まれると分かったら、消した時間の分だけ要約を畳む（開けば届くまま）
- 要約の計算は D2〜D6 の正規化を**そのまま**使う。`GET /day/records` と同じ関数を通す（要約と中身がずれない）

### D8. 開いた行の中身は、開いたときに `GET /day/records?from=&to=` を 1 回読み、**畳みは画面で行う**

- 開くのは滞在・移動・記録なし・ほかの記録の行（ST22 の「開ける行」を広げる。状態は ST22 の `openId` 1 つのまま —— 一度に 1 つ）
- 畳みの規則は `web/src/timeline.ts`（新規）の純粋な関数 `foldRuns(records)`: **同じ `source` の同じアプリ**（ブラウザは同じ `browser`）が続く間を 1 つにし、
  始まり・終わり・件数を持たせる。取れていない時間の記録は畳まず 1 行で出す。畳んだ行の開閉も 1 つの状態で持つ
- 位置の点は ST22 の `GET /stays/detail` の件数（「場所 N 件」）をそのまま使う（新しく数えない）。移動・記録なしの行では `GET /day/records` の応答に位置の件数を添えない —— **件数は滞在の行だけ**（ST22 の詳細）。
  （仮）**反転条件**: 移動の行でも位置の件数を見たいと分かったら、`/day/records` に位置の件数を足す
- ST22 の「この滞在を消す」と確認は、滞在の行の中身の**末尾**に置いたまま（ST22 の design D9）
- 地域が Asia/Tokyo でない記録は、時刻に `（London 8:02）` の形で地域の時刻を添える（C11）。地域の名前は `tz_id` の最後の区切りの後
- **畳みを画面に置く理由**: 開いた 1 行ぶん（多くて数百件）しか運ばず、畳み方は表示の規則で、サーバの契約にしない（記録 1 件ずつの口のまま ST26 が使える）

### D9. 取れていない時間の行は `GET /day` の `gaps` で返し、**置く位置は画面が決める**

- `gaps` は `[{kind: "away"|"stopped"|"excluded"|"unavailable", start, end, count?, reason?}]`。日の端で切る。今日はいまで切る
- **除外の行**: 除外の記録の始まりから**次の前景の記録**まで（無ければ停止の始まり・日の端・いま）。`range_end` は下限。
  間を置かずに続く除外の記録（ST07 は送信の契機の 5 分ごとに区間を切り、数えが 0 の続きは記録にしない）は 1 行にまとめ、件数を足す（spec-review R5）。
  収集側の形は変えない（`desktop-collection` は ST25 の範囲外）
- **スマホの行**: `reason` を写し、`retention` と `clock_skew_abandoned` で文字を分ける（ST06 の契約「混ぜない」。spec に文言を置いた）
- **離席の行**: leave の記録の区間。今日だけ、leave の無い enter をいままでの離席にする（spec「いま離席中の時間は今日の行になる」）
- **離席は 10 分以上だけ**: `pub const AWAY_ROW_MIN_MINUTES: i64 = 10`（Q1 の本人の決定。試験が名指しで固定する）。要約の差し引きは 10 分未満も含める（D6）
- 置く位置: 画面が `gaps` を、始まりを含む滞在・移動・記録なし・ほかの記録の行の直後に、始まりの時刻順に差し込む（`web/src/timeline.ts` の `placeGaps`）。
  どれにも入らない（消した行の中など）ものは、始まりより前で最も近い行の直後（spec に置いた）
- 行の文字は spec のとおり（「離席」「PC 停止」「除外 …（N 件。本文なし）」「スマホ 取得元に残っていない」）。種類を色で分けない（C1）

### D10（仮）. ブラウザ履歴の「最後に届いた時刻」は、ブラウザ履歴の記録の `ingest_time` の最大

サーバに「どこまで読んだか」の印は無い（ST08 は PC の置き場に持つ）。`GET /day` の `browser_through` は
`core.event` の `c02-browser-history` の `ingest_time` の最大（削除済みを含む —— 届いた事実の時刻）。spec の言葉も「最後に届いた時刻」にした。

- **生存信号は使わない**: ST08 の生存信号は、行を読まない起動直後でも「写しを取って開けるか」を 1 試行・成功と数える（ST08 design D12）ので、
  `successes > 0` は「取得した」を意味しない（spec-review R6）
- 行を出すのは、`c02-browser-history` の `collection_started_on` がその日以前で、`browser_through` が `min(その日の終わり, いま)` より前のとき
- 代償: 新しい訪問の無い取得では `ingest_time` が進まないので、ブラウザを開かなかった日の翌日にも「まだ届いていない」が出る（何も失わない —— 表示だけ）
- **反転条件**: ST08 か後続が「最後の成功」をサーバに送るようになったら、それに置き換える

### D11. 記録なしの事情は、記録なしの行ごとにサーバが稼働記録と書庫を読んで付ける（Q3）

`reasons: [{kind, count?}]`。並びは次の順（spec に置いた。（仮）**反転条件**: 読みにくいと分かったら並びを変える）:

| `kind` | 当たる条件 | 件数 |
|---|---|---|
| `before_start` | 行の日が `c01-location` の `collection_started_on` より前（登録簿の値が無ければ当たらない） | — |
| `stopped` | `core.coverage_span` の `kind = 'stopped'`（`c01-location`）が行と重なる | — |
| `dropped` | `core.drop_report`（と旧形の `coverage_span kind = 'dropped'`）の `c01-location` が行と重なる | 重なる時間の `drop_report_hour` の件数の和（旧形は行の件数） |
| `archive` | 書庫の位置の点（`c03-timeline-route` / `c03-timeline-signal` / `c03-legacy-location`。D3 で畳んだ後。タイムラインと移行前の 2 経路の重なりは**両方数える** —— Q2 の（1）を畳まない） | その件数 |

- **行の判定（`day_view`）は変えない**。事情は添えるだけ（spec）
- 止めていた時間を書く経路は ST15 まで無いが、読む側は表の形（`coverage_rebuild.sql`）で作り、試験は表に直接行を入れて確かめる（`testdb.rs` と同じ）
- 稼働状況（`coverage.rs`）の判定・関数には触らない（`collection-coverage` は ST12 が走っている）。読む SQL は `day.rs` に別に書く

### D12. ルートと行き先（Q4）

- `Root.tsx`: `#/coverage` → 稼働状況（`App`）/ `#/master` → マスタ管理 / `#/day/…` → 1 日の画面 / **それ以外（空・`#/`）→ 今日の 1 日の画面**
- 1 日の画面の上の並び（`nav`）に「稼働状況」（`#/coverage`）と「マスタ管理」（`#/master`）。稼働状況の「1 日の画面へ」（`#/day/`）は残し、マスタ管理への入口は稼働状況にも残す
- 暦に無い日付の画面は `useScheme()` を使う（C10）
- 見出しは「<日付>の滞在」から「<日付>」へ（spec に置いた）
- `docs/screens.md` の表を直す（確認バッチの手順書が読む）。e2e の `stack.spec.ts` と `coverage-year.spec.ts` は `/#/coverage` を開く

### D13. 偽データ（`tools/seed.sh normal`）に 2026-09-07 の他のソースを足す。書庫は入れない

e2e と確認バッチが同じ日を見る（製造準備 A-4。全 Story がこの偽データを使う）。proto の「いつもの日」と同じ構成:

- PC の前景 約 1,500 件（同じアプリの連続を含む）/ 離席 4 回（うち 1 回は 10 分未満）/ 除外 1 回（件数 3）/ PC の停止 23:40 から翌日
- ブラウザの訪問 約 190 件（うち数件は 2 台目の PC の `device_id` から同じ訪問）。
  **ST08 の移行（`external_id_kind = 'record'`。記録が 0 件のときだけ当たる）が済んでいるときだけ入れる** —— 先に訪問を入れると、その DB では ST08 の移行が二度と当たらない（spec-review R18）。
  済んでいなければ訪問を入れず、そう出して続ける
- スマホの前景 約 220 件と後ろに下がったイベント
- 登録簿に無い論理ソース（main に未登録のもの）は、seed の先頭で `ON CONFLICT DO NOTHING` で足す（既存の `seed-location` と同じやり方）
- 原文は固定の乱数で作り、何度入れても同じ本文（再送として畳まれる）
- **書庫は入れない**（proposal も揃えた）。本人の測った「書庫を置いた日」は開いてすぐ見える行が 4 で、いつもの日の基準（6 行以上）と同じ日には置けない。書庫の画面の振る舞いは jsdom と Rust の試験で持つ

### D14. 変えないもの

| もの | なぜ |
|---|---|
| `GET /events` / `GET /stays` / `GET /stays/detail` / `/ingest` の応答 | ST01 の「壊してはいけないもの」・ST22 の契約（C3） |
| ST22 が書き換えた `browsing-views` の要件 | 並走（proposal の Impact） |
| `coverage.rs` の判定 | `collection-coverage` は ST12。稼働状況の明暗は ST14（Q5） |
| `core.event_folded` | D3 |
| `stay_store::day_view` の行の判定 | Q3（行の判定は変えない） |

### D15（仮）. e2e の実寸の基準は「行の数・見える行・1 行の高さ」で、日の高さの合計は撃たない

deep.md Q1 の 4 つの日の値（行・高さ・見える行）は proto の偽データとフォントでの測定で、本物のブラウザとフォントでは合計の高さが変わる。
下流の e2e は、本人の決定の**構造**（行の単位が滞在のまま・閉じた行に記録 1 件ごとの題名が出ない）と、**1 画面の読みやすさ**（360 × 640 で 6 行以上・1 行 200 px 以下・横にはみ出さない）を撃つ（spec に値を置いた）。
6 行は ST16 の深掘り Q4（360 px の 1 画面に 6 行）、200 px は ST16 の実測（いちばん高い行 171 px）に余裕を足したもの。
**反転条件**: 要約が 4 ソースぶん並ぶ行が 200 px を超えると分かったら、要約の折り返しを詰めるか値を見直す（本人の決めた構造は変えない）

### D16（仮）. 書庫どうしの組も畳まない（Q2 の（1）（3））

論理ソースが違う記録は、同じ出来事でも別の記録として返す（例外は D4 の 2 台の PC の訪問だけ）。YouTube の視聴履歴とマイアクティビティの YouTube は 2 件出る。
書庫の位置（移行前のロケーション履歴と Timeline.json の重なり）は記録なしの事情（D11）で**両方数える**。本人の Q2 の答え（（1）〜（3）は両方出す）を spec に写したもの（spec-review R8）。
**反転条件**: 書庫を置いた日に同じ出来事が 2 行ずつ並んで読みにくいと分かったら（deep.md Q2 の反転条件の候補）、組ごとに片方を優先する。記録は書き換えないので戻せる

### D17（仮）. 1 日の読み出しは感度で記録を外さない（C5）

spec の読み出しの要件に「感度の値で記録を外さない」を置いた（spec-review R11）。1 日の画面は本人が手元の網の中で見るもので、感度は外へ出す経路（AI・書き出し）の話（PERM-2〜6）。
**反転条件**: ST24（感度）が 1 日の画面にも感度を効かせると決めたら、ST24 が同じ capability でこの文を MODIFIED する（正典に「外さない」があるので、隠す側への変更が差分として見える）

### D18（仮）. ST25 の `requires` に ST22 を足す

ST25 の画面は ST22 の開ける行と詳細の上に積み、spec も ST22 の要件を名指しする。admission に ST22 の archive を待たせるため、`docs/stories/stories.json` の ST25 の `requires` に ST22 を足し（layer 1 → 3）、
`docs/stories/INDEX.md` に訂正を書いた（spec-review R19。前例は ST22 → ST16）。
**反転条件**: ST22 が開ける行を作らずに閉じたら、ST25 が開ける行を作り、`requires` から ST22 を外す

### D19（仮）. どの行にも入らない時間の記録は「ほかの記録」の行に出す

D7 の「ほかの記録」の行（消した時間・今日の最後の位置からいままで）。FR-56 ★「記録 1 件まで届く」を、ST22 の「消した行に詳細を出さない」を変えずに満たす（spec-review R20）。
**反転条件**: 「ほかの記録」の行が消した行のすぐ後に出るのが、本人に「消したのに出ている」と読まれると分かったら、消した時間の分だけ要約を畳む（開けば届くまま）

### D20（仮）. NFR-20 は「時刻順の記録を足しても消す操作が 44 px のまま」で満たす

ST25 は取り返しの付かない操作を足さない（Q6）が、ST22 の詳細の中身を組み替えるので、「この滞在を消す」が 44×44 のまま詳細の末尾に残ることを ST25 の Scenario にした（spec-review R21 の (a)）。
**反転条件**: NFR-20 の宛先を ST22（消す）と ST15（止める）に移すと決めたら、ST25 の `satisfies` から NFR-20 を外し、INDEX に訂正を書く

### D21（仮）. スマホの取りこぼしは `reason` ごとに文字を分ける

`retention` は「スマホ 取得元に残っていない」、`clock_skew_abandoned` は「スマホ 時計のずれで取れていない」（ST06 の契約「2 つを混ぜない」。spec-review R4）。
**反転条件**: 文言が本人に読みにくいと分かったら変える（表示の規則。記録は残る）

## Risks / Trade-offs

- **ST06 / ST08 / ST12 の記録の形がまだ main に無い** → D2 の表を 1 か所に置き、試験は表の行ごとに偽の記録で持つ。各下流が merge された後、契約と表がずれていれば表を直す（ST25 の下流の開始時に `docs/collector-contract.md` を読む。Task 1）
- **ブラウザの「最後の取得」は近似**（D10）→ 新しい訪問が無い日に取得しても `ingest_time` は進まない。生存信号で補うが、生存信号の順序次第で 1 日ずれうる。反転条件を書いた
- **`GET /day` は 1 日ぶんの記録を毎回正規化する**（3,000〜4,000 件）→ `event_by_source_time` の索引に当たる範囲で読み、正規化は Rust で 1 回。数十 ms を見込む。
  遅ければ要約を日ごとに持つ（記録から作り直せるので後から足せる）
- **区間の記録の 40 日の下限**（D5）→ それより長い停止は日の頭に出ない。反転条件を書いた
- **ST22 の merge 前に下流を始めると、`DayView.tsx` の開ける行を 2 本の Story が別々に作る** → Task 1 が ST22 のコードが main にあることを確かめてから始める

## Migration Plan

- 移行は無い（D3）。口を 2 本足し、画面の行き先を変える。戻すときは画面のルートを稼働状況に戻せば、口が残っていても害は無い
- ブックマークでルートを開いていた人には 1 日の画面が出る（BREAKING。proposal）。稼働状況は `#/coverage`
