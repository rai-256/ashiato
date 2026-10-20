# ST13 実装タスク — アカウント系のソースを定期取得する（登録簿の取り方と間隔・定期取得の枠・NFR-12 の直し）

読む順: `deep.md`（**最優先。本人が決めたこと**。この版は推奨の答えで書いた）→ このファイル → `specs/external-ingestion/spec.md` → `specs/device-collection/spec.md` → `design.md`
→ `docs/stories/ST13.md` → `docs/handoff/ST13.md`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

触るのは `migrations/`（1 本）・`crates/server/src/`（`archive/{config,scan,worker,open}.rs`・新しい `scheduled.rs`・`lib.rs`・`outage_store.rs`・`bin/openapi.rs`・試験）・`web/src/`（`SettingsView.tsx`・新しい `sources.ts`・`messages.ts`）・`web/e2e/`（新しい spec）・
`collector-android/`（`MainActivity.kt`・`Heartbeat.kt`・新しい `UnusedAppRestrictions.kt`・試験）・`tools/`（`smoke.sh` の 1 行・新しい `smoke.d/` `seed.d/` のファイル・新しい検査）・`docs/archive-inbox.md`。
検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` と `tools/db-roles.sh` が前提（`docs/testing.md` §8）。

## Global Constraints（規律。**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 起動の直後に 1 回取りに行く`）、bash は `echo`。
  `scripts/check_scenarios.py . st13-scheduled-account-fetch` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change が足す Scenario は 50 本**（ADDED の Requirement の 34 本 + MODIFIED の 3 本に足した 9 本 + `device-collection` の 7 本）。MODIFIED で写した既存の 21 本は既存の印を生かす（名前を変えない）。
  THEN に主張が 2 つ以上ある Scenario は、試験が主張を 1 つずつ assert する
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで **`CT <絞り込み> (≥N)`** と書いたものは、
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [0-9]+ passed' /tmp/ct.log && test "$(cargo test -p ashiato-server <絞り込み> -- --list 2>/dev/null | grep -c ': test$')" -ge N`
  が rc=0 になることを指す（N 本以上あり、すべて通る）。走らせるのは repo の根から `scripts/quiet-run`（AGENTS.md の「長いコマンド」）。
  **`GT <絞り込み> (≥N)`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '<絞り込み>'` が rc=0 で、`collector-android/app/build/test-results/testDebugUnitTest/` の該当する結果の `<testcase` が N 個以上あることを指す
- **深掘りの答え（この版は推奨。答えが届いたら `deep.md` が正本）**: 定期取得するソースはいま無い・枠と登録簿の間隔だけ（Q1）/ 申し送りは R25・R34・M7・M8 だけ（Q2）/
  S-7 設定の「ソース」節に取り方でまとめて読むだけ・書庫は手作業と通知までの残り日数（Q3）/ 携帯の収集アプリの休止を初回に 1 回頼んで防ぐ（Q4）/ 形の確認の印は CLI のまま（Q5）/ 間隔は画面から変えない（Q6）
- **聞かずに決めた既定（Q1 の context）と D の対応**: 取得の間隔は想定間隔と別の列・想定間隔より長い間隔は断る → D1 / 前回の時刻を持たず、起動の直後に 1 回、戻らない時計 → D5。
  **D2 / D6 / D9〜D14 / D16 は（仮）決め。** 反転条件は `design.md`。変えたらその D 番号を書き直す
- **ST12 の格納の関門に触らない**（design D7。ST12 の D4 / 本人の決定 Q6）。記録は 1 件 1 トランザクションのまま。トランザクションにまとめるのは台帳の行と論理ソースごとの行だけ
- **`/ingest`・`/heartbeat`・`/archives/status`・`/coverage` の形を変えない。** 休止の除外の状態は生存信号の**原文**に載せる（design D12）。`device-collection` の「端末は想定間隔ごとに生存信号を送る」（ST09 が MODIFIED 中）の印を動かさない
- **定期取得の枠は壁時計を読まない**（`Utc::now` / `SystemTime` / `chrono::Local`。design D5）
- **ログに出すのは件数・論理ソースの名前・失敗の種別だけ**（ST12 の「取り込み器のログに記録の本文を出さない」）。定期取得の枠のログも同じ
- **移行の名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`**（と `.down.sql`）。`migrations/ORDER` の末尾へ 1 行。連番にしない
- **縦串・偽データは Story ごとの新しいファイル**（`tools/smoke.d/<時刻>_st13-*.sh`・`tools/seed.d/<時刻>_st13-*.sh`）。共有の台本の末尾に書き足さない（`tools/check-shared-appends.sh`）。
  `tools/smoke.sh` の `export ASHIATO_ARCHIVE_SCAN_SEC=1` の 1 行は**その場で書き換える**（design D4）
- **画面の Scenario は `web/e2e` で担保する**（`docs/testing.md` §4.5。`cd web && npm run test:e2e`）。アサートは数値と経路（`scrollWidth`・可視・要素の数）。スクリーンショット比較は使わない
- **ST09 と同じファイル（`MainActivity.kt`・`Heartbeat.kt`）を触る。** ST09 の gate 済みの SHA の形に継ぎ足し、ST09 の権限の要求の流れを壊さない（design D12）

## Task 1: 登録簿に取り方・契機・取得の間隔を足す移行（design D1 / D2 / D3）

- [ ] 1.1 移行 1 本で `core.source` に `acquisition` / `fetch_trigger` / `fetch_interval_sec` を足し、design D1 の名前付きの CHECK 制約 4 つ・D2 の `BEFORE INSERT` の trigger・D3 の既存の行の値（`WHERE acquisition IS NULL` の `UPDATE`）を入れ、最後に 2 列を `NOT NULL` にする。
  `c02-browser-history` の取得の間隔は `crates/collector-windows/src/` の履歴を読む間隔の定数を読んで写す。`.down.sql` は trigger・制約・列を落とす。`migrations/ORDER` の末尾へ 1 行。
  `worker::ensure_myactivity_source` と `testdb::source` は列を指定しないまま（D2 の trigger が入れる）。
  Scenario: `移行を当てると各ソースが取り方と契機を持つ` / `取り方の無いソースは登録簿に残らない` / `想定間隔より長い取得の間隔は断られる` / `取り方を指定せずに足したソースは名前から取り方が入る` /
  `前置きの分からないソースは取り方を指定しないと断られる` / `間隔ごとでない契機のソースに取得の間隔を持たせると断られる` / `製品ごとに増えるマイアクティビティのソースは書庫の取り方で入る` / `本人が変えた取得の間隔は移行を当て直しても戻らない`。
  検証: `tools/check-migrations.sh` rc=0、`tools/check-immutable.sh` rc=0、`CT source_acquisition (≥8)`（新しい `source_acquisition_tests.rs`。Scenario 1 本に試験 1 本以上。断られる側は DB の制約違反のエラーと、値が元のままであることの両方を見る）
- [ ] 1.2 登録簿の端末と PC の取得の間隔が収集側の定数と一致することを見る試験を置く（design D3 / D9）: Rust の定数（`collector-windows` の履歴の間隔）は `cargo test` で比べ、Kotlin の定数（位置 60 秒・アプリ利用 30 分・時計 1 時間）は値を grep して登録簿の値と比べる台本 `tools/check-source-intervals.sh`（DB の値は移行の SQL から読む。DB を要しない）。
  検証: `CT source_acquisition_matches_collector (≥1)`、`bash tools/check-source-intervals.sh` rc=0、定数の 1 つをわざと変えた木で rc≠0（手で 1 回確かめ、結果を PR の本文に書く）

## Task 2: 取り込み器の見回りの間隔を登録簿から読む（design D4）

- [ ] 2.1 `archive/config.rs` から `scan_sec` を消し、`ASHIATO_ARCHIVE_SCAN_SEC` があれば `bail!`（文言に「登録簿の s01-archive-inbox の fetch_interval_sec」）。`worker::spawn_inspecting` は `sleep` の直前に登録簿の値を読む（読めなければ前の値で寝て `warn`）。
  `archive_tests.rs` / `archive_flow_tests.rs` の `scan_sec` は、登録簿を `UPDATE` する補助に置き換える（`archive_flow_tests.rs:210` の 10 分の勘定も登録簿の値で）。`tools/smoke.sh` の `export ASHIATO_ARCHIVE_SCAN_SEC=1` を、S-01 の起動後に `s01-archive-inbox` の取得の間隔を 1 秒にする行へその場で書き換える。
  `docs/archive-inbox.md` の環境変数の表から `ASHIATO_ARCHIVE_SCAN_SEC` を外し、登録簿の列で変えることを書く。
  Scenario: `見回りの間隔の初期値は 120 秒` / `登録簿で見回りの間隔を変えると起動し直さずに効く` / `300 秒を超える見回りの間隔は断られる` / `見回りの間隔の環境変数が残っていると起動を断る`。
  検証: `CT archive_scan_interval (≥4)`（初期値の試験は `tokio::time::pause` と `advance` で 10 分を進め、走査の回数を数える）、`CT archive_ (≥1)` で ST12 / ST47 の既存の書庫の試験がすべて通る、`bash tools/check-shared-appends.sh` rc=0

## Task 3: 定期取得の枠（design D5 / D6）

- [ ] 3.1 `crates/server/src/scheduled.rs` に `Fetcher` と本番の一覧 `fetchers()`（空）と、取る手を差し込める `spawn_with` を置き、`lib.rs` の起動で取り込み器の隣に握る（`ASHIATO_ARCHIVE_USER_ID` が無ければ起こさない）。
  論理ソースごとに task を 1 つ、起動の直後に 1 回 → 終わってから登録簿の間隔を読み直して `tokio::time::sleep` → 繰り返す。失敗と取る手の無い行は `warn!`（`kind` と論理ソースの名前だけ）。
  Scenario: `起動の直後に 1 回取りに行く` / `取得の間隔ごとに取りに行く` / `壁時計が戻っても取りに行く回数は変わらない` / `止まっていた間に契機を過ぎても起動の直後に 1 回だけ取りに行く` /
  `前回の取得が終わる前に次の契機が来ても重ねない` / `1 つのソースの失敗はほかのソースを止めない` / `取る手の無い定期取得のソースは取りに行かない` / `定期取得のソースが 0 本でも起動して失敗を出さない`。
  検証: `CT scheduled_ (≥8)`（試験は登録簿に `acquisition = 'scheduled'` の試験用の行を足し、回数を数える取る手を差し込む。時間は `tokio::time::pause` / `advance`。
  「重ねない」は取る手の中で同時に走っている数の最大が 1 であることを見る。「取る手の無い」はログを集めて論理ソースの名前が出ることを見る）、
  `tools/check-scheduled-clock.sh`（新しい台本: `scheduled.rs` に `Utc::now` / `SystemTime` / `chrono::Local` / `Local::now` が 0 件）rc=0、わざと `Utc::now()` を 1 行足した木で rc≠0

## Task 4: 台帳の行と論理ソースごとの行を 1 トランザクションで書く（design D7。申し送り R25）

- [ ] 4.1 `archive/worker.rs` の読み手で、読み終えた後の台帳の行（`read` と、読めなかった項目を持つ行）と `archive_ledger_source` の行を 1 つの `sqlx::Transaction` で書く。`store_requests` は変えない。
  `testdb.rs` に、指定した `sha256` の `archive_ledger_source` への `INSERT` で例外を投げる試験用の trigger を作る補助を足す（本番のコードに分岐を置かない）。
  Scenario: `論理ソースごとの行を書けなかった書庫は読み終えた行も残らない` / `台帳を書けなかった書庫は次の走査で読み直されて最終日が出る`。
  検証: `CT archive_ledger_atomic (≥2)`（2 本目は trigger を外して次の走査を回し、最終日が出ることと、視聴履歴の件数が 1 回だけ読んだ DB と同じことを見る）、`CT archive_ (≥1)`

## Task 5: 置き場を別々に読み、専用のフォルダを作り、I/O の失敗を別の種別にする（design D8。申し送り R34 / M8）

- [ ] 5.1 `archive/scan.rs` の `scan_once_with_hasher` を、2 つの置き場を別々に読み、読めない置き場を返す形にし、`spawn_inspecting` は読めた置き場の候補を読み手へ送り、読めない置き場を生存信号の `blockers` に積む（成功回数の定義は変えない）。
  起動時に専用のフォルダが無ければ作る（ダウンロードのフォルダは作らない）。`archive/open.rs` の I/O の失敗を `file_unreadable` にし、台帳の理由の種別の CHECK に値があれば Task 1 の移行に足す。`web/src/messages.ts` に文言を足す。
  Scenario: `専用のフォルダが無くてもダウンロードのフォルダの書庫は読まれる` / `片方の置き場が読めない走査は成功に数えない` / `専用のフォルダが無ければ起動時に作られる` / `ダウンロードのフォルダは作られない` /
  `読み出せないファイルは形の対応と別の種別で台帳に残る`。
  検証: `CT archive_inbox_independent (≥5)`（読み出せないファイルは、見つけた後に権限を落とす（Unix の `chmod 000`）材料で作る）、`CT archive_ (≥1)`、`cd web && npx vitest run` rc=0

## Task 6: 読み終えた・中身の変わった書庫の失敗の回数を戻す（design D8。申し送り M7）

- [ ] 6.1 読み手が台帳に `read` を書けたら、その `path` の `archive_sighting` の `consecutive_failures` と `retry_after` を戻す。`scan.rs` は同じ `path` の `sha256` が前回と違えば同じ 2 列を戻してから候補にする。
  Scenario: `中断が続いた書庫を読み終えると回数が戻る` / `同じ場所に中身の違う書庫を置き直すと待たされない`。
  検証: `CT archive_failure_reset (≥2)`（中断は Task 4 の試験用の trigger か、ST12 の既存の失敗の差し込みで起こす）、`CT archive_ (≥1)`

## Task 7: Google 系の手作業の範囲を縦串で固定する（spec「Google 系の手作業は…」。design D13）

- [ ] 7.1 `archive_flow_tests.rs` の流れで 3 本の Scenario を試験にし、`tools/smoke.d/<時刻>_st13-archive-hands-off.sh`（新しいファイル）に、専用のフォルダの無い状態で S-01 を起動 → 作られたフォルダに `Timeline.json` を置く → ダウンロードのフォルダに合成の `takeout-*.zip` を落とす → 印を置いた形の 2 冊目を置く、を本物の S-01 で通す縦串を置く。
  `docs/archive-inbox.md` に 2 か月ごとの手作業の一覧（Takeout の書き出しとダウンロード・タイムラインの書き出しと運搬・年 1 回の予約のかけ直し・確認待ちが出たときだけ `tools/archive-shape.sh` で印）を書く。
  Scenario: `Takeout をダウンロードのフォルダに落とすだけで入る` / `同じ形の 2 回目の書庫は印を求めずに入る` / `専用のフォルダを作らずに端末のタイムラインが入る`。
  検証: `CT archive_hands_off (≥3)`、`scripts/quiet-run smoke -- tools/smoke.sh` rc=0（新しいファイルの `echo "== Scenario: …"` が 3 本出る）、`bash tools/check-shared-appends.sh` rc=0

## Task 8: ソースの一覧の読み出しの口 `GET /sources`（design D10 / D11）

- [ ] 8.1 `lib.rs` に `GET /sources`（ほかの読み出しと同じ認証）を足し、design D10 の応答を返す。通知までの残り日数と最後の書庫は `outage_store.rs` の最後の書庫の導き方を関数に切り出して共有する（2 つ目の計算を書かない）。
  `device_notice` は利用者の `c01-location` の最新の生存信号の原文の `unused_app_restrictions`。`src/bin/openapi.rs` に載せる。`latest_at` の問い合わせは `EXPLAIN` で索引が使われることを確かめ、使われなければ索引を足す移行を Task 1 の移行に入れる。
  Scenario（API の側。画面の側は Task 9）: `書庫のソースは手作業と通知までの残り日数が出る` / `通知の日を過ぎた書庫のソースはそのことが出る` / `退役したソースは通知までの残り日数を出さない` /
  `記録がまだ無いソースは「まだ無い」と出る` / `休止の対象から外れていないことが端末の節に出る` / `ソースの一覧は認証の無い要求には返らない`。
  検証: `CT sources_ (≥7)`（残り日数は最後の書庫を 53 日前・80 日前に置いた DB で `alert_days_left` が 22 / 負でなく過ぎた印、を見る。`outage_store` の途切れの判定と同じ日を返すことを 1 本で比べる。
  認証の無い要求は 401 で本文に一覧が無い）、`tools/check-openapi.sh` rc=0

## Task 9: S-7 設定の「ソース」節と e2e（design D10 / D11。spec「S-7 設定の「ソース」の節に…」）

- [ ] 9.1 `web/src/sources.ts`（型・取得・間隔の文言 D11）と `SettingsView.tsx` の「ソース」節（書き出しの節の前。取り方の見出し 5 つ・行・注記・定期取得の空の文・端末の節の休止の注記。入力とボタンを置かない）を作る。文言は `messages.ts`。
  `proto.html` の初期表示（Q3 の推奨の形）と同じ並び。1024 px は 4 列、360 px は 2 段。
  検証: `cd web && npx vitest run` rc=0（`web/src/__tests__/sources.test.ts`: D11 の間隔の文言 120 → 「2 分ごと」・60 → 「60 秒ごと」・86400 → 「24 時間ごと」と、取り方の並びの順）、`cd web && npx tsc --noEmit` rc=0
- [ ] 9.2 `tools/seed.d/<時刻>_st13-sources.sh`（新しいファイル）に、視聴履歴を運んだ最後の書庫が 53 日前・別の書庫のソースが 80 日前・記録の無いソース・退役した移行前のロケーション履歴・`c01-location` の生存信号の原文に `"unused_app_restrictions":"not_exempt"` を入れる偽データを置き、
  `web/e2e/settings-sources.spec.ts`（新しいファイル）で 11 本の Scenario を本物の Chromium で確かめる。
  Scenario: `全ソースが取り方ごとにまとまって並ぶ` / `間隔ごとのソースは取得の間隔が出る` / `書庫のソースは手作業と通知までの残り日数が出る` / `通知の日を過ぎた書庫のソースはそのことが出る` /
  `退役したソースは通知までの残り日数を出さない` / `記録がまだ無いソースは「まだ無い」と出る` / `定期取得するソースが無いことが文字で出る` / `休止の対象から外れていないことが端末の節に出る` /
  `ソースの一覧に変える操作は無い` / `360 px の幅でソースの一覧が横に溢れない` / `ソースの一覧は認証の無い要求には返らない`。
  アサート: 見出しの順と各見出しの下の行の数（登録簿の行数と一致し、重複が無い）/ 行の文字 / 節の中の `input` `select` `button` `textarea` の数が 0 / 360 px で `document.documentElement.scrollWidth <= 360` と節の `scrollWidth <= clientWidth` / 認証の無い `request.get('/api/sources')` が 401。
  検証: `cd web && npm run test:e2e -- settings-sources` rc=0（11 本が passed）、`bash tools/check-shared-appends.sh` rc=0

## Task 10: 携帯の収集アプリが休止の対象から外れるよう頼む（design D12。spec `device-collection`）

- [ ] 10.1 `collector-android/app/src/main/kotlin/dev/ashiato/collector/UnusedAppRestrictions.kt`（新しいファイル）に、`PackageManagerCompat.getUnusedAppRestrictionsStatus` の結果を 4 つの状態に写す関数と、自動で頼んだかの印（`SharedPreferences`）を置く。
  `MainActivity.kt` の初回の流れ（ST09 の権限の要求の後）で、`not_exempt` かつ印が無ければ説明のダイアログ → 応じたら `IntentCompat.createManageUnusedAppRestrictionsIntent` を開き、印を立てる。`not_exempt` の間は画面に 1 行と「設定を開く」のボタン（24 dp 以上）。
  `Heartbeat.kt` の `c01-location` の原文に `unused_app_restrictions` を載せる（`capturable` と `blockers` は変えない）。
  Scenario: `対象から外れていない端末で初めて開くと設定の画面へ案内される` / `対象から外れている端末では案内されない` / `2 回目に開いたときは設定の画面を自動では開かず、画面に出し続ける` /
  `仕組みを持たない端末では頼まない` / `位置の生存信号の原文に除外の状態が載る` / `外れていなくても取得できる状態は変わらない`。
  検証: `GT '*UnusedAppRestrictions*' (≥6)`（Robolectric。状態の読み口は差し替えられる形にし、4 つの状態ごとに案内の有無・開いた `Intent` の action・印・画面の文字を見る。生存信号は原文の JSON を解いて欄の値を見る）、
  `GT '*Heartbeat*' (≥1)` で既存の生存信号の試験が通る

## 人間の確認待ち

Android の休止は「数か月操作されない」ことが条件で、エミュレータで早送りできない（`adb shell cmd app_hibernation` は休止を手で入れるだけ。design D12）。

- Scenario: 数か月操作しなくても権限が剥がれない
  > 物理: time —— 本人の端末で収集アプリを一時停止の対象から外し、3 か月後の確認バッチで、位置の権限が残っていて位置の記録が途切れていないことを見る
