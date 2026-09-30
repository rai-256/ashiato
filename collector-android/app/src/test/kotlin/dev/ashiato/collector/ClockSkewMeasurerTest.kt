// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.DateTimeException
import java.time.ZoneId
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** 端末の測定記録の組み立て（ST05 / design D5）。OS の時計・端末の時計は偽物に差し替える。 */
class ClockSkewMeasurerTest {
    private class FakeSources(
        override var sdkInt: Int = 36,
        var network: (() -> Long)? = null,
        var gnss: (() -> Long)? = null,
        var wall: Long = 1_800_000_000_000,
        var mono: Long = 500,
    ) : SystemTimeSources {
        override fun networkMs(): Long = network?.invoke() ?: throw DateTimeException("none")

        override fun gnssMs(): Long = gnss?.invoke() ?: throw DateTimeException("none")

        override fun wallMs(): Long = wall

        override fun monoMs(): Long = mono
    }

    private val date = "Tue, 29 Sep 2026 00:51:00 GMT"
    private val dateMs = 1_790_643_060_000L

    private fun measure(
        sources: FakeSources,
        cache: ResponseDateCache = ResponseDateCache(),
        clock: DeviceClock = FakeDeviceClock(wall = sources.wall, mono = 123_456_789, boot = 42),
        trigger: String = "hourly",
    ): IngestRequest =
        ClockSkewMeasurer(
            references = ClockReferences(sources, cache),
            clock = clock,
            deviceId = "device-1",
            userId = "user-1",
            zone = ZoneId.of("Asia/Tokyo"),
            newId = { "id-1" },
        ).measure(trigger)

    private fun IngestRequest.refs(): List<JsonObject> = payload["references"]!!.jsonArray.map { it.jsonObject }

    private fun IngestRequest.unavail(): List<JsonObject> = payload["unavailable"]!!.jsonArray.map { it.jsonObject }

    private fun JsonObject.str(k: String) = getValue(k).jsonPrimitive.content

    // Scenario: 端末の時計を 5 分進めると差が約 300000 ミリ秒で残る
    @Test
    fun `端末の時計がネットワーク時刻より 5 分進んでいると差が 300000 で残る`() {
        val s = FakeSources(wall = 1_800_000_000_000 + 300_000, network = { 1_800_000_000_000 })

        val net = measure(s).refs().single { it.str("source") == "network" }

        val before = net.getValue("mono_before_ms").jsonPrimitive.long
        val after = net.getValue("mono_after_ms").jsonPrimitive.long
        val skew = net.getValue("skew_ms").jsonPrimitive.long
        assertTrue(skew in (300_000 - (after - before))..(300_000 + (after - before)))
    }

    // Scenario: 端末の時計が遅れていると差が負で残る
    @Test
    fun `端末の時計がネットワーク時刻より 5 分遅れていると差が負で残る`() {
        val s = FakeSources(wall = 1_800_000_000_000 - 300_000, network = { 1_800_000_000_000 })

        val net = measure(s).refs().single { it.str("source") == "network" }

        val width = net.getValue("mono_after_ms").jsonPrimitive.long - net.getValue("mono_before_ms").jsonPrimitive.long
        val skew = net.getValue("skew_ms").jsonPrimitive.long
        assertTrue(skew in (-300_000 - width)..(-300_000 + width))
    }

    // Scenario: 応答の日付の差は秒の分解能の幅に収まる
    @Test
    fun `S-01 の応答の日付の差は 300000 以上 301000 未満で残る`() {
        val cache = ResponseDateCache()
        // 端末の時計は S-01 より 5 分進み、日付の秒未満の切り捨て分（0〜999 ms）だけ余分に出る
        cache.put(ResponseDateCache.Received(date, monoBeforeMs = 100, monoAfterMs = 100, wallAfterMs = dateMs + 300_000 + 700))
        val s = FakeSources()

        val d = measure(s, cache).refs().single { it.str("source") == "s01-date" }

        val skew = d.getValue("skew_ms").jsonPrimitive.long
        assertTrue("差=$skew", skew in 300_000 until 301_000)
        assertEquals(date, d.str("raw"))
    }

    // Scenario: 測定記録に測った時刻が入っている
    @Test
    fun `測ったときの端末の壁時計が device_time と出来事時刻に入る`() {
        val s = FakeSources(wall = 1_800_000_000_123)

        val r = measure(s)

        assertEquals("2027-01-15T08:00:00.123Z", r.payload.str("device_time"))
        assertEquals(r.eventTime, r.payload.str("device_time"))
        assertEquals(r.eventTime, Json.parseToJsonElement(r.raw).jsonObject.str("device_time"))
    }

    // Scenario: 起動の識別と起動からの経過時間が入っている
    @Test
    fun `起動の識別と起動からの経過時間が入る`() {
        val r = measure(FakeSources())

        assertEquals(123_456_789L, r.payload.getValue("elapsed_ms").jsonPrimitive.long)
        assertEquals(42, r.payload.getValue("boot_count").jsonPrimitive.int)
    }

    // Scenario: 起動の識別が取れない端末では取れないことが残る
    @Test
    fun `起動の識別が取れない端末では boot_count が null で elapsed_ms は残る`() {
        val r = measure(FakeSources(), clock = FakeDeviceClock(mono = 777, boot = null))

        assertEquals(JsonNull, r.payload.getValue("boot_count"))
        assertEquals(777L, r.payload.getValue("elapsed_ms").jsonPrimitive.long)
    }

    // Scenario: 取れる基準は全部並ぶ
    @Test
    fun `取れる 3 つの基準が 1 件に並び、それぞれが必要な欄を持つ`() {
        val cache = ResponseDateCache()
        cache.put(ResponseDateCache.Received(date, 10, 20, dateMs + 5_000))
        val s = FakeSources(network = { 1_799_999_999_000 }, gnss = { 1_799_999_998_000 })

        val r = measure(s, cache)

        assertEquals(listOf("network", "gnss", "s01-date"), r.refs().map { it.str("source") })
        for (ref in r.refs()) {
            for (k in listOf("source", "time", "skew_ms", "mono_before_ms", "mono_after_ms")) {
                assertTrue("$k がある", ref.containsKey(k))
            }
        }
        assertTrue(r.payload.getValue("available").jsonPrimitive.boolean)
        assertEquals(0, r.unavail().size)
    }

    // Scenario: 3 つの出どころは取れたか取れなかったかのどちらかに 1 回ずつ出る
    @Test
    fun `基準の取れ方 8 通りのどれでも 3 つの出どころが参照か取れなかったかに 1 回ずつ出る`() {
        val all = listOf("network", "gnss", "s01-date")
        for (mask in 0 until 8) {
            val cache = ResponseDateCache()
            if (mask and 4 != 0) cache.put(ResponseDateCache.Received(date, 1, 2, dateMs))
            val s = FakeSources(
                network = if (mask and 1 != 0) ({ 1_799_999_999_000 }) else null,
                gnss = if (mask and 2 != 0) ({ 1_799_999_999_000 }) else null,
            )

            val r = measure(s, cache)

            val seen = (r.refs() + r.unavail()).map { it.str("source") }
            assertEquals("mask=$mask", all.sorted(), seen.sorted())
            assertEquals("mask=$mask", Integer.bitCount(mask), r.refs().size)
            assertEquals("mask=$mask", mask != 0, r.payload.getValue("available").jsonPrimitive.boolean)
        }
    }

    // Scenario: 自宅 PC に届かない間も端末の中の基準で測る
    @Test
    fun `応答が無くてもネットワーク時刻か衛星の時刻が取れれば測定記録が 1 件できる`() {
        val r = measure(FakeSources(gnss = { 1_799_999_999_000 }))

        assertTrue(r.payload.getValue("available").jsonPrimitive.boolean)
        assertEquals(listOf("gnss"), r.refs().map { it.str("source") })
    }

    // Scenario: 届かない間の応答の日付は取れなかった基準に残る
    @Test
    fun `応答が無い間の s01-date は理由つきで取れなかった側に残る`() {
        val r = measure(FakeSources(network = { 1_799_999_999_000 }))

        val d = r.unavail().single { it.str("source") == "s01-date" }
        assertEquals("no_response_since_last", d.str("reason"))
    }

    @Test
    fun `読めなかった Date の原文と宛先は取れなかった側に残る`() {
        val cache = ResponseDateCache()
        cache.put(ResponseDateCache.Received("not a date", 10, 20, dateMs, host = "s01.lan:8787"))

        val r = measure(FakeSources(), cache)

        val d = r.unavail().single { it.str("source") == "s01-date" }
        assertEquals("unreadable", d.str("reason"))
        assertEquals("not a date", d.str("raw"))
        assertEquals("s01.lan:8787", d.str("host"))
        // 原文の無い取れなかった基準には欄を足さない
        assertEquals(setOf("source", "reason"), r.unavail().single { it.str("source") == "network" }.keys)
    }

    // Scenario: 測定記録は位置の記録と別のソースに入る
    @Test
    fun `論理ソースは c01-clock で位置の記録の論理ソースと違う`() {
        val r = measure(FakeSources())

        assertEquals("c01-clock", r.logicalSource)
        assertNotEquals(LOGICAL_SOURCE, r.logicalSource)
        assertEquals("collected", r.origin)
        assertEquals("device-1", r.deviceId)
    }

    // Scenario: 基準が 1 つも取れないと取れなかった印の付いた記録が 1 件残る
    @Test
    fun `基準が 1 つも取れないと available が false で references が空の記録になる`() {
        val r = measure(FakeSources())

        assertEquals(false, r.payload.getValue("available").jsonPrimitive.boolean)
        assertEquals(JsonArray(emptyList()), r.payload.getValue("references"))
    }

    // Scenario: 取れなかった記録は基準ごとの理由を持つ
    @Test
    fun `取れなかった記録は 3 つの出どころそれぞれの理由を持つ`() {
        val r = measure(FakeSources(sdkInt = 31))

        val reasons = r.unavail().associate { it.str("source") to it.str("reason") }
        assertEquals(
            mapOf("network" to "unsupported", "gnss" to "not_available", "s01-date" to "no_response_since_last"),
            reasons,
        )
    }

    // Scenario: 取れなかった記録にも端末の時計と起動の識別が入る
    @Test
    fun `取れなかった記録にも測った時刻・起動からの経過時間・起動の識別が入る`() {
        val r = measure(FakeSources(wall = 1_800_000_000_123))

        assertEquals("2027-01-15T08:00:00.123Z", r.payload.str("device_time"))
        assertEquals(123_456_789L, r.payload.getValue("elapsed_ms").jsonPrimitive.long)
        assertEquals(42, r.payload.getValue("boot_count").jsonPrimitive.int)
    }

    @Test
    fun `契機の名前は trigger に入り、原文は payload と同じ JSON の文字列である`() {
        val r = measure(FakeSources(), trigger = "startup")

        assertEquals("startup", r.payload.str("trigger"))
        assertEquals("clock-skew", r.payload.str("kind"))
        assertEquals(r.payload, Json.parseToJsonElement(r.raw))
        assertEquals("Asia/Tokyo", r.tzId)
        assertEquals(540, r.tzOffsetMin)
    }
}
