# ST04 実装タスク — 圏外でも記録が失われない

読む順: `deep.md`（**最優先。本人が決めたこと**）→ このファイル → `specs/device-collection/spec.md`
→ `specs/collection-coverage/spec.md` → `design.md` → `docs/stories/ST04.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D16）。**名前は作成時刻 `YYYYMMDDHHMM_drop_reports.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
**`record-envelope` と、取り込み（`/ingest`）・生存信号（`/heartbeat`）の契約の形、PC 側（`collector-windows`）には触らない**（design D14）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` が前提。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 90 日に届かない記録は捨てられない`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。
  印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change が新しく足す Scenario は 81 本**（`device-collection` 51 本 / `collection-coverage` 30 本）。
  MODIFIED で写した既存の Scenario は、既存の印をそのまま生かす（名前を変えていない）。**既存の「一時的な失敗では捨てない」は WHEN が変わった**ので、そのテストの前提（上限を超えていない）を確かめ直す
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで **`CT <絞り込み>`** と書いたものは、
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log` が rc=0 になることを指す。
  **`VT <ファイル>`** は `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/<ファイル> 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log` が rc=0。
  **`AT <クラス>`**（エミュレータの計測テスト）は `./tools/android-emulator.sh` rc=0 の後に
  `grep -l 'dev.ashiato.collector.<クラス>' collector-android/app/build/outputs/androidTest-results/connected/**/TEST-*.xml` が 1 件以上、かつそのファイルに `failures="0"`（CI の `android-instrumented` が緑でも可。そのときは同じ XML を成果物で見る）。
  **`GT <クラス>`** は `bash -o pipefail -c 'cd collector-android && ./gradlew :app:testDebugUnitTest --tests "dev.ashiato.collector.<クラス>" 2>&1 | tee /tmp/gt.log' && grep -q "BUILD SUCCESSFUL" /tmp/gt.log && ls collector-android/app/build/test-results/testDebugUnitTest/TEST-dev.ashiato.collector.<クラス>.xml` が rc=0
- **本人の決定（下流は変えない）**: 上限は送れない理由を問わず全部にかける（Q1）/ 格子に形の印・期間と件数は週の詳細だけ・一覧は作らない（Q3）/
  90 日は積んでから（Q4）/ 7 日前に 1 回鳴らす（Q5）/ 溜まっている間は続けて送る（Q6）
- **D1 / D3 / D5 / D8 / D11 / D12 と、D2 の 30 日・D10 の印の色は（仮）決め。** 反転条件は `design.md`。変えたらその D 番号を書き直す
- **ログに出すのは件数・ソース名・所要時間・エラーの種別だけ**（製造準備 A-2）。捨てた記録の位置・時刻の値は出さない
- **上限の値は `RetentionPolicy` にだけ置く。** 試験だけが差し替え、本番では常に 90 日 / 2 GB / 83 日（design D15）

## 1. 移行（design D7 / D16）

- [x] 1.1 移行 `migrations/YYYYMMDDHHMM_drop_reports.sql` と `.down.sql` を足す —— `core.drop_report`（`user_id` あり）、`core.drop_report_hour`、
  `(user_id, logical_source, content_hash)` の一意索引、`drop_report (logical_source, range_start)` の索引、
  `CHECK (range_end IS NULL OR range_start < range_end)` / `CHECK (count > 0)`、2 表の UPDATE / DELETE / TRUNCATE を拒むトリガ。
  **当て直せる形**（`IF NOT EXISTS`）。`MIGRATIONS` 配列の末尾に足す。
  検証: `tools/check-migrations.sh` rc=0、`CT drop_reports_migration_applies_twice`
- [x] 1.2 `tools/check-immutable.sh` に 2 表を足す（全列の UPDATE と DELETE が拒まれる）。
  Scenario: `格納された破棄の報告は書き換えられない`。検証: `tools/check-immutable.sh` rc=0

## 2. 破棄の報告の受け口（design D7）

- [x] 2.1 `crates/server/src/drops.rs` に `DropReport` の形と検査（`heartbeat.rs` に揃える）。
  Scenario: `時間ごとの件数が合わない報告は受け付けられない` / `範囲が空の報告は受け付けられない` / `読めなかった行の報告は範囲なしで受け付けられる` /
  `端末識別子の無い報告は受け付けられない` / `知らない理由の報告は受け付けられない` / `件数が 0 の報告は受け付けられない` /
  `範囲を欠く 90 日の報告は受け付けられない` / `範囲の外の時間に件数を置いた報告は受け付けられない`。
  検証: `CT drop_report_validate`
- [x] 2.2 `POST /drops` を足す（配列でも 1 件でも / 1 件ごとの結果 / 1 件も受け付けなければ 400 / 応答に値を含めない / 冪等キーは `logical_source` + `raw`）。
  Scenario: `破棄の報告を複数件まとめて受け取る` / `一部が不正でも正しい破棄の報告は受け付けられる` /
  `破棄の報告がすべて拒否されたときだけ要求が拒否される` / `破棄の報告の拒否の応答に受け取った値が含まれない` /
  `同じ破棄の報告を 2 回送っても 1 つ` / `破棄の報告の原文が 1 バイトも変わらずに残る` / `破棄の報告の受信時刻が時刻のまま残る` /
  `1 件だけの破棄も範囲として残る` / `破棄が端末と理由と時間ごとの件数で残る`。
  **既存の `crates/server/src/coverage/tests/spans.rs` の `drop_is_stored_with_count`（Scenario: `破棄が期間と件数で残る`）は、`core.coverage_span` へ直接 INSERT している。**
  ST04 の後は書き手のいない表なので、**同じ印を `/drops` 経由で入れて読み戻す結合テストにも置く**（直接 INSERT のテストは `coverage_span` の読み手の試験として残す）。
  検証: `CT drops_api`、`grep -rn 'Scenario: 破棄が期間と件数で残る' crates/server/src | grep -v spans.rs` が 1 件以上
- [x] 2.3 `docs/openapi.json` と `docs/collector-contract.md` に `/drops` の形を足す。検証: `tools/check-openapi.sh` rc=0、
  `grep -c '"/drops"' docs/openapi.json` が 1 以上

## 3. 稼働状況（design D8 / D9）

- [x] 3.1 `coverage.rs` の `dropped_full` を、同じソースの破棄の範囲（`drop_report` ＋ `coverage_span` の `dropped`）を**端が接するもの・重なるものでつないだ範囲**で判定する形にする。
  Scenario: `端が接する 2 本の破棄は合わせて丸ごと判定される` / `離れた 2 本の破棄は合わせない`。
  検証: `CT dropped_ranges_merge`、既存の `CT coverage` がすべて通る
- [x] 3.2 `DayCell` に `dropped_count` と `dropped_ranges`（`from` / `to` は `Asia/Tokyo` の `HH:MM`、日の終わりは `24:00`）を足す。`unreadable` の報告は日に入れない。
  Scenario: `日をまたぐ破棄は日ごとに切られる` / `範囲を持たない破棄は日の件数に入らない` / `同じ報告を 2 回受けても日の件数は 1 回ぶん`。
  検証: `CT day_cell_dropped`（10:00〜13:00 の 180 件 → `dropped_count = 180`・`dropped_ranges = [{from:"10:00",to:"13:00",count:180}]`、
  丸ごとの日 → 状態 `dropped` と件数）、`tools/check-openapi.sh` rc=0

## 4. 画面（design D10）

- [x] 4.1 `web/src/coverage.ts` の `DayCell` に 2 欄を足し、`CoverageGrid.tsx` の `WeekRow` に印（右下の三角。`dropped_count > 0` かつ状態が `dropped` でない日だけ）を付ける。
  印の色は段ごと（92 / 40 の上は 9、9 の上は `TEXT.muted`）。
  Scenario: `一部を破棄した日のセルに形の印が付く` / `丸ごと覆う破棄の日には印が付かない` / `破棄の無い日には印が付かない` / `印はどの段の上でも 3:1 以上`。
  検証: `VT drop-mark.test.tsx`（`contrast.ts` で比を数える試験を含む）
- [x] 4.2 `WeekDetail` に「うち N 件を破棄（from〜to）」と、丸ごとの日の件数を足す。一覧は作らない。
  Scenario: `週を選ぶと破棄の件数と時刻が文字で出る` / `丸ごと覆う破棄の日は件数が添えられる` / `破棄の一覧は出ない`。
  検証: `VT drop-detail.test.tsx`、`VT one-scroll.test.tsx`、`git diff --exit-code origin/main -- web/src/__tests__/one-scroll.test.tsx` rc=0（予算の試験を書き換えずに通す）
- [x] 4.3 検証: `cd web && npx tsc -b && npm run lint && npm run build` rc=0、`tools/check-boundaries.sh` rc=0

## 5. 端末の置き場（design D1 / D6）

- [x] 5.1 区切りファイルの置き場（`SegmentStore`）を作る —— 最新の区切りへの追記、1 MB で次の区切り、見出しと行ごとの `enq_ms`、
  先頭から `MAX_BATCH` ぶんだけ読む、`.acked` への追記で取り除き、区切りの全件が済んだら消す。**全件をメモリに載せない・全件を書き直さない**。
  Scenario: `読み戻しでメモリに載る件数は 1 回に載る件数を超えない` / `送れた分を取り除いても残りは書き直されない`。
  検証: `GT SegmentStoreTest`（1,000,001 バイトで区切りが 2 本になる / 読み戻しで組み立てた件数を数える / 取り除く前後で残りの区切りファイルのハッシュが同じ）
- [x] 5.2 既存の `outbox.jsonl` / `heartbeat.jsonl` を初回の起動で区切りへ取り込み、取り込めたら元を消す。検証: `GT SegmentMigrationTest`
- [x] 5.3 読めない行を `outbox/unreadable.jsonl` へ退避し、既存の `.unreadable.<時刻>` は数えて残す（D6）。
  Scenario: `読めない行は捨てずに退避される` / `読めない行の件数が報告される` / `退避した行は上限を超えても消されない`。検証: `GT UnreadableLineTest`
- [x] 5.4 `Outbox` / `LocationService` を区切りの置き場に差し替える。既存の `OutboxTest` / `OutboxStoreTest` / `HeartbeatOutboxTest` を新しい置き場で通す
  （未送信が停止と再開をまたいで残る Scenario の印は既存のまま）。検証: `cd collector-android && ./gradlew :app:testDebugUnitTest` rc=0

## 6. 上限（design D2 / D3 / D13）

- [x] 6.1 `RetentionPolicy`（90 日 / 2 GB / 83 日。試験だけが差し替える）と、経過の数え方（同じ起動は単調時計、起動をまたぐときは 30 日を超える空白を数えない）。
  Scenario: `時計が先へ飛んでも 90 日の側では捨てない` / `再起動をまたぐ長い空白は 90 日に数えない` /
  `再起動をまたぐ 30 日以内の空白は 90 日に数える` / `再起動をまたいで時計が戻っても経過は減らない`。検証: `GT ElapsedClockTest`
- [x] 6.2 90 日と 2 GB で行単位に古い順に捨てる。生存信号と破棄の報告は対象外。送れない理由を見ない。
  Scenario: `端末に積んでから 90 日を超えた記録は捨てられる` / `90 日に届かない記録は捨てられない` / `2 GB を超えると積んだ順の古いものから捨てられる` /
  `到達できても断られ続ける未送信にも上限がかかる` / `出来事の時刻が古くても積んだばかりの記録は捨てられない` /
  `生存信号は上限を超えても捨てられない` / `破棄の報告は上限を超えても捨てられない`。
  検証: `GT RetentionTest`
- [x] 6.3 既存の `SenderTest` の「一時的な失敗では捨てない」を、上限を超えていない前提で書き直して通す。検証: `GT SenderTest`

## 7. 破棄の報告（design D4 / D5）

- [x] 7.1 捨てた記録から破棄の報告を作る —— ソース × 理由ごとに送る前の報告を 1 本、`hourly` は UTC の 1 時間、範囲の終わりは残った最も古い記録（1 時間以内）か最後 + 1 ms、
  1 時間を超えて離れる・戻るなら新しい報告、位置の値を持たない。
  Scenario: `上限で捨てると範囲と件数と理由の報告が積まれる` / `件数は出来事の時刻の 1 時間ごとに数えられる` / `1 件だけ捨てても範囲の終わりは始まりより後` /
  `続けて捨てた範囲は残った記録の時刻で途切れない` / `送る前の報告には続けて起きた破棄が足される` / `破棄の報告に位置の値が含まれない` /
  `理由が違う破棄は別の報告になる` / `出来事の時刻が離れた破棄は別の報告になる` /
  `残った記録が離れていれば範囲の終わりは最後に捨てた記録の直後` / `時間ごとの件数は範囲の内側にある`。
  検証: `GT DropReportTest`
- [x] 7.2 送信に載せる前に凍結し、凍結した報告は書き換えず再送で同じ `raw` を送る（`Sender` を `dropPermanentlyRejected = false` で `/drops` に向ける）。
  Scenario: `送ろうとした報告は書き換えられない` / `送ろうとした後の破棄は新しい報告になる` / `再送した報告は最初と同じ冪等キーを持つ` /
  `断られた破棄の報告も未送信から取り除かれない`。検証: `GT DropReportSendTest`
- [x] 7.3 書けなかった記録の数え（固定長 4 KiB の `outbox/write-failed.bin` に時間ごとの枠。起動時に 0 でなければ `write_failed` の報告、あふれは範囲なしの報告。design D5）。
  Scenario: `置き場に書けなかった記録も報告される` / `書けなかった記録の報告は時間ごとの件数を持つ` / `書けなかった記録が 1 件でも範囲の終わりは始まりより後`。
  検証: `GT WriteFailedTest`（`write-failed.bin` の大きさが書き込みの前後で 4096 バイトのまま、も確かめる）

## 8. 送信（design D12）

- [x] 8.1 溜まっている間は続けて送る（未送信が `MAX_BATCH` を超えて残り、応答が 200 / 400 で、1 件以上を取り除けたら次を呼ぶ。到達できない・それ以外・0 件なら 5 分に戻る。design D12）。
  続けて送る 1 回ごとに生存信号と破棄の報告も送る。
  Scenario: `溜まっている間は間隔を待たずに続けて送る` / `溜まりが 1 回に載る件数以下になったら一定の間隔に戻る` /
  `断られ続ける未送信だけが溜まっていても一定の間隔に戻る` / `記録が溜まっていても生存信号と破棄の報告が送られる` /
  `続けて送る途中で失敗したら一定の間隔に戻る` / `溜まっていないときは一定の間隔のまま`。
  検証: `GT DrainTest`

## 9. 知らせ（design D11）

- [x] 9.1 常駐の通知の本文に未送信の日数（1 日以上のとき）、83 日で `retention` チャネルに 1 回、83 日を下回ったら印を消す。権限が無ければ `retention_alert_blocked` をログへ。
  数えるのは上限の対象になる記録だけ（design D11）。
  Scenario: `常駐の通知に未送信の日数が出る` / `1 日に満たない未送信では日数が出ない` / `上限の 7 日前に音の鳴る通知が出る` /
  `83 日を過ぎても同じ通知は鳴り直さない` / `生存信号だけが古くても鳴らない` / `送り切った後の次の長い圏外ではまた鳴る`。
  検証: `GT RetentionNotifierTest`（Robolectric で通知の中身と回数を数える）

## 10. 完了の判定を機械で確かめる（design D15）

- [ ] 10.1 エミュレータの計測テスト `RetentionInstrumentedTest`: 偽の `Transport` を 1 時間ぶん失敗させてから戻す → 60 件が届き報告が無い。
  Scenario: `1 時間の圏外の記録は全部が届く` / `1 時間の圏外では破棄の報告が作られない`。
  検証: `AT RetentionInstrumentedTest`
- [ ] 10.2 同じテストに、129,600 件の区切りを置いてから `LocationService` を起こす段を足す → 落ちずに起動し、偽のサーバに全件が届き、取得も続く。送り切るまでに生まれた記録が 1 時間以内に送られる。
  Scenario: `90 日ぶんを溜めた端末が起動して送り切る` / `溜まった状態でも取得は続く` / `溜まった分を送っている間に生まれた記録も 1 時間以内に届く`。
  検証: `AT RetentionInstrumentedTest`
- [ ] 10.2b 同じテストに、未送信の記録を 1.9 GB 置いてから `LocationService` を起こす段を足す（エミュレータの空きが足りなければ `-partition-size` を上げる）。
  Scenario: `2 GB に近い量でも収集は起動する`。検証: `AT RetentionInstrumentedTest`
- [x] 10.3 `tools/smoke.sh` に 1 段足す: `/drops` に 180 件（10:00〜13:00）の報告を送り、同じものをもう 1 回送り、`GET /coverage` のその日が `dropped_count = 180` で状態が変わらないことを `jq -e` で見る。
  検証: `tools/smoke.sh` rc=0
- [x] 10.4 `tools/seed.sh normal` に、一部を破棄した日（`c01-location`、`2026-09-07` の 10:00〜13:00・180 件）と、2 本に割れて丸ごと覆う日（`2026-09-05`）の破棄の報告を `/drops` で足す
  （確認バッチの画面で印と文字が見える材料）。`c01-location` は移行 `202609111111_coverage_rebuild.sql` が登録簿に入れてある。
  検証: `tools/seed.sh normal` rc=0 の後、
  `curl -sf -H "authorization: Bearer $API_TOKEN" "http://127.0.0.1:18787/coverage?from=2026-09-01&to=2026-09-08" | jq -e '[.[]|select(.logical_source=="c01-location").days[]|select(.day=="2026-09-07")][0].dropped_count==180'` rc=0
  （**式を直した**（下流）: `GET /coverage` の応答はソースの配列そのもので `.sources` を持たない。`DayCell` の欄は 3.2 のとおり）

## 11. まとめの検査

- [x] 11.1 検証: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` rc=0、
  `cd web && npm run test && npm run lint && npm run build` rc=0、`cd collector-android && ./gradlew :app:assembleDebug :app:testDebugUnitTest` rc=0
- [x] 11.2 検証: `python3 scripts/check_scenarios.py . st04-offline-retention` rc=0（81 本すべてに印か、人間の確認待ち）
- [x] 11.3 検証: `python3 scripts/check_chain.py .` rc=0、`openspec validate st04-offline-retention --strict` rc=0、
  `tools/check-migrations.sh` / `tools/check-openapi.sh` / `tools/check-boundaries.sh` / `tools/check-immutable.sh` がすべて rc=0
- [ ] 11.4 PR 本文に **仮決め（D1 / D2 / D3 / D5 / D8 / D10 / D11 / D12）と反転条件**を列挙する。検証: `gh pr view --json body -q .body | grep -cE "D(1|2|3|5|8|10|11|12)（仮）"` が 8 以上
- [x] 11.5 `docs/handoff/` を読み直す（開始時と PR 前の 2 回）。検証: `ls docs/handoff/ST04.md 2>/dev/null` が空か、あればその各項目に PR 本文で触れている

## 人間の確認待ち

**機械で確かめられないのは「違和感」だけ**（2026-09-14 の決定）。正しさは 10 章までのテストが持つ。
確認バッチ（`/verify`）の手順書が、この Story について「触ってみて違和感は無かったか」を 1 問だけ聞く
（見るもの: `SEED=normal` の稼働状況で、一部を破棄した日の印と、その週を選んだときの「うち 180 件を破棄（10:00〜13:00）」）。
