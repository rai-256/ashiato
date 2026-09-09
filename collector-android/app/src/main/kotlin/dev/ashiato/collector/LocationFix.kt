// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/** この収集アプリが名乗る論理ソース。登録簿に同じ名前が要る（FR-61）。 */
const val LOGICAL_SOURCE: String = "c01-location"

/** 位置取得の間隔（FR-1 が定めている）。 */
const val FIX_INTERVAL_MS: Long = 60_000

/** 送信の間隔。**60 秒ごとに 1 件ずつ送らない**（design D9）。 */
const val SEND_INTERVAL_MS: Long = 300_000

/**
 * 1 回の送信に載せる上限（design D23）。
 *
 * **未送信が永続化されたので、日をまたいで積み上がるようになった**（design D17）。
 * 上限が無いと、1 日圏外だった端末が 1,440 件（約 0.9 MB）を 1 回の POST に載せ、
 * サーバはそれを逐次に処理する。`HttpTransport` の読み取り上限 15 秒を超えると
 * `Unreachable` になって**1 件も取り除けず**、5 分後にまた同じ全件を送る ——
 * その間に 5 件増えているので、**二度と復帰しない**。
 *
 * 上限を置くと 1 回で送り切れないが、5 分ごとに 200 件ずつ減るので追いつく
 * （60 秒に 1 件しか増えない）。**保持の上限と破棄は ST04** の担当で、ここでは触らない。
 */
const val MAX_BATCH: Int = 200

/**
 * 端末から受け取った 1 回ぶんの位置。**ここでふるい落としはしない** ——
 * 水平精度が悪くても捨てない（design D11）。捨てた記録は後から復元できない。
 */
data class LocationFix(
    val latitude: Double,
    val longitude: Double,
    val accuracyMeters: Float,
    /** 端末時計での取得時刻 */
    val at: Instant,
)

/**
 * 契約どおりの 1 件に変換する。**原文（raw）には取得できたものをそのまま入れる** ——
 * 解釈して減らすのは payload の側の仕事（FR-18）。
 *
 * 原文は**文字列**で送る（design D16）。`ingestJson` で直列化するので、
 * 同じ `LocationFix` は毎回同じ文字列になる —— 冪等キーがこの文字列から作られるため。
 */
fun LocationFix.toIngestRequest(
    id: String,
    userId: String,
    deviceId: String,
    zone: ZoneId,
): IngestRequest {
    val offsetMin = zone.rules.getOffset(at).totalSeconds / 60
    val fields = mapOf(
        "lat" to JsonPrimitive(latitude),
        "lon" to JsonPrimitive(longitude),
        "acc_m" to JsonPrimitive(accuracyMeters),
        "device_time" to JsonPrimitive(at.toString()),
        "device_id" to JsonPrimitive(deviceId),
    )
    return IngestRequest(
        id = id,
        userId = userId,
        logicalSource = LOGICAL_SOURCE,
        externalId = null,
        deviceId = deviceId,
        origin = "collected",
        eventTime = at.toString(),
        tzOffsetMin = offsetMin,
        tzId = zone.id,
        schemaVersion = 1,
        unitSystem = "si",
        crs = "EPSG:4326",
        raw = ingestJson.encodeToString(JsonObject(fields)),
        payload = JsonObject(fields),
    )
}
