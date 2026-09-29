// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.DateTimeException
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * 測定記録の形の固定（ST05 / design D5）。**欄の名前と並びを 1 文字ずつ固定する。**
 * 冪等キーは原文の文字列から作られるので、同じ入力で原文が揺れると重複が入る。
 */
class ClockSkewPayloadShapeTest {
    private class Sources(private val networkAt: Long?, private val gnssAt: Long?) : SystemTimeSources {
        override val sdkInt = 36

        override fun networkMs(): Long = networkAt ?: throw DateTimeException("none")

        override fun gnssMs(): Long = gnssAt ?: throw DateTimeException("none")

        override fun wallMs(): Long = 1_800_000_000_123

        override fun monoMs(): Long = 500
    }

    private fun build(networkAt: Long?, gnssAt: Long?, cache: ResponseDateCache, boot: Int? = 42) =
        ClockSkewMeasurer(
            references = ClockReferences(Sources(networkAt, gnssAt), cache),
            clock = FakeDeviceClock(wall = 1_800_000_000_123, mono = 123_456_789, boot = boot),
            deviceId = "device-1",
            userId = "user-1",
            zone = ZoneId.of("Asia/Tokyo"),
            newId = { "id-1" },
        ).measure("hourly")

    @Test
    fun `取れた基準の原文の欄の名前と並び`() {
        val cache = ResponseDateCache()
        cache.put(ResponseDateCache.Received("Fri, 15 Jan 2027 07:55:00 GMT", 100, 410, 1_800_000_000_456, "s01.lan:8787"))

        val r = build(1_799_999_700_100, 1_799_999_700_090, cache)

        assertEquals(
            """{"kind":"clock-skew","trigger":"hourly","available":true,""" +
                """"device_time":"2027-01-15T08:00:00.123Z","elapsed_ms":123456789,"boot_count":42,""" +
                """"references":[""" +
                """{"source":"network","time":"2027-01-15T07:55:00.100Z","skew_ms":300023,"mono_before_ms":500,"mono_after_ms":500},""" +
                """{"source":"gnss","time":"2027-01-15T07:55:00.090Z","skew_ms":300033,"mono_before_ms":500,"mono_after_ms":500},""" +
                """{"source":"s01-date","time":"2027-01-15T07:55:00.000Z","skew_ms":300456,"mono_before_ms":100,"mono_after_ms":410,""" +
                """"raw":"Fri, 15 Jan 2027 07:55:00 GMT","host":"s01.lan:8787"}""" +
                """],"unavailable":[]}""",
            r.raw,
        )
    }

    @Test
    fun `取れなかった記録の原文の欄の名前と並び`() {
        val r = build(null, null, ResponseDateCache(), boot = null)

        assertEquals(
            """{"kind":"clock-skew","trigger":"hourly","available":false,""" +
                """"device_time":"2027-01-15T08:00:00.123Z","elapsed_ms":123456789,"boot_count":null,""" +
                """"references":[],"unavailable":[""" +
                """{"source":"network","reason":"not_available"},""" +
                """{"source":"gnss","reason":"not_available"},""" +
                """{"source":"s01-date","reason":"no_response_since_last"}""" +
                """]}""",
            r.raw,
        )
    }

    @Test
    fun `同じ入力なら原文の文字列が毎回一致する`() {
        val a = build(1_799_999_700_100, null, ResponseDateCache())
        val b = build(1_799_999_700_100, null, ResponseDateCache())

        assertEquals(a.raw, b.raw)
    }
}
