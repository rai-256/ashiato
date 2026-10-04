// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.content.pm.ApplicationInfo
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

@RunWith(RobolectricTestRunner::class)
class AppLabelsTest {
    @Test
    fun `起動入口のない任意の第三者アプリを可視にして表示名を引く`() {
        val app: Application = ApplicationProvider.getApplicationContext()
        val pm = app.packageManager
        val permissions = pm.getPackageInfo(app.packageName, PackageManager.GET_PERMISSIONS).requestedPermissions
        // 起動 intent の queries だけでは、入口のない任意のアプリを覆えない。
        assertTrue("任意のパッケージの可視性が宣言されていない",
            permissions.orEmpty().contains("android.permission.QUERY_ALL_PACKAGES"))
        shadowOf(pm).installPackage(PackageInfo().apply {
            packageName = "unrelated.third.party"
            applicationInfo = ApplicationInfo().apply {
                packageName = "unrelated.third.party"
                nonLocalizedLabel = "第三者のアプリ"
            }
        })
        assertEquals("第三者のアプリ", PackageManagerAppLabels(app).label("unrelated.third.party"))
    }
}
