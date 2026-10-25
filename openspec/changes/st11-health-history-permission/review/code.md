# ST11 のコードレビュー

## final review（934d18d..38cc427）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）が review package
`.superpowers/sdd/tasks/review-934d18dc..38cc4272.diff`（35 commits）と Task 1〜8 の指摘の記録（Minor・⚠️）、
`docs/handoff/ST11.md` の C6 を見た。判定は「直せば可」。Critical 0 / Important 4 / Minor 8。
走らせたもの: `check_scenarios.py` → `scenarios: OK` rc=0。

## R1. 2 つの recorder が `emitted` の控えを別々に持ち、同じ置き場を共有するので、変化を取りこぼすか重ねて記録する
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthPermissionRecord.kt
- 根拠: `observeHealthPermissions` の `if ((emitted ?: store.last()) == canonical) return`。`MainActivity` と `LocationService` が別々の `HealthPermissionRecorder` を作り、`HealthStateStore` は共有、`emitted` と `lock` はインスタンスごと。周期の見直しが S2 を記録 → 許し直してアプリを開き MainActivity が S1 を記録（置き場は S1）→ 再び外す（実際 S2）と、サービスは `emitted == S2` で送らず S-01 の最後は S1 のまま（spec「後から設定で外すと次の見直しで記録される」が崩れる）。逆向きでは同じ状態を 2 件積む（D2 に反する）。Task 6 の ⚠️（2 つの recorder の排他）はこれで確定
- kind: technical

## R2. 正典の「その場では取れない権限」の型の文（handoff C6）が手当てされていない
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md
- 根拠: 正典 `openspec/specs/device-collection/spec.md:958-960` が「ST11 が引き継ぐ…同じ『その場では取れない権限』の型」と書く。ST11 の premises P4・P14 では種類と一緒に求めればその場で画面が出る。delta の MODIFIED は「ソースごとに独立して収集する」だけで、design.md にも直さない理由が無い（`deep.md:94` は spec の段へ先送りのまま）
- kind: technical

## R3. 6 時間ごとの見直しの `runBlocking` に時間の上限が無く、Health Connect の IPC が返らないと位置の生存信号が止まる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `observeHealthPeriodically` は位置の生存信号の `scheduler.every` の中から呼ばれ、`runCatching` は止まった呼び出しを受けない。止まると生存信号と `outagePoller.poll()` が同じ糸で止まり、稼働状況に途絶として出る。Task 6 F1 の格上げ
- kind: technical

## R4. 計測テスト（Grant 5・HistoryDenied 1・Denied 1・PermissionDenied 1）が一度も走っていない
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermission*InstrumentedTest.kt / tools/android-emulator.sh
- 根拠: `evidence.jsonl` に AT の件数つきの検証が 0 件。Task 7 の report が「WSL でエミュレータが起動しない」と自認。UI Automator が読む Health Connect の文言・2 枚目のトグル・`pm reset-permissions` で断った回数が戻るかは未確認。`compileOnly(guava)` の実行時の classpath（M8）も端末の上でしか確かめられない
- kind: technical

## R5. 計測テストの `healthRecords()` の退避先が本番と違い、試験が生きている未送信を書き換えうる
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthConnectScreens.kt
- 根拠: `HealthConnectScreens.kt:114` の `File(dir, "u.jsonl")`。本番は `LocationService.UNREADABLE`（`unreadable.jsonl`）。Task 7 F3 の未対応
- kind: technical

## R6. `HealthRecordHandoff.drainInto` の失敗時の扱いと、主糸についてのコメントが実態と食い違う
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthRecordHandoff.kt / LocationService.kt
- 根拠: 1 件でも積めないと poll 済みの記録が消え、積めた分の action も捨てる（次に観たとき作り直す側に倒れる）。`onStartCommand` の中で主糸のままディスクに書くのに、直上のコメントは「主糸では回さない」。Task 5 F7 の未対応
- kind: technical

## R7. `health_permission_settings_missing` を通知を組み立てるたびに出している
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `healthPermissionPendingIntent` が日数の本文の差し替えのたびにも同じログを出す。Task 6 F4
- kind: technical

## R8. 計測テストの判定が緩い・doc が実態と違う
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermissionGrantInstrumentedTest.kt / HealthPermissionDeniedInstrumentedTest.kt
- 根拠: `:104` の `kindsVisible() || additionalAccessVisible()` は種類の画面を見なくても通る（Task 7 F6）。`:168` が鍵の順序まで比べる（F4）。`:31` の doc（各試験が自分で開き直す）が実態と違う（F5）。Denied の doc は実際は 2 回断っている（F2）
- kind: technical

## R9. 試験の名前 `st09_mig_applies_twice_and_is_last` が中身と食い違う
- 成果物: crates/server/src/media_migration_tests.rs
- 根拠: `:47`。末尾とは限らない版を見ている。Task 1 F2
- kind: technical

## R10. `day_tests.rs` が収集開始日を無条件で 2002 年に上書きするようになった
- 成果物: crates/server/src/day_tests.rs
- 根拠: `:1447`。Task 1 の範囲の外の変更で、共有 DB を並走する試験との窓が残る（Task 1 F1）。理由はコメントにあり、元へ戻している
- kind: technical

## R11. 試験の穴（`parseResult`・13 以前の設定の intent・バックアップ規則の節ごと・戻しの移行の `core.event`・例外時の生存信号）
- 成果物: collector-android/app/src/test/ / crates/server/src/
- 根拠: Task 3 F1・Task 6 F2・Task 2 F1・Task 1 F3・Task 6 F3。final reviewer はどれも許容と判定
- kind: technical
