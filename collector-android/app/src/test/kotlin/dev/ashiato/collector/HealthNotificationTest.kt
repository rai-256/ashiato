// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config as RoboConfig

/** 健康データの口を差し、読みが例外を投げる状態も作れる本番の `LocationService`（Robolectric が無引数で作るので、差し替えは companion に置く）。 */
class HealthTestService : TestableLocationService() {
    override fun healthPermissions(): HealthPermissions = object : HealthPermissions by health {
        override suspend fun read(context: android.content.Context): HealthReading {
            if (readThrows) throw IllegalStateException("boom")
            return health.read(context)
        }
    }

    companion object {
        var health: FakeHealthPermissions = FakeHealthPermissions.available()
        var readThrows = false
    }
}

/** 常駐の通知の健康データの権限の操作と、6 時間ごとの見直し（ST11 / design D8 / D9）。 */
@RunWith(RobolectricTestRunner::class)
@RoboConfig(sdk = [34])
class HealthNotificationTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun start(health: FakeHealthPermissions, readThrows: Boolean = false): HealthTestService {
        HealthTestService.health = health
        HealthTestService.readThrows = readThrows
        val controller = Robolectric.buildService(HealthTestService::class.java, Intent()).create()
        controller.startCommand(0, 1)
        return controller.get()
    }

    private fun posted(): Notification =
        shadowOf(app.getSystemService(NotificationManager::class.java)).getNotification(LocationService.NOTIFICATION_ID)

    private fun healthAction(n: Notification) = n.actions?.singleOrNull { it.title == HEALTH_PERMISSION_ACTION_LABEL }

    private fun healthRecords(s: TestableLocationService) =
        s.outboxForTest.snapshot().filter { it.logicalSource == HEALTH_LOGICAL_SOURCE }

    // Scenario: 常駐の通知から健康データの権限の画面が開く
    @Test
    fun `使える端末の通知の操作は収集アプリの権限の画面を開く intent を持つ`() {
        start(FakeHealthPermissions.available())

        val action = healthAction(posted())
        assertNotNull("操作が出ていない", action)
        val intent = shadowOf(action!!.actionIntent).savedIntent
        assertEquals("android.health.connect.action.MANAGE_HEALTH_PERMISSIONS", intent.action)
        assertEquals(app.packageName, intent.getStringExtra(Intent.EXTRA_PACKAGE_NAME))
        assertTrue(shadowOf(action.actionIntent).isActivity)
        // 本文の押下（利用状況へのアクセス）は変えない
        assertEquals(android.provider.Settings.ACTION_USAGE_ACCESS_SETTINGS, shadowOf(posted().contentIntent).savedIntent.action)
        assertEquals(PendingIntent.FLAG_IMMUTABLE, shadowOf(action.actionIntent).flags and PendingIntent.FLAG_IMMUTABLE)
    }

    // Scenario: Health Connect が使えない端末の通知には健康データの権限の操作が出ない
    @Test
    fun `使えない端末の通知には操作が出ない`() {
        for (availability in listOf(HealthAvailability.NOT_INSTALLED, HealthAvailability.UPDATE_REQUIRED, HealthAvailability.UNREADABLE)) {
            start(FakeHealthPermissions.unavailable(availability, errorType = "x.Err"))
            assertNull("$availability で操作が出ている", healthAction(posted()))
        }
    }

    // Scenario: 後から設定で外すと次の見直しで記録される
    @Test
    fun `後から履歴の読み取りを外すと 6 時間ごとの見直しで 1 件が未送信に積まれる`() {
        val health = FakeHealthPermissions.available().also { it.allowAll() }
        val s = start(health)
        assertEquals(6 * 3_600_000L, s.beatScheduler.periodMs)
        s.beatScheduler.fire() // 全部許された状態を最初に記録する
        val before = healthRecords(s).size

        health.reading = health.reading.copy(
            grantedPermissions = health.reading.grantedPermissions - HEALTH_PERMISSIONS.getValue("history"),
        )
        s.beatScheduler.fire()

        val added = healthRecords(s).drop(before)
        assertEquals(1, added.size)
        assertEquals(HealthTrigger.PERIODIC, added.single().payload["trigger"]!!.jsonPrimitive.content)
        assertEquals("false", (added.single().payload["granted"] as kotlinx.serialization.json.JsonObject)["history"]!!.jsonPrimitive.content)
        // 生存信号は送らない
        assertTrue(s.heartbeatOutboxForTest.snapshot().none { it.logicalSource == HEALTH_LOGICAL_SOURCE })
    }

    @Test
    fun `見直しが例外を投げても位置の記録は契機ごとに生成され続ける`() {
        val s = start(FakeHealthPermissions.available(), readThrows = true)

        s.beatScheduler.fire() // 例外が外へ出ない
        s.source.deliver()
        s.beatScheduler.fire()
        s.source.deliver()

        assertEquals(2, s.outboxForTest.snapshot().count { it.logicalSource == LOGICAL_SOURCE })
    }
}
