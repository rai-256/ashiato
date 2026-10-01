// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.AppOpsManager
import android.app.Application
import android.os.Process
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

/**
 * 取得可否を端末から読む口（tasks 5.2 / 本人の決定 C9 / 深掘り Q5）。
 *
 * アプリ利用の「利用状況へのアクセス」は**実行時権限ではない** ——
 * `checkSelfPermission` は宣言しただけの `PACKAGE_USAGE_STATS` に対して
 * 端末によって拒否を返し続けるので、本人が設定画面で許した事実は
 * `AppOpsManager` の `GET_USAGE_STATS` でしか読めない。
 * ここを取り違えると、**許可済みの端末が「取れない」と生存信号を送り続ける**
 * （画面には③が出たまま、記録だけが届く）。
 */
@RunWith(RobolectricTestRunner::class)
class AndroidCapabilityTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun setUsageAccess(mode: Int) = shadowOf(app.getSystemService(AppOpsManager::class.java))
        .setMode(AppOpsManager.OPSTR_GET_USAGE_STATS, Process.myUid(), app.packageName, mode)

    @Test
    fun `利用状況へのアクセスが許されていれば取れている`() {
        setUsageAccess(AppOpsManager.MODE_ALLOWED)

        assertFalse(
            "許されているのに権限が満たされていないと報告している",
            usageAccessCapability(app).blockers.contains(Capability.PERMISSION),
        )
    }

    @Test
    fun `利用状況へのアクセスが無ければ権限が満たされていない`() {
        setUsageAccess(AppOpsManager.MODE_IGNORED)

        val capability = usageAccessCapability(app)

        assertFalse("アクセスが無いのに取れていることになっている", capability.capturable)
        assertTrue(
            "何が満たされていないか（権限）を示していない",
            capability.blockers.contains(Capability.PERMISSION),
        )
    }

    /**
     * **本人が 1 度も触っていない状態**（`MODE_DEFAULT`）では、宣言した権限の付与状態で決まる。
     * `MODE_DEFAULT` を「許された」と読むと、**初回起動の端末が全部「取れている」**になり、
     * 1 件も取れていない期間が③ではなく①として残る。
     */
    @Test
    fun `まだ触られていない状態は付与状態で決まる`() {
        setUsageAccess(AppOpsManager.MODE_DEFAULT)
        shadowOf(app).denyPermissions(Manifest.permission.PACKAGE_USAGE_STATS)

        assertFalse("触られていないのに取れていることになっている", usageAccessCapability(app).capturable)

        shadowOf(app).grantPermissions(Manifest.permission.PACKAGE_USAGE_STATS)

        assertFalse(
            "付与されているのに権限が満たされていないと報告している",
            usageAccessCapability(app).blockers.contains(Capability.PERMISSION),
        )
    }

    /**
     * **アプリ利用に `sensor` は無い**（本人の決定 C9）。
     * 位置の口（[androidCapability]）をそのまま使い回すと、端末の位置情報が切られているだけで
     * アプリ利用が「センサが無い」と報告し、受け手の③の理由が誤る。
     */
    @Test
    fun `アプリ利用の blockers は permission と network の 2 つだけ`() {
        setUsageAccess(AppOpsManager.MODE_IGNORED)
        shadowOf(app.getSystemService(android.location.LocationManager::class.java))
            .setLocationEnabled(false)

        val blockers = usageAccessCapability(app).blockers

        assertFalse("アプリ利用にセンサの理由が載っている: $blockers", blockers.contains(Capability.SENSOR))
        assertTrue(
            "permission と network 以外の理由が載っている: $blockers",
            blockers.all { it == Capability.PERMISSION || it == Capability.NETWORK },
        )
    }

    /**
     * **読めなかったら「取れない」に倒す**（tasks 5.2 / `androidCapability` の R31 と同じ規律）。
     * 端末の口はどれも投げうるので、貫通させると起動時の生存信号がプロセスごと落とし、
     * `START_STICKY` と合わさってクラッシュループになる。
     */
    @Test
    fun `取得可否を読めない端末でも落とさず取れないに倒す`() {
        shadowOf(app).removeSystemService(android.content.Context.APP_OPS_SERVICE)

        val capability = usageAccessCapability(app)

        assertFalse("読めないのに取れていることになっている", capability.capturable)
        assertEquals(
            "読めなかった理由が権限として残っていない",
            true,
            capability.blockers.contains(Capability.PERMISSION),
        )
    }
}
