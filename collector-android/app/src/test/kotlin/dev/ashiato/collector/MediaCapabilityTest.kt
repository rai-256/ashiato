// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import java.io.File
import java.nio.file.Files

/**
 * メディアの取得可否・4 本の生存信号・試行と成功の数え方（tasks 4.2 / design D6・D7・D22 / C19 / Q15）。
 * 一覧とファイルは偽物（[FakeMediaListing]）、未送信・台帳・数えは実物の置き場に書く。
 */
@RunWith(RobolectricTestRunner::class)
class MediaCapabilityTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    /** 読み手 + 4 つのソースの数え + 知らせの宛先を組んだもの。時計は [MediaScanEnv.wall]。 */
    private class Rig(outbox: Outbox<IngestRequest>? = null, blockLedger: Boolean = false) {
        val env = MediaScanEnv()
        val timer = VirtualTimer()
        val notices = mutableListOf<MediaPermissionState>()
        var network = true
        private var state = MediaPermissionState(MediaAccess.FULL, location = true)

        init {
            if (outbox != null) env.outbox = outbox
            // 台帳の置き場が書けない（同じ名前のファイルが居座っている）
            if (blockLedger) File(env.dir, MEDIA_LEDGER_DIR).writeText("not a directory")
        }

        val channels = MEDIA_CADENCES.map {
            MediaChannel(it, AttemptCounters(now = { env.wall }, intervalMs = it.intervalMs))
        }
        val capture = MediaCapture(
            channels = channels,
            permissions = { state },
            notice = { notices += it },
            listing = env.listing,
            driver = { onOutcome -> MediaScanDriver(env.newScanner(), timer, onOutcome = onOutcome) },
        )

        fun permissions(access: MediaAccess, location: Boolean) {
            state = MediaPermissionState(access, location)
            env.access = access
            env.location = location
        }

        fun camera(id: Long, body: String) = env.listing.put(PRIMARY, id, "DCIM/Camera/", "IMG_$id.jpg", body)

        /** 読み手を始め、最初の見直し（契機 1 回）まで回す。 */
        fun begin() {
            capture.start()
            timer.advance(0)
        }

        /** 次の 6 時間の見直しまで進める（契機 1 回）。 */
        fun review() = timer.advance(REVIEW_EVERY_MS)

        /** 保存の知らせが 1 回届き、まとめの待ちが明ける（契機 1 回）。 */
        fun notified() {
            env.listing.changed()
            timer.advance(OBSERVER_DEBOUNCE_MS)
        }

        /** そのソースの生存信号を 1 件積んで返す（本番と同じ組み立て）。 */
        fun beat(cadence: SourceCadence = SourceCadence.PHOTO): HeartbeatRequest {
            val channel = channels.first { it.cadence == cadence }
            val out = testHeartbeatOutbox()
            HeartbeatEmitter(
                outbox = out,
                counters = channel.counters,
                userId = "user-1",
                deviceId = "device-1",
                logicalSource = cadence.logicalSource,
                capability = { capture.capability(cadence.logicalSource) },
                now = { env.wall },
                newId = { "hb-1" },
                network = { network },
                onStored = { channel.stages.reset() },
            ).emit()
            return out.snapshot().single()
        }
    }

    /** 記録を未送信に積めない置き場（区切りの置き場が居座る名前のファイルで塞がれている）。 */
    private fun brokenOutbox(): Outbox<IngestRequest> {
        val dir = Files.createTempDirectory("broken-outbox").toFile()
        File(dir, "segments").writeText("not a directory")
        return Outbox.inDir(dir, IngestRequest.serializer()) {}
    }

    // Scenario: 写真の位置の権限が無ければ写真を 1 件も読まない
    @Test
    fun without_the_photo_location_permission_not_one_file_is_opened() {
        val rig = Rig()
        rig.permissions(MediaAccess.FULL, location = false)
        rig.camera(1, "taken")

        rig.begin()
        rig.review()

        assertEquals("ファイルを開いている", 0, rig.env.files.attempts)
        assertTrue("記録が作られている", rig.env.sent().isEmpty())
        assertFalse("台帳が書かれている", File(rig.env.dir, MEDIA_LEDGER_DIR).exists())
    }

    // Scenario: 写真の位置の権限が無いと生存信号は取れない状態になる
    @Test
    fun without_the_photo_location_permission_the_heartbeat_says_so() {
        val rig = Rig()
        rig.permissions(MediaAccess.FULL, location = false)
        rig.camera(1, "taken")
        rig.begin()

        val beat = rig.beat()

        assertFalse(beat.capturable)
        assertEquals(listOf(MediaBlocker.MEDIA_LOCATION), beat.blockers)
        assertEquals("c01-photo", beat.logicalSource)
    }

    // Scenario: 権限が戻ると待っていた間の写真も取る
    @Test
    fun when_the_permission_comes_back_the_photos_taken_meanwhile_are_captured() {
        val rig = Rig()
        rig.permissions(MediaAccess.FULL, location = false)
        rig.camera(1, "while waiting 1")
        rig.camera(2, "while waiting 2")
        rig.begin()
        assertTrue(rig.env.sent().isEmpty())
        assertTrue("知らせが出ていない", mediaNoticeText(rig.notices.last()) != null)

        rig.permissions(MediaAccess.FULL, location = true)
        rig.review()

        assertEquals("待っていた 2 枚が取られていない", 2, rig.env.photos().size)
        val beat = rig.beat()
        assertTrue("取得できる状態に戻っていない: ${beat.blockers}", beat.capturable)
        assertNull("知らせが消えていない", mediaNoticeText(rig.notices.last()))
    }

    // Scenario: 選んだ写真だけの許可では選んだものを取り、取れない状態と報告する
    @Test
    fun a_selected_photos_only_grant_captures_the_selected_and_reports_partial() {
        val rig = Rig()
        rig.permissions(MediaAccess.PARTIAL, location = true)
        rig.camera(1, "selected 1")
        rig.camera(2, "selected 2")

        rig.begin()

        assertEquals("選んだ 2 枚が取られていない", 2, rig.env.photos().size)
        val beat = rig.beat()
        assertFalse(beat.capturable)
        assertEquals(listOf(MediaBlocker.PARTIAL), beat.blockers)
    }

    // Scenario: 圏外でも写真の生存信号は取れる状態のまま接続が無かったことが載る
    @Test
    fun offline_the_photo_heartbeat_stays_capturable_and_records_no_connection() {
        val rig = Rig()
        rig.camera(1, "taken")
        rig.begin()
        rig.network = false

        val beat = rig.beat()

        assertTrue("接続が無いだけで取れない状態になっている: ${beat.blockers}", beat.capturable)
        assertTrue(beat.blockers.isEmpty())
        assertEquals(false, beat.network)
        assertTrue("原文に接続の欄が無い: ${beat.raw}", beat.raw.contains("\"network\":false"))
    }

    // Scenario: 接続があれば接続ありが載る
    @Test
    fun online_the_photo_heartbeat_records_the_connection() {
        val rig = Rig()
        rig.camera(1, "taken")
        rig.begin()

        val beat = rig.beat()

        assertTrue(beat.capturable)
        assertEquals(true, beat.network)
        assertTrue(beat.raw.contains("\"network\":true"))
    }

    // Scenario: ハッシュの計算に失敗し続けると取れない状態になる
    @Test
    fun a_hash_that_keeps_failing_makes_the_source_uncapturable_with_the_stage() {
        val rig = Rig()
        rig.env.files.failStage = MediaStage.HASH
        rig.camera(1, "cannot hash")

        rig.begin()
        rig.review()
        val beat = rig.beat()

        assertFalse(beat.capturable)
        assertEquals(listOf("stage_hash"), beat.blockers)
    }

    // Scenario: 記録を未送信に積めないと取れない状態になる
    @Test
    fun records_that_cannot_be_queued_make_the_source_uncapturable_with_the_stage() {
        val rig = Rig(outbox = brokenOutbox())
        rig.camera(1, "cannot queue")

        rig.begin()
        rig.review()
        val beat = rig.beat()

        assertFalse(beat.capturable)
        assertEquals(listOf("stage_enqueue"), beat.blockers)
    }

    @Test
    fun a_ledger_that_cannot_be_written_is_reported_as_the_ledger_stage() {
        val rig = Rig(blockLedger = true)
        rig.camera(1, "cannot remember")

        rig.begin()
        val beat = rig.beat()

        assertFalse(beat.capturable)
        assertEquals(listOf("stage_ledger"), beat.blockers)
    }

    @Test
    fun a_file_that_cannot_be_opened_is_reported_as_the_read_stage() {
        val rig = Rig()
        rig.env.files.unreadable = true
        rig.camera(1, "locked")

        rig.begin()

        assertEquals(listOf("stage_read"), rig.beat().blockers)
    }

    @Test
    fun one_success_in_the_interval_keeps_the_stage_word_off() {
        val rig = Rig()
        rig.env.files.failStage = MediaStage.HASH
        rig.camera(1, "flaky")
        rig.begin()
        rig.env.files.failStage = null
        rig.review()

        val beat = rig.beat()

        assertTrue("成功があったのに取れない状態になっている: ${beat.blockers}", beat.capturable)
        assertEquals(1, beat.successes)
        assertEquals("失敗も試行に数える", 2, beat.attempts)
    }

    @Test
    fun the_stage_word_goes_away_once_the_heartbeat_has_been_sent() {
        val rig = Rig()
        rig.env.files.failStage = MediaStage.HASH
        rig.camera(1, "cannot hash")
        rig.begin()
        assertEquals(listOf("stage_hash"), rig.beat().blockers)

        // 区間が変わった: 次の区間の契機が成功すれば、前の区間の段は引きずらない
        rig.env.files.failStage = null
        rig.review()

        assertTrue(rig.beat().capturable)
    }

    // Scenario: アプリ利用の取得可否は接続を含めたまま
    @Test
    fun the_app_usage_capability_still_includes_the_connection() {
        val connectivity = app.getSystemService(android.net.ConnectivityManager::class.java)
        org.robolectric.Shadows.shadowOf(connectivity).setActiveNetworkInfo(null)
        org.robolectric.Shadows.shadowOf(connectivity).setNetworkCapabilities(
            connectivity.activeNetwork,
            null,
        )
        val appOps = app.getSystemService(android.app.AppOpsManager::class.java)
        org.robolectric.Shadows.shadowOf(appOps).setMode(
            android.app.AppOpsManager.OPSTR_GET_USAGE_STATS,
            android.os.Process.myUid(),
            app.packageName,
            android.app.AppOpsManager.MODE_ALLOWED,
        )

        val capability = usageAccessCapability(app)

        assertFalse(capability.capturable)
        assertEquals(listOf(Capability.NETWORK), capability.blockers)
    }

    // Scenario: メディアの 4 つのソースがそれぞれ生存信号を送る
    @Test
    fun the_four_media_sources_each_send_a_heartbeat_when_no_photo_is_taken() {
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val service = controller.get()
        service.realMediaCapability = true
        controller.create().startCommand(0, 1)

        val sent = service.heartbeatOutboxForTest.snapshot().map { it.logicalSource }
        for (source in listOf("c01-photo", "c01-screenshot", "c01-image", "c01-video")) {
            assertEquals("$source の生存信号が 1 件ずつ出ていない", 1, sent.count { it == source })
        }
        val records = service.outboxForTest.snapshot()
        assertTrue("写真は 1 枚も撮っていない", records.none { it.logicalSource in MEDIA_CADENCES.map { c -> c.logicalSource } })
    }

    // Scenario: メディアの生存信号の試行は見直しと知らせの回数で数える
    @Test
    fun media_attempts_are_counted_by_reviews_and_notifications() {
        val rig = Rig()
        rig.camera(1, "first")
        rig.begin() // 6 時間の見直しが 1 回
        repeat(3) { index ->
            rig.camera(10L + index, "taken $index")
            rig.notified() // 保存の知らせによる取得が 3 回
        }
        rig.env.advanceHours(6)

        val beat = rig.beat()

        assertEquals(4, beat.attempts)
        assertEquals(4, beat.successes)
    }

    @Test
    fun every_trigger_counts_one_attempt_for_each_of_the_four_sources_even_with_no_items() {
        val rig = Rig()
        rig.camera(1, "a photo only")
        rig.begin()
        rig.notified()
        rig.env.advanceHours(6)

        for (cadence in MEDIA_CADENCES) {
            val beat = rig.beat(cadence)
            assertEquals("${cadence.logicalSource} の試行", 2, beat.attempts)
            assertEquals("${cadence.logicalSource} の成功（0 件でも成功）", 2, beat.successes)
        }
    }

    @Test
    fun the_four_media_cadences_are_six_hours_and_satisfy_the_interval_invariant() {
        val six = 6 * 60 * 60 * 1000L
        assertEquals(
            listOf("c01-photo", "c01-screenshot", "c01-image", "c01-video"),
            MEDIA_CADENCES.map { it.logicalSource },
        )
        for (cadence in MEDIA_CADENCES) {
            assertEquals(six, cadence.intervalMs)
            assertEquals(six, cadence.heartbeatIntervalMs)
            assertTrue(SourceCadence.entries.contains(cadence))
        }
    }

    @Test
    fun the_capability_vocabulary_follows_design_d6() {
        fun words(access: MediaAccess, location: Boolean, stage: MediaStage? = null) =
            mediaCapability(MediaPermissionState(access, location), stage).blockers
        assertEquals(emptyList<String>(), words(MediaAccess.FULL, true))
        assertEquals(listOf("permission"), words(MediaAccess.NONE, true))
        assertEquals(listOf("permission", "media_location"), words(MediaAccess.NONE, false))
        assertEquals(listOf("media_location"), words(MediaAccess.FULL, false))
        assertEquals(listOf("partial"), words(MediaAccess.PARTIAL, true))
        assertEquals(listOf("stage_read"), words(MediaAccess.FULL, true, MediaStage.READ))
        assertEquals(listOf("stage_hash"), words(MediaAccess.FULL, true, MediaStage.HASH))
        assertEquals(listOf("stage_enqueue"), words(MediaAccess.FULL, true, MediaStage.ENQUEUE))
        assertEquals(listOf("stage_ledger"), words(MediaAccess.FULL, true, MediaStage.LEDGER))
        // 接続は取得可否に含めない（別の欄）
        assertFalse(Capability.NETWORK in words(MediaAccess.FULL, true))
    }
}
