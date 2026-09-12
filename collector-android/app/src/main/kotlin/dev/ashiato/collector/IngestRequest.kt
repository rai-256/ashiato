// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject

/**
 * 未送信に積めるもの。**識別子で取り除く**ので、それだけを要求する（`Outbox.remove`）。
 *
 * 記録（`IngestRequest`）と生存信号（`HeartbeatRequest`）が同じ仕組みに乗る
 * —— ST02 の specs が「生存信号を、記録と同じ未送信の仕組みに乗せて再送する」と定めている。
 * 別の置き場を作ると、`FileOutboxStore` が実測で積み上げた復旧（書きかけの回収・
 * 壊れた行の退避・追記）を生存信号だけが持たないことになる。
 */
interface Outboxable {
    val id: String
}

/**
 * 取り込み口へ送る 1 件。**Rust 側（crates/server/src/ingest.rs）と同じ形**でなければならない。
 * 契約の正典は docs/collector-contract.md。片方だけ直すと、同じ 1 件が別物として入る。
 *
 * **冪等キーはここでは作らない。** 鍵の算出はサーバ側だけが行う（design D1）。
 */
@Serializable
data class IngestRequest(
    override val id: String,
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
    /**
     * 外部サービス側の更新時刻（ST03 / 深掘り Q20）。**端末の収集では常に null** ——
     * 外部サービスから取り込む ST12 / ST13 が使う。届かない到着は「届いた順」で
     * 適用され、保存済みの値を消さない。
     */
    @SerialName("source_updated_at") val sourceUpdatedAt: String? = null,
    /**
     * 「対象ごと」の外部識別子（動画 ID など。ST03 / 深掘り Q24）。
     * **重複の判定には使われない** —— 同じ対象の記録を後から集めるために持つ。
     */
    @SerialName("external_ref") val externalRef: String? = null,
    /**
     * 原文。**JSON の値ではなく文字列**（design D16 / docs/collector-contract.md）——
     * JSON の値で送るとサーバ側の DB がキー順・重複キー・数値表記を正規化し、
     * 「受け取ったまま」が成り立たなくなる。
     *
     * **同じ 1 件は毎回同じ文字列にする。** 冪等キーはこの文字列から作られるので、
     * 再送のたびに形が変わると重複が入る（FR-22）。
     */
    val raw: String,
    val payload: JsonObject,
) : Outboxable

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
