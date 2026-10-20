// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * 原本の送り待ちの台帳（tasks 5.1。design D8）。保持の上限の側は `RetentionPolicy.overrideForTest` で 90 日と 2 GB を縮める。
 */
class OriginalLedgerTest {
    private val day = AgeClock.DAY_MS

    @After
    fun resetPolicy() = RetentionPolicy.clearOverrideForTest()

    private fun OriginalHarness.retention() =
        Retention(st.records, st.ledger, st.age::now, onDropped = link::dropped)

    // Scenario: 原本は未送信の置き場の大きさを増やさない
    @Test
    fun originals_do_not_grow_the_unsent_store() {
        val h = OriginalHarness()
        // 1 枚 3 MB の 100 枚を、試験の置き場を軽くするため 1 枚 300 KB で（比べるのは「原本の大きさに比例しない」こと）
        val body = "p".repeat(300_000)
        repeat(100) { h.photo(it + 1L, body + it) }
        h.scan()
        val originalsBytes = 100L * 300_000
        assertEquals(100, h.st.records.size())
        assertTrue("未送信が原本の大きさで増えた: ${h.st.records.bytes()}", h.st.records.bytes() < originalsBytes / 50)
        assertEquals(100, h.env.originals.all().count { it.state == ORIGINAL_AWAITING_RECORD })
        // 送り待ちの印は別のファイルにあり、未送信の置き場のバイトに数えられない
        assertTrue(File(File(h.env.dir, MEDIA_LEDGER_DIR), ORIGINALS_FILE).exists())
    }

    // Scenario: 原本の送り待ちは 90 日を過ぎても捨てない
    @Test
    fun a_waiting_mark_survives_91_days_and_the_original_is_sent_when_reachable() {
        RetentionPolicy.overrideForTest(RetentionPolicy(maxAgeMs = 90 * day, maxBytes = 1_000))
        val h = OriginalHarness()
        h.photo(1, "waiting a long time")
        h.scan()
        h.acceptRecords()
        // 到達できないまま 91 日。ほかの記録は上限で捨てられる
        h.env.listing.dateAddedSec = h.now().epochSecond
        h.photo(2, "recorded later")
        h.scan()
        h.st.clock.advance(91 * day)
        assertEquals(1, h.retention().enforce())
        assertEquals(1, h.env.originals.all().count { it.state == ORIGINAL_READY })
        h.transport.unreachable = true
        h.uploader.run()
        assertEquals(1, h.env.originals.all().count { it.state == ORIGINAL_READY })
        h.transport.unreachable = false
        h.advanceMs(2 * 60_000)
        assertEquals(1, h.uploader.run())
        assertEquals(h.shaOf(1), h.transport.stored.keys.single())
    }

    // Scenario: 記録が未送信から破棄されると原本も欠けになる
    @Test
    fun a_dropped_record_turns_its_original_into_a_missing_original() {
        RetentionPolicy.overrideForTest(RetentionPolicy(maxAgeMs = 90 * day))
        val h = OriginalHarness()
        h.photo(1, "its record will be dropped")
        h.scan()
        val sha = h.shaOf(1)
        h.st.clock.advance(91 * day)
        assertEquals(1, h.retention().enforce())
        val report = h.missingReports().single()
        assertEquals("record_dropped", report.reason)
        assertEquals(sha, report.sha256)
        assertNull(h.env.originals.get(sha))
    }

    /** 破棄の後に台帳を書けず行を消せなかったら、次の知らせで消し直す（review/code.md R25）。 */
    @Test
    fun a_row_that_could_not_be_removed_after_a_drop_is_removed_on_the_next_notice() {
        val h = OriginalHarness()
        h.photo(1, "dropped while the ledger is unwritable")
        h.scan()
        val sha = h.shaOf(1)
        val record = h.st.records.snapshot().single()
        // 全体の書き直しの一時ファイルの場所をフォルダで塞ぐ（台帳に書けない）
        val tmp = File(h.env.dir, "media/originals.jsonl.tmp").apply { mkdirs() }
        assertTrue(tmp.isDirectory)
        assertFalse(h.link.dropped(listOf(record)))
        assertEquals(ORIGINAL_AWAITING_RECORD, h.env.originals.get(sha)?.state)
        tmp.delete()
        assertTrue(h.link.accepted(emptyList()))
        assertNull("書けるようになっても行が残った", h.env.originals.get(sha))
        assertEquals("報告は 1 回", 1, h.missingReports().count { it.reason == "record_dropped" })
    }

    // Scenario: 記録が未送信から破棄されると原本も欠けになる
    @Test
    fun records_dropped_across_several_segments_all_turn_their_originals_into_missing_originals() {
        RetentionPolicy.overrideForTest(RetentionPolicy(maxAgeMs = 90 * day))
        val h = OriginalHarness(TestStores(segmentBytes = 300))
        repeat(4) { h.photo(it + 1L, "dropped across segments $it") }
        h.scan()
        val segments = h.st.dir.walkTopDown().count { it.isFile && it.name.endsWith(".jsonl") && it.length() > 0 }
        assertTrue("区切りが 2 本以上に分かれていない: $segments", segments >= 2)
        h.st.clock.advance(91 * day)
        assertEquals(4, h.retention().enforce())
        assertEquals(4, h.missingReports().count { it.reason == "record_dropped" })
        assertEquals(0, h.env.originals.all().size)
    }

    // Scenario: 記録が未送信から破棄されると原本も欠けになる
    @Test
    fun a_segment_the_drop_ledger_refused_keeps_its_record_and_its_original() {
        RetentionPolicy.overrideForTest(RetentionPolicy(maxAgeMs = 90 * day))
        val h = OriginalHarness(TestStores(segmentBytes = 300))
        repeat(4) { h.photo(it + 1L, "second segment is refused $it") }
        h.scan()
        h.st.clock.advance(91 * day)
        // 破棄の下書きの 2 回目の保存（= 2 本目の区切り）だけが書けない
        var saves = 0
        val dir = object : File(h.st.dir, "flaky") {
            override fun mkdirs(): Boolean {
                if (++saves == 2) throw java.io.IOException("disk full")
                return super.mkdirs()
            }
        }
        val openFile = object : File(dir, "drops-open.json") {
            override fun getParentFile(): File = dir
        }
        val ledger = DropLedger(
            openFile,
            h.st.drops,
            { "user-1" },
            "device-1",
            { h.st.now },
            { "r${saves}x" },
            h.st.log,
            LOGICAL_SOURCE,
        )
        val dropped = Retention(h.st.records, ledger, h.st.age::now, onDropped = h.link::dropped).enforce()
        assertTrue("1 本目の区切りは捨てられる: $dropped", dropped in 1..3)
        assertEquals(4 - dropped, h.st.records.size())
        assertEquals(dropped, h.missingReports().count { it.reason == "record_dropped" })
        assertEquals(4 - dropped, h.env.originals.all().size)
    }

    @Test
    fun an_accepted_record_stays_in_the_unsent_store_while_the_ledger_cannot_be_written() {
        val h = OriginalHarness()
        h.photo(1, "ledger write fails once")
        h.scan()
        val sha = h.shaOf(1)
        // 台帳の一時ファイルの場所をふさいで、書き込みを失敗させる
        val tmp = File(File(h.env.dir, MEDIA_LEDGER_DIR), "$ORIGINALS_FILE.tmp").also { it.mkdirs() }
        h.acceptRecords()
        assertEquals(1, h.st.records.size())
        assertEquals(ORIGINAL_AWAITING_RECORD, h.env.originals.get(sha)?.state)
        tmp.delete()
        h.acceptRecords()
        assertEquals(0, h.st.records.size())
        assertEquals(ORIGINAL_READY, h.env.originals.get(sha)?.state)
    }

    // Scenario: 恒久的に断られた記録の原本は欠けになる
    @Test
    fun a_permanently_rejected_record_turns_its_original_into_a_missing_original() {
        val h = OriginalHarness()
        h.photo(1, "its record is refused")
        h.scan()
        val sha = h.shaOf(1)
        h.recordSender(rejectEverything).flush()
        assertEquals(0, h.st.records.size())
        val report = h.missingReports().single()
        assertEquals("record_rejected", report.reason)
        assertEquals(sha, report.sha256)
        assertEquals(0, h.env.originals.all().size)
    }

    // Scenario: 原本の欠けの報告は上限を超えても捨てられない
    @Test
    fun a_missing_original_report_is_never_dropped_by_the_limits() {
        RetentionPolicy.overrideForTest(RetentionPolicy(maxAgeMs = 90 * day, maxBytes = 1_000))
        val h = OriginalHarness()
        h.photo(1, "vanishes")
        h.scan()
        h.env.listing.remove(PRIMARY, 1)
        h.acceptRecords()
        h.uploader.run()
        val report = h.missingReports().single()
        // 到達できないまま 91 日、未送信は上限を超える
        repeat(20) { h.photo(100L + it, "filler $it") }
        h.scan()
        h.st.clock.advance(91 * day)
        h.retention().enforce()
        // 上限で捨てた記録の原本の欠け（record_dropped）が増えても、最初の報告は残っている
        assertTrue(report.id in h.missingReports().map { it.id })
        // 到達できるようになると送られる
        val sent = mutableListOf<String>()
        val sender = Sender(
            h.missing,
            Transport { body ->
                sent += body
                val n = ingestJson.decodeFromString<List<MediaMissingReport>>(body).size
                Outcome.Responded(200, verdicts(n, """{"accepted":true}"""))
            },
            MediaMissingReport.serializer(),
        )
        assertTrue(sender.flush().removed >= 1)
        assertTrue(sent.single().contains("\"reason\":\"vanished\""))
        assertEquals(0, h.missing.size())
    }

    @Test
    fun an_accepted_record_makes_the_original_ready_even_when_it_is_a_duplicate() {
        val h = OriginalHarness()
        h.photo(1, "accepted twice")
        h.scan()
        assertEquals(ORIGINAL_AWAITING_RECORD, h.env.originals.all().single().state)
        h.acceptRecords()
        assertEquals(ORIGINAL_READY, h.env.originals.all().single().state)
    }

    @Test
    fun a_video_and_a_refused_record_of_a_resent_image_leave_the_ledger_alone() {
        val h = OriginalHarness()
        h.env.listing.putVideo(PRIMARY, 1, "DCIM/Camera/", "VID_1.mp4", "a video")
        h.scan()
        assertEquals(0, h.env.originals.all().size)
        h.recordSender(rejectEverything).flush()
        assertTrue(h.missingReports().isEmpty())
    }

    @Test
    fun the_ledger_is_read_back_after_a_restart_and_a_failed_write_changes_nothing() {
        val h = OriginalHarness()
        h.photo(1, "persisted")
        h.scan()
        h.acceptRecords()
        val file = File(File(h.env.dir, MEDIA_LEDGER_DIR), ORIGINALS_FILE)
        val reread = OriginalLedger(file)
        assertEquals(h.env.originals.all(), reread.all())
        assertEquals(ORIGINAL_READY, reread.all().single().state)
        // 書けない（置き場の場所がファイル）: 変更は戻り、メモリと台帳が食い違わない
        val blocked = File(h.env.dir, "blocked").apply { writeText("x") }
        val broken = OriginalLedger(File(blocked, "originals.jsonl"))
        assertFalse(broken.queue(reread.all().single().copy(sha256 = "z")))
        assertEquals(0, broken.all().size)
    }

    // 足すのは 1 行の追記（足すたびに全体を書き直さない。R6）。切れた最後の行は捨て、次の追記を繋げない
    @Test
    fun queueing_appends_one_line_and_a_torn_last_line_is_skipped_on_reload() {
        val dir = java.nio.file.Files.createTempDirectory("originals").toFile()
        val file = File(dir, ORIGINALS_FILE)
        val ledger = OriginalLedger(file)
        fun row(n: Int) = OriginalEntry(
            sha256 = "%064d".format(n),
            volume = PRIMARY,
            mediaId = n.toLong(),
            source = MEDIA_PHOTO_SOURCE,
            size = 10,
            backlog = false,
            dateAdded = 0,
            queuedAt = n.toLong(),
            state = ORIGINAL_AWAITING_RECORD,
            recordId = "r$n",
        )
        assertTrue(ledger.queue(row(1)))
        val first = file.readText()
        assertTrue(ledger.queue(row(2)))
        assertTrue("前の行が書き直された", file.readText().startsWith(first))
        assertEquals(2, file.readLines().count { it.isNotBlank() })
        // 同じ行を同じ記録で足し直しても書かない
        val before = file.length()
        assertTrue(ledger.queue(row(2)))
        assertEquals(before, file.length())
        // 追記の途中で落ちた: 最後の行が改行なしで切れている
        file.appendText("{\"sha256\":\"torn")
        val reopened = OriginalLedger(file)
        assertEquals(2, reopened.all().size)
        assertTrue(reopened.queue(row(3)))
        // 記録だけを送り直した版（行の付け替え）も追記で、読み直すと後の行が勝つ
        assertTrue(reopened.queue(row(1).copy(recordId = "r1b")))
        val again = OriginalLedger(file)
        assertEquals(listOf("r1b", "r2", "r3"), again.all().map { it.recordId })
        // 全体の書き直し（ready）で重なった古い版と切れた行が消える
        assertTrue(again.markReady(listOf(row(1).sha256)))
        assertEquals(3, file.readLines().count { it.isNotBlank() })
        assertEquals(again.all(), OriginalLedger(file).all())
    }
}
