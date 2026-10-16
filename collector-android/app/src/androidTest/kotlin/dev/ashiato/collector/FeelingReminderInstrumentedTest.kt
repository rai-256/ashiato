// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Activity
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
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
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.FileInputStream
import java.time.Instant
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/**
 * 通知を押して開かれたアドレスを受けるだけの画面（`androidTest/AndroidManifest.xml`）。
 * テスト用 APK は収集アプリとは別のプロセスなので、受けたアドレスは logcat に 1 行残して試験が読む。
 */
class FeelProbeActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.i(PROBE_TAG, "opened ${intent.dataString.orEmpty()}")
        finish()
    }

    companion object {
        const val PROBE_TAG = "FeelProbe"
    }
}

/**
 * 携帯の通知を**本物の AlarmManager / NotificationManager / PendingIntent** で通す（ST17 tasks 11.3 / design D10）。
 * 差し替えるのはサーバの答えと時計だけ。通知を押したときに開かれるアドレスは、受け皿の画面（[FeelProbeActivity]）で受けて読む。
 */
@RunWith(AndroidJUnit4::class)
class FeelingReminderInstrumentedTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(Manifest.permission.POST_NOTIFICATIONS)

    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)
    private val tokyo = ZoneId.of("Asia/Tokyo")
    private val day = LocalDate.of(2026, 10, 7)

    private var now: Instant = at(21, 5)
    private var recorded = false
    private var finished = CountDownLatch(1)
    private val reminderAlarms = CopyOnWriteArrayList<Instant>()

    private fun at(h: Int, m: Int = 0): Instant = day.atTime(h, m).atZone(tokyo).toInstant()

    private fun shell(cmd: String): String =
        InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(cmd)
            .let { FileInputStream(it.fileDescriptor) }
            .use { it.readBytes().toString(Charsets.UTF_8) }

    @Before
    fun setUp() {
        Config.overrideWebUrlForTest("ashiato-feel-test://web")
        manager.cancelAll()
        shell("logcat -c")
        context.getSharedPreferences("feeling_reminder", Context.MODE_PRIVATE).edit().clear().commit()
        FeelingReminderWiring.runnerForTest = { ctx ->
            val real = FeelingReminderScheduler(ctx)
            FeelingReminderRunner(
                source = { ReminderAnswer(LocalTime.of(21, 0), recorded) },
                state = FeelingReminderState(ctx),
                alarms = object : ReminderAlarms {
                    override fun scheduleReminder(at: Instant) {
                        real.scheduleReminder(at)
                        reminderAlarms += at
                    }

                    override fun scheduleRefresh(at: Instant) {
                        real.scheduleRefresh(at)
                        finished.countDown() // 1 回の起床の最後の呼び出し
                    }
                },
                notifier = FeelingNotification(ctx, Config.webUrl),
                clock = { now },
            )
        }
    }

    @After
    fun tearDown() {
        FeelingReminderWiring.runnerForTest = null
        Config.clearOverrideForTest()
        manager.cancelAll()
    }

    /** 予約した時刻に OS が配る放送と同じ形（同じアプリ内の明示的な宛先）。 */
    private fun wakeByAlarm() = context.sendBroadcast(
        Intent(context, FeelingReminderReceiver::class.java).setAction(FeelingReminderReceiver.ACTION_REMINDER),
    )

    private fun feelNotifications() =
        manager.activeNotifications.filter { it.id == FeelingNotification.NOTIFICATION_ID }

    private fun awaitWake() = assertTrue("受け手が最後まで走らない", finished.await(30, TimeUnit.SECONDS))

    private fun awaitNotification() {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10)
        while (feelNotifications().isEmpty() && System.nanoTime() < deadline) Thread.sleep(100)
    }

    // Scenario: 書いていない日は通知の時刻に通知が出る
    // Scenario: 通知を押すと今日の主観入力の画面が開く
    @Test
    fun anUnrecordedDayPostsOneNotificationAndTappingOpensTheFeelAddress() {
        recorded = false
        wakeByAlarm()
        awaitWake()
        awaitNotification()

        val posted = feelNotifications()
        assertEquals(1, posted.size)
        val n = posted.single().notification
        val title = n.extras.getCharSequence(android.app.Notification.EXTRA_TITLE).toString()
        val text = n.extras.getCharSequence(android.app.Notification.EXTRA_TEXT).toString()
        assertEquals("今日はどうでしたか", title)
        assertEquals("気分を残す", text)

        assertNotNull("押す先が付いていない", n.contentIntent)
        // 本物の押し方: 通知の欄を開き、通知を押す（PendingIntent を自分で送るとバックグラウンドからの起動として止められる）
        val device = UiDevice.getInstance(InstrumentationRegistry.getInstrumentation())
        device.wakeUp()
        device.openNotification()
        assertTrue("通知が欄に出ない", device.wait(Until.hasObject(By.text("今日はどうでしたか")), 15_000))
        device.findObject(By.text("今日はどうでしたか")).click()
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15)
        var opened = emptyList<String>()
        while (opened.isEmpty() && System.nanoTime() < deadline) {
            Thread.sleep(200)
            opened = shell("logcat -d -s ${FeelProbeActivity.PROBE_TAG}:I").lines().filter { "opened " in it }
        }
        assertEquals(1, opened.size)
        assertTrue(opened.single(), opened.single().endsWith("opened ashiato-feel-test://web/#/feel/2026-10-07"))
        // 次の予約は翌日の通知の時刻
        assertEquals(at(21).plusSeconds(24 * 3600), reminderAlarms.last())
    }

    // Scenario: 書いた日は通知が出ない
    @Test
    fun aRecordedDayPostsNothing() {
        recorded = true
        wakeByAlarm()
        awaitWake()
        Thread.sleep(1_000)

        assertTrue(feelNotifications().isEmpty())
        val prefs = context.getSharedPreferences("feeling_reminder", Context.MODE_PRIVATE)
        assertNull(prefs.getString("last_notified", null))
    }

    // Scenario: 通知の時刻の後に再起動した端末はその日のうちに通知する
    @Test
    fun aRebootAfterTheReminderTimeRestoresTheAlarmAndPostsTheSameDay() {
        recorded = false
        now = at(23)
        // BOOT_COMPLETED は保護された放送で、shell からも送れない（Permission Denial。実測）。
        // 受け手を本物のフレームワークの上で直接起こし、OS が配る側は「宣言されて外から届く」ことで確かめる
        val bootIntent = Intent(Intent.ACTION_BOOT_COMPLETED).setPackage(context.packageName)
        val declared = context.packageManager.queryBroadcastReceivers(bootIntent, 0)
        assertTrue(
            "再起動の受け手が宣言されていない",
            declared.any {
                it.activityInfo.name == FeelingReminderBootReceiver::class.java.name && it.activityInfo.exported
            },
        )
        FeelingReminderBootReceiver().onReceive(context, bootIntent)
        awaitWake()
        awaitNotification()

        assertEquals(1, feelNotifications().size)
        // 予約が戻っている: 明日の通知の時刻を張り直し、OS の予約表にこのアプリの予約がある
        assertEquals(at(21).plusSeconds(24 * 3600), reminderAlarms.last())
        assertTrue(shell("dumpsys alarm").contains(context.packageName))
    }
}
