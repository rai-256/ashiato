// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import java.security.MessageDigest
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 区切りファイルの置き場（ST04 / tasks 5.1 / 深掘り C7 / R13 / design D1）。
 *
 * ST01 の置き場は起動時に全件を 1 つの文字列として読み、送るたびに全件を書き直していた。
 * 90 日ぶん（位置 129,600 件・約 94 MB）で**起動で落ちて立て直しを繰り返し、収集そのものが止まりうる**。
 */
class SegmentStoreTest {
    private val dir: File = Files.createTempDirectory("seg").toFile()
    private val lines = mutableListOf<String>()

    private fun req(i: Int) =
        LocationFix(35.681236, 139.767125, 10f, Instant.parse("2026-06-01T00:00:00Z").plusSeconds(60L * i))
            .toIngestRequest("id-%06d".format(i), "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    private fun store(segmentBytes: Long = SegmentStore.SEGMENT_BYTES) =
        SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, "unreadable.jsonl"), { lines += it }, segmentBytes)

    private fun segmentFiles() = File(dir, "records").listFiles { f -> f.name.endsWith(".jsonl") }.orEmpty().sortedBy { it.name }

    private fun sha(f: File) = MessageDigest.getInstance("SHA-256").digest(f.readBytes()).joinToString("") { "%02x".format(it) }

    @Test
    fun `1,000,001 バイトで区切りが 2 本になる`() {
        val s = store()
        val one = "{\"enq\":0,\"item\":${ingestJson.encodeToString(IngestRequest.serializer(), req(0))}}\n"
            .toByteArray(Charsets.UTF_8).size
        // 1 本目にちょうど 1,000,000 バイトまでは入る。**1 行ぶん足して超えたら 2 本目**
        var written = 0L
        var i = 0
        while (written + one <= 1_000_000) {
            assertTrue(s.append(req(i++), 0))
            written += one
        }
        assertEquals("まだ 1 本に収まる", 1, segmentFiles().size)
        assertTrue(s.append(req(i++), 0))
        assertEquals("1 本の上限を超えて書いている", 2, segmentFiles().size)
        assertTrue("1 本目が 1 MB を超えた", segmentFiles().first().length() <= 1_000_000)
        assertEquals(i, store().count())
    }

    // Scenario: 読み戻しでメモリに載る件数は 1 回に載る件数を超えない
    @Test
    fun `読み戻しで組み立てるのは 1 回に載る件数まで`() {
        // 129,600 件ぶんを置く（位置の 90 日）。**組み立てた記録の数を数える**
        val s = store()
        repeat(129_600) { assertTrue(s.append(req(it), it.toLong())) }
        var built = 0
        val counting = object : kotlinx.serialization.KSerializer<IngestRequest> by IngestRequest.serializer() {
            override fun deserialize(decoder: kotlinx.serialization.encoding.Decoder): IngestRequest {
                built++
                return IngestRequest.serializer().deserialize(decoder)
            }
        }
        val reopened = SegmentStore(File(dir, "records"), counting, File(dir, "unreadable.jsonl"), {})
        assertEquals("起動で記録を組み立てている", 0, built)

        val batch = Outbox(reopened, age = { 0L }).head(MAX_BATCH)
        assertEquals(MAX_BATCH, batch.size)
        assertTrue("組み立てを数えられていない（試験が空振りしている）", built > 0)
        assertTrue("1 回に載る件数（$MAX_BATCH）を超えて組み立てた: $built", built <= MAX_BATCH)
        assertEquals("id-000000", batch.first().id)
    }

    // Scenario: 送れた分を取り除いても残りは書き直されない
    @Test
    fun `送れた分を取り除いても残りの区切りのバイトは変わらない`() {
        val s = store()
        repeat(129_600) { s.append(req(it), 0) }
        val files = segmentFiles()
        assertTrue("区切りが 1 本しかない（試験にならない）", files.size > 2)
        val before = files.associate { it.name to sha(it) }

        val outbox = Outbox(s, age = { 0L })
        val batch = outbox.head(MAX_BATCH)
        assertTrue(outbox.remove(batch.map { it.id }))

        for (f in files) {
            assertTrue("${f.name} が消えた", f.exists())
            assertEquals("${f.name} が書き直された", before[f.name], sha(f))
        }
        // 取り除いた印は `.acked` への追記だけ
        assertTrue(File(dir, "records").listFiles { f -> f.name.endsWith(".acked") }.orEmpty().isNotEmpty())
        assertEquals("id-%06d".format(MAX_BATCH), store().head(1).single().item.id)
    }

    @Test
    fun `区切りの全件が済んだら区切りと印を消す`() {
        val s = store(segmentBytes = 3_000)
        repeat(10) { s.append(req(it), 0) }
        val first = segmentFiles().first()
        val inFirst = first.readLines().count { it.isNotBlank() }
        assertTrue(inFirst in 1..9)

        assertTrue(s.remove((0 until inFirst).map { "id-%06d".format(it) }))
        assertFalse("済んだ区切りが残っている", first.exists())
        assertFalse("済んだ区切りの印が残っている", File(first.path.replace(".jsonl", ".acked")).exists())
        assertEquals(10 - inFirst, store(segmentBytes = 3_000).count())
    }

    @Test
    fun `飛ばす識別子は数に入れずに次を読む`() {
        val s = store()
        repeat(5) { s.append(req(it), 0) }
        assertEquals(
            listOf("id-000002", "id-000003"),
            s.head(2, skip = setOf("id-000000", "id-000001")).map { it.item.id },
        )
        assertTrue(s.hasMoreThan(4))
        assertFalse(s.hasMoreThan(5))
    }

    @Test
    fun `積んだときの経過が行と一緒に残る`() {
        val s = store()
        s.append(req(0), 123L)
        s.append(req(1), 456L)
        assertEquals(listOf(123L, 456L), store().head(10).map { it.enqAgeMs })
    }
}
