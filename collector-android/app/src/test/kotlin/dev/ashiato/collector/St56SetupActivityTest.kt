// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.app.Application
import android.content.Intent
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import androidx.test.core.app.ApplicationProvider
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.ServerSocket

private class St56ProbeActivity : SetupActivity() {
    var answer: Outcome = Outcome.Responded(200, "{\"contract_version\":1}")
    var probeCalls = 0

    override fun newContractGet(): () -> Outcome = {
        probeCalls++
        answer
    }

    override fun runContractProbe(task: () -> Unit) = task()
}

@RunWith(RobolectricTestRunner::class)
class St56SetupActivityTest {
    private val app: Application = ApplicationProvider.getApplicationContext()

    @Before
    fun setUp() {
        app.getSharedPreferences(Config.SETUP_PREFS, 0).edit().clear().commit()
        app.getSharedPreferences(MainActivity.PREFS, 0).edit().clear().commit()
        Config.clearOverrideForTest()
        Config.initialize(app)
    }

    @After
    fun tearDown() {
        app.getSharedPreferences(Config.SETUP_PREFS, 0).edit().clear().commit()
        app.getSharedPreferences(MainActivity.PREFS, 0).edit().clear().commit()
    }

    private fun setupActivity(answer: Outcome = Outcome.Responded(200, "{\"contract_version\":1}")): St56ProbeActivity {
        val controller = Robolectric.buildActivity(St56ProbeActivity::class.java).create().resume()
        controller.get().answer = answer
        return controller.get()
    }

    private fun root(activity: St56ProbeActivity): LinearLayout =
        activity.findViewById<ViewGroup>(android.R.id.content).getChildAt(0) as LinearLayout

    private fun save(activity: St56ProbeActivity, token: String = "token-for-test-0123456789") {
        (root(activity).getChildAt(1) as EditText).setText(
            buildSetupString("https://collector.example/api", token),
        )
        root(activity).findViewWithText<Button>("保存").performClick()
        assertTrue(Config.hasSavedSetup)
        assertEquals(token, Config.apiToken)
    }

    private inline fun <reified T : TextView> LinearLayout.findViewWithText(text: String): T =
        (0 until childCount).asSequence().map { getChildAt(it) }.filterIsInstance<T>()
            .first { it.text.toString() == text }

    @Test
    fun `初めて開くと、権限の要求より前に設定の画面が出る`() {
        // Scenario: 初めて開くと、権限の要求より前に設定の画面が出る
        val activity = Robolectric.buildActivity(MainActivity::class.java, Intent(Intent.ACTION_MAIN)).create().get()
        val next = shadowOf(activity).nextStartedActivity
        assertNotNull(next)
        assertEquals(SetupActivity::class.java.name, next.component?.className)
    }

    @Test
    fun `「後で」を押しても、権限の要求と収集が始まる`() {
        // Scenario: 「後で」を押しても、権限の要求と収集が始まる
        val activity = setupActivity()
        root(activity).findViewWithText<Button>("後で").performClick()
        assertTrue(app.getSharedPreferences(Config.SETUP_PREFS, 0).getBoolean(MainActivity.SETUP_SKIPPED, false))
        assertFalse(app.getSharedPreferences(MainActivity.PREFS, 0).contains(MainActivity.SETUP_SKIPPED))
        assertTrue(activity.isFinishing)
    }

    @Test
    fun `未設定の間は通知で促し、押すと設定の画面が開く`() {
        // Scenario: 未設定の間は通知で促し、押すと設定の画面が開く
        val service = Robolectric.buildService(LocationService::class.java, Intent()).create().get()
        service.showSetupNotice()
        val notification = shadowOf(service.getSystemService(android.app.NotificationManager::class.java))
            .getNotification(LocationService.SETUP_NOTIFICATION_ID)
        assertEquals(
            SetupActivity::class.java.name,
            shadowOf(notification.contentIntent).savedIntent.component?.className
        )
        assertEquals("送り先が設定されていません", notification.extras.getCharSequence("android.text").toString())
    }

    @Test
    fun `組み立ての時に送り先を渡した収集アプリは、設定の画面を先に出さない`() {
        // Scenario: 組み立ての時に送り先を渡した収集アプリは、設定の画面を先に出さない
        Config.overrideForTest("https://build.example/api", "build-token", "user")
        val activity = Robolectric.buildActivity(MainActivity::class.java, Intent(Intent.ACTION_MAIN)).create().get()
        assertEquals(null, shadowOf(activity).nextStartedActivity)
    }

    @Test
    fun `繋がれば「繋がりました」`() {
        // Scenario: 繋がれば「繋がりました」
        val activity = setupActivity()
        save(activity)
        assertEquals("繋がりました", (root(activity).getChildAt(root(activity).childCount - 1) as TextView).text)
    }

    @Test
    fun `401 なら「資格情報か送り先が違います」と出て、設定は保存される`() {
        // Scenario: 401 なら「資格情報か送り先が違います」と出て、設定は保存される
        val activity = setupActivity(Outcome.Responded(401, ""))
        save(activity)
        assertEquals("資格情報か送り先が違います", (root(activity).getChildAt(root(activity).childCount - 1) as TextView).text)
        assertTrue(Config.hasSavedSetup)
    }

    @Test
    fun `系統の違うサーバなら、その旨を出す`() {
        // Scenario: 系統の違うサーバなら、その旨を出す
        val activity = setupActivity(
            Outcome.Responded(200, "{\"contract_version\":1}", headers = mapOf("ashiato-channel" to "dev"))
        )
        save(activity)
        assertEquals("送り先が開発版のサーバです", (root(activity).getChildAt(root(activity).childCount - 1) as TextView).text)
    }

    @Test
    fun `届かなくても保存し、繋がったら送ると示す`() {
        // Scenario: 届かなくても保存し、繋がったら送ると示す
        val activity = setupActivity(Outcome.Unreachable("ConnectException"))
        save(activity)
        assertEquals("保存しました。繋がったら送ります", (root(activity).getChildAt(root(activity).childCount - 1) as TextView).text)
        assertTrue(Config.hasSavedSetup)
    }

    @Test
    fun `確かめの問い合わせでは記録を送らない`() {
        // Scenario: 確かめの問い合わせでは記録を送らない
        val activity = setupActivity()
        save(activity)
        assertEquals(1, activity.probeCalls)
        assertEquals(LocationService.ACTION_SETUP_CHANGED, shadowOf(app).peekNextStartedService()?.action)
    }

    private fun paste(activity: St56ProbeActivity, text: String) {
        (root(activity).getChildAt(1) as EditText).setText(text)
        root(activity).findViewWithText<Button>("保存").performClick()
    }

    private fun screenText(activity: St56ProbeActivity): String =
        (0 until root(activity).childCount).map { root(activity).getChildAt(it) }
            .filterIsInstance<TextView>().joinToString("\n") { it.text }

    @Test
    fun `読めない文字列と携帯から届かない送り先は、spec の文で断る`() {
        // review/code.md R6: 文を分けて、試験から文言を見る
        val activity = setupActivity()
        paste(activity, "hello")
        assertEquals("設定の文字列として読めません", SetupActivity.UNREADABLE_TEXT)
        assertEquals(
            SetupActivity.UNREADABLE_TEXT,
            (root(activity).getChildAt(root(activity).childCount - 1) as TextView).text
        )
        paste(activity, buildSetupString("http://127.0.0.1:18787/api", "token-for-test-0123456789"))
        assertEquals(
            "この送り先は携帯から届きません。サーバの画面を Tailscale のアドレスで開いて、設定の文字列を出し直してください",
            SetupActivity.UNREACHABLE_TEXT,
        )
        assertEquals(
            SetupActivity.UNREACHABLE_TEXT,
            (root(activity).getChildAt(root(activity).childCount - 1) as TextView).text
        )
        assertFalse(Config.hasSavedSetup)
        assertEquals(0, activity.probeCalls)
    }

    @Test
    fun `保存すると「設定済み」と送り先だけを示し、資格情報は画面に残さない`() {
        // Scenario: 資格情報は画面にもログにも出ない
        // review/code.md R6: spec「設定の画面は「設定済み」と送り先だけを示す」。R29: 開き直しと送信 1 回まで見る
        org.robolectric.shadows.ShadowLog.clear()
        val activity = setupActivity()
        save(activity)
        val text = screenText(activity)
        assertTrue(text, text.contains(SetupActivity.CONFIGURED_TEXT))
        assertTrue(text, text.contains("https://collector.example/api"))
        assertFalse(text.contains("token-for-test-0123456789"))
        assertEquals("", (root(activity).getChildAt(1) as EditText).text.toString())
        val reopened = setupActivity()
        val again = screenText(reopened)
        assertTrue(again, again.contains(SetupActivity.CONFIGURED_TEXT))
        assertFalse(again.contains("token-for-test-0123456789"))
        // 送信を 1 回行う（届かない送り先でよい。ログに資格情報が出ないことを見る）
        val sent = HttpTransport({ "http://127.0.0.1:1/api" }, { Config.apiToken }).post("[]")
        assertTrue(sent is Outcome.Unreachable)
        assertTrue(
            org.robolectric.shadows.ShadowLog.getLogs().none { it.msg.orEmpty().contains("token-for-test-0123456789") }
        )
    }

    @Test
    fun `未設定の知らせは 24 時間ごとに出し直し、設定したら消す`() {
        // review/code.md R7: 試験の時計で見る
        val service = Robolectric.buildService(LocationService::class.java, Intent()).create().get()
        val manager = service.getSystemService(android.app.NotificationManager::class.java)
        val shadow = shadowOf(manager)
        fun shown() = shadow.getNotification(LocationService.SETUP_NOTIFICATION_ID) != null
        app.getSharedPreferences(Config.SETUP_PREFS, 0).edit().remove(SetupNotifier.UNSET_NOTIFIED_AT_KEY).commit()
        manager.cancel(LocationService.SETUP_NOTIFICATION_ID)
        val t0 = 1_800_000_000_000L
        val hour = 60L * 60L * 1000L
        service.refreshSetupNotice(t0)
        assertTrue(shown())
        manager.cancel(LocationService.SETUP_NOTIFICATION_ID)
        service.refreshSetupNotice(t0 + 24 * hour - 60_000L)
        assertFalse(shown())
        service.refreshSetupNotice(t0 + 24 * hour + 60_000L)
        assertTrue(shown())

        SetupStore(app).save(buildSetupString("https://collector.example/api", "token-for-test-0123456789"))
        service.refreshSetupNotice(t0 + 25 * hour)
        assertFalse(shown())
        service.refreshSetupNotice(t0 + 72 * hour)
        assertFalse(shown())
    }

    @Test
    fun `資格情報だけを変えると、次の送信から新しい値で送る`() {
        // Scenario: 資格情報だけを変えると、次の送信から新しい値で送る
        SetupStore(app).save(buildSetupString("https://collector.example/api", "new-token-0123456789"))
        val server = ServerSocket(0)
        var authorization = ""
        val worker = Thread {
            server.accept().use { socket ->
                val reader = BufferedReader(InputStreamReader(socket.getInputStream(), Charsets.UTF_8))
                reader.readLine()
                while (true) {
                    val line = reader.readLine() ?: break
                    if (line.isEmpty()) break
                    if (line.startsWith(
                            "Authorization:",
                            ignoreCase = true
                        )
                    ) {
                        authorization = line.substringAfter(':').trim()
                    }
                }
                socket.getOutputStream().use {
                    it.write(
                        "HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n[]".toByteArray()
                    )
                }
            }
        }
        worker.start()
        val transport = HttpTransport({ "http://127.0.0.1:${server.localPort}" }, { Config.apiToken })
        transport.post("{}")
        worker.join(1_000)
        server.close()
        assertEquals("new-token-0123456789", Config.apiToken)
        assertEquals("Bearer new-token-0123456789", authorization)
    }
}
