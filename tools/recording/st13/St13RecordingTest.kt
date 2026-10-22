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
import java.util.regex.Pattern
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用（人間が後から動画で見る）。ST13「携帯の収集アプリが使っていないアプリの一時停止の対象から外れるよう頼む」
 * （design D12（仮））を 1 本の流れで撮る:
 * 対象のままの端末で開く → 画面に 1 行と「設定を開く」・説明のダイアログ（自動では 1 度だけ）→ 応じると設定の画面
 * → 戻るとダイアログは出ず 1 行とボタンだけ → ボタンで設定の画面 → 一時停止の対象から外す → 戻ると画面が閉じる
 * → 結果の札（読み口が読んだ状態の移り変わり）。
 *
 * **初めの状態は shell で揃える。** adb で入れたアプリの除外の状態は端末の版によって違いうるので、
 * `appops set … AUTO_REVOKE_PERMISSIONS_IF_UNUSED allow`（対象のまま）から始める。外す操作は設定の画面のスイッチを押し、
 * スイッチが見つからない版の設定では同じ appops を `ignore` にして代える（どちらで外したかは結果の札に出す）。
 * 位置・通知の権限は先に許可し、利用状況へのアクセスへ送る 1 度だけの印も先に立てる（導線は ST06 の録画が撮る）。
 *
 * **録画のときだけ走る**（`-e recording 1`）。
 */
@RunWith(AndroidJUnit4::class)
class St13RecordingTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
    )

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val prefs get() = context.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE)

    @After
    fun tearDown() {
        context.getSystemService(NotificationManager::class.java).cancel(CARD_ID)
    }

    @Test
    fun unusedAppRestrictionsForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        shell("appops set ${context.packageName} AUTO_REVOKE_PERMISSIONS_IF_UNUSED allow")
        prefs.edit().clear().putBoolean(MainActivity.SENT_TO_USAGE_ACCESS, true).commit()
        val before = SystemUnusedAppRestrictionsReader.read(context)
        assertEquals("対象のままから始められない", UnusedAppRestrictionsState.NOT_EXEMPT, before)
        device.pressHome()

        step("1. 一時停止の対象のまま、あしあと。を開く: 画面に 1 行と「設定を開く」、説明のダイアログ")
        launch()
        assertNotNull("説明のダイアログが出ない", device.wait(Until.findObject(By.textContains(EXPLANATION_HEAD)), TIMEOUT_MS))
        // 裏の 1 行はダイアログの窓の後ろにあり、UiAutomator は前面の窓しか読まない。1 行は工程 3 で確かめる
        assertTrue("「あとで」が無い", device.hasObject(By.text(MainActivity.NOT_NOW_LABEL)))
        assertFalse("答える前に印が立っている", asked())
        hold(4_000)

        step("2. ダイアログの「設定を開く」→ 設定の画面（あしあと。のアプリ情報）")
        device.findObject(By.res("android:id/button1")).click()
        assertTrue("設定の画面が開かない", waitUntil { device.currentPackageName == SETTINGS_PACKAGE })
        assertTrue("応じたのに印が立たない", asked())
        hold(3_000)

        step("3. 外さずに戻る: ダイアログはもう自動では出ず、1 行と「設定を開く」だけが残る")
        device.pressBack()
        assertNotNull("戻っても 1 行が出ない", device.wait(Until.findObject(By.text(MainActivity.NOT_EXEMPT_LINE)), TIMEOUT_MS))
        hold()
        assertFalse("2 回目に案内が自動で出た", device.hasObject(By.textContains(EXPLANATION_HEAD)))
        hold(3_000)

        step("4. 画面の「設定を開く」→ 設定の画面で、一時停止の対象から外す")
        device.findObject(By.text(MainActivity.OPEN_SETTINGS_LABEL)).click()
        assertTrue("ボタンで設定の画面が開かない", waitUntil { device.currentPackageName == SETTINGS_PACKAGE })
        hold()
        val how = exemptInSettings()
        assertTrue("外れない", waitUntil { SystemUnusedAppRestrictionsReader.read(context) == UnusedAppRestrictionsState.EXEMPT })
        hold(3_000)

        step("5. 戻る: 外れたので 1 行は出ず、画面は閉じる（収集は常駐のまま）")
        device.pressBack()
        assertTrue(
            "外れたのに画面が閉じない",
            waitUntil { !device.hasObject(By.text(MainActivity.NOT_EXEMPT_LINE)) && device.currentPackageName != context.packageName },
        )
        hold(2_000)

        step("6. 結果: 読み口が読んだ状態（生存信号の原文の unused_app_restrictions と同じ値）")
        showCard(
            "【録画の結果】使っていないアプリの一時停止",
            listOf(
                "開いたとき: ${before.wire}（対象のまま）→ 案内した",
                "外した後: ${SystemUnusedAppRestrictionsReader.read(context).wire}（対象から外れた）→ 何も出さない",
                "外し方: $how",
            ),
        )
    }

    private fun launch() {
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    private fun asked() = prefs.getBoolean(UnusedAppRestrictionsAsked.KEY, false)

    /** 設定の画面のスイッチを探して押す。見つからない版の設定では appops で代える。どちらで外したかを返す。 */
    private fun exemptInSettings(): String {
        val label = By.text(TOGGLE_LABEL)
        val found = device.wait(Until.findObject(label), 3_000)
            ?: device.findObject(By.scrollable(true))?.scrollUntil(Direction.DOWN, Until.findObject(label))
        if (found != null) {
            hold()
            found.click()
            if (waitUntil(5_000) { SystemUnusedAppRestrictionsReader.read(context) == UnusedAppRestrictionsState.EXEMPT }) {
                return "設定の画面のスイッチ（${found.text}）"
            }
        }
        shell("appops set ${context.packageName} AUTO_REVOKE_PERMISSIONS_IF_UNUSED ignore")
        return "設定のスイッチが見つからない版なので shell（appops … ignore）で代えた"
    }

    private fun shell(cmd: String): String =
        instrumentation.uiAutomation.executeShellCommand(cmd)
            .let { FileInputStream(it.fileDescriptor) }
            .use { it.readBytes().toString(Charsets.UTF_8) }

    /** 結果の札を通知で出し、通知の一覧を開いて見せる（`St05RecordingTest` と同じ）。 */
    private fun showCard(title: String, lines: List<String>) {
        val nm = context.getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel(CARD_CHANNEL, "録画の結果", NotificationManager.IMPORTANCE_HIGH))
        nm.notify(
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
        nm.cancel(CARD_ID)
    }

    private fun step(label: String) {
        instrumentation.runOnMainSync { Toast.makeText(context, label, Toast.LENGTH_LONG).show() }
        hold()
    }

    private fun hold(ms: Long = HOLD_MS) = Thread.sleep(ms)

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
        const val CARD_ID = 9_013
        /** 説明のダイアログの書き出し（`MainActivity.UNUSED_APP_RESTRICTIONS_EXPLANATION`） */
        val EXPLANATION_HEAD = MainActivity.UNUSED_APP_RESTRICTIONS_EXPLANATION.take(12)
        /** アプリ情報の画面のスイッチ（日本語・英語の版。Android 15 の日本語は「使用していないアプリを管理する」と実測） */
        val TOGGLE_LABEL: Pattern = Pattern.compile(
            "使用していないアプリを管理する|.*(使用されていない|使っていない|未使用).*(一時停止|権限を削除).*|" +
                "Pause app activity if unused|Remove permissions if app isn.t used|Manage app if unused",
        )
    }
}
