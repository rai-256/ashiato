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

### Task 3〜11 の実施結果と設計上の既知の候補

| 場所（試験・補助） | 型 | 方針 | 理由 |
|---|---|---|---|
| `crates/collector-windows/src/runtime.rs:3612` `history_large_fetch_keeps_observing_foreground` | 待たない | 直さない | 観測を続ける試験で、回数による待ちを持たない。 |
| `crates/collector-windows/src/runtime.rs:1936` `clock_worker_slow_read_does_not_hold_the_patrol` | 待たない | 直さない | 止めた読み手を見張る試験で、経過比較を維持する。 |
| `crates/collector-windows/src/runtime.rs:2499`・`:3462` `tick_until_read`・`tick_until_read_again` | 条件の待ち | 回数を維持 | 時刻を兼ねるループなので D4 の除外に該当する。 |
| `crates/collector-windows/src/runtime.rs:2512`・`:2470`・`:2545` | 条件の待ち | 30 秒の期限へ変更 | 条件を観測でき、期限切れ時の診断を残せる。 |
| `crates/collector-windows/src/history/collect.rs:868`・`:969` | 条件の待ち | 30 秒の期限へ変更 | 履歴取得の終了を観測できる。 |
| `crates/server/src/archive_scan_interval_tests.rs:157`・`:227` | 条件の待ち | 30 秒の期限へ変更 | スキャン結果を観測して競り合いを待つ。 |
| `crates/server/src/archive_scan_interval_tests.rs:207`・`:250`・`:258` | 固定の眠り | 直さない | 相手役の進行を作る固定待ちである。 |
| `crates/server/src/web_session_tests.rs:273` | 起きない・下限 | 直さない | 300ms 以上の下限を検証する試験である。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt:34`・`:36`・`:58`・`:62`・`:69`・`:75` | 条件の待ち | 30 秒の期限へ変更 | ログ到着を条件として待ち、期限を揃える。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt:72` | 起きない・下限 | 直さない | ログが来ないことの下限を検証する。 |
| `web/playwright.config.ts`・`web/e2e/*.spec.ts` | 暗黙の上限 | 30 秒/120 秒へ統一 | Playwright の既定値と個別上限を一覧化し、Task 9〜11 で調整した。 |
| `web/e2e/feel.spec.ts:208`・`:290` | 固定の眠り | 条件待ちへ変更または維持 | 表示条件へ置換できる箇所と録画上必要な待ちを分けた。 |
| `web/e2e/day-erase.spec.ts:70` | 起きない・下限 | `// timing-ok:` で維持 | 任意操作を飛ばすための短い上限で、成立条件の待ちではない。 |
| `web/e2e/places.spec.ts:37` | 暗黙の上限 | 120 秒へ変更 | 画面 e2e の試験上限を設定値へ揃えた。 |
| `collector-android/app/src/androidTest` の `SETTLE_MS` | 条件の待ち / 起きない・下限 | 条件待ちだけ変更 | 条件成立で抜ける待ちと、何も起きないことを見る窓を分けた。 |
| `tools/smoke.sh:171`・`:190`・`:211` | 条件の待ち | 回数×sleep を 30 秒以上に維持 | 縦串の既存台本は行単位で直し、末尾へ節を足していない。 |
| `tools/smoke.sh:199` | 起きない・下限 | 直さない | 件数が増えないことを見る下限である。 |
| `crates/server/src/stay_tests.rs:2389` | 計算の形 | 直さない | 実時間の計算を検証し、製品コードを変えない。 |
| `crates/server/src/testdb_tests.rs:371`・`crates/collector-windows/src/time_sync.rs:650` | 打ち切り | 相手と上限を同時に延長 | 打ち切りの再現条件を失わない。 |
| `tools/check-verify-env-reset.sh`・`tools/check-test-db-killed.sh` | 速さが要件 | 直さない | 検査台本自身の上限・終了条件であり、試験の条件待ちではない。 |

機械走査の個別行は、上表の型・方針をファイル単位に適用した記録である。録画用ファイルは `一覧化（別 change）`、製品側の補助コードは `一覧化（製品コードは変更しない）` とし、Task 3〜11 で直した試験・台本は上表の具体的な行を正本とする。

check-inventory.sh --list が拾う全ファイルを、D14 の表の列で記録する。録画用の候補は
合否を出す試験ではないため、候補と場所だけを一覧化し、修正は別 change の責務とする。

| 場所（試験・補助） | 型 | 方針 | 理由 |
|---|---|---|---|
|  crates/collector-windows/examples/browser_history_smoke.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 browser_history_smoke.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/collector-windows/src/time_sync.rs:650 `time_sync_timeout_kills_the_child` |  打ち切り | 製品実装なので一覧化のみ | 製品実装 time_sync.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/collector-windows/src/history/fetch.rs |  条件の待ち | 製品実装なので一覧化のみ | 製品実装 fetch.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/collector-windows/src/platform.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 platform.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/collector-windows/src/sender.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 sender.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/collector-windows/tests/runtime_windows.rs:225 `observe_until`・`:241` `run_for`・`:253` `run_until` |  条件の待ち / 起きない・下限 | Rust 試験の条件待ちとして個別に整理 | Rust 試験 runtime_windows.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/collector-windows/src/history/collect.rs:868・:969 |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 collect.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/collector-windows/src/runtime.rs:2499・:2512・:3462 |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 runtime.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/collector-windows/src/clock.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 clock.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/collector-windows/src/clock_worker.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 clock_worker.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/server/src/testdb_tests.rs:371 `ch04_statement_timeout_cancels_a_long_query` |  打ち切り | 相手と上限を同時に整理 | Rust 試験 testdb_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/outage.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 outage.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/server/src/access_log_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 access_log_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/access_migration_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 access_migration_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/archive_flow_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 archive_flow_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/archive_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 archive_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/attributes_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 attributes_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/deletion_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 deletion_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/export_job_tests.rs |  条件の待ち | 既存の 30 秒超の期限を維持 | 3000 回×100ms の上限は 30 秒未満ではなく、候補の分類だけを行う。 |
|  crates/server/src/feelings_ingest_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 feelings_ingest_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/places_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 places_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/purge_compaction_tests.rs |  条件の待ち | 既存の 30 秒超の期限を維持 | 500 回×`POLL` のロック待ちは 30 秒未満の期限ではなく、既存の打ち切り条件を一覧に残す。 |
|  crates/server/src/stay_store.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 stay_store.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/server/src/web_session.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 web_session.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/server/src/lib.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 lib.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/server/tests/server_startup.rs |  暗黙の上限 | Rust 試験の条件待ちとして個別に整理 | Rust 試験 server_startup.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/export_file_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 export_file_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/testdb.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 testdb.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/server/src/stay_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 stay_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/scheduled_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 scheduled_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/web_session_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 web_session_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/archive/worker.rs |  暗黙の上限 | 製品実装なので一覧化のみ | 製品実装 worker.rs の時間依存は候補として記録し、Q0 によりこの change では変更しない。 |
|  crates/server/src/outage_api_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 outage_api_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  crates/server/src/archive_scan_interval_tests.rs |  条件の待ち | Rust 試験の条件待ちとして個別に整理 | Rust 試験 archive_scan_interval_tests.rs は観測条件・打ち切り・固定待ちを行番号ごとに分類し、製品実装の候補と混ぜない。 |
|  web/playwright.config.ts |  暗黙の上限 | 設定値へ統一または一覧化 | Playwright の設定・e2e の待ちを対象にした。 |
|  tools/recording/st22/st22-phone-dark.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st22-phone-dark.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st22/st22-erase-reload.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st22-erase-reload.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/satellites.test.tsx` | 暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 satellites.test.tsxundefined の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st08/st08-result.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st08-result.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/e2e/settings-export.spec.ts |  暗黙の上限 | 個別の画面条件または設定値として整理 | 本物のブラウザで走る settings-export.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  web/src/__tests__/places-change.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 places-change.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st13/st13-sources.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st13-sources.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/src/__tests__/retired-source.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 retired-source.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st25/st25-day-timeline.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st25-day-timeline.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/src/__tests__/session.test.ts |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 session.test.ts の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st33/st33-export.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st33-export.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/e2e/feel-parts.spec.ts |  暗黙の上限 | 個別の画面条件または設定値として整理 | 本物のブラウザで走る feel-parts.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  web/e2e/day-erase.spec.ts |  起きない・下限 | timing-ok を付けて維持 | 本物のブラウザで走る day-erase.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  tools/recording/st17/st17-feel.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st17-feel.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st23/st23-purge.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st23-purge.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/e2e/feel-ch01.spec.ts |  暗黙の上限 | 個別の画面条件または設定値として整理 | 本物のブラウザで走る feel-ch01.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  web/src/__tests__/master-view-limits.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 master-view-limits.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/app.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 app.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st50/st50-result.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st50-result.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/e2e/st49-client-version.spec.ts |  暗黙の上限 | 個別の画面条件または設定値として整理 | 本物のブラウザで走る st49-client-version.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  web/src/__tests__/st25-open.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 st25-open.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/e2e/day-timeline.spec.ts |  暗黙の上限 | 個別の画面条件または設定値として整理 | 本物のブラウザで走る day-timeline.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  web/src/__tests__/FeelView.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 FeelView.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/ch08/ch08-nested.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 ch08-nested.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st14/st14-coverage.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st14-coverage.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/src/__tests__/st25-gaps.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 st25-gaps.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/master-form.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 master-form.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st28/st28-login.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st28-login.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st29/st29-satellites.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st29-satellites.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/src/__tests__/master-view.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 master-view.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/DayFeeling.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 DayFeeling.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/st25-summary.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 st25-summary.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st07/st07-result.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st07-result.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/src/__tests__/places-add.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 places-add.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/FeelViewParts.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 FeelViewParts.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/DayView.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 DayView.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/e2e/places.spec.ts |  暗黙の上限 | 個別の画面条件または設定値として整理 | 本物のブラウザで走る places.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  tools/recording/ch01/ch01-feel.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 ch01-feel.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st12/st12-archive.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st12-archive.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/src/__tests__/DayView-erase-erased-row-keyboard.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 DayView-erase-erased-row-keyboard.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st21/st21-places.rec.ts |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 st21-places.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  web/src/__tests__/one-scroll.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 one-scroll.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/places-view.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 places-view.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/archive-one-scroll.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 archive-one-scroll.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/st25-routes.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 st25-routes.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/src/__tests__/places-tabs.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 places-tabs.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  web/e2e/st23-purge.spec.ts |  暗黙の上限 | 個別の画面条件または設定値として整理 | 本物のブラウザで走る st23-purge.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  web/e2e/feel.spec.ts |  固定の眠り | 個別の画面条件または設定値として整理 | 本物のブラウザで走る feel.spec.ts の待ちを、Playwright の条件待ち・上限・下限に分けて扱う。 |
|  web/src/__tests__/SettingsView.test.tsx |  暗黙の上限 | 画面単体試験の候補として記録 | 画面単体試験 SettingsView.test.tsx の待ち候補であり、Scenario の担保は web/e2e に置く。 |
|  tools/recording/st13/St13RecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St13RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st06/St06LaterGrantRecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St06LaterGrantRecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st06/St06RecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St06RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st17/St17RecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St17RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HttpTransportInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の HttpTransportInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/FeelingReminderInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の FeelingReminderInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  tools/recording/st11/St11RecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St11RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st09/St09RecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St09RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaCaptureInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の MediaCaptureInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/UsageAccessInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の UsageAccessInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/PermissionDeniedInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の PermissionDeniedInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/RetentionInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の RetentionInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/LocationServiceInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の LocationServiceInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/ClockSkewInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の ClockSkewInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermissionGrantInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の HealthPermissionGrantInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/UnusedAppRestrictionsInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の UnusedAppRestrictionsInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaPremisesInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の MediaPremisesInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/ForegroundServiceTypeInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の ForegroundServiceTypeInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MainActivityInstrumentedTest.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の MainActivityInstrumentedTest.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt:34・:36・:58・:62・:69・:75 |  条件の待ち | 単体試験の条件待ちとして個別に整理 | Robolectric/JUnit の UnavailableLogGateTest.kt はログ・通知の到着条件を持つ箇所と下限確認を分ける。 |
|  collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt:72 |  起きない・下限 | timing-ok を付けて維持 | Robolectric/JUnit の UnavailableLogGateTest.kt はログ・通知の到着条件を持つ箇所と下限確認を分ける。 |
|  collector-android/app/src/test/kotlin/dev/ashiato/collector/RollupImportTest.kt |  暗黙の上限 | 単体試験の条件待ちとして個別に整理 | Robolectric/JUnit の RollupImportTest.kt はログ・通知の到着条件を持つ箇所と下限確認を分ける。 |
|  collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthConnectScreens.kt |  条件の待ち | 計測テストの条件待ちとして個別に整理 | エミュレータ上の HealthConnectScreens.kt は端末状態の成立を待つため、固定上限と条件成立を混同しない。 |
|  collector-android/app/src/test/kotlin/dev/ashiato/collector/FeelingReminderReceiverTest.kt |  暗黙の上限 | 単体試験の条件待ちとして個別に整理 | Robolectric/JUnit の FeelingReminderReceiverTest.kt はログ・通知の到着条件を持つ箇所と下限確認を分ける。 |
|  collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthPermissionFlowTest.kt |  暗黙の上限 | 単体試験の条件待ちとして個別に整理 | Robolectric/JUnit の HealthPermissionFlowTest.kt はログ・通知の到着条件を持つ箇所と下限確認を分ける。 |
|  collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthNotificationTest.kt |  暗黙の上限 | 単体試験の条件待ちとして個別に整理 | Robolectric/JUnit の HealthNotificationTest.kt はログ・通知の到着条件を持つ箇所と下限確認を分ける。 |
|  collector-android/app/src/test/kotlin/dev/ashiato/collector/LocationServiceClockTest.kt |  暗黙の上限 | 単体試験の条件待ちとして個別に整理 | Robolectric/JUnit の LocationServiceClockTest.kt はログ・通知の到着条件を持つ箇所と下限確認を分ける。 |
|  tools/recording/st05/St05RecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St05RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  collector-android/app/src/test/kotlin/dev/ashiato/collector/AppUsageFixtures.kt |  暗黙の上限 | 単体試験の条件待ちとして個別に整理 | Robolectric/JUnit の AppUsageFixtures.kt はログ・通知の到着条件を持つ箇所と下限確認を分ける。 |
|  tools/recording/st14/St14RecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St14RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/recording/st50/St50RecordingTest.kt |  暗黙の上限 | 録画の確認用として別 change で整理 | 録画台本 St50RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
|  tools/check-exposure.sh |  速さが要件 | 検査・起動台本として一覧化 | check-exposure.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-released-migrations.sh |  速さが要件 | 検査・起動台本として一覧化 | check-released-migrations.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-verify-env-reset.sh |  速さが要件 | 検査・起動台本として一覧化 | check-verify-env-reset.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/verify-prep.sh |  速さが要件 | 検査・起動台本として一覧化 | verify-prep.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/ports.sh |  速さが要件 | 検査・起動台本として一覧化 | ports.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/verify-env.sh |  速さが要件 | 検査・起動台本として一覧化 | verify-env.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/android-emulator.sh |  速さが要件 | 検査・起動台本として一覧化 | android-emulator.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-private.sh |  速さが要件 | 検査・起動台本として一覧化 | check-private.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/pg_dump-docker.sh |  速さが要件 | 検査・起動台本として一覧化 | pg_dump-docker.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/agent-env.sh |  速さが要件 | 検査・起動台本として一覧化 | agent-env.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-tmp-hygiene.sh |  速さが要件 | 検査・起動台本として一覧化 | check-tmp-hygiene.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-db-version-stop.sh |  速さが要件 | 検査・起動台本として一覧化 | check-db-version-stop.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-ports.sh |  速さが要件 | 検査・起動台本として一覧化 | check-ports.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/usage-volume.sh |  速さが要件 | 検査・起動台本として一覧化 | usage-volume.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/load-repeat.sh |  速さが要件 | 検査・起動台本として一覧化 | load-repeat.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/export-scaling.sh |  速さが要件 | 検査・起動台本として一覧化 | export-scaling.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/tmp-leak-check.sh |  速さが要件 | 検査・起動台本として一覧化 | tmp-leak-check.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-test-db-killed.sh |  速さが要件 | 検査・起動台本として一覧化 | check-test-db-killed.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/smoke.d/202610241500_st29-satellite-credentials.sh |  条件の待ち | 既存台本の該当行だけを 30 秒以上へ整理 | 縦串 202610241500_st29-satellite-credentials.sh の回数×sleep は台本の既存行で判定し、末尾に Scenario を追加しない。 |
|  tools/check-panic-log.sh |  速さが要件 | 検査・起動台本として一覧化 | check-panic-log.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-test-db-growth.sh |  速さが要件 | 検査・起動台本として一覧化 | check-test-db-growth.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-openapi.sh |  速さが要件 | 検査・起動台本として一覧化 | check-openapi.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/smoke.d/202610251200_st23-purge.sh |  条件の待ち | 既存台本の該当行だけを 30 秒以上へ整理 | 縦串 202610251200_st23-purge.sh の回数×sleep は台本の既存行で判定し、末尾に Scenario を追加しない。 |
|  tools/check-offsite.sh |  速さが要件 | 検査・起動台本として一覧化 | check-offsite.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/stack.sh |  速さが要件 | 検査・起動台本として一覧化 | stack.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-contract-version.sh |  速さが要件 | 検査・起動台本として一覧化 | check-contract-version.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-db-secret.sh |  速さが要件 | 検査・起動台本として一覧化 | check-db-secret.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-no-time-server.sh |  速さが要件 | 検査・起動台本として一覧化 | check-no-time-server.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/docker-tidy.sh |  速さが要件 | 検査・起動台本として一覧化 | docker-tidy.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/smoke.d/202610161200_st08-browser-history.sh |  条件の待ち | 既存台本の該当行だけを 30 秒以上へ整理 | 縦串 202610161200_st08-browser-history.sh の回数×sleep は台本の既存行で判定し、末尾に Scenario を追加しない。 |
|  tools/smoke.sh |  条件の待ち | 既存台本の該当行だけを 30 秒以上へ整理 | 縦串 smoke.sh の回数×sleep は台本の既存行で判定し、末尾に Scenario を追加しない。 |
|  tools/check-build-stamp.sh |  速さが要件 | 検査・起動台本として一覧化 | check-build-stamp.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/st25-seed-check.sh |  速さが要件 | 検査・起動台本として一覧化 | st25-seed-check.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-api-token-reads.sh |  速さが要件 | 検査・起動台本として一覧化 | check-api-token-reads.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-migrations.sh |  速さが要件 | 検査・起動台本として一覧化 | check-migrations.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/smoke.d/202610212100_st13-archive-hands-off.sh |  条件の待ち | 既存台本の該当行だけを 30 秒以上へ整理 | 縦串 202610212100_st13-archive-hands-off.sh の回数×sleep は台本の既存行で判定し、末尾に Scenario を追加しない。 |
|  tools/seed.sh |  速さが要件 | 検査・起動台本として一覧化 | seed.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-licenses.sh |  速さが要件 | 検査・起動台本として一覧化 | check-licenses.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-log-private.sh |  速さが要件 | 検査・起動台本として一覧化 | check-log-private.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/web-login.sh |  速さが要件 | 検査・起動台本として一覧化 | web-login.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/smoke.d/202610141130_st14-outage-alert.sh |  条件の待ち | 既存台本の該当行だけを 30 秒以上へ整理 | 縦串 202610141130_st14-outage-alert.sh の回数×sleep は台本の既存行で判定し、末尾に Scenario を追加しない。 |
|  tools/record-env.sh |  速さが要件 | 検査・起動台本として一覧化 | record-env.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/ci-db-env.sh |  速さが要件 | 検査・起動台本として一覧化 | ci-db-env.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/wait-android-boot.sh |  速さが要件 | 検査・起動台本として一覧化 | wait-android-boot.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-handoff-reaches-brief.sh |  速さが要件 | 検査・起動台本として一覧化 | check-handoff-reaches-brief.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/android-media-fixtures.sh |  速さが要件 | 検査・起動台本として一覧化 | android-media-fixtures.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/db-roles.sh |  速さが要件 | 検査・起動台本として一覧化 | db-roles.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/archive-shape.sh |  速さが要件 | 検査・起動台本として一覧化 | archive-shape.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-immutable.sh |  速さが要件 | 検査・起動台本として一覧化 | check-immutable.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-satellite-boundary.sh |  速さが要件 | 検査・起動台本として一覧化 | check-satellite-boundary.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/break-check.sh |  速さが要件 | 検査・起動台本として一覧化 | break-check.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |
|  tools/check-shared-appends.sh |  速さが要件 | 検査・起動台本として一覧化 | check-shared-appends.sh の timeout/sleep は製品試験の待ちではなく、検査または起動手順の制限である。 |

## Task 12 の探した範囲・探していない範囲

探した範囲は crates/**/*.rs、web/src/__tests__、web/e2e、web/playwright.config.ts、
collector-android/app/src/test、collector-android/app/src/androidTest、tools/*.sh、tools/smoke.d/*.sh
および録画の tools/recording である。録画は一覧だけに載せ、合否を出す試験としては扱わない。
これらの外側（依存ライブラリ、生成物、target、node_modules、端末・Windows 側の clone）は探していない。

## Task 12 の製品の不具合の疑い

無し。候補は試験・補助・台本の待ちであり、製品コードの変更は行っていない。
