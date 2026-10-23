// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Application
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.io.File

/** 求める権限をまとめて記録する入口（本番の経路は変えない）。 */
class MediaCountingMainActivity : MainActivity() {
    val singles = mutableListOf<String>()
    val groups = mutableListOf<List<String>>()

    override fun requestPermission(permission: String) {
        singles += permission
        super.requestPermission(permission)
    }

    override fun requestPermissionGroup(permissions: List<String>) {
        groups += permissions
        super.requestPermissionGroup(permissions)
    }
}

/** 写真と動画の権限の宣言と求め方（tasks 4.1 / design D6 / Q15）。 */
@RunWith(RobolectricTestRunner::class)
class MediaPermissionTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun manifest(): String = listOf("src/main/AndroidManifest.xml", "app/src/main/AndroidManifest.xml")
        .map(::File).first { it.exists() }.readText()

    @Test
    fun the_manifest_declares_the_media_permissions() {
        val text = manifest()
        for (name in listOf(
            "READ_MEDIA_IMAGES",
            "READ_MEDIA_VIDEO",
            "READ_MEDIA_VISUAL_USER_SELECTED",
            "ACCESS_MEDIA_LOCATION",
        )) {
            assertTrue("$name が宣言されていない", text.contains("android.permission.$name\""))
        }
        // 32 以下だけの読み取り権限（minSdk 30 の端末用）
        assertTrue(
            "READ_EXTERNAL_STORAGE が maxSdkVersion=32 で宣言されていない",
            Regex("""READ_EXTERNAL_STORAGE"\s+android:maxSdkVersion="32"""").containsMatchIn(text),
        )
    }

    @Test
    fun the_permissions_asked_follow_the_android_version() {
        assertEquals(listOf(Manifest.permission.READ_EXTERNAL_STORAGE), mediaAccessPermissions(31))
        assertEquals(
            listOf(Manifest.permission.READ_MEDIA_IMAGES, Manifest.permission.READ_MEDIA_VIDEO),
            mediaAccessPermissions(Build.VERSION_CODES.TIRAMISU),
        )
        assertEquals(
            listOf(
                Manifest.permission.READ_MEDIA_IMAGES,
                Manifest.permission.READ_MEDIA_VIDEO,
                Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED,
            ),
            mediaAccessPermissions(Build.VERSION_CODES.UPSIDE_DOWN_CAKE),
        )
        // 写真の位置はどの版でも一緒に求める
        for (sdk in listOf(31, Build.VERSION_CODES.TIRAMISU, Build.VERSION_CODES.UPSIDE_DOWN_CAKE)) {
            assertTrue(Manifest.permission.ACCESS_MEDIA_LOCATION in mediaPermissionsToAsk(sdk))
        }
    }

    // Scenario: 初回の起動で写真と動画と写真の位置の権限を求める
    @Test
    fun the_first_launch_asks_for_media_access_and_photo_location_after_location() {
        shadowOf(app).denyPermissions(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
            *mediaPermissionsToAsk(Build.VERSION.SDK_INT).toTypedArray(),
        )
        val controller = Robolectric.buildActivity(MediaCountingMainActivity::class.java).create().resume()
        val activity = controller.get()

        fun answer(permission: String) =
            activity.onRequestPermissionsResult(1, arrayOf(permission), intArrayOf(PackageManager.PERMISSION_DENIED))
        answer(Manifest.permission.ACCESS_FINE_LOCATION)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) answer(Manifest.permission.POST_NOTIFICATIONS)

        assertEquals("位置を先に求める", Manifest.permission.ACCESS_FINE_LOCATION, activity.singles.first())
        assertEquals("写真と動画と写真の位置を 1 度にまとめて求める", 1, activity.groups.size)
        assertEquals(mediaPermissionsToAsk(Build.VERSION.SDK_INT), activity.groups.single())
        assertTrue(Manifest.permission.ACCESS_MEDIA_LOCATION in activity.groups.single())
    }

    @Test
    fun media_permissions_that_are_refused_do_not_stop_collection_and_are_not_asked_again() {
        shadowOf(app).grantPermissions(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
        )
        shadowOf(app).denyPermissions(*mediaPermissionsToAsk(Build.VERSION.SDK_INT).toTypedArray())
        val controller = Robolectric.buildActivity(MediaCountingMainActivity::class.java).create().resume()
        val activity = controller.get()
        assertEquals(1, activity.groups.size)

        // 断られた（実状態は未許可のまま）
        activity.onRequestPermissionsResult(
            1,
            mediaPermissionsToAsk(Build.VERSION.SDK_INT).toTypedArray(),
            IntArray(mediaPermissionsToAsk(Build.VERSION.SDK_INT).size) { PackageManager.PERMISSION_DENIED },
        )
        controller.pause().resume()

        assertEquals("断られた権限を求め直している", 1, activity.groups.size)
        // 入口が閉じるかは見ない: 閉じるかは写真の権限の外の条件（休止の対象かどうか。ST13）でも決まる
        assertNotNull("写真の権限を断られただけで収集が始まっていない", shadowOf(app).peekNextStartedService())
    }

    // Scenario: 写真の権限が無くても位置は集まる
    @Test
    fun location_is_collected_even_without_any_photo_permission() {
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val service = controller.get()
        service.mediaPermissions = MediaPermissionState(MediaAccess.NONE, location = false)
        service.realMediaCapability = true
        controller.create().startCommand(0, 1)

        service.source.deliver()

        assertTrue(
            "写真の権限が無いだけで位置が積まれていない",
            service.outboxForTest.snapshot().any { it.logicalSource == LOGICAL_SOURCE },
        )
        val media = MEDIA_CADENCES.map { it.logicalSource }
        assertFalse(
            "権限の無い写真の記録が積まれている",
            service.outboxForTest.snapshot().any { it.logicalSource in media },
        )
    }
}
