// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.os.ParcelFileDescriptor
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import org.junit.After
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 休止の対象のままの端末で、画面の 1 行と「設定を開く」が**本物の端末で見えて押せる**こと（ST13 / design D12（仮））。
 *
 * Robolectric は窓の帯（状態の帯・題の帯）を描かないので、中身が帯の裏に隠れても緑のままだった。
 * 録画で実測: Android 15（targetSdk 36 で全面表示が強いられる）では 1 行もボタンも題の帯の裏に入り、
 * 押しても設定の画面が開かなかった。ここでは本物の framework で押して、設定の画面が前に出ることを見る。
 */
@RunWith(AndroidJUnit4::class)
class UnusedAppRestrictionsInstrumentedTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
        // 入口が位置の後に写真と動画の権限も求める（ST09 design D6）。許可済みにしないと権限の画面が 1 行を覆う ——
        // suite の並びで先に写真の試験が許可を残したときだけ通っていた（単独で走らせると毎回落ちる）
        *mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).toTypedArray(),
    )

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)

    @After
    fun tearDown() {
        shell("appops set ${context.packageName} AUTO_REVOKE_PERMISSIONS_IF_UNUSED default")
        context.stopService(Intent(context, LocationService::class.java))
        // 印は後に走る計測テスト（1 回の install で suite 全部が走る）へ持ち込まない
        context.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE).edit().clear().commit()
        // 設定の画面は MainActivity と**同じ task** に積まれる（`FLAG_ACTIVITY_NEW_TASK` を付けずに開くため）。
        // 残すと、次のテストの `startActivity(NEW_TASK)` はこの task を前に出すだけで、上に居る設定の画面が
        // 前面になり MainActivity は再開しない —— 後に走る `UsageAccessInstrumentedTest` が
        // 「利用状況へのアクセスの画面ではない」「収集が始まっていない」で落ちた（CI run 39720789999・手元で再現）。
        // task ごと閉じて、次のテストを何も無い状態から始めさせる
        context.getSystemService(ActivityManager::class.java).appTasks.forEach { it.finishAndRemoveTask() }
        device.pressHome()
    }

    // Scenario: 対象から外れていない端末で初めて開くと設定の画面へ案内される
    @Test
    fun theLineAndTheButtonAreVisibleAndTheButtonOpensSettings() {
        shell("appops set ${context.packageName} AUTO_REVOKE_PERMISSIONS_IF_UNUSED allow")
        // 自動の案内（ダイアログ）は済んだことにして、画面の 1 行とボタンだけを見る
        context.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE).edit()
            .putBoolean(MainActivity.SENT_TO_USAGE_ACCESS, true)
            .putBoolean(UnusedAppRestrictionsAsked.KEY, true)
            .commit()
        skipHealthPermissionStep(context) // 健康データの権限の段はこの試験の関心の外（ST11）
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

        val line = device.wait(Until.findObject(By.text(MainActivity.NOT_EXEMPT_LINE)), TIMEOUT_MS)
        assertNotNull("1 行が出ない", line)
        val button = device.findObject(By.text(MainActivity.OPEN_SETTINGS_LABEL))
        assertNotNull("「設定を開く」が無い", button)
        button.click()
        assertTrue(
            "「設定を開く」を押しても設定の画面が開かない（帯の裏に隠れている）",
            waitUntil { device.currentPackageName == SETTINGS },
        )
    }

    // PFD を読み終えるまで持つ（AutoCloseInputStream）。fd だけを FileInputStream に渡すと、読む前に PFD が回収され
    // finalizer が別のスレッドで fd を閉じる（FeelingReminderInstrumentedTest の CI の赤と同じ穴）
    private fun shell(cmd: String): String =
        ParcelFileDescriptor.AutoCloseInputStream(instrumentation.uiAutomation.executeShellCommand(cmd))
            .use { it.readBytes().toString(Charsets.UTF_8) }

    private fun waitUntil(timeoutMs: Long = TIMEOUT_MS, check: () -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (check()) return true
            Thread.sleep(250)
        }
        return false
    }

    private companion object {
        const val TIMEOUT_MS = 10_000L
        const val SETTINGS = "com.android.settings"
    }
}
