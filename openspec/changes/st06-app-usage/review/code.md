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
