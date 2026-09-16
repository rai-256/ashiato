// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 置き場に書けなかった記録を数える（ST04 / tasks 7.3 / 深掘り C5 / R7 / design D5）。
 *
 * ST01 の `Outbox.add` は書けなかった記録をメモリにだけ積むので、端末の空きが尽きると**立て直しで痕跡なく消えた**。
 */
class WriteFailedTest {
    private val st = TestStores()
    private val ledgerFile = File(st.dir, "write-failed.bin")

    private fun req(id: String, time: String) =
        LocationFix(35.68, 139.76, 10f, Instant.parse(time)).toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 書き込みを失敗させる置き場に、固定長の数えをつないだ Outbox（本番の `LocationService` と同じ配線）。 */
    private fun failingOutbox(counter: WriteFailedLedger): Outbox<IngestRequest> {
        val blocked = File(st.dir, "blocked").apply { writeText("ディレクトリではない") }
        return Outbox(
            SegmentStore(File(blocked, "records"), IngestRequest.serializer(), st.unreadable, st.log),
            st.age::now,
            writeFailures = object : WriteFailures<IngestRequest> {
                override fun failed(item: IngestRequest) = counter.failed(Instant.parse(item.eventTime))
                override fun recovered(item: IngestRequest) = counter.recovered(Instant.parse(item.eventTime))
                override fun lost(item: IngestRequest) = counter.recovered(Instant.parse(item.eventTime))
            },
        )
    }

    /** 立て直し: 同じ置き場から数えを読み、0 でなければ報告にする（`LocationService.openStores` と同じ手順）。 */
    private fun restart(): Pair<List<WriteFailedLedger.Slot>, Int> {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        val (slots, overflow) = counter.peek()
        if (st.ledger.writeFailed(LOGICAL_SOURCE, slots, overflow)) counter.clear()
        // 送信の前に凍結される（`Drainer`）
        st.ledger.freeze()
        return slots to overflow
    }

    // Scenario: 置き場に書けなかった記録も報告される
    @Test
    fun `書けなかった記録が立て直しで失われると、次の起動で書けなかった理由の報告が積まれる`() {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        val outbox = failingOutbox(counter)
        assertFalse(outbox.add(req("a", "2026-06-01T10:10:00Z")))
        // ここでプロセスが立て直された（メモリの 1 件は消える）
        restart()

        val r = st.drops.snapshot().single()
        assertEquals("write_failed", r.reason)
        assertEquals(1, r.count)
        // 送信に載る（凍結済み）
        val t = Transport { Outcome.Responded(200, """[{"accepted":true}]""") }
        Sender(st.drops, t, DropReport.serializer()).flush()
        assertEquals(0, st.drops.size())
    }

    // Scenario: 書けなかった記録の報告は時間ごとの件数を持つ
    @Test
    fun `10 時台の 3 件が失われると、範囲と 10 時台 3 件を持つ`() {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        val outbox = failingOutbox(counter)
        listOf("10:05", "10:20", "10:40").forEachIndexed { i, hm -> outbox.add(req("r$i", "2026-06-01T$hm:00Z")) }
        restart()

        val r = st.drops.snapshot().single()
        assertEquals(3, r.count)
        assertEquals(listOf(DropHour("2026-06-01T10:00:00Z", 3)), r.hourly)
        assertEquals("2026-06-01T10:05:00Z", r.rangeStart)
        assertEquals("2026-06-01T10:40:00.001Z", r.rangeEnd)
    }

    // Scenario: 書けなかった記録が 1 件でも範囲の終わりは始まりより後
    @Test
    fun `1 件でも範囲の終わりは始まりより後`() {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        failingOutbox(counter).add(req("a", "2026-06-01T10:10:00Z"))
        restart()
        val r = st.drops.snapshot().single()
        assertTrue(Instant.parse(r.rangeEnd).isAfter(Instant.parse(r.rangeStart)))
    }

    @Test
    fun `数えのファイルは 4096 バイトのまま伸びない`() {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        assertEquals(4096L, ledgerFile.length())
        val outbox = failingOutbox(counter)
        // 枠（204）を超える時間ぶん書けなかった —— あふれは件数だけを数える
        repeat(300) { outbox.add(req("r$it", Instant.parse("2026-06-01T00:00:00Z").plusSeconds(3600L * it).toString())) }
        assertEquals("書き込みの後に大きさが変わった", 4096L, ledgerFile.length())
        assertEquals(4096L, counter.fileBytes())

        val (slots, overflow) = restart()
        assertEquals(WriteFailedLedger.SLOTS, slots.size)
        assertEquals(300 - WriteFailedLedger.SLOTS, overflow)
        assertEquals(4096L, ledgerFile.length())
        // あふれは範囲を持たない報告になる
        assertTrue(st.drops.snapshot().any { it.rangeStart == null && it.count == overflow })
        assertEquals(300, st.drops.snapshot().sumOf { it.count })
    }

    @Test
    fun `書き直せた記録は数えから引かれ、報告にならない`() {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        counter.failed(Instant.parse("2026-06-01T10:10:00Z"))
        counter.recovered(Instant.parse("2026-06-01T10:10:00Z"))
        restart()
        assertEquals(0, st.drops.size())
    }

    @Test
    fun `書けるようになったらメモリの分を先に書く（積んだ順を保つ）`() {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        val dir = File(st.dir, "flaky")
        val store = SegmentStore(dir, IngestRequest.serializer(), st.unreadable, st.log)
        val outbox = Outbox(
            store,
            st.age::now,
            writeFailures = object : WriteFailures<IngestRequest> {
                override fun failed(item: IngestRequest) = counter.failed(Instant.parse(item.eventTime))
                override fun recovered(item: IngestRequest) = counter.recovered(Instant.parse(item.eventTime))
                override fun lost(item: IngestRequest) = Unit
            },
        )
        dir.setWritable(false)
        assertFalse(outbox.add(req("first", "2026-06-01T10:00:00Z")))
        dir.setWritable(true)
        assertTrue(outbox.add(req("second", "2026-06-01T10:01:00Z")))
        assertEquals(listOf("first", "second"), SegmentStore(dir, IngestRequest.serializer(), st.unreadable, st.log).readAll().map { it.id })
        val (slots, overflow) = counter.take()
        assertTrue("書き直せたのに数えが残っている", slots.isEmpty() && overflow == 0)
    }

    /**
     * **報告を保存できなければ数えを 0 に戻さない**（review R15）。先に戻すと、空きが尽きたまま立て直したときに痕跡が消える。
     */
    @Test
    fun `報告の下書きを保存できなければ、固定長の数えは残る`() {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        counter.failed(Instant.parse("2026-06-01T10:10:00Z"))
        val blockedDir = File(st.dir, "no-space").apply { writeText("x") }
        val ledger = DropLedger(File(blockedDir, "drops-open.json"), st.drops, { "user-1" }, "device-1", { st.now }, { "w" }, st.log)

        val reborn = WriteFailedLedger(ledgerFile, st.log)
        val (slots, overflow) = reborn.peek()
        if (ledger.writeFailed(LOGICAL_SOURCE, slots, overflow)) reborn.clear()

        assertEquals("保存できないのに数えを 0 に戻した", 1, WriteFailedLedger(ledgerFile, st.log).peek().first.single().count)
        assertTrue("保存できない下書きがメモリに残っている（次の起動で二重に報告する）", ledger.drafts().isEmpty())
    }

    /**
     * メモリに持ちきれず手放した記録（`lost`）: **下書きを保存できたときだけ固定長の数えから引く**（review R1）。
     * 本番の `LocationService` の `lost` と同じ手順を、書けない下書きのファイルで確かめる。
     */
    @Test
    fun `手放した記録は下書きを保存できなければ数えに残る`() {
        val counter = WriteFailedLedger(ledgerFile, st.log)
        val t = Instant.parse("2026-06-01T10:10:00Z")
        counter.failed(t)
        val blockedDir = File(st.dir, "no-space-2").apply { writeText("x") }
        val ledger = DropLedger(File(blockedDir, "drops-open.json"), st.drops, { "user-1" }, "device-1", { st.now }, { "w" }, st.log)

        if (ledger.record(DropReason.WRITE_FAILED, listOf(LOGICAL_SOURCE to t)) != null) counter.recovered(t)

        assertEquals("保存できないのに数えから引いた（痕跡が消える）", 1, counter.peek().first.single().count)
        assertTrue(ledger.drafts().isEmpty())
        // 保存できる下書きなら引く
        if (st.ledger.record(DropReason.WRITE_FAILED, listOf(LOGICAL_SOURCE to t)) != null) counter.recovered(t)
        assertTrue(counter.peek().first.isEmpty())
        assertEquals(1, st.ledger.drafts().single().count)
    }

    @Test
    fun `枠は 204 個で、205 時間目は件数だけのあふれになる`() {
        assertEquals(204, WriteFailedLedger.SLOTS)
        val counter = WriteFailedLedger(ledgerFile, st.log)
        repeat(205) { counter.failed(Instant.parse("2026-06-01T00:00:00Z").plusSeconds(3600L * it)) }
        assertEquals(204, counter.peek().first.size)
        assertEquals(1, counter.peek().second)
    }
}
