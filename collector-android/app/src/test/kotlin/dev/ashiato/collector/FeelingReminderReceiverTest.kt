// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.AlarmManager
import android.app.Application
import android.app.Notification
import android.app.NotificationManager
import android.content.Intent
import android.content.pm.PackageManager
import androidx.test.core.app.ApplicationProvider
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.net.InetAddress
import java.net.ServerSocket
import java.time.Instant
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId

/**
 * 携帯の通知（ST17 design D10 / D11）。**Robolectric の本物の AlarmManager / NotificationManager** を通し、
 * 外へ出ていく値（予約の窓・通知の文面・開く先）を観測する。サーバへの問い合わせだけ偽物に差し替える。
 */
@RunWith(RobolectricTestRunner::class)
class FeelingReminderReceiverTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val manager: NotificationManager get() = app.getSystemService(NotificationManager::class.java)
    private val alarms: AlarmManager get() = app.getSystemService(AlarmManager::class.java)
    private val tokyo = ZoneId.of("Asia/Tokyo")
    private val day = LocalDate.of(2026, 10, 7)

    private var now: Instant = at(20, 59)
    private var answer: ReminderAnswer? = ReminderAnswer(LocalTime.of(21, 0), recorded = false)
    private var webUrl = "https://ashiato.example.ts.net"
    private var posts = 0
    private val lines = mutableListOf<String>()

    private fun at(h: Int, m: Int = 0): Instant = day.atTime(h, m).atZone(tokyo).toInstant()

    @Before
    fun setUp() {
        shadowOf(app).grantPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        FeelingReminderWiring.async = { it() }
        FeelingReminderWiring.runnerForTest = { ctx ->
            val real = FeelingNotification(ctx, webUrl)
            FeelingReminderRunner(
                source = { answer },
                state = FeelingReminderState(ctx),
                alarms = FeelingReminderScheduler(ctx),
                notifier = { date -> real.post(date).also { if (it) posts++ } },
                clock = { now },
                log = { lines += it },
            )
        }
        app.getSharedPreferences("feeling_reminder", 0).edit().clear().commit()
    }

    @After
    fun tearDown() {
        FeelingReminderWiring.runnerForTest = null
        FeelingReminderWiring.async = { task -> Thread(task, "feeling-reminder").start() }
        Config.clearOverrideForTest()
    }

    private fun wake() = FeelingReminderReceiver().onReceive(app, Intent(FeelingReminderReceiver.ACTION_REMINDER))

    private fun posted(): Notification? = shadowOf(manager).getNotification(FeelingNotification.NOTIFICATION_ID)

    /** 予約のうち `action` のもの。 */
    private fun alarmOf(action: String) = shadowOf(alarms).scheduledAlarms
        .single { shadowOf(it.operation).savedIntent.action == action }

    @Test
    fun `起動の予約は網の問い合わせ中の錠を主糸で待たない`() {
        // Scenario: 通知の仕組みは位置の送信を止めない
        val queued = mutableListOf<() -> Unit>()
        FeelingReminderWiring.async = { queued += it }

        FeelingReminderWiring.ensureScheduled(app)

        // 主糸では何も回さず、作業は別の糸へ渡されただけ
        assertEquals(1, queued.size)
        assertTrue(shadowOf(alarms).scheduledAlarms.isEmpty())
        queued.single()()
        assertEquals(2, shadowOf(alarms).scheduledAlarms.size)
    }

    @Test
    fun `通知は 15 分の窓で予約される`() {
        // Scenario: 通知は 15 分の窓で予約される
        wake()

        val reminder = alarmOf(FeelingReminderReceiver.ACTION_REMINDER)
        assertEquals(at(21).toEpochMilli(), reminder.triggerAtMs)
        assertEquals(15 * 60 * 1000L, reminder.windowLengthMs)
        assertEquals(REMINDER_WINDOW_MS, reminder.windowLengthMs)
        assertEquals(AlarmManager.RTC_WAKEUP, reminder.type)
        // 20:59 には出ない
        assertNull(posted())
        // 12:00 の読み直しも同じ形で予約される
        val refresh = alarmOf(FeelingReminderReceiver.ACTION_REFRESH)
        assertEquals(day.plusDays(1).atTime(12, 0).atZone(tokyo).toInstant().toEpochMilli(), refresh.triggerAtMs)
        assertEquals(REMINDER_WINDOW_MS, refresh.windowLengthMs)
    }

    @Test
    fun `通知の時刻の後に再起動した端末はその日のうちに通知する`() {
        // Scenario: 通知の時刻の後に再起動した端末はその日のうちに通知する
        now = at(23)
        FeelingReminderBootReceiver().onReceive(app, Intent(Intent.ACTION_BOOT_COMPLETED))

        assertNotNull(posted())
        assertEquals(1, posts)
        // 予約は戻っている（次は明日）
        assertEquals(
            day.plusDays(1).atTime(21, 0).atZone(tokyo).toInstant().toEpochMilli(),
            alarmOf(FeelingReminderReceiver.ACTION_REMINDER).triggerAtMs,
        )
        // 再起動の通知でも、同じ日には 2 回出ない
        FeelingReminderBootReceiver().onReceive(app, Intent(Intent.ACTION_BOOT_COMPLETED))
        assertEquals(1, posts)
    }

    @Test
    fun `再起動の受け手は起動以外の通知では動かない`() {
        now = at(23)
        FeelingReminderBootReceiver().onReceive(app, Intent(Intent.ACTION_TIME_CHANGED))

        assertNull(posted())
        assertTrue(shadowOf(alarms).scheduledAlarms.isEmpty())
    }

    @Test
    fun `再起動した時刻が通知の時刻の前なら通知せず予約だけ戻す`() {
        now = at(8)
        FeelingReminderBootReceiver().onReceive(app, Intent(Intent.ACTION_BOOT_COMPLETED))

        assertNull(posted())
        assertEquals(at(21).toEpochMilli(), alarmOf(FeelingReminderReceiver.ACTION_REMINDER).triggerAtMs)
    }

    @Test
    fun `書いた日は通知が出ない`() {
        answer = ReminderAnswer(LocalTime.of(21, 0), recorded = true)
        now = at(21, 5)
        wake()

        assertNull(posted())
        assertEquals("kind=feeling_reminder date=2026-10-07 decision=skip", lines.single())
    }

    @Test
    fun `通知の文面に主観の中身が出ない`() {
        // Scenario: 通知の文面に主観の中身が出ない
        // 前の日にひとことを書いた状態でも、題と本文は定数（応答は 2 欄だけで、中身は端末に届かない）
        now = at(21, 5)
        wake()

        val n = posted()!!
        assertEquals("今日はどうでしたか", n.extras.getCharSequence(Notification.EXTRA_TITLE).toString())
        assertEquals("気分を残す", n.extras.getCharSequence(Notification.EXTRA_TEXT).toString())
        assertEquals(FeelingNotification.TITLE, "今日はどうでしたか")
        assertEquals(FeelingNotification.TEXT, "気分を残す")
        assertFalse(FeelingNotification.TITLE.any { it.isDigit() })
        assertFalse(FeelingNotification.TEXT.any { it.isDigit() })
        // 端末に残すのは通知した日と通知の時刻だけ
        val prefs = app.getSharedPreferences("feeling_reminder", 0).all
        assertEquals(setOf("at", "last_notified"), prefs.keys)
        // ログは日付と判断だけ
        assertEquals("kind=feeling_reminder date=2026-10-07 decision=notify", lines.single())
    }

    @Test
    fun `通知の仕組みは正確な時刻の権限を求めない`() {
        // Scenario: 通知の仕組みは正確な時刻の権限を求めない
        val flags = PackageManager.GET_PERMISSIONS or PackageManager.GET_RECEIVERS
        val info = app.packageManager.getPackageInfo(app.packageName, flags)
        val permissions = info.requestedPermissions.orEmpty().toSet()

        assertFalse(permissions.contains("android.permission.SCHEDULE_EXACT_ALARM"))
        assertFalse(permissions.contains("android.permission.USE_EXACT_ALARM"))
        assertTrue(permissions.contains("android.permission.RECEIVE_BOOT_COMPLETED"))
        // 再起動の受け手が宣言されている
        val bootIntent = Intent(Intent.ACTION_BOOT_COMPLETED).setPackage(app.packageName)
        val boot = app.packageManager.queryBroadcastReceivers(bootIntent, 0)
        assertTrue(boot.any { it.activityInfo.name == FeelingReminderBootReceiver::class.java.name })
    }

    @Test
    fun `通知を押すと今日の主観入力の画面が開く`() {
        // Scenario: 通知を押すと今日の主観入力の画面が開く
        now = at(21, 5)
        wake()

        val view = shadowOf(posted()!!.contentIntent).savedIntent
        assertEquals(Intent.ACTION_VIEW, view.action)
        assertEquals("https://ashiato.example.ts.net/#/feel/2026-10-07", view.data.toString())
        // 末尾のスラッシュがあっても同じアドレス
        assertEquals(
            "https://ashiato.example.ts.net/#/feel/2026-10-07",
            FeelingNotification.feelUrl("https://ashiato.example.ts.net/", day),
        )
    }

    @Test
    fun `画面のアドレスを持たないビルドでも通知は出る`() {
        // Scenario: 画面のアドレスを持たないビルドでも通知は出る
        webUrl = ""
        now = at(21, 5)
        wake()

        assertEquals(1, posts)
        assertNull(posted()!!.contentIntent)
        // ビルドの既定は空（CI・計測テスト・確認バッチは渡さない）
        assertEquals("", BuildConfig.WEB_URL)
        assertEquals("", Config.webUrl)
    }

    @Test
    fun `十二時の読み直しで時刻が早まれば、その日の予約が新しい時刻に張り直る`() {
        // 21:00 だったものが、9:00 に 20:00 へ変わった。12:00 の起床で読み直す
        now = at(12)
        answer = ReminderAnswer(LocalTime.of(20, 0), recorded = false)
        FeelingReminderReceiver().onReceive(app, Intent(FeelingReminderReceiver.ACTION_REFRESH))

        assertNull(posted())
        assertEquals(at(20).toEpochMilli(), alarmOf(FeelingReminderReceiver.ACTION_REMINDER).triggerAtMs)
        assertEquals(LocalTime.of(20, 0), FeelingReminderState(app).at)
    }

    @Test
    fun `届かない日は最後に得た時刻を端末から使う`() {
        now = at(12)
        answer = ReminderAnswer(LocalTime.of(22, 30), recorded = false)
        wake()
        answer = null
        now = at(21, 30)
        wake()

        assertNull(posted())
        assertEquals(at(22, 30).toEpochMilli(), alarmOf(FeelingReminderReceiver.ACTION_REMINDER).triggerAtMs)
        now = at(22, 31)
        wake()
        assertEquals(1, posts)
    }

    @Test
    fun `通知の仕組みは位置の送信を止めない`() {
        // Scenario: 通知の仕組みは位置の送信を止めない
        Config.overrideForTest(baseUrl = "http://127.0.0.1:1", apiToken = "t", userId = "u")
        val service = Robolectric.buildService(TestableLocationService::class.java, Intent())
            .create().startCommand(0, 1).get()
        // サービスの起動が予約を張る（ネットワークには触れない）
        assertNotNull(alarmOf(FeelingReminderReceiver.ACTION_REMINDER))

        service.source.deliver(Instant.parse("2026-10-07T01:00:00Z"))
        now = at(21, 5)
        wake()
        service.source.deliver(Instant.parse("2026-10-07T01:01:00Z"))

        val fixes = service.outboxForTest.snapshot().filter { it.logicalSource == SourceCadence.LOCATION.logicalSource }
        assertEquals(2, fixes.size)
        service.scheduler.fire()
        assertTrue(service.posted.any { it.first == "/ingest" })
        assertEquals(1, posts)
    }

    /** 生の `ServerSocket` の偽サーバ（`com.sun.net.httpserver` は Android の classpath に無い）。1 件ごとに `reply` を返す。 */
    private class FakeServer(private val reply: () -> Pair<Int, String>) : AutoCloseable {
        private val socket = ServerSocket(0, 0, InetAddress.getLoopbackAddress())
        val requests = mutableListOf<List<String>>()
        val base = "http://127.0.0.1:${socket.localPort}"

        init {
            Thread {
                while (!socket.isClosed) {
                    runCatching {
                        socket.accept().use { c ->
                            val reader = c.getInputStream().bufferedReader()
                            val lines = generateSequence { reader.readLine() }.takeWhile { it.isNotEmpty() }.toList()
                            requests += lines
                            val (status, body) = reply()
                            val bytes = body.toByteArray()
                            val head = "HTTP/1.1 $status X\r\ncontent-length: ${bytes.size}\r\n" +
                                "connection: close\r\n\r\n"
                            c.getOutputStream().write(head.toByteArray() + bytes)
                        }
                    }
                }
            }.also { it.isDaemon = true }.start()
        }

        override fun close() = socket.close()
    }

    @Test
    fun `サーバの応答を読む`() {
        FakeServer { 200 to """{"at":"22:30","recorded":true,"extra":1}""" }.use { server ->
            val got = HttpReminderSource(server.base + "/", "tok", "u-1").fetch(day)

            assertEquals(ReminderAnswer(LocalTime.of(22, 30), true), got)
            val request = server.requests.single()
            assertEquals("GET /feelings/reminder?date=2026-10-07&user_id=u-1 HTTP/1.1", request.first())
            assertTrue(request.any { it.equals("authorization: Bearer tok", ignoreCase = true) })
            // 設定が揃っていなければ聞かない
            assertNull(HttpReminderSource(server.base, "", "u-1").fetch(day))
            assertEquals(1, server.requests.size)
        }
    }

    @Test
    fun `サーバが断る・届かない・読めないときは得られなかったことにする`() {
        var reply = 401 to ""
        val base = FakeServer { reply }.use { server ->
            assertNull(HttpReminderSource(server.base, "tok", "u").fetch(day))
            reply = 200 to "not json"
            assertNull(HttpReminderSource(server.base, "tok", "u").fetch(day))
            reply = 200 to """{"at":"25:99","recorded":false}"""
            assertNull(HttpReminderSource(server.base, "tok", "u").fetch(day))
            server.base
        }
        // 網に届かない（閉じたポート）
        assertNull(HttpReminderSource(base, "tok", "u", timeoutMs = 500).fetch(day))
    }

    @Test
    fun `網に届かないときは通知が出る`() {
        // Scenario: 網に届かないときは通知が出る
        answer = null
        now = at(21, 5)
        wake()

        assertEquals(1, posts)
    }
}
