// SPDX-License-Identifier: AGPL-3.0-only
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
import java.util.regex.Pattern
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用（人間が後から動画で見る）。ST06 の**許可しなかったとき**と**後から許可したとき**を 1 本の流れで撮る:
 * 位置を許可しない → 通知は許可 → 利用状況へのアクセスを許さずに戻る → それでも収集は始まっている（常駐の通知）
 * → 後から常駐の通知をたどって利用状況へのアクセスを許可 → 次の契機からアプリ利用が集まる。
 *
 * spec「許可しなくても収集は始まる」「後から許可すると次の契機から集まる」「以後は常駐の通知から同じ画面へたどれる」（design D6）。
 * 「次の契機」はここではアプリを開き直すこと（収集の起動時にその場で取る）。放っておけば 30 分ごとの刻み。
 *
 * **録画のときだけ走る**（`-e recording 1`）。入れたばかりの状態から始める（record-run が入れ直してから起動する）。
 * 結果の札は録画のテストが出す通知（「【録画の結果】」）で、中身はアプリの未送信の置き場を読んだもの。
 */
@RunWith(AndroidJUnit4::class)
class St06LaterGrantRecordingTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)

    @Test
    fun denyThenGrantLaterForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        assertFalse("入れたばかりの状態から始める（位置がもう許可されている）", granted(Manifest.permission.ACCESS_FINE_LOCATION))

        step("1. あしあと。を初めて起動する")
        device.pressHome()
        launchApp()

        step("2. 位置: 許可しない")
        click(DENY)

        step("3. 通知の送信は許可")
        click("^(許可|Allow)$")

        step("4. 利用状況へのアクセス: 許可せずに戻る")
        assertNotNull("利用状況へのアクセスの画面が開かない", device.wait(Until.findObject(By.text(Pattern.compile("あしあと。"))), TIMEOUT_MS))
        hold()
        device.pressBack()

        step("5. それでも収集は始まっている（常駐の通知）")
        assertTrue("前景サービスの通知が出ない（許可しなくても収集は始まるはず）", waitUntil { notificationShown() })
        assertFalse(granted(Manifest.permission.ACCESS_FINE_LOCATION))
        assertFalse(usageAccessGranted())
        showCard(
            "【録画の結果】許可しなかった状態",
            listOf(
                "位置: 未許可 / 利用状況へのアクセス: 未許可 / 通知: 許可",
                "収集のサービス: 動いている（常駐の通知が出ている）",
                "アプリ利用の記録: ${outbox().count { it.logicalSource == APP_USAGE_LOGICAL_SOURCE }} 件（許可が無いので取らない）",
            ),
        )

        step("6. 後から: 常駐の通知をたどって利用状況へのアクセスを許可")
        device.openNotification()
        // 本文（「…記録を集めています」）を押す。タイトルの「あしあと。」を押しても開かなかった（実測 2026-10-03。
        // 手で本文を押すと利用状況へのアクセスの画面が開く）
        val ongoing = device.wait(Until.findObject(By.pkg("com.android.systemui").textContains("記録を集めています")), TIMEOUT_MS)
        assertNotNull("通知の一覧にあしあと。の常駐の通知が出ない", ongoing)
        hold()
        ongoing.click()
        // 通知の一覧が閉じきる前は一覧の「あしあと。」も拾えてしまう（実測: 押す前に消えて StaleObjectException）。
        // 設定画面が前に出るのを待ち、その中の「あしあと。」を押す
        assertTrue("常駐の通知から設定画面が開かない", device.wait(Until.hasObject(By.pkg(SETTINGS_PACKAGE)), TIMEOUT_MS))
        val app = device.wait(Until.findObject(By.pkg(SETTINGS_PACKAGE).text("あしあと。")), TIMEOUT_MS)
        assertNotNull("設定画面にあしあと。が出ない", app)
        hold()
        app.click()
        click("使用状況へのアクセスを許可|Permit usage access")
        assertTrue("利用状況へのアクセスが許可にならない", waitUntil { usageAccessGranted() })
        hold()
        device.pressBack()
        device.pressBack()
        device.pressHome()

        step("7. 次の契機（ここではアプリを開き直す）")
        launchApp()
        // 位置をもう一度求められたら、また断る（2 度目の起動でも収集は始まる）
        device.wait(Until.findObject(By.text(Pattern.compile(DENY))), 5_000)?.let { hold(); it.click() }
        hold(3_000)
        assertFalse("2 度目の起動で設定画面へ送られた", device.currentPackageName == SETTINGS_PACKAGE)
        device.pressHome()

        step("8. 許可した後のアプリ利用が集まった")
        assertTrue("アプリ利用の記録が未送信に積まれない", waitUntil(30_000) { outbox().any { it.logicalSource == APP_USAGE_LOGICAL_SOURCE } })
        showCard("【録画の結果】後から許可した後", summary())
    }

    private fun launchApp() =
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

    private fun usageAccessGranted() = usageAccessCapability(context).blockers.none { it == Capability.PERMISSION }

    /** 未送信の置き場を読む（アプリと同じ置き場・同じ形。`ClockSkewInstrumentedTest` と同じ読み方） */
    private fun outbox(): List<IngestRequest> {
        val dir = File(context.filesDir, LocationService.OUTBOX_DIR)
        return SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, LocationService.UNREADABLE), {}).readAll()
    }

    private fun summary(): List<String> {
        val all = outbox()
        val apps = all.filter { it.logicalSource == APP_USAGE_LOGICAL_SOURCE }.groupingBy { r ->
            (r.payload["app_label"] ?: r.payload["package"])?.jsonPrimitive?.content ?: "?"
        }.eachCount().entries.sortedByDescending { it.value }.take(5)
        return listOf(
            "記録の種類ごとの件数: " + all.groupingBy { it.logicalSource }.eachCount().entries.joinToString(" / ") { "${it.key} ${it.value}" },
            "アプリ利用（イベント）の多いアプリ: " + apps.joinToString(" / ") { "${it.key} ${it.value}" },
            "位置: 未許可のまま（位置の記録は増えない）",
            "送信は 5 分ごと（この動画の間にはサーバへ送らない）",
        )
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
        const val DENY = "^(許可しない|Don.t allow)$"
        const val SETTINGS_PACKAGE = "com.android.settings"
        const val CARD_CHANNEL = "recording-card"
        const val CARD_ID = 9_002
    }
}
