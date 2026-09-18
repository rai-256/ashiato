// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.io.FileInputStream
import java.util.regex.Pattern
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * **位置の権限を拒否したとき**: アプリは落ちず、収集を始めず、`permission_denied` を残して終わる。
 *
 * ここだけが UI Automator を使う。権限ダイアログは `permissioncontroller` のもので、
 * **自分のプロセスの外**にあるため Espresso では触れない。
 * `MainActivityInstrumentedTest` は `GrantPermissionRule` で「揃っている側」を見ており、
 * **拒否した側は 2026-09-18 まで機械も人間も見ていなかった**
 * （`MainActivityInstrumentedTest` の doc が「持ち越し」と書いていたもの）。
 *
 * `MainActivity` の約束（design D27）:
 *   - 結果コードではなく**実際の権限状態**を見る
 *   - 前景の位置が無ければ、一度求めて、それでも無いなら `permission_denied` を残して `finish()`
 *   - **落とさない**（tasks 6.2）。収集は始めない
 */
@RunWith(AndroidJUnit4::class)
@NeedsPristinePermissions
class PermissionDeniedInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val pkg: String get() = context.packageName

    private fun shell(cmd: String): String =
        FileInputStream(instrumentation.uiAutomation.executeShellCommand(cmd).fileDescriptor)
            .use { it.readBytes().toString(Charsets.UTF_8) }

    /**
     * 前提は**テストの外**が作る（`@NeedsPristinePermissions` の説明）。
     * ここで `pm revoke` すると自分のプロセスが死ぬので、**確かめるだけ**で作りに行かない。
     */
    @Before
    fun requirePristinePermissions() {
        assertEquals(
            "前提が作れていない。位置の権限が残っている。" +
                "`adb shell pm clear $pkg` の後に、" +
                "annotation=dev.ashiato.collector.NeedsPristinePermissions で単独に走らせる " +
                "（tools/android-emulator.sh と CI がその順で走らせる）",
            PackageManager.PERMISSION_DENIED,
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION),
        )
        shell("logcat -c")
    }

    // **spec の Scenario には対応しない。** 正典に「拒否したときの振る舞い」の Scenario が無く、
    // 足すなら change → archive の工程が要る。ここは design D27 / tasks 6.2 の回帰テストとして置く
    // （印を付けると `check_scenarios.py` が「spec に無い Scenario を指す印」として warn を出す）
    @Test
    fun denyingLocationLeavesTheAppAliveAndCollectionStopped() {
        context.startActivity(
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        )

        // ---- システムの権限ダイアログで「拒否」を押す
        val deny = device.wait(
            Until.findObject(By.res(Pattern.compile(".*:id/permission_deny_button"))),
            DIALOG_TIMEOUT_MS,
        ) ?: device.wait(
            Until.findObject(By.text(Pattern.compile("(?i)(許可しない|拒否|don't allow|deny)"))),
            DIALOG_TIMEOUT_MS,
        )
        // **見つからなければ落とす。** 飛ばすと「拒否していないのに緑」になる
        assertTrue(
            "権限ダイアログの拒否ボタンが見つからない（前面: ${device.currentPackageName}）",
            deny != null,
        )
        deny!!.click()

        // ---- 1. 約束どおりの経路で終わったか（結果コードではなく権限状態を見た証拠）
        assertTrue(
            "logcat に kind=permission_denied が出ない（別の経路で終わっている）",
            waitForLog("kind=permission_denied"),
        )

        // ---- 2. 収集は始まっていない（前景サービスの通知が出ない）
        val nm = context.getSystemService(NotificationManager::class.java)
        val deadline = System.currentTimeMillis() + SETTLE_MS
        while (System.currentTimeMillis() < deadline) {
            assertFalse(
                "位置の権限が無いのに前景サービスが立った",
                nm.activeNotifications.any { it.id == LocationService.NOTIFICATION_ID },
            )
            Thread.sleep(250)
        }

        // ---- 3. 落ちていない（crash バッファに自分の名前が無い）
        val crash = shell("logcat -d -b crash -t 400")
        assertFalse("crash ログにアプリが出ている:\n$crash", crash.contains(pkg))

        // ---- 4. 権限は拒否のまま（押した先が「許可」ではなかった）
        assertEquals(
            PackageManager.PERMISSION_DENIED,
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION),
        )
    }

    private fun waitForLog(needle: String): Boolean {
        val deadline = System.currentTimeMillis() + LOG_TIMEOUT_MS
        while (System.currentTimeMillis() < deadline) {
            if (shell("logcat -d -s ashiato:W -t 400").contains(needle)) return true
            Thread.sleep(250)
        }
        return false
    }

    private companion object {
        const val DIALOG_TIMEOUT_MS = 15_000L
        const val LOG_TIMEOUT_MS = 15_000L
        const val SETTLE_MS = 3_000L
    }
}
