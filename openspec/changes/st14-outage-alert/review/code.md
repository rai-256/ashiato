# ST14 下流のコードレビュー

## final review（e32f33a..078d803）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package は
`.superpowers/sdd/tasks/review-e32f33a9..078d803c.diff`（60 commit）。全 Task の ledger（deferred-minor / parked / Ruling の行は 0）と、
全 Task の指摘の記録の Minor / ⚠️ を指した。判定は **With fixes**（Critical 0 / Important 3 / Minor 6）。

申し送り（`docs/handoff/ST14.md`）は開始時から増えていない（最後の変更は 00398a42、preflight の生成より前）。
C1（`export::CATALOG`）は base にも main にも無いので ST33 の下流が受ける。C3 / K1 / K3 / R4 / R7 / R24 / R26 / st05 R6 は実装に入っている（final reviewer の突き合わせ）。

## R1. 収集開始日はあるが「最後に生きていた時刻」の材料が何も無いソースは、永久に途切れと判定されない
- 成果物: crates/server/src/outage.rs:122-134（`last_alive` が `None` → `judge` の `filter_map` で落ちる）・outage.rs:319-320 の試験
- 根拠: 取得できない状態の生存信号も `touch_started_on(Arrival::Heartbeat)` で収集開始日を立てる（`lib.rs` の `heartbeat_one`）。
  最初から権限が無い写真・ブラウザ履歴、窓を一度も読めず clock-skew しか届かない PC は、開始日を持ち判定の対象なのに判定から黙って外れる。
  deep.md Q2 は「知らせる —— 取得できる状態の生存信号だけを生きていると数える」、C2 が鳴らさないとしたのは「一度も届かない」（開始日の無い）収集だけ。
  spec の (4) は開始日の無いものだけを外し、この場合は書いていない。実装が D 番号なしで決めていた
- kind: conflict
- 提案: 収集開始日の Asia/Tokyo の 0 時を「最後に生きていた時刻」の下限に足し（`alive_floor` と同じ形）、design に D<n>（仮）と反転条件を書く
- 処置: fixed D26 仮 —— 収集開始日の Asia/Tokyo の 0 時を下限に足した（c210c161 / 59121b30 / 08b820ee。`outage::tests::st14_judge_started_day_is_alive_floor`）。巻き込まれた `st14_api_outages_empty` / `st14_api_outages_no_record_content` を直した（後者は `raw` の文字を応答全体では見なくなった）

## R2. 判定の材料を台帳の助言ロックの外で読んでいるので、同時の問い合わせで途切れが開き直され、もう一度鳴る
- 成果物: crates/server/src/lib.rs:2740-2748（`outages_get`）・crates/server/src/outage_store.rs:169-177（`record` の `pg_advisory_xact_lock`）
- 根拠: `outage_store::inputs` と `outage::judge` は `record` の取引の前に走る。A が「X は途切れ」と判定 → B が「X は戻った」と判定して先にロックを取り `closed` → A がロックを取ると開いた途切れが無いので新しい `opened`。
  識別子が変わり端末で再び鳴る（Q4「途切れ 1 回につき端末ごとに 1 回」に反する）。`st14_ledger_concurrent_opens_once` は単一スレッドの `join!` で重なりを保証しない（Task 7 F1 / F2）
- kind: technical
- 提案: ロックを取った同じ取引の中で `inputs` を読み `judge` する（`inputs` が取引を受ける）
- 処置: fixed D7 —— `outage_store::judge_and_record` が助言ロックの後の同じ取引で inputs → judge → 台帳（7ee76c12。`outage_api_tests::st14_ledger_judges_inside_the_lock` が別接続のロックで重なりを作る。旧コードで落ちるのを確認）

## R3. `has_non_collected` を支える索引が無く、記録が増えると問い合わせが遅くなり通知が黙って止まる
- 成果物: crates/server/src/outage_store.rs（`EXISTS (... AND e.origin <> 'collected')`）・migrations/202610141130_outage_alert.sql
- 根拠: `core.event` の索引に `origin` を含むものが無い。収集した記録しか持たないソース（位置は年に数十万行）では一致が無く、問い合わせのたびに利用者 × ソースの全行を読む。
  収集アプリは 10 秒で打ち切るので遅くなると Unreachable →「通知が変わらない」（Task 5 F1）。design D4 は「足りなければ D 番号を足して部分索引」
- kind: technical
- 提案: まだ merge していない同じ移行に `(user_id, logical_source) WHERE origin <> 'collected'` の部分索引を足し、D<n>（仮）を書く
- 処置: fixed D27 仮 —— 同じ移行に部分索引 `event_non_collected_by_source`（9c424f6f。`st14_reg_event_has_non_collected_partial_index`）

## R4. design.md が実装に合わせて書き戻されていない（D7 の追記のみの関数・D21 の no-preference）
- 成果物: openspec/changes/st14-outage-alert/design.md（D7 / D21）
- 根拠: D7 は「既存の台帳と同じ関数」のまま。実装は申し送り K3 どおり台帳専用の `core.reject_outage_ledger_change()`（Task 1 ⚠️）。
  D21 の no-preference の読み替え（Task 10 F3）も書かれていない
- kind: technical
- 提案: D7 と D21 を実装の形に直す
- 処置: fixed D7 —— D7 を専用関数と K3 の理由に、D21 に no-preference の読み替えを書いた（21a6dce6）

## R5. Scenario の印を括弧の中に書いていて、`check_scenarios.py` が `）` まで名前として拾う
- 成果物: crates/server/src/outage_tests.rs:422
- 根拠: `（Scenario: 停止が終わった時刻から数え直す）` —— 印の名前が `停止が終わった時刻から数え直す）` になり warn
- kind: technical
- 提案: 印を独立した行の `// Scenario: <名前>` にする
- 処置: fixed 5.1 —— :422 と同じ形の :252 も独立した行にした（56fb0def。`check_scenarios.py` OK）

## R6. 台帳の形の `CHECK` が `opened` に `last_alive_at` を求めていない
- 成果物: migrations/202610141130_outage_alert.sql（`outage_ledger_shape`）
- 根拠: design D7 は「`last_alive_at` は `opened` だけが持つ」。いまの CHECK は `opened` で NULL を許す
- kind: technical
- 提案: `opened` のとき `last_alive_at IS NOT NULL` を CHECK に足し、試験で固定する
- 処置: fixed D7 —— 別名の CHECK `outage_ledger_opened_has_last_alive`（shape の差し替えは DROP CONSTRAINT になり check-migrations が断る。既存 DB へは NOT VALID）（21ac16ff。`st14_reg_ledger_opened_requires_last_alive_at`）

## R7. 偽データの clock-skew の payload の鍵が契約と違い、コメントの開始日が古い
- 成果物: tools/seed.sh（`c02-window` の `kind = clock-skew`）
- 根拠: seed は `trigger` / `available` / `references` / `unavailable`、契約（`crates/server/src/contract.rs:502`）は `clock_trigger` / `clock_available` / `clock_references` / `clock_unavailable` / `skew_ms`。
  サーバは `kind` しか読まないのでいまは害が無い。コメントの「開始日（2026-09-07）」も古い（Task 10 F1）
- kind: technical
- 提案: 契約の鍵にそろえ、コメントを直す
- 処置: fixed D18 —— 鍵を契約（`crates/collector-windows/src/contract.rs:502`。根拠の行のパスは誤り）にそろえ、コメントの開始日を 2026-08-31 に（afb2f363）

## R8. 達成日（NFR-13）は取りこぼしの日でも取得できる状態の信号があれば数え、格子は同じ日を③に塗る
- 成果物: crates/server/src/coverage.rs（`achieve_one` は `decide` を通らない）
- 根拠: 格子は C9 / D 番号どおり取りこぼしの日を③にするが、達成の数えは生存信号の `capturable` を見るだけ。spec は達成の側の扱いを書いていない
- kind: conflict
- 提案: いまの振る舞いを D<n>（仮）として書き、反転条件（達成も格子の状態から数える）を付ける
- 処置: fixed D28 仮 —— いまの振る舞いと反転条件を design に書いた（eb6fa85b。コードは変えない）

## R9. Android の問い合わせは位置のスケジューラにだけ乗っている
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutagePoller.kt・LocationService.kt
- 根拠: 位置の収集が動いていない構成では、アプリ利用の途切れも携帯に届かない
- kind: technical
- 提案: 注記のみ（final reviewer も「D12 の設計どおり」としている）
- 処置: rejected: design.md:193 の D12「生存信号の契機に相乗りする」どおり。問い合わせは `LocationService.kt:840` / `:845` の `if (isLocation) outagePoller.poll()` で位置の契機に乗せると決めている。位置の収集が止まった構成ではアプリ利用の途切れも届かないことを、この行に注記として残す

## R10. 取りこぼしの理由の文言を、端末から来た文字で `messages` のオブジェクトから直に引いている
- 成果物: web/src/CoverageGrid.tsx（`m.gapReason[g.reason]`）
- 根拠: `reason` は端末から来る値で、`constructor` などは prototype を引く（Task 10 F2）
- kind: technical
- 提案: `Object.hasOwn` で確かめ、無ければ既定の文言にする
- 処置: fixed 10.2 —— 実際の場所は `web/src/coverage.ts` の `gapNotes`。`Object.hasOwn` で確かめ、無ければ `gapUnknown`（59330af1。`coverage-gaps.test.ts` の it.each 5 件）

## scoped re-review（e2f245c..08b820e）

席: `re-review-prompt.md`。package は `.superpowers/sdd/tasks/review-e2f245c5..08b820ee.diff`（11 commit）、fix の報告は
`.superpowers/sdd/st14-final-fix-report.md`。R1〜R8・R10 はすべて ADDRESSED、fix による新しい破損なし、範囲外の観察なし。

## code-verify（HEAD 5589c6d4）

席: code-verify（申告と実態のずれ）。最後の証跡（evidence.jsonl）は 11.x の bc4e76e0 で、その後の 15 commit（e2f245c5〜5589c6d4）が
`outage*.rs`・移行・`tools/seed.sh`・`web/src/coverage.ts` を変えている。**全項目の検証コマンドを HEAD で走らせ直した。**
変異は作業ツリーの外の複製（`~/dev/.st14cv/mut`。独立した git で 1 か所ずつ戻す）で当て、本体のコードは触っていない。

申告: **31/31**（`- [x]` 31・`- [ ]` 0）。

| 項目 | 検証コマンド（tasks.md の本文どおり） | 実測（HEAD） | 一致 |
|---|---|---|---|
| 1.1 | `bash -c 'grep -q DEVICE_SUBJECT … && grep -q archive_ledger_source …'` | rc=0 | 一致 |
| 1.2 / 1.3 | `CT st14_reg_` / `CT st14_reg_default`・`check-migrations.sh`・`check-immutable.sh` | 10 本 / 1 本（`-- --list`）・rc=0・rc=0（ただし R15） | 一致 |
| 2.1〜2.3 | `CT st14_grid_nonobs` / `st14_grid_gap`・`check-openapi.sh` / `coverage::tests::`・`st14_grid_` | 7 / 7 本・rc=0 / 93 / 14 本 | 一致 |
| 3.1〜3.3 | `CT st14_rollup_started` / `st14_rollup_order` / `coverage::tests::`・`api_tests` | 4 / 2 / 93・46 本 | 一致 |
| 4.1 / 4.2 | `CT st14_ach_` / `coverage::tests::`・`npx vitest run` | 5 本 / 93 本・41 files 352 passed | 一致 |
| 5.1 / 5.2 | `CT st14_judge_` / `st14_inputs_device`・`st14_inputs_nonobs_registry` | 24 / 13・1 本 | 一致 |
| 6.1 / 6.2 | `CT st14_judge_archive`・`st14_inputs_archive` / `st14_inputs_archive_grid` | 4・10 / 1 本 | 一致 |
| 7.1 / 7.2 | `CT st14_api_outages_`・`check-openapi.sh` / `CT st14_ledger_` | 11 本・rc=0 / 9 本 | 一致 |
| 8.1〜8.3 | `KT OutageNoticeTest` / `OutageNotifierTest` / `OutagePollerTest`・grep・`detekt :app:assembleDebug` | 12 / 6 / 5 testcase・failures 0（`LocationServiceTest` 26 も 0）・rc=0（ただし R14）・rc=0 | 一致 |
| 9.1 / 9.2 | `WU outage_notice_` / `WU outage_notice_poll`・windows 向け clippy | 18 passed / 4 本・rc=0 | 一致 |
| 9.3 | rsync → `cmd.exe … cargo test --test runtime_windows outage` | `outage_toast_is_shown_replaced_silently_and_removed` 1 passed・rc=0 | 一致 |
| 10.1 | stack を setsid で起こす → `/coverage` → `jq -e` | `true`・rc=0（`WEB_PORT=5914` を環境で渡した。5180 は tailnet 側で使用中） | 一致 |
| 10.2〜10.4 | `VT coverage-gaps.test.ts`・`ET coverage-gaps` / `coverage-rollup`・`archive-layout` / `coverage-scheme`・`surface` | vitest に含む / e2e 5 ファイル 33 passed（`CI=1 STACK_RESET=1 WEB_PORT=5914`） | 一致 |
| 10.5 | `npm run lint && tsc --noEmit && vitest run && build` | rc=0 | 一致 |
| 11.1 | `scripts/quiet-run smoke -- tools/smoke.sh` | rc=0（「OK outages warming_up -> ready」） | 一致 |
| 11.2 | `check_scenarios.py . st14-outage-alert` | rc=0（1106 件すべて担保あり） | 一致 |
| 11.3 | fmt / clippy -D warnings / `cargo test --workspace` / check-openapi・migrations・immutable・log-private | rc=0 / rc=0 / 151 + 896 + 7 passed・rc=0 / rc=0 ×4 | 一致 |

**手 1（固定値の再計算）: 該当なし。** 確かめた範囲: `coverage-scheme.spec.ts` の面の色のリテラル `LIGHT_GROUND = [252,253,252]`・`DARK_GROUND = [27,39,31]` を、
`tokens.ts` の `surface(99)` / `surface(13)`（hsl(140 18% 99%) / hsl(140 18% 13%)）から python の `colorsys` で計算し直した —— 251.99/252.91/252.30・27.18/39.12/31.16 で、丸めると一致する。
取りこぼしの理由の文字（`retention` / `clock_skew_abandoned`）と payload の鍵（`kind` / `begin` / `end` / `reason`）は、端末の `AppUsageRecord.kt:111・121・146-149` とサーバの `gaps_by_day` の SQL と `messages.ts` で一致する。
通知の ID 3 は既存の 1（常駐）・2（retention）と重ならない（`LocationService.kt:941・945`）。

**手 2（ガードを壊す）:** 複製で 1 か所ずつ壊した。
- 新しい DB（`CREATE DATABASE` から全移行を当てる。対照は 110 passed）で移行を壊す: `c01-clock` の印を外す → `st14_reg_excluded_sources_are_marked` が落ちる / 75 → 74 → `st14_reg_archive_sources_are_75_days` / 集計の `started_on_from` を入れない → `st14_reg_dirty_rollup_started_on_is_repaired` /
  UPDATE・DELETE のトリガを作らない → `st14_reg_ledger_rows_are_append_only` / `opened` の CHECK を両方の置き場から外す → `st14_reg_ledger_opened_requires_last_alive_at` / 部分索引を作らない → `st14_reg_event_has_non_collected_partial_index`。
  **TRUNCATE のトリガを作らないと 110 passed のまま（R15）。**
- `check_scenarios.py`: `// Scenario: 途切れの一覧は表示名の順に返る` の印を消すと `FAIL (担保なし 1 件)`・rc=1。
- 8.3 の grep: `LocationService.kt` に `import android.app.AlarmManager` を足しても rc=0（R14）。

**手 3（Scenario と test）:** 印は 1106 件すべてにある（rc=0）。主張の階層を 1 件ずつ見た範囲（台帳の列・応答の欄の形・トーストの履歴の件数と `SuppressPopup`・権限なしの通知 0 件と送信の箱・
取りこぼしの理由の文字・明暗の背景色）は、Scenario の THEN を観測している。ずれは手 4・手 6 の R11〜R13 に出た。

**手 4（本人の決定と、Global Constraints が「名指しで固定する」とした値）:** 複製で値を 1 つずつ変えて `cargo test -p ashiato-server --lib`（896 本）・`--lib`（collector-windows 151 本）・gradle の `*Outage*`（23 本）を走らせた。
- 落ちる（固定されている）: `ALERT_GAP_FACTOR` 3→4（19 本）/ 厳密な `>` → `>=`（4 本）/ `ARCHIVE_ALERT_AFTER_DAYS` 75→74（2 本）・移行の 75→74（新しい DB で 1 本）/
  Q2 の `h.capturable` を外す（1 本）/ 出どころの除外を外す（3 本）/ 退役の日を含めない（1 本）/ D26 の下限を外す（1 本）/ 停止の終わりを外す（2 本）/ `NON_OBSERVATION_ALERT` の `idle` を外す（1 本）/
  `NON_OBSERVATION_GRID` の `clock-skew` を外す（4 本）/ Q7 の③を外す（1 本）/ Q8 の当て直しの止めを外す（1 本）/ PC の分母を絞らない（2 本）/ 集計を並びに入れない（2 本）/ 端末ごとの `delivered` を 1 本にする（1 本）/
  未来の記録を捨てる（1 本）/ 書庫の `outcome = 'read'` を外す（1 本）/ 表示名の並びを外す（1 本）/ Q6 `c01-clock` の印（新しい DB で 1 本。共有の DB では行が既に `false` なので緑になる —— 移行は既存の行を戻さない）/
  Q5 の音の規則を「集合が変われば鳴る」にする（Windows 1 本・Android 1 本）/ 届かないときに消す（Windows 2 本）/ PC で出せなくても覚えを進める（Windows 1 本）/ `OUTAGE_NOTIFICATION_ID` 3→4（Android 1 本）。
- **落ちない: `OUTAGE_WARMUP` 30→29 分（896 passed。R11）/ Android で出せなくても覚えを進める（23 本すべて緑。R12）。** ほかに PC の問い合わせの打ち切り `OUTAGE_QUERY_TIMEOUT_SEC` 10→30 も 151 passed のまま
  （design D11 の値で、本人の答えでも Global Constraints の列挙でもないので指摘にしない）。
- 共有の試験 DB で `web_session_tests::web_session_endpoint_api_token_reads_and_writes` が変異なしでも落ちるようになった（`/events` の本文が 64 MiB を超える。変異の実行で記録が溜まったため）。ST14 の変更とは関係が無く、
  集計からは除いた。最後に `tools/db.sh up -d --wait db && tools/db-roles.sh` で作り直し、`st14_` 103 passed を確かめた。

**手 5（`[x]` と実体）:** 31 件すべて、本文の検証コマンドが実在し HEAD で rc=0（上の表）。`CT` の絞り込みは全部 `-- --list` で 1 本以上。`--test runtime_windows` は `crates/collector-windows/tests/runtime_windows.rs` が実在する。

**手 6（隙間）:** 記録を捨てる経路はこの Story に無い（読むだけ・台帳は追記のみ・移行が直す集計の開始日は `min(event_time)` から計算し直せる）。
見つかったのは、端末の問い合わせが別の利用者で判定される経路（R13）と、台帳が「返した」を「鳴らした」として残す経路（R16）。

## R11. `OUTAGE_WARMUP`（30 分。D5（仮））を 29 分にしても全部の試験が通る
- 成果物: crates/server/src/outage.rs:15 / crates/server/src/outage_api_tests.rs:18-22・:236-247・:261-271
- 根拠: 複製で `from_secs(30 * 60)` → `from_secs(29 * 60)` にして `cargo test -p ashiato-server --lib` → **896 passed / 0 failed**。試験は「起動 10 分後は `warming_up`」（:241 `started(now - 10 分)`）と
  「31 分後は `ready`」（:21 `app_at` の `started(now - 31 分)`）しか見ないので、10 分を超え 31 分以下のどの値でも緑になる。`assert_eq!(OUTAGE_WARMUP, …)` も無い。
  tasks.md の Global Constraints は「`OUTAGE_WARMUP = 30 分`（D5（仮））… 試験が名指しで固定する」、Scenario の名前は「起動して 30 分の間は…」「起動して 30 分が経つと…」
- kind: technical
- 提案: `OUTAGE_WARMUP` を名指しで 30 分と固定し、境界（29 分 59 秒は `warming_up`・30 分ちょうどは `ready`）を `started(now - OUTAGE_WARMUP ± 1 秒)` で 1 本ずつ見る
- 処置: fixed 7.1 —— `st14_api_outages_warming_up` が `OUTAGE_WARMUP` を 30 分と名指しで固定し、起動 29 分 59 秒で `warming_up`、`st14_api_outages_after_warmup` が 30 分ちょうどで `ready` を見る。29 分に変えると `st14_api_outages_warming_up` が落ちることを確かめた

## R12. 携帯は「通知を出せなかったら覚えを進めない」を試験が固定しておらず、変えても 23 本すべて緑
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutagePoller.kt:61-62 / app/src/test/kotlin/dev/ashiato/collector/OutageNotifierTest.kt:98-125・OutagePollerTest.kt
- 根拠: 複製で `if (delivered) notice.save(decision.next)` → `notice.save(decision.next)` にして `gradlew :app:testDebugUnitTest --tests '*Outage*'` → OutageNoticeTest 12・OutageNotifierTest 6・OutagePollerTest 5 で **failures 0**。
  同じ変異を PC（`outage_notice.rs:252` の `return;` を外す）に当てると `outage_notice_failing_toaster_neither_panics_nor_stops_polling` が落ちる（:622 が覚えの空を見ている）。携帯の側にはその assert が無い。
  覚えが進むと、通知の権限を戻した後の同じ途切れは「続く途切れ」として**音なし**で出る —— Q4「途切れ 1 回につき音は 1 回」の 1 回が、一度も鳴らないまま消費される
- kind: technical
- 提案: `OutageNotifierTest` の権限なしの試験に「覚え（`OutageNotice.load()`）が空のまま」と、権限を戻した次の契機で音つきで出ることを足す（PC の :622 と同じ形）
- 処置: fixed 8.3 —— `OutageNotifierTest`「通知を出せなかったら覚えを進めず、権限が戻った次の契機で音つきで出る」を足した（権限なしで poll → 通知 0・覚えが空 → 権限を戻して poll → 音つき）。`if (delivered)` を外すと落ちることを確かめた

## R13. 収集アプリは `user_id` を付けずに問い合わせ、サーバは nil の利用者で判定する。利用者 ID が nil でない構成では、記録が届いているソースも全部「途切れ」で返る
- 成果物: collector-android/…/LocationService.kt:202（`"?device_id=" + …` だけ）/ crates/collector-windows/src/sender.rs:283-285（`.query("device_id", …)` だけ）/ crates/server/src/lib.rs:2739（`q.user_id.unwrap_or_default()` = nil）・:2467（`/coverage` は「省略すると絞らない」）
- 根拠: 複製に探りの試験を足して実行した —— 利用者 U（nil でない）に 1 時間前の記録があるソース（想定間隔 6 時間・開始日 2026-01-01）について、
  端末と同じ `GET /outages?device_id=probe-dev` → **そのソースが途切れとして返る**（`device_query_lists_it=true`・一覧 4132 件）・台帳に nil の利用者で `delivered` が 1 行。
  `&user_id=U` を付けると返らない（`with_user_lists_it=false`・一覧 4131 件。差の 1 件が探りのソース。件数が大きいのは共有の試験 DB の登録簿に試験のソースが数千本あるため）。
  直接測ったのはこの 1 ソース。nil の利用者には記録も生存信号も無いので、D26（仮）の「開始日の 0 時」だけが最後に生きていた時刻になり、**開始日を持つ対象のソースは全部が途切れになる**（判定式からの導出）。
  収集アプリは自分の利用者 ID を持っている（Android `Config.userId`・PC `ASHIATO_USER_ID`）が送っていない。`tools/record-env.sh:96・112` は利用者 ID を乱数で作り、:160 で APK に焼く（録画の環境はこの構成）。
  一方 `/coverage` は省略時に全利用者を見るので、同じ環境で格子は正常に見え、端末だけが鳴り続ける。縦串の 45（smoke.sh:1100）は `.outages | type == "array"` しか見ないので素通りする。既定の `.env`（`env-init.sh:26`）は nil なので、いまの手元の構成では起きない
- kind: technical
- 提案: 両方の収集アプリが自分の利用者 ID を `user_id=` に付ける（端末の試験で URL に付くことを見る）。サーバの省略時の扱い（nil に潰す / 全利用者 / 断る）は `/coverage` と揃えて design に D<n>（仮）で書く
- 処置: fixed D29 仮 —— 携帯は `outageQueryString(id, Config.userId)`、PC は `HttpOutageQuery::new(…, cfg.user_id)` で `user_id=` を付ける（`OutagePollerTest`「問い合わせは端末と自分の利用者を付ける」・`sender::tests::outage_query_sends_device_and_user` が本物の HTTP の要求行を見る）。サーバの省略時は既定の利用者のまま（`/coverage` とは役が違うので揃えない）と反転条件を D29（仮）に書いた

## R14. 8.3 の「OS の定期実行の仕組みを使わない」の grep は `Outage*.kt` しか見ず、問い合わせを配線している `LocationService.kt` の外にある
- 成果物: openspec/changes/st14-outage-alert/tasks.md（8.3 の検証）/ collector-android/app/src/test/kotlin/dev/ashiato/collector/OutagePollerTest.kt:92-98
- 根拠: 複製の `LocationService.kt` の先頭に `import android.app.AlarmManager` を足して 8.3 の `bash -c '! grep -nE "AlarmManager|WorkManager" …/Outage*.kt'` → **rc=0**。
  `OutagePollerTest.kt:93` は「本体の確かめは verify の grep」と書き、試験の側は位置以外のスケジューラが問い合わせないことだけを見る。いま `app/src/main/kotlin` 全体で `AlarmManager|WorkManager` は 0 件（`grep -rln`）なので、範囲を広げても偽陽性は出ない
- kind: technical
- 提案: grep の範囲を `collector-android/app/src/main` 全体にする（tasks の検証の訂正は `plan_fix.py` の手順で）
- 処置: fixed 8.3 —— 検証コマンドは書かれたままで成立している（FAIL の証跡が無いので `plan_fix.py` の訂正の対象ではない）ので tasks.md は変えず、`OutagePollerTest`「収集アプリの本体は AlarmManager も WorkManager も使わない」が `src/main` 全体（kt / java / xml）を読んで 0 件を見る。`LocationService.kt` に `import android.app.AlarmManager` を足すと落ちることを確かめた

## R15. 途切れの台帳の TRUNCATE の拒否は、外しても試験も `check-immutable.sh` も緑のまま
- 成果物: migrations/202610141130_outage_alert.sql:93-95（`outage_no_truncate`）/ crates/server/src/outage_registry_tests.rs:189-204 / tools/check-immutable.sh
- 根拠: 新しい DB で `CREATE TRIGGER outage_no_truncate …` を外した移行を当てて `cargo test -p ashiato-server --lib -- st14_ outage` → **110 passed / 0 failed**（UPDATE・DELETE のトリガを外すと 1 本落ちる）。
  `st14_reg_ledger_rows_are_append_only` は UPDATE と DELETE しか撃たない。`check-immutable.sh` は `outage` を 1 度も含まず（`grep -c` 0）、HEAD で rc=0（ログの `outage` はすべて移行の NOTICE）。
  同じ型の台帳（`access_log`・`drop_report`）は TRUNCATE を撃つ試験を持つ（`access_migration_tests.rs:56-60`・`drops_tests.rs:483`）。アプリの役割には TRUNCATE の権限が無い（`app_role_tests`）ので、残るのは所有者の接続
- kind: technical
- 提案: `st14_reg_ledger_rows_are_append_only` に所有者の接続での `TRUNCATE core.outage_ledger` が拒まれることを足す（`check-immutable.sh` に ST12 と同じ段を足してもよい）
- 処置: fixed 1.2 —— `st14_reg_ledger_rows_are_append_only` が所有者の接続で `TRUNCATE core.outage_ledger` を取引の中で撃ち（拒まれなかったときに共有の試験 DB を空にしないため戻す）、「追記のみ」で拒まれることを見る。`check-immutable.sh` には足さない（試験で固定した）

## R16. 台帳の `delivered` は「サーバが返した」で、端末が通知を出せなかったときも残る（C6 は「鳴らしたこと」を残し「いつ気づけたか」を数えると書いている）
- 成果物: crates/server/src/outage_store.rs:231（判定と同じ取引で `delivered`）/ collector-android/…/OutagePoller.kt:55-62（問い合わせの後に `notifier.show`）/ openspec/changes/st14-outage-alert/deep.md:60（C6）
- 根拠: サーバは応答を返す前の取引で `delivered` を書く（`st14_ledger_delivered_per_device` が HEAD で通る）。携帯は問い合わせた後に通知を出し、権限が無ければ出さない
  （`OutageNotifierTest` の「通知を出せなくても収集は止まらない」が HEAD で通る —— 通知 0 件）。この 2 つが同時に起きると、台帳は「端末に届けた」、端末には何も出ていない。
  応答を受け取る前に端末が 10 秒で打ち切った場合も同じ。spec の Scenario は「返した」と書いているので spec とは合っているが、C6 の理由（後から「いつ気づけたか」を数える）には使えない。台帳は追記のみなので後から直せない
- kind: premise
- 提案: まず C6 の理由を言い直す（`delivered` は「返した」で、「気づけた」とは読まない）。「いつ気づけたか」を数える必要が残るなら、端末が通知を出せたことを別の行（例: `shown`）で返す口を足すかを design に D<n>（仮）で書く
- 処置: escalated —— 端末で出せたかどうかは追記のみの台帳に後から足せない（`uncaptured`）。deep.md の「下流 finish で積んだ A」Q-R16（`deep-questions-r3.json` / `docs/briefs/ST14-deep-r3.html`）
  → 答え（2026-10-15。`deep-answers-6.txt`）: C6 の理由を言い直す（返したことだけ残す）。deep.md の C6 は record の段が言い直した。残っていた同じ言い回し 2 か所（`specs/collection-coverage/spec.md:362` の本文・`go.json:469`）を final の段で「いつ端末に返したか」へ言い直した。Requirement の SHALL と Scenario・コードは変えない（既に「返した」で一致）。端末で出せたかは残さない（`uncaptured` を受け入れた）

## scoped re-review（5589c6d..0c60901。code-verify の R11〜R15）

席: `re-review-prompt.md`。package は `.superpowers/sdd/tasks/review-5589c6d4..0c609019.diff`（2 commit）、fix の報告は
`.superpowers/sdd/st14-final-fix-report.md` の末尾。R11〜R15 はすべて ADDRESSED、Critical / Important の新しい破損なし、範囲外の観察なし。
Minor 2 件（`warming_up` / `after_warmup` が実時間の経過で境界から揺れる）は当たらない —— 試験の App は `.at(now)` で時刻を固定し、
`uptime()` は `cfg(test)` で `now - started_at` を返す（`crates/server/src/lib.rs:445-449`）ので、29 分 59 秒・30 分ちょうどはそのまま比べられる。
R16 は escalated（deep.md Q-R16）で対象外。

## final review（6613ee6..f8aa9d0。Q-R16 の答えの後）: 指摘なし

前回の final review（e32f33a..078d803）・code-verify（5589c6d4）・scoped re-review（5589c6d..0c60901）の後、ブランチは main（#91 `fix/verify-worktree-ports`）の上へ rebase された。
rebase 前の 0c609019 と HEAD の差は、#91 が持ち込んだ `tools/ports.sh`・`tools/verify-prep.sh`・`crates/server/src/testdb.rs` と、
`openspec/` `docs/` の記録（evidence・答え・deep.md・PR 本文）だけで、**この Story のコードは前回の review と re-review が見たものと同じ**
（`git diff --stat 0c609019 HEAD -- . ':!openspec' ':!docs' ':!tools/ports.sh' ':!tools/verify-prep.sh' ':!crates/server/src/testdb.rs'` が空）。
同じ範囲に whole-branch review を重ねない。この段でやったのは R16 の答えを spec の本文と go.json に写すことだけ（上の R16 の処置）。
申し送り（`docs/handoff/ST14.md`）は前回の final から増えていない（最後の変更は 00398a42）。合意（`agreement.json`）は下流の design の書き戻し（R4 など）で既に stale で、下流の gate はそれを見ない。
