# ST06 実装タスク — 携帯端末のアプリ利用を集める

読む順: `deep.md`（**最優先。本人が決めた 8 件と、聞かずに決めた既定 C1〜C12**）→ このファイル →
`specs/device-collection/spec.md` → `design.md` → `docs/stories/ST06.md` →
`docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D3）。**名前は作成時刻 `YYYYMMDDHHMM_app_usage_rollup_source.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。**サーバの取り込みのコードは変えない。**
**`collection-coverage` と `record-envelope` には触らない**（ST12 が走行中）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` と `tools/seed.sh` が前提。
Android の計測テストは `tools/android-emulator.sh`（2 段実行。`@NeedsPristinePermissions` の前提はテストの外で作る）。

## Global Constraints（規律。**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Kotlin と Rust はコメント（`// Scenario: 0 件のときは窓が進む`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする
- 印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **印を置くテストは、その Scenario の THEN を確かめるものにする**。THEN が「D-01 に格納されている」なら
  取り込み口まで通す検査（`tools/smoke.sh` の psql）に置く
- **この change の delta は 58 本**（`device-collection`。ADDED 34 本 + MODIFIED 24 本）。
  **新しく足した Scenario は 36 本**で、残る 22 本は MODIFIED で写した既存分（印は ST01 / ST04 のテストに既にある）
- **「人間の確認待ち」に逃がせる Scenario は 1 本も無い。** ST06 は面を持たず、
  取得元は偽物に差し替えられる（`FixSource` と同じ形）。物理が要るものが出たら
  `> 物理: <lock|battery|gps|time|realdata|device>` を書けるか確かめ、書けなければ機械で再現する
- **契約の形を変えない**（`/ingest` `/heartbeat` `/drops`）。`cargo test payload_shape_is_pinned` は最初から最後まで緑のまま
- **D3 / D6 / D7 は（仮）決め。** 反転条件は `design.md` にある。反転条件のうち spec を破る手は本人に返す
- **ログに位置の値・アプリの表示名・原文を出さない。** 出すのは件数・ソース名・所要時間・エラーの種別だけ
- 同じ名前のテストで絞る検証（`gradlew test --tests '*AppUsage*'` など）は、
  **1 本以上走ったことも見る**（0 本でも rc=0 になるため）

## Task 1: 足場（ソースを 2 本持てる形にする）

- [ ] 1.1 `collector-android` に `CollectionSource` の口を置く（`logicalSource` / `intervalMs` /
  `capability(context)` / `collect(window): Result`）。位置を `LocationSourceAdapter` としてその口に載せ替え、
  **振る舞いは 1 つも変えない**（design D5）。
  検証: `./gradlew :app:testDebugUnitTest` rc=0（既存の単体が全部緑のまま）/
  `git diff --numstat -- collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationFix.kt` が 0 行
- [ ] 1.2 `Telemetry` のソース名の焼き込み（`LOGICAL_SOURCE` 定数）を外し、**書き手が名乗る**形にする（独立レビュー R10）。
  Scenario: `端末のログのソース名がそのソースを指す`。
  検証: `./gradlew :app:testDebugUnitTest --tests '*TelemetryTest*'` rc=0 /
  `grep -c LOGICAL_SOURCE collector-android/app/src/main/kotlin/dev/ashiato/collector/Telemetry.kt` が **0**
  （**いまは 1。実測 2026-09-18** —— `Telemetry.kt:19` の `append(" source=").append(LOGICAL_SOURCE)`。
  spec レビュー R5: 以前の検証パターンは作業前から 0 件で通っていた）
- [ ] 1.3 `AttemptCounters` の満点の刻みをソースごとに取り、数えの置き場を**ソースごとの別ファイル**にする
  （いまは `heartbeat-counters.txt` の 1 本。R10）。既存の 1 本は位置の名前へ移す（読めなければ新品から始める）。
  Scenario: `取得率はソースごとの刻みで数えられる` / `どのソースも生存信号の区間に取得契機が 1 回以上入る` /
  `集計の取得率は 6 時間を刻みとして数えられる`（数えの側。取り込みの側は 4.3）。
  **`successes <= attempts` を型か `require` で壊せなくする**（契約が `invalid_counts` で恒久的に断る。spec レビュー R4）。
  検証: `./gradlew :app:testDebugUnitTest --tests '*HeartbeatCountersTest*'` rc=0 /
  2 ソースを同時に数える試験が 1 本以上ある / 全ソースの（生存信号の区間 ≥ 取得契機の間隔）を総当たりで見る試験が 1 本ある
- [ ] 1.4 `AndroidManifest.xml` に `PACKAGE_USAGE_STATS` の宣言を足し、前景サービスの種別を
  位置が取れない状態でも立てられる形にする（design D5 のリスク）。
  検証: 計測テストで **位置の権限を拒否した状態で前景サービスが立つ**ことを確かめる
  （`./gradlew :app:connectedDebugAndroidTest --tests '*PermissionDeniedInstrumentedTest*'` rc=0）。
  立てられないと分かったら design D5 の落とし所へ倒し、**その事実を `design.md` に追記**する

## Task 2: 取得元の口と偽物

- [ ] 2.1 `UsageSource` の口を置く（`events(begin, end): EventsResult`（`Unreadable` / `Events(list)`）/
  `rollups(granularity, begin, end)` / `retentionFloor(now)`）。本番は `UsageStatsManager`、
  試験は偽物（design D5 / リスクの「`null` と 0 件の取り違え」）。
  検証: `./gradlew :app:testDebugUnitTest --tests '*UsageSourceTest*'` rc=0 /
  `Unreadable` を返す偽物と 0 件を返す偽物の**両方**が試験にある
- [ ] 2.2 **見込みの**保持の下限（イベント 10 日 / 年 2 年 / 月 6 か月 / 週 4 週）を 1 か所（`UsageRetention`）に置く。
  出所を逐語でコメントに残す。**この値で問い合わせの窓を切り詰めない**（spec レビュー R3。API から読めない見込みなので、
  切り詰めると残っているイベントを飛ばす）—— 使うのは「gap を積むかどうか」の判定だけ。
  検証: `./gradlew :app:testDebugUnitTest --tests '*UsageRetentionTest*'` rc=0 /
  **見込みより古いイベントを返す偽の取得元でも 1 件も落ちない**試験が 1 本ある
  （Scenario `見込みより古いイベントが返ったときは gap が積まれない` が 4.1 で拾う）

## Task 3: イベントの取得と窓

- [ ] 3.1 30 分ごとにイベントを取り、**1 イベント 1 記録**で積む。種別も欄もふるいにかけない（design D1）。
  `raw` は取得元が返した値だけ、表示名は `payload` にだけ（design D2）。
  Scenario: `契機ごとに前回以降のイベントが記録になる` / `種別でふるい落とさない` /
  `1 件が取得元の公開しているすべての欄を持つ` / `表示名は解析済みにだけ入る` / `同じイベントの原文は毎回同じ文字列になる`。
  検証: `./gradlew :app:testDebugUnitTest --tests '*AppUsageCollectorTest*'` rc=0 /
  原文を 2 回組み立てて**バイト列が一致する**試験が 1 本ある
- [ ] 3.2 窓の終わりを端末に保存し、次の契機はその手前から取り直す。`Unreadable` では窓を進めない。
  Scenario: `境界のイベントが落ちない` / `読めなかったときは窓が進まない` / `0 件のときは窓が進む` /
  `窓の終わりは収集の停止と再開をまたいで残る` / `遡って取った記録の地域は取得時点の端末の地域である`（design D7（仮））。
  検証: `./gradlew :app:testDebugUnitTest --tests '*UsageWindowTest*'` rc=0 /
  **ガードをわざと壊す**（`Unreadable` でも窓を進める）と `読めなかったときは窓が進まない` が落ちることを確かめる
- [ ] 3.3 端末の時計の前進が単調な経過時間と食い違う間は、窓を進めず取り直さない（design D5 / R4）。
  `AgeClock`（ST04）の単調な経過時間を使い、**新しい時計を足さない**。
  Scenario: `時計が飛んでいる間は取り直さない`。
  検証: `./gradlew :app:testDebugUnitTest --tests '*UsageWindowClockTest*'` rc=0
- [ ] 3.4 `payload` の形（欄の並びと省略の規則）を 1 文字単位で固定する試験を置く
  （C-02 の `payload_shape_is_pinned` と同じ形。design D2）。
  検証: `./gradlew :app:testDebugUnitTest --tests '*AppUsagePayloadShapeTest*'` rc=0

## Task 4: 取りこぼしと集計

- [ ] 4.1 窓の始まりが**見込みの**下限より前なら、`[窓の始まり, min(見込みの下限, 返った最古のイベントの時刻))` を
  種別 `gap` の記録 1 件として積む（長さが 0 なら積まない）。**窓は切り詰めない**（design D4）。
  Scenario: `見込みの下限より前から取ろうとすると gap が積まれる` / `見込みの下限の内側だけを取ったときは gap が積まれない` /
  `見込みより古いイベントが返ったときは gap が積まれない` / `gap の記録にアプリの名前が入らない` /
  `gap の記録の出来事の時刻は期間の終わりである`。
  **出来事の時刻は期間の終わり**（spec レビュー R2）—— 始まりに置くと収集開始日より前へ落ちうる。
  検証: `./gradlew :app:testDebugUnitTest --tests '*UsageGapTest*'` rc=0 /
  ガードをわざと壊す（出来事の時刻を期間の始まりにする）と対応する試験が落ちる
- [ ] 4.2 移行 `YYYYMMDDHHMM_app_usage_rollup_source.sql`（と `.down.sql`）で `core.source` に
  `c01-app-usage-rollup` を足す。**`expected_gap_sec = 21600` と `external_id_kind = 'none'` を明示する**
  （既定の `'record'` のままだと端末の未送信が永久に詰まる。独立レビュー R7）。`MIGRATIONS` の末尾に足す。
  テスト: (a) 全移行を当てると行がある / (b) `external_id_kind` が `'none'` /
  (c) **識別子なしの `c01-app-usage-rollup` の要求が受理される** / (d) 当て直しても値が変わらない。
  検証: `cargo test -p ashiato-server app_usage_rollup_source` rc=0 / `tools/check-migrations.sh` rc=0
- [ ] 4.3 収集を始めた時点で、年・月・週・日の 4 粒度の集計を取り込む。途中で終わったら次の契機で続きから。
  **以後は 6 時間ごとに日ごとの粒度だけを取り込む**（生存信号の区間と同じ刻み。spec レビュー R4 ——
  24 時間にすると 6 時間の区間に契機が入らず、試行 0 / 成功 1 の信号が契約に恒久的に断られて端末に居座る）。
  Scenario: `初回に 4 つの粒度が取り込まれる` / `集計はイベントとは別のソースに積まれる` /
  `集計の 1 件が粒度と期間と合計時間を持つ` / `取り込みが途中で終わっても次の契機で続きから入る` /
  `初回の後も日ごとの集計が 6 時間ごとに取り込まれる` / `集計の取得率は 6 時間を刻みとして数えられる`。
  検証: `./gradlew :app:testDebugUnitTest --tests '*RollupImportTest*'` rc=0
- [ ] 4.4 集計の原文を 2 回組み立てて**バイト列が一致する**ことを固定する（design のリスク「集計の重複」）。
  Scenario: `同じ集計の原文は毎回同じ文字列になる`。
  検証: `./gradlew :app:testDebugUnitTest --tests '*RollupPayloadShapeTest*'` rc=0

## Task 5: ソースごとの独立と権限

- [ ] 5.1 収集の開始を取得条件に依らず行い、取得条件が欠けたソースは `collect()` を呼ばずに
  生存信号だけ出す（design D5）。`MainActivity` の「位置が無ければ終了」をやめる。
  Scenario: `位置の取得条件が欠けてもアプリ利用は集まる` / `アプリ利用の取得条件が欠けても位置は集まる` /
  `どのソースも取得できなくても収集は始まり信号は届く`。
  検証: `./gradlew :app:testDebugUnitTest --tests '*SourceIndependenceTest*'` rc=0 /
  計測テスト `./gradlew :app:connectedDebugAndroidTest --tests '*PermissionDeniedInstrumentedTest*'` rc=0
  （**いまのテストの期待が変わる** —— 「前景サービスの通知が出ない」は「出る」へ。
  `docs/handoff/ST11.md` の **4 点のうち 2 と、「文面の叩き台」**を書き直す。
  点 4「権限は拒否のまま」は変わらない。叩き台の「収集は始まらず」は ST06 の
  `収集の開始を、ソースの取得条件が満たされているかに依らず行う` と**正面から矛盾する**ので、
  ST11 がそのまま採ると正典の中で 2 つの Requirement が食い違う。spec レビュー R9。ST06 の上流で叩き台は書き直してある）
- [ ] 5.2 アプリ利用の取得可否（`PACKAGE_USAGE_STATS` の付与状態）を `AndroidCapability` と同じ形で読み、
  `blockers` は `permission` と `network` の 2 つにする（既定 C9）。**読めなかったら「取れない」に倒す**。
  Scenario: `位置の取得条件が欠けても位置の生存信号は届く`。
  検証: `./gradlew :app:testDebugUnitTest --tests '*AndroidCapabilityTest*'` rc=0
- [ ] 5.3 初回起動で設定画面へ送り、許可されなくても収集を始める。自動で送るのは 1 度だけで、
  以後は常駐の通知からたどれるようにする（design D6（仮））。
  Scenario: `特別なアクセスが無ければ初回起動で設定画面へ送られる` / `許可しなくても収集は始まる` /
  `2 度目の起動では自動で送られない` / `後から許可すると次の契機から集まる`。
  検証: 計測テスト `./gradlew :app:connectedDebugAndroidTest --tests '*UsageAccessInstrumentedTest*'` rc=0
  （設定画面へ遷移したことは UI Automator で確かめる。`appops set <pkg> GET_USAGE_STATS` で前提を作り、
  **前提はテストの外**で作る＝`@NeedsPristinePermissions` と同じ 2 段実行）
- [ ] 5.4 常駐の通知の文言を、位置だけの決め打ちからソースの数に合わせた形へ直す（R10）。
  ST04 の「未送信の日数」の表示は**壊さない**。
  検証: `./gradlew :app:testDebugUnitTest --tests '*RetentionNotifierTest*'` rc=0

## Task 6: ST04 の置き場を 2 ソースで正しく使う

- [ ] 6.1 アプリ利用と集計の記録を、位置と**同じ置き場**（`SegmentStore`）に積む（既定 C11）。
  保持の上限は全ソースを通して古い順のまま。
  検証: `./gradlew :app:testDebugUnitTest --tests '*RetentionTest*'` rc=0 /
  2 ソースを混ぜた置き場で古い順に捨てる試験が 1 本以上ある
- [ ] 6.2 `Retention` が破棄の報告の範囲の終わりに使う「残った最も古い記録」を、
  **同じソースの中から**取るよう直す（独立レビュー R11。いまは置き場全体の先頭）。
  Scenario: `破棄の範囲は別のソースの記録で閉じない`（MODIFIED で新設。spec レビュー R7 ——
  既存の Scenario はソースを 1 本しか登場させないので、直す前も後も同じく緑だった）。
  検証: `./gradlew :app:testDebugUnitTest --tests '*RetentionTest*'` rc=0 /
  ガードをわざと戻す（置き場全体の先頭を使う）とその試験が落ちる

## Task 7: 実測と結合

- [ ] 7.1 `tools/usage-volume.sh` を作る —— エミュレータで**イベントを N 件流し込んでから 1 時間ぶん**を取得し、
  1 件あたりのバイト数 × 実測の 1 日あたり件数から **90 日ぶんの未送信のバイト数**を標準出力に出し、
  **2 GB の上限の半分（1 GB）を超えたら rc=1** で落とす（design のリスク「端末の置き場を位置と食い合う」）。
  出た数を `deep.md` の「確かめたが問わなかったこと」に書き戻す。
  **rc=1 になったら実装を止めて本人に返す**（次の深掘りの問い。種別を絞るかどうかは Q1 の再問）。
  検証: `tools/usage-volume.sh` rc=0 / 出力の数が `deep.md` に写っている
  （**24 時間の実測はしない** —— 「時間そのもの」を待つ形にすると下流の 1 セッションで終わらない。spec レビュー R8）
- [ ] 7.2 遅延の上限を**単体で**止める —— `USAGE_INTERVAL_MS == 1_800_000` と
  `USAGE_INTERVAL_MS + SEND_INTERVAL_MS < 3_600_000` を assert する（`IntervalTest.kt` と同じ形）。
  Scenario: `アプリ利用は出来事の時刻から数えても 1 時間以内に届く`。
  **smoke には置かない**（spec レビュー R6 ——`tools/smoke.sh` の台本は自分で `event_time` に「いま」を入れて
  即 POST するので、**どんな実装でも緑になる**。`IntervalTest.kt:40-50` が同じ過ちを既に記録している）。
  検証: `./gradlew :app:testDebugUnitTest --tests '*IntervalTest*'` rc=0
- [ ] 7.2b `tools/smoke.sh` に、アプリ利用と集計の 1 件が取り込み口を通って `core.event` に入ることと、
  **同じ 1 件を 2 回送っても行が増えない**ことを足す。
  `tools/smoke.sh` の `registered_at` を揃える対象に **`c01-app-usage-rollup` を足す**
  （足さないと集計の 1 件が「登録より前」になる。spec レビュー R6）。
  検証: `tools/smoke.sh` rc=0
- [ ] 7.3 `docs/collector-contract.md` に `c01-app-usage` と `c01-app-usage-rollup` の `payload` の形を、
  C-02 と同じ粒度（欄の表・省略の規則・並び）で足す。
  検証: 3.4 / 4.4 の固定試験の期待値を**この表から読む**形にし、表を 1 行変えると試験が落ちることを確かめる
  （`check_scenarios.py` は `docs/` を読まないので、契約に表を足したことの検査にならない。spec レビュー R8）

## Task 8: 仕上げ

- [ ] 8.1 `python3 scripts/check_scenarios.py .` で、この change の全 Scenario に印があることを確かめる。
  検証: rc=0（「人間の確認待ち」は 0 本）
- [ ] 8.2 `./gradlew :app:testDebugUnitTest` / `./gradlew :app:connectedDebugAndroidTest` /
  `cargo test --workspace` / `cargo clippy --all-targets -- -D warnings` / `tools/smoke.sh` が全部 rc=0
- [ ] 8.3 `docs/handoff/` を PR の前にもう 1 度読む（`ST11.md` は 5.1 で書き直したもの、
  `ST14.md` は ST06 が申し送った 3 件）。
  検証: `python3 scripts/review_triage.py . st06-app-usage` rc=0
