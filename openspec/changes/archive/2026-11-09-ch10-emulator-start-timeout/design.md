# CH10 design — エミュレータが起動できないとき android-emulator.sh が無期限に待たない

> 第 1 回の深掘りの推奨の答え（Q1〜Q5）と、聞かずに決めた既定（`premises.md` の C1〜C10。ここでは D6〜D15、印「C」）で先に書いた。
> 本人の答えが推奨と違えば、その Q の D（仮）を反転条件のとおりに書き直す。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-05 の repo）:

- `tools/android-emulator.sh:34-35` はエミュレータを `>/dev/null 2>&1 &` で起こし、`adb wait-for-device`（上限なし・**どれかの**端末で返る）で待つ。
  起動の完了の待ち（`:36-40`）は 180 回 × 2 秒の回数の上限を持つが、1 回の `adb shell` には時間の上限が無い。後片付け（`:30`）は `KEEP=1` でなければ `emulator-5554` を名指しで止める
- `tools/usage-volume.sh:137-142` も同じ形で起こし、待ちは `tools/wait-android-boot.sh`（有限）。死んでも約 6 分待ち、理由は残らない。動いているエミュレータがあれば起こさずに使う（`:127`）。失敗は rc=2
- /dev/kvm に /dev/null を被せた再現では、エミュレータは 0 秒で rc=1 で終わり、出力に「/dev/kvm」の語は無かった（`premises.md` 1・2 行目）。Codex の sandbox が開くこと自体を拒む形は再現していない
- verify-run は、台本の名前か中身に `adb` / `android-emulator` があれば、/dev/kvm を開けないとき走らせる前に BLOCKED_INFRA にする。走らせた後は、rc≠0 の出力に `/dev/kvm` などがあれば BLOCKED_INFRA（`premises.md` 3・12 行目）。
  今回の 20 分は verify-run の外（`scripts/quiet-run` 直）で起きた
- CI の計測テストはこの台本を使わない（`android-emulator-runner`）。CI の検査の job では /dev/kvm を開けない（`premises.md` 6・11 行目）

## Goals / Non-Goals

**Goals:**
- 起こせない 4 つの場合（/dev/kvm を開けない・起動の途中で終わる・端末が現れない・同じ AVD が既に動いている）で、台本が上限か即時に止まり、理由と出力の末尾を出す（spec `emulator-startup` の 1〜4 つ目の Requirement）
- 失敗で止まるとき、自分の起こしたプロセスを残さず、他人の端末を止めない（5 つ目）
- 正常に立つときの振る舞いを変えず、それを偽の SDK の試験で固定する（6 つ目）
- 採寸の台本も同じ部品で同じ止まり方をする（7 つ目。Q1）

**Non-Goals:**
- 待つ相手・止める相手の取り違えを直すこと（Q4。D4（仮））
- ハーネス側（`.harness2/scripts/evidence.py` の判定と理由の文言・`record-run` の起動）を変えること
- CI の計測テストの起動（`android-emulator-runner`）を変えること
- 計測テスト（gradle）の途中でエミュレータが死んだときの扱い（gradle が端末を失って落ちる。今も無期限には待たない）
- 走っている st54 が足す `tools/android-side-by-side.sh`（同じ形で起こし、上限の無い `adb wait-for-device` で待つ）を直すこと。st54 の merge の後に、この change の部品 `tools/emulator-start.sh` を使う別の change で直す（spec-review R13。走っている Story へ差し戻さない）

## Decisions

### D1（仮）. 起動と待ちを 1 つの部品 `tools/emulator-start.sh` にし、2 つの台本がそれを使う（Q1）

- `tools/emulator-start.sh` は **source して使う関数の集まり**（実行しない）。プロセスを背景に残して PID を呼び出し元に渡す必要があるので、別の過程にしない。関数:
  - `emu_check_kvm` —— D11 の /dev/kvm の確かめ
  - `emu_start <AVD> <引数…>` —— D12 の置き場へ出力を書き直しながら `"$ANDROID_HOME/emulator/emulator" -avd <AVD> <引数…>` を背景で起こし、PID を `EMU_PID` に置く
  - `emu_wait_device [serial]` —— D2 の上限まで 1 秒ごとに、`EMU_PID` が生きているか → 端末が `device` か（serial が無ければ `adb get-state`、あれば `adb -s <serial> get-state`。どちらも 1 回 5 秒の `timeout`）を見る
  - `emu_wait_boot [serial]` —— D6・D15 の起動の完了の待ち
  - `emu_fail <文言>` —— `error: <文言>` と出力の末尾（D12）と、加速の不足なら D17 の行を出し、D13 の後片付けをする。rc は返さない（呼び出し元が 1 か 2 で終わる）
- `tools/android-emulator.sh` は `:33-43`（起動・待ち・アニメーション）をこの部品の呼び出しに替える。D3 の同じ AVD の確かめは起こす前にこの台本だけが呼ぶ。`:44` 以降（写真と動画・6 段の計測テスト）は**行を変えない**（`tools/test-usage-volume.sh:78-90` が行の形を grep する）
- `tools/usage-volume.sh` の `emulator_sample` は `:137-142` をこの部品に替え、失敗は今の約束どおり `return 2`。`started` は端末が現れてから置く（D13: 現れる前に止まったとき、`emulator-5554` を名指しで止めない）。
  動いている端末を使う（`:127`）・`-wipe-data` の引数・同じ AVD の確かめをしない、は変えない
- `tools/wait-android-boot.sh` は変えない（呼び出し方は `tools/test-usage-volume.sh:73` が固定する）。採寸の台本からは呼ばれなくなるが、消さない
- 反転条件: Q1 が「`tools/android-emulator.sh` だけを直す」なら、部品を作らずに同じ関数を `tools/android-emulator.sh` の中に置き、`tools/usage-volume.sh` と spec の 7 つ目の Requirement を外す

### D2（仮）. 端末が adb に現れるまでの上限は既定 120 秒（Q2）

- `ASHIATO_EMULATOR_DEVICE_TIMEOUT`（秒。正の整数でなければ `error:` で rc=1）で上書きできる。待ち始めに `== 端末が adb に現れるのを最大 <N> 秒待つ` を出す（既定を待たずに観測できる。spec-review R3）。既定 120 秒は、手元の実測（約 15 秒。`premises.md` 5 行目）の 8 倍
- 起動の完了の待ちは別に数える（D6）。上限を超えたら `error: <N> 秒待っても端末が adb に現れない` と出力の末尾を出す
- 反転条件: Q2 が「300 秒」なら既定の値だけを変える。「上限を 1 つにする」なら、端末の待ちを独立に数えず、起動の完了の待ちの回数（180 回 × 2 秒）に端末の待ちを含める（spec の 3 つ目の Requirement の本文を書き直す）

### D3（仮）. 同じ AVD のエミュレータが既に動いていれば、計測テストの台本は起こさずに止まる（Q3）

- 起こす前に `adb devices` の `emulator-*` で `device` の端末ごとに `adb -s <serial> shell getprop ro.boot.qemu.avd_name` を 1 秒ずつ最大 5 回聞き、空なら `adb -s <serial> emu avd name`（同じく 1 秒の `timeout`）へ fallback し、答えの 1 行目が `AVD` と同じなら
  `error: 同じ AVD（<AVD>）のエミュレータが既に動いている（<serial>）。KEEP=1 の残りなら adb -s <serial> emu kill で止めてから` で rc=1。起こさず、その端末に何も送らない
- adb に見えない同じ AVD のプロセス（起動の途中）は、この確かめでは見つけない。そのときは新しいプロセスが約 3 秒で `FATAL | Running multiple emulators with the same AVD …` で終わり（`premises.md` 9 行目）、D6 の「起動の途中で終わった」で止まる。どちらでも止まり方は同じく理由つき
- emulator 37.1.11 では実機によって `emu avd name` の答えが空になるため、同じ AVD の判定は `ro.boot.qemu.avd_name` を本線にする。Task 6 は両方の生の 1 行目を記録する。今回の実機検証では `emu avd name` の 1 行目も `ashiato-api35` だったが、これは環境差を示す実測であり、空の応答に備えた fallback は残す。偽の adb は本線を返す
- 採寸の台本には当てない（adb に見えているエミュレータがあれば起こさずに使うので、同じ AVD を見つける前にそれを使う）
- 反転条件: Q3 が「動いている端末を使う」なら、確かめで見つけた端末の serial を使って起動を飛ばし、後片付けもその端末を止めない。「`-read-only` で別に立てる」なら、空いている番号を選んで `-port` と `-read-only` で起こし、
  待つ・使う・止めるをその番号に限る（D4 の反転と同じ直しが要る）

### D4（仮）. 待つ相手・止める相手の取り違えは直さない（Q4）

- 計測テストの台本の端末の待ちは今のとおり「どれかの端末」（`adb get-state`）。正常に進んだ後の後片付けは今のとおり `KEEP=1` でなければ `emulator-5554` を止める
- 失敗で止まるときだけ、D13 で名指しの `emu kill` をやめる（依頼の「早く失敗する」に伴う後片付けで、取り違えを直すものではない）
- 反転条件: Q4 が「直す」なら、空いている番号を選んで `-port` で起こし、`emu_wait_device <serial>`・gradle の `ANDROID_SERIAL`・後片付けをその番号に限る。gradle が `ANDROID_SERIAL` に従うかを先に本物で確かめる
- 他の端末（実機など）が繋がっていると、`-s` 無しの `adb get-state` が `more than one device` になり、端末の待ちは上限まで待って『端末が adb に現れない』で落ちる（取り違えを直さない D4 の範囲。偽の adb はこれを再現しない）

### D5（仮）. 壊してはいけないものは、偽の SDK の試験・既存の試験・本物の 1 回で固定する（Q5）

- `docs/changes/CH10.md` の「壊してはいけないもの」(1)〜(6) を spec の 6 つ目の Requirement にした。(1)〜(4) は D16 の偽の SDK の試験（6 段の結果・引数・`KEEP=1`・無いときの文言・AVD を作る）、
  (6) は `tools/test-usage-volume.sh` がこの change の後も rc=0、(1) の本物の流れは Task 6 で手元（kvm あり）で 1 回走らせる。(5) CI は台本を使わないので試験は無い
- 反転条件: Q5 で足す・外すものが返ったら、spec の 6 つ目の Requirement の Scenario と CH10.md の一覧を同じに直す

### D6（C1）. 起動の完了を待つ間にも、プロセスが終わればすぐ止まる

- `emu_wait_boot` は 1 回ごとに `EMU_PID` が生きているかを先に見る。回数の上限（既定 180 回 × 2 秒）は今のまま。`ASHIATO_EMULATOR_BOOT_ATTEMPTS` で回数を上書きできる（試験が小さい値を渡す）
- 上限を超えたら今の文言 `error: 起動が終わらない`（`:40` と同じ）

### D7（C2）. 失敗の終了コードは今と同じ（計測テストの台本 1・採寸の台本 2）。文言は `error: ` で始める

- 何が起きたか（起動の途中で終わった・上限を超えた・/dev/kvm を開けない・同じ AVD が動いている）と、出力の置き場を書く。spec の Scenario はこの文言の頭を見る

### D8（C3）. 上限と調べる場所は環境変数で上書きできる

| 環境変数 | 既定 | 何か |
|---|---|---|
| `ASHIATO_KVM_DEVICE` | `/dev/kvm` | 読み書きで開いてみる場所（verify-run の `EVIDENCE_KVM_DEVICE` と同じ考え。CI の検査の job では本物を開けないので、試験は開ける場所・開けない場所の両方を渡す） |
| `ASHIATO_EMULATOR_DEVICE_TIMEOUT` | 120 | 端末が adb に現れるまでの秒（D2） |
| `ASHIATO_EMULATOR_BOOT_ATTEMPTS` | 180 | 起動の完了を見る回数（2 秒ごと。D6） |
| `ASHIATO_ADB_CALL_TIMEOUT` | 10 | 起動の完了の待ちとアニメーションの設定の、1 回の `adb shell` の秒（D15） |

値が正の整数でなければ `error:` で rc=1（採寸の台本は 2）

### D9（C4）. 試験は `tools/test-android-emulator.sh` に置き、CI の検査の job で毎回走らせる

- 本物のエミュレータも /dev/kvm も要らない（D16）。`.github/workflows/ci.yml` の `rust` の job の `./tools/docker-tidy.sh --self-test` の後に 1 行
- verify-run はこの試験を名前と中身で「エミュレータが要る」と読む（`premises.md` 10 行目）。Codex の sandbox では申請が 1 回要る（AGENTS.md の手順）。名前を変えても中身の `adb` で当たるので、避けない
- Scenario の印は `echo "Scenario: <名前>"` を各場合の前に出す

### D10（C5）. /dev/kvm を開けないときの文言は「/dev/kvm」の語を必ず含む

- 試験は標準エラーに `/dev/kvm` の語があることを見る。`.harness2/scripts/evidence.py` がある木（手元）では、それを import して `INFRA_SIGNS` を文言に当てる（判定の正本を写さない）。CI には `.harness2` が無いので、語の有無だけを見る

### D11（C6）. 起こす前に /dev/kvm を読み書きで開いてみる

- 在ること（`[ -e ]`）を先に見て、在れば bash の `exec {fd}<>"$ASHIATO_KVM_DEVICE"` で開けるかを見て、すぐ閉じる。`<>` は無いファイルを作るので、在ることを先に見ないと「無い」を「開けた」と読む（spec-review R1）。
  これで verify-run の `kvm_usable`（`O_RDWR`、作らない）と同じ判定になる。文字デバイスかどうかは見ない（試験の開ける側は使い捨ての木の普通のファイル）。開けなければ起こさずに
  `error: /dev/kvm を開けない（調べた場所: <場所>）。エミュレータを起こさない。verify-run で走らせ直す（Codex なら AGENTS.md の「/dev/kvm」の手順で sandbox の外を申請する）`
- `/dev/null` を被せた形は開けてしまう（開いた後の ioctl で落ちる）。そのときは起こした後に D6・D17 で止まる

### D12（C7）. エミュレータの出力は `collector-android/app/build/outputs/emulator.log` に毎回書き直し、失敗したら末尾 40 行を出す

- 置き場が無ければ作る。正常時の出力に「kvm」の語は無い（`premises.md` 5 行目）ので、末尾を出しても正常な行が /dev/kvm の判定に掛からない
- verify-run の下でも作業ツリーの `build/` の下で、`$TMPDIR` の残りとしては数えられない

### D13（C8）. 失敗の経路の後片付けは、自分の起こしたプロセスだけを止める

- `emu_fail` は `EMU_PID` が生きていれば、その子（`pkill -TERM -P`。本物の起動は子の qemu を持つ）と本体に TERM を送り、5 秒待って残れば KILL。`KEEP=1` に依らない
- 計測テストの台本の `cleanup`（`trap … EXIT`）は、失敗の経路では `emulator-5554` へ `emu kill` を送らない（印の変数で分ける）。正常に計測テストまで進んだ後は今のまま
- 子の qemu が TERM で止まるかは本物で確かめていない（Task 6 の本物の確かめで、上限で止めた後に `-avd <AVD>` を持つ過程が残らないことを見る）

### D14（C9）. `error:` の行に次の手を書き、AGENTS.md に quiet-run 直の場合を 1 文足す

- AGENTS.md の「/dev/kvm（Codex のとき）」に「`scripts/quiet-run` で直に走らせて `error: /dev/kvm を開けない` で落ちたときも、`scripts/verify-run` で走らせ直す（そこで BLOCKED_INFRA になったものを申請する）」を足す
- ハーネス側の理由の文言（`evidence.py:420` の「環境の欠落を示す出力: /dev/kvm」）は変えない（範囲の外）

### D15（C10）. 1 回ごとの `adb shell` に `timeout` を掛ける

- 起動の完了の待ち（`getprop sys.boot_completed`）とアニメーションの設定 3 つ。既定 10 秒（D8）。返らないことが本物で起きるかは確かめていない
- 写真と動画・計測テストの段（`:44` 以降）の `adb` には掛けない（行を変えない。D1）

### D16. 正常な経路の試験は、台本を使い捨ての木に写し、隣の台本を偽物にして走らせる

- `tools/android-emulator.sh` は `cd "$(dirname "$0")/.."` で根に移り、`./tools/android-env.sh`・`./tools/android-media-fixtures.sh`・`collector-android/gradlew` を相対で呼ぶ。
  `tools/usage-volume.sh` も根に移るが、`android-env.sh` を読まず `ANDROID_HOME` を環境から受け取るので、試験は採寸の台本に `ANDROID_HOME=<偽の SDK>` を渡す（spec-review R4）。
  試験は `mktemp -d` の木に**本物の** `tools/android-emulator.sh`・`tools/usage-volume.sh`・`tools/emulator-start.sh` を写し、隣に偽の `tools/android-env.sh`（`ANDROID_HOME` を偽の SDK に向ける）・
  `tools/android-media-fixtures.sh`・`collector-android/gradlew`（結果の XML を 1 つ書く）を置く。台本に試験用の口を足さずに、起動から 6 段の計測テストと後片付けまで走る
- 偽の `emulator`（偽の SDK の `emulator/emulator`）は、受け取った引数を記録し、`FAKE_EMU`（`die` すぐ終わる・`sleep` 端末を見せず眠る・`late-die` 端末を見せた数秒後に終わる・`ok` 端末を見せて起動を終える）で振る舞いを変える。
  本物の qemu に当たる子を 1 つ起こす。試験は残りを、その回の使い捨ての木の場所（`pgrep -f "$tmp/"`）で数える（並走する worktree の同じ試験を数えない。spec-review R7）
- 各場合は `timeout 30` で包み（process group ごとに送る既定の形）、rc=124 を「見張りで打ち切った: <場合の鍵>」として別に出す。試験の `trap` は使い捨ての木を指す過程を全部止めて木を消す（spec-review R5）
- 偽の `adb`・`avdmanager` は PATH の先頭に置き、受け取った引数を記録する。端末の見え方は偽のエミュレータが書く状態のファイルで決まる
- 反転条件: 写した木で動かない台本の形（根の外のファイルを読むなど）が出たら、その台本に場所の環境変数を 1 つ足す（ほかの振る舞いは変えない）

### D17. 起動の途中で終わった出力が加速の不足を言うなら、/dev/kvm の語を含む行も出す

- 出力の全文に `requires hardware acceleration` か `permissions to use KVM` の句があれば（前者は `premises.md` 1 行目の実物の出力、後者は emulator 37.1.11 の文言。どちらも加速が使えないことだけを言う句で、広い語 `KVM`・`acceleration` は使わない。spec-review R12）、`error: エミュレータが /dev/kvm（ハードウェアの加速）を使えなかった。verify-run で走らせ直す（Codex なら AGENTS.md の「/dev/kvm」の手順）` を足す。
  /dev/null を被せた形（開けるが使えない）と、開くのを拒まれたときの `This user doesn't have permissions to use KVM`（`premises.md` 2 行目）の両方を、verify-run の出力の判定に乗せる
- 問いに依らない既定（C と同じ扱い。後から句を変えても失うものは無い）。Task 6.1 が本物の `ok`・`same-avd` の `emulator.log` でこの 2 つの句を数えて `real-proof.txt` に書く（0 のはず）
- 反転条件: 本物の sandbox で開くのを拒まれたときの文言が別の句だと分かったら、その句を足す

## Risks / Trade-offs

- [同じ AVD を `KEEP=1` で残して続けて走らせていた使い方が、止める手間 1 回ぶん増える] → 文言に止め方の 1 行を入れる（D3）。st54・ch09 の検証にも当たる（proposal の Impact）
- [120 秒の上限が、遅い機械の正常な起動を落とす] → 環境変数で延ばせる（D8）。実測は 15 秒
- [子の qemu が TERM で止まらず残る] → 5 秒後に KILL、Task 6 の本物で残りを数える（D13）
- [偽の SDK の試験は本物のエミュレータの文言の変化を捕まえない] → 文言に頼るのは D17 の補助の行だけで、止まること自体は終了と時間で決める。本物は Task 6 で 1 回通す
- [verify-run の下の Codex では、偽の試験にも申請が 1 回要る] → AGENTS.md の手順のまま（D9）

## Migration Plan

道具の台本だけ。戻すときは commit を戻せば元の形に戻る（DB・設定・配った物に残るものは無い）。

## Open Questions

（無い）
