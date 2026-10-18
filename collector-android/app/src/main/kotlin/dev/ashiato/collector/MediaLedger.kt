// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File
import java.io.IOException
import java.time.Duration

/** 1 回の契機で読む件数（design D1（仮））。残りは同じ契機の続きとして続けて読む。 */
const val SCAN_BATCH: Int = 200

/** 保存の知らせをまとめる待ち（design D1）。続けて来た知らせを 1 回にする。 */
const val OBSERVER_DEBOUNCE_MS: Long = 10_000L

/** 世代の数を聞く見直しの間隔（design D1 の 2 / Q11）。 */
const val REVIEW_EVERY_MS: Long = 6 * 60 * 60 * 1000L

/** 全部の突き合わせの間隔（design D1 の 3 / D2）。 */
val FULL_RECONCILE_EVERY: Duration = Duration.ofHours(HOURS_PER_DAY)

private const val HOURS_PER_DAY = 24L

/** 台帳の置き場の名前（design D2）。 */
const val MEDIA_LEDGER_DIR: String = "media"

/**
 * 見分けの台帳の 1 行（design D2）。置き場ごとの `media/ledger-<置き場>.jsonl` に 1 行ずつ。
 *
 * [rawDigest] は**最後に送った原文の指紋**（番号と世代の数を除いたもの）。D2 の列に足した 1 列で、
 * 「中身は同じで行や原文だけ変わった」を、送り直す／送らないに分けるために要る（D4 の更新）。
 */
@Serializable
data class LedgerEntry(
    @SerialName("media_id") val mediaId: Long,
    val sha256: String,
    val size: Long,
    @SerialName("date_modified") val dateModified: Long,
    @SerialName("generation_modified") val generationModified: Long,
    val source: String,
    val trashed: Boolean,
    @SerialName("raw_digest") val rawDigest: String,
)

/** 置き場ごとの読み進めた印（design D2 の `cursor.json`）。 */
@Serializable
data class VolumeCursor(val generation: Long, val version: String?)

@Serializable
data class MediaCursor(
    val volumes: Map<String, VolumeCursor> = emptyMap(),
    /** 前回の全部の突き合わせの時刻（ISO）。無ければまだ 1 度も行っていない */
    @SerialName("last_full_reconcile") val lastFullReconcile: String? = null,
)

/**
 * 台帳とカーソルを端末の保存領域に置く。**書いてから差し替える**（途中で落ちても前の版が残る）。
 *
 * 読めなければ空から始める（落とさない）—— 失うのは「どこまで見たか」だけで、
 * 写真そのものは端末にあり、S-01 は外部識別子（内容ハッシュ）で畳むので記録は増えない（D2）。
 */
class MediaLedgerStore(private val dir: File, private val log: (String) -> Unit = {}) {
    private val json = Json { ignoreUnknownKeys = true }

    private fun ledgerFile(volume: String) = File(dir, "ledger-${volume.replace(UNSAFE, "_")}.jsonl")

    private val cursorFile get() = File(dir, "cursor.json")

    fun loadLedger(volume: String): MutableMap<Long, LedgerEntry> {
        val out = LinkedHashMap<Long, LedgerEntry>()
        val file = ledgerFile(volume)
        if (!file.exists()) return out
        try {
            file.forEachLine { line ->
                if (line.isBlank()) return@forEachLine
                try {
                    val entry = json.decodeFromString(LedgerEntry.serializer(), line)
                    out[entry.mediaId] = entry
                } catch (_: IllegalArgumentException) {
                    log(Telemetry.line("media_ledger_line_unreadable", source = null))
                }
            }
        } catch (e: IOException) {
            log(Telemetry.line("media_ledger_unreadable", source = null, error = e.javaClass.simpleName))
        }
        return out
    }

    /** 書けたか。書けなければ呼び手は読み進めた印を進めない。 */
    fun saveLedger(volume: String, entries: Collection<LedgerEntry>): Boolean =
        write(ledgerFile(volume), entries.joinToString("") { json.encodeToString(LedgerEntry.serializer(), it) + "\n" })

    fun loadCursor(): MediaCursor = try {
        if (cursorFile.exists()) {
            json.decodeFromString(MediaCursor.serializer(), cursorFile.readText())
        } else {
            MediaCursor()
        }
    } catch (e: IOException) {
        log(Telemetry.line("media_cursor_unreadable", source = null, error = e.javaClass.simpleName))
        MediaCursor()
    } catch (e: IllegalArgumentException) {
        log(Telemetry.line("media_cursor_unreadable", source = null, error = e.javaClass.simpleName))
        MediaCursor()
    }

    fun saveCursor(cursor: MediaCursor): Boolean =
        write(cursorFile, json.encodeToString(MediaCursor.serializer(), cursor))

    private fun write(file: File, text: String): Boolean = try {
        dir.mkdirs()
        val tmp = File(dir, "${file.name}.tmp")
        tmp.writeText(text)
        if (!tmp.renameTo(file)) throw IOException("rename")
        true
    } catch (e: IOException) {
        log(Telemetry.line("media_ledger_write_failed", source = null, error = e.javaClass.simpleName))
        false
    }

    private companion object {
        val UNSAFE = Regex("[^A-Za-z0-9_-]")
    }
}

/** 台帳の 1 行がどの置き場のものか。[stale] は一覧が作り直される前の番号、[fresh] はこの契機で作った行。 */
class Slot(val volume: String, val entry: LedgerEntry, val stale: Boolean, val fresh: Boolean)

/** 同じハッシュを持つ台帳の行を引く（置き場をまたぐ）。 */
class HashIndex {
    private val byHash = HashMap<String, MutableList<Slot>>()

    fun add(slot: Slot) {
        byHash.getOrPut(slot.entry.sha256) { ArrayList() }.add(slot)
    }

    fun remove(volume: String, entry: LedgerEntry) {
        byHash[entry.sha256]?.removeAll { it.volume == volume && it.entry.mediaId == entry.mediaId }
    }

    /** その番号の行とは別の、同じハッシュの行 */
    fun twinOf(sha: String, volume: String, mediaId: Long): Slot? =
        byHash[sha].orEmpty().firstOrNull { !(it.volume == volume && it.entry.mediaId == mediaId) }

    /** 条件に合う行を 1 つでも持つハッシュ */
    fun hashesWhere(test: (Slot) -> Boolean): Set<String> =
        byHash.filterValues { slots -> slots.any(test) }.keys
}
