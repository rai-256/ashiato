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
- kind: premise
- 処置: escalated —— 検証コマンドの直しは plan の訂正なので Task agent も fixer も tasks.md を直せない（`.harness2/docs/flow-gates.md` の「凍結後に検証コマンドそのものが成立しないとき」）。`scripts/verify-run 11.3` は rc=2「本文に検証コマンドが無い」（`AT` の展開先 `find …` が `CMD_LIKE` に当たらず `declared()` が空）で、`plan_fix.py` が要る FAIL の実証も取れない。premise として `deep.md` の QR1（`deep-questions-r13.json`）に積んだ。harness2 側の穴（証跡 0 件の `[x]` を `evidence.py check --story` が通す）も問いに書いた

## R2. 1 日の画面で、前の日から続く滞在の行に気分が出ない
- 成果物: web/src/DayFeeling.tsx（`useDayFeelings` が `from=date&to=date` だけを読む）・web/src/DayView.tsx:305
- 根拠: 記録は滞在の始まりの日に数える（subjective-log spec:422・design D7）。1 日の画面は前の日に始まった滞在も行に出すが、`stayFeelingText` はその日の `stays` だけを探す。
  browsing-views spec:72「その滞在に紐づく最新の主観の記録…を添える」に反し、主観入力の画面（design:219 は 30 日分から滞在の識別子で引く）と表示が食い違う。例: 偽データ 2026-09-11
- kind: technical
- 処置: fixed 10.1 —— 1 日の画面は対象の日で終わる 30 日を読み、行の気分は全部の日の `stays[]` から滞在の識別子で集める。頭の欄はその日の分だけ（be577b38。design D20 を合わせた。jsdom と e2e に 2026-09-11 の前の日から続く滞在を 1 本ずつ。書き方だけ直した合意の試験は `agreed-test-changes.md`）

## R3. `seed.sh` の冪等の検査が、確認バッチで人間が窓の中に 1 回書くと二度と通らない
- 成果物: tools/seed.sh（`in_window`）
- 根拠: 検査は「窓（09-04〜10-03）の中の `s01-feeling` の行の数 == seed が入れた件数」。主観の行は消せない（錠）ので、窓の中に 1 件書いた DB では `seed.sh normal` が永久に落ち、
  `stack.sh:103` は警告で先へ進むので以後の読み戻しの検査（未記入 9 など）も効かない
- kind: technical
- 処置: fixed 6.1 —— 冪等の検査を seed の uuid5 の id の行に絞った。窓に seed 以外の主観が無ければ従来の厳密な読み戻し、あれば下限で読み戻す旨を出す（7c96eb6a。窓に 1 件足した DB で旧 rc=1 / 新 rc=0 を実測）

## R4. 届かなかった記録の送り直しが通った後、次に欄を離れると同じひとことをもう一度送る（Task 8 F2）
- 成果物: web/src/FeelView.tsx（`post` が受理のときに `sentNote.current` を更新しない）
- 根拠: 主観の行は消せないので、重複は永久に残る
- kind: technical
- 処置: fixed 8.1 —— 受理のとき「日」の記録なら `sentNote.current` を更新する（aba12ffe。直す前に落ちる jsdom を 1 本）

## R5. 押された縁の e2e の題は「快–不快のボタン」だが、滞在の行のボタンを測っている
- 成果物: web/e2e/feel-limits.spec.ts:160-171
- 根拠: e78f1f0f で滞在の行の押されたボタンに切り替えた。`page.clock` を 2026-09-15 の夜に置けば日の快–不快のボタンが押された状態で出るので、題のとおりに測れる
- kind: technical
- 処置: fixed 12.1 —— `page.clock` を 2026-09-15 22:00 JST に置き、日の快–不快の `valence-0` が押された状態であることを確かめてからその縁を測る（0caae51e。09-15 の「日」の値を seed.sh の読み戻しに足した）

## R6. 再送のとき紐づけ先の検査を飛ばす経路（`feelings::is_resend`）に試験が無く、design に決定も無い（Task 3 F1）
- 成果物: crates/server/src/lib.rs:1259
- 根拠: `crates/server/src/lib.rs:1259`（`if !feelings::is_resend(...)` の中でだけ紐づけ先を検査する）。`.superpowers/sdd/st17-task-3/task-3-findings.md` F1。`st17_reject_` に再送の試験が無かった（109720ed の木）
- kind: technical
- 処置: fixed D21 —— `st17_reject_target_resend_skips_the_target_check`（1 回目の後に滞在を動かし、同じ原文の再送は畳まれ、新しい原文は断られる）と design D21「再送は紐づけ先の検査をやり直さない」（4195491a）

## R7. `load_response` が壊れた行を跡を残さずに捨てる（Task 4 F1）
- 成果物: crates/server/src/feelings.rs（`load_response` の `filter_map`）
- 根拠: 109720ed の `crates/server/src/feelings.rs:682`（`load_response` の `.filter_map(|(id, event_time, ingested_at, p)| …)` が読めない行を `None` にして件数も残さない）。`.superpowers/sdd/st17-task-4/task-4-findings.md` F1
- kind: technical
- 処置: fixed 4.2 —— 捨てた件数を `tracing::warn!(kind = "feeling_unreadable", dropped)` で出す（値・本文・id は出さない。`tools/check-log-private.sh` OK）。`st17_read_api_unreadable_row_is_dropped_with_a_count_in_the_log`（224da497）

## R8. 2030 年を過ぎると前提が崩れる試験と e2e がある
- 成果物: crates/server/src/feelings_read_tests.rs:824 ほか e2e の「これから」の日
- 根拠: 2030-01-10 を実際の now と比べている
- kind: technical
- 処置: fixed 4.2 —— 実際の now と比べる試験の日を 2100-01-10 に、書き込む e2e の乱数の日を 2100 年以降にした。期待の行は変えていない（224da497 / c14d4412 / 5f2a7e9c）。明示の today を渡す純粋な試験の 2030 は時計に依らないので残した

## R9. 制御文字を含むひとことが、画面で「長すぎる」の文で出る（Task 2 F4）
- 成果物: crates/server/src/feelings.rs:150 付近（`invalid_note`）・web/src/FeelView.tsx の文の選び方
- 根拠: 制御文字は `invalid_note` で断られ、画面は `invalid_note` を一律に「長すぎる」と読む。spec の表にこの条件は無い
- kind: daily
- 処置: fixed D22 仮 —— 画面は `invalid_note` のとき、送ったひとことに制御文字（改行とタブを除く Cc。サーバの `char::is_control` と同じ範囲）があれば `invalid_note_control` の文を出す。サーバの種別は増やさない。反転条件は D22（aba12ffe。Ruling は Task 8 の ledger）

## R10. 最初の記録が未来の日だけだと、今日が「使い始める前」になる
- 成果物: crates/server/src/feelings.rs（`first_day`）
- 根拠: `crates/server/src/feelings.rs:514`（「`first_day` は利用者の生きた主観で最も早い記録の日」）
- kind: technical
- 処置: rejected: `crates/server/src/feelings.rs:514` のとおり使い始めの日 = 最初の記録の日で、reviewer 自身が「spec の字面どおり」と書いている。未来の日だけに書いて今日を見るのは、Q5 で許した「未来の日に書く」の端で、表示が「使い始める前」になっても失うものは無い。変えるなら spec の変更で、計算し直しで戻る

## R11. 携帯の通知の時刻の問い合わせのタイムアウトが接続と読み出しで 10 秒ずつ（合計約 20 秒）、D10 は 10 秒。D10 のファイル表も実物とずれている（Task 11 F2 / F3）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/FeelingReminderSource.kt:45-46・design.md D10
- 根拠: 109720ed の `FeelingReminderSource.kt:45-46`（`connectTimeout = timeoutMs` / `readTimeout = timeoutMs` で各 10 秒）。`.superpowers/sdd/st17-task-11/task-11-findings.md` F2 / F3
- kind: technical
- 処置: fixed D10 —— 期限の見張り（daemon の `Timer`）で接続から読み終わりまで合計 10 秒で切る。Robolectric に 1 バイトずつ流すサーバで 1 秒で打ち切る試験。D10 のファイル表を実物に合わせた（1bc3e553）

## R12. `LocationService` が起動するたびに、書いた日でもサーバへ通知の時刻を問い合わせる
- 成果物: collector-android の `nextReminderOnStart`
- 根拠: `collector-android/app/src/main/kotlin/dev/ashiato/collector/FeelingReminderReceiver.kt:72`（起動ごとに `nextReminderOnStart(now, at, state.lastNotified)` で予約し直す）
- kind: technical
- 処置: rejected: 書いた日は端末に「通知した日」として残らない（D11 で端末に残すのは日付と時刻だけ、書いたかはサーバが持つ）ので、問い合わせは正しさのために要る。費用は起動ごとに GET 1 本で、通知は出ない（書いた日の判断はサーバの `recorded`）

## R13. 画面は通知の時刻を nil の UUID で書き、携帯は `Config.userId` で読む —— `tools/record-env.sh` の乱数の UUID の環境では携帯に届かない
- 成果物: web/src（通知の時刻の設定）・collector-android `Config.userId`・tools/record-env.sh
- 根拠: 既定（`.env.example`）ではどちらも nil で一致する。食い違うのは録画の試行の環境だけ
- kind: technical
- 処置: rejected: 指摘は事実だが ST17 だけの問題ではない —— 画面は全画面で nil の UUID を使う（`web/src/attributes.ts:198`・`places.ts:276` は呼び出し元が `userId` を渡さず、`feelings.ts:242` も同じ）。食い違うのは録画の試行の台本 `tools/record-env.sh:96,112` が携帯の id を乱数にする環境だけで、そこでは ST21（場所）・属性の画面も携帯と別の利用者になる。直すなら録画の台本の側（携帯の id も nil にする）

## R14. 未使用の `messages.savedNoteOnly` と、`linkStyle` の重複（Task 12 F3）
- 成果物: web/src/messages.ts:378・web/src/surface.ts・web/src/FeelView.tsx・web/src/FeelParts.tsx
- 根拠: 109720ed の `web/src/messages.ts:378`（`savedNoteOnly` の参照 0 件）。`.superpowers/sdd/st17-task-12/task-12-findings.md` F3（`FeelView.tsx:430` の `{ ...linkStyle(c), background: "transparent", border: "none" }` が 2 か所）
- kind: technical
- 処置: fixed 12.2 —— 参照 0 件を確かめて `savedNoteOnly` を消し、リンクの見た目のボタンを `surface.ts` の `linkButtonStyle` 1 か所にした（aba12ffe / 9ed520fa）

## R15. 名前付き色を見張る網が狭い（Task 12 F2）・通知の時刻の列が秒を持てる（Task 1 F5）
- 成果物: web/src/__tests__（色の網）・migrations（`core.feeling_reminder` の時刻の列）
- 根拠: `.superpowers/sdd/st17-task-12/task-12-findings.md` F2（`FeelViewParts.test.tsx:452` の網）・`crates/server/src/feelings.rs:753`（「秒は持たない。`21:00:00` は読めない」）
- kind: technical
- 処置: rejected: 色の網は 12.1 の範囲（`FeelView.tsx` / `DayFeeling.tsx` に `tokens.ts` 以外の色の文字列が無い）を固定していて、名前付き色まで広げるのは全画面の lint の仕事（ST17 の画面だけを広げても網の穴は他の画面に残る）。秒の列は、書く口（`POST /feelings/reminder`）が `HH:MM` しか受けないので秒の入った行は入らない。列の型を変える移行は凍結した台帳の書き直しになるので、入口で閉じているいまは変えない

## fix wave と scoped re-review（109720e..a41eaa9）

fix は 1 回（fresh な fixer 1 本に R2〜R9・R11・R14 を渡した。11 commits）。直しの後の最後のコードの commit 9ed520fa に対して
3.1 3.2 4.1 4.2 6.1 6.2 7.1 8.1 8.2 9.1 9.2 10.1 10.2 11.1 11.2 12.1 12.2 を `scripts/verify-run` で走らせ直し、全部 PASS（evidence.jsonl、a41eaa96）。
3.1 / 3.2 / 4.1 の証跡の summary は末尾の別バイナリの「0 passed」を写しているが、ログの本体では lib の試験が 1 本以上通っている（例: 3.1 は 17 passed）。

scoped re-review（`re-review-prompt.md`）: 対象の 10 件すべて ADDRESSED、新しい Critical / Important の壊れなし。Minor 4 件は直さずに残す:
`feelings.test.ts` の `});  it(` が 1 行に連なる書式 / Rust 側の 2030→2100 は `agreed-test-changes.md` に載っていない（実時計との比較で Scenario の期待ではない）/
R11 の `disconnect()` を別スレッドから呼ぶときの例外の種類（Android は IOException に畳まれる）/ `feelings.rs` の変更後に 1.x・2.x・5.1 の証跡を取り直していない（変更はログの追加だけ）。

残るのは R1（premise。`deep.md` の QR1）だけ。11.3 の計測テストは R1 の答えの後に新しい検証コマンドで走らせ直す。
