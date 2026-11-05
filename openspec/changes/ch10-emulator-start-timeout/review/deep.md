# CH10 deep-review（第 1 回の問いの独立検査）

検査対象: `openspec/changes/ch10-emulator-start-timeout/deep-questions.json`（Q1〜Q7。全問 B・推奨あり）と `premises.md`（前提 8 行・既定 C1〜C5）。
読んだもの: `docs/changes/CH10.md`、`tools/android-emulator.sh`・`tools/usage-volume.sh`・`tools/wait-android-boot.sh`・`tools/test-usage-volume.sh`・`tools/android-env.sh`・`tools/android-media-fixtures.sh`、
`.harness2/scripts/evidence.py`（`NEEDS_EMULATOR`・`INFRA_SIGNS`・`kvm_usable`・`needs_emulator`・`run_one`）、`scripts/quiet-run`、`.github/workflows/ci.yml`、`AGENTS.md`、
`python3 scripts/upstream_misses.py report`、`.harness2/skills/story-upstream/SKILL.md` の stage: questions。

走らせたもの（2026-11-05、手元。kvm あり）:
- 隔離した AVD 置き場（`ANDROID_AVD_HOME=$(mktemp -d)`）で AVD を作り、1 台目を `-port 5610` で立てて adb に見えた後、**同じ AVD** で 2 台目を台本と同じ引数（`-read-only` なし）で起こした → **3 秒で rc=1**、
  `FATAL        | Running multiple emulators with the same AVD is an experimental feature.Please use -read-only flag to enable this feature.`。1 台目は `emu kill` で止め、置き場は消した
- `evidence.needs_emulator(root, cmd)`: `tools/test-usage-volume.sh` → True、`bash tools/test-android-emulator.sh` → **True**（コマンドの字面の `android-emulator` に当たる）、`tools/check-ports.sh --self-test` → False。`evidence.kvm_usable()` → True（手元）
- `strings` で emulator 37.1.11 の文言を見た: `libandroid-emu-shared.so` に `This user doesn't have permissions to use KVM (%s).`、qemu に上の FATAL の文言
- `python3 scripts/ask_wizard.py … -o /tmp/…`（止める 0 / 仮でよい 7。描けることだけ確かめて消した）

## 手順のやり直し（問いの一覧を見る前）

- **手順 1 要件どうしの衝突**: `satisfies: []`（change）。依頼の中で「早く失敗する（プロセスが死んだらすぐ失敗）」と「正常にエミュレータが立つ場合の振る舞いは変えない」が、
  **同じ AVD のエミュレータが既に動いている場合**にぶつかる（R1）。もう 1 つ、「verify-run が BLOCKED_INFRA と判定できる文言」と「AGENTS.md の手順に乗るように」は、
  verify-run の走らせた後の理由の文言が AGENTS.md の字面と違うため、台本の文言だけでは両立しない（R6）
- **手順 2 扉の幅**: 該当なし（確かめた範囲: change なので doors が無い。`docs/changes/CH10.md` の front matter は `satisfies: [] requires: [] capability: []`）
- **手順 3 新たに立つ一方通行**: 該当なし（確かめた範囲: この change が捨てる・変換する・外に出すものは無い。エミュレータの出力は今は捨てていて（`android-emulator.sh:34` の `>/dev/null 2>&1`）、この change は残す側に動く。
  ログを毎回上書きしても、失敗の末尾は呼び出し元のログ（`scripts/quiet-run:72-77`・`evidence.py` の `logs/`）に残る。列・保存形式・鍵は無い）。全問 B は妥当
- **手順 4 日常に影響する選択**: 上限の値（Q2）が、遅い起動を誤って落とす頻度と固まったときに待つ長さを決める。問いにある。ほかに常時通知・容量に当たるものは無い
  （ログは 1 回の起動で約 100 行。`premises.md` 5 行目）
- **手順 5 既存コードが依頼を満たしていない箇所**: `android-emulator.sh:35`（上限なし）・`:34`（出力を捨てる）・`:30`（後片付けは `emulator-5554` 決め打ち）・
  `:36-40`（回数の上限はあるが、1 回ごとの `adb shell` に時間の上限が無い。R10）・`:9` の注記「無ければ数十倍遅い」は実測（`premises.md` 1 行目: 0 秒で rc=1）と合わない。
  `usage-volume.sh:137-142` は同じ形（Q1 が扱う）。CI は台本を使わない（`ci.yml:295`）ので、**台本の正常な経路を機械で走らせる場所が無い**（R5）
- **手順 6 過去の取りこぼし**: 「凍結前に tasks の全検証コマンドを verify-run で空走させる」（st06 Q11・st14 Q-T1・st17 QR1。族「検証コマンドが成立しない」11 件）が当たる。
  C4 の試験は verify-run が「エミュレータが要る」と読み、Codex の sandbox では走らせずに BLOCKED_INFRA になる（R3）。CI の検査の job では kvm を開けず、前もって調べる形（Q3 の推奨）だと偽のエミュレータまで届かない（R4）。
  「X を後のために残す」の置き場の寿命（ch05 Q9）は Q4 の context が書いている
- **手順 7 族の網羅**: Q2 の族「エミュレータが使える状態にならない場合」に、**同じ AVD が既に動いていて新しいプロセスが 3 秒で死ぬ**場合が無い（R1）。
  **adb には見えたが、1 回の `adb shell` が返らない**場合も無い（R10）
- **手順 8 前提の確かさ**: `loss` のある決定は無いので、確かさ 4 未満に `loss` が載る違反は無い。ただし 1・2 行目の確かさ 5 は `/dev/null` を被せた再現の確かさで、
  事故が起きた Codex の sandbox（開けない理由が別でありうる）の確かさではない（R7）。7 行目（HOME を向け直せば差し替えられる）は正常な経路について成り立たない（R5）。
  verify-run の字面の判定（R3）・CI の kvm（R4）・同じ AVD の二重起動（R1）は前提の表に無い

## R1. 同じ AVD が既に動いているとき、今は「先へ進む」のに、この change の後は「すぐ失敗」に変わる。問いにも「壊してはいけないもの」にも無い
- 成果物: openspec/changes/ch10-emulator-start-timeout/deep-questions.json（Q2 の family・Q6・Q7）
- 根拠: 実測（上の「走らせたもの」）: 同じ AVD の 2 台目は 3 秒で rc=1、`FATAL | Running multiple emulators with the same AVD …`。
  `tools/android-emulator.sh:20`（AVD の既定は `ashiato-api35`。worktree をまたいで同じ `~/.android/avd`）・`:34-35`（起動を背景に回し、`adb wait-for-device` は**どれかの**端末で返る）・
  `:4-5`（`KEEP=1` は「続けて手で触る」ために残す）。今は `KEEP=1` の後の再実行や、別の worktree が同じ AVD で走らせている間は、新しいプロセスが死んでも前からある端末で計測テストが走る。
  依頼の「死んだらすぐ失敗」を入れると、この場合は失敗になる。`tools/usage-volume.sh:127` は動いている端末があれば起動せずに使う（台本ごとに扱いが違う）。
  FATAL の形は `evidence.py:72` の `emulator: (ERROR|FATAL)` に当たらない
- kind: technical
- 提案: Q2 の family に「同じ AVD のエミュレータが既に動いている（`KEEP=1` の残り・別の worktree）」を兄弟として足し、決まっていないので問いにする
  （例: 理由つきで失敗する / `usage-volume.sh` と同じく動いている端末を使う / `-read-only` で別に立てる）。Q7 の一覧にこの場合の振る舞いを入れる
- 処置: fixed deep-questions.json —— 新しい Q3（同じ AVD が既に動いているとき）を足し、Q2 の family に兄弟として置いた。前提の表に 1 行足した
## R2. Q6 の context「新しいエミュレータは別の番号で立つ」は、既定の同じ AVD では起きない。推奨の detail「正常時の振る舞いは変わらない」も誤り
- 成果物: openspec/changes/ch10-emulator-start-timeout/deep-questions.json（Q6）
- 根拠: R1 の実測（同じ AVD の 2 台目は番号を変えて立たず、3 秒で死ぬ）。別の番号で立つのは AVD が違うか `-read-only` のとき（`premises.md` 5 行目の計測自身が `-port 5600 -read-only` を付けている）。
  `tools/android-emulator.sh:30-31`: 後片付けは `KEEP` でなければ `adb -s emulator-5554 emu kill` を無条件に呼ぶので、自分のプロセスが 3 秒で死んだ新しい失敗の経路でも、
  `emulator-5554` にいる他人のエミュレータを止める（今は計測テストの後に止めている。新しい経路では数秒で止める）。Q5 の推奨「台本が起こしたプロセスだけを止める」はこの後片付けに触れていない
- kind: premise
- 提案: Q6 の context を「同じ AVD が動いていれば新しいプロセスは死ぬ。別の AVD なら別の番号で立つ」に直し、推奨の detail から「正常時の振る舞いは変わらない」を外す（R1 の答え次第で変わる）。
  失敗の経路の後片付けが `emulator-5554` を止めるかを、Q5 か Q6 の選択肢に書く
- 処置: fixed deep-questions.json —— Q4（旧 Q6）の context と推奨の detail を直した。失敗の経路の後片付けは premises.md C8（自分のプロセスが死んだ・起動に失敗したときは emulator-5554 を止めない）
## R3. C4 の試験（`tools/test-android-emulator.sh`）は、verify-run が「エミュレータが要る」と読み、Codex の sandbox では走らせずに BLOCKED_INFRA になる
- 成果物: openspec/changes/ch10-emulator-start-timeout/premises.md（C4）
- 根拠: `.harness2/scripts/evidence.py:69`（`NEEDS_EMULATOR = …|android-emulator|…|\badb\b`）・`:253-259`（コマンドの字面か、指す台本の中身に当たれば真）・`:367-368`（真で /dev/kvm を開けなければ走らせない）。
  `needs_emulator(root, "bash tools/test-android-emulator.sh")` → True（名前だけで当たる。中身も `android-emulator.sh` と偽の `adb` を書くので当たる）。
  既存の `tools/test-usage-volume.sh` も True。AGENTS.md:53-57 の手順で sandbox の外を申請すれば走るが、C4 の「本物のエミュレータも /dev/kvm も要らない」と verify-run の扱いが合わない。
  過去の取りこぼし「凍結前に tasks の全検証コマンドを verify-run で空走させる」（st06 Q11・st14 Q-T1）
- kind: technical
- 提案: premises.md に「verify-run はこの試験を、コマンドの名前と中身の字面で、エミュレータが要るものと読む」を 1 行足し（確かさ 4: 上の呼び出しで確かめた）、
  C4 に「Codex の sandbox では申請が 1 回要る（AGENTS.md の手順）」か、申請を要らなくする形（ハーネス側の判定を直すのは repo の範囲の外）のどちらを採るかを書く。問いにするほどの幅は無い（C でよい）
- 処置: fixed premises.md —— 前提の表に needs_emulator の 1 行を足し、C4 に「Codex の sandbox では申請が 1 回要る」を書いた（ハーネス側の判定は直さない）
## R4. Q3 の推奨（起動の前に /dev/kvm を開いてみる）を採ると、C4 の試験は CI でも手元でも、片方の経路にしか届かない。差し替えの口が C3 / C4 に無い
- 成果物: openspec/changes/ch10-emulator-start-timeout/premises.md（C3・C4・C5）
- 根拠: `.github/workflows/ci.yml:289-294` —— /dev/kvm を開けるようにする udev の規則は `android-instrumented` の job にだけあり、C4 が置き場に挙げる検査の job（`rust`。`:57` 以降、`check-*.sh --self-test` が並ぶ所）には無い。
  そこでは前もって調べる段で止まり、偽の emulator が「起動直後に死ぬ」経路（依頼の「検証」の 1 つ目）に届かない。手元では `kvm_usable()` が True で、「/dev/kvm を開けない」経路（依頼の「検証」の 2 つ目）に届かない。
  verify-run 自身は同じ問題を `EVIDENCE_KVM_DEVICE`（`evidence.py:247`）で解いている
- kind: technical
- 提案: C3 に「調べる /dev/kvm の場所も環境変数で差し替えられる（試験は開ける場所・開けない場所の両方を渡す）」を足し、premises.md に「CI の検査の job では /dev/kvm を開けない（`ci.yml:289-294` の規則は計測テストの job だけ）」を 1 行足す（確かさ 2）
- 処置: fixed premises.md —— C3 に /dev/kvm の場所の差し替えの口を足し、前提の表に CI の検査の job では kvm を開けない行を足した
## R5. Q7 の推奨「偽の emulator / adb の試験で (1) の流れを確かめる」は成り立たない。正常な経路（6 段・XML）を走らせる場所がどこにも無い
- 成果物: openspec/changes/ch10-emulator-start-timeout/deep-questions.json（Q7）/ premises.md（7 行目）
- 根拠: `tools/android-env.sh:4`（`JAVA_HOME=$HOME/.local/opt/jdk21`）、`collector-android/gradle/wrapper/gradle-wrapper.properties:1,3`（配布物は `GRADLE_USER_HOME` = `$HOME/.gradle` の下）、
  `tools/android-emulator.sh:75`ほか（`./gradlew` は PATH でなく相対パスで呼ぶ）。HOME を一時フォルダへ向けると、起動の後の段は JDK も gradle の配布物も無く落ちる（偽に差し替えられない）。
  CI はこの台本を走らせない（`ci.yml:295`）。premises.md 7 行目「台本を変えずに差し替えられる」は起動まで（emulator / adb / avdmanager）の話で、正常な経路の全体ではない。
  あわせて、Q1 の推奨（起動と待ちを 1 つの部品にする）を採ると `tools/test-usage-volume.sh:270-275`（`wait-android-boot.sh` の有限の待ちの試験）も対象になるが、Q7 の (6) は grep の形だけを挙げている
- kind: premise
- 提案: Q7 の context と推奨の detail を、「(1) は偽の試験では確かめられない。手元（kvm あり）で台本を 1 回本物で走らせる項目を tasks に置く（Codex では申請が要る）」か「gradle の呼び出しを差し替えられる口を足して偽で確かめる」のどちらかに直し、
  後者なら Q7 の選択肢に出す。(6) に `test-usage-volume.sh:270-275` を足す
- 処置: fixed deep-questions.json —— Q5（旧 Q7）の context と推奨の detail を直した（(1) は手元で本物を 1 回走らせる項目にする・(6) に test-usage-volume.sh:270-275）。premises.md 7 行目を起動までに絞った
## R6. 「/dev/kvm を開けない」で落ちた後に agent が何をするかが決まっていない。verify-run の走らせた後の理由は AGENTS.md の字面と違い、事故の経路（quiet-run 直）は AGENTS.md の手順の外
- 成果物: openspec/changes/ch10-emulator-start-timeout/deep-questions.json（Q3）/ premises.md（3 行目・C5）
- 根拠: `AGENTS.md:55` は「`scripts/verify-run` が `BLOCKED_INFRA`:「/dev/kvm を開けない」を返したもの**だけ**」を sandbox の外で走らせ直すとする。
  verify-run が走らせる前に止めるとき（`evidence.py:367-368`）の理由は「エミュレータが要るが /dev/kvm を開けない（走らせていない）」で字面に合うが、
  走らせた後に出力で判定するとき（`evidence.py:419-420`）の理由は「環境の欠落を示す出力: /dev/kvm」（`m.group(0)`）で、「を開けない」を含まない。
  事故は `scripts/quiet-run … -- ./tools/android-emulator.sh`（`docs/changes/CH10.md:16`）で起きていて、quiet-run は判定をしない。premises.md 3 行目もこの経路の差を書いているが、問いがそこから先を扱っていない
- kind: technical
- 提案: 「台本の `error:` の行に次の手（verify-run で走らせ直す・Codex なら sandbox の外を申請する）を書くか / AGENTS.md の手順に quiet-run 直で落ちた場合を足すか / どちらもしないか」を B の問い（推奨つき）か C に足す。
  ハーネス側の理由の文言（`evidence.py:420`）は repo の範囲の外なので、変えないなら「やらないこと」に書く
- 処置: fixed premises.md —— C9（error の行に次の手・AGENTS.md に quiet-run 直の場合を足す・evidence.py の文言は変えない）と、前提の表に理由の字面の違いの行を足した
## R7. 前提 1・2 行目の確かさ 5 は `/dev/null` を被せた再現の確かさで、事故の環境（Codex の sandbox）のものではない。Q3 の context はそれを一般化している
- 成果物: openspec/changes/ch10-emulator-start-timeout/premises.md（1・2 行目）/ deep-questions.json（Q3 の context・Q2 の why）
- 根拠: premises.md 1 行目の確かめ方は `unshare -rm` + /dev/null の bind で、開いた後の ioctl が失敗する形（`Inappropriate ioctl for device`）。開くこと自体を拒まれたときの文言は、
  emulator 37.1.11 の `lib64/libandroid-emu-shared.so` に `This user doesn't have permissions to use KVM (%s).`（`strings` で確認）があり、こちらは「/dev/kvm」を含んで `INFRA_SIGNS`（`evidence.py:72`）に当たりうる。
  Codex の sandbox が /dev/kvm をどう拒むか（EACCES / EPERM / ほか）、そこで qemu が「すぐ死ぬ」のか「生きたまま固まる」のかは、出力が捨てられていて（`CH10.md:19`）誰も見ていない。
  Q3 の context「/dev/kvm を使えないと…出す文言は…判定に掛からない」は 2 行目の「権限が無いときの文言は確かめていない」を落としている。Q2 の why「この上限が効くのは生きたまま固まった場合だけ」は、事故の環境がその場合でないと確かめていない
- kind: premise
- 提案: 1・2 行目の「何が依るか」か「結果」に「再現は /dev/null を被せた形。sandbox で開くのを拒まれる形は再現していない（その文言は /dev/kvm を含む）」と書き、Q3 の context を同じ範囲に絞る。
  推奨（前もって開いてみる）はどちらの形でも成り立つので変わらない。Q2 の why に「事故の環境で死ぬか固まるかは見ていない。固まるならこの上限だけが効く」を足す
- 処置: fixed premises.md —— 1・2 行目を /dev/null を被せた形の確かさに絞り、拒まれたときの文言を書いた。Q2 の why に「死んだか固まったかは見ていない」を足した（Q3 は R8 で外した）
## R8. Q3 は依頼で決まっていて幅が無い（不要）
- 成果物: openspec/changes/ch10-emulator-start-timeout/deep-questions.json（Q3）
- 根拠: 依頼（`docs/changes/CH10.md:29-30`）は「verify-run が BLOCKED_INFRA と判定できる文言で落ちる」を求める。2 つ目の選択肢はエミュレータの文言に依り、
  premises.md 2 行目の実測（`ERROR        |` の形で /dev/kvm の語が出ない）では判定に掛からないことがある —— 依頼を満たすのは 1 つ目だけ。費用も小さく、後から変えても失うものが無い
- kind: technical
- 提案: 問いから外して C（「起動の前に /dev/kvm を読み書きで開いてみる。開けなければ `error: /dev/kvm を開けない` で止まる」）にし、design の D に残す。R4 の差し替えの口と一緒に書く
- 処置: fixed deep-questions.json —— 旧 Q3 を外し、premises.md C6 にした
## R9. Q4 は人間に聞く幅が無い（不要）。2 つ目の選択肢の「作業ツリーに残らない」は verify-run の下では誤り
- 成果物: openspec/changes/ch10-emulator-start-timeout/deep-questions.json（Q4）
- 根拠: 置き場と行数は日常にも失うものにも効かず、失敗の末尾はどちらでも呼び出し元のログに残る（`scripts/quiet-run:72-77`）。
  2 つ目の detail「作業ツリーに残らない」: verify-run の下では `$TMPDIR` は worktree の中の `.superpowers/sdd/tmp/v…`（`evidence.py:381`）で、残したものは `tmp_left` に記録され（`:415`）、
  `HARNESS_TMP_LEAK=fail` なら rc=0 でも FAIL（`:425-426`）。verify-run の外（quiet-run 直）では共有の `/tmp` に残る
- kind: technical
- 提案: 問いから外して C（推奨の形: `collector-android/app/build/outputs/emulator.log` に上書き、失敗で末尾を出す）にする。残すなら 2 つ目の detail を上の事実に直す
- 処置: fixed deep-questions.json —— 旧 Q4 を外し、premises.md C7 にした
## R10. 起動の完了の待ちの「約 6 分の上限」は回数の上限で、1 回の `adb shell` が返らなければ効かない。族の兄弟として無い
- 成果物: openspec/changes/ch10-emulator-start-timeout/deep-questions.json（Q2 の family）/ premises.md（C1）
- 根拠: `tools/android-emulator.sh:37,40`・`:41-43`・`tools/wait-android-boot.sh:11` の `adb shell …` には時間の上限が無い（`for` の回数と `sleep 2` だけ）。
  C1 は「死んだらすぐ失敗」を足すが、プロセスが生きていて adb に見えた後に 1 回の呼び出しが返らない場合は、死の検出にも回数の上限にも掛からない。返らないことが起きるかは確かめていない（確かさ 1。だから問いにでなく前提か C に置く）
- kind: technical
- 提案: Q2 の family に「adb に見えた後、1 回の `adb shell` が返らない」を足し、決定は C（1 回ごとに `timeout` を掛ける、起動の完了の待ち全体に時間の上限を掛ける）か「範囲の外」と書く
- 処置: fixed deep-questions.json —— Q2 の family に兄弟を足し、premises.md C10（1 回ごとの adb shell に timeout）にした
## R11. Q5 は人間に聞く幅が小さい（不要の候補）
- 成果物: openspec/changes/ch10-emulator-start-timeout/deep-questions.json（Q5）
- 根拠: `tools/android-emulator.sh:4-5` の `KEEP=1` は「続けて手で触るとき」で、起動に失敗した（adb に現れない）端末は触れない。後から変えても失うものが無く、扉を開けたままにする既定（台本が起こしたプロセスだけを止める）がある。
  ただし R2 の後片付け（`emulator-5554` を無条件に止める）とは絡むので、外すなら R2 の扱いを Q6 か C に移す
- kind: technical
- 提案: 問いから外して C（推奨の形）にし、design の D に「台本が起こした PID を止める。`emulator-5554` を名指しで止めるのは自分の端末と確かめたときだけ」のように R2 と一緒に書く。R2 を問いで扱うなら Q5 は残してよい
- 処置: fixed deep-questions.json —— 旧 Q5 を外し、premises.md C8 に R2 と一緒に書いた

## 確かめて指摘しなかったもの

- 構成の記号（`S-01` など）: 問い・premises.md に 0 件（`grep -noE '\b[SCDV]-0[0-9]\b|FR-[0-9]+|NFR-[0-9]+'`）
- B に推奨が無い問い: 無い（Q1〜Q7 すべて `recommended` が 1 つ。ask_wizard は「止める 0 / 仮でよい 7」で描けた）
- 分類違い（`loss` が実はある / 無い）: 無い。手順 3 のとおり、この change で失われるものは無く、全問 B は実態と合う
- Q1（usage-volume.sh も直すか）・Q2（上限の値）・Q6（取り違えを直すか）・Q7（壊してはいけないもの）は、change の第 1 回で問う範囲・変えてよい振る舞い・閾値に当たり、要る
- premises.md 3・6・8 行目（verify-run の走らせる前の判定・CI は台本を使わない・usage-volume.sh の待ちは有限）はコードと合う（`evidence.py:367-368`・`ci.yml:295`・`usage-volume.sh:137-142`・`wait-android-boot.sh:10-16`）
- Q7 の (1)「6 段」は `keep_stage` の呼び出し 6 回と合う（`android-emulator.sh:78,84,91,99,104,109`）
