// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.location.Location
import com.google.android.gms.location.LocationResult
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 位置取得の契機が 1 件になる（tasks 9.8 / specs/device-collection の
 * 「契機ごとに 1 件生成される」）。
 *
 * **本番経路を通す。** `LocationService` が実際に `requestLocationUpdates` へ渡すのが
 * この `FixCollector` で、OS が呼ぶのがこの `onLocationResult`。
 * 変換関数だけを試すと、契機と件数の対応が担保されない。
 */
@RunWith(RobolectricTestRunner::class)
class FixCollectorTest {
    private val at = Instant.parse("2026-09-08T02:00:00Z")

    private fun location(lat: Double, lon: Double, accuracy: Float): Location =
        Location("fused").apply {
            latitude = lat
            longitude = lon
            this.accuracy = accuracy
            time = at.toEpochMilli()
        }

    private fun collector(outbox: Outbox, log: (String) -> Unit = {}): FixCollector {
        var n = 0
        return FixCollector(
            outbox = outbox,
            deviceId = "device-1",
            userId = "user-1",
            zone = ZoneId.of("Asia/Tokyo"),
            newId = { "id-${n++}" },
            log = log,
        )
    }

    // Scenario: 契機ごとに 1 件生成される
    @Test
    fun `契機ごとに1件生成される`() {
        val outbox = testOutbox()
        val callback = collector(outbox)

        // OS が呼ぶのと同じ形で 3 回の契機を渡す
        repeat(3) { callback.onLocationResult(LocationResult.create(listOf(location(35.68, 139.76, 10f)))) }

        assertEquals(3, outbox.size())
    }

    // Scenario: 契機ごとに 1 件生成される
    @Test
    fun `緯度・経度・水平精度・端末時刻・端末識別子が入る`() {
        val outbox = testOutbox()
        collector(outbox).onLocationResult(
            LocationResult.create(listOf(location(35.681236, 139.767125, 12.5f))),
        )

        val raw = outbox.snapshot().single().raw
        for (part in listOf("\"lat\":35.681236", "\"lon\":139.767125", "\"acc_m\":12.5",
                            "\"device_time\":\"2026-09-08T02:00:00Z\"", "\"device_id\":\"device-1\"")) {
            assertTrue("原文に $part が無い: $raw", raw.contains(part))
        }
    }

    // 印は置かない —— **渡した定数が消えていないことしか見ていない**（review R4）。
    // 「端末をまたいで一意」の担保は DeviceIdTest が持つ
    @Test
    fun `同じ端末の記録は同じ端末識別子を持つ`() {
        val outbox = testOutbox()
        val callback = collector(outbox)
        repeat(2) { callback.onLocationResult(LocationResult.create(listOf(location(35.68, 139.76, 10f)))) }

        assertEquals(listOf("device-1", "device-1"), outbox.snapshot().map { it.deviceId })
    }

    @Test
    fun `1回の契機で複数件届いたら件数ぶん積まれる`() {
        // FusedLocationProvider は貯めた位置をまとめて返すことがある。**取りこぼさない**
        val outbox = testOutbox()
        collector(outbox).onLocationResult(
            LocationResult.create(listOf(location(35.68, 139.76, 10f), location(35.69, 139.77, 20f))),
        )

        assertEquals(2, outbox.size())
    }

    @Test
    fun `水平精度が悪くてもふるい落とさない`() {
        // **捨てた記録は後から復元できない**（design D11。本人が決めた）
        val outbox = testOutbox()
        collector(outbox).onLocationResult(
            LocationResult.create(listOf(location(35.68, 139.76, 5000f))),
        )

        assertEquals(1, outbox.size())
    }

    @Test
    fun `取得のログに位置の値が出ない`() {
        val lines = mutableListOf<String>()
        collector(testOutbox(), lines::add).onLocationResult(
            LocationResult.create(listOf(location(35.681236, 139.767125, 10f))),
        )

        assertTrue("ログが出ていない（試験が空振りしている）", lines.isNotEmpty())
        for (line in lines) {
            for (secret in listOf("35.68", "139.76", "user-1", "device-1")) {
                assertFalse("ログに私的な内容が出ている: $line", line.contains(secret))
            }
        }
        assertTrue("件数は出てよい", lines.any { it.contains("count=1") })
    }
}
