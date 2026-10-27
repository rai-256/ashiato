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
- 処置: fixed 4.1 — 6e505420（`emitted` を書き換えを遅らせている間だけの `pending` にし、2 つの recorder が置き場を共有する試験 4 本。scoped re-review で ADDRESSED）

## R2. 正典の「その場では取れない権限」の型の文（handoff C6）が手当てされていない
- 成果物: openspec/changes/st11-health-history-permission/specs/device-collection/spec.md
- 根拠: 正典 `openspec/specs/device-collection/spec.md:958-960` が「ST11 が引き継ぐ…同じ『その場では取れない権限』の型」と書く。ST11 の premises P4・P14 では種類と一緒に求めればその場で画面が出る。delta の MODIFIED は「ソースごとに独立して収集する」だけで、design.md にも直さない理由が無い（`deep.md:94` は spec の段へ先送りのまま）
- kind: technical
- 処置: fixed D13 — （delta に MODIFIED「利用状況へのアクセスを初回起動で求める」を逐語で写し、申し送りの段落だけを直した。8403e4ab）

## R3. 6 時間ごとの見直しの `runBlocking` に時間の上限が無く、Health Connect の IPC が返らないと位置の生存信号が止まる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `observeHealthPeriodically` は位置の生存信号の `scheduler.every` の中から呼ばれ、`runCatching` は止まった呼び出しを受けない。止まると生存信号と `outagePoller.poll()` が同じ糸で止まり、稼働状況に途絶として出る。Task 6 F1 の格上げ
- kind: technical
- 処置: fixed 6.1 — f27fbe7b・a602479b（見直しを取得の糸へ移し、読みに 30 秒の `withTimeout`。返らない読みでも生存信号が出続ける試験）

## R4. 計測テスト（Grant 5・HistoryDenied 1・Denied 1・PermissionDenied 1）が一度も走っていない
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermission*InstrumentedTest.kt / tools/android-emulator.sh
- 根拠: `evidence.jsonl` に AT の件数つきの検証が 0 件。Task 7 の report が「WSL でエミュレータが起動しない」と自認。UI Automator が読む Health Connect の文言・2 枚目のトグル・`pm reset-permissions` で断った回数が戻るかは未確認。`compileOnly(guava)` の実行時の classpath（M8）も端末の上でしか確かめられない
- kind: technical
- 処置: fixed 7.1 — 7a9bc6b6（`-wipe-data` の AVD で初めて走らせた: PermissionDenied 1/0・HistoryDenied 1/0・Denied 1/0・Grant 5 本中 t4 が失敗・他 5 クラス 6/0。XML は `collector-android/app/build/outputs/androidTest-results-st11-*/`。`pm reset-permissions` で断った回数は戻る。t4 の失敗は R12 で人間へ。Android 14 以降で初回の権限の画面が一度も出ていなかった製品の不具合を見つけ e2ef8442 で直した（D3 の反転条件）。`RetentionInstrumentedTest` は端末で走らせていない）

## R5. 計測テストの `healthRecords()` の退避先が本番と違い、試験が生きている未送信を書き換えうる
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthConnectScreens.kt
- 根拠: `HealthConnectScreens.kt:114` の `File(dir, "u.jsonl")`。本番は `LocationService.UNREADABLE`（`unreadable.jsonl`）。Task 7 F3 の未対応
- kind: technical
- 処置: fixed 7.1 — 687823f5

## R6. `HealthRecordHandoff.drainInto` の失敗時の扱いと、主糸についてのコメントが実態と食い違う
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthRecordHandoff.kt / LocationService.kt
- 根拠: 1 件でも積めないと poll 済みの記録が消え、積めた分の action も捨てる（次に観たとき作り直す側に倒れる）。`onStartCommand` の中で主糸のままディスクに書くのに、直上のコメントは「主糸では回さない」。Task 5 F7 の未対応
- kind: technical
- 処置: fixed 5.1 — 6e505420・f27fbe7b（受け渡し口を預けた順の 1 本の列にし、書けた記録の手続きは走らせる。受け渡しを `runCollection` の糸へ。書けなかったとき画面側の `pending` が残り、サービスが同じ状態をもう 1 件積みうる —— 失わない側の重複で、design D2 の「作り直す側に倒れる」と同じ向き）

## R7. `health_permission_settings_missing` を通知を組み立てるたびに出している
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `healthPermissionPendingIntent` が日数の本文の差し替えのたびにも同じログを出す。Task 6 F4
- kind: technical
- 処置: fixed 6.1 — f27fbe7b（サービスの寿命で 1 度。ただし実機では `resolveActivity` が null を返さないので R12 の不具合は捕まえない）

## R8. 計測テストの判定が緩い・doc が実態と違う
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermissionGrantInstrumentedTest.kt / HealthPermissionDeniedInstrumentedTest.kt
- 根拠: `:104` の `kindsVisible() || additionalAccessVisible()` は種類の画面を見なくても通る（Task 7 F6）。`:168` が鍵の順序まで比べる（F4）。`:31` の doc（各試験が自分で開き直す）が実態と違う（F5）。Denied の doc は実際は 2 回断っている（F2）
- kind: technical
- 処置: fixed 7.1 — 687823f5・7a9bc6b6（Grant のクラス doc に残ったずれは R14）

## R9. 試験の名前 `st09_mig_applies_twice_and_is_last` が中身と食い違う
- 成果物: crates/server/src/media_migration_tests.rs
- 根拠: `:47`。末尾とは限らない版を見ている。Task 1 F2
- kind: technical
- 処置: fixed 1.1 — 2a3fad1c（`st09_mig_applies_twice_and_is_in_order`）

## R10. `day_tests.rs` が収集開始日を無条件で 2002 年に上書きするようになった
- 成果物: crates/server/src/day_tests.rs
- 根拠: `:1447`。Task 1 の範囲の外の変更で、共有 DB を並走する試験との窓が残る（Task 1 F1）。理由はコメントにあり、元へ戻している
- kind: technical
- 処置: rejected: `crates/server/src/day_tests.rs:1447` の上書きは同じ試験の終わりで元へ戻し、理由をコメントに書いている。Task 1 の範囲の外の変更で final reviewer も許容と判定（PR 本文に一言書く）

## R11. 試験の穴（`parseResult`・13 以前の設定の intent・バックアップ規則の節ごと・戻しの移行の `core.event`・例外時の生存信号）
- 成果物: collector-android/app/src/test/ / crates/server/src/
- 根拠: Task 3 F1・Task 6 F2・Task 2 F1・Task 1 F3・Task 6 F3。final reviewer はどれも許容と判定
- kind: technical
- 処置: rejected: final reviewer が許容と判定した試験の穴で、振る舞いは他が覆う —— `parseResult` は計測テスト Grant t1〜t3（`collector-android/app/build/outputs/androidTest-results-st11-HealthPermissionGrantInstrumentedTest/`）、13 以前の intent は端末が無く Q11 の family に載せた、バックアップ規則と戻しの移行は `./collector-android/gradlew -p collector-android :app:testDebugUnitTest`（rc=0・684 本）と `cargo test -p ashiato-server st09_mig`（rc=0）の既存の試験

## scoped re-review（1359e5d..7a9bc6b）

R1〜R9 は ADDRESSED（R4 は走らせたうえで Grant t4 が失敗のまま）。新たな問題は Minor 3 件（うち 1 件は R6 の処置に書いた重複）。
計測テストを走らせて分かった前提の崩れを R12 として足す。

## R12. 常駐の通知の「健康データの権限」を押しても、Android 14 以降で権限の画面が開かない（premises P12 の崩れ）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / openspec/changes/st11-health-history-permission/design.md（D8（仮））/ specs/device-collection/spec.md（Scenario「常駐の通知から健康データの権限の画面が開く」）
- 根拠: 計測テスト `HealthPermissionGrantInstrumentedTest` t4 が失敗。logcat に `SecurityException: Permission Denial … requires android.permission.GRANT_RUNTIME_PERMISSIONS`（`.superpowers/sdd/st11-r4/`）。`HealthConnectManager.ACTION_MANAGE_HEALTH_PERMISSIONS` は通常のアプリからは開けない。premises P12 は adb の shell（その権限を持つ）から開いて確かめていた。D8 と Scenario を変えないと直せない
- kind: premise
- 処置: escalated — deep.md Q11・`deep-questions-r2.json`・`docs/briefs/ST11-deep-r2.html`

## R13. Android 14 以降の経路で、求めが中断されても「1 度求めた」印が立つ
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt
- 根拠: e2ef8442 の `onRequestPermissionsResult(HEALTH_REQUEST)` は無条件に `ASKED_HEALTH_PERMISSIONS` を立てる。framework は求めが中断されたとき空の配列でこの callback を呼ぶので、答えていないのに印が立ちうる。design D4「答えずに終わると次の起動でもう一度求める」に反する。中断の型は計測テストでも Robolectric でも確かめていない（scoped re-review の Minor）
- kind: technical
- 処置: fixed 5.1 — 空の配列で返ったら（中断）印を立てず、この起動は求めずに先へ進む（`onHealthRequestReturned(answered = permissions.isNotEmpty())`）。試験 `HealthPermissionFlowTest`「Android 14 以降で求めが中断されて空で返ると…」。壊して落ちることを確かめた

## R14. `HealthPermissionGrantInstrumentedTest` のクラス doc が t4・t5 の実態と違う
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermissionGrantInstrumentedTest.kt
- 根拠: doc は「各試験は印を消して開き直し、求める画面を自分で出して答えまで進める」だが、そうするのは t1〜t3 だけ（t4 は通知から開くだけ、t5 は未送信を読むだけ）。scoped re-review の Minor
- kind: technical
- 処置: fixed 7.1 — クラス doc を「求める画面を使う t1〜t3 は…、t4 は通知から設定をたどるだけ、t5 は未送信を読むだけ」に直した

## code_verify（HEAD `65c78b32`。作業ツリーは差分なし）

席: code_verify（`final` の後・`finish` の前）。手 1〜7 を全部。壊した定数・足した一時テストはすべて戻し、毎回 `git status --porcelain` と `git diff` が壊す前と同じであることを `diff` で確かめた（`restored-identical`）。

**結論: 今の HEAD は finish を通せない。** 本人が答えた Q11 がコードにも design にも spec にも入っていない（R15）、`:app:detekt` が rc=1（R16）、
計測テスト `HealthPermissionGrantInstrumentedTest` の t4 が HEAD と同じコードで赤（R17）。どれも merge を止める。

### 申告と実測

申告: tasks.md の `[x]` は **13/13**（1.1 1.2 2.1 3.1 4.1 4.2 5.1 6.1 7.1 7.2 8.1 9.1 9.2）。実測は HEAD でもう一度走らせた。

| 項目 | 検証コマンド（tasks.md の本文） | 実測 | 一致 |
|---|---|---|---|
| 1.1 | `tools/check-migrations.sh` / `tools/check-immutable.sh` | rc=0 / rc=0 | 一致 |
| 1.1 | `CT st11_`（`-- --list` で 7 本以上） | `-- --list` 7 本・7 passed・rc=0 | 一致 |
| 1.1 | `CT export_`（1 本以上） | `-- --list` 159 本・159 passed・rc=0 | 一致 |
| 1.1 | `cargo test -p ashiato-server`（全体） | **1393 passed / 0 failed・rc=0**（588 秒。`.superpowers/sdd/logs/20261027-163513-ct.log`） | 一致 |
| 1.2 | `grep -q` 2 本 | rc=0 | 一致 |
| 2.1 | `GT HealthManifestTest`（4 本以上） | 4 本・失敗 0 | 一致 |
| 2.1 | `! git grep -n "永久に読めない"` | rc=0 | 一致 |
| **2.1** | `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:detekt` | **rc=1**（`Analysis failed with 240 weighted issues`。→ R16） | **不一致** |
| 3.1 | `GT HealthPermissionsTest`（6 本以上）/ `:app:assembleDebug` | 8 本 / rc=0 | 一致 |
| 4.1・4.2 | `GT HealthPermissionRecordTest`（9 本以上）/ `GT ClockSkewPayloadShapeTest` / `check-log-private.sh` | 18 本 / 3 本 / rc=0 | 一致 |
| 4.2・5.1・7.2 | `GT SourceIndependenceTest`（11 本以上）ほか `MainActivityTest` `MediaPermissionTest` `UnusedAppRestrictionsTest` | 11 / 13 / 5 / 11、失敗 0 | 一致 |
| 5.1 | `GT HealthPermissionFlowTest`（18 本以上） | 20 本・失敗 0 | 一致 |
| 6.1 | `GT HealthNotificationTest`（4 本以上）/ `LocationServiceTest` / `LocationServiceClockTest` | 6 / 26 / 12、失敗 0（ただし 1 本目が守る値は覆った決定の側。→ R15） | 一致 |
| （全体） | `:app:assembleDebug :app:testDebugUnitTest` | rc=0・684 本・失敗 0 | 一致 |
| **7.1** | `AT HealthPermissionGrantInstrumentedTest`（5 本以上）ほか 2 クラス（`scripts/quiet-run instr -- tools/android-emulator.sh` rc=0） | 台本は通しで走ったことが無い（`androidTest-results-stage*` が 0 件・evidence.jsonl に AT の行が 0 件）。HEAD と同じコードの走行（final review R4、7a9bc6b6 の後 17:14）の XML: Grant **5 本中 1 失敗**（t4）・HistoryDenied 1/0・Denied 1/0。tasks の判定（台本 rc=0 かつ本数）は満たさない（台本そのものは走らせていない。→ R17） | **不一致** |
| 7.1 | `grep -c HealthPermission` が台本と CI に 3 以上 | rc=0 | 一致 |
| 7.2 | `AT PermissionDeniedInstrumentedTest` / `grep -q` | 同じ走行で 1/0 / rc=0 | 一致 |
| 8.1 | `scripts/quiet-run smoke -- tools/smoke.sh` / `check-shared-appends.sh` / `check_scenarios.py` | rc=0（`== ST11. 健康データの権限の記録…` の節を通過。`20261027-164508-smoke.log`） / rc=0 / rc=0（`scenarios: OK`） | 一致 |
| 9.1・9.2 | `grep -q` 3 本 / `check_chain.py` | rc=0 / rc=0（`chain: OK`） | 一致 |
| （change） | `openspec validate st11-health-history-permission --strict` | valid | 一致 |

注（ST11 の指摘にしない）: 最初の 2 回の全体の走行（`20261027-155717-ct.log` 989 passed / 404 failed、`20261027-161156-ct.log` は途中で止めた）は、同じ時間に私が走らせた `tools/check-immutable.sh` が終わりに `docker compose down -v`（check-immutable.sh:4）でこの worktree の DB を消したためで、`permission denied for schema core` で落ちていた。`./tools/db.sh up -d --wait db && ./tools/db-roles.sh` で立て直して単独で走らせたのが上の 1393 本。縦串も DB を作り直すので、終わった後に同じ手順で DB を立て直した

### 手 1: 固定値を独立に再計算する

- `HealthPermissionRecordTest.欄の名前と並びは設計の形のまま` の期待の原文を、design.md D2 の JSON の例から python の `json.loads` → `json.dumps(separators=(',',':'))`（`device_time` だけ試験の値に差し替え）で作り直した → **一致**（`design==test: True`）。
  縦串（`tools/smoke.d/202610251234_st11-health-permission.sh:8`）の `payload` もキーの並び・`granted` の並びとも D2 と同じ
- `HEALTH_PERMISSIONS` の 6 つの権限の名前を、`connect-client-1.1.0.aar` の `androidx/health/connect/client/permission/HealthPermission*.class` の文字列と突き合わせた → 6 つとも**一致**
  （`READ_SLEEP` `READ_HEART_RATE` `READ_STEPS` `READ_EXERCISE` `READ_HEALTH_DATA_HISTORY` `READ_HEALTH_DATA_IN_BACKGROUND`）

### 手 2: ガードをわざと壊す

- `HealthPermissionRecord.kt:179` のログに `error = state.granted.toString()`（種類ごとの許可の値）を足した → `bash tools/check-log-private.sh` は **rc=0**（`check-log-private: OK`）。
  同じ書き換えで `GT HealthPermissionRecordTest` は **rc=1**（`ログに記録したことと契機が残り、許可の値と権限の名前は出ない FAILED`）。
  ST09 R18 の `check_telemetry_args` はこの branch に入っているが、語の一覧に `granted` / 権限の語が無い。振る舞いは Robolectric の試験が守るので指摘にしない（ログの出口は 1 か所）
- 「健康データを読まない」（`ReadRecordsRequest` を呼ばない。tasks の Global Constraints）を機械で守るものは無い（`git grep -n "ReadRecordsRequest\|readRecords"` は `build.gradle.kts:168` のコメントだけ）。spec の Scenario ではないので指摘にしない
- 移行の行は `CT st11_registry_migration_applies_twice_with_the_designed_row` が `expected_gap_sec = 21600`・`external_id_kind = 'none'`・`outage_target = false` を直書きで比べている

### 手 3: Scenario と test

`python3 scripts/check_scenarios.py . st11-health-history-permission` → `scenarios: OK` rc=0（warn は他の道具の正規表現を印と読んだもので ST11 と無関係）。

- 「常駐の通知から健康データの権限の画面が開く」: GT の印（`HealthNotificationTest.kt:73-87`）が見るのは `PendingIntent` の action が `android.health.connect.action.MANAGE_HEALTH_PERMISSIONS` であること（`:81` 直書き）。THEN の「画面が開く」を観測する AT t4 は赤。GT が固定している値は Q11 で本人が退けた側 → **R15**
- 「許すと許した状態と観た時刻の記録が S-01 に格納される」: サーバの側は縦串が `payload->'granted' = '{…}'::jsonb`（6 つ全部 `true`）と `event_time = '$HEALTH_TIME'` を見る（`smoke.d/…st11…sh:22,24`）。端末の側は AT t5（緑）。階層は合っている
- 「Health Connect が使えるかを読めない端末では求めずに収集が始まる」: 印の先は読む口が**例外を投げる**場合だけ。読む口が**返らない**場合は収集が始まらない → **R20**

### 手 4: 本人の決定が test で固定されているか

tasks の Global Constraints が「定数にして試験が名指しで固定する」と挙げた値と、本人の答え（Q3）を書き換えて `:app:testDebugUnitTest`（684 本）を走らせた:

| 値 | 書き換え | 結果 |
|---|---|---|
| `HEALTH_LOGICAL_SOURCE = "c01-health-permission"`（D1） | `"c01-health-perm"` | 落ちた（`HealthPermissionRecordTest` のログの試験が文字列で比べる。S-01 側は `CT st11_` が直書き） |
| `HEALTH_PREFS` / `HEALTH_RATIONALE` | （書き換えず grep） | `HealthManifestTest` がバックアップの規則の `path` と文の語を 1 つずつ見る |
| Q9（背景も求める）・D2 の欄 | （書き換えず） | `欄の名前と並びは設計の形のまま` が `granted` の 6 キーを直書きで比べる |
| **`ASKED_HEALTH_PERMISSIONS = "asked_health_permissions"`**（D4） | `"asked_health_permissions_v2"` | **落ちない** → R18 |
| **`HEALTH_PERMISSION_ACTION_LABEL = "健康データの権限"`**（D8） | `"健康データ"` | **落ちない** → R18 |
| **`HEALTH_READ_TIMEOUT_MS = 30_000L`**（final review R3 の直し） | `60_000L` | **落ちない** → R18 |
| **Q3 の 4 種類**（`HEALTH_PERMISSIONS` の `exercise`） | `READ_EXERCISE` → `READ_WEIGHT`（定数と manifest の両方） | **落ちない**（rc=0・684 本）。AT t2 の `KIND_LABELS`（`HealthConnectScreens.kt:182`）だけが捕まえる → R18 |

上の 4 つの書き換えのうち最初の 3 つと `HEALTH_LOGICAL_SOURCE` は同じ走行に入れた（失敗は `HEALTH_LOGICAL_SOURCE` の 1 本だけ。`.superpowers/sdd/logs/20261027-160254-unit.log`）。Q3 は別の走行（`20261027-160431-unit.log`）。

### 手 5: tasks の `[x]` と実体

- **2.1**: 検証の `:app:detekt` が HEAD で rc=1 → R16
- **7.1**: 検証の `tools/android-emulator.sh` は通しで走ったことが無く、同じコードの走行で t4 が赤 → R17
- `cargo test --test <名前>` 型の検証は無い。ほかの項目は上の表のとおり実在し rc=0

### 手 6: 隙間

- **未送信から健康データの記録が捨てられたとき**（90 日の保持・2 GB の上限。`Retention.kt:64-68`）→ **R19**（捨てたものが作り直されない型）
- **画面の側の Health Connect の読みが返らないとき** → **R20**（収集が始まらない）
- 受け渡し口（`HealthRecordHandoff`）はメモリだけだが、最後の状態の書き換えは未送信に書けた後（`MainActivity.kt:298-304`）なので、プロセスが落ちても次に観たとき作り直す。`start()` は起動のたびに `startForegroundService` を呼ぶ（`MainActivity.kt:337`）ので `onStartCommand` の受け渡しは毎回走る。指摘にしない
- `Outbox.add` は置き場に書けなければ `false` を返す（`Outbox.kt:45-63`）ので、`MAX_UNWRITTEN` の `lost` の経路では最後の状態は書き換わらない。指摘にしない

### 手 7: 機械が承認した検証コマンドの訂正

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st11-health-history-permission` → `[]`）。

## R15. 本人が答えた Q11（Health Connect の設定のトップを開く）が、コード・design D8・spec・試験のどれにも入っていない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthPermissions.kt / openspec/changes/st11-health-history-permission/design.md / openspec/changes/st11-health-history-permission/specs/device-collection/spec.md / collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthNotificationTest.kt
- 根拠: `deep-answers-2.txt` の答えは「Health Connect の設定のトップを開き、そこから『アプリの権限 → ashiato』へ本人がたどる」。記録した 65c78b32 は `git show --stat` で `deep-answers-2.txt` と `deep.md` だけ。
  `git grep -n "設定のトップ\|HEALTH_HOME\|ACTION_HEALTH_CONNECT_SETTINGS" -- collector-android openspec/changes/st11-*` は premises.md:28（P9 の手順）の 1 件だけ。
  HEAD の `healthPermissionSettingsIntent`（HealthPermissions.kt:50-54）は 14 以降で `MANAGE_HEALTH_PERMISSIONS` のまま、design D8（design.md:184）は「P12 で開くと確かめた」のまま、spec の Scenario（spec.md:86）は「収集アプリの Health Connect の権限の画面が開く」のまま。
  `GT HealthNotificationTest` は緑（6 本）だが、`:81` が固定しているのは退けた側の action。THEN を観測する AT t4 は同じコードの走行で
  `AssertionError: 常駐の通知の「健康データの権限」を押しても権限の画面が開かない（前面: dev.ashiato.collector）`。本人の端末は Android 16（Q2）で、通知の操作は押しても何も開かない
- kind: technical
- 提案: Q11 の答えで design D8（と P12 の扱い）・spec の Scenario の THEN（「Health Connect の設定が開く」）を書き直し、14 以降の intent を設定のトップ（`HealthConnectClient.ACTION_HEALTH_CONNECT_SETTINGS`）に変え、
  `HealthNotificationTest:81` の期待と AT t4 の判定をそれに合わせる。本人の答えは出ているので人間へは戻さない
- 処置: fixed D8 — Q11 の答えで 14 以降の intent を `HealthConnectClient.ACTION_HEALTH_CONNECT_SETTINGS`（14 以降の値は `android.health.connect.action.HEALTH_HOME_SETTINGS`。connect-client 1.1.0 の bytecode で確かめた）に変え、design D8・spec の Requirement の文と Scenario の THEN（設定のトップが開き「アプリの権限 → 収集アプリ」とたどると権限の画面）を書き直した。`HealthNotificationTest` の期待を新しい action に、AT t4 を「設定のトップが開く → アプリの権限 → あしあと。→ 4 つの種類が並ぶ」に直した（Android 15 の AVD の本物の Health Connect で緑。R17）。Scenario の名前は変えていない（tasks.md の印を保つ）

## R16. `:app:detekt` が HEAD で rc=1（240 件。どれも ST11 が触ったファイル）で、2.1 の `[x]` の検証が成り立たず、CI の android job も落ちる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthPermissionRecord.kt / LocationService.kt / MainActivity.kt / Telemetry.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/Health*.kt / collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/Health*.kt
- 根拠: `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:detekt` → **rc=1**、`Analysis failed with 240 weighted issues`（`.superpowers/sdd/logs/20261027-160219-unit.log`）。
  違反の行が多いのは `HealthPermissionRecordTest.kt` 72・`LocationService.kt` 54・`MainActivity.kt` 40・`HealthNotificationTest.kt` 19（`LongParameterList` `LargeClass` `ImportOrdering` `Wrapping` ほか）。違反の出たファイルはどれも 934d18dc..HEAD で変わっている。
  `collector-android/config/detekt-baseline.xml` は ST11 で触っていない（`git log 934d18dc..HEAD -- …baseline.xml` が空）。evidence.jsonl の detekt は 2.1 の 1dcea7d0 の 1 回（PASS）だけで、その後の Task 3〜7 と final review の fix では一度も走っていない。
  `.github/workflows/ci.yml:207` が `./gradlew detekt` を走らせる
- kind: technical
- 提案: 違反を直す（`LongParameterList` は引数をまとめる、`LargeClass` は健康データの見直しと通知の操作を `LocationService` の外へ出す）。baseline に足して凍結するなら、その理由を PR の本文に書く
- 処置: fixed 2.1 — `:app:detekt` rc=0。整形は detekt-formatting の自動修正を ST11 が触ったファイルだけに当て（他のファイルの書き換えは戻した）、残りを手で直した（`drainInto` の入れ子を `attempt` に、`observeHealthPermissions` の return を `recordIfChanged` に、`HealthTestSupport.kt` を `FakeHealthPermissions.kt` に改名）。構造の 4 件（`LongParameterList` の `Telemetry.line`・`healthPermissionRecord`・`HealthPermissionRecorder`、`LargeClass` の `LocationService`）は `detektBaseline` で凍結した —— 引数を束ねる・`LocationService` を割るのは既存の同じ型（baseline の `LongParameterList` 10 件）と一緒に直す作り直しの change の範囲で、ST11 で割ると差分が振る舞いと関係なく膨らむ。PR 本文に書く

## R17. 7.1 は `[x]` だが、検証の `tools/android-emulator.sh` は通しで走ったことが無く、HEAD と同じコードで `HealthPermissionGrantInstrumentedTest` の t4 が赤のまま
- 成果物: openspec/changes/st11-health-history-permission/tasks.md / tools/android-emulator.sh / collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermissionGrantInstrumentedTest.kt
- 根拠: `ls collector-android/app/build/outputs/androidTest-results-stage*` → 無い（台本が段ごとに残す置き場）。evidence.jsonl の 78 行に `android-emulator.sh` の行は 0。
  final review R4 の手の走行（`androidTest-results-st11-*`。7a9bc6b6 の commit 17:10 の後の 17:14。その後の commit は文書だけで `git diff --stat 7a9bc6b6 HEAD -- collector-android tools/android-emulator.sh .github` は空）:
  Grant `tests="5" failures="1"`（t4）・HistoryDenied 1/0・Denied 1/0・PermissionDenied 1/0・ほか 6/0。tasks の判定（台本 rc=0 かつ本数）は満たさない。直すのは R15 と同じ。
  同じ走行に `MediaCaptureInstrumentedTest` と `RetentionInstrumentedTest` は入っていない（XML の `classname=` で 0 件）。tasks の Global Constraints が「緑のまま」と名指しする `MediaCaptureInstrumentedTest` は HEAD のコードで一度も走っていない。
  `tools/android-emulator.sh` は今回も走らせていない（同じコードの走行の XML で判定した）
- kind: technical
- 提案: R15 を直した後に `scripts/quiet-run instr -- tools/android-emulator.sh` を通しで走らせ（`MediaCaptureInstrumentedTest`・`RetentionInstrumentedTest` を含む）、rc と段ごとの本数を evidence に残してから `[x]` にする
- 処置: fixed 7.1 — R15 を直した後に `scripts/quiet-run instr -- tools/android-emulator.sh` を通しで走らせて **rc=0・1022 秒**。段ごとの XML（`androidTest-results-stage1〜6`）を `classname=` で数えて 1 段目 18 本（`MediaCaptureInstrumentedTest` 2・`RetentionInstrumentedTest` 3 を含む）・2 段目 2 本（`PermissionDeniedInstrumentedTest` 1）・3 段目 3 本・4 段目 `HealthPermissionGrantInstrumentedTest` 5 本・5 段目 `HealthPermissionHistoryDeniedInstrumentedTest` 1 本・6 段目 `HealthPermissionDeniedInstrumentedTest` 1 本、失敗 0。1 回目の通しでは t4 が「操作が見つからない」で落ちた —— 新しい AVD では OS の知らせ「画面ロックを設定」が上に展開され、常駐の通知（重要度 低）は「Silent」の節で畳まれて操作が見えない（操作そのものは通知に載っていた）。t4 は常駐の通知のほかを消し、見えなければその行の展開ボタンを押して操作を押す（`HealthConnectScreens.clickOngoingAction`）

## R18. 本人の決定と仮決めの値のうち 4 つは、書き換えても単体テストが全部通る（Q3 の種類は計測テストだけが見る）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthPermissions.kt / LocationService.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/
- 根拠: `ASKED_HEALTH_PERMISSIONS` を `"asked_health_permissions_v2"`、`HEALTH_PERMISSION_ACTION_LABEL` を `"健康データ"`、`HEALTH_READ_TIMEOUT_MS` を `60_000L` にして `:app:testDebugUnitTest` → 684 本中の失敗は同時に入れた `HEALTH_LOGICAL_SOURCE` の 1 本だけ（`20261027-160254-unit.log`）。
  `HEALTH_PERMISSIONS` の `exercise` を `READ_WEIGHT` にし manifest:60 も同じにして → **rc=0・684 本全部緑**（`20261027-160431-unit.log`）。
  試験は定数を参照して比べる（`grep -l '"asked_health_permissions"\|"健康データの権限"\|READ_SLEEP\|READ_HEALTH_DATA_IN_BACKGROUND' app/src/test` は 0 件）。
  tasks の Global Constraints は `ASKED_HEALTH_PERMISSIONS` と `HEALTH_PERMISSION_ACTION_LABEL` を「試験が名指しで固定する」に挙げる。印の鍵が変わると、更新した端末で答え済みの本人にもう一度自動で求める（D4 に反する）。
  Q3 の種類は AT t2 の `KIND_LABELS` だけが見るが、tasks は「AT だけで閉じない」
- kind: technical
- 提案: `assertEquals("asked_health_permissions", ASKED_HEALTH_PERMISSIONS)`・`assertEquals("健康データの権限", HEALTH_PERMISSION_ACTION_LABEL)`・`assertEquals(30_000L, LocationService.HEALTH_READ_TIMEOUT_MS)` と、
  `HEALTH_PERMISSIONS` の 6 つの権限の名前を直書きで比べる 1 本を `HealthPermissionsTest` に足す
- 処置: fixed 3.1 — `HealthPermissionsTest`「決めた値は直書きで固定されている」（6 つの権限の名前と順・`asked_health_permissions`・`健康データの権限`・`30_000L`）

## R19. 未送信から健康データの権限の記録が捨てられても最後の状態は「記録した」のまま残り、もう一度観ても作り直さない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthPermissionRecord.kt / Retention.kt
- 根拠: 一時テスト（`HealthPermissionRecorder` を本物の `HealthStateStore` と `testOutbox()` で組み、`request_result` で 1 件積む → `Retention.kt:65` と同じ `outbox.dropHead({ true }, { true }, {})` で捨てる →
  同じ recorder で `periodic`、新しい recorder（起動の識別を変えた）で `app_open` を観る）を `--tests '*ZzProbeHealthDropTest*'` で走らせて削除した →
  `ZZPROBE before=1 dropped=1 after_reobserve=0 last_state={"availability":"available",…,"granted":{…全部 true},"auto_requested":true}`（`.superpowers/sdd/logs/20261027-160559-unit.log`）。
  `Retention.kt` `DropReport.kt` `Outbox.kt` `SegmentStore.kt` は `HEALTH_PREFS` / `last_state` に触れない（grep 0 件）。
  90 日送れない・2 GB を超えると記録は ST04 の破棄の報告に数として残るが、S-01 は許可の状態を、次に状態が変わるまで（変わらなければ永久に）受け取らない。
  ST34 は最初に種類が許された記録の `device_time` から読める範囲の起点を出す（`docs/handoff/ST34.md`・P13）。状態は観直せば分かるのに作り直されない
- kind: irreversible
- 提案: 保持の上限で `c01-health-permission` の記録を捨てたら、`HEALTH_PREFS` の `last_state` を消す（次に観たとき必ず 1 件作る）。観た時刻が後ろへずれることは handoff の ST34 の節に書く
- 処置: fixed D15 仮 — 保持の上限の宛先（`onRecordsDropped`）で `c01-health-permission` を捨てたら `HealthStateStore.forget()`。試験 `HealthNotificationTest`「保持の上限で…次の見直しで作り直す」（壊して落ちることを確かめた）。`provisional.py add … --id D15 --source review/code.md#R19`

## R20. 画面の側の Health Connect の読みには時間の上限が無く、読みが返らない端末では初回起動で収集が始まらない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt
- 根拠: `healthRequestIntent()`（MainActivity.kt:223-236）は `runBlocking` で `observeHealthPermissions(APP_OPEN)` と `health.read` を呼び、終わるまで `inFlight` で `start()` へ進まない。
  一時テスト（`ExemptMainActivity` を継ぎ、`runHealth` を別の糸に、`read` を `awaitCancellation()` にした入口で開き、主 looper を 5 秒回して `nextStartedService` を見る）を走らせて削除した →
  `ZZPROBE hang(read never returns): started_service=null` / 対照（read が返る）`started_service=dev.ashiato.collector.LocationService`（`20261027-160724-unit.log`）。
  サービスの側は final review R3 で 30 秒の `withTimeout` を付けたが、画面の側には無い。spec「Health Connect が使えるかを読めない端末では求めずに収集が始まる」の試験は読む口が例外を投げる場合だけを見る
- kind: technical
- 提案: 画面の読み（`app_open` の観測と `health.read`）も `withTimeout(HEALTH_READ_TIMEOUT_MS)` で包み、時間切れは `unreadable` に倒して先へ進む。返らない読みでも `LocationService` の起動の intent が出ることを見る試験を `HealthPermissionFlowTest` に足す
- 処置: fixed 5.1 — 画面の読み（`app_open` の観測と読み、`request_result` の観測）を `withTimeout(healthReadTimeoutMs())`（サービスと同じ 30 秒）で包み、時間切れは求めずに先へ進む。試験 `HealthPermissionFlowTest`「Health Connect の読みが返らなくても、上限で諦めて求めずに収集が始まる」

## security review（934d18d..65c78b3）

見た経路:
- 端末 → `POST /ingest`（`c01-health-permission`）→ `core.event`。サーバの差分は `export/excluded.rs` の 2 表と `lib.rs` の試験の mod だけで、router（`lib.rs:3483` 以下）に新しい入口は無い（`git diff --stat 934d18d..HEAD -- crates/server/src ':!*tests.rs'` が 2 ファイル 7 行）（4: 下の台本）
- 書き出し: `NO_COORD_SOURCES` に `c01-health-permission` を足した。端末が作る payload の欄は `HealthPermissionRecord.kt` の `healthPermissionRecord`（`kind` `trigger` `device_time` `elapsed_ms` `boot_count` + `HealthState.fields()` の 4 欄 + 例外の型名の `error`）で固定され、座標も自由文も入らない（3）
- 外から開ける新しい入口: `HealthRationaleActivity`（`exported="true"`・intent-filter に権限なし）は固定の文 `HEALTH_RATIONALE` を出すだけで `intent` も extras も読まない（`HealthRationaleActivity.kt:16-30`）。14 以降の alias は `START_VIEW_PERMISSION_USAGE` で守られる（manifest の merge 後 `processDebugMainManifest/AndroidManifest.xml:131-141`）（3）
- 依存が足した入口: `connect-client:1.1.0` の AAR が `HealthDataSdkService`（`exported="true"`・権限なし）を merge する。bytecode（`HealthDataSdkServiceStubImpl.verifyPackageName`）は呼び手の uid が `com.google.android.apps.healthdata` を名乗ることを 3 つの口すべてで確かめ、扱うのは権限の token と前景かだけで記録には触れない（2: ライブラリの javap）
- 通知の操作: `healthPermissionPendingIntent`（`LocationService.kt:698-707`）は `FLAG_IMMUTABLE`。載せるのは自分の package 名だけで、記録の中身は渡さない（3）
- ログ: 足した語は `Telemetry.line` の `trigger` / `availability`（自分で名付けた語の一覧）と `error`（例外の `simpleName`）だけ。種類ごとの許可の値を渡す口は無い。`check-log-private.sh` は `granted` を見ないが、出口が 1 か所で `HealthPermissionRecordTest` が落とすことを code_verify の手 2 が壊して確かめている（3）
- バックアップ: `health_permission.xml` だけを cloud-backup / device-transfer から外した。記録そのものは未送信（`filesDir/outbox/records`。`LocationService.kt:254,268,282-285`）に積まれ、既定のまま自動バックアップに入る（3。design D14 が範囲の外と書いた既存の型。下の「範囲外」）
- 依存: `androidx.health.connect:connect-client:1.1.0`（版の固定・Google の group）と `com.google.guava:guava:31.1-android` の `compileOnly`（apk に入らない）。取り違えの名前ではない（2）
- 縦串 `tools/smoke.d/202610251234_st11-health-permission.sh`: 固定の偽の UUID と偽の payload。秘密の固定値は無い（token は既存の `post` が持つ）（3）

安全の決め手: サーバに新しい入口は無く、`c01-health-permission` の記録も既存の `/ingest` の token 検査を通る —— 確かさ 4。
`health_permission_tests.rs` の末尾に一時の試験（router を `oneshot` で通し、token なし・違う token・正しい token の 3 通りで `permission_record` を送る）を足して
`scripts/quiet-run ct -- cargo test -p ashiato-server zzsec_st11 -- --nocapture` → `ZZSEC none status=401 Unauthorized rows_for_user=0` / `ZZSEC wrong status=401 Unauthorized rows_for_user=0` / `ZZSEC right status=200 OK rows_for_user=1`（`.superpowers/sdd/logs/20261027-165052-ct.log`）。
試験は消して元に戻した（`git status --porcelain` は `review/code.md` だけ）

指摘なし

範囲外（R 番号なし）: 未送信の置き場（`filesDir/outbox`）は既定の自動バックアップに入り、ST11 からは健康データの権限の状態もそこへ載る。入れ直した後の戻しで、削除した記録がもう一度送られうる型も含めて全ソース共通の既存の穴（design D14・`docs/briefs/ST09-pr.md:60`）。`allowBackup` を決める change で扱う

## /code-review（934d18d..65c78b3）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R21. 本人が答えた Q11（Android 14 以降は Health Connect の設定のトップを開く）が実装されておらず、常駐の通知の操作は今も ACTION_MANAGE_HEALTH_PERMISSIONS + EXTRA_PACKAGE_NAME を開く（R12/R15 が未処置）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthPermissions.kt:50
- 根拠: Android 14 以降の端末で常駐の通知の「健康データの権限」を押す → 権限の画面が開かない（R12 の実測）。本人がたどる道が無いまま（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D8 — R15 と同じ直し

## R22. drainInto の failed は 1 回の drain の中だけの局所変数。Record と、その後の afterDrain の Action が別々の drain に分かれると、記録が未送信に書けなくても手続き（store.remember）が走る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthRecordHandoff.kt:32
- 根拠: recorder が offer(Record S1) した直後・afterDrain の前に、収集の糸の drain が [Record S1] だけを取る。outbox.add が false（ディスクが一杯）で S1 はメモリにだけ残る。次の drain は [Action] だけを取り、failed=false なので最後の状態を S1 に書き換える。この後プロセスが終わると S1 は消え、置き場が S1 なので二度と作り直されない（変化が S-01 に届かない）（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.1 — 「書けなかった」を drain をまたいで持つ（`failedSinceAction`）。再現: 記録だけの drain → 手続きだけの drain で手続きが走った（直す前）。試験 `HealthPermissionRecordTest`「記録と手続きが別々の drain に分かれても…」（壊して落ちることを確かめた）

## R23. runCollection { HealthRecordHandoff.drainInto(outbox) } が包まれていない。drainInto は先に queue を空にするので、outbox.add や afterAdd（保持の上限の見回り）が投げると、残りの記録と手続きが失われ、例外は収集の糸の外へ抜ける
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:781
- 根拠: store.append や afterAdd が実行時例外を投げる → ExecutorService の糸の未捕捉例外で Android がプロセスを落とす。START_STICKY で立て直されても受け渡し口はメモリなので、画面が預けた記録は消える。heartbeat_crashed や health_permission_crashed のように種別だけ残して続ける作りと食い違う（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.1 — `drainInto` は積むところ・手続きが投げても残りを続け、投げたものを `onError`（サービスは `health_permission_crashed`）へ渡す。投げた記録は書けなかったものとして手続きを捨てる。試験「積むところが投げても drain は投げず、残りの記録と手続きを続ける」

## R24. 画面の recorder の控え（pending）は Activity のインスタンスごと。置き場の書き換えはサービスの drain まで遅れるので、Activity が作り直されると同じ状態をもう一度受け渡し口へ積む
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HealthPermissionRecord.kt:166
- 根拠: app_open で S1 を預ける（pending=S1、置き場は S0 のまま）。Health Connect の画面を出している間に Activity が破棄されて作り直される（回転・メモリ不足・開発者向けオプションの「アクティビティを保持しない」）。新しい recorder は pending=null で置き場は S0 なので、request_result で状態が同じ S1 でも別の id でもう 1 件積む → S-01 に同じ状態の記録が 2 件入る（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.1 — 画面の recorder の控えをプロセスで 1 つの `HealthRecordHandoff.screenPending`（`HealthPendingSlot`。錠も兼ねる）に置く。試験 `HealthPermissionFlowTest`「Activity が作り直されても…重ねて預けない」（控えを recorder ごとに戻すと落ちることを確かめた）

## R25. 画面側の healthRequestIntent は runBlocking で observe と health.read を時間の上限なしに待つ。終わるまで inFlight で start() へ進まない（R20。サービス側にだけ 30 秒の withTimeout がある）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:223
- 根拠: Health Connect の IPC が返らない端末で初回起動 → LocationService が一度も始まらず、位置もアプリ利用も収集されない（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.1 — R20 と同じ直し

## R26. Android 14 以降の framework の口（requestPermissions）では、求めが中断されて permissions と grantResults が空で返っても、onHealthRequestReturned が無条件で ASKED_HEALTH_PERMISSIONS を立てる（R13）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:269
- 根拠: 権限の画面を出している最中に別の画面が割り込み、求めが中断される → 本人は答えていないのに印が立ち、以後の起動で自動では二度と求めない（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.1 — R13 と同じ直し

## R27. setHealthAvailable の通知の組み直しと RetentionNotifier の ongoing(text) が別の糸から同じ NOTIFICATION_ID へ notify する。healthAvailable を読んで組み立てた古い通知が後から届くと、操作の出し入れが逆戻りし、変化したときにしか組み直さないので戻らない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:1040
- 根拠: 見回りの糸が healthAvailable=false の時点で通知を組み立てる。同時に収集の糸が true にして操作つきの通知を出す。その後に見回りの糸の notify が届いて操作が消え、Health Connect が使える端末でも次に使えるかが変わるまで操作が出ない（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 6.1 — 常駐の通知の組み立てと notify を 1 つの錠（`ongoingNotificationLock`）の中で行う（`setHealthAvailable` と `AndroidRetentionAlerts.ongoing`）。糸の前後は Robolectric で再現できないので試験は足していない（錠の範囲で読んで確かめた）

## R28. 健康データの段を 1 回通るたびに Health Connect を 2 回読む（observeHealthPermissions の中の read と、直後の health.read）。サービス側の refreshHealthAvailability も、起動のたびに使えるかを知るためだけに対応 2 つと許可の一覧まで読む
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:229
- 根拠: 初回起動ごとに IPC が 2 往復（対応 ×2 と許可の一覧で、計 6 回の呼び出し）。2 回の読みの間に状態が変わると、記録と求めるかの判断が別の状態に基づく。1 回の読みを observe に渡す（または observe が HealthReading を返す）形にでき、サービス側は sdkStatus だけで足りる（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.1 — `observeHealthPermissions` が読んだ `HealthReading` を返し、画面は同じ読みで求めるかを決める（起動の最初の通過で読みは 1 回）。サービスの起動時の `refreshHealthAvailability` は残す —— 起動ごとに 1 回で、使えるかだけを読む口を `HealthPermissions` に足すほどの費用ではない

## R29. runCatching { runCollection { ... } } の外側の包みは冗長。ExecutorFlushScheduler.every は既に task を runCatching で包んでおり、この行は刻みの最後なので、投げても生存信号（先に済んだ emitter.emit()）は止まらない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:1009
- 根拠: コメントが「投げると刻みそのもの（生存信号）が止まる」と実態と違う理由を述べ、守りが二重になる。保守する人が既定の scheduler の守りを信用できなくなる（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 6.1 — コメントを実態に合わせた（既定の刻みも受けるが `tick_crashed` になる。健康データの見直しの失敗として種別を残すためにここで受ける）。包みは残す

## R30. onHealthRequestReturned が主スレッドで SharedPreferences の commit()（同期のディスク書き込み）をする。読みは runHealth へ逃がしているのに、書きだけが主スレッドに残っている
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:269
- 根拠: ストレージが遅い端末で権限の画面から戻るたびに主スレッドが塞がる（StrictMode の DiskWrite 違反、画面が応答なしに近づく）。runHealth の中で書くか apply() にする（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.1 — 印の書き込みを `runHealth` の糸へ移した（観る前に書く順は保つ）

## scoped re-review（finish の fix。HEAD `65c78b32` からの差分）

R13〜R30 はすべて ADDRESSED。Critical 0 / Important 0 / Minor 4。錠（`HealthPendingSlot`・`HealthRecordHandoff`・`ongoingNotificationLock`）の循環は無く、
`withTimeout` は `runBlocking` の中で効く（`ClientHealthPermissions.read` は `CancellationException` を投げ直す）。process-wide の控えは試験の間で漏れない。
Minor 4 件（本番の控えの既定を見る試験が無い・捨てた手続きの控えがプロセスの寿命まで残る・試験の片付けが `failedSinceAction` を戻さない・自動整形の折り返し 3 か所）は
fix 1 回の規律で直さず、Ruling をつけて park した（`docs/briefs/ST11-pr.md` の「仮決め（B / P）と Ruling」）
