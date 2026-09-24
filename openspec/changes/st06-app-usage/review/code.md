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
