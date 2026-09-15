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
    /**
     * もう伸ばさない（凍結を待っている）。**立てたら書き換えない** —— 凍結の途中で落ちて下書きが戻ってきても、
     * 伸ばされずに同じ原文で積み直されるので、受け手の冪等で 1 件になる（review R19）
     */
    val closed: Boolean = false,
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
 * - 送る前の下書きは `openFile` に置く（小さい。書き直してよい）
 * - `freeze()` で下書きを `frozen`（区切りの置き場）へ積み、以後は書き換えない。**送信に載せる前に必ず呼ぶ**
 * - 範囲の終わり: 同じソースで残った最も古い記録の出来事の時刻が、最後に捨てた記録から 1 時間以内ならその時刻、
 *   なければ最後に捨てた記録の直後（+1 ms）。**1 件だけ捨てても終わりは始まりより後**（R4）
 * - 次に捨てる記録の出来事の時刻が、下書きの範囲の終わりから 1 時間を超えて離れる、または始まりより前に戻るなら、
 *   下書きを閉じて新しい下書きにする（過去の写真をまとめて積んだソースで、何年もの範囲が 1 本にならないように）
 *
 * **証拠は消す前に書く**（review R26）。捨てる側（`Retention`）は `record` で下書きに足して保存し、
 * 保存できたときだけ置き場から消す。保存できなければ下書きを元に戻す。
 *
 * 「残った最も古い記録」は**積んだ順で先頭の記録**で読む（全件の出来事の時刻を読まない。位置では同じ。design D17）。
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
    /** 下書き。閉じていないものは**ソース × 理由ごとに 1 本だけ** */
    private val drafts = ArrayList<DropDraft>()
    private var blankUserLogged = false

    init {
        load()
    }

    private fun openIndex(source: String, reason: String) =
        drafts.indexOfFirst { !it.closed && it.source == source && it.reason == reason }

    /** 捨てた記録 1 件を下書きに足す。**保存はしない**（`record` / `endBatch` がまとめて書く）。 */
    @Synchronized
    fun dropped(source: String, reason: DropReason, eventTime: Instant) {
        val t = eventTime.toEpochMilli()
        var i = openIndex(source, reason.wire)
        if (i >= 0) {
            val d = drafts[i]
            if (d.startMs != null && d.endMs != null && (t < d.startMs || t - d.endMs > HOUR_MS)) {
                // 離れた・戻った —— 伸ばさずに閉じ、新しい下書きにする
                drafts[i] = d.copy(closed = true)
                i = -1
            }
        }
        val base = if (i >= 0) drafts[i] else DropDraft(newId(), source, reason.wire, now().toString())
        val hour = Math.floorDiv(t, HOUR_MS) * HOUR_MS
        val last = maxOf(base.lastMs ?: t, t)
        val next = base.copy(
            startMs = minOf(base.startMs ?: t, t),
            lastMs = last,
            // ひとまず「最後に捨てた記録の直後」。残った記録が近ければ `endBatch` が伸ばす
            endMs = maxOf(base.endMs ?: (last + 1), last + 1),
            count = base.count + 1,
            hourly = base.hourly + (hour to ((base.hourly[hour] ?: 0) + 1)),
        )
        if (i >= 0) drafts[i] = next else drafts += next
    }

    /**
     * 捨てた記録をまとめて足し、**保存できたときだけ真**（保存できなければ足す前に戻す）。
     * 置き場から消すのはこれが真を返した後（`SegmentStore.dropHead`）。戻すための控えを返す。
     */
    @Synchronized
    fun record(reason: DropReason, dropped: List<Pair<String, Instant?>>): List<DropDraft>? {
        val before = drafts.toList()
        for ((source, t) in dropped) {
            if (t != null) {
                dropped(source, reason, t)
            } else {
                // 出来事の時刻が読めない記録は範囲に置けない。範囲を持たない報告に数える
                val i = openIndex(source, DropReason.UNREADABLE.wire)
                if (i >= 0) {
                    drafts[i] = drafts[i].copy(count = drafts[i].count + 1)
                } else {
                    drafts += DropDraft(newId(), source, DropReason.UNREADABLE.wire, now().toString(), count = 1)
                }
            }
        }
        if (save()) return before
        drafts.clear()
        drafts += before
        return null
    }

    /** `record` の後で置き場から消せなかったとき、足した分を戻す（二重に数えない）。 */
    @Synchronized
    fun restore(before: List<DropDraft>) {
        drafts.clear()
        drafts += before
        save()
    }

    /**
     * ひと続きの破棄の終わり。`remainingOldest` は同じソースで残った最も古い記録の出来事の時刻。
     * 範囲の終わりを決めて下書きを保存する。保存できたか。
     */
    @Synchronized
    fun endBatch(source: String, reason: DropReason, remainingOldest: Instant?): Boolean {
        val i = openIndex(source, reason.wire)
        if (i < 0) return true
        val d = drafts[i]
        val last = d.lastMs ?: return true
        val r = remainingOldest?.toEpochMilli()
        if (r != null && r > last && r - last <= HOUR_MS && r > (d.endMs ?: 0)) {
            drafts[i] = d.copy(endMs = r)
        }
        return save()
    }

    /** 置き場の行が読めなかった件数（出来事の時刻が分からないので範囲を持たない。design D6）。保存できたか。 */
    @Synchronized
    fun unreadable(source: String, count: Int): Boolean {
        if (count <= 0) return true
        val before = drafts.toList()
        val i = openIndex(source, DropReason.UNREADABLE.wire)
        if (i >= 0) {
            drafts[i] = drafts[i].copy(count = drafts[i].count + count)
        } else {
            drafts += DropDraft(newId(), source, DropReason.UNREADABLE.wire, now().toString(), count = count)
        }
        if (save()) return true
        drafts.clear()
        drafts += before
        return false
    }

    /**
     * 立て直しで失われた、書けなかった記録（design D5）。**時間の枠ごとに範囲と時間ごとの件数を持つ報告**にし、
     * 数えきれなかった分は範囲を持たない報告にする。どれも失われた事実なので、閉じた下書きにする。
     * **保存できたときだけ真**（呼び出し側はそのときだけ数えを 0 に戻す。review R15）。
     */
    @Synchronized
    fun writeFailed(source: String, slots: List<WriteFailedLedger.Slot>, overflow: Int): Boolean {
        val before = drafts.toList()
        val sorted = slots.filter { it.count > 0 }.sortedBy { it.hourMs }
        var group = ArrayList<WriteFailedLedger.Slot>()
        fun emit() {
            if (group.isEmpty()) return
            val start = group.minOf { it.firstMs }
            val last = group.maxOf { it.lastMs }
            drafts += DropDraft(
                id = newId(),
                source = source,
                reason = DropReason.WRITE_FAILED.wire,
                createdAt = now().toString(),
                startMs = start,
                endMs = last + 1,
                lastMs = last,
                count = group.sumOf { it.count },
                hourly = group.associate { it.hourMs to it.count },
                closed = true,
            )
            group = ArrayList()
        }
        for (s in sorted) {
            if (group.isNotEmpty() && s.firstMs - group.maxOf { it.lastMs } > HOUR_MS) emit()
            group += s
        }
        emit()
        if (overflow > 0) {
            drafts += DropDraft(newId(), source, DropReason.WRITE_FAILED.wire, now().toString(), count = overflow, closed = true)
        }
        if (save()) return true
        drafts.clear()
        drafts += before
        return false
    }

    /**
     * 下書きを全部凍結する（**送信に載せる前に呼ぶ**。spec「送ろうとした報告は書き換えられない」）。
     *
     * - **利用者識別子が決まっていなければ凍結しない**（review R20）。空のまま凍結した報告は受け手に断られ続け、書き換えられない
     * - 先に全部を閉じて保存してから積む。**積めなかった下書きは消さない**（メモリの報告は捨て、次の凍結でもう一度積む。review R16）
     */
    @Synchronized
    fun freeze(): Boolean {
        if (drafts.isEmpty()) return true
        val user = userId()
        if (user.isBlank()) {
            if (!blankUserLogged) log(Telemetry.line("drop_report_waiting", count = drafts.size, error = "no_user_id"))
            blankUserLogged = true
            return false
        }
        for (i in drafts.indices) if (!drafts[i].closed) drafts[i] = drafts[i].copy(closed = true)
        save()
        var all = true
        val it = drafts.iterator()
        while (it.hasNext()) {
            val d = it.next()
            val report = d.toReport(user, deviceId)
            if (frozen.add(report)) {
                log(Telemetry.line("drop_report", count = d.count, error = d.reason))
                it.remove()
            } else {
                // 積めなかった。メモリにだけある報告は捨て、閉じた下書きのまま次の凍結で積み直す（同じ原文になる）
                frozen.remove(listOf(report.id))
                log(Telemetry.line("drop_report_not_persisted", count = d.count))
                all = false
            }
        }
        return save() && all
    }

    /** 下書き（試験のため）。 */
    @Synchronized
    fun drafts(): List<DropDraft> = drafts.toList()

    private fun load() {
        if (!openFile.exists()) return
        val list = try {
            ingestJson.decodeFromString(ListSerializer(DropDraft.serializer()), openFile.readText())
        } catch (e: IOException) {
            // 一時的に読めないだけかもしれない。**上書きしないよう、脇へ退けてから新しく始める**
            aside(e)
            return
        } catch (e: SerializationException) {
            aside(e)
            return
        }
        // 凍結して積んだ後、下書きを消す前に落ちた跡は、同じ識別子の報告が既に積まれている。二重に積まない
        val already = frozen.snapshot().mapTo(HashSet()) { it.id }
        for (d in list) if (d.id !in already) drafts += d
    }

    /**
     * 読めない下書きのファイルを退避する（review R10。C9 / D6 の「捨てずに退避」をこのファイルにも）。
     * 中の件数は読めないので、**読めなかったこと 1 件**を範囲を持たない報告に数える。
     */
    private fun aside(e: Exception) {
        val moved = File(openFile.parentFile, "${openFile.name}.unreadable.${System.currentTimeMillis()}")
        val ok = openFile.renameTo(moved)
        log(Telemetry.line(if (ok) "drop_drafts_unreadable" else "drop_drafts_salvage_failed", error = e.javaClass.simpleName))
        if (ok) drafts += DropDraft(newId(), LOGICAL_SOURCE, DropReason.UNREADABLE.wire, now().toString(), count = 1)
    }

    /** 下書きを書く。書けたか。**書けなければメモリに持つ**（呼び出し側が戻すかを決める）。 */
    @Synchronized
    fun save(): Boolean = try {
        openFile.parentFile?.mkdirs()
        val tmp = File(openFile.parentFile, "${openFile.name}.tmp")
        tmp.writeText(ingestJson.encodeToString(ListSerializer(DropDraft.serializer()), drafts.toList()))
        if (!tmp.renameTo(openFile)) throw IOException("rename")
        true
    } catch (e: IOException) {
        log(Telemetry.line("drop_drafts_save_failed", error = e.javaClass.simpleName))
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

    /** いまの数え（起動時に読む。0 でなければ前のプロセスで失われた分）。**0 に戻さない。** */
    @Synchronized
    fun peek(): Pair<List<Slot>, Int> = slots.filterNotNull() to overflow.toInt()

    /**
     * 0 に戻す。**報告を保存できた後にだけ呼ぶ**（review R15）。先に戻すと、報告を書けないまま立て直したときに痕跡が消える。
     * ファイルに書けたか。
     */
    @Synchronized
    fun clear(): Boolean {
        slots.fill(null)
        overflow = 0
        return write()
    }

    /** 取り出して 0 に戻す（試験のため）。 */
    @Synchronized
    fun take(): Pair<List<Slot>, Int> = peek().also { clear() }

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

    private fun write(): Boolean {
        if (!usable) {
            // **ずっと諦めない**（review R34）。起動時に空きが無くて作れなかっただけなら、空きが戻れば作れる
            usable = runCatching {
                RandomAccessFile(file, "rw").use { if (it.length() != SIZE.toLong()) it.setLength(SIZE.toLong()) }
            }.isSuccess
            if (!usable) return false
        }
        return try {
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
            true
        } catch (e: IOException) {
            // ファイルにも書けない。メモリにだけ数える（立て直されたら消える。design D5）
            log(Telemetry.line("write_failed_ledger_save_failed", error = e.javaClass.simpleName))
            false
        }
    }

    companion object {
        const val SIZE: Int = 4096
        private val MAGIC = byteArrayOf('W'.code.toByte(), 'F'.code.toByte(), 'L'.code.toByte(), '1'.code.toByte())

        /** (4096 - 4 - 8) / 20 */
        const val SLOTS: Int = 204
    }
}
