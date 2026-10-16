# ST17 毎日 30 秒で「その日どう感じたか」を残す（下流・実装）

change: `openspec/changes/st17-daily-feeling/`。capability `subjective-log`（新規）と `browsing-views`（S-2 の頭と滞在の行）。

## 深掘りに積んだ未決（A）

**なし**（`deep.md` の未回答 0）。下流で立てた問いは 2 つで、どちらも本人が答えた:

- **QP1**（Task 12 の fix round。premise）: 押された快–不快のボタンの 3:1 は「面」でなく「縁」で測る → spec を直して e2e の印を戻した
- **QR1**（whole-branch review R1。premise）: 11.3 の検証コマンドが成立しない → `tools/android-emulator.sh` が 1 段目の結果の XML を
  `collector-android/app/build/outputs/androidTest-results-stage1/` に残し、11.3 はクラス名（`classname="dev.ashiato.collector.FeelingReminderInstrumentedTest"`）で数える。
  **この PR で入れたのは道具の側だけ**。tasks.md 11.3 の検証コマンドの差し替えは `scripts/plan_fix.py` で行うが、旧コマンドが `declared()` に拾われない（`AT` の展開先 `find …` が `CMD_LIKE` に当たらない）ため
  harness2 側で `plan_fix.py` を直してから（本人が QR1 で承認済み）。それまで 11.3 は `[x]` のまま証跡 0 件（下の「残したもの」）

## 仮決め（B）と Ruling

design の（仮）: D3 既定の感度はコードの分岐 / D6 読み出しの紐づけ先の解決と最新の決め方 / D7 前/後と日の状態 / D10 携帯の通知は AlarmManager の不正確な時刻（問い合わせは合計 10 秒で打ち切る）/
D12 押したら 1 件・ひとことは欄を離れたら / D13 S-3 の滞在の並び / D14 暦と S-2 の頭 / D19 主観のソースは途絶の判定に入れない / D20 S-2 の滞在の行に気分を 1 行 /
D22 ひとことの断りの文を送ったひとことで選び分ける / **D23 「確定した色だけを使う」はソースの静的な検査で担保し e2e の印を置かない**（code-verify R21。この段で足した）。反転条件は各 D に。

ledger の Ruling（全 Task の `progress.md` と report の Ruling の節の全部）:

- Task 8: サーバが `invalid_note` で断ったとき、画面は送ったひとことに制御文字（改行とタブを除く Cc）があれば「使えない文字（制御文字）が入っています」、無ければ「長すぎます」と出す（design D22（仮））— サーバは長すぎるのと制御文字を同じ種別で断り、画面が一律に「長すぎる」と読んでいたため（whole-branch review R9）。種別を増やすとサーバ・OpenAPI・ST19 の口まで触る — 間違っていたときの費用は文の差し替えだけ（表示のみ。記録は何も失われない）
- Task 11: 12:00 の読み直しも通知の時刻に達していれば通常の起床と同じく判断する（読み直しだけして日を取りこぼさないため）。設計の「読み直して予約し直すだけ」との差は、新しい時刻が 12:00 以前のときだけ
- Task 11: `LocationService` 起動の契機は網に触れず予約だけ張る（時刻を過ぎて未通知なら今すぐの窓）。既存の `LocationService` 試験への影響をなくすため
- Task 12: 12.1 の `主観入力の画面は確定した色だけを使う` は e2e の印を置かず、jsdom の静的な検査（ソースの色の直書き）で担保する（design D23（仮））— Scenario の WHEN が「画面のコードに書かれた色を集める」で観測の階層がソースのため。ST21・個人属性の同じ Scenario も静的な検査（code-verify R21）。間違っていたときの費用は試験の置き場の移し替えだけ

parked: なし。

## Task

12 Task・25 項目、全部 `[x]`（グラフの `mark`）。`ci` / `human` の残りは無し。

- **残したもの: 11.3 の証跡**。計測テストそのものは通る（この段で `tools/android-emulator.sh` を走らせ直した結果は下）が、tasks.md 11.3 の旧コマンドは結果のファイル名に合わず、`scripts/verify-run 11.3` は rc=2（本文に検証コマンドが無い）。QR1 の決定どおり、harness2 の `plan_fix.py` の直しの後に検証コマンドを差し替えて取り直す

## 検証（`evidence.jsonl`。`scripts/verify-run` が走らせて記録した最新の行）

1.1 / 1.2（`check-migrations.sh`・`CT st17_lock_`）/ 1.3 / 2.1 / 2.2 / 2.3 / 3.1 / 3.2（＋`check-openapi.sh`）/ 4.1 / 4.2（＋`check-openapi.sh`）/ 5.1（＋`check-openapi.sh`）/
6.1（`tools/seed.sh normal` を 2 回）/ 6.2（`tools/smoke.sh`）/ 7.1（`VT feelings.test.ts`・`npm run lint`）/ 8.1 / 8.2 / 9.1 / 9.2 / 10.1 / 10.2 / 11.1 / 11.2 / 12.1（e2e と jsdom）/
12.2（`check_scenarios.py`・`check-log-private.sh`・`check-openapi.sh`・`--list` の検査）—— どれも PASS（exit 0）。
code-verify の処置の後に 3.1・4.1・11.1 を取り直した（`CT st17_reject_shape_` / `CT st17_read_rules_` / `GT FeelingReminderPolicyTest`）。

11.3（手で走らせた結果。証跡ではない）: 2a884adb ＋ この PR の `tools/android-emulator.sh` で rc=0（955 秒）。1 段目の XML（`androidTest-results-stage1/`）は `tests="15" failures="0" errors="0"` で、`classname="dev.ashiato.collector.FeelingReminderInstrumentedTest"` の testcase が 3 本（`aRecordedDayPostsNothing` / `anUnrecordedDayPostsOneNotificationAndTappingOpensTheFeelAddress` / `aRebootAfterTheReminderTimeRestoresTheAlarmAndPostsTheSameDay`）

## レビュー

- Task ごと: 12 Task の task reviewer（指摘は `.superpowers/sdd/st17-task-<N>/task-<N>-findings.md`）
- whole-branch review: R1〜R15。fixed 10・rejected 4（R10 未来の日だけの使い始め / R12 起動ごとの問い合わせ / R13 nil の UUID は録画の台本の側 / R15 色の網と秒の列）・escalated 1（R1 → QR1）
- code-verify: R16〜R21。fixed 5（R16 乱数の 22 文字の境界 / R17 尺度の下限 / R18 前/後の境界 / R20 携帯の打ち切り 10 秒 / R21 D23 仮）・followup 1（R19）。
  R16〜R18・R20 は、写しで値を壊すと新しい試験が落ちることを確かめた

## 申し送り（未処置）

- **R19 → `docs/handoff/ST17.md`**: `tools/check-log-private.sh` は許した欄の名前しか見ず、値に何を入れても通す。ST17 のコードに該当の行は無い。ST17 の merge 後に `fix/` で検査を絞る
- **QR1 の harness2 側**: `plan_fix.py` が `declared()` の空の項目を受けられるようにする → その後 11.3 の検証コマンドを差し替える
