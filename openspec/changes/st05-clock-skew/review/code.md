# ST05 code レビュー —— st05-clock-skew

## final review（308dd45..d499da1）

- 席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package `.superpowers/sdd/tasks/review-308dd45..d499da1.diff`
- 入力: 全 Task の `task-*-findings.md` の Minor と ⚠️、`st05-task-10/progress.md` の Ruling
- 判定: With fixes（Critical なし / Important 5 / Minor 13）
- 日付: 2026-09-30

## R1. `HttpTransport` がコンストラクタで `URL()` の例外を投げ、`BASE_URL` が不正だとサービスの起動が落ち続ける
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HttpTransport.kt
- 根拠: `HttpTransport.kt:30` の `hostPort` がプロパティ初期化で `URL(this.baseUrl)`。`Config.isComplete` は空でないことしか見ず、`startFlushing()` の `newTransport()` で `MalformedURLException` → `onStartCommand` が落ち `START_STICKY` で繰り返す。以前は `post()` 内で `Unreachable` になり未送信に積まれていた（Task 5 F6）
- kind: technical

## R2. 応答の `Date` を置くのが `/ingest` の Sender だけで、design D2 の「`/ingest` `/heartbeat` `/drops` のどれでも」と違う
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `LocationService.kt:367-387` —— `responseDates` を `sender` にだけ渡し、`beatSender` / `dropSender` には渡していない（Task 4 ⚠️）
- kind: technical

## R3. `ClockSkewScheduler` が `stop()` の後・測り直しを畳んだ後にも測る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ClockSkewScheduler.kt
- 根拠: `ClockSkewScheduler.kt:52-69` —— `shutdownNow()` は `synchronized(lock)` を待つ糸を止めない。(a) hourly が取れて retry を畳んだ直後に待っていた retry が余分な `retry` 記録を積み、D4 の反転条件の比（`2*retry > hourly`）を汚す。(b) `stop()` と同時に待っていた hourly が取れないと新しい retry の executor を立て、`onDestroy` の後も 5 分ごとに回る（Task 6 F1）
- kind: technical

## R4. `clock_skew_runtime` のループが「未送信 0 件」で抜けるので、worker の結果より先に別の記録が積まれると `skew.len() == 1` が落ちうる。W32Time が動いている経路は一度も走っていない
- 成果物: crates/collector-windows/tests/runtime_windows.rs
- 根拠: `runtime_windows.rs:680` の終了条件 `rt.pending().0 == 0`。evidence の 9.3 の PASS は W32Time が止まっている経路のもの（Task 9 F1 / ⚠️）。このブランチの CI は PR 前でまだ走っていない
- kind: technical

## R5. `tasks.md` の 9.3 の本文が commit 51cebee で書き換わっている（「★ 2026-09-30 本人の指示で訂正」）
- 成果物: openspec/changes/st05-clock-skew/tasks.md
- 根拠: `git show 51cebee -- openspec/changes/st05-clock-skew/tasks.md`。evidence の executor は manual。CLAUDE.md は implementer が tasks.md を触らないと定める（Task 9 ⚠️）
- kind: technical

## R6. `s01-date` の壁時計を `monoAfter` の後に読んでおり、Scenario「差に使う端末の時計は基準を読む前後の間で読む」に字面で反する
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HttpTransport.kt
- 根拠: `HttpTransport.kt:51` —— `wallClock()` が `monoAfter` の後（Task 4 F4）。差は µs 規模
- kind: technical

## R7. `clock_skew_crashed` のログの source が `c01-location` になる（測定のログは `c01-clock`）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `LocationService.kt:239` —— `Telemetry.line` の既定の source。試験もそれで固定している
- kind: technical

## R8. 移行の `.down.sql` の guard が `core.event` しか見ず、`core.drop_report` に c01-clock の行があると FK エラーで落ちる
- 成果物: migrations/202609291230_clock_source.down.sql
- 根拠: spec「測定記録も…破棄として報告される」で `core.drop_report` に c01-clock が入りうる。騒がしく落ちるので失われるものは無い（Task 2 F4）
- kind: technical

## R9. PC で飛び（`reset(Jump)`）の後に飛ぶ前に始めた読み取りが失敗で返ると、`failed(mono)` が `last` を上書きし jump の測定が 60 秒遅れる
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: `runtime.rs:469-499`（Task 9 F5。以前からある性質）
- kind: technical

## R10. PC の取れなかったときのログの error に `s01-date` の理由しか出ず、`unwrap_or("time_sync")` の枝に来ない
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: `runtime.rs:491`（Task 9 F3）
- kind: technical

## R11. `clock_worker.rs` の `#[cfg(test)] settled` と `settle_clock_worker: true`（既定）で、Runtime の既存の試験はほぼ同期の経路しか通らない
- 成果物: crates/collector-windows/src/clock_worker.rs
- 根拠: Task 8 F3
- kind: technical

## R12. `LocationService` の `onReceive` と `onStartCommand` → `start()` が main スレッドで測定と `outbox.add`（ファイル I/O）を行う
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: Task 6 F3
- kind: technical

## R13. `w32tm` を PATH / exe の置き場所から解決している（`%SystemRoot%\System32\w32tm.exe` が固い）
- 成果物: crates/collector-windows/src/time_sync.rs
- 根拠: `time_sync.rs:15`（Task 8 F4）
- kind: technical

## R14. `tools/check-no-time-server.sh` が最上位の `#[cfg(test)]` 以降を全部読み飛ばし、自己検査は `DatagramSocket` の 1 種だけ、awk の失敗を拾わない
- 成果物: tools/check-no-time-server.sh
- 根拠: Task 10 F2〜F4
- kind: technical

## R15. ポート分け（ruling）の周辺: `docs/testing.md:133` の `docker compose up -d --wait db && cargo test` が Story の worktree で繋がらない / `tools/seed.sh:17` の `BIND` 既定が 18787 のままで main のサーバへ偽データを入れる / `ashiato2-up-stNN` と `ashiato2-stNN` が同じポートを取り合う
- 成果物: docs/testing.md / tools/seed.sh / tools/ports.sh
- 根拠: `docs/testing.md:133`、`tools/seed.sh:17`、testdb の試験がポートの一致を固定
- kind: technical

## R16. D4 の反転条件の比を数えるのが確認バッチの DB（偽データ）で、本物の端末の c01-clock が届かない限り判定されない
- 成果物: tools/verify-prep.sh
- 根拠: `tools/verify-prep.sh:73`（Task 10 ⚠️）
- kind: daily

## R17. `LocationFix` の既定値（`receivedDeviceTime = at` / `fixElapsedNs = 0` など）が本物の値に見える
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationFix.kt
- 根拠: Task 3 F1。本番の組み立ては `FixCollector` の 1 か所
- kind: technical

## R18. `ClockSkewMeasurer` が `unreadable` の枝で `Date` の原文を `unavailable` に載せずに捨てる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ClockSkewMeasurer.kt
- 根拠: Task 5 F2
- kind: technical
