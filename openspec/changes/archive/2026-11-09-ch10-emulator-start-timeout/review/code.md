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

## code_verify（独立検証。HEAD 9df54ee09、2026-11-07）

実装者とも final reviewer とも別の席が、作業ツリーで実際に走らせて確かめた。壊すのは `mktemp -d` の写しの木（`tools/` の 6 本と `.harness2` の link）だけで、作業ツリーのコードは触っていない（手 7 の文書の一時的な書き換えは戻し、`git status --porcelain` / `git diff` が前と同じことを cmp で確かめた）。

### 申告と実測

申告: tasks 11/11 が `[x]`、「23 Scenario・CI 全緑の見込み」。

| 項目 | 検証コマンド | 申告 | 実測 |
|---|---|---|---|
| 1.1 / 2.2 / 3.1 / 4.1 | `tools/test-android-emulator.sh` | rc=0 | 一致: rc=0・33 秒・`^Scenario: ` 23 行 |
| 1.2 / 6.2 | `tools/test-android-emulator.sh --self-test` | rc=0 | 一致: rc=0 |
| 2.1 | `bash -n tools/emulator-start.sh` / `bash -n tools/android-emulator.sh` | rc=0 | 一致: rc=0 / rc=0 |
| 2.3 | `ASHIATO_TEST_ONLY=dies ASHIATO_TEST_SCRIPT_REV=11c0c043 … \| grep -q "見張りで打ち切った: dies"` / `ASHIATO_TEST_ONLY=dies …` | rc=0 | 一致: rc=0 / rc=0 |
| 4.1 | `tools/test-usage-volume.sh` | rc=0 | 一致: rc=0 |
| 5.1 | `./openspec/changes/ch10-emulator-start-timeout/check-5.1.sh` | rc=0 | 一致: rc=0 |
| 5.2 | ci.yml の grep / `tools/check-shared-appends.sh` | rc=0 | 一致: rc=0 / rc=0（ci.yml:134-135 に 2 行）。CI の run は未観測（PR・run が無い）。代わりに `.harness2` の無い写し（CI と同じ条件）で本体 rc=0・23 行、self-test rc=0 |
| 6.1 | `scripts/quiet-run real-{dies,timeout,same,ok} -- …/real-proof.sh <case>` | rc=0（最後の実測は 86b5bcb75。final の fix 35a74316a の後は `bash -n` のみ） | HEAD で走らせ直した: dies rc=0（3 秒）/ timeout rc=0（21 秒）/ same-avd rc=0（19 秒）/ ok rc=0（1123 秒。real-proof.txt に ok: pass・ok-accel-lines: 0） |
| 6.2 | `python3 scripts/check_scenarios.py . ch10-emulator-start-timeout` | rc=0 | 一致: `scenarios: OK` |
| archive | `openspec validate ch10-emulator-start-timeout --strict` | rc=0 | 一致: valid |

総合: **一致**（申告の rc に偽りは無い）。ずれは手 3 の「Scenario の AND のうち試験が観測していないもの」にある。

### 手 1: 固定値を独立に再計算する

試験が直書きする起動引数 2 本を、直す前の `11c0c043` の台本から python の正規表現で独立に取り出して比べた。`-avd ashiato-api35 -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot`（計測）と `-avd ashiato-api35 -wipe-data -no-window …`（採寸）の両方が、試験（tools/test-android-emulator.sh の `assert_normal` / `assert_usage_dies`）の値と一字一句一致。試験は実装の出力の写しでなく、直す前の形を固定している。指摘なし。

### 手 2・手 3: ガードを壊す / Scenario と試験の突き合わせ

写しの木で台本を 1 か所ずつ壊し、その Scenario の鍵だけを走らせた（`ASHIATO_TEST_ONLY=<鍵>`）。落ちた（試験が観測している）もの:
`-no-snapshot` を消す（self-test）・KEEP の扱いを消す（self-test）・読み書きの確かめを `-e` だけにする（kvm-denied rc=1）・採寸の `emu_check_kvm` を消す（usage-kvm rc=1）・起動の完了の待ちから PID の確かめを消す（late-die rc=1）・`emu_fail` が `emulator-5554` へ `emu kill` を送る（no-named-kill rc=1）・同じ AVD の判定を常に真にする（other-device rc=1）・末尾を 41 行にする（tail40 rc=1）・`emu_fail` と計測の EXIT trap の両方から `emu_stop_own` を消す（keep-cleanup rc=1。片方だけ消すと他方が補って rc=0 —— 二重の後片付けで、振る舞いは保たれる）。
対照（コメントを 1 字変える）は rc=0。下の 3 件が、印はあるのに Scenario の主張を観測していないもの。

## R11. kvm-missing の試験は「調べた場所にファイルは作られていない」を見ていない
- 成果物: tools/test-android-emulator.sh:177-189（`assert_kvm_failure`）/ specs/emulator-startup/spec.md の Scenario「開けない場所を渡すと、エミュレータを起こさずに /dev/kvm の文言で止まる」の 3 つ目の AND
- 根拠: 写しの `tools/emulator-start.sh:41` を `local device=…; : > "$device"; chmod 000 "$device"`（調べた場所にファイルを作る）に壊して `ASHIATO_TEST_ONLY=kvm-missing` → rc=0。`assert_kvm_failure` に `[ ! -e "$root/no-kvm" ]` が無い。spec がわざわざ挙げた「調べるだけで作らない」（`exec 3<>` は無い場所を作りうる。今の実装は `[ ! -e ]` を先に見るので作らない）が、どの試験からも固定されていない
- kind: technical
- 提案: `assert_kvm_failure missing` の判定に `[ ! -e "$root/no-kvm" ]` を足す（採寸の `assert_usage_kvm` にも同じ 1 項）
- 処置: fixed D11 （`assert_kvm_failure missing` と `assert_usage_kvm` に `[ ! -e "$root/no-kvm" ]` を足した。写しで `emu_check_kvm` に `: > "$device"` を足すと kvm-missing・usage-kvm とも rc=1、戻すと全体 rc=0）

## R12. bad-value の試験は `0` しか渡さず、spec の `abc` を試していない
- 成果物: tools/test-android-emulator.sh:236（`assert_device_wait bad`）/ spec の Scenario「上限に正の整数でない値を渡すと、起こさずに止まる」
- 根拠: spec の WHEN は「`0` を、次に `abc` を渡して」。試験は `0` の 1 回だけ。写しの `tools/emulator-start.sh:18` の `[[ "$value" =~ ^[1-9][0-9]*$ ]]` を `[ "$value" != 0 ]`（`abc` を通す）に壊して `ASHIATO_TEST_ONLY=bad-value` → rc=0。あわせて THEN の「`error:` で始まり」も見ず、`ASHIATO_EMULATOR_DEVICE_TIMEOUT` の部分一致だけ（`grep -Fq 'ASHIATO_EMULATOR_DEVICE_TIMEOUT'`）
- kind: technical
- 提案: `abc` でもう 1 回走らせ、どちらも `grep -Eq '^error: .*ASHIATO_EMULATOR_DEVICE_TIMEOUT'` と `[ ! -s run.log ]` を見る
- 処置: fixed D8 （bad-value を `0` と `abc` の 2 回にし、どちらも `^error: .*ASHIATO_EMULATOR_DEVICE_TIMEOUT`・run.log 空・rc=1 を見る。写しで正規表現を `[ "$value" != 0 ]` にすると bad-value rc=1）

## R13. dies の試験は emulator.log の置き場と全文を見ていない
- 成果物: tools/test-android-emulator.sh:209-225（`assert_dies`）/ spec の Scenario「起こしたエミュレータが起動直後に終わると、すぐ止まって出力の末尾を出す」の 2 つ目・3 つ目の AND / tools/emulator-start.sh:61・:85
- 根拠: (1) 写しで既定の置き場を `collector-android/app/build/emulator.log`（spec と違う）に替えて `ASHIATO_TEST_ONLY=dies` → rc=0。試験は標準エラーの `emulator.log` の部分一致しか見ない。(2) `emu_fail` の終わりに `rm -f "$log"`（全文を消す）を足して dies → rc=0。「`collector-android/app/build/outputs/emulator.log` に偽のエミュレータの出力の全文がある」を試験は一度も読んでいない。spec の Requirement 2 の「起動のたびに書き直して残す」は、落ちた理由を後から読むためのもの（CH10 の発端は理由がログに残らなかったこと）で、それが消えても緑になる
- kind: technical
- 提案: `assert_dies dies` で、標準エラーに `collector-android/app/build/outputs/emulator.log` の全体の文字列があることと、使い捨ての木の `collector-android/app/build/outputs/emulator.log` に `x86_64 emulation currently requires hardware acceleration!` があることを見る

### 手 4: 本人の決定が試験で固定されているか

写しで値を書き換えて全体を走らせた。
- Q1（2 つの台本が同じ部品を使う）: 採寸の `emu_wait_device` を `adb wait-for-device` に戻す → usage-dies が見張りの 30 秒で落ちる（rc=1）。固定されている
- Q2（120 秒）: `tools/emulator-start.sh:101` の既定を 60 に → device-default rc=1。固定されている
- Q3（同じ AVD なら起こさずに止まる）: `tools/android-emulator.sh:63` の呼び出しを消す → same-avd rc=1。固定されている
- Q4（取り違えは直さない）・Q5（一覧のとおり）: 値を持たない。Q5 の 1〜4・6・7 は手 3 の self-test と各場合が見ている。5（CI の計測テストの job）は ci.yml の diff に `android-emulator-runner` の行の変化が無いことを見た
- 処置: fixed D12 （`assert_dies dies` で標準エラーの `error: エミュレータの出力全文: collector-android/app/build/outputs/emulator.log` と、使い捨ての木のその置き場に偽のエミュレータの出力があることを見る。写しで `emu_fail` に `rm -f "$log"` を足す・既定の置き場を替える、のどちらも dies rc=1）

## R14. spec が「今のまま」と言う既定の 2 つの値は、変えても全部通る
- 成果物: tools/emulator-start.sh:128（`ASHIATO_EMULATOR_BOOT_ATTEMPTS:-180`・`ASHIATO_ADB_CALL_TIMEOUT:-10`）/ spec Requirement 3（「既定 10 秒」「既定 180 回 × 2 秒…は今のまま」）
- 根拠: 写しで 180 → 18 にして全体を走らせる → rc=0。10 → 1000 にして全体 → rc=0。本人の答え（Q1〜Q5）ではないが、spec が明示する既定で、180 は直す前の台本から引き継いだ「壊してはいけない」側の値。D2 の 120 秒のように標準出力に出す行が無いので、どの試験からも見えない
- kind: technical
- 提案: 既定の 2 つを見る場合を 1 つ足す（例: getprop が返らない偽の adb で `ASHIATO_ADB_CALL_TIMEOUT` だけ渡さず、1 回の問い合わせが 10 秒で切られたことを偽の adb の記録の時刻で見る）か、待ち始めの行に 2 つの値を出して grep する

### 手 5: tasks の `[x]` と実体

上の表のとおり、11 項目すべての検証コマンドが実在し rc=0。`cargo test --test` 型の項目は無い。`check-5.1.sh`・`real-proof.sh` は change の中に実在する。指摘なし（5.1 の行に旧コマンドの後半が残るのは final R9 で処置済み）。

### 手 6: 隙間

確かめた範囲: 端末が `offline` のまま（同じ AVD の確かめは `device` だけを見るので起こしに進み、AVD の lock で起動直後に終わる → 起動の途中で終わった、で早く止まる。設計 D3 の範囲）・`emu avd name` が空（同じく起こしに進む）・SIGTERM（final R2 で直り、term-cleanup が見る）・実機の HEAD の再走（上の表）。捨てたものを復元できない型は R13（ログの全文が消えても試験が緑）が最も近い。
- 処置: fixed D15 （`emu_wait_boot` が待ち始めに `== 起動の完了を最大 <回> 回（2 秒ごと。1 回の問い合わせは最大 <秒> 秒）待つ` を出し、device-default がそれを grep する。写しで 180→18・10→1000 のどちらも device-default rc=1）

## R15. 実装中の本物の検証の残りのエミュレータが 2 本、CPU を使い続けている
- 成果物: openspec/changes/ch10-emulator-start-timeout/real-proof.sh（過去の版）
- 根拠: `ps -ef | grep '[q]emu'` → PID 1811997（14:50 開始、CPU 99%・累計 4 時間 19 分、HOME=`.superpowers/sdd/tmp/a/tmp.htxh12r5ZO/home`、引数は計測の台本と同じ `-avd ashiato-api35 … -no-snapshot`）と PID 2718742（17:25 開始、`-port 5668`、HOME=`.superpowers/sdd/tmp/vvud91_mm/…` = evidence.jsonl 2026-11-07T08:25:10 の same-avd FAIL「line 104: serial: unbound variable」の回）。どちらも親は systemd --user、HOME の木はもう無く、`adb devices` には出ない（同じ AVD の確かめでも見えない）。HEAD の real-proof.sh は `SAME_PID=$!` の直後に `trap cleanup_same_avd EXIT` を張り、HEAD で走らせ直した timeout・same-avd は残りを出さなかった（timeout の baseline 外の `-avd` 過程 0 本）。残っているのは直す前の版の残骸
- kind: daily
- 提案: 人間が `kill 1811997 2718742` で止める（この席は止めない）。並走の Story が計測テストを回すとき、CPU を取り合う
- 処置: rejected: 台本の側は直っている —— HEAD の `real-proof.sh` は `SAME_PID=$!` の直後に `trap cleanup_same_avd EXIT` を張り、code-verify が HEAD で走らせ直した timeout・same-avd は残りを出さなかった。残っている 2 本（`ps -o pid,lstart,args -p 1811997,2718742` で 14:50・17:25 開始、HOME はこの worktree の消えた一時木）は直す前の版の残骸でコードの指摘ではない。この席から 2 本に TERM を送ろうとしたが auto mode の権限で拒まれたので、PR 本文で人間に止めてもらう

## security review（ca8e114..9df54ee）

見た経路:
- 環境変数（`ASHIATO_EMULATOR_DEVICE_TIMEOUT` ほか 3 つ）→ `emu_positive_int` の `^[1-9][0-9]*$` → `timeout`・算術・`seq` にだけ入る。シェルへの展開（`eval` など）は無い（tools/emulator-start.sh:16-29・:101-146）（3）
- `ASHIATO_KVM_DEVICE` → `[ -e ]` の後に `( exec 3<>… )` で開くだけ。`-e` で先に止まるので、無い場所にファイルは作らない（tools/emulator-start.sh:41-46）（3）
- エミュレータの標準出力と標準エラー → `EMU_LOG`（既定 `collector-android/app/build/outputs/emulator.log`）→ 失敗したときに `tail -n 40` を標準エラーへ。ログの中身は adb の公開鍵・`/home/…` `/run/user/…` のパス・gRPC の待ち受けの行で、記録の中身（URL・位置・本文）は無い（実物の `emulator.log` を grep した）（4）
- `emulator.log` は `.gitignore:28`（`collector-android/app/build/`）の下にあり、`git ls-files` にも 0 件。CI に足した 2 行（ci.yml:134-135）は偽の SDK で走る `tools/test-android-emulator.sh` だけ。計測テストの job は `android-emulator-runner` のままで、本物のログを CI の出力へ流す経路は無い（4）
- 待ち受け: エミュレータ（console・adb・gRPC）と、real-proof.sh が `ANDROID_ADB_SERVER_PORT` で立てる adb server。いま動いている残り（R15 の PID 2718742 と adb の各 server）を `ss -ltnp` で見ると、すべて `127.0.0.1` で待ち受けている。`0.0.0.0` は 0 件。gRPC は `auth: +token`（4）
- 後片付けで止める相手: `emu_stop_own` は `$!` で取った自分の PID と、その `pgrep -P` の子孫だけにシグナルを送る。名前で探して止める経路は無い（tools/emulator-start.sh:49-81）。real-proof.sh も `SAME_PID=$!` の子孫だけ（3）
- `rm -rf`: real-proof.sh の `$TMP` は `mktemp -d`、`__internal` では `${REAL_PROOF_TMP:?}` を通る。中の SDK と JDK は symlink で、`rm -rf` は symlink の先までたどらない。試験の `$root` は `mktemp -d`（tools/test-android-emulator.sh:154）（3）
- `sed -i "/^${CASE}:/d"` の `CASE` は、その前の `case` で 4 つの語に絞られている（real-proof.sh の末尾）（3）
- commit した証跡（evidence.jsonl・real-proof.txt・.superpowers の報告）: パスは使い捨ての木の 1 本だけで、秘密も記録の中身も無い（grep した）（4）
- 依存: 新しい crate・npm・gradle・Action は無い（ci.yml の差分は `run:` の 2 行だけ）（4）

安全の決め手: この差分は開発の機械で走る試験の台本だけで、外から入ってくる値がない。外に出るものは、gitignore の下にあるエミュレータのログと 127.0.0.1 の待ち受けだけ —— 確かさ 4。`git check-ignore -v collector-android/app/build/outputs/emulator.log` → `.gitignore:28:collector-android/app/build/`。`git ls-files | grep -c emulator.log` → 0。`ss -ltnp | grep -E 'qemu|adb'` → 全行の待ち受けが `127.0.0.1:*`。

指摘なし

## /code-review（ca8e114..9df54ee）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R16. emu_wait_device and emu_wait_boot run without a serial (SERIAL is usually empty), so unscoped `adb get-state` / `adb shell getprop` accept any attached device; a phone, or a different-AVD emulator on 5554, either satisfies the wait or blocks it.
- 成果物: tools/android-emulator.sh:66
- 根拠: If a phone is plugged in, `adb get-state` returns `device` straight away and `getprop sys.boot_completed` on the phone returns 1. The script then runs uninstall, `pm reset-permissions` and the instrumentation on the user's phone while the new emulator is still booting. If another-AVD emulator is on 5554, real adb answers `more than one device`, the wait fails after 120 s, and the success-path cleanup would `emu kill` emulator-5554, which is the other AVD.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 本人の答え deep.md Q4「直さない（この change は「待たない」だけ。取り違えは「やらないこと」に書く）」の範囲。直す前の台本も `-s` 無しの `adb wait-for-device` / `adb shell getprop`（`git show ca8e1148:tools/android-emulator.sh` の :34-37）で、実機が繋がっているときの振る舞いはこの change で変わっていない。design.md D4 の 4 項目めに書いてある

## R17. The fake adb answers `device` to an unscoped `get-state` even when several devices are listed, so the other-device Scenario passes against behaviour real adb does not have.
- 成果物: tools/test-android-emulator.sh:116
- 根拠: With FAKE_EXISTING_DEVICES='R5CT00000:device emulator-5556:device', the fake returns `device` and the test goes green. Real adb with 2 or more devices returns `error: more than one device/emulator`, and the script fails after 120 s. Design D4 says so too, which means the Scenario's claim 'starts the emulator as now' is not actually guaranteed.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: final R6 と同じ指摘で、design.md D4 の 4 項目め（openspec/changes/ch10-emulator-start-timeout/design.md:70「偽の adb はこれを再現しない」）に書いて処置済み。other-device の Scenario が見るのは「同じ AVD でない端末なら起こしに進む」（tools/test-android-emulator.sh の other-device が起動の引数を見る）で、待ちの取り違えは Q4 で直さないと決めた範囲

## R18. emu_pid_alive relies on `kill -0` plus a zombie check, but bash reaps finished background jobs on its own. So the zombie check never fires, and once the PID is freed it can be reused by an unrelated process. EMU_PID is also never cleared after `wait`.
- 成果物: tools/emulator-start.sh:31
- 根拠: Once the emulator exits, bash reaps it and the PID is free. If an unrelated process gets that PID, emu_wait_device / emu_wait_boot keep waiting until their limits instead of failing fast. emu_stop_own, called again from emu_fail and then the EXIT trap, sends TERM/KILL to that unrelated process and its descendants. Clearing EMU_PID after wait/emu_stop_own (or tracking the job with `jobs -p`) would close this.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D13 （`emu_stop_own` の `wait` の後と、`emu_fail` で終わりを見た後に `EMU_PID=` を空にする。以後の `emu_pid_alive` は偽になり、EXIT の trap が空いた PID にシグナルを送らない。待ちの途中で終わってから次の確かめ（1〜2 秒）までに PID が使い回される余地は残る —— pid_max の下では実際上起きない）

## R19. The `getprop ro.boot.qemu.avd_name` result is not passed through `tr -d '\r'`, unlike the `emu avd name` fallback on the next line and the boot_completed checks.
- 成果物: tools/android-emulator.sh:51
- 根拠: On an adb/host where shell output ends in CRLF (older adb, or no shell protocol v2), existing_avd becomes `ashiato-api35\r`. The comparison with the AVD name fails, the duplicate is not detected, and the script starts a second emulator, which dies with the FATAL 'Running multiple emulators with the same AVD'. The intended clear error message for D3 is never shown.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D3 （tools/android-emulator.sh:51 の `getprop ro.boot.qemu.avd_name` の後にも `tr -d '\r'` を挟んだ。same-avd・other-device は rc=0）

## R20. The final R2 'cleanup on interruption' lives only in the EXIT trap of the `( main )` subshell. A SIGTERM sent to the top-level usage-volume.sh process is not forwarded to that subshell.
- 成果物: tools/usage-volume.sh:198
- 根拠: `kill -TERM <pid of usage-volume.sh>` (not a process-group kill) during emu_wait_device: the outer bash dies on the default disposition, but the subshell and the emulator it started keep running, and the build and measurement continue unattended. term-cleanup only tests android-emulator.sh, so this path is not covered.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 直す前の台本も同じ形 —— `git show ca8e1148:tools/usage-volume.sh` の :52 で trap は `( main )` の中、:189 の `( main )` の外に TERM の転送は無く、外の bash だけに TERM を送ったときに中が残るのは base から変わっていない。final R2 が直したのは「待ちの途中で止められたとき、起こしたものを中の EXIT の trap が止めない」（base より後退した分）で、中に届く SIGTERM・Ctrl-C（プロセスグループ）では止まる。外から中へ転送する作りの変更は CH10（無期限に待たない）の外

## R21. With an unscoped wait, emu_wait_boot can pass on a phone's boot_completed (or a reused 5554 emulator's), and its 180 × (10 s + 2 s) bound means up to 36 minutes in the worst case. That is far beyond the documented '180 × 2 s'.
- 成果物: tools/emulator-start.sh:128
- 根拠: If every getprop call hangs until ASHIATO_ADB_CALL_TIMEOUT=10, 180 attempts take about 2160 s, versus the 360 s implied by the docs and spec. The 'fail fast' goal is not met for an emulator whose adbd hangs.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: specs/emulator-startup/spec.md:58-59 が定めるのは「1 回ごとの問い合わせを既定 10 秒で打ち切る」と「回数の上限 180 回 × 2 秒は今のまま」で、実装はそのとおり。直す前は 1 回の `adb shell getprop` に上限が無く（`git show ca8e1148:tools/android-emulator.sh` :36）、adbd が固まると無期限だった —— 最悪 180 ×（10＋2）秒は有界になった分で、毎回 `emu_pid_alive` を見るのでプロセスが終われば早く止まる。上限の総和を 6 分にするのは spec の変更

## R22. usage-volume.sh now uses emu_wait_boot, so wait-android-boot.sh is dead code; only its own check in tools/test-usage-volume.sh:73 still calls it.
- 成果物: tools/wait-android-boot.sh:1
- 根拠: Two boot-wait implementations with different defaults (attempts/delay versus attempts/call timeout) are kept and tested. A future fix to one will not reach the other. Delete the script and its test line, or have emu_wait_boot call it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: tasks.md の Global Constraints（`tools/wait-android-boot.sh` と `tools/test-usage-volume.sh` は変えない）と、本人が Q5「この一覧のとおり」で決めた docs/changes/CH10.md:54 の (6)（`tools/wait-android-boot.sh` の有限の待ちがそのまま通る・呼び出し方は変えない）が消すことを禁じている。使い手が無くなったことは PR 本文の申し送りに挙げる

## R23. emu_reject_existing_avd is defined in android-emulator.sh rather than in the shared tools/emulator-start.sh, so the shared start-up component does not own the same-AVD check.
- 成果物: tools/android-emulator.sh:45
- 根拠: usage-volume.sh, or any future caller of emu_start, cannot reuse the check without copying it. Its retry loop (5 × 1 s per device, two adb fallbacks) also duplicates the polling pattern that emu_wait_* already implements.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design.md D3 の 4 項目め「採寸の台本には当てない（adb に見えているエミュレータがあれば起こさずに使う）」—— 同じ AVD の確かめは計測テストの台本だけの振る舞いで、Q3 の問いも計測テストの台本についてのもの。部品に移しても使い手は 1 つで、振る舞いは変わらない。名前を聞く 5 回の繰り返しは端末ごとの有界の問い合わせで、`emu_wait_*` の「プロセスの生死を見ながら待つ」とは別の待ち

## R24. The three animation-setting lines repeat the same timeout / adb / error-handling code with only the key changed.
- 成果物: tools/android-emulator.sh:68
- 根拠: Any change (adding `-s $SERIAL`, a different timeout, a different message) has to be made identically three times. A loop such as `for k in window_animation_scale transition_animation_scale animator_duration_scale; do …; done` does the same job.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D15 （3 行を `for key in window_animation_scale transition_animation_scale animator_duration_scale` の 1 つのループにした。base の :41-43 の位置で、tasks.md の「:44 以降は行を変えない」の外。tools/test-usage-volume.sh rc=0）

## R25. `: > "$EMU_LOG"` is redundant, because the next line's `>"$EMU_LOG"` truncates the file anyway.
- 成果物: tools/emulator-start.sh:63
- 根拠: It is an extra file operation that adds no behaviour and suggests the truncation matters separately when it does not.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D12 （`: > "$EMU_LOG"` を消した。次の行の `>"$EMU_LOG"` が毎回書き直す。tail40・dies は rc=0）

## scoped re-review（9df54ee0..15ae74ae）

R11〜R14・R18・R19・R24・R25（直した 8 件）は ADDRESSED。fix による新しい破損は無い（`./tools/test-android-emulator.sh` rc=0・`tools/test-usage-volume.sh` rc=0、`tools/wait-android-boot.sh` と `tools/test-usage-volume.sh` は不変）。退けたもの（R15〜R17・R20〜R23）は上の処置のとおり。
