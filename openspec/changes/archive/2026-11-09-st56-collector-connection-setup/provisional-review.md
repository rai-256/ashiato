# 仮で進めた決定の確かめ（ST56 st56-collector-connection-setup）

## D25
- 判定: 確か
- 根拠: 
  1. 失わない側（次の要求から止める）で 4 つの loss は起きない。`exported`: 門は `Drainer` の周回と送り手の 1 本ごと（collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt:260-281 の `held()`）、`OriginalUploader.pass` の 1 件ごと（OriginalUploader.kt:176 `canSend()`）、要求ごと（LocationService.kt:256-259 `sendingToken`）で見ている。`sendingToken` は HttpTransport.kt:58 / OriginalAndroid.kt:85 の `setRequestProperty` の中、つまり `outputStream`・`responseCode` で接続する前に呼ばれるので、投げると何も送らずに `Outcome.Unreachable`（HttpTransport.kt:71）で終わる。送り先 → 資格情報 → 門の順なので、保存（送り先・資格情報・保留を 1 回の編集で書く）の後に読んだ資格情報なら、必ず保留を見る。`discarded`: 止めた記録は送り箱に残る（Unreachable では消さない）。原本は `Step.Retry` → `scheduleRetry` でバックオフ（メモリだけ。OriginalUploader.kt:214-217）になるだけで、欠けにする道は `409 record_missing` の 7 日（:355-362）だけ。要求を出さないので 409 は起きない。破棄の報告は `freeze()` の後に止まっても凍結したまま残り、承認の後に送る（書き直しではない）。`uncaptured` と `rewrite-all` は関わらない。90 日で捨てるのは Q7 の答えの結果で、D25 が足したものではない。
     `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St56DestinationGateTest*'` → rc=0（`続けて送っている最中に別の送り先を保存すると残りを送らない` は St56DestinationGateTest.kt:528）
  2. 費用: 押した後の契機まで残りが待つだけ（design.md D25）。変更は Story の中の 3 か所の門で済み、要件の本文は変えていない。台帳の `cost` は null だが、design の記述で足りる
  3. `follows: Q7`: deep-answers-1.txt:7 で Q7 は「『この送り先へ送る』を押すまで何も送らない」。D25 は送っている途中でもこれを守る側で、同じ向き
  4. `rests_on`: D11（要求ごとに読む）と Q7 で足りる。保留が立つ条件（D13・D19（仮）の比べ方）は D25 の止め方を変えない（D25 は「保留なら止める」だけで、保留の計算に依らない）ので、土台からの抜けとはみなさない
