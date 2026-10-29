// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/** 消えた事実（tasks 3.3 / design D5）。全部の突き合わせでだけ判定する。 */
class MediaVanishTest {
    @get:Rule val tmp = TemporaryFolder()

    private fun MediaScanEnv.camera(id: Long, body: String, volume: String = PRIMARY) =
        listing.put(volume, id, "DCIM/Camera/", "IMG_$id.jpg", body)

    /** 24 時間後の全部の突き合わせまで進めて走らせる。 */
    private fun MediaScanEnv.reconcile() {
        advanceHours(25)
        scan()
    }

    // Scenario: 端末から消えた写真は消えた事実が 1 件残る
    @Test
    fun a_photo_removed_from_the_device_leaves_one_vanished_fact() {
        val env = MediaScanEnv(tmp.newFolder())
        env.camera(1, "gone")
        env.scan()
        val sha = env.photos().single().externalId
        env.listing.remove(PRIMARY, 1)
        env.reconcile()
        val gone = env.vanished().single()
        assertEquals("vanished:$sha:$PRIMARY:1", gone.externalId)
        assertEquals(sha, gone.payload["sha256"]?.jsonPrimitive?.content)
        assertEquals(env.wall.toString(), gone.eventTime)
        assertEquals(env.wall.toString(), gone.payload["noticed_at"]?.jsonPrimitive?.content)
        assertEquals("c01-photo", gone.logicalSource)
        assertEquals("写真の記録は積んだまま", 1, env.photos().size)
    }

    // Scenario: 消えた事実を 2 度見ても 1 件のまま
    @Test
    fun a_vanished_fact_is_not_made_twice() {
        val env = MediaScanEnv(tmp.newFolder())
        env.camera(1, "gone")
        env.scan()
        env.listing.remove(PRIMARY, 1)
        env.reconcile()
        env.reconcile()
        assertEquals(1, env.vanished().size)
    }

    // Scenario: ごみ箱に入っただけでは消えたと数えない
    @Test
    fun a_trashed_photo_is_not_vanished() {
        val env = MediaScanEnv(tmp.newFolder())
        env.camera(1, "trash")
        env.scan()
        env.listing.trash(PRIMARY, 1)
        env.reconcile()
        assertEquals(0, env.vanished().size)
    }

    // Scenario: ごみ箱から消えた時点で消えたと数える
    @Test
    fun a_photo_gone_from_the_trash_is_vanished() {
        val env = MediaScanEnv(tmp.newFolder())
        env.camera(1, "trash")
        env.scan()
        env.listing.trash(PRIMARY, 1)
        env.reconcile()
        env.listing.remove(PRIMARY, 1)
        env.reconcile()
        assertEquals(1, env.vanished().size)
    }

    // Scenario: 外れた置き場の写真は消えたと数えない
    @Test
    fun a_photo_on_a_detached_volume_is_not_vanished() {
        val env = MediaScanEnv(tmp.newFolder())
        env.listing.volumeSet += SD_CARD
        env.camera(1, "on sd", SD_CARD)
        env.scan()
        env.listing.volumeSet -= SD_CARD
        env.reconcile()
        assertEquals(0, env.vanished().size)
    }

    // Scenario: 選んだ写真だけの許可に変わっても消えたと数えない
    @Test
    fun narrowing_access_to_selected_photos_does_not_vanish_the_rest() {
        val env = MediaScanEnv(tmp.newFolder())
        repeat(10) { env.camera(it + 1L, "p$it") }
        env.scan()
        repeat(9) { env.listing.remove(PRIMARY, it + 2L) } // 選んだ 1 枚だけが見える
        env.access = MediaAccess.PARTIAL
        env.reconcile()
        assertEquals(0, env.vanished().size)
        env.access = MediaAccess.FULL
        repeat(9) { env.camera(it + 2L, "p${it + 1}") }
        env.reconcile()
        assertEquals("戻した後も数えない", 0, env.vanished().size)
    }

    // Scenario: 動かした写真は消えたと数えない
    @Test
    fun a_moved_photo_with_a_new_number_is_not_vanished() {
        val env = MediaScanEnv(tmp.newFolder())
        env.camera(1, "moves")
        env.scan()
        env.listing.remove(PRIMARY, 1)
        env.listing.put(PRIMARY, 77, "Pictures/Trip/", "IMG_1.jpg", "moves")
        env.reconcile()
        assertEquals(0, env.vanished().size)
        assertEquals("動かした先で同じ記録の更新として送る", 2, env.photos().size)
        assertEquals(env.photos()[0].externalId, env.photos()[1].externalId)
    }

    // Scenario: 写真と動画へのアクセスを外して戻しても消えたと数えない
    @Test
    fun revoking_and_restoring_access_does_not_vanish_anything() {
        val env = MediaScanEnv(tmp.newFolder())
        repeat(10) { env.camera(it + 1L, "p$it") }
        env.scan()
        env.access = MediaAccess.NONE
        env.advanceHours(25)
        assertTrue(env.scan() is MediaScanResult.Unavailable)
        env.access = MediaAccess.FULL
        env.scan()
        assertEquals(0, env.vanished().size)
        assertEquals(10, env.photos().size)
    }

    // Scenario: 一覧が作り直されて番号が振り直されても消えたと数えない
    @Test
    fun a_rebuilt_listing_with_new_numbers_vanishes_nothing() {
        val env = MediaScanEnv(tmp.newFolder())
        repeat(10) { env.camera(it + 1L, "p$it") }
        env.scan()
        env.listing.rebuild(PRIMARY, shift = 500)
        env.advanceHours(1)
        env.scan()
        assertEquals(0, env.vanished().size)
        env.reconcile()
        assertEquals(0, env.vanished().size)
    }

    // Scenario: 名前を変えた写真は消えたと数えない
    @Test
    fun a_renamed_photo_is_not_vanished() {
        val env = MediaScanEnv(tmp.newFolder())
        env.camera(1, "named")
        env.scan()
        env.listing.put(PRIMARY, 1, "DCIM/Camera/", "renamed.jpg", "named")
        env.reconcile()
        assertEquals(0, env.vanished().size)
    }

    @Test
    fun the_same_hash_under_another_number_keeps_the_photo_from_vanishing() {
        val env = MediaScanEnv(tmp.newFolder())
        env.camera(1, "twin")
        env.listing.put(PRIMARY, 2, "Download/", "copy.jpg", "twin")
        env.scan()
        env.listing.remove(PRIMARY, 1)
        env.reconcile()
        assertEquals(0, env.vanished().size)
    }

    @Test
    fun an_incremental_review_never_judges_vanishing() {
        val env = MediaScanEnv(tmp.newFolder())
        env.camera(1, "gone")
        env.scan()
        env.listing.remove(PRIMARY, 1)
        env.advanceHours(6)
        env.scan()
        assertEquals(0, env.vanished().size)
    }
}
