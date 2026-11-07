# CH10 実装タスク — エミュレータが起動できないとき android-emulator.sh が無期限に待たない

読む順: `deep.md`（**本人の答え Q1〜Q5**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/emulator-startup/spec.md` → `design.md` → `premises.md` → `review/deep.md` → `docs/changes/CH10.md` → `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。Task 1〜5 は本物の SDK も /dev/kvm も要らない（偽の SDK）。Task 6 だけが手元（kvm あり）の本物のエミュレータを使う。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** 試験は bash なので `echo "Scenario: <名前>"` を、その場合を走らせる直前に出す。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 23 本**（`emulator-startup` の ADDED 7 つ: 3・3・4・2・2・6・3 本）。どれも `tools/test-android-emulator.sh` が印を出す（Task 1〜4 が名指しする）
- **本人の決定（下流は変えない）**: Q1 起動と待ちを 1 つの部品にし、2 つの台本が使う（D1）/ Q2 端末が現れるまでの上限 120 秒（D2）/ Q3 同じ AVD が動いていれば起こさずに理由つきで止まる（D3）/
  Q4 取り違えは直さない（D4）/ Q5 壊してはいけないものは `docs/changes/CH10.md` の一覧のとおり（D5）（第 1 回は推奨で先に書いた。推奨と違った答えは `spec --answers` が写す）
- **`tools/android-emulator.sh` の `:44` 以降（写真と動画・6 段の計測テスト・段ごとの XML）は行を変えない**（D1。`tools/test-usage-volume.sh:78-90` が行の形を grep する）。`tools/wait-android-boot.sh` と `tools/test-usage-volume.sh` は変えない
- **試験は台本に試験用の口を足さない**。使い捨ての木に本物の台本を写し、隣の台本と SDK を偽物にする（D16）。環境変数（D8）は台本の正規の設定で、試験用の口ではない
- **試験は本物の adb の server・本物の `~/.android`・作業ツリーの `collector-android/app/build/` に触らない**（偽の adb を PATH の先頭、`HOME` と `ANDROID_AVD_HOME` を使い捨ての木の中に向ける）。
  一時の場所は `mktemp -d`（`/tmp` の固定名に書かない）で、`trap` で必ず消す。偽のエミュレータの過程（印 `ch10-fake-emulator`）も `trap` で止める
- **どの場合も `timeout 30` で包む**（process group ごとに送る既定の形。`--foreground` を使わない）。rc=124 は `見張りで打ち切った: <場合の鍵>` を出してその場合を落とす。
  止まることを見る場合の「30 秒以内」はこの見張りで判定する（普段は 0〜数秒）。正常な経路に時間の assert を置かない（CH09 の方針）。試験の `trap` は使い捨ての木を指す過程（`pgrep -f "$tmp/"`）を全部止めて木を消す
- **偽のエミュレータの残りは、その回の使い捨ての木の場所で数える**（`pgrep -f "$tmp/"`。並走する worktree の同じ試験を数えない）。偽のエミュレータは子を 1 つ起こす（本物の qemu に当たる）
- verify-run はこの試験を「エミュレータが要る」と読む（D9）。**Codex の sandbox では申請が 1 回要る**（AGENTS.md の「/dev/kvm（Codex のとき）」の手順）。名前を変えて避けない。
  `android-emulator` の字面を含むほかの検証（Task 2.1 の `bash -n`・Task 5 の grep）も同じく申請が要る

## Task 1: 偽の SDK の試験と、今の正常な経路の固定

- [x] 1.1 `tools/test-android-emulator.sh` を design D16 のとおりに書く（冒頭のコメントに何を確かめるか・なぜ要るか（CH10）・走らせ方）。使い捨ての木に本物の `tools/android-emulator.sh`（と、あれば `tools/emulator-start.sh`・`tools/usage-volume.sh`）を写し、
  偽の `tools/android-env.sh`・`tools/android-media-fixtures.sh`・`collector-android/gradlew`（`collector-android/app/build/outputs/androidTest-results/connected/TEST-fake-_app-.xml` を 1 つ書く）と、
  偽の SDK（`emulator/emulator`・`system-images/android-35/google_apis/x86_64/`）・PATH の偽の `adb`・`avdmanager` を置く。偽の adb は `devices`・`get-state`・`wait-for-device`・`-s <serial> …`・`shell getprop sys.boot_completed`・`emu avd name`・`emu kill` に答え、受け取った引数を 1 行ずつ記録する。
  偽のエミュレータは D16 の `FAKE_EMU`（`die` / `sleep` / `late-die` / `ok`）で振る舞いを変え、引数を記録する。**この段では今の台本（直す前）に当てて**、次の 5 つを確かめる。それぞれの前に印を出す:
  `echo "Scenario: 正常に立つとき、今と同じ引数でエミュレータを起こす"` /
  `echo "Scenario: 正常に最後まで走ると、6 段の結果を残してエミュレータを止める"` /
  `echo "Scenario: KEEP=1 なら、通った後にエミュレータを止めない"` /
  `echo "Scenario: emulator かシステムイメージが無いときは、今の文言で止まる"` /
  `echo "Scenario: AVD が無ければ作ってから起こす"` /
  `echo "Scenario: 計測テストの台本の行の形の検査は、今のまま通る"`（`tools/test-usage-volume.sh` を走らせて rc=0）。
  採寸の台本は `android-env.sh` を読まないので、`ANDROID_HOME=<偽の SDK>` を渡して走らせる（D16）。
  検証: `tools/test-android-emulator.sh` rc=0
- [x] 1.2 1.1 の試験が今の台本の形を本当に見ていることを、写した台本を壊して確かめる手を試験の `--self-test` に置く: 写した `tools/android-emulator.sh` の `-no-snapshot` を消すと 1 つ目の場合が、`KEEP` の扱いを消すと 2 つ目の場合が落ちる
  （壊すのは使い捨ての木の写しだけ。元の木は変えない）。
  検証: `tools/test-android-emulator.sh --self-test` rc=0

## Task 2: 部品と、計測テストの台本の失敗の経路

- [x] 2.1 `tools/emulator-start.sh` を design D1・D2・D6〜D8・D11〜D13・D15・D17 のとおりに書く（source して使う関数: `emu_check_kvm` / `emu_start` / `emu_wait_device` / `emu_wait_boot` / `emu_fail`。冒頭のコメントに使い方・環境変数の表・なぜ要るか（CH10））。
  `tools/android-emulator.sh` の `:33-43` をこの部品の呼び出しに替え、`cleanup` を D13 のとおりに失敗の経路で `emu kill` を送らない形にする（`:44` 以降は変えない）。
  検証: `bash -n tools/emulator-start.sh` rc=0 / `bash -n tools/android-emulator.sh` rc=0
- [x] 2.2 `tools/test-android-emulator.sh` に次の 8 つを足す。それぞれの前に印を出す:
  `echo "Scenario: 開けない場所を渡すと、エミュレータを起こさずに /dev/kvm の文言で止まる"` /
  `echo "Scenario: 在っても読み書きできない場所を渡しても、同じく起こさずに止まる"`（`id -u` が 0 なら飛ばし、飛ばしたことを出す） /
  `echo "Scenario: /dev/kvm を開けないときの文言は、verify-run の環境の欠落の判定に掛かる"`（`.harness2/scripts/evidence.py` があれば `python3 -I` で `sys.path` に足して import し、`INFRA_SIGNS.search` が当たること。無ければ `/dev/kvm` の語だけ。D10） /
  `echo "Scenario: 起こしたエミュレータが起動直後に終わると、すぐ止まって出力の末尾を出す"` /
  `echo "Scenario: 標準エラーに出す出力の末尾は 40 行まで"` /
  `echo "Scenario: 端末が見えた後、起動の完了を待つ間に終わっても、すぐ止まる"` /
  `echo "Scenario: 上限を渡さなければ、端末を最大 120 秒待つ"` /
  `echo "Scenario: 上限に正の整数でない値を渡すと、起こさずに止まる"` /
  `echo "Scenario: 生きたまま端末が現れなければ、上限で止まる"` /
  `echo "Scenario: 起動の完了の問い合わせが返らなくても、回数の上限で止まる"`（`ASHIATO_EMULATOR_BOOT_ATTEMPTS=2 ASHIATO_ADB_CALL_TIMEOUT=1`） /
  `echo "Scenario: 上限で止まったとき、KEEP=1 でも自分の起こしたプロセスは残らない"`（`pgrep -f "$tmp/"` で、偽のエミュレータとその子が 0 本） /
  `echo "Scenario: 起動の途中で終わったとき、名指しの端末を止めない"`。
  場合ごとに鍵（`kvm-missing` `kvm-denied` `kvm-sign` `dies` `tail40` `late-die` `device-default` `bad-value` `device-timeout` `getprop-hang` `keep-cleanup` `no-named-kill`、Task 1 の `args` `stages` `keep` `missing` `avd-create` `usage-shape`、Task 3 の `same-avd` `other-device`、Task 4 の `usage-dies` `usage-kvm` `usage-reuse`）を持たせ、
  冒頭のコメントに鍵と Scenario の名前の表を置く。`ASHIATO_TEST_ONLY` に知らない鍵が渡されたら rc=2。
  正常な経路の場合（Task 1 の 6 つ）は、開ける場所（使い捨ての木の中の普通のファイル）を `ASHIATO_KVM_DEVICE` に渡して走らせる（CI の検査の job には本物の /dev/kvm が無い）。
  検証: `tools/test-android-emulator.sh` rc=0
- [x] 2.3 2.2 の失敗の経路の場合が、直す前の台本では落ちることを確かめる。試験に 2 つの環境変数を持たせる（試験自身の選び方で、台本の口ではない）: `ASHIATO_TEST_ONLY=<場合の名前>`（その場合だけ走らせる）と
  `ASHIATO_TEST_SCRIPT_REV=<rev>`（写す `tools/android-emulator.sh` を `git show <rev>:tools/android-emulator.sh` から取る）。偽の adb の `wait-for-device` は、端末が見えるまで返らない（本物と同じ）。
  直す前の版（`11c0c043`。この change の上流の分かれ目で、main の履歴に残る）では「起動直後に終わる」場合が 30 秒の見張りで落ちる。
  検証: `bash -c 'ASHIATO_TEST_ONLY=dies ASHIATO_TEST_SCRIPT_REV=11c0c043 tools/test-android-emulator.sh 2>&1 | grep -q "見張りで打ち切った: dies"'` rc=0 / `bash -c 'ASHIATO_TEST_ONLY=dies tools/test-android-emulator.sh'` rc=0

## Task 3: 同じ AVD のエミュレータが既に動いているとき

- [x] 3.1 `tools/android-emulator.sh` に design D3 の確かめを足す（部品の呼び出しの前。`adb devices` の `emulator-*` の `device` ごとに 5 秒の `timeout` で `emu avd name`）。`tools/test-android-emulator.sh` に次の 2 つを足す。それぞれの前に印を出す:
  `echo "Scenario: 同じ AVD が既に動いていると、起こさず・止めずに理由つきで止まる"` /
  `echo "Scenario: 実機か別の AVD のエミュレータだけが繋がっているときは、今のとおり起こす"`（偽の adb が `R5CT00000\tdevice` と、`emu avd name` に `other-avd` と答える `emulator-5556\tdevice` を見せる）。
  検証: `tools/test-android-emulator.sh` rc=0

## Task 4: 利用状況の採寸の台本も同じ部品で起こす

- [x] 4.1 `tools/usage-volume.sh` の `emulator_sample` の `:137-142` を `tools/emulator-start.sh` の呼び出しに替える（design D1。失敗は `return 2`・`started` は端末が現れてから置く・`-wipe-data` の引数は今のまま・動いている端末を使う `:127` は変えない）。
  失敗で止まるときは自分の起こしたプロセスを止める（部品の `emu_fail`）。台本は `ANDROID_HOME` を環境から受け取るので、試験は偽の SDK を渡す（D16）。
  `tools/test-android-emulator.sh` に次の 3 つを足す。それぞれの前に印を出す:
  `echo "Scenario: 採寸の台本で起こしたエミュレータが起動直後に終わると、すぐ rc=2 で止まる"` /
  `echo "Scenario: 採寸の台本でも /dev/kvm を開けなければ、起こさずに rc=2 で止まる"` /
  `echo "Scenario: 採寸の台本は、adb に見えているエミュレータがあれば起こさない"`。
  検証: `tools/test-android-emulator.sh` rc=0 / `tools/test-usage-volume.sh` rc=0

## Task 5: 文書と CI

- [ ] 5.1 `AGENTS.md` の「/dev/kvm（Codex のとき）」に design D14 の 1 文を足す。`tools/android-emulator.sh:9` の注記「無ければ数十倍遅い」を実測（無ければ起動直後に終わる。この台本は起こす前に確かめて止まる）に直す。
  `docs/testing.md` の `手元: \`./tools/android-emulator.sh\`` の行（今は 177 行目）に「起こせなければ理由つきで rc=1（/dev/kvm を開けない・起動の途中で終わる・120 秒で端末が現れない・同じ AVD が動いている）」を書き足す（**行を直すだけ。末尾に足さない**）。
  検証: `bash -c 'grep -q "error: /dev/kvm を開けない" AGENTS.md && ! grep -q "数十倍遅い" tools/android-emulator.sh && grep "手元: \`./tools/android-emulator.sh\`" docs/testing.md | grep -q "同じ AVD"'` rc=0
- [ ] 5.2 `.github/workflows/ci.yml` の `rust` の job の `- run: ./tools/docker-tidy.sh --self-test` の直後に `- run: ./tools/test-android-emulator.sh` と `- run: ./tools/test-android-emulator.sh --self-test` を足す（design D9）。
  検証: `bash -c 'grep -A2 -- "- run: ./tools/docker-tidy.sh --self-test" .github/workflows/ci.yml | grep -q "./tools/test-android-emulator.sh"'` rc=0 / `tools/check-shared-appends.sh` rc=0

## Task 6: 本物のエミュレータで確かめる（手元・kvm あり）

- [ ] 6.1 change の中に `real-proof.sh` を書く（`<case>`: `dies` / `timeout` / `same-avd` / `ok`。本物の SDK と kvm が要る。Codex では sandbox の外の申請が 1 回要る）:
  `dies` は `unshare -rm` で /dev/kvm に /dev/null を被せ（`premises.md` 1 行目と同じ形）、その中で `tools/android-emulator.sh` を走らせ、30 秒以内に rc=1・`error: エミュレータが起動の途中で終わった`・`/dev/kvm` の語があることを見る。
  `timeout` は `ASHIATO_EMULATOR_DEVICE_TIMEOUT=1` で走らせ、rc=1・`秒待っても端末が adb に現れない` と、終わった 10 秒後に `-avd ashiato-api35` を持つ過程が 0 本であることを見る（D13 の子の qemu）。
  `same-avd` は台本と同じ引数でエミュレータを背景で起こし、端末が見えた後に `tools/android-emulator.sh` を走らせ、rc=1・`error: 同じ AVD（ashiato-api35）のエミュレータが既に動いている` を見て、`emu avd name` の 1 行目が AVD の名前であることを記録し（D3）、最後に止める。
  `ok` は `tools/android-emulator.sh` を最後まで走らせ、rc=0・`collector-android/app/build/outputs/androidTest-results-stage1`〜`stage6` の 6 つに XML があることを見る（20 分前後）。
  `ok` と `same-avd` では、`collector-android/app/build/outputs/emulator.log` に `requires hardware acceleration` か `permissions to use KVM` の句が何行あるかも数え、`<case>-accel-lines: <n>` で書く（D17 の句が正常な出力・別の理由の失敗に出ないかの確かめ。0 のはず）。
  `timeout` の過程の残りは、台本が出した PID と、`pgrep -f -- "-avd ashiato-api35"` の両方で数える。
  結果を change の中の `real-proof.txt` に `<case>: pass|fail` で書く。
  検証: `scripts/quiet-run real-dies -- openspec/changes/ch10-emulator-start-timeout/real-proof.sh dies` rc=0 / `scripts/quiet-run real-timeout -- openspec/changes/ch10-emulator-start-timeout/real-proof.sh timeout` rc=0 /
  `scripts/quiet-run real-same -- openspec/changes/ch10-emulator-start-timeout/real-proof.sh same-avd` rc=0 / `scripts/quiet-run real-ok -- openspec/changes/ch10-emulator-start-timeout/real-proof.sh ok` rc=0
- [ ] 6.2 Scenario の印が全部あることを確かめる。
  検証: `bash -c '[ "$(tools/test-android-emulator.sh 2>/dev/null | grep -c "^Scenario: ")" -ge 23 ]'` rc=0 / `tools/test-android-emulator.sh --self-test` rc=0 / `python3 scripts/check_scenarios.py . ch10-emulator-start-timeout` rc=0
