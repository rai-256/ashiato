# CH10 spec-review（spec --draft。第 1 回の先読み）

読んだもの: `proposal.md`・`specs/emulator-startup/spec.md`・`design.md`・`tasks.md`・`deep.md`・`premises.md`・`review/deep.md`・`deep-questions.json`・`docs/changes/CH10.md`、
触る実物 `tools/android-emulator.sh`・`tools/usage-volume.sh`・`tools/wait-android-boot.sh`・`tools/test-usage-volume.sh`・`tools/android-env.sh`・`tools/android-media-fixtures.sh`・`AGENTS.md`・`.github/workflows/ci.yml`・`docs/testing.md`・`.harness2/scripts/evidence.py`、
走っている枝 `origin/feat/st53-license-bundle`・`origin/feat/st54-dev-side-by-side`・`origin/feat/st55-server-installer`（`origin/main` = 11c0c043 との差分）。
`deep.md` の `（答え待ち・推奨: …）` は推奨の答えとして読んだ。

## 機械の検査

```
$ openspec validate ch10-emulator-start-timeout --strict
Change 'ch10-emulator-start-timeout' is valid                                   rc=0
$ python3 scripts/check_chain.py .
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                                      rc=0
$ python3 scripts/check_scenarios.py . ch10-emulator-start-timeout
[FAIL] 担保の無い Scenario: … （emulator-startup の 18 本すべて）
scenarios: FAIL (担保なし 18 件 / 名無しの確認待ち 0 件)                       rc=1
$ python3 scripts/review_triage.py . ch10-emulator-start-timeout
指摘 11 件 / 仮決め 0 件 / 要件へ戻すもの 0 件   triage: OK                    rc=0
$ python3 scripts/spec_overlap.py candidates . ch10-emulator-start-timeout
[]                                                                              rc=0
```

- check_scenarios の FAIL 18 件は、実装の前（`tools/test-android-emulator.sh` がまだ無い）だから。指摘にしない。18 本の名前は tasks の Task 1.1・2.2・3.1・4.1 の `echo "Scenario: …"` と一字一句合っている（突き合わせた）
- 付け足し: `python3 scripts/provisional.py list openspec/changes/ch10-emulator-start-timeout` → `[]`（R14）
- check_scenarios の `[warn] spec に無い Scenario を指す印` 24 行は repo の既存のもので、この change を指すものは無い（上の写しでは省いた）
- この review を置いた後の `review_triage.py` は FAIL（16 件）になる。理由はどれも「処置が無い」で、処置は呼び出し元が付ける

## R1. D11 の開け方（`exec {fd}<>`）は無いファイルを作ってしまうので、「存在しない場所を渡す」Scenario は D11 のとおりに作ると通らない（無いのに開けたことになる）
- 成果物: openspec/changes/ch10-emulator-start-timeout/design.md（D11）・specs/emulator-startup/spec.md（1 つ目の Requirement の 2 本の Scenario、7 つ目の「/dev/kvm を開けなければ」）
- 根拠: design.md:108「bash の `exec {fd}<>"$ASHIATO_KVM_DEVICE"` … verify-run の `kvm_usable` と同じ `O_RDWR`」。手元で `d=$(mktemp -d); ( exec {fd}<>"$d/nope" && echo opened )` → `opened` と出て、`$d/nope`（0 バイト）ができた。親のディレクトリが無いときだけ `No such file or directory` で rc=1。bash の `<>` は O_RDWR|O_CREAT で、`.harness2/scripts/evidence.py:247` の `os.open(…, os.O_RDWR)`（O_CREAT 無し）と同じではない。spec.md:16・:142 の WHEN は「`ASHIATO_KVM_DEVICE` に存在しない場所を渡す」。
  同じ形は本物でも起きる: /dev/kvm が無い機械で /dev に書ける（root の container など）と、`/dev/kvm` という普通のファイルを作って「開けた」と読む
- kind: technical
- 提案: D11 の確かめを「在る（`[ -e ]`）かつ O_CREAT 無しで読み書きで開ける」に直す（`python3 -c 'os.open(…, O_RDWR)'` でも）。文字デバイス（`[ -c ]`）まで求めるなら、tasks.md:51 の「開ける場所（使い捨ての木の中の普通のファイル）」は通らなくなるので、開ける側の試験には `/dev/null`（D11 自身が「開けてしまう」と言う形）を渡すと書き添える。Scenario の WHEN に「親のある・無いに依らず」と、事故の形に近い「在るが開けない場所（mode 000 のファイル）」の場合を足す（`root` で走るときは後者を飛ばす旨も）
- 処置: fixed D11 —— 在ることを先に見てから開く形に直した。spec の 1 つ目の Scenario に「調べた場所にファイルは作られていない」、権限 000 の場合の Scenario を足した（root では飛ばす）。開ける側は使い捨ての木の普通のファイルのまま（文字デバイスは見ない）

## R2. /dev/kvm を開けない経路で、今の `trap cleanup EXIT` が先に張られていると `emulator-5554` へ `emu kill` が飛ぶ。spec がこの経路の「止めない」を言っていない
- 成果物: openspec/changes/ch10-emulator-start-timeout/specs/emulator-startup/spec.md（1 つ目と 5 つ目の Requirement）
- 根拠: `tools/android-emulator.sh:30-31` は起動の前に `trap cleanup EXIT` を張り、`KEEP` が無ければ `adb -s emulator-5554 emu kill` を送る。5 つ目の Requirement の WHEN（spec.md:80）は「端末が現れない・起動が終わらない・起動の途中で終わった」の 3 つだけで、/dev/kvm を開けない・環境変数の値が正の整数でない（design D8 の `error:` rc=1）を含まない。
  1 つ目の Requirement の Scenario（spec.md:15-24）には「偽の adb に `emu kill` が届いていない」が無い（同じ AVD の Scenario spec.md:72 には有る）。D1・D11 も確かめを trap の前に置くか後に置くかを書いていない。事故の経路（sandbox の中の quiet-run 直）そのものが、他の worktree の `emulator-5554` を止める形になりうる
- kind: technical
- 提案: 5 つ目の Requirement の WHEN に「/dev/kvm を開けない・設定の値が誤り」を足す（起こす前に止まるときは何も止めない）。1 つ目の Scenario に `AND 偽の adb に emu kill は届いていない` を足す
- 処置: fixed specs/emulator-startup/spec.md —— 5 つ目の Requirement に「起こす前に止まるときは何も止めない」を書き、1 つ目の Scenario に「emu kill は届いていない」を足した

## R3. 本人の決定の数値（Q2 の 120 秒）と、本文の数値（40 行・10 秒・180 回 × 2 秒）が Scenario に無い。実装が既定を変えても spec が落ちない
- 成果物: openspec/changes/ch10-emulator-start-timeout/specs/emulator-startup/spec.md（2・3 つ目の Requirement）
- 根拠: 120 秒は spec.md:47 の本文だけ。Scenario（spec.md:53-59）は `ASHIATO_EMULATOR_DEVICE_TIMEOUT=2`・回数 2・1 秒で上書きした値しか見ない。「末尾（40 行まで）」（spec.md:30）は Scenario（spec.md:38）が「最後の行がある」だけを見る。既定 10 秒（:49）・180 回（:50）も同じ
- kind: technical
- 提案: 上書きしないで走らせたとき、台本が使う上限を 1 行出す（例 `== 端末を最大 120 秒待つ`）形にし、「既定のまま走らせると、その行に 120 がある」Scenario を足す（待たずに既定値を観測できる）。40 行は「偽のエミュレータが 100 行出すと、標準エラーに出る出力は最後の 40 行で、41 行目より前は無い」の形で 1 本
- 処置: fixed specs/emulator-startup/spec.md —— 待ち始めに上限の行を出す形にし（D2）、「上限を渡さなければ最大 120 秒待つ」と「出力の末尾は 40 行まで」の Scenario を足した。10 秒・180 回は誤った値の Scenario と本文の環境変数で縛る

## R4. D16 の「`tools/usage-volume.sh` も `./tools/android-env.sh` を相対で呼ぶ」は誤り。採寸の 3 本の Scenario は偽の `android-env.sh` では偽の SDK に向かない
- 成果物: openspec/changes/ch10-emulator-start-timeout/design.md（D16）・tasks.md（Task 1.1・4.1）
- 根拠: design.md:135。`tools/usage-volume.sh` 全体に `android-env.sh` の読み込みは無く、`$ANDROID_HOME` と PATH を呼び出し元の環境から使う（`tools/usage-volume.sh:131`〔`emulator_sample` の `"$ANDROID_HOME/emulator/emulator"`〕。`tools/test-usage-volume.sh:54-56` が `ANDROID_HOME=/nonexistent`・`env -u ANDROID_HOME` で頼っている）。
  試験が `ANDROID_HOME` を自分で渡し忘れると、手元では本物の SDK（`~/.local/opt/android-sdk/emulator/emulator` は在る）や、`ANDROID_HOME` を持つ CI の runner の SDK に当たる
- kind: technical
- 提案: D16 に「採寸の台本は `ANDROID_HOME` を環境で受け取るので、試験は `ANDROID_HOME=<偽の SDK>` を渡して走らせる」を書き、Task 4.1 の本文にも 1 句足す。台本は変えなくてよい
- 処置: fixed D16 —— 採寸の台本には ANDROID_HOME を渡すと書き、Task 1.1・4.1 にも足した

## R5. 「30 秒以内」を `SECONDS` で測ると、台本が止まらなかったときに試験自体が止まらない。Task 2.3 の旧版に当てる形はまさにそれで、見張りの殺し方と残りの片付けが決まっていない
- 成果物: openspec/changes/ch10-emulator-start-timeout/tasks.md（Global Constraints の時間・Task 2.3）
- 根拠: tasks.md:18「`SECONDS` で測る」は、終わった後に経過を測るだけで打ち切らない。tasks.md:54-55 は偽の adb の `wait-for-device` を「端末が見えるまで返らない」にし、旧版（`git show 11c0c043:tools/android-emulator.sh` の `:35`）は無期限に待つ ——「30 秒の見張り」の実体（`timeout 30` か、自前の kill か）が書かれていない。
  打ち切ったときに、偽の adb（`wait-for-device` のループ）と偽のエミュレータが孫として残る形（KILL なら台本の trap は走らない）の片付けも、Global Constraints の「偽のエミュレータの過程も trap で止める」だけでは偽の adb に届かない。CI の検査の job で回すので、止まらないと job の上限まで掛かる
- kind: technical
- 提案: 各場合を `timeout 30`（process group ごと送る既定の形。`--foreground` を使わない）で包み、rc=124 を「見張りで打ち切った」として別に出す。Task 2.3 の 1 つ目の検証は rc≠0 でなく「見張りで打ち切った」の文言で見る（R6）。試験の trap で使い捨ての木を指す過程を全部止める
- 処置: fixed tasks.md —— Global Constraints に、どの場合も timeout 30 で包む・rc=124 は「見張りで打ち切った: <鍵>」・trap で木を指す過程を全部止める、を書いた（D16 にも）

## R6. Task 2.3 の「旧版では落ちる」の検証は、落ちた理由を見ていない。`git show` の失敗や場合の名前の取り違えでも通る
- 成果物: openspec/changes/ch10-emulator-start-timeout/tasks.md（Task 2.3）
- 根拠: tasks.md:56 の 1 つ目は `>/dev/null 2>&1; [ $? -ne 0 ]`。`ASHIATO_TEST_ONLY=dies` の `dies` は Scenario の名前でも、どこかで定義された鍵でもない（D16 の `FAKE_EMU=die` とも違う）。旧版が「30 秒の見張り」で落ちたのか、写し損ね・知らない鍵・別の assert で落ちたのかを区別しない
- kind: technical
- 提案: 鍵と Scenario の対応を Task 2.2 に表で置き、知らない鍵は rc=2 で止める。1 つ目の検証を `… 2>&1 | grep -q '<見張りで打ち切った文言>'` にする（2 つ目の rc=0 と合わせて、旧版で止まらず新版で止まることを両方から言う）
- 処置: fixed 2.3 —— 場合の鍵と Scenario の表を Task 2.2 に置き、知らない鍵は rc=2。旧版の検証を「見張りで打ち切った: dies」の文言で見る形にした

## R7. 偽のエミュレータの残りを `pgrep -f ch10-fake-emulator` で数えると、並走する worktree の同じ試験を数える
- 成果物: openspec/changes/ch10-emulator-start-timeout/specs/emulator-startup/spec.md（spec.md:86-89）・design.md（D16）・tasks.md（Task 2.2・6.1）
- 根拠: spec.md:88 は「印 `ch10-fake-emulator` を持つもの」、design.md:139 は `pgrep -f` で機械全体を数える。この試験は CI に加えて verify-run で各 Story・change の作業場所から走り（proposal の Why「並走する Story と change が計測テストを検証に使う」）、`sleep` の場合は同時に走る別の試験の偽のエミュレータが生きている。
  Task 6.1 の `timeout` の「`-avd ashiato-api35` を持つ過程が 0 本」（tasks.md:86）も、別の worktree の本物の同じ AVD を数える。
  また Requirement の本文（spec.md:81）は「プロセス（とその子）」だが、偽のエミュレータの子（`sleep` など）に印が無ければ子の残りは数えない
- kind: technical
- 提案: 印をその回の `mktemp -d` の場所（`pgrep -f "$tmp/"`）にし、偽のエミュレータは印を持つ子を 1 つ起こす（本物の qemu の子に当たる）。Task 6.1 は台本が起こした PID（とその子）を記録して数える
- 処置: fixed D16 —— 残りはその回の使い捨ての木の場所（pgrep -f "$tmp/"）で数え、偽のエミュレータは子を 1 つ起こす。spec の Scenario も「その回の…とその子」に直した。Task 6.1 は PID と -avd の両方で数える

## R8. 5 つ目の Requirement（失敗のとき名指しの端末を止めない）が採寸の台本にも当たるのか書かれておらず、D1 の採寸の台本の形はそれに反する
- 成果物: openspec/changes/ch10-emulator-start-timeout/specs/emulator-startup/spec.md（5・7 つ目の Requirement）・design.md（D1）
- 根拠: spec.md:78-84 の主語は「台本」で、Scenario（:86-93）は計測テストの台本だけ。design.md:43 は採寸の台本の `started` を「端末が現れてから置く」—— 端末が現れた後に起動が終わらない・途中で終わったときは、`tools/usage-volume.sh:103`（`cleanup_main`）が `adb -s emulator-5554 emu kill` を送る。これは 5 つ目の Requirement の「名指しの端末へ `emu kill` を送らない」の WHEN（起動が終わらない・起動の途中で終わった）に当たる
- kind: technical
- 提案: 5 つ目の Requirement の主語を「計測テストの台本」に絞り、7 つ目に採寸の台本の失敗の後片付け（自分の PID を止める・`started` の扱い）を書いて Scenario を 1 本足す。あるいは D1 を「`started` は起動が終わってから置く」に直す
- 処置: fixed specs/emulator-startup/spec.md —— 5 つ目の主語を計測テストの台本に絞り、7 つ目の本文に採寸の台本の失敗の後片付け（自分のプロセスを止める・端末が現れる前は名指しで止めない）を書いて、1 つ目の Scenario に AND を足した。D1 の started の扱いはこれと合う

## R9. Requirement の本文が言い、Scenario が言っていないこと
- 成果物: openspec/changes/ch10-emulator-start-timeout/specs/emulator-startup/spec.md
- 根拠:
  - spec.md:66「別の AVD のエミュレータ…が繋がっているだけのときは、今のとおり起こす」—— Scenario（:74-76）は実機だけ。別の AVD の `emulator-5556` を見せる場合が無い（`emu avd name` の比較を外しても、`emulator-*` を全部断る実装でも通る）
  - spec.md:30「出力の全文の置き場を標準エラーに出す」・:31「起動のたびに書き直して」—— Scenario は置き場の表示も、2 回目で前の出力が消えることも見ない
  - spec.md:49「起動の後のアニメーションの設定の問い合わせ…上限で打ち切る」—— Scenario が無い
  - spec.md:99「アニメーションを切り、計測テストの写真と動画を入れ」—— 正常な経路の Scenario（:105-110）は偽の adb の `settings put` も偽の `android-media-fixtures.sh` の呼び出しも見ない
- kind: technical
- 提案: 別の AVD の 1 本は Scenario を足す（同じ AVD の判定の対になる）。ほかは正常な経路の Scenario の THEN に偽の adb の記録で足すか、本文から外す
- 処置: fixed specs/emulator-startup/spec.md —— 別の AVD の端末を実機の Scenario に足した（other-avd の emulator-5556）。置き場の表示は起動直後に終わる Scenario の THEN に足した。アニメーションの設定の上限は本文から外した（design D15 にだけ残す実装の決定）

## R10. 1 つの Scenario に独立した主張を束ねている所と、ほかの Scenario に頼る WHEN
- 成果物: openspec/changes/ch10-emulator-start-timeout/specs/emulator-startup/spec.md
- 根拠: spec.md:105-110 は rc=0・引数・6 段の XML・`emu kill` の 4 つを 1 本の印で持つ（「壊してはいけないもの」の (1)(2) をまたぐ。印は 1 つなので、assert が 1 つ欠けても check_scenarios は気づかない）。spec.md:22「上と同じく」は別の Scenario の WHEN に頼り、単独で読めない。
  また spec.md:24 の AND は「`.harness2` がある木では」の条件つきで、CI（`.harness2` が無い）では :23 だけになり、:18 の 1 本目と同じことしか見ない
- kind: technical
- 提案: :105 を「引数」と「6 段の結果と後片付け」の 2 本に分ける（Task 1.2 の `--self-test` の壊し方とも 1 対 1 になる）。:22 の WHEN を自分で書き切る。:24 の条件つきは「手元でだけ確かめる」と明記するか、正規表現の写しを試験に持たない理由を本文に書く
- 処置: fixed specs/emulator-startup/spec.md —— 引数と 6 段・後片付けを 2 本に分け（後片付けの THEN にアニメーションの設定を足した）、「上と同じく」の WHEN を書き切り、INFRA_SIGNS の AND は手元でだけ確かめると理由つきで書いた

## R11. D8 の観測できる振る舞い（環境変数の名前・誤った値で `error:` と rc）が design にだけある
- 成果物: openspec/changes/ch10-emulator-start-timeout/design.md（D8）
- 根拠: design.md:87-94 の表と「値が正の整数でなければ `error:` で rc=1（採寸の台本は 2）」。spec は `ASHIATO_KVM_DEVICE`・`ASHIATO_EMULATOR_DEVICE_TIMEOUT` は名を出すが、`ASHIATO_EMULATOR_BOOT_ATTEMPTS`・`ASHIATO_ADB_CALL_TIMEOUT` は「回数と 1 回の上限は環境変数で上書きできる」（spec.md:50）だけで、Scenario（:58）は名前の無い「回数を 2 回・1 回の上限を 1 秒にして」。誤った値の振る舞いは spec にも Scenario にも無い。archive で正典に残らない
- kind: technical
- 提案: 3 つ目の Requirement の本文に 2 つの名前を書き、「誤った値（`0`・`abc`）を渡すと、起こさずに `error:` で rc=1（採寸は 2）」の Scenario を 1 本足す（R2 の「起こす前に止まるときは何も止めない」と合わせられる）
- 処置: fixed specs/emulator-startup/spec.md —— 3 つ目の本文に 3 つの環境変数の名前と、誤った値で起こさずに rc=1（採寸は 2）を書き、Scenario を 1 本足した

## R12. D17 の `acceleration` の語は、正常な出力・別の理由で終わった出力に出ないと確かめていない。出れば別の理由の失敗が verify-run で環境の欠落（BLOCKED_INFRA）に読まれる
- 成果物: openspec/changes/ch10-emulator-start-timeout/design.md（D17）・premises.md
- 根拠: premises.md:11 は正常時の出力 101 行に「kvm」の語が無いことだけを見た。design.md:147 の反転条件は「今の 1 回の実測では出ない」とこの語（`KVM`・`acceleration`）全体に言っている。同じ AVD の 2 台目が `FATAL | Running multiple emulators…` で終わる出力（premises.md:15。D3 の「adb に見えない同じ AVD」はここに落ちる）に `acceleration` を含む行があるかも見ていない。
  あれば D17 の行（`/dev/kvm` の語）が出て、`.harness2/scripts/evidence.py:71` の `INFRA_SIGNS` の `/dev/kvm` に当たり、本当の理由（同じ AVD）が BLOCKED_INFRA で隠れる
- kind: premise
- 提案: Task 6.1 の `ok` と `same-avd`（と、同じ AVD を adb に見える前に起こす場合）で、`emulator.log` に `KVM|acceleration` が出るかを `real-proof.txt` に記録させる。出るなら反転条件のとおり語を絞る
- 処置: rejected: D17 が当てる語を、実物で見た句 `requires hardware acceleration`（premises.md 1 行目）と emulator 37.1.11 の `permissions to use KVM` に絞った。どちらも加速が使えないことだけを言う句で、「広い語が別の理由の出力に出ないか」という前提に依る決定が無くなった。Task 6.1 は念のため本物の出力で数える

## R13. 走っている st54 が 3 つ目の同じ形の起動（`tools/android-side-by-side.sh`）を足していて、Q1 の前提（同じ形は採寸の台本だけ）が merge の後は崩れる。proposal の st54 の記述も実物と違う
- 成果物: openspec/changes/ch10-emulator-start-timeout/proposal.md（Impact の st54）・deep.md（Q1）・premises.md:14
- 根拠: `git show origin/feat/st54-dev-side-by-side:tools/android-side-by-side.sh` の :25-26 が `emulator -avd "$avd" … >/dev/null 2>&1 &` と上限の無い `adb wait-for-device`（事故と同じ形。:34 にも `adb -s "$serial" wait-for-device`）。st54 の tasks.md の 9.3（`origin/feat/st54-dev-side-by-side` の :135-137）の検証は `tools/android-side-by-side.sh` で、`tools/android-emulator.sh` は「同じ AVD」の参照に出るだけ。
  proposal.md:47「st54: `tasks.md` が `tools/android-emulator.sh` を検証コマンドに使う」は実物と合わない（検証に使うのは ch09 の Task 10.1。`openspec/changes/ch09-load-tolerant-timing-tests/tasks.md:137`）。spec の Requirement は重ならないので spec_overlap の候補には出ない
- kind: defer
- 提案: proposal の st54 の行を直し、「st54 の `tools/android-side-by-side.sh` は同じ形で無期限に待つ。この change の範囲の外（st54 の merge の後に、部品 `tools/emulator-start.sh` を使う change で直す）」を「やらないこと」に書く。本人に Q1 を聞く資料にも 1 句入れる（範囲の問いの前提が変わる）
- 処置: fixed proposal.md —— st54 の行を実物（tools/android-side-by-side.sh:25-26 が同じ形）に直し、検証に使うのは ch09 の Task 10.1 と書き直した。side-by-side の台本は design の Non-Goals と proposal の「やらないこと」に、st54 の merge の後に部品を使う別の change で直すと書いた（走っている Story へ差し戻さない）

## R14. D17（仮）は問いに依らない AI の仮の決定だが、仮の台帳に載っていない
- 成果物: openspec/changes/ch10-emulator-start-timeout/design.md（D17）
- 根拠: design.md:143 の見出しに（仮）と反転条件があり、proposal.md:16 も「D17（仮）」。D1〜D5（仮）は Q1〜Q5 の答えに従うが、D17 に対応する Q は無い。`python3 scripts/provisional.py list openspec/changes/ch10-emulator-start-timeout` → `[]`。上流の書き手も、失わない側で仮に進めた決定は `provisional.py add` に載せる（`.harness2/skills/story-upstream/SKILL.md:311-312`）。台帳に無いと deep-review の確かめ（`provisional_review`）も確認バッチの仮の一覧にも出ない
- kind: technical
- 提案: 書き手が `provisional.py add … --id D17 --keeps … --cost …` で載せるか、仮でない既定（premises の C と同じ扱い）として（仮）を外す
- 処置: fixed D17 —— 問いに依らない既定（C と同じ扱い）にし、見出しの印を外した。反転条件は残した

## R15. tasks の検証の抜け: `--self-test` を直した後に走らせ直していない・行番号に頼る grep・sandbox の中で申請が要る検証が書かれていない
- 成果物: openspec/changes/ch10-emulator-start-timeout/tasks.md
- 根拠:
  - `tools/test-android-emulator.sh --self-test` の検証は Task 1.2（直す前の台本）だけ。Task 2.1 が `cleanup` と起動の行を書き換えた後（`-no-snapshot` は部品への引数に、`KEEP` の扱いは失敗の経路と分かれる）に、壊し方が当たり続けるかを Task 2〜6 のどれも走らせない（Task 5.2 は ci.yml を grep するだけ）
  - Task 5.1（tasks.md:78）の `sed -n 177p docs/testing.md` は行番号に頼る。今は st53 の `docs/testing.md` の変更が :221 なので崩れないが、177 行より前に 1 行入れば落ちる
  - `evidence.needs_emulator` に当てると、`bash -n tools/android-emulator.sh`（Task 2.1）と Task 5.1 の grep（文字列に `android-emulator` を含む）も True。Codex の sandbox ではこの 2 つも走らせる前に BLOCKED_INFRA になり申請が要るが、tasks.md:19 は試験だけを挙げる
- kind: technical
- 提案: Task 4.1 か 6.2 の検証に `tools/test-android-emulator.sh --self-test` rc=0 を足す。Task 5.1 は `grep -n 'android-emulator.sh' docs/testing.md | grep -q '同じ AVD'` のように中身で探す。tasks.md:19 に「`android-emulator` の字面を含む検証（`bash -n`・grep）も申請が 1 回要る」を足す
- 処置: fixed 6.2 —— --self-test を 6.2 の検証に足した。Task 5.1 の grep は行番号でなく行の中身で探す。Global Constraints に android-emulator の字面を含む検証も申請が要ると書いた

## R16. 参照の誤り（行番号・置き場）
- 成果物: openspec/changes/ch10-emulator-start-timeout/design.md・deep-questions.json・review/deep.md
- 根拠: design.md:3「聞かずに決めた既定（`deep.md` の C1〜C10」—— C1〜C10 は `premises.md:22-31` にあり、`deep.md` には Q1〜Q5 しか無い。deep-questions.json の Q5 の context と review/deep.md:80 の `tools/test-usage-volume.sh:270-275`（`wait-android-boot.sh` の有限の待ちの試験）—— ファイルは 90 行で、その試験は :71-76。本人が Q5 を読む GO 資料に誤った行が出る
- kind: technical
- 提案: 出所を `premises.md` に、行を `:71-76` に直す

## 観点ごとの確かめ

- 観点 1（deep の決定が正典に写っているか）: Q1 → 7 つ目の Requirement（3 本）、Q3 → 4 つ目（文言・止め方も推奨と同じ）、Q4 → proposal の「やらないこと」・design の Non-Goals と、5 つ目の Requirement の「正常な後片付けは今のまま」、Q5 → 6 つ目（5 本。(5) CI は試験無しの理由が D5 にある）。当初案を覆したもの・要件へ戻すものは無い（change なので `docs/requirements.md` は触らない）。数値は R3
- 観点 2: R3・R9・R10。WHEN / THEN は終了コード・標準エラーの文字列・偽の道具の記録で、観測できる言葉になっている
- 観点 3: R11（D8）。specs に関数名（`emu_*`）は無い。台本の名前・環境変数・ログの置き場は道具の外の口なので可。ただし spec.md:24 がハーネス内部の名前 `INFRA_SIGNS` に頼る（ハーネス側が名前を変えると Scenario が読めなくなる）のは R10 の条件つきの AND と合わせて直すのがよい。proposal の Capabilities と `specs/emulator-startup/` は一致。change なので INDEX の表には足さない（proposal.md:32 に明記）
- 観点 4: R5・R6・R15。依存順（Task 1 の今の台本の固定 → Task 2 の直し → Task 2.3 の旧版での反証）は正しい。人間の確認待ちは無く、Task 6 は手元の kvm で機械が走らせる形で書かれている
- 観点 5（change）: `docs/changes/CH10.md` の「完了の判定」は空でなく、tasks・specs と合っている。「壊してはいけないもの」(1)〜(4)(6)(7) は Scenario になっている（(5) は台本を使わない CI のことで試験が無い理由が D5 にある）。「依頼（逐語）」の 6 項と 2 つの検証はどれも Requirement と Scenario に当たり、越えるもの（採寸の台本・同じ AVD・D13・D15）は Q1・Q3・premises の C に決定がある。
  付記（指摘にしない程度）: (6) の「`tools/wait-android-boot.sh` の有限の待ち」は、D1 の後は誰も呼ばない台本の性質になり（`grep -rn wait-android-boot` で使い手は `tools/usage-volume.sh` だけ）、採寸の台本の有限の待ちを守るのは部品の方になる。範囲の外への越えは R13
- 観点 6: 該当なし（spec --draft。answers.md は書かない）

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

付記（Requirement ではなくファイルの重なり。呼び出し元の問いへ）: `.github/workflows/ci.yml` の差分は st54 が :98 の後に 9 行、st55 が :52・:113 の後・:385 以降、st53 が :215・:360 以降。CH10 は :124（`- run: ./tools/docker-tidy.sh --self-test`）の直後で、どれとも同じ行を変えない（st55 の :113 の後の 3 行とは 11 行離れる）。
`docs/testing.md` は st53 が :221 の 1 行、CH10 が :177 の 1 行で別の行。`tools/android-emulator.sh`・`tools/usage-volume.sh`・`tools/wait-android-boot.sh`・`tools/test-usage-volume.sh`・`AGENTS.md` は 3 本のどれも触らない（`git diff --stat origin/main...origin/<枝>`）。振る舞いの重なりは R13（st54 の新しい台本）だけ
- 処置: fixed design.md —— C の出所を premises.md に直した。deep-questions.json の Q5 の context（:270-275）は第 1 回の問いとして既に描いて渡す版なので直さない（spec と CH10.md は行番号に頼らず書いてある）

