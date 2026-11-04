# Task 9 実装報告

## 実装内容

- Android の manifest label を `manifestPlaceholders` から解決し、`-dev.` 版を「 あしあと。開発版」、正式版を「 あしあと。」にした。
- `BuildConfig.CHANNEL` と `/contract` の `Ashiato-Channel` 読み取りを追加し、系統不一致では 5 つの送り手と原本送信を停止して手元に残すようにした。
- 系統不一致の画面・通知文、24 時間間隔、上限通知への案内を既存の契約通知経路へ統合した。
- `check-artifacts.sh` に `apk-label` / `apk-manifest` / `--self-test-android` を追加した。
- `tools/android-side-by-side.sh` を追加し、正式版と開発版を同じ AVD に入れて正式版の未送信記録が残ることを検査するようにした。

## TDD Evidence

- RED: `scripts/quiet-run channel-red -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*ChannelGateTest*'`。新 API 未実装のため `CHANNEL_VALUE` 未解決（および Kotlin daemon の sandbox 警告）で失敗した。
- GREEN: `scripts/quiet-run channel-final2 -- ... --tests '*ChannelGateTest*'` は、修正後に 10 tests completed で成功。

## テストと検証

- `scripts/quiet-run unit-all-final -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest` — PASS。
- `scripts/verify-run 9.1` — PASS、開発版・正式版の APK 名と manifest 宣言を確認。
- `scripts/verify-run 9.2` — PASS、ChannelGateTest 10 本以上および ContractGate 互換テスト。
- `scripts/verify-run 9.3` — PASS、AVD 上で side-by-side と正式版未送信 1 件の保持を確認。

## 変更ファイル

`collector-android/app/build.gradle.kts`, `collector-android/app/src/main/AndroidManifest.xml`, `collector-android/app/src/main/kotlin/dev/ashiato/collector/{ContractGate.kt,ContractNotifier.kt,HttpTransport.kt,OriginalUploader.kt,Sender.kt}`, `collector-android/app/src/test/kotlin/dev/ashiato/collector/{ChannelGateTest.kt,ContractGateTest.kt}`, `tools/release/check-artifacts.sh`, `tools/android-side-by-side.sh`, `openspec/changes/st54-dev-side-by-side/evidence.jsonl`

## セルフレビュー

- 正式版の既定値は `official` と従来のアプリ名・識別子を維持している。
- APK manifest 比較では applicationId 由来の動的 receiver 権限だけを匿名化し、宣言の差分を隠していない。
- 9.3 の証跡に、エミュレータ基盤が作る `android-yosis` / `hostapd_*.conf` / `netsim.ini` の残留が記録された。side-by-side 台本自身の一時 payload は cleanup で削除している。

## Fix round 1

- F1: `HttpTransport.get` が応答 headers を `Outcome.Responded` に渡すようにし、偽 TCP HTTP の GET テストを追加した。
- F2: 系統違いを ST50 の `ContractReasons`・通知 ID 5・通知時刻から分離し、永続する別状態と通知 ID 6 を追加した。画面・7 日前の案内には必要に応じて両方の文を添える。
- F3: 記録、生存信号、破棄報告、原本欠け、原本の系統違い停止ログに expected/actual の系統を残すようにした。
- F4/F5: `ChannelGateTest` を Robolectric の `ShadowLog` / `ShadowNotificationManager` と実状態で検証し、`ContractGateTest` の検証入口も維持した。`ChannelGateTest` は 12 本。
- F6: side-by-side の未送信データを `files/outbox/records/000000000000.jsonl` に置き、`run-as` でその実体を数えるようにした。
- F7: `premises.md` の P2 を 4（APK 並列インストールと正式版 SegmentStore 区切りファイルの維持を実機確認）へ更新した。
- F8: `apk-manifest` の service/receiver 抽出を `aapt2 dump xmltree` に変更し、APK の宣言実名を比較するようにした。
- F9: Kotlin compiler を in-process にして、PASS ログから daemon の stack trace を除いた。

### Fix round 1 の検証

- RED: 初回修正テストは Kotlin daemon の sandbox 書き込み制限と `LocationService.kt:86` のコンパイルエラーで失敗。`ContractGate` の trailing lambda が誤った引数へ束縛されていたためで、引数を明示して修正した。
- GREEN: `scripts/quiet-run channel-clean -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*ChannelGateTest*' --tests '*HttpTransportTest*'` — PASS（ChannelGateTest 12、HttpTransportTest 10）。
- `scripts/quiet-run unit-full-final -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest` — PASS。
- `scripts/verify-run 9.1` — PASS。
- `scripts/verify-run 9.2` — PASS（ChannelGateTest 12 本、ContractGate 系も PASS）。
- `scripts/verify-run 9.3` — 最初の 2 回は `run-as` 内の件数取得が失敗したが、実データ相対パスと固定 SegmentStore 区切りファイルへ修正後、sandbox 外（KVM）で PASS。

### Fix round 1 の変更ファイル

`collector-android/app/src/main/kotlin/dev/ashiato/collector/{ContractGate.kt,ContractNotifier.kt,HttpTransport.kt,LocationService.kt,MainActivity.kt,OriginalUploader.kt,Retention.kt,Sender.kt}`, `collector-android/app/src/test/kotlin/dev/ashiato/collector/{ChannelGateTest.kt,HttpTransportTest.kt}`, `collector-android/gradle.properties`, `tools/release/check-artifacts.sh`, `tools/android-side-by-side.sh`, `openspec/changes/st54-dev-side-by-side/{evidence.jsonl,premises.md}`

### Fix round 1 の concern

`scripts/check_scenarios.py .` は後続 Task 10 の配布 Scenario 10 件が未担保として FAIL するが、Task 9 の変更範囲外である。9.3 の emulator 基盤による一時ファイル残留は証跡に記録され、台本自身の payload は cleanup している。

### Commit hook 対応

commit hook の A-2 検査が系統名を `Telemetry.error` に埋め込む形式を拒否したため、系統の相手/自分は `trigger=dev|official` / `availability=dev|official` として `Telemetry.line` から出す形に修正した。`scripts/quiet-run channel-log -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*ChannelGateTest*'` と `scripts/verify-run 9.2` は修正後に PASS。

その後 hook が補助関数経由も拒否したため、6 箇所の送信停止ログを `Telemetry.line` の直接呼び出しへ展開し、`scripts/quiet-run channel-log-final -- ... --tests '*ChannelGateTest*'` と `scripts/verify-run 9.2` を再実行して PASS。

## Fix round 2（F4・F5・F14）

- 系統不一致だけの `MainActivity` の行が ST50 の「サーバを更新してください」を含めないよう、`ContractNotice` に契約更新待ちかどうかを持たせ、系統不一致専用の行を分離した。契約更新待ちと系統不一致が同時にある 7 日前の案内は、従来どおり両方の文を添える。
- `Sender` と `OriginalUploader` の系統を既定値付きで注入可能にし、`ChannelGateTest` を実際の `Drainer`・`Sender`・Robolectric の通知・永続状態を通る 10 本へ書き直した。開発版／正式版双方の送信抑止と保持、同系統の送信、通知の表示・解除・24 時間再通知、404／読めない 200、途絶問い合わせを確認する。
- 比較だけの作り物だった `ContractGateTest` は削除した。検証コマンド `--tests "*ContractGate*"` は実在する ST50 テスト名と一致せず `No tests found` になるため、検証コマンドの食い違いとして controller へ引き継ぐ。

### Fix round 2 の検証

- GREEN: `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*ChannelGateTest*'` — PASS、10 tests completed。
- `scripts/quiet-run unit-all -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest` — PASS、BUILD SUCCESSFUL。
- `scripts/check_scenarios.py .` — Task 10 の既存未担保 Scenario 10 件で FAIL。今回の ChannelGateTest の印は未担保として報告されていない。
- `scripts/verify-run 9.2` — ChannelGateTest 側は PASS（10 本以上）。ST50 側は `No tests found for given includes: [*ContractGate*]` で FAIL。過去の PASS で置き換えず、`evidence.jsonl` に現 commit の結果を記録した。

### Fix round 2 の変更ファイル

`collector-android/app/src/main/kotlin/dev/ashiato/collector/{ContractNotifier.kt,MainActivity.kt,OriginalLedger.kt,OriginalUploader.kt,Sender.kt}`, `collector-android/app/src/test/kotlin/dev/ashiato/collector/ChannelGateTest.kt`、削除 `collector-android/app/src/test/kotlin/dev/ashiato/collector/ContractGateTest.kt`、`openspec/changes/st54-dev-side-by-side/evidence.jsonl`

## Fix round 3（F4・F5・F14 の再確認）

- `ChannelGateTest` の実系統試験に、記録・生存信号・破棄報告・原本欠け・原本の 5 つの送り手を組み込み、系統不一致時に各未送信件数と原本を保持することを確認するようにした。
- `ContractGate` の `localChannel` を試験から渡し、`ShadowLog` で相手／自分の系統ログ、`ShadowNotificationManager` で系統不一致専用通知を確認するようにした。
- F5 の `ContractGateTest` は作り物を再追加せず削除状態を維持した。`*ContractGate*` は `No tests found` になるため、検証コマンドの不一致として引き継ぐ。
- F14 の `MainActivity` は既存の `contractWaiting = false` 修正を確認した。系統不一致だけの行は ST50 の更新待ち文を含まない。

### Fix round 3 の検証

- RED: `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*ChannelGateTest*'` — 変更直後は matching channel の送信件数期待値が 1 のままで、5 送信系統の実体化により失敗した（10 tests completed, 1 failed）。
- GREEN: 同コマンド — PASS、10 tests completed、`BUILD SUCCESSFUL`。
- `scripts/quiet-run unit-all -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest` — PASS、`BUILD SUCCESSFUL`。
- Task 9.2 の ChannelGateTest 検証コマンド — PASS、10 本以上。
- `scripts/quiet-run contract-gate -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*ContractGate*'` — FAIL、`No tests found for given includes: [*ContractGate*]`。ST50 の実体を偽テストで置き換えないため、この不一致は未解決の concern として残す。
- `python3 scripts/check_scenarios.py .` — Task 10 の既存未担保 Scenario 10 件で FAIL。今回の変更対象外。

### Fix round 3 の変更ファイル

`collector-android/app/src/test/kotlin/dev/ashiato/collector/ChannelGateTest.kt`
