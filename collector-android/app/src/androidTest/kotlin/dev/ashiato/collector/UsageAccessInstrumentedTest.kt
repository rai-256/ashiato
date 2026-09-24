// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.UiDevice
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * **「利用状況へのアクセス」は初回起動で設定画面へ送り、許可されなくても収集を始める**
 * （tasks 5.3 / 本人の決定 Q3 / design D6（仮））。
 *
 * この特別なアクセスは**その場のダイアログでは取れない** —— アプリから開けるのは設定画面までで、
 * 開いたかどうかは**自分のプロセスの外**（`com.android.settings`）にあるので Espresso では触れない。
 * `PermissionDeniedInstrumentedTest` と同じく UI Automator で前面を見る。
 *
 * **前提は「まだ許されていない」**。この状態は端末の既定で、テストの中では作れない
 * （`appops set` は `uiAutomation` から打てるが、自分の op を落とすと
 * 計測テストのプロセスが道連れになりうる。実測 2026-09-18 の `pm revoke` と同じ型）。
 * ここでは**確かめるだけ**にして、許されていたら前提不備として落とす。
 */
@RunWith(AndroidJUnit4::class)
class UsageAccessInstrumentedTest {
    /** 位置のダイアログを出さないために先に揃える（見たいのはアプリ利用の側だけ）。 */
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

    @Before
    fun requireNoUsageAccessAndFirstLaunch() {
        assertTrue(
            "前提が作れていない。「利用状況へのアクセス」が既に許されている " +
                "（`adb shell appops set dev.ashiato.collector GET_USAGE_STATS default` で戻す）",
            usageAccessCapability(context).blockers.contains(Capability.PERMISSION),
        )
        // 「1 度だけ」の印は**自分の置き場**なので、プロセスを殺さずに消せる（初めての起動に戻す）
        context.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE).edit().clear().commit()
        device.pressHome()
    }

    @After
    fun stopService() {
        device.pressHome()
        context.stopService(Intent(context, LocationService::class.java))
    }

    /**
     * 1 度目は送られ、**許可せずに戻っても収集は始まり**、2 度目は自動で送られない。
     *
     * **1 本にしてあるのは順番が要るから** —— 「2 度目」は「1 度目」の後でしか作れず、
     * JUnit は実行順を約束しない。
     */
    @Test
    fun firstLaunchOpensUsageAccessSettingsThenSecondLaunchDoesNot() {
        // ---- Scenario: 特別なアクセスが無ければ初回起動で設定画面へ送られる
        launchEntry()
        assertTrue(
            "設定アプリが前面に来ない（前面: ${device.currentPackageName}）",
            waitForForeground { it.contains("settings") },
        )
        // **「その」設定画面であること**（独立レビュー M4）。設定アプリの適当な画面では THEN を満たさない。
        // 前面の Activity の名前（`…Settings$UsageAccessSettingsActivity`）と画面の中身の両方を材料にする ——
        // 名前は端末の言語に依らず、中身は別名で開かれた場合にも効く
        val evidence = resumedActivity() + " | " + screenText()
        assertTrue(
            "開いたのが利用状況へのアクセスの画面ではない: $evidence",
            Regex("(?i)usage|使用状況|利用状況").containsMatchIn(evidence),
        )

        // ---- 許可せずに戻っても収集は始まる（`Scenario: 許可しなくても収集は始まる` の印は置かない ——
        //      その THEN は「位置の記録が生成される」で、ここは通知までしか見ていない。独立レビュー I3）
        device.pressBack()
        assertTrue(
            "許可せずに戻っただけで収集が始まっていない（前景サービスの通知が出ない）",
            waitForNotification(),
        )
        assertTrue(
            "許可していないのに取れていることになっている",
            usageAccessCapability(context).blockers.contains(Capability.PERMISSION),
        )

        // ---- Scenario: 2 度目の起動では自動で送られない
        device.pressHome()
        launchEntry()
        assertFalse(
            "2 度目の起動で設定画面へ自動で送られている",
            waitForForeground(SETTLE_MS) { it.contains("settings") },
        )
    }

    /** いま前面にある Activity の名前（端末の言語に依らない材料）。 */
    private fun resumedActivity(): String =
        java.io.FileInputStream(
            instrumentation.uiAutomation.executeShellCommand("dumpsys activity activities").fileDescriptor,
        ).use { it.readBytes().toString(Charsets.UTF_8) }
            .lineSequence().firstOrNull { it.contains("mResumedActivity") }.orEmpty()

    /** いま出ている画面の中身（表示名で確かめる側の材料）。 */
    private fun screenText(): String = java.io.ByteArrayOutputStream()
        .also { device.dumpWindowHierarchy(it) }
        .toString(Charsets.UTF_8.name())

    private fun launchEntry() {
        context.startActivity(
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        )
    }

    private fun waitForForeground(timeoutMs: Long = TIMEOUT_MS, match: (String) -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (match(device.currentPackageName.orEmpty().lowercase())) return true
            Thread.sleep(250)
        }
        return false
    }

    private fun waitForNotification(): Boolean {
        val manager = context.getSystemService(NotificationManager::class.java)
        val deadline = System.currentTimeMillis() + TIMEOUT_MS
        while (System.currentTimeMillis() < deadline) {
            if (manager.activeNotifications.any { it.id == LocationService.NOTIFICATION_ID }) return true
            Thread.sleep(250)
        }
        return false
    }

    private companion object {
        const val TIMEOUT_MS = 15_000L

        /** 「開かない」ことの確かめは待ち切る（短すぎると、遅れて開いても緑になる）。 */
        const val SETTLE_MS = 5_000L
    }
}
