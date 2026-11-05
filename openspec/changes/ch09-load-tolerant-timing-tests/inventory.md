# CH09 の洗い出しの一覧（design D14）

`check-inventory.sh --list` が拾う全ファイルと、design の「既知の候補」を 1 つの表にまとめる。場所の欄の先頭は `path` か `path:行`（行番号は HEAD の値）。
型は D14 の 8 つ（待たない / 条件の待ち / 固定の眠り / 打ち切り / 計算の形 / 起きない・下限 / 暗黙の上限 / 速さが要件）。
`check-inventory.sh` は、候補を持つファイルが全部この表に出ること、場所のパスが実在すること、条件の待ちとして直した行と試験のコード全体に
回数の待ち・30 秒より短い上限が `timing-ok:` の理由なしに残っていないことを見る。

| 場所（試験・補助） | 型 | 方針 | 理由 |
|---|---|---|---|
| `crates/server/src/scheduled_tests.rs:102` `wait_for_calls`・`:120` `wait_for_log`（4 件のログ待ち） | 条件の待ち | 実時間 30 秒の期限へ変えた | 止めた Tokio の時計の刻みは変えず、回数の上限だけを実時間の期限にした（D2・D4）。文言に待った時間と最後に見た値を出す。 |
| `crates/server/src/scheduled_tests.rs:573` `scheduled_retired_source_is_not_fetched` | 条件の待ち | 実時間 30 秒の期限へ変えた | 枠の task が終わるのを 40 回 × 25ms（実時間 1 秒）で待っていた。印の理由（短い下限）は事実と違ったので、期限 30 秒の条件の待ちにした（final review R4）。取りに行かないことは `probe.calls()` で見る。 |
| `crates/server/src/outage_api_tests.rs:419` 台帳の錠の待ち | 条件の待ち | 実時間 30 秒の期限へ変えた | DB の待ちの数を観測でき、最後に見た待ちの数を文言に出す。 |
| `crates/server/src/archive_scan_interval_tests.rs:157` `wait_for_scans`・`:176` `wait_for_scans_or_panic`・`:278` 間隔の変更の歩 | 条件の待ち | 実時間 30 秒の期限・届かなければ進め直す | 走査の数を観測できる。競り合いは同じ歩を進め直して吸収し、時計の経過を 30 秒未満で見る（D2・D4）。 |
| `crates/server/src/archive_scan_interval_tests.rs:271` `settle(500)` | 固定の眠り | 直さない | 取り込み器が待ちの眠りに入ったことを観測する口が無い（製品のコードに口を足さない。Q0・D5）。 |
| `crates/server/src/archive_flow_tests.rs:6019` `archive_ledger_atomic_source_row_failure_has_its_own_log_kind` | 条件の待ち | 実時間 30 秒の期限へ変えた | ログの行を 400 回 × 50ms（20 秒）で待っていた。期限 30 秒の条件の待ちにし、文言に待った時間と最後に見たログの行数を出す（final review R5）。 |
| `crates/server/src/purge_compaction_tests.rs:206` 書き直しの前提のスナップショット待ち | 条件の待ち | 回数のまま（1500 回 × 20ms = 30 秒） | 回数 × 眠りがもう 30 秒あり、30 秒より短い上限ではない（`check-inventory.sh` が計算して見る）。 |
| `crates/server/src/purge_compaction_tests.rs:242` 消去が錠を待つまでの待ち | 条件の待ち | 実時間 30 秒の期限へ変えた | 500 回 × 20ms（10 秒）で待っていた（前の一覧は「30 秒超を維持」と誤記）。期限 30 秒の条件の待ちにした（final review R5）。 |
| `crates/server/src/export_job_tests.rs:92` `wait_idle`・`:197` `wait_until_blocked` | 条件の待ち | 回数のまま（3000 回 × 100ms = 300 秒） | 回数 × 眠りが 30 秒を超え、30 秒より短い上限ではない。 |
| `crates/server/src/testdb.rs:297` `create_from_template` | 条件の待ち | 実時間 30 秒の期限へ変えた | `testdb.rs` は `#[cfg(test)]` の試験の足場（製品のコードではない）。55006 の打ち直しを 100 回 × 50ms（5 秒）で切っていたのを期限 30 秒にした（final review R5）。 |
| `crates/server/src/testdb.rs:410` `TestDb::drop` の `DROP DATABASE` のやり直し | 条件の待ち | 実時間 30 秒の期限へ変えた | 倍々の間隔はそのまま、8 回（約 9.5 秒）で切っていたのを期限 30 秒にした（`check-inventory.sh` の広げた拾い方で見つけた）。 |
| `crates/server/src/testdb_tests.rs:370` `ch04_statement_timeout_cancels_a_long_query` | 打ち切り | `pg_sleep(60)`・30 秒未満へ（相手と上限を一緒に） | 打ち切りの相手と上限を一緒に延ばした（D6。正典 `test-database` の MODIFIED）。 |
| `crates/server/src/stay_tests.rs:2384` | 計算の形 | 直さない | 作り直しの計算の形を実時間で見ている。製品のコードを変えずに数で見る口が無い（Q0・D6）。 |
| `crates/server/src/web_session_tests.rs:271` | 起きない・下限 | 直さない | 300ms 以上かかる下限を見る試験（D12）。 |
| `crates/collector-windows/src/runtime.rs:3672` `history_large_fetch_keeps_observing_foreground` | 待たない | 止めた読み ＋ 見張り ＋ 1 回に積む件数に書き換えた | 見回り 1 回の実時間の比較をやめた（D1）。 |
| `crates/collector-windows/src/runtime.rs:1962` `clock_worker_slow_read_does_not_hold_the_patrol` | 待たない | 経過の比較を見張りに替えた | 止めた読み手（`SlowReference`）はそのまま、「30 秒未満」の比較を D1 の見張りの形に替えた。 |
| `crates/collector-windows/src/runtime.rs:2497` `HistoryRuntime::drop`・`:2553` `tick_until_history_idle`・`:2587` `tick_until_history_beats` | 条件の待ち | 実時間 30 秒の期限へ変えた | 履歴の worker の終了・`is_reading`・生存信号の件数を観測でき、文言に最後に見た値を出す（D4）。 |
| `crates/collector-windows/src/runtime.rs:2539` `tick_until_read`・`:3519` `tick_until_read_again`・`:4232` やり直しの取得の待ち | 条件の待ち（時刻を兼ねる） | 回数のまま（`// timing-ok:`） | 1 回ごとに `t(sec)` を進めるループで、期限に変えると時刻が進みすぎる（D4 の除く・Q5）。`:4232` の印は final review R11 で付けた。 |
| `crates/collector-windows/src/history/collect.rs:859`・`:967` 履歴の取得の待ち | 条件の待ち | 実時間 30 秒の期限へ変えた | `#[cfg(test)] mod tests` の中の試験のコード。取得の結果と積み込みの状態を観測できる（D4）。 |
| `crates/collector-windows/src/time_sync.rs:649` `time_sync_timeout_kills_the_child` | 打ち切り | `sleep 60`・30 秒未満へ（相手と上限を一緒に） | `#[cfg(test)] mod tests` の中の試験のコード。打ち切りの相手と上限を一緒に延ばした（D6）。 |
| `crates/collector-windows/tests/runtime_windows.rs:136` `HelperWindow::drop`・`:419` 題名の変化・`:963` `Edge::drop`・`:1243` `trigger`・`:1446` `HistoryBrowser::drop` | 固定の眠り | `// timing-ok:` を付けて残した | 相手役の終了・題名の変化・プロファイルの解放・問い合わせの完了を観測する口が無い（D5）。 |
| `crates/collector-windows/tests/runtime_windows.rs:248` `run_for` | 起きない・下限 | 直さない（`// timing-ok:`） | 決まった秒数だけ見続ける窓そのもの（D3・D12）。 |
| `crates/collector-windows/tests/runtime_windows.rs:225` `observe_until`・`:260` `run_until`・`:195` `focus_window_of`・`:877` `wait_loaded`・`:1225` `wait_history`・`:718` 時計のずれの測定待ち | 条件の待ち | 実時間 30 秒の期限へ変えた・期限を過ぎたら落ちる | 期限を過ぎても黙って返っていた `observe_until`・`wait_history`・時計のずれの待ちを、待った時間と最後に見た値を出して落ちるようにした（final review R13）。 |
| `crates/collector-windows/tests/runtime_windows.rs:1463` `fetch_history_until` | 条件の待ち | 実時間 90 秒の期限のまま | ブラウザの履歴の遅れと 25 時間刻みを含む、30 秒を超える上限。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt:54`・`:56`・`:79`・`:83`・`:90`・`:98` | 条件の待ち | 実時間 30 秒の期限へ変えた | ログの到着を条件として待つ（D4）。糸の中の失敗は `Future` で呼んだ側へ伝える（final review R13）。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt:95` | 起きない・下限 | 直さない（`// timing-ok:`） | ログ中に復旧が割り込まないことを見る 200ms の窓（D12）。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthNotificationTest.kt:158` | 打ち切り | 上限を 30 秒へ（相手は返らない） | 相手（`awaitCancellation()`）は返らないので相手はもう十分長い。確かめの上限 10 秒を 30 秒にそろえた（D6。final review R6）。試験が渡す打ち切り 100ms は変えない。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/FeelingReminderReceiverTest.kt:374` | 打ち切り | 相手 60 秒・上限 30 秒へ（相手と上限を一緒に） | 相手（本文を 1 バイトずつ送る）を合計 6 秒から 60 秒へ、確かめの上限を 3 秒から 30 秒へ延ばした（D6。final review R6）。試験が渡す打ち切り `timeoutMs = 1_000` と製品の既定値は変えない（Q0）。 |
| `collector-android/app/src/androidTest` の `*_TIMEOUT_MS`・`NOTICE_WAIT_MS`・`SCREEN_TIMEOUT_MS`・`SHADE_TIMEOUT_MS`・`Tasks.await`・`CountDownLatch.await`、条件で抜ける `SETTLE_MS` | 条件の待ち | 30 秒以上へ変えた | 端末の状態が成り立った時点で抜ける待ち（D17）。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/UsageAccessInstrumentedTest.kt:158` `SETTLE_MS`・`HealthConnectScreens.kt:103` タップごとの `device.wait` | 起きない・下限 | 直さない（`// timing-ok:`） | 設定の画面が開かないことを見続ける窓・タップごとに遷移を確かめる操作の単位（D12・D17）。 |
| `web/playwright.config.ts`・`web/e2e/*.spec.ts` | 暗黙の上限 | 待ち 30 秒・試験 120 秒へそろえた | Playwright の既定と個別の上限を設定へ寄せた（D15）。 |
| `web/e2e/feel.spec.ts:208`・`:290` | 固定の眠り | 条件の待ちへ変えた・録画に要る待ちは残した | 表示の条件に置き換えられるものと録画の都合の待ちを分けた（D5・D15）。 |
| `web/e2e/day-erase.spec.ts:70` | 起きない・下限 | 直さない（`// timing-ok:`） | 任意の操作を飛ばすための短い上限で、成り立つ条件の待ちではない。 |
| `web/e2e/places.spec.ts:37` | 暗黙の上限 | 120 秒へ変えた | 画面の e2e の試験の上限を設定の値にそろえた。 |
| `web/src/__tests__` | 暗黙の上限 | 設定で一括して上げた | `waitFor` / `findBy` の既定の上限を設定で上げた（D9）。各ファイルは下の行。 |
| `tools/smoke.sh:51` | 条件の待ち | 回数のまま（60 回 × 1 秒。`# timing-ok:`） | もう 30 秒以上ある起動の待ち（D16）。 |
| `tools/smoke.sh:173`・`:194`・`:218` | 条件の待ち | `SECONDS` の期限 30 秒へ変えた | 15 回 × 1 秒の待ちを期限 30 秒にし、文言に待った時間と最後に見た値を出す（D16）。 |
| `tools/smoke.sh:205` | 起きない・下限 | 直さない（`# timing-ok:`） | 件数が増えないことを見る 3 秒の窓（D12）。 |
| `tools/check-panic-log.sh` | 条件の待ち | ログの待ちを期限 30 秒へ変えた・起動の待ちは回数のまま | 固定の `sleep 2` をログの行が出るまでの期限 30 秒の待ちにした。起動の待ちは 60 回 × 1 秒（`# timing-ok:`）。 |
| `tools/web-login.sh` | 条件の待ち | `SECONDS` の期限 30 秒へ変えた | self-test の port のファイルを 100 回 × 0.01 秒で待っていたのを期限 30 秒にした。 |
| `tools/load-repeat.sh` | 条件の待ち | 期限 30 秒（この change の台本） | DB の負荷が立つのを実時間の期限 30 秒で待つ。負荷の sh の数え方は自分の argv に当たらない（final review R8）。 |
| `tools/break-check.sh` | 打ち切り | 上限は引数（既定 900 秒。この change の台本） | 壊した木の命令を上限で殺す。殺した（124 / 137）と命令自身の rc を経過時間で分ける（final review R9）。 |
| `tools/android-emulator.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 回数 × 眠りがもう 30 秒以上ある起動・接続・取り込みの待ちで、混み具合では長くなる側へだけ外れる（D16）。 |
| `tools/android-media-fixtures.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 回数 × 眠りがもう 30 秒以上ある起動・接続・取り込みの待ちで、混み具合では長くなる側へだけ外れる（D16）。 |
| `tools/check-verify-env-reset.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 回数 × 眠りがもう 30 秒以上ある起動・接続・取り込みの待ちで、混み具合では長くなる側へだけ外れる（D16）。 |
| `tools/record-env.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 回数 × 眠りがもう 30 秒以上ある起動・接続・取り込みの待ちで、混み具合では長くなる側へだけ外れる（D16）。 |
| `tools/st25-seed-check.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 回数 × 眠りがもう 30 秒以上ある起動・接続・取り込みの待ちで、混み具合では長くなる側へだけ外れる（D16）。 |
| `tools/stack.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 回数 × 眠りがもう 30 秒以上ある起動・接続・取り込みの待ちで、混み具合では長くなる側へだけ外れる（D16）。 |
| `tools/verify-env.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 回数 × 眠りがもう 30 秒以上ある起動・接続・取り込みの待ちで、混み具合では長くなる側へだけ外れる（D16）。 |
| `crates/collector-windows/examples/browser_history_smoke.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | `examples/` の手で走らせる道具（製品の扱い。Q0）。合否を出す試験ではない。 |
| `crates/collector-windows/src/history/fetch.rs` | 条件の待ち | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/collector-windows/src/platform.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/collector-windows/src/sender.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/collector-windows/src/clock.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/collector-windows/src/clock_worker.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/server/src/outage.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/server/src/access_log_tests.rs` | 条件の待ち | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/access_migration_tests.rs` | 条件の待ち | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/archive_tests.rs` | 条件の待ち | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/attributes_tests.rs` | 条件の待ち | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/deletion_tests.rs` | 条件の待ち | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/feelings_ingest_tests.rs` | 条件の待ち | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/places_tests.rs` | 条件の待ち | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/stay_store.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/server/src/web_session.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/server/src/lib.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `crates/server/tests/server_startup.rs` | 暗黙の上限 | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/export_file_tests.rs` | 条件の待ち | 直すものは無い | 回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。待ちは実時間の期限か、30 秒以上の回数 × 眠り。 |
| `crates/server/src/archive/worker.rs` | 暗黙の上限 | 一覧だけ（製品のコード） | 製品のコードの時間の扱い（Q0 で変えない）。同じファイルの `#[cfg(test)] mod tests` に回数の待ち・30 秒未満の上限は無い（`check-inventory.sh` が試験のコードを全部見る）。 |
| `tools/recording/st22/st22-phone-dark.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st22-phone-dark.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st22/st22-erase-reload.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st22-erase-reload.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/satellites.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 satellites.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st08/st08-result.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st08-result.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/e2e/settings-export.spec.ts` | 暗黙の上限 | 設定の待ち 30 秒・試験 120 秒へそろえた（D15） | 本物のブラウザで走る settings-export.spec.ts の個別の短い上限を設定へ寄せた。残す短い上限は `// timing-ok:` を付けた。 |
| `web/src/__tests__/places-change.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 places-change.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st13/st13-sources.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st13-sources.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/retired-source.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 retired-source.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st25/st25-day-timeline.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st25-day-timeline.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/session.test.ts` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 session.test.ts の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st33/st33-export.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st33-export.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/e2e/feel-parts.spec.ts` | 暗黙の上限 | 設定の待ち 30 秒・試験 120 秒へそろえた（D15） | 本物のブラウザで走る feel-parts.spec.ts の個別の短い上限を設定へ寄せた。残す短い上限は `// timing-ok:` を付けた。 |
| `tools/recording/st17/st17-feel.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st17-feel.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st23/st23-purge.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st23-purge.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/e2e/feel-ch01.spec.ts` | 暗黙の上限 | 設定の待ち 30 秒・試験 120 秒へそろえた（D15） | 本物のブラウザで走る feel-ch01.spec.ts の個別の短い上限を設定へ寄せた。残す短い上限は `// timing-ok:` を付けた。 |
| `web/src/__tests__/master-view-limits.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 master-view-limits.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/app.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 app.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st50/st50-result.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st50-result.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/e2e/st49-client-version.spec.ts` | 暗黙の上限 | 設定の待ち 30 秒・試験 120 秒へそろえた（D15） | 本物のブラウザで走る st49-client-version.spec.ts の個別の短い上限を設定へ寄せた。残す短い上限は `// timing-ok:` を付けた。 |
| `web/src/__tests__/st25-open.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 st25-open.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/e2e/day-timeline.spec.ts` | 暗黙の上限 | 設定の待ち 30 秒・試験 120 秒へそろえた（D15） | 本物のブラウザで走る day-timeline.spec.ts の個別の短い上限を設定へ寄せた。残す短い上限は `// timing-ok:` を付けた。 |
| `web/src/__tests__/FeelView.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 FeelView.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/ch08/ch08-nested.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 ch08-nested.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st14/st14-coverage.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st14-coverage.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/st25-gaps.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 st25-gaps.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/master-form.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 master-form.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st28/st28-login.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st28-login.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st29/st29-satellites.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st29-satellites.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/master-view.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 master-view.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/DayFeeling.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 DayFeeling.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/st25-summary.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 st25-summary.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st07/st07-result.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st07-result.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/places-add.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 places-add.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/FeelViewParts.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 FeelViewParts.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/DayView.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 DayView.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/ch01/ch01-feel.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 ch01-feel.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st12/st12-archive.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st12-archive.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/DayView-erase-erased-row-keyboard.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 DayView-erase-erased-row-keyboard.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st21/st21-places.rec.ts` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 st21-places.rec.ts は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `web/src/__tests__/one-scroll.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 one-scroll.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/places-view.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 places-view.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/archive-one-scroll.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 archive-one-scroll.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/st25-routes.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 st25-routes.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/src/__tests__/places-tabs.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 places-tabs.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `web/e2e/st23-purge.spec.ts` | 暗黙の上限 | 設定の待ち 30 秒・試験 120 秒へそろえた（D15） | 本物のブラウザで走る st23-purge.spec.ts の個別の短い上限を設定へ寄せた。残す短い上限は `// timing-ok:` を付けた。 |
| `web/src/__tests__/SettingsView.test.tsx` | 暗黙の上限 | 設定で一括して上げた（D9） | 画面の単体試験 SettingsView.test.tsx の `waitFor` / `findBy` の待ち。Scenario の担保は web/e2e に置く。 |
| `tools/recording/st13/St13RecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St13RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st06/St06LaterGrantRecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St06LaterGrantRecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st06/St06RecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St06RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st17/St17RecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St17RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HttpTransportInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る HttpTransportInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/FeelingReminderInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る FeelingReminderInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `tools/recording/st11/St11RecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St11RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st09/St09RecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St09RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaCaptureInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る MediaCaptureInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/PermissionDeniedInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る PermissionDeniedInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/RetentionInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る RetentionInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/LocationServiceInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る LocationServiceInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/ClockSkewInstrumentedTest.kt:75` `waitFor` | 条件の待ち | 直すものは無い | 実時間の期限（既定 30 秒）の待ちがもう入っている（D17）。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthPermissionGrantInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | `device.wait` の上限は `HealthConnectScreens.SCREEN_TIMEOUT_MS`（30 秒へ変えた）を使う。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/UnusedAppRestrictionsInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る UnusedAppRestrictionsInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaPremisesInstrumentedTest.kt:166` | 速さが要件 | 直さない | `System.nanoTime()` は写真のファイル名を一意にするためで、待ちではない（拾う形に当たっただけ）。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/ForegroundServiceTypeInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る ForegroundServiceTypeInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MainActivityInstrumentedTest.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る MainActivityInstrumentedTest.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/RollupImportTest.kt` | 暗黙の上限 | 直すものは無い | RollupImportTest.kt の時間の扱いは時計の読み・固定の時刻で、30 秒未満の上限は無い（`check-inventory.sh` が見る）。 |
| `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthConnectScreens.kt` | 条件の待ち | 30 秒以上へ変えた（D17） | エミュレータの上で走る HealthConnectScreens.kt の、端末の状態が成り立つまでの待ち。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthPermissionFlowTest.kt:533` 読みが返らないときの収集の始まりの待ち | 条件の待ち | 実時間 30 秒の期限へ変えた | 10 秒の期限を 30 秒にし、文言に待った時間と最後に見た値を出す（D4）。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/LocationServiceClockTest.kt` | 暗黙の上限 | 直すものは無い | LocationServiceClockTest.kt の時間の扱いは時計の読み・固定の時刻で、30 秒未満の上限は無い（`check-inventory.sh` が見る）。 |
| `tools/recording/st05/St05RecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St05RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `collector-android/app/src/test/kotlin/dev/ashiato/collector/AppUsageFixtures.kt` | 暗黙の上限 | 直すものは無い | AppUsageFixtures.kt の時間の扱いは時計の読み・固定の時刻で、30 秒未満の上限は無い（`check-inventory.sh` が見る）。 |
| `tools/recording/st14/St14RecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St14RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/recording/st50/St50RecordingTest.kt` | 暗黙の上限 | 一覧だけ（別 change） | 録画の台本 St50RecordingTest.kt は合否を出す試験ではなく、待ちを含む場所だけを記録する。 |
| `tools/check-exposure.sh` | 速さが要件 | 直さない | check-exposure.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-released-migrations.sh` | 速さが要件 | 直さない | check-released-migrations.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/verify-prep.sh` | 速さが要件 | 直さない | verify-prep.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/ports.sh` | 速さが要件 | 直さない | ports.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-private.sh` | 速さが要件 | 直さない | check-private.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/pg_dump-docker.sh` | 速さが要件 | 直さない | pg_dump-docker.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/agent-env.sh` | 速さが要件 | 直さない | agent-env.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-tmp-hygiene.sh` | 速さが要件 | 直さない | check-tmp-hygiene.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-db-version-stop.sh` | 速さが要件 | 直さない | check-db-version-stop.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-ports.sh` | 速さが要件 | 直さない | check-ports.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/usage-volume.sh` | 速さが要件 | 直さない | usage-volume.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/export-scaling.sh` | 速さが要件 | 直さない | export-scaling.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/tmp-leak-check.sh` | 速さが要件 | 直さない | tmp-leak-check.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-test-db-killed.sh` | 速さが要件 | 直さない | check-test-db-killed.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/smoke.d/202610241500_st29-satellite-credentials.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 起動・取り込みの待ちは回数 × 眠りがもう 30 秒以上ある（D16）。台本の末尾に節や印を足していない。 |
| `tools/check-test-db-growth.sh` | 速さが要件 | 直さない | check-test-db-growth.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-openapi.sh` | 速さが要件 | 直さない | check-openapi.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/smoke.d/202610251200_st23-purge.sh` | 速さが要件 | 直さない | `for i in $(seq 0 20)` は投入するデータを組み立てる回数で、眠りも待ちも無い（拾う形に当たっただけ）。 |
| `tools/check-offsite.sh` | 速さが要件 | 直さない | check-offsite.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-contract-version.sh` | 速さが要件 | 直さない | check-contract-version.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-db-secret.sh` | 速さが要件 | 直さない | check-db-secret.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-no-time-server.sh` | 速さが要件 | 直さない | check-no-time-server.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/docker-tidy.sh` | 速さが要件 | 直さない | docker-tidy.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/smoke.d/202610161200_st08-browser-history.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 起動・取り込みの待ちは回数 × 眠りがもう 30 秒以上ある（D16）。台本の末尾に節や印を足していない。 |
| `tools/check-build-stamp.sh` | 速さが要件 | 直さない | check-build-stamp.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-api-token-reads.sh` | 速さが要件 | 直さない | check-api-token-reads.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-migrations.sh` | 速さが要件 | 直さない | check-migrations.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/smoke.d/202610212100_st13-archive-hands-off.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 起動・取り込みの待ちは回数 × 眠りがもう 30 秒以上ある（D16）。台本の末尾に節や印を足していない。 |
| `tools/seed.sh` | 速さが要件 | 直さない | seed.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-licenses.sh` | 速さが要件 | 直さない | check-licenses.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-log-private.sh` | 速さが要件 | 直さない | check-log-private.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/smoke.d/202610141130_st14-outage-alert.sh` | 条件の待ち | 回数のまま（30 秒以上。`# timing-ok:`） | 起動・取り込みの待ちは回数 × 眠りがもう 30 秒以上ある（D16）。台本の末尾に節や印を足していない。 |
| `tools/ci-db-env.sh` | 速さが要件 | 直さない | ci-db-env.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/wait-android-boot.sh` | 速さが要件 | 直さない | wait-android-boot.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-handoff-reaches-brief.sh` | 速さが要件 | 直さない | check-handoff-reaches-brief.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/db-roles.sh` | 速さが要件 | 直さない | db-roles.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/archive-shape.sh` | 速さが要件 | 直さない | archive-shape.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-immutable.sh` | 速さが要件 | 直さない | check-immutable.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-satellite-boundary.sh` | 速さが要件 | 直さない | check-satellite-boundary.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |
| `tools/check-shared-appends.sh` | 速さが要件 | 直さない | check-shared-appends.sh の待ち・上限は検査や起動の手順の制限で、試験の条件の待ちではない。 |

Windows の実行時テストは `cargo test -p ashiato-collector-windows --test runtime_windows -- --test-threads=1` を Windows で走らせ、
`test result: ok. 15 passed; 0 failed`（2026-11-05、evidence fingerprint `ae55e14e`）を得た。final review R13 の直し（期限を過ぎたら落ちる）の後は Windows で走らせていない。

## 探した範囲・探していない範囲

探した範囲は `crates/**/*.rs`、`web/src/__tests__`、`web/e2e`、`web/playwright.config.ts`、`collector-android/app/src/test`、
`collector-android/app/src/androidTest`、`tools/*.sh`、`tools/smoke.d/*.sh`、録画の `tools/recording` である。録画は一覧にだけ載せ、合否を出す試験としては扱わない。
これらの外側（依存のライブラリ、生成物、`target`、`node_modules`、端末や Windows の側の clone）は探していない。

## 製品の不具合の疑い

無し。候補は試験・試験の足場・台本の待ちで、製品のコードは変えていない（Q0。`check-product-unchanged.sh`）。
