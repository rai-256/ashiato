// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
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
 * **位置の権限を拒否したとき**: アプリは落ちず、`permission_denied` を残して終わり、**収集は始まる**。
 *
 * ここだけが UI Automator を使う。権限ダイアログは `permissioncontroller` のもので、
 * **自分のプロセスの外**にあるため Espresso では触れない。
 * `MainActivityInstrumentedTest` は `GrantPermissionRule` で「揃っている側」を見ており、
 * **拒否した側は 2026-09-18 まで機械も人間も見ていなかった**
 * （`MainActivityInstrumentedTest` の doc が「持ち越し」と書いていたもの）。
 *
 * `MainActivity` の約束（design D27 / ST06 の tasks 5.1）:
 *   - 結果コードではなく**実際の権限状態**を見る
 *   - 前景の位置が無ければ、一度求めて、それでも無いなら `permission_denied` を残して `finish()`
 *   - **落とさない**（tasks 6.2）
 *   - **収集は始める**（ST06 / 本人の決定 Q7 / design D5）—— ここが 2026-09-18 に反転した。
 *     止めていた間は**生存信号も出なかった**ので、受け手の画面には③「動いていたが取れない状態」ではなく
 *     ⑥「途絶」が出て、アプリ利用はその間 1 件も取れなかった（`docs/handoff/ST11.md` の点 2）
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
                "`adb shell am force-stop $pkg && adb shell pm reset-permissions` の後に、" +
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
    fun denyingLocationLeavesTheAppAliveAndCollectionRunning() {
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

        // ---- 2. 収集は始まっている（前景サービスが立ち、生存信号が出る）。**欠けたソースだけが止まる**
        //
        // **通知そのものは数えない**（実測 2026-09-24）。この前提は「未許可・未要求」なので
        // `POST_NOTIFICATIONS` も無く、Android 13 以降は**前景サービスが立っても通知は表示されない**
        // （`activeNotifications` に出ない）。立ったことの証拠は `startForeground` の直後の 1 行 ——
        // `startForeground` が投げれば `onCreate` ごと落ちてこの行は出ない
        // （`ForegroundServiceTypeInstrumentedTest` と同じ見方）
        assertTrue(
            "位置を拒否しただけで前景サービスが立たない（kind=${LocationService.FOREGROUND_SPECIAL_USE} が出ない）",
            waitForLog("kind=${LocationService.FOREGROUND_SPECIAL_USE}", "ashiato:I"),
        )
        // 位置は③「取得できない状態」として生存信号に載る（止まったのは位置だけ）
        assertTrue(
            "位置の生存信号が理由つきで出ていない",
            waitForLog("kind=heartbeat source=$LOGICAL_SOURCE count=0 error=permission", "ashiato:I"),
        )
        // アプリ利用の側も同じ区間の信号を出している（**道連れに止まっていない**）
        assertTrue(
            "アプリ利用の生存信号が出ていない（欠けたソースが他を道連れにしている）",
            waitForLog("kind=heartbeat source=$APP_USAGE_LOGICAL_SOURCE", "ashiato:I"),
        )

        // ---- 3. 落ちていない（crash バッファに自分の名前が無い）
        val crash = shell("logcat -d -b crash -t 400")
        assertFalse("crash ログにアプリが出ている:\n$crash", crash.contains(pkg))

        // ---- 4. 権限は拒否のまま（押した先が「許可」ではなかった）
        assertEquals(
            PackageManager.PERMISSION_DENIED,
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION),
        )
    }

    private fun waitForLog(needle: String, filter: String = "ashiato:W"): Boolean {
        val deadline = System.currentTimeMillis() + LOG_TIMEOUT_MS
        while (System.currentTimeMillis() < deadline) {
            if (shell("logcat -d -s $filter -t 400").contains(needle)) return true
            Thread.sleep(250)
        }
        return false
    }

    /** **次のテストへ持ち越さない。** 収集が始まるようになったので、前景サービスが残る。 */
    @org.junit.After
    fun stopService() {
        context.stopService(Intent(context, LocationService::class.java))
    }

    private companion object {
        const val DIALOG_TIMEOUT_MS = 15_000L
        const val LOG_TIMEOUT_MS = 15_000L
    }
}
