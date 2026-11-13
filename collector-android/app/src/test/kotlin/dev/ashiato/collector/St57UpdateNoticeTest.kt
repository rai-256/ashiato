package dev.ashiato.collector

import android.Manifest
import android.app.AlertDialog
import android.content.Intent
import android.widget.TextView
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.After
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.shadows.ShadowAlertDialog
import org.robolectric.shadows.ShadowNetworkInfo
import java.net.InetAddress
import java.net.ServerSocket
import java.time.Instant
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

private class St57MainActivity : MainActivity() {
    override fun shouldOfferInitialSetup() = false
    override fun unusedAppRestrictionsReader() = UnusedAppRestrictionsReader { state }
    override fun healthPermissions(): HealthPermissions = noHealthConnect()
    override fun runHealth(task: () -> Unit) = task()
    override fun healthRecordSink(): (IngestRequest) -> Boolean = { true }
    override fun healthPendingSlot() = HealthPendingSlot()
    fun resumeForTest() = onResume()
    companion object { var state = UnusedAppRestrictionsState.EXEMPT }
}

private class St57UpdateLocationService : TestableLocationService() {
    internal override fun newUpdateChecker(): UpdateChecker = UpdateChecker(this, destination)
    companion object { lateinit var destination: String }
}

@RunWith(RobolectricTestRunner::class)
class St57UpdateNoticeTest {
    private val app = ApplicationProvider.getApplicationContext<android.app.Application>()
    private val now = Instant.parse("2026-11-12T00:00:00Z")

    @Before fun reset() {
        app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().commit()
        app.getSharedPreferences(MainActivity.PREFS, 0).edit().clear().commit()
        app.getSharedPreferences(Config.SETUP_PREFS, 0).edit().clear().commit()
        Config.clearOverrideForTest()
        shadowOf(app).grantPermissions(
            Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS, Manifest.permission.READ_EXTERNAL_STORAGE,
            Manifest.permission.READ_MEDIA_IMAGES, Manifest.permission.READ_MEDIA_VIDEO,
            Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED, Manifest.permission.ACCESS_MEDIA_LOCATION,
        )
        shadowOf(app.getSystemService(android.app.AppOpsManager::class.java)).setMode(
            android.app.AppOpsManager.OPSTR_GET_USAGE_STATS, android.os.Process.myUid(), app.packageName,
            android.app.AppOpsManager.MODE_ALLOWED,
        )
        St57MainActivity.state = UnusedAppRestrictionsState.EXEMPT
    }

    @After fun cleanup() {
        app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().commit()
        app.getSharedPreferences(MainActivity.PREFS, 0).edit().clear().commit()
        app.getSharedPreferences(Config.SETUP_PREFS, 0).edit().clear().commit()
        Config.clearOverrideForTest()
    }

    private fun configured() = Config.overrideForTest("http://127.0.0.1", "token", Config.DEFAULT_USER_ID)

    /** 実際の HttpURLConnection が何件届いたかを読む小さな loopback の受け手。 */
    private fun releasesServer(expected: Int = 1, body: String = "[{\"tag_name\":\"v0.5.0\",\"draft\":false,\"assets\":[{\"name\":\"ashiato-collector-0.5.0.apk\"}]}]", status: Int = 200, location: String? = null): Triple<String, MutableList<String>, CountDownLatch> {
        val server = ServerSocket(0, 0, InetAddress.getLoopbackAddress())
        val seen = mutableListOf<String>()
        val done = CountDownLatch(1)
        Thread {
            server.use { listener ->
                listener.soTimeout = 500
                repeat(expected) {
                    try {
                        listener.accept().use { socket ->
                            val input = socket.getInputStream().bufferedReader()
                            while (input.readLine()?.isNotEmpty() == true) {}
                            synchronized(seen) { seen += "request" }
                            val redirect = location?.let { "Location: $it\r\n" } ?: ""
                            socket.getOutputStream().write("HTTP/1.1 $status test\r\n${redirect}Content-Length: ${body.toByteArray().size}\r\n\r\n$body".toByteArray())
                        }
                    } catch (_: java.net.SocketTimeoutException) { return@repeat }
                }
            }
            done.countDown()
        }.start()
        return Triple("http://127.0.0.1:${server.localPort}", seen, done)
    }

    private fun texts(activity: android.app.Activity): List<String> {
        val result = mutableListOf<String>()
        fun visit(view: android.view.View) {
            if (view is TextView) result += view.text.toString()
            if (view is android.view.ViewGroup) for (i in 0 until view.childCount) visit(view.getChildAt(i))
        }
        visit(activity.window.decorView)
        return result
    }

    private class TestActivity(private val controller: org.robolectric.android.controller.ActivityController<St57MainActivity>) : AutoCloseable {
        fun onActivity(block: (St57MainActivity) -> Unit) = block(controller.get())
        override fun close() { controller.pause().stop().destroy() }
    }

    private fun open(action: String = Intent.ACTION_MAIN): TestActivity {
        val controller = Robolectric.buildActivity(St57MainActivity::class.java, Intent(app, St57MainActivity::class.java).setAction(action))
            .create().start().resume()
        return TestActivity(controller)
    }

    @Test fun `新しい版を最初の画面の先頭に出す`() {
        // Scenario: 携帯は新しい版を最初の画面に出す
        configured(); app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().putBoolean("intro_shown", true).putString("latest", "0.5.0").commit()
        open().use { it.onActivity { activity -> assertTrue(texts(activity).first().startsWith("更新あり 0.5.0")) } }
    }
    @Test fun `送り先未設定では確認も記録もしない`() {
        // Scenario: 携帯の収集アプリは送り先を設定するまで確かめない
        app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().putBoolean("intro_shown", true).commit()
        assertFalse(UpdateChecker(app).check(now)); assertNull(app.getSharedPreferences(UpdateCheck.PREFS, 0).getString("checked_at", null))
    }
    @Test fun `更新確認は収集サーバでなく配布元だけへ送る`() {
        // Scenario: 携帯の収集アプリは新しい版をサーバに問い合わせない
        val (server, releaseSeen, releaseDone) = releasesServer()
        val (base, baseSeen, baseDone) = releasesServer()
        Config.overrideForTest(base, "token", Config.DEFAULT_USER_ID)
        val checker = UpdateChecker(app, server)
        checker.markIntroShown(now.minusSeconds(25 * 3600))
        assertTrue(checker.check(now))
        assertTrue(releaseDone.await(2, TimeUnit.SECONDS))
        assertTrue(baseDone.await(2, TimeUnit.SECONDS))
        assertEquals(1, releaseSeen.size)
        assertTrue(baseSeen.isEmpty())
    }
    @Test fun `入れ替え直後は画面で切り方を示す`() {
        // Scenario: 確認の無い前の版から入れ替えた携帯は、切り方を示してから確かめる
        configured(); open().use { it.onActivity { activity -> assertTrue(texts(activity).first().startsWith("更新の確認: 有効")) } }
    }
    @Test fun `通知権限が無い間は画面に出ず問い合わせない`() {
        // Scenario: 入れ替えた携帯は、通知を出せず画面も開かなければ問い合わせない
        configured(); shadowOf(app).denyPermissions(Manifest.permission.POST_NOTIFICATIONS)
        open().use { it.onActivity { activity -> assertFalse(texts(activity).any { text -> text.startsWith("更新の確認") }) } }
    }
    @Test fun `通知を許可すると次の画面で切り方を示す`() {
        // Scenario: 入れ替えた携帯は、通知を後から許すと次の機会に切り方を通知してから確かめる
        configured(); app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().putBoolean("intro_shown", false).commit()
        open().use { it.onActivity { activity -> assertTrue(texts(activity).any { text -> text.startsWith("更新の確認") }) } }
    }
    @Test fun `設定画面のスイッチを入力欄より先に置く`() {
        // Scenario: 入れ替えた携帯は、設定の画面を開くと示したとみなす
        val scenario = ActivityScenario.launch(SetupActivity::class.java)
        scenario.onActivity { activity ->
            val content = activity.window.decorView.findViewById<android.view.ViewGroup>(android.R.id.content)
            fun find(type: Class<out android.view.View>): android.view.View? {
                fun visit(view: android.view.View): android.view.View? {
                    if (type.isInstance(view)) return view
                    return (view as? android.view.ViewGroup)?.let { group ->
                        (0 until group.childCount).firstNotNullOfOrNull { visit(group.getChildAt(it)) }
                    }
                }
                return visit(content)
            }
            val switch = find(android.widget.Switch::class.java)!!
            val input = find(android.widget.EditText::class.java)!!
            assertTrue((switch.parent as android.view.ViewGroup).indexOfChild(switch) < (input.parent as android.view.ViewGroup).indexOfChild(input))
        }; scenario.close()
    }
    @Test fun `update チャンネルを切ると示したと数えず問い合わせない`() {
        // Scenario: 入れ替えた携帯は、チャンネル update だけを切ってあっても示したとみなさない
        configured()
        val manager = app.getSystemService(android.app.NotificationManager::class.java)
        manager.createNotificationChannel(android.app.NotificationChannel(UpdateCheck.CHANNEL, "更新", android.app.NotificationManager.IMPORTANCE_NONE))
        val (url, seen, done) = releasesServer()
        assertFalse(UpdateChecker(app, url).check(now))
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertTrue(seen.isEmpty())
        assertFalse(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false))
    }
    @Test fun `切り方を示した後の次の機会に問い合わせる`() {
        // Scenario: 入れ替えた携帯は、画面に切り方を出した後の機会に確かめる
        configured()
        val (url, seen, done) = releasesServer()
        val checker = UpdateChecker(app, url)
        checker.markIntroShown(now.minusSeconds(25 * 3600))
        assertTrue(checker.check(now))
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertEquals(1, seen.size)
    }

    @Test fun `切り方の通知を更新ありで置き換えない`() {
        // Scenario: 切り方の通知を出した後に更新ありの通知が出ても、切り方の通知は残る
        configured()
        val (url, seen, done) = releasesServer()
        val checker = UpdateChecker(app, url)
        assertFalse(checker.check(now))
        assertTrue(checker.check(now.plusSeconds(1)))
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertEquals(1, seen.size)
        val texts = shadowOf(app.getSystemService(android.app.NotificationManager::class.java)).allNotifications
            .map { it.extras.getCharSequence(android.app.Notification.EXTRA_TEXT).toString() }
        assertTrue(texts.contains("更新の確認: 有効（切るには設定の画面）"))
        assertTrue(texts.contains("新しい版 0.5.0 が出ています"))
    }
    @Test fun `権限要求中は印を立てない`() {
        // Scenario: 入れ替えた携帯は、権限を求めている途中は画面に出したとみなさない
        configured(); shadowOf(app).denyPermissions(Manifest.permission.ACCESS_FINE_LOCATION)
        open().use { it.onActivity { activity -> assertNotNull(shadowOf(activity).lastRequestedPermission); assertFalse(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false)) } }
    }
    @Test fun `通知から開いても入れ替えの行を閉じない`() {
        // Scenario: 入れ替えた携帯は、ランチャー以外から開かれて他の行が無くても入れ替えの行を閉じずに出す
        configured(); open("dev.ashiato.UPDATE").use { it.onActivity { activity ->
            assertFalse(activity.isFinishing)
            assertTrue(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false))
        } }
    }
    @Test fun `確認窓が出ている間は印を立てない`() {
        // Scenario: 入れ替えた携帯は、同じ回に確認の窓を重ねたときは窓が閉じた時に画面に出したとみなす
        configured(); St57MainActivity.state = UnusedAppRestrictionsState.NOT_EXEMPT; open().use {
            assertFalse(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false))
            ShadowAlertDialog.getLatestAlertDialog().getButton(AlertDialog.BUTTON_NEGATIVE).performClick()
            shadowOf(android.os.Looper.getMainLooper()).idle()
            assertTrue(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false))
        }
    }
    @Test fun `設定を開く窓を閉じた回は戻るまで印を立てない`() {
        // Scenario: 入れ替えた携帯は、確認の窓を「設定を開く」で閉じた回は画面に出したとみなさない
        configured(); St57MainActivity.state = UnusedAppRestrictionsState.NOT_EXEMPT
        open().use { scenario ->
            ShadowAlertDialog.getLatestAlertDialog().getButton(AlertDialog.BUTTON_POSITIVE).performClick()
            shadowOf(android.os.Looper.getMainLooper()).idle()
            assertNotNull(shadowOf(app).getNextStartedActivity())
            assertFalse(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false))
            scenario.onActivity { it.resumeForTest() }
            assertTrue(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false))
        }
    }
    @Test fun `残った確認窓の間も組み直しで印を立てない`() {
        // Scenario: 入れ替えた携帯は、前の回の確認の窓が出たまま画面を組み直しても画面に出したとみなさない
        configured(); St57MainActivity.state = UnusedAppRestrictionsState.NOT_EXEMPT; open().use { scenario -> scenario.onActivity { it.resumeForTest() }; assertFalse(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false)) }
    }
    @Test fun `示した後に新しい画面へ行くと行を出さない`() {
        // Scenario: 入れ替えた携帯は、示した後に新しく開いた最初の画面には入れ替えの行を出さない
        configured(); UpdateChecker(app).markIntroShown(now); open().use { it.onActivity { activity -> assertFalse(texts(activity).any { text -> text.startsWith("更新の確認") }) } }
    }
    @Test fun `同じ画面の組み直しでは行を残す`() {
        // Scenario: 入れ替えた携帯は、同じ画面を組み直しても入れ替えの行を残す
        configured(); open().use { it.onActivity { activity -> assertTrue(texts(activity).any { text -> text.startsWith("更新の確認") }); activity.resumeForTest(); assertTrue(texts(activity).any { text -> text.startsWith("更新の確認") }) } }
    }
    @Test fun `別画面を前に出している間は印を立てない`() {
        // Scenario: 入れ替えた携帯は、同じ回に別の画面を前に出したときは画面に出したとみなさない
        configured()
        shadowOf(app.getSystemService(android.app.AppOpsManager::class.java)).setMode(
            android.app.AppOpsManager.OPSTR_GET_USAGE_STATS, android.os.Process.myUid(), app.packageName,
            android.app.AppOpsManager.MODE_IGNORED,
        )
        open().use {
            assertEquals(android.provider.Settings.ACTION_USAGE_ACCESS_SETTINGS, shadowOf(app).getNextStartedActivity()?.action)
            assertFalse(app.getSharedPreferences(UpdateCheck.PREFS, 0).getBoolean("intro_shown", false))
        }
    }
    @Test fun `切り方を出す画面では未確認行を出さない`() {
        // Scenario: 入れ替えた携帯は、切り方を画面に出した時に確かめられていないを並べない
        configured(); open().use { it.onActivity { activity -> assertFalse(texts(activity).any { text -> text.contains("7 日確かめられていません") }) } }
    }
    @Test fun `スイッチを切ると示した後でも問い合わせない`() {
        // Scenario: 携帯でスイッチを切ると問い合わせが無くなる
        configured()
        val (url, seen, done) = releasesServer()
        val checker = UpdateChecker(app, url)
        checker.markIntroShown(now.minusSeconds(25 * 3600))
        checker.setEnabled(false)
        assertFalse(checker.check(now))
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertTrue(seen.isEmpty())
    }
    @Test fun `新しい版ごとに通知は一度だけ出る`() {
        // Scenario: 携帯は新しい版ごとに 1 回だけ通知する
        configured()
        val (url, seen, done) = releasesServer(2)
        val checker = UpdateChecker(app, url)
        checker.markIntroShown(now.minusSeconds(25 * 3600))
        assertTrue(checker.check(now))
        assertTrue(checker.check(now.plusSeconds(25 * 3600)))
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertEquals(2, seen.size)
        val notifications = shadowOf(app.getSystemService(android.app.NotificationManager::class.java)).allNotifications
        assertEquals(1, notifications.count { it.extras.getCharSequence(android.app.Notification.EXTRA_TEXT).toString().contains("新しい版") })
    }
    @Test fun `通知の遷移先は MainActivity である`() {
        // Scenario: 携帯の更新の通知を押すと最初の画面が開く
        configured(); UpdateChecker(app).check(now)
        val notification = shadowOf(app.getSystemService(android.app.NotificationManager::class.java)).allNotifications.single()
        assertEquals(MainActivity::class.java.name, shadowOf(notification.contentIntent).savedIntent.component?.className)
    }
    @Test fun `7 日確かめられないと画面と通知は一度だけ出る`() {
        // Scenario: 携帯は 7 日確かめられなければ画面に出し、1 回通知する
        configured()
        val checkNow = Instant.now()
        val (url, seen, done) = releasesServer(2, "{")
        val checker = UpdateChecker(app, url)
        checker.markIntroShown(checkNow.minusSeconds(8 * 86400))
        assertTrue(checker.check(checkNow))
        assertTrue(checker.check(checkNow.plusSeconds(25 * 3600)))
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertEquals(2, seen.size)
        val notifications = shadowOf(app.getSystemService(android.app.NotificationManager::class.java)).allNotifications
        assertEquals(1, notifications.count { it.extras.getCharSequence(android.app.Notification.EXTRA_TEXT).toString().contains("7 日確かめられていません") })
        open().use { it.onActivity { activity -> assertTrue(texts(activity).any { text -> text == "更新を 7 日確かめられていません（最後に確かめた —・読めない答え）" }) } }
    }
    @Test fun `見つけた日は最初に見つけた日のまま表示する`() {
        // Scenario: 携帯は新しい版を最初の画面に出す
        configured()
        app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().putBoolean("intro_shown", true)
            .putString("latest", "0.5.0").putString("found_at", "2026-11-10T00:00:00Z").commit()
        open().use { it.onActivity { activity -> assertTrue(texts(activity).contains("更新あり 0.5.0（11/10 に見つけた）")) } }
    }
    @Test fun `別ホストへの転送は確かめない`() {
        // Scenario: 携帯の問い合わせは版の一覧の取得だけである
        configured()
        val (other, otherSeen, otherDone) = releasesServer()
        val (url, seen, done) = releasesServer(status = 302, location = other.replace("127.0.0.1", "localhost"))
        val checker = UpdateChecker(app, url)
        checker.markIntroShown(now.minusSeconds(25 * 3600))
        assertTrue(checker.check(now))
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertTrue(otherDone.await(2, TimeUnit.SECONDS))
        assertEquals(1, seen.size)
        assertTrue(otherSeen.isEmpty())
        assertEquals("unreadable", app.getSharedPreferences(UpdateCheck.PREFS, 0).getString("last_reason", null))
    }
    @Test fun `JSON Lines の制御文字をエスケープする`() {
        assertEquals("\"tab\\tcontrol\\u0001\"", UpdateCheck.json("tab\tcontrol\u0001"))
    }
    @Test fun `更新の通知は既存のサーバ更新通知と別のチャンネルと ID を使う`() {
        // Scenario: 携帯の更新の通知は既存のサーバの更新の知らせを置き換えず、文が違う
        assertEquals("update", UpdateCheck.CHANNEL)
        assertEquals("contract", ContractNotifier.CHANNEL)
        assertFalse("新しい版 0.5.0 が出ています" == CONTRACT_UPDATE_HEADLINE)
        assertEquals(
            7,
            setOf(
                ContractNotifier.CONTRACT_NOTIFICATION_ID,
                UpdateCheck.INTRO_NOTIFICATION_ID,
                UpdateCheck.AVAILABLE_NOTIFICATION_ID,
                UpdateCheck.STALE_NOTIFICATION_ID,
                LocationService.SETUP_NOTIFICATION_ID,
                LocationService.DESTINATION_NOTIFICATION_ID,
                ChannelMismatchNotifier.CHANNEL_NOTIFICATION_ID,
            ).size,
        )
    }
    @Test fun `携帯回線でも配布元へ問い合わせる`() {
        // Scenario: 携帯は携帯の回線でも確かめる
        configured()
        val connectivity = app.getSystemService(android.net.ConnectivityManager::class.java)
        shadowOf(connectivity).setActiveNetworkInfo(ShadowNetworkInfo.newInstance(
            android.net.NetworkInfo.DetailedState.CONNECTED, android.net.ConnectivityManager.TYPE_MOBILE, 0, true,
            android.net.NetworkInfo.State.CONNECTED,
        ))
        val (url, seen, done) = releasesServer()
        val checker = UpdateChecker(app, url)
        checker.markIntroShown(now.minusSeconds(25 * 3600))
        assertTrue(checker.check(now))
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertEquals(1, seen.size)
    }

    @Test fun `配布元の状態番号を理由に記録する`() {
        // Scenario: 確かめられない理由を状態の番号で分ける
        mapOf(403 to "rate_limited", 429 to "rate_limited", 404 to "not_found", 500 to "unreadable").forEach { (status, reason) ->
            app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().commit()
            configured(); val (url, _, done) = releasesServer(body = "{}", status = status)
            val checker = UpdateChecker(app, url); checker.markIntroShown(now.minusSeconds(25 * 3600))
            assertTrue(checker.check(now)); assertTrue(done.await(2, TimeUnit.SECONDS))
            assertEquals(reason, app.getSharedPreferences(UpdateCheck.PREFS, 0).getString("last_reason", null))
        }
    }

    @Test fun `LocationService の起動と生存信号は更新確認を呼ぶ`() {
        // Scenario: 携帯の収集アプリは新しい版をサーバに問い合わせない
        configured()
        val (url, seen, done) = releasesServer(2)
        St57UpdateLocationService.destination = url
        app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().putBoolean("intro_shown", true).putString("intro_shown_at", now.minusSeconds(25 * 3600).toString()).commit()
        val controller = Robolectric.buildService(St57UpdateLocationService::class.java, Intent()).create().startCommand(0, 1)
        app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().putString("checked_at", Instant.now().minusSeconds(25 * 3600).toString()).commit()
        controller.get().beatScheduler.fire()
        assertTrue(done.await(2, TimeUnit.SECONDS))
        assertEquals(2, seen.size)
        controller.destroy()
    }
}
