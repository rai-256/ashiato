// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Activity
import android.app.Application
import android.content.Intent
import android.content.pm.PackageManager
import android.widget.TextView
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import java.io.File

@RunWith(RobolectricTestRunner::class)
class HealthManifestTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun resolvedTarget(intent: Intent): String {
        val info = app.packageManager.resolveActivity(intent.setPackage(app.packageName), 0)
        assertNotNull("開く画面に解決されない: $intent", info)
        val activity = info!!.activityInfo
        return activity.targetActivity ?: activity.name
    }

    @Test
    fun manifestDeclaresExactlyTheSixHealthPermissions() {
        val requested = app.packageManager
            .getPackageInfo(app.packageName, PackageManager.GET_PERMISSIONS)
            .requestedPermissions.orEmpty()
            .filter { it.startsWith("android.permission.health.") }
        assertEquals(HEALTH_PERMISSIONS.values.sorted(), requested.sorted())
        assertEquals(6, HEALTH_PERMISSIONS.size)
    }

    // Scenario: 説明の画面は 2 つの開き方のどちらでも開く
    @Test
    fun rationaleOpensFromBothEntryPoints() {
        val android13 = Intent("androidx.health.ACTION_SHOW_PERMISSIONS_RATIONALE")
        val android14 = Intent(Intent.ACTION_VIEW_PERMISSION_USAGE)
            .addCategory("android.intent.category.HEALTH_PERMISSIONS")
        val expected = HealthRationaleActivity::class.java.name
        assertEquals(expected, resolvedTarget(android13))
        assertEquals(expected, resolvedTarget(android14))
    }

    // Scenario: 権限の画面からプライバシーポリシーを開くと説明の画面が出る
    @Test
    fun rationaleScreenShowsKindsDestinationApprovalAndNotReadingYet() {
        val activity = Robolectric.buildActivity(HealthRationaleActivity::class.java).create().resume().get()
        val text = textsOf(activity).joinToString("\n")
        assertEquals(HEALTH_RATIONALE, text)
        // 定数との一致だけでは文を変えても通る（spec-review R19）。中身を 1 つずつ見る
        for (kind in listOf("睡眠", "心拍", "歩数", "運動")) {
            assertEquals("$kind が 1 つ", 1, count(text, kind))
        }
        assertEquals("自宅の PC", 1, count(text, "自宅の PC"))
        assertEquals("承認の設定", 1, count(text, "承認の設定"))
        assertEquals("取り込みを入れてから", 1, count(text, "取り込みを入れてから"))
    }

    @Test
    fun backupRulesExcludeHealthPrefsAndKeepCollector() {
        val dir = File("src/main/res/xml")
        val rules = listOf("health_data_extraction_rules.xml", "health_full_backup_content.xml")
            .map { File(dir, it).readText() }
        for (xml in rules) {
            assertTrue(xml, xml.contains("""domain="sharedpref" path="$HEALTH_PREFS.xml""""))
            assertFalse(xml, xml.contains("collector.xml"))
        }
        // 2 つの規則は、クラウドと端末の移行の両方で外す
        assertEquals(2, count(rules[0], "<exclude"))
        val manifest = File("src/main/AndroidManifest.xml").readText()
        assertTrue(manifest.contains("""android:dataExtractionRules="@xml/health_data_extraction_rules""""))
        assertTrue(manifest.contains("""android:fullBackupContent="@xml/health_full_backup_content""""))
    }

    private fun count(text: String, needle: String): Int = text.windowed(needle.length).count { it == needle }

    private fun textsOf(activity: Activity): List<String> {
        val out = mutableListOf<String>()
        fun walk(v: android.view.View) {
            if (v is TextView) out += v.text.toString()
            if (v is android.view.ViewGroup) (0 until v.childCount).forEach { walk(v.getChildAt(it)) }
        }
        walk(activity.findViewById(android.R.id.content))
        return out
    }
}
