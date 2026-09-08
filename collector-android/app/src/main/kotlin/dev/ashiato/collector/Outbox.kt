// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * まだ送れていない記録の置き場。**上限も破棄も持たない** —— それは ST04 の担当で、
 * ここでは「失敗したら未送信のまま残す」だけを保証する（design の Non-Goals）。
 *
 * 位置取得は前景サービスの糸から、送信は別の契機から触るので同期する。
 */
class Outbox {
    private val pending = ArrayDeque<IngestRequest>()

    /** 積む。**精度でも件数でもふるい落とさない**（design D11）。 */
    @Synchronized
    fun add(request: IngestRequest) {
        pending.addLast(request)
    }

    /** いま溜まっているもの。送信はこの全部をまとめて 1 回で送る（design D9）。 */
    @Synchronized
    fun snapshot(): List<IngestRequest> = pending.toList()

    @Synchronized
    fun size(): Int = pending.size

    /**
     * 送れたものだけを取り除く。**識別子で消す** ——
     * 送信中に新しい記録が積まれても取り違えないため。
     */
    @Synchronized
    fun remove(ids: Collection<String>) {
        val gone = ids.toHashSet()
        pending.removeAll { it.id in gone }
    }
}
