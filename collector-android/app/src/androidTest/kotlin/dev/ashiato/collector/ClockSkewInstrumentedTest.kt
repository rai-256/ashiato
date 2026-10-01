// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import java.io.File
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * **本物の `LocationService` が端末の時計のずれを測って残す**（ST05）。
 *
 * サービスは `MainActivity` 経由で立てる（`LocationServiceInstrumentedTest` と同じ入口）。
 * 測定記録は記録の未送信（ファイル）から読む。時計は `cmd alarm set-time`（Task 1.2 で変えられると確かめた）で変える。
 */
@RunWith(AndroidJUnit4::class)
class ClockSkewInstrumentedTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
    )

    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val outboxDir get() = File(context.filesDir, "outbox")
    private val uiAutomation get() = InstrumentationRegistry.getInstrumentation().uiAutomation

    private var restoreClockAtMs: Long? = null
    private var clockChangedAtElapsedMs = 0L

    @Before
    fun cleanStart() {
        context.stopService(Intent(context, LocationService::class.java))
        outboxDir.deleteRecursively()
    }

    @After
    fun tearDown() {
        context.stopService(Intent(context, LocationService::class.java))
        restoreClockAtMs?.let { setClock(it + (android.os.SystemClock.elapsedRealtime() - clockChangedAtElapsedMs)) }
        shell("settings put global auto_time 1")
    }

    private fun shell(command: String) {
        uiAutomation.executeShellCommand(command).use { fd ->
            java.io.FileInputStream(fd.fileDescriptor).readBytes()
        }
    }

    private fun setClock(ms: Long) = shell("cmd alarm set-time $ms")

    private fun clockRecords(): List<IngestRequest> =
        SegmentStore(File(outboxDir, "records"), IngestRequest.serializer(), File(outboxDir, "u.jsonl"), {})
            .readAll()
            .filter { it.logicalSource == CLOCK_LOGICAL_SOURCE }

    private fun waitFor(what: String, timeoutMs: Long = 30_000, until: () -> Boolean) {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (until()) return
            Thread.sleep(500)
        }
        throw AssertionError("時間内に満たされなかった: $what")
    }

    private fun trigger(r: IngestRequest) = r.payload["trigger"]!!.jsonPrimitive.content

    // Scenario: 収集の起動時にその場で測る
    @Test
    fun startingTheServiceQueuesOneClockRecordWithEveryReferenceAccountedFor() {
        ActivityScenario.launch(MainActivity::class.java).close()
        waitFor("start の測定記録が未送信に積まれる") { clockRecords().any { trigger(it) == "start" } }

        val start = clockRecords().first { trigger(it) == "start" }
        assertEquals(CLOCK_LOGICAL_SOURCE, start.logicalSource)
        assertEquals("collected", start.origin)
        assertEquals("clock-skew", start.payload["kind"]!!.jsonPrimitive.content)
        // 3 つの出どころは references か unavailable のどちらかに 1 回ずつ出る
        val sources = (start.payload["references"] as JsonArray).map { it.jsonObject["source"]!!.jsonPrimitive.content } +
            (start.payload["unavailable"] as JsonArray).map { it.jsonObject["source"]!!.jsonPrimitive.content }
        assertEquals(listOf("gnss", "network", "s01-date"), sources.sorted())
        assertTrue("raw と payload が同じ種類を持つ: ${start.raw}", start.raw.contains("\"clock-skew\""))
    }

    // Scenario: 端末の時計が変更されるとその場で測る
    @Test
    fun changingTheDeviceClockQueuesATimeSetRecordRightAway() {
        ActivityScenario.launch(MainActivity::class.java).close()
        waitFor("start の測定記録が未送信に積まれる") { clockRecords().any { trigger(it) == "start" } }
        val before = clockRecords().count { trigger(it) == "time_set" }

        shell("settings put global auto_time 0")
        val now = System.currentTimeMillis()
        restoreClockAtMs = now
        clockChangedAtElapsedMs = android.os.SystemClock.elapsedRealtime()
        setClock(now + 5 * 60 * 1000L)

        waitFor("time_set の測定記録が 1 件増える") { clockRecords().count { trigger(it) == "time_set" } > before }
        assertEquals(before + 1, clockRecords().count { trigger(it) == "time_set" })
    }
}
