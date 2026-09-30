# ST05 実装タスク —— 端末時計のずれを測って残す

読む順: `deep.md`（**最優先。本人が決めた 3 件と、聞かずに決めた C1〜C8**）→ このファイル →
`specs/device-collection/spec.md` → `specs/desktop-collection/spec.md` → `design.md` → `docs/stories/ST05.md` →
`docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D1）。**名前は作成時刻 `YYYYMMDDHHMM_clock_source.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。**取り込みのコード（`ingest.rs`）と稼働状況の数え方（`coverage.rs`）は変えない。**

検証は各項目の本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する（`scripts/verify-run <項目>` が走らせる）。
DB を使う検査は `docker compose up -d db` が前提。Android の計測テストは `tools/android-emulator.sh`。
Windows の実行時テストは WSL から Windows 側の cargo で回す（`C:\dev\ashiato2-rt` へ同期して `cmd.exe` をフルパスで起動。CI の `collector-windows-runtime` と同じテスト）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Kotlin と Rust はコメント（`// Scenario: 1 時間ごとに測定記録が 1 件残る`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。
  印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **印を置くテストは、その Scenario の THEN を確かめるものにする。** THEN が「格納される」なら取り込み口まで通す（DB を使うテスト）
- **この change の Scenario は 66 本**（`device-collection` 41 本 / `desktop-collection` 25 本）。
  **既に印があるのは 3 本**（`契機ごとに 1 件生成される` / `端末識別子が端末をまたいで一意である` / `1 時間ごとにずれの測定記録が残る`）。
  1 本目は位置の記録の欄が増えるので、3 本目は PC の測定の形が変わるので、既存のテスト（`FixCollectorTest` / `clock.rs` の `clock_skew_is_measured`）を**直す**（印を移すだけにしない）
- **「人間の確認待ち」に逃がせる Scenario は 1 本も無い。** 基準（ネットワーク時刻・衛星の時刻・応答の日付・Windows の時刻同期の状態）と
  時計はすべて口の後ろに置き、試験は偽物に差し替える（`DeviceClock` / `FixSource` と同じ形）
- **Q1〜Q3 は本人の決定。下流は変えない** —— 外部の時刻サーバに問い合わせない / 位置の記録に 3 項目を足す / PC の測定を揃え Windows の時刻同期の状態を並べる
- **D1 / D3 / D4 / D6（`schema_version`）/ D8 / D12 は（仮）決め。** 反転条件は `design.md` にある。
  **D8 の反転条件の最後（どの口でも Windows の同期の状態が読めない）は本人に返すもの** ——
  そこに当たったら実装で決めずに `deep.md` へ R 番号つき `(未回答)` で書いて止まる（Q3 の ③ の前提が崩れる）
- **測定のために通信を起こさない**（端末。FR-7 / Q1）。端末の測定の口（`ClockSkew*.kt` / `ClockReferences.kt` / `ResponseDateCache.kt`）は
  `Transport` も `java.net` も使わない。PC の測定の要求は取り込み口の `/healthz` への 1 本だけ（design D7）。**時刻サーバへの経路は端末にも PC にも作らない**
- **記録の時刻を補正しない**（C4）。位置の記録の `event_time` は `Location.getTime()` のまま、PC の前景の記録の時刻も観測したときの PC の時計のまま
- **ログに位置の値・時刻の値・差・原文・ウィンドウ題名を出さない。** 出すのは件数・ソース名・所要時間・エラーの種別・`available` だけ（製造準備 A-2）
- **契約の形のうち既存の欄を変えない。** `payload_shape_is_pinned`（PC）は clock-skew 以外の種類について期待値を 1 文字も変えない（並走する ST08 も同じ試験を「最初から最後まで緑」にしている）。
  足した欄は `docs/collector-contract.md` に書く
- **件数つき検証**: `cargo test` は一致するテストが 0 本でも rc=0 になるので、このファイルの cargo の検証は
  `bash -o pipefail -c 'cargo test … 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. (<下限>) passed' /tmp/ct.log` の形で書く。
  Gradle の `--tests` はフィルタ全体で一致が 0 本なら失敗するので、**新しい試験だけに一致するクラス名を 1 つだけ**渡す
- **重なる Story**: ST06（`LocationService` / `FixCollector` / `Sender` / `AgeClock` を触っている）/ ST08（`contract.rs` を触っている）。
  測定は別ファイルに置き、共有ファイルの差分を design の Risks に書いた範囲に絞る。**走っている Story へ差し戻さない**

## Task 1: 前提を確かめる（エミュレータと Windows。設計を覆しうるものを先に）

- [x] 1.1 融合プロバイダの `Location.getTime()` がどの時計から来るかをエミュレータで確かめる
  （端末の時計を 5 分ずらし、`getTime()` と `System.currentTimeMillis()` と `getElapsedRealtimeNanos()` を並べる。design D6）。
  結果（どちらだったか・確かめた手順・日付）を `docs/collector-contract.md` の位置の節に見出し「`Location.getTime()` の出どころ」で書く。
  **結果がどちらでも spec と以降の Task は変わらない。** 時計をずらせなかったらそう書く（推測で埋めない）。
  検証: `bash -c 'grep -q "Location.getTime() の出どころ" docs/collector-contract.md'` rc=0
- [x] 1.2 エミュレータで、(a) 端末の時計を変えられるか（`UiAutomation.executeShellCommand` で `cmd alarm set-time` など）、
  (b) 変えたときに `Intent.ACTION_TIME_CHANGED` がサービスの動的な受け手に届くか、を確かめる。
  結果（試したコマンド・届いたか・日付）を `design.md` の D3 の末尾に「ACTION_TIME_CHANGED の確かめ」として追記する。
  届かなければ D3 の反転条件（1 分ごとの見回りで 60 秒以上の食い違いを見る）へ倒して Task 6 を作る。
  検証: `bash -c 'grep -q "ACTION_TIME_CHANGED の確かめ" openspec/changes/st05-clock-skew/design.md'` rc=0
- [x] 1.3 `w32tm /query /status /verbose` が**管理者権限なしで**読めるか、出力の見出しが何語かを、手元の Windows と `windows-latest` の実行時テストで確かめる（design D8）。
  `crates/collector-windows/tests/runtime_windows.rs` に、本物の `TimeSyncSource` で読み、**終了コード 0 で、最後に同期した時刻か同期元のどちらかが解析できる**ことを
  assert する実行時テスト `clock_time_sync_is_readable` を置く（読めなければ落ちる）。結果（読めたか・エラー符号・表示言語・見出し）を
  `design.md` の D8 の末尾に「w32tm の確かめ」として追記する。読めなければ D8 の反転条件（イベントログ）を確かめ、
  **それでも読めなければ `deep.md` に R 番号つき `(未回答)` で書いて止まる**。
  検証: `bash -c 'rsync -a --delete --exclude target --exclude .git ./ /mnt/c/dev/ashiato2-rt/ && /mnt/c/Windows/System32/cmd.exe /c "cd /d C:\dev\ashiato2-rt && set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p ashiato-collector-windows --test runtime_windows clock_time_sync_is_readable -- --test-threads=1"'` rc=0 /
  `bash -c 'grep -q "w32tm の確かめ" openspec/changes/st05-clock-skew/design.md'` rc=0

## Task 2: 登録簿に端末の時計のソースを足す（サーバ）

- [x] 2.1 移行 `migrations/YYYYMMDDHHMM_clock_source.sql` と `.down.sql` を足す —— `core.source` に
  `('c01-clock', '携帯端末の時計のずれ', 21600, 'none')`（`ON CONFLICT (logical_source) DO NOTHING`）。
  `.down.sql` は `c01-clock` の記録が 1 件も無いときだけ行を消す。`MIGRATIONS` 配列の末尾に足す（design D1 / Migration Plan）。
  試験 `clock_source_migration_applies_twice`（全版を 2 回当てて落ちず、行が 1 行で `external_id_kind = 'none'`）を置く。
  検証: `tools/check-migrations.sh` rc=0 /
  `bash -o pipefail -c 'cargo test -p ashiato-server clock_source_migration 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`
- [x] 2.2 `/ingest` に識別子を持たない `c01-clock` の記録を送って格納される試験と、`c01-clock` の記録だけがある日が
  稼働状況の端末が主語の達成日に数えられない試験（`coverage.rs` の数えを通す）を、名前 `clock_record_ingest_*` で置く。
  Scenario: `識別子を持たない測定記録が格納される` / `測定記録だけの日は端末が主語の達成日にならない`。
  検証: `bash -o pipefail -c 'cargo test -p ashiato-server clock_record_ingest_ 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. ([2-9]|[1-9][0-9]+) passed' /tmp/ct.log`（2 本以上）

## Task 3: 位置の記録に 3 項目を足す（Android）

- [x] 3.1 `LocationFix` に `receivedDeviceTime` / `fixElapsedNs` / `receivedElapsedMs` / `bootCount` を足し、原文と解析済みの両方に
  `received_device_time` / `fix_elapsed_ns` / `received_elapsed_ms` / `boot_count` として載せる（design D6）。
  `FixCollector` は `DeviceClock` を受け取り、受け取ったときの値を入れる（`bootCount()` が `null` なら `null` を載せる）。
  **`event_time` と `device_time` は `Location.getTime()` のまま。** 既存の `FixCollectorTest` の「契機ごとに 1 件」を新しい欄に合わせて直す。
  新しい試験は `LocationFixClockFieldsTest` に置く（同じ `LocationFix` から原文を 2 回組み立てて文字列が一致する試験を含む）。
  Scenario: `位置の記録が受け取ったときの端末の時計の時刻を持つ` / `位置の記録が測位の起動からの経過時間と起動の識別を持つ` /
  `位置の記録の出来事時刻は測位の結果が持つ時刻のまま` / `契機ごとに 1 件生成される`。
  検証: `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*LocationFixClockFieldsTest*'` rc=0 /
  `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*FixCollectorTest*'` rc=0
- [x] 3.2 `docs/collector-contract.md` の位置の節に 4 欄を足す（足す前の記録には無いこと・出来事時刻の意味は変わらないこと・`boot_count` は取れなければ `null`）。
  検証: `bash -c 'grep -q "received_device_time" docs/collector-contract.md && grep -q "fix_elapsed_ns" docs/collector-contract.md && grep -q "received_elapsed_ms" docs/collector-contract.md'` rc=0

## Task 4: 端末の基準を読む口（Android）

- [x] 4.1 `HttpTransport` の応答に `Date` 見出し（文字列そのまま）と、接続を開く直前・応答を読み終えた直後の `elapsedRealtime`、
  応答を読み終えた直後の壁時計を載せる（`Outcome.Responded` に既定値つきの欄を足す。既存の呼び出し元は変えない。design D2）。
  `Sender` が受け取った最後の 1 件を `ResponseDateCache` に置く（`/ingest` `/heartbeat` `/drops` のどれでも）。
  **取り出すと空になる。時計の変更の通知で空になり、そのことを理由（`clock_changed_since`）として返せる。**
  新しい試験は `ResponseDateCacheTest` に置く（`HttpTransport` の見出しの読み取りは `HttpTransportTest` と同じ形の偽のサーバで、同じクラスの中に置く）。
  Scenario: `前回の測定より前の応答の日付は使わない` / `時計の変更より前に受け取った応答の日付は使わない`。
  検証: `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*ResponseDateCacheTest*'` rc=0 /
  `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*SenderTest*'` rc=0（既存が緑のまま）
- [x] 4.2 `ClockReferences` を置く —— `network`（`currentNetworkTimeClock`。API 33 未満は `unsupported`、`DateTimeException` は `not_available`）/
  `gnss`（`currentGnssTimeClock`）/ `s01-date`（`ResponseDateCache` から取り出す）。
  各基準は出どころ・時刻・差・読む直前と直後の単調時計を返し、取れなければ理由を返す。**差に使う壁時計は、その基準を読む直前と直後の間で読む。**
  **例外を外へ出さない**（`error:<型名>`）。OS の時計の読み取りは口（`SystemTimeSources`）の後ろに置き、試験は偽物に差し替える。
  Scenario: `差に使う端末の時計は基準を読む前後の間で読む` / `OS の版が対応していない基準はいま取れない基準と区別して残る` /
  `測るために通信を起こさない`（偽の `Transport` を `Sender` に持たせたまま測り、その呼び出し回数が前後で変わらない）。
  検証: `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*ClockReferencesTest*'` rc=0 /
  `bash -c 'set -e; f="collector-android/app/src/main/kotlin/dev/ashiato/collector"; ls $f/ClockReferences.kt $f/ResponseDateCache.kt >/dev/null; ! grep -nE "java\.net|HttpURLConnection|Transport\b|Sender\b" $f/ClockReferences.kt $f/ResponseDateCache.kt $f/ClockSkew*.kt'` rc=0

## Task 5: 端末の測定記録を組み立てる（Android）

- [x] 5.1 `ClockSkewMeasurer` を置く —— 3 つの基準を読み、`c01-clock` の `IngestRequest` を 1 件組み立てる（形は design D5。
  `raw` は `payload` と同じ JSON を `ingestJson` で直列化した文字列。時刻はミリ秒まで・UTC・`Z`）。
  出来事時刻は測ったときの端末の壁時計。`elapsed_ms` と `boot_count` は `DeviceClock` から（取れなければ `null`）。
  3 つの出どころは `references` と `unavailable` のどちらかに 1 回ずつ。基準が 1 つも取れなければ `available: false`。
  Scenario: `端末の時計を 5 分進めると差が約 300000 ミリ秒で残る` / `端末の時計が遅れていると差が負で残る` /
  `応答の日付の差は秒の分解能の幅に収まる` / `測定記録に測った時刻が入っている` / `起動の識別と起動からの経過時間が入っている` /
  `起動の識別が取れない端末では取れないことが残る` / `取れる基準は全部並ぶ` / `3 つの出どころは取れたか取れなかったかのどちらかに 1 回ずつ出る` /
  `自宅 PC に届かない間も端末の中の基準で測る` / `届かない間の応答の日付は取れなかった基準に残る` / `測定記録は位置の記録と別のソースに入る` /
  `基準が 1 つも取れないと取れなかった印の付いた記録が 1 件残る` / `取れなかった記録は基準ごとの理由を持つ` /
  `取れなかった記録にも端末の時計と起動の識別が入る`。
  「どの組み合わせでも」は 3 つの基準の取れる / 取れないの 8 通りを総当たりで回す。
  検証: `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*ClockSkewMeasurerTest*'` rc=0
- [x] 5.2 測定記録の形を固定する試験を置く（欄の名前と並び。同じ入力で原文の文字列が毎回一致する）。
  検証: `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*ClockSkewPayloadShapeTest*'` rc=0
- [x] 5.3 `docs/collector-contract.md` に `c01-clock` の節を足す（欄・出どころの 3 種・理由の値・差の符号と、差に使う壁時計を読む時点・`s01-date` の 0〜+999 ms の偏り・生存信号を送らないこと）。
  検証: `bash -c 'grep -q "c01-clock" docs/collector-contract.md && grep -q "no_response_since_last" docs/collector-contract.md && grep -q "clock_changed_since" docs/collector-contract.md'` rc=0

## Task 6: 端末の測る契機と測り直し（Android）

- [x] 6.1 `ClockSkewScheduler` を置く —— 1 時間・起動時・時計の変更の契機で測り、取れなかったら 1 件積んで 5 分ごとに測り直す。
  測り直しで取れなければ積まない。取れたら `trigger: retry` で 1 件積んで測り直しをやめる。次の 1 時間の契機で測り直しの印を下ろす（design D3 / D4。1.2 の結果で方式が変わったらそれに従う）。
  測定のログは件数・種別・`available` だけ。時計は偽物（`DeviceClock`）と偽の刻みで進める。
  Scenario: `1 時間ごとに測定記録が 1 件残る` / `測った契機が記録に残る` / `収集の起動時にその場で測る` / `端末の時計が変更されるとその場で測る` /
  `測り直しのたびには記録を増やさない` / `測り直しで取れたら別の 1 件が残る` / `圏外が 1 日続くと取れなかった記録は 24 件` /
  `測定のログに時刻の値と差が出ない`。
  検証: `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*ClockSkewSchedulerTest*'` rc=0
- [x] 6.2 `LocationService` に組み込む —— `newClockScheduler()`（試験だけが差し替える）で 1 時間と測り直しの刻みを立て、`onStartCommand` で 1 回測り、
  `ACTION_TIME_CHANGED` の受け手を `onCreate` で登録・`onDestroy` で解除する（受けたら `ResponseDateCache` を空にしてから測る）。
  測定記録は**記録の未送信（`outbox`）**に積む（保持の上限と送信に乗る）。`c01-clock` の生存信号は出さない。
  測定の 1 回を `runCatching` で包み、失敗は `clock_skew_crashed` と型名だけをログに出す（design D11）。**`LocationService` の差分は組み込みだけにする**（ST06 と重なる）。
  試験は `LocationServiceClockTest` に置く（Robolectric。既存の `LocationServiceTest` と同じ差し替え口を使う）。
  Scenario: `測定記録の論理ソースは生存信号を送らない` / `到達できない間の測定記録は後から届く` / `測定記録も保持の上限で捨てられ破棄として報告される` /
  `位置の記録の時刻は補正されない` / `測定が例外で落ちても位置の記録は生成される` / `測定の失敗は種別だけがログに残る`。
  検証: `cd collector-android && ./gradlew :app:testDebugUnitTest --tests '*LocationServiceClockTest*'` rc=0 /
  `cd collector-android && ./gradlew :app:testDebugUnitTest` rc=0（既存の単体が全部緑のまま）

## Task 7: 端末の計測テスト（エミュレータ）

- [x] 7.1 本物の `LocationService` を起動し、起動時の測定記録が `c01-clock` として記録の未送信に積まれ、3 つの出どころが `references` と `unavailable` に 1 回ずつ出ることを確かめる
  計測テスト `ClockSkewInstrumentedTest` を置く。1.2 で時計を変えられたなら、変えた直後に `trigger: time_set` の記録が 1 件増えることも確かめる。
  Scenario: `収集の起動時にその場で測る` / `端末の時計が変更されるとその場で測る`（1.2 で変えられなかったときは、6.1 の印だけが担保する）。
  検証: `tools/android-emulator.sh` rc=0（計測テストを全部走らせる）/
  `bash -c 'grep -q "Scenario: 収集の起動時にその場で測る" collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/ClockSkewInstrumentedTest.kt'` rc=0

## Task 8: PC の基準を読む口と、輪の外で読む作業スレッド（Windows）

- [x] 8.1 `TimeSyncSource` の口と本番の実装（`w32tm /query /status /verbose` の子プロセス。5 秒で打ち切る）と偽物を置く。
  出力は原文のまま持ち、英語と日本語の見出しから最後に正常に同期した時刻・同期元・位相のずれを解析する。解析できなければ原文を持ったまま `unparsed`、
  起動しない・非 0・打ち切りは理由で返す。**子プロセスに渡す引数は照会だけに固定し、試験で固定する**（1.3 の結果で反転したらその口で同じことをする）。
  試験の名前は `time_sync_*`（英語の出力・日本語の出力・解析できない出力・打ち切り・引数の固定の 5 通り以上）。
  Scenario: `Windows の時刻同期の状態が入っている` / `OS の見積もったずれが読めたときは並ぶ` / `Windows の時刻同期の状態は読んだままの出力が残る` /
  `項目を読み取れなかった出力も残る` / `PC は Windows に時刻を同期させない`。
  検証: `bash -o pipefail -c 'cargo test -p ashiato-collector-windows time_sync_ 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. ([5-9]|[1-9][0-9]+) passed' /tmp/ct.log`（5 本以上）
- [x] 8.2 `ReferenceClock::now()` の戻り値を「基準の時刻・読む直前と直後の起動からの経過時間・受け取った直後の壁時計」の組にし、差をその壁時計で計算する
  （見回りの先頭の `wall` を差に使わない。Q3 ②。design D7）。`Uptime` の口（`GetTickCount64`。非 Windows は偽物）を置く（design D9）。
  口の形が変わるので、`crates/collector-windows/tests/runtime_windows.rs` の `NoReference` と `main.rs` の組み立ても合わせる。
  試験の名前は `clock_reference_*`。
  Scenario: `PC の時計が進んでいると差が正で残る` / `PC の時計が遅れていると差が負で残る` / `差に使う PC の時計は基準を読む前後の間で読む` /
  `測るための要求は取り込み口の生存確認の 1 本だけ`（偽の取り込み口が受けた要求を数える）。
  検証: `bash -o pipefail -c 'cargo test -p ashiato-collector-windows clock_reference_ 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. ([4-9]|[1-9][0-9]+) passed' /tmp/ct.log`（4 本以上）
- [x] 8.3 基準の読み取り（`/healthz` と `TimeSyncSource`）を 1 本の作業スレッドで行い、結果を通り道で見回りへ返す（design D7 / D11）。
  作業スレッドが走っている間は次の測定を始めない。作業スレッドが panic したら `worker_failed` として扱う。試験の名前は `clock_worker_*`。
  Scenario: `基準の読み取りが長引いても前景の切り替えは記録に残る`（読み取りを 5 秒止める偽物の間に 1 秒ごとの見回りで切り替えを起こし、その記録が残る）/
  `測定が失敗し続けても送信は続く`。
  検証: `bash -o pipefail -c 'cargo test -p ashiato-collector-windows clock_worker_ 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. ([2-9]|[1-9][0-9]+) passed' /tmp/ct.log`（2 本以上）/
  `cargo clippy -p ashiato-collector-windows --all-targets -- -D warnings` rc=0

## Task 9: PC の測定記録を揃える（Windows）

- [x] 9.1 `WindowPayload` に `clock_trigger` / `clock_available` / `uptime_ms` / `clock_references` / `clock_unavailable` を足し（`skip_serializing_if`）、
  clock-skew でも `boot_at` を載せる。`skew_ms` / `skew_reference` は `s01-date` が取れたときだけ入れる（design D10）。
  2 つの出どころは `clock_references` と `clock_unavailable` のどちらかに 1 回ずつ。
  `payload_shape_is_pinned` の clock-skew の期待値だけを直す（他の種類の期待値は 1 文字も変えない）。新しい試験の名前は `clock_skew_payload_*`。
  Scenario: `PC の測定記録に起動の識別と起動からの経過時間が入っている` / `基準ごとに読む直前と直後の経過時間が入っている` /
  `同じ機械の構成でも Windows の時刻同期の状態が並ぶ` / `2 つの出どころは取れたか取れなかったかのどちらかに 1 回ずつ出る`。
  検証: `bash -o pipefail -c 'cargo test -p ashiato-collector-windows payload_shape_is_pinned 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log` /
  `bash -o pipefail -c 'cargo test -p ashiato-collector-windows clock_skew_payload_ 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. ([4-9]|[1-9][0-9]+) passed' /tmp/ct.log`（4 本以上）
- [x] 9.2 取れなかった契機を 1 件残し、測り直し（60 秒）のたびには増やさず、取れたら `retry` で 1 件残す。起動時・壁時計の飛び・戻りの契機を `clock_trigger` に残す。
  既存の `clock_skew_is_measured` を新しい形に直す。新しい試験の名前は `clock_skew_record_*`。
  Scenario: `1 時間ごとにずれの測定記録が残る` / `壁時計が飛ぶとその場で測る` / `時計が飛んだ直後の測定は飛んだ後の差を持つ` / `PC の測定記録に測った契機が残る` /
  `PC で基準が 1 つも取れないと取れなかった印の付いた記録が残る` / `PC の取れなかった記録は基準ごとの理由を持つ` /
  `PC の測り直しのたびには記録を増やさない` / `PC の測り直しで取れたら別の 1 件が残る` / `PC の記録の時刻は補正されない`。
  検証: `bash -o pipefail -c 'cargo test -p ashiato-collector-windows clock_skew_record_ 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. ([8-9]|[1-9][0-9]+) passed' /tmp/ct.log`（8 本以上）/
  `bash -o pipefail -c 'cargo test -p ashiato-collector-windows clock_skew_is_measured 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`
- [x] 9.3 PC の実行時テストに、本物の `TimeSyncSource` と `Uptime` と `boot_time` で 1 回測り、clock-skew の記録の `clock_references` に `windows-time-sync` があり、
  `uptime_ms` と `boot_at` を持つことを足す（`clock_skew_runtime`）。★ 2026-09-30 本人の指示で訂正: W32Time は止まっていることがあるので、動いていれば `clock_references` の `windows-time-sync` を、止まっていれば `clock_unavailable` の `reason: service_stopped` を見る（design D8）。
  `docs/collector-contract.md` の C-02 の表に 9.1 の欄を足し、`skew_ms` / `skew_reference` が「`s01-date` が取れなかった記録では無い」と書く。
  検証: `bash -c 'rsync -a --delete --exclude target --exclude .git ./ /mnt/c/dev/ashiato2-rt/ && /mnt/c/Windows/System32/cmd.exe /c "cd /d C:\dev\ashiato2-rt && set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p ashiato-collector-windows --test runtime_windows clock_skew_runtime -- --test-threads=1"'` rc=0 /
  `bash -c 'grep -q "clock_references" docs/collector-contract.md && grep -q "clock_unavailable" docs/collector-contract.md'` rc=0

## Task 10: 通しの検査

- [x] 10.1 時刻サーバへの経路が無いことの静的検査 `tools/check-no-time-server.sh` を置く —— `collector-android/app/src/main` と `crates/collector-windows/src` を見て、
  時刻のプロトコルの送信（`DatagramSocket` / `SntpClient` / `NtpTrustedTime` / `UdpSocket`）・時刻サーバの宛先（`ntp.org` / `time.google.com` / `time.windows.com` / `time.apple.com` / `:123`）・
  同期を起こす指示（`/resync`）が 1 つでもあれば rc=1。`--self-test` で、一時ディレクトリに経路を 1 つ植えたときに rc=1 になることを自分で確かめる。
  `echo "Scenario: 外部の時刻サーバへ問い合わせない"` と `echo "Scenario: PC は外部の時刻サーバへ問い合わせない"` を出す。CI（`.github/workflows/ci.yml`）の静的検査の段に足す。
  検証: `tools/check-no-time-server.sh` rc=0 / `tools/check-no-time-server.sh --self-test` rc=0 /
  `bash -c 'grep -q "check-no-time-server.sh" .github/workflows/ci.yml'` rc=0
- [x] 10.2 実物の取り込み口に通す —— `tools/smoke.sh` に、`c01-clock` の測定記録（取れた記録と取れなかった記録の 2 件）を `/ingest` へ送って
  格納されることと、PC の新しい形の clock-skew が格納されることを足す（`echo "== Scenario: 識別子を持たない測定記録が格納される"`）。
  検証: `tools/smoke.sh` rc=0（`docker compose up -d db` の後）
- [x] 10.3 確かめる係を置く（design D4 の反転条件）—— `tools/verify-prep.sh` の手順書の組み立てに、`c01-clock` の `trigger = retry` と `trigger = hourly` の
  直近 7 日の件数を出す SQL を足し、手順書に数を載せる（人間には聞かない。数を残すだけ）。
  検証: `bash -c 'grep -q "c01-clock" tools/verify-prep.sh && grep -q "retry" tools/verify-prep.sh'` rc=0
- [x] 10.4 Scenario と test の突き合わせ・全体のテスト・静的検査を通す。
  検証: `python3 scripts/check_scenarios.py . st05-clock-skew` rc=0 /
  `bash -o pipefail -c 'cargo test --workspace 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log` /
  `cd collector-android && ./gradlew :app:assembleDebug :app:testDebugUnitTest` rc=0 /
  `tools/check-private.sh` rc=0 / `tools/check-panic-log.sh` rc=0
