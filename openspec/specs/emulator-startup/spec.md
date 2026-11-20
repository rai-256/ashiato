# emulator-startup Specification

## Purpose
手元でエミュレータを起こす台本（計測テストの `tools/android-emulator.sh` と、利用状況の採寸の `tools/usage-volume.sh`）が、エミュレータを起こせないときに無期限に待たず、早く・理由つきで止まること。/dev/kvm を開けない・起動の途中で終わった・端末が現れない・同じ AVD が既に動いている、のどれでも止まり方が決まっていて、正常に立つときの振る舞いは変わらない。並走する Story と change が同じ機械で計測テストを回し続けるための、開発の道具の約束。

## Requirements

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

### Requirement: ONLY を渡さなければ、計測テストの台本は今の全段をそのまま走らせる

WHEN 環境変数 `ONLY` が無いか空で計測テストの台本（`tools/android-emulator.sh`）を走らせるとき、
THE SYSTEM SHALL この change の前の版の台本と同じ命令を同じ順に送り（gradle の計測テストの各段とその引数・`adb` のアプリを外す / 入れる / 止める / 権限を戻す / バックアップの往復・写真と動画・入れ替えの台本）、
全体の経路が残す段の数（この change の時点で 8）の結果の XML を `androidTest-results-stage<N>` に残す。
ONLY の口を足しても、採寸の試験を通常の段から除く行と、1 段目の gradle の前にアプリを外す行の形は変えず、`tools/test-usage-volume.sh` は今の検査のまま通る。
正典の Requirement「正常にエミュレータが立つときの計測テストの台本の振る舞いは変わらない」の段の数の約束は、`ONLY` が無いか空のときの約束として読む（`ONLY` にクラスを渡したときは、下の ONLY の Requirement が優先する）。
導出元: CH19

#### Scenario: ONLY が空なら、渡さないときと同じ命令の列で全段を走らせる
- **WHEN** 偽の SDK・偽の adb・偽の gradle（どちらも受け取った命令を同じ 1 本の記録に 1 行ずつ足す）を置いた使い捨ての木で、`ONLY` を渡さずに計測テストの台本を走らせ、別の同じ作りの木で `ONLY=`（空）で走らせる
- **THEN** どちらも rc=0 で終わり、段の結果の置き場の数は 2 つの実行で同じで、全体の経路が残す段の数（台本から数える。決め打たない）と等しく、入れ替えの台本が 1 回呼ばれる
- **AND** 起動の待ちの問い合わせ（`getprop`・`devices`・`get-state`・`wait-for-device`・`emu avd name`）を除いた命令の列（gradle と adb をまたいだ順のまま）は、2 つの実行で同じである

#### Scenario: ONLY を渡さなければ、この change の前の版の台本と同じ命令の列で全段を走らせる
- **WHEN** 比べる元の版（この change の前の版。PR の証跡では `origin/main` との分岐点）を明示して渡し、その版の台本と今の台本を、`ONLY` を渡さずに同じ作りの使い捨ての木でそれぞれ走らせる
- **THEN** どちらも rc=0 で終わり、起動の待ちの問い合わせを除いた命令の列（gradle と adb をまたいだ順のまま）が同じである
- **AND** 比べる元の版が渡されないときは、比べられなかったことを出して飛ばす（後の Story が全体の段を足しても落ちないように、既定では比べない。この確かめは、元の版を渡す PR の証跡で効く）

### Requirement: ONLY にクラスを 1 つ渡すと、計測テストの台本はそのクラスだけを 1 段で走らせる

WHEN 環境変数 `ONLY` に計測テストのクラスの完全修飾名（例: `dev.ashiato.collector.MediaPremisesInstrumentedTest`）を 1 つ渡して計測テストの台本を走らせるとき、
THE SYSTEM SHALL 全体のときと同じにエミュレータを起こし、アニメーションを切り、写真と動画を入れ、アプリと試験のアプリを外したうえで、
前の実行の結果（`collector-android/app/build/outputs/androidTest-results`）を消し、アプリを止めて権限を戻してから、
gradle の計測テストを `-Pandroid.testInstrumentationRunnerArguments.class=<ONLY>` だけで絞って 1 回走らせ、結果の XML を `androidTest-results-stage1` の 1 つだけに残す。
全体のときの他の段・バックアップの往復・入れ替えの台本は走らせない。
gradle の計測テストが落ちたら（渡したクラスのテストが落ちたとき）、全体のときと同じに rc≠0 で止まる。
その段の結果の XML に、渡したクラスの結果（`classname="<ONLY>"`）が 1 本も無ければ、`error: ONLY=<クラス> のテストが 1 本も走っていない` を標準エラーに出して rc=1 で止まる。
`KEEP=1` の扱いと、終わりに（通っても落ちても）エミュレータを止める扱いは全体のときと同じ。
これは正典の Requirement「正常にエミュレータが立つときの計測テストの台本の振る舞いは変わらない」の段の数の約束の例外で、`ONLY` にクラスを渡したときはこの Requirement が優先する。
導出元: CH19

#### Scenario: ONLY にクラスを渡すと、そのクラスだけに絞った計測テストを 1 回だけ走らせる
- **WHEN** 偽の SDK・偽の adb・偽の gradle を置いた使い捨ての木で、`ONLY=dev.ashiato.collector.MediaPremisesInstrumentedTest` で計測テストの台本を走らせる
- **THEN** 台本は rc=0 で終わり、偽の gradle への `connectedDebugAndroidTest` の呼び出しはちょうど 1 回で、その引数に `-Pandroid.testInstrumentationRunnerArguments.class=dev.ashiato.collector.MediaPremisesInstrumentedTest` があり、`notClass` と `annotation` / `notAnnotation` は無い
- **AND** 段の結果の置き場は `androidTest-results-stage1` の 1 つだけで、入れ替えの台本も `bmgr` も呼ばれず、終わりに `-s emulator-5554 emu kill` が届いている

#### Scenario: ONLY でも、全体と同じにアニメーションを切り、写真と動画を入れる
- **WHEN** 上の Scenario と同じに `ONLY` を渡して計測テストの台本を走らせる
- **THEN** 偽の adb にアニメーションの設定 3 つ（`settings put global window_animation_scale 0` など）が届き、写真と動画を入れる台本が 1 回呼ばれ、どちらも gradle の計測テストより前である

#### Scenario: ONLY の段の前に、アプリを外し、止めて、権限を戻す
- **WHEN** 上の Scenario と同じに `ONLY` を渡して計測テストの台本を走らせる
- **THEN** 命令の記録に、`uninstall dev.ashiato.collector.test`・`uninstall dev.ashiato.collector`・`shell am force-stop dev.ashiato.collector`・`shell pm reset-permissions` がこの順で、gradle の計測テストより前にある

#### Scenario: ONLY と KEEP=1 を一緒に渡すと、通った後にエミュレータを止めない
- **WHEN** `ONLY` にクラスを渡し、`KEEP=1` で計測テストの台本を走らせる
- **THEN** 台本は rc=0 で終わり、偽の adb に `emu kill` は届いていない

#### Scenario: ONLY のクラスのテストが落ちたら、rc≠0 で止まり、エミュレータを止める
- **WHEN** 偽の gradle の計測テストが rc=1 で終わる状態で、`ONLY` にクラスを渡して計測テストの台本を走らせる
- **THEN** 台本は rc≠0 で終わり、段の後の確かめに進まず、終わりに `-s emulator-5554 emu kill` が届いている

#### Scenario: 渡したクラスの結果が無ければ、0 本として止まる
- **WHEN** 偽の gradle が渡したクラスと違うクラスの結果の XML だけを書く状態で、`ONLY=dev.ashiato.collector.MediaPremisesInstrumentedTest` で計測テストの台本を走らせる
- **THEN** 台本は rc=1 で終わり、標準エラーに `error: ONLY=dev.ashiato.collector.MediaPremisesInstrumentedTest のテストが 1 本も走っていない` がある

#### Scenario: 前の実行の結果が残っていても、今回の結果として数えない
- **WHEN** 前の実行が残した結果の XML（渡したクラスの `classname` を持ち、偽の gradle が書くものと違う名前のもの）を `androidTest-results` の下に置き、偽の gradle が渡したクラスの結果を書かない状態で、`ONLY` にそのクラスを渡して台本を走らせる
- **THEN** 台本は rc=1 で終わり、標準エラーに `error: ONLY=<そのクラス> のテストが 1 本も走っていない` がある

### Requirement: ONLY にバックアップの試験を渡すと、全体と同じ準備・往復・確かめの手順だけを走らせる

WHEN `ONLY=dev.ashiato.collector.St56BackupTest` で計測テストの台本を走らせるとき、
THE SYSTEM SHALL gradle の計測テスト（`connectedDebugAndroidTest`）を呼ばず、全体のときのバックアップの段と同じ手順 ——
アプリと試験のアプリを gradle で入れ、バックアップを有効にし、`am instrument` で `St56BackupTest#prepareBackup` を走らせ、
`tools/android-backup-roundtrip.sh` で往復させ、`am instrument` で `St56BackupTest#verifyRestored` を走らせて `OK (1 test)` を確かめる —— だけを同じ順に走らせる。
このとき段の結果の置き場（`androidTest-results-stage<N>`）は作らず、前の実行の結果の XML を今回の結果として写さない（判定は `OK (1 test)` の照合だけ）。
確かめが `OK (1 test)` でなければ、全体のときと同じ `error: St56BackupTest の復元の確かめが通らない` を出して rc=1 で止まる。入れ替えの台本とほかの段は走らせない。
導出元: CH19

#### Scenario: バックアップの試験を渡すと、gradle の計測テストを呼ばずに準備・往復・確かめを順に走らせる
- **WHEN** 偽の SDK・偽の adb・偽の gradle（計測テストのときだけ結果の XML を書く）を置き、`androidTest-results` の無い使い捨ての木で、`ONLY=dev.ashiato.collector.St56BackupTest` で計測テストの台本を走らせる
- **THEN** 台本は rc=0 で終わり、偽の gradle に `connectedDebugAndroidTest` は届かず、`:app:installDebug :app:installDebugAndroidTest` が届いている
- **AND** 命令の記録に、`bmgr enable true`・`St56BackupTest#prepareBackup` の `am instrument`・`bmgr restore`・`St56BackupTest#verifyRestored` の `am instrument` がこの順にあり、入れ替えの台本は呼ばれず、段の結果の置き場は 1 つも無い

#### Scenario: バックアップの試験の確かめが通らなければ、全体と同じ文言で止まる
- **WHEN** 偽の adb が `St56BackupTest#verifyRestored` の `am instrument` に `OK (1 test)` を返さない状態で、`ONLY=dev.ashiato.collector.St56BackupTest` で台本を走らせる
- **THEN** 台本は rc=1 で終わり、標準エラーに `error: St56BackupTest の復元の確かめが通らない` があり、終わりに `-s emulator-5554 emu kill` が届いている

### Requirement: ONLY に専用の台本で走るクラスや形の違う値を渡すと、エミュレータを起こさずに止まる

WHEN `ONLY` に、走らせ方が別の台本にある計測テストのクラス（入れ替えの `dev.ashiato.collector.St58PackageReplacedInstrumentedTest` は `tools/android-st58-replace.sh`、採寸の `dev.ashiato.collector.UsageVolumeInstrumentedTest` は `tools/usage-volume.sh`）を渡すか、
完全修飾のクラス名 1 つの形でない値（パッケージの無い名前・カンマで並べた複数のクラス・`#` で付けたメソッド・空白を含むもの）を渡して計測テストの台本を走らせるとき、
THE SYSTEM SHALL エミュレータを起こさず、AVD も作らず、標準エラーに `error: ONLY=` で始まり渡した値と理由（専用の台本のクラスならその台本の名前）を含む行を出して rc=1 で止まる（同じ台本の既存の設定の誤りと同じ rc）。
導出元: CH19

#### Scenario: 入れ替えの試験か採寸の試験を渡すと、専用の台本の名前を出して起こさずに止まる
- **WHEN** 偽の SDK を置いた使い捨ての木で、`ONLY=dev.ashiato.collector.St58PackageReplacedInstrumentedTest` で台本を走らせ、次に `ONLY=dev.ashiato.collector.UsageVolumeInstrumentedTest` で走らせる
- **THEN** どちらも rc=1 で、偽のエミュレータは呼ばれず、標準エラーにそれぞれ `tools/android-st58-replace.sh` と `tools/usage-volume.sh` を含む `error: ONLY=` の行がある

#### Scenario: 完全修飾のクラス名 1 つの形でない値を渡すと、起こさずに止まる
- **WHEN** 偽の SDK を置いた使い捨ての木で、`ONLY` に `MediaPremisesInstrumentedTest`・`dev.ashiato.collector.A,dev.ashiato.collector.B`・`dev.ashiato.collector.St56BackupTest#prepareBackup`・`dev.ashiato.collector.A B` を 1 つずつ渡して台本を走らせる
- **THEN** どれも rc=1 で、偽のエミュレータも偽の `avdmanager` も呼ばれず、標準エラーに渡した値を含む `error: ONLY=` の行がある
