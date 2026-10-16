// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.io.FileInputStream
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用（人間が後から動画で見る）。ST17 の携帯の通知「今日はどうでしたか」を 1 本の流れで撮る:
 * 書いていない日（2026-10-07）の通知の時刻を過ぎて受け手が起きる → 本物のサーバに聞いて通知が出る → 通知を押すと
 * その日の主観入力のアドレスが開く（結果の札）→ 書いた日（2026-09-06。偽データ）は通知が出ない（結果の札）。
 *
 * 通知の判断は**本物のサーバ**（録画用の APK の接続先。`tools/record-env.sh` が立てた偽データ入りのサーバ）に聞く。
 * 偽データの主観は既定の利用者（全部 0）のものなので、問い合わせはその利用者で行う。差し替えるのは時計と予約 ——
 * 実際の時刻は通知の時刻（21:00）の前かもしれず、その日はまだ待つ判断になるため。時計を過ぎた日に置くと次の予約も
 * 過去になり、本物の AlarmManager ではその場で鳴って受け手が重なるので、予約は張らない（予約が鳴ることは
 * `FeelingReminderInstrumentedTest` が持つ）。通知は本物の NotificationManager で、受け手は予約した時刻に OS が配るのと
 * 同じ放送で起こす。
 *
 * **押した先で本物のブラウザは開かない。** 録画用の APK は Web のアドレスを持たず（`tools/record-env.sh` が渡すのは API の
 * 接続先だけ）、端末のブラウザには画面のログインも無い。押した先は受け皿の画面（[FeelProbeActivity]。androidTest の
 * マニフェスト）で受け、開かれたアドレスを結果の札で見せる。開いた先の画面そのものは Web の録画（ST17-feel）が撮る。
 *
 * **録画のときだけ走る**（`-e recording 1`）。
 */
@RunWith(AndroidJUnit4::class)
class St17RecordingTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(Manifest.permission.POST_NOTIFICATIONS)

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)
    private val tokyo = ZoneId.of("Asia/Tokyo")

    private var now: Instant = Instant.now()
    private var finished = CountDownLatch(1)
    private val verdicts = mutableListOf<String>()

    @After
    fun tearDown() {
        FeelingReminderWiring.runnerForTest = null
        Config.clearOverrideForTest()
        manager.cancelAll()
    }

    @Test
    fun feelingReminderForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        assertTrue("録画用の APK にサーバの接続先が入っていない", Config.isComplete)
        Config.overrideWebUrlForTest(PROBE_WEB_URL)
        FeelingReminderWiring.runnerForTest = { ctx ->
            FeelingReminderRunner(
                source = HttpReminderSource(Config.baseUrl, Config.apiToken, SEED_USER),
                state = FeelingReminderState(ctx),
                // 予約は張らない: 時計を過去の日に置くので、本物の予約はその場で鳴って受け手をもう一度起こす（動画が乱れる）
                alarms = object : ReminderAlarms {
                    override fun scheduleReminder(at: Instant) = Unit

                    override fun scheduleRefresh(at: Instant) {
                        finished.countDown() // 1 回の起床の最後の呼び出し
                    }
                },
                notifier = FeelingNotification(ctx, Config.webUrl),
                clock = { now },
                log = { verdicts += it },
            )
        }
        device.pressHome()

        step("1. 書いていない日（$UNRECORDED）の 21:05。予約した時刻に受け手が起き、サーバに聞く")
        wakeAt(UNRECORDED)
        assertTrue("判断が notify でない: $verdicts", verdicts.last().endsWith("decision=notify"))
        val posted = waitUntil { feelNotifications().isNotEmpty() }
        assertTrue("通知が出ない", posted)
        val n = feelNotifications().single().notification
        assertEquals(FeelingNotification.TITLE, n.extras.getCharSequence(Notification.EXTRA_TITLE).toString())
        assertEquals(FeelingNotification.TEXT, n.extras.getCharSequence(Notification.EXTRA_TEXT).toString())

        step("2. 通知の一覧に「今日はどうでしたか」が出ている")
        shell("logcat -c")
        device.openNotification()
        assertNotNull("通知が欄に出ない", device.wait(Until.findObject(By.text(FeelingNotification.TITLE)), TIMEOUT_MS))
        hold(4_000)

        step("3. 通知を押すと、その日の主観入力のアドレスが開く")
        device.findObject(By.text(FeelingNotification.TITLE)).click()
        var opened = emptyList<String>()
        waitUntil {
            opened = shell("logcat -d -s ${FeelProbeActivity.PROBE_TAG}:I").lines().filter { "opened " in it }
            opened.isNotEmpty()
        }
        assertEquals(1, opened.size)
        val address = opened.single().substringAfter("opened ")
        assertEquals("$PROBE_WEB_URL/#/feel/$UNRECORDED", address)
        device.pressHome()
        showCard(
            "【録画の結果】通知を押して開いたアドレス",
            listOf(address, "（録画では受け皿の画面で受けた。開いた先の画面は Web の録画が撮る）", "サーバの判断: 未記入 → 通知した"),
        )

        step("4. 書いた日（$RECORDED。偽データで +1）の 21:05。サーバに聞くと、通知は出ない")
        manager.cancelAll()
        wakeAt(RECORDED)
        assertTrue("判断が skip でない: $verdicts", verdicts.last().endsWith("decision=skip"))
        hold(2_000)
        assertTrue("書いた日に通知が出た", feelNotifications().isEmpty())
        showCard("【録画の結果】書いた日の判断", listOf("$RECORDED: 記録あり → 通知しない", "「今日はどうでしたか」は出ていない"))
    }

    /** その日の 21:05 に時計を置き、端末に残す状態を消して、予約した時刻に OS が配るのと同じ放送で受け手を起こす。 */
    private fun wakeAt(day: LocalDate) {
        context.getSharedPreferences("feeling_reminder", Context.MODE_PRIVATE).edit().clear().commit()
        now = day.atTime(21, 5).atZone(tokyo).toInstant()
        finished = CountDownLatch(1)
        context.sendBroadcast(
            Intent(context, FeelingReminderReceiver::class.java).setAction(FeelingReminderReceiver.ACTION_REMINDER),
        )
        assertTrue("受け手が最後まで走らない", finished.await(30, TimeUnit.SECONDS))
    }

    private fun feelNotifications() = manager.activeNotifications.filter { it.id == FeelingNotification.NOTIFICATION_ID }

    private fun shell(cmd: String): String =
        instrumentation.uiAutomation.executeShellCommand(cmd)
            .let { FileInputStream(it.fileDescriptor) }
            .use { it.readBytes().toString(Charsets.UTF_8) }

    /** 結果の札を通知で出し、通知の一覧を開いて見せる（`St05RecordingTest` と同じ）。 */
    private fun showCard(title: String, lines: List<String>) {
        manager.createNotificationChannel(NotificationChannel(CARD_CHANNEL, "録画の結果", NotificationManager.IMPORTANCE_HIGH))
        manager.notify(
            CARD_ID,
            Notification.Builder(context, CARD_CHANNEL)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setContentTitle(title)
                .setContentText(lines.first())
                .setGroup(CARD_CHANNEL)
                .setStyle(Notification.BigTextStyle().bigText(lines.joinToString("\n")))
                .build(),
        )
        device.openNotification()
        assertNotNull("結果の札が通知の一覧に出ない", device.wait(Until.findObject(By.text(title)), TIMEOUT_MS))
        val lastLine = By.textContains(lines.last().take(12))
        if (!device.hasObject(lastLine)) {
            device.findObject(By.text(title))?.swipe(Direction.DOWN, 0.8f)
            if (!device.wait(Until.hasObject(lastLine), 3_000)) {
                var node = device.findObject(By.text(title))
                var depth = 0
                while (node != null && depth < 4) {
                    val expand = node.findObject(By.res("android:id/expand_button"))
                    if (expand != null) { expand.click(); break }
                    node = node.parent; depth++
                }
                device.wait(Until.hasObject(lastLine), 3_000)
            }
        }
        assertTrue("結果の札を広げられない（最後の行が見えない）", device.hasObject(lastLine))
        hold(8_000)
        device.pressBack()
        manager.cancel(CARD_ID)
    }

    private fun step(label: String) {
        instrumentation.runOnMainSync { Toast.makeText(context, label, Toast.LENGTH_LONG).show() }
        hold()
    }

    private fun hold(ms: Long = HOLD_MS) = Thread.sleep(ms)

    private fun waitUntil(timeoutMs: Long = 15_000, check: () -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (check()) return true
            Thread.sleep(200)
        }
        return false
    }

    private companion object {
        const val TIMEOUT_MS = 15_000L
        const val HOLD_MS = 1_500L
        const val PROBE_WEB_URL = "ashiato-feel-test://web"
        /** 偽データ（`tools/seed.sh` の主観の節）の利用者 */
        const val SEED_USER = "00000000-0000-0000-0000-000000000000"
        val UNRECORDED: LocalDate = LocalDate.of(2026, 10, 7)
        val RECORDED: LocalDate = LocalDate.of(2026, 9, 6)
        const val CARD_CHANNEL = "recording-card"
        const val CARD_ID = 9_017
    }
}
