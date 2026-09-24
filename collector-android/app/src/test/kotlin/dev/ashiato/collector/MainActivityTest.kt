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
/** 権限を求めた回数を数えるだけの入口（本番の経路は 1 つも変えない）。 */
class CountingMainActivity : MainActivity() {
    val requested = mutableListOf<String>()

    override fun requestPermission(permission: String) {
        requested += permission
        super.requestPermission(permission)
    }
}

@RunWith(RobolectricTestRunner::class)
class MainActivityTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun grant(vararg permissions: String) = shadowOf(app).grantPermissions(*permissions)
    private fun deny(vararg permissions: String) = shadowOf(app).denyPermissions(*permissions)

    private fun launch(): ActivityController<MainActivity> =
        Robolectric.buildActivity(MainActivity::class.java).create().resume()

    /** OS が結果を返す（**中身は問わない** —— 実装は実状態しか見ない）。 */
    private fun systemAnswers(controller: ActivityController<out MainActivity>, permission: String) {
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

    /**
     * **ST06 で期待が反転した**（tasks 5.1 / 本人の決定 Q7 / design D5 / spec
     * 「収集の開始を、ソースの取得条件が満たされているかに依らず行う」）。
     *
     * ST06 より前は「取るものが無いので始めない」で `finish()` していた。
     * そのときは**アプリ利用も止まり、生存信号も出なかった**ので、
     * 受け手の画面には③「動いていたが取れない状態」ではなく⑥「途絶」が出た。
     * `docs/handoff/ST11.md` の点 2 はこの反転を指している（点 4「権限は拒否のまま」は変わらない）。
     */
    @Test
    fun `前景の位置を断られても、落とさずに収集を始める`() {
        deny(Manifest.permission.ACCESS_FINE_LOCATION)
        val controller = launch()

        systemAnswers(controller, Manifest.permission.ACCESS_FINE_LOCATION)

        val started = startedService()
        assertNotNull("前景を断られただけで収集が始まっていない", started)
        assertEquals(LocationService::class.java.name, started.component?.className)
        assertTrue("画面が閉じていない", controller.get().isFinishing)
    }

    @Test
    fun `同じ権限を無限に求め直さない`() {
        // 実状態を見る作りなので、記録が無いと「まだ許可されていない」を理由に永久に求め続ける
        deny(Manifest.permission.ACCESS_FINE_LOCATION)
        val controller = Robolectric.buildActivity(CountingMainActivity::class.java).create().resume()

        repeat(3) { systemAnswers(controller, Manifest.permission.ACCESS_FINE_LOCATION) }

        // **求めたのは 1 回だけ**（ST06 より前は `assertNull(startedService())` で代用していたが、
        // 収集を始める側が反転したので、求めた回数そのものを数える）
        assertEquals(
            "同じ権限を求め直している: ${controller.get().requested}",
            1,
            controller.get().requested.count { it == Manifest.permission.ACCESS_FINE_LOCATION },
        )
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

    // ------------------------------------------------------------------ 利用状況へのアクセス（tasks 5.3）

    /** その端末で「利用状況へのアクセス」が許されている状態にする。 */
    private fun allowUsageAccess(allowed: Boolean) =
        shadowOf(app.getSystemService(android.app.AppOpsManager::class.java)).setMode(
            android.app.AppOpsManager.OPSTR_GET_USAGE_STATS,
            android.os.Process.myUid(),
            app.packageName,
            if (allowed) android.app.AppOpsManager.MODE_ALLOWED else android.app.AppOpsManager.MODE_IGNORED,
        )

    private fun startedSettings(): android.content.Intent? =
        shadowOf(app).getNextStartedActivity()?.takeIf { it.action == android.provider.Settings.ACTION_USAGE_ACCESS_SETTINGS }

    /** 「1 度だけ」の印を端末から消す（初めての起動にする）。 */
    private fun forgetSent() =
        app.getSharedPreferences(MainActivity.PREFS, android.content.Context.MODE_PRIVATE).edit().clear().commit()

    // Scenario: 特別なアクセスが無ければ初回起動で設定画面へ送られる
    @Test
    fun `特別なアクセスが無ければ初回起動で設定画面へ送られる`() {
        forgetSent()
        allowUsageAccess(false)
        grant(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_BACKGROUND_LOCATION, Manifest.permission.POST_NOTIFICATIONS)

        launch()

        assertNotNull("利用状況へのアクセスの設定画面が開いていない", startedSettings())
    }

    // Scenario: 許可しなくても収集は始まる
    @Test
    fun `設定画面で許可せずに戻っても収集は始まる`() {
        forgetSent()
        allowUsageAccess(false)
        grant(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_BACKGROUND_LOCATION, Manifest.permission.POST_NOTIFICATIONS)
        val controller = launch()

        // 設定画面で何もせずに戻ってきた（許可されていないまま）
        controller.pause().resume()

        val started = startedService()
        assertNotNull("許可しなかっただけで収集が始まっていない", started)
        assertEquals(LocationService::class.java.name, started.component?.className)
    }

    // Scenario: 2 度目の起動では自動で送られない
    @Test
    fun `2 度目の起動では自動で送られない`() {
        forgetSent()
        allowUsageAccess(false)
        grant(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_BACKGROUND_LOCATION, Manifest.permission.POST_NOTIFICATIONS)
        launch()
        assertNotNull("1 度目で送られていない", startedSettings())
        shadowOf(app).clearNextStartedActivities()

        // **プロセスごと作り直しても印は残る**（端末の保存領域に置いてある）
        Robolectric.buildActivity(MainActivity::class.java).create().resume()

        assertNull("2 度目の起動で自動で送られている", startedSettings())
    }

    /** 既に許している本人を設定画面へ連れていかない。 */
    @Test
    fun `既に許されていれば設定画面へ送らない`() {
        forgetSent()
        allowUsageAccess(true)
        grant(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_BACKGROUND_LOCATION, Manifest.permission.POST_NOTIFICATIONS)

        launch()

        assertNull("許されているのに設定画面へ送っている", startedSettings())
    }
}
