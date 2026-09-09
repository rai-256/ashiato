// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test

/** 1 回の取得が契約どおりの 1 件になる（tasks 6.3 / FR-1）。 */
class LocationFixTest {
    private val tokyo = ZoneId.of("Asia/Tokyo")
    private val at = Instant.parse("2026-09-08T02:00:00Z")

    private fun fix(accuracy: Float = 12.5f) =
        LocationFix(latitude = 35.681236, longitude = 139.767125, accuracyMeters = accuracy, at = at)

    @Test
    fun `緯度・経度・水平精度・端末時刻・端末識別子を含む1件になる`() {
        val r = fix().toIngestRequest("id-1", "user-1", "device-1", tokyo)
        // 原文は**文字列**（design D16）。中身を見るときだけ解釈する
        val raw = Json.parseToJsonElement(r.raw).jsonObject
        assertEquals(35.681236, raw["lat"]!!.jsonPrimitive.content.toDouble(), 1e-9)
        assertEquals(139.767125, raw["lon"]!!.jsonPrimitive.content.toDouble(), 1e-9)
        assertEquals(12.5, raw["acc_m"]!!.jsonPrimitive.content.toDouble(), 1e-6)
        assertEquals("2026-09-08T02:00:00Z", raw["device_time"]!!.jsonPrimitive.content)
        assertEquals("device-1", raw["device_id"]!!.jsonPrimitive.content)
    }

    @Test
    fun `原文は文字列で、同じ位置なら毎回同じ文字列になる`() {
        // **冪等キーはこの文字列から作られる**（design D16 / docs/collector-contract.md）。
        // 再送のたびに形が変われば、同じ 1 件が別の鍵になって重複が入る（FR-22）。
        val a = fix().toIngestRequest("id-1", "user-1", "device-1", tokyo).raw
        val b = fix().toIngestRequest("id-2", "user-1", "device-1", tokyo).raw
        assertEquals(a, b)
        assertEquals(
            """{"lat":35.681236,"lon":139.767125,"acc_m":12.5,""" +
                """"device_time":"2026-09-08T02:00:00Z","device_id":"device-1"}""",
            a,
        )
    }

    @Test
    fun `契約どおり原文は JSON の値ではなく文字列として送られる`() {
        // JSON の値で送ると、サーバ側の DB が並び・重複キー・数値表記を正規化する（0003）
        val json = ingestJson.encodeToString(fix().toIngestRequest("id-1", "user-1", "device-1", tokyo))
        assert(json.contains("\"raw\":\"{")) { "原文が文字列で送られていない: $json" }
    }

    @Test
    fun `エンベロープの欄が埋まる`() {
        val r = fix().toIngestRequest("id-1", "user-1", "device-1", tokyo)
        assertEquals("c01-location", r.logicalSource)
        assertEquals("collected", r.origin)          // FR-25
        assertEquals("2026-09-08T02:00:00Z", r.eventTime)  // FR-19
        assertEquals(540, r.tzOffsetMin)             // FR-20
        assertEquals("Asia/Tokyo", r.tzId)           // FR-20
        assertEquals(1, r.schemaVersion)             // FR-26
        assertEquals("si", r.unitSystem)             // FR-28
        assertEquals("EPSG:4326", r.crs)             // FR-28
        assertEquals("user-1", r.userId)             // FR-29
        assertEquals("device-1", r.deviceId)         // FR-24
        assertNotNull(r.id)                          // FR-21
    }

    @Test
    fun `夏時間のある地域では取得時点のずれが入る`() {
        // ずれだけでは夏時間を再現できないので識別子も持つ（FR-20 の理由）
        val ny = ZoneId.of("America/New_York")
        val summer = LocationFix(0.0, 0.0, 1f, Instant.parse("2026-07-01T12:00:00Z"))
        val winter = LocationFix(0.0, 0.0, 1f, Instant.parse("2026-01-01T12:00:00Z"))
        assertEquals(-240, summer.toIngestRequest("a", "u", "d", ny).tzOffsetMin)
        assertEquals(-300, winter.toIngestRequest("b", "u", "d", ny).tzOffsetMin)
    }

    @Test
    fun `契約どおりの欄名で直列化される`() {
        val json = ingestJson.encodeToString(fix().toIngestRequest("id-1", "user-1", "device-1", tokyo))
        for (field in listOf(
            "\"user_id\"", "\"logical_source\"", "\"external_id\"", "\"device_id\"",
            "\"event_time\"", "\"tz_offset_min\"", "\"tz_id\"", "\"schema_version\"",
            "\"unit_system\"", "\"crs\"", "\"raw\"", "\"payload\"",
        )) {
            assert(json.contains(field)) { "欄 $field が送られていない: $json" }
        }
    }
}
