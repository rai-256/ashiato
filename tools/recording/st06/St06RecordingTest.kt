// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.widget.Toast
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.regex.Pattern

/**
 * 録画専用（人間が後から動画で見る）。ST06 の初回起動の導線を 1 本の流れで撮る:
 * 位置（使用中）→ 位置（常に）→ 通知 → 利用状況へのアクセス → 常駐の通知。
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

    private fun waitUntil(check: () -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + TIMEOUT_MS
        while (System.currentTimeMillis() < deadline) {
            if (check()) return true
            Thread.sleep(250)
        }
        return false
    }

    private companion object {
        const val TIMEOUT_MS = 15_000L
        const val HOLD_MS = 1_500L
    }
}
