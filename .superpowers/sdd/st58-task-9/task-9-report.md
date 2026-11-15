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

## 変更ファイル

`UpdateInstall.kt`、`LocationService.kt`、`AndroidManifest.xml`、`St58UpdateInstallTest.kt`、`St58PackageReplacedTest.kt`、`St58PackageReplacedInstrumentedTest.kt`、`tools/android-st58-replace.sh`、`premises.md`、`evidence.jsonl`。

## 自己レビュー

9.2 の実機通しはエミュレータ基盤障害で未完了。台本の旧 APK 保存・権限設定・同一 pid 検証・self-test 欠落検知は実装済みだが、実エミュレータ PASS は主張しない。
