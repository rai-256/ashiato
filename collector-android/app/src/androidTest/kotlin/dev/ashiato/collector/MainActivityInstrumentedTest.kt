// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.rule.GrantPermissionRule
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 権限が揃っているときの入口: `MainActivity` を開くと**前景サービスが本当に立つ**（通知が出る）。
 *
 * ST01 の実機初日に見つかった欠陥（design D27: 権限のフローは結果コードではなく実際の権限状態を見る）は
 * Robolectric では再現できなかった。ここでは本物の framework が権限を持った状態で Activity を通す。
 * 権限の**ダイアログ**そのもの（拒否したときの振る舞い）はエミュレータでも UI 操作が要るので持ち越し。
 */
@RunWith(AndroidJUnit4::class)
class MainActivityInstrumentedTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
    )

    private val context: Context get() = ApplicationProvider.getApplicationContext()

    @After
    fun stopService() {
        context.stopService(Intent(context, LocationService::class.java))
    }

    @Test
    fun openingTheActivityWithPermissionsStartsTheForegroundService() {
        ActivityScenario.launch(MainActivity::class.java).use {
            // Activity は権限が揃っていればサービスを起動して自分を閉じる（`finish()`）
            val nm = context.getSystemService(NotificationManager::class.java)
            val deadline = System.currentTimeMillis() + 15_000
            var shown = false
            while (System.currentTimeMillis() < deadline) {
                shown = nm.activeNotifications.any { it.id == LocationService.NOTIFICATION_ID }
                if (shown) break
                Thread.sleep(250)
            }
            assertTrue("前景サービスの通知（id=${LocationService.NOTIFICATION_ID}）が出る", shown)
        }
    }
}
