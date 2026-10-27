// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.util.regex.Pattern
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用（人間が後から動画で見る）。ST11「健康データの権限を履歴ごと求める」の端末の側を 1 本の流れで撮る:
 * 初めて起動 → 位置・通知・写真と動画の後に Health Connect の権限の画面 → プライバシーポリシーから説明の画面 →
 * 4 つの種類（睡眠・心拍数・歩数・エクササイズ）をすべて許可 → 過去のデータとバックグラウンドのアクセスも許可 →
 * 収集が始まる → 常駐の通知の「健康データの権限」から Health Connect の設定 → アプリの権限 → あしあと。で許した種類が並ぶ →
 * 許した状態と観た時刻の記録が未送信に積まれている（結果の札）。
 *
 * 位置は断る（位置の流れは ST06 の録画）。写真と動画は許可する（断ったときの知らせは ST09 の録画）。
 * 断る側（履歴だけ断る・すべて断る）は計測テスト（`HealthPermission*DeniedInstrumentedTest`）が持つ ——
 * 2 回断ると画面が出なくなる（P6）ので、許す流れと同じ 1 本には入れない。
 * 送信は 5 分ごとなので動画の間にサーバへは届かない（格納は `tools/smoke.d/` と Rust の試験が持つ）。
 *
 * **録画のときだけ走る**（`-e recording 1`）。入れたばかりの状態から始める（record-run が入れ直してから起動する）。
 * 端末の言語は日本語（recording.json の android.locale）を前提に、英語の文言も受ける。
 */
@RunWith(AndroidJUnit4::class)
class St11RecordingTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)
    private val screens get() = HealthConnectScreens(context)

    @After
    fun cleanUp() {
        manager.cancel(CARD_ID)
    }

    @Test
    fun healthPermissionFlowForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        assertFalse("入れたばかりの状態から始める（健康データの権限がもう許可されている）", screens.grantedCount() > 0)

        step("1. あしあと。を初めて起動する")
        device.pressHome()
        launchApp()

        step("2. 位置: 許可しない（位置の流れは ST06 の録画）")
        click(DENY)

        step("3. 通知の送信は許可")
        click("^(許可|Allow)$")

        step("4. 写真と動画はすべて許可（断る流れは ST09 の録画）")
        click("^(すべて許可|Allow all)$")

        step("5. 写真と動画の後に、健康データの権限の画面が出る")
        // 端末で初めて Health Connect を使うときは「はじめに」が 1 度出る（P7）
        device.wait(Until.findObject(GET_STARTED), 5_000)?.click()
        assertNotNull(
            "健康データの権限の画面が出ない（前面: ${device.currentPackageName}）",
            device.wait(Until.findObject(ALLOW_ALL), TIMEOUT_MS),
        )
        hold(3_000)

        step("6. プライバシーポリシーを開くと、読む種類・送る先・いまは読まないことの説明が出る")
        openPrivacyPolicy()
        assertNotNull(
            "説明の画面が出ない",
            device.wait(Until.findObject(By.textContains(HEALTH_RATIONALE.substringBefore("。"))), TIMEOUT_MS),
        )
        hold(5_000)
        device.pressBack()

        step("7. 4 つの種類（睡眠・心拍数・歩数・エクササイズ）をすべて許可")
        val allowAll = device.wait(Until.findObject(ALLOW_ALL), TIMEOUT_MS)
        assertNotNull("権限の画面に戻らない（前面: ${device.currentPackageName}）", allowAll)
        // 種類の並びを見せる（画面の下に続く）
        device.findObject(By.scrollable(true))?.scroll(Direction.DOWN, 1.0f)
        for (kind in KINDS) {
            assertNotNull("種類「$kind」が並ばない", device.wait(Until.findObject(By.text(Pattern.compile(kind))), TIMEOUT_MS))
        }
        hold()
        click(ALLOW_ALL)
        hold()
        click(ALLOW)

        step("8. 過去のデータとバックグラウンドのアクセスも許可")
        click(By.text(HealthConnectScreens.HISTORY_LABEL))
        click(By.text(HealthConnectScreens.BACKGROUND_LABEL))
        hold()
        click(ALLOW)
        assertTrue(
            "6 つ（4 つの種類・過去・背景）が許された状態にならない",
            screens.waitForGrantedCount(HEALTH_PERMISSIONS.size) == HEALTH_PERMISSIONS.size,
        )

        step("9. 収集が始まる（利用状況へのアクセスの画面は戻る。ST06）")
        assertTrue("前景サービスの通知が出ない", screens.waitForCollectionStarted())
        device.wait(Until.hasObject(By.pkg(SETTINGS_PACKAGE)), TIMEOUT_MS)
        hold()
        device.pressBack()
        // 使っていないアプリの一時停止の案内（ST06）は「あとで」
        device.wait(Until.findObject(By.text(Pattern.compile("^(あとで|Not now)$"))), 5_000)?.let { hold(); it.click() }
        device.pressHome()

        step("10. 常駐の通知の「健康データの権限」から Health Connect の設定を開く")
        screens.clickOngoingAction(HEALTH_PERMISSION_ACTION_LABEL)
        assertNotNull(
            "Health Connect の設定が開かない（前面: ${device.currentPackageName}）",
            device.wait(Until.findObject(HealthConnectScreens.APP_PERMISSIONS), TIMEOUT_MS),
        )
        hold(3_000)

        step("11. アプリの権限 → あしあと。で、許した種類が並ぶ")
        click(HealthConnectScreens.APP_PERMISSIONS)
        click(HealthConnectScreens.APP_LABEL)
        assertNotNull(
            "収集アプリの権限の画面までたどれない（前面: ${device.currentPackageName}）",
            device.wait(Until.findObject(By.text(Pattern.compile(KINDS.first()))), TIMEOUT_MS),
        )
        hold(5_000)
        device.pressHome()

        step("12. 許した状態と観た時刻の記録が未送信に積まれている")
        assertTrue(
            "すべて許された記録が未送信に積まれない",
            waitUntil { screens.healthRecords().lastOrNull()?.let { screens.grantedFlags(it).values.all { v -> v } } == true },
        )
        showCard("【録画の結果】健康データの権限の記録（未送信）", summary())
    }

    /**
     * 「プライバシー ポリシー」の案内を押す。案内は文の中の 1 語だけが押せる。英語は最後の行にあるが（[HealthConnectScreens.openPrivacyPolicy]）、
     * 日本語は 3 行の真ん中の行の右寄りにある（2026-10-27 実測）ので、真ん中の行から押していく。
     */
    private fun openPrivacyPolicy() {
        val line = device.wait(Until.findObject(HealthConnectScreens.PRIVACY_POLICY), TIMEOUT_MS)
        assertNotNull("プライバシーポリシーの案内が権限の画面に無い（前面: ${device.currentPackageName}）", line)
        val b = line.visibleBounds
        for (y in listOf(b.centerY(), b.bottom - b.height() / 6)) {
            for (f in listOf(0.65, 0.5, 0.8, 0.35)) {
                device.click((b.left + b.width() * f).toInt(), y)
                if (device.wait(Until.hasObject(By.pkg(context.packageName)), 2_000)) return
            }
        }
        throw AssertionError("プライバシーポリシーの案内を押しても説明の画面が出ない（前面: ${device.currentPackageName}）")
    }

    private fun launchApp() =
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

    private fun summary(): List<String> {
        val records = screens.healthRecords()
        val last = records.last()
        val flags = screens.grantedFlags(last)
        return listOf(
            "記録: ${records.size} 件（${last.logicalSource}）",
            "観た時刻: ${last.eventTime}",
            "許された: " + flags.entries.joinToString(" / ") { "${it.key} ${if (it.value) "○" else "×"}" },
            "送信は 5 分ごと（この動画の間にはサーバへ送らない）",
        )
    }

    /** 結果の札を通知で出し、通知の一覧を開いて見せる（アプリには画面が無いので。St09 の録画と同じ出し方） */
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

    private fun click(text: String) = click(By.text(Pattern.compile(text)))

    private fun click(selector: androidx.test.uiautomator.BySelector) {
        val target = device.wait(Until.findObject(selector), TIMEOUT_MS)
        assertNotNull("「$selector」が画面に出ない（前面: ${device.currentPackageName}）", target)
        hold()
        target.click()
    }

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
        const val CARD_ID = 9_011
        val KINDS = listOf("^(睡眠|Sleep)$", "^(心拍数|Heart rate)$", "^(歩数|Steps)$", "^(エクササイズ|Exercise)$")
        val GET_STARTED = By.text(Pattern.compile("^(開始|使ってみる|Get started)$"))
        val ALLOW_ALL = By.text(Pattern.compile("^(すべて許可|Allow all)$"))
        val ALLOW = By.text(Pattern.compile("^(許可|Allow)$"))
    }
}
