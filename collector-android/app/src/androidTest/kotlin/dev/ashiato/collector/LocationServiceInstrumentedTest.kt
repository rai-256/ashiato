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
import com.google.android.gms.location.FusedLocationProviderClient
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
        // **終わるまで待つ**（`executeShellCommand` は非同期。閉じるだけだと、直後の `setMockMode` が
        // 許可の反映より先に走って `Caller must be selected as the mock location app` になる。
        // `RetentionInstrumentedTest.shell` と同じ理由）
        shell("appops set ${context.packageName} android:mock_location allow")
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
        shell("appops set ${context.packageName} android:mock_location deny")
    }

    private fun shell(cmd: String) {
        val fd = InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(cmd)
        android.os.ParcelFileDescriptor.AutoCloseInputStream(fd).use { it.readBytes() }
    }

    /**
     * **サービスを立てる前に**偽装を有効にする（code-verify R29）。
     *
     * 立ててから有効にすると、その間に端末の本物の位置（エミュレータの GPS は
     * `39.237255,-123.150032`）が fused から配られ、**先頭の記録が偽装でない位置になる**
     * （実測 2026-09-27: 単独で走らせても毎回落ちた。`dumpsys location` の gps の最後の位置がその値）。
     * 偽装を先に有効にしておけば、サービスが受け取る位置は全部この試験が入れたものになる。
     * 許可の反映が遅れて断られることがあるので、数回だけ当たり直す（`RetentionInstrumentedTest.mockLocations` と同じ）。
     */
    private fun enableMockMode(client: FusedLocationProviderClient) {
        var lastError: Throwable? = null
        for (attempt in 1..5) {
            if (runCatching { Tasks.await(client.setMockMode(true), 10, TimeUnit.SECONDS) }.onFailure { lastError = it }.isSuccess) return
            Thread.sleep(1_000)
        }
        throw AssertionError("偽装位置を有効にできない", lastError)
    }

    // Scenario: 契機ごとに 1 件生成される
    @Test
    fun aMockFixBecomesOneRecordInTheOutbox() {
        val client = LocationServices.getFusedLocationProviderClient(context)
        enableMockMode(client)
        ActivityScenario.launch(MainActivity::class.java).close()

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
        // 測定記録（c01-clock）や、偽装の前に届いた本物の位置も同じ未送信に積まれるので、偽装した位置の記録だけを見る
        while (System.currentTimeMillis() < deadline) {
            Tasks.await(client.setMockLocation(fix), 10, TimeUnit.SECONDS)
            Thread.sleep(1_000)
            stored = SegmentStore(File(outboxDir, "records"), IngestRequest.serializer(), File(outboxDir, "u.jsonl"), {}).readAll()
                .filter { it.logicalSource == "c01-location" && it.raw.contains("35.681236") }
            if (stored.isNotEmpty()) break
        }
        assertTrue("偽装した位置が未送信に 1 件以上書かれる（${outboxDir.exists()}）", stored.isNotEmpty())
        val first = stored.first()
        assertEquals("c01-location", first.logicalSource)
        assertEquals("collected", first.origin)
        assertTrue("原文に緯度が残る: ${first.raw}", first.raw.contains("35.681236"))
    }
}
