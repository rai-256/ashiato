// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.content.Intent
import android.location.Location
import androidx.test.core.app.ApplicationProvider
import com.google.android.gms.location.LocationCallback
import com.google.android.gms.location.LocationResult
import java.time.DateTimeException
import java.time.Instant
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.shadows.ShadowLog

/** 位置の契機を試験から配れる取得元。 */
private class CapturingFixSource : FixSource {
    var callback: LocationCallback? = null
    override fun start(intervalMs: Long, callback: LocationCallback) {
        this.callback = callback
    }
    override fun stop(callback: LocationCallback) = Unit
}

/** OS の時計の偽物。`broken` なら読む前に例外（測定そのものが落ちる状態）。 */
private class ScriptedSources : SystemTimeSources {
    var network: Long? = null
    var broken = false
    override val sdkInt: Int get() = if (broken) throw IllegalStateException("boom") else 36
    override fun networkMs(): Long = network ?: throw DateTimeException("none")
    override fun gnssMs(): Long = throw DateTimeException("none")
    override fun wallMs(): Long = System.currentTimeMillis()
    override fun monoMs(): Long = 500
}

/** 本番の `LocationService` に、測定の刻みと OS の時計の偽物を差す。 */
private class ClockTestService : TestableLocationService() {
    val fixes = CapturingFixSource()
    val hourly = FakeScheduler()
    val retry = FakeScheduler()
    val sources = ScriptedSources()

    override fun newFixSource(): FixSource = fixes
    override fun newClockScheduler(): ClockTicks = ClockTicks(hourly, retry)
    override fun newSystemTimeSources(): SystemTimeSources = sources
}

/** 端末の時計の測定を `LocationService` に組み込んだ配線（ST05 / design D3 / D4 / D11）。 */
@RunWith(RobolectricTestRunner::class)
class LocationServiceClockTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun create(configure: ClockTestService.() -> Unit = {}): Pair<org.robolectric.android.controller.ServiceController<ClockTestService>, ClockTestService> {
        val controller = Robolectric.buildService(ClockTestService::class.java, Intent()).create()
        controller.get().configure()
        controller.startCommand(0, 1)
        return controller to controller.get()
    }

    private fun clockRecords(s: TestableLocationService) =
        s.outboxForTest.snapshot().filter { it.logicalSource == CLOCK_LOGICAL_SOURCE }

    private fun location(at: Instant) = Location("fused").apply {
        latitude = 35.68
        longitude = 139.76
        accuracy = 10f
        time = at.toEpochMilli()
    }

    private fun withConfig(block: () -> Unit) {
        Config.overrideForTest(baseUrl = "http://127.0.0.1:1", apiToken = "t", userId = "u")
        try {
            block()
        } finally {
            Config.clearOverrideForTest()
        }
    }

    @Test
    fun `起動すると測定記録が記録の未送信に 1 件積まれ 1 時間と測り直しの刻みが立つ`() {
        val (_, s) = create()

        val records = clockRecords(s)
        assertEquals(1, records.size)
        assertEquals("start", records.single().payload["trigger"]!!.jsonPrimitive.content)
        assertEquals(CLOCK_SKEW_INTERVAL_MS, s.hourly.periodMs)
        assertEquals(CLOCK_SKEW_RETRY_MS, s.retry.periodMs) // 基準が無いので測り直しに入る
    }

    // Scenario: 測定記録の論理ソースは生存信号を送らない
    @Test
    fun `生存信号の刻みが来ても位置の論理ソースだけで測定記録の論理ソースは送られない`() {
        withConfig {
            val (_, s) = create()
            assertEquals(1, clockRecords(s).size)

            s.beatScheduler.fire()
            s.scheduler.fire()

            val beats = s.posted.filter { it.first == "/heartbeat" }
            assertTrue("生存信号が送られていない", beats.isNotEmpty())
            assertTrue(beats.all { it.second.contains(LOGICAL_SOURCE) })
            assertTrue(beats.none { it.second.contains(CLOCK_LOGICAL_SOURCE) })
            assertTrue(s.heartbeatOutboxForTest.snapshot().all { it.logicalSource == LOGICAL_SOURCE })
        }
    }

    // Scenario: 到達できない間の測定記録は後から届く
    @Test
    fun `到達できない間の測定記録は到達できるようになると受け口へ送られる`() {
        withConfig {
            val (_, s) = create()
            s.reply = { _, _ -> Outcome.Unreachable("timeout") }
            s.scheduler.fire()
            assertEquals("到達できないのに取り除かれた", 1, clockRecords(s).size)

            s.reply = { _, n -> Outcome.Responded(200, (1..n).joinToString(",", "[", "]") { """{"accepted":true}""" }) }
            s.scheduler.fire()

            assertTrue(s.posted.any { it.first == "/ingest" && it.second.contains(CLOCK_LOGICAL_SOURCE) })
            assertEquals(0, clockRecords(s).size)
        }
    }

    // Scenario: 測定記録も保持の上限で捨てられ破棄として報告される
    @Test
    fun `90 日を超えた測定記録は捨てられ、破棄の報告に測定記録の論理ソースの件数が載る`() {
        val (_, s) = create()
        assertEquals(1, clockRecords(s).size)

        s.clock.advance(90 * AgeClock.DAY_MS + 60_000)
        s.maintainForTest()

        assertEquals(0, clockRecords(s).size)
        val draft = s.ledgerForTest.drafts().single { it.source == CLOCK_LOGICAL_SOURCE }
        assertEquals(1, draft.count)
    }

    // Scenario: 位置の記録の時刻は補正されない
    @Test
    fun `端末の時計が進んでいても位置の記録の出来事時刻は測位の結果の時刻のまま`() {
        val (_, s) = create()
        val at = Instant.parse("2026-09-08T02:00:00Z")
        s.clock.wall = at.toEpochMilli() + 300_000 // 端末の時計が 5 分進んでいる
        s.sources.network = at.toEpochMilli()
        s.hourly.fire()

        s.fixes.callback!!.onLocationResult(LocationResult.create(listOf(location(at))))

        val fix = s.outboxForTest.snapshot().single { it.logicalSource == LOGICAL_SOURCE }
        assertEquals(at, Instant.parse(fix.eventTime))
        assertTrue(clockRecords(s).size >= 2)
    }

    // Scenario: 測定が例外で落ちても位置の記録は生成される
    @Test
    fun `測定が例外で落ちても位置の記録は契機ごとに 1 件生成され続ける`() {
        val (_, s) = create { sources.broken = true }
        val at = Instant.parse("2026-09-08T02:00:00Z")

        repeat(3) { i ->
            s.hourly.fire() // 測定の契機
            s.fixes.callback!!.onLocationResult(LocationResult.create(listOf(location(at.plusSeconds(60L * i)))))
        }

        assertEquals(3, s.outboxForTest.snapshot().count { it.logicalSource == LOGICAL_SOURCE })
        assertEquals(0, clockRecords(s).size)
    }

    // Scenario: 測定の失敗は種別だけがログに残る
    @Test
    fun `測定の失敗はログに種別だけが残り、時刻の値と位置の値は含まれない`() {
        ShadowLog.clear()
        val (_, s) = create { sources.broken = true }
        s.fixes.callback!!.onLocationResult(LocationResult.create(listOf(location(Instant.parse("2026-09-08T02:00:00Z")))))

        val lines = ShadowLog.getLogs().map { it.msg }.filter { it.contains("clock_skew_crashed") }

        assertEquals(listOf("kind=clock_skew_crashed source=$LOGICAL_SOURCE error=IllegalStateException"), lines)
        assertTrue(lines.none { it.contains("35.68") || it.contains("139.76") || it.contains("boom") || Regex("20\\d\\d-\\d\\d").containsMatchIn(it) })
    }

    @Test
    fun `時計の変更の通知でその場で測り、応答の日付を捨て、タイムゾーンの変更では測らない`() {
        val (_, s) = create { sources.network = System.currentTimeMillis() }
        assertEquals(1, clockRecords(s).size)

        app.sendBroadcast(Intent(Intent.ACTION_TIMEZONE_CHANGED))
        org.robolectric.shadows.ShadowLooper.idleMainLooper()
        assertEquals("タイムゾーンの変更で測っている", 1, clockRecords(s).size)

        app.sendBroadcast(Intent(Intent.ACTION_TIME_CHANGED))
        org.robolectric.shadows.ShadowLooper.idleMainLooper()

        val records = clockRecords(s)
        assertEquals(2, records.size)
        assertEquals("time_set", records.last().payload["trigger"]!!.jsonPrimitive.content)
        assertTrue(records.last().raw.contains("clock_changed_since"))
    }

    @Test
    fun `止めると時計の変更の受け手が解除され、測る刻みも畳まれる`() {
        val (controller, s) = create { sources.network = System.currentTimeMillis() }

        controller.destroy()
        app.sendBroadcast(Intent(Intent.ACTION_TIME_CHANGED))
        org.robolectric.shadows.ShadowLooper.idleMainLooper()

        assertEquals("解除後に測っている", 1, clockRecords(s).size)
        assertTrue("1 時間の刻みが畳まれていない", s.hourly.cancelled)
        assertTrue("測り直しの刻みが畳まれていない", s.retry.cancelled)
    }
}
