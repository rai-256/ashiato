# 独立レビュー（code） — st06-app-usage

Task ごとの独立レビュー（SDD の task reviewer）と、全 Task 完了後の whole-branch review
（`code-reviewer.md` ＋ `code-verify`）の指摘を、出た順に R 番号で積む。
**1 件も黙って消さない** —— `scripts/review_triage.py` が処置の無い指摘を止める。

---

## R1. 集計の取り込みが `c01-app-usage-rollup` の収集開始日を 1〜2 年前へ下げ、API では戻せない

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageRollupRecord.kt`
  （`eventTime = lastAt`）/ `migrations/202609240758_app_usage_rollup_source.sql`
- 根拠:
  - `crates/server/src/coverage.rs:255` —— `Arrival::Record` の gate は逐語で `"true".to_string()`
    （**記録には閾値が無い**）。SQL は `collection_started_on IS NULL OR collection_started_on > <その記録の日>`
    のときだけ動くので、**前にしか進まない**。
  - 初回の取り込みは年の粒度から入り、年ごとの箱は 2 年ぶん残る（`deep.md` の保持の表）。
    最も古い箱の終わり ＝ その記録の `event_time` は**導入の 1〜2 年前**になる。
  - ただし `crates/server/src/coverage.rs:28-53` —— `must_sources()` は
    `DEVICE_SUBJECT`（`c01-location` / `c01-app-usage`）と `USAGE_SUBJECT`（`c01-photo` /
    `c02-window` / `c02-browser-history`）の **5 本しか返さない**。
    **`c01-app-usage-rollup` はいまの稼働状況の格子に 1 行も出ない。**
    レビュアーが描いた「導入前の 1〜2 年を②/⑥で塗る」は、**このソースが格子に載った後に初めて起きる**。
- kind: technical
- 処置: followup ST14 —— **ST06 の側で閉じる手が無い**。
  (a) `event_time` を取得時点へ倒すと、**凍結される `event_time` が箱の期間と食い違ったまま全行に残る**
  （直せる列 1 つを、直せない行の山と取り替えることになる）。
  (b) 受け手の判定（`Arrival::Record` の閾値）を変えるのは `collection-coverage` の領分で、
  **ST12 が走行中なので触れない**（Global Constraints に明記）。
  よって現状を受け入れ、**ST14 がこのソースを格子に載せる前に開始日を決め直せる**ように
  `docs/handoff/ST14.md` の R7 へ「いまの値と、API では戻らないこと」を書き足した。
  **失われるものは無い** —— 記録も `event_time` も正しいまま入り、
  `core.source.collection_started_on` は SQL 1 文で直せる（API が戻さないだけ）。

## R2. spec の Requirement 本文が実装と食い違ったまま正典になる

- 成果物: `openspec/changes/st06-app-usage/specs/device-collection/spec.md`
- 根拠: 本文は逐語で「IF 取得の**窓の始まり**が …… **窓の始まりから** …… の期間を」と書くが、
  実装は保存された窓の終わり（`mark.end`）を使い、`mark == null` では生成しない。
  Scenario の AND は逐語で「**保存された窓の終わり**から見込みの下限まで」と書いており、
  **本文と Scenario が spec の中で食い違っていた**。`design.md` の（仮）は spec を上書きしない。
- kind: technical
- 処置: fixed specs/device-collection/spec.md —— 本文を「前回の取得で保存された窓の終わり」に直し、
  「1 度も取得できていなければ生成しない」を 1 行足した。根拠（重ね幅の 1 分は前回の契機で取れている /
  期間の終わりが収集の動いていた窓に入るのは収集が既に動いていた場合だけ）を本文の下に書き、
  `design.md` の D9 を指した。

## R3. 初回の後の日ごとの取り込みが、保持されている全日ぶんを 6 時間ごとに積み直す

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageRollupSourceAdapter.kt`
- 根拠: `ROLLUP_QUERY_BEGIN = Instant.EPOCH` のまま `[EPOCH, now)` で日の粒度を引くので、
  保持されている約 10 日ぶん × アプリ数の箱が毎回返る。6 時間ごとなので同じ日の箱が寿命の間に約 40 回積まれる。
  `design.md` の D3 は逐語で「日ごとの集計を **4 回**読み直すことになる」と見積もっており、**桁が 1 つ違う**。
  サーバは内容の鍵で畳むので正しさは壊れないが、端末の未送信は位置と同じ置き場（C11）なので
  design の Risk「端末の置き場を位置と食い合う」に直撃し、Task 7.1 の実測にそのまま乗る。
- kind: technical
- 処置: fixed 4.3

## R4. 集計の問い合わせの窓を固定するテストが無い

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/RollupImportTest.kt`
- 根拠: `inner.rollupQueries.map { it.first }`（粒度）しか見ておらず、窓（`it.second`）を 1 度も見ていない。
  「窓を切り詰めない」は C3 / spec レビュー R3 の明示の決定で、イベント側は `FakeUsageSource.eventQueries` で見ている。
- kind: technical
- 処置: fixed 4.3

## R5. `usage_gap` のログが `elapsed_ms` を期間の長さに流用している

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageSourceAdapter.kt`
- 根拠: `Telemetry.line` の `elapsedMs` は所要時間の欄で、他の `kind` はそう使っている。
  3 日の gap で `elapsed_ms=259200000` が出るので、ログから所要時間を集計すると壊れる。
- kind: technical
- 処置: fixed 4.1

## R6. 集計の記録の封筒（出来事の時刻・地域）を固定するテストが無い

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/RollupPayloadShapeTest.kt`
- 根拠: `raw` と `payload` の文字列しか見ていない。「出来事の時刻は箱の終わり」「地域は取得時点」（C12）は
  KDoc にしかなく、gap 側には `gap の記録の出来事の時刻は期間の終わりである` があるのに対称になっていない。
  R1 を将来直すなら、ここが回帰の受け皿になる。
- kind: technical
- 処置: fixed 4.4

## R7. `UsageRollupProgressStore.save` の `.tmp` が rename 失敗時に残る

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageRollupSourceAdapter.kt`
- 根拠: `throw IOException("rename")` が自分の `catch` に入って log だけして戻るので、
  `usage-rollup-progress-*.txt.tmp` が置き場に残る。次回の書き込みで上書きされるので溜まりはしない。
- kind: technical
- 処置: fixed 4.3

## R8. 積んだ原文の指紋を「未送信に書けたか」より先に台帳へ入れている

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageRollupSourceAdapter.kt`
- 根拠: `Outbox.kt:57` —— `unwritten.size > MAX_UNWRITTEN`（10,000）を超えると古いほうから
  `failures.lost(...)` で手放す。置き場は位置と共用（本人の決定 C11）なので、空きが尽きた端末では超える。
  手放された**確定済みの**箱は原文が変わらないので指紋も変わらず、次の契機で `skipped` になって二度と積まれない。
  R3 の直し（台帳）が足した経路で、**「積み直しは安全・積まないのは取りこぼし」の線の逆へ倒れていた**。
- kind: technical
- 処置: fixed 4.3

## R9. 台帳の有界化の要の分岐に試験が 1 本も無い

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/RollupImportTest.kt`
- 根拠: `ROLLUP_SEEN_MAX`（10,000 件）で溢れたときに**新しい箱を残す**分岐は、
  `sortedByDescending` を `sortedBy` に入れ替えても全 290 本が緑のまま通った
  （新しい箱＝まだ取得元に残っている箱を先に忘れる ＝ 毎契機積み直す、で R3 が再発する）。
  `pruneRollupSeen(seen, at)` を `seen` に書き換えても全緑だった（`collect` から呼ばれていることが未固定）。
  **実装者の報告にあった「試験で固定してある」は事実と違っていた**（報告も訂正させた）。
- kind: technical
- 処置: fixed 4.3

## R10. 取り込み済みの印を「読めた」粒度に付けている

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageRollupSourceAdapter.kt`
- 根拠: spec の Scenario は逐語で「年と月の粒度まで**取り込んだ**ところで収集が止まり」であり、
  `UsageStatsManager` から読めたことは取り込んだことではない（取り込みは未送信に積むところまで）。
  置き場に 1 件も書けなかった契機でも年・月・週の印が付き、二度と読まれない。
  **年は 2 年ぶんで取得元からも消えていく**（`loss: uncaptured` の経路）。
  実装者が R8 の直しの後に自分で見つけて返した。
- kind: technical
- 処置: fixed 4.3

## R11. 窓の終わりを「未送信に書けたか」を見ずに進めている

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageSourceAdapter.kt`
- 根拠: R8 / R10 と同じ型が Task 3 の成果物に残っていた。置き場が満杯の端末で `lost` になったイベントは
  二度と取りに行かず、取得元の保持 10 日で消える。**gap の記録は積み直す経路が無い**
  （窓が進むと `RetentionFloor.excludes(...)` が偽になる）。
  同じ型は ST04 が 1 度踏んで直してある（`AttemptCounters` / `HeartbeatEmitter` の `takeAfter`。
  ST04 の `review/code.md` R24）。`AgeClock` は同型ではない（書けなければ経過を少なく見積もる ＝ 捨てない側）。
- kind: technical
- 処置: fixed 4.1

## R12. 窓を据え置いている間、gap の記録だけはサーバで畳まれない

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageSourceAdapter.kt` /
  `UsageRetention.kt`
- 根拠: gap の範囲の終わりは `min(見込みの下限, 返った最古のイベント)` で、下限は**取得の時点**から数える。
  窓が据え置かれている間、始まりは固定でも終わりが 30 分ごとにずれるので、
  **始まりが同じ・終わりだけ違う原文**が積まれ、内容の鍵が毎回変わって畳まれない。
  イベントの記録は原文が同じなので畳まれる（＝ R8〜R11 の「積み直しは冪等」が効く）。
  この経路に入るのは置き場が 10 日以上満杯の端末だけで、1 契機あたり 1 件。
- kind: daily
- 処置: fixed D10 仮 —— 取りこぼしではなく重複側なので、引いた線からは正しい倒し方。
  反転条件（tasks 7.1 の実測でこの重複が見えたら、gap の範囲の終わりを
  「窓を最後に進めた時点の下限」に固定する側へ倒す。spec は終わりを「見込みの下限」としか書いていないので破らない）を
  `design.md` の D10 に書いた。

## R13. 位置を拒否した端末では通知の権限を 1 度も求めないので、常駐の通知が出ない

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt`
- 根拠: `proceed()` の組み替えで `niceToHave()`（背景の位置 ＋ `POST_NOTIFICATIONS`）が
  `granted(fine)` が真の枝の中に入っていた。背景の位置が「前景の後でしか求められない」のは事実だが、
  **通知の権限にその依存は無い**。帰結は 3 つ —— Android 13 以降で位置を拒んだ端末は前景サービスが立つが
  通知が見えず、**design D6 の「以後は常駐の通知から同じ設定画面へたどれる」がその本人にだけ効かない**
  （自動送出は 1 度きりなので、利用状況へのアクセスへ戻る道が消える）、ST04 の 83 日の知らせも
  `retention_alert_blocked` のまま出ない。
  **実装者はこの症状を `PermissionDeniedInstrumentedTest` で実際に踏みながら、テストの観測のほうを
  `logcat` の `kind=foreground_special_use` に替えて回避していた** —— 本物の振る舞いを見るのをやめた結果、
  欠陥が緑のまま残った。**この repo がこのハーネスを作った理由そのものの型**（`.claude/skills/story`）。
- kind: technical
- 処置: fixed 5.1 —— 通知の権限は位置の可否に依らず求め、前提を持つのは背景の位置だけにした。
  `PermissionDeniedInstrumentedTest` の観測も通知へ戻させた（ダイアログが出なければその手前で落ちる ＝ 再発検知）。

## R14. 申し送りの「テストが実際に見ている点」が実測と食い違ったまま残る

- 成果物: `docs/handoff/ST11.md`
- 根拠: `tasks.md` 5.1 が名指しで書き直しを求めた箇所。「★ 前景サービスの通知は出る」が
  「**テストが実際に見ている 4 点**」という見出しの下にあるのに、R13 の回避で当のテストは通知を見るのをやめていた。
  **ST11 はここを材料に正典の Scenario を書く**ので、事実でない点が持っていかれる。
- kind: technical
- 処置: fixed 5.1 —— 点を 1 つ足して 5 点にし（「位置を拒否した後に通知の権限のダイアログが出る」）、
  点 2「通知は出る」がその点に依存することも書いた（ST11 が点 2 だけを材料にすると同じ穴が開く）。

## R15. Scenario の印を置いたテストが、その Scenario の THEN を確かめていない

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/MainActivityTest.kt` /
  `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/UsageAccessInstrumentedTest.kt`
- 根拠: `許可しなくても収集は始まる` の THEN は「収集は始まり、**位置の記録が生成される**」＋
  AND「アプリ利用の生存信号は取得できない状態と、何が満たされていないか（権限）を示す」。
  印を置いた単体は `startedService()` が `LocationService` であることしか見ておらず、
  計測は通知と未許可しか見ていない。Global Constraints の
  「**印を置くテストは、その Scenario の THEN を確かめるものにする**」に反する（plan-mandated）。
  振る舞い自体は `SourceIndependenceTest` が THEN と AND の両方を assert していた。
- kind: technical
- 処置: fixed 5.1 —— 印を THEN を確かめているテストへ移し、見ていない 2 本から外した。
  残り 7 本の印も同じ目で見直させた。

## R16. ソース名を読めない破棄が位置を名乗る

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt`
  （`UNATTRIBUTED_SOURCE`）
- 根拠: 退避先の**ファイルごと**読めない場合、中のソースが分からない。契約は登録簿にある名前しか
  受け付けない（受け付けない名前は `unknown_source` で断られ、**報告が端末に居座る**）ので、
  どれか 1 つを名乗るしかない。報告を落とすのは扉 #14 の証拠を捨てることになり、
  登録簿に「不明」を足すのは `collection-coverage` / `record-envelope` の変更で ST12 が走行中なので触れない。
  当たる範囲は「退避行のうち先頭 512 バイトに `logical_source` が残っていないもの」だけで、
  件数は `kind=unreadable_source_unknown` で測れる。
- kind: daily
- 処置: fixed D11 仮 —— 反転条件（`unreadable_source_unknown` が実測で無視できない件数になったら、
  ST12 の archive 後に登録簿へ「不明」のソースを足す）を `design.md` の D11 に書いた。

## R17. 本人が選んだ Q9=c の自動再開が未実装

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageSourceAdapter.kt`
- 根拠: `deep.md` の本人回答 Q9=c は、時計の食い違いが解消しないまま取得元の保持（10 日）を超えたら、見込みの下限から取得を再開し、保存された窓の終わりから下限までを gap に残す、と決めている。現状の `AppUsageSourceAdapter.kt:85` は期間によらず `Unavailable` を返し続けるため、再開も gap 生成も起きない。
- kind: technical
- 処置: fixed 3.3

## R18. API 35 の `getExtras()` を API 31〜34 でも呼ぶ

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/UsageSource.kt`
- 根拠: `UsageEvents.Event.getExtras()` は API 35 の API だが、`UsageSource.kt:219` のガードは `VERSION_CODES.S`。minSdk 30 のため API 31〜34 で `USER_INTERACTION` を含む問い合わせを処理すると `NoSuchMethodError` になり、窓全体の収集が失敗する。
- kind: technical
- 処置: fixed 2.1

## R19. 任意の第三者アプリの表示名を取得するための package visibility 宣言が無い

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppLabels.kt` / `collector-android/app/src/main/AndroidManifest.xml`
- 根拠: `AppLabels.kt:38` は `getApplicationInfo()` で表示名を読むが、manifest に `<queries>` / `QUERY_ALL_PACKAGES` が無い。Android 11 以降、可視でない第三者アプリは `NameNotFoundException` となり、UsageStats が返したイベントでも表示名が省略される。D2 の「削除後にも何のアプリだったか残す」を通常の利用アプリで満たせない。
- kind: technical
- 処置: fixed D2

## R20. heartbeat だけが存在する通常状態で rollup source の down migration が失敗する

- 成果物: `migrations/202609240758_app_usage_rollup_source.down.sql`
- 根拠: `core.event` だけで削除可否を判定するが、権限未許可でも heartbeat が送られ、`core.heartbeat.logical_source` は `core.source` を参照する。event が 0 件でも外部キー違反で DELETE が止まる。`drop_report` 等の参照も同様に確認が要る。
- kind: technical
- 処置: fixed 4.2

## R21. `Collected.enqueued` の説明が永続化成功を保証するように読める

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/CollectionSource.kt`
- 根拠: `CollectionSource.kt:79` は「積めたもの」と説明するが、両アダプタは `outbox.add()` が失敗した記録も返す。将来この件数を成功数として使うと誤集計になる。
- kind: technical
- 処置: fixed 1.1

## R22. 定常取得でも rollup の `notImported` が窓の据え置きを示すように見える

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageRollupSourceAdapter.kt`
- 根拠: 初回完了後の DAILY は既に `done` でも、書込み失敗で `notImported++` が増え `usage_rollup_not_imported` が出る。印は変更されず、ログの説明と一致しない。
- kind: technical
- 処置: fixed 4.3

---

# code-verify（whole-branch・2026-09-26・HEAD `5a5eeaf`）

実装者の申告を疑って、コマンドを実行した結果だけを書く。変異とプローブは作業ツリーの外の複製
（`/tmp/st06-mut`・`/tmp/st06-rs`）で行い、作業ツリーのコードは触っていない。

## 申告と実測

申告: **27/27 `[x]`**。検証コマンドは tasks 本文のとおり（8.2 は `plan-corrections.md` の訂正後の入口）。

| 項目 | 申告の検証コマンド | 実測（2026-09-26・HEAD `5a5eeaf`） | 一致 |
|---|---|---|---|
| 1.1 | `./gradlew :app:testDebugUnitTest` / `git diff --numstat -- LocationFix.kt` 0 行 | harness が立てる `ASHIATO_ROBOLECTRIC_JARS` のままでは **rc=1**（2 本 FAIL）。外すと 341/341 rc=0 / numstat 0 行 | 不一致（R30） |
| 1.2 | `--tests '*TelemetryTest*'` / `grep -c LOGICAL_SOURCE Telemetry.kt` = 0 | 7 本 rc=0 / 0 | 一致（ただし R27） |
| 1.3 | `--tests '*HeartbeatCountersTest*'` | 13 本 rc=0 | 一致 |
| 1.4 | `./gradlew :app:connectedDebugAndroidTest --tests '*PermissionDeniedInstrumentedTest*'` | **`Unknown command-line option '--tests'` rc=1**。中身は `tools/android-emulator.sh` の 2 段目で PASS | 不一致（R28） |
| 2.1 | `--tests '*UsageSourceTest*'` | 14 本 rc=0（offline を外したとき） | 一致（R30） |
| 2.2 | `--tests '*UsageRetentionTest*'` | 7 本 rc=0 | 一致 |
| 3.1 | `--tests '*AppUsageCollectorTest*'` | 9 本 rc=0 | 一致（ただし R23） |
| 3.2 | `--tests '*UsageWindowTest*'` ＋ガードを壊す | 10 本 rc=0 / `Unreadable` でも窓を保存する変異 → rc=1 | 一致 |
| 3.3 | `--tests '*UsageWindowClockTest*'` | 10 本 rc=0 | 一致（ただし R24〜R26） |
| 3.4 | `--tests '*AppUsagePayloadShapeTest*'` | 5 本 rc=0 | 一致 |
| 4.1 | `--tests '*UsageGapTest*'` ＋ガードを壊す | 5 本 rc=0 / 出来事の時刻を期間の始まりにする変異 → rc=1 | 一致 |
| 4.2 | `cargo test -p ashiato-server app_usage_rollup_source` / `tools/check-migrations.sh` | 6 passed rc=0 / rc=0 | 一致（ただし R34） |
| 4.3 | `--tests '*RollupImportTest*'` | 16 本 rc=0 | 一致 |
| 4.4 | `--tests '*RollupPayloadShapeTest*'` | 5 本 rc=0 | 一致 |
| 5.1 | `--tests '*SourceIndependenceTest*'` / `connectedDebugAndroidTest --tests …` | 10 本 rc=0 / **`--tests` で rc=1** | 不一致（R28） |
| 5.2 | `--tests '*AndroidCapabilityTest*'` | 5 本 rc=0 | 一致 |
| 5.3 | `connectedDebugAndroidTest --tests '*UsageAccessInstrumentedTest*'` | **`--tests` で rc=1**。suite の中では前提不備で FAIL、単独の再実行では PASS | 不一致（R28 / R29） |
| 5.4 | `--tests '*RetentionNotifierTest*'` | 10 本 rc=0 | 一致 |
| 6.1 | `--tests '*RetentionTest*'` / 2 ソースを混ぜた試験 | 11 本 rc=0 / 1 本ある | 一致 |
| 6.2 | 同 ＋ガードを戻す | `records.oldest()` に戻す変異 → rc=1 | 一致 |
| 7.1 | `tools/usage-volume.sh` rc=0 / 数が deep.md に写る | N=40: 321 件/時・750 bytes・rc=0 / **N=300: 878 件/時・rc=1** | 不一致（R31） |
| 7.2 | `--tests '*IntervalTest*'` | 6 本 rc=0 | 一致 |
| 7.2b | `tools/smoke.sh` | rc=0（9b の 2 件と再送） | 一致 |
| 7.3 | 表を 1 行変えると試験が落ちる | `--rerun` では rc=1。**docs だけを変えて同じコマンドを叩くと UP-TO-DATE で rc=0** | 一部不一致（R33） |
| 8.1 | `python3 scripts/check_scenarios.py .` | rc=0（470 件・担保 470・人間の確認待ち 0） | 一致（ただし R23 / R27） |
| 8.2 | unit / `tools/android-emulator.sh` / `cargo test --workspace` / clippy / smoke | unit は R30 / **emulator rc=1**（1 段目 3 本 FAIL・2 段目は未実行）/ cargo 423 passed rc=0 / clippy rc=0 / smoke rc=0 | 不一致（R29 / R30） |
| 8.3 | `python3 scripts/review_triage.py . st06-app-usage` | rc=0（処置前の 45 件） | 一致 |

## R23. 本番の変換（`UsageStatsSource`）で種別や欄を落としても、341 本すべてが緑のまま通る

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/UsageSource.kt` /
  `collector-android/app/src/test/kotlin/dev/ashiato/collector/AppUsageCollectorTest.kt`
- 根拠: 複製の中で本番側だけを 5 通り壊し、`./gradlew :app:testDebugUnitTest`（全 341 本）を流した。**5 件とも rc=0**:
  `UsageSource.kt:189` で `STANDBY_BUCKET_CHANGED` を捨てる / `:238` の `standbyBucket` を常に null /
  `:231` の `configuration` を null / `:235` の `shortcutId` を null / `:227` の `className` を null。
  `種別でふるい落とさない` と `1 件が取得元の公開しているすべての欄を持つ` の印（`AppUsageCollectorTest.kt:59` / `:80`）は、
  `UsageEventSnapshot` を直に組んで `FakeUsageSource` から返しているので、**本番の `snapshotOf` を 1 度も通らない**。
  本番の変換を通すのは `UsageSourceTest.kt:26` の 1 本だけで、見ているのは `USER_INTERACTION` の 2 欄だけ。
  計測テストも `UsageVolumeInstrumentedTest`（件数を測るだけ。suite からも外されている）しか本物に当たらない。
  Q1 / Q8 は `loss: uncaptured`（落とした分は 10 日で消える）の決定。
- kind: technical
- 提案: `UsageSourceTest` の形（`ShadowUsageStatsManager.EventBuilder`）で、種別ごとの 1 件（`setConfiguration` /
  `setShortcutId` / `setAppStandbyBucket` を含む）を本番の `UsageStatsSource.events` に通し、種別と欄が全部残ることを確かめる。
  2 つの Scenario の印はそちらへ移す。
- 処置: fixed 3.1 —— `AppUsageCollectorTest` に本番の `UsageStatsSource` 越しに 1 契機を回す足場（`realSourceEnv`。
  `ShadowUsageStatsManager.EventBuilder`）を足し、`本番の変換を通しても種別は 1 件も落ちない` と
  `本番の変換が種別ごとの欄まで解析済みに残す` を置いた。2 つの Scenario の印はこちらへ移した（`e5e2067`）。

## R24. 経過の置き場（`age-clock.txt`）が読めなくなると、アプリ利用が「それまでの稼働日数＋10 日」止まる

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageSourceAdapter.kt` /
  `AgeClock.kt`
- 根拠: `AgeClock` は置き場が読めないと `Seen(0, …)` から数え直す（`AgeClock.kt:66`・ログは `age_clock_unreadable`）。
  窓の印は前の経過（`mark.ageMs`）を持ったまま残るので、`AppUsageSourceAdapter.kt:85` の `elapsedMs` が負になり、
  `skewMs`（`:86`）はそれまでの稼働日数ぶんの一定値になって `USAGE_CLOCK_SKEW_TOLERANCE_MS` を越え続ける。
  `eventsRetentionExceeded(elapsedMs)`（`UsageRetention.kt:89`）は負の経過では偽なので、再開するのは
  経過が `mark.ageMs + 10 日` に届いたとき。複製でプローブを書いて実測した（30 分ごとに契機を回し、
  途中で `age-clock.txt` に `broken` を書いて `AgeClock` を作り直す）:
  **稼働 1 日の後なら 11.0 日（528 契機）、稼働 40 日の後なら 50.0 日（2,400 契機）** `Unavailable(clock_skew)` が続き、
  再開時に gap が 1 件積まれた。時計は 1 秒も飛んでいない。止まる長さは端末を使った日数に比例して伸び
  （1 年使った端末なら 1 年と 10 日）、**10 日を超えた分のイベントは取得元から消える**（`loss: uncaptured`）。
- kind: technical
- 提案: `ageNow < mark.ageMs`（経過が戻った＝経過の置き場が作り直された）は時計の飛びの証拠にならないので、
  そのときは印の経過を今の値へ付け替えて通常の取得に進める（壁時計の差だけで判定する）。試験は上のプローブの形で 1 本。
- 処置: fixed 3.3 —— 経過が巻き戻った（`ageNow < mark.ageMs` か破棄の数えが減った）ときは飛びと読まず、
  印の経過を今の値へ付け替えて通常の取得に進む（`usage_age_clock_restamped`。窓は進めない）。
  試験 `経過の置き場が作り直されても次の契機で取れる`（`UsageWindowClockTest`。`e5e2067`）。

## R25. 1 時間未満の時計の変化で、戻りはイベントを黙って失い、進みは同じイベントを別の時刻で 2 行にする

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageSourceAdapter.kt` /
  `UsageWindow.kt`（`USAGE_CLOCK_SKEW_TOLERANCE_MS`）
- 根拠: design の Context が前提にしている取得元の振る舞い（時計が変わると統計を差分だけずらす `onTimeChanged`）を
  偽の取得元に入れてプローブを書き、複製で実行した。
  - **50 分戻る**: 09:00 に保存 → 09:10 にイベント → 09:20 に壁時計が 08:30 へ戻る（取得元もずらす）→ 08:35 にイベント →
    08:40 の契機は `usage_window_ahead` → 09:10 の契機と 09:40 の契機。記録は `later@09:10` の **1 件だけ**で、
    戻る前の 1 件と戻った後の 1 件は**どの記録にも gap にもならなかった**（ログは `usage_window_ahead` だけで、`usage_clock_skew` は出ない）。
  - **50 分進む**: 08:50 のイベントを 09:00 の契機で取得 → 09:20 に壁時計が 10:10 へ進む（取得元もずらす）→ 次の契機。
    記録は `fetched-once@08:50` と `fetched-once@09:40` の **2 行**。`event_time` は凍結されるので後から畳めない。
  spec は「時計の前進が単調な経過と食い違う THEN 窓を進めず、既に取った期間を取り直さない」と幅を持たずに書くが、
  実装は食い違いが 1 時間以内なら通常の取得に進む（design D8（仮））。D8 は「広すぎると行が増える」は書いているが、
  **戻る向きで失うこと**は書いていない。
- kind: technical
- 提案: 食い違い（`skewMs`）は取得元がずらした幅そのものなので、閾値の内側でも「保存した終わり＋食い違い」を
  次の窓の始まりにする（機械で決まる）。1 時間の閾値は「窓を止めるか」ではなく「そのずらしを信用するか」に使う。
- 処置: fixed D8 仮 —— 閾値の内側でも保存した終わりを食い違い（`skewMs`）の分だけ写し、次の窓をそこから組む。
  D8 の表の「なぜ」と反転条件を「幅はずらしを信用するかを決める」に書き直した。試験は提案の 2 通り
  （`閾値の内側で時計が50分戻っても戻る前後のイベントを取る` / `…進んでも同じイベントが2行にならない`。`e5e2067`）。

## R26. Q9 の自動再開で積む gap の理由が `retention`（取得元に無かった）になる。問い合わせていない期間で、取得元にはまだイベントがあった

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/AppUsageSourceAdapter.kt:213` /
  `AppUsageRecord.kt:111`（理由の定数は `USAGE_GAP_REASON_RETENTION` の 1 つだけ）
- 根拠: `UsageWindowClockTest` の Q9 と同じ形（保存 09:00・2 時間飛ぶ・11 日経過）をプローブで回した。gap の原文は
  `{"kind":"gap","begin":"2026-05-20T09:00:00Z","end":"2026-05-21T11:00:00Z","reason":"retention"}`。
  問い合わせは `2026-04-20T09:00..2026-05-20T09:00` と `2026-05-21T11:00..2026-05-31T11:00` の 2 回だけで、
  **gap の期間は 1 度も問い合わせていない**。偽の取得元には `2026-05-20T10:00` のイベントが残っていて、記録にならなかった。
  FR-85 / spec が gap に持たせる意味は「**取りに行って**取得元に無かった期間」と「取れなかった理由」で、
  扉 #14 の区別（収集が壊れていたのか）の材料になる。Q9=c で**諦めた**期間を「取得元が消していた」と書くことになる。
- kind: technical
- 提案: 再開の経路だけ別の理由（例: `clock_skew_abandoned`）を渡し、契約表（R32）にも載せる。
  Q9 の試験で `reason` を確かめる。
- 処置: fixed 3.3 —— 再開の経路だけ理由を `clock_skew_abandoned`（`USAGE_GAP_REASON_CLOCK_SKEW_ABANDONED`）にした。
  Q9 の試験で `reason` を確かめ、契約表（R32）にも 2 つの理由と「問い合わせたか」を載せた（`e5e2067`）。

## R27. 送信のログは、位置と混ざったひと組ではソースを名乗らない。通常の運用ではひと組はほぼ常に混ざる

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt:110` /
  `collector-android/app/src/test/kotlin/dev/ashiato/collector/TelemetryTest.kt:91`
- 根拠: 位置 1 件とアプリ利用 1 件を同じ置き場に積み、到達できない送信を 1 回流すプローブを実行した。
  ログは `kind=send_failed count=2 error=timeout` で、**`source=` が無い**（`sourceOf` が `singleOrNull()` で null を返す）。
  置き場は全ソースで 1 本（C11）で、位置は 60 秒ごと・送信は 5 分ごとなので、アプリ利用が載るひと組には位置も載る。
  Scenario `端末のログのソース名がそのソースを指す` の THEN（「そのログのソース名はアプリ利用の論理ソース」）を、
  印の先の試験は**アプリ利用だけのひと組**でしか観測していない。
- kind: technical
- 提案: ひと組の中のソースごとに件数を分けて 1 行ずつ書く（`send_failed source=c01-location count=…` と
  `source=c01-app-usage count=…`）。試験は混ざったひと組で持つ。
- 処置: fixed 1.2 —— `Sender` がひと組の中をソースごとに数え、失敗・受理・拒否をソースごとに 1 行ずつ書く。
  試験は混ざったひと組で 2 本（`TelemetryTest`。`e5e2067`）。

## R28. tasks 1.4 / 5.1 / 5.3 の検証コマンドは `--tests` を受け付けず rc=1。証跡は 1 件も無いまま `[x]`

- 成果物: `openspec/changes/st06-app-usage/tasks.md:56` / `:128` / `:142`
- 根拠: `./gradlew :app:connectedDebugAndroidTest --tests '*PermissionDeniedInstrumentedTest*' -m` →
  `Problem configuring task :app:connectedDebugAndroidTest from command line. > Unknown command-line option '--tests'.` rc=1。
  `evidence.jsonl` に item `1.4` / `5.1` / `5.3` の行は無い。同じ型の 8.2 は `plan-corrections.md` で入口を直したが、
  この 3 件は直されていない。中身のテストは `tools/android-emulator.sh` が走らせる
  （2 段目の `PermissionDeniedInstrumentedTest` / `ForegroundServiceTypeInstrumentedTest` は今回 PASS。
  `UsageAccessInstrumentedTest` は R29）。
- kind: technical
- 提案: 8.2 と同じ手順（`scripts/plan_fix.py`）で入口を `tools/android-emulator.sh`（または
  `-Pandroid.testInstrumentationRunnerArguments.class=…`）へ直し、証跡を取り直す。
- 処置: fixed 1.4 —— 2026-09-27 本人の回答（`deep.md` の **Q11** → a）。`scripts/plan_fix.py` が旧コマンドの FAIL の証跡
  （いまのコード・端末あり・`Unknown command-line option '--tests'` rc=1）と承認の出所を確かめてから、1.4 / 5.1 / 5.3 の入口だけを
  `tools/android-emulator.sh` に直した（`plan-corrections.md`。受け入れ条件は変えていない）。新しい入口の証跡は
  1.4 / 5.1 / 5.3 とも **rc=0**（それぞれ 879 / 880 / 878 秒。R29 の修正の後の HEAD）。

## R29. `tools/android-emulator.sh`（8.2）が HEAD で rc=1。`LocationServiceInstrumentedTest` は 2 回とも落ちた

- 成果物: `tools/android-emulator.sh` /
  `collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/LocationServiceInstrumentedTest.kt:98` /
  `UsageAccessInstrumentedTest.kt:51`
- 根拠: `tools/android-emulator.sh` → `BUILD FAILED in 41m 36s` rc=1。1 段目 9 本のうち 3 本 FAIL で、
  `set -e` により **2 段目は走らなかった**。
  - `LocationServiceInstrumentedTest.aMockFixBecomesOneRecordInTheOutbox`: `原文に緯度が残る: {"lat":39.237255,"lon":-123.1500317,…}`
    （先頭の記録が偽装位置 35.681236 ではない）。3 本だけを再実行しても**同じ値で FAIL**（2/2）
  - `UsageAccessInstrumentedTest`: `前提が作れていない。「利用状況へのアクセス」が既に許されている`。単独の再実行では PASS
    （再実行の前は `appops get` が `No UID` ＝未導入で、端末に allow が残っていたのではない）
  - `RetentionInstrumentedTest.ninetyDays…`: `送り切れていない（129800 / 129600）`。単独の再実行では PASS
  直近の PASS の証跡は `096720e`（`5a5eeaf` の manifest と実装の変更より前）で、HEAD では取られていない。
- kind: technical
- 提案: 位置の試験は「先頭」ではなく偽装した座標を持つ記録を探す形にするか、実位置が先に入る原因
  （収集の開始を位置の許可より先にした ST06 の変更との関係）を調べる。suite の中でだけ落ちる 2 本は
  前のテストが残す状態（サービス・置き場・権限）を洗う。HEAD で rc=0 の証跡を取り直す。
- 処置: fixed 8.2 —— 原因は 2 つで、どちらも再現してから直した（`349c456`）。
  (1) `LocationServiceInstrumentedTest` はサービスを立ててから偽装を有効にしていたので、その間にエミュレータの本物の GPS
  （`39.237255,-123.150032`）が先頭の記録になっていた → 偽装を**サービスより先に**有効にする（判定は変えない）。
  (2) suite の中でだけ落ちる 2 本は、AVD の userdata に `usage-volume.sh` が付けた `GET_USAGE_STATS` の allow が残り、
  `install -r` が引き継いでいた → `android-emulator.sh` は suite の前に、`usage-volume.sh` は終わりにアプリを外す。
  HEAD `349c456` で `scripts/verify-run 8.2` → `tools/android-emulator.sh` **rc=0（881 秒）**、unit / cargo（424 passed）/ clippy / smoke も rc=0。

## R30. harness の offline 設定のままだと `./gradlew :app:testDebugUnitTest` が rc=1

- 成果物: `collector-android/app/src/test/kotlin/dev/ashiato/collector/UsageSourceTest.kt:25`（`@Config(sdk = [34, 35])`。`5a5eeaf` で追加）
- 根拠: この環境の既定（`ASHIATO_ROBOLECTRIC_JARS=/home/yosis/.m2/robolectric-jars`。harness の
  `effects.py:android_env` が立てる）で全体を流すと `341 tests completed, 2 failed` rc=1 ——
  `Path is not a file: …/android-all-instrumented-14-robolectric-10818077-i7.jar`（15 も同じ）。置き場には
  `android-all-instrumented-16-…` しか無い。`env -u ASHIATO_ROBOLECTRIC_JARS` なら 341/341 rc=0。
  `robolectric_jars()` は置き場が空のときしか集めないので、次の Codex 実行でも同じく落ちる。
  2.1 の証跡（`d5a6b7c`・executor codex）は `@Config` を足す前の tree。
- kind: daily
- 提案: 置き場に API 34 / 35 の jar を足す（harness 側）か、試験を 1 つの SDK と「版で分岐する関数」の単体に分ける。
- 処置: rejected: **repo 側の欠陥ではない**。原因は harness の `robolectric_jars()`（置き場が空のときしか集めない）で、
  そちらを直した —— いま `ls /home/yosis/.m2/robolectric-jars` は `android-all-instrumented-14…` / `-15…` / `-16…` の
  3 本を返し、harness が立てる `ASHIATO_ROBOLECTRIC_JARS` のままで `./gradlew :app:testDebugUnitTest` が rc=0 になる
  （この波の `scripts/verify-run 1.1` の証跡）。提案のもう一方（`UsageSourceTest.kt:25` の `@Config(sdk = [34, 35])` を
  分ける）は**採らない** —— API 35 の `getExtras()` を 31〜34 で呼ばないこと（R18）は、版ごとに本番の変換を
  通して初めて固定できる。repo のコードは変えていない。

## R31. 7.1 の「実測」は 1 時間の件数がスクリプトの流し込み回数で決まる。回数を変えると判定が反転する

- 成果物: `tools/usage-volume.sh:43`（`n=${USAGE_VOLUME_EVENTS:-40}`）/ `openspec/changes/st06-app-usage/deep.md:130`
- 根拠: エミュレータで 3 通り流した:
  N=40 → `events_per_hour=321` `bytes_90_days=520020000` **rc=0** /
  N=100 → `446` `733116960` **rc=0** /
  N=300 → `878` `1467875520` **rc=1**。
  件数は N に連動し、上限（1 GB）に届くのは約 680 件/時（`1073741824 / (731 × 24 × 90)` を python で再計算）。
  `deep.md` は「現在の実装を測った未送信容量は 323 件/時 … 2 GB 上限の半分を下回り rc=0」と書き、
  7.1 はこの rc で「Q1（種別を絞るか）を再問しない」を決める形になっている。**323 件/時は本人の使い方の観測ではなく、
  スクリプトが選んだ 40 回の画面遷移の結果**で、24 時間ずっとその率とする外挿も入っている。
  あわせて、N=100 の 1 回目は前のエミュレータが落ちきる前に拾って `adb: … device offline` で rc=1 になった ——
  **「上限を超えた」と「adb が失敗した」が同じ rc=1** で、7.1 の「rc=1 なら本人に返す」と区別がつかない。
- kind: premise
- 提案: 取得元は本人の端末に**直近 10 日の実イベントを既に持っている**ので、実機でその 10 日を数えれば
  1 日あたりの件数は機械が観測できる（流し込みの回数を選ぶ必要が無い）。スクリプトは上限超過だけを rc=1、
  端末・ビルドの失敗は rc=2 にする。
- 処置: deferred ST14 —— 2026-09-27 本人の回答（`deep.md` の **Q10** → b）。40 回の流し込みの実測は根拠から外し
  （`deep.md` の該当行を ★ で打ち消した）、判定は**実機の直近 10 日の実イベント数**で取り直す。
  **実機で数えてしきい値を確定し、超えていたら Q1 を再問する作業は ST14**（収集が途切れたら気づける。
  「未送信が上限で捨てられ始めたら気づける」が趣旨）へ送る。この change では入口だけをそろえる ——
  `tools/usage-volume.sh` の rc を「上限超過だけ 1・端末/エミュレータ/ビルドの失敗は 2」に分け、
  実機の直近 10 日を数える経路を足す（下流）。残タスクが ST14 に残るので PR は `refs`。

## R32. 契約の `c01-app-usage` の表に gap の記録の形が無く、表の規則（`event_type` / `event_time` は省略しない）と矛盾する

- 成果物: `docs/collector-contract.md:169`（`c01-app-usage` の節）/ `AppUsageRecord.kt:125`（`usageGapRequest`）
- 根拠: `grep -n "gap" docs/collector-contract.md` は該当 0 件（ヒットは `expected_gap_sec` 等だけ）。
  節の本文は「利用状況のイベント 1 件につき記録を 1 件送る」「`event_type` と `event_time` は省略しない」。
  同じ論理ソースに積まれる gap の原文は、プローブで `{"kind":"gap","begin":…,"end":…,"reason":"retention"}` —— **どちらの欄も無い**。
  C-02 の節は `powered-off` を `kind` の列挙と欄の表に載せている（`:218` / `:226` / `:233`）ので、7.3 の「C-02 と同じ粒度」に届いていない。
  受け手（ST14）はこの契約を読んで gap を畳む。
- kind: technical
- 提案: `c01-app-usage` の節に `kind: gap` の形（`kind` / `begin` / `end` / `reason` と理由の列挙）を足し、
  gap の原文も表から読む固定試験にする。
- 処置: fixed 7.3 —— `docs/collector-contract.md` に `c01-app-usage` の `kind: gap` の節（`kind` / `begin` / `end` / `reason`・
  理由の列挙）を足し、`AppUsagePayloadShapeTest` が gap の原文と理由の列挙も表から読むようにした（`e5e2067`）。

## R33. docs の契約表だけを変えると単体テストが UP-TO-DATE になり、7.3 のガードが手元で効かない

- 成果物: `collector-android/app/build.gradle.kts:44`（`unitTests.all`）/
  `collector-android/app/src/test/kotlin/dev/ashiato/collector/CollectorContractFixture.kt:83`
- 根拠: 複製で `--tests '*PayloadShapeTest*'` を 1 度通した後、`docs/collector-contract.md` の `total_visible_ms` の行を消して
  同じコマンドを叩くと `> Task :app:testDebugUnitTest UP-TO-DATE` rc=0。`--rerun` を付けると
  `RollupPayloadShapeTest > 集計 1 件の形 FAILED` rc=1（`standby_bucket` の行・`class` の省略規則でも同じく rc=1）。
  試験は `user.dir` から上へたどって docs を読むが、Gradle の入力に宣言されていない。CI（まっさらな build）では効く。
- kind: daily
- 提案: `unitTests.all` の中で `test.inputs.file(rootProject.file("../docs/collector-contract.md"))` を宣言する。
- 処置: fixed D12 仮 —— `unitTests.all` で `docs/collector-contract.md` を試験の入力に宣言した（`build.gradle.kts`。`e5e2067`）。
  build の設定なので design に D12（仮）と反転条件を足した。

## R34. 移行の `ON CONFLICT DO NOTHING`（本人が変えた想定間隔を再起動で戻さない）を上書きに変えても 6 本すべて緑

- 成果物: `migrations/202609240758_app_usage_rollup_source.sql:24` / `crates/server/src/registry_tests.rs:186`
- 根拠: 複製で `DO NOTHING` を `DO UPDATE SET expected_gap_sec = EXCLUDED.expected_gap_sec, external_id_kind = …, display_name = …`
  に変えて `cargo test -p ashiato-server app_usage_rollup_source` → `6 passed; 0 failed` rc=0。
  `app_usage_rollup_source_migration_is_idempotent` は当て直しの前後で値が同じことしか見ておらず、
  値を変えてから当て直す形になっていない。移行のコメント（`:19`）が理由に挙げる「本人が変えた想定間隔が
  再起動のたびに初期値へ戻る」を止める試験が無い。
- kind: technical
- 提案: 試験の中で `expected_gap_sec` を別の値へ変えてから移行を当て直し、変えた値が残ることを確かめる（トランザクション内で戻す）。
- 処置: fixed 4.2 —— `app_usage_rollup_source_migration_keeps_values_changed_by_hand` を足した。トランザクション内で
  想定間隔と表示名を変えてから移行を当て直し、変えた値が残ることを確かめる（`registry_tests.rs`。`e5e2067`）。

## R35. Q9 の「10 日」は試験で 1 日ぶん緩い（閾値を 10 日＋12 時間にしても全緑）

- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/UsageRetention.kt:89` /
  `UsageWindowClockTest.kt:25` / `:37`
- 根拠: 複製で `elapsedMs > EVENTS.toMillis()` を `elapsedMs > EVENTS.toMillis() + 12 * 60 * 60 * 1000L` に変え、
  全 341 本 → rc=0。試験は「ちょうど 10 日なら再開しない」と「11 日なら再開する」の 2 点しか見ていないので、
  (10 日, 11 日] のどこに閾値を置いても通る。`false`（再開しない）に変える変異は rc=1 で止まった。
- kind: daily
- 提案: 10 日＋1 契機（30 分）で再開することを 1 本足す。
- 処置: fixed D8 仮 —— `10日を1契機超えたところで再開する` を足した（10 日ちょうどでは `Unavailable`、
  30 分後に `Collected`）。閾値そのもの（Q9=c の 10 日）は D8 の本人回答の段にあり、変えていない（`e5e2067`）。

## 手ごとの結果

- 手 1（固定値の独立な再計算）: 該当なし。確かめた範囲: `deep.md` の容量（323×24 = 7,752 / 7,752×90×731 = 510,004,080）、
  `UsageWindowClockTest` の gap の終わり（05-20T09:00 ＋2 時間＋11 日 −10 日 = 05-21T11:00）を python で再計算して一致。
  payload の直書きの期待値は、7.3 の仕掛けで契約表から読まれていることを `--rerun` の変異で確かめた（R33 は手元の入口の話）。
  ハッシュを直書きした試験は無い（`rollupFingerprint` の値を固定した試験は 0 本）。
- 手 2（ガードを壊す）: 3.2 / 4.1 / 6.2 / 7.3 の「壊すと落ちる」は実際に rc=1。
  `tools/check-migrations.sh` rc=0、`tools/test-usage-volume.sh` rc=0。生き残ったのは R23 / R34 / R35。
- 手 3（Scenario と test）: `check_scenarios.py . st06-app-usage` rc=0（470 / 470）。印の先が THEN を観測していないのは R23 / R27。
- 手 4（本人の決定が test で固定されているか）: 変異で確かめた。
  Q1 / Q8（ふるわない・全欄）→ **本番の変換では固定されていない**（R23）/ Q3（自動で送るのは 1 度）→ 印を外す変異で rc=1 /
  Q5（初回に 4 粒度）→ 年を飛ばす変異で rc=1 / Q6（30 分）→ `IntervalTest` と `SourceIndependenceTest` が 1,800,000 を直に見る /
  集計の 6 時間 → 3 時間にする変異で rc=1 / Q9（10 日を超えたら再開）→ 再開しない変異は rc=1、閾値は 1 日ぶん緩い（R35）/
  Q4 / Q7 → 4.1 のガードと `SourceIndependenceTest` で止まる。
  本人の決定ではない（仮）の値は、`USAGE_CLOCK_SKEW_TOLERANCE_MS` を 90 分・`USAGE_FIRST_WINDOW_MS` を 11 日にしても全緑
  （D8（仮）なので指摘にはしない。ただし R25 の閾値でもある）。
- 手 5（`[x]` と実体）: 表のとおり。コマンドが成立しないものは R28、HEAD で rc≠0 のものは R29 / R30、判定の根拠が合成値のものは R31。
  テスト名で絞る検証は、どれも 1 本以上走っていることを `test-results` で数えた（0 本で rc=0 のものは無い）。
- 手 6（隙間）: 経過の置き場の作り直し（R24）、時計の 1 時間未満の変化（R25）、諦めた期間の理由（R26）。
  受け手が断らないか（遡った記録が「登録より前」で恒久的に断られて端末から消える経路）は、`registered_at` が
  `coverage.rs:258` の稼働状況の判定にしか使われていないことを確かめた（断る経路は無い）。
