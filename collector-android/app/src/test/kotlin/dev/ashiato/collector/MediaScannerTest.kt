// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.Duration

/**
 * 読み手と見分けの台帳（tasks 3.1 / 3.2 / design D1・D2・D3・D4）。端末の一覧と開くファイルは偽物
 * （[FakeMediaListing]）で、台帳・読み進めた印・未送信は実物の置き場に書く。
 */
class MediaScannerTest {
    private fun MediaScanEnv.camera(id: Long, body: String, volume: String = PRIMARY) =
        listing.put(volume, id, "DCIM/Camera/", "IMG_$id.jpg", body)

    // Scenario: 導入した時点で端末にある過去の写真が取られる
    @Test
    fun a_photo_already_on_the_device_is_captured_on_first_scan() {
        val env = MediaScanEnv()
        env.camera(1, "three years ago")
        val result = env.scan()
        assertEquals(1, env.photos().size)
        assertTrue(result is MediaScanResult.Scanned && result.fullReconcile)
    }

    // Scenario: 収集を止めていた間に保存された写真も取られる
    @Test
    fun a_photo_saved_while_collection_was_stopped_is_captured_on_restart() {
        val env = MediaScanEnv()
        env.camera(1, "before")
        env.scan()
        env.camera(2, "while stopped")
        env.scanner = env.newScanner() // 収集を止めて、作り直して再び始める
        env.advanceHours(1)
        env.scan()
        assertEquals(2, env.photos().size)
    }

    // Scenario: 保存の知らせで 6 時間の見直しを待たずに取られる
    @Test
    fun a_save_notification_captures_the_photo_within_a_minute() {
        val env = MediaScanEnv()
        val timer = VirtualTimer()
        val driver = MediaScanDriver(env.scanner, timer)
        driver.start()
        timer.advance(0) // 最初の見直しは契機の口で走る（呼び手を塞がない）
        assertEquals(0, env.photos().size)
        env.camera(1, "just taken")
        driver.onMediaChanged()
        driver.onMediaChanged() // 続けて来た知らせは 1 回にまとまる
        timer.advance(OBSERVER_DEBOUNCE_MS)
        assertEquals(1, env.photos().size)
        assertTrue(OBSERVER_DEBOUNCE_MS <= Duration.ofMinutes(1).toMillis())
    }

    // Scenario: 知らせが届かなくても 6 時間の見直しで取られる
    @Test
    fun without_notifications_the_six_hour_review_captures_the_photo() {
        val env = MediaScanEnv()
        val timer = VirtualTimer()
        val driver = MediaScanDriver(env.scanner, timer)
        driver.start()
        timer.advance(0) // 最初の見直しは契機の口で走る（呼び手を塞がない）
        env.camera(1, "no notification")
        timer.advance(REVIEW_EVERY_MS - 1) { }
        assertEquals(0, env.photos().size)
        timer.advance(1)
        assertEquals(1, env.photos().size)
    }

    // Scenario: 変わったかの印が上がらなくても 1 日 1 回の突き合わせで取られる
    @Test
    fun the_daily_full_reconcile_finds_a_photo_whose_generation_never_moved() {
        val env = MediaScanEnv()
        env.camera(1, "known")
        env.scan()
        env.listing.putSilently(PRIMARY, 2, "DCIM/Camera/", "IMG_2.jpg", "silent")
        env.advanceHours(6)
        env.scan()
        assertEquals("世代の数が同じなら見直しでは読まない", 1, env.photos().size)
        env.advanceHours(18)
        env.scan()
        assertEquals(2, env.photos().size)
    }

    // Scenario: 一覧が作り直されても記録は増えない
    @Test
    fun a_rebuilt_listing_is_matched_again_without_new_records() {
        val env = MediaScanEnv()
        repeat(10) { env.camera(it + 1L, "photo $it") }
        env.scan()
        assertEquals(10, env.photos().size)
        env.listing.rebuild(PRIMARY, shift = 1000)
        val opened = env.files.opened
        env.advanceHours(1)
        env.scan()
        assertEquals("番号が振り直されても記録は増えない", 10, env.photos().size)
        assertEquals("振り直しでは全部を計算し直す", opened + 10, env.files.opened)
        assertEquals(10, env.photos().map { it.externalId }.toSet().size)
    }

    // Scenario: 2 つの置き場の写真がどちらも取られる
    @Test
    fun photos_on_two_volumes_are_both_captured() {
        val env = MediaScanEnv()
        env.listing.volumeSet += SD_CARD
        env.camera(1, "internal")
        env.camera(1, "sd card", SD_CARD)
        env.scan()
        assertEquals(2, env.photos().size)
    }

    // Scenario: 同じ写真を 2 回見ても記録は増えない
    @Test
    fun looking_at_the_same_photo_twice_adds_nothing() {
        val env = MediaScanEnv()
        env.camera(1, "once")
        env.scan()
        env.advanceHours(25)
        val opened = env.files.opened
        env.scan()
        env.scan()
        assertEquals(1, env.photos().size)
        assertEquals("変わっていない項目のファイルは開かない", opened, env.files.opened)
    }

    // Scenario: カメラで撮った写真は写真のソースに入る
    @Test
    fun camera_photo_goes_to_the_photo_source() {
        val env = MediaScanEnv()
        env.camera(1, "x")
        env.scan()
        assertEquals("c01-photo", env.photos().single().logicalSource)
    }

    // Scenario: スクリーンショットはスクリーンショットのソースに入る
    @Test
    fun screenshot_goes_to_the_screenshot_source() {
        val env = MediaScanEnv()
        env.listing.put(PRIMARY, 1, "Pictures/Screenshots/", "Screenshot_1.png", "s")
        env.listing.put(PRIMARY, 2, "DCIM/Screenshots/", "Screenshot_2.png", "t")
        env.scan()
        assertEquals(setOf("c01-screenshot"), env.photos().map { it.logicalSource }.toSet())
        assertEquals(2, env.photos().size)
    }

    // Scenario: 受け取った画像はカメラ以外の画像のソースに入る
    @Test
    fun downloaded_image_goes_to_the_image_source() {
        val env = MediaScanEnv()
        env.listing.put(PRIMARY, 1, "Download/", "received.jpg", "d")
        env.listing.put(PRIMARY, 2, "Pictures/Chat/", "edit.jpg", "e")
        env.scan()
        assertEquals(setOf("c01-image"), env.photos().map { it.logicalSource }.toSet())
    }

    // Scenario: 動画は動画のソースに入る
    @Test
    fun video_goes_to_the_video_source_wherever_it_is_saved() {
        val env = MediaScanEnv()
        env.listing.putVideo(PRIMARY, 1, "DCIM/Camera/", "VID_1.mp4", "v")
        env.listing.putVideo(PRIMARY, 2, "Download/", "VID_2.mp4", "w")
        env.scan()
        assertEquals(setOf("c01-video"), env.photos().map { it.logicalSource }.toSet())
    }

    @Test
    fun the_source_stays_where_the_first_record_put_it_after_a_move() {
        val env = MediaScanEnv()
        env.camera(1, "stays")
        env.scan()
        env.listing.put(PRIMARY, 1, "Download/", "IMG_1.jpg", "stays")
        env.scan()
        assertEquals(setOf("c01-photo"), env.sent().map { it.logicalSource }.toSet())
    }

    @Test
    fun a_big_listing_is_read_in_batches_of_200() {
        val env = MediaScanEnv()
        repeat(450) { env.camera(it + 1L, "p$it") }
        env.scan()
        assertEquals(450, env.photos().size)
        assertEquals(200, env.listing.biggestPage)
    }

    @Test
    fun nothing_is_opened_while_the_location_permission_is_missing() {
        val env = MediaScanEnv()
        env.camera(1, "x")
        env.location = false
        val result = env.scan()
        assertTrue(result is MediaScanResult.Unavailable)
        assertEquals(0, env.files.opened)
        env.location = true
        env.scan()
        assertEquals(1, env.photos().size)
    }

    @Test
    fun a_file_that_cannot_be_opened_holds_the_cursor_and_is_retried() {
        val env = MediaScanEnv()
        env.camera(1, "x")
        env.files.unreadable = true
        assertTrue(env.scan() is MediaScanResult.Held)
        env.files.unreadable = false
        env.scan()
        assertEquals(1, env.photos().size)
    }

    @Test
    fun one_unopenable_file_does_not_make_every_later_scan_reopen_everything() {
        val env = MediaScanEnv()
        repeat(5) { env.camera(it + 1L, "photo $it") }
        env.files.unreadableIds += 5L
        assertTrue(env.scan() is MediaScanResult.Held)
        assertEquals(4, env.photos().size)
        env.files.attempts = 0
        env.advanceHours(1)
        env.scan()
        assertEquals("開けない 1 枚だけを試す", 1, env.files.attempts)
        env.files.unreadableIds.clear()
        env.advanceHours(1)
        env.scan()
        assertEquals(5, env.photos().size)
    }

    @Test
    fun an_unopenable_file_does_not_stop_the_vanished_judgement() {
        val env = MediaScanEnv()
        env.camera(1, "stays")
        env.camera(2, "goes")
        env.scan()
        env.listing.remove(PRIMARY, 2)
        env.camera(3, "stuck")
        env.files.unreadableIds += 3L
        env.advanceHours(25)
        env.scan()
        assertEquals(1, env.vanished().size)
    }

    @Test
    fun a_rebuild_keeps_the_old_ledger_on_disk_until_it_is_matched_again() {
        val env = MediaScanEnv()
        repeat(3) { env.camera(it + 1L, "photo $it") }
        env.scan()
        env.listing.rebuild(PRIMARY, 100)
        env.files.unreadableIds += 101L
        assertTrue(env.scan() is MediaScanResult.Held)
        env.scanner = env.newScanner() // 落ちて再び始める: ディスクの台帳だけが頼り
        env.files.unreadableIds.clear()
        env.advanceHours(1)
        env.scan()
        assertEquals("振り直されただけで記録は増えない", 3, env.photos().size)
        assertEquals(0, env.vanished().size)
    }

    @Test
    fun the_constants_are_pinned() {
        assertEquals(200, SCAN_BATCH)
        assertEquals(10_000L, OBSERVER_DEBOUNCE_MS)
        assertEquals(Duration.ofHours(24), FULL_RECONCILE_EVERY)
        assertEquals(6 * 3600 * 1000L, REVIEW_EVERY_MS)
    }

    // ---- 3.2 見分けと変わった後の版 -------------------------------------------------------------

    // Scenario: 同じ中身の写真が 2 つ保存されても記録は 1 件
    @Test
    fun the_same_content_under_two_names_is_sent_once() {
        val env = MediaScanEnv()
        env.camera(1, "same bytes")
        env.listing.put(PRIMARY, 2, "Download/", "copy.jpg", "same bytes")
        env.scan()
        assertEquals(1, env.photos().size)
        assertEquals(sha256Hex("same bytes".byteInputStream()), env.photos().single().externalId)
    }

    // Scenario: 原文だけ変わった写真は 1 件のまま前の版が残る
    @Test
    fun a_moved_photo_is_resent_under_the_same_external_id() {
        val env = MediaScanEnv()
        env.camera(1, "moves")
        env.scan()
        env.listing.put(PRIMARY, 1, "Pictures/Trip/", "IMG_1.jpg", "moves")
        env.scan()
        val sent = env.photos()
        assertEquals(2, sent.size)
        assertEquals(sent[0].externalId, sent[1].externalId)
        assertNotEquals(sent[0].raw, sent[1].raw)
        assertEquals("Pictures/Trip/", sent[1].payload["relative_path"]?.jsonPrimitive?.content)
        assertNull(sent[1].payload["previous_sha256"])
    }

    // Scenario: 中身が変わった写真は新しい記録になり前の記録も残る
    @Test
    fun an_edited_photo_becomes_a_new_record_that_names_the_old_hash() {
        val env = MediaScanEnv()
        env.camera(1, "original")
        env.scan()
        val before = env.photos().single()
        env.listing.put(PRIMARY, 1, "DCIM/Camera/", "IMG_1.jpg", "edited!")
        env.scan()
        val sent = env.photos()
        assertEquals(2, sent.size)
        val after = sent.last()
        assertNotEquals(before.externalId, after.externalId)
        assertEquals(before.externalId, after.payload["previous_sha256"]?.jsonPrimitive?.content)
        assertEquals("1", after.payload["media_id"]?.jsonPrimitive?.content)
        assertEquals("前の記録は積んだまま", before.externalId, sent.first().externalId)
    }

    // Scenario: 消えた事実を作っても写真の記録は前の版にならない
    @Test
    fun a_vanished_fact_has_its_own_identifier_and_never_names_a_previous_version() {
        val env = MediaScanEnv()
        env.camera(1, "will vanish")
        env.scan()
        env.listing.remove(PRIMARY, 1)
        env.advanceHours(25)
        env.scan()
        val photo = env.photos().single()
        val gone = env.vanished().single()
        assertNotEquals(photo.externalId, gone.externalId)
        assertEquals("vanished:${photo.externalId}:$PRIMARY:1", gone.externalId)
        assertNull(gone.payload["previous_sha256"])
        assertFalse(env.photos().size > 1)
    }
}
