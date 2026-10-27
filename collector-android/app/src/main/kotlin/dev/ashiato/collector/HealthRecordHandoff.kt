// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * 画面（`MainActivity`）が観た健康データの権限の記録を、収集のサービスの未送信へ渡す受け渡し口（ST11 / design D11）。
 * 未送信はサービスが開く置き場で、画面が別に開くと 2 つの書き手になる。画面の観測は収集が始まる前（`start()` の前）なので、
 * ここに預け、サービスが立つとき（`onStartCommand`）に積む。
 *
 * 記録と「書けた後」の手続きは**預けた順の 1 本の列**に並ぶ。手続きは、直前の手続きより後に預けた記録（＝その手続きの記録）が
 * 全部書けたときだけ走る。
 */
object HealthRecordHandoff {
    private sealed interface Item {
        class Record(val record: IngestRequest) : Item
        class Action(val action: () -> Unit) : Item
    }

    private val queue = ArrayDeque<Item>()

    /**
     * 直前の手続きより後に預けた記録のどれかが書けなかったか（review/code.md R22）。**drain をまたいで持つ** ——
     * 記録とその手続きが別々の drain に分かれても、書けなかった記録の手続き（最後の状態の書き換え）を走らせない。
     */
    private var failedSinceAction = false

    /** 画面の recorder の控え（review/code.md R24）。Activity が作り直されても残るよう、プロセスで 1 つ。 */
    val screenPending = HealthPendingSlot()

    /** 預けるだけ（メモリ）。未送信に届いたことは [afterDrain] で知る —— 最後の状態の書き換えはそこまで待つ。 */
    fun offer(record: IngestRequest): Boolean = synchronized(this) { queue.add(Item.Record(record)) }

    /** 直前の [afterDrain] より後に預けた記録が**置き場に書けた後**に呼ぶ。書けなかったら呼ばない（最後の状態が古いまま残り、次に観たときもう一度作る側に倒れる）。 */
    fun afterDrain(action: () -> Unit) {
        synchronized(this) { queue.addLast(Item.Action(action)) }
    }

    /**
     * 預けた分を未送信へ積む（review/code.md R6）。1 件が置き場に書けなくても、**他の記録の手続きは走らせる**。
     * 書けなかった記録は未送信がメモリに持って次に積むときに書き直す（[Outbox.add]）ので、ここで持ち直さない ——
     * 持ち直すと同じ記録を 2 度積む。その記録の手続きだけを捨てる（置き場が古いまま残り、次に観たとき作り直す側に倒れる）。
     * 積むところ・手続きが投げても**残りを捨てずに続け**、投げたものは [onError] に渡す（収集の糸から例外を抜かない。R23）。
     * 投げた記録は書けなかったものとして扱う。
     */
    fun drainInto(outbox: Outbox<IngestRequest>, onError: (Throwable) -> Unit = {}) {
        val taken = synchronized(this) { queue.toList().also { queue.clear() } }
        for (item in taken) {
            when (item) {
                is Item.Record -> {
                    val written = attempt(onError) { outbox.add(item.record) } ?: false
                    if (!written) synchronized(this) { failedSinceAction = true }
                }
                is Item.Action -> {
                    val failed = synchronized(this) { failedSinceAction.also { failedSinceAction = false } }
                    if (!failed) attempt(onError) { item.action() }
                }
            }
        }
    }

    /** [block] を走らせ、投げたら [onError] に渡して null を返す。 */
    // 何が投げられても収集の糸を落とさず、種別だけ渡して続けるのが仕様（R23）
    @Suppress("TooGenericExceptionCaught")
    private inline fun <R> attempt(onError: (Throwable) -> Unit, block: () -> R): R? = try {
        block()
    } catch (e: Exception) {
        onError(e)
        null
    }
}
