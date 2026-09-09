// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * まだ送れていない記録の置き場。**上限も破棄も持たない** —— それは ST04 の担当で、
 * ここでは「失敗したら未送信のまま残す」だけを保証する（design の Non-Goals）。
 *
 * 位置取得は前景サービスの糸から、送信は別の契機から触るので同期する。
 *
 * **中身は `store` に置く**（深掘り 第 2 回 / specs「未送信を、収集の停止と再開をまたいで保持する」）。
 * インスタンスの中だけに積むと、`START_STICKY` でプロセスが立て直されたとき
 * 最大 5 分ぶんが無言で消える。
 */
class Outbox(private val store: OutboxStore) {
    private val pending = ArrayDeque(store.load())

    /** 積む。**精度でも件数でもふるい落とさない**（design D11）。 */
    @Synchronized
    fun add(request: IngestRequest): Boolean {
        pending.addLast(request)
        // **追記できる置き場なら追記する**（design D22）—— 全件書き直しは
        // 圏外が続いて未送信が伸びたとき、60 秒ごとにフラッシュを焼く。
        // 書けなくてもメモリには積む（次の契機で書き直される）。**失敗は呼び出し側へ返す**
        return store.append(request) { pending.toList() }
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
    fun remove(ids: Collection<String>): Boolean {
        val gone = ids.toHashSet()
        pending.removeAll { it.id in gone }
        // 取り除いたあとは全件を書き直す。**送信の契機（5 分）でしか起きない**ので割に合う
        return store.save(pending.toList())
    }
}
