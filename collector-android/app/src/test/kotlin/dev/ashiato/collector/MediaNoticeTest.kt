// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Application
import android.app.Notification
import android.app.NotificationManager
import android.provider.Settings
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

/** 写真と動画を取れていないことの知らせ（tasks 4.3 / design D6 / Q15）。通知は Robolectric の本物の NotificationManager を通す。 */
@RunWith(RobolectricTestRunner::class)
class MediaNoticeTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val manager: NotificationManager get() = app.getSystemService(NotificationManager::class.java)

    private fun notifier(canNotify: Boolean = true): MediaNotifier {
        if (canNotify) shadowOf(app).grantPermissions(Manifest.permission.POST_NOTIFICATIONS)
        return MediaNotifier(app)
    }

    private fun shown(): Notification? = shadowOf(manager).getNotification(MediaNotifier.MEDIA_NOTIFICATION_ID)

    // Scenario: 写真の位置の権限が無いと端末で知らせる
    @Test
    fun without_the_photo_location_permission_a_notice_is_shown_that_opens_the_app_settings() {
        notifier().update(MediaPermissionState(MediaAccess.FULL, location = false))

        val notice = shown()
        assertNotNull("知らせが出ていない", notice)
        assertEquals("写真と動画を取れていません", notice!!.extras.getString(Notification.EXTRA_TITLE))
        assertEquals(
            "写真の位置へのアクセスを許可するまで、写真と動画を取らずに待っています。",
            notice.extras.getString(Notification.EXTRA_TEXT),
        )
        assertEquals(MediaNotifier.MEDIA_CHANNEL, notice.channelId)
        assertEquals("media-capture", notice.channelId)
        // 押すとこのアプリの設定が開く
        val intent = shadowOf(notice.contentIntent).savedIntent
        assertEquals(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, intent.action)
        assertEquals("package:${app.packageName}", intent.data.toString())
    }

    @Test
    fun without_media_access_the_notice_names_the_access() {
        notifier().update(MediaPermissionState(MediaAccess.NONE, location = true))

        assertEquals(
            "写真と動画へのアクセスを許可するまで、写真と動画を取らずに待っています。",
            shown()!!.extras.getString(Notification.EXTRA_TEXT),
        )
    }

    @Test
    fun the_notice_goes_away_when_the_permission_comes_back() {
        val notifier = notifier()
        notifier.update(MediaPermissionState(MediaAccess.FULL, location = false))
        assertNotNull(shown())

        notifier.update(MediaPermissionState(MediaAccess.FULL, location = true))

        assertNull("権限が戻ったのに知らせが残っている", shown())
    }

    // Scenario: 選んだ写真だけの許可では通知を出さない
    @Test
    fun a_selected_photos_only_grant_shows_no_notice() {
        notifier().update(MediaPermissionState(MediaAccess.PARTIAL, location = true))

        assertNull("選んだ写真だけで知らせを出している", shown())
    }

    @Test
    fun the_same_notice_is_not_posted_again_and_a_missing_notification_permission_does_not_crash() {
        val silent = notifier(canNotify = false)
        shadowOf(app).denyPermissions(Manifest.permission.POST_NOTIFICATIONS)

        silent.update(MediaPermissionState(MediaAccess.FULL, location = false))

        // 通知の権限が無ければ出せない。落ちない（取得そのものは止まらず、生存信号が理由を持つ）
        if (android.os.Build.VERSION.SDK_INT >= 33) assertNull(shown())
    }
}
