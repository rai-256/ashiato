# CH10 の前提

外のサービスの書き出しに依る決定は無い。依るのは Android SDK のエミュレータ・adb の振る舞いと、ハーネスの `scripts/verify-run`（`evidence.py`）の判定。どちらも手元で走らせて確かめた（2026-11-05）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| /dev/kvm を使えないと、x86_64 のエミュレータは起動直後に非 0 で終わる（待ち続けない） | 「プロセスが死んだらすぐ失敗」で足りるか（上限に頼らずに早く落ちるか） | `unshare -rm` で /dev/kvm に /dev/null を被せ、台本と同じ引数で `emulator -avd ashiato-api35 … -read-only` を走らせた | 0 秒で rc=1（**再現は /dev/null を被せた形**。sandbox が開くこと自体を拒む形は再現していない。事故の環境で qemu が死んだのか固まったのかは見ていない）。`ERROR        \| x86_64 emulation currently requires hardware acceleration!` と `CPU acceleration status: Could not extract KVM version: Inappropriate ioctl for device` を出した | 実物 | 5 |
| エミュレータ自身の失敗の文言は「/dev/kvm」を含むとは限らない | 文言を verify-run の判定に掛けるには、台本が自分で決まった文言を出す必要があるか | 上と同じ実行の出力を `evidence.py` の `INFRA_SIGNS`（`/dev/kvm\|KVM is (not\|required)\|emulator: (ERROR\|FATAL)\|…`）と照らした | 掛からない。新しいエミュレータは `ERROR        \|` の形で出し、`emulator: ERROR` の形でない。この再現では「/dev/kvm」の語も出ない。開くのを拒まれたときの文言は emulator 37.1.11 の `libandroid-emu-shared.so` に `This user doesn't have permissions to use KVM (%s).` があり（deep-review が `strings` で確認）、こちらは「/dev/kvm」を含みうるが、出ると確かめてはいない | 実物（/dev/null を被せた形） | 5 |
| verify-run は、台本の中身に `adb` / `emulator -avd` / `android-emulator` があり /dev/kvm を開けないとき、**走らせる前に** BLOCKED_INFRA（「エミュレータが要るが /dev/kvm を開けない（走らせていない）」）にする | 台本の文言が効くのは、verify-run の外（`scripts/quiet-run` の ci_fix など）か、verify-run の前の検査をすり抜けた場合だけか | `.harness2/scripts/evidence.py:69`（`NEEDS_EMULATOR`）・`:245-258`（`kvm_usable`・`needs_emulator`）・`:367-368` | そのとおり。今回の 20 分の待ちは verify-run ではなく `quiet-run` の中で起きた。走らせた後は、rc≠0 のときに出力の全文を `INFRA_SIGNS` で探す（`:410` 付近） | 実物（コード） | 2 |
| /dev/kvm を開けるかは、`/dev/kvm` を読み書きで開いてみれば分かる（verify-run と同じ判定） | 台本が起動の前に /dev/kvm を調べる形にできるか | `evidence.py:245-250` の `kvm_usable` は `os.open(…, O_RDWR)`。手元で `emulator -accel-check` は kvm ありで rc=0「KVM (version 12) is installed and usable.」、/dev/null を被せて rc=13 | 開けるかどうかで判定できる。`-accel-check` も使えるが、終了コードの意味は公式文書で確かめていない | 実物 | 4 |
| 正常に立つとき、端末が adb に見えるまで約 15 秒、起動の完了（`sys.boot_completed=1`）まで約 35 秒（kvm あり・`-no-snapshot`・手元 16 コア） | 上限の値（Q2） | `-port 5600 -read-only` で台本と同じ引数で起動し、`adb -s emulator-5600 wait-for-device` と `getprop` の待ちを計った（1 回） | device 15 秒・boot 35 秒。正常時の出力 101 行に「kvm」の語は無い（失敗時に末尾を出しても、正常な行が /dev/kvm の判定に掛からない） | 実物 | 5（1 回だけ） |
| CI の計測テストは `tools/android-emulator.sh` を使わない（`reactivecircus/android-emulator-runner` が起動する） | 「CI の振る舞いは変えない」が何を守るか | `.github/workflows/ci.yml:290-303` | 使わない。CI の振る舞いはこの change で変わらない。この台本を使うのは手元と、tasks の検証（ST54・CH09 などが検証コマンドに書いている） | 実物（コード） | 2 |
| `tools/android-env.sh` は `ANDROID_HOME` を `$HOME/.local/opt/android-sdk` に固定で上書きする | 偽の emulator / adb で試験するとき、PATH だけで差し替えられるか | `tools/android-env.sh:4-7` | PATH だけでは足りない（台本は `$ANDROID_HOME/emulator/emulator` の有無を見て、`PATH` の先頭に `$ANDROID_HOME/emulator` を足す）。`HOME` を一時フォルダに向けて偽の SDK を置けば、台本を変えずに差し替えられる | 実物（コード） | 2 |
| 同じ「起動の出力を捨て、プロセスの死を見ない」形を `tools/usage-volume.sh:137-142` も持つ。ただし待ちは `tools/wait-android-boot.sh` で有限（180 回 × 2 秒） | 範囲の問い（Q1） | `grep -rn "emulator.*-avd\|wait-for-device"` を `tools/` に | 有限なので無期限には待たないが、死んでも約 6 分待ち、理由も残らない。`.harness2/scripts/record-run`（録画）は自分で起動とログを持つ（ハーネス側。この repo の範囲外） | 実物（grep） | 2 |
| 同じ AVD のエミュレータが既に動いていると、台本と同じ引数（`-read-only` なし）の 2 台目は約 3 秒で rc=1 で死ぬ | Q3（今は前からある端末で先へ進む。この change の後は何もしなければ失敗に変わる） | deep-review が使い捨ての AVD 置き場で 1 台目を立て、同じ AVD で 2 台目を起こした | 3 秒で rc=1、`FATAL        \| Running multiple emulators with the same AVD …`。この形も `INFRA_SIGNS` の `emulator: (ERROR\|FATAL)` に掛からない | 実物 | 5 |
| verify-run は、検証コマンドの字面か、指す台本の中身に `android-emulator` / `adb` があれば「エミュレータが要る」と読む | 偽の emulator / adb の試験（C4）が、Codex の sandbox では走らせる前に BLOCKED_INFRA になる | `evidence.needs_emulator(root, "bash tools/test-android-emulator.sh")` を呼んだ（deep-review） | True。既存の `tools/test-usage-volume.sh` も True | 実物（呼んだ） | 4 |
| CI の検査の job では /dev/kvm を開けない（開けるようにする udev の規則は計測テストの job だけ） | 試験が「開ける」「開けない」の両方の経路に届くか（C3） | `.github/workflows/ci.yml:289-294` | 計測テストの job だけ。試験は調べる場所を差し替える口が要る | 実物（コード） | 2 |
| verify-run が**走らせた後**に出力で判定したときの理由は「環境の欠落を示す出力: /dev/kvm」で、AGENTS.md:55 の字面「/dev/kvm を開けない」と違う。走らせる前の判定の理由は「エミュレータが要るが /dev/kvm を開けない（走らせていない）」 | 落ちた後に agent が何をするか（C6） | `.harness2/scripts/evidence.py:367-368`・`:419-420`、`AGENTS.md:55` | そのとおり。ハーネス側の文言は repo の範囲の外 | 実物（コード） | 2 |

## 聞かずに決めた既定（C。spec の段が design の D に写す）

- C1. 起動の完了（`sys.boot_completed`）を待っている間にも、エミュレータのプロセスが死んだらすぐ失敗で返る（依頼の「待っている間に」を両方の待ちに当てる）。完了の待ちの上限（180 回 × 2 秒）は今のまま
- C2. 失敗の終了コードは今の台本の失敗と同じ 1。文言は `error: ` で始め、何が起きたか（死んだ・上限を超えた・/dev/kvm を開けない）と、出力の置き場を書く
- C3. 上限は環境変数で上書きできる（既定は Q2）。調べる /dev/kvm の場所も環境変数で差し替えられる（verify-run の `EVIDENCE_KVM_DEVICE` と同じ考え）。試験は小さい値と、開ける場所・開けない場所の両方を渡して走らせる
- C4. 試験は `tools/test-android-emulator.sh`（新しいファイル）に置き、偽の emulator / adb / avdmanager を一時フォルダの偽の SDK（`HOME` を向け直す）に置いて走らせる。本物のエミュレータも /dev/kvm も要らないので、CI の検査の job（`./tools/check-*.sh --self-test` が並ぶ所）で毎回走らせる。verify-run はこの試験を名前と中身で「エミュレータが要る」と読むので、Codex の sandbox では申請が 1 回要る（AGENTS.md の手順。ハーネス側の判定は直さない）
- C5. /dev/kvm を開けないときの文言は「/dev/kvm」の語を必ず含む（verify-run の `INFRA_SIGNS` の最初の選択肢）。試験は文言にその語があることを見る。`.harness2` があるとき（手元）は `.harness2/scripts/evidence.py` の `INFRA_SIGNS` を import して文言に当てる（判定の正本を写さない）。CI には `.harness2` が無いので、そこでは語の有無だけを見る
- C6. 起動の前に /dev/kvm を読み書きで開いてみて、開けなければ起動せずに `error: /dev/kvm を開けない（…）` で止まる（deep-review R8。依頼を満たすのはこの形だけ）。起動した後に死んだときは、別にエミュレータの出力の末尾を出す
- C7. エミュレータの出力は `collector-android/app/build/outputs/emulator.log` に毎回書き直し、失敗したら末尾 40 行を標準エラーに出す（R9）
- C8. 失敗の経路の後片付け: 台本が起こしたプロセスがまだ生きていれば、`KEEP=1` に依らず PID で止める。自分のプロセスが死んだ・起動に失敗したときは、名指しの `emulator-5554` を止めない（他人の端末を止めない。R2・R11）
- C9. `error:` の行に次の手を書く（`/dev/kvm` のときは「verify-run で走らせ直す。Codex なら AGENTS.md の手順で sandbox の外を申請する」）。AGENTS.md の「/dev/kvm（Codex のとき）」に「quiet-run で直に走らせて /dev/kvm で落ちたときも verify-run で走らせ直す」を足す。ハーネス側の理由の文言（`evidence.py:420`）は変えない（やらないこと。R6）
- C10. 1 回ごとの `adb shell`（起動の完了の待ちとアニメーションの設定）に `timeout` を掛ける（R10。返らないことが起きるかは確かめていない）
