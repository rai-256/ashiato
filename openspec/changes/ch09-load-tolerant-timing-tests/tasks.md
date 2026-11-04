# CH09 実装タスク — 実時間で「〜秒以内」を求める試験を混み具合で落ちない形にする

読む順: `deep.md`（**本人の答え Q0〜Q10**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/test-timing/spec.md`・`specs/external-ingestion/spec.md`・
`specs/test-database/spec.md` → `design.md` → `premises.md` → `review/deep.md`（R1 の競り合い・R2 の FIFO）→ `docs/changes/CH09.md` → `CLAUDE.md`。

**製品のコードは変えない**（Q0。`#[cfg(test)] mod tests` の外は 0 行。Task 13 の検査が見る）。**既存の試験の名前・`Scenario:` の印・本数は減らさない**（design D13）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検証は `./tools/db.sh up -d --wait db && ./tools/db-roles.sh` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 履歴の読みを止めている間も、見回りは返り、前景の変化が積まれる`）、bash は `echo "Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 24 本**（`test-timing` の ADDED 5 つ: 3・4・4・4・2 本 = 17 本、`external-ingestion` の MODIFIED 1 つ: 4 本、`test-database` の MODIFIED 1 つ: 3 本）。
  MODIFIED の 7 本は既存の試験に印がある（印を消さない）。ADDED の 17 本は、このファイルのどこかの Task が名指しする
- **正典の Scenario の印の付いた試験の期待の行（assert）を書き換えるときは**、MODIFIED に無い Requirement なら `agreed-test-changes.md`（change の中）に
  `- Scenario: <名前> — <理由>` を書く（design D13。Task gate の `check_agreed_tests.py` が見る）
- **件数つき検証**: このファイルで
  - **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0
  - **`CW <名前>`** は `scripts/quiet-run cw -- cargo test -p ashiato-collector-windows --lib <名前>` rc=0（DB は要らない）
  - **`NL <crate> <名前> <下限>`** は `bash -c 'set -- <crate> <名前> <下限>; [ "$(cargo test -p "$1" --lib "$2" -- --list 2>/dev/null | grep -c ": test$")" -ge "$3" ]'` rc=0（`cargo test` は 0 本でも rc=0 になるので、本数の下限を別に見る）
  - **`KT <絞り込み>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --rerun --tests '<絞り込み>'` rc=0
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **本人の決定（下流は変えない）**: Q0 製品のファイルに試験用の口を足さない / Q1 止めた読み ＋ 1 回に積む件数 / Q2 全部探して全部直す（関門で走る 3 か所に加え、Windows の実行時テスト・画面の e2e・携帯の計測テスト・縦串と検査の台本。design D3・D15〜D17。**本人の答えで、推奨と違う**） /
  Q3 計算の形は直さず、打ち切りは相手と上限を一緒に延ばす / Q4 観測できる条件があれば条件の待ち / Q5 実時間の期限で一律 30 秒・時刻を兼ねるループは回数のまま /
  Q6 `tools/` の台本・直す前に再現を残す / Q7 製品の不具合は直さず一覧に / Q8 届かなければ進め直す・経過 30 秒未満（推奨の「例: 60 秒」を 30 秒にした。design D2） / Q9 画面は待ち 10 秒・試験 30 秒 /
  Q10 使い捨ての作業木に壊す差分を当てる台本（第 1 回は推奨で先に書き、推奨と違った Q2 を 2026-11-04 に写した）
- **1 の試験と 2 の試験の名前は変えない**（正典の Scenario の印がある。design D13）。上限を緩めるだけ（1 秒 → 5 秒）の直しはしない（依頼の 1）
- **台本は `/tmp` の固定名に書かない**（`"$TMPDIR"` か `mktemp`）。負荷の busy loop は `trap` で必ず止める（design D7）
- **広げた範囲の確かめは重い（Q2）**: `tools/smoke.sh` は先頭で DB を作り直すので、`cargo test`・`break-check.sh`・`load-proof.sh` と並べて走らせない（`docs/testing.md` §4）。
  計測テスト（`tools/android-emulator.sh`）は 20 分前後かかる。どれも `scripts/quiet-run` で 1 回で待つ
- **縦串の台本（`tools/smoke.sh`・`tools/smoke.d/*.sh`）は、ある行を直すだけ**。末尾に節や見出し・`Scenario:` の印を足さない（`tools/check-shared-appends.sh`。design D16）
- 走っている st53・st55 のブランチには触らない。各ファイルの 1 行目（SPDX の行）は変えない（proposal の Impact）

## Task 1: 負荷の台本と、直す前の再現

- [x] 1.1 `tools/load-repeat.sh` を design D7 のとおりに書く: `[--times N] [--load host|db|none] [--expect-fail] -- <命令…>`・`--self-test`。冒頭のコメントに使い方・rc・なぜ要るか（CH09）を書く。
  `--self-test` は `--load none` と `--load host` の両方で偽の命令を使い、次の 2 つを見る。それぞれの前に印を出す:
  `echo "Scenario: 負荷の台本は、全部通れば通り、1 回でも落ちれば何回目かを挙げて落ちる"`（必ず通る命令 5 回 → rc=0 と `pass 5/5`、3 回目だけ落ちる命令 5 回 → rc=1 と 3 回目。
  どちらの後にも、台本の負荷の過程が `pgrep -P` で 0 本）/
  `echo "Scenario: 落ちることを期待する指定では、落ちる回が無ければ台本が落ちる"`（`--expect-fail` で必ず通る 3 回 → rc=1、2 回目だけ落ちる 3 回 → rc=0）/
  `echo "Scenario: 負荷の台本を途中で止めても、かけた負荷は残らない"`（`--load db` と `--load host` で 60 秒眠る命令を背景で走らせ、負荷が立ったのを見てから INT を送る →
  台本が終わり、手元の子と DB のコンテナの中の印 `ch09-load-<pid>` の過程が 0 本。DB のコンテナが無ければ self-test は rc=1 で「DB が要る」と出す）。
  検証: `tools/load-repeat.sh --self-test` rc=0
- [x] 1.2 change の中に `load-proof.sh` を書く: `before` は 1 の試験（`history_large_fetch_keeps_observing_foreground`）と 2 の試験（`archive_scan_interval_change_takes_effect_without_restart`）に
  `tools/load-repeat.sh --expect-fail --times 10 --load host` を当て、2 で再現しなければ `--load db` でもう一度当て、`load-before.txt`（change の中）に `history: yes|no` / `scan: yes|no` と、
  当てた負荷・各回の rc を書く（rc は 0。結果の yes / no で台本は落ちない）。`after` は Task 13.3。**1 と 2 の試験を書き換える前（Task 3・4 の前）に `before` を走らせ、`load-before.txt` を commit する**。
  試験ごとに再現した負荷も `load-history: host|db|none` / `load-scan: host|db|none` で書く（両方で再現したら `host,db`。Task 13.3 がこの負荷で通す。design D7）。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run before -- openspec/changes/ch09-load-tolerant-timing-tests/load-proof.sh before && [ "$(grep -cE "^(history|scan): (yes|no)$" openspec/changes/ch09-load-tolerant-timing-tests/load-before.txt)" -eq 2 ] && [ "$(grep -cE "^load-(history|scan): " openspec/changes/ch09-load-tolerant-timing-tests/load-before.txt)" -eq 2 ]'` rc=0

## Task 2: 壊すと落ちることを確かめる台本

- [x] 2.1 `tools/break-check.sh` を design D11 のとおりに書く: `--patch <差分> --expect <文言> [--expect <文言>…] [--timeout 秒] -- <命令…>`・`--self-test`。`--expect` は全部が出ることを見る。
  `CARGO_TARGET_DIR` は既定で元の木の `target/` に向ける（使い捨ての木の組み立てを短くする）。使い捨ての作業木は `git worktree add --detach` で `mktemp -d` の下に作り、
  `trap` で `git worktree remove --force` と `git worktree prune` を必ず走らせる。上限は `timeout` で、殺されたら rc=1 と「上限で殺した」。
  `--self-test` は一時の git の repo（`mktemp -d` に `git init`）の上で、次の 3 つを見る。それぞれの前に印を出す:
  `echo "Scenario: 壊した木で命令が決まった文言で落ちれば、台本は通る"`（差分を当てると `exit 1` して `BROKEN` を出す命令 → rc=0）/
  `echo "Scenario: 壊しても命令が通るか、文言が違えば、台本は落ちる"`（当てても通る命令・`OTHER` を出して落ちる命令・当たらない差分 → どれも rc=1 で、それぞれ「通った」「文言が違う」「差分が当たらない」）/
  `echo "Scenario: 台本の後、元の作業木は変わらず、使い捨ての作業木は残らない"`（前後の `git status --porcelain` と `git worktree list` が同じ）/
  `echo "Scenario: 上限より長くかかる命令は殺され、台本は落ちる"`（`--timeout 2` で 60 秒眠る命令 → rc=1 と「上限で殺した」）。
  検証: `tools/break-check.sh --self-test` rc=0

## Task 3: 1 の試験 —— 止めた読み・見張り・1 回に積む件数

- [x] 3.1 `crates/collector-windows/src/runtime.rs` の `history_large_fetch_keeps_observing_foreground` を design D1 のとおりに書き換える（`FakeReader::gated`・見張り 30 秒・
  読みを止めた間の見回り 5 回以上と前景の切り替え・放した後の 1 回あたりの増え方 ≤ `QUEUE_PER_TICK`）。`slowest` と `POLL_INTERVAL_SEC` の比較は消し、ほかの assert は残す。
  既存の `// Scenario: 訪問の多い取得でも前景の観測は止まらない` は残し、`// Scenario: 履歴の読みを止めている間も、見回りは返り、前景の変化が積まれる` と
  `// Scenario: 読みを放した後、見回り 1 回で積む履歴の記録は 2,000 件以下` を足す。
  `agreed-test-changes.md` に `- Scenario: 訪問の多い取得でも前景の観測は止まらない — 見回り 1 回の実時間の比較を、止めた読みと見張りに替えた（正典の文は時間を言っていない。期待は同じ）` を書く。
  検証: `CW history_large_fetch_keeps_observing_foreground` rc=0 / `NL ashiato-collector-windows history_large_fetch_keeps_observing_foreground 1` rc=0
- [x] 3.2 **3.1 を commit してから**（break-check は HEAD から作業木を作る。design D11）、壊す差分 `break-wait-on-read.patch` を change の中に置く（design D11。`history/collect.rs` の `Stage::Reading` で読みのスレッドの終わりを待つ）。
  change の中に `break-proof.sh` を書き、`history` で `tools/break-check.sh --patch <この差分> --expect 見張りが読みを放した --timeout 900 -- cargo test -p ashiato-collector-windows --lib history_large_fetch_keeps_observing_foreground` を走らせる。
  前に `echo "Scenario: 見回りが履歴の取得を待つように壊すと、見張りの上限で試験が落ちる"` を出す。
  検証: `scripts/quiet-run break -- openspec/changes/ch09-load-tolerant-timing-tests/break-proof.sh history` rc=0
- [x] 3.3 同じ形の `clock_worker_slow_read_does_not_hold_the_patrol`（`runtime.rs:1936`。「30 秒未満」）を D1 の見張りの形に替える（読み手は既に止めてある。経過の比較を消し、
  見回りが返らなければ見張りが放して落とす）。印 `// Scenario: 基準の読み取りが長引いても前景の切り替えは記録に残る` は残す。
  `agreed-test-changes.md` に `- Scenario: 基準の読み取りが長引いても前景の切り替えは記録に残る — 「30 秒未満」の比較を見張りに替えた（期待は同じ）` を書く。
  検証: `CW clock_worker_slow_read_does_not_hold_the_patrol` rc=0 / `NL ashiato-collector-windows clock_worker_slow_read_does_not_hold_the_patrol 1` rc=0

## Task 4: 2 の試験 —— 進め直しと、時計の経過 30 秒未満

- [ ] 4.1 `crates/server/src/archive_scan_interval_tests.rs` の `archive_scan_interval_change_takes_effect_without_restart` を design D2 のとおりに書き換える（歩ごとに最大 1 秒待ち、
  届かなければ同じ歩を進め直す・時計の経過 30 秒か実時間 30 秒で落ちる・終わりは「10 回が時計の経過 30 秒未満」）。`wait_for_scans` は D4 の期限 30 秒にし、文言に `待った時間` と `最後に見た数` を出す。
  印 `// Scenario: 登録簿で見回りの間隔を変えると起動し直さずに効く` は残す（正典の文は MODIFIED で書き直してある）。`settle(500)`・`settle(25)`・`advance_secs(600, 25)` は残す（D5）。
  検証: `CT archive_scan_interval_change_takes_effect_without_restart` rc=0 / `NL ashiato-server archive_scan_interval_change_takes_effect_without_restart 1` rc=0
- [ ] 4.2 **4.1 を commit してから**、壊す差分 `break-ignore-interval.patch` を change の中に置く（design D11。`archive/worker.rs` の `wait_for_next_scan` で間隔を眠りの前に 1 回だけ読む）。
  `break-proof.sh` に `scan` を足す: `.env` を読み、`tools/break-check.sh --patch <この差分> --expect 待った時間 --expect 最後に見た数 --timeout 900 -- cargo test -p ashiato-server archive_scan_interval_change_takes_effect_without_restart`。
  前に `echo "Scenario: 取り込み器が変えた間隔を読まないように壊すと、間隔の変更の試験は期限で落ちる"` を出す。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run break -- openspec/changes/ch09-load-tolerant-timing-tests/break-proof.sh scan'` rc=0
- [ ] 4.3 同じファイルの `archive_scan_interval_default_is_120_seconds` が、`wait_for_scans` を替えた後も通る（`advance_secs(600, 25)` は残す。D5）。
  検証: `CT archive_scan_interval_` rc=0 / `NL ashiato-server archive_scan_interval_ 3` rc=0

## Task 5: Rust の残りの候補 —— 条件の待ち・打ち切り

- [ ] 5.1 `crates/` で `check-inventory.sh`（Task 12）の拾う形に当たる箇所を読み、design D4（条件の待ち → 実時間の期限 30 秒・文言に待った時間と最後に見た値）と
  D5（観測できる条件のある固定の眠り → 条件の待ち）を当てる。既知のもの: `scheduled_tests.rs` の 200 回の待ち 5 か所（刻みは変えない）・`outage_api_tests.rs:419`・
  `runtime.rs` の `tick_until_history_idle`・`:2470`・`:2545`・`history/collect.rs:868`・`:969`。`tick_until_read`・`tick_until_read_again` は回数のまま（D4 の除く）。
  直した箇所と直さなかった箇所は Task 12 の `inventory.md` に書く。印の付いた試験の期待の行を変えたら `agreed-test-changes.md` に書く。
  回数のまま残す待ち（時刻を兼ねるループ）には、1 行上か同じ行に `// timing-ok: <理由>` を書く（design D14）。
  検証: `CW history` rc=0 / `NL ashiato-collector-windows history 20` rc=0 / `CT scheduled` rc=0 / `NL ashiato-server scheduled 5` rc=0 / `CT outage` rc=0 / `NL ashiato-server outage 5` rc=0
- [ ] 5.2 打ち切りの 2 か所を design D6 のとおりに延ばす: `testdb_tests.rs` の `ch04_statement_timeout_cancels_a_long_query`（`pg_sleep(60)`・「30 秒未満」。印 `// Scenario: 上限を超える問い合わせは、止まって失敗する` は残す。
  正典の文は MODIFIED で書き直してある）と `time_sync.rs` の `time_sync_timeout_kills_the_child`（`sleep 60`・「30 秒未満」）。`stay_tests.rs:2389` は直さない（計算の形）。
  検証: `CT ch04_statement_timeout_cancels_a_long_query` rc=0 / `NL ashiato-server ch04_statement_timeout_cancels_a_long_query 1` rc=0 / `CW time_sync_timeout_kills_the_child` rc=0

## Task 6: 画面の単体試験の暗黙の上限

- [ ] 6.1 design D9 のとおりに `web/src/__tests__/setup.ts` に `configure({ asyncUtilTimeout: 10_000 })`、`web/vitest.config.ts` の `test` に `testTimeout: 30_000` を足す（1 行目は変えない）。
  `web/src/__tests__/ch09-timeouts.test.ts` を足し、`// Scenario: 画面の単体試験の待ちの上限は 10 秒、試験の上限は 30 秒` の印のもとで、`getConfig().asyncUtilTimeout` が 10000、
  `vitest.config.ts` の設定の `test.testTimeout` が 30000 であることを見る。`web/src/__tests__` の `{ timeout: N }` で 10 秒より短いものがあれば同じ形で延ばし、`inventory.md` に書く。
  検証: `bash -c 'cd web && npx vitest run src/__tests__/ch09-timeouts.test.ts'` rc=0 / `bash -c 'cd web && npx vitest run'` rc=0

## Task 7: 携帯の収集アプリの単体試験

- [ ] 7.1 `collector-android/app/src/test` で `check-inventory.sh` の拾う形に当たる箇所に design D4 を当てる（既知: `UnavailableLogGateTest.kt` の `await(5, TimeUnit.SECONDS)` 6 か所 → 30 秒）。
  `:72` の `await(200, TimeUnit.MILLISECONDS)` の `assertFalse` は直さない（D12）。
  検証: `KT *UnavailableLogGateTest*` rc=0

## Task 8: Windows の実行時テスト（Q2）

- [ ] 8.1 `crates/collector-windows/tests/runtime_windows.rs` に design D3 を当てる: 条件が来た時点で返る待ち（`observe_until`・`run_until`・`focus_window_of` の 10 秒・`:697` の 20 秒・
  `:856` の 30 秒・`:1203` の 10 秒と、それを呼ぶ箇所の 5・8・20 秒）を D4 の 30 秒（期限・文言に待った時間）に揃える。`run_for`（決まった時間の窓）は延ばさず `// timing-ok:` を書く。
  相手役を待つ固定の眠り（`:140`・`:406`・`:950`・`:1215`・`:1424`）には D5 を当てる（観測できる条件が無ければ残して `// timing-ok:`）。直したもの・残したものは Task 12 の `inventory.md` に書く。
  検証: `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 /
  `bash -c 'rsync -a --delete --exclude target --exclude .git --exclude node_modules ./ /mnt/c/dev/ashiato2-rt-ch09/ && /mnt/c/Windows/System32/cmd.exe /c "cd /d C:\dev\ashiato2-rt-ch09 && set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p ashiato-collector-windows --test runtime_windows -- --test-threads=1" > "$TMPDIR/rt.log" 2>&1; rc=$?; cat "$TMPDIR/rt.log"; [ $rc -eq 0 ] && n=$(grep -oE "test result: ok\. [0-9]+ passed" "$TMPDIR/rt.log" | grep -oE "[0-9]+" | head -1) && [ "${n:-0}" -ge 13 ]'` rc=0（CI の `collector-windows-runtime` と同じ 13 本以上）

## Task 9: 画面の e2e（Q2）

- [ ] 9.1 design D15 のとおりに `web/playwright.config.ts` に `expect: { timeout: 30_000 }` と `timeout: 120_000` を足し（1 行目は変えない）、`web/e2e/*.spec.ts` の 30 秒より短い `{ timeout: N }` を消す
  （任意の操作の `day-erase.spec.ts:70` は残して `// timing-ok:`）。`test.describe.configure({ timeout: N })` で 120 秒より短いものは 120 秒にする。`feel.spec.ts` の `waitForTimeout` には D5 を当てる。
  `web/e2e/ch09-timeouts.spec.ts` を足し、`// Scenario: 画面の e2e の待ちの上限は 30 秒、試験の上限は 120 秒` の印のもとで、`playwright.config.ts` を import して `expect.timeout` が 30000、`test.info().timeout` が 120000 以上であることを見る（`test.info()` は `expect` の上限を持たない。design D15）。
  検証: `bash -c 'cd web && npm run typecheck && npm run lint'` rc=0 / `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e'` rc=0

## Task 10: 携帯の計測テスト（Q2）

- [ ] 10.1 `collector-android/app/src/androidTest` に design D17 を当てる（条件の待ちの定数と `Tasks.await`・`CountDownLatch.await` など `await(<数>, TimeUnit…)` の秒数を 30 秒に・`SETTLE_MS` は使い方で分ける・`Thread.sleep` は D5・Espresso の待ちは変えない）。
  直したもの・残したもの（`// timing-ok:`）は Task 12 の `inventory.md` に書く。
  検証: `scripts/quiet-run assemble -- ./collector-android/gradlew -p collector-android :app:assembleDebugAndroidTest` rc=0 / `scripts/quiet-run inst -- tools/android-emulator.sh` rc=0

## Task 11: 縦串と検査の台本（Q2）

- [ ] 11.1 `tools/*.sh` と `tools/smoke.d/*.sh` に design D16 を当てる（回数 × 眠りが 30 秒より短い待ちを `SECONDS` の期限 30 秒に・文言に待った秒数と最後に見た値・30 秒以上の回数の待ちは `# timing-ok:`・`smoke.sh:199` の `sleep 3` は D12 として残す）。
  末尾に節や見出しを足さない。直したもの・残したものは Task 12 の `inventory.md` に書く。
  検証: `bash -c 'for f in tools/*.sh tools/smoke.d/*.sh; do bash -n "$f" || exit 1; done'` rc=0 / `tools/check-shared-appends.sh` rc=0 / `scripts/quiet-run smoke -- ./tools/smoke.sh` rc=0

## Task 12: 洗い出しの一覧

- [ ] 12.1 change の中に `check-inventory.sh` を design D14 と D3（Q2 で広げた範囲と拾う形: `web/e2e`・`web/playwright.config.ts`・`app/src/androidTest`・`tests/runtime_windows.rs`・`tools/*.sh`・`tools/smoke.d/*.sh`）のとおりに書く（候補を拾い、候補を持つファイルが全部 `inventory.md` に出ることを見る。`--list` で候補を出す。
  一覧で「条件の待ち」・直したとしたファイルに、回数の待ちと 30 秒より短い上限が `timing-ok:` の印なしに残っていないことも見る。その前に
  `echo "Scenario: 一覧で直したとした条件の待ちに、回数の待ちと 30 秒より短い期限が残っていない"` を出す）。
  `inventory.md` を design D14 の列と型で書く（Task 3〜11 で直したもの・直さなかったものとその理由・探した範囲と探していない範囲・製品の不具合の疑い）。
  design の「既知の候補」の行は全部載せる。
  検証: `openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh` rc=0

## Task 13: 壊してはいけないものと、負荷の下の 10 回

- [ ] 13.1 change の中に `check-product-unchanged.sh` を design D10 のとおりに書く（`--self-test` 付き）。本体の前に
  `echo "Scenario: CH09 の差分は、試験のモジュールの外の製品のコードに 0 行"` を出す。
  検証: `bash -c 'openspec/changes/ch09-load-tolerant-timing-tests/check-product-unchanged.sh --self-test && openspec/changes/ch09-load-tolerant-timing-tests/check-product-unchanged.sh'` rc=0
- [ ] 13.2 change の中に `check-test-names.sh` を design D13 のとおりに書く（CH03 の `openspec/changes/archive/2026-10-30-ch03-test-tmp-cleanup/check-test-names.sh` を写し、画面の単体試験の `it(` / `test(` の名前と本数を足す。Q2 で広げた分として、design D3 のとおり e2e の `test(`・計測テストの `@Test` の `fun`・`runtime_windows.rs` の `#[test]` の `fn` の名前と、縦串の台本の `Scenario:` の印も比べる）。
  本体の前に `echo "Scenario: 既存の試験の名前と印は消えず、試験の本数は減らない"` を出す。
  検証: `openspec/changes/ch09-load-tolerant-timing-tests/check-test-names.sh` rc=0
- [ ] 13.3 `load-proof.sh after` を書く: 1 と 2 の試験に、`load-before.txt` の `load-history` / `load-scan` の負荷（`none` なら `host`。両方なら両方）で `tools/load-repeat.sh --times 10` を当て、どれも `pass 10/10` であることを見る。前に
  `echo "Scenario: 全コアに負荷をかけた状態で、見回りと走査の間隔の 2 本の試験が続けて 10 回通る"` を出す。結果を `load-after.txt`（change の中）に残して commit する。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run load -- openspec/changes/ch09-load-tolerant-timing-tests/load-proof.sh after'` rc=0
- [ ] 13.4 全体: `cargo test --workspace` と、spec の Scenario の印の突き合わせ。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ws -- cargo test --workspace'` rc=0 / `python3 scripts/check_scenarios.py . ch09-load-tolerant-timing-tests` rc=0 / `python3 scripts/check_agreed_tests.py . --change ch09-load-tolerant-timing-tests` rc=0
