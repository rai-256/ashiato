# ST04 実装レビュー（独立検証 / code-verify）

対象: ブランチ `feat/st04-offline-retention`（PR #49）、HEAD `5d133d4`（`git diff origin/main` 53 ファイル）。
突き合わせの正本: `deep.md`（Q1 / Q3 / Q4 / Q5 / Q6、C1〜C13）、`specs/device-collection/spec.md`、`specs/collection-coverage/spec.md`、`design.md`（D1〜D20）、`tasks.md`。

実施日: 2026-09-15。**作業ツリーのコードは触っていない**（`git status --short` は空のまま）。
わざと壊す検査と使い捨ての観測テストは、`git archive HEAD` で作った複製 `/tmp/st04v` の中だけで行った。
DB は `st04-testdb`（55404）と、複製用に別に立てた compose（project `st04verify`、55446。終了後 `down -v`）。
ポート 18787 と共有の 55432 には触れていない（偽データの確認は 18806 で行った）。

## 申告と実測

申告: tasks 32 件のうち **28 件 `[x]`**（未完了は 10.1 / 10.2 / 10.2b / 11.4）。

| 申告 | 検証コマンド | 実測 |
|---|---|---|
| cargo test --workspace 339 本緑 | `DATABASE_URL=…55404 cargo test --workspace` | 一致（339 passed。うち `ashiato-server` 253） |
| fmt / clippy | `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` | 一致（rc=0） |
| web 78 本緑 | `cd web && npm run test` | 一致（16 files / 78 passed） |
| Android 単体 174 本緑 | `./gradlew :app:testDebugUnitTest` | 一致（XML 集計 174 / failures 0） |
| `tools/smoke.sh` rc=0 | 複製で `COMPOSE_PROJECT_NAME=st04verify … BIND=127.0.0.1:18806 tools/smoke.sh` | 一致（rc=0、42 段目 `dropped_count=180`・`recorded`） |
| check_scenarios / check_chain / openspec validate | `python3 scripts/check_scenarios.py . st04-offline-retention` ほか | 一致（rc=0。担保あり 259 / 人間の確認待ち 1） |
| check-migrations / check-openapi / check-boundaries | `tools/check-*.sh` | 一致（rc=0） |
| check-immutable | 複製で `tools/check-immutable.sh` | 一致（rc=0）。**ただしトリガを潰しても rc=0**（R4） |
| 本人の決定 Q1/Q3/Q4/Q5/Q6 を守った | 手 4 | 部品では一致。**Q5 は本番の配線から外しても全部通る**（R3） |
| CI | `gh pr checks 49` | android / web / rust / smoke / chain / collector-windows* は pass、`android-instrumented` は pending（申告どおり） |

## 手 1: 固定値を独立に再計算する

python（`colorsys` / `zoneinfo`）で実装とは別に計算した。**すべて一致**。

| 固定値 | 独立の計算 | 実装・テスト |
|---|---|---|
| 印の輝度比（hsl(132,30%,L)、WCAG 相対輝度） | 9 on 92 = 14.823 / 9 on 40 = 3.892 / 72 on 9 = 9.758 | design D10 の 14.82 / 3.89 / 9.76、`drop-mark.test.tsx` |
| 固定長の枠の数 | (4096 − 4 − 8) / 20 = 204.2 → 204 枠で 4092 バイト | `WriteFailedLedger.SLOTS = 204`（`DropReport.kt`） |
| 範囲の終わり（180 件、01:00Z から 1 分ごと、残りなし） | 03:59:00.001Z | `DropReportTest` の期待値 |
| 日をまたぐ区間（13:00Z〜18:00Z） | 05-01 は 22:00〜24:00・120 件 / 05-02 は 00:00〜03:00・180 件 | `day_cell_dropped_is_cut_per_day` |
| 日の区間の丸め（01:00:00Z〜01:00:00.001Z） | from 10:00（切り捨て）/ to 10:01（切り上げ） | `day_cell_dropped_rounds_the_end_up_and_counts_hours_once` |
| **区間ごとの件数**（spec の規則「その区間と重なる時間の件数の合計」） | 10:30〜10:31 の区間は 10 時台（2 件）と重なるので **2** | **実装とテストは 0**（R6） |

## 手 2: ガードをわざと壊す

複製 `/tmp/st04v` で 1 つずつ置き換え、テストを走らせ、元に戻した（`/tmp/mutate.py`。置き換え前に出現回数 1 を assert）。
`Drainer` を壊す段は `DrainTest` の `maxRounds = 10` と `timeout 900` の下で走らせた。
対照として「必ず落ちるはずの壊し方」（A3c / A4c）を入れ、置き換えがテストに届いていることを確かめた。

| # | 壊したもの | 走らせたもの | 結果 |
|---|---|---|---|
| A1 | `Drainer` の続ける条件から `flushed.removed > 0` を外す | 単体 174 | 落ちた（`DrainTest.1 件も取り除けなかったら続けない`） |
| A2 | 続ける条件から `hasMoreThan(MAX_BATCH)` を外す | 単体 174 | 落ちた（DrainTest 3 本） |
| A3 | 範囲の終わりの「1 時間以内」を 2 時間に | DropReportTest | **通った**（R9） |
| A3c | 同じ所を 0 ms に（対照） | DropReportTest | 落ちた（2 本） |
| A4 | 「1 時間を超えて離れたら新しい報告」を 90 分に | DropReportTest | **通った**（R9） |
| A4c | 同じ所を 3 時間に（対照） | DropReportTest | 落ちた（1 本） |
| A5 | `LocationService` の `/drops` の `Sender` を `dropPermanentlyRejected = true` に | 単体 174 | **通った**（R3） |
| A6 | `openStores` の `ledger.writeFailed(...)` を消す | 単体 174 | **通った**（R3） |
| A7 | `maintain` の `ledger.unreadable(...)` を消す | 単体 174 | **通った**（R3） |
| A8 | `Outbox.MAX_UNWRITTEN` を 10,000 → 50 | 単体 174 | 落ちた（`WriteFailedTest.数えのファイルは 4096 バイトのまま伸びない`。300 未満の値でだけ落ちる） |
| A9 | `WriteFailedLedger.SLOTS` を 204 → 100 | 単体 174 | **通った**（R9） |
| A10 | `maintain` の `notifier.update()` を呼ばない | 単体 174 | **通った**（R3） |
| A11 | `LocationService` の `lost()` を何もしない形に | 単体 174 | **通った**（R3） |
| S1 | 移行の `drop_report_immutable` を `BEFORE UPDATE` だけにし、`drop_report_no_truncate` を消す | `tools/check-immutable.sh` / `cargo test drops_api_rows_are_immutable` | **どちらも通った**（R4） |

検査の外側: `check-immutable.sh` と `drops_api_rows_are_immutable` が消そうとする行は、どちらも**時間ごとの件数を持つ行**だけで、外部キーが削除を先に止める。範囲を持たない報告（`unreadable` / `write_failed` のあふれ）を消す試みはどこにも無い。

## 手 3: Scenario と test の突き合わせ

`python3 scripts/check_scenarios.py . st04-offline-retention` → `scenarios: OK`（Scenario 260 / 担保あり 259 / 人間の確認待ち 1）。
この change の 81 本はすべて印を持つ（印の先は上の一覧どおり実在）。1 本ずつ読んで、主張の階層とテストの階層がずれていたものは次のとおり。

- **本番の配線を観測していない**（R3）: `置き場に書けなかった記録も報告される` / `書けなかった記録の報告は時間ごとの件数を持つ` / `書けなかった記録が 1 件でも範囲の終わりは始まりより後`（`WriteFailedTest.restart()` が `openStores` の手順を**写した**もの）、`読めない行の件数が報告される`（テストが `st.ledger.unreadable` を手で呼ぶ）、`断られた破棄の報告も未送信から取り除かれない`（テストが自分で `Sender(..., dropPermanentlyRejected = false)` を組む）、`常駐の通知に未送信の日数が出る` ほか通知の 6 本（`RetentionNotifier` を直に組む）、`1 時間の圏外の記録は全部が届く` / `1 時間の圏外では破棄の報告が作られない`（計測テストが `Drainer` を自分で組む。`LocationService` を起こさない）
- **spec の SHALL と逆の値を固定している**（R6）: `稼働状況の応答は日ごとの破棄の件数と時刻の範囲を持つ` の「区間ごとの件数」。Scenario は日をまたぐ 1 区間しか置かず、2 区間が同じ時間に重なる場合は Scenario が無い。テスト `day_cell_dropped_rounds_the_end_up_and_counts_hours_once` は spec と逆の 0 を期待値にしている
- **境界を挟んでいない**（R9）: `出来事の時刻が離れた破棄は別の報告になる`（2 時間だけ）、`残った記録が離れていれば範囲の終わりは最後に捨てた記録の直後`（2 時間 1 分だけ）
- `格納された破棄の報告は書き換えられない`: UPDATE は全列を見ているが、DELETE / TRUNCATE の側は外部キーの失敗でも同じ結果になる（R4）
- `時計が先へ飛んでも 90 日の側では捨てない` の THEN は「未送信から取り除かれない」だが、テストは `AgeClock` の差（1 日）を見る。`Retention` がこの時計を使う配線は `LocationServiceTest` の 90 日の段が通っているので、指摘にはしない

## 手 4: 本人の決定が test で固定されているか

| 決定 | 値 | 書き換えたら落ちるテスト |
|---|---|---|
| Q1 上限は送れない理由を問わず全部 | —— | あり（`RetentionTest.登録簿に無いソースとして…`、`LocationServiceTest.設定が揃っていなくても積む契機で上限をかける`） |
| Q3 右下の三角・期間と件数は週の詳細だけ・一覧なし | 12 px の三角 | あり（`drop-mark.test.tsx` が位置・枠線の色、`drop-detail.test.tsx` が一覧の不在） |
| Q4 積んでから数える | 90 日 | あり（`RetentionTest.本番の上限は 90 日と 2 GB と 83 日` が `DEFAULT` を直値で、`出来事の時刻が 3 年前でも…`） |
| Q5 常駐の通知に日数 / 83 日で 1 回 | 83 日・1 回 | 部品ではあり（`RetentionNotifierTest`）。**`LocationService` から `notifier.update()` を外しても全部通る**（A10 → R3） |
| Q6 溜まっている間は続けて送る | 200 件超・取り除けたら続ける | あり（A1 / A2）。本番の上限 `maxRounds = 100_000` は固定なし（R5 の上限として効く値） |
| ST01 D9 の 5 分 | 5 分 | 既存の `IntervalTest` / `LocationServiceTest`（今回は壊していない） |
| D2（仮）30 日 | 30 日 | あり（`ElapsedClockTest` が 10 + 30 日を直値で） |
| D5（仮）204 枠 | 204 | **値を変えても全部通る**（A9。4096 バイトに収まる値なら何でも通る） |
| D18（仮）1 万件 | 10,000 | 300 未満にしたときだけ落ちる（A8） |
| spec の 1 時間（範囲の終わり / 報告を分ける） | 1 時間 | **値を変えても全部通る**（A3 / A4） |

## 手 5: tasks の `[x]` と実体

28 件の `[x]` の検証コマンドを本文のとおりに走らせた。**名指しのテスト・クラス・ファイルはすべて実在し、rc=0**。

- `CT drop_reports_migration_applies_twice`（1 passed）/ `CT drop_report_validate`（11）/ `CT drops_api`（12）/ `CT dropped_ranges_merge`（3）/ `CT coverage`（68）/ `CT day_cell_dropped`（5）: いずれも rc=0 かつ件数の grep が真
- `grep -rn 'Scenario: 破棄が期間と件数で残る' crates/server/src | grep -v spans.rs` → 1 件 / `grep -c '"/drops"' docs/openapi.json` → 1
- `VT drop-mark.test.tsx`（6）/ `VT drop-detail.test.tsx`（4）/ `VT one-scroll.test.tsx`（4）: rc=0。`git diff --exit-code origin/main -- web/src/__tests__/one-scroll.test.tsx` rc=0
- 4.3: 複製で `npx tsc -b && npm run lint && npm run build` rc=0、`tools/check-boundaries.sh` rc=0
- `GT` 11 クラス（SegmentStoreTest 6 / SegmentMigrationTest 5 / UnreadableLineTest 4 / ElapsedClockTest 6 / RetentionTest 9 / SenderTest 14 / DropReportTest 12 / DropReportSendTest 5 / WriteFailedTest 6 / DrainTest 7 / RetentionNotifierTest 6）: 全部 rc=0 で XML が実在
- 10.3 `tools/smoke.sh` rc=0（上表）
- 10.4 `tools/seed.sh normal` rc=0。検証の curl は 18787 を名指しするが、そこは人間の確認用なので**同じ式を 18806 の自前のサーバに対して**実行 → `true`（rc=0）。2026-09-05 は `dropped` / 1,440 件 / `00:00〜24:00`、2026-09-07 は `recorded` / 180 件 / `10:00〜13:00`
- 11.1 / 11.2 / 11.3: 上表のとおり rc=0。`./gradlew :app:assembleDebug` も複製で rc=0
- 11.5: `ls docs/handoff/ST04.md` は無い（空）

テスト名だけ挙がって実体が無いもの・0 本で緑になるもの: **該当なし**。
ただし 5.4 の本文「既存の `OutboxStoreTest` … を新しい置き場で通す」について、ST02 の R17（`読めなかった未送信が、次の書き直しで消えない`）ほか 7 本が削除され、同じ性質（一時的に読めない置き場で失わない）を新しい置き場で確かめるテストは無い（R2 の根拠）。

## 手 6: 隙間

複製に使い捨てのテスト（`ZzVerifyGapsTest.kt` / `ZzVerify2Test.kt`。assert せず観測値を出す）を置いて走らせた。本番の配線が要る所は `LocationService.kt:121-127` の `WriteFailures` をそのまま写した。
記録・報告が黙って消える経路を 3 つ（R1 / R2 / R10）、件数が合わない経路を 1 つ（R7）、読める記録が読めない行に変わる経路を 1 つ（R8）、止まらない送信を 1 つ（R5）見つけた。
他の Story の担当に落ちるもの（defer）は無かった。

---

## R1. 書けない記録が 1 万件を超えると、手放した分を固定長の数えから引いてメモリの下書きへ移すので、空きが尽きたまま立て直すと報告ごと消える
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / Outbox.kt / DropReport.kt
- 根拠: 複製の使い捨てテスト `g1_lostMovesFromDurableCounterToVolatileDraft`。置き場・下書き・報告の置き場をすべて書けない場所に置き、`LocationService.kt:121-127` と同じ `lost()` をつないで `MAX_UNWRITTEN + 5` 件を積み、固定長の数えと下書きを開き直した → `G1 added=10005 inMemoryDraftCount=5` / `afterRestart durableCounter=10000 rebornDrafts=0 lostWithoutTrace=5`。`lost()` は先に `writeFailed.recovered(t)`（`LocationService.kt:124`）で**書き込めている固定長の数え**から引き、書けない `drops-open.json` の下書きへ移す（`DropReport.kt:270-279` は失敗するとメモリに持つだけ）。design D18（`design.md:264`「固定長の数えからは引く。二重に数えない」）はこの移し替えを決めているが、固定長の数え（D5）は「空きが尽きても上書きは通る」ために作った置き場で、D18 はその唯一の生き残る置き場から数えを抜いている。A11（`lost()` を何もしない形）でも単体 174 本が通るので、この経路はテストに無い
- kind: technical
- 提案: 下書きか報告の置き場に書けたことを確かめてから固定長の数えを引く（書けなければ数えに残し、次の起動の `write_failed` の報告に任せる）。空きが尽きた状態で 1 万件を超えてから立て直す単体テストを置く
- 処置: fixed D18 — `lost()` は `DropLedger.record` で下書きを保存できたときだけ固定長の数えから引く。単体 `WriteFailedTest.手放した記録は下書きを保存できなければ数えに残る`


## R2. 先頭の区切りが一時的に読めないと、2 GB の側がそれを飛ばして新しい区切りから捨てる（ST02 の R17 のテストが置き換えられずに消えた）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/OutboxStoreTest.kt
- 根拠: 使い捨てテスト `g5_transientUnreadableHeadSegmentDropsNewerFirst`（区切り 3,000 バイト・20 件、先頭の区切りを `setReadable(false)`、上限を 2,000 バイト下げて `dropUntilBytes`）→ `G5 dropped=[id-05, id-06, id-07, id-08] remainingHead=[id-00, id-01, id-02]`。`forEachLine` は `IOException` をログにして何も渡さず戻り（`SegmentStore.kt:282-284`）、`dropUntilBytes` / `dropHead` はそのまま次の区切りへ進む（`SegmentStore.kt:178` / `:209`）。`liveBytes()` は読めない区切りのバイトも数えるので、**新しい記録を捨てて古い記録を残す**。spec「端末に積んだ順が古いものから」（`specs/device-collection/spec.md:160`）と逆で、捨てた分は戻らない。ST02 の review R17（一過性の EMFILE・direct boot 中のアクセス）を固定していた `OutboxStoreTest.読めなかった未送信が、次の書き直しで消えない` は、`git show origin/main:…/OutboxStoreTest.kt` にあって HEAD では削除され、新しい置き場で同じ性質を見るテストは無い
- kind: technical
- 提案: 区切りを読めなかったら、その見回りでは捨てるのをやめる（次の見回りで当たり直す）。先頭の区切りを読めなくして `dropHead` / `dropUntilBytes` が 0 件を返すテストを、消した R17 の代わりに置く
- 処置: fixed D3 — `SegmentStore.dropHead` / `dropUntilBytes` は区切りを読めなかったらその見回りで止める。単体 `SegmentStoreTest.先頭の区切りが読めないと 90 日も 2 GB も捨てずに止まる`（ST02 R17 の置き直し）


## R3. 本番の配線（報告を捨てない・書けなかった分の報告・読めない行の報告・知らせ）を外しても単体 174 本が全部通る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/WriteFailedTest.kt / UnreadableLineTest.kt / DropReportSendTest.kt / RetentionNotifierTest.kt / collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/RetentionInstrumentedTest.kt
- 根拠: 手 2 の A5（`LocationService.kt:297` を `dropPermanentlyRejected = true`）/ A6（`:208` の `ledger.writeFailed` を削除）/ A7（`:237` の `ledger.unreadable` を削除）/ A10（`:241` の `notifier.update()` を呼ばない）/ A11（`lost()` を空に）→ いずれも `./gradlew :app:testDebugUnitTest` rc=0・174 passed・failures 0。対応する Scenario の印の先は、配線を**テストの中で写したもの**を観測している: `WriteFailedTest.kt:39-43` の `restart()`（「`LocationService.openStores` と同じ手順」とコメント）、`UnreadableLineTest.kt:59` の手呼び `st.ledger.unreadable`、`DropReportSendTest.kt:33` の自前の `Sender`、`RetentionNotifierTest.kt:46` の自前の `RetentionNotifier`、`RetentionInstrumentedTest.kt:96-103` の自前の `Drainer`。A5 が本番に入ると、**恒久的に断られた破棄の報告が端末から消える**（C2「送れるまで捨てない」/ spec `device-collection/spec.md:268`）。A10 が入ると Q5（本人の決定）の日数も 83 日の知らせも出ない
- kind: technical
- 提案: `LocationServiceTest`（`TestableLocationService` は `newTransport` / `newDeviceClock` を差し替えられる）に、(1) `/drops` が 400 + `malformed` を返しても `dropsOutboxForTest.size()` が減らない、(2) 書けなかった数えを置いてから起動すると `write_failed` の報告が積まれる、(3) 区切りに読めない行を置いて見回ると `unreadable` の下書きができる、(4) 12 日進めて見回ると常駐の通知の本文に「未送信 12 日」、の 4 本を足す
- 処置: fixed 6.2 — `LocationServiceTest` に本番の配線の試験 5 本（断られた破棄の報告が残る / 起動時の書けなかった分の報告 / 読めない行の報告 / 常駐の通知の日数 / 設定の無い端末の積む契機の見回り）。A5 / A6 / A10 / afterAdd を外す改変で、それぞれ落ちることを確かめた


## R4. `drop_report` の DELETE と TRUNCATE のトリガを消しても、`check-immutable.sh` も結合テストも通る。範囲を持たない報告は実際に消せる
- 成果物: migrations/202609151546_drop_reports.sql / tools/check-immutable.sh / crates/server/src/drops_tests.rs
- 根拠: 複製で `migrations/202609151546_drop_reports.sql:63` を `BEFORE UPDATE ON core.drop_report` にし、`:67-69` の `drop_report_no_truncate` を消した → `tools/check-immutable.sh` rc=0（「OK 破棄の報告は行ごとも表ごとも消せない」）、`cargo test -p ashiato-server drops_api_rows_are_immutable` → `1 passed`。同じ schema で範囲を持たない報告（`reason='unreadable'`、時間ごとの件数の行なし）を入れて `DELETE FROM core.drop_report WHERE id=…` → rc=0、行数 0。元の移行に戻して同じ DELETE → トリガの例外で拒否。検査が消そうとする行（`check-immutable.sh:569` / `:571`、`drops_tests.rs:421`）はどれも `drop_report_hour` の行を持ち、外部キーと `drop_report_hour` 側のトリガが先に止めるので、`drop_report` 自身のトリガの有無を区別できない。spec `collection-coverage/spec.md:437`「受け付けた破棄の報告を後から書き換えない」
- kind: technical
- 提案: `check-immutable.sh` と `drops_api_rows_are_immutable` に、範囲を持たない報告を 1 件置いて DELETE と `TRUNCATE core.drop_report`（CASCADE なし・あり）が拒まれることを足す
- 処置: fixed D7 — `tools/check-immutable.sh` に範囲を持たない報告の DELETE と、4 つの錠の実在（`pg_trigger`）を足し、`drops_api_rangeless_rows_cannot_be_deleted` を足した。S1 と同じ改変（DELETE を外す）で check-immutable が NG になることを確かめた


## R5. `.acked` に追記できないと、1 件も取り除けていないのに「取り除けた」と数えて同じ 200 件を送り続ける（本番は 10 万回まで）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt / Retention.kt（`Drainer`）
- 根拠: 使い捨てテスト `g2_drainerLoopsWhenAckCannotBeWritten`（600 件を積み、`000000000000.acked` をディレクトリにして追記を失敗させ、全件受け付ける受け口で `maxRounds = 500` の `tick()`）→ `G2 rounds=500 (cap 500; 本番 100000) recordsLeft=600`。`Sender.kt:124` は `outbox.remove` の失敗をログにするだけで、`:130` は `verdict.remove.size` を `removed` として返す。`Drainer` の続ける条件（`Retention.kt:198`）はこの値を見る。design D12（`design.md:209`）の「その 1 回で 1 件以上を取り除けた」は、実際に置き場から取り除けたかを見ていない。端末の空きが尽きた状態（`.acked` への追記が失敗する典型）で網に届くと、1 回の契機で 200 件ぶんの POST を最大 10 万回繰り返す。Q6 の反転条件「復旧後の電池・通信量」（`design.md:214`）そのものに当たる
- kind: technical
- 提案: `Flushed.removed` を、置き場から取り除けた件数（`outbox.remove` が成功したときだけ）にする。`.acked` を書けなくした `DrainTest` を 1 本足す
- 処置: fixed D12 仮 — `Sender.flush` は `outbox.remove` が失敗したら `removed = 0` を返す。`DrainTest.取り除きを書けなかったら続けて送らない`（元に戻す改変で落ちることを確かめた）


## R6. 「区間ごとの件数」を spec は重なる時間の合計とし、実装とテストは前の区間にだけ数える。画面に「うち 0 件を破棄」が出る
- 成果物: crates/server/src/coverage.rs / crates/server/src/coverage/tests/drops.rs / openspec/changes/st04-offline-retention/specs/collection-coverage/spec.md / design.md（D19）
- 根拠: spec `collection-coverage/spec.md:474`「区間ごとの件数を、その区間と重なる時間の件数の合計とする」。実装 `coverage.rs:815-821` は重なる区間のうち最初の 1 つにだけ足し、design D19（仮。`design.md:272`）がそう書いている。テスト `day_cell_dropped_rounds_the_end_up_and_counts_hours_once`（`coverage/tests/drops.rs:280`）は `range("10:30", "10:31", 0)` を期待値にして通っている（`CT day_cell_dropped` 5 passed）。spec の規則で独立に数えると 10:30〜10:31 の区間は 10 時台と重なるので 2。その応答を複製の web に渡して週を選ぶと、行の文字は `2026-05-05記録あり うち 2 件を破棄（10:00〜10:01） うち 0 件を破棄（10:30〜10:31）`（使い捨ての vitest）。D19 は（仮）だが spec の SHALL と逆で、archive で正典に入るのは spec の側
- kind: conflict
- 提案: どちらかに揃える。D19 を採るなら spec の SHALL を「時間が 2 つの区間にまたがるときは前の区間に数える」に直し、Scenario を 1 本足す。spec を採るなら二重に数える（区間の合計が日の件数を超えうる）ことを文字で許す。どちらでも件数 0 の区間を「うち 0 件を破棄」と出さない扱いを決める
- 処置: fixed D19 仮 — design D19（前の区間にだけ数える）を採り、spec の SHALL に「最も早い区間にだけ数える」と「件数を持たない区間は件数を添えずに時刻だけ」を足し、Scenario 2 本（同じ時間に重なる 2 つの区間では前の区間にだけ数える / 件数を持たない区間は件数を添えずに時刻だけが出る）。web は件数 0 の区間を「破棄（from〜to）」と出す。反転条件は D19


## R7. 旧 JSONL の取り込みが書けないまま立て直すと、元の JSONL を取り込み直すのに「書けなかった」の報告も積む（届いた記録を破棄とも数える）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LegacyOutbox.kt / LocationService.kt
- 根拠: 使い捨てテスト `g3_legacyImportCountedAsWriteFailedAndReimported`（3 件の `outbox.jsonl` を書けない置き場へ取り込み → `openStores` の順で数えを `take` して `ledger.writeFailed` → 書ける置き場へもう一度取り込む）→ `G3 first=LegacyMigration(imported=0, unreadable=0, complete=false) second=LegacyMigration(imported=3, unreadable=0, complete=true) reportedWriteFailed=3 recordsNow=3`。`migrateLegacyOutbox` は `into.add(item)`（`LegacyOutbox.kt:54`）が false でも元を消さない（`:58`）一方、`Outbox.add` は同じ記録を `writeFailures.failed` で固定長の数えに入れる。次の起動で 3 件は再取り込みされて送られ、同時に `write_failed` 3 件の報告が `/drops` に届く。受け手はこの時間を「一部を破棄」の印にする
- kind: technical
- 提案: 取り込みでは書けなかった記録を固定長の数えに入れない（元の JSONL が残ることが記録の置き場になっている）。上の手順をそのまま単体テストにする
- 処置: fixed D5 — 取り込みは `Outbox.importLegacy`（書けなかった記録を数えない・メモリに持たない）で書く。`SegmentMigrationTest.取り込みで書けなかった記録は書けなかった数えに入らず、次の起動で取り込み直す`


## R8. 追記が途中まで書けて失敗した同じ起動のうちは、書き直した記録がその半端な行に繋がって読めない行になる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt
- 根拠: 使い捨てテスト `g4_partialAppendConcatenatesNextRecordInSameProcess`（`a` を追記 → 同じ区切りへ `b` の行の先頭 40 バイトだけを足す（`appendBytes` が途中で `IOException` になった跡）→ 同じ `SegmentStore` で `b` と `c` を追記 → 開き直して読む）→ `G4 readable=[a, c] unreadableLines=1`。末尾の改行を見るのは区切りごとに起動で 1 度だけ（`SegmentStore.kt:94-98` の `terminated`）で、`append` の `IOException`（`:102-110`）は `terminated` を戻さない。`java.io.FileOutputStream.write` は書けた分を残して例外を投げるので、空きが尽きた端末で起きる。`b` は `Outbox` のメモリから書き直せたとして固定長の数えから引かれ（`Outbox.kt:66`）、その後で読めない行として範囲を持たない報告になる。行のバイトは `unreadable.jsonl` に残るが、送られはしない
- kind: technical
- 提案: `append` が `IOException` を投げたら、その区切りを `terminated` から外す（次の追記の前に末尾の 1 バイトを見直す）。上の手順を `SegmentStoreTest` に置く
- 処置: fixed D1 — `SegmentStore.write` の `IOException` で `terminated` から外し、次の追記の前に末尾を見直す。`SegmentStoreTest.途中で失敗した追記の後の 1 件は半端な行に繋がらない`（元に戻す改変で落ちることを確かめた）


## R9. spec の「1 時間」2 つと design D5 の 204 枠は、値を変えても全部通る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/DropReportTest.kt / WriteFailedTest.kt
- 根拠: 手 2 の A3（`DropReport.kt:177` の `r - last <= HOUR_MS` を 2 時間）と A4（`:149` の `t - d.endMs > HOUR_MS` を 90 分）→ `DropReportTest` 12 passed。対照の A3c（0 ms）/ A4c（3 時間）は落ちた。テストが置く点は、範囲の終わりが 59 分と 2 時間 1 分、報告を分けるのが 2 時間ちょうどだけで、1 時間を挟んでいない。spec `device-collection/spec.md:262` / `:266` はどちらも「1 時間」を SHALL にしている。A9（`SLOTS` を 100）→ 単体 174 passed。`WriteFailedTest` は `WriteFailedLedger.SLOTS` を記号のまま使うので、4096 バイトに収まる値なら何でも通る
- kind: technical
- 提案: 1 時間ちょうど・1 時間 + 1 ms の 2 点を両方の規則に置く。`SLOTS` は `assertEquals(204, WriteFailedLedger.SLOTS)` か、205 時間ぶんで 1 件があふれることで固定する
- 処置: fixed D4 — `DropReportTest` に 1 時間ちょうど / 1 時間 + 1 ms の 2 点を両方の規則に置き、`WriteFailedTest.枠は 204 個で、205 時間目は件数だけのあふれになる` で 204 を固定した


## R10. 送る前の下書きのファイルが読めないと、ログ 1 行を残して次の保存で上書きし、捨てた記録の件数が端末から消える
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: 使い捨てテスト `g6_unreadableDraftFileIsOverwritten`（途中で切れた `drops-open.json`（`"count":180` を含む）を置いて `DropLedger` を開き、1 件捨てて `endBatch`）→ `G6 files=[drops-open2.json] garbageKept=false log=[kind=drop_drafts_unreadable … error=JsonDecodingException]`。`DropLedger.load`（`DropReport.kt:254-264`）は読めなければ戻るだけで、`save()`（`:270-275`）が元のファイルを置き換える。下書きに入っている件数の記録は、置き場からはもう取り除かれている（`Retention` は取り除いてから `endBatch` で保存する）。`save()` は tmp + rename なので、起きるのはファイルそのものが壊れたときに限られる。C9 / D6（読めないものは捨てずに退避）の規律がこのファイルにだけ無い
- kind: technical
- 提案: 読めない下書きのファイルは `outbox/unreadable.jsonl` か `.unreadable.<時刻>` へ退避してから新しく始め、`unreadable` の報告に 1 件足す
- 処置: fixed D6 — 読めない下書きのファイルは `.unreadable.<時刻>` へ退避し、読めなかったこと 1 件を数える。`DropReportTest.読めない下書きのファイルは上書きせずに退避する`

---

# pr-review-toolkit（code-reviewer / silent-failure-hunter / pr-test-analyzer）

2026-09-15、`git diff origin/main`（HEAD `5d133d4`）に 3 本をかけた。**会話に返った指摘をここに R 番号で写した**（上の R1〜R10 と重なるものは、その R 番号を処置に書いた）。
略記: CR = code-reviewer / SF = silent-failure-hunter / TA = pr-test-analyzer。

## R11. 週の詳細の時刻で、日の最後の 1 分に終わる破棄が `24:00` でなく `00:00` になる
- 成果物: crates/server/src/coverage.rs
- 根拠: CR #3。`CASE WHEN ce >= de THEN '24:00'` は切り上げる前の終わりで比べるので、23:59:00.001〜23:59:59.999 に終わる区間が翌日の 0 時に丸まって `to_char` が `00:00` を返す
- kind: technical
- 処置: fixed D19 — 分に切り上げた終わりで日の終わりと比べる。`day_cell_dropped_ending_in_the_last_minute_is_24_00`

## R12. 設定が揃っていない端末で上限がかかる経路（本人の決定 Q1）を、本番の配線で確かめていない
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/LocationServiceTest.kt
- 根拠: TA C2。試験は `maintainForTest()`（間引かない口）を直に呼んでおり、本番の唯一の経路 `afterAdd = { maintain(force = false) }` を外しても緑
- kind: technical
- 処置: fixed 6.2 — `ShadowSystemClock.advanceBy` で間引きの 1 分を越えてから積み、90 日を超えた記録が捨てられることを見る試験に替えた。afterAdd を外す改変で落ちることを確かめた

## R13. 置き場への書き込みが途中で失敗すると、次の 1 件がその断片につながる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt
- 根拠: CR #2 / SF C3（R8 と同じ原因。SF は読み手が間に入ると `#L<n>` の印の下に隠れる経路まで示した）
- kind: technical
- 処置: fixed D1 — R8 の処置（`terminated` を外して見直す）に加え、行の鍵を行番号にしたので、繋がった行が既存の印の下に隠れる経路も無くなった

## R14. 記録の置き場と上限の見回りが逆の順で錠を取り、位置の糸と送信の糸が止まる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Outbox.kt / LocationService.kt / Retention.kt
- 根拠: CR #1。`Outbox.add` は錠を持ったまま `afterAdd` → `maintain` → `Retention.enforce`（`Retention` の錠 → 置き場の錠）を呼ぶ。送信の糸は `Drainer` から `Retention` の錠 → 置き場の錠の順で入る
- kind: technical
- 処置: fixed D1 — `afterAdd` を置き場の錠の外で呼び、見回りは専用の錠で 1 本の糸ずつにした。錠の順を D1 に書いた

## R15. 起動時に「書けなかった記録」の数えを、報告を保存する前に 0 に戻す
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / DropReport.kt
- 根拠: SF C1。`take()` が先に 0 を書き、報告の追記は空きが尽きていると失敗してメモリにだけ残る。送る前に立て直すと痕跡が消える
- kind: technical
- 処置: fixed D5 — `peek()` で読み、`ledger.writeFailed` が閉じた下書きを保存できたときだけ `clear()`。`WriteFailedTest.報告の下書きを保存できなければ、固定長の数えは残る`

## R16. 凍結で積めなかった報告の下書きを消し、メモリにしか無い報告が残る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: SF H2。`freeze()` は `frozen.add` が false でも下書きを消して保存する
- kind: technical
- 処置: fixed D4 — 積めなかった下書きは閉じたまま残し、メモリの報告は捨てる（次の凍結で同じ原文を積む）。`DropReportTest.凍結で積めなかった下書きは残り、次の凍結で同じ原文を積む`

## R17. メモリから手放した記録（`lost`）を、報告より先に数えから引く
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: SF C2（R1 と同じ経路）
- kind: technical
- 処置: fixed D18 — R1 の処置

## R18. 「溜まった分を送っている間に生まれた記録も 1 時間以内に届く」の遅延の検査は単独では落ちない
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/RetentionInstrumentedTest.kt
- 根拠: TA。`lag < 60 分` は、その前の「40 分以内に送り切った」からすでに言える。偽のサーバは端末の中で遅延が 0
- kind: technical
- 処置: rejected: 検査の目的は「古い順に送る間に新しい記録が後ろで待たされても、続けて送るので 1 時間に収まる」ことで、実物の網の遅延ではない。溜まった 129,600 件を送り切る前に新しい記録が届かない（続けて送らず 5 分ごとに戻る）実装なら、送り切りに約 55 時間かかり、40 分の待ちの側で落ちる —— 2 つの検査は同じ欠陥を別の観測で捕まえている。CI（run 34953342314）で 24 分・8 本緑

## R19. 受け口 `/drops` で識別子の重複が 500 になり、報告の送信が止まる
- 成果物: crates/server/src/drops.rs / collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: SF H6。`ON CONFLICT` の対象が冪等キーだけで、主キーの衝突は 500。凍結の途中で落ちて下書きが戻り、伸ばされると同じ `id` で原文が変わる
- kind: technical
- 処置: fixed D7 — サーバは衝突の相手を見分け、原文が同じなら重複、違えば `id_reused` で 1 件ごとに断る（`drops_api_rejects_reused_id_per_item`）。端末は下書きを閉じてから凍結するので、戻ってきた下書きが伸ばされない（D4）

## R20. 利用者識別子が未設定のうちに凍結した報告が、恒久的に断られ続ける
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: SF H7 / TA。起動時の書けなかった分・離れた破棄・`lost` はその場で凍結しており、`Config.userId` が空だと `user_id: ""` で固まる
- kind: technical
- 処置: fixed D4 — 凍結は `freeze()` の 1 か所だけにし、利用者識別子が空なら凍結しない。`DropReportTest.利用者識別子が空のうちは凍結せず、決まってから凍結する`

## R21. 読めない行の件数がメモリだけにあり、立て直しで消える
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: SF H1。行は退避先と `.acked` に永続するが、件数は `AtomicInteger` にしか入らない
- kind: technical
- 処置: fixed D6 — 件数は退避先の行数から取り、報告に足せた行数を小さなファイルに書く。`LocationServiceTest.読めない行は見回りで報告に数えられ、立て直しても二重に数えない`

## R22. 報告の下書き（`DropLedger`）の失敗経路に試験が無い
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/DropReportTest.kt
- 根拠: TA。「報告を置き場に書けない」「下書きのファイルが読めない」「数えを 0 に戻してから報告を積む」の 3 経路
- kind: technical
- 処置: fixed 7.2 — R10 / R15 / R16 の試験で 3 経路とも置いた

## R23. 下書きファイルが読めないと、次の保存で上書きして消す
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: SF H5（R10 と同じ）
- kind: technical
- 処置: fixed D6 — R10 の処置

## R24. 登録簿に無いソースで断ったとき、サーバのログに残らない
- 成果物: crates/server/src/drops.rs
- 根拠: SF M11。他の拒否は `tracing::warn!` を出すが `UnknownSource` だけ出さない
- kind: technical
- 処置: fixed D7 — `drop_unknown_source` を件数も値も載せずに出す

## R25. 3 種類の置き場の読めない行を、すべて位置の記録として報告する
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: SF M1。生存信号や破棄の報告の行が読めなくても `c01-location` の読めない行 1 件になる
- kind: technical
- 処置: rejected: 端末の 3 つの置き場はどれも `c01-location` の 1 ソースだけを持つ（`LOGICAL_SOURCE`）ので、ソースの付け違いは起きない。読めなくなった破棄の報告が抱えていた件数は、読めない以上だれにも分からず、「読めなかった 1 件（範囲なし）」として残すのが C9 の「捨てるより印を付ける」の限度。ST09 が端末に 2 本目のソースを足すときに置き場をソースごとに分ける（D5 の反転条件と同じ所）

## R26. 置き場の行を消した後、下書きを保存するまでの間が無防備
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt / Retention.kt / DropReport.kt
- 根拠: SF C4。`.acked` に印を書いてから下書きに足し、保存は全区切りを回った後。保存の失敗も戻り値を見ていない
- kind: technical
- 処置: fixed D4 — 区切りごとに `DropLedger.record` で下書きへ足して保存し、保存できたときだけ印を付ける。印を書けなければ足した分を戻す。`RetentionTest.破棄の報告の下書きを保存できなければ記録を捨てない` / `SegmentStoreTest.証拠を書けなければ行を消さない`

## R27. 同じ識別子の行がある区切りは消えずに残り、2 GB の勘定が膨らむ
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt
- 根拠: TA。取り込みのやり直しで同じ識別子の行が 2 本できると、`.acked` のバイト数が 1 本ぶんしか増えず、区切りが消えない。JSON として読めるが記録として不正な行でも鍵が食い違う
- kind: technical
- 処置: fixed D1 — 行の鍵を行番号にし、区切りを終わりまで読んだときの行数で消すかを決める。`SegmentStoreTest.同じ識別子の行が 2 本あっても両方取り除いて区切りを消す`

## R28. `clear()` がファイルに書けないと、次の起動で同じ分を別の報告として出す
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: SF M4。メモリは 0 になるがファイルは元の値のまま。次の起動で新しい識別子と作成時刻で組み直すので、受け手の冪等では防げない
- kind: technical
- 処置: rejected: 起きるのは、直前に `ledger.writeFailed` の下書きの保存（tmp への書き込みと rename）が通った直後に、4 KiB の既存ブロックの上書きが失敗するときに限る。結果は**多く数える**側（証拠が消えるのではない）で、D5 の「上書きだけで伸びない」ファイルに識別子を持たせる変更（枠を削る）と引き換えにするほどではない。多く数えた件数は破棄の報告の行に残るので、後から突き合わせられる

## R29. 古い 1 本の JSONL の取り込みを、前景に上がる前に main で 1 件ずつ書いている
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / LegacyOutbox.kt
- 根拠: CR #4。1 行ごとに経過の時計の保存・追記の開き直しが走り、数万行で前景に上がる期限（10 秒）を越えて、起動で落ちては立て直す輪になる
- kind: technical
- 処置: fixed D1 — `startForeground` を置き場を開く前に移し、取り込みは 500 件ずつまとめて、積んだ時点の経過を 1 つにして書く

## R30. 出来事の時刻が読めない記録は、書けなかった数えにも報告にも入らない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: SF M5。`failed` の `?.let` と `lost` の `?: return`
- kind: technical
- 処置: fixed D18 — `lost` は出来事の時刻が読めなくても `record` に渡し、範囲を持たない報告に数える。`failed` 側は、端末の記録の出来事の時刻は `LocationFix` が `Instant.toString()` で組むので読めない値が作られない（数えの枠は時間に置くため、読めない値は置けない）

## R31. 送信の契機と見回りの `runCatching` が、あらゆる例外を種別名だけで握りつぶす
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt / LocationService.kt
- 根拠: SF M7。`OutOfMemoryError` も続行し、`Retention.enforce` が毎回投げても `retention_crashed` がログに積もるだけ
- kind: technical
- 処置: rejected: 刻みの中の例外を受け止めるのは ST02 の design D26 / review R30 の規律（`scheduleWithFixedDelay` は投げると以後の実行を黙って打ち切り、前景通知を出したまま送信も生存信号も止まる）。種別名はログに出ており、生存信号は ST02 の取得率で「動いているが取れていない」を別に持つ。上限が効かない状態は 2 GB の側の端末の空きで気付かれず残りうるが、それは例外の握り方ではなく見回りの不具合の話で、見回りの各段は単体と配線の試験（R3 / R12）で固定した

## R32. 生存信号と破棄の報告の置き場は、上限 1 万件を超えた分をログも無しに捨てる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Outbox.kt
- 根拠: SF H3。`writeFailures = null` の置き場では、`MAX_UNWRITTEN` を超えた分を取り出して捨てるだけ
- kind: technical
- 処置: fixed D18 — メモリから手放すのは、失ったことを数える口を持つ置き場（記録）だけにした

## R33. バイト数を読んだ文字列から数えるので CRLF や壊れたバイトでずれる／捨てた理由が読めない時刻で置き換わる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt / Retention.kt
- 根拠: SF LOW 2 件
- kind: technical
- 処置: fixed D1 — 行をバイトのまま読み、バイト数を実際のバイトで数えるようにした。理由の置き換わりは R30 と同じ（端末の記録の出来事の時刻は常に読める形で組まれる）

## R34. 固定長ファイルに書けないとき、メモリの数えは報告にならず、`usable = false` は二度と戻らない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt / design.md（D5）
- 根拠: SF H4。design D5 は「メモリにだけ数え、次の送信で報告にする」と書いているが、報告にするのは起動時だけ
- kind: technical
- 処置: fixed D5 — 書くたびにファイルの作り直しを試みる。「次の送信で報告にする」は design から外した（理由を D5 に書いた: メモリの数えが指す記録はまだメモリにあって送れるので、報告にすると届く記録を破棄とも数える）

## R35. `.acked` の読み直しの失敗・書きかけの印・区切りの削除の失敗で、取り除いた行が戻ってくる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt
- 根拠: SF M2。`loadAcked` の `IOException` を空として扱う / `.acked` の追記に改行の確認が無い / `.jsonl` を消せずに `.acked` だけ消える
- kind: technical
- 処置: fixed D1 — 読めない `.acked` は区切りを読めなかったものとして扱い、`.acked` の追記も末尾の改行を見直し、区切りを消せたときだけ `.acked` を消す。`SegmentStoreTest.acked の書きかけの行があっても次の印は読める`

## R36. 取り込みが途中で止まると、読めない行を毎起動退避して件数が膨らむ
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LegacyOutbox.kt
- 根拠: SF M3（書けなかった数えの二重は R7）
- kind: technical
- 処置: fixed D6 — 読めない行は一時ファイルに溜め、取り込みが最後まで通ったときだけ退避先へ移す。`SegmentMigrationTest.途中で止まった取り込みは読めない行を退避しない`

## R37. 経過の時計のファイルが壊れると、90 日の上限と 83 日の知らせが止まる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/AgeClock.kt
- 根拠: SF M6。`writeText` は原子的でなく、`Outbox.add` のたびに書かれる
- kind: technical
- 処置: fixed D2 — tmp に書いてから差し替える

## R38. 区切りのファイル一覧が取れないと、既存の未送信が見えないまま連番 0 から書く
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt
- 根拠: SF M9。`listFiles()` の null を `orEmpty()` で黙って進む
- kind: technical
- 処置: fixed D1 — 一覧が取れないうちは書かず（書けなかった記録として扱う）、次の操作で取り直す。ログに `outbox_list_failed`

## R39. 知らせが出せなくても印を付けるので、権限を戻しても鳴らない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt
- 根拠: SF M8。権限が無くて出せなかったときも「鳴らした」印を書く / `mark.delete()` の戻り値を見ていない
- kind: daily
- 処置: fixed D11 仮 — 出せなかったら印を付けず、ログは 1 度だけ。印を消せなければログに残す。`RetentionNotifierTest.出せなかった知らせは権限を戻した後に鳴る`

## R40. 区切りを読めないと、その区切りを飛ばして新しい記録から捨てる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SegmentStore.kt
- 根拠: SF M10（R2 と同じ）
- kind: technical
- 処置: fixed D3 — R2 の処置

## R41. 「読めない行は捨てずに退避される」の試験が、UTF-8 として正しい行でしか作られていない
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/UnreadableLineTest.kt
- 根拠: TA。文字列で読んで書き戻すので、電源断で多バイト文字の途中が切れた行では「1 バイトも変わらずに」が成り立たない
- kind: technical
- 処置: fixed D6 — 行をバイトのまま読み、バイトのまま退避する。`UnreadableLineTest.UTF-8 として不正なバイト列の行も 1 バイトも変えずに退避する`

## R42. 「生存信号／破棄の報告は上限を超えても捨てられない」は構造上落ちない
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/RetentionTest.kt / RetentionNotifierTest.kt
- 根拠: TA。`Retention` は記録の置き場しか受け取らないので、守るべきは `LocationService` で置き場を取り違えないこと
- kind: technical
- 処置: fixed 6.2 — `LocationServiceTest.設定が無い端末でも積む契機の見回りで 90 日を超えた記録を捨て、生存信号は捨てない` で本番の配線を見る

## R43. 83 日の通知を鳴らした印が、立て直しをまたいで残るかを見ていない
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/RetentionNotifierTest.kt
- 根拠: TA
- kind: technical
- 処置: fixed 9.1 — `RetentionNotifierTest.鳴らした印は立て直しをまたいで残る`

## R44. 件数 0 の区間が画面に「うち 0 件を破棄」と出る
- 成果物: web/src/coverage.ts / crates/server/src/coverage/tests/drops.rs
- 根拠: TA（R6 と同じ）
- kind: conflict
- 処置: fixed D19 仮 — R6 の処置。`drop-detail.test.tsx` に件数 0 の区間の試験を足した

## R45. 経過の時計の起動回数の分岐と、ファイルが壊れた場合に試験が無い
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/ElapsedClockTest.kt
- 根拠: TA。`FakeDeviceClock.reboot()` は必ず単調時計を戻すので、起動回数の比較を消しても緑
- kind: technical
- 処置: fixed 6.1 — `ElapsedClockTest.起動回数が変わったら単調時計が進んでいても再起動として扱う`。壊れにくくする処置は R37

## R46. 「破棄の一覧は出ない」の試験が `CoverageGrid` だけを描いている
- 成果物: web/src/__tests__/drop-detail.test.tsx
- 根拠: TA。`App.tsx` の階層に一覧を足しても落ちない
- kind: technical
- 処置: rejected: 破棄の件数と区間を運ぶのは `DayCell` だけで、`App.tsx` は `SourceCoverage` を `CoverageGrid` と達成の表（`AchievementPanel`、破棄の欄を読まない）に渡すだけ。一覧を足すには新しい部品と応答の読みが要り、その変更は `one-scroll.test.tsx`（`App` を描いて高さの予算を積む。`git diff` で書き換えていないことを tasks 4.2 が見る）に行が増えて落ちる

## R47. サーバの稼働状況で、破棄の利用者ごとの分離・引き継ぎの鎖・件数の上限に試験が無い
- 成果物: crates/server/src/coverage/tests/drops.rs / crates/server/src/drops.rs
- 根拠: TA
- kind: technical
- 処置: fixed 3.2 — `day_cell_dropped_is_per_user` / `day_cell_dropped_follows_the_chain` と、`drop_report_validate_rejects_zero_count` に `i32::MAX + 1` を足した

## R48. 「1 時間を超えて離れたら別の報告」の境界を固定していない
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/DropReportTest.kt
- 根拠: TA（R9 と同じ）
- kind: technical
- 処置: fixed D4 — R9 の処置

## R49. 1 時間の圏外の計測テストは `LocationService` を使わず部品を手で組んでいる
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/RetentionInstrumentedTest.kt
- 根拠: TA / code-verify R3 の一部
- kind: technical
- 処置: rejected: 1 時間の圏外を本物のサービスで作るには 1 時間の実時間（送信の刻みは 5 分の実時計）が要り、エミュレータの試験の時間に入らない。送信の配線（`Drainer` に 3 つの送り手と見回りを渡す）は `LocationServiceTest` の `scheduler.fire()` の試験 2 本が本番の `LocationService` で見ており、サービスを起こす段（90 日ぶん・1.9 GB）は同じ計測テストが本物のサービスで見ている
