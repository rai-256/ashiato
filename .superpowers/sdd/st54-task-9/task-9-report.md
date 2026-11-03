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
