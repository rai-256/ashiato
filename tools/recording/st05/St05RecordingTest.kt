// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.os.SystemClock
import android.provider.Settings
import android.widget.Toast
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.io.File
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import org.junit.After
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用（人間が後から動画で見る）。ST05「端末時計のずれを測って残す」を 1 本の流れで撮る:
 * 収集を始める → その場で 1 回測る（結果の札）→ 端末の時計を 5 分進める → その場で測り直す（結果の札。差が約 +300000 ms）
 * → 時計を戻す。
 *
 * ST05 には画面が無い。**結果の札**は録画のテストが出す通知（「【録画の結果】」）で、中身はアプリの未送信の置き場
 * （`filesDir/outbox`）にある測定記録（`c01-clock`）を読んだもの。送信は 5 分ごとなので、この動画の間にはサーバへ届かない
 * （届くことは smoke が持つ）。時計は `ClockSkewInstrumentedTest` と同じく `cmd alarm set-time` で変え、終わりに戻す。
 * PC（Windows）の時計は動かさない —— その PC の全部に効くので、Windows の実行時テストに任せる。
 *
 * 権限の導線は撮らない（ST06 の録画が撮る）。位置・通知は先に許可しておく。
 * **録画のときだけ走る**（`-e recording 1`）。
 */
@RunWith(AndroidJUnit4::class)
class St05RecordingTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
    )

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)

    private var restoreClockAtMs: Long? = null
    private var clockChangedAtElapsedMs = 0L

    @After
    fun restoreClock() {
        restoreClockAtMs?.let { shell("cmd alarm set-time ${it + (SystemClock.elapsedRealtime() - clockChangedAtElapsedMs)}") }
        shell("settings put global auto_time 1")
    }

    @Test
    fun clockSkewForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")

        step("1. あしあと。を起動する（収集を始めると、その場で 1 回測る）")
        device.pressHome()
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        hold(3_000)
        // ST06 以降は初回起動で利用状況へのアクセスの設定画面へ 1 度送られる。ここでは許可せずに戻る
        if (device.currentPackageName == SETTINGS_PACKAGE) device.pressBack()
        device.pressHome()
        assertTrue("起動時の測定記録（trigger=start）が積まれない", waitUntil { clock().any { trigger(it) == "start" } })
        showCard("【録画の結果】起動時の測定", describe(clock().first { trigger(it) == "start" }))

        step("2. 端末の時計を 5 分進める（自動の時刻設定を切る）")
        context.startActivity(Intent(Settings.ACTION_DATE_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        hold(3_000)
        val before = clock().count { trigger(it) == "time_set" }
        shell("settings put global auto_time 0")
        hold()
        val now = System.currentTimeMillis()
        restoreClockAtMs = now
        clockChangedAtElapsedMs = SystemClock.elapsedRealtime()
        shell("cmd alarm set-time ${now + FIVE_MINUTES_MS}")
        hold(4_000)   // 設定画面の時刻と上の時計が 5 分進むのを見せる
        device.pressHome()

        step("3. 時計が変わった直後に、その場で測り直した")
        assertTrue("時計を変えた後の測定記録（trigger=time_set）が積まれない", waitUntil { clock().count { trigger(it) == "time_set" } > before })
        val after = clock().filter { trigger(it) == "time_set" }.last()
        val skews = references(after).map { it.second }
        // 基準が 1 つでも取れていれば、その差は 5 分前後（spec「端末の時計を 5 分進めると差が約 300000 ミリ秒で残る」）
        assertTrue("差が 5 分前後でない: $skews", skews.isEmpty() || skews.any { it in 270_000L..330_000L })
        showCard("【録画の結果】時計を 5 分進めた直後の測定", describe(after))

        step("4. 時計を元に戻す（自動の時刻設定も戻す）")
        restoreClock()
        restoreClockAtMs = null
        hold(3_000)
    }

    private fun shell(command: String) {
        instrumentation.uiAutomation.executeShellCommand(command).use { fd ->
            java.io.FileInputStream(fd.fileDescriptor).readBytes()
        }
    }

    /** 未送信の置き場の測定記録（アプリと同じ置き場・同じ形。`ClockSkewInstrumentedTest` と同じ読み方） */
    private fun clock(): List<IngestRequest> {
        val dir = File(context.filesDir, LocationService.OUTBOX_DIR)
        return SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, LocationService.UNREADABLE), {})
            .readAll()
            .filter { it.logicalSource == CLOCK_LOGICAL_SOURCE }
    }

    private fun trigger(r: IngestRequest) = r.payload["trigger"]!!.jsonPrimitive.content

    private fun references(r: IngestRequest): List<Pair<String, Long>> =
        (r.payload["references"] as JsonArray).map {
            it.jsonObject["source"]!!.jsonPrimitive.content to it.jsonObject["skew_ms"]!!.jsonPrimitive.long
        }

    private fun describe(r: IngestRequest): List<String> {
        val taken = references(r).map { (source, skew) -> "$source: 差 ${"%+,d".format(skew)} ms" }
        val missed = (r.payload["unavailable"] as JsonArray).map {
            "${it.jsonObject["source"]!!.jsonPrimitive.content}: 取れない（${it.jsonObject["reason"]!!.jsonPrimitive.content}）"
        }
        return listOf("端末の時計: ${r.payload["device_time"]!!.jsonPrimitive.content}（契機: ${trigger(r)}）") +
            taken.ifEmpty { listOf("取れた基準: なし") } + missed +
            "差 = 端末の時計 − 基準（正なら端末が進んでいる）"
    }

    /** 結果の札を通知で出し、通知の一覧を開いて見せる（アプリには画面が無いので） */
    private fun showCard(title: String, lines: List<String>) {
        val nm = context.getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel(CARD_CHANNEL, "録画の結果", NotificationManager.IMPORTANCE_HIGH))
        nm.notify(
            CARD_ID,
            Notification.Builder(context, CARD_CHANNEL)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setContentTitle(title)
                .setContentText(lines.first())
                // アプリの常駐の通知と一緒に畳まれないよう、札だけの束にする（実測: 束ねられて 1 行しか見えなかった）
                .setGroup(CARD_CHANNEL)
                .setStyle(Notification.BigTextStyle().bigText(lines.joinToString("\n")))
                .build(),
        )
        device.openNotification()
        assertNotNull("結果の札が通知の一覧に出ない", device.wait(Until.findObject(By.text(title)), TIMEOUT_MS))
        // 札を広げて全行を見せる。広がっていなければ札の展開の印を押す
        val lastLine = By.textContains(lines.last().take(12))
        if (!device.hasObject(lastLine)) {
            // 1. 札を下へ引く（SystemUI の広げる操作）→ 2. だめなら札の展開の印を押す
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
        // 見せ終えたら消す。札もアプリの名前（「あしあと。」）で出るので、残すと常駐の通知と取り違える（実測）
        nm.cancel(CARD_ID)
    }

    private fun step(label: String) {
        instrumentation.runOnMainSync { Toast.makeText(context, label, Toast.LENGTH_LONG).show() }
        hold()
    }

    private fun hold(ms: Long = HOLD_MS) = Thread.sleep(ms)

    private fun waitUntil(timeoutMs: Long = 30_000, check: () -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (check()) return true
            Thread.sleep(500)
        }
        return false
    }

    private companion object {
        const val TIMEOUT_MS = 15_000L
        const val HOLD_MS = 1_500L
        const val FIVE_MINUTES_MS = 5 * 60 * 1000L
        const val SETTINGS_PACKAGE = "com.android.settings"
        const val CARD_CHANNEL = "recording-card"
        const val CARD_ID = 9_003
    }
}
