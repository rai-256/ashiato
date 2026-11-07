# CH10 コードレビュー

## final review（ca8e1148..2066eec1）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）がブランチ全体の差分（review package `.superpowers/sdd/tasks/review-ca8e1148b..2066eec19.diff`、26 commit）と全 Task の指摘の記録（Minor と ⚠️）を見た。判定は「With fixes」。reviewer が走らせた: `scripts/quiet-run unit -- ./tools/test-android-emulator.sh` rc=0（23 Scenario）/ `--self-test` rc=0 / `python3 scripts/check_scenarios.py . ch10-emulator-start-timeout` OK。⚠️ の CI の ubuntu runner は job の中身から緑の見込み（実際は PR の rust job で見る）。

## R1. 採寸の台本は上限の値が正の整数でないと、エミュレータを起こしてから落ち、起こしたものを残す
- 成果物: tools/usage-volume.sh:139-145 / tools/emulator-start.sh:89・:112-113
- 根拠: 採寸の台本は値を事前に確かめず、`emu_start` の後の `emu_wait_device` / `emu_wait_boot` の `emu_positive_int … || return 1` で初めて落ちる。`emu_fail` を通らないので `EMU_PID` を止めない。偽の木で `ASHIATO_EMULATOR_DEVICE_TIMEOUT=0` → rc=2、run.log に `emulator -avd ashiato-api35 -wipe-data …`、偽のエミュレータが 2 本残る（reviewer が再現）。spec の 3 つ目の Requirement（正の整数でなければ起こさずに、採寸は rc=2）と 7 つ目（失敗では自分の起こしたものを止める）に反する。Task 2 F7（検証が 2 か所）が不具合になったもの。BOOT_ATTEMPTS / ADB_CALL_TIMEOUT も同じく起こしてから落ちる
- kind: technical
- 提案: 3 つの値の検証を部品の 1 関数にまとめ、2 つの台本とも起こす前に呼ぶ。`emu_wait_*` の中の検証の失敗も自分の PID を止める経路を通す。試験に採寸の bad-value の場合を足す（Scenario の印は足さない —— spec の Scenario は 23 本のまま）
- 処置: fixed D8 （35a74316a: `emu_check_env`（tools/emulator-start.sh）に 3 つの値の検証をまとめ、2 つの台本とも AVD を作る前に呼ぶ（計測 `|| exit 1`・採寸 `|| return 2`）。`emu_wait_*` の中の検証の失敗も `emu_stop_own` を通す。試験 usage-bad-value（Scenario 外）を足し、呼び出しを外すと落ちることを確かめた。scoped re-review で ADDRESSED）

## R2. 待っている途中で台本が止められると、起こしたエミュレータが残る（base より後退）
- 成果物: tools/android-emulator.sh:30-34（`cleanup`）/ tools/usage-volume.sh の `cleanup_main`
- 根拠: 端末の待ち・起動の完了の待ちの最中に SIGTERM / SIGINT で止まると EXIT の trap は走るが、`EMU_CLEANUP_WITH_ADB=0` なので何もせず、背景のエミュレータが残る（reviewer が bash で再現）。base の `cleanup` は無条件に `adb -s emulator-5554 emu kill` を送っていた。残ると次の回が「同じ AVD が動いている」で止まる
- kind: technical
- 提案: `emu_fail` の止める部分（子孫に TERM、5 秒後に KILL）を `emu_stop_own` に切り出し、`cleanup` / `cleanup_main` で `EMU_CLEANUP_WITH_ADB=0` かつ `EMU_PID` が生きていれば呼ぶ
- 処置: fixed D13 （35a74316a: `emu_stop_own` を切り出し、計測の `cleanup`（`EMU_CLEANUP_WITH_ADB` が 1 でないとき）と採寸の `cleanup_main`（`started` が空のとき）から呼ぶ。`emu_pid_alive` は EMU_PID 未設定で偽。試験 term-cleanup（Scenario 外）を足し、呼び出しを外すと偽のエミュレータが 2 本残って落ちることを確かめた。scoped re-review で ADDRESSED）

## R3. 同じ AVD の確かめの `emu avd name` の fallback が CR を落とさず、`while read` の中の adb がループの stdin を共有する
- 成果物: tools/android-emulator.sh:39-53
- 根拠: Task 3 F2・F3 の未処置。console は CRLF で返すのが一般的で、`real-proof.sh:167-174` は `tr -d '\r'` の後を記録しているので本番の式で fallback が当たるかは確かめていない。`adb shell` は stdin を転送するので、2 台目以降の serial を読まれうる
- kind: technical
- 提案: fallback の結果に `tr -d '\r'` を挟む。ループの中の adb に `</dev/null`
- 処置: fixed D3 （35a74316a: tools/android-emulator.sh:51-52 の adb 2 か所に `</dev/null`、fallback に `tr -d '\r'`。scoped re-review で ADDRESSED）

## R4. 端末の待ちの上限を回数で数えている
- 成果物: tools/emulator-start.sh:89-101
- 根拠: Task 2 F5 の未処置。`timeout 5 adb get-state` が遅いと最悪で上限の約 6 倍待つ（D2 の「120 秒」を超える）
- kind: technical
- 提案: `SECONDS` で締め切りを決める
- 処置: fixed D2 （35a74316a: `emu_wait_device` を `deadline=$((SECONDS + limit))` で決め、adb 1 回の待ちを残り（1〜5 秒）に切り詰める。最悪でも上限＋約 6 秒。scoped re-review で ADDRESSED）

## R5. 同じ AVD の確かめの待ちが D2 の上限の外に足される
- 成果物: tools/android-emulator.sh:39-53
- 根拠: Task 6 F8。名前を答えない端末 1 台ごとに最大 10〜15 秒。端末が複数並ぶのは稀で、待ちには上限がある（無期限ではない）
- kind: technical
- 処置: rejected: spec の求めは「無期限に待たない」で、この待ちは端末 1 台ごとに有界（tools/android-emulator.sh:49-52 の 5 回 × getprop 1 秒＋fallback 1 秒＋sleep 1 秒）。端末が複数並ぶのは稀で、D2 の「端末が現れるまで」の上限とは別の段の待ち。直さずに park（ledger の Ruling）

## R6. 偽の adb が、端末が複数あるときの `-s` 無しの `get-state` / `shell` を通してしまう
- 成果物: tools/test-android-emulator.sh:114 / design.md D4
- 根拠: 本物は `error: more than one device/emulator` になる。other-device（実機＋emulator-5556）の場合は本物では 120 秒待って「端末が adb に現れない」の誤った文言で落ちる。D4（待つ相手の取り違えは直さない）の範囲の既存の制約だが、試験の rc=0 がそれを隠す
- kind: technical
- 提案: D4 に「他の端末が繋がっていると端末の待ちで誤った文言で落ちる」ことを一言残す
- 処置: fixed D4 仮 （35a74316a: design.md:70 の D4 に、他の端末が繋がっていると端末の待ちが上限まで待って誤った文言で落ちること・偽の adb がこれを再現しないことを 1 項足した。D4 は上流の仮のまま）

## R7. 部品・試験の古い Minor の残り
- 成果物: tools/emulator-start.sh / tools/usage-volume.sh / tools/test-android-emulator.sh
- 根拠: Task 2 F9（`${EMU_PID:?}` で文言無しに落ちる）、Task 4 F3（`emu_check_kvm` が AVD を作った後、usage-volume.sh:139）、Task 1 F7（:413 で `tools/test-usage-volume.sh` を相対で呼ぶ・写しが未使用）、Task 2 F10（:207 が `.harness2/scripts` を cwd から引く）、Task 2 F8（:186 の届かない早期 return）、Task 1 F6 / F8・Task 2 F6・Task 3 F4 / F5・Task 4 F2 / F4（試験の判定の分け方・数え方・self-test の網羅・出力の混ざり・証拠の弱さ）
- kind: technical
- 処置: fixed D16 （35a74316a: Task 2 F9 は `[ -n "${EMU_PID:-}" ] || return 1`、Task 4 F3 は採寸の `emu_check_kvm` を AVD 作成の前へ、Task 1 F7 は `"$ROOT/tools/test-usage-volume.sh"`、Task 2 F10 は `$ROOT` 起点、Task 2 F8 は届かない return を削除。scoped re-review で ADDRESSED。残り（判定の分け方・数え方・self-test の網羅・出力の混ざり・使われない写し）は R10 に分けた）

## R8. real-proof.sh の残り
- 成果物: openspec/changes/ch10-emulator-start-timeout/real-proof.sh:93・:97・:125・:129
- 根拠: Task 6 F13（`!= Z*` が文字列の比較で zombie を除けない。偽の FAIL の向きだけ）、F14（`script_pid=emu_pid=` で emu_pid が local にならない）、F9（pgrep がホスト全体を数える。baseline で緩和済み）
- kind: technical
- 処置: fixed 6.1 （35a74316a: F13 は `case … in Z*)`、F14 は `local … script_pid= emu_pid=`。本物は走らせず `bash -n` のみ。F9 は直さない —— real-proof.sh:97 で走らせる前の `pgrep -f -- '[-]avd ashiato-api35'` を baseline に取り、それを除いて数えるので、並走の分は数えない。残る誤りは偽の FAIL の向きだけ）

## R9. tasks.md 5.1 の検証の行に、切れた旧コマンドの後半が残っている
- 成果物: openspec/changes/ch10-emulator-start-timeout/tasks.md:91 / plan-corrections.md:7
- 根拠: Task 5 F1・F2。plan_fix.py の置き換えが旧コマンドの前半だけを差し替えた。原因はハーネス（plan_fix.py）
- kind: technical
- 処置: rejected: 検証として拾われる入口は `./openspec/changes/ch10-emulator-start-timeout/check-5.1.sh` の 1 つで、evidence.jsonl 2026-11-07T05:36:32+00:00 / 05:36:50 に PASS がある。行の残りは plan_fix.py（ハーネス）の置き換えの不具合で、tasks.md は plan_fix 以外で直さない（凍結）。ハーネス側の直しとして PR 本文に挙げる

## R10. 試験の判定の分け方・数え方・self-test の網羅の残り（R7 から分けた Minor）
- 成果物: tools/test-android-emulator.sh / tools/usage-volume.sh
- 根拠: Task 1 F6（6 段をディレクトリの数だけで見る）・F8（args と stages が同じ assert_normal を 2 回）・Task 2 F6（no-named-kill が assert_late_die の流用）・Task 3 F4（other-device が起動の回数を数えない）・F5（self-test に同じ AVD の確かめを壊す場合が無い）・Task 4 F2（`== …` が採寸の stdout に混ざる）・F4（usage-reuse の証拠が弱い）、scoped re-review の残り（tools/test-android-emulator.sh:67 で test-usage-volume.sh の写しを置くが使わない）
- kind: technical
- 処置: rejected: どれも Scenario の振る舞いを外す誤りではない。`scripts/quiet-run unit -- ./tools/test-android-emulator.sh` rc=0（23 Scenario）と `--self-test` rc=0 で、self-test は `-no-snapshot` と KEEP を壊した写しで本物の assert が落ちることを確かめている。no-named-kill は spec の WHEN より強い場合で試している。採寸の `== …` の行を読むのは sample の経路だけ（tools/usage-volume.sh:140）。試験の読みやすさの改善として park（ledger の Ruling）

## scoped re-review（2066eec1..35a74316）

R1〜R4・R6・R7（対象の 5 件）・R8（F13・F14）は ADDRESSED。fix による新しい破損は無い。対象外にしたもの（R5・R8 F9・R9・R10）は上の処置のとおり。
