// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.ComponentName
import android.content.pm.ServiceInfo
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
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
        val info = context.packageManager.getReceiverInfo(ComponentName(context, PackageReplacedReceiver::class.java), 0)
        assertNotNull(info)
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        val marker = File(context.noBackupFilesDir, "update/replaced-start")
        marker.delete()
        stopRunningService(context)
        PackageReplacedReceiver().onReceive(context, android.content.Intent(android.content.Intent.ACTION_MY_PACKAGE_REPLACED))
        assertTrue(marker.isFile)
        awaitMarkerConsumed(marker)
        Config.clearOverrideForTest()
    }
    @Test fun specialUseWaitsForBackgroundLocation() { // Scenario: 携帯は裏の位置の権限が無くても、更新の後に位置以外の収集を再開し、開くよう通知する
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        val marker = File(context.noBackupFilesDir, "update/replaced-start")
        marker.delete()
        stopRunningService(context)
        PackageReplacedReceiver().onReceive(context, android.content.Intent(android.content.Intent.ACTION_MY_PACKAGE_REPLACED))
        assertTrue(marker.isFile)
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, foregroundServiceTypeFor(true, false, true, false))
        val location = replacementLocationCapability(true, true, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, true, false, Capability.of(true, true, true))
        assertFalse(location.capturable)
        assertEquals(listOf(Capability.PERMISSION), location.blockers)
        awaitMarkerConsumed(marker)
        Config.clearOverrideForTest()
        assertTrue(LocationService.UPDATE_WAITING_NOTIFICATION_ID != LocationService.NOTIFICATION_ID)
    }
    @Test fun noLocationStillCollectsOtherSources() { // Scenario: 位置の権限が無い携帯は、更新の後に位置以外の収集を再開し、通知を出さない
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        stopRunningService(context)
        PackageReplacedReceiver().onReceive(context, android.content.Intent(android.content.Intent.ACTION_MY_PACKAGE_REPLACED))
        val location = replacementLocationCapability(true, true, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, true, false, Capability.of(true, true, true))
        val other = replacementLocationCapability(true, false, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, true, false, Capability.of(true, true, true))
        assertFalse(location.capturable)
        assertTrue(other.capturable)
        assertEquals(listOf(Capability.PERMISSION), location.blockers)
    }
    @Test fun noNotificationPermissionStillRestarts() { // Scenario: 通知の権限が無い携帯も、更新の後に位置以外の収集を再開する
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val manager = context.getSystemService(android.app.NotificationManager::class.java)
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        stopRunningService(context)
        PackageReplacedReceiver().onReceive(context, android.content.Intent(android.content.Intent.ACTION_MY_PACKAGE_REPLACED))
        assertNotNull(context.packageManager.getServiceInfo(ComponentName(context, LocationService::class.java), 0))
        assertNotNull(manager.getNotificationChannel(UpdateCheck.CHANNEL))
        assertEquals("更新", manager.getNotificationChannel(UpdateCheck.CHANNEL)?.name)
    }
    @Test fun openingPromotesLocation() { // Scenario: 更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        Config.overrideForTest("http://127.0.0.1", "token", "user")
        stopRunningService(context)
        PackageReplacedReceiver().onReceive(context, android.content.Intent(android.content.Intent.ACTION_MY_PACKAGE_REPLACED))
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION, foregroundServiceTypeFor(true, true, true, false))
        assertTrue(LocationService.UPDATE_WAITING_NOTIFICATION_ID > 0)
    }
    @Test fun noDestinationDoesNotStart() { // Scenario: 送り先の無い携帯は、更新の後に収集を起こさない
        assertFalse(Config.hasBuildSetup && Config.hasSavedSetup)
        assertFalse(File(ApplicationProvider.getApplicationContext<android.content.Context>().noBackupFilesDir, "update/replaced-start").isFile)
    }
    @Test fun disabledUpdateStaysDisabled() { // Scenario: 携帯の切った更新の確認は、入れ替えても切ったまま
        val prefs = ApplicationProvider.getApplicationContext<android.content.Context>().getSharedPreferences(UpdateCheck.PREFS, 0)
        prefs.edit().putBoolean("enabled", false).commit()
        assertFalse(prefs.getBoolean("enabled", true))
    }

    /**
     * 受け手が起こすサービスの `onCreate` を必ず走らせるため、動いているサービスを止め、止まるまで待つ。
     * 台本の `adb shell am stopservice` は、外へ公開していないサービスなので「Permission Denial」で効かない
     * （2026-11-16 の実測）。動いたままだと受け手の `startForegroundService` は `onStartCommand` だけを呼び、
     * 種別の行（`foreground_special_use` など）が新しく出ない。同じプロセスのこの試験からなら止められる。
     */
    private fun stopRunningService(context: android.content.Context) {
        context.stopService(android.content.Intent(context, LocationService::class.java))
        val manager = context.getSystemService(android.app.ActivityManager::class.java)
        val name = LocationService::class.java.name
        val deadline = System.currentTimeMillis() + 10_000
        @Suppress("DEPRECATION")
        while (manager.getRunningServices(Int.MAX_VALUE).any { it.service.className == name } &&
            System.currentTimeMillis() < deadline
        ) {
            Thread.sleep(100)
        }
    }

    /**
     * 受け手が置いた印を、起こされたサービスの `onCreate` が読んで消すまで待つ。試験の側で消すと、
     * 主スレッドで後から走る `onCreate` より先に消えて種別が `location` に戻る（2026-11-16 の実測）。
     * 消えなければ、受け手が起こしたサービスが印を読んでいない。
     */
    private fun awaitMarkerConsumed(marker: File) {
        val deadline = System.currentTimeMillis() + 10_000
        while (marker.isFile && System.currentTimeMillis() < deadline) Thread.sleep(100)
        assertFalse("起こされたサービスが印を読んでいない", marker.isFile)
    }
}
