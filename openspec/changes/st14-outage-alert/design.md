# ST14 設計 — 収集が途切れたら気づける

## Context

動機は `proposal.md` の Why。振る舞いの契約は `specs/collection-coverage/spec.md`。本人の決定（Q1〜Q9）と聞かずに決めた既定（C1〜C13）は `deep.md`。
**ここで決めるのは、それを実装に落とす技術判断だけ。** C1〜C13 は下の D 番号に写した（対応は末尾の表）。

いまの形（2026-10-13 に読んだもの）:

| 場所 | いまの形 |
|---|---|
| 登録簿 `core.source` | `logical_source` / `display_name` / `expected_gap_sec`（NOT NULL）/ `registered_at` / `user_id` / `collection_started_on` / `retired_on` / `succeeds`（引き継ぎ元）/ `external_id_kind`。**出どころ（`origin`）の列は無い** —— `origin` は `core.event` の列（`collected` / `authored` / `derived`） |
| 登録されている名前 | `c01-location` / `c01-app-usage` / `c01-photo` / `c02-window`（21600）・`c02-browser-history`（86400）・`c01-app-usage-rollup`（21600）・`c01-clock`（21600。生存信号を送らない）・`s01-stay` / `s01-attribute`（86400）・`s01-archive-inbox`（86400）・`c03-*` 11 本（5184000）。マイアクティビティの製品ごとの `c03-myactivity-*` は取り込み器が実行時に足す（`archive/worker.rs` の `ensure_myactivity_source`）。ST17 の `s01-feeling`・ST21 の `s01-place` はまだ main に無い |
| 生存信号 `core.heartbeat` | `emitted_at` / `received_at` / `capturable` / `blockers text[]`（自由な文字列。Windows は `foreground` / `uiautomation` / `idle`、Android は `permission` / `sensor` / `network`、取り込み器は `dedicated_inbox_unreadable` など） |
| 観測ではない記録 | `c02-window` の `payload.kind` が `idle` / `powered-off` / `excluded` / `clock-skew`（観測は `foreground`）。`c01-app-usage` は通常のイベントに `kind` が無く、`kind: "gap"` が取りこぼしの期間（`begin` / `end` / `reason` が `retention` か `clock_skew_abandoned`。`event_time` は `end`。`docs/collector-contract.md:199-225`） |
| 日の状態 `coverage.rs` | `decide()`（:940-981）は (1) 導入前 (2) 退役 (3) 丸ごと破棄 (4) 丸ごと停止 (5) `event_count > 0` → ① (6) 生存信号 (7) 前後の活動 の順。`facts()`（:606-716）は `core.event` を `kind` を見ずに `count(*)` する（**gap も時計の測定も①に数える**） |
| 開始日 | `touch_started_on`（:240-294）は記録なら無条件（gate が `"true"`）、生存信号なら登録日以降で前にしか動かさない。取り込み（`lib.rs:1183`）と生存信号（`lib.rs:2062`）から呼ぶ |
| 達成 | `DEVICE_SUBJECT = [c01-location, c01-app-usage]`（:28。NFR-13 ★ 2026-09-18 と食い違い）。分母は停止と退役を除いた全日で、**PC の 2 ソースの但し書き（生存信号の届いた日）が無い** |
| `GET /coverage` | Must の 5 本 + `c03-%`。集計は出ない |
| 書庫の台帳 | `core.archive_ledger`（`outcome` が `read` / `already_read` / `unreadable` / `pending_shape` / `store_failed`、`created_at` は NULL 可、`discovered_at` NOT NULL）と `core.archive_ledger_source`（`ledger_id` × `logical_source`）。**`pending_shape` の行には `archive_ledger_source` が付かない**（`worker.rs:1378-1400`。印を置いて読み直すと `read` の行と一緒に付く） |
| 周期の処理 | 書庫の取り込み器のループだけ（`worker.rs:1803-1876`）。途切れを見る仕組みは無い |
| 資格情報 | `API_TOKEN`（Bearer）と画面のログイン（cookie）の 2 つ。収集アプリも同じ `API_TOKEN` を使い、どの口も両方を受け付ける（`authorize()`。`lib.rs:515-541`） |
| PC の収集 | 通知の口は無い。`windows` crate への直接の依存も無い。1 時間ごとの時計の測定は `runtime.rs` の `tick_at` → `maybe_measure_skew`（`clock.rs` の `SkewSchedule`）。送信は `sender.rs` の `HttpTransport`（`ASHIATO_BASE_URL` + Bearer） |
| 携帯の収集 | 生存信号は `LocationService.kt:817-819` の `newHeartbeatScheduler(...).every(6h)`（`ScheduledExecutorService`。起動時にも 1 発）。通知チャネルは `location`（常駐。IMPORTANCE_LOW）と `retention`（未送信の保持。IMPORTANCE_DEFAULT）。`POST_NOTIFICATIONS` は求め済み。HTTP は `HttpTransport.kt` |
| 画面 | S-1 は 2026-10-12 の表面の作り直しで `useScheme` を読み、明暗に追従している（`App.tsx:72` / `CoverageGrid.tsx:32` / `AchievementPanel.tsx:18`。`web/e2e/surface.spec.ts` がライトとダークを測る）。週の文字は `CoverageGrid.tsx` の `WeekDetail`（:215-242）、並びは `archives.ts` の `orderCoverageWithArchives` |

## Goals / Non-Goals

**Goals**

- 判定を**サーバの純粋な関数 1 つ**に置き、材料（記録・生存信号・台帳・登録簿）の読み出しと分ける —— 判定の規則（Q2 / Q3 / Q9 / C1 / C2）を DB 無しの単体で全部固定できる
- 通知の出し方（Q4 / Q5）を、携帯と PC のそれぞれで**OS に触らない純粋な決め方**と**OS の口**に分ける —— 決め方は Robolectric / ubuntu の単体で、OS の口は Robolectric の影と Windows の実行時テストで確かめる
- 格子の直し（C9 / Q7）は `facts()` の材料を直すだけにし、評価の順は `decide()` の 1 か所で変える

**Non-Goals**（proposal の Non-Goals に足すもの）

- サーバに周期のループを足すこと（D5）。判定は問い合わせのたびに引く
- 収集アプリの資格情報を画面の資格情報と分けること（ST44）
- 途切れの通知を押したときに画面を開くこと（D14（仮））

## Decisions

### D1. 判定はサーバの `outage.rs` の純粋な関数で、問い合わせのたびに引く（C4）

```
outage_store::inputs(pool, user, now) -> Vec<SourceInput>     // SQL。登録簿 × 材料
outage::judge(&[SourceInput], now) -> Vec<Outage>             // 純粋。DB を見ない
```

- `SourceInput` は `{logical_source, display_name, expected_gap_sec, alert_after_days, target, has_non_collected, retired_on, collection_started_on, stopped_now, last_stop_end, last_record_at, last_capturable_heartbeat_at, last_archive_at, kind}`。`kind` は `Device` / `Archive` / `ServerHeartbeat`
- `judge` は spec の 3 つの Requirement（対象・最後に生きていた時刻・書庫）をこの順で当てる。閾値は `expected_gap_sec * 3`（`ALERT_GAP_FACTOR = 3`）か `alert_after_days`。**「超える」は厳密な `>`**（Scenario `18 時間ちょうどでは途切れにならない`）
- 判定の結果を行に焼かない（ST02 の design D6 と同じ理由。spec「判定を引くたびに導き」）。台帳（D7）は結果の事実を残すだけで、判定の材料にしない
- `kind` の決め方: `alert_after_days` を持つ → `Archive`、`logical_source` が `s01-archive-inbox`（サーバ自身が生存信号を出すもの。定数 `SERVER_HEARTBEAT_SOURCES`）→ `ServerHeartbeat`、それ以外 → `Device`

> 採らなかった案: 判定を `coverage.rs` の `decide()` に足す。日の状態は「日」の単位で、途切れは「いま」の単位なので材料の粒度が違う（日に畳むと 18 時間の判定ができない）。

### D2. 登録簿に 3 列を足す（C1 / Q9 / Q8）。移行は 1 本

`migrations/YYYYMMDDHHMM_outage_alert.sql`（作成時刻で名付ける）:

| 列 | 型 | 既定 | 意味 |
|---|---|---|---|
| `outage_target` | `boolean NOT NULL` | `true` | 途切れの判定の対象か（C1。既定は「対象」） |
| `alert_after_days` | `integer NULL CHECK (alert_after_days > 0)` | NULL | 通知までの日数（Q9）。NULL なら想定間隔の 3 倍 |
| `started_on_from` | `text NULL REFERENCES core.source(logical_source)`、`CHECK (started_on_from IS DISTINCT FROM logical_source)` | NULL | 収集開始日を借りる先（Q8）。集計だけが `c01-app-usage` を持つ |

同じ移行で:

- `UPDATE core.source SET outage_target = false WHERE logical_source IN ('c01-clock','c01-app-usage-rollup','s01-stay','s01-attribute','s01-feeling','s01-place')` —— 無い名前は何もしない（ST17 / ST21 が先に入っていれば当たる。後から入っても出どころで外れる。D3）
- `UPDATE core.source SET alert_after_days = 75 WHERE logical_source LIKE 'c03-%' AND alert_after_days IS NULL` —— **本人が変えた値を戻さない**（Scenario `本人が変えた通知までの日数は移行を当て直しても戻らない`。ST12 の想定間隔と同じ形）
- `UPDATE core.source SET started_on_from = 'c01-app-usage' WHERE logical_source = 'c01-app-usage-rollup'`
- 集計の汚れた `collection_started_on` を `c01-app-usage` の値へ直す（`UPDATE … SET collection_started_on = (SELECT collection_started_on FROM core.source WHERE logical_source = 'c01-app-usage') WHERE logical_source = 'c01-app-usage-rollup'`）。記録は触らない（申し送り st06 R7「失われたものは無い」）
- 台帳の表（D7）
- `.down.sql` は台帳の表と 3 列を落とす（台帳は判定の材料ではないので、落としても判定は戻る）

取り込み器が実行時に足す `c03-myactivity-*`（`ensure_myactivity_source`）にも `alert_after_days = 75` を入れる（`INSERT … ON CONFLICT DO NOTHING` の列に足す。既にある行は触らない）。定数 `ARCHIVE_ALERT_AFTER_DAYS = 75` を `archive` に置き、移行の 75 と試験で突き合わせる。

> 採らなかった案: 判定の対象を名前の表（定数）で持つ。新しいソースが表に無いと黙って外れる（C1 が避けた「既定は対象外」と同じ）。列にして既定を `true` にする。

### D3. 「本人が書いた・導出した記録のソース」は記録の出どころから引く（C1。整合の確認 C3）

登録簿に出どころの列は無いので、`inputs` の SQL で「収集した記録を持つか」（`EXISTS … origin = 'collected'`）と「本人が書いた・導出した記録を持つか」（`EXISTS … origin <> 'collected'`）の 2 つを引き、
**収集した記録が 1 件も無く、本人が書いた・導出した記録があるときだけ**外す（`has_collected` / `has_non_collected`）。
「1 件でも本人が書いた記録があれば外す」は採らない —— `/ingest` はどのソースにも `authored` を受け付ける（`ingest.rs:505-516`）ので、収集のソースに 1 件混ざると、記録は書き換えも削除もできないまま永久に鳴らなくなる（spec-review R2）。
**記録が 1 件も無いソースは収集開始日も無い**ので、出どころが分からない間に鳴ることは無い（spec の (4)）。
衛星が本人の代わりに書く記録（FR-25 ★ 2026-10-11）も `authored` で入るので、ST39 が何もしなくても外れる。

### D4. 最後に生きていた時刻の材料（Q2 / Q3 / C9 / C12）

| kind | 最後に生きていた時刻 |
|---|---|
| `Device` | `max(観測の記録の event_time, 取得できる状態の生存信号の emitted_at, 最後に終わった停止の終わり)` |
| `Archive` | `max(coalesce(l.created_at, l.discovered_at))`（`core.archive_ledger l JOIN core.archive_ledger_source ls ON ls.ledger_id = l.id WHERE ls.logical_source = $1 AND l.outcome = 'read'`） |
| `ServerHeartbeat` | `max(取得できる状態の生存信号の emitted_at)` |

- **観測ではない記録**は `(logical_source, payload->>'kind')` の組の定数で持つ（`coverage.rs` の `NON_OBSERVATION_GRID` と `outage.rs` の `NON_OBSERVATION_ALERT`）:
  - 格子（D15）: `("c01-app-usage", "gap")`・`("c02-window", "clock-skew")`
  - 途切れ: 格子の 2 つ ＋ `("c02-window", "idle")`・`("c02-window", "powered-off")`・`("c02-window", "excluded")`
  - 引き継ぎの鎖で名前が変わったときは、同じ組を新しい名前にも足す（試験 `st14_nonobs_registry` が、`c02-%` と `c01-app-usage%` の登録簿の名前がこの表で扱われているかを見る）
- **未来の時刻**: `event_time > now` の記録・`emitted_at > now` の生存信号は、`ingest_time` / `received_at` で数える（spec。Scenario `未来の時刻の記録は途切れを隠さない`）
- 引き継ぎの鎖（`succeeds`）: 判定はソースの名前ごとで、鎖をたどらない。退役した名前は (3) で外れ、後継は自分の記録で判定する。後継の収集開始日は鎖の根から継ぐので、後継が 1 件も届かないうちも開始済みとして判定し、
  **最後に生きていた時刻の下限を後継の `registered_at`** にする（名前を分けた直後に鳴らず、後継が想定間隔の 3 倍届かなければ鳴る。spec の Scenario `引き継いだソースは届く前から判定の対象になる`）
- 破棄の報告（`core.drop_report`）は材料に入れない（C8。spec）
- 書庫: そのソースを運んだ `read` の台帳が 1 件も無く記録はあるとき、そのソースの記録の `max(ingest_time)` を代わりにする（spec。台帳より前の取り込み・台帳の行を残せなかった経路 `worker.rs:2240-2252`）。
  `archive_ledger_source` は格納を試みた記録からしか立たない（`worker.rs:286-310`）ので、項目が 0 件のファイルだけの書庫は「運んだ」に入らない（D24（仮））
- 読み出しは `core.event`（削除済みも含む。削除は観測の事実を消さない —— `record-deletion` の「稼働状況は削除済みの記録も数える」と同じ）
- **時刻の索引**: `core.event` の `(logical_source, event_time)` の索引を `ORDER BY event_time DESC` で後ろから読み、観測ではない種類を飛ばす（PC の時計の測定は 1 日 24 件なので、飛ばす量は小さい）。足りなければ D 番号を足して部分索引を検討する

### D5. 起動直後は 30 分判定しない（C7。仮）

**D5（仮）**: サーバのプロセスが起動してから `OUTAGE_WARMUP = 30 分` の間、`GET /outages` は `{"status":"warming_up"}` を返し、台帳に書かない（spec の Scenario `判定を待つ間は台帳に書かない`）。
携帯は到達できれば 5 分ごとに未送信を送る（FR-10）ので、30 分あれば溜まった分が届く。
起動時刻はプロセスの単調時計で持ち（`AppState.started`）、試験では差し替えられるようにする（`Clock` の口）。

- **反転条件**: 起動し直した後に、溜まった分が届く前に鳴った（台帳の `opened` の直後に同じソースの `closed` が続き、その間が 1 時間未満）と分かったとき —— 待ち時間を延ばす

> 採らなかった案: 周期のループで判定し、台帳に書く。検知する側が同じ理由（サーバの停止）で止まりうるのは同じで、問い合わせの頻度（PC 1 時間・携帯 6 時間）で十分。ループを足すと、起動直後の待ちと停止の扱いがループ側にも要る。

### D6. 口は `GET /outages`（C5 / 整合の確認 C1）

```
GET /outages?device_id=<端末識別子>&user_id=<任意>
200 {"status":"ready","judged_at":"…","outages":[{"episode_id":123,"logical_source":"c01-location","display_name":"位置","last_alive_at":"…"}]}
200 {"status":"warming_up","judged_at":"…"}
401（資格情報なし）
```

- 認証は既存の `authorize()`（Bearer の `API_TOKEN` と画面の cookie の両方が通る）。読み出しの記録（`access_log`）にもそのまま乗る
- `device_id` は任意。あれば台帳に「届けた」を書く（D7）。収集アプリは必ず付ける（携帯は既存の端末識別子、PC は `contract.rs` の端末識別子）
- `outages` は `display_name` の昇順、同じなら `logical_source` の昇順
- 応答の型は `utoipa` で OpenAPI に出す（`tools/check-openapi.sh`）
- **ST44 への約束**: Bearer で 200 を見る試験（`st14_api_outages_bearer`）を置く。ST44 が収集アプリの資格情報を締めたときに落ちて気づける（`docs/handoff/ST44.md`）

### D7. 台帳は `core.outage_ledger`（追記のみ。C6）

| 列 | 型 | |
|---|---|---|
| `id` | `bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY` | 途切れの識別子は `opened` の行の `id` |
| `user_id` | `uuid NOT NULL` | |
| `logical_source` | `text NOT NULL REFERENCES core.source` | |
| `kind` | `text NOT NULL CHECK (kind IN ('opened','closed','delivered'))` | |
| `episode_id` | `bigint NULL REFERENCES core.outage_ledger(id)` | `closed` / `delivered` が指す `opened`。`opened` は NULL |
| `device_id` | `text NULL` | `delivered` だけが持つ |
| `last_alive_at` | `timestamptz NULL` | `opened` だけが持つ（書庫なら最後の書庫の時刻） |
| `judged_at` | `timestamptz NOT NULL DEFAULT now()` | |

- `CHECK` で種類ごとの列の有無を縛る（`opened` は `episode_id IS NULL`、それ以外は NOT NULL、`delivered` だけ `device_id` NOT NULL）
- 一意索引: `(episode_id) WHERE kind = 'closed'`、`(episode_id, device_id) WHERE kind = 'delivered'`
- 追記のみ: `UPDATE` / `DELETE` / `TRUNCATE` を断るトリガ。**関数は台帳専用の `core.reject_outage_ledger_change()`**（書庫の台帳と同じ形で、共有しない）。
  共有すると ST12 の `202609181600_archive_ingestion.down.sql:21`（`CASCADE` なし）が依存で落ちて戻せなくなる（申し送り `docs/handoff/ST14.md` K3。ST16 / ST19 / ST21 も同じ理由で共有していない）
- **書き込みは `GET /outages` の中で 1 回の取引**: `pg_advisory_xact_lock(<利用者から決めた鍵>)` → 開いている途切れ（`opened` のうち `closed` の無いもの）を読む → 判定と突き合わせ、新しい途切れに `opened`、戻ったものに `closed` → `device_id` があれば `delivered`（`ON CONFLICT DO NOTHING`）。同時に 2 台が問い合わせても `opened` は 1 行
- 記録の本文・座標・原文の列を持たない

### D8（仮）. 通知に使う「取得できない」を blockers の種類で絞らない（整合の確認 C14）

**取得できない状態の生存信号は、blockers の中身を問わず「生きていた」に数えない。** ST08 のブラウザ履歴（プロファイルが 1 つ読めないと取得できない）と ST07 のウィンドウ（URL の経路が応答しないと取得できない）は、そのまま 72 時間・18 時間で鳴る。
どちらも、そのまま放っておくと取れない分が失われる（ブラウザの訪問は 90 日で消え、URL は後から取れない）ので、鳴らすのは「既定は厳しい側」にあたる。

- **反転条件**: 確認バッチか実際の使用で、本人が直す必要のない理由（使っていないプロファイルが鍵を掛けている、など）でこの 2 つの途切れが繰り返し鳴ると分かったとき —— `blockers` の種類で「部分的な取れなさ」を生きていたに数える表を `outage.rs` に足す（ST08 には差し戻さない。判定はいつでも引き直せる）

### D9（仮）. 集計と滞在と個人属性に「対象外」の印を付ける（C1 / Q6）

`c01-clock`（Q6 の本人の決定）のほかに、集計（`c01-app-usage-rollup`）・滞在（`s01-stay`）・個人属性（`s01-attribute`）に印を付ける。
滞在と個人属性は出どころ（`derived` / `authored`）でも外れるが、記録が入る前の登録の時点で印を見て分かるようにする。
集計は 1 日に 1 回ほど OS の箱を取り直すもので、生存信号を送らない。生きているかは同じ携帯の `c01-app-usage` の生存信号が持つ。

- **反転条件**: 集計の取り込みだけが止まり、アプリ利用の生存信号は届いているのに気づけなかったと分かったとき —— 集計の印を「対象」に戻し、通知までの日数を集計の取り直しの間隔に合わせる

### D10（仮）. 集計の格子は Must の 5 本の直後、書庫のソースの前（Q8 / 整合の確認 C7）

`GET /coverage` の並びを「Must の 5 本 → 集計 → `c03-%`」にする（`coverage_get` の名前の並び）。画面の `orderCoverageWithArchives` は「退役していない c03 以外（サーバの順）→ 退役していない c03 → 退役」のままで、集計は 1 つ目の群の 6 本目に入る。
集計の格子も開いた直後から直近 4 週を出す（書庫のソースと同じ）。
「`c01-app-usage` の隣（Must の 2 本目と 3 本目の間）」は採らない —— Must の 5 本の連なりが割れ、ひとスクロールの予算（Must の最後の格子の下端）が集計の格子の高さぶん下がる。

- **反転条件**: 本人が GO 資料か確認バッチで、集計はアプリ利用の隣にあるほうが読みやすいと言ったとき（予算の数を置き直す判断と一緒に）
- 正典 `external-ingestion` の並びの条項（:676 / :709-712。Must・書庫・退役の順）は集計を名指ししていない。画面の並びと既存の試験は壊れない（食い違うのは正典の文だけ）ので、`docs/handoff/ST47.md` の R17 に置いた

### D11. PC の通知は Windows のトースト。差出人（AUMID）は利用者の登録簿に置く

- 依存: `windows` crate（`cfg(windows)`。機能は `UI_Notifications`・`Data_Xml_Dom`・`Foundation` の 3 つだけ —— 全機能の `windows` crate はコンパイルが OOM する（`crates/collector-windows/Cargo.toml` の注記））と、登録簿の書き込みに `winreg` crate。
  **作業場は `unsafe_code = "forbid"`**（`Cargo.toml:27`）なので、`unsafe fn` の Win32 の登録簿の関数を直に呼ばない（spec-review R4）。WinRT の通知の呼び出しは safe
- **差出人の登録**: 起動時に `HKCU\Software\Classes\AppUserModelId\dev.ashiato.collector` に `DisplayName = "ashiato"` を書く（管理者権限は要らない。未パッケージの exe がスタートアップのフォルダから起動しても通知を出せる）。書けなければ警告を出して通知だけを諦め、収集は止めない
- **出し方**: `ToastNotificationManager::CreateToastNotifierWithId("dev.ashiato.collector")` で、`Tag = "outage"`・`Group = "ashiato"` のトーストを出す。音を鳴らさないときは `<audio silent="true"/>` **と `SuppressPopup = true`**（画面の端に出さず、通知の一覧の中身だけを差し替える。spec-review R5 —— `audio silent` が止めるのは音だけで、PC は 1 時間ごとに問い合わせるので毎時表示される）。差し替えは同じ `Tag` / `Group` で出し直す（Windows は前のものを置き換える）。消すのは `ToastNotificationManager::History()?.RemoveGroupedTagWithId("outage", "ashiato", AUMID)`
- **モジュール**: `outage_notice.rs`（純粋。D13 の決め方。ubuntu でも走る）と `toast.rs`（`cfg(windows)`。OS の口）。`runtime.rs` は `maybe_measure_skew` の契機で `OutagePoller` を 1 回呼ぶ（別スレッド。応答 10 秒で打ち切り。時計の測定と同じ `ClockWorker` の型に倣う）
- **鳴らした途切れの覚え**: `outage-notice.json`（収集アプリの状態の置き場。未送信の箱と同じディレクトリ）に、鳴らした途切れの識別子の集合を書く。起動し直しても同じ途切れで鳴らない（spec の Scenario `収集アプリを起動し直しても同じ途切れでは鳴らない`）。書けなければ通知の振る舞いだけを諦め、収集は止めない
- **ST08 との重なり**: `feat/st08-browser-history` は `Cargo.toml`（`rusqlite`）・`src/lib.rs`（モジュール）・`src/contract.rs`・`src/sender.rs`・`src/runtime.rs`（+1,645 行）・`tests/runtime_windows.rs` を変えている（spec-review R15）。ST08 が先に merge されていれば、契機の場所は ST08 の後の `runtime.rs` で読み直す
- **確かめ方**: 決め方は単体（ubuntu）。トーストは `tests/runtime_windows.rs` で、`toast.rs` に出させてから `History().GetHistoryWithId(AUMID)` の件数と `Tag` を読む。音そのものは物理（確認バッチ）

> 採らなかった案: スタートメニューにショートカットを作って AUMID を持たせる。ファイルを本人の置き場に足すことになり、消し忘れが残る。登録簿の 1 鍵なら `HKCU` の中で閉じる。

### D12. 携帯の通知はチャネル `outage`。生存信号の契機に相乗りする（C11）

- `OutageNotice.kt`（純粋。D13 の決め方）・`OutagePoller.kt`（`GET /outages` を 1 回。10 秒で打ち切り。既存の `HttpTransport`）・`OutageNotifier.kt`（チャネル `outage`「収集の途切れ」IMPORTANCE_DEFAULT、通知 ID は既存の 1・2 と重ならない定数 `OUTAGE_NOTIFICATION_ID = 3`）
- 契機: 生存信号のスケジューラは**ソースごとに**ある（`LocationService.kt:795-819` の `for (running in sources)`）ので、**位置（`c01-location`）のスケジューラにだけ** `outagePoller.poll()` を足す（起動時の 1 発の後も位置の分だけ）。
  `poll` → `decide` → 通知 → `sounded` の書き込みは、`OutagePoller` が持つ単一スレッドの executor で直列にする（spec-review R3。ソースごとに呼ぶと並行に走り、どれも「まだ鳴らしていない」を読んで 2 回以上鳴る）。**`AlarmManager` と WorkManager を使わない**（ST17 の `AlarmManager` と重ならない）。
  位置の収集を止めている間は問い合わせない（位置のスケジューラが動かない）—— 携帯の収集アプリは位置の前景サービスとして動くので、位置を止めれば他のソースも止まっている
- 音を鳴らさない差し替え: `NotificationCompat.Builder.setSilent(true)`。鳴らすときは `setSilent(false)` で出し直す。`setOnlyAlertOnce` は使わない（新しく途切れたソースが増えたときに鳴らなくなる）
- 鳴らした途切れの覚え: `SharedPreferences`（`outage_notice`）に識別子の集合
- `POST_NOTIFICATIONS` が無いときは出さずにログに残す（既存の保持の通知と同じ振る舞い）。権限を求め直す画面は足さない
- 確かめ方: 決め方は JUnit、通知は Robolectric の `ShadowNotificationManager`（チャネル・音の有無・本文）。契機は Robolectric でスケジューラを進めて、問い合わせが 1 回出ることを偽の transport で見る

### D13. 通知の決め方は両端末で同じ表（Q4 / Q5）

入力: 前の状態 `sounded: Set<episode_id>`（鳴らした途切れ）と `shown: bool`、問い合わせの結果（`Unreachable` / `WarmingUp` / `Ready(outages)`）。

| 結果 | 条件 | 出すもの | 次の `sounded` |
|---|---|---|---|
| `Unreachable` / `WarmingUp` | — | 何もしない | そのまま |
| `Ready([])` | `shown` | 黙って消す | 空 |
| `Ready([])` | `!shown` | 何もしない | 空 |
| `Ready(xs)` | `xs` に `sounded` に無い識別子がある | 全部の表示名を並べて**音つき**で出す | `xs` の識別子 |
| `Ready(xs)` | 全部 `sounded` にある | 全部の表示名で**音なし**で出す（差し替え） | `xs` の識別子 |

- 次の `sounded` を「いまの一覧の識別子」にするので、戻った途切れの識別子は消える。戻った後にまた途切れると新しい識別子になり、また鳴る（Scenario `戻った後にまた途切れると別の識別子になる`）
- Rust と Kotlin に同じ表を書き、試験の印（Scenario 名）も同じものを両方に置く

### D14（仮）. 通知の文面

- 題: 「収集が途切れています」
- 本文: 表示名と最後に生きていた日時を「、」で並べる —— `位置（最後 10/12 08:15）、写真（最後 10/11 22:00）`。日時は `Asia/Tokyo`（`DAY_TZ`）の `MM/DD HH:MM`
- 押しても画面を開かない（`Intent` を付けない。PC は既定の起動だけ）。収集アプリは画面を持たず、稼働状況の URL を端末が知らない構成がある（ST17 の D10 と同じ事情）
- 記録の中身・件数を載せない（C5）
- **反転条件**: 本人が「どこを見ればいいか」を通知から辿れないと言ったとき（稼働状況の URL を `Intent` に付ける。ST17 の `ashiato.webUrl` と同じ設定を使う）

### D15. 格子の材料を直す（C9 / Q7 / 整合の確認 C6）

- `facts()` の記録の数え（CTE `c`）に `NOT (logical_source, payload->>'kind') IN (NON_OBSERVATION_GRID)` を足す。`active_days`（途絶の前後）も同じ条件で数える
- 取りこぼしの期間: CTE を 1 本足し、`c01-app-usage`（とその鎖の名前）の `payload->>'kind' = 'gap'` の記録から `[begin, end)` を取り、`Asia/Tokyo` の日に重なる日へ `{reason, begin, end}` を配る。SQL は `event_time >= from AND (payload->>'begin')::timestamptz < to + 1 日`（`event_time` は `end`）
- `DayFacts` に `gaps: Vec<GapSpan>` を足し、`decide()` の (5) の後に「`gaps` が空でなければ③」を足す。`DayCell` にも `gaps` を足して API に出す
- 稼働記録の件数（`core.coverage`）は変えない（spec「観測ではない記録も稼働記録の件数には入る」）

### D16. 開始日の例外は `started_on_from` で読む（Q8）

- 読む側: 登録簿の行を引くとき、`started_on_from` を持つソースは借り先の `collection_started_on` を自分のものとして返す（`coverage.rs` の登録簿の読み出しと `resolve_chain` の後）。借り先が引き継ぎの鎖を持つなら、借り先の鎖の根から引く
- 書く側: `touch_started_on` は `started_on_from` を持つソースの記録では何もしない（重複した到着も同じ。`lib.rs:1183` の呼び出しは変えず、関数の中で見る）
- 移行の `UPDATE`（D2）は保存されている値も揃える（読む側が借りるので、値が違っても見え方は同じ。揃えるのは SQL で直に読む人のため）

### D17. 達成の主語と PC の分母（C10）

- `DEVICE_SUBJECT = ["c01-location"]`、`USAGE_SUBJECT = ["c01-app-usage", "c01-photo", "c02-window", "c02-browser-history"]`。`must_sources()` の並び（画面の並び）は今までどおり 位置・アプリ利用・写真・ウィンドウ・ブラウザ履歴 にする（`Subject` と並びを別の定数にする）
- `PC_SOURCES = ["c02-window", "c02-browser-history"]` を足し、分母を「停止と退役を除いた日のうち、生存信号が 1 件でも届いた日」にする（`DayFacts` の生存信号の有無。取得できない状態も「届いた」に数える）
- 達成日（端末が主語）の「記録が 1 件以上ある日」は D15 の観測の記録で数える（位置には観測ではない種類が無いので値は変わらない）
- 判定式の値は試験が名指しで固定する（`docs/testing.md` §3）

### D18. 偽データ（`tools/seed.sh`）

e2e の基準にするため、`SEED=normal` に足す: アプリ利用の取りこぼしの期間 2 本（理由が違う。2026-09-01 08:00〜2026-09-03 08:00 と、同じ週の別の日）、ウィンドウの時計の測定だけの日 1 日、集計の記録（開始日より前の箱を 1 件含む）。
途切れの通知は e2e の対象ではない（画面に出ない。C13）。

### D19. 申し送り st06 R31 は確認バッチの手順に置く（C10）

`tools/usage-volume.sh --device` を本人の実機で 1 回走らせる手順を、tasks の「人間の確認待ち」に置く（`> 物理: device`）。rc=1（上限超過）なら種別を絞るかを本人に聞く（下流が決めない）。rc=2（端末・adb の失敗）は測れなかったとして残す。

### D20. `external-ingestion` の正典の文は ST47 へ（整合の確認 C5）

正典 `external-ingestion` の :697「60 日を過ぎたことを知らせるのは ST14」と :588「60 日の想定間隔が効かなくなる」は、通知が「最後の書庫から 75 日」になったことと食い違う。
ST47（上流。`external-ingestion` を触っている）が走っている間は ST14 が MODIFIED にできない（同じ capability を 2 本が同時に触らない）ので、`docs/handoff/ST47.md` に置いた。
ST14 の spec は `collection-coverage` の側で「通知は最後の書庫から 75 日・格子は最後の記録から 60 日」と書く（Requirement「書庫のソースは最後の書庫から登録簿の日数で途切れとする」）。

### D22（仮）. 格子では離席・止まっていた期間・除外の印を今までどおり数える（申し送り ST02 の st07 R38 / spec-review R1）

格子で数えない記録は FR-54 ★ の列挙（`gap` と `clock-skew`）だけにし、`idle` / `powered-off` / `excluded` は格子の「記録あり」に数え続ける。途切れの判定（FR-35 の列挙）では 5 つとも数えない。
3 つは PC の収集がその日に動いていた証拠（離席の出入りは入力を見ている収集が書き、`powered-off` の `event_time` は止まる前のまだ動いていた時刻、`excluded` は前景を読んだ上で外したもの）で、
ST02 への申し送り R38 が心配した「PC を開かなかった日に①が立つ」は、`powered-off` の始まりが前夜に落ちる日（前夜は実際に動いていた）か、離席の区間を閉じた日（入力を見ていた）で、事実と食い違わない。
**R38 の「達成日数が動くので本人に問う」は、いまは当たらない** —— `c02-window` の達成日は NFR-13 ★ 2026-09-12 で「取得できる状態の生存信号があった日」（`coverage.rs:1123-1126` の `Subject::Usage`）になっていて、格子の①に依らない。
格子の見え方だけの導出の規則なので、記録の種類が残っていれば引き直せる（B）。
この範囲（`NON_OBSERVATION_GRID`）は①「記録あり」と⑥「途絶」の前後の記録の両方に効く（MODIFIED「記録も生存信号も来ない期間は途絶として残る」。spec-review R7）—— 時計の測定が毎時届いても、前景も生存信号も無い PC の日は途絶になる。

- **反転条件**: 前景が読めなくなった PC の日が、離席の出入りだけで①に見えて本人が壊れに気づけなかったと分かったとき —— `NON_OBSERVATION_GRID` に `idle` / `powered-off` を足す（`excluded` は前景を読んでいるので残す）

### D23（仮）. 取りこぼしの理由の文言は 1 日の画面（ST25）にそろえる（spec-review R16）

「取得元に残っていない（期間）」「時計のずれで取れていない（期間）」。ST25 の 1 日の画面（`openspec/changes/st25-day-timeline/specs/browsing-views/spec.md:354-355`）と同じ言葉にし、同じ取りこぼしが S-1 と 1 日の画面で別の言葉にならないようにする。
期間の区切りは S-1 の破棄の文字（`（10:00〜13:00）`）にそろえて `〜`。ST25 には差し戻さない。文言は `messages.ts` の 1 か所。

- **反転条件**: 本人が GO 資料か確認バッチで、S-1 の文字は理由（「問い合わせなかった」）まで書いたほうが分かると言ったとき

### D24（仮）. 項目が 0 件の書庫は「運んだ」に数えない（spec-review R9）

`archive_ledger_source` は格納を試みた記録からしか立たないので、製品のファイルがあっても項目が 0 件の書庫ではそのソースの「最後の書庫」は進まない。書き出しの範囲の誤りで中身が空の書庫を「書き出した」と数えない厳しい側。
取り込み器（`external-ingestion`）の台帳の作り方は変えない。

- **反転条件**: 本当に使わなかった製品の書庫（項目 0 件）で 75 日ごとに鳴り、本人が煩わしいと言ったとき —— 取り込み器が製品のファイルを見つけたソースに 0 件の行を足す（`external-ingestion` の変更。その時点で触っている Story と相談する）

### D25（仮）. PC の音なしの差し替えは画面の端に出さない（Q4 / spec-review R5）

続く途切れの差し替えは `SuppressPopup = true` で通知の一覧の中身だけを替える（D11）。音つきで出すとき（新しい途切れ）だけ画面の端に出る。
PC は 1 時間ごとに問い合わせるので、毎回画面の端に出すと途切れが続く間は毎時表示され、Q4 の「続く間は音の無い通知を残す」が「鳴り続ける通知」と同じになる。

- **反転条件**: 本人が、通知の一覧に残っているだけでは途切れが続いていることに気づけないと言ったとき（1 日 1 回だけ画面の端に出す、などに変える）

### D21. 明暗はコードを変えず、試験で固定する（C10 / st25 Q5）

S-1 は既に `useScheme` で追従している。spec の Requirement「稼働状況の画面は OS の明暗に追従する」の Scenario を、`web/e2e/coverage-scheme.spec.ts`（新規）で `emulateMedia({ colorScheme: 'light' | 'dark' | 'no-preference' })` の 3 通りの背景色と、ライトの 3 段・印・本文の比を測って固定する。
`surface.spec.ts` の既存の測りは残す（印を足すだけにしない —— Scenario ごとに何を測るかを 1 本ずつ明示する）。

**no-preference の読み替え**（Task 10 F3。`.superpowers/sdd/st14-task-10/task-10-findings.md`）: Scenario「明暗の設定が取れなければダークで出る」の
「取れない」は **`window.matchMedia` が無いこと**として固定する（`addInitScript` で消して、`emulateMedia` の値に従わずダークになるのを測る）。
`no-preference` は「取れない」に当てない —— Chromium は `no-preference` のとき `(prefers-color-scheme: light)` に合うと答えるので、
画面は取得できたライトに従う。この振る舞いはコードを変えずに e2e で記録する（3 通りのうち no-preference は「取れない」の代わりではなく、記録の 1 本）。

- **反転条件**: Chromium が `no-preference` で `(prefers-color-scheme: light)` に合わないと答えるようになったとき（その試験が落ちる。画面がダークに出るのは「取れない」と同じ側なので、試験の期待をそちらへ直す）

## C と D の対応

| C | 何を | D |
|---|---|---|
| C1 | 判定の対象（列・既定は対象・出どころ） | D2 / D3 / D9（仮） |
| C2 | 鳴らさない期間 | D1（spec の対象の (3)〜(5)） |
| C3 | 書庫と取り込み器 | D1 / D4 |
| C4 | 判定の置き場 | D1 |
| C5 | 文面 | D6 / D14（仮） |
| C6 | 台帳 | D7 |
| C7 | 起動直後 | D5（仮） |
| C8 | 破棄の報告では鳴らさない | D4（材料に入れない。spec に Scenario） |
| C9 | 観測でない記録 | D4 / D15 |
| C10 | 申し送りの引き受け | D15 / D16 / D17 / D19 / D21 |
| C11 | 問い合わせの間隔 | D11 / D12 |
| C12 | PC を閉じていた時間 | D4（経過から除かない） |
| C13 | 画面に新しい置き場を作らない | D18（e2e の対象外） |

## Risks / Trade-offs

- [ST44 が収集アプリの資格情報を締めて `GET /outages` が 401 になる] → 試験 `st14_api_outages_bearer` が落ちる。`docs/handoff/ST44.md` に例外として置き済み
- [携帯で鳴るのは止まってから 18〜24 時間後] → 本人の答え（C11 の理由に書いた）。完了の判定「18 時間以上止めると通知が来る」は「以上」で成り立つ
- [サーバが止まっている間は鳴らない] → FR-9 ★ の「S-01 が判定する」と同じ。収集アプリは届かない間、通知を変えない
- [PC の通知が出せない環境（通知を OS で切っている・登録簿に書けない）] → 収集は止めずに警告のログだけ。携帯がもう 1 台の知らせ手になる（Q1 の理由）
- [`windows-latest` の runner で通知の履歴が読めない] → 実行時テストが `BLOCKED_INFRA` を返したら、トーストの組み立て（XML）を単体で確かめた上で、確認バッチの物理の項目（音）で見る。完了にはしない（`.harness2/docs/flow-gates.md` の関門の 3 段）
- [観測ではない種類の表が名前の組で、名前を分けたときに漏れる] → 試験 `st14_nonobs_registry` が登録簿の `c02-%` / `c01-app-usage%` を見て落ちる
- [達成の数え方が変わり、画面の達成日数が変わる] → BREAKING として proposal に書いた。まだどの窓も閉じていない
- [`core.event` から最後の観測の時刻を引く読み出しが重い] → 問い合わせは 1 時間に 1〜2 回。重ければ部分索引を足す（D4）

## Migration Plan

1. 移行 1 本を当てる（`ashiato-server migrate`）。登録簿の 3 列・印・75 日・集計の開始日・台帳の表
2. サーバを入れ替える。起動から 30 分は `warming_up`
3. PC の収集アプリ・携帯の APK を入れ替える。古い収集アプリは `GET /outages` を呼ばないだけで、壊れない
4. 戻すときは `.down.sql`。台帳と 3 列が消え、判定の口は消える。集計の開始日は直した値のまま残る（戻すと格子に導入前の 1〜2 年が出るので戻さない）

## Open Questions

- Takeout の Chrome の履歴（`c03-chrome-history`）が何日ぶんを持つか（FR-35 ★ C8）。本物の書庫が置かれた後に、いちばん古い訪問と書き出しの時刻の差を測る。75 日より短ければ登録簿の日数を縮める（列は本人が変えられる。spec も設計も変わらない）
