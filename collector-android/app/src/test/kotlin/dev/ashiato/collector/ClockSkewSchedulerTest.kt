// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.DateTimeException
import java.time.ZoneId
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** 端末の測る契機と測り直し（ST05 / design D3 / D4）。時計は偽物、刻みは手で進める。 */
class ClockSkewSchedulerTest {
    private class FakeSources : SystemTimeSources {
        var network: Long? = null
        override val sdkInt: Int = 36
        override fun networkMs(): Long = network ?: throw DateTimeException("none")
        override fun gnssMs(): Long = throw DateTimeException("none")
        override fun wallMs(): Long = 1_800_000_000_000
        override fun monoMs(): Long = 500
    }

    private class Rig {
        val sources = FakeSources()
        val hourly = FakeScheduler()
        val retry = FakeScheduler()
        val emitted = mutableListOf<IngestRequest>()
        val logs = mutableListOf<String>()
        val crashes = mutableListOf<Throwable>()
        var explode = false
        private val measurer = ClockSkewMeasurer(
            references = ClockReferences(sources, ResponseDateCache()),
            clock = FakeDeviceClock(),
            deviceId = "d",
            userId = "u",
            zone = ZoneId.of("Asia/Tokyo"),
            newId = { "id-${emitted.size}" },
        )
        val scheduler = ClockSkewScheduler(
            measure = { trigger -> if (explode) error("boom") else measurer.measure(trigger) },
            emit = { emitted += it },
            ticks = ClockTicks(hourly, retry),
            log = { logs += it },
            onCrash = { crashes += it },
        )

        fun triggers() = emitted.map { it.payload["trigger"]!!.jsonPrimitive.content }
        fun unavailableCount() = emitted.count { !it.payload["available"]!!.jsonPrimitive.boolean }
    }

    // Scenario: 1 時間ごとに測定記録が 1 件残る
    @Test
    fun `基準が取れる状態で 1 時間経つと測定記録が 1 件増える`() {
        val r = Rig().apply { sources.network = 1_800_000_000_000 }
        r.scheduler.start()
        assertEquals(CLOCK_SKEW_INTERVAL_MS, r.hourly.periodMs)
        val before = r.emitted.size

        r.hourly.fire()

        assertEquals(before + 1, r.emitted.size)
        assertTrue(r.emitted.last().payload["available"]!!.jsonPrimitive.boolean)
    }

    // Scenario: 測った契機が記録に残る
    @Test
    fun `1 時間・起動・時計の変更・測り直しのそれぞれで契機が記録に残る`() {
        val r = Rig()
        r.scheduler.start() // start（取れない → 測り直しへ）
        r.scheduler.timeChanged() // time_set
        r.hourly.fire() // hourly
        r.sources.network = 1_800_000_000_000
        r.retry.fire() // retry

        assertEquals(listOf("start", "time_set", "hourly", "retry"), r.triggers())
    }

    // Scenario: 収集の起動時にその場で測る
    @Test
    fun `起動すると 1 時間を待たずに測定記録が 1 件生成される`() {
        val r = Rig().apply { sources.network = 1_800_000_000_000 }

        r.scheduler.start()

        assertEquals(listOf("start"), r.triggers())
    }

    // Scenario: 端末の時計が変更されるとその場で測る
    @Test
    fun `時計の変更の通知でその場で測定記録が 1 件生成される`() {
        val r = Rig().apply { sources.network = 1_800_000_000_000 }
        r.scheduler.start()
        val before = r.emitted.size

        r.scheduler.timeChanged()

        assertEquals(before + 1, r.emitted.size)
        assertEquals("time_set", r.triggers().last())
    }

    // Scenario: 測り直しのたびには記録を増やさない
    @Test
    fun `基準が取れない状態が 1 時間続いても取れなかった記録は 1 件だけである`() {
        val r = Rig()
        r.scheduler.start()
        assertEquals(CLOCK_SKEW_RETRY_MS, r.retry.periodMs)

        repeat(11) { r.retry.fire() } // 5 分 × 11 = 55 分

        assertEquals(1, r.unavailableCount())
        assertEquals(1, r.emitted.size)
    }

    // Scenario: 測り直しで取れたら別の 1 件が残る
    @Test
    fun `測り直しで取れたら取れなかった記録とは別に retry の記録が 1 件残り測り直しは止まる`() {
        val r = Rig()
        r.scheduler.start()
        r.retry.fire()
        r.sources.network = 1_800_000_000_000

        r.retry.fire()

        assertEquals(listOf("start", "retry"), r.triggers())
        assertEquals(1, r.unavailableCount())
        assertTrue("取れたのに測り直しを続けている", r.retry.cancelled)
    }

    @Test
    fun `次の 1 時間の契機で改めて測り、まだ取れなければ取れなかった記録がもう 1 件残る`() {
        val r = Rig()
        r.scheduler.start()
        repeat(3) { r.retry.fire() }

        r.hourly.fire()

        assertEquals(listOf("start", "hourly"), r.triggers())
        assertEquals(2, r.unavailableCount())
    }

    // Scenario: 圏外が 1 日続くと取れなかった記録は 24 件
    @Test
    fun `基準が 1 つも取れないまま 24 時間動くと取れなかった記録は 24 件である`() {
        val r = Rig()
        r.scheduler.start() // 0 時間目
        repeat(23) { hour ->
            repeat(11) { r.retry.fire() }
            r.hourly.fire() // hour + 1 時間目
        }

        assertEquals(24, r.unavailableCount())
        assertEquals(24, r.emitted.size)
    }

    // Scenario: 測定のログに時刻の値と差が出ない
    @Test
    fun `測定のログは件数と種別と available だけで時刻の値と差を含まない`() {
        val r = Rig().apply { sources.network = 1_800_000_000_000 - 300_000 }

        r.scheduler.start()

        val line = r.logs.single()
        assertEquals("kind=clock_skew_measured source=c01-clock count=1 available=true", line)
        assertFalse(line.contains("300000"))
        assertFalse(line.contains("2027") || line.contains("1800000000"))
    }

    @Test
    fun `測定が例外で落ちても外へ出さず、契機の記録は積まず、失敗は onCrash に渡る`() {
        val r = Rig().apply { explode = true }

        r.scheduler.start()
        r.scheduler.timeChanged()
        r.hourly.fire()

        assertEquals(0, r.emitted.size)
        assertEquals(3, r.crashes.size)
        assertNull(r.retry.periodMs)
    }

    @Test
    fun `止めると両方の刻みが畳まれる`() {
        val r = Rig()
        r.scheduler.start()

        r.scheduler.stop()

        assertTrue(r.hourly.cancelled)
        assertTrue(r.retry.cancelled)
    }
}
