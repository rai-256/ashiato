// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.Notification
import android.app.NotificationManager
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.time.Instant

/** 携帯の通知（ST14 / design D12・D14（仮））。Robolectric の本物の NotificationManager を通す。 */
@RunWith(RobolectricTestRunner::class)
class OutageNotifierTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val manager: NotificationManager get() = app.getSystemService(NotificationManager::class.java)
    private val logs = mutableListOf<String>()
    private val notifier get() = OutageNotifier(app) { logs += it }

    private val location = Outage(1, "c01-location", "位置", Instant.parse("2026-10-11T23:15:00Z"))
    private val photo = Outage(2, "c01-photo", "写真", Instant.parse("2026-10-11T13:00:00Z"))

    private fun posted(): Notification? = shadowOf(manager).getNotification(OutageNotifier.OUTAGE_NOTIFICATION_ID)

    private fun text(n: Notification, key: String) = n.extras.getCharSequence(key)?.toString()

    private fun silent(n: Notification) = n.groupAlertBehavior == Notification.GROUP_ALERT_SUMMARY

    private fun grant() = shadowOf(app).grantPermissions(android.Manifest.permission.POST_NOTIFICATIONS)

    @Test
    fun `携帯の途切れの通知は専用のチャネルで出る`() {
        // Scenario: 携帯の途切れの通知は専用のチャネルで出る
        grant()
        notifier.show(listOf(location), sound = true)

        val n = posted()!!
        assertEquals("outage", n.channelId)
        assertNotEquals(LocationService.CHANNEL, n.channelId)
        assertNotEquals(LocationService.RETENTION_CHANNEL, n.channelId)
        assertEquals(NotificationManager.IMPORTANCE_DEFAULT, manager.getNotificationChannel("outage").importance)
        assertEquals(3, OutageNotifier.OUTAGE_NOTIFICATION_ID)
    }

    @Test
    fun `途切れの通知に記録の中身が載らない`() {
        // Scenario: 途切れの通知に記録の中身が載らない
        grant()
        notifier.show(listOf(location, photo), sound = true)

        val n = posted()!!
        assertEquals("収集が途切れています", text(n, Notification.EXTRA_TITLE))
        // 表示名と最後に生きていた日時（Asia/Tokyo の MM/DD HH:MM）と定まった文言だけ
        assertEquals("位置（最後 10/12 08:15）、写真（最後 10/11 22:00）", text(n, Notification.EXTRA_TEXT))
        assertNull(n.contentIntent)
    }

    @Test
    fun `新しい途切れで音つきの通知が 1 通出る`() {
        // Scenario: 新しい途切れで音つきの通知が 1 通出る
        grant()
        assertTrue(notifier.show(listOf(location), sound = true))

        assertEquals(1, shadowOf(manager).size())
        assertFalse("音つき", silent(posted()!!))
    }

    @Test
    fun `音なしの差し替えは setSilent で出る`() {
        grant()
        notifier.show(listOf(location, photo), sound = true)
        notifier.show(listOf(photo), sound = false)

        assertEquals(1, shadowOf(manager).size())
        assertTrue("音なし", silent(posted()!!))
        assertTrue(text(posted()!!, Notification.EXTRA_TEXT)!!.startsWith("写真"))
    }

    @Test
    fun `戻ると通知が黙って消える`() {
        // Scenario: 戻ると通知が黙って消える
        grant()
        notifier.show(listOf(location), sound = true)
        notifier.dismiss()

        assertNull(posted())
        assertEquals(0, shadowOf(manager).size())
    }

    @Test
    fun `通知を出せなくても収集は止まらない`() {
        // Scenario: 通知を出せなくても収集は止まらない
        shadowOf(app).denyPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        val running = org.robolectric.Robolectric
            .buildService(TestableLocationService::class.java, android.content.Intent())
            .create().startCommand(0, 1).get()
        running.outageReply = readyBody()
        val before = running.heartbeatOutboxForTest.snapshot().size
        running.beatScheduler.fire()

        assertNull("通知は出ない", posted())
        assertEquals(0, shadowOf(manager).allNotifications.count { it.channelId == OutageNotifier.CHANNEL })
        val expected = Telemetry.line("outage_notice_skipped", source = null)
        val skipped = org.robolectric.shadows.ShadowLog.getLogs().filter { it.msg == expected }
        assertEquals("通知を諦めたことがログに 1 行", 1, skipped.size)
        assertTrue("生存信号の送信の箱に記録が残る", running.heartbeatOutboxForTest.snapshot().size > before)
    }

    private fun readyBody() = Outcome.Responded(200, OutageFixtures.READY_LOCATION)
}
