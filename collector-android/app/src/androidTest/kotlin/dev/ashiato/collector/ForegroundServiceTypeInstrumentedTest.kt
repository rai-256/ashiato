// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.FileInputStream
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * **位置の権限が無くても前景サービスが立つ**（tasks 1.4 / design D5 のリスク）。
 *
 * targetSdk 34 以降、`foregroundServiceType="location"` の前景サービスは
 * **位置の実行時権限を持たないまま `startForeground` を呼ぶと `SecurityException`** になる。
 * 立てられないと 2 本目のソース（アプリ利用）まで道連れに止まり、
 * 本人の決定 Q7「欠けたソースだけを止め、他は取り続ける」が壊れる。
 *
 * **これは実機（エミュレータ）でしか分からない。** Robolectric は `startForeground` の
 * 種別ごとの権限検査を持たないので、単体では常に通る。
 *
 * **何を見て「立った」と判定するか**: `LocationService.onCreate` は
 * `startForeground` の**直後**に種別の名前を 1 行残す。`startForeground` が投げれば
 * `onCreate` ごと落ちてその行は出ない —— **行が出たこと自体が「前景に上がれた」の証拠**。
 * 前景サービスの通知そのものを数えないのは、位置の権限が無いと `onStartCommand` が
 * `stopSelf()` するので（ST06 の tasks 5.1 まではその振る舞い）、
 * 通知を覗きに行くころには畳まれているため。
 *
 * `PermissionDeniedInstrumentedTest` と同じく「未許可・未要求」から始める必要があるので
 * [NeedsPristinePermissions] を付ける（前提はテストの外が作る）。
 */
@RunWith(AndroidJUnit4::class)
@NeedsPristinePermissions
class ForegroundServiceTypeInstrumentedTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val pkg: String get() = context.packageName

    private fun shell(cmd: String): String =
        FileInputStream(instrumentation.uiAutomation.executeShellCommand(cmd).fileDescriptor)
            .use { it.readBytes().toString(Charsets.UTF_8) }

    /** 前提は**テストの外**が作る（`@NeedsPristinePermissions`）。ここは確かめるだけ。 */
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

    /** **次のテストへ持ち越さない。** 前景サービスが残っていると他のテストの前提が崩れる。 */
    @After
    fun stopService() {
        context.stopService(Intent(context, LocationService::class.java))
    }

    @Test
    fun foregroundServiceStartsWithoutLocationPermission() {
        context.startForegroundService(Intent(context, LocationService::class.java))

        assertTrue(
            "位置の権限が無い状態で前景サービスが立たなかった" +
                "（`kind=${LocationService.FOREGROUND_DATA_SYNC}` が出ていない）",
            waitForLog("kind=${LocationService.FOREGROUND_DATA_SYNC}"),
        )
        // 位置が取れないので `location` の種別は要求していない（要求すれば立てられない）
        assertFalse(
            "位置の権限が無いのに location の種別で立てようとしている",
            logs().contains("kind=${LocationService.FOREGROUND_LOCATION}"),
        )
        // 落ちていない（crash バッファに自分の名前が無い）
        val crash = shell("logcat -d -b crash -t 400")
        assertFalse("crash ログにアプリが出ている:\n$crash", crash.contains(pkg))
    }

    private fun logs(): String = shell("logcat -d -s ashiato:I -t 400")

    private fun waitForLog(needle: String): Boolean {
        val deadline = System.currentTimeMillis() + LOG_TIMEOUT_MS
        while (System.currentTimeMillis() < deadline) {
            if (logs().contains(needle)) return true
            Thread.sleep(250)
        }
        return false
    }

    private companion object {
        const val LOG_TIMEOUT_MS = 20_000L
    }
}
