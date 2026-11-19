## ADDED Requirements

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
