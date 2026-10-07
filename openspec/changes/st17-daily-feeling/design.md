# ST17 設計 — 毎日 30 秒で「その日どう感じたか」を残す

## Context

D-01 に主観を書く口も表も無い。取り込み口（`/ingest`）は `origin = 'authored'` を端末識別子なしで受け付け、
ST19 の個人属性の主張（`s01-attribute`）がその形で入っている（`crates/server/src/lib.rs:726-760`、`crates/server/src/attributes.rs`）。
主張の錠は**主張の論理ソースだけ**を見る別関数（`migrations/202609160220_personal_attributes.sql:94-235`）で、主観の行には掛からない。
`tools/check-immutable.sh:797-805` は「主張以外の本人が書いた記録は書き換えられる」を `logical_source = 'immutable-check'` の行で確かめている（ST17 で主観に錠を掛けても落ちない）。

ほかの前提（Explore で確かめた事実）:

- **API の認証は経路で分かれていない。** どの口も `authorize()`（`lib.rs:468-498`）で、収集側の Bearer トークンでも画面の session でも通る（ST29 で絞る。`docs/handoff/ST29.md`）
- **滞在は `core.event` の行**（`logical_source='s01-stay'`、`origin='derived'`、識別子は UUID、始まりは `event_time`、終わりは `payload.end`）。
  吸収は `core.stay_absorbed`（`event_id` → `into_event_id`）、本人の削除は `deleted_by = 'user'`（`rebuild:` 接頭辞は作り直し）（`stay_store.rs:258-266`）
- **稼働状況の口は NFR-13 の 5 ソースだけを引く**（`coverage::must_sources()`。`lib.rs:1999`）
- **Web の経路**は `Root.tsx` の hash（`#/day/…`・`#/master`・ルート）。ST25 の下流がルートを 1 日の画面に、稼働状況を `#/coverage` に移す
- **携帯の収集アプリに時刻で起きる仕組みは無い**（WorkManager / AlarmManager を使っていない。定期処理は前景サービスの `ScheduledExecutorService`）。
  通知は前景サービスの常駐と保持の警告だけ（`LocationService.kt:881-990`）。`POST_NOTIFICATIONS` は既に求めている

動機は `proposal.md` の Why。本人が決めた 13 件と、聞かないで決めた C1〜C10 は `deep.md`。
**ここで決めるのは、それを実装に落とすときの技術判断だけ**で、観測可能な振る舞いは `specs/subjective-log/spec.md` と `specs/browsing-views/spec.md` に置いてある。

## Goals / Non-Goals

**Goals**
- `subjective-log` を作り、主観を積む・読む・S-3 で書く・携帯が促す
- 主観の「書き換えない」を DB が強制し、FR-50 / FR-51 の開口部だけを通す（C2）
- 1 日の画面（S-2）の頭に気分か「未記入」を出し、滞在の行にその滞在の気分を添える（Q1 の軸 4 / ST16 の Q4）

**Non-Goals**
- **主観を消す操作**（画面・API）。消す経路は ST22 / ST23 のもの。ST17 は錠を開けておき、読み出しが削除の印と消去を効かせるところまで
- **S-2 の滞在の行から気分を書くこと。** 書く口は S-3（Q1 の軸 5）。行には読むだけの 1 行を添える（D20）
- **1 日の時刻順の記録（ST25 の `/day/records`）に主観を載せること**（ST25 の許可リストは閉じる側で、主観のソースは無い）。滞在の行の 1 行と頭の欄で足りるかを見てから、後続の `fix/` で決める
- **紐づけ先の種類を足すこと**（ST18）/ **感度を変える操作**（ST24）/ **主観を語で探すこと**（ST26）/ **AI への経路**（ST27）/ **資格情報の範囲**（ST29）
- **PWA・端末での未送信の保持。** 網に届かないときは S-3 が開かない（Q6 の選択肢に書いた代償）
- **`record-envelope` の要件の変更。** 主観は既存の約束のまま入る

## Decisions

### D1. 主観は `core.event` の 1 行。ソースは `s01-feeling`。取り込み口を通す

ST19 の D1 と同じ理由（製造準備 A-1。エンベロープ・削除の印・感度・書き出しが既存のまま掛かる）。

| 列 | 入れるもの |
|---|---|
| `id` | 記録の識別子（画面が送るたびに振る UUID。FR-21） |
| `event_time` | **対象の時刻**（Q12）: 日は `<date>T00:00:00+09:00`、滞在は原文の `target.start` |
| `tz_offset_min` / `tz_id` | `540` / `Asia/Tokyo`（対象は Asia/Tokyo の日で区切る。記入した端末の地域は原文の `written_tz`） |
| `origin` / `device_id` / `external_id` | `authored` / NULL / NULL |
| `raw` | 画面が組んだ JSON の**文字列**。受け取ったまま |
| `payload` | **サーバが `raw` から組み直した値**（NFC。`nonce` を除く。送り主の `payload` は使わない） |
| `sensitivity` | 2（ローカル AI まで。D3） |
| `content_hash` | 既存の `content_hash_of(ソース, 出来事の時刻, 原文)` |

原文の形（`schema_version = 1`）:

```json
{"feeling":"<id と同じ UUID>","nonce":"<128 bit の乱数を base64url>",
 "target":{"kind":"day","date":"2026-10-07"}
        | {"kind":"stay","stay":"<滞在の UUID>","start":"RFC 3339","end":"RFC 3339"},
 "written_at":"2026-10-07T21:30:05.123+09:00","written_tz":"Asia/Tokyo",
 "scales":[{"id":"valence","version":1,"value":1},{"id":"energy","version":1,"value":-1}],
 "note":"..." | null}
```

- **登録簿の行**: `('s01-feeling', '主観', 86400, 'none')`。`expected_gap_sec` は使われない（稼働状況は 5 ソースだけ）が `NOT NULL`
- **書くたびに 1 件増える**: 原文に `feeling`（= `id`）・`nonce`・`written_at` が入るので内容の鍵は記録ごとに違う。届かずに送り直した同じ原文は既存の冪等で 1 件に畳まれる（C2）
- **代わりに考えたもの**: 専用の表。ST19 の D1 と同じ理由で採らない（感度・削除・書き出しを表ごとに持ち直す）

### D2. 主観の行の錠 —— 新しい関数で、ST03 / ST19 の関数は書き換えない

ST19 の D2 と同じ 3 つを、`s01-feeling` について足す: 即時の錠 `core.reject_feeling_rewrite()`（`BEFORE UPDATE`。付け替えと、識別子・利用者・由来・論理ソース・外部識別子・端末・時刻・地域・D-01 に入った時刻・版の変化を拒む）/
COMMIT 時の門 `core.require_feeling_erasure_ledger()`（`raw` / `payload` / `content_hash` の変化は、消去の形で、**その記録の** `core.erasure_ledger` の行が同じトランザクションにあるときだけ）/
行の削除の拒否 `core.reject_feeling_delete()`。削除の印と感度は通す。

- **記録の地域は `Asia/Tokyo`（+540）に固定し、違えば `malformed_feeling`**（spec-review R8）。ST19 は地域の範囲（-1439〜1439）だけを見るが、主観は対象の日を `Asia/Tokyo` で区切るので値が 1 つに決まる
- **ST19 の関数を一般化しない**（`logical_source IN (...)` にしない）。ST19 の `.down.sql` と `check-immutable.sh` の前提が動く。ST21 も同じ判断で場所の錠を別関数にしている
- `tools/check-immutable.sh` に主観の節を足す（spec「主観の記録は書き換えられない」の Scenario を 1 本ずつ）。既存の `immutable-check` の段は触らない（主観の行が DB にあっても通ることを確かめる段を足す）

### D3（仮）. 既定の感度はコードの分岐で持つ

`default_sensitivity()`（`lib.rs:576`）に `s01-feeling` → 2 を足す。取り込みの契約に感度の欄は足さない（ST19 の D3 と同じ）。

- **反転条件**: ST24 が登録簿に「ソースごとの既定の感度」を持たせたとき、この分岐を登録簿の値に移す（値は 2 のまま）

### D4. 消去の後に値を当てられない —— 原文にだけ乱数を入れる

ST19 の D4 と同じ。画面が記録ごとに 128 bit の乱数（`crypto.getRandomValues`）を `nonce` に入れ、`payload` にも他の列にも写さない。サーバは 22 文字以上の base64url であることを検査する。
確かめは 2 か所に分ける —— 乱数の強さと導き方は `web` の `feelings.test.ts`、「乱数を知らなければ鍵を作り直せない」と「解析済みに写らない」は Rust の結合テスト。

### D5. 主観の検査の場所と、理由の種別

`crates/server/src/feelings.rs`（新規）に原文の解釈と形の検査を置き、`ingest_one` は `logical_source = 's01-feeling'` のときだけ呼ぶ（ST19 の `parse_claim` と同じ差し込み方）。
理由の種別と当たる条件、検査の順（表の上から）は spec「形の合わない主観の記録は受け付けない」に置いた（spec-review R9）。紐づけ先の中は 形 → 登録簿 → DB の滞在 → 区間の重なり → 出来事の時刻との一致 の順。
DB を見る検査を最後に置くのは、形の合わない記録で DB を引かないため。

- **尺度の表はコードの定数**: `valence` 版 1（必須）/ `energy` 版 1（任意）、どちらも -2〜+2 の整数。知らない組は断る（器は並びなので、尺度を足すときは表に 1 行足す）
- **ひとことの上限は NFC にして 2,000 文字（Unicode のコードポイントの数。spec に置いた）**（C10「大きめの上限」）。画面の欄は `maxLength`（UTF-16 の単位）を使わず、同じ数え方の関数で数えて超えたら知らせる。前後の空白を除いて空なら `null` として扱う（原文は空のまま残る）
- **滞在の検査**: `core.event` に `id = 鍵 AND user_id = 送り主 AND logical_source = 's01-stay'` の行があること。**削除の印は見ない**（消した滞在・吸収された滞在も通す。spec の理由）。
  `start <= end`、`event_time = start`、**`[start, end]` がその滞在のいまの `[event_time, payload.end]` と重なる**ことを確かめる（spec-review R8。行は消せないので、まったく別の時刻を入れさせない）。
  **一致までは求めない** —— 画面が読んでから書くまでに作り直しが挟まると断ってしまう。書いた時点の値は画面が読んだ値
- **日の検査**: `date` が暦にあり、`event_time = date 0:00 Asia/Tokyo`
- **紐づけ先の種類**は登録簿 `core.feeling_target_kind`（D15）に行があり、かつコードがその種類の検査を持つときだけ受ける。ST18 が種類を足すときは、表の行とコードの検査を同じ change で足す
- 応答は既存の 1 件ごとの結果の形のまま（識別子と種別だけ）。ログにも値・ひとことを出さない（製造準備 A-2）

### D6（仮）. 読み出し —— 紐づけ先の解決と最新の決め方

`feelings.rs` の純粋な関数 `day_feelings(records, stays, today) -> Vec<DayFeelings>` で組む（DB を持たないので単体テストで固定する）。DB からは:

1. 範囲の日に紐づく主観（`core.event_live` の `s01-feeling` で、`event_time` が範囲の日の `Asia/Tokyo` の 0:00 から翌日 0:00 の前まで。`raw <> ''`）
2. 滞在に紐づく記録の滞在の行（削除の印を含めて `core.event` から）と、吸収の台帳 `core.stay_absorbed`

**滞在の分類は `stay_store::Mark` の 4 種に合わせる**（spec-review R12。`rebuild:` の接頭辞だけで分けると、範囲を消して隠れた `rebuild:erased-range` を吸収と読む）:

| `Mark` | 扱い |
|---|---|
| `Live` | その滞在の記録として返す |
| `Absorbed`（`rebuild:` で `erased-range` 以外） | `stay_absorbed` の `into_event_id` をたどり、`Live` に着いたらその滞在の記録として返す（元の滞在の識別子を添える）。16 回で打ち切り、着かなければ表示しない側 |
| `ErasedRange`（`rebuild:erased-range`）/ `UserDeleted`（NULL・`user`・`user:late` など） | 本人が消した。表示しない側（Q10） |

表示しない側の記録は**日の状態には数え**（記録そのものは生きている。Q13「問わない」）、`hidden_stay_records`（件数）と「記録のある滞在の数」にだけ入れる（spec-review R10）。
S-3 は「消した滞在の気分 N 件（ここには出しません）」、S-2 の頭は「気分 滞在ごとに N 件」で、どちらも「記入あり」なのに何も見えない、という形を作らない。

**最新**は（紐づけ先・前/後）ごとに記入の日時 → D-01 に入った時刻の最大。前/後は D7 の式。

- **反転条件**: 消した滞在の気分を日の欄に出したくなったとき（Q10 の第 2 選択肢）/ 消した滞在にだけ書いた日を「記入あり」に数えるのが変だと分かったとき（数えない側へ）/ 吸収の連鎖が 16 を超える実例が出たとき。どれも記録を変えずに読み方だけ変わる

### D7（仮）. 前/後と日の状態

- **前/後**: `written_at < 対象の終わり` なら前。対象の終わりは、日は翌日 0:00 `Asia/Tokyo`、滞在は原文の `target.end`。**読むたびに計算し、`payload` に持たない**（FR-39 ★。後から数え方を変えられる）
- **記録が紐づく日** = `event_time` の `Asia/Tokyo` の日付（Q12 で日も滞在も同じ列で引ける）。日をまたぐ滞在は始まりの日に数える
- **使い始める前** = 利用者の生きた主観のうち `event_time` が最も早い記録の日より前。未来の日に書いた記録しか無いと、今日以前は全部「使い始める前」になる（初めて書いた日が未来の日だけ、という稀な形）
- **反転条件**: 日をまたぐ滞在を両方の日に数えたくなったとき / 「使い始める前」を最初に**書いた日**（記入の日時）で区切りたくなったとき（昔の日をまとめて書き足すと、その間が全部「未記入」に変わるのが煩わしいと分かったとき）

### D8. 読み出しの口 `GET /feelings`

`GET /feelings?from=YYYY-MM-DD&to=YYYY-MM-DD&user_id=`（`user_id` は省けば nil UUID。ほかの読み出しと同じ。範囲は両端を含み最大 62 日）。応答:

```json
{"today":"2026-10-07","first_day":"2026-09-04"|null,
 "days":[{"date":"2026-10-07","status":"recorded"|"missing"|"before_start"|"future",
   "day":{"latest":{"before":Feeling|null,"after":Feeling|null},"records":[Feeling]},
   "stays":[{"stay":"<滞在の UUID>","latest":{"before":…,"after":…},"records":[Feeling]}],
   "stays_with_records":2,"hidden_stay_records":0}]}
Feeling = {"id","scales":[{"id","version","value"}],"note":string|null,
           "written_at":"RFC 3339（地域のずれつき）","ingested_at":"RFC 3339","phase":"before"|"after",
           "target_stay":"<元の滞在の UUID>"|null}
```

- `stays` は記録のある滞在だけ（spec に置いた。滞在の一覧は画面が `/stays` から読む。D13）。`records` は記入の日時の新しい順
- 前の日から続く滞在（D13）の記録は前の日の `days[]` に入る。画面は暦のために 30 日を 1 回で読むので、滞在の識別子で前の日の分も引ける
- OpenAPI に載せる（`tools/check-openapi.sh`）。62 日は S-3 の暦（30 日）と S-2 の 1 日を 1 回で読める大きさに、余裕を持たせた値

### D9. 通知の時刻の台帳と、通知の判断の口

```sql
core.feeling_reminder (id bigint IDENTITY PK, user_id uuid NOT NULL, at time NOT NULL, created_at timestamptz NOT NULL DEFAULT now())
```

- **追記のみ**（UPDATE / DELETE / TRUNCATE を拒む。ST16 の `stay_criteria` と同じ形）。いまの時刻は利用者ごとに `id` が最大の行。無ければ 21:00（Q7。定数 `DEFAULT_REMINDER_AT`）
- `POST /feelings/reminder`（`{user_id?, at:"HH:MM"}`）で 1 行足す。秒は持たない
- `GET /feelings/reminder?date=YYYY-MM-DD&user_id=` → `{"at":"21:00","recorded":true}` の 2 欄だけ。`recorded` は D7 の「記録が紐づく日」で 1 件以上あるか（削除の印・消去を除く）
- **返すのは真偽だけ**（spec）。ST29 までは収集側のトークンで `/events` も `/feelings` も中身ごと読める（ST28 Q4 で本人が「ST29 まで全読みのまま」を選んだ）ので、
  真偽だけにする理由は**いまの漏れを防ぐことではなく**、ST29 で範囲を絞ったときに収集側に残す口を最小にしておくこと（spec-review R5。当初は「中身を返さないので守れる」と書いたが事実と違った）。
  `docs/handoff/ST29.md` に `/feelings` と `/feelings/reminder` を足した**判断（鳴らすか）は端末に置く** —— 網に届かないときに鳴らす（FR-43 ★）のは端末にしかできない
- **代わりに考えたもの**: 通知の時刻を端末の設定に持つ。収集アプリは画面を持たない（Q7 の context）ので、Web で変えて端末が読む（Q6 の答え）

### D10（仮）. 携帯の通知は AlarmManager の不正確な時刻で起こす

`collector-android` に新しいファイルで置く（**収集の経路と分ける**。`LocationService.kt` は起動の 1 行だけ触る）:

| ファイル | 持つもの |
|---|---|
| `FeelingReminderPolicy.kt` | 純粋な判断 `decide(now, at, lastNotifiedDate, recorded: Boolean?) -> Notify / Skip / WaitUntil(t)`。`Asia/Tokyo` の日、1 日 1 回、`recorded == null`（得られない）なら Notify |
| `FeelingReminderScheduler.kt` | `AlarmManager.setWindow(RTC_WAKEUP, at, 15 分, …)` で次の時刻を、同じ形で毎日 12:00 の読み直しを予約する。**正確な時刻の権限は求めない**（C8）。窓の値は定数 `REMINDER_WINDOW = 15 分` |
| `FeelingReminderReceiver.kt` | 起きたら `GET /feelings/reminder?date=` を 1 回（10 秒で打ち切り）→ `decide` → 通知 → 次を予約。12:00 の起床では時刻を読み直して予約し直すだけ。得た `at` と通知した日を `SharedPreferences` に残す |
| `FeelingReminderBootReceiver.kt` | `BOOT_COMPLETED` で予約し直し、時刻を過ぎていれば判断する（`RECEIVE_BOOT_COMPLETED` を足す。実行時の確認の要らない権限） |
| `FeelingNotification.kt` | チャネル `feeling`（IMPORTANCE_DEFAULT）。題「今日はどうでしたか」、本文「気分を残す」。押すと `ACTION_VIEW` で `<webUrl>/#/feel/<その日>` |

- **予約の契機**: 端末の再起動（`BOOT_COMPLETED`）・`LocationService` の起動・受け手が起きるたび。時刻を過ぎていてその日まだ通知していなければ、すぐに判断する。
  **当初は「再起動後も前景サービスが立つ」と書いたが事実と違った**（マニフェストに `BOOT_COMPLETED` の受け手が無く、前景サービスを起こすのは `MainActivity` だけ。spec-review R6）。
  受け手を足さないと、再起動の後はアプリを開くまで通知が出ない。位置の収集が再起動の後に止まったままなのは既存の振る舞いで、この Story では変えない
- **時刻を早めた日**: 端末が新しい時刻を知るのは起きたときだけなので、毎日 12:00 に読み直す（spec-review R7）。12:00 より後に早めた時刻は、その日は前の時刻のまま（次の日から効く）
- **画面の URL が無いビルド**: 通知は出し、`Intent` を付けない（押しても何も開かない）。gradle で止めない —— CI・計測テスト・確認バッチのビルドは `ashiato.webUrl` を渡していない（spec-review R16）
- **Web の URL** は新しい gradle の値 `ashiato.webUrl`（`BuildConfig.WEB_URL`）。`BASE_URL`（サーバの API）とは別の値 —— 携帯は画面を Tailscale の https で開く（Q6 の context）
- **代わりに考えたもの**: WorkManager の 1 日ごとの仕事。依存が 1 つ増え、時刻の幅が最大で数十分になる / 前景サービスの `ScheduledExecutorService`。Doze の間は止まり、21:00 を何時間も過ぎうる
- **反転条件**: 通知が 15 分の窓に収まらない日が続くと分かったとき（`setAndAllowWhileIdle` へ）/ 21:00 の通知を押さずに流す日が続くと分かったとき（Q7 の反転条件。時刻は Web で変えられる）/
  再起動の受け手の権限を減らしたくなったとき（アプリを開くまで鳴らない側へ）

### D11. 通知の文面と状態に主観の中身を持たない

通知の題・本文は定数で、記録の値・件数・前の日の値を組み込まない（C9）。端末に残すのは通知した日と通知の時刻だけで、`recorded` の真偽も残さない。ログに残すのは日付と判断（notify / skip）だけ。

### D12（仮）. S-3 の送り方 —— 押したら 1 件、ひとことは欄を離れたら

`web/src/FeelView.tsx` と `web/src/feelings.ts`（型・原文の組み立て・乱数・前/後と日の状態の表示の規則）。経路は `#/feel`（今日）と `#/feel/YYYY-MM-DD`（`Root.tsx` に足す）。

- **快–不快を押した**: その時点の（快–不快・元気さ・ひとこと）で原文を組み、`POST /api/ingest` に 1 件で送る
- **元気さを押した**: 快–不快が選ばれていれば同じように 1 件。選ばれていなければ画面の状態だけ変える（spec「快不快を選ぶ前は元気さだけを送らない」）
- **ひとこと**: 最後に送ったひとことと違う文字で欄を離れたとき（`blur`）か、欄の下の「ひとことを残す」を押したときに 1 件。快–不快が選ばれていなければ送らずに持っておき、快–不快を押したときに一緒に送る
- **開いたときの選択**: 対象の「日」の、いま書けば付く前/後（`now < 対象の終わり` なら前）の最新の値。もう一方の最新があれば「前に書いた: +2 とても快」/「後に書いた: …」を 1 行
- **送り直し**: 届かなかった原文を持っておき、「もう一度送る」で同じ原文を送る。新しい操作（別の値を押す）をしたら、その原文は捨てて新しく組む（前の原文は届いていないので 1 件も増えていない）
- **結果の読み方**: ST19 の D9 と同じ —— 200 でも 400 でも本文の 1 件ごとの結果を読み、`accepted` と `error` で分ける。本文が読めない・届かない（`fetch` が投げる / 5xx）ときだけ「届かなかった」
- 受理なら `GET /feelings` を読み直す（最新・履歴・暦が変わる）
- **反転条件**: ひとことを書く途中で欄を離れて半端な記録が積まれるのが煩わしいと分かったとき（「残す」を押したときだけにする）/ 押し間違いが多いと分かったとき（Q1 の第 2 選択肢「選んで保存」）

| 種別 | 画面の文 |
|---|---|
| `invalid_note` | ひとことが長すぎます（2,000 文字まで） |
| `invalid_feeling_target` | この日（滞在）には書けませんでした。画面を読み直してください |
| それ以外 | 受け付けられませんでした（種別の名前） |
| 届かなかった | サーバに届きませんでした。選んだ値とひとことはそのまま残っています |

### D13（仮）. S-3 の滞在の並び

対象の日の滞在は既存の `GET /stays?date=`（ST16 / ST22 の `day_view`）の `kind: "stay"` の区間から取る（消した滞在と吸収された滞在は既に出ない）。
各行の最新は `GET /feelings` の `stays[].stay` を**読んだ 30 日の全部から**滞在の識別子で引く（吸収はサーバが解決済み。前の日から続く滞在の記録は前の日に入っているため）。
滞在の行の 5 つのボタンは 44 px（proto の値）、数値だけで言葉は `aria-label` に持つ。送る原文の `start` / `end` は `/stays` の区間の値。

- **行の最新**は日の頭・暦と同じく「後があれば後、無ければ前」（spec-review R11）。2 件以上あれば行ごとに「書き直した履歴 N 件」（Q11 の読み方「対象の日（滞在）ごとに開ける」）
- **前の日から続く滞在**（始まりが対象の日より前）の行には「前の日から」を添える。その行で書いた記録は始まりの日（前の日）に数える（D7）ので、対象の日の状態は変わらない（spec-review R3）。
  毎朝の「寝ていた自宅」の滞在はこの形になる
- **反転条件**: 朝に前の日から続く滞在に書いても今日が「未記入」のままなのが分かりにくいと分かったとき（日をまたぐ滞在を両方の日に数える。D7 の反転条件と同じ）

### D14（仮）. 暦と、S-2 の頭の出し方

- **暦**は対象の日で終わる 30 日（proto の「9 月 4 日〜10 月 3 日」と同じ長さ）。対象の日が未来なら、その日で終わる 30 日に「これから」の日が並ぶ。曜日の列は日曜始まり（proto と同じ）
- 「記入あり」の日は「日」の最新（後があれば後、無ければ前）の快–不快の数値、「日」の記録が無ければ「滞」。「未記入」は破線と「未」。「使い始める前」「これから」は日付だけ
- **S-2 の頭**（`browsing-views`）は `DayView.tsx` の見出しの下に小さい部品 `DayFeeling.tsx` を置き、`GET /feelings?from=<date>&to=<date>` を**並びの読み出しと別に**読む（片方の失敗が他方を巻き込まない）。「書く」は `#/feel/<date>`。
  **1 行・高さ 40 px 以下**（spec に置いた）。ST25 の「開いてすぐ 6 行以上」（ST25 の design は頭に箱を置いて 4 行に減った実測を持つ）を崩さないため、ST25 の 1 日の画面の e2e を ST17 の Task でも走らせる（spec-review R2）
- 色は `tokens.ts` の既存の値だけ。押された状態は ST19 の選ばれた状態と同じ面と文字の組、破線は `tone` の既存の輪郭の色（3:1 は e2e で測る）
- **反転条件**: 暦の長さや始まりの曜日が使いにくいと分かったとき / 滞在だけの日を「滞」より別の書き方にしたくなったとき / 40 px の 1 行に収まらない文言が要ると分かったとき。どれも表示だけ

### D15. 紐づけ先の種類の登録簿

```sql
core.feeling_target_kind (kind text PK CHECK (kind ~ '^[a-z][a-z0-9-]*$'), label text NOT NULL, created_at timestamptz NOT NULL DEFAULT now())
-- 行: ('day','日'), ('stay','滞在')
```

- **追記のみ**（UPDATE / DELETE / TRUNCATE を拒む）。扉 #2「種類を登録簿で持つ多相参照」の器。**主観の行から外部キーで縛らない**（鍵ではなく索引。C5）—— 種類を足すときに既存の行に触れない（FR-38。ST18 の完了の判定）
- 種類の名前は原文の `target.kind` と同じ文字列。解析済みにも同じ形で写す

### D16. 移行は 1 本

`migrations/YYYYMMDDHHMM_subjective_log.sql`（作成時刻）と `.down.sql`。中身は D2 の 3 関数とトリガ、D9 の台帳と錠、D15 の登録簿と錠、登録簿 `core.source` の 1 行。
当て直せる形（`IF NOT EXISTS` / `CREATE OR REPLACE` / `DROP TRIGGER IF EXISTS` / `ON CONFLICT DO NOTHING`）。`MIGRATIONS` 配列の末尾に足す。
`.down.sql` は ST19 の D12 と同じく、**主観の行か通知の時刻の行が 1 つでも残っていれば**、登録簿の行・紐づけ先の種類の表・通知の時刻の台帳を残す（本人が書いたものは作り直せない）。錠の関数とトリガは落とす。
アプリの役割（ST28 の `grants.sql`）に、新しい 2 表の SELECT / INSERT を足す。

### D17. 完了の判定を機械で確かめる

| 完了の判定 | 確かめるもの |
|---|---|
| 通知から開いて 30 秒以内に、快–不快の 5 段階を入れて保存できる | e2e（Scenario `通知から開いて 1 回押すと保存される`。ページを開いてから保存の応答までを測り、1 タップで 30 秒を下回る）と、Android の計測テスト（Scenario `通知を押すと今日の主観入力の画面が開く`。通知の `Intent` の URI） |
| 保存した記録に、対象の日付と記入した日時が別々に入っている | 結合テスト（Scenario `日の記録の出来事の時刻はその日の 0 時`） |
| 昨日の分を後から書ける | 結合テスト（`昨日の分を後から書ける`）と e2e（`昨日の分は前の日へ 1 回で書ける`） |
| 書いていない日が画面で区別されて出る | e2e（`暦に未記入の日が破線と未で出る` / `書いていない日の 1 日の画面の頭に未記入が出る`） |

### D18. 変えないもの

- `record-envelope` の要件と取り込みの契約の形。足すのは理由の種別の値だけ
- ST03 / ST19 / ST21 の錠の関数、ST16 / ST22 の滞在の経路（`/stays` は読むだけ）
- `coverage.rs`（主観は `must_sources()` に無いので何もしない。試験で固定する）
- `LocationService.kt` の収集と送信（起動時に通知の予約を 1 回呼ぶ行だけ足す）。マニフェストには通知の受け手 2 つと `RECEIVE_BOOT_COMPLETED` を足すだけ
- ST25 の `/day` と `/day/records`（並走中。S-2 の頭の部品は並びの読み出しと別に読む）

### D19（仮）. 主観のソースは途絶の判定に入れない

C7（本人は異論なし）を要件に戻した: FR-35 に「本人が書くソース（主観・個人属性）は途絶の判定の対象にしない」を ★ 2026-10-07 で足した（spec-review R4。FR-35 の逐語どおりに作ると、登録簿の `expected_gap_sec = 86400` で 3 日書かなければ途絶になる）。
途絶の判定は ST14 が作る。ST17 は稼働状況の口に入らないこと（`must_sources()`）を試験で固定し、`docs/handoff/ST14.md` に「`s01-feeling`（と `s01-attribute`）を外す」を書いた。

- **反転条件**: 書き忘れを途絶と同じ通知でも知りたいと本人が言ったとき（ただし FR-43 の通知と二重になる）

### D20（仮）. S-2 の滞在の行に気分を 1 行添える

ST16 の深掘り Q4 で本人が決めた「S-2 の 1 行は主観の本文を行の中に出す。置き場を作るのは ST17」（`docs/ui-direction.md` ★ 2026-09-13）を、ST17 の Q1（書く口は S-3）は取り消していない（spec-review R1。当初は Non-Goals に置いて落としていた）。
`DayView.tsx` の滞在の行（ST25 の後の形）に「気分 -1 不快」を 1 行添える。読むのは頭の欄と同じ `GET /feelings` の 1 日分（`stays[]`）で、行の最新は D13 と同じ規則。ひとことは行に出さない（行の高さを増やさない）。

- **反転条件**: 行の高さが増えて開いてすぐ見える行が減ったと分かったとき / ひとことも行で読みたいと分かったとき。どれも表示だけ

## Risks / Trade-offs

- [ST25 の下流が `DayView.tsx` と `Root.tsx` を組み替えている] → ST17 の下流は admission が ST25 の archive を待つ（同じ `browsing-views`）。S-2 の頭は見出しの下に部品を 1 つ差すだけにして、並びの部品に手を入れない
- [ST29 までは収集側のトークンで主観の中身も読める（`/events` と `/feelings`）] → ST28 Q4 の本人の決定（ST29 まで全読み）のまま。読み出しの記録には残る。`docs/handoff/ST29.md` に口を足した
- [記入の日時は画面の端末の時計] → ずれていると前/後がずれる。D-01 に入った時刻（サーバの時計）が別の欄に残るので後から突き合わせられる
- [網に届かないと S-3 が開かない] → 本人が Q6 で承知して選んだ。後で過去の日として書ける（FR-42）
- [ひとことを欄を離れるたびに送る] → 書き直すたびに記録が増える（D12 の反転条件）。消えるよりは増える側
- [`lib.rs` の `MIGRATIONS` と route / `docs/openapi.json` / `Root.tsx` / `tools/*.sh` に ST21・ST25・ST08・ST12 と並んで追記する] → 後から merge する側が追従する。移行の名前は作成時刻で取り合わない

## Migration Plan

1. 移行を当てる（起動のたびに全版を当てる既存の形）
2. 既存の行は触らない（主観のソースの行はまだ無い）
3. 戻すときは `.down.sql`。主観の行か通知の時刻の行があるうちは、登録簿と台帳が消えない（D16）
4. 携帯は APK の入れ替え。`ashiato.webUrl` が無いビルドは通知を出すが、押しても画面を開かない（D10。ビルドは止めない）
