// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.Duration

/**
 * 原本の送り手（tasks 5.2 / 5.3。design D9）。端末のファイルは [FakeMediaListing] の中身、受け口は [FakeOriginalTransport]
 * （S-01 と同じく、ハッシュが合うものだけ置く）。届いたバイトを端末のファイルと突き合わせる。
 */
class OriginalUploaderTest {
    private val minute = 60_000L

    // Scenario: 写真を撮るとメタが D-01 に、原本が D-02 に入る
    @Test
    fun a_new_photo_sends_its_record_and_then_its_original_byte_for_byte() {
        val h = OriginalHarness()
        h.photo(1, "a photo just taken", hoursAgo = 0)
        h.scan()
        assertEquals("記録が 1 件送られて受理される", 1, h.acceptRecords().accepted)
        assertEquals(1, h.uploader.run())
        val onDevice = h.env.listing.contents.getValue(PRIMARY to 1L)
        assertArrayEquals(onDevice, h.transport.stored.getValue(sha256Of(onDevice)))
        assertEquals(listOf(onDevice.size.toLong()), h.transport.lengths)
    }

    // Scenario: 動画を撮るとメタだけが入り原本は送られない
    @Test
    fun a_video_leaves_only_its_record_and_the_original_is_never_sent() {
        val h = OriginalHarness()
        h.env.listing.putVideo(PRIMARY, 1, "DCIM/Camera/", "VID_1.mp4", "moving pictures")
        h.scan()
        assertEquals(1, h.env.photos().size)
        assertEquals(0, h.env.originals.all().size)
        h.acceptRecords()
        h.uploader.run()
        assertTrue(h.transport.heads.isEmpty())
        assertTrue(h.transport.puts.isEmpty())
        assertEquals(0, h.source.opens)
    }

    // Scenario: 原本は形式を変えずに送られる
    @Test
    fun a_heic_original_arrives_byte_for_byte() {
        val h = OriginalHarness()
        h.photo(1, "x", name = "IMG_1.heic")
        // 先頭の入れ物の印 `....ftypheic` のあとに、文字として読めないバイトが続く
        val heic = byteArrayOf(0, 0, 0, 24) + "ftypheic".toByteArray() + byteArrayOf(-1, 0, 127, -128)
        h.env.listing.contents[PRIMARY to 1L] = heic
        h.scan()
        h.acceptRecords()
        assertEquals(1, h.uploader.run())
        assertArrayEquals(heic, h.transport.stored.getValue(sha256Of(heic)))
        assertEquals(heic.size.toLong(), h.transport.lengths.single())
        assertEquals(0, h.env.originals.all().size)
    }

    // Scenario: 送るときに中身が変わっていたら古い版のバイトとして送らない
    @Test
    fun an_edited_photo_is_not_sent_as_the_old_version() {
        val h = OriginalHarness()
        h.photo(1, "before the edit")
        h.scan()
        val oldSha = h.shaOf(1)
        h.acceptRecords()
        h.env.listing.contents[PRIMARY to 1L] = "after the edit".toByteArray()
        assertEquals(0, h.uploader.run())
        assertTrue("上書き後のバイトを送った", h.transport.puts.isEmpty())
        val report = h.missingReports().single()
        assertEquals("changed", report.reason)
        assertEquals(oldSha, report.sha256)
        assertNull(h.env.originals.get(oldSha))
    }

    @Test
    fun a_photo_edited_while_it_is_being_sent_is_refused_and_found_changed_next_time() {
        val h = OriginalHarness()
        h.photo(1, "before the edit")
        h.scan()
        h.acceptRecords()
        h.source.afterFirstOpen = "edited during the upload".toByteArray()
        h.uploader.run()
        // S-01 はハッシュが合わないので置かない（422）。行は残り、欠けにもまだしない
        assertTrue(h.transport.stored.isEmpty())
        assertEquals(1, h.env.originals.all().size)
        assertTrue(h.missingReports().isEmpty())
        // 次の回の 1 回目で「変わった」と分かる
        h.env.listing.contents[PRIMARY to 1L] = "edited during the upload".toByteArray()
        h.advanceMs(2 * minute)
        h.uploader.run()
        assertEquals(listOf("changed"), h.missingReports().map { it.reason })
        assertEquals(0, h.env.originals.all().size)
    }

    // Scenario: 新しく撮った写真の原本はモバイル通信でも送られる
    @Test
    fun a_new_photo_is_sent_over_a_metered_connection() {
        val h = OriginalHarness()
        h.unmetered = false
        h.photo(1, "just taken", hoursAgo = 0)
        h.scan()
        h.acceptRecords()
        assertEquals(1, h.uploader.run())
        assertEquals(1, h.transport.stored.size)
    }

    // Scenario: 過去ぶんの原本はモバイル通信では送られない
    @Test
    fun a_backlog_original_is_not_sent_over_a_metered_connection_but_its_record_is() {
        val h = OriginalHarness()
        h.unmetered = false
        h.photo(1, "three years ago", hoursAgo = 3 * 365 * 24L)
        h.scan()
        assertEquals("記録は送られる", 1, h.acceptRecords().accepted)
        assertEquals(0, h.uploader.run())
        assertTrue(h.transport.heads.isEmpty())
        assertTrue(h.transport.stored.isEmpty())
        assertEquals(ORIGINAL_READY, h.env.originals.all().single().state)
    }

    // Scenario: 過去ぶんの原本は Wi-Fi になると送られる
    @Test
    fun a_backlog_original_is_sent_once_wifi_is_connected() {
        val h = OriginalHarness()
        h.unmetered = false
        h.photo(1, "three years ago", hoursAgo = 3 * 365 * 24L)
        h.scan()
        h.acceptRecords()
        h.uploader.run()
        h.unmetered = true
        assertEquals(1, h.uploader.run())
        assertEquals(1, h.transport.stored.size)
    }

    // Scenario: 新しく撮った写真の原本は過去ぶんより先に送られる
    @Test
    fun a_new_photo_goes_before_the_backlog() {
        val h = OriginalHarness()
        repeat(20) { h.photo(it + 1L, "old photo $it", hoursAgo = (2 + it) * 24L) }
        h.scan()
        h.acceptRecords()
        h.advanceMs(60 * minute)
        h.photo(100, "taken a moment ago", hoursAgo = 0)
        h.scan()
        h.acceptRecords()
        h.uploader.run()
        assertEquals(21, h.transport.puts.size)
        assertEquals(h.shaOf(100), h.transport.puts.first())
    }

    // Scenario: D-02 に書けなければ送り直せる失敗になり、端末に送り待ちが残る
    @Test
    fun a_storage_failure_keeps_the_waiting_mark_and_is_retried() {
        val h = OriginalHarness()
        h.photo(1, "needs a place")
        h.scan()
        h.acceptRecords()
        h.transport.putStatus = 503
        assertEquals(0, h.uploader.run())
        assertEquals(ORIGINAL_READY, h.env.originals.get(h.shaOf(1))?.state)
        assertTrue(h.missingReports().isEmpty())
        assertTrue(h.lines.any { it.contains("kind=original_send_failed") && it.contains("error=server_503") })
        h.transport.putStatus = null
        h.advanceMs(2 * minute)
        assertEquals(1, h.uploader.run())
        assertEquals(0, h.env.originals.all().size)
    }

    // Scenario: 送る前に端末から消えた写真の原本は欠けとして残る
    @Test
    fun a_photo_deleted_before_it_could_be_sent_becomes_a_missing_original() {
        val h = OriginalHarness()
        h.photo(1, "short lived")
        h.scan()
        val sha = h.shaOf(1)
        h.env.listing.remove(PRIMARY, 1)
        h.acceptRecords()
        h.uploader.run()
        val report = h.missingReports().single()
        assertEquals("vanished", report.reason)
        assertEquals(sha, report.sha256)
        assertEquals(MEDIA_PHOTO_SOURCE, report.logicalSource)
        assertEquals(0, h.env.originals.all().size)
        assertTrue(h.transport.puts.isEmpty())
    }

    @Test
    fun a_file_that_exists_but_cannot_be_opened_is_reported_unreadable() {
        val h = OriginalHarness()
        h.photo(1, "locked")
        h.scan()
        h.acceptRecords()
        h.source.unreadable = true
        h.uploader.run()
        assertEquals(listOf("unreadable"), h.missingReports().map { it.reason })
        assertEquals(0, h.env.originals.all().size)
    }

    // Scenario: 受け取り済みの原本は送り直さない
    @Test
    fun an_original_the_server_already_has_is_not_read_or_sent_again() {
        val h = OriginalHarness()
        h.photo(1, "already there")
        h.scan()
        h.acceptRecords()
        h.transport.stored[h.shaOf(1)] = "kept".toByteArray()
        assertEquals(0, h.uploader.run())
        assertEquals("ファイルを読んだ", 0, h.source.opens)
        assertTrue(h.transport.puts.isEmpty())
        assertEquals(0, h.env.originals.all().size)
    }

    // Scenario: 大きすぎて受け取られない原本は欠けになる
    @Test
    fun an_original_that_is_too_large_becomes_a_missing_original() {
        val h = OriginalHarness()
        h.photo(1, "enormous")
        h.scan()
        h.acceptRecords()
        h.transport.putStatus = 413
        h.uploader.run()
        assertEquals(listOf("too_large"), h.missingReports().map { it.reason })
        assertEquals(0, h.env.originals.all().size)
    }

    // Scenario: 保存から 24 時間たっていなければ過去ぶんにしない
    @Test
    fun a_photo_found_23_hours_after_saving_is_new_and_one_found_25_hours_after_is_backlog() {
        val h = OriginalHarness()
        h.unmetered = false
        h.photo(1, "found after 23 hours", hoursAgo = 23)
        h.photo(2, "found after 25 hours", hoursAgo = 25)
        h.scan()
        h.acceptRecords()
        h.uploader.run()
        assertEquals(listOf(h.shaOf(1)), h.transport.puts)
        assertTrue(h.env.originals.get(h.shaOf(2))?.backlog == true)
        assertTrue(h.env.originals.get(h.shaOf(1)) == null)
    }

    // Scenario: 撮影が古くても保存が新しい画像は新しいものとして送る
    @Test
    fun an_image_taken_long_ago_but_saved_today_is_sent_as_new() {
        val h = OriginalHarness()
        h.unmetered = false
        h.env.files.metaOf = { MediaFileMeta(kind = it.kind, takenAtRaw = "2023:09:14 10:00:00", offsetRaw = "+09:00") }
        h.photo(1, "received from a friend", hoursAgo = 1)
        h.scan()
        assertEquals("taken", h.env.photos().single().payload["time_basis"]?.jsonPrimitive?.content)
        h.acceptRecords()
        assertEquals(1, h.uploader.run())
    }

    // Scenario: 過去ぶんどうしは保存の新しいものから送られる
    @Test
    fun backlog_originals_go_newest_saved_first() {
        val h = OriginalHarness()
        val year = 365 * 24L
        h.photo(1, "saved in 2020", hoursAgo = 6 * year)
        h.photo(2, "saved in 2025", hoursAgo = 1 * year)
        h.photo(3, "saved in 2023", hoursAgo = 3 * year)
        h.scan()
        h.acceptRecords()
        h.uploader.run()
        assertEquals(listOf(2L, 3L, 1L).map(h::shaOf), h.transport.puts)
    }

    // Scenario: 原本の送信の失敗がログに出てもファイル名と位置は出ない
    @Test
    fun a_failed_upload_logs_only_the_kind_and_the_count() {
        val h = OriginalHarness()
        h.env.files.metaOf = { MediaFileMeta(kind = it.kind, latitude = 35.681236, longitude = 139.767125) }
        h.photo(1, "with a place", name = "secret-name.jpg")
        h.scan()
        h.acceptRecords()
        h.transport.putStatus = 503
        h.uploader.run()
        val failed = h.lines.single { it.startsWith("kind=original_send_failed") }
        assertTrue(failed.contains("count=1"))
        assertTrue(failed.contains("error=server_503"))
        val all = (h.lines + h.env.lines).joinToString("\n")
        for (secret in listOf("35.68", "139.76", "secret-name", "DCIM", h.shaOf(1))) {
            assertFalse("ログに $secret が出ている", all.contains(secret))
        }
    }

    @Test
    fun an_unreachable_server_stops_the_pass_and_keeps_every_mark() {
        val h = OriginalHarness()
        repeat(3) { h.photo(it + 1L, "photo $it") }
        h.scan()
        h.acceptRecords()
        h.transport.unreachable = true
        assertEquals(0, h.uploader.run())
        assertEquals("届かないのに全部に当たった", 1, h.transport.heads.size)
        assertEquals(3, h.env.originals.all().size)
        assertTrue(h.lines.any { it.contains("error=unreachable") })
    }

    @Test
    fun retries_wait_one_minute_then_double_up_to_one_hour() {
        val h = OriginalHarness()
        h.photo(1, "never lands")
        h.scan()
        h.acceptRecords()
        h.transport.unreachable = true
        var wait = UPLOAD_RETRY_FIRST.toMillis()
        repeat(10) {
            h.uploader.run()
            val attempts = h.transport.heads.size
            h.advanceMs(wait - 1)
            h.uploader.run()
            assertEquals("間隔の前に試し直した（待ち ${wait}ms）", attempts, h.transport.heads.size)
            h.advanceMs(1)
            h.uploader.run()
            assertEquals("間隔が来ても試し直さない（待ち ${wait}ms）", attempts + 1, h.transport.heads.size)
            wait = minOf(wait * 2, UPLOAD_RETRY_MAX.toMillis())
        }
        assertEquals(Duration.ofHours(1).toMillis(), wait)
    }

    @Test
    fun record_missing_is_retried_and_becomes_a_missing_original_after_seven_days() {
        val h = OriginalHarness()
        h.photo(1, "its record went away")
        h.scan()
        h.acceptRecords()
        h.transport.putStatus = 409
        h.uploader.run()
        assertEquals(1, h.env.originals.all().size)
        assertNotNull(h.env.originals.all().single().missingSince)
        h.advanceMs(RECORD_MISSING_GIVE_UP.toMillis() - minute)
        h.uploader.run()
        assertTrue("7 日に届かないのに欠けにした", h.missingReports().isEmpty())
        assertEquals(1, h.env.originals.all().size)
        h.advanceMs(2 * minute)
        h.uploader.run()
        assertEquals(listOf("record_missing"), h.missingReports().map { it.reason })
        assertEquals(0, h.env.originals.all().size)
    }

    @Test
    fun nothing_is_opened_while_the_location_permission_is_missing() {
        val h = OriginalHarness()
        h.photo(1, "waiting for the permission")
        h.scan()
        h.acceptRecords()
        h.access = MediaAccess.NO_LOCATION
        assertEquals(0, h.uploader.run())
        assertEquals(0, h.source.opens)
        assertTrue(h.transport.heads.isEmpty())
        h.access = MediaAccess.FULL
        assertEquals(1, h.uploader.run())
    }

    @Test
    fun a_permission_that_is_not_there_for_now_keeps_the_mark_without_a_missing_report() {
        val h = OriginalHarness()
        h.photo(1, "no longer selected")
        h.scan()
        h.acceptRecords()
        h.source.denied = true
        h.uploader.run()
        assertTrue(h.missingReports().isEmpty())
        assertEquals(1, h.env.originals.all().size)
        assertTrue(h.lines.any { it.contains("error=denied") })
    }
}
