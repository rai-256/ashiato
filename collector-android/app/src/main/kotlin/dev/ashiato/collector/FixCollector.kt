// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import com.google.android.gms.location.LocationCallback
import com.google.android.gms.location.LocationResult
import java.time.Instant
import java.time.ZoneId

/**
 * 位置取得の契機を受けて、契機ごとに 1 件を未送信へ積む（FR-1 / design D11）。
 *
 * **`LocationService` が実際に登録するのはこの型**。無名オブジェクトのままだと
 * 本番経路を試験から呼べず、「契機ごとに 1 件生成される」の担保が取れない（tasks 9.8）。
 *
 * 識別子の採番と時計と地域を外から渡すのは、試験のためではなく
 * **この型が端末の状態に依存しないようにする**ため。
 */
class FixCollector(
    private val outbox: Outbox,
    private val deviceId: String,
    private val userId: String,
    private val zone: ZoneId,
    private val newId: () -> String,
    private val log: (String) -> Unit = {},
) : LocationCallback() {
    override fun onLocationResult(result: LocationResult) {
        for (location in result.locations) {
            // **水平精度でふるい落とさない**（design D11）。捨てた記録は復元できない
            val fix = LocationFix(
                latitude = location.latitude,
                longitude = location.longitude,
                accuracyMeters = location.accuracy,
                at = Instant.ofEpochMilli(location.time),
            )
            outbox.add(fix.toIngestRequest(newId(), userId, deviceId, zone))
        }
        // 出すのは件数だけ。位置の値はログに出さない（製造準備 A-2）
        log(Telemetry.line("fix", count = result.locations.size))
    }
}
