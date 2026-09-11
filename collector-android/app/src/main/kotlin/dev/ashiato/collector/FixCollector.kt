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
    private val outbox: Outbox<IngestRequest>,
    private val deviceId: String,
    private val userId: String,
    private val zone: ZoneId,
    private val newId: () -> String,
    private val log: (String) -> Unit = {},
    /**
     * 取得できた契機を数える口（第 5 回 Q17）。**既定は何もしない** ——
     * 数えは生存信号のためだけのもので、記録の生成はこれに依存しない。
     */
    private val onFix: () -> Unit = {},
) : LocationCallback() {
    override fun onLocationResult(result: LocationResult) {
        var persisted = 0
        for (location in result.locations) {
            // **取得できた契機を数える。** 送れたかでも残せたかでもなく「取れたか」——
            // 生存信号の取得率は、Doze で眠っていた区間を見分けるためのもの（ST01 の R46）
            onFix()
            // **水平精度でふるい落とさない**（design D11）。捨てた記録は復元できない
            val fix = LocationFix(
                latitude = location.latitude,
                longitude = location.longitude,
                accuracyMeters = location.accuracy,
                at = Instant.ofEpochMilli(location.time),
            )
            if (outbox.add(fix.toIngestRequest(newId(), userId, deviceId, zone))) persisted++
        }
        // 出すのは件数だけ。位置の値はログに出さない（製造準備 A-2）
        log(Telemetry.line("fix", count = result.locations.size))
        // **「取れた件数」と「置き場に残せた件数」は別**（review MEDIUM-17）。
        // 揃っているときだけ黙る —— 揃っていないなら、送れないのではなく**残せていない**
        if (persisted != result.locations.size) {
            log(Telemetry.line("fix_not_persisted", count = result.locations.size - persisted))
        }
    }
}
