// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.NotificationManager
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

/** 携帯の途切れの問い合わせ（ST14 / design D12）。契機は位置の生存信号だけ。 */
@RunWith(RobolectricTestRunner::class)
class OutagePollerTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val manager: NotificationManager get() = app.getSystemService(NotificationManager::class.java)

    private val ready = Outcome.Responded(200, OutageFixtures.READY_LOCATION)

    /** 常駐の通知（ID 1）は数えない */
    private fun outageNotifications() =
        shadowOf(manager).allNotifications.count { it.channelId == OutageNotifier.CHANNEL }

    private fun start(): TestableLocationService {
        shadowOf(app).grantPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        return Robolectric.buildService(TestableLocationService::class.java, Intent())
            .create().startCommand(0, 1).get()
    }

    @Test
    fun `携帯は生存信号の契機で問い合わせる`() {
        // Scenario: 携帯は生存信号の契機で問い合わせる
        val service = start()
        val afterStart = service.outageQueries.size
        assertEquals("起動時の 1 発の後に 1 回", 1, afterStart)

        service.beatScheduler.fire()

        assertEquals("契機ごとに 1 回", afterStart + 1, service.outageQueries.size)
        assertEquals("同じ問い合わせを打つ", setOf(service.outageQueries.first()), service.outageQueries.toSet())
        assert(service.outageQueries.first().isNotBlank())
    }

    @Test
    fun `ソースが複数あっても携帯の問い合わせは契機ごとに 1 回`() {
        // Scenario: ソースが複数あっても携帯の問い合わせは契機ごとに 1 回
        val service = start()
        val before = service.outageQueries.size

        // 位置・アプリ利用・写真の生存信号の契機が同じ時刻に重なる
        assert(service.beatSchedulers.size >= 2)
        service.beatSchedulers.values.forEach { it.fire() }

        assertEquals(before + 1, service.outageQueries.size)
    }

    @Test
    fun `問い合わせの答えが通知まで届く`() {
        val service = start()
        service.outageReply = ready
        service.beatScheduler.fire()

        assertNotNull(shadowOf(manager).getNotification(OutageNotifier.OUTAGE_NOTIFICATION_ID))
        assertEquals(1, outageNotifications())

        // 戻ると黙って消える
        service.outageReply = Outcome.Responded(200, OutageFixtures.READY_EMPTY)
        service.beatScheduler.fire()
        assertNull(shadowOf(manager).getNotification(OutageNotifier.OUTAGE_NOTIFICATION_ID))
    }

    @Test
    fun `読めない答えは届かなかった扱いで通知は変わらない`() {
        val service = start()
        service.outageReply = ready
        service.beatScheduler.fire()

        service.outageReply = Outcome.Responded(200, "not json")
        service.beatScheduler.fire()
        service.outageReply = Outcome.Responded(500, "")
        service.beatScheduler.fire()

        assertNotNull(shadowOf(manager).getNotification(OutageNotifier.OUTAGE_NOTIFICATION_ID))
    }

    @Test
    fun `問い合わせは端末と自分の利用者を付ける`() {
        // review/code.md R13 / design D29（仮）
        val q = outageQueryString("dev a", "6f1c2a4e-0b7d-4e8a-9c3f-1d2e3f4a5b6c")
        assertEquals("?device_id=dev+a&user_id=6f1c2a4e-0b7d-4e8a-9c3f-1d2e3f4a5b6c", q)
    }

    @Test
    fun `収集アプリが打つ問い合わせに設定の利用者が付く`() {
        // 関数だけでなく LocationService の組み立てを通す（review/code.md R17）
        val user = "6f1c2a4e-0b7d-4e8a-9c3f-1d2e3f4a5b6c"
        Config.overrideForTest(baseUrl = "http://127.0.0.1:1", apiToken = "t", userId = user)
        try {
            val query = start().outageQueries.first()
            assertTrue("端末識別子が付く: $query", query.startsWith("?device_id=") && !query.startsWith("?device_id=&"))
            assertTrue("設定の利用者が付く: $query", query.endsWith("&user_id=$user"))
        } finally {
            Config.clearOverrideForTest()
        }
    }

    @Test
    fun `途切れの通知の経路は AlarmManager も WorkManager も使わない`() {
        // 途切れの通知の経路（Outage*.kt と、問い合わせを配線する LocationService）を見る（review/code.md R14）。
        // 本体全体ではない: ST17 の催促は AlarmManager で起きる（design D「ST17 の AlarmManager と重ならない」）
        val main = java.io.File("src/main/kotlin/dev/ashiato/collector")
        assertTrue("本体のソースが見つからない: ${main.absolutePath}", main.isDirectory)
        val path = main.listFiles().orEmpty().filter { it.isFile && (it.name.startsWith("Outage") || it.name == "LocationService.kt") }
        assertTrue("途切れの経路のソースが見つからない: $path", path.size >= 4)
        val hits = path.filter { f -> f.readText().let { "AlarmManager" in it || "WorkManager" in it } }
            .map { it.path }.toList()
        assertEquals(emptyList<String>(), hits)
    }

    @Test
    fun `問い合わせの口は AlarmManager も WorkManager も使わない`() {
        // 本体の確かめは verify の grep。ここは位置以外のソースが問い合わせを持たないことだけ見る
        val service = start()
        val before = service.outageQueries.size
        service.beatSchedulers.filterKeys { it != SourceCadence.LOCATION.logicalSource }.values.forEach { it.fire() }
        assertEquals(before, service.outageQueries.size)
    }
}
