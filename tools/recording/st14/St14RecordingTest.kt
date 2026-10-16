// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.widget.Toast
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.After
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用（人間が後から動画で見る）。ST14「収集の途切れを携帯に知らせる」の通知を 1 本の流れで撮る:
 * 位置が途切れる → 「収集が途切れています」の通知（表示名と最後に生きていた日時。記録の中身は載せない）→
 * 写真も途切れる → 2 つを並べた通知に差し替わる → 全部戻る → 通知が黙って消える。
 *
 * **サーバの判定は撮らない。** 入れたばかりの端末には途切れの履歴が無く、録画の数分では途切れにならない
 * （判定と `GET /outages` の応答はサーバの試験と `tools/smoke.sh` が持つ）。ここでは `GET /outages` の応答
 * （サーバの `OutagesResponse` と同じ形）を返す偽の問い合わせを、アプリと同じ [OutagePoller] → [OutageNotice] →
 * [OutageNotifier] に通す —— 通知の中身・差し替え・消え方はアプリのコードのまま。覚え（鳴らした途切れ）は
 * アプリの覚えと混ざらないよう録画専用の置き場に書き、終わりに消す。
 *
 * 通知の権限は先に許可しておく（導線は ST06 の録画が撮る）。**録画のときだけ走る**（`-e recording 1`）。
 */
@RunWith(AndroidJUnit4::class)
class St14RecordingTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(Manifest.permission.POST_NOTIFICATIONS)

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    private val executor = Executors.newSingleThreadExecutor()
    private var response = ""
    private val poller by lazy {
        OutagePoller(
            query = { Outcome.Responded(200, response) },
            notice = OutageNotice(context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)),
            notifier = OutageNotifier(context),
            deviceId = { "st14-recording" },
            executor = executor,
        )
    }

    @After
    fun cleanUp() {
        executor.shutdownNow()
        manager.cancel(OutageNotifier.OUTAGE_NOTIFICATION_ID)
        context.deleteSharedPreferences(PREFS)
    }

    @Test
    fun outageNoticeForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        context.deleteSharedPreferences(PREFS)
        device.pressHome()

        step("1. 位置の記録がサーバに届かなくなった（途切れ）→ 通知が出る（音あり）")
        pollWith(ready(LOCATION))
        val location = "位置（最後 10/12 08:15）"
        showShade(location)

        step("2. 写真も途切れた → 2 つを並べた通知に差し替わる（新しい途切れなのでまた鳴る）")
        pollWith(ready(LOCATION, PHOTO))
        showShade("$location、写真（最後 10/11 22:00）")

        step("3. 全部戻った → 通知は黙って消える")
        pollWith(READY_EMPTY)
        device.openNotification()
        assertTrue("戻ったのに通知が残っている", device.wait(Until.gone(By.text(OutageNotifier.TITLE)), TIMEOUT_MS))
        hold(4_000)
        device.pressBack()
    }

    /** 契機 1 回ぶん。アプリと同じ直列の糸で問い合わせ → 決め方 → 通知 → 覚えまで済ませる */
    private fun pollWith(body: String) {
        response = body
        poller.poll()
        executor.submit {}.get(TIMEOUT_MS, TimeUnit.MILLISECONDS)
    }

    /** 通知の一覧を開いて、途切れの通知と本文を見せる */
    private fun showShade(text: String) {
        device.openNotification()
        assertNotNull("途切れの通知が通知の一覧に出ない", device.wait(Until.findObject(By.text(OutageNotifier.TITLE)), TIMEOUT_MS))
        assertNotNull("通知の本文が違う: $text", device.wait(Until.findObject(By.textContains(text)), TIMEOUT_MS))
        hold(6_000)
        device.pressBack()
    }

    private fun step(label: String) {
        instrumentation.runOnMainSync { Toast.makeText(context, label, Toast.LENGTH_LONG).show() }
        hold()
    }

    private fun hold(ms: Long = HOLD_MS) = Thread.sleep(ms)

    private companion object {
        const val TIMEOUT_MS = 15_000L
        const val HOLD_MS = 2_500L
        const val PREFS = "outage_notice_recording"

        // `GET /outages` の応答（サーバの `OutagesResponse` と同じ形。`OutageFixtures` と同じ書き方）
        const val LOCATION = """{"episode_id":1,"logical_source":"c01-location","display_name":"位置",""" +
            """"last_alive_at":"2026-10-11T23:15:00Z"}"""
        const val PHOTO = """{"episode_id":2,"logical_source":"c01-photo","display_name":"写真",""" +
            """"last_alive_at":"2026-10-11T13:00:00Z"}"""
        const val READY_EMPTY = """{"status":"ready","judged_at":"2026-10-12T01:00:00Z","outages":[]}"""

        fun ready(vararg outages: String) =
            """{"status":"ready","judged_at":"2026-10-12T00:00:00Z","outages":[${outages.joinToString(",")}]}"""
    }
}
