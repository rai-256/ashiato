# ST05 code レビュー —— st05-clock-skew

## final review（308dd45..d499da1）

- 席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package `.superpowers/sdd/tasks/review-308dd45..d499da1.diff`
- 入力: 全 Task の `task-*-findings.md` の Minor と ⚠️、`st05-task-10/progress.md` の Ruling
- 判定: With fixes（Critical なし / Important 5 / Minor 13）
- 日付: 2026-09-30
- fix: 1 回（dc3fb28。R1〜R4・R6〜R8・R14・R15）→ scoped re-review 1 回（a2e5539..dc3fb28）: 9 件とも ADDRESSED、新たな Critical / Important なし（Minor: check-no-time-server.sh の `mod` 検出は rustfmt の形に依存）
- 処置の無いもの（R5・R9〜R13・R16〜R18）は finish で付けた（下の code-verify の R19〜R24 と合わせて fix 1 回 → scoped re-review 1 回）

## R1. `HttpTransport` がコンストラクタで `URL()` の例外を投げ、`BASE_URL` が不正だとサービスの起動が落ち続ける
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HttpTransport.kt
- 根拠: `HttpTransport.kt:30` の `hostPort` がプロパティ初期化で `URL(this.baseUrl)`。`Config.isComplete` は空でないことしか見ず、`startFlushing()` の `newTransport()` で `MalformedURLException` → `onStartCommand` が落ち `START_STICKY` で繰り返す。以前は `post()` 内で `Unreachable` になり未送信に積まれていた（Task 5 F6）
- kind: technical
- 処置: fixed 4.1 —— dc3fb28: `hostPort` を `by lazy` + `runCatching` にし、不正な送り先は `post()` の中で `Unreachable` に畳む。試験 `ResponseDateCacheTest`「不正な送り先でも組み立ては投げず、送ると到達できないに畳まれる」

## R2. 応答の `Date` を置くのが `/ingest` の Sender だけで、design D2 の「`/ingest` `/heartbeat` `/drops` のどれでも」と違う
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `LocationService.kt:367-387` —— `responseDates` を `sender` にだけ渡し、`beatSender` / `dropSender` には渡していない（Task 4 ⚠️）
- kind: technical
- 処置: fixed D2 —— dc3fb28: `beatSender` / `dropSender` にも同じ `responseDates` を渡す。試験 `LocationServiceClockTest` 2 本（/heartbeat・/drops の `Date` を次の測定が使う）

## R3. `ClockSkewScheduler` が `stop()` の後・測り直しを畳んだ後にも測る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ClockSkewScheduler.kt
- 根拠: `ClockSkewScheduler.kt:52-69` —— `shutdownNow()` は `synchronized(lock)` を待つ糸を止めない。(a) hourly が取れて retry を畳んだ直後に待っていた retry が余分な `retry` 記録を積み、D4 の反転条件の比（`2*retry > hourly`）を汚す。(b) `stop()` と同時に待っていた hourly が取れないと新しい retry の executor を立て、`onDestroy` の後も 5 分ごとに回る（Task 6 F1）
- kind: technical
- 処置: fixed 6.1 —— dc3fb28: `stopped` の印を持ち、`measureLocked` の先頭で `stopped` / 畳んだ後の `retry` を捨てる。試験 `ClockSkewSchedulerTest` 2 本

## R4. `clock_skew_runtime` のループが「未送信 0 件」で抜けるので、worker の結果より先に別の記録が積まれると `skew.len() == 1` が落ちうる。W32Time が動いている経路は一度も走っていない
- 成果物: crates/collector-windows/tests/runtime_windows.rs
- 根拠: `runtime_windows.rs:680` の終了条件 `rt.pending().0 == 0`。evidence の 9.3 の PASS は W32Time が止まっている経路のもの（Task 9 F1 / ⚠️）。このブランチの CI は PR 前でまだ走っていない
- kind: technical
- 処置: fixed 9.3 —— dc3fb28: ループの終了を「`outbox.jsonl` か Capture の /ingest に `clock-skew` が現れる（か 20 秒）」に。Windows 側 cargo で `clock_skew_runtime` rc=0（W32Time 停止側）。動いている経路は PR の `collector-windows-runtime` で見る

## R5. `tasks.md` の 9.3 の本文が commit 51cebee で書き換わっている（「★ 2026-09-30 本人の指示で訂正」）
- 成果物: openspec/changes/st05-clock-skew/tasks.md
- 根拠: `git show 51cebee -- openspec/changes/st05-clock-skew/tasks.md`。evidence の executor は manual。CLAUDE.md は implementer が tasks.md を触らないと定める（Task 9 ⚠️）
- kind: technical
- 処置: rejected: 実装者の書き換えではない —— `git log -1 --format=%B 51cebee` は人間（ライ）の指示のセッションの commit で本文に「本人の指示。design D8 と tasks 9.3 に記録」、`tasks.md:194` の ★ と `design.md:208`「W32Time が止まっているとき（2026-09-30。本人の指示）」が同じ日付で揃う。CLAUDE.md の禁止は implementer が `[x]` と本文を触ることで、本人の訂正はその外。訂正の中身（止まっている間に何も残らない）は R22 で本人に問うた

## R6. `s01-date` の壁時計を `monoAfter` の後に読んでおり、Scenario「差に使う端末の時計は基準を読む前後の間で読む」に字面で反する
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HttpTransport.kt
- 根拠: `HttpTransport.kt:51` —— `wallClock()` が `monoAfter` の後（Task 4 F4）。差は µs 規模
- kind: technical
- 処置: fixed 4.1 —— dc3fb28: 壁時計を `monoAfter` の前に読む。試験 `ResponseDateCacheTest`「応答の壁時計は前後の単調時計の間で読む」（`[mono, wall, mono]`）

## R7. `clock_skew_crashed` のログの source が `c01-location` になる（測定のログは `c01-clock`）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `LocationService.kt:239` —— `Telemetry.line` の既定の source。試験もそれで固定している
- kind: technical
- 処置: fixed 6.2 —— dc3fb28: `source=c01-clock` で出す。試験の期待を直した

## R8. 移行の `.down.sql` の guard が `core.event` しか見ず、`core.drop_report` に c01-clock の行があると FK エラーで落ちる
- 成果物: migrations/202609291230_clock_source.down.sql
- 根拠: spec「測定記録も…破棄として報告される」で `core.drop_report` に c01-clock が入りうる。騒がしく落ちるので失われるものは無い（Task 2 F4）
- kind: technical
- 処置: fixed 2.1 —— dc3fb28: guard を `core.event` / `drop_report` / `coverage` / `coverage_span` / `heartbeat` に広げ、参照があれば NOTICE で残す。試験 `clock_source_down_keeps_row_referenced_by_drop_report`（旧 down.sql で FK エラーの FAIL を確認）

## R9. PC で飛び（`reset(Jump)`）の後に飛ぶ前に始めた読み取りが失敗で返ると、`failed(mono)` が `last` を上書きし jump の測定が 60 秒遅れる
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: `runtime.rs:469-499`（Task 9 F5。以前からある性質）
- kind: technical
- 処置: fixed 9.3 —— `SkewSchedule` に `superseded` の印を持ち、`reset` の後の `failed` は `last` を上書きしない（`clock.rs` の `failed`）。試験 `skew_failure_of_a_read_started_before_the_jump_does_not_delay_the_jump`（印を外すと落ちることを確認）

## R10. PC の取れなかったときのログの error に `s01-date` の理由しか出ず、`unwrap_or("time_sync")` の枝に来ない
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: `runtime.rs:491`（Task 9 F3）
- kind: technical
- 処置: fixed 9.3 —— ログの error を `unavailable_error`（`runtime.rs`）で 2 つの基準の理由を `s01-date:<理由>,windows-time-sync:<理由>` と並べる。`unwrap_or("time_sync")` の枝は消えた。試験 `clock_skew_unavailable_log_names_both_reasons`

## R11. `clock_worker.rs` の `#[cfg(test)] settled` と `settle_clock_worker: true`（既定）で、Runtime の既存の試験はほぼ同期の経路しか通らない
- 成果物: crates/collector-windows/src/clock_worker.rs
- 根拠: Task 8 F3
- kind: technical
- 処置: rejected: 非同期の経路は試験が持っている —— `runtime.rs` の `clock_worker_slow_read_does_not_hold_the_patrol` が `settle_clock_worker = false` で読み取りの途中（`Pending`）を 8 秒回し、`clock_worker_panic_is_unavailable_and_retried_after_a_minute` / `clock_worker_failing_reads_do_not_stop_sending` が失敗の経路を回す。本番の経路（`#[cfg(test)]` の無い `ClockWorker`）は Windows の実行時テスト `clock_skew_runtime` が通す。`settle` は既存の試験の時刻の進め方を決定的にするだけで、`poll` の分岐を変えない（`clock_worker.rs:104-113`）。非同期で見落としていた型（R9）は `SkewSchedule` の段の試験で固定した

## R12. `LocationService` の `onReceive` と `onStartCommand` → `start()` が main スレッドで測定と `outbox.add`（ファイル I/O）を行う
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: Task 6 F3
- kind: technical
- 処置: rejected: 測定で main スレッドに新しく載る I/O ではない —— 位置の記録は `FixSource.kt:32` で `Looper.getMainLooper()` に届き、`FixCollector.kt:53` が 60 秒ごとに同じ `outbox.add` を main で行う（起動時の生存信号も `LocationService.kt:439` で main）。測定の `outbox.add` はその 1/60 の頻度で同じ置き場への 1 行の追記、基準の読み取りは通信を起こさない（`ClockReferences` は `SystemClock` と記憶した応答の日付だけ）。測定だけを別の糸へ移しても main の I/O は減らない

## R13. `w32tm` を PATH / exe の置き場所から解決している（`%SystemRoot%\System32\w32tm.exe` が固い）
- 成果物: crates/collector-windows/src/time_sync.rs
- 根拠: `time_sync.rs:15`（Task 8 F4）
- kind: technical
- 処置: fixed 8.1 —— `time_sync.rs` の `program()` で `%SystemRoot%\System32\w32tm.exe`（無ければ `C:\Windows\System32\w32tm.exe`）を走らせ、PATH から探さない。試験 `time_sync_program_is_resolved_from_the_system_root` と `time_sync_arguments_are_pinned_to_the_query`（置き場所を固定）

## R14. `tools/check-no-time-server.sh` が最上位の `#[cfg(test)]` 以降を全部読み飛ばし、自己検査は `DatagramSocket` の 1 種だけ、awk の失敗を拾わない
- 成果物: tools/check-no-time-server.sh
- 根拠: Task 10 F2〜F4
- kind: technical
- 処置: fixed 10.1 —— dc3fb28: 読み飛ばしを `#[cfg(test)]` 直後の `mod X {` 〜行頭 `}` に限り、awk / find の失敗を rc=2。自己検査で植える経路を 11 種と読めないファイルに

## R15. ポート分け（ruling）の周辺: `docs/testing.md:133` の `docker compose up -d --wait db && cargo test` が Story の worktree で繋がらない / `tools/seed.sh:17` の `BIND` 既定が 18787 のままで main のサーバへ偽データを入れる / `ashiato2-up-stNN` と `ashiato2-stNN` が同じポートを取り合う
- 成果物: docs/testing.md / tools/seed.sh / tools/ports.sh
- 根拠: `docs/testing.md:133`、`tools/seed.sh:17`、testdb の試験がポートの一致を固定
- kind: technical
- 処置: fixed 10.2 —— dc3fb28: `docs/testing.md:133` を `./tools/db.sh up -d --wait db && cargo test --workspace` に、`tools/seed.sh` の `BIND` 既定を `127.0.0.1:${ASHIATO_HTTP_PORT}` に。上流・下流の worktree のポートの衝突は触っていない（finish で処置）

## R16. D4 の反転条件の比を数えるのが確認バッチの DB（偽データ）で、本物の端末の c01-clock が届かない限り判定されない
- 成果物: tools/verify-prep.sh
- 根拠: `tools/verify-prep.sh:73`（Task 10 ⚠️）
- kind: daily
- 処置: fixed D4 仮 —— 数える DB は確認バッチの DB のまま（偽データは `c01-clock` を作らない: `grep -c c01-clock tools/seed.sh` = 0。`hourly` 0 件なら判定しないと手順書に出る）。design D4 に「数える DB（仮）」と反転条件（3 回続けて判定できなければ本番の S-01 の DB を数える口へ移す）を書いた

## R17. `LocationFix` の既定値（`receivedDeviceTime = at` / `fixElapsedNs = 0` など）が本物の値に見える
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationFix.kt
- 根拠: Task 3 F1。本番の組み立ては `FixCollector` の 1 か所
- kind: technical
- 処置: fixed 3.1 —— `LocationFix` の時計の 3 項目と `bootCount` から既定値を外した（本番の組み立ては `FixCollector` の 1 か所で全部渡している）。試験の組み立ては試験側の `testFix`（`test` / `androidTest` の `TestFix.kt`）に移した

## R18. `ClockSkewMeasurer` が `unreadable` の枝で `Date` の原文を `unavailable` に載せずに捨てる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ClockSkewMeasurer.kt
- 根拠: Task 5 F2
- kind: technical
- 処置: fixed D5 —— `unreadable` のときは `unavailable` の 1 件に `raw`（`Date` の原文）と `host` も載せる（`ClockSkewMeasurer.unavailableJson`）。design D5 に 1 行。試験 `ClockSkewMeasurerTest`「読めなかった Date の原文と宛先は取れなかった側に残る」

---

## finish の fix（25680be）と scoped re-review

- fix: 1 回（25680be。R9・R10・R13・R16〜R21・R23 を直し、R5・R11・R12 を反証、R24 を ST14 へ、R22 を deep.md の Q4 へ）
- scoped re-review 1 回（独立の subagent。25680be の diff だけ）: ADDRESSED 11 件（R9・R13・R16・R17・R18・R19・R20・R21・R23・R24 と R16 は B 仮として）、
  **PARTIAL 1 件**（R10: `time_sync` が `Ok` で `unparsed` のときログに出ない）、反証 3 件はいずれも妥当（R5 は「根拠はコミットの本文の自己申告だけ。中身は Q4 で本人に返っているので実害なし」）
- 新たな Important 1 件: R20 の子プロセスの許可一覧が行単位の文字列判定で、改行・別名の import ですり抜ける。Critical なし（`cargo test -p ashiato-collector-windows --lib` 122 passed、clippy 通過）
- R10 の残りと R20 の残りは fix 1 回の規則に従い直さず、ledger（st05-task-9 / st05-task-10 の progress.md）に Ruling つきで park した。PR 本文に載せる
- 証跡はいまの木で取り直した（3.1・5.2・6.1・6.2・8.1・9.1・9.2・9.3・1.3・10.1・10.4。すべて PASS）

## code-verify（独立検証。HEAD 5aa5b2f）

- 席: code-verify（実装者ではない）。日付: 2026-09-30
- **作業ツリーのコードは触っていない**（`git status --short` は空のまま）。壊す検査は `git archive HEAD` の複製 `/tmp/v-st05`
  （DB は別に立てた compose `st05verify`・55599）と、Windows 側の別の同期先 `C:\dev\ashiato2-rt-st05v` だけで行った
- Windows の実行時テストは共有の `C:\dev\ashiato2-rt` ではなく Story 用の `C:\dev\ashiato2-rt-st05` で走らせた（コマンドの中身は tasks と同じ。同期先の名前だけ違う）
- PR はまだ無い（`gh pr list --head feat/st05-clock-skew` は空）。CI は一度も走っていない

### 申告と実測

申告: tasks **29/29 `[x]`**（Task 1〜10）。final review の fix（dc3fb28）の後に取り直した証跡は無い
（`evidence.jsonl` の最新の head は 06adcec。4.1・6.2・9.3・10.1 のコードは dc3fb28 で変わっている）ので、全部 HEAD で走らせ直した。

| 項目 | 検証コマンド | 実測（HEAD 5aa5b2f） |
|---|---|---|
| 1.1 / 1.2 / 1.3（文書） | `grep -q …`（3 本） | 一致（rc=0） |
| 1.3 / 9.3（Windows 実行時） | `cargo test -p ashiato-collector-windows --test runtime_windows clock_ -- --test-threads=1` | 一致（rc=0、2 passed）。**W32Time は `STOPPED`**（`sc query w32time`）なので、動いている側の経路は走っていない |
| 2.1 | `tools/check-migrations.sh` / `cargo test -p ashiato-server clock_source_migration` | 一致（rc=0 / 1 passed） |
| 2.2 | `cargo test -p ashiato-server clock_record_ingest_` | 一致（2 passed。下限 2） |
| 3.1 / 4.1 / 4.2 / 5.1 / 5.2 / 6.1 / 6.2 | `./gradlew :app:testDebugUnitTest --tests '*<クラス>*'`（9 本） | 一致（9 本とも rc=0） |
| 3.2 / 5.3 / 9.3（文書） | `grep -q …` | 一致（rc=0） |
| 4.2（静的） | `! grep -nE "java\.net|…" ClockReferences.kt ResponseDateCache.kt ClockSkew*.kt` | 一致（rc=0） |
| 7.1 | `tools/android-emulator.sh` / `grep -q "Scenario: 収集の起動時にその場で測る" …ClockSkewInstrumentedTest.kt` | 一致（rc=0 / rc=0）。台本は `set -euo pipefail` なので 1 段目（`ClockSkewInstrumentedTest` を含む）が落ちれば非 0 で終わる。ただし 1 段目の結果の XML は 2 段目（`PermissionDeniedInstrumentedTest` 1 本）に上書きされ、走った件数は残らない。`@Ignore` / `assume` は無い |
| 8.1 / 8.2 / 8.3 | `cargo test … time_sync_` / `clock_reference_` / `clock_worker_` | 一致（11 / 6 / 3 passed。下限 5 / 4 / 2） |
| 8.3 | `cargo clippy -p ashiato-collector-windows --all-targets -- -D warnings` | 一致（rc=0） |
| 9.1 / 9.2 | `payload_shape_is_pinned` / `clock_skew_payload_` / `clock_skew_record_` / `clock_skew_is_measured` | 一致（1 / 5 / 8 / 1 passed。下限 1 / 4 / 8 / 1） |
| 10.1 | `tools/check-no-time-server.sh` / `--self-test` / `grep ci.yml` | 一致（rc=0）。**ただし一覧の外の経路 5 種は素通り**（R20） |
| 10.2 | `tools/smoke.sh` | 一致（rc=0、「縦串 OK」） |
| 10.3 | `grep -q "c01-clock" … "retry" tools/verify-prep.sh` | 一致（rc=0） |
| 10.4 | `check_scenarios.py` / `cargo test --workspace` / `assembleDebug testDebugUnitTest` / `check-private` / `check-panic-log` | 一致（rc=0。Scenario 496 件すべて担保あり、workspace 454 passed） |

### 手 1: 固定値を独立に再計算する — **すべて一致**

python（`datetime` / `email.utils.parsedate_to_datetime` / `str.encode('cp932')`）で実装とは別に計算した。

| 固定値 | 独立の計算 | 実装・テスト |
|---|---|---|
| 端末の壁時計 1_800_000_000_123 ms | `2027-01-15T08:00:00.123Z` | `ClockSkewPayloadShapeTest` の `device_time` |
| network / gnss の差 | 300023 / 300033 | 同上 |
| `Fri, 15 Jan 2027 07:55:00 GMT` と差 | 1799999700000 ms（金曜）、`wallAfter` との差 300456 | 同上の `s01-date` |
| cp932 の見出し 4 つ（最終正常同期時刻 / ソース / フェーズ オフセット / 未指定） | `8dc58f49…8d8f` / `835c815b8358` / `8374…8367` / `96a28e7792e8` | `time_sync.rs` の `LAST_SYNC_KEYS` ほか（4 つとも一致） |
| `0x80070426` を i32 で | −2147023834 | `SERVICE_NOT_STARTED` と `time_sync_stopped_service_is_reported_as_such` |
| 位相のずれ `0.0004680s` / `-0.0012345s` | 0 ms / −1 ms（四捨五入） | `time_sync_parses_*` |

### 手 2: ガードをわざと壊す

| ガード | 壊し方（複製で） | 結果 |
|---|---|---|
| `check-no-time-server.sh`（Q1） | 一覧の語（`UdpSocket`）を `crates/collector-windows/src/zz.rs` に植える | rc=1（止まる） |
| 同上 | 一覧の外の経路 5 種を 1 つずつ植える | **5 種とも rc=0**（R20） |
| 移行の `external_id_kind = 'none'`（C1） | `'record'` にする | 2 本落ちる（`clock_source_migration_applies_twice` / `clock_record_ingest_day_is_not_device_achieved_day`）。`…stores_record_without_external_id` は共有 DB に前の行が残るので通るが、CI の新しい DB では落ちる経路 |
| 移行の想定間隔 21600（D1 仮） | 3600 にする | 336 本緑（仮なので指摘にしない。反転条件の受け手が居ない件は R24） |
| `w32tm` の引数（照会だけ） | `/resync` にする | `time_sync_arguments_are_pinned_to_the_query` が落ちる |
| `.down.sql` の guard | final review の R8 で試験済み（`clock_source_down_keeps_row_referenced_by_drop_report`） | 触っていない |

### 手 3: Scenario と test

`python3 scripts/check_scenarios.py . st05-clock-skew` rc=0。この change の 66 本はすべて印がある（印の置き場所は 66 本とも grep で確かめた）。
印の先を読んで、主張の階層とずれていたのは 3 本（R19 の「1 時間ごとに 1 件」、R21 の「測るために通信を起こさない」、R23 の「読んだままの出力が残る」）。
それ以外で読んだもの（`差に使う端末の時計は…`（呼び出し順 `mono → network → wall → mono` を固定）/ `PC の時計が進んでいると…`（偽の取り込み口で 300000〜301000+幅）/
`時計の変更より前に…`（書き換えると落ちる。A4）/ `圏外が 1 日続くと…`（書き換えると落ちる。A7））は主張を観測している。

### 手 4: 本人の決定が test で固定されているか

複製で 1 つずつ書き換え、テストを走らせて元に戻した（`/tmp/st05v/mut.py`）。

| 決定 | 書き換え | 結果 |
|---|---|---|
| FR-7 / C2: 端末は 1 時間ごと | `CLOCK_SKEW_INTERVAL_MS` を 2 時間に | **全部緑**（R19） |
| D4（仮）: 測り直し 5 分 | `CLOCK_SKEW_RETRY_MS` を 15 分に | **全部緑**（R19） |
| C8: PC は 1 時間ごと | `SKEW_INTERVAL_SEC` を 7200 に | 4 本落ちる |
| C3: 飛び 60 秒で測る（PC） | `CLOCK_JUMP_SEC` を 600 に | 1 本落ちる |
| C3: 時計の変更で測る・応答の日付を捨てる（端末） | 受け手から `timeChanged()` / `clockChanged()` を消す | それぞれ 1 本落ちる |
| C2: 取れなかった記録は 1 時間の契機ごとに 1 件（24 件/日） | 端末: 測り直し中の 1 時間の契機も積まない / PC: 毎回積む・一度も積まない | 3 本 / 4 本 / 5 本落ちる |
| C1: 別の論理ソース `c01-clock` | `c01-location` にする | 10 本落ちる |
| C7: 出来事時刻 = 測ったときの端末の時計 | `+1 ms` ずらす | 1 本落ちる |
| C5 / D2: 版の区別（`unsupported`） | `minSdk = 33` を 29 に | 2 本落ちる |
| Q2: 位置の 3 項目 | 受け取り時刻を測位の時刻に / 経過時間を 0 に / 起動の識別を null に | それぞれ 1 本落ちる |
| Q3 ②: 差は受け取った直後の壁時計 | `wall_after` を記録の時刻 `ctx.at` に | 3 本落ちる |
| Q3 ③: Windows の時刻同期の状態を並べる | 作業スレッドで `time_sync.read()` を呼ばず常に `spawn_failed` | **Linux の 118 本は全部緑**。Windows 側（`C:\dev\ashiato2-rt-st05v`）の `clock_skew_runtime` は **rc=101 で落ちる**（`runtime_windows.rs:730`: `left: "spawn_failed" right: "service_stopped"`）。固定しているのは Windows の実行時テストだけ（CI の `collector-windows-runtime`） |
| Q1: 時刻サーバへ問い合わせない | 手 2 のとおり | 一覧の語は止まる / 外は素通り（R20） |
| D8: `w32tm` の打ち切り 5 秒 | 60 秒に | 全部緑（design の値で本人の決定ではないので指摘にしない） |

### 手 5: tasks の `[x]` と実体

29 件とも、本文の検証コマンドが実在し、HEAD で rc=0（上の表）。名指しの試験（`clock_source_migration_applies_twice`・
`LocationFixClockFieldsTest` の「原文を 2 回組み立てて一致」・`ClockSkewMeasurerTest` の 8 通りの総当たり・`clock_time_sync_is_readable`・`clock_skew_runtime` ほか）も実在する。
tasks の cargo は全部件数つきの形で、0 本一致の rc=0 はない。
食い違いは 2 点だけで、どちらも既出か実害なし: 9.3 の本文の書き換え（final review の R5）と、dc3fb28 の後に証跡が取り直されていないこと（HEAD で走らせ直すと全部通る）。

### 手 6: 隙間

- 端末で測定記録を未送信に書けなかったときは、位置と同じ `Outbox.add` → ST04 の書けなかった記録の数えに乗る（`Outbox.kt:45-60`）。失われるのではなく破棄として報告される
- `ResponseDateCache` はメモリだけで、プロセスを立て直すと消える —— design D2 が書いており、起動時の測定は `no_response_since_last` で残る
- **W32Time が止まっている間、PC の測定に同期の状態が残らない**（R22。手元の既定の状態）
- **D1 の反転条件を見張る Story が居ない**（R24。ST14）

## R19. 端末の「1 時間ごと」と測り直しの「5 分」が test で固定されていない。2 時間・15 分に変えても全部緑
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ClockSkewScheduler.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/ClockSkewSchedulerTest.kt / LocationServiceClockTest.kt
- 根拠: 複製で `ClockSkewScheduler.kt:8` の `CLOCK_SKEW_INTERVAL_MS` を `2 * 60 * 60 * 1000L` にして `./gradlew :app:testDebugUnitTest --tests '*Clock*' --tests '*LocationService*' …` → rc=0（`compileDebugKotlin` が走ったことをログで確認）。
  `:11` の `CLOCK_SKEW_RETRY_MS` を 15 分にしても rc=0。試験は `assertEquals(CLOCK_SKEW_INTERVAL_MS, r.hourly.periodMs)`（`ClockSkewSchedulerTest.kt:58`・`LocationServiceClockTest.kt:91`）と
  `assertEquals(CLOCK_SKEW_RETRY_MS, …)`（`ClockSkewSchedulerTest.kt:108`・`LocationServiceClockTest.kt:92`）で**定数を定数と比べている**。
  同じ repo の `LocationServiceTest.kt:328` が「`assertEquals(HEARTBEAT_INTERVAL_MS, periodMs)` だけだと定数を 1 分にしても緑」と書いて避けた型そのもの。
  Scenario「1 時間ごとに測定記録が 1 件残る」「圏外が 1 日続くと取れなかった記録は 24 件」の 24 件は偽の刻みを 24 回叩いた数で、時間から出た数ではない。
  PC 側（`SKEW_INTERVAL_SEC`）は 86400 秒を回す試験があり、7200 にすると 4 本落ちる
- kind: technical
- 提案: `IntervalTest` と同じく `assertEquals(3_600_000L, CLOCK_SKEW_INTERVAL_MS)` と `assertEquals(300_000L, CLOCK_SKEW_RETRY_MS)` をリテラルで置く（5 分は D4（仮）なので、反転したら試験も一緒に直す）
- 処置: fixed 6.1 —— 間隔をリテラルで固定した（`ClockSkewSchedulerTest` と `LocationServiceClockTest` で `3_600_000L` / `300_000L`）。定数を 2 時間・15 分にすると 3 本落ちることを確認

## R20. `check-no-time-server.sh` は一覧の語しか見ず、一覧の外の時刻サーバへの経路 5 種を素通しする。依存（`build.gradle.kts` / `Cargo.toml`）も見ていない
- 成果物: tools/check-no-time-server.sh
- 根拠: 複製の `collector-android/app/src/main/…/Zz.kt` と `crates/collector-windows/src/zz.rs` に 1 つずつ植えて `tools/check-no-time-server.sh` を実行 —— 5 種とも **rc=0**:
  `org.apache.commons.net.ntp.NTPUDPClient().getTime(InetAddress.getByName("ntp.nict.jp"))` /
  `java.nio.channels.DatagramChannel.open().send(buf, InetSocketAddress("ntp.nict.jp", 123))` /
  `Command::new("w32tm").args(["/stripchart", "/computer:time.nist.gov", "/samples:1"])`（外部の時刻サーバへ直接問い合わせる）/
  `Command::new("sc").args(["start", "w32time"])`（Windows に同期させる操作。spec「Windows に時刻を同期させる操作もしない」）/
  `sntpc::simple_get_time(("ntp.nict.jp", 123), &sock)`。対照に `std::net::UdpSocket::bind` を植えると rc=1。
  `PATTERN`（`tools/check-no-time-server.sh:15`）は語の一覧で、`find` は `*.kt|*.java|*.rs|*.xml` だけ（`:22`）なので、依存に NTP の部品を足しても見えない。
  Scenario「外部の時刻サーバへ問い合わせない」「PC は外部の時刻サーバへ問い合わせない」の印はこの台本だけ。守っているのは Q1（`exported`）
- kind: technical
- 提案: 語を足す（`DatagramChannel` / `NTPUDPClient` / `sntpc` / `/stripchart` / `/config` / `start w32time` / `time.nist.gov` / `nict`）だけでなく、
  PC は `Command::new` の相手を `w32tm` 1 つ・引数を照会だけに限る検査（許可の一覧）にし、`Cargo.toml` と `build.gradle.kts` の依存も同じ一覧で見る。自己検査に上の 5 種を植える
- 処置: fixed 10.1 —— `tools/check-no-time-server.sh` に一覧の外だった 5 種の語（`DatagramChannel` / `NTPUDPClient` / `sntp` / `/stripchart` / `/computer` / `/config` / `time.nist.gov` / `nict`）と、PC の `Command::new` の相手を `program(...)`（w32tm の照会）だけに限る許可の一覧・端末の子プロセス（`ProcessBuilder` / `Runtime.exec`）を足し、依存（`Cargo.toml` 2 つ・`build.gradle.kts` 2 つ）も同じ一覧で見る。自己検査に 5 種と依存 3 種・子プロセス 1 種を植えた（20 種とも止まる。照会の子プロセスは通る）

## R21. Scenario「測るために通信を起こさない」の印は通信の口を持たない部品（`ClockReferences`）の上にあり、`LocationService` が測るたびに POST しても全部緑
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/ClockReferencesTest.kt / collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: 複製で `LocationService.kt:236` の `emit = { outbox.add(it) }` を `emit = { outbox.add(it); newTransport("/ingest").post("[]") }` にして
  `./gradlew :app:testDebugUnitTest`（全部）→ **rc=0**。印の試験（`ClockReferencesTest.kt:140`）は `ClockReferences` に `Transport` を渡しておらず、
  `Sender` の呼び出し回数が変わらないのは構造上いつも真。spec の THEN は「収集側が S-01 へ送った要求の数」で、測定を組み込む場所（`LocationService`）の段
- kind: technical
- 提案: `LocationServiceClockTest` で、偽の `Transport` の呼び出し回数を `start()`（起動時の測定）・時計の変更の通知・1 時間の刻みの前後で数える試験を置き、印を移す
- 処置: fixed 6.2 —— `LocationServiceClockTest`「起動・時計の変更・1 時間・測り直しで測っても送信の要求は 1 つも増えない」を置き、印を移した（`emit` に POST を足すと落ちることを確認）

## R22. W32Time が止まっている間（手元の既定の状態）、PC の測定記録に「最後に同期した時刻・同期元」が 1 つも残らない。同じ情報はイベントログにあるが、D8 の反転条件（イベントログ）は確かめられていない
- 成果物: crates/collector-windows/src/time_sync.rs / openspec/changes/st05-clock-skew/design.md（D8）
- 根拠: `sc.exe query w32time` → `STATE : 1 STOPPED`、`sc.exe qc w32time` → `START_TYPE : 3 DEMAND_START`、`sc.exe qtriggerinfo w32time` → 起動の契機はドメインへの参加と独自のシステム状態の変化だけ。
  この状態で HEAD の `clock_skew_runtime` の記録は `clock_unavailable` に `windows-time-sync` / `service_stopped` だけを持つ（`time_sync.rs:50`）。
  一方 `wevtutil.exe qe System "/q:*[System[Provider[@Name='Microsoft-Windows-Time-Service']]]" /c:5 /rd:true /f:text` → rc=0 で、
  Event 35（`2026-09-30T01:18:08Z`「タイム ソース time.windows.com,0x9 … の同期をとっています」）と Event 37 が読めた —— Q3 ③ が求めた「最後に同期した時刻と同期元」はサービスが止まっていても OS に残っている。
  design D8 は読めないときの候補にこのイベントログを挙げ、「どの口でも読めなければ deep に止める」としていたが、2026-09-30 の訂正（commit 51cebee、tasks 9.3 の ★）は `service_stopped` で残す形に倒し、
  イベントログは確かめていない。`deep.md` に W32Time 停止の記録は無い（`grep -n "W32Time\|service_stopped" deep.md` は 0 件）。
  **確かめられなかったこと**: 管理者権限なしで System ログが読めるか（読んだのは WSL から起動した高い完全性のプロセス。権限を下げて走らせる操作は実行を拒否されたので行っていない）
- kind: premise
- loss: uncaptured
- 提案: 本人に問う —— 「W32Time が止まっている間は、`w32tm` の代わりにイベントログ（Time-Service の 35 / 37）から最後の同期の時刻と同期元を読んで並べるか / 止まっていることだけを残す（いま）か」。
  並べるなら先に一般の利用者の権限で読めるかを確かめる。どちらでもサービスは起動しない（本人の指示のまま）
- 処置: escalated —— `deep.md` の第 2 回 Q4（`docs/briefs/ST05-deep-r2.html`）。本人の答え（2026-09-30。推奨の側）を 4703145 で入れた: `w32tm` が `0x80070426` のときだけ `wevtutil qe` で Time-Service の Event 35 / 37 の最新 1 件を読み、`sync_via: eventlog` で並べる。記録が無い・読めなければ `service_stopped` のまま。試験 `time_sync_stopped_service_falls_back_to_the_event_log` / `time_sync_event_log_*` / `clock_skew_payload_marks_time_sync_read_from_the_event_log`、実行時テスト `clock_time_sync_event_log_is_readable`（手元で PASS）。一般の利用者の権限はチャネルの ACL（`IU` に読み取り）で確かめた。権限を下げた実測はしていない（design D8）

## R23. Scenario「Windows の時刻同期の状態は読んだままの出力が残る」は解析の構造体の段でしか観測されず、記録の `raw` を元のバイト列へ戻せることを固定する試験が無い。`\` をエスケープしなくても 118 本緑
- 成果物: crates/collector-windows/src/time_sync.rs / crates/collector-windows/src/clock_record.rs
- 根拠: 印の試験（`time_sync_parses_english_output` / `…_japanese_output`）は `parse_status` の戻り値の `raw: Vec<u8>` が入力と等しいことだけを見る。
  記録に載るのは `raw_text()`（`time_sync.rs:144`）で ASCII 以外と `\` を `\xNN` にした文字列（design D10（仮）の「元のバイト列へ戻せる」）。
  複製で `if b.is_ascii() && b != b'\\' {` を `if b.is_ascii() {` にして `cargo test -p ashiato-collector-windows` → **118 passed, rc=0**。
  こうすると出力に `\x8d` という 4 文字があったとき、cp932 の 1 バイトと区別できなくなる（戻せない）
- kind: technical
- 提案: `\` と `\x8d` の文字列と cp932 のバイト列を混ぜた入力で、`raw_text` を戻す関数（試験の中でよい）を通すと元のバイト列に一致する往復の試験を置き、印をそこへ移す
- 処置: fixed 9.1 —— `time_sync_raw_text_round_trips_to_the_bytes`（`\` と `\x8d` の 4 文字と cp932 のバイトを混ぜた入力を `raw_text` から戻して一致）を置き、Scenario の印を付けた。`\` のエスケープを外すと落ちることを確認

## R24. D1 の反転条件（`c01-clock` は生存信号を送らないので ST14 の途絶の判定で途絶に見える）が ST14 に申し送られていない
- 成果物: docs/handoff/ST14.md / openspec/changes/st05-clock-skew/design.md（D1）
- 根拠: `docs/stories/ST14.md:18-19` の FR-35 は「あるソースの最後の記録または最後の生存信号からの経過時間が、そのソースに登録された想定間隔の 3 倍を超える」で通知する ——
  `c01-clock` は登録簿に 21600 秒（`migrations/202609291230_clock_source.sql`）で入るので、端末が 18 時間測れない（電源断・Doze の長い遅れ）と `c01-location` とは別に通知が鳴る。
  design D1 の反転条件はちょうどこの場合を「ST14 の側で測定のソースを除くか、想定間隔を 3600 にする」と書くが、`grep -n "c01-clock\|clock" docs/stories/ST14.md` は 0 件、
  `docs/handoff/ST14.md` に載っているのは st05 の R6（D12）だけ。ST14 は tasks.md を持たない（`ls openspec/changes | grep st14` は空）ので、いまなら申し送れる
- kind: defer
- 提案: `docs/handoff/ST14.md` に「st05-clock-skew D1 —— `c01-clock` は生存信号を送らない測定のソース。FR-35 の途絶の判定から除くか、想定間隔で判定するかを ST14 の上流で決める」を足し、処置を `followup ST14` にする

- 処置: deferred ST14 —— ST14 は tasks.md を持たない（`ls openspec/changes | grep st14` は空）。畳み方の候補と根拠は `docs/handoff/ST14.md` の「st05-clock-skew R24」に書いた

## Q4 の答えの反映（4703145）と scoped re-review

- deep 第 2 回 Q4 の本人の答え（2026-09-30。推奨の側）を 4703145 で入れた（R22 の処置の本文）
- scoped re-review 1 回（独立の subagent。4703145 の diff だけ。変異は `git archive` の写しで）: Critical 0 / Important 3 / Minor 3。**6 件とも fe2a8f6 で直した**
  - I-1 本番の `read` の組み込みが試験で固定されていない（写しで組み込みを外しても 128 本緑）→ 子プロセスを走らせる口を差し替えられる `read_with` に切り出し、
    `time_sync_read_runs_the_event_log_query_only_when_stopped`（止まっていれば 2 本目に `wevtutil` と固定の引数、動いていれば 1 本だけ）で固定。
    残るのは `read` の 1 行（`read_with` に本物の `run_with_timeout` を渡す）で、実機では `clock_skew_runtime` が W32Time の止まっているときに見る
  - I-2 `<Event` の無い出力を全部「0 件」にして黙って `service_stopped` に倒す → 空（空白だけ）のときだけ 0 件、それ以外は `unparsed` で原文つき
    （`time_sync_event_log_unexpected_output_is_not_taken_as_no_records`）
  - I-3 `program()` が exe 名を取るようになり、`check-no-time-server.sh` が System32 のどの exe（`sc start w32time` も）も通すようになった（写しで植えて rc=0）
    → 口を `w32tm_program()` / `wevtutil_program()` に戻し、検査は `Command::new` にその 2 つの形だけを許す。自己検査に `program(…, "sc.exe")` を植える 1 種を足した（21 種）
  - M-1「`0x80070426` のときだけ」が固定されていない → `exit:1` はイベントログを読まないことを足した
  - M-2 固定入力が Event 37 だけ → 手元で読めた Event 35 の XML を足した（`time_sync_event_log_reads_event_35`）
  - M-3 design の「w32tm は 5 秒」→「最大 2 本 × 5 秒」に直した（design D8 と D7 の危険の節、`clock_worker.rs` の説明）
- 反映の fix は答え 1 件について 1 回。re-review の後の fix（fe2a8f6）はさらに re-review していない（SDD の final と同じく 1 回ずつ）
