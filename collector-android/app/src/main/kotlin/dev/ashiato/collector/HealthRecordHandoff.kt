// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.util.concurrent.ConcurrentLinkedQueue

/**
 * 画面（`MainActivity`）が観た健康データの権限の記録を、収集のサービスの未送信へ渡す受け渡し口（ST11 / design D11）。
 * 未送信はサービスが開く置き場で、画面が別に開くと 2 つの書き手になる。画面の観測は収集が始まる前（`start()` の前）なので、
 * ここに預け、サービスが立つとき（`onStartCommand`）に積む。
 */
object HealthRecordHandoff {
    private val pending = ConcurrentLinkedQueue<IngestRequest>()
    private val afterWritten = ConcurrentLinkedQueue<() -> Unit>()

    /** 預けるだけ（メモリ）。未送信に届いたことは [afterDrain] で知る —— 最後の状態の書き換えはそこまで待つ。 */
    fun offer(record: IngestRequest): Boolean = pending.add(record)

    /** 預けた記録が**置き場に書けた後**に呼ぶ。書けなかったら呼ばない（最後の状態が古いまま残り、次に観たときもう一度作る側に倒れる）。 */
    fun afterDrain(action: () -> Unit) {
        afterWritten.add(action)
    }

    fun drainInto(outbox: Outbox<IngestRequest>) {
        var allWritten = true
        while (true) {
            val record = pending.poll() ?: break
            if (!outbox.add(record)) allWritten = false
        }
        while (true) {
            val action = afterWritten.poll() ?: break
            if (allWritten) action()
        }
    }
}
