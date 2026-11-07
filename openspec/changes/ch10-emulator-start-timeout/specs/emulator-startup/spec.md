## Purpose

手元でエミュレータを起こす台本（計測テストの `tools/android-emulator.sh` と、利用状況の採寸の `tools/usage-volume.sh`）が、エミュレータを起こせないときに無期限に待たず、早く・理由つきで止まること。/dev/kvm を開けない・起動の途中で終わった・端末が現れない・同じ AVD が既に動いている、のどれでも止まり方が決まっていて、正常に立つときの振る舞いは変わらない。並走する Story と change が同じ機械で計測テストを回し続けるための、開発の道具の約束。

## ADDED Requirements

### Requirement: /dev/kvm を開けなければ、エミュレータを起こさずに理由つきで止まる

WHEN 計測テストの台本（`tools/android-emulator.sh`）がエミュレータを起こす前に、/dev/kvm（環境変数 `ASHIATO_KVM_DEVICE` で調べる場所を差し替えられる。既定は `/dev/kvm`）が無いか、在っても読み書きで開けないとき、
THE SYSTEM SHALL エミュレータを起こさず、`error: /dev/kvm を開けない` で始まる行を標準エラーに出して rc=1 で終わる。
その行は、調べた場所と、次の手（verify-run で走らせ直す。Codex なら AGENTS.md の「/dev/kvm」の手順で sandbox の外を申請する）を含む。
その文言は、ハーネスの verify-run が走らせた後に出力で「環境の欠落」と読む判定（`/dev/kvm` の語）に掛かる。
導出元: CH10

#### Scenario: 開けない場所を渡すと、エミュレータを起こさずに /dev/kvm の文言で止まる
- **WHEN** 偽の SDK の上で、`ASHIATO_KVM_DEVICE` に存在しない場所を渡して計測テストの台本を走らせる
- **THEN** 台本は 30 秒以内に rc=1 で終わる
- **AND** 標準エラーに `error: /dev/kvm を開けない` で始まり `verify-run` を含む行がある
- **AND** 偽のエミュレータは 1 度も呼ばれておらず、調べた場所にファイルは作られておらず、偽の adb に `emu kill` は届いていない

#### Scenario: 在っても読み書きできない場所を渡しても、同じく起こさずに止まる
- **WHEN** 偽の SDK の上で、`ASHIATO_KVM_DEVICE` に権限 000 の普通のファイルを渡して計測テストの台本を走らせる（root で走るときは読み書きできてしまうので、この場合を飛ばす）
- **THEN** 台本は 30 秒以内に rc=1 で終わり、標準エラーに `error: /dev/kvm を開けない` で始まる行があり、偽のエミュレータは 1 度も呼ばれていない

#### Scenario: /dev/kvm を開けないときの文言は、verify-run の環境の欠落の判定に掛かる
- **WHEN** 偽の SDK の上で、`ASHIATO_KVM_DEVICE` に存在しない場所を渡して計測テストの台本を走らせ、標準エラーの全文を取る
- **THEN** 全文に `/dev/kvm` の語がある
- **AND** ハーネスの判定の正本（`.harness2/scripts/evidence.py` の `INFRA_SIGNS`）がある木（手元）では、その正規表現が全文に当たる（CI には正本が無いので語の有無だけを見る。正規表現の写しを試験に持たないのは、正本が変わったときに写しだけが古くなるため）

### Requirement: 待っている間にエミュレータのプロセスが終わったら、すぐ理由と出力の末尾を出して止まる

WHILE 台本が、自分の起こしたエミュレータの端末が adb に現れるのを待っている間と、起動の完了（`sys.boot_completed` が 1）を待っている間、
WHEN そのエミュレータのプロセスが終わったとき、
THE SYSTEM SHALL 待ちの上限を待たずに、`error: エミュレータが起動の途中で終わった` で始まる行と、エミュレータの出力の末尾（40 行まで）と、出力の全文の置き場を標準エラーに出して rc=1 で終わる。
エミュレータの標準出力と標準エラーは、起動のたびに `collector-android/app/build/outputs/emulator.log` に書き直して残す。
出力の全文が加速を使えないこと（`requires hardware acceleration` か `permissions to use KVM` の句）を言うときは、`/dev/kvm` の語を含む行も出す。
導出元: CH10

#### Scenario: 起こしたエミュレータが起動直後に終わると、すぐ止まって出力の末尾を出す
- **WHEN** 偽の SDK の上で、`x86_64 emulation currently requires hardware acceleration!` を出してすぐ rc=1 で終わる偽のエミュレータを置き、計測テストの台本を走らせる
- **THEN** 台本は 30 秒以内に rc=1 で終わる
- **AND** 標準エラーに `error: エミュレータが起動の途中で終わった` で始まる行と、偽のエミュレータが出した最後の行と、`/dev/kvm` の語と、`collector-android/app/build/outputs/emulator.log` の置き場がある
- **AND** `collector-android/app/build/outputs/emulator.log` に偽のエミュレータの出力の全文がある

#### Scenario: 標準エラーに出す出力の末尾は 40 行まで
- **WHEN** 偽の SDK の上で、`line-1`〜`line-100` の 100 行を出してすぐ終わる偽のエミュレータを置き、計測テストの台本を走らせる
- **THEN** 標準エラーに `line-61`〜`line-100` があり、`line-60` は無い

#### Scenario: 端末が見えた後、起動の完了を待つ間に終わっても、すぐ止まる
- **WHEN** 偽の SDK の上で、端末を adb に見せた数秒後に終わり、起動の完了は返さない偽のエミュレータを置いて、計測テストの台本を走らせる
- **THEN** 台本は 30 秒以内に rc=1 で終わり、標準エラーに `error: エミュレータが起動の途中で終わった` で始まる行がある

### Requirement: 端末が上限までに adb に現れなければ止まり、1 回の adb の問い合わせも上限で打ち切る

WHEN 台本が起こしたエミュレータのプロセスが生きたまま、端末が上限（既定 120 秒。環境変数 `ASHIATO_EMULATOR_DEVICE_TIMEOUT` の秒で上書きできる）までに adb に現れないとき、
THE SYSTEM SHALL `error: <上限> 秒待っても端末が adb に現れない` で始まる行とエミュレータの出力の末尾を標準エラーに出して rc=1 で終わる。
待ち始めるときに、使う上限を `== 端末が adb に現れるのを最大 <上限> 秒待つ` の行で標準出力に出す。
起動の完了を待つ間の 1 回ごとの `adb shell` の問い合わせは、時間の上限（既定 10 秒。`ASHIATO_ADB_CALL_TIMEOUT`）で打ち切る。
起動の完了を待つ回数の上限（既定 180 回 × 2 秒。回数は `ASHIATO_EMULATOR_BOOT_ATTEMPTS`）は今のままで、超えたら今の文言 `error: 起動が終わらない` で rc=1 になる。
この 3 つの環境変数に正の整数でない値が渡されたら、エミュレータを起こさずに `error:` で始まる行を出して rc=1（採寸の台本は rc=2）で終わる。
導出元: CH10

#### Scenario: 上限を渡さなければ、端末を最大 120 秒待つ
- **WHEN** 偽の SDK の上で、上限の環境変数を渡さずに、正常に立つ偽のエミュレータで計測テストの台本を走らせる
- **THEN** 標準出力に `== 端末が adb に現れるのを最大 120 秒待つ` の行がある

#### Scenario: 上限に正の整数でない値を渡すと、起こさずに止まる
- **WHEN** 偽の SDK の上で、`ASHIATO_EMULATOR_DEVICE_TIMEOUT` に `0` を、次に `abc` を渡して計測テストの台本を走らせる
- **THEN** どちらも rc=1 で、標準エラーに `error:` で始まり `ASHIATO_EMULATOR_DEVICE_TIMEOUT` を含む行があり、偽のエミュレータは 1 度も呼ばれていない

#### Scenario: 生きたまま端末が現れなければ、上限で止まる
- **WHEN** 偽の SDK の上で、眠り続けて端末を adb に見せない偽のエミュレータを置き、`ASHIATO_EMULATOR_DEVICE_TIMEOUT=2` で計測テストの台本を走らせる
- **THEN** 台本は 30 秒以内に rc=1 で終わり、標準エラーに `error: 2 秒待っても端末が adb に現れない` で始まる行がある

#### Scenario: 起動の完了の問い合わせが返らなくても、回数の上限で止まる
- **WHEN** 偽の SDK の上で、端末は adb に見せるが `getprop sys.boot_completed` の問い合わせが眠ったまま返らない偽の adb を置き、起動の完了を待つ回数を 2 回・1 回の上限を 1 秒にして計測テストの台本を走らせる
- **THEN** 台本は 30 秒以内に rc=1 で終わり、標準エラーに `error: 起動が終わらない` がある

### Requirement: 同じ AVD のエミュレータが既に動いていれば、計測テストの台本は起こさずに止まる

WHEN 計測テストの台本が起こそうとする AVD（既定 `ashiato-api35`。環境変数 `AVD`）と同じ AVD のエミュレータが、起こす前に既に adb に見えているとき、
THE SYSTEM SHALL エミュレータを起こさず、その端末で計測テストを走らせず、その端末を止めず、
`error: 同じ AVD（<AVD>）のエミュレータが既に動いている（<serial>）` で始まり、止め方（`adb -s <serial> emu kill`）を含む行を標準エラーに出して rc=1 で終わる。
別の AVD のエミュレータや実機が繋がっているだけのときは、今のとおり起こす。
導出元: CH10

#### Scenario: 同じ AVD が既に動いていると、起こさず・止めずに理由つきで止まる
- **WHEN** 偽の SDK の上で、偽の adb が `emulator-5554` を端末として見せ、その AVD の名前を `ashiato-api35` と答えるようにして、計測テストの台本を走らせる
- **THEN** 台本は rc=1 で終わり、標準エラーに `error: 同じ AVD（ashiato-api35）のエミュレータが既に動いている（emulator-5554）` で始まり `adb -s emulator-5554 emu kill` を含む行がある
- **AND** 偽のエミュレータは 1 度も呼ばれておらず、偽の adb に `emu kill` は届いていない

#### Scenario: 実機か別の AVD のエミュレータだけが繋がっているときは、今のとおり起こす
- **WHEN** 偽の SDK の上で、偽の adb が実機（`emulator-` で始まらない serial）と、AVD の名前を `other-avd` と答える `emulator-5556` を端末として見せる状態で、計測テストの台本を走らせる
- **THEN** 偽のエミュレータは 1 度呼ばれる

### Requirement: 失敗で止まるとき、計測テストの台本は自分の起こしたプロセスだけを止め、ほかの端末を止めない

WHEN 計測テストの台本が、端末が現れない・起動が終わらない・起動の途中で終わった、のどれかで止まるとき、
THE SYSTEM SHALL 自分が起こしたエミュレータのプロセス（とその子）がまだ生きていれば、`KEEP=1` に依らず止める。
そのとき、名指しの端末（`emulator-5554`）へ `emu kill` を送らない（その番号にいるのが自分の端末とは限らない）。
起こす前に止まるとき（/dev/kvm を開けない・設定の値が誤り・同じ AVD が動いている）は、何も止めない。
正常に計測テストまで進んだ後の後片付け（`KEEP=1` でなければ `emulator-5554` を止める）は今のまま。
導出元: CH10

#### Scenario: 上限で止まったとき、KEEP=1 でも自分の起こしたプロセスは残らない
- **WHEN** 端末を見せない偽のエミュレータで、`KEEP=1 ASHIATO_EMULATOR_DEVICE_TIMEOUT=2` で計測テストの台本を走らせる
- **THEN** 台本が終わった後、その回の偽のエミュレータのプロセスとその子は 1 本も残っていない
- **AND** 偽の adb に `emu kill` は届いていない

#### Scenario: 起動の途中で終わったとき、名指しの端末を止めない
- **WHEN** 起動直後に終わる偽のエミュレータで、`KEEP` を付けずに計測テストの台本を走らせる
- **THEN** 偽の adb に `-s emulator-5554 emu kill` は届いていない

### Requirement: 正常にエミュレータが立つときの計測テストの台本の振る舞いは変わらない

WHEN /dev/kvm を開け、同じ AVD のエミュレータが動いておらず、起こしたエミュレータが端末として adb に現れて起動を終えるとき、
THE SYSTEM SHALL この change の前と同じ引数（`-avd <AVD> -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot`）でエミュレータを起こし、
アニメーションを切り、計測テストの写真と動画を入れ、計測テストを 6 段で走らせて段ごとの結果の XML を `androidTest-results-stage1`〜`stage6` に残し、
`KEEP=1` でなければ終わりに `emulator-5554` を止め、`KEEP=1` なら止めない。
emulator が無い・システムイメージが無いときは、今の文言（`error: emulator が無い。…`・`error: <イメージ> が無い`）で rc=1。AVD が無ければ作る。
計測テストの台本の行の形（採寸の試験を通常の段から除く・1 段目の前にアプリを外す）は、`tools/test-usage-volume.sh` が今の検査のまま通る形を保つ。
導出元: CH10

#### Scenario: 正常に立つとき、今と同じ引数でエミュレータを起こす
- **WHEN** 偽の SDK・偽の adb・偽の計測テスト（gradle の代わりに結果の XML を 1 つ書く）を置いた使い捨ての木で、`KEEP` を付けずに計測テストの台本を走らせる
- **THEN** 台本は rc=0 で終わり、偽のエミュレータが受け取った引数は `-avd ashiato-api35 -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot` と一字一句同じである

#### Scenario: 正常に最後まで走ると、6 段の結果を残してエミュレータを止める
- **WHEN** 上の Scenario と同じ使い捨ての木で、`KEEP` を付けずに計測テストの台本を走らせる
- **THEN** 台本は rc=0 で終わり、`androidTest-results-stage1`〜`androidTest-results-stage6` の 6 つの置き場に結果の XML がある
- **AND** 偽の adb にアニメーションの設定 3 つ（`settings put global window_animation_scale 0` など）と、終わりに `-s emulator-5554 emu kill` が届いている

#### Scenario: KEEP=1 なら、通った後にエミュレータを止めない
- **WHEN** 偽の SDK・偽の adb・偽の計測テストを置いた使い捨ての木で、`KEEP=1` で計測テストの台本を走らせる
- **THEN** 台本は rc=0 で終わり、偽の adb に `emu kill` は届いていない

#### Scenario: emulator かシステムイメージが無いときは、今の文言で止まる
- **WHEN** 偽の SDK から emulator を外して台本を走らせ、次にシステムイメージの置き場を外して走らせる
- **THEN** どちらも rc=1 で、標準エラーにそれぞれ `error: emulator が無い。` と `error: system-images;android-35;google_apis;x86_64 が無い` がある

#### Scenario: AVD が無ければ作ってから起こす
- **WHEN** 偽の `avdmanager` が AVD の一覧を空で返す状態で、計測テストの台本を最後まで走らせる
- **THEN** 偽の `avdmanager` に `create avd -n ashiato-api35` が届き、その後に偽のエミュレータが呼ばれる

#### Scenario: 計測テストの台本の行の形の検査は、今のまま通る
- **WHEN** この change の後の木で `tools/test-usage-volume.sh` を走らせる
- **THEN** rc=0 で終わる

### Requirement: 利用状況の採寸の台本も同じ部品でエミュレータを起こし、起こせなければ「測れなかった」で止まる

WHEN 利用状況の採寸の台本（`tools/usage-volume.sh`）が、adb に見えているエミュレータが無いためにエミュレータを起こすとき、
THE SYSTEM SHALL 計測テストの台本と同じ部品で起こし、/dev/kvm を開けない・起動の途中で終わった・端末が現れない・起動が終わらない、のどれでも、同じ `error:` の行と出力の末尾を標準エラーに出し、
採寸の台本の約束どおり rc=2（測れなかった）で終わる。起こすときの引数（`-wipe-data` を含む）は今のまま。
失敗で止まるときは自分の起こしたプロセス（とその子）を止め、端末が adb に現れる前に止まったときは名指しの端末（`emulator-5554`）へ `emu kill` を送らない。
adb に見えているエミュレータがあれば、今のとおり起こさずにそれを使う。同じ AVD の確かめ（上の Requirement）は採寸の台本には当てない。
導出元: CH10

#### Scenario: 採寸の台本で起こしたエミュレータが起動直後に終わると、すぐ rc=2 で止まる
- **WHEN** 偽の SDK の上で、起動直後に終わる偽のエミュレータを置き、adb に端末が無い状態で採寸の台本を走らせる
- **THEN** 台本は 30 秒以内に rc=2 で終わり、標準エラーに `error: エミュレータが起動の途中で終わった` で始まる行がある
- **AND** その回の偽のエミュレータのプロセスは残っておらず、偽の adb に `emu kill` は届いていない
- **AND** 偽のエミュレータが受け取った引数は `-avd ashiato-api35 -wipe-data -no-window -gpu swiftshader_indirect -noaudio -no-boot-anim -camera-back none -no-snapshot` と一字一句同じである

#### Scenario: 採寸の台本でも /dev/kvm を開けなければ、起こさずに rc=2 で止まる
- **WHEN** 偽の SDK の上で、`ASHIATO_KVM_DEVICE` に存在しない場所を渡し、adb に端末が無い状態で採寸の台本を走らせる
- **THEN** 台本は rc=2 で終わり、標準エラーに `error: /dev/kvm を開けない` で始まる行があり、偽のエミュレータは 1 度も呼ばれていない

#### Scenario: 採寸の台本は、adb に見えているエミュレータがあれば起こさない
- **WHEN** 偽の adb が `emulator-5554` を端末として見せる状態で、採寸の台本を走らせる
- **THEN** 偽のエミュレータは 1 度も呼ばれていない（その後の組み立てや計測で落ちても、起こしていないことだけを見る）
