# Task 9 修正報告

## 実装

- `UpdateInstall` の一覧取得・APK ダウンロード要求に既定の `User-Agent` を付与した。
- PackageInstaller の結果受け手を mutable PendingIntent にし、`STATUS_PENDING_USER_ACTION` の確認 Intent を起こすようにした。
- 重複していた manifest の権限・receiver 宣言を削除した。
- 更新後の foreground service type 選択と、specialUse 中の位置 Capability を実際の判定関数として固定した。
- 更新待ち通知に MainActivity への content Intent を付けた。
- Robolectric / instrumented の ST58 テストを定数照合から実動作の判定へ置き換え、D10 の 5 系統と Capability を追加した。
- Android 入れ替え台本で旧版・新版を別々にビルドして保存し、権限・pidof・self-test の出力検証を追加した。
- premises.md の P6・P7・P26 の確かさを 4 に更新した。

## テスト

- RED: 既存レビューで `St58PackageReplacedTest.kt` 不在、恒真式試験、旧 APK 未使用を確認。修正前の 9.2 台本は新版を二度入れ、旧 APK の存在確認に失敗した。
- GREEN: `scripts/verify-run 9.1` — PASS。ST58 単体 31 本、下限 22 本以上、backup rules PASS。
- `tools/android-st58-replace.sh --self-test` — PASS。
- `scripts/verify-run 9.2` — エミュレータの `am instrument` が system_server の `ProcessRecord` null で落ちたため、実エミュレータ段は FAIL。self-test は PASS。証跡の FAIL を置き換えていない。
- `git diff --check` — PASS。

## 修正ラウンド 3

- `St58UpdateInstallTest` に PackageInstaller の取り消し結果・成功結果を受け手へ渡し、APK が消え、確認画面を開かないことを実際に確認する試験を追加した。
- 計測試験の specialUse / 位置昇格系で `PackageReplacedReceiver` を実行し、単なる定数照合だけにならないようにした。
- 入れ替え台本で権限切替後の対象プロセスを確認・必要時に起こし直し、サービス停止、非同期ログ待ち、PID の存在確認を追加した。self-test の欠落検知は維持した。

### 修正ラウンド 3 の検証

- `scripts/verify-run 9.1` — PASS（ST58 32 本、backup rules PASS）。
- `tools/android-st58-replace.sh --self-test` — PASS。
- `scripts/verify-run 9.2` — FAIL（rc=1）。先頭段は PASS したが、specialUse 段で対象サービスの `foreground_special_use` ログが出ず台本が停止した。エミュレータの既存プロセス／サービス状態により `onCreate` の種別ログを新たに観測できなかったためで、PASS へ置き換えていない。
- `git diff --check` — PASS。

## 修正ラウンド 5

### 対応

- F3: `St58PackageReplacedInstrumentedTest` の 7 段を、受け手実行後の印の消費、`LocationService` の実ログ、通知 ID 12、サービス生存、specialUse から location への昇格を確認する実動作試験にした。段間に残る待機通知は各段の開始時に消す。
- F15: `tools/android-st58-replace.sh` は `ashiato` タグの `kind=foreground_special_use` / `kind=foreground_location` を待って検査するようにした。昇格時にも production log を残す。
- F6/F14: 9.2 が FAIL の間は P6・P7・P26 を確かさ 2 とし、実エミュレータ 7 段が PASS した後に、根拠と確かさ 4 へ更新した。

### 検証

- `scripts/quiet-run st58-unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St58*'` — PASS（32 本）。
- `scripts/quiet-run st58-android-test-build-2 -- ./collector-android/gradlew -p collector-android :app:assembleDebugAndroidTest` — PASS。
- `tools/android-st58-replace.sh --self-test` — PASS。
- `scripts/verify-run 9.1` — PASS（ST58 32 本、backup rules PASS）。
- `scripts/verify-run 9.2`（KVM を許可した sandbox 外） — PASS（7 段すべて `OK (1 test)`、self-test PASS）。
- `git diff --check` — PASS。

### 残る注意

- `verify-run 9.2` はエミュレータ基盤が作る一時項目 `android-yosis`、`hostapd_*.conf`、`netsim.ini` を 3 件残したと報告した。台本自身の APK 一時置き場は cleanup されており、これはエミュレータ基盤側の残留として記録する。

## 変更ファイル

`UpdateInstall.kt`、`LocationService.kt`、`AndroidManifest.xml`、`St58UpdateInstallTest.kt`、`St58PackageReplacedTest.kt`、`St58PackageReplacedInstrumentedTest.kt`、`tools/android-st58-replace.sh`、`premises.md`、`evidence.jsonl`。

## 自己レビュー

9.2 の実機通しはエミュレータ基盤障害で未完了。台本の旧 APK 保存・権限設定・同一 pid 検証・self-test 欠落検知は実装済みだが、実エミュレータ PASS は主張しない。

## 修正ラウンド 4

- `St58PackageReplacedTest` に印なし・位置権限なしの種別、specialUse 中の位置以外の Capability を追加し、D10 の 5 系統と位置以外の継続を実動作の判定で固定した。
- 計測試験の specialUse 段で受け手が置く印と位置 Capability の不足を確認し、通知権限段で受け手起動後の通知チャンネルを確認するようにした。
- 入れ替え台本の self-test 欠落判定を明示的な条件にし、PID の空値を成功扱いしないようにした。
- `premises.md` の P6・P7 の確かさ表記に対応する前置きを更新した。P26 は既存の確かさ 4 を維持した。

### 修正ラウンド 4 の検証

- RED: 前回レビューで、計測試験の specialUse・通知段が定数／恒真条件だけで、台本の PID 検査が空値を通す状態だった。
- GREEN: `tools/android-st58-replace.sh --self-test` — PASS。
- `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St58*'` — PASS（32 本）。
- `scripts/verify-run 9.1` — PASS（単体 32 本、backup rules PASS）。
- `scripts/quiet-run android-test-build -- ./collector-android/gradlew -p collector-android :app:assembleDebugAndroidTest` — PASS。
- `bash -n tools/android-st58-replace.sh` / `git diff --check` — PASS。
- `scripts/verify-run 9.2` — sandbox 内は `/dev/kvm` により BLOCKED_INFRA。sandbox 外で `scripts/quiet-run emu -- tools/android-st58-replace.sh` を再実行したが、specialUse 段で `foreground_special_use` ログが無く rc=1。FAIL を PASS に置き換えていない。

## 修正ラウンド 2

- `UpdateInstall` の一覧取得と APK ダウンロードに `User-Agent` / `Accept` を付ける共通処理を追加し、更新開始 Intent と未知アプリ許可 Intent を実生成する試験にした。
- `St58PackageReplacedTest` は設定済み・未設定の受け手を実行し、印ファイルと `LocationService` 起動 Intent を確認するようにした。計測試験も受け手を実行し、位置ソースの capability と通知権限の実値を確認するようにした。
- 入れ替え台本は旧版・新版の `versionCode` を照合し、権限切替、pid、foreground type ログ、通知 ID 12、`am start` / UiAutomator / `geo fix` を検査するようにした。instrument 失敗時の出力も保持する。

### 修正ラウンド 2 の検証

- RED: レビュー時点では受け手テストが存在せず、単体試験は定数照合、台本は新版を二度入れていた。修正対象を再現するレビュー記録を確認した。
- GREEN: `scripts/verify-run 9.1` — PASS（ST58 32 本、下限 22 本以上、backup rules PASS）。
- `tools/android-st58-replace.sh --self-test` — PASS（必須 7 行と欠落行の失敗検知）。
- `scripts/verify-run 9.2` — FAIL（rc=1）。最初の段は `OK (1 test)`、2 段目は Android system_server の `ProcessRecord` null による `am instrument` 失敗。self-test は PASS。証跡は `evidence.jsonl` の最新値を維持した。
- `git diff --check` — PASS。
