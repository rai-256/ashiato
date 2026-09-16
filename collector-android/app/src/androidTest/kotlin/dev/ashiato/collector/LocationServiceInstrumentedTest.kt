// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.content.Context
import android.content.Intent
import android.location.Location
import android.os.SystemClock
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import com.google.android.gms.location.LocationServices
import com.google.android.gms.tasks.Tasks
import java.io.File
import java.util.concurrent.TimeUnit
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * **本物の位置の経路**: 前景サービス → `FusedLocationProviderClient` → `FixCollector` → 未送信（ファイル）。
 *
 * 位置は偽装で入れる（Play services の mock mode。エミュレータでも実機でも同じ）。
 * Robolectric の `LocationServiceTest` は `FakeFixSource` で経路を差し替えていたので、
 * fused の callback が本当に呼ばれて 1 件が書かれることは端末の上でしか分からない。
 *
 * サービスは `MainActivity` 経由で立てる —— Android 12 以降、背景からの `startForegroundService` は
 * 拒否されるので、本番と同じ入口を通す。
 */
@RunWith(AndroidJUnit4::class)
class LocationServiceInstrumentedTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
    )

    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val outboxDir get() = File(context.filesDir, "outbox")

    @Before
    fun allowMockLocation() {
        // 偽装位置を入れる許可（設定アプリの「仮の現在地情報アプリ」と同じもの）。shell 経由でしか付けられない
        InstrumentationRegistry.getInstrumentation().uiAutomation
            .executeShellCommand("appops set ${context.packageName} android:mock_location allow")
            .close()
        context.stopService(Intent(context, LocationService::class.java))
        outboxDir.deleteRecursively()
    }

    @After
    fun tearDown() {
        context.stopService(Intent(context, LocationService::class.java))
        runCatching {
            val client = LocationServices.getFusedLocationProviderClient(context)
            Tasks.await(client.setMockMode(false), 5, TimeUnit.SECONDS)
        }
        InstrumentationRegistry.getInstrumentation().uiAutomation
            .executeShellCommand("appops set ${context.packageName} android:mock_location deny")
            .close()
    }

    // Scenario: 契機ごとに 1 件生成される
    @Test
    fun aMockFixBecomesOneRecordInTheOutbox() {
        ActivityScenario.launch(MainActivity::class.java).close()

        val client = LocationServices.getFusedLocationProviderClient(context)
        Tasks.await(client.setMockMode(true), 10, TimeUnit.SECONDS)
        val fix = Location("fused").apply {
            latitude = 35.681236
            longitude = 139.767125
            accuracy = 12f
            time = System.currentTimeMillis()
            elapsedRealtimeNanos = SystemClock.elapsedRealtimeNanos()
        }
        // サービスが購読を始めるまで少し掛かるので、何度か入れる
        val deadline = System.currentTimeMillis() + 30_000
        var stored = emptyList<IngestRequest>()
        while (System.currentTimeMillis() < deadline) {
            Tasks.await(client.setMockLocation(fix), 10, TimeUnit.SECONDS)
            Thread.sleep(1_000)
            stored = SegmentStore(File(outboxDir, "records"), IngestRequest.serializer(), File(outboxDir, "u.jsonl"), {}).readAll()
            if (stored.isNotEmpty()) break
        }
        assertTrue("偽装した位置が未送信に 1 件以上書かれる（${outboxDir.exists()}）", stored.isNotEmpty())
        val first = stored.first()
        assertEquals("c01-location", first.logicalSource)
        assertEquals("collected", first.origin)
        assertTrue("原文に緯度が残る: ${first.raw}", first.raw.contains("35.681236"))
    }
}
