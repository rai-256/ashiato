// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.io.IOException
import java.io.RandomAccessFile
import java.time.Instant
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/** 捨てた理由（spec の 4 つ。`crates/server/src/drops.rs` の `REASONS` と同じ語）。 */
enum class DropReason(val wire: String) {
    AGE("age"),
    BYTES("bytes"),
    WRITE_FAILED("write_failed"),
    UNREADABLE("unreadable"),
}

/** 出来事の時刻の 1 時間（UTC）ごとの件数（C12）。 */
@Serializable
data class DropHour(val hour: String, val count: Int)

/**
 * 破棄の報告 1 件（ST04 / FR-9 / design D4）。**Rust 側（`crates/server/src/drops.rs`）と同じ形。**
 * 契約の正典は `docs/collector-contract.md`。**位置の値・原文の中身を持たない。**
 *
 * 冪等キーはサーバが `logical_source` + `raw` から作る。**一度でも送ろうとした報告は書き換えない**ので、
 * 再送は最初と同じ鍵になる（R2）。
 */
@Serializable
data class DropReport(
    override val id: String,
    @SerialName("user_id") val userId: String,
    @SerialName("logical_source") val logicalSource: String,
    @SerialName("device_id") val deviceId: String,
    val reason: String,
    @SerialName("created_at") val createdAt: String,
    @SerialName("range_start") val rangeStart: String? = null,
    @SerialName("range_end") val rangeEnd: String? = null,
    val count: Int,
    val hourly: List<DropHour> = emptyList(),
    val raw: String,
) : Outboxable

/** 捨てた記録から報告を組むのに要るもの。**位置の値は取り出さない**（出来事の時刻とソースだけ）。 */
interface Retainable : Outboxable {
    val logicalSource: String

    /** 出来事の時刻（RFC 3339） */
    val eventTime: String
}

/**
 * まだ送ろうとしていない報告（下書き）。**ソース × 理由ごとに 1 本だけ**持ち、続けて捨てた分を足す（C3）。
 * 送信に載せる前に `DropReport` へ凍結し、以後は書き換えない。
 */
@Serializable
data class DropDraft(
    val id: String,
    val source: String,
    val reason: String,
    val createdAt: String,
    /** 範囲の始まり（エポックミリ秒）。範囲を持たない報告では null */
    val startMs: Long? = null,
    /** 範囲の終わり（含まない） */
    val endMs: Long? = null,
    /** 最後に捨てた記録の出来事の時刻 */
    val lastMs: Long? = null,
    val count: Int = 0,
    /** UTC の正時（エポックミリ秒）→ 件数 */
    val hourly: Map<Long, Int> = emptyMap(),
) {
    /**
     * 報告へ組む。**原文は欄を決まった順に組んだ JSON の文字列そのもの** ——
     * 同じ下書きからは毎回同じ文字列になる（凍結の前に落ちて組み直しても、同じ鍵になる）。
     */
    fun toReport(userId: String, deviceId: String): DropReport {
        val start = startMs?.let { Instant.ofEpochMilli(it).toString() }
        val end = endMs?.let { Instant.ofEpochMilli(it).toString() }
        val hours = hourly.entries.sortedBy { it.key }
            .map { DropHour(Instant.ofEpochMilli(it.key).toString(), it.value) }
        val fields = linkedMapOf(
            "id" to JsonPrimitive(id),
            "user_id" to JsonPrimitive(userId),
            "logical_source" to JsonPrimitive(source),
            "device_id" to JsonPrimitive(deviceId),
            "reason" to JsonPrimitive(reason),
            "created_at" to JsonPrimitive(createdAt),
            "range_start" to (start?.let { JsonPrimitive(it) } ?: JsonNull),
            "range_end" to (end?.let { JsonPrimitive(it) } ?: JsonNull),
            "count" to JsonPrimitive(count),
            "hourly" to JsonArray(hours.map { JsonObject(mapOf("hour" to JsonPrimitive(it.hour), "count" to JsonPrimitive(it.count))) }),
        )
        return DropReport(
            id = id,
            userId = userId,
            logicalSource = source,
            deviceId = deviceId,
            reason = reason,
            createdAt = createdAt,
            rangeStart = start,
            rangeEnd = end,
            count = count,
            hourly = hours,
            raw = ingestJson.encodeToString(JsonObject(fields)),
        )
    }
}

/**
 * 捨てたことを破棄の報告にする（ST04 / FR-9 / 深掘り C2 / C3 / C8 / C12 / design D4）。
 *
 * - 送る前の下書きは `openFile` に置く（小さい。書き直してよい）。書けなければメモリに持つ
 * - `freeze()` で下書きを `frozen`（区切りの置き場）へ積み、以後は書き換えない。**送信に載せる前に必ず呼ぶ**
 * - 範囲の終わり: 同じソースで残った最も古い記録の出来事の時刻が、最後に捨てた記録から 1 時間以内ならその時刻、
 *   なければ最後に捨てた記録の直後（+1 ms）。**1 件だけ捨てても終わりは始まりより後**（R4）
 * - 次に捨てる記録の出来事の時刻が、下書きの範囲の終わりから 1 時間を超えて離れる、または始まりより前に戻るなら、
 *   下書きを凍結して新しい下書きにする（過去の写真をまとめて積んだソースで、何年もの範囲が 1 本にならないように）
 *
 * 「残った最も古い記録」は**積んだ順で先頭の記録**で読む（全件の出来事の時刻を読まない。位置では同じ）。
 */
class DropLedger(
    private val openFile: File,
    private val frozen: Outbox<DropReport>,
    private val userId: () -> String,
    private val deviceId: String,
    private val now: () -> Instant,
    private val newId: () -> String,
    private val log: (String) -> Unit,
) {
    private val drafts = LinkedHashMap<Pair<String, String>, DropDraft>()

    init {
        load()
    }

    /** 捨てた記録 1 件を下書きに足す。**保存はしない**（`endBatch` がまとめて書く）。 */
    @Synchronized
    fun dropped(source: String, reason: DropReason, eventTime: Instant) {
        val t = eventTime.toEpochMilli()
        val key = source to reason.wire
        var d = drafts[key]
        if (d != null && d.startMs != null && d.endMs != null && (t < d.startMs || t - d.endMs > HOUR_MS)) {
            // 離れた・戻った —— 伸ばさずに凍結し、新しい下書きにする
            freezeOne(key, d)
            d = null
        }
        val base = d ?: DropDraft(newId(), source, reason.wire, now().toString())
        val hour = Math.floorDiv(t, HOUR_MS) * HOUR_MS
        val last = maxOf(base.lastMs ?: t, t)
        drafts[key] = base.copy(
            startMs = minOf(base.startMs ?: t, t),
            lastMs = last,
            // ひとまず「最後に捨てた記録の直後」。残った記録が近ければ `endBatch` が伸ばす
            endMs = maxOf(base.endMs ?: (last + 1), last + 1),
            count = base.count + 1,
            hourly = base.hourly + (hour to ((base.hourly[hour] ?: 0) + 1)),
        )
    }

    /**
     * ひと続きの破棄の終わり。`remainingOldest` は同じソースで残った最も古い記録の出来事の時刻。
     * 範囲の終わりを決めて下書きを保存する。
     */
    @Synchronized
    fun endBatch(source: String, reason: DropReason, remainingOldest: Instant?) {
        val key = source to reason.wire
        val d = drafts[key] ?: return
        val last = d.lastMs ?: return
        val r = remainingOldest?.toEpochMilli()
        if (r != null && r > last && r - last <= HOUR_MS && r > (d.endMs ?: 0)) {
            drafts[key] = d.copy(endMs = r)
        }
        save()
    }

    /** 置き場の行が読めなかった件数（出来事の時刻が分からないので範囲を持たない。design D6）。 */
    @Synchronized
    fun unreadable(source: String, count: Int) {
        if (count <= 0) return
        val key = source to DropReason.UNREADABLE.wire
        val d = drafts[key] ?: DropDraft(newId(), source, DropReason.UNREADABLE.wire, now().toString())
        drafts[key] = d.copy(count = d.count + count)
        save()
    }

    /**
     * 立て直しで失われた、書けなかった記録（design D5）。**時間の枠ごとに範囲と時間ごとの件数を持つ報告**にし、
     * 数えきれなかった分は範囲を持たない報告にする。どれも失われた事実なので、その場で凍結する。
     */
    @Synchronized
    fun writeFailed(source: String, slots: List<WriteFailedLedger.Slot>, overflow: Int) {
        val sorted = slots.filter { it.count > 0 }.sortedBy { it.hourMs }
        var group = ArrayList<WriteFailedLedger.Slot>()
        fun emit() {
            if (group.isEmpty()) return
            val start = group.minOf { it.firstMs }
            val last = group.maxOf { it.lastMs }
            val draft = DropDraft(
                id = newId(),
                source = source,
                reason = DropReason.WRITE_FAILED.wire,
                createdAt = now().toString(),
                startMs = start,
                endMs = last + 1,
                lastMs = last,
                count = group.sumOf { it.count },
                hourly = group.associate { it.hourMs to it.count },
            )
            freezeDraft(draft)
            group = ArrayList()
        }
        for (s in sorted) {
            if (group.isNotEmpty() && s.firstMs - group.maxOf { it.lastMs } > HOUR_MS) emit()
            group += s
        }
        emit()
        if (overflow > 0) {
            freezeDraft(DropDraft(newId(), source, DropReason.WRITE_FAILED.wire, now().toString(), count = overflow))
        }
    }

    /** 送る前の下書きを全部凍結する（**送信に載せる前に呼ぶ**。spec「送ろうとした報告は書き換えられない」）。 */
    @Synchronized
    fun freeze(): Boolean {
        if (drafts.isEmpty()) return true
        for ((key, d) in drafts.entries.toList()) freezeOne(key, d)
        return save()
    }

    /** 送る前の下書き（試験のため）。 */
    @Synchronized
    fun drafts(): List<DropDraft> = drafts.values.toList()

    private fun freezeOne(key: Pair<String, String>, d: DropDraft) {
        drafts.remove(key)
        freezeDraft(d)
    }

    private fun freezeDraft(d: DropDraft) {
        // 書けなくても `Outbox` がメモリに持ち、次の送信で送る（spec「報告を置き場に書けない」）
        if (!frozen.add(d.toReport(userId(), deviceId))) {
            log(Telemetry.line("drop_report_not_persisted", count = d.count))
        }
        log(Telemetry.line("drop_report", count = d.count, error = d.reason))
    }

    private fun load() {
        if (!openFile.exists()) return
        val list = try {
            ingestJson.decodeFromString(ListSerializer(DropDraft.serializer()), openFile.readText())
        } catch (e: IOException) {
            log(Telemetry.line("drop_drafts_unreadable", error = e.javaClass.simpleName))
            return
        } catch (e: SerializationException) {
            log(Telemetry.line("drop_drafts_unreadable", error = e.javaClass.simpleName))
            return
        }
        // 凍結して積んだ後、下書きを消す前に落ちた跡は、同じ識別子の報告が既に積まれている。二重に積まない
        val already = frozen.snapshot().mapTo(HashSet()) { it.id }
        for (d in list) if (d.id !in already) drafts[d.source to d.reason] = d
    }

    private fun save(): Boolean = try {
        openFile.parentFile?.mkdirs()
        val tmp = File(openFile.parentFile, "${openFile.name}.tmp")
        tmp.writeText(ingestJson.encodeToString(ListSerializer(DropDraft.serializer()), drafts.values.toList()))
        if (!tmp.renameTo(openFile)) throw IOException("rename")
        true
    } catch (e: IOException) {
        // 書けなければメモリに持つ（次の送信で凍結して送る）
        log(Telemetry.line("drop_drafts_save_failed", error = e.message ?: e.javaClass.simpleName))
        false
    }

    companion object {
        const val HOUR_MS: Long = 60 * 60 * 1000L
    }
}

/**
 * 置き場に書けなかった記録の数え（ST04 / 深掘り C5 / R7 / design D5（仮））。
 *
 * **固定長 4 KiB のファイルを上書きするだけで伸びない** —— 端末の空きが尽きていても、
 * 既存のブロックの上書きは通ることが多い。中身は「UTC の時間 × 件数 × その時間の最初と最後の出来事の時刻」の枠と、
 * 枠に入りきらなかった件数。書き直せたら枠から引き、**起動時に枠が 0 でなければ、それは立て直しで失われた分**。
 *
 * 形（ビッグエンディアン）: `WFL1` / あふれ件数（8）/ 枠 × 204（正時のエポック秒 8・件数 4・最初 4・最後 4。
 * 最初と最後は正時からのミリ秒）。ファイルにも書けないときはメモリにだけ数える。
 */
class WriteFailedLedger(private val file: File, private val log: (String) -> Unit) {
    data class Slot(val hourMs: Long, val count: Int, val firstMs: Long, val lastMs: Long)

    private val slots = arrayOfNulls<Slot>(SLOTS)
    private var overflow = 0L
    private var usable = true

    init {
        try {
            file.parentFile?.mkdirs()
            if (!file.exists() || file.length() != SIZE.toLong()) {
                if (file.exists()) log(Telemetry.line("write_failed_ledger_reset", error = "size"))
                RandomAccessFile(file, "rw").use { it.setLength(SIZE.toLong()); it.seek(0); it.write(MAGIC) }
            }
            read()
        } catch (e: IOException) {
            usable = false
            log(Telemetry.line("write_failed_ledger_unavailable", error = e.javaClass.simpleName))
        }
    }

    /** 書けなかった記録 1 件を数える。 */
    @Synchronized
    fun failed(eventTime: Instant) {
        val t = eventTime.toEpochMilli()
        val hour = Math.floorDiv(t, DropLedger.HOUR_MS) * DropLedger.HOUR_MS
        val i = slots.indexOfFirst { it != null && it.hourMs == hour }
        if (i >= 0) {
            val s = slots[i]!!
            slots[i] = s.copy(count = s.count + 1, firstMs = minOf(s.firstMs, t), lastMs = maxOf(s.lastMs, t))
        } else {
            val free = slots.indexOfFirst { it == null }
            if (free >= 0) slots[free] = Slot(hour, 1, t, t) else overflow++
        }
        write()
    }

    /** 書き直せた（または送れた）記録 1 件を引く。 */
    @Synchronized
    fun recovered(eventTime: Instant) {
        val t = eventTime.toEpochMilli()
        val hour = Math.floorDiv(t, DropLedger.HOUR_MS) * DropLedger.HOUR_MS
        val i = slots.indexOfFirst { it != null && it.hourMs == hour }
        if (i >= 0) {
            val s = slots[i]!!
            slots[i] = if (s.count <= 1) null else s.copy(count = s.count - 1)
        } else if (overflow > 0) {
            overflow--
        }
        write()
    }

    /** 数えを取り出して 0 に戻す（起動時に呼ぶ。0 でなければ失われた分）。 */
    @Synchronized
    fun take(): Pair<List<Slot>, Int> {
        val got = slots.filterNotNull() to overflow.toInt()
        slots.fill(null)
        overflow = 0
        write()
        return got
    }

    /** ファイルの大きさ（試験のため。**伸びない**こと）。 */
    fun fileBytes(): Long = file.length()

    private fun read() {
        RandomAccessFile(file, "r").use { raf ->
            val magic = ByteArray(4)
            raf.readFully(magic)
            if (!magic.contentEquals(MAGIC)) return
            overflow = raf.readLong()
            for (i in 0 until SLOTS) {
                val hourSec = raf.readLong()
                val count = raf.readInt()
                val first = raf.readInt()
                val last = raf.readInt()
                if (hourSec != 0L && count > 0) {
                    val h = hourSec * 1000
                    slots[i] = Slot(h, count, h + first, h + last)
                }
            }
        }
    }

    private fun write() {
        if (!usable) return
        try {
            RandomAccessFile(file, "rw").use { raf ->
                val buf = java.nio.ByteBuffer.allocate(SIZE)
                buf.put(MAGIC)
                buf.putLong(overflow)
                for (s in slots) {
                    if (s == null) {
                        buf.putLong(0); buf.putInt(0); buf.putInt(0); buf.putInt(0)
                    } else {
                        buf.putLong(s.hourMs / 1000)
                        buf.putInt(s.count)
                        buf.putInt((s.firstMs - s.hourMs).toInt())
                        buf.putInt((s.lastMs - s.hourMs).toInt())
                    }
                }
                raf.seek(0)
                // **上書きだけ。** `setLength` を呼ばない（伸ばしも縮めもしない）
                raf.write(buf.array())
            }
        } catch (e: IOException) {
            // ファイルにも書けない。メモリにだけ数える（立て直されたら消える。spec）
            log(Telemetry.line("write_failed_ledger_save_failed", error = e.javaClass.simpleName))
        }
    }

    companion object {
        const val SIZE: Int = 4096
        private val MAGIC = byteArrayOf('W'.code.toByte(), 'F'.code.toByte(), 'L'.code.toByte(), '1'.code.toByte())

        /** (4096 - 4 - 8) / 20 */
        const val SLOTS: Int = 204
    }
}
