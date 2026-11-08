// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.Manifest
import android.app.Application
import android.app.Notification
import android.app.NotificationManager
import android.content.SharedPreferences
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

@RunWith(RobolectricTestRunner::class)
class St56SetupNotifierTest {
    @get:Rule
    val tmp = TemporaryFolder()
    private val app: Application = ApplicationProvider.getApplicationContext()
    private lateinit var prefs: SharedPreferences
    private var now = 0L
    private lateinit var notifier: SetupNotifier

    @Before
    fun setUp() {
        prefs = app.getSharedPreferences(Config.SETUP_PREFS, 0)
        shadowOf(app).grantPermissions(Manifest.permission.POST_NOTIFICATIONS)
        prefs.edit().clear().commit()
        app.getSystemService(NotificationManager::class.java).cancelAll()
        notifier = SetupNotifier(app, prefs, { now })
        notifier.setupSaved()
    }

    private fun unauthorized() = Outcome.Responded(401, "")
    private fun ok() = Outcome.Responded(200, "")

    // Scenario: 401 が 1 時間続くと知らせる
    @Test fun `401 が 1 時間続くと知らせる`() {
        notifier.observe(unauthorized())
        now = 60 * 60 * 1000L + 1
        notifier.update()
        val n = shadowOf(app.getSystemService(NotificationManager::class.java))
            .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID)
        assertEquals("サーバに断られています。資格情報か送り先が違います。設定し直してください", n.extras.getCharSequence(Notification.EXTRA_TEXT))
    }

    // Scenario: 断られ続ける間は 24 時間ごとに出し直す
    @Test fun `断られ続ける間は 24 時間ごとに出し直す`() {
        notifier.observe(unauthorized())
        now = 60 * 60 * 1000L + 1
        notifier.update()
        app.getSystemService(NotificationManager::class.java).cancel(SetupNotifier.AUTH_NOTIFICATION_ID)
        now += 24 * 60 * 60 * 1000L - 60 * 1000L
        notifier.observe(unauthorized())
        notifier.update()
        assertNull(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID)
        )
        now += 2 * 60 * 1000L
        notifier.observe(unauthorized())
        notifier.update()
        assertTrue(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID) != null
        )
    }

    // Scenario: 401 で断られた記録は取り除かない
    @Test fun `401 で断られた記録は取り除かない`() {
        val outbox = testOutbox(tmp.newFolder())
        repeat(5) {
            outbox.add(
                testFix(35.68, 139.76, 10f, java.time.Instant.parse("2026-10-01T00:00:00Z"))
                    .toIngestRequest("record-$it", "user-1", "device-1", java.time.ZoneId.of("Asia/Tokyo"))
            )
        }
        val sender = Sender(
            outbox,
            Transport { unauthorized() },
            IngestRequest.serializer(),
            dropPermanentlyRejected = true,
            onOutcome = notifier::observe
        )
        repeat(3) { sender.flush() }
        assertEquals(5, outbox.size())
    }

    // Scenario: 一度も届かない送り先は 24 時間で知らせる
    @Test fun `一度も届かない送り先は 24 時間で知らせる`() {
        now = 24 * 60 * 60 * 1000L - 60 * 1000L
        notifier.update()
        assertNull(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.UNREACHABLE_NOTIFICATION_ID)
        )
        now = 24 * 60 * 60 * 1000L + 1
        notifier.update()
        val notification = shadowOf(app.getSystemService(NotificationManager::class.java))
            .getNotification(SetupNotifier.UNREACHABLE_NOTIFICATION_ID)
        assertEquals("送り先に届きません。設定を確かめてください", notification.extras.getCharSequence(Notification.EXTRA_TEXT))
        assertEquals(
            SetupActivity::class.java.name,
            shadowOf(notification.contentIntent).savedIntent.component?.className
        )
    }

    // Scenario: 断られる状態は再起動をまたいで数える
    @Test fun `断られる状態は再起動をまたいで数える`() {
        notifier.observe(unauthorized())
        now = 60 * 60 * 1000L + 1
        SetupNotifier(app, prefs, { now }).update()
        assertTrue(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID) != null
        )
    }

    // Scenario: 届けば数えをやめて通知を消す
    @Test fun `届けば数えをやめて通知を消す`() {
        notifier.observe(unauthorized())
        now = 60 * 60 * 1000L + 1
        notifier.update()
        notifier.observe(ok())
        assertNull(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID)
        )
        now += 30 * 60 * 1000L
        notifier.observe(unauthorized())
        notifier.update()
        assertNull(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID)
        )
    }

    // Scenario: 401 だけを返す送り先は「届かない」と知らせない
    @Test fun `401 だけを返す送り先は「届かない」と知らせない`() {
        notifier.observe(unauthorized())
        now = 24 * 60 * 60 * 1000L + 1
        notifier.update()
        assertNull(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.UNREACHABLE_NOTIFICATION_ID)
        )
    }

    // Scenario: 断られている間に 5xx が挟まっても数え直さない
    @Test fun `断られている間に 5xx が挟まっても数え直さない`() {
        notifier.observe(unauthorized())
        now = 30 * 60 * 1000L
        notifier.observe(Outcome.Responded(503, ""))
        now = 60 * 60 * 1000L + 1
        notifier.observe(unauthorized())
        notifier.update()
        assertTrue(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID) != null
        )
    }

    @Test fun `5xx だけが続いても断られ続ける知らせを出さない`() {
        repeat(3) {
            notifier.observe(Outcome.Responded(503, ""))
            now += 60 * 60 * 1000L
        }
        notifier.update()
        assertNull(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID)
        )
    }

    @Test fun `設定時刻が無くても401の数えは通知する`() {
        prefs.edit().remove(SetupNotifier.SETUP_AT_KEY).apply()
        notifier.observe(unauthorized())
        now = 60 * 60 * 1000L + 1
        notifier.update()
        assertTrue(
            shadowOf(app.getSystemService(NotificationManager::class.java))
                .getNotification(SetupNotifier.AUTH_NOTIFICATION_ID) != null
        )
    }
}
