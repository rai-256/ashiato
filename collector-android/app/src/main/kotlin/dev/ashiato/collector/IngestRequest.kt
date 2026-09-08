// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject

/**
 * 取り込み口へ送る 1 件。**Rust 側（crates/server/src/ingest.rs）と同じ形**でなければならない。
 * 契約の正典は docs/collector-contract.md。片方だけ直すと、同じ 1 件が別物として入る。
 *
 * **冪等キーはここでは作らない。** 鍵の算出はサーバ側だけが行う（design D1）。
 */
@Serializable
data class IngestRequest(
    val id: String,
    @SerialName("user_id") val userId: String,
    @SerialName("logical_source") val logicalSource: String,
    @SerialName("external_id") val externalId: String? = null,
    @SerialName("device_id") val deviceId: String? = null,
    val origin: String,
    @SerialName("event_time") val eventTime: String,
    @SerialName("tz_offset_min") val tzOffsetMin: Int,
    @SerialName("tz_id") val tzId: String,
    @SerialName("schema_version") val schemaVersion: Int,
    @SerialName("unit_system") val unitSystem: String? = null,
    val crs: String? = null,
    val raw: JsonObject,
    val payload: JsonObject,
)

/** 送った 1 件ごとの結果。**位置で対応づける**（docs/collector-contract.md §返る形）。 */
@Serializable
data class IngestResult(
    val id: String? = null,
    val duplicate: Boolean = false,
    /** 未送信から取り除いてよいか。**収集側はこれだけを見る** */
    val accepted: Boolean = false,
    val error: String? = null,
)

/** 契約どおりの JSON を作る唯一の入口。欄名の食い違いをここ 1 か所に閉じる。 */
val ingestJson: Json = Json {
    encodeDefaults = true
    ignoreUnknownKeys = true   // サーバが欄を足しても収集側は落ちない
    explicitNulls = true
}
