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

    fun offer(record: IngestRequest): Boolean = pending.add(record)

    fun drainInto(outbox: Outbox<IngestRequest>) {
        while (true) outbox.add(pending.poll() ?: return)
    }
}
