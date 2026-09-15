// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import java.io.File
import java.time.Instant
import java.time.ZoneId
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

/**
 * 上限が近づくと端末で知らせる（ST04 / tasks 9.1 / 深掘り Q5 / spec R9 / design D11）。
 * **Robolectric の本物の NotificationManager** を通して、通知の中身と回数を数える。
 */
@RunWith(RobolectricTestRunner::class)
class RetentionNotifierTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val manager: NotificationManager get() = app.getSystemService(NotificationManager::class.java)
    private val st = TestStores()
    private val day = AgeClock.DAY_MS

    private lateinit var service: Service

    private fun req(id: String) =
        LocationFix(35.68, 139.76, 10f, Instant.parse("2026-06-01T00:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    private fun notifier(): RetentionNotifier {
        service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
        shadowOf(app).grantPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        val alerts = AndroidRetentionAlerts(service) { text ->
            android.app.Notification.Builder(service, LocationService.CHANNEL)
                .setContentText(text).setSmallIcon(android.R.drawable.ic_menu_mylocation).build()
        }
        return RetentionNotifier(st.records, st.age::now, alerts, File(st.dir, "retention-alerted"))
    }

    private fun ongoingText(): String? =
        shadowOf(manager).getNotification(LocationService.NOTIFICATION_ID)
            ?.extras?.getCharSequence(android.app.Notification.EXTRA_TEXT)?.toString()

    /** 音の鳴るチャネルに出た通知の数（`notify` が呼ばれた回数）。 */
    private var alertsPosted = 0

    private fun countAlert() {
        if (shadowOf(manager).getNotification(LocationService.RETENTION_NOTIFICATION_ID) != null) {
            alertsPosted++
            manager.cancel(LocationService.RETENTION_NOTIFICATION_ID)
        }
    }

    @After
    fun tearDown() {
        manager.cancelAll()
    }

    // Scenario: 常駐の通知に未送信の日数が出る
    @Test
    fun `積んでから 12 日の記録が最も古いと常駐の通知に 12 日と出る`() {
        val n = notifier()
        st.records.add(req("a"))
        st.clock.advance(12 * day + 5_000)
        n.update()
        assertEquals("位置を記録しています · 未送信 12 日", ongoingText())
    }

    // Scenario: 1 日に満たない未送信では日数が出ない
    @Test
    fun `積んでから 3 時間なら日数は出ない`() {
        val n = notifier()
        st.records.add(req("a"))
        st.clock.advance(3 * 60 * 60 * 1000L)
        n.update()
        assertEquals("位置を記録しています", ongoingText())
    }

    // Scenario: 上限の 7 日前に音の鳴る通知が出る
    // Scenario: 83 日を過ぎても同じ通知は鳴り直さない
    @Test
    fun `83 日で 1 回だけ鳴り、84 日・85 日では鳴り直さない`() {
        val n = notifier()
        st.records.add(req("a"))
        st.clock.advance(82 * day)
        n.update(); countAlert()
        assertEquals("82 日で鳴っている", 0, alertsPosted)
        st.clock.advance(1 * day)
        n.update(); countAlert()
        assertEquals(1, alertsPosted)
        val channel = manager.getNotificationChannel(LocationService.RETENTION_CHANNEL)
        assertEquals("音の鳴るチャネルでない", NotificationManager.IMPORTANCE_DEFAULT, channel.importance)
        st.clock.advance(1 * day)
        n.update(); countAlert()
        st.clock.advance(1 * day)
        n.update(); countAlert()
        assertEquals("鳴り直している", 1, alertsPosted)
    }

    // Scenario: 生存信号だけが古くても鳴らない
    @Test
    fun `未送信の記録が無く生存信号だけが 83 日を超えていても鳴らず日数も出ない`() {
        val n = notifier()
        st.beats.add(HeartbeatRequest("h1", "user-1", LOGICAL_SOURCE, "device-1", "2026-06-01T00:00:00Z", true, emptyList(), 1, 1, "{}"))
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, Instant.parse("2026-03-01T00:00:00Z"))
        st.ledger.freeze()
        st.clock.advance(100 * day)
        n.update(); countAlert()
        assertEquals(0, alertsPosted)
        assertEquals("位置を記録しています", ongoingText())
    }

    // Scenario: 送り切った後の次の長い圏外ではまた鳴る
    @Test
    fun `送り切った後の次の長い圏外ではまた 1 回鳴る`() {
        val n = notifier()
        st.records.add(req("a"))
        st.clock.advance(83 * day)
        n.update(); countAlert()
        st.records.remove(listOf("a"))
        n.update(); countAlert()
        assertEquals("日数が消えていない", "位置を記録しています", ongoingText())
        st.records.add(req("b"))
        st.clock.advance(83 * day)
        n.update(); countAlert()
        assertEquals(2, alertsPosted)
    }

    @Test
    fun `通知の権限が無ければ出さずにログへ残す`() {
        val n = notifier()
        shadowOf(app).denyPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        val lines = mutableListOf<String>()
        val blocked = RetentionNotifier(
            st.records, st.age::now,
            AndroidRetentionAlerts(service) { android.app.Notification.Builder(service, LocationService.CHANNEL).build() },
            File(st.dir, "blocked-mark"), log = { lines += it },
        )
        st.records.add(req("a"))
        st.clock.advance(83 * day)
        blocked.update(); countAlert()
        assertEquals(0, alertsPosted)
        assertTrue(lines.any { it.contains("kind=retention_alert_blocked") })
        assertFalse(lines.any { it.contains("35.68") })
        n.hashCode()
    }
}
