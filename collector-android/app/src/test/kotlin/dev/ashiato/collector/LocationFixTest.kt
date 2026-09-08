// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
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
        assertEquals(35.681236, r.raw["lat"]!!.jsonPrimitive.content.toDouble(), 1e-9)
        assertEquals(139.767125, r.raw["lon"]!!.jsonPrimitive.content.toDouble(), 1e-9)
        assertEquals(12.5, r.raw["acc_m"]!!.jsonPrimitive.content.toDouble(), 1e-6)
        assertEquals("2026-09-08T02:00:00Z", r.raw["device_time"]!!.jsonPrimitive.content)
        assertEquals("device-1", r.raw["device_id"]!!.jsonPrimitive.content)
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
