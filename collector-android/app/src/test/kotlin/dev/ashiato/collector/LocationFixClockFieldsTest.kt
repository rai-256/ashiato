// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.location.Location
import com.google.android.gms.location.LocationResult
import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 位置の記録が「受け取ったときの端末の時計」の 3 項目を持つ（ST05 / design D6）。
 *
 * **本番経路（`FixCollector.onLocationResult`）を通す。** 変換関数だけを試すと、
 * 時計から値を取る配線が切れていても気付けない。
 */
@RunWith(RobolectricTestRunner::class)
class LocationFixClockFieldsTest {
    private val fixTime = Instant.parse("2026-09-08T02:00:00Z")

    private fun location(): Location =
        Location("fused").apply {
            latitude = 35.68
            longitude = 139.76
            accuracy = 10f
            time = fixTime.toEpochMilli()
            elapsedRealtimeNanos = 123_456_789_000L
        }

    private fun receive(clock: DeviceClock): IngestRequest {
        val outbox = testOutbox()
        FixCollector(
            outbox = outbox,
            deviceId = "device-1",
            userId = "user-1",
            zone = ZoneId.of("Asia/Tokyo"),
            newId = { "id-1" },
            clock = clock,
        ).onLocationResult(LocationResult.create(listOf(location())))
        return outbox.snapshot().single()
    }

    private fun IngestRequest.rawObject(): JsonObject = Json.parseToJsonElement(raw).jsonObject

    // Scenario: 位置の記録が受け取ったときの端末の時計の時刻を持つ
    @Test
    fun `受け取ったときの端末の壁時計が測位の時刻とは別の項目で載る`() {
        // 端末の時計は測位の結果の時刻より 5 分進んでいる
        val received = fixTime.plusSeconds(300)
        val r = receive(FakeDeviceClock(wall = received.toEpochMilli(), mono = 42_000L, boot = 7))

        for (obj in listOf(r.rawObject(), r.payload.jsonObject)) {
            assertEquals("2026-09-08T02:05:00Z", obj["received_device_time"]!!.jsonPrimitive.content)
            assertEquals("2026-09-08T02:00:00Z", obj["device_time"]!!.jsonPrimitive.content)
        }
    }

    // Scenario: 位置の記録が測位の起動からの経過時間と起動の識別を持つ
    @Test
    fun `測位の経過時間と受け取りの経過時間と起動の識別が載る`() {
        val r = receive(FakeDeviceClock(wall = fixTime.toEpochMilli(), mono = 42_000L, boot = 7))

        for (obj in listOf(r.rawObject(), r.payload.jsonObject)) {
            assertEquals("123456789000", obj["fix_elapsed_ns"]!!.jsonPrimitive.content)
            assertEquals("42000", obj["received_elapsed_ms"]!!.jsonPrimitive.content)
            assertEquals("7", obj["boot_count"]!!.jsonPrimitive.content)
        }
    }

    // Scenario: 位置の記録が測位の起動からの経過時間と起動の識別を持つ
    @Test
    fun `起動の識別が取れない端末では null が載る`() {
        val r = receive(FakeDeviceClock(boot = null))

        assertEquals(JsonNull, r.rawObject()["boot_count"])
        assertEquals(JsonNull, r.payload.jsonObject["boot_count"])
    }

    // Scenario: 位置の記録の出来事時刻は測位の結果が持つ時刻のまま
    @Test
    fun `端末の時計が測位の時刻と違っても出来事時刻は測位の結果の時刻のまま`() {
        val r = receive(FakeDeviceClock(wall = fixTime.plusSeconds(300).toEpochMilli()))

        assertEquals("2026-09-08T02:00:00Z", r.eventTime)
        assertEquals("2026-09-08T02:00:00Z", r.rawObject()["device_time"]!!.jsonPrimitive.content)
    }

    @Test
    fun `同じ位置の記録から原文を 2 回組み立てると同じ文字列になる`() {
        // 冪等キーは原文から作られる（design D16）。新しい欄が入っても再送で鍵が変わらない
        val fix = LocationFix(
            latitude = 35.68, longitude = 139.76, accuracyMeters = 10f, at = fixTime,
            receivedDeviceTime = fixTime.plusSeconds(1), fixElapsedNs = 5L, receivedElapsedMs = 6L, bootCount = 7,
        )
        val tokyo = ZoneId.of("Asia/Tokyo")
        val a = fix.toIngestRequest("id-1", "user-1", "device-1", tokyo).raw
        val b = fix.toIngestRequest("id-2", "user-1", "device-1", tokyo).raw

        assertEquals(a, b)
        assertTrue(a.contains("\"received_device_time\":\"2026-09-08T02:00:01Z\""))
    }
}
