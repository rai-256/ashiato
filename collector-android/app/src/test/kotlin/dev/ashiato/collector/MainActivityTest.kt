// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Application
import android.content.pm.PackageManager
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.android.controller.ActivityController

/**
 * 権限のフロー（tasks 6.2 / design D27）。
 *
 * この項目は一度「実装した」として `[x]` が入ったが**検証が 1 本も無く**、
 * 独立検証で戻された（2026-09-08）。テストを入れた後も、**実機で初回起動が
 * まったく通らない**ことが分かった（2026-09-10）——
 * 背景の位置は Android 11 以降ダイアログで取れず、結果が必ず「拒否」で返るのに、
 * それを信じて `finish()` していた。テストが**結果コードを直接渡していた**ので、
 * 実機で何が返るかを写していなかった。
 *
 * いまは**結果コードを見ない**。テストも権限の実状態だけを動かす。
 */
@RunWith(RobolectricTestRunner::class)
class MainActivityTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun grant(vararg permissions: String) = shadowOf(app).grantPermissions(*permissions)
    private fun deny(vararg permissions: String) = shadowOf(app).denyPermissions(*permissions)

    private fun launch(): ActivityController<MainActivity> =
        Robolectric.buildActivity(MainActivity::class.java).create().resume()

    /** OS が結果を返す（**中身は問わない** —— 実装は実状態しか見ない）。 */
    private fun systemAnswers(controller: ActivityController<MainActivity>, permission: String) {
        controller.get().onRequestPermissionsResult(
            1,
            arrayOf(permission),
            intArrayOf(PackageManager.PERMISSION_DENIED),
        )
    }

    private fun startedService() = shadowOf(app).peekNextStartedService()

    @Test
    fun `背景の位置が設定画面送りで拒否として返っても、収集は始まる`() {
        // **実機で起きた不具合そのもの**（design D27）。Android 11 以降、
        // ACCESS_BACKGROUND_LOCATION は許可ダイアログを出せず、結果は必ず拒否で返る。
        // それを信じて終わると、前景を許可した直後に必ず終了してサービスが一度も起動しない
        grant(Manifest.permission.ACCESS_FINE_LOCATION)
        deny(Manifest.permission.ACCESS_BACKGROUND_LOCATION, Manifest.permission.POST_NOTIFICATIONS)
        val controller = launch()

        // 背景 → 通知 の順に求められ、どちらも「拒否」で返る
        systemAnswers(controller, Manifest.permission.ACCESS_BACKGROUND_LOCATION)
        systemAnswers(controller, Manifest.permission.POST_NOTIFICATIONS)

        val started = startedService()
        assertNotNull("背景を断られただけで収集が始まっていない", started)
        assertEquals(LocationService::class.java.name, started.component?.className)
    }

    @Test
    fun `全部許可されたら収集を始める`() {
        grant(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
        )

        val controller = launch()

        assertNotNull("全部許可されたのに収集が始まらない", startedService())
        assertTrue(controller.get().isFinishing)
    }

    @Test
    fun `前景の位置を断られたら、落とさずに終わり何も送らない`() {
        // 取るものが無いので始めない。**落ちない**（tasks 6.2）
        deny(Manifest.permission.ACCESS_FINE_LOCATION)
        val controller = launch()

        systemAnswers(controller, Manifest.permission.ACCESS_FINE_LOCATION)

        assertNull("前景を断られたのに収集を始めている", startedService())
        assertTrue("画面が閉じていない", controller.get().isFinishing)
    }

    @Test
    fun `同じ権限を無限に求め直さない`() {
        // 実状態を見る作りなので、記録が無いと「まだ許可されていない」を理由に永久に求め続ける
        deny(Manifest.permission.ACCESS_FINE_LOCATION)
        val controller = launch()

        repeat(3) { systemAnswers(controller, Manifest.permission.ACCESS_FINE_LOCATION) }

        assertNull(startedService())
        assertTrue(controller.get().isFinishing)
    }

    @Test
    fun `設定画面から戻って許可されていれば、そのまま収集を始める`() {
        // 背景の位置は設定画面でしか許可できない。**戻りを拾えないと先へ進めない**
        grant(Manifest.permission.ACCESS_FINE_LOCATION)
        deny(Manifest.permission.ACCESS_BACKGROUND_LOCATION)
        val controller = launch()
        systemAnswers(controller, Manifest.permission.ACCESS_BACKGROUND_LOCATION)
        shadowOf(app).clearStartedServices()

        // 設定画面で「常に許可」にして戻ってきた
        grant(
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
        )
        controller.pause().resume()

        assertNotNull("設定画面から戻った許可を拾えていない", startedService())
    }
}
