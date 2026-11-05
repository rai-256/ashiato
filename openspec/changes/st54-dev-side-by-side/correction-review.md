# 機械が承認した検証コマンドの訂正の確かめ（code-verify 手 7）

## 9.2 2026-11-04T09:21:00+00:00
- 壊したもの: ST50 の「404（問い合わせの口が無い古いサーバ）なら更新を待つ」を壊した。collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt:64 の `HTTP_NOT_FOUND -> ContractAnswer.NoEndpoint` を `HTTP_NOT_FOUND -> ContractAnswer.Transient` に変えた（作業ツリーで一時的に。戻した後の `git status --porcelain` と `git diff` は壊す前と同じ）
- 実行: scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests *St50*
- 結果: 落ちた
