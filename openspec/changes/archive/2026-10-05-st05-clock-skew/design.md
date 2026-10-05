# ST05 design —— 端末時計のずれを測って残す

## Context

動機は `proposal.md` の Why。振る舞いは `specs/device-collection/spec.md` と `specs/desktop-collection/spec.md`。
ここには「どう作るか」と、仮で決めたこと（（仮）と反転条件）だけを書く。

いまの状態（2026-09-29 に読んだもの）:

- **携帯端末（`collector-android`）は時計のずれを 1 行も測っていない。** 位置の記録の出来事時刻は
  `Location.getTime()`（`FixCollector.kt`）。原文と解析済みの項目は `lat` / `lon` / `acc_m` / `device_time` / `device_id` だけ（`LocationFix.kt`）
- 端末の 3 つの時計（壁時計・`elapsedRealtime`・`Settings.Global.BOOT_COUNT`）は ST04 が `DeviceClock`（`AgeClock.kt`）に置いた。
  `AgeClock` は壁時計と単調時計が **1 時間を超えて**食い違うとログに `clock_jump` を出すだけ
- `HttpTransport.post()` は状態符号と本文だけを返し、応答の見出し（`Date`）を読まない
- **PC（`crates/collector-windows`）は ST07 が測っている。** `clock.rs` の `HttpDateClock` が 1 時間ごとに
  `/healthz` を叩いて応答の `date` を読み、`c02-window` の `kind = clock-skew` として 1 件残す。差には**見回りの先頭で読んだ壁時計**を使う
  （`runtime.rs` の `maybe_measure_skew`）。取れなければ記録を残さず 60 秒後に測り直す。起動時と、壁時計が単調時計と 60 秒以上食い違ったとき
  （`CLOCK_JUMP_SEC`）は契機を戻してその場で測る。`/healthz` は見回りの輪の中で同期に叩く（`sender::agent()` の打ち切りまで輪が止まる）
- 登録簿（`core.source`）の `external_id_kind` の既定は `'record'`（`202609120940_source_columns.sql`）。宣言しないソースは識別子を欠く記録を断られる

## Goals / Non-Goals

**Goals**

- 端末と PC の両方で、1 時間ごと（と起動時・時計の変更時）に、取れる基準を全部並べた測定記録を 1 件残す
- 取れなかった契機も 1 件残す。基準ごとに読む前後の経過時間・起動の識別を持つ
- 位置の記録に、受け取ったときの端末の時計の時刻・測位の経過時間・起動の識別を足す

**Non-Goals**

- **記録の時刻の補正**（C4）。補正は後から派生で作る。この change は測って残すだけ
- **ずれの画面**。ブリーフの面は「なし」。稼働状況の画面は Must の 5 ソースしか出さない（`coverage_get`）ので変わらない
- **取り込み口と稼働状況の数え方の変更**。登録簿に 1 行を足す移行だけで、`/ingest` の検査・応答・冪等キー、`coverage.rs` は変えない
- **ST04 の `AgeClock` の変更**。`DeviceClock` を読むだけ。時計の変更の検知は D3 で別に持つ

## Decisions

### D1. 端末の測定記録は論理ソース `c01-clock`。識別子の種類は `none`、生存信号は送らない（想定間隔は仮）

移行 1 本で `core.source` に `('c01-clock', '携帯端末の時計のずれ', 21600, 'none')` を足す（名前は作成時刻 `YYYYMMDDHHMM_clock_source.sql`）。
`ON CONFLICT (logical_source) DO NOTHING`、`.down.sql` は `c01-clock` の記録が 1 件も無いときだけ行を消す。

- **別のソースにする理由**と**生存信号を送らない理由**は spec（C1）。`coverage.rs` の `DEVICE_SUBJECT` は変えない
- **`external_id_kind = 'none'`**: 測定は外部サービスの記録ではない。既定の `'record'` のままだと全件が `missing_external_id` で断られ、
  理由が「受け手側の設定で変わりうる」なので端末の未送信に残り続ける（深掘りレビュー R3）
- **想定間隔 21600 秒（仮）**: 使う者がいまはいない（稼働状況の画面に出ない）。登録簿の 1 行なので後から戻る
- **並走する ST06 の「ソースごとに独立して収集する」**（取得できないソースも生存信号を送り続ける・全ソースの生存信号の区間に取得契機が入る）の
  「ソース」は収集のソースで、`c01-clock` は含まない —— ST05 の spec が「収集のソースではなく、生存信号を送らない」と書いて境界を引く。
  ST06 へ戻すものは無い（spec レビュー R5）

反転条件: ST14（途絶の通知）が登録簿の全ソースに途絶を判定するなら、`c01-clock` は生存信号が無いので途絶に見える ——
そのときは ST14 の側で「測定のソースを除く」か、`c01-clock` の想定間隔を 3600 秒にして記録の到着で途絶を判定する（どちらも登録簿か ST14 の中で閉じる）。

### D2. 端末の基準の読み方（3 つ。どれも通信を起こさない）

| 出どころ（`source`） | 読み方 | 取れないとき（`reason`） |
|---|---|---|
| `network` | `SystemClock.currentNetworkTimeClock().millis()`（API 33 以上） | API 30〜32 は `unsupported`（OS の版）、`DateTimeException` は `not_available`（いま取れない） |
| `gnss` | `SystemClock.currentGnssTimeClock().millis()`（API 29 以上） | `DateTimeException` は `not_available` |
| `s01-date` | **送信がすでに受け取っている応答**の `Date` 見出し（`/ingest` `/heartbeat` `/drops` のどれでも） | 前回の測定より後に応答が無い `no_response_since_last`、時計の変更より後に応答が無い `clock_changed_since`、見出しが無い・読めない `unreadable` |

- **読む直前と直後の単調時計**は `SystemClock.elapsedRealtime()`。`network` と `gnss` は呼び出しの前後、
  `s01-date` は**接続を開く直前と応答を読み終えた直後**（`HttpTransport` の中で取る）
- **差**（`skew_ms`）は「端末の壁時計 − 基準の時刻」。壁時計は**その基準を読む直前と直後の間で**読む ——
  `network` / `gnss` は読んだ直後、`s01-date` は応答を読み終えた直後（`HttpTransport` の中）。測定の先頭で 1 回読んだ壁時計を使い回さない（spec の「前後の間で読む」。PC の Q3 ② と同じ穴を端末で作らない）。
  HTTP の日付は秒で切り捨てなので `s01-date` の差は 0〜+999 ms 大きく出る（ST07 design D17 と同じ）
- **`s01-date` の置き場**: `HttpTransport` の `Outcome.Responded` に `date`（見出しの文字列そのまま）・`monoBeforeMs`・`monoAfterMs`・`wallAfterMs` を足し、
  `Sender` が受け取った最後の 1 件を**メモリに**持つ（`ResponseDateCache`）。**測定で取り出したら空にする**（「前回の測定より後」を構造で守る）。
  **時計の変更の通知（D3）を受けたら空にする**（`clock_changed_since` の理由を残すため、空にした事実を持つ）。
  プロセスの立て直しで消えるが、起動時の測定は送信より前に走るので元々取れない（`no_response_since_last` で残る）
- **読み取りは例外を外へ出さない。** 1 つの基準の失敗は、その基準の `reason` に種別で残る（`error:<例外の型名>`）
- 基準の口（`ClockReferences` と OS の時計の読み取り `SystemTimeSources`）は `Transport` にも網にも依存しない

### D3. 端末の測る契機 —— 1 時間・起動時・時計の変更（変更の検知は OS の通知。仮）

- **1 時間**: `LocationService` に刻みを 1 本足す（`newClockScheduler()`。`ExecutorFlushScheduler` と同じ形）。
  Doze で遅れるのは受け入れ済み（ST01 の R46）
- **起動時**: `onStartCommand` で刻みを立てるときに 1 回測る（`START_STICKY` の立て直しも含む）。生存信号の起動時の 1 発と同じ型
- **時計の変更**: サービスの中で `Intent.ACTION_TIME_CHANGED` の受け手を動的に登録する（`onCreate` で登録、`onDestroy` で解除）。
  OS は変更の大きさによらず通知するので、spec の「60 秒以上」を満たす。タイムゾーンの変更（`ACTION_TIMEZONE_CHANGED`）では測らない ——
  壁時計のエポックからのミリ秒は変わらない
- 記録の `trigger` に `hourly` / `start` / `time_set` / `retry` のどれで測ったかを残す

**確かめる順**: 下流の最初の Task（Task 1）で、エミュレータで `ACTION_TIME_CHANGED` が動的な受け手に届くかを確かめ、結果をこの D3 に追記する。
反転条件（仮）: 届かないと分かったら、1 分ごとの軽い見回り（壁時計と `elapsedRealtime` の進みの差が 60 秒以上なら測る。PC の `CLOCK_JUMP_SEC` と同じ）へ倒す。
spec の「60 秒以上変更されると測る」はどちらの方式でも成り立つので、spec は変わらない。

**ACTION_TIME_CHANGED の確かめ（2026-09-29・Task 1.2）**:
- (a) 端末の時計は変えられた。API 35 の `google_apis` エミュレータで `settings put global auto_time 0` の後、計測テストの
  `UiAutomation.executeShellCommand("cmd alarm set-time <ms>")` が rc 0・出力なしで通り、`System.currentTimeMillis()` が指定した値へ跳んだ（+5 分と −5 分の 2 回）
- (b) `Intent.ACTION_TIME_CHANGED` は、テストのプロセスが `registerReceiver(rx, IntentFilter(Intent.ACTION_TIME_CHANGED), RECEIVER_EXPORTED)` で
  動的に登録した受け手に、**2 回とも数 ms 以内に届いた**（全ての実行で。届いた `action` の文字列は `android.intent.action.TIME_SET`）。
  `Intent.ACTION_TIME_CHANGED` の定数の値が `TIME_SET` であって、別の通知ではない
- 結論: **反転条件には倒さない**。D3 は「サービスの動的な受け手で `ACTION_TIME_CHANGED` を受ける」のまま、Task 6 もこの方式で作る。
  限界: 受け手を登録したのは計測テストのプロセスで、`LocationService`（前景サービス）の中ではない。サービスの中での受信は Task 6 の計測テストが担保する

### D4. 取れなかった契機と測り直し（1 時間の契機ごとに取れなかった記録は 1 件まで。測り直しは 5 分ごと。仮）

1 時間・起動時・時計の変更の契機で測って**基準が 1 つも取れなければ**、`available: false` の記録を 1 件積み、
「測り直し中」の印をメモリに立てる。印が立っている間、**5 分ごと**（専用の刻み）に測り直し、
取れなければ何も積まない。取れたら `trigger: retry` の記録を 1 件積んで印を下ろす。次の 1 時間の契機が来たら印は下ろす（その契機で改めて測る）。

- 圏外が 1 日続くと取れなかった記録は 24 件（spec）。測り直しで積むと 1 日 288 件になる
- 5 分は仮。反転条件: `c01-clock` の `trigger = retry` の記録が、1 週間で `hourly` の記録の半分を超えるなら（取れる / 取れないが細かく揺れている）、
  測り直しを 15 分にする。**観測する係**: 確認バッチの準備（`tools/verify-prep.sh`）が、この比を出す SQL を走らせて手順書に数を載せる（Task 10）。
  spec は「次の 1 時間の契機までの間に測り直す」だけを定めているので変わらない
- **数える DB（仮。review R16）**: 確認バッチの DB で数える。偽データ（`tools/seed.sh`）は `c01-clock` を作らないので、数は確認の間に本物の端末が送った分だけで、
  `hourly` が 0 件なら判定しないと手順書に書く（`tools/verify-prep.sh`）。1 回のバッチでは 1 週間ぶんが溜まらないことが多いのは受け入れる。
  反転条件: 確認バッチが 3 回続けて「`hourly` が 0 件で判定しない」を出したら、数える係を確認バッチから外し、本番の S-01 の DB を数える口（サーバの定期の集計）に移す

### D5. 端末の測定記録の形（`c01-clock`、`schema_version = 1`）

原文（`raw`）は解析済み（`payload`）と同じ JSON を `ingestJson` で直列化した文字列（位置と同じ。ST01 design D16）。

```json
{
  "kind": "clock-skew",
  "trigger": "hourly",
  "available": true,
  "device_time": "2026-09-29T01:00:00.123Z",
  "elapsed_ms": 123456789,
  "boot_count": 42,
  "references": [
    {"source": "network",  "time": "2026-09-29T00:55:00.100Z", "skew_ms": 300023, "mono_before_ms": 123456780, "mono_after_ms": 123456781},
    {"source": "gnss",     "time": "2026-09-29T00:55:00.090Z", "skew_ms": 300033, "mono_before_ms": 123456782, "mono_after_ms": 123456783},
    {"source": "s01-date", "time": "2026-09-29T00:51:00Z", "skew_ms": 300456, "mono_before_ms": 123200000, "mono_after_ms": 123200310,
     "raw": "Tue, 29 Sep 2026 00:51:00 GMT", "host": "s01.lan:8787"}
  ],
  "unavailable": []
}
```

- `device_time` = 出来事時刻（`event_time`）。**測ったときの端末の壁時計**（C7）。`elapsed_ms` はその瞬間の `elapsedRealtime`
- `boot_count` は `DeviceClock.bootCount()`（ST04）。取れない端末では `null`（spec の「取れないことを示す値」）。そのときは `elapsed_ms` が戻ったことで起動を知る
- `unavailable` は `[{"source": "network", "reason": "unsupported"}, …]`。**3 つの出どころが `references` と `unavailable` のどちらかに 1 回ずつ**（spec）
- `s01-date` が `unreadable`（`Date` 見出しを読めなかった）のときは、`unavailable` の 1 件に `raw`（見出しの原文）と `host` も載せる（review R18。読み方を直せば後から差を出せる）
- `available` は `references` が 1 件以上か。取れなかった記録は `references: []`
- 時刻の書き方はミリ秒まで・UTC・`Z` 終わり（`Instant.toString()` は秒ちょうどでミリ秒を落とすので、ミリ秒に固定する書き方を 1 か所に置く）
- `tz_offset_min` / `tz_id` は位置と同じく `ZoneId.systemDefault()`。`device_id` は位置と同じ端末識別子。`origin = collected`
- **ログに出すのは件数・種別・`available` だけ**（時刻の値・差を出さない。spec）

### D6. 位置の記録に足す項目（Q2）

`LocationFix` に 4 つ足し、原文と解析済みの両方に載せる:

| 欄 | 値 | 出どころ |
|---|---|---|
| `received_device_time` | 受け取ったときの端末の壁時計（ミリ秒まで・UTC） | `onLocationResult` の中で `DeviceClock.wallMs()` |
| `fix_elapsed_ns` | 測位の結果が持つ起動からの経過時間（ナノ秒） | `Location.getElapsedRealtimeNanos()` |
| `received_elapsed_ms` | 受け取ったときの起動からの経過時間 | `DeviceClock.monoMs()` |
| `boot_count` | 起動の識別（取れなければ `null`） | `DeviceClock.bootCount()` |

- **`received_elapsed_ms` は Q2 の選択肢に無い 4 つ目**だが、C（列を持つ）。`fix_elapsed_ns` と並べると
  「測位から受け取りまでの遅れ」が引け、`received_device_time` から測位の時点の端末の壁時計を戻せる
- 出来事時刻（`event_time` と `device_time`）は `Location.getTime()` のまま（C4 / spec）
- **原文が変わるのは新しい記録だけ。** 未送信に積まれた記録は積んだときの文字列のまま送られるので、再送で鍵は変わらない
- `schema_version` は 1 のまま（仮）。欄を足すだけで既存の欄の意味は変わらない。
  反転条件: 読む側（ST16 の滞在の導出など）が欄の有無で分岐する必要が出たら 2 に上げる（新しい記録だけ）
- 下流の最初（Task 1）に、**融合プロバイダの `Location.getTime()` がどの時計から来るか**をエミュレータで確かめ、
  `docs/collector-contract.md` の位置の節に結果を書く（Q2 の context の「確かめる手順」）。結果がどちらでも spec は変わらない

### D7. PC の取り込み口の基準は、測定の契機に `/healthz` を叩いて読む（ST07 のまま。仮）。読み取りは見回りの輪の外で行う

- **ST07 の `HttpDateClock`（`/healthz` を叩く）を残す。** 送信の応答から読む案（spec レビュー前の D7）は捨てた ——
  手元の応答は時計が飛ぶ前のもので、**飛んだ直後の測定に飛んだ後の差が 1 つも残らない**（Windows の同期の状態は差を持たない。spec レビュー R2）。
  宛先は自宅の S-01 で外部には出ない。FR-7 の C-02 の文をこれに合わせて直した（★ 2026-09-29）
- **差に使う PC の壁時計は、応答を受け取った直後に読む**（Q3 ②）。`ReferenceClock::now()` の戻り値を
  「基準の時刻・読む直前と直後の起動からの経過時間・受け取った直後の壁時計」の組にする。見回りの先頭の `wall` を `measure` に渡さない
- `source` は `s01-date`、`host` は基点 URL の `host:port`（いまの `skew_reference` と同じ値）
- **読み取りは見回りの輪の外で行う**（spec「基準の読み取りを待つ間も前景の観測を止めない」。spec レビュー R8）。
  測定の契機に、`/healthz` と Windows の時刻同期の状態（D8）を**1 本の作業スレッド**で読み、結果を通り道（チャネル）で返す。
  見回りは毎回通り道を覗き、結果が来ていれば測定記録を積む。作業スレッドが走っている間は次の測定を始めない
  （測り直しの 60 秒・契機の戻しはどちらも「前の読み取りが終わってから」）

反転条件（仮）: 本人が「PC も S-01 へ測るための要求を足さない」を選ぶなら、送信の応答から読む方式（端末の D2 と同じ）へ倒す。
そのときは**時計の飛びの契機で飛ぶ前の応答を使わない**（端末の `clock_changed_since` と同じ）ことと、
飛んだ直後の測定に取り込み口の差が残らないことを spec の Scenario（`時計が飛んだ直後の測定は飛んだ後の差を持つ`）ごと直し、FR-7 の C-02 の文も戻す。

### D8. Windows の時刻同期の状態は `w32tm /query /status /verbose` の出力を原文で持つ（仮。Task 1 で確かめる）

`TimeSyncSource` の口を置き、本番は `w32tm /query /status /verbose` を子プロセスで走らせる（5 秒で打ち切る。D7 の作業スレッドの中）。

- **出力は原文のまま** `raw` に入れ、解析できた欄（最後に正常に同期した時刻・同期元・位相のずれ）だけを解析済みに入れる。
  出力の見出しは OS の表示言語で変わるので、**英語と日本語の見出しの両方**を解析する。解析できなければ `raw` を持ったまま `reason: unparsed`
- この基準の `skew_ms` は置かない（時刻そのものではなく状態）。OS の見積もったずれ（位相のずれ）が読めたら `os_offset_ms` に入れる
- 取れないとき（子プロセスが起動しない・非 0 で終わる・打ち切り）は `unavailable` に `reason`（`spawn_failed` / `service_stopped` / `exit:<code>` / `timeout`）
- **子プロセスに渡す引数は `/query /status /verbose` に固定する**（試験で固定する）。`/resync` や `/config` は渡さない。外部への通信は起きない
- **Task 1 で確かめ、結果（読めたか・エラー符号・表示言語・出力の見出し）をこの D8 に追記する**:
  管理者権限なしで読めるか（`windows-latest` の実行時テストと手元の Windows の両方）
- 反転条件: 管理者権限なしでは読めない（`0x80070005` など）と分かったら、**最後に同期した時刻と同期元が読める別の口**を探す
  （候補: System のイベントログの `Microsoft-Windows-Time-Service` の同期の記録。一般の利用者で読めるかも Task 1 で確かめる）。
  設定（レジストリの `NtpServer`）や調整量（`GetSystemTimeAdjustment`）は「最後に同期した時刻」ではないので代わりにしない（spec レビュー R7）。
  **どの口でも読めなければ、Q3 の ③（同じ機械の構成で独立に比べる）の前提が崩れるので、実装で決めずに `deep.md` に R 番号つき `(未回答)` で書いて止まる**

**w32tm の確かめ（2026-09-29・Task 1.3）**:
- **読めた。反転条件には倒さない。** 終了コード 0。手元の Windows（日本語表示）で、管理者権限あり（WSL から起動したプロセスは High 完全性）と
  **管理者権限なし**（`schtasks /create /rl LIMITED` で Medium 完全性にして実行。`whoami /groups` で `Medium Mandatory Level` を確認）の両方で同じ出力が読めた。エラー符号（`0x80070005` など）は出なかった
- 表示言語: 手元は**日本語**。出力は**コンソールの符号ページ（cp932）のバイト列**で、UTF-8 ではない（`from_utf8_lossy` では見出しが化ける）。
  子プロセスの出力は**バイト列で受けて `raw` に持ち**、見出しの照合はバイト列（または cp932 の復号）で行うこと（Task 8.1 への注意）
- 見出し（日本語 / 英語の対応。英語は Windows の既定の出力）。行の順序は両言語で同じ:
  `閏インジケーター` / `Leap Indicator`、`階層` / `Stratum`、`精度` / `Precision`、`ルート遅延` / `Root Delay`、`ルート分散` / `Root Dispersion`、
  `参照 ID` / `ReferenceId`、**`最終正常同期時刻` / `Last Successful Sync Time`**、**`ソース` / `Source`**、`ポーリング間隔` / `Poll Interval`、
  **`フェーズ オフセット` / `Phase Offset`**、`クロック レート` / `ClockRate`、`最終同期エラー` / `Last Sync Error`、`最終正常同期時刻からの時間` / `Time since Last Good Sync Time`。
  英語の見出しは記憶による**未実測の推定**（この機械で実測したのは日本語だけ）。英語の出力は `windows-latest` の実行時テストの失敗ログ（英語の Windows）で確かめ、違っていれば Task 8.1 で見出しを直す
- 値の癖: 同期していない機械（手元）では `最終正常同期時刻: 未指定`（英語は `unspecified`）、`ソース: Local CMOS Clock`、`最終同期エラー: 1 (…)` だった。
  つまり**「最後に同期した時刻」が無いのは正常な状態**で、同期元だけが解析できる。実行時テスト `clock_time_sync_is_readable` が「時刻か同期元のどちらか」を assert するのはこのため
- 実行時テスト `clock_time_sync_is_readable`（`crates/collector-windows/tests/runtime_windows.rs`）は、`w32tm /query /status /verbose` を本物で走らせ、
  終了コード 0 と、上の 2 見出し（両言語）のどちらかが解析できることを見る。手元（WSL から Windows 側の cargo）で PASS。`windows-latest` の結果は CI で確かめる。
  `TimeSyncSource` はまだ無いので、テストは同じ引数の子プロセスを直接走らせる（Task 8.1 で本物の `TimeSyncSource` に差し替える）

**W32Time が止まっているとき（2026-09-30。本人の指示）**:
- W32Time の起動の種類は既定で**手動（トリガー起動）**（手元は `DEMAND_START`）で、**止まっていることがある**。止まっていると `w32tm` は
  `0x80070426`（サービスが開始されていない）で終わる。Task 1.3 の確かめは**たまたま動いていたとき**のもので、「いつでも読める」の根拠ではなかった
- 製品はそれを異常にしない: `unavailable` に `reason: service_stopped` で残す（`exit:-2147023834` のままでは止まっていたと読めない）。
  **サービスを起動しない・起動の種類を変えない**（利用者の OS の設定を収集のために変えない）
- 実行時テスト（`clock_time_sync_is_readable` / `clock_skew_runtime`）は **その時の状態で経路を分ける**: 動いていれば取れた経路を、
  止まっていれば `service_stopped` の経路を見る。状態は `sc query w32time` の STATE の数値（権限不要）を読む前と後で見て、結果と食い違えば落ちる。
  出力の解析そのものは実 OS に依らない固定入力の単体テスト（`time_sync_parses_english_output` など）が持つ

**止まっている間はイベントログから最後の同期を読む（2026-09-30。deep Q4 の本人の答え。review R22）**:
- `w32tm` が `0x80070426`（サービスが開始されていない）で終わったときだけ、`%SystemRoot%\System32\wevtutil.exe qe System
  "/q:*[System[Provider[@Name='Microsoft-Windows-Time-Service'] and (EventID=35 or EventID=37)]]" /c:1 /rd:true /f:xml` を走らせる
  （同じ 5 秒の打ち切り。**読み取りは最大 2 本 × 5 秒 = 10 秒**になる。同じ作業スレッドの中で、`mono_before_ms` / `mono_after_ms` は 2 つの子プロセスをまたぐ）。
  **引数は照会（`qe`）だけに固定し、試験で固定する**（`time_sync_event_log_arguments_are_pinned_to_the_query`）。サービスは起動しない・起動の種類も変えない
- 35 = 同期元を選んで同期している / 37 = 同期元から正しい時刻を受けている。**新しいほうから 1 件**の `TimeCreated` の `SystemTime` を `last_sync`
  （**UTC の RFC 3339**。`w32tm` の表示のままのローカル時刻とは形が違う）、`TimeSource` を `sync_source` にする。`raw` はその XML の原文。
  `os_offset_ms` は無い。どちらから読んだかは `clock_references[]` の `sync_via`（`w32tm` / `eventlog`）に残す。項目を読めなければ `unparsed`（`raw` つき）
- **記録が 0 件（出力が空）・読めない（権限・非 0・打ち切り）ときは `service_stopped` のまま**（止まっていたことだけを残す。deep Q4 の推奨の条件）。
  空でないのに `<Event` の無い出力は 0 件と見なさず、`unparsed` で原文を持つ
- 子プロセスの口は `w32tm_program()` / `wevtutil_program()` の 2 つだけ（exe 名を引数に取らない）。`tools/check-no-time-server.sh` はこの 2 つの形しか `Command::new` に許さない
- **一般の利用者の権限で読めるか**: System のチャネルの ACL（`wevtutil gl System` の `channelAccess`）が `(A;;0x1;;;IU)`（対話ログオンの利用者に読み取り）・
  `(A;;0x1;;;S-1-5-32-573)`（Event Log Readers）を持つ（2026-09-30 手元で読んだ）。PC の収集は対話ログオンの利用者の下で動くので読める側に入る。
  権限を下げたプロセスでの実測は、タスクスケジューラへの登録が実行の許可で止められたので**していない**（Task 1.3 の w32tm は同じ方法で実測した）。
  読めなかったときは上のとおり `service_stopped` に戻るので、記録は失われない
- 実行時テスト `clock_time_sync_event_log_is_readable` は W32Time の状態に依らずイベントログを本物で読み、読めること（0 件は読めた扱い）と最新の記録から時刻と同期元が取れることを見る。
  `clock_skew_runtime` は通った経路（`w32tm` / `eventlog` / `service_stopped`）ごとに、W32Time の状態・イベントログの記録と食い違わないことを見る。
  2026-09-30 の手元の実行では W32Time は**動いていた**（起動の契機で動いた）ので、実機で通ったのは `w32tm` の経路とイベントログを読む口で、
  止まっている経路の切り替えは単体テスト `time_sync_stopped_service_falls_back_to_the_event_log` が持つ
- 反転条件: 一般の利用者で読めないと分かったら、この口は常に `service_stopped` に倒れるだけなので、別の口を探すか deep に返す

### D9. PC の起動の識別と単調時計

- **起動の識別** = OS が最後に起動した時刻（`Source::boot_time()`。いまの `powered-off` の `boot_at` と同じ値・同じ欄名）
- **起動からの経過時間** = Windows の `GetTickCount64()`（ミリ秒。スリープの間も進む）。非 Windows のビルドでは試験用の偽物。
  `Uptime` の口に置く。`runtime.rs` の `clocks()`（プロセスの中の `Instant`）は見回りの判定に使うもので、記録には載せない
  （プロセスの立て直しで 0 に戻り、起動の識別と組にならない）
- **D9（仮）: 本番の刻みは秒**（sysinfo の `uptime()` ×1000。unsafe を書かないため `GetTickCount64` を直接呼ばない）。
  `Uptime::resolution_ms()` を持ち、読んだ直後の値に `刻み - 1` を足して `mono_after_ms - mono_before_ms` を実際の読み取り時間の**上限**にする。
  反転条件: ミリ秒で読める安全な口が入る、または幅の精度が ST07 の判定に足りないと分かったら `GetTickCount64` に替える（`SystemUptime` の中だけ）。

### D10. PC の測定記録の形（`c02-window` の `kind = clock-skew` に欄を足す）

`WindowPayload` に `skip_serializing_if = "Option::is_none"` の欄を足す（他の種類の形は変わらない。`payload_shape_is_pinned` の他の種類は緑のまま）:

| 欄 | 型 |
|---|---|
| `clock_trigger` | `hourly` / `start` / `jump` / `retry` |
| `clock_available` | bool |
| `uptime_ms` | integer（D9） |
| `boot_at` | 既存の欄を clock-skew でも使う（D9） |
| `clock_references` | 配列。`{source, time?, skew_ms?, os_offset_ms?, mono_before_ms, mono_after_ms, host?, raw?, last_sync?, sync_source?}` |
| `clock_unavailable` | 配列。`{source, reason, raw?}`（`unparsed` のときは原文を持つ） |

- **D10（仮）: `raw` は JSON の文字列なので、cp932 のバイト列は ASCII 以外のバイトと `\` を `\xNN` にして持つ**（元のバイト列へ戻せる。`from_utf8_lossy` は原文を壊す）。
  `last_sync` は表示のまま（ロケール・ローカル時刻）で、正規化しない。反転条件: ST07 が時刻として読む必要が出たら、`raw` から引き直す（原文があるので計算し直せば戻る）。
- **`skew_ms` / `skew_reference` は残す**。`s01-date` が取れたときだけ、その差と `host` を入れる（ST07 の読む側を壊さない）。
  取れなかった記録では省く（いまは必ずあった —— `docs/collector-contract.md` の表に「取れなかった記録では無い」と書く）
- 測り直しは今の `SKEW_RETRY_SEC`（60 秒）のまま。取れなかった記録を 1 時間の契機ごとに 1 件までにするのは D4 と同じ形の印（`SkewSchedule` に持たせる）
- 出来事時刻は PC の時計（いまと同じ）。**差に使う壁時計（D7）とは別**に、測定を積むときの壁時計

### D11. 失敗の隔離

- 端末: 測定の 1 回を `runCatching` で包み、失敗は `Telemetry.line("clock_skew_crashed", error = 型名)` だけを出す（生存信号の起動時の 1 発と同じ規律）。
  刻みの例外は `ExecutorFlushScheduler` が構造で握る（ST04 design D26）
- PC: 基準の読み取りは作業スレッド（D7）。作業スレッドが panic しても通り道が閉じたことを「取れなかった（`reason: worker_failed`）」として扱い、見回りは止まらない

### D12. PC の測定記録は `c02-window` に残す。稼働状況の日の状態が ① に塗られる件は ST14 へ申し送る（仮）

spec レビュー R6: `coverage.rs` の `decide()` は `event_count > 0` を先に見るので、`c02-window` に毎時入る clock-skew の記録が、
PC が動いていて窓が読めなかった日（③）や記録の無い日（②）を ① 記録あり に塗る。**これは ST07 から既にある性質**
（ST07 も取り込み口に届く日は毎時 1 件入れていた。同じ機械の構成ではいつも届く）で、ST05 は取れなかった契機の 1 件で「届かない日」にも広げる。

- **`c02-window` に残す理由**: 過去の測定記録は `c02-window` にある。新しい論理ソース（`c02-clock`）へ移すと、ずれの履歴を読む側が 2 つのソースを継ぐことになり、
  PC の送信の口がソースを 2 本持つ形は並走する ST08 が作っている最中（`c02-browser-history`）で、同じ形を 2 本が同時に作ることになる
- 稼働状況の数え方は `collection-coverage`（ST12 が走行中）の領分なので、ST05 は触らない。**`docs/handoff/ST14.md` に申し送る**
  （ST06 の R5 と同じ型: 「取れなかった」と主張する 1 件が、その日を ① に塗る）。`c02-window` は `USAGE_SUBJECT` なので NFR-13 の達成日の数えは変わらない
- 反転条件: ST14 が「`kind = clock-skew` を日の状態の数えから除く」形を取れないと判断したら、PC の測定記録を `c02-clock` へ移す（新しい記録だけ。登録簿に 1 行）

## Risks / Trade-offs

- [ST06 と同じファイルを触る] ST06 の下流（PR #17）の差分は `collector-android/app/src/main` の 24 ファイルで、
  ST05 が触る `LocationService.kt`（+628 行。「収集の親」に作り替える）・`FixCollector.kt`・`Sender.kt`・`AgeClock.kt`・`IngestRequest.kt` を含む
  （2026-09-29 に `git diff --stat main...origin/feat/st06-app-usage` で確かめた）。`LocationFix.kt` と `HttpTransport.kt` は触っていない
  → ST05 は測定を別ファイル（`ClockSkew*.kt` / `ClockReferences.kt` / `ResponseDateCache.kt`）に置き、共有ファイルの差分を
  「刻みと受け手の組み込み」「`FixCollector` に `DeviceClock` を渡す」「`Sender` が応答を `ResponseDateCache` に置く 1 行」に絞る。
  `DeviceClock` の口（`AgeClock.kt`）は読むだけで変えない。後から merge する側が rebase で追従する（merge の順序は決めない）
- [ST08 と同じファイルを触る] ST08 の下流（PR #14）の差分は `crates/collector-windows` の `contract.rs`（+105 行）・`heartbeat.rs`・`telemetry.rs`・`exclusion.rs` などで、
  `runtime.rs` と `main.rs` は触っていない（2026-09-29 に `git diff --stat main...origin/feat/st08-browser-history` で確かめた）。
  **ST05 も `contract.rs` の `WindowPayload` と `payload_shape_is_pinned` を触る** —— ST08 はこの試験を「最初から最後まで緑」の検証にしている
  （`openspec/changes/st08-browser-history/tasks.md` の Global Constraints）→ ST05 は clock-skew の期待値だけを直し、他の種類の期待値は 1 文字も変えない
- [ネットワーク時刻が API 33 未満で取れない] 本人の端末の API は未確認 → `unsupported` として毎回残るので、後から「取れなかったのは OS の版のせい」と分かる
- [衛星の時刻は最後の測位から外挿した値] `currentGnssTimeClock` は最後の衛星の測位の時刻に経過を足したもの → 読む前後の経過時間を並べるので、読む側で重み付けできる。値そのものは補正しない
- [`Date` 見出しは秒の分解能] → 差が 0〜+999 ms 大きく出ることを spec の Scenario の幅と契約文書に書く
- [時計の変更の通知の取りこぼし] 通知が届かない端末では変更直後に測らない → 1 時間以内には必ず測るので、ずれは 1 時間遅れで残る。D3 の反転条件
- [PC の `w32tm` の出力の言語] → 原文を持つので、解析を後から直せば過去の記録も読み直せる
- [PC の作業スレッドが打ち切られずに残る] `/healthz` は `sender::agent()` の打ち切り、`w32tm` と（止まっているときの）`wevtutil` はそれぞれ 5 秒の打ち切り（最大 2 本で 10 秒）を持つので、作業スレッドは有限の時間で終わる

## Migration Plan

1. 移行 `YYYYMMDDHHMM_clock_source.sql` を `crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す（起動時に当たる）。
   **端末の新しい版より先にサーバへ出す** —— 逆だと `c01-clock` が `unknown_source` で断られ、端末の未送信に残る（捨てられはしない。受け手側の設定で変わる理由なので未送信に残す規則。ST03）
2. 端末と PC の新しい版を入れる。戻すときは古い版を入れるだけ（測定記録が増えなくなる。入った記録はそのまま）
3. 移行を戻す（`.down.sql`）のは `c01-clock` の記録が 1 件も無いときだけ

## Open Questions

- 本人の端末の API（33 以上か）—— ネットワーク時刻が取れるかが決まる。実装は API によらず同じなので後で分かってよい
