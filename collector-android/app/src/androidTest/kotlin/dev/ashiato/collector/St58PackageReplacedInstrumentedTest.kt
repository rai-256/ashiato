// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.ComponentName
import android.os.ParcelFileDescriptor
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

@RunWith(AndroidJUnit4::class)
class St58PackageReplacedInstrumentedTest {
    @Test fun restartsWithoutOpening() { // Scenario: 携帯は更新の後、開かなくても収集を再開する
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        assertNotNull(
            context.packageManager.getReceiverInfo(ComponentName(context, PackageReplacedReceiver::class.java), 0)
        )
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        val marker = marker(context)
        marker.delete()
        stopRunningService(context)
        clearWaitingNotification(context)
        clearLogs()
        PackageReplacedReceiver().onReceive(context, replacedIntent())
        assertTrue(marker.isFile)
        awaitMarkerConsumed(marker)
        assertTrue("更新後にサービスが再開していない", waitForLog("kind="))
        Config.clearOverrideForTest()
    }

    @Test fun specialUseWaitsForBackgroundLocation() { // Scenario: 携帯は裏の位置の権限が無くても、更新の後に位置以外の収集を再開し、開くよう通知する
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        val marker = marker(context)
        marker.delete()
        stopRunningService(context)
        clearWaitingNotification(context)
        clearLogs()
        PackageReplacedReceiver().onReceive(context, replacedIntent())
        assertTrue(marker.isFile)
        awaitMarkerConsumed(marker)
        assertTrue("specialUse でサービスが立っていない", waitForLog("kind=foreground_special_use"))
        assertTrue("位置の収集停止通知が出ていない", hasNotification(context, LocationService.UPDATE_WAITING_NOTIFICATION_ID))
        Config.clearOverrideForTest()
    }

    @Test fun noLocationStillCollectsOtherSources() { // Scenario: 位置の権限が無い携帯は、更新の後に位置以外の収集を再開し、通知を出さない
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        stopRunningService(context)
        clearWaitingNotification(context)
        clearLogs()
        PackageReplacedReceiver().onReceive(context, replacedIntent())
        assertTrue("位置権限なしでもサービスが再開していない", waitForLog("kind=foreground_special_use"))
        assertFalse("位置権限なしで待機通知を出している", hasNotification(context, LocationService.UPDATE_WAITING_NOTIFICATION_ID))
        assertTrue("位置以外の収集サービスが生きていない", isServiceRunning(context))
        Config.clearOverrideForTest()
    }

    @Test fun noNotificationPermissionStillRestarts() { // Scenario: 通知の権限が無い携帯も、更新の後に位置以外の収集を再開する
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        stopRunningService(context)
        clearWaitingNotification(context)
        clearLogs()
        PackageReplacedReceiver().onReceive(context, replacedIntent())
        assertTrue("通知権限なしでもサービスが再開していない", waitForLog("kind=foreground_special_use"))
        assertNotNull(context.packageManager.getServiceInfo(ComponentName(context, LocationService::class.java), 0))
        assertFalse("通知権限なしで待機通知を出している", hasNotification(context, LocationService.UPDATE_WAITING_NOTIFICATION_ID))
        Config.clearOverrideForTest()
    }

    @Test fun openingPromotesLocation() { // Scenario: 更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        stopRunningService(context)
        clearWaitingNotification(context)
        clearLogs()
        PackageReplacedReceiver().onReceive(context, replacedIntent())
        assertTrue("昇格前の specialUse が観測できない", waitForLog("kind=foreground_special_use"))
        context.startService(
            android.content.Intent(context, LocationService::class.java)
                .putExtra(MainActivity.EXTRA_STARTED_FROM_FOREGROUND, true)
        )
        assertTrue("location への昇格が観測できない", waitForLog("kind=foreground_location"))
        assertFalse("昇格後も更新待機通知が残っている", hasNotification(context, LocationService.UPDATE_WAITING_NOTIFICATION_ID))
        Config.clearOverrideForTest()
    }

    @Test fun noDestinationDoesNotStart() { // Scenario: 送り先の無い携帯は、更新の後に収集を起こさない
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        assertFalse(Config.hasBuildSetup && Config.hasSavedSetup)
        assertFalse(marker(context).isFile)
    }

    @Test fun disabledUpdateStaysDisabled() { // Scenario: 携帯の切った更新の確認は、入れ替えても切ったまま
        val prefs = ApplicationProvider.getApplicationContext<android.content.Context>().getSharedPreferences(
            UpdateCheck.PREFS,
            0
        )
        prefs.edit().putBoolean("enabled", false).commit()
        assertFalse(prefs.getBoolean("enabled", true))
    }

    private fun marker(context: android.content.Context) = File(context.noBackupFilesDir, "update/replaced-start")
    private fun replacedIntent() = android.content.Intent(android.content.Intent.ACTION_MY_PACKAGE_REPLACED)

    private fun stopRunningService(context: android.content.Context) {
        context.stopService(android.content.Intent(context, LocationService::class.java))
        val manager = context.getSystemService(android.app.ActivityManager::class.java)
        val deadline = System.currentTimeMillis() + 10_000

        @Suppress("DEPRECATION")
        val running = {
            manager.getRunningServices(Int.MAX_VALUE).any { it.service.className == LocationService::class.java.name }
        }
        while (running() && System.currentTimeMillis() < deadline) Thread.sleep(100)
    }

    private fun awaitMarkerConsumed(marker: File) {
        val deadline = System.currentTimeMillis() + 10_000
        while (marker.isFile && System.currentTimeMillis() < deadline) Thread.sleep(100)
        assertFalse("起こされたサービスが印を読んでいない", marker.isFile)
    }

    private fun clearLogs() = shell("logcat -c")
    private fun logs(): String = shell("logcat -d -s ashiato:I -t 400")

    private fun waitForLog(needle: String): Boolean {
        val deadline = System.currentTimeMillis() + 10_000
        while (System.currentTimeMillis() < deadline) {
            if (logs().contains(needle)) return true
            Thread.sleep(100)
        }
        return false
    }

    private fun hasNotification(context: android.content.Context, id: Int): Boolean =
        context.getSystemService(android.app.NotificationManager::class.java).activeNotifications.any { it.id == id }

    private fun clearWaitingNotification(context: android.content.Context) {
        context.getSystemService(android.app.NotificationManager::class.java)
            .cancel(LocationService.UPDATE_WAITING_NOTIFICATION_ID)
    }

    private fun isServiceRunning(context: android.content.Context): Boolean {
        @Suppress("DEPRECATION")
        return context.getSystemService(android.app.ActivityManager::class.java)
            .getRunningServices(Int.MAX_VALUE).any { it.service.className == LocationService::class.java.name }
    }

    private fun shell(command: String): String =
        ParcelFileDescriptor.AutoCloseInputStream(
            InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(command),
        ).use { it.readBytes().toString(Charsets.UTF_8) }
}
