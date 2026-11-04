# CH09 — 実時間で「〜秒以内」を求める試験を混み具合で落ちない形にする

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違ったのは Q2（洗い出しの範囲。「全部探して全部直す」）だけで、2026-11-04 の `spec --answers Q2` が写した。
> `docs/changes/CH09.md` の「壊してはいけないもの」「完了の判定」も、この proposal・design・tasks に合わせてこの段で書いた。

## Why

2026-11-04、ST55 の関門（`cargo test --workspace`）が、ST55 と関係のない既存の試験 2 本で 2 度止まった。どちらも同じコードで通ったり落ちたりし、
落ちたのはマシンが混んでいたとき（load average 9〜14 / 16 コア）。1 本は見回り 1 回の実時間を「1 秒未満」と計り、もう 1 本は止めた時計を進める間合いが
取り込み器の DB の往復と競り合っていた（`premises.md` 3 行目・`review/deep.md` R1）。証跡の最新が FAIL になると関門が進まず、診断の席や人間が取り直す。
並走が当たり前になった今、同じ形の試験が残る限り、別の Story の関門が同じ理由で止まり続ける。

## What Changes

- **1 の試験（`history_large_fetch_keeps_observing_foreground`）は、時間を計らずに「見回りが履歴の取得を待たない」を直接確かめる**（Q1 の推奨）。
  偽の履歴の読み手を止めた間に見回り（`tick_at`）が返り、前景の変化が積まれることを見る。返らなければ 30 秒の見張りが読みを放して試験を落とす。
  読みを放した後は、見回り 1 回で増えた履歴の記録が 2,000 件（`QUEUE_PER_TICK`）以下であることを数で見る。時間の assert は消し、
  `queueing_ticks`・積みこぼし・眠りが入らないことの assert は残す
- **2 の試験（`archive_scan_interval_change_takes_effect_without_restart`）は、届かなければ 1,001ms ずつ進め直し（実時間の期限 30 秒）、
  仮想時間の経過を「10 秒ちょうど」でなく「30 秒未満」で見る**（Q8 の推奨。推奨の「例: 60 秒」を、30 秒ごとの読み直しの退行も捕まえる 30 秒にした。design D2）。正典 `external-ingestion` の Scenario「登録簿で見回りの間隔を変えると起動し直さずに効く」の
  確かめ方を、それに合わせて書き直す（**MODIFIED**。Requirement の本文は変えない）
- **同じ形の試験を洗い出して同じ方針で直す**（Q3〜Q5・Q9 の推奨と、Q2 の本人の答え「全部探して全部直す」）。範囲は Rust の `crates/`（Windows でだけ組まれ、CI の `collector-windows-runtime` が走らせる `tests/runtime_windows.rs` を含む）・
  `web/src/__tests__`・`web/e2e`・`collector-android/app/src/test`・`collector-android/app/src/androidTest`・`tools/*.sh`・`tools/smoke.d/*.sh`（design D3・D15〜D17）。条件を待つ上限は実時間の期限で一律 30 秒（試験の中の時刻を兼ねるループは回数のまま）、
  打ち切りの試験は相手の長さと上限を一緒に延ばす、計算の形を時間で見る試験は直さず載せる、条件を待たない固定の眠りは観測できる条件があれば条件の待ちに、
  画面の単体試験は設定 1 か所で待ち 10 秒・試験 30 秒、画面の e2e は playwright の設定 1 か所で待ち 30 秒・試験 120 秒（D15）、台本の短い回数の待ちは `SECONDS` の期限 30 秒（D16）、計測テストの条件の待ちは 30 秒（D17）。
  広げた範囲の確かめ（`npm run test:e2e`・`tools/android-emulator.sh`・`tools/smoke.sh`・Windows 側の clone での実行時テスト）もこの change に入る。洗い出した一覧は `design.md` の表を下流が完成させ、PR 本文に写す
- **負荷の下で続けて通ることと、壊すと落ちることを確かめる台本を `tools/` に足す**（Q6・Q10 の推奨）。
  `tools/load-repeat.sh`（全コアの busy loop の下で命令を N 回。直す前に落ちる回が出ることも記録できる）と
  `tools/break-check.sh`（使い捨ての作業木に壊す差分を当て、命令が決まった文言で落ちることを期待する）。どちらも `--self-test` 付き
- **振る舞いを変えないもの**: 製品のコード（試験のモジュール `#[cfg(test)] mod tests` の外）は 1 行も変えない（Q0 の推奨。design D10 が機械で見る）。
  製品の不具合の疑いが出たら直さず一覧に書いて別の change に出す（Q7）。既存の試験の名前・`// Scenario:` の印・本数は減らさない
- **変わってしまう確かめ（2 つ）**: 打ち切りの試験（`testdb_tests.rs` の statement timeout）は、正典 `test-database` の Scenario の問い合わせを 60 秒・待つ上限を 30 秒にする（**MODIFIED**。Q3）。
  もう 1 つ —— 2 の試験は「変えた値が 1 秒以内に効く」を「10 秒ちょうどで 10 回」でなく「10 回が 30 秒未満」で見るようになり、精度が粗くなる。
  古い間隔（120 秒）で眠る退行と、30 秒ごとにしか読み直さない退行は捕まる。数秒の遅れは捕まえない（Q8）

## Capabilities

### New Capabilities

- `test-timing`: 試験が実時間の混み具合で落ちないこと —— 「待たない」を止めた偽物で確かめる・条件を待つ上限を期限で持つ・負荷の下で続けて通ることと
  壊すと落ちることを台本で確かめる。CH03 の `test-hygiene`（一時ファイル）と同じく、製品ではなく開発の道具の約束。`test-hygiene` の Purpose は一時フォルダに
  限っているので、別の capability にする。schema の「tooling は `skip_specs`」は採らない: 見張りで落ちる・負荷の下で 10 回通る・壊すと落ちる・設定の上限は
  外から確かめられる振る舞いで、Scenario を `check_scenarios.py` が印に縛り、archive の後も正典が固定する。change の capability は `docs/stories/INDEX.md` に足さない

### Modified Capabilities

- `external-ingestion`: Requirement「取り込み器の見回りの間隔は登録簿から読む」の Scenario「登録簿で見回りの間隔を変えると起動し直さずに効く」の
  WHEN / THEN を、止めた時計の上で混み具合に依らず確かめられる形に書き直す（Q8）。Requirement の本文（変わってから 1 秒以内に新しい間隔で待ち始める）と、
  ほかの 3 本の Scenario は変えない
- `test-database`: Requirement「試験の 1 本の問い合わせは、時間の上限を超えたら止まって試験が落ちる」の Scenario「上限を超える問い合わせは、止まって失敗する」の
  問い合わせを `pg_sleep(5)` → `pg_sleep(60)`、「5 秒を待たずに」→「30 秒を待たずに」にする（Q3 の打ち切りの型。上限 1 秒が効くことの確かめは弱めない）。
  Requirement の本文とほかの 2 本は変えない

## Impact

- 試験のコード: `crates/collector-windows/src/runtime.rs` の `mod tests`（1 の試験と、同じ形の `clock_worker_slow_read_does_not_hold_the_patrol` など）・
  `crates/collector-windows/src/history/collect.rs` の `mod tests`・`time_sync.rs` の `mod tests`・`crates/server/src/archive_scan_interval_tests.rs`・
  `scheduled_tests.rs`・`testdb_tests.rs`・`outage_api_tests.rs` ほか（候補は design の表）。`web/vitest.config.ts`・`web/src/__tests__/setup.ts`・
  `collector-android/app/src/test/.../UnavailableLogGateTest.kt`。Q2 で広げた分: `crates/collector-windows/tests/runtime_windows.rs`・`web/playwright.config.ts`・`web/e2e/*.spec.ts`・
  `collector-android/app/src/androidTest/**`・`tools/smoke.sh`・`tools/smoke.d/*.sh` ほか `tools/*.sh` の待ち（ある行を直すだけ。末尾に足さない）
- 製品のコード: 変えない（`#[cfg(test)] mod tests` の外の差分は 0 行。D10）
- 道具: 新しいファイル `tools/load-repeat.sh`・`tools/break-check.sh`。change の中に `check-product-unchanged.sh`・`check-test-names.sh`・`check-inventory.sh`・`agreed-test-changes.md`・
  壊す差分 `break-*.patch`・証跡の台本 `load-proof.sh`
- 依存: 足さない（`stress` は入れない。busy loop は bash で作る）
- **走っている Story・change との重なり**（`origin/main` = 3235f293 との差分）:
  - st53（`feat/st53-license-bundle`）: `runtime.rs`・`collect.rs`・`time_sync.rs`・`archive_scan_interval_tests.rs`・`scheduled_tests.rs`・`testdb_tests.rs`・
    `web/src/__tests__/setup.ts`・`web/vitest.config.ts` の **1 行目（SPDX の行）だけ**を変える。CH09 は 1 行目に触らないので、後から merge する側の rebase は
    自動で解ける見込み
  - st55（`feat/st55-server-installer`）: `runtime.rs` の製品の部分（:649・:754 付近）と `mod tests` の :2935〜:3100 付近（1 の試験 :3612 とは別の箇所）、
    `web/vitest.config.ts` の `define` の行。CH09 が触る `test:` の節とは別の行
  - st53 は Q2 で広げた `web/e2e`・`tools/smoke.sh`・`app/src/androidTest`・`tests/runtime_windows.rs` の 50 ファイルでも **1 行目（SPDX の行）だけ**を変える（`git diff origin/main...origin/feat/st53-license-bundle` で SPDX 以外の行が 0）
  - st54: 重ならない。どれも走っている Story へ差し戻さない。merge の順は決めない
