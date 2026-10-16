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

## code-verify（24fd09cb）

席: code-verify（申告と実態のずれ）。HEAD 24fd09cb。最後にコードが変わった commit 9ed520fa から HEAD までの差分は
`openspec/changes/st17-daily-feeling/` の 5 ファイルだけ（`git diff --stat 9ed520fa HEAD`）で、コードの木は証跡の木と同じ。
DB はこの worktree の port（`tools/db.sh`・55517）で立て、`tools/db-roles.sh` の後に走らせた。

### 申告: 24/24 `[x]`・検証コマンド一覧と実測

| 項目 | 検証コマンド（略記は tasks.md の定義） | 実測（HEAD） | 一致 |
|---|---|---|---|
| 1.1 | `story_deps.py included ST17 ST25 && test -f deletion.rs && grep stay_absorbed` | rc=0（`[OK] ST25 は archive 済み`） | 一致 |
| 1.2 | `tools/check-migrations.sh`・`CT st17_lock_` | rc=0・`cargo test -p ashiato-server st17_` 111 passed のうち `st17_lock_` 18 本 | 一致 |
| 1.3 | `tools/check-immutable.sh` | rc=0（主観の節 17 行） | 一致 |
| 2.1 / 2.2 / 2.3 | `CT st17_ingest_` / `_nonce_` / `_notcov_` | 21 / 2 / 1 本、失敗 0 | 一致 |
| 3.1 / 3.2 | `CT st17_reject_shape_` / `_target_`・`check-openapi.sh` | 17 / 10 本、失敗 0・openapi rc=0 | 一致 |
| 4.1 / 4.2 | `CT st17_read_rules_` / `_api_`・`check-openapi.sh` | 17 / 20 本、失敗 0・rc=0 | 一致 |
| 5.1 | `CT st17_remind_`・`check-openapi.sh` | 8 本、失敗 0・rc=0 | 一致 |
| 6.1 | `tools/seed.sh normal && tools/seed.sh normal` | rc=0（「主観を 28 件入れ、畳まれて増えていない…未記入 9 / 使い始める前 2 / 記録あり 19」） | 一致 |
| 6.2 | `scripts/quiet-run smoke -- tools/smoke.sh` | rc=0（`== 45. 主観を 1 件入れて、読み出しと通知の判断で読む` → `OK feeling ingest + read + reminder`、`縦串 OK`） | 一致 |
| 7.1 | `VT feelings.test.ts`・`npm run lint` | 10 passed・lint rc=0 | 一致 |
| 8.1 / 9.1 / 10.1 | `VT FeelView` / `FeelViewParts` / `DayFeeling` | 27 / 28 / 18 passed | 一致 |
| 8.2 / 9.2 / 10.2 / 12.1(e2e) | `ET feel` / `feel-parts` / `day-feeling`・`day-timeline`・`day-stays` / `feel-limits` | 6 ファイルを 1 回で 74 passed・rc=0（21 / 15 / 15・14・2 / 7） | 一致 |
| 11.1 / 11.2 | `GT FeelingReminderPolicyTest` / `ReceiverTest`・全単体 | rc=0（10 / 17 本、失敗 0。全 XML で失敗 0） | 一致 |
| **11.3** | `AT FeelingReminderInstrumentedTest` | `tools/android-emulator.sh` rc=0（1032 秒）**だが** `find … -name 'TEST-*FeelingReminderInstrumentedTest*.xml'` は 0 件 → **rc=1**。残った XML は 2 段目（`ForegroundServiceType` / `PermissionDenied` の 2 本）だけ | **不一致**（R1。QR1 の決定は出たが `tools/android-emulator.sh`・tasks.md 11.3 はどちらも未変更。下の注） |
| 12.2 | `check_scenarios.py`・`check-log-private.sh`・`check-openapi.sh`・`--list` の検査 | どれも rc=0（`scenarios: OK`、担保あり 1176/1176） | 一致 |

**注（11.3）**: クラスだけを絞って走らせると（`connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=dev.ashiato.collector.FeelingReminderInstrumentedTest`）
rc=0・`tests="3" failures="0" errors="0" skipped="0"`（`aRecordedDayPostsNothing` / `anUnrecordedDayPostsOneNotificationAndTappingOpensTheFeelAddress` /
`aRebootAfterTheReminderTimeRestoresTheAlarmAndPostsTheSameDay`）。**中身は HEAD で通る。成立しないのは検証コマンドのほう**で、R1 と同じ事柄なので新しい R は立てない。
QR1 の答え（24fd09cb）の後も 11.3 は旧コマンドのまま `[x]`・証跡 0 件。

### 手 1: 固定値の再計算
- ST17 の試験に内容の鍵・hash の直書きは無い（`grep -rnE '"[0-9a-f]{64}"' crates/server/src/feelings*_tests.rs web/src/__tests__/feelings.test.ts` 0 件）
- DB の主観の行 17 件の `content_hash` を python（`hashlib.sha256`、各欄を 8 バイトの長さ（big endian）＋本体、出来事の時刻はマイクロ秒の i64）で計算し直して **17/17 一致**
- 128 bit の base64url は 22 文字・NFD の「か＋゛」は NFC で 1 コードポイント・絵文字 2,000 個は 2,000 コードポイント（UTF-16 では 4,000）—— 試験と design の値に一致
- `raw` の列型は `text`（`information_schema`）。`st17_ingest_raw_is_kept_byte_for_byte` は `raw.as_bytes()` どうしを比べていて、spec の「バイト単位」と階層が合う

### 手 2: ガードを壊す
- `migrations/202610141000_subjective_log.sql` の写しで主観の錠を 1 つずつ作らなくした移行を、この worktree の DB に当てて写しの `check-immutable.sh` を走らせた:
  即時の錠なし rc=1（NG 14）/ COMMIT の門なし rc=1（NG 9）/ 削除の錠なし rc=1（NG 9）。原本を当て直して rc=0 に戻した。**効いている**
- `check-log-private.sh` は**欄の名前**しか見ない → R19
- `check_scenarios.py` は e2e と jsdom を区別しない → 画面の Scenario を自分で `web/e2e` に grep した → R21

### 手 3: Scenario と test
`python3 scripts/check_scenarios.py . st17-daily-feeling` rc=0（担保なし 0）。印の先を見て階層がずれていたのは R16（乱数）・R21（色）。
`主観の原文が 1 バイトも変わらずに残る`（bytes 比較・列は text）・`押された状態と未記入の破線は 3:1 以上`（QP1 の後の縁）・`日付の無いアドレスは夜中も暦の今日を開く`（00:30）は主張の階層で観測している。

### 手 4: 本人の決定・仮決めの値を書き換える（写し `/tmp/st17mut` で 1 つずつ。`cargo test -p ashiato-server st17_` / vitest 4 本 / gradle `*FeelingReminder*`）

| 変えたもの | 結果 |
|---|---|
| `NOTE_MAX_CHARS` 2000→2001（rs / ts） | 落ちる（rs 3 本・ts 8 本） |
| `RANGE_MAX_DAYS` 62→63 / `ABSORB_HOPS_MAX` 16→15・17 | 落ちる（2 本 / 1 本・1 本） |
| `DEFAULT_REMINDER_AT` "21:00"→"20:00"（rs）・`DEFAULT_SENSITIVITY` 2→1 | 落ちる（2 本 / 1 本）。Kotlin の 21:00・12:00・15 分は `FeelingReminderPolicyTest:21-23` が名指し |
| `CALENDAR_DAYS` 30→31 | 落ちる（3 本） |
| Q9: `#/feel` の今日を 4 時区切りにする | 落ちる（jsdom 1 本。e2e も 00:30 の印あり） |
| `SCALE_MAX` 2→3 | 落ちる（3 本） |
| **`SCALE_MIN` -2→-3** | **全部通る（111 passed）** → R17 |
| **前/後の `written_at < target_end` → `<=`** | **全部通る** → R18 |
| **`NONCE_MIN_CHARS` 22→21** | **全部通る** → R16 |
| **携帯の `TIMEOUT_MS` 10_000→60_000** | **全部通る（gradle rc=0、10 + 17 本）** → R20 |

### 手 5: `[x]` と実体
上の表のとおり。`[x]` で検証が成立しないのは 11.3 だけ（R1）。`CT` の名前はどれも 1 本以上ある（`-- --list` ではなく実行の行で数えた）。

### 手 6: 隙間
該当なし（確かめた範囲: 端末の地域が東京でないときの記録 —— `written_tz: "Europe/Paris"`・`+01:00` の原文を `/ingest` に送って `accepted: true`、`GET /feelings` で `after` として読めた /
通知の権限が無いとき —— `FeelingNotification.kt:47-48` が `POST_NOTIFICATIONS` と `areNotificationsEnabled()` を見る /
画面の送信が届かないとき —— `FeelView.tsx` の `post` は応答が読めない・例外のどちらも `pending` に残して同じ原文で送り直す。タブを閉じたときに消えるのは design の Non-Goals「PWA・端末での未送信の保持」で決めた代償 /
`GET /feelings` の読み出しの記録 —— `access_log.rs:110` は router 全体に掛かる middleware で、経路ごとの列挙ではない）。

## R16. spec の「乱数が 128 bit に満たない記録は断る」を、サーバは「22 文字以上の英数字」としか見ていない —— 96 bit の乱数の行が 28 件入っている
- 成果物: crates/server/src/feelings.rs:24-25,98-106 / crates/server/src/feelings_reject_tests.rs:225-227 / tools/seed.sh:512,669 / specs/subjective-log/spec.md:102,229,368
- 根拠: 写しで `NONCE_MIN_CHARS` を 22→21 にしても `cargo test -p ashiato-server st17_` は 111 passed（試験の短い乱数は 11 文字の `"AAAAAAAAAAA"` = 64 bit だけで、境界を撃たない）。
  DB の主観の 740 行のうち 28 行（seed）の乱数は `^[0-9a-f]{24}$`（例 `abd3a9a5933a53bca8cddae2`。16 進 24 文字 = 96 bit。`uuid5(...).hex[:24]` で版の 4 bit も固定）で、`malformed_feeling` にならずに受理されている。
  サーバは文字の数と文字種しか見ないので、文字種の狭い文字列は 128 bit 未満でも通る。spec は「128 bit に満たない」を拒否の条件に書いている（主張は情報量、検査は文字数）
- kind: technical
- 提案: サーバは文字列から情報量を測れない（16 進 24 文字も base64url として復号すれば 18 バイトになる）ので、spec の条件をサーバが観測できる形（「base64url で 22 文字に満たない」）に書き直し、128 bit は画面の側（`feelings.test.ts`）の主張に置く。
  境界の試験（21 文字は断る・22 文字は通る）を足す。seed の乱数を 16 バイトの base64url にする
- 処置: fixed 3.1 —— 境界の試験 `st17_reject_shape_nonce_boundary`（21 文字は `malformed_feeling`、22 文字は受理。`NONCE_MIN_CHARS == 22` も名指し）を足した。写しで 22→21 にすると落ちる（実測）。spec の「128 bit」は design D4 のとおり 2 か所で担保する —— サーバが観測できるのは「base64url の 22 文字以上」、128 bit の乱数を作ることは画面の `feelings.test.ts`。seed の 16 進 24 文字の乱数は直さない: seed の行は uuid5 の固定の id で、既に入った DB の主観の行は錠で書き換えられないので、原文を変えると既存の DB で seed が通らなくなる（偽データで、サーバの検査は通る長さ）

## R17. 尺度の下限 -2 を固定する試験が無い —— `SCALE_MIN` を -3 にしても全部通る
- 成果物: crates/server/src/feelings.rs:37 / crates/server/src/feelings_reject_tests.rs:95 / specs/subjective-log/spec.md:12,105,127-130
- 根拠: 写しで `const SCALE_MIN: i64 = -3;` にして `cargo test -p ashiato-server st17_` は rc=0・111 passed。範囲外の試験は `valence(3)` だけ（`feelings_reject_tests.rs:95`）。`SCALE_MAX` を 3 にしたときは 3 本落ちる。
  快–不快 -3 / 元気さ -3 の記録が入っても誰も気づかない。主観の行は錠で書き換えられない（入った -3 は台帳つきの消去でしか消えない）
- kind: technical
- 提案: `st17_reject_shape_` に快–不快 -3・元気さ -3（と +3）の試験を足し、`invalid_scale` で断られることを固定する
- 処置: fixed 3.1 —— `st17_reject_shape_valence_below_the_minimum`（快–不快 -3）・`st17_reject_shape_energy_out_of_range`（元気さ ±3）・`st17_reject_shape_scale_ends_are_accepted`（両方の尺度の -2 / +2 は受理）。写しで `SCALE_MIN` を -3 にすると 2 本落ちる（実測）

## R18. 前/後の境界（記入の日時 = 対象の終わり）を固定する試験が無い —— `<` を `<=` にしても全部通る
- 成果物: crates/server/src/feelings.rs:439 / crates/server/src/feelings_read_tests.rs:123-125 / specs/subjective-log/spec.md:254,287-290
- 根拠: 写しで `if written_at < target_end {` を `<=` にして `cargo test -p ashiato-server st17_` は rc=0・111 passed。いちばん近い試験は 00:00:30（`feelings_read_tests.rs:125`）で、ちょうど 00:00:00 を撃たない。
  spec:254 は「対象の終わりより前なら前、それ以外なら後」（= ちょうどは後）。本人の Q5「前/後は時刻から機械で決める」の式そのもの。読むたびに計算するので記録は失われない。画面の判定（`web/src/feelings.ts` の「いま書けば付く前/後」）もいまは同じ `<` だが、こちらも境界を撃つ試験が無いので、片方だけ変わっても見つからない
- kind: technical
- 提案: `phase_of_day("2026-10-07", "2026-10-08T00:00:00+09:00") == After` と 1 ミリ秒前が `Before` の試験を Rust と vitest の両方に 1 本ずつ
- 処置: fixed 4.1 —— `st17_read_rules_exactly_at_the_end_is_after`（翌日 00:00:00 ちょうどは後、23:59:59.999 は前）。写しで `<` を `<=` にすると落ちる（実測）。画面の側は既に `feelings.test.ts` の「前/後」が `14:59:59Z` → 前・`15:00:00Z` → 後（= JST 0:00 ちょうど）を撃っている（reviewer の「vitest にも無い」は事実と違った）

## R19. `check-log-private.sh` は許された欄の**名前**しか見ず、値に何を入れても通す —— ひとことを `count =` で出す行が rc=0
- 成果物: tools/check-log-private.sh（`ALLOWED`）/ crates/server/src/feelings.rs / tasks.md:42（Global Constraints「主観の値・ひとことをログに出さない… `tools/check-log-private.sh` rc=0」）
- 根拠: 写しの `feelings.rs:707` の後に `tracing::info!(count = note.as_deref().unwrap_or(""), kind = "feeling_read", "読んだ");` を植えて `tools/check-log-private.sh` → **rc=0（`check-log-private: OK`）**。
  同じ値を `"読んだ {}"` で埋め込んだ行は rc=1 で止まる。ST17 の制約は「ログに出さない」を、この検査の rc=0 で担保したことにしている。いまの木に該当の行は無い（`grep -n 'tracing::' crates/server/src/feelings.rs` は 721 行の `dropped` の件数 1 か所だけ。携帯の `Feeling*.kt` の `Log.` は `Telemetry` の中継と `// log-ok:` つきの例外の型名だけ）が、一度出たログは消せない（`exported`。外へ出たものは戻らない）
- kind: technical
- 提案: 許した欄の値を、識別子の名前が欄の名前と同じとき（`dropped` の省略形）・数値のリテラル・`.len()` / `.count()` などの件数の式に限る。ST17 だけで閉じない（検査は製造準備の道具）なら、次の Story か `fix/` の handoff に積む
- 処置: followup ST17 —— いまの木の ST17 のコードに該当の行は無く（`feelings.rs` の tracing は `dropped` の件数だけ。`check-log-private.sh` OK）、検査の値の側を絞るのは製造準備の道具の全体（約 60 か所の書き方）の線引きになるので、ST17 の merge 後に `fix/` で拾う。案と根拠は `docs/handoff/ST17.md`

## R20. 携帯の問い合わせの打ち切り「合計 10 秒」（D10。R11 の処置）を固定する試験が無い —— `TIMEOUT_MS` を 60 秒にしても全部通る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/FeelingReminderSource.kt:38,76 / collector-android/app/src/test/kotlin/dev/ashiato/collector/FeelingReminderReceiverTest.kt / design.md D10
- 根拠: 写しで `const val TIMEOUT_MS = 60_000` にして `gradlew :app:testDebugUnitTest --tests '*FeelingReminder*'` → rc=0（`FeelingReminderPolicyTest` 10・`FeelingReminderReceiverTest` 17、失敗 0）。
  R11 の試験は `timeoutMs` に 1 秒を差し込んで打ち切りの仕組みを見るが、既定の値は名指しされていない（`grep -rn TIMEOUT_MS collector-android/app/src/test` 0 件）。Global Constraints の「仮決めの値は定数にして試験が名指しで固定する」の外に落ちている
- kind: technical
- 提案: `assertEquals(10_000, HttpReminderSource.TIMEOUT_MS)` を `FeelingReminderPolicyTest` の定数の試験（21-23 行）に並べる
- 処置: fixed 11.1 —— `HttpReminderSource.TIMEOUT_MS` を試験から見えるようにし（companion を internal に。`JSON` は private のまま）、`FeelingReminderPolicyTest.namedConstantsArePinned` に `assertEquals(10_000, HttpReminderSource.TIMEOUT_MS)` を足した。写しで 60_000 にすると落ちる（実測）

## R21. 12.1 の `主観入力の画面は確定した色だけを使う` に e2e の印が無い —— Global Constraints は「12.1 に挙げたものは全部 web/e2e」と書く
- 成果物: tasks.md:17-18,215-218 / web/src/__tests__/FeelViewParts.test.tsx:442 / web/e2e/feel-limits.spec.ts
- 根拠: tasks 8.1・9.1・10.1・12.1 が挙げる画面の Scenario 66 本を 1 本ずつ `grep -rlF "Scenario: <名前>" web/e2e` した結果、e2e に印が無いのはこの 1 本だけ（印は jsdom の `FeelViewParts.test.tsx:442` だけ）。`check_scenarios.py` は e2e と jsdom を区別しないので rc=0 で通る。
  ただし Scenario の WHEN は「画面のコードに書かれた色を集める」で、観測の階層はソースのほう（jsdom の静的検査が主張に合っている）。制約の文と Scenario のどちらかがずれている
- kind: conflict
- 提案: Global Constraints の「12.1 は全部 e2e」からこの 1 本を外す旨を tasks に書くか、e2e で S-3 の要素の計算済みの色が `tokens.ts` の値の集合に入ることを測って印を置く
- 処置: fixed D23 仮 —— Scenario の WHEN は「画面のコードに書かれた色を集める」で観測の階層はソースなので、jsdom の静的な検査を担保とし e2e の印は置かない（ST21 `places-colors.test.ts`・個人属性 `master-view-limits.test.tsx` の同じ Scenario と同じ置き場）。反転条件は design D23。Ruling は Task 12 の ledger
