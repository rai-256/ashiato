# ST11 実装タスク — 健康データの権限を初回起動で求め、許可の状態を記録する

読む順: `deep.md`（**最優先。本人が決めたこと。いまは答え待ちで推奨の形**）→ このファイル → `specs/device-collection/spec.md` → `design.md` → `premises.md`（Health Connect の実測）
→ `docs/stories/ST11.md` → `docs/handoff/ST11.md`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

触るのは `collector-android/`（manifest・`build.gradle.kts`・`MainActivity.kt`・`LocationService.kt`・新しい `HealthPermissions.kt` / `HealthPermissionRecord.kt` / `HealthRationaleActivity.kt`・試験・`README.md`）・
`collector-android/app/src/main/res/xml/` の自動バックアップの規則 2 つ・`migrations/`（1 本）・`crates/server/src/export/excluded.rs` と `crates/server/src/` の試験・`docs/collector-contract.md`・`docs/handoff/ST34.md`・`docs/handoff/ST38.md`・
`tools/smoke.d/` の新しいファイル・`tools/android-emulator.sh` と `.github/workflows/ci.yml` の android-instrumented（計測テストの段を足す）。
検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` と `tools/db-roles.sh` が前提（`docs/testing.md` §8）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin はコメント（`// Scenario: 状態が変わらなければ記録しない`）、bash は `echo`。
  `scripts/check_scenarios.py . st11-health-history-permission` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change が足す Scenario は 35 本**（ADDED の 6 本の Requirement の 33 本 + MODIFIED の「ソースごとに独立して収集する」に足した 2 本）。**35 本は全部、このファイルのどこかの Task が名指しする。**
  MODIFIED で写した既存の 7 本は既存の印を生かす（名前を変えない）。THEN / AND に主張が 2 つ以上ある Scenario は、試験が主張を 1 つずつ assert する
- **印を置くテストは、その Scenario の THEN の階層を観測する。** 「S-01 に格納」は本物の PostgreSQL まで通すサーバの試験か縦串に、端末の振る舞い（求める・積む・通知）は Robolectric か計測テストに置く。
  端から端の Scenario（許すと S-01 に格納される）は端末と縦串の両方に印を置く（Task 8）
- **件数つき検証**: このファイルで
  **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
  `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` が、その項目に書いた本数（`-- --list で N 本以上`）以上、
  **`GT <クラス>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*<クラス>*'` rc=0 かつ
  `grep -ho 'tests="[0-9]*"' collector-android/app/build/test-results/testDebugUnitTest/TEST-*<クラス>*.xml | tr -dc '0-9\n' | awk '{s+=$1} END{print s+0}'` が
  その項目に書いた本数（`N 本以上`。書いていなければ 1）以上、
  **`AT <クラス>`** は `scripts/quiet-run instr -- tools/android-emulator.sh` rc=0 かつ
  `cat collector-android/app/build/outputs/androidTest-results-stage*/TEST-*.xml | grep -o '<testcase [^>]*classname="[^"]*\.<クラス>"' | wc -l` が同じく本数以上
  が rc=0 になることを指す（`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので `-- --list` の下限を必ず見る）。
  件数を数える一時ファイルは `"$TMPDIR"` の下に書く（`/tmp` の固定名に書かない）
- **計測テスト（`AT`）だけで閉じる Task を作らない。** 手元の `/dev/kvm` が無いことがある（`scripts/verify-run` が `BLOCKED_INFRA` を返す）。
  `AT` を挙げた項目は、同じ振る舞いを `GT`（Robolectric と差し替えた `HealthPermissions`）でも確かめる。`AT` は CI の `android-instrumented` で必ず走る
- **試験の名前の接頭辞**: サーバは `st11_`。Kotlin はクラスで分ける（`HealthPermissionsTest` / `HealthPermissionRecordTest` / `HealthPermissionFlowTest` / `HealthManifestTest` / `HealthNotificationTest` / `HealthPermissionInstrumentedTest`）
- **深掘りの答え（`deep.md` が正本。いまは推奨のまま先に書いた）**: 初回起動で求める（Q1）/ 睡眠・心拍・歩数・運動（Q3）/ 自動で求めるのは 1 度・以後は常駐の通知から（Q4）/
  S-01 へ新しい論理ソースの記録として送る（Q5）/ 説明の画面は短い固定の文（Q6）/ 写真と動画の後・収集を始める前（Q7）/ 無い端末では求めずに記録（Q8）/ 背景も一緒に求める（Q9）/ 位置の拒否の Scenario は THEN / AND で全部（Q10）。
  **本人の答えが推奨と違えば、`spec --answers` がこのファイルを直してから下流に入る**
- **D3〜D10・D12・D14 は（仮）決め。** 反転条件は `design.md`。変えたらその D 番号を書き直す
- **本人の決定と仮決めの値は定数にして試験が名指しで固定する**（`docs/testing.md` §3）: `HEALTH_PERMISSIONS`（D5 の 6 つ）・`HEALTH_LOGICAL_SOURCE = "c01-health-permission"`（D1）・
  `ASKED_HEALTH_PERMISSIONS`（D4 の印の鍵）・`HEALTH_RATIONALE`（D7 の文）・`HEALTH_PERMISSION_ACTION_LABEL = "健康データの権限"`（D8）・`HEALTH_PREFS = "health_permission"`（D14 の置き場）
- **健康データを読まない。** `ReadRecordsRequest` など読み取りの口を呼ばない（ST34）。健康データの生存信号を作らない（design D11）。`c01-health-permission` を `CollectionSource` にしない
- **既存の権限の流れを壊さない**（Story の「壊してはいけないもの」: ST01 の初回起動フロー）。`MainActivityTest`・`MediaPermissionTest`・`SourceIndependenceTest`・`UnusedAppRestrictionsTest` と、
  計測テストの `MainActivityInstrumentedTest`・`PermissionDeniedInstrumentedTest`・`UsageAccessInstrumentedTest`・`UnusedAppRestrictionsInstrumentedTest`・`MediaCaptureInstrumentedTest` を緑のまま
- **`MainActivity` は `android.app.Activity` のまま**（design D3（仮））。結果のコードは見ず、そのつど実際の権限状態を読む（design D27 の教訓）
- **ログに許されているものの中身を出さない**（製造準備 A-2）。出すのは種別（`health_permission_recorded`）・`trigger`・`availability` だけ
- **移行の名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`**（と `.down.sql`）。`migrations/ORDER` の末尾へ 1 行。連番にしない
- **縦串は Story ごとの新しいファイル**（`tools/smoke.d/<作成時刻>_st11-health-permission.sh`）。共有の台本の末尾に書き足さない（`tools/check-shared-appends.sh`）
- 長いコマンド（`cargo test` / gradle / `tools/smoke.sh` / `tools/android-emulator.sh`）は repo の根から `scripts/quiet-run` で 1 回で待つ

## Task 1: 登録簿に `c01-health-permission` を足し、S-01 が受け付け、書き出しに入れる（design D1 / D12）

- [x] 1.1 移行 `YYYYMMDDHHMM_health_permission_source.sql`（と `.down.sql`）を書き、`migrations/ORDER` の末尾に 1 行足す。中身は design D1 の表の 1 行（`ON CONFLICT DO NOTHING`。
  `acquisition` / `fetch_trigger` は列を指定せず ST13 の trigger に入れさせる）。注記に種類が「稼働状況」であること（FR-102。列はまだ無い）を書く。
  `.down.sql` は `202609291230_clock_source.down.sql` と同じく、`core.event` / `core.drop_report` / `core.coverage` / `core.coverage_span` / `core.heartbeat` が参照していれば行を残す。
  `crates/server/src/export/excluded.rs` の `SPAN_RULES` に `("c01-health-permission", SpanRule::Point)`、`NO_COORD_SOURCES` に `"c01-health-permission"` を足す（配列の長さも直す。D12）。
  新しい試験 `crates/server/src/health_permission_tests.rs`（`lib.rs` の `#[cfg(test)]` に 1 行）:
  Scenario: `識別子を持たない健康データの権限の記録が格納される` / `健康データの権限の記録が何日も届かなくても途切れの知らせは出ない` / `稼働状況の格子に健康データの権限の行は出ない` / `書き出しに健康データの権限の記録が入る`。
  ほかに、移行を 2 回当てても行が 1 つ・`external_id_kind = 'none'`・`outage_target = false`・`acquisition = 'device'`・`fetch_trigger = 'on_change'`・`fetch_interval_sec IS NULL` を見る試験と、
  戻しの移行が記録の参照で落ちずに行を残す試験（`clock_tests.rs` の型）。途切れの試験は ST14 の判定（`outage_store`）に 30 日後の時刻を渡して、このソースが出ないことを見る。
  検証: `tools/check-migrations.sh` rc=0 / `tools/check-immutable.sh` rc=0 / `CT st11_`（`-- --list` で 7 本以上）/ `CT export_`（`-- --list` で 1 本以上。既存の書き出しの試験が一覧の突き合わせを含めて全部緑）/
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server'` rc=0（既存が全部緑）

- [x] 1.2 `docs/collector-contract.md` に節「C-01（`c01-health-permission`）が送る `payload` の形（ST11 / design D2）」を足す（`c01-clock` の節の後。欄の表・`trigger` と `availability` の値・
  状態が変わったときだけ送ること・判定に時刻を使わないこと・生存信号を送らないこと・種類は稼働状況）。
  検証: `bash -c 'grep -q "c01-health-permission" docs/collector-contract.md && grep -q "auto_requested" docs/collector-contract.md'` rc=0

## Task 2: manifest の宣言と説明の画面（design D5 / D7 / D13）

- [x] 2.1 新しい `HealthPermissions.kt` に定数 `HEALTH_PERMISSIONS`（D5 の 6 つの権限の名前と D2 の欄の名前の対応）だけを先に置く（読む口と求める口は Task 3.1）。
  manifest に D5 の 6 つの権限（`READ_HEALTH_DATA_HISTORY` は既にある）・Health Connect のアプリを引く `<queries>`・新しい `HealthRationaleActivity` と D7 の 2 つの開き方
  （`ACTION_SHOW_PERMISSIONS_RATIONALE` の `intent-filter` と、`VIEW_PERMISSION_USAGE` + `HEALTH_PERMISSIONS` の `activity-alias`）を足す。
  `HealthRationaleActivity` は D7 の文 `HEALTH_RATIONALE` を出すだけ（全面表示の inset は `MainActivity.showUnusedAppRestrictions` と同じ扱い）。
  D14: `res/xml/` に `dataExtractionRules` と `fullBackupContent` の規則を置き、`HEALTH_PREFS` の置き場だけをクラウドのバックアップと端末の移行から外す（manifest の `application` に 2 つの属性）。
  `AndroidManifest.xml` の FR-11 の注記と `collector-android/README.md` の「永久に読めない」（:103）と「5 つの権限は、いずれも宣言しない期間が取り返せないか…」（:100。数も数え直す）を、
  訂正後の理由（履歴の権限は後から与えても全履歴が読める。初回起動で求めるのは履歴の権限が無い端末で読める範囲の起点を早めるため。`premises.md` P2・P13）に書き直す（D13）。
  新しい試験 `HealthManifestTest`（Robolectric）: manifest が `HEALTH_PERMISSIONS` の 6 つをちょうど宣言している・2 つの intent がどちらも `HealthRationaleActivity` に解決される・
  開いた画面の文が 4 つの種類の名前（睡眠・心拍・歩数・運動）と「自宅の PC」と「承認の設定」と「取り込みを入れてから」を**1 つずつ**含む（定数との一致だけを見ない。spec-review R19）・
  バックアップの 2 つの規則が `health_permission` の置き場を外し、`collector` の置き場を外していない。
  Scenario: `説明の画面は 2 つの開き方のどちらでも開く` / `権限の画面からプライバシーポリシーを開くと説明の画面が出る`（2 本目は Task 7 の計測テストにも印）。
  検証: `GT HealthManifestTest`（4 本以上）/ `bash -c '! git grep -n "永久に読めない" -- collector-android/'` rc=0 /
  `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:detekt` rc=0

## Task 3: Health Connect の読み口と求める口（design D3 / D6）

- [x] 3.1 `app/build.gradle.kts` に `androidx.health.connect:connect-client`（1.1 系）を足す。`HealthPermissions.kt`（2.1 で定数を置いた）に、差し替えられる型
  （使えるか: `available` / `not_installed` / `update_required` / `unreadable`・履歴と背景に対応しているか・D5 の 6 つの許可を読む口と、求める intent を作る口と、結果を受ける口）と、
  client を使う本番の実装を置く。読む口の例外は `unreadable`（例外の型名だけを持つ）に倒し、投げ返さない。D6 の表の「求めるもの」を返す純粋な関数 `healthPermissionsToAsk(availability, features)` を置く。
  `HealthPermissionsTest`（Robolectric と偽の client）: D6 の表の 5 行がそれぞれの「求めるもの」を返す・履歴と背景に未対応なら 4 種類だけ・読む口が例外で `unreadable`・求める intent が `HEALTH_PERMISSIONS` の 6 つを含む。
  検証: `GT HealthPermissionsTest`（6 本以上）/ `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:assembleDebug` rc=0（依存が取れて組める。取れなければ design D3 の反転条件を当てて D3 を書き直す）

## Task 4: 許可の状態の記録と、状態が変わったかの判定（design D2 / D9 / D11）

- [x] 4.1 新しい `HealthPermissionRecord.kt` に、観た状態（D2 の `availability`・`features`・`granted`・`auto_requested`）と契機・端末の時計・起動からの経過時間・起動の識別から
  `c01-health-permission` の記録（`IngestRequest`。`external_id` なし・`schema_version = 1`・`raw` は `payload` の直列化）を作る関数と、最後に記録した状態を `SharedPreferences` に持つ置き場と、
  「観る → 前と違えば 1 件を未送信に積み、積めたら最後の状態を書き換える」1 つの入口 `observeHealthPermissions(trigger)` を置く（最後の状態の置き場は `HEALTH_PREFS`。D14）（時計・経過・起動の識別は `ClockSkewMeasurer` と同じ読み口を差し替えて渡す）。
  ログは `Telemetry.line("health_permission_recorded", …)` に `trigger` と `availability` だけ。
  `HealthPermissionRecordTest`（Robolectric・`TestOutbox`）:
  Scenario: `初めて観たときに健康データの権限の記録が 1 件できる` / `状態が変わらなければ記録しない` / `時計が戻っても状態が変わらなければ記録しない` /
  `健康データの権限の記録は観た時の端末の時計と起動からの経過時間と起動の識別を持つ` / `健康データの権限のログに許されているものの中身が出ない`。
  ほかに、`payload` の欄とキーの順が D2 の形のまま（`ClockSkewPayloadShapeTest` の型）・同じ状態から 2 回作った原文が契機と時刻だけで違う・未送信に積めなかったら最後の状態を書き換えない を見る。
  「時計が戻っても」は偽の壁時計を 1 日戻して同じ状態で観る。「ログ」は `ShadowLog` を集め、`granted` の値（`true` / `false`）と権限の名前が出ないことを見る。
  検証: `GT HealthPermissionRecordTest`（8 本以上）/ `GT ClockSkewPayloadShapeTest`（既存が緑）/ `bash tools/check-log-private.sh` rc=0

- [x] 4.2 `c01-health-permission` が生存信号の対象にならないことを固定する。`HealthPermissionRecordTest` に 1 本足し、収集を始めて生存信号の契機に達しても
  `c01-health-permission` の生存信号が積まれないことと、`CollectionSource` の一覧に入っていないことを見る。
  Scenario: `健康データの権限の生存信号は送られない`。
  検証: `GT HealthPermissionRecordTest`（9 本以上。4.1 と合わせて）/ `GT SourceIndependenceTest`（既存が緑）

## Task 5: 初回起動の流れに健康データの段を入れる（design D4 / D6 / D9）

- [x] 5.1 `MainActivity.proceed()` の写真と動画の後・`start()` の前に健康データの段を入れる（D4）。`protected open fun healthPermissions(): HealthPermissions` を差し替え口にし、
  読む口は主スレッドを塞がずに読み、読み終えるまで `inFlight` で先へ進まない。`startActivityForResult` で求め、`onActivityResult` で印 `ASKED_HEALTH_PERMISSIONS` を立て、
  `observeHealthPermissions(request_result)` をしてから `proceed()` に戻る。健康データの段を最初に通るときに `observeHealthPermissions(app_open)` をする（D9）。
  `app_open` は Health Connect が使えるかに依らず、起動ごとに写真と動画の段を初めて越えたときに観る（D9。求めるかどうかとは別）。
  `HealthPermissionFlowTest`（Robolectric・`MainActivityTest` の型。偽の `HealthPermissions` で求めた回数と求めたものを数える）:
  Scenario: `初回起動で健康データの権限の画面が出る`（求めた intent が 6 つを含む）/ `種類と履歴と背景をすべて許すとすべて許された状態になる`（偽の結果で 6 つを許した後に読み直した状態）/
  `入れ直すと初回起動でもう一度求められる`（`HEALTH_PREFS` を消してから開く）/ `種類を許して履歴の読み取りだけを断っても収集は始まる` /
  `権限の画面に答えずにアプリが終わると次の起動でもう一度求められる`（求めた後に結果を返さずに画面を壊し、作り直して開く）/ `断った後に設定で許すと次に観たときに記録される` /
  `健康データの権限は写真と動画の後・収集を始める前に求められる` / `前の版から更新した端末では更新の後の最初の起動で 1 度求められる` / `2 度目の起動では健康データの権限を自動で求めない` /
  `求めるものがすべて許されていれば画面を出さない` / `Health Connect が無い端末では求めずに収集が始まる` / `更新が要る Health Connect の端末では求めずに収集が始まる` /
  `Health Connect が使えるようになったら 1 度だけ求める` / `履歴の読み取りに対応していない端末では対応しているものだけを求める` / `Health Connect が使えるかを読めない端末では求めずに収集が始まる` /
  `健康データの権限をすべて断っても収集は始まる` / `断ると自動で求めたことと許されていないことが記録される` / `健康データの権限が無くても位置とアプリ利用と写真は集まる`。
  Health Connect の画面そのもの（並び・2 枚目のトグル）は Task 7.1 の計測テストだけが見る。ここは偽の `HealthPermissions` で、求めたものと、読み直した状態と、収集の始まりを見る
  「更新した端末」は `SENT_TO_USAGE_ACCESS` などの既存の印を立て、`ASKED_HEALTH_PERMISSIONS` だけが無い `SharedPreferences` で開く。
  「収集が始まる」は `LocationService` の起動の intent が出たことを見る（`MediaPermissionTest` と同じ）。「位置とアプリ利用と写真は集まる」は `SourceIndependenceTest` と `MediaScannerTest` の型で健康データの権限を全部外して取得契機を進め、写真を 1 枚置く。
  AND の「記録に〜が載る」は `TestOutbox` に積まれた `c01-health-permission` の `availability` を見る。
  検証: `GT HealthPermissionFlowTest`（18 本以上）/ `GT MainActivityTest` / `GT MediaPermissionTest` / `GT UnusedAppRestrictionsTest` / `GT SourceIndependenceTest`（既存がすべて緑）

## Task 6: 常駐の通知の操作と 6 時間ごとの見直し（design D8 / D9）

- [x] 6.1 `LocationService` の前景サービスの通知に、`availability = available` のときだけ D8 の操作（ラベル `HEALTH_PERMISSION_ACTION_LABEL`）を足す。開く intent は版で分ける（D8）。
  生存信号と同じ 6 時間の刻みの隣で `observeHealthPermissions(periodic)` を呼ぶ（生存信号は送らない。失敗しても位置の収集を止めない —— 時計の測定と同じく `runCatching` で包み、種別だけログ）。
  `HealthNotificationTest`（Robolectric）:
  Scenario: `常駐の通知から健康データの権限の画面が開く` / `Health Connect が使えない端末の通知には健康データの権限の操作が出ない` / `後から設定で外すと次の見直しで記録される`。
  1 本目は通知の操作の `PendingIntent` の中身（action と package）を見る（Task 7 の計測テストにも印）。3 本目は偽の `HealthPermissions` の履歴を外してから 6 時間を進め、`TestOutbox` に 1 件積まれたことを見る。
  ほかに、見直しが例外を投げても位置の記録が契機ごとに生成され続けることを見る。
  検証: `GT HealthNotificationTest`（4 本以上）/ `GT LocationServiceTest` / `GT LocationServiceClockTest`（既存が緑）

## Task 7: 本物の Health Connect での計測テスト（design D4 / D10。Risks）

- [x] 7.1 Health Connect の権限の画面を本物で押す計測テストを、始まりの状態ごとにクラスで分けて置く（どれも `@NeedsPristinePermissions`。spec-review R6）。
  UI Automator で Health Connect の画面の文字を読んで押す（「はじめに」の画面が出たら「Get started」を押してから進む。P7）。
  自分の権限は自分で外さない（計測テストのプロセスが死ぬ）。始まりの状態は `tools/android-emulator.sh` と `.github/workflows/ci.yml` の android-instrumented に足す段の前の
  `adb shell am force-stop dev.ashiato.collector` と `adb shell pm reset-permissions` で作る（`pm clear` は使わない —— 前景サービスが残っていると Failed）。2 回断ると画面が出なくなる（P6）ので、1 段に断る本を 1 つまで。
  - `HealthPermissionGrantInstrumentedTest`（新しい段）: Scenario: `初回起動で健康データの権限の画面が出る` / `種類と履歴と背景をすべて許すとすべて許された状態になる` /
    `権限の画面からプライバシーポリシーを開くと説明の画面が出る` / `常駐の通知から健康データの権限の画面が開く` /
    `許すと許した状態と観た時刻の記録が S-01 に格納される`（端末の側: 許した後に未送信に積まれた `c01-health-permission` の `granted` が全部 `true`）
  - `HealthPermissionHistoryDeniedInstrumentedTest`（新しい段）: Scenario: `種類を許して履歴の読み取りだけを断っても収集は始まる`
  - `HealthPermissionDeniedInstrumentedTest`（新しい段。最後に置く）: Scenario: `健康データの権限をすべて断っても収集は始まる`。続けて、試験の中で自分の `HEALTH_PREFS` を消して開き直し、
    `入れ直すと初回起動でもう一度求められる` を見る（`pm reset-permissions` が Health Connect の「断った回数」を戻すかは確かめていない（Risks）。戻らずに画面が出なければ、
    その結果を PR の本文に書き、この Scenario は Task 5.1 の `GT` だけで閉じる）
  「すべて許された状態」は `checkSelfPermission` で 6 つを読む。本人の携帯では入れ直さない（端末識別子が変わる）。
  検証: `AT HealthPermissionGrantInstrumentedTest`（5 本以上）/ `AT HealthPermissionHistoryDeniedInstrumentedTest`（1 本以上）/ `AT HealthPermissionDeniedInstrumentedTest`（1 本以上）/
  `bash -c 'test "$(grep -c HealthPermission tools/android-emulator.sh)" -ge 3 && test "$(grep -c HealthPermission .github/workflows/ci.yml)" -ge 3'` rc=0（3 つのクラスの段がどちらにもある）。同じ振る舞いの `GT` は Task 2・5・6

- [x] 7.2 `PermissionDeniedInstrumentedTest` に `// Scenario: 位置の権限を拒否しても収集は始まり、拒否したことが残る` の印を付け、「spec の Scenario には対応しない」の注記を消す（D10）。
  THEN の「位置は取得できない状態として生存信号に載る」は計測テストでは見ていないので、`SourceIndependenceTest` に、位置を拒否した後に始めた収集の位置の生存信号が取得できない状態と権限を示すことを見る 1 本を足し、同じ印を置く。
  検証: `AT PermissionDeniedInstrumentedTest`（1 本以上）/ `GT SourceIndependenceTest`（11 本以上。いまの 10 本 + 1）/
  `bash -c 'grep -q "Scenario: 位置の権限を拒否しても収集は始まり、拒否したことが残る" collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/PermissionDeniedInstrumentedTest.kt'` rc=0

## Task 8: 縦串 —— 端末が送る形の記録が S-01 に格納される

- [x] 8.1 `tools/smoke.d/<作成時刻>_st11-health-permission.sh` を置く（`tools/smoke.d/README.md` の形。ST09 の縦串と同じく `post` / `rawstr` を使う）。
  D2 の形の `c01-health-permission` の記録（4 種類・履歴・背景が `true`、`external_id` なし）を本物のサーバに送り、受理されること・D-01 に 1 行入ること・`payload->'granted'->>'history'` が `true`・
  `event_time` が送った `device_time` と同じことを見る。同じ記録をもう 1 度送って `duplicate: true` で行が増えないことも見る。
  Scenario: `許すと許した状態と観た時刻の記録が S-01 に格納される`（サーバの側。端末の側は Task 7.1）。
  記録の形は台本の中に D2 の形の JSON で書く（ST09 の縦串と同じ）。端末が作る形が D2 と同じことは Task 4.1 の形の試験と Task 1.2 の契約の節が固定する（spec-review R9）。
  検証: `scripts/quiet-run smoke -- tools/smoke.sh` rc=0 / `bash tools/check-shared-appends.sh` rc=0 / `python3 scripts/check_scenarios.py . st11-health-history-permission` rc=0

## Task 9: ST34・ST38 への申し送り

- [x] 9.1 `docs/handoff/ST34.md` に節を 1 つ足す（無ければ作る。`docs/handoff/README.md` の形）: 求めた種類と権限（D5）・背景での読み取りを求めたこと（Q9）・
  `c01-health-permission` の記録の読み方（履歴の権限が無い端末で、読める範囲の起点を最初に種類が許された記録の `device_time` の 30 日前と読む。P13）・
  足りない種類を ST34 が求めるときに `ASKED_HEALTH_PERMISSIONS` とは別の印を持つこと（1 度だけの案内を使い切っているので）。
  本人の端末の実物（Q2。`premises.md` P10・P17）: Android 16、Health Connect に書いているのは Google Fit だけで 19 種類（ST11 が求めない 15 種類を含む）、
  自動削除の設定が見当たらなかった（読み始めるまでに消える経路を ST34 が確かめる）。
  検証: `bash -c 'grep -q "c01-health-permission" docs/handoff/ST34.md && grep -q "自動削除" docs/handoff/ST34.md'` rc=0 / `python3 scripts/check_chain.py .` rc=0

- [x] 9.2 `docs/handoff/ST38.md` に節を 1 つ足す（ST17 の `s01-feeling`・ST09 のメディアのソースと同じ型）: `c01-health-permission` を種類「稼働状況」に属させること（FR-102。登録簿に種類の列がまだ無い）・
  その理由（収集が残すもの）・「稼働状況」は外部 AI に承認済み（FR-88）なので、外部 AI の経路ができた後は許可の状態も外へ出ること（spec-review R3）。
  検証: `bash -c 'grep -q "c01-health-permission" docs/handoff/ST38.md'` rc=0
