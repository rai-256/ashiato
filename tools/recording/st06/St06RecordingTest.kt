// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.widget.Toast
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.io.File
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.regex.Pattern

/**
 * 録画専用（人間が後から動画で見る）。ST06 の初回起動の導線を 1 本の流れで撮る:
 * 位置（使用中）→ 位置（常に）→ 通知 → 利用状況へのアクセス → 常駐の通知
 * → もう一度開いても設定画面へは送られない → 端末に集まったアプリ利用（結果の札）。
 *
 * **結果の札**は録画のテストが出す通知（「【録画の結果】」）。中身はアプリの未送信の置き場（`filesDir/outbox`）を
 * 読んだもの —— 送信は 5 分ごとなので、この動画の間にはサーバへ届かない（届くことは計測テストと smoke が持つ）。
 *
 * **録画のときだけ走る**（`-e recording 1`）。通常の計測テストでは飛ばす ——
 * 権限が未許可・未要求の「入れたばかり」から始める必要があり、外で入れ直してから起動する。
 * 端末の言語は日本語を前提に、英語の文言も受ける。各工程の見出しは Toast で画面に出す（動画の目印）。
 *
 * 置き場は `tools/recording/st06/`（宣言は `tools/recording/recording.json`）。録画はハーネスの `scripts/record-run` が
 * 対象コミットの androidTest へ写し、エミュレータを入れたばかりの状態から起こして走らせる。
 */
@RunWith(AndroidJUnit4::class)
class St06RecordingTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)

    @Test
    fun firstLaunchFlowForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        assertFalse("入れたばかりの状態から始める（位置がもう許可されている）", granted(Manifest.permission.ACCESS_FINE_LOCATION))

        step("1. あしあと。を初めて起動する")
        device.pressHome()
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

        step("2. 位置: アプリの使用時のみ許可")
        click("アプリの使用時のみ|While using the app")

        step("3. 位置: 常に許可（背景の位置）")
        click("常に許可|Allow all the time")
        hold()
        device.pressBack()

        step("4. 通知の送信を許可")
        click("許可|Allow")

        step("5. 利用状況へのアクセス: あしあと。を許可")
        click("あしあと。")
        click("使用状況へのアクセスを許可|Permit usage access")
        assertTrue("利用状況へのアクセスが許可にならない", waitUntil { usageAccessCapability(context).blockers.none { it == Capability.PERMISSION } })
        hold()
        device.pressBack()
        device.pressBack()

        step("6. 常駐の通知（収集中）")
        assertTrue("前景サービスの通知が出ない", waitUntil { notificationShown() })
        device.openNotification()
        // 通知の一覧は systemui が描くので、パッケージではなく通知のタイトル（LocationService の「あしあと。」）で探す
        assertNotNull(
            "通知の一覧にあしあと。が出ない",
            device.wait(Until.findObject(By.pkg("com.android.systemui").text(Pattern.compile("あしあと。.*"))), TIMEOUT_MS),
        )
        hold(3_000)
        device.pressBack()

        step("7. 結果: 位置（常に）・通知・利用状況のアクセスが揃った")
        assertTrue(granted(Manifest.permission.ACCESS_FINE_LOCATION))
        assertTrue(granted(Manifest.permission.ACCESS_BACKGROUND_LOCATION))
        assertTrue(granted(Manifest.permission.POST_NOTIFICATIONS))
        assertTrue(notificationShown())
        device.pressHome()
        hold()

        // spec「2 度目の起動では自動で送られない」。開き直しは収集の契機にもなる（onStartCommand がその場で取る）
        step("8. もう一度開く: 設定画面へは送られない")
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        hold(3_000)
        assertFalse("2 度目の起動で設定画面へ送られた", device.currentPackageName == SETTINGS_PACKAGE)
        device.pressHome()

        step("9. 端末に集まったアプリ利用")
        assertTrue("アプリ利用の記録が未送信に積まれない", waitUntil(30_000) { outbox().any { it.logicalSource == APP_USAGE_LOGICAL_SOURCE } })
        showCard("【録画の結果】端末に集まった記録", summary())
    }

    /** 未送信の置き場を読む（アプリと同じ置き場・同じ形。`ClockSkewInstrumentedTest` と同じ読み方） */
    private fun outbox(): List<IngestRequest> {
        val dir = File(context.filesDir, LocationService.OUTBOX_DIR)
        return SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, LocationService.UNREADABLE), {}).readAll()
    }

    private fun summary(): List<String> {
        val all = outbox()
        val usage = all.filter { it.logicalSource == APP_USAGE_LOGICAL_SOURCE }
        val apps = usage.groupingBy { r ->
            (r.payload["app_label"] ?: r.payload["package"])?.jsonPrimitive?.content ?: "?"
        }.eachCount().entries.sortedByDescending { it.value }.take(5)
        return listOf("記録の種類ごとの件数: " + all.groupingBy { it.logicalSource }.eachCount().entries.joinToString(" / ") { "${it.key} ${it.value}" }) +
            "アプリ利用（イベント）の多いアプリ: " + apps.joinToString(" / ") { "${it.key} ${it.value}" } +
            "送信は 5 分ごと（この動画の間にはサーバへ送らない）"
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

    private fun click(text: String) {
        val target = device.wait(Until.findObject(By.text(Pattern.compile(text))), TIMEOUT_MS)
        assertNotNull("「$text」が画面に出ない（前面: ${device.currentPackageName}）", target)
        hold()
        target.click()
    }

    private fun granted(permission: String) =
        context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

    private fun notificationShown() = context.getSystemService(NotificationManager::class.java)
        .activeNotifications.any { it.id == LocationService.NOTIFICATION_ID }

    private fun waitUntil(timeoutMs: Long = TIMEOUT_MS, check: () -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (check()) return true
            Thread.sleep(250)
        }
        return false
    }

    private companion object {
        const val TIMEOUT_MS = 15_000L
        const val HOLD_MS = 1_500L
        const val SETTINGS_PACKAGE = "com.android.settings"
        const val CARD_CHANNEL = "recording-card"
        const val CARD_ID = 9_001
    }
}
