// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.io.IOException
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.contentOrNull
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
 * - `dir/<連番>.acked` —— その区切りから取り除いた行の追記（`<識別子>\t<バイト数>`）。**残りを書き直さない**
 * - 区切りの全行が取り除かれたら、区切りと `.acked` を消す
 * - **読むのは先頭（最も古い区切り）から必要な分だけ**。全件をメモリに載せない
 * - 読めない行は `unreadable` へ**追記して退避**し、`.acked` に印を付ける（C9 / design D6。捨てない）
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
    /** 読めない行を退避したときに呼ぶ（件数）。破棄の報告に数える（design D6） */
    private val onUnreadable: (Int) -> Unit = {},
) {
    /** 区切り 1 本の、メモリに持つ分だけの状態。**行そのものは持たない。** */
    private class Segment(val seq: Long, val file: File, val ackFile: File) {
        /** 取り除いた行の鍵（識別子、または読めなかった行の `#L<行番号>`）。読んだときにだけ埋める */
        var acked: MutableSet<String>? = null

        /** 取り除いた行のバイト数の合計 */
        var ackedBytes: Long = 0
    }

    private val segments = java.util.TreeMap<Long, Segment>()

    /** 末尾が改行で終わっていることを確かめた区切り（起動ごとに 1 度だけ見る）。 */
    private val terminated = HashSet<Long>()

    init {
        dir.mkdirs()
        val names = dir.listFiles().orEmpty()
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
    }

    private fun name(seq: Long, ext: String) = "%012d.%s".format(seq, ext)

    private fun segmentOf(seq: Long) = Segment(seq, File(dir, name(seq, "jsonl")), File(dir, name(seq, "acked")))

    /** 1 件を追記する。**全件を書き直さない。** 書けたか。 */
    fun append(item: T, enqAgeMs: Long): Boolean {
        val line = try {
            lineOf(item, enqAgeMs)
        } catch (e: SerializationException) {
            log(Telemetry.line("outbox_append_failed", error = e.javaClass.simpleName))
            return false
        }
        val bytes = line.toByteArray(Charsets.UTF_8)
        val tail = segments.lastEntry()?.value
        val target = if (tail != null && tail.file.length() + bytes.size <= segmentBytes) {
            tail
        } else {
            // 次の連番を開く。**最新の区切りが上限を超える書き込みはしない**（1 本 1 MB まで）
            segmentOf((tail?.seq ?: -1) + 1).also { segments[it.seq] = it }
        }
        return try {
            // **書きかけの末尾行に続けて書かない。** 電源断で改行の無い行が残っていると、
            // 次の 1 件がそこに繋がって 2 行とも読めなくなる
            if (target.seq !in terminated) {
                if (target.file.length() > 0 && lastByte(target.file) != '\n'.code) {
                    target.file.appendBytes(byteArrayOf('\n'.code.toByte()))
                }
                terminated += target.seq
            }
            target.file.appendBytes(bytes)
            true
        } catch (e: IOException) {
            // 開いたばかりで 1 行も書けなかった区切りは覚えない（空のファイルを先頭に残さない）
            if (target.file.length() == 0L) {
                segments.remove(target.seq)
                target.file.delete()
            }
            log(Telemetry.line("outbox_append_failed", error = e.javaClass.simpleName))
            false
        }
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
     */
    fun remove(ids: Collection<String>): Boolean {
        val want = ids.toHashSet()
        if (want.isEmpty()) return true
        var ok = true
        for (s in segments.values.toList()) {
            if (want.isEmpty()) break
            val found = ArrayList<Pair<String, Int>>()
            forEachLine(s) { key, bytes, _ ->
                if (key in want) {
                    found += key to bytes
                    want.remove(key)
                }
                want.isNotEmpty()
            }
            if (found.isNotEmpty() && !ack(s, found)) ok = false
        }
        return ok
    }

    /**
     * 先頭から、`shouldDrop` が真の行を続けて取り除く（保持の上限。design D2 / D3）。
     * **最初に偽になった行で止まる** —— 積んだ順に並んでいるので、その先はもっと新しい。
     * 取り除いた行ごとに `onDrop` を呼ぶ。行は溜めない。
     */
    fun dropHead(shouldDrop: (Stored<T>) -> Boolean, onDrop: (Stored<T>) -> Unit): Int {
        var dropped = 0
        for (s in segments.values.toList()) {
            val found = ArrayList<Pair<String, Int>>()
            val gone = ArrayList<Stored<T>>()
            var stop = false
            forEachLine(s) { key, bytes, entry ->
                if (entry == null) return@forEachLine true
                if (!shouldDrop(entry)) {
                    stop = true
                    return@forEachLine false
                }
                found += key to bytes
                gone += entry
                true
            }
            if (found.isNotEmpty() && !ack(s, found)) {
                // 印を書けなかった。**取り除いたことにしない**（報告にも数えない。次の見回りでもう一度当たる）
                return dropped
            }
            gone.forEach(onDrop)
            dropped += gone.size
            if (stop) break
        }
        return dropped
    }

    /**
     * 取り除かれていない行のバイト数が `maxBytes` 以下になるまで、先頭から行単位で取り除く（2 GB の上限。design D3）。
     * 取り除いた行ごとに `onDrop` を呼ぶ。行は溜めない。
     */
    fun dropUntilBytes(maxBytes: Long, onDrop: (Stored<T>) -> Unit): Int {
        var dropped = 0
        for (s in segments.values.toList()) {
            if (liveBytes() <= maxBytes) break
            val found = ArrayList<Pair<String, Int>>()
            val gone = ArrayList<Stored<T>>()
            // 区切りの途中で上限を下回ることがあるので、まだ `.acked` に書いていない分も引いて 1 行ごとに勘定する
            var pending = 0L
            forEachLine(s) { key, bytes, entry ->
                if (entry == null) return@forEachLine true
                if (liveBytes() - pending <= maxBytes) return@forEachLine false
                found += key to bytes
                pending += bytes
                gone += entry
                true
            }
            if (found.isNotEmpty() && !ack(s, found)) return dropped
            gone.forEach(onDrop)
            dropped += gone.size
        }
        return dropped
    }

    /** 取り除かれていない行のバイト数（2 GB の勘定。`.acked` は数えない。design D3）。 */
    fun liveBytes(): Long = segments.values.sumOf { it.file.length() - it.ackedBytes }

    /** 区切りの数（試験のため）。 */
    fun segmentCount(): Int = segments.size

    // ------------------------------------------------------------------ 内側

    /** 取り除かれていない行を先頭から渡す。`visit` が偽を返したら止める。 */
    private fun scan(visit: (Stored<T>) -> Boolean) {
        for (s in segments.values.toList()) {
            var stop = false
            forEachLine(s) { _, _, entry ->
                if (entry == null) return@forEachLine true
                if (!visit(entry)) {
                    stop = true
                    false
                } else {
                    true
                }
            }
            if (stop) return
        }
    }

    /**
     * 区切りの行を 1 行ずつ読む（取り除いた行は飛ばす）。**1 行ずつ復号して渡し、溜めない。**
     * 読めない行はここで退避して印を付け、`entry = null` で渡す。
     */
    private fun forEachLine(s: Segment, visit: (key: String, bytes: Int, entry: Stored<T>?) -> Boolean) {
        if (!s.file.exists()) return
        val acked = s.acked ?: loadAcked(s)
        val broken = ArrayList<Pair<String, Int>>()
        var lineNo = 0
        try {
            s.file.bufferedReader(Charsets.UTF_8).use { reader ->
                while (true) {
                    val raw = reader.readLine() ?: break
                    val n = lineNo++
                    val bytes = raw.toByteArray(Charsets.UTF_8).size + 1
                    if (raw.isBlank()) continue
                    val entry = decode(raw)
                    val key = entry?.item?.id ?: "#L$n"
                    if (key in acked) continue
                    if (entry == null) {
                        // **捨てずに退避する**（C9）。印を付けてから数える
                        if (moveAside(raw)) broken += key to bytes
                        continue
                    }
                    if (!visit(key, bytes, entry)) break
                }
            }
        } catch (e: IOException) {
            log(Telemetry.line("outbox_read_failed", error = e.javaClass.simpleName))
        }
        if (broken.isNotEmpty() && ack(s, broken)) {
            log(Telemetry.line("outbox_line_broken", count = broken.size))
            onUnreadable(broken.size)
        }
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
        java.io.RandomAccessFile(f, "r").use { raf ->
            raf.seek(raf.length() - 1)
            raf.read()
        }

    private fun lineOf(item: T, enqAgeMs: Long): String =
        "{\"enq\":$enqAgeMs,\"item\":${ingestJson.encodeToString(serializer, item)}}\n"

    private fun loadAcked(s: Segment): MutableSet<String> {
        val set = HashSet<String>()
        var bytes = 0L
        if (s.ackFile.exists()) {
            try {
                s.ackFile.forEachLine(Charsets.UTF_8) { line ->
                    // 書きかけの最後の 1 行は読み飛ばす（その行は取り除かれていない扱い＝再送。冪等なので増えない）
                    val tab = line.indexOf('\t')
                    val n = if (tab > 0) line.substring(tab + 1).toIntOrNull() else null
                    if (tab > 0 && n != null) {
                        if (set.add(line.substring(0, tab))) bytes += n
                    }
                }
            } catch (e: IOException) {
                log(Telemetry.line("outbox_acked_unreadable", error = e.javaClass.simpleName))
            }
        }
        s.acked = set
        s.ackedBytes = bytes
        return set
    }

    /** 取り除いた印を追記する。区切りの全行が済んだら区切りごと消す。 */
    private fun ack(s: Segment, keys: List<Pair<String, Int>>): Boolean {
        val acked = s.acked ?: loadAcked(s)
        try {
            s.ackFile.appendText(keys.joinToString("") { "${it.first}\t${it.second}\n" }, Charsets.UTF_8)
        } catch (e: IOException) {
            log(Telemetry.line("outbox_shrink_failed", count = keys.size, error = e.javaClass.simpleName))
            return false
        }
        for ((k, b) in keys) if (acked.add(k)) s.ackedBytes += b
        if (s.ackedBytes >= s.file.length() && allAcked(s)) {
            segments.remove(s.seq)
            s.file.delete()
            s.ackFile.delete()
        }
        return true
    }

    /** 区切りの全行が取り除かれたか（バイト数の一致だけで決めず、行を読んで確かめる）。 */
    private fun allAcked(s: Segment): Boolean {
        val acked = s.acked ?: return false
        var lineNo = 0
        return try {
            s.file.bufferedReader(Charsets.UTF_8).use { reader ->
                while (true) {
                    val raw = reader.readLine() ?: break
                    val n = lineNo++
                    if (raw.isBlank()) continue
                    val key = idOf(raw) ?: "#L$n"
                    if (key !in acked) return false
                }
                true
            }
        } catch (e: IOException) {
            false
        }
    }

    /** 行から識別子だけを取る（全体を復号しない）。読めなければ null。 */
    private fun idOf(raw: String): String? = try {
        ingestJson.parseToJsonElement(raw).jsonObject["item"]?.jsonObject?.get("id")?.jsonPrimitive?.contentOrNull
    } catch (e: SerializationException) {
        null
    } catch (e: IllegalArgumentException) {
        null
    }

    /** 読めない行を退避先へ**そのまま**追記する（1 バイトも変えない。spec「読めない行は捨てずに退避される」）。 */
    private fun moveAside(raw: String): Boolean = try {
        unreadable.parentFile?.mkdirs()
        unreadable.appendText(raw + "\n", Charsets.UTF_8)
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
