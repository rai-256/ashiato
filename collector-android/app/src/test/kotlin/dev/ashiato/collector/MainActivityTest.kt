// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Application
import android.content.pm.PackageManager
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

/**
 * 権限を断られたときに**何も送らずに落ちない**（tasks 6.2）。
 *
 * この項目は一度「実装した」として `[x]` が入ったが、**検証が 1 本も無かった**ので
 * 独立検証で戻された（2026-09-08）。落ちると次の起動まで収集が止まり、
 * 成功条件 1（1 年間途切れない）に直接効く。
 *
 * 本番経路を通す —— OS が呼ぶのと同じ `onRequestPermissionsResult` を叩く。
 */
@RunWith(RobolectricTestRunner::class)
class MainActivityTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun grant(vararg permissions: String) {
        shadowOf(app).grantPermissions(*permissions)
    }

    @Test
    fun `位置の権限を断られても落ちず、何も送らない`() {
        val controller = Robolectric.buildActivity(MainActivity::class.java).create()
        val activity = controller.get()

        // OS が「拒否」を返してくる
        activity.onRequestPermissionsResult(
            1,
            arrayOf(Manifest.permission.ACCESS_FINE_LOCATION),
            intArrayOf(PackageManager.PERMISSION_DENIED),
        )

        assertNull("断られたのに収集を始めている", shadowOf(app).peekNextStartedService())
        // 「落ちない」は、例外が飛べばこの試験自体が落ちることで担保される。
        // `isDestroyed` は destroy() を呼んでいない以上つねに false なので見ない（review R8）
        assertTrue("画面が閉じていない", activity.isFinishing)
    }

    @Test
    fun `背景の権限だけ断られても落ちず、何も送らない`() {
        // 前景だけ許可された状態。**2 段目で断られる経路**が別にある
        grant(Manifest.permission.ACCESS_FINE_LOCATION)
        val controller = Robolectric.buildActivity(MainActivity::class.java).create()
        val activity = controller.get()

        activity.onRequestPermissionsResult(
            1,
            arrayOf(Manifest.permission.ACCESS_BACKGROUND_LOCATION),
            intArrayOf(PackageManager.PERMISSION_DENIED),
        )

        assertNull("断られたのに収集を始めている", shadowOf(app).peekNextStartedService())
        assertTrue(activity.isFinishing)
    }

    @Test
    fun `結果が空でも落ちず、何も送らない`() {
        // 要求が途中で取り消されると grantResults が空で返る（Android の仕様）
        val controller = Robolectric.buildActivity(MainActivity::class.java).create()
        val activity = controller.get()

        activity.onRequestPermissionsResult(1, arrayOf(Manifest.permission.ACCESS_FINE_LOCATION), IntArray(0))

        assertNull(shadowOf(app).peekNextStartedService())
        assertTrue(activity.isFinishing)
    }

    @Test
    fun `全部許可されたら収集を始める`() {
        // **この 1 本が無いと上の 3 本は空振りしうる** ——
        // どう転んでもサービスを起動しない実装でも緑になる
        grant(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
        )

        val activity = Robolectric.buildActivity(MainActivity::class.java).create().get()

        val started = shadowOf(app).peekNextStartedService()
        assertNotNull("全部許可されたのに収集が始まらない", started)
        assertTrue(
            "起動したのが LocationService でない: ${started.component}",
            started.component?.className == LocationService::class.java.name,
        )
        assertTrue(activity.isFinishing)
    }
}
