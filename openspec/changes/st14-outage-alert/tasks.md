# ST14 実装タスク — 収集が途切れたら気づける

読む順: `deep.md`（**最優先。本人が決めた 9 件と、聞かずに決めた既定 C1〜C13**）→ このファイル →
`specs/collection-coverage/spec.md` → `design.md` → `review/spec.md`（処置の理由）→ `docs/stories/ST14.md` →
`docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `./tools/db.sh up -d --wait db && ./tools/db-roles.sh` が前提（`.env` の `DATABASE_OWNER_URL`）。

## Global Constraints

- **spec は `collection-coverage` だけ**（proposal の Capabilities）。`external-ingestion` / `device-collection` / `desktop-collection` の正典の文は直さない（`docs/handoff/ST47.md` / design D20）。
  並走中の st08 / st17 / st21 / st25 の change のファイルを触らない
- **移行は 1 本だけ足す**（design D2。`migrations/YYYYMMDDHHMM_outage_alert.sql` と `.down.sql`。名前は作成時刻。連番にしない）。`crates/server/src/lib.rs` の `MIGRATIONS` の末尾に足し、配列の長さも増やす
- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 18 時間ちょうどでは途切れにならない`）。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 153 本**（ADDED 97 本 / MODIFIED 56 本）。MODIFIED のうち 44 本（`途絶` 6 本・`ソース × 日` 11 本・`開始日` 11 本・`達成` 16 本）は正典に元からあり、**既存の試験の印がそのまま使える**。
  その試験は緑のまま通ること（Task 2.3 / 3.3 / 4.2）。**新しい Scenario 109 本**（ADDED 97 本と MODIFIED に足した 12 本）は下の Task が名指しする（`check_scenarios.py` の「担保なし 109 件」が上流での正常値）。
  THEN / AND が 2 つ以上の主張を持つ Scenario は、試験が主張ごとに別々に assert する（spec-review R10）
- **本人の決定（下流は変えない）**: 両方の収集アプリが問い合わせて OS の通知・外の push は作らない（Q1）/ 取得できる状態の生存信号だけを数える（Q2）/
  端末のソースは端末で作られた時刻・書庫は運んだ最後の書庫の時刻（Q3）/ 音は途切れ 1 回につき端末ごとに 1 回・続く間は音なし・戻ったら黙って消す（Q4）/
  1 通にまとめ新しい途切れでだけ鳴る（Q5）/ `c01-clock` は入れない（Q6）/ 取りこぼしの期間の日は③・理由を分けて出す（Q7）/
  集計を格子に出し開始日は `c01-app-usage`（Q8）/ **書庫は最後の書庫から 75 日**（Q9。A）
- **本人の決定の値は定数にして試験が名指しで固定する**（`docs/testing.md` §3）: `ALERT_GAP_FACTOR = 3`・`ARCHIVE_ALERT_AFTER_DAYS = 75`・`OUTAGE_WARMUP = 30 分`（D5（仮））・`OUTAGE_NOTIFICATION_ID = 3`・
  `NON_OBSERVATION_GRID` / `NON_OBSERVATION_ALERT`（D4）・`DEVICE_SUBJECT` / `USAGE_SUBJECT` / `PC_SOURCES`（D17）
- **D5 / D8 / D9 / D10 / D14 / D22 / D23 / D24 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号を書き直す
- **「人間の確認待ち」に逃がせる Scenario は無い。** 通知の決め方は単体、Android の通知は Robolectric、Windows のトーストは `tests/runtime_windows.rs`、画面は e2e。
  人間の確認待ちに置くのは物理（音・実機の量）だけ（末尾の節）
- **件数つき検証**: このファイルで
  **`CT <名前>`** は `bash -o pipefail -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`、
  **`WU <名前>`** は `bash -o pipefail -c 'scripts/quiet-run wu -- cargo test -p ashiato-collector-windows --lib <名前> 2>&1 | tee /tmp/wu.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/wu.log`、
  **`KT <クラス>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*<クラス>*'` が rc=0 で、
  `grep -ho '<testcase ' collector-android/app/build/test-results/testDebugUnitTest/TEST-*<クラス>*.xml | wc -l` が 1 以上、
  **`VT <ファイル>`** は `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/<ファイル> 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log`、
  **`ET <ファイル>`** は `bash -o pipefail -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && npx playwright test e2e/<ファイル> 2>&1 | tee /tmp/et.log' && grep -Eq '[1-9][0-9]* passed' /tmp/et.log`
  が rc=0 になることを指す（`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **試験の名前の接頭辞は Task ごとに重ならない**: `st14_reg_`（1）/ `st14_grid_`（2）/ `st14_rollup_`（3）/ `st14_ach_`（4）/ `st14_judge_`（5・6 の純粋な判定）/ `st14_inputs_`（5・6 の材料の SQL）/
  `st14_api_outages_`（7）/ `st14_ledger_`（7）/ Windows の `outage_notice_`（9）。作ったら `cargo test -p ashiato-server st14_ -- --list` にこの change の試験しか出ないことを確かめる
- 記録の値（座標・題名・URL・アプリ名）を**ログにも通知にも出さない**（製造準備 A-2 / C5）。出すのは件数・日時・論理ソース名・表示名・利用者だけ

## Task 1: 移行と登録簿（判定の対象の印・通知までの日数・開始日を借りる先・台帳の表）

`migrations/YYYYMMDDHHMM_outage_alert.sql`（新規）と `.down.sql`、`crates/server/src/lib.rs` の `MIGRATIONS`、`crates/server/src/archive/worker.rs` の `ensure_myactivity_source`。
試験は `crates/server/src/outage_registry_tests.rs`（新規）。design D2 / D7。

- [x] 1.1 前提を確かめる: `crates/server/src/coverage.rs` に `DEVICE_SUBJECT` があり、`core.source` に `outage_target` / `alert_after_days` / `started_on_from` の列がまだ無い。`core.archive_ledger` と `core.archive_ledger_source` がある（ST12 が main にある）。
  検証: `bash -c 'grep -q "DEVICE_SUBJECT" crates/server/src/coverage.rs && grep -q "archive_ledger_source" migrations/202609181600_archive_ingestion.sql'` rc=0
  ★ 2026-10-14 訂正: `outage_target` の否定を外した。受け入れ条件は変えない。理由: 着手前の確認（移行に outage_target が無い）なので、1.2 で移行を書いた後は必ず FAIL する（evidence.jsonl 1.1 の FAIL。head b2dfc1a1）。旧コマンドは bash -c 'grep -q "DEVICE_SUBJECT" crates/server/src/coverage.rs && ! grep -rq "outage_target" migrations/ && grep -q "archive_ledger_source" migrations/202609181600_archive_ingestion.sql'。新しい入口の最初の語が repo の実行ファイルでないので plan_fix.py を通さず手で直した。承認: 2026-10-14 本人（deep-answers-3.txt の Q-T1「承認する」）。記録: plan-corrections.md
- [x] 1.2 移行を書く: 3 列（`outage_target boolean NOT NULL DEFAULT true` / `alert_after_days integer NULL CHECK (> 0)` / `started_on_from text NULL REFERENCES core.source` と自己参照を断る `CHECK`）、
  印（`c01-clock` / `c01-app-usage-rollup` / `s01-stay` / `s01-attribute` / `s01-feeling` / `s01-place` を `false`。無い名前は何もしない）、`c03-%` の `alert_after_days = 75`（`IS NULL` のときだけ）、
  集計の `started_on_from = 'c01-app-usage'` と `collection_started_on` の直し、台帳 `core.outage_ledger`（D7 の列・`CHECK`・一意索引 2 本・追記のみのトリガ）。`.down.sql` は台帳と 3 列を落とす。
  `ensure_myactivity_source` の `INSERT` に `alert_after_days = ARCHIVE_ALERT_AFTER_DAYS` を足す。
  Scenario: `時計の測定と集計と滞在と個人属性は対象外で登録されている` / `書庫のソースは 75 日で登録されている`（実行時に足すマイアクティビティの製品の行も含めて見る） /
  `本人が変えた通知までの日数は移行を当て直しても戻らない` / `汚れた集計の開始日は移行で直る` / `途切れの台帳の行は書き換えられない` / `途切れの台帳に記録の中身が載らない` /
  `実行時に足された製品のソースも 75 日で登録される`（取り込み器の `ensure_myactivity_source` を通して足した行を読む）。
  `ARCHIVE_ALERT_AFTER_DAYS == 75` と移行の 75 が一致することを試験で突き合わせる。
  検証: `CT st14_reg_`、`tools/check-migrations.sh` rc=0、`tools/check-immutable.sh` rc=0
- [x] 1.3 新しく登録したソースの既定が「対象」であること（移行の後に `testdb::source` で足した行の `outage_target` が `true`）。
  Scenario: `新しく登録したソースは判定の対象になる`（登録簿の側。判定の側は Task 5）。
  検証: `CT st14_reg_default`

## Task 2: 格子の材料（観測ではない記録・取りこぼしの期間・評価の順・応答の `gaps`）

`crates/server/src/coverage.rs` の `facts()` / `active_days` / `DayFacts` / `decide()` / `DayCell`。試験は `crates/server/src/coverage/tests/nonobs.rs`（新規）。design D4 / D15。

- [x] 2.1 `NON_OBSERVATION_GRID`（`("c01-app-usage","gap")`・`("c02-window","clock-skew")`）を定数にし、`facts()` の記録の数えと `active_days` から除く。稼働記録の件数（`core.coverage`）は変えない。
  Scenario: `時計の測定しか無い日は記録ありにならない` / `時計の測定しか無く生存信号も無い日は途絶になりうる` / `前景の記録がある日は時計の測定があっても記録あり` /
  `格子で数えない記録も読み出せる` / `格子で数えない記録も稼働記録の件数には入る` / `取りこぼしの期間を主張する 1 件だけでは記録ありにならない` / `時計の測定だけが届く期間も途絶になる`（MODIFIED「途絶」）。
  検証: `CT st14_grid_nonobs`
- [x] 2.2 取りこぼしの期間を `c01-app-usage`（とその鎖の名前）の `kind = 'gap'` の記録から `[begin, end)` で取り、`Asia/Tokyo` の日に配る（D15 の SQL）。`DayFacts.gaps` と `DayCell.gaps`（`{reason, begin, end}`）を足し、OpenAPI に出す。
  `decide()` の (5) の後に「重なる取りこぼしがあれば③」を足す（spec の評価の順 (1)〜(10)）。
  Scenario: `取りこぼしの期間の日は取得できる状態の信号があっても取れない状態になる` / `イベントの記録がある日は取りこぼしの期間が重なっても記録あり` /
  `取りこぼしの期間より停止が優先される` / `終わりの時刻ちょうどの日には重ならない` / `UTC の真夜中に終わる取りこぼしは日本時間のその日に重なる` / `稼働状況の応答の日が取りこぼしの期間と理由を持つ`。
  検証: `CT st14_grid_gap`、`tools/check-openapi.sh` rc=0
- [x] 2.3 正典に元からある `途絶` / `ソース × 日` の Scenario の試験が、評価の順を変えた後も緑のまま通る（印は `coverage/tests/{states,retired,drops,spans}.rs` などに散っている。spec-review R12）。
  検証: `CT coverage::tests::`（`-- --list` で 1 本以上）、`CT st14_grid_`

## Task 3: 集計の開始日と格子の並び（サーバ）

`crates/server/src/coverage.rs` の登録簿の読み出し・`resolve_chain` の後・`touch_started_on`・`coverage_get` の名前の並び。試験は `crates/server/src/coverage/tests/rollup.rs`（新規）。design D10（仮）/ D16。

- [x] 3.1 `started_on_from` を持つソースは借り先（の鎖の根）の収集開始日を自分のものとして返す。`touch_started_on` は `started_on_from` を持つソースの記録では何もしない（重複の到着も）。
  Scenario: `集計の開始日はアプリ利用の開始日になる` / `集計の記録を送り直しても開始日は戻らない` / `アプリ利用の開始日が遡ると集計の開始日も遡る`。
  検証: `CT st14_rollup_started`
- [x] 3.2 `GET /coverage` の並びを「Must の 5 本 → `c01-app-usage-rollup` → `c03-%`」にする。達成（`must_sources()`）には入れない。
  Scenario: `集計の格子が Must の後ろで書庫の前に出る`（サーバの並び。画面は Task 10）/ `集計は達成に数えられない`。
  検証: `CT st14_rollup_order`
- [x] 3.3 正典に元からある `開始日` の Scenario の試験が緑のまま通る（印は `coverage/tests/*.rs` と `api_tests.rs` にある）。検証: `CT coverage::tests::`、`CT api_tests`（どちらも `-- --list` で 1 本以上）

## Task 4: 達成の主語と PC の分母

`crates/server/src/coverage.rs` の `DEVICE_SUBJECT` / `USAGE_SUBJECT` / `PC_SOURCES` / `must_sources()` / 達成の数え。試験は `crates/server/src/coverage/tests/achievement_subject.rs`（新規）。design D17。

- [x] 4.1 アプリ利用を利用が主語へ移す。`must_sources()` の並び（位置・アプリ利用・写真・ウィンドウ・ブラウザ履歴）は変えない（主語と並びを別の定数にする）。
  `PC_SOURCES` の分母を「停止と退役を除いた日のうち、生存信号が 1 件でも届いた日」にする。値は名指しで固定する。
  Scenario: `アプリ利用は取得できる状態の信号で数える` / `PC のソースは生存信号の届いた日だけを分母に数える` / `携帯のソースは生存信号が届かない日も分母に数える`。
  検証: `CT st14_ach_`
- [x] 4.2 正典に元からある `達成` の Scenario の試験を、主語の変更に合わせて直す（アプリ利用を端末が主語として固定していた試験があれば、`アプリ利用は取得できる状態の信号で数える` に合わせる。印は残す）。
  検証: `CT coverage::tests::`（`achievement.rs` / `retired.rs` / `states.rs` の `達成` の印を含む。`-- --list` で 1 本以上）、`web` の `AchievementPanel` の単体（`cd web && npx vitest run` rc=0）

## Task 5: 途切れの判定 —— 対象と端末のソース

`crates/server/src/outage.rs`（新規。純粋な `judge`）と `crates/server/src/outage_store.rs`（新規。`inputs` の SQL）。
試験は `outage.rs` の `#[cfg(test)]`（`st14_judge_`）と `crates/server/src/outage_tests.rs`（新規。本物の DB。`st14_inputs_`）。design D1 / D3 / D4 / D8（仮）/ D9（仮）。

- [x] 5.1 `judge(&[SourceInput], now)` を書く。対象の判定（印・出どころ・退役・開始日・いまの停止）→ 最後に生きていた時刻 → 閾値（`expected_gap_sec * ALERT_GAP_FACTOR`、厳密な `>`）。
  取得できない状態の生存信号は blockers を問わず数えない（D8（仮））。純粋な単体で全部の分岐を固定する。
  Scenario: `新しく登録したソースは判定の対象になる` / `対象外の印のソースは途切れにならない` / `本人が書いた記録のソースは印が無くても途切れにならない` /
  `収集した記録と本人が書いた記録が混ざったソースは判定の対象のまま` / `導出した記録のソースは途切れにならない` / `引き継いだソースは届く前から判定の対象になる` / `退役した日からは途切れにならない` / `収集を始めていないソースは途切れにならない` / `停止している間は途切れにならない` /
  `位置の記録が 18 時間を超えて来ないと途切れになる` / `18 時間ちょうどでは途切れにならない` / `取得できる状態の生存信号があれば記録が無くても途切れにならない` /
  `取れない状態の生存信号だけが届き続けると途切れになる` / `停止が終わった時刻から数え直す` / `PC を閉じていた時間も経過に数える` / `想定間隔を変えると判定も変わる`。
  検証: `CT st14_judge_`
- [x] 5.2 `inputs` の SQL を書く: 登録簿 × 観測の記録の最後の `event_time`（`NON_OBSERVATION_ALERT` を飛ばす。未来の時刻は `ingest_time`）× 取得できる状態の生存信号の最後の `emitted_at`（未来は `received_at`）×
  出どころ（`origin <> 'collected'` の有無）× いまの停止と最後に終わった停止。本物の DB に記録と生存信号を置いて、`inputs` → `judge` の結果を見る。
  Scenario: `時計の測定だけが届き続けても途切れになる` / `取りこぼしの期間の主張は生きていた時刻に数えない` / `離席と停止と除外の印は生きていた時刻に数えない` /
  `溜めてから送った記録は出来事の時刻で測る` / `未来の時刻の記録は途切れを隠さない` / `本人が書いた記録のソースは印が無くても途切れにならない`（DB の側）/ `取れない状態の生存信号だけが届き続けると途切れになる`（DB の側）/
  `削除済みの記録も生きていた時刻に数える` / `破棄の報告は生きていた時刻に数えない` / `破棄の報告だけでは途切れにならない`。
  あわせて `st14_inputs_nonobs_registry`: 登録簿の `c02-%` と `c01-app-usage%` の名前がどれも `NON_OBSERVATION_ALERT` の表で扱われているか、扱わないと明示した表にあることを見る（D4）。
  検証: `CT st14_inputs_device`、`CT st14_inputs_nonobs_registry`

## Task 6: 途切れの判定 —— 書庫と取り込み器

`outage.rs` / `outage_store.rs`。試験は同じファイル。design D1 / D4。

- [ ] 6.1 `Archive` の最後に生きていた時刻（`outcome = 'read'` の台帳 × `archive_ledger_source` の `max(coalesce(created_at, discovered_at))`）と、`alert_after_days` の閾値。
  `ServerHeartbeat`（`s01-archive-inbox`）は取得できる状態の生存信号だけで測る。
  Scenario: `最後の書庫から 75 日を過ぎると書庫のソースが途切れになる` / `2 か月ごとの書き出しでは書庫のソースは途切れにならない` / `新しい行の無い書庫でも最後の書庫は進む` /
  `後の書庫が古い出来事を運ばなくても最後の書庫で測る` / `形の確認を待っている書庫は最後の書庫に数えない` / `取り込み器は生存信号から 3 日で途切れになる` /
  `作られた時刻の無い書庫は見つけた時刻で測る` / `項目が 0 件の書庫では最後の書庫は進まない` / `運んだ書庫が台帳に無いソースは届いた時刻で測る` / `使わなかった製品は途切れにならない`。
  検証: `CT st14_judge_archive`、`CT st14_inputs_archive`
- [ ] 6.2 格子と通知の測り方の違いを固定する（同じ入力で `GET /coverage` の 61 日目が「途絶」。判定の側は 6.1）。
  Scenario: `使わなかった製品は格子では途絶に見える`。
  検証: `CT st14_inputs_archive_grid`

## Task 7: `GET /outages` と台帳

`crates/server/src/lib.rs`（ハンドラと router）と `outage_store.rs`（台帳の取引）。試験は `crates/server/src/outage_api_tests.rs`（新規）。design D5（仮）/ D6 / D7。

- [ ] 7.1 `GET /outages?device_id=&user_id=` を足す。認証は `authorize()`。起動から `OUTAGE_WARMUP` の間は `warming_up`（起動時刻は `AppState` の差し替えられる時計）。応答の型を OpenAPI に出す。
  Scenario: `途切れたソースの表示名と最後に生きていた時刻が返る` / `途切れの応答に記録の中身が含まれない`（応答の JSON に緯度・経度・`payload`・`raw` の鍵も値も無いことを文字列で見る）/
  `収集アプリの資格情報で途切れの判定が読める`（Bearer の `API_TOKEN` で 200。**ST44 が締めたら落ちる試験** —— 名前は `st14_api_outages_bearer`）/ `画面の資格情報で途切れの判定が読める`（ログインの cookie で 200）/
  `資格情報の無い途切れの問い合わせは断られる` / `起動して 30 分の間は判定を待っていると返る` / `起動して 30 分が経つと判定が返る` / `途切れが無ければ空の一覧が返る` /
  `端末識別子の無い問い合わせにも判定が返る` / `途切れの一覧は表示名の順に返る`。
  検証: `CT st14_api_outages_`、`tools/check-openapi.sh` rc=0
- [ ] 7.2 台帳への追記を 1 回の取引で書く（助言ロック → 開いている途切れ → `opened` / `closed` → `delivered`）。2 本の問い合わせを同時に走らせて `opened` が 1 行であることも見る。
  Scenario: `途切れが続く間は同じ識別子が返る` / `新しく途切れると台帳に始まりが 1 行残る` / `戻ると台帳に戻りが 1 行残る` / `戻った後にまた途切れると別の識別子になる` /
  `端末に初めて返した途切れが台帳に残る` / `同じ途切れでも端末ごとに 1 回鳴る`（サーバの側: 2 つの端末識別子にそれぞれ `delivered` が 1 行）/ `戻ったソースは応答の一覧から消える` /
  `端末識別子の無い問い合わせは届けたことを残さない` / `判定を待つ間は台帳に書かない`。
  検証: `CT st14_ledger_`

## Task 8: 携帯の通知（チャネル `outage`・生存信号の契機）

`collector-android/app/src/main/kotlin/dev/ashiato/collector/` に `OutageNotice.kt`（純粋な決め方）・`OutagePoller.kt`・`OutageNotifier.kt`（新規）、`LocationService.kt` は契機に 1 行。
試験は `app/src/test` の `OutageNoticeTest.kt` / `OutageNotifierTest.kt` / `OutagePollerTest.kt`（新規。JUnit4 + Robolectric）。design D12 / D13 / D14（仮）。

- [x] 8.1 `OutageNotice.decide(state, result)` を design D13 の表のとおりに書く（`Unreachable` / `WarmingUp` / `Ready`）。鳴らした途切れの覚えは `SharedPreferences`。
  Scenario: `新しい途切れで音つきの通知が 1 通出る` / `続く途切れでは音を鳴らさずに通知が残る` / `同時に途切れたソースは 1 通にまとまる` / `新しく途切れたソースが増えたときだけまた鳴る` /
  `途切れたソースが減ると音を鳴らさずに差し替わる` / `戻ると通知が黙って消える` / `サーバに届かない間は通知が変わらない` / `判定を待っている間は通知が変わらない` / `同じ途切れでも端末ごとに 1 回鳴る`（端末の側: 前の状態が空なら必ず鳴る）/
  `収集アプリを起動し直しても同じ途切れでは鳴らない`（`SharedPreferences` に書いた後、新しい `OutageNotice` で読み直す）。
  検証: `KT OutageNoticeTest`
- [x] 8.2 `OutageNotifier`: チャネル `outage`（IMPORTANCE_DEFAULT）、`OUTAGE_NOTIFICATION_ID = 3`、音なしは `setSilent(true)`、文面は D14（仮）。Robolectric の `ShadowNotificationManager` でチャネル・ID・音の有無・本文を見る。
  Scenario: `携帯の途切れの通知は専用のチャネルで出る`（`location` とも `retention` とも違う）/ `途切れの通知に記録の中身が載らない`（本文が表示名・日時・定まった文言だけ）/ `新しい途切れで音つきの通知が 1 通出る`（OS の側）/ `戻ると通知が黙って消える`（OS の側）/
  `通知を出せなくても収集は止まらない`（`POST_NOTIFICATIONS` を外した Robolectric で、通知が 0 件・ログが 1 行・送信の箱に記録が残る）。
  検証: `KT OutageNotifierTest`
- [x] 8.3 `OutagePoller`: `GET /outages?device_id=` を 1 回（10 秒で打ち切り。既存の `HttpTransport`）。**位置**の生存信号のスケジューラだけで `emit()` の後と起動時の 1 発の後に呼び、`poll` → 決め方 → 通知 → 覚えの書き込みを単一スレッドの executor で直列にする（design D12）。
  偽の transport とスケジューラで、契機ごとに 1 回出ること・端末識別子が付くこと・位置・アプリ利用・写真の 3 ソースが同時に動く構成でも 1 回であることを見る。
  `AlarmManager` / WorkManager を使っていないことを `grep` で確かめる。
  Scenario: `携帯は生存信号の契機で問い合わせる` / `ソースが複数あっても携帯の問い合わせは契機ごとに 1 回`。
  検証: `KT OutagePollerTest`、`bash -c '! grep -nE "AlarmManager|WorkManager" collector-android/app/src/main/kotlin/dev/ashiato/collector/Outage*.kt'` rc=0、
  `scripts/quiet-run lint -- ./collector-android/gradlew -p collector-android detekt :app:assembleDebug` rc=0

## Task 9: PC の通知（Windows のトースト・時計の測定の契機）

`crates/collector-windows/src/outage_notice.rs`（新規。純粋。ubuntu でも走る）・`toast.rs`（新規。`cfg(windows)`）・`runtime.rs` の契機・`Cargo.toml` の `windows` 依存。
試験は `outage_notice.rs` の `#[cfg(test)]` と `tests/runtime_windows.rs`。design D11 / D13 / D14（仮）。

- [x] 9.1 `outage_notice::decide` を D13 の表のとおりに書く（Kotlin と同じ表・同じ Scenario の印）。鳴らした途切れの覚えは `outage-notice.json`（状態の置き場）に書き、読み直せることも見る。
  Scenario: `新しい途切れで音つきの通知が 1 通出る` / `続く途切れでは音を鳴らさずに通知が残る` / `同時に途切れたソースは 1 通にまとまる` / `新しく途切れたソースが増えたときだけまた鳴る` /
  `途切れたソースが減ると音を鳴らさずに差し替わる` / `戻ると通知が黙って消える` / `サーバに届かない間は通知が変わらない` / `判定を待っている間は通知が変わらない` /
  `同じ途切れでも端末ごとに 1 回鳴る` / `途切れの通知に記録の中身が載らない`（トーストの XML の文字列を組んで見る。音なしのときは `<audio silent="true"/>` を持つ）/
  `収集アプリを起動し直しても同じ途切れでは鳴らない`（`outage-notice.json` を書いた後に読み直す）/ `通知を出せなくても収集は止まらない`（通知の口が失敗を返す偽物で、決め方と送信の契機が続くことを見る）。
  検証: `WU outage_notice_`
- [x] 9.2 `runtime.rs` の `maybe_measure_skew` の契機で `GET /outages?device_id=` を 1 回呼ぶ（別スレッド・10 秒で打ち切り）。偽の transport と単調時計で、契機ごとに 1 回・端末識別子つきで出ることを見る。
  Scenario: `PC は時計の測定の契機で問い合わせる`。
  検証: `WU outage_notice_poll`、`cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0（CI の `collector-windows` job と同じ）
- [x] 9.3 `toast.rs`: AUMID を `HKCU\Software\Classes\AppUserModelId\dev.ashiato.collector` に `winreg` で登録し（unsafe を書かない。作業場は `unsafe_code = "forbid"`）、`Tag = "outage"`・`Group = "ashiato"` で出す・差し替える（音なしは `SuppressPopup = true`）・消す。
  `windows` crate の機能は `UI_Notifications`・`Data_Xml_Dom`・`Foundation` の 3 つだけ（design D11）。登録簿に書けなければ警告だけで収集を止めない。
  実行時テストで、出した後に `History().GetHistoryWithId(AUMID)` に 1 件・差し替えた後も 1 件で `SuppressPopup` が真・消した後に 0 件を見る。
  Scenario: `PC の途切れの通知は Windows の通知として出る` / `PC の音なしの差し替えは画面の端に出ない`。
  検証: `bash -c 'rsync -a --delete --exclude target --exclude .git --exclude node_modules ./ /mnt/c/dev/ashiato2-rt-st14/ && /mnt/c/Windows/System32/cmd.exe /c "cd /d C:\dev\ashiato2-rt-st14 && set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p ashiato-collector-windows --test runtime_windows outage -- --test-threads=1"'` rc=0（`-- --list` で 1 本以上。CI の `collector-windows-runtime` job。手元は WSL から Windows 側の cargo）
  ★ 2026-10-14 訂正: WSL から Windows 側の cargo を呼ぶ形（ST08 の 10.1 / 10.2 と同じ）。受け入れ条件は変えない。理由: runtime_windows.rs は Windows でしかコンパイルされず、WSL の cargo では試験が 0 本で FAIL になる（evidence.jsonl 9.3 の fp=e49719f9f0 が 2 回）。旧コマンドは cargo test -p ashiato-collector-windows --test runtime_windows outage -- --test-threads=1。新しい入口の最初の語が repo の実行ファイルでないので plan_fix.py を通さず手で直した。承認: 2026-10-14 本人（会話で「st14 は直して良い」）。記録: plan-corrections.md

## Task 10: 画面（取りこぼしの週の文字・集計の格子・明暗）と偽データ

`web/src/coverage.ts`（`DayCell.gaps`・文字の組み立て）・`messages.ts`（文言）・`CoverageGrid.tsx`（`WeekDetail`）・`archives.ts`（並び）・`tools/seed.sh`。
試験は `web/src/__tests__/coverage-gaps.test.ts`（新規。jsdom）と `web/e2e/coverage-gaps.spec.ts` / `coverage-rollup.spec.ts` / `coverage-scheme.spec.ts`（新規。本物の Chromium）。design D10（仮）/ D18 / D21。

- [ ] 10.1 偽データ（D18）: `SEED=normal` に取りこぼしの期間 2 本（理由が違う。`2026-09-01T08:00+09:00〜2026-09-03T08:00+09:00` と同じ週の別の日）・ウィンドウの時計の測定だけの日・集計の記録（開始日より前の箱を含む）を足す。
  検証: `bash -c 'set -a; . ./.env; set +a; . tools/ports.sh; rm -f /tmp/st14-stack.log /tmp/cov.json; STACK_RESET=1 setsid tools/stack.sh up >/tmp/st14-stack.log 2>&1 & pid=$!; for i in $(seq 1 300); do grep -q "^画面: http" /tmp/st14-stack.log && break; kill -0 $pid 2>/dev/null || break; sleep 2; done; curl -sf -H "authorization: Bearer ${API_TOKEN}" "http://127.0.0.1:${ASHIATO_HTTP_PORT}/coverage?from=2026-08-01&to=2026-09-30" > /tmp/cov.json; rc=$?; kill -- -$pid 2>/dev/null; wait $pid 2>/dev/null; exit $rc' && jq -e '(map(select(.logical_source=="c01-app-usage-rollup")) | length == 1) and ([.[].days[]? | select((.gaps // []) | length > 0)] | length > 0)' /tmp/cov.json` rc=0
  ★ 2026-10-14 訂正: スタックを裏で起こし（setsid）→ 起動を待つ → ASHIATO_HTTP_PORT を読む → 止める、までを 1 本の bash -c にした。受け入れ条件（jq の判定）は変えない。理由: tools/stack.sh up は前景で待つので && の後ろの curl に着かず（verify-run が 9 分以上返らなかった）、読み先の ${ASHIATO_PORT:-18787} は存在しない変数で、既定の 18787 は main の本番のサーバ。旧コマンドは bash -c 'set -a; . ./.env; set +a; tools/stack.sh up >/dev/null && curl -sf -H "authorization: Bearer ${API_TOKEN}" "http://127.0.0.1:${ASHIATO_PORT:-18787}/coverage?from=2026-08-01&to=2026-09-30" > /tmp/cov.json'（その後に同じ jq -e）。新しい入口の最初の語が repo の実行ファイルでないので plan_fix.py を通さず手で直した。承認: 2026-10-14 本人（deep-answers-4.txt の Q-T10「1 コマンドの入口に直す」）。記録: plan-corrections.md
  （応答の鍵の名前は Task 2.2 で決めた `DayCell` に、ポートは tools/stack.sh が出す値に読み替える。spec-review R13。★ 2026-10-14: 訂正後のコマンドは ASHIATO_HTTP_PORT を読むので読み替えは要らない。tools/stack.sh の backtick は verify-run が検証コマンドとして拾い前景で走らせるので外した）
- [ ] 10.2 週の文字: 取りこぼしの期間が重なる日に、理由ごとに「取得元に残っていない（期間）」「時計のずれで取れていない（期間）」、知らない理由は「取りこぼし（期間）」（design D23（仮））。期間は `YYYY-MM-DD HH:MM〜YYYY-MM-DD HH:MM`（`Asia/Tokyo`）。文言は `messages.ts`。
  jsdom で文字の組み立てを、e2e で本物の画面の文字を見る。
  Scenario: `取得元に残っていない期間が週の文字に出る` / `時計のずれで取れていない期間は別の文字で出る` / `2 つの理由が同じ日に重なると 2 つとも出る` / `知らない理由の取りこぼしは理由の文字を付けずに出る`。
  検証: `VT coverage-gaps.test.ts`、`ET coverage-gaps.spec.ts`
- [ ] 10.3 集計の格子: サーバの並び（Task 3）のまま Must の後ろ・書庫の前に出し、開いた直後から直近 4 週を出す。360 × 640 で Must の最後の格子の下端が押し下がらないことを測る（`archive-layout.spec.ts` と同じ測り方）。
  Scenario: `集計の格子が Must の後ろで書庫の前に出る` / `集計の格子を足しても Must の 5 本はひとスクロール以内` / `集計の格子は開いた直後から直近 4 週を出す`。
  検証: `ET coverage-rollup.spec.ts`、`ET archive-layout.spec.ts`（既存。緑のまま）
- [ ] 10.4 明暗（コードは変えない）: `emulateMedia({ colorScheme })` の `light` / `dark` / `no-preference` で背景色を測り、ライトで 3 段・破棄の印・本文の比を測る（比の下限はリテラルで持つ）。
  Scenario: `OS がライトなら稼働状況もライトで出る` / `OS がダークなら稼働状況もダークで出る` / `明暗の設定が取れなければダークで出る` / `ライトでも格子の 3 段と印と文字が下限を満たす`。
  検証: `ET coverage-scheme.spec.ts`、`ET surface.spec.ts`（既存。緑のまま）
- [ ] 10.5 画面の検査一式。検証: `bash -c 'cd web && npm run lint && npx tsc --noEmit && npx vitest run && npm run build'` rc=0

## Task 11: 縦串と全体の検査

- [ ] 11.1 縦串（`tools/smoke.sh`）に `GET /outages` を 1 本足す（起動直後は `warming_up`、`OUTAGE_WARMUP` を環境変数で 0 にした起動では `ready`）。値の待ち時間は既定のまま、試験だけが短くする口を持つ。
  検証: `scripts/quiet-run smoke -- tools/smoke.sh` rc=0
- [ ] 11.2 Scenario の担保: `python3 scripts/check_scenarios.py . st14-outage-alert` rc=0（担保なし 0・名無しの確認待ち 0）
- [ ] 11.3 全体: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && scripts/quiet-run all -- cargo test --workspace'` rc=0、
  `tools/check-openapi.sh` / `tools/check-migrations.sh` / `tools/check-immutable.sh` / `tools/check-log-private.sh` rc=0

## 人間の確認待ち（物理だけ）

- 携帯の途切れの通知で音が 1 回鳴り、次の契機では鳴らないことを、本人の携帯で 1 回確かめる（携帯の位置の収集を 24 時間止めて PC の通知も見る）。
  > 物理: device —— 通知の音と鳴り方は端末の設定（マナーモード・通知の重要度の上書き）に依り、エミュレータでは鳴ったかを聞けない
- アプリ利用の未送信の量を本人の実機で数える（申し送り st06 R31。design D19）: `tools/usage-volume.sh --device <serial>` を 1 回。rc=1（上限超過）なら種別を絞るかを本人に聞く（下流は決めない）。rc=2 は測れなかったとして残す。
  > 物理: device —— 本人の使い方の 10 日ぶんの実イベントは本人の実機にしか無い
