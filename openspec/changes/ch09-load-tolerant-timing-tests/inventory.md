# Task 5 の Rust 候補一覧

| 場所（試験・補助） | 型 | 方針 | 理由 |
|---|---|---|---|
| `crates/server/src/scheduled_tests.rs` の `wait_for_calls`、4 件のログ待ち | 条件の待ち | 実時間 30 秒期限へ変更 | 停止した Tokio 時計の刻みは変えず、回数上限だけを実時間期限に置き換えた。 |
| `crates/server/src/outage_api_tests.rs:419` の台帳ロック待ち | 条件の待ち | 実時間 30 秒期限へ変更 | DB の待機数を観測できるため、最後の待機数を含む期限診断にした。 |
| `crates/collector-windows/src/runtime.rs` の `HistoryRuntime::drop` | 条件の待ち | 実時間 30 秒期限へ変更 | 履歴 worker の終了を観測でき、期限切れ時に最後の終了状態を出せる。 |
| `crates/collector-windows/src/runtime.rs` の `tick_until_history_idle` | 条件の待ち | 実時間 30 秒期限へ変更 | `is_reading` を観測できるため、最後に見た値を含む期限診断にした。 |
| `crates/collector-windows/src/runtime.rs` の `tick_until_history_beats` | 条件の待ち | 実時間 30 秒期限へ変更 | 生存信号の件数を観測できるため、最後の件数を含む期限診断にした。 |
| `crates/collector-windows/src/history/collect.rs:868`・`:969` の履歴取得待ち | 条件の待ち | 実時間 30 秒期限へ変更 | 取得結果と積み込み状態を観測できるため、最後に見た値を含む期限診断にした。 |
| `crates/collector-windows/src/runtime.rs` の `tick_until_read`・`tick_until_read_again` | 条件の待ち（時刻を兼ねる） | 直さない | 1 回ごとに試験時刻を進めるループであり、期限化すると時刻が進みすぎる（D4）。 |
| `crates/server/src/stay_tests.rs:2389` | 計算の形 | 直さない | 計算の形を実時間で見ており、製品コードを変えずに観測できる条件がない（D6）。 |
| `crates/server/src/testdb_tests.rs:371` `ch04_statement_timeout_cancels_a_long_query` | 打ち切り | `pg_sleep(60)`・30 秒未満へ変更 | 打ち切りの相手と上限を一緒に延ばした（D6）。 |
| `crates/collector-windows/src/time_sync.rs:650` `time_sync_timeout_kills_the_child` | 打ち切り | `sleep 60`・30 秒未満へ変更 | 打ち切りの相手と上限を一緒に延ばした（D6）。 |

## 探した範囲・探していない範囲

Task 5 が名指しした `crates/server` と `crates/collector-windows` の Rust 試験を走査した。画面・Android・Windows 実機試験・縦串台本は別 Task の担当であり、この一覧では扱っていない。

## 製品の不具合の疑い

無し。変更は `#[cfg(test)]` の試験コード内に限る。

## Task 8: Windows 実行時テスト

| 場所（試験・補助） | 型 | 方針 | 理由・確認状況 |
|---|---|---|---|
| `crates/collector-windows/tests/runtime_windows.rs:141` `HelperWindow::drop`、`:217` `focus_window_of`、`:412` 題名変化、`:957` `Edge::drop`、`:1223` `trigger`、`:1433` `HistoryBrowser::drop` | 固定の眠り | `// timing-ok:` を付けて維持 | 相手役の終了・窓の出現・題名変化・プロファイル解放・問い合わせ完了を観測する口がないため、D5 に従い余裕として残した。Windows runtime 15 本の実行で確認済み。 |
| `crates/collector-windows/tests/runtime_windows.rs:241` `run_for` | 決まった時間の窓 | `// timing-ok:` を付けて維持 | 指定秒数の観測窓そのものなので D3/D4 に従い延ばさない。Windows runtime 15 本の実行で確認済み。 |
| `crates/collector-windows/tests/runtime_windows.rs:225` `observe_until`、`:253` `run_until`、`:195` `focus_window_of` の期限、`:865` W32Time、`:1203` 通知履歴 | 条件の待ち | 実時間 30 秒期限へ変更 | 条件が来た時点で返る待ちを D4 に統一し、失敗時に待った時間を出す。Windows runtime 15 本の実行で確認済み。 |
| `crates/collector-windows/tests/runtime_windows.rs:1441` `fetch_history_until` | 条件の待ち | 実時間 90 秒期限を維持 | ブラウザ履歴の遅延と 25 時間刻みを含む実行時テスト固有の上限。Windows runtime 15 本の実行で確認済み。 |

Windows 側の検証は `cargo test -p ashiato-collector-windows --test runtime_windows -- --test-threads=1` を実行し、`test result: ok. 15 passed; 0 failed`（2026-11-05、evidence fingerprint `ae55e14e`）を得た。前回の 9/15 は対話デスクトップの前景窓取得に失敗した環境結果であり、今回の再実行で解消を確認した。

## Task 12: 機械走査で拾った候補ファイル

check-inventory.sh --list が拾う全ファイルを、D14 の表の列で記録する。録画用の候補は
合否を出す試験ではないため、候補と場所だけを一覧化し、修正は別 change の責務とする。

| 場所（試験・補助） | 型 | 方針 | 理由 |
|---|---|---|---|
| crates/collector-windows/examples/browser_history_smoke.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/collector-windows/src/time_sync.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/collector-windows/tests/runtime_windows.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/collector-windows/src/history/collect.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/collector-windows/src/runtime.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/collector-windows/src/clock.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/collector-windows/src/clock_worker.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/testdb_tests.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/outage.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/web_session.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/lib.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/tests/server_startup.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/export_file_tests.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/testdb.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/stay_tests.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/scheduled_tests.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/web_session_tests.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/archive/worker.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/outage_api_tests.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| crates/server/src/archive_scan_interval_tests.rs | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/playwright.config.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st22/st22-phone-dark.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st22/st22-erase-reload.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/src/__tests__/satellites.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st08/st08-result.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/e2e/settings-export.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/places-change.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st13/st13-sources.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/src/__tests__/retired-source.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st25/st25-day-timeline.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/src/__tests__/session.test.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st33/st33-export.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/e2e/feel-parts.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/e2e/day-erase.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st17/st17-feel.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st23/st23-purge.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/e2e/feel-ch01.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/master-view-limits.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/app.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st50/st50-result.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/e2e/st49-client-version.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/st25-open.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/e2e/day-timeline.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/FeelView.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/ch08/ch08-nested.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st14/st14-coverage.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/src/__tests__/st25-gaps.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/master-form.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st28/st28-login.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st29/st29-satellites.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/src/__tests__/master-view.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/DayFeeling.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/st25-summary.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st07/st07-result.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/src/__tests__/places-add.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/FeelViewParts.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/DayView.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/e2e/places.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/ch01/ch01-feel.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st12/st12-archive.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/src/__tests__/DayView-erase-erased-row-keyboard.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st21/st21-places.rec.ts | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| web/src/__tests__/one-scroll.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/places-view.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/archive-one-scroll.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/st25-routes.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/places-tabs.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/e2e/st23-purge.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/e2e/feel.spec.ts | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| web/src/__tests__/SettingsView.test.tsx | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st13/St13RecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st06/St06LaterGrantRecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st06/St06RecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st17/St17RecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HttpTransportInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/FeelingReminderInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st11/St11RecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st09/St09RecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaCaptureInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/UsageAccessInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/PermissionDeniedInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/RetentionInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/LocationServiceInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/ClockSkewInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermissionGrantInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/UnusedAppRestrictionsInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaPremisesInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/ForegroundServiceTypeInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MainActivityInstrumentedTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/test/kotlin/dev/ashiato/collector/RollupImportTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthConnectScreens.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/test/kotlin/dev/ashiato/collector/FeelingReminderReceiverTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthPermissionFlowTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthNotificationTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| collector-android/app/src/test/kotlin/dev/ashiato/collector/LocationServiceClockTest.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st05/St05RecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| collector-android/app/src/test/kotlin/dev/ashiato/collector/AppUsageFixtures.kt | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/recording/st14/St14RecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/recording/st50/St50RecordingTest.kt | 暗黙の上限 | 一覧化 | 録画用の待ちを記録し、別 change で扱う。 |
| tools/check-exposure.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-released-migrations.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-verify-env-reset.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/verify-prep.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/ports.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/verify-env.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/android-emulator.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-private.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/pg_dump-docker.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/agent-env.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-tmp-hygiene.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-db-version-stop.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-ports.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/usage-volume.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/load-repeat.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/export-scaling.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/tmp-leak-check.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-test-db-killed.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/smoke.d/202610241500_st29-satellite-credentials.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-panic-log.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-test-db-growth.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-openapi.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/smoke.d/202610251200_st23-purge.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-offsite.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/stack.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-contract-version.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-db-secret.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-no-time-server.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/docker-tidy.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/smoke.d/202610161200_st08-browser-history.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/smoke.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-build-stamp.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/st25-seed-check.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-api-token-reads.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-migrations.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/smoke.d/202610212100_st13-archive-hands-off.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/seed.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-licenses.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-log-private.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/web-login.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/smoke.d/202610141130_st14-outage-alert.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/record-env.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/ci-db-env.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/wait-android-boot.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-handoff-reaches-brief.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/android-media-fixtures.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/db-roles.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/archive-shape.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-immutable.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-satellite-boundary.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/break-check.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |
| tools/check-shared-appends.sh | 暗黙の上限 | 一覧化 | 候補を拾い、該当箇所は個別の Task 方針に従った。 |

## Task 12 の探した範囲・探していない範囲

探した範囲は crates/**/*.rs、web/src/__tests__、web/e2e、web/playwright.config.ts、
collector-android/app/src/test、collector-android/app/src/androidTest、tools/*.sh、tools/smoke.d/*.sh
および録画の tools/recording である。録画は一覧だけに載せ、合否を出す試験としては扱わない。
これらの外側（依存ライブラリ、生成物、target、node_modules、端末・Windows 側の clone）は探していない。

## Task 12 の製品の不具合の疑い

無し。候補は試験・補助・台本の待ちであり、製品コードの変更は行っていない。
