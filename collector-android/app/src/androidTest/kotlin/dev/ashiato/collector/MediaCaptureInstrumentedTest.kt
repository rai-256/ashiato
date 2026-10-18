// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.regex.Pattern

/**
 * 写真と動画の権限の求め方と、取れていないことの知らせを**本物の端末（エミュレータ）**で確かめる（ST09 tasks 4.1 / 4.3）。
 * 端から端（写真を MediaStore に入れて S-01 に届く）は Task 10.2 が同じクラスへ足す。
 */
@RunWith(AndroidJUnit4::class)
class MediaCaptureInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    /** いま出ているシステムの権限ダイアログの本文。無ければ null。 */
    private fun dialogMessage(): String? = device.wait(
        Until.findObject(By.res(Pattern.compile(".*:id/permission_message"))),
        DIALOG_TIMEOUT_MS,
    )?.text

    private fun pressDeny() {
        val deny = device.wait(
            Until.findObject(By.res(Pattern.compile(".*:id/permission_deny_button"))),
            DIALOG_TIMEOUT_MS,
        ) ?: device.wait(
            Until.findObject(By.text(Pattern.compile("(?i)(許可しない|拒否|don't allow|deny)"))),
            DIALOG_TIMEOUT_MS,
        )
        assertNotNull("権限ダイアログの拒否ボタンが見つからない（前面: ${device.currentPackageName}）", deny)
        deny!!.click()
    }

    // Scenario: 初回の起動で写真と動画と写真の位置の権限を求める
    //
    // 「未許可・未要求」から始める（CI の 2 段目で走る。`NeedsPristinePermissions`）。
    @Test
    @NeedsPristinePermissions
    fun the_first_launch_asks_for_photos_and_videos_after_location() {
        assertEquals(
            "前提が作れていない。権限が残っている（tools/android-emulator.sh の 2 段目で走らせる）",
            PackageManager.PERMISSION_DENIED,
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION),
        )
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

        // 位置のダイアログ（および通知）を断りながら、写真と動画のダイアログが出るまで進む
        val seen = mutableListOf<String>()
        var mediaDialog = false
        repeat(MAX_DIALOGS) {
            if (mediaDialog) return@repeat
            val message = dialogMessage() ?: return@repeat
            seen += message
            if (MEDIA_WORDS.containsMatchIn(message)) {
                mediaDialog = true
            } else {
                pressDeny()
            }
        }
        assertTrue("位置の後に写真と動画の権限を求める画面が出ない。出たもの: $seen", mediaDialog)
        assertTrue(
            "写真と動画より先に位置を求めていない。出たもの: $seen",
            seen.size > 1 || Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU,
        )
        pressDeny()
    }

    // Scenario: 写真の位置の権限が無いと端末で知らせる
    @Test
    fun a_notice_is_posted_when_the_photo_location_permission_is_missing_and_opens_the_app_settings() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            instrumentation.uiAutomation.grantRuntimePermission(
                context.packageName,
                Manifest.permission.POST_NOTIFICATIONS,
            )
        }
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
        const val MAX_DIALOGS = 6

        /** 写真と動画のダイアログの本文に出る語（言語差を許す） */
        val MEDIA_WORDS = Regex("(photo|video|media|写真|動画|画像|メディア)", RegexOption.IGNORE_CASE)
    }
}
