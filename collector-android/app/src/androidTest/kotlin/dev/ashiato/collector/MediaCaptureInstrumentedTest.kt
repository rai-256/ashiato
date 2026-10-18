// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.util.regex.Pattern

/**
 * 写真と動画の権限の求め方と、取れていないことの知らせを**本物の端末（エミュレータ）**で確かめる（ST09 tasks 4.1 / 4.3）。
 * 権限を求める画面は、未許可から始める同じ流れの `PermissionDeniedInstrumentedTest` が見る。
 * 端から端（写真を MediaStore に入れて S-01 に届く）は Task 10.2 が同じクラスへ足す。
 */
@RunWith(AndroidJUnit4::class)
class MediaCaptureInstrumentedTest {
    @get:Rule
    val notifications: GrantPermissionRule = GrantPermissionRule.grant(Manifest.permission.POST_NOTIFICATIONS)

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    // Scenario: 写真の位置の権限が無いと端末で知らせる
    @Test
    fun a_notice_is_posted_when_the_photo_location_permission_is_missing_and_opens_the_app_settings() {
        val notifier = MediaNotifier(context)

        notifier.update(MediaPermissionState(MediaAccess.FULL, location = false))

        val notice = manager.activeNotifications.firstOrNull { it.id == MediaNotifier.MEDIA_NOTIFICATION_ID }
        assertNotNull("知らせが出ていない", notice)
        assertEquals(MediaNotifier.MEDIA_CHANNEL, notice!!.notification.channelId)
        assertEquals(MEDIA_NOTICE_TITLE, notice.notification.extras.getString(Notification.EXTRA_TITLE))
        // 押すとこのアプリの設定が開く: 本物の PendingIntent を送り、設定アプリが前面に出ることを見る
        notice.notification.contentIntent.send()
        val opened = device.wait(Until.hasObject(By.pkg(Pattern.compile(".*settings.*"))), DIALOG_TIMEOUT_MS)
        assertTrue("知らせを押してもアプリの設定が開かない（前面: ${device.currentPackageName}）", opened == true)
        device.pressBack()

        // 権限が戻ったら消える
        notifier.update(MediaPermissionState(MediaAccess.FULL, location = true))
        assertNull(
            "権限が戻ったのに知らせが残っている",
            manager.activeNotifications.firstOrNull { it.id == MediaNotifier.MEDIA_NOTIFICATION_ID },
        )
    }

    @After
    fun cleanUp() {
        manager.cancel(MediaNotifier.MEDIA_NOTIFICATION_ID)
        context.stopService(Intent(context, LocationService::class.java))
    }

    private companion object {
        const val DIALOG_TIMEOUT_MS = 15_000L
    }
}
