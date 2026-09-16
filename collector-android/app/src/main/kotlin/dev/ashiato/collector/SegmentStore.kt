// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.io.RandomAccessFile
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull

/** 置き場の 1 件と、それを積んだときの経過（`AgeClock.now()`）。 */
data class Stored<T>(val item: T, val enqAgeMs: Long)

/**
 * 未送信を**区切ったファイル**に置く（ST04 / 深掘り C7 / R13 / design D1（仮））。
 *
 * ST01 の `FileOutboxStore` は起動時に全件を 1 つの文字列として読み、送るたびに全件を書き直していた。
 * 位置だけでも 90 日で約 94 MB・129,600 件になり、読み戻しの `OutOfMemoryError` は受け止めていないので
 * **起動で落ちて立て直しを繰り返し、収集そのものが止まりうる**。
 *
 * - `dir/<連番>.jsonl` —— 1 行 1 件 `{"enq":<経過>,"item":{…}}`。**追記は最新の区切りだけ**。1 本あたり `segmentBytes` まで
 * - `dir/<連番>.acked` —— その区切りから取り除いた**行番号**の追記（`L<行番号>\t<バイト数>`）。**残りを書き直さない**
 * - 区切りの全行が取り除かれたら、区切りと `.acked` を消す
 * - **読むのは先頭（最も古い区切り）から必要な分だけ**。全件をメモリに載せない
 * - 読めない行は `unreadable` へ**バイトのまま追記して退避**し、`.acked` に印を付ける（C9 / design D6。捨てない）
 *
 * 行の鍵は識別子ではなく**行番号**（review/code.md R27）。識別子を鍵にすると、同じ識別子の行が 2 本ある区切り
 * （取り込みを途中からやり直した跡）で片方が永久に隠れ、区切りも消えない。
 *
 * 同期は呼び出し側（`Outbox`）が持つ。
 */
class SegmentStore<T : Outboxable>(
    private val dir: File,
    private val serializer: KSerializer<T>,
    /** 読めない行の退避先。**上限の対象にせず、消さない**（design D6） */
    private val unreadable: File,
    private val log: (String) -> Unit,
    private val segmentBytes: Long = SEGMENT_BYTES,
    /** 読めない行を退避したときに呼ぶ（件数）。報告は退避先の行数から数える（`LocationService`） */
    private val onUnreadable: (Int) -> Unit = {},
) {
    /** 区切り 1 本の、メモリに持つ分だけの状態。**行そのものは持たない。** */
    private class Segment(val seq: Long, val file: File, val ackFile: File) {
        /** 取り除いた行の行番号。読んだときにだけ埋める（読めなければ null のまま） */
        var acked: MutableSet<Int>? = null

        /** 取り除いた行のバイト数の合計 */
        var ackedBytes: Long = 0

        /** 空でない行の数。区切りを終わりまで読んだときにだけ分かる */
        var lines: Int? = null
    }

    /** 1 本の区切りを読んだ結果。 */
    private enum class Read { DONE, STOPPED, FAILED }

    private val segments = java.util.TreeMap<Long, Segment>()

    /** 区切りの一覧を取れたか。**取れないうちは何も書かない**（既存の区切りの前に新しい連番を作らない） */
    private var listed = false

    /** 末尾が改行で終わっていることを確かめた区切りと `.acked`（書き込みに失敗したら外して見直す） */
    private val terminated = HashSet<Long>()
    private val ackTerminated = HashSet<Long>()

    init {
        list()
    }

    private fun list(): Boolean {
        if (listed) return true
        dir.mkdirs()
        val names = dir.listFiles()
        if (names == null) {
            // **黙って空として進まない**（review R38）。既存の未送信が見えないまま、連番 0 から書き始めることになる
            log(Telemetry.line("outbox_list_failed"))
            return false
        }
        for (f in names) {
            val seq = f.name.substringBefore('.').toLongOrNull() ?: continue
            when {
                f.name.endsWith(".jsonl") -> segments.getOrPut(seq) { segmentOf(seq) }
                // 区切りが無い `.acked` は、区切りを消す途中で落ちた跡。消してよい
                f.name.endsWith(".acked") && !File(dir, name(seq, "jsonl")).exists() -> f.delete()
            }
        }
        // **取り除いた分のバイトは先に数える**（2 GB の勘定。design D3）。`.acked` は先頭の数本にしか無い
        for (s in segments.values) if (s.ackFile.exists()) loadAcked(s)
        listed = true
        return true
    }

    private fun name(seq: Long, ext: String) = "%012d.%s".format(seq, ext)

    private fun segmentOf(seq: Long) = Segment(seq, File(dir, name(seq, "jsonl")), File(dir, name(seq, "acked")))

    /** 1 件を追記する。**全件を書き直さない。** 書けたか。 */
    fun append(item: T, enqAgeMs: Long): Boolean = appendAll(listOf(item), enqAgeMs)

    /**
     * まとめて追記する（取り込みのため。1 件ずつ開き直さない）。**全部書けたときだけ true。**
     * 途中で書けなくなったら、そこまでに書けた分は置き場に残る（呼び出し側は元を消さない）。
     */
    fun appendAll(items: List<T>, enqAgeMs: Long): Boolean {
        if (!list()) return false
        if (items.isEmpty()) return true
        var i = 0
        while (i < items.size) {
            val line = try {
                lineOf(items[i], enqAgeMs).toByteArray(Charsets.UTF_8)
            } catch (e: SerializationException) {
                log(Telemetry.line("outbox_append_failed", error = e.javaClass.simpleName))
                return false
            }
            val tail = segments.lastEntry()?.value
            val target = if (tail != null && tail.file.length() + line.size <= segmentBytes) {
                tail
            } else {
                // 次の連番を開く。**最新の区切りが上限を超える書き込みはしない**（1 本 1 MB まで）
                segmentOf((tail?.seq ?: -1) + 1).also { segments[it.seq] = it }
            }
            // 同じ区切りに入る分をまとめる
            val buf = ByteArrayOutputStream()
            buf.write(line)
            var room = segmentBytes - target.file.length() - line.size
            i++
            while (i < items.size) {
                val next = try {
                    lineOf(items[i], enqAgeMs).toByteArray(Charsets.UTF_8)
                } catch (e: SerializationException) {
                    break
                }
                if (next.size > room) break
                buf.write(next)
                room -= next.size
                i++
            }
            if (!write(target, buf.toByteArray())) return false
        }
        return true
    }

    private fun write(target: Segment, bytes: ByteArray): Boolean = try {
        // **書きかけの末尾行に続けて書かない。** 電源断や途中で失敗した書き込みで改行の無い行が残っていると、
        // 次の 1 件がそこに繋がって 2 行とも読めなくなる
        if (target.seq !in terminated) {
            if (target.file.length() > 0 && lastByte(target.file) != '\n'.code) {
                target.file.appendBytes(byteArrayOf('\n'.code.toByte()))
            }
            terminated += target.seq
        }
        FileOutputStream(target.file, true).use { it.write(bytes) }
        target.lines = null
        true
    } catch (e: IOException) {
        // **途中まで書けているかもしれない**（review R8 / R13）。次に書く前に末尾をもう一度見る
        terminated -= target.seq
        target.lines = null
        // 開いたばかりで 1 行も書けなかった区切りは覚えない（空のファイルを先頭に残さない）
        if (target.file.length() == 0L) {
            segments.remove(target.seq)
            target.file.delete()
        }
        log(Telemetry.line("outbox_append_failed", error = e.javaClass.simpleName))
        false
    }

    /**
     * 先頭から、取り除かれていない行を最大 `limit` 件。`skip` の識別子は飛ばす（数に入れない）。
     * **読み戻しでメモリに載るのは返す分だけ**（spec「読み戻しでメモリに載る件数は 1 回に載る件数を超えない」）。
     */
    fun head(limit: Int, skip: Set<String> = emptySet()): List<Stored<T>> {
        val out = ArrayList<Stored<T>>(minOf(limit, 256))
        if (limit <= 0) return out
        scan { entry ->
            if (entry.item.id !in skip) out += entry
            out.size < limit
        }
        return out
    }

    /** 取り除かれていない行が `n` 件より多いか。**行を溜めずに数える。** */
    fun hasMoreThan(n: Int): Boolean {
        var seen = 0
        scan { seen++; seen <= n }
        return seen > n
    }

    /** 取り除かれていない行の数。**全区切りを読む**ので送信の経路では使わない（試験と診断のため）。 */
    fun count(): Int {
        var n = 0
        scan { n++; true }
        return n
    }

    /** 全件（試験と診断のため。**送信の経路では使わない**）。 */
    fun readAll(): List<T> {
        val out = ArrayList<T>()
        scan { out += it.item; true }
        return out
    }

    /**
     * 識別子で取り除く。**区切りの `.acked` に追記するだけで、残りの行を書き直さない**
     * （spec「送れた分を取り除いても残りは書き直されない」）。
     * 印を書けなかった・区切りを読めなかったら false（**取り除けたと数えない**。review R5）。
     */
    fun remove(ids: Collection<String>): Boolean {
        val want = ids.toHashSet()
        if (want.isEmpty()) return true
        if (!list()) return false
        var ok = true
        for (s in segments.values.toList()) {
            if (want.isEmpty()) break
            val found = ArrayList<Pair<Int, Int>>()
            val hit = HashSet<String>()
            // 区切りの中は終わりまで読む —— 同じ識別子の行が 2 本あっても両方を取り除く
            val r = forEachLine(s) { lineNo, bytes, entry ->
                if (entry != null && entry.item.id in want) {
                    found += lineNo to bytes
                    hit += entry.item.id
                }
                true
            }
            if (r == Read.FAILED) ok = false
            if (found.isNotEmpty() && !ack(s, found)) ok = false else want.removeAll(hit)
        }
        return ok
    }

    /**
     * 先頭から、`shouldDrop` が真の行を続けて取り除く（保持の上限。design D2 / D3）。
     * **最初に偽になった行で止まる** —— 積んだ順に並んでいるので、その先はもっと新しい。
     *
     * **証拠を書いてから消す**（review R26）: 区切りごとに、取り除く行を `commit` に渡し、真が返ったときだけ印を付ける。
     * 印を書けなかったら `rollback` を呼ぶ。**区切りを読めなかったら、そこで止める**（新しい記録から捨てない。review R2）。
     */
    fun dropHead(
        shouldDrop: (Stored<T>) -> Boolean,
        commit: (List<Stored<T>>) -> Boolean,
        rollback: (List<Stored<T>>) -> Unit,
    ): Int {
        if (!list()) return 0
        var dropped = 0
        for (s in segments.values.toList()) {
            val found = ArrayList<Pair<Int, Int>>()
            val gone = ArrayList<Stored<T>>()
            var stop = false
            val r = forEachLine(s) { lineNo, bytes, entry ->
                if (entry == null) return@forEachLine true
                if (!shouldDrop(entry)) {
                    stop = true
                    return@forEachLine false
                }
                found += lineNo to bytes
                gone += entry
                true
            }
            if (r == Read.FAILED) {
                log(Telemetry.line("retention_paused", error = "segment_unreadable"))
                return dropped
            }
            if (gone.isNotEmpty()) {
                if (!commit(gone)) return dropped
                if (!ack(s, found)) {
                    rollback(gone)
                    return dropped
                }
                dropped += gone.size
            }
            if (stop) break
        }
        return dropped
    }

    /**
     * 取り除かれていない行のバイト数が `maxBytes` 以下になるまで、先頭から行単位で取り除く（2 GB の上限。design D3）。
     * 証拠の書き方と、読めない区切りで止まるのは `dropHead` と同じ。
     */
    fun dropUntilBytes(
        maxBytes: Long,
        commit: (List<Stored<T>>) -> Boolean,
        rollback: (List<Stored<T>>) -> Unit,
    ): Int {
        if (!list()) return 0
        var dropped = 0
        for (s in segments.values.toList()) {
            if (liveBytes() <= maxBytes) break
            val found = ArrayList<Pair<Int, Int>>()
            val gone = ArrayList<Stored<T>>()
            // 区切りの途中で上限を下回ることがあるので、まだ `.acked` に書いていない分も引いて 1 行ごとに勘定する
            var pending = 0L
            val r = forEachLine(s) { lineNo, bytes, entry ->
                if (entry == null) return@forEachLine true
                if (liveBytes() - pending <= maxBytes) return@forEachLine false
                found += lineNo to bytes
                pending += bytes
                gone += entry
                true
            }
            if (r == Read.FAILED) {
                log(Telemetry.line("retention_paused", error = "segment_unreadable"))
                return dropped
            }
            if (gone.isEmpty()) continue
            if (!commit(gone)) return dropped
            if (!ack(s, found)) {
                rollback(gone)
                return dropped
            }
            dropped += gone.size
        }
        return dropped
    }

    /** 取り除かれていない行のバイト数（2 GB の勘定。`.acked` は数えない。design D3）。 */
    fun liveBytes(): Long = segments.values.sumOf { it.file.length() - it.ackedBytes }

    /** 区切りの数（試験のため）。 */
    fun segmentCount(): Int = segments.size

    // ------------------------------------------------------------------ 内側

    /** 取り除かれていない行を先頭から渡す。`visit` が偽を返したら止める。読めない区切りは飛ばす（送信の順が崩れるだけ）。 */
    private fun scan(visit: (Stored<T>) -> Boolean) {
        if (!list()) return
        for (s in segments.values.toList()) {
            val r = forEachLine(s) { _, _, entry ->
                if (entry == null) true else visit(entry)
            }
            if (r == Read.STOPPED) return
        }
    }

    /**
     * 区切りの行を 1 行ずつ**バイトのまま**読む（取り除いた行は飛ばす）。**1 行ずつ復号して渡し、溜めない。**
     * 読めない行はここで退避して印を付け、`entry = null` で渡す。
     */
    private fun forEachLine(s: Segment, visit: (lineNo: Int, bytes: Int, entry: Stored<T>?) -> Boolean): Read {
        if (!s.file.exists()) return Read.DONE
        val acked = s.acked ?: (if (loadAcked(s)) s.acked!! else return Read.FAILED)
        val broken = ArrayList<Pair<Int, Int>>()
        var lineNo = 0
        var nonBlank = 0
        var result = Read.DONE
        try {
            s.file.inputStream().buffered().use { input ->
                val buf = ByteArrayOutputStream()
                while (true) {
                    val c = input.read()
                    if (c != -1 && c != '\n'.code) {
                        buf.write(c)
                        continue
                    }
                    if (c == -1 && buf.size() == 0) break
                    val raw = buf.toByteArray()
                    buf.reset()
                    val n = lineNo++
                    val bytes = raw.size + if (c == -1) 0 else 1
                    if (raw.all { it == ' '.code.toByte() || it == '\t'.code.toByte() || it == '\r'.code.toByte() }) {
                        if (c == -1) break else continue
                    }
                    nonBlank++
                    if (n in acked) {
                        if (c == -1) break else continue
                    }
                    val entry = decode(String(raw, Charsets.UTF_8))
                    if (entry == null) {
                        // **捨てずに退避する**（C9）。印を付けてから数える
                        if (moveAside(raw)) broken += n to bytes
                    } else if (!visit(n, bytes, entry)) {
                        result = Read.STOPPED
                        break
                    }
                    if (c == -1) break
                }
            }
        } catch (e: IOException) {
            log(Telemetry.line("outbox_read_failed", error = e.javaClass.simpleName))
            result = Read.FAILED
        }
        if (result == Read.DONE) s.lines = nonBlank
        if (broken.isNotEmpty() && ack(s, broken)) {
            log(Telemetry.line("outbox_line_broken", count = broken.size))
            onUnreadable(broken.size)
        }
        return result
    }

    private fun decode(raw: String): Stored<T>? = try {
        val obj = ingestJson.parseToJsonElement(raw).jsonObject
        val enq = obj["enq"]?.jsonPrimitive?.longOrNull
        val item = obj["item"]
        if (enq == null || item !is JsonObject) null else Stored(ingestJson.decodeFromJsonElement(serializer, item), enq)
    } catch (e: SerializationException) {
        null
    } catch (e: IllegalArgumentException) {
        null
    }

    private fun lastByte(f: File): Int =
        RandomAccessFile(f, "r").use { raf ->
            raf.seek(raf.length() - 1)
            raf.read()
        }

    private fun lineOf(item: T, enqAgeMs: Long): String =
        "{\"enq\":$enqAgeMs,\"item\":${ingestJson.encodeToString(serializer, item)}}\n"

    /** `.acked` を読む。**読めなければ false**（空として進むと、取り除いた行が戻ってきて二重に数える。review R35）。 */
    private fun loadAcked(s: Segment): Boolean {
        val set = HashSet<Int>()
        var bytes = 0L
        if (s.ackFile.exists()) {
            try {
                s.ackFile.forEachLine(Charsets.UTF_8) { line ->
                    // 書きかけの行は読み飛ばす（その行は取り除かれていない扱い＝再送。受け口は冪等なので増えない）
                    val tab = line.indexOf('\t')
                    val n = if (line.startsWith("L") && tab > 1) line.substring(1, tab).toIntOrNull() else null
                    val b = if (tab > 0) line.substring(tab + 1).toIntOrNull() else null
                    if (n != null && b != null && set.add(n)) bytes += b
                }
            } catch (e: IOException) {
                log(Telemetry.line("outbox_acked_unreadable", error = e.javaClass.simpleName))
                return false
            }
        }
        s.acked = set
        s.ackedBytes = bytes
        return true
    }

    /** 取り除いた印を追記する。区切りの全行が済んだら区切りごと消す。 */
    private fun ack(s: Segment, keys: List<Pair<Int, Int>>): Boolean {
        val acked = s.acked ?: (if (loadAcked(s)) s.acked!! else return false)
        try {
            // `.acked` の書きかけの行にも次の印を繋げない（繋がると両方の印が消える。review R35）
            if (s.seq !in ackTerminated) {
                if (s.ackFile.exists() && s.ackFile.length() > 0 && lastByte(s.ackFile) != '\n'.code) {
                    s.ackFile.appendBytes(byteArrayOf('\n'.code.toByte()))
                }
                ackTerminated += s.seq
            }
            s.ackFile.appendText(keys.joinToString("") { "L${it.first}\t${it.second}\n" }, Charsets.UTF_8)
        } catch (e: IOException) {
            ackTerminated -= s.seq
            log(Telemetry.line("outbox_shrink_failed", count = keys.size, error = e.javaClass.simpleName))
            return false
        }
        for ((n, b) in keys) if (acked.add(n)) s.ackedBytes += b
        val lines = s.lines
        if (lines != null && acked.size >= lines) deleteSegment(s)
        return true
    }

    /** 区切りを消す。**区切りを消せたときだけ `.acked` を消す**（逆だと次の起動で全行が戻る。review R35）。 */
    private fun deleteSegment(s: Segment) {
        if (s.file.delete() || !s.file.exists()) {
            s.ackFile.delete()
            segments.remove(s.seq)
            terminated -= s.seq
            ackTerminated -= s.seq
        } else {
            log(Telemetry.line("outbox_segment_delete_failed"))
        }
    }

    /** 読めない行を退避先へ**バイトのまま**追記する（spec「読めない行は捨てずに退避される」）。 */
    private fun moveAside(raw: ByteArray): Boolean = try {
        unreadable.parentFile?.mkdirs()
        FileOutputStream(unreadable, true).use {
            it.write(raw)
            it.write('\n'.code)
        }
        true
    } catch (e: IOException) {
        // 退避できなければ印を付けない（区切りに残したまま。次に読んだときにもう一度当たる）
        log(Telemetry.line("outbox_salvage_failed", error = e.javaClass.simpleName))
        false
    }

    companion object {
        /** 1 本の区切りの上限（位置で約 1,400 件 ≒ 1 日。design D1）。 */
        const val SEGMENT_BYTES: Long = 1_000_000
    }
}
