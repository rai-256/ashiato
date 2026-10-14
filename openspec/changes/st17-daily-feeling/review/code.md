# ST17 下流のコードレビュー

## final review（c2e7f08..eabe072）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package は
`.superpowers/sdd/tasks/review-c2e7f08f..eabe072f.diff`（54 commits）。全 Task の ledger（deferred-minor / parked / Ruling の行は 0 件）と、
全 Task の指摘の記録の Minor / ⚠️ を指した。判定は **With fixes**（Critical 0 / Important 3 / Minor 多数）。
reviewer の実測: `cargo test -p ashiato-server st17_` 109 passed、vitest の ST17 の 4 ファイル 79 passed。e2e と gradle は evidence.jsonl に頼った。
各 Task の ⚠️ は R1 の 11.3 を除いて reviewer が確かめて解消した（T1 の地域の検査・T2/T3 の紐づけ先の検査・T5 の台帳の錠・T6 の places の件数・
T7 の上限 2000・T8/T9 の CI の build・T10 の吸収の試験・T12 の 12.2 の証跡）。

申し送り（`docs/handoff/ST17.md`）は開始時から増えていない（最後の変更は fec431d7 / 479b0a07）。ST33・ST38 はまだ archive されていないので、
C6（登録簿の種類）と ST33 C1（`NO_COORD_SOURCES`）は ST17 の側では足せない（PR 本文の「未処置の申し送り」に挙げる）。
`git merge-tree --write-tree HEAD origin/main`（origin/main は 19 commits 先）は衝突なし。

## R1. 11.3 の計測テストが証跡なしで `[x]` になっている —— `AT` の定義の結果ファイル名が実物と合わない
- 成果物: openspec/changes/st17-daily-feeling/tasks.md:29-30, 205-208 / evidence.jsonl
- 根拠: evidence.jsonl に item 11.3 の行が 0 件（11.1 / 11.2 だけ）。`AT` の glob `TEST-*FeelingReminderInstrumentedTest*.xml` は、計測テストの結果の実名
  `TEST-ashiato-api35(AVD) - 15-_app-.xml`（結果ファイルは端末ごとで、クラス名を持たない）に一致しない（Task 11 の ⚠️ と報告の懸念 1）。計画側の欠陥
- kind: technical
- 処置: （fix の後に書く）

## R2. 1 日の画面で、前の日から続く滞在の行に気分が出ない
- 成果物: web/src/DayFeeling.tsx（`useDayFeelings` が `from=date&to=date` だけを読む）・web/src/DayView.tsx:305
- 根拠: 記録は滞在の始まりの日に数える（subjective-log spec:422・design D7）。1 日の画面は前の日に始まった滞在も行に出すが、`stayFeelingText` はその日の `stays` だけを探す。
  browsing-views spec:72「その滞在に紐づく最新の主観の記録…を添える」に反し、主観入力の画面（design:219 は 30 日分から滞在の識別子で引く）と表示が食い違う。例: 偽データ 2026-09-11
- kind: technical
- 処置: （fix の後に書く）

## R3. `seed.sh` の冪等の検査が、確認バッチで人間が窓の中に 1 回書くと二度と通らない
- 成果物: tools/seed.sh（`in_window`）
- 根拠: 検査は「窓（09-04〜10-03）の中の `s01-feeling` の行の数 == seed が入れた件数」。主観の行は消せない（錠）ので、窓の中に 1 件書いた DB では `seed.sh normal` が永久に落ち、
  `stack.sh:103` は警告で先へ進むので以後の読み戻しの検査（未記入 9 など）も効かない
- kind: technical
- 処置: （fix の後に書く）

## R4. 届かなかった記録の送り直しが通った後、次に欄を離れると同じひとことをもう一度送る（Task 8 F2）
- 成果物: web/src/FeelView.tsx（`post` が受理のときに `sentNote.current` を更新しない）
- 根拠: 主観の行は消せないので、重複は永久に残る
- kind: technical
- 処置: （fix の後に書く）

## R5. 押された縁の e2e の題は「快–不快のボタン」だが、滞在の行のボタンを測っている
- 成果物: web/e2e/feel-limits.spec.ts:160-171
- 根拠: e78f1f0f で滞在の行の押されたボタンに切り替えた。`page.clock` を 2026-09-15 の夜に置けば日の快–不快のボタンが押された状態で出るので、題のとおりに測れる
- kind: technical
- 処置: （fix の後に書く）

## R6. 再送のとき紐づけ先の検査を飛ばす経路（`feelings::is_resend`）に試験が無く、design に決定も無い（Task 3 F1）
- 成果物: crates/server/src/lib.rs:1259
- kind: technical
- 処置: （fix の後に書く）

## R7. `load_response` が壊れた行を跡を残さずに捨てる（Task 4 F1）
- 成果物: crates/server/src/feelings.rs（`load_response` の `filter_map`）
- kind: technical
- 処置: （fix の後に書く）

## R8. 2030 年を過ぎると前提が崩れる試験と e2e がある
- 成果物: crates/server/src/feelings_read_tests.rs:824 ほか e2e の「これから」の日
- 根拠: 2030-01-10 を実際の now と比べている
- kind: technical
- 処置: （fix の後に書く）

## R9. 制御文字を含むひとことが、画面で「長すぎる」の文で出る（Task 2 F4）
- 成果物: crates/server/src/feelings.rs:150 付近（`invalid_note`）・web/src/FeelView.tsx の文の選び方
- 根拠: 制御文字は `invalid_note` で断られ、画面は `invalid_note` を一律に「長すぎる」と読む。spec の表にこの条件は無い
- kind: daily
- 処置: （fix の後に書く）

## R10. 最初の記録が未来の日だけだと、今日が「使い始める前」になる
- 成果物: crates/server/src/feelings.rs（`first_day`）
- kind: technical
- 処置: rejected: reviewer 自身が「spec の字面どおり」と書いている（使い始めの日 = 最初の記録の日）。未来の日だけに書いて今日を見るのは、Q5 で許した「未来の日に書く」の端で、表示が「使い始める前」になっても失うものは無い。変えるなら spec の変更で、計算し直しで戻る

## R11. 携帯の通知の時刻の問い合わせのタイムアウトが接続と読み出しで 10 秒ずつ（合計約 20 秒）、D10 は 10 秒。D10 のファイル表も実物とずれている（Task 11 F2 / F3）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/FeelingReminderSource.kt:45-46・design.md D10
- kind: technical
- 処置: （fix の後に書く）

## R12. `LocationService` が起動するたびに、書いた日でもサーバへ通知の時刻を問い合わせる
- 成果物: collector-android の `nextReminderOnStart`
- kind: technical
- 処置: rejected: 書いた日は端末に「通知した日」として残らない（D11 で端末に残すのは日付と時刻だけ、書いたかはサーバが持つ）ので、問い合わせは正しさのために要る。費用は起動ごとに GET 1 本で、通知は出ない（書いた日の判断はサーバの `recorded`）

## R13. 画面は通知の時刻を nil の UUID で書き、携帯は `Config.userId` で読む —— `tools/record-env.sh` の乱数の UUID の環境では携帯に届かない
- 成果物: web/src（通知の時刻の設定）・collector-android `Config.userId`・tools/record-env.sh
- 根拠: 既定（`.env.example`）ではどちらも nil で一致する。食い違うのは録画の試行の環境だけ
- kind: technical
- 処置: （fix の後に書く）

## R14. そのほかの Minor（名前付き色を見張る網が狭い（Task 12 F2）・時刻の列が秒を持てる（Task 1 F5）・`messages.savedNoteOnly` が未使用・`linkStyle` の重複（Task 12 F3））
- 成果物: web/src/__tests__（色の網）・migrations（`core.feeling_reminder` の時刻の列）・web/src/messages.ts:378・web/src/surface.ts
- kind: technical
- 処置: （fix の後に書く）
