// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 置き場の読めない行は捨てずに退避し、件数を破棄の報告に載せる（ST04 / tasks 5.3 / 深掘り C9 / design D6）。
 *
 * ST01 の置き場は壊れた行を読み飛ばし、**次の書き直しでファイルから消していた**。
 * 同じ「端末で失われた」が経路によって格子に出たり出なかったりすると、扉 #14 の区別がその経路だけ欠ける（R8）。
 */
class UnreadableLineTest {
    private val st = TestStores(segmentBytes = 4_000)

    private fun req(i: Int) =
        LocationFix(35.68, 139.76, 10f, Instant.parse("2026-06-01T00:00:00Z").plusSeconds(60L * i))
            .toIngestRequest("id-$i", "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    private val broken1 = """{"enq":1,"item":{"id":"x1","raw":"半端"""
    private val broken2 = "これは JSON ではない {\"lat\":1}"

    private fun plantTwoBrokenLines(): File {
        st.records.add(req(0))
        val seg = File(st.recordsDir, "000000000000.jsonl")
        seg.appendText("$broken1\n$broken2\n")
        st.records.add(req(1))
        return seg
    }

    // Scenario: 読めない行は捨てずに退避される
    @Test
    fun `読めない行は 1 バイトも変わらずに退避先に残る`() {
        plantTwoBrokenLines()
        val reopened = Outbox(
            SegmentStore(st.recordsDir, IngestRequest.serializer(), st.unreadable, st.log) { st.unreadableSeen += it },
            st.age::now,
        )
        assertEquals(listOf("id-0", "id-1"), reopened.head(10).map { it.id })
        assertEquals("$broken1\n$broken2\n", st.unreadable.readText())
        // もう一度読んでも二重に退避しない
        reopened.snapshot()
        assertEquals("$broken1\n$broken2\n", st.unreadable.readText())
    }

    // Scenario: 読めない行の件数が報告される
    @Test
    fun `読めない行の件数が範囲を持たない破棄の報告になる`() {
        plantTwoBrokenLines()
        var seen = 0
        Outbox(SegmentStore(st.recordsDir, IngestRequest.serializer(), st.unreadable, st.log) { seen += it }, st.age::now).head(10)
        assertEquals(2, seen)

        st.ledger.unreadable(LOGICAL_SOURCE, seen)
        st.ledger.freeze()
        val report = st.drops.snapshot().single()
        assertEquals("unreadable", report.reason)
        assertEquals(2, report.count)
        assertEquals(null, report.rangeStart)
        assertEquals(null, report.rangeEnd)
        assertTrue(report.hourly.isEmpty())
    }

    // Scenario: 退避した行は上限を超えても消されない
    @Test
    fun `退避した行は 2 GB を超えて捨てても消えない`() {
        plantTwoBrokenLines()
        st.records.head(10)
        val before = st.unreadable.readBytes()
        repeat(40) { st.records.add(req(10 + it)) }

        val retention = Retention(st.records, st.ledger, st.age::now, policy = { RetentionPolicy(maxBytes = 3_000) })
        assertTrue("捨てていない（試験になっていない）", retention.enforce() > 0)
        assertTrue(st.records.bytes() <= 3_000)
        assertTrue("退避先が消えた・変わった", before.contentEquals(st.unreadable.readBytes()))
    }

    @Test
    fun `ST01 が退けた unreadable ファイルは行数を数えて移し、消さない`() {
        val files = Files.createTempDirectory("legacy").toFile()
        val aside = File(files, "outbox.jsonl.unreadable.1757000000000").apply { writeText("壊れ1\n壊れ2\n\n壊れ3\n") }
        val got = migrateLegacyOutbox(
            File(files, "outbox.jsonl"), st.records, IngestRequest.serializer(), st.unreadable, File(st.dir, "salvaged"), st.log,
        )
        assertEquals(3, got.unreadable)
        val moved = File(st.dir, "salvaged/${aside.name}")
        assertTrue("消した", moved.exists())
        assertEquals("壊れ1\n壊れ2\n\n壊れ3\n", moved.readText())
        // 2 回目の起動では数えない
        val again = migrateLegacyOutbox(
            File(files, "outbox.jsonl"), st.records, IngestRequest.serializer(), st.unreadable, File(st.dir, "salvaged"), st.log,
        )
        assertEquals(0, again.unreadable)
    }
}
