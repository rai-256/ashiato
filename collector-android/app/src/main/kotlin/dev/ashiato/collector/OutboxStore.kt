// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.io.IOException
import kotlinx.serialization.SerializationException

/**
 * 未送信の置き場の**永続化先**。Android のファイルと試験用の偽物を差し替えられるようにしてある。
 *
 * **既定を持たせない。** 持たせるとインスタンスの中だけに積む実装が黙って選ばれ、
 * プロセスが立て直されたときに未送信が消える —— それがこの型を足した理由そのもの。
 */
interface OutboxStore {
    fun load(): List<IngestRequest>
    fun save(requests: List<IngestRequest>)
}

/**
 * 未送信を端末の保存領域に置く（深掘り 第 2 回 / specs「未送信を、収集の停止と再開をまたいで保持する」）。
 *
 * **これが無いと、`START_STICKY` でプロセスが立て直されるたびに最大 5 分ぶんが無言で消える。**
 * この Story は「捨てたものは復元できない」を根拠に精度フィルタを外している（design D11）のに、
 * 同じ理由で失われる経路が残っていた。
 *
 * 書き込みは 60 秒に 1 回（取得の契機ごと）なので、毎回まるごと書き直して構わない。
 */
class FileOutboxStore(
    private val file: File,
    private val log: (String) -> Unit = {},
) : OutboxStore {
    override fun load(): List<IngestRequest> {
        if (!file.exists()) return emptyList()
        return try {
            ingestJson.decodeFromString<List<IngestRequest>>(file.readText())
        } catch (e: SerializationException) {
            emptyList<IngestRequest>().also { salvage(e.javaClass.simpleName) }
        } catch (e: IOException) {
            emptyList<IngestRequest>().also { salvage(e.javaClass.simpleName) }
        }
    }

    /**
     * 読めなかったものを**捨てずに脇へ退ける**。上書きして消すと、
     * 何が失われたのか後から誰にも分からない（FR-18 と同じ理由）。
     */
    private fun salvage(kind: String) {
        val aside = File(file.parentFile, "${file.name}.unreadable")
        val moved = file.renameTo(aside)
        log(Telemetry.line(if (moved) "outbox_unreadable" else "outbox_unreadable_kept", error = kind))
    }

    override fun save(requests: List<IngestRequest>) {
        // 一時ファイルへ書いてから差し替える。途中で落ちても半端なファイルが残らない
        val tmp = File(file.parentFile, "${file.name}.tmp")
        try {
            tmp.writeText(ingestJson.encodeToString(requests))
            if (!tmp.renameTo(file)) {
                // **黙って諦めない。** 積んだものはメモリには残るので、次の契機で書き直される
                log(Telemetry.line("outbox_save_failed", count = requests.size, error = "rename"))
            }
        } catch (e: IOException) {
            log(Telemetry.line("outbox_save_failed", count = requests.size, error = e.javaClass.simpleName))
        }
    }
}
