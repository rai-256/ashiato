// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import kotlinx.serialization.KSerializer

/**
 * 置き場に書けなかった記録の行き先（ST04 / 深掘り C5 / design D5）。
 * 書けなかったものは**メモリに持って書き直しを試みる**。書き直せたら `recovered`、
 * メモリに持ちきれず手放したら `lost`（そのときに破棄として報告する）。
 */
interface WriteFailures<T> {
    fun failed(item: T)

    fun recovered(item: T)

    fun lost(item: T)
}

/**
 * まだ送れていないものの置き場（ST01 / ST02 / ST04）。
 *
 * **中身は端末の保存領域の区切りファイルに置く**（`SegmentStore`。深掘り 第 2 回 / ST04 の C7）。
 * インスタンスの中だけに積むと `START_STICKY` の立て直しで消え、1 本の文字列に全件を読むと 90 日ぶんで落ちる。
 *
 * 位置取得は前景サービスの糸から、送信は別の契機から触るので同期する。
 * **保持の上限はここでは持たない** —— それは `Retention` の担当（design D2 / D3）。
 */
class Outbox<T : Outboxable>(
    private val store: SegmentStore<T>,
    /** 積んだ時点の経過（`AgeClock.now`）。90 日の数えの起点（深掘り Q4） */
    private val age: () -> Long,
    /** 書けなかった記録を数える口。**記録だけが持つ**（生存信号と破棄の報告は数えない） */
    private val writeFailures: WriteFailures<T>? = null,
    /** 積んだ後に呼ぶ（保持の上限の見回りを積む契機に相乗りさせる。新しい契機を起こさない） */
    private val afterAdd: () -> Unit = {},
) {
    /** 置き場に書けなかったもの。**書けるようになったら先に書く**（積んだ順を保つ） */
    private val unwritten = ArrayDeque<Stored<T>>()

    /**
     * 積む。**精度でも件数でもふるい落とさない**（ST01 design D11）。書けたかを返す。
     * 書けなくてもメモリには持つ（次に積むときに書き直す）。
     */
    fun add(request: T): Boolean {
        val ok = synchronized(this) {
            val enq = age()
            val written = flushUnwritten() && store.append(request, enq)
            if (!written) {
                unwritten.addLast(Stored(request, enq))
                writeFailures?.failed(request)
                // **メモリに持ちきれない分は手放す**（端末の空きが尽きたまま続くと、メモリで落ちて収集が止まる）。
                // 手放すのは**失ったことを報告できる置き場だけ**（記録）。生存信号と破棄の報告は数えを持たないので、
                // 手放すと痕跡なく消える（review R32。量は 1 日数件で、メモリに持ちきれないほど溜まらない）
                val failures = writeFailures
                if (failures != null) {
                    while (unwritten.size > MAX_UNWRITTEN) failures.lost(unwritten.removeFirst().item)
                }
            }
            written
        }
        // **錠の外で呼ぶ**（review R14）。錠を持ったまま上限の見回りへ入ると、見回りの錠 → 置き場の錠の順で
        // 入ってくる送信の糸と行き違い、位置の取得も送信も止まる
        afterAdd()
        return ok
    }

    /**
     * ST01 / ST02 の JSONL から取り込む（`migrateLegacyOutbox`）。**書けなかった記録を数えない・メモリに持たない** ——
     * 元のファイルが残ることがその記録の置き場になる（数えると、取り込み直した記録を「失った」とも報告する。review R7）。
     */
    @Synchronized
    fun importLegacy(items: List<T>, enqAgeMs: Long): Boolean = flushUnwritten() && store.appendAll(items, enqAgeMs)

    private fun flushUnwritten(): Boolean {
        while (unwritten.isNotEmpty()) {
            val next = unwritten.first()
            if (!store.append(next.item, next.enqAgeMs)) return false
            unwritten.removeFirst()
            writeFailures?.recovered(next.item)
        }
        return true
    }

    /**
     * 送信に載せる分。先頭（最も古いもの）から最大 `limit` 件で、`skip` の識別子は飛ばす。
     * **全件をメモリに載せない**（spec「読み戻しでメモリに載る件数は 1 回に載る件数を超えない」）。
     */
    @Synchronized
    fun head(limit: Int, skip: Set<String> = emptySet()): List<T> {
        val out = store.head(limit, skip).mapTo(ArrayList()) { it.item }
        for (u in unwritten) {
            if (out.size >= limit) break
            if (u.item.id !in skip) out += u.item
        }
        return out
    }

    /** 最も古い未送信（積んだときの経過つき）。上限の見回りと知らせが使う。 */
    @Synchronized
    fun oldest(): Stored<T>? = store.head(1).firstOrNull() ?: unwritten.firstOrNull()

    /** 未送信が `n` 件より多いか（溜まっている間は続けて送る。design D12）。 */
    @Synchronized
    fun hasMoreThan(n: Int): Boolean =
        unwritten.size > n || store.hasMoreThan(n - unwritten.size)

    /** いま溜まっているもの全部。**試験と診断のため** —— 送信は `head` を使う。 */
    @Synchronized
    fun snapshot(): List<T> = store.readAll() + unwritten.map { it.item }

    /** 件数。**全区切りを読む**ので送信の経路では使わない。 */
    @Synchronized
    fun size(): Int = store.count() + unwritten.size

    /**
     * 送れたものだけを取り除く。**識別子で消す** —— 送信中に新しい記録が積まれても取り違えないため。
     * **残りを書き直さない**（`.acked` への追記だけ）。
     */
    @Synchronized
    fun remove(ids: Collection<String>): Boolean {
        val gone = ids.toHashSet()
        val sentFromMemory = unwritten.filter { it.item.id in gone }
        if (sentFromMemory.isNotEmpty()) {
            unwritten.removeAll { it.item.id in gone }
            // メモリにだけあったものが送れた —— 失われていないので数えを戻す
            sentFromMemory.forEach { writeFailures?.recovered(it.item) }
        }
        return store.remove(gone - sentFromMemory.mapTo(HashSet()) { it.item.id })
    }

    /**
     * 先頭から `shouldDrop` が真の間取り除く（90 日。design D2）。
     * **`commit` が証拠を書けたときだけ消す**。印を書けなければ `rollback`（`SegmentStore.dropHead`）。
     */
    @Synchronized
    fun dropHead(
        shouldDrop: (Stored<T>) -> Boolean,
        commit: (List<Stored<T>>) -> Boolean,
        rollback: (List<Stored<T>>) -> Unit,
    ): Int = store.dropHead(shouldDrop, commit, rollback)

    /** 置き場のバイトが `maxBytes` 以下になるまで先頭から取り除く（2 GB。design D3）。証拠の書き方は `dropHead` と同じ。 */
    @Synchronized
    fun dropUntilBytes(
        maxBytes: Long,
        commit: (List<Stored<T>>) -> Boolean,
        rollback: (List<Stored<T>>) -> Unit,
    ): Int = store.dropUntilBytes(maxBytes, commit, rollback)

    /** 置き場のバイト（2 GB の勘定）。 */
    @Synchronized
    fun bytes(): Long = store.liveBytes()

    companion object {
        /** 書けなかった記録をメモリに持つ上限（位置で約 1 週間ぶん）。超えた分は `lost` で報告する */
        const val MAX_UNWRITTEN: Int = 10_000

        /**
         * 置き場を関心にしない呼び出し（試験・移行）のための組み立て。
         * 時計は壁時計ではなく**単調時計**を使う（`JvmDeviceClock`）。
         */
        fun <T : Outboxable> inDir(
            dir: File,
            serializer: KSerializer<T>,
            log: (String) -> Unit,
        ): Outbox<T> {
            val clock = AgeClock(JvmDeviceClock, File(dir, "age-clock.txt"), log)
            return Outbox(
                SegmentStore(File(dir, "segments"), serializer, File(dir, "unreadable.jsonl"), log),
                age = clock::now,
            )
        }
    }
}

/** JVM の時計（試験と、端末の時計を使わない組み立てのため）。起動回数は持たない。 */
object JvmDeviceClock : DeviceClock {
    override fun wallMs(): Long = System.currentTimeMillis()

    override fun monoMs(): Long = System.nanoTime() / 1_000_000

    override fun bootCount(): Int? = null
}
