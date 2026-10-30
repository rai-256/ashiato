// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.EncodeDefault
import kotlinx.serialization.ExperimentalSerializationApi
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

    /**
     * 登録簿の名前。**どのソースが書いたものかを積んだ後からも読めるようにする**（ST06 / R10）——
     * 送信の失敗をログに残すとき、`Telemetry` に焼き込んだ 1 本の名前ではなく
     * **その 1 件が属するソース**を名乗るために要る。
     */
    val logicalSource: String

    /**
     * 積んだ時の送信契約の版（ST50 / design D1）。**積む 1 か所（`Outbox.add`）で焼き込み**、送るときは積んだ値のまま送る。
     * 版を持たない前の形の行を読み戻すと 1。
     */
    val contractVersion: Int

    /** 版だけを差し替えた写し（`Outbox.add` が焼き込みに使う。同じ型を返す）。 */
    fun withContractVersion(version: Int): Outboxable
}

/**
 * 送信契約の版（ST50 / design D12）。S-01 の `CONTRACT_VERSION`・PC の `CONTRACT_VERSION`・
 * `docs/contract/envelope.json` の `version` と同じ値でなければならない（`tools/check-contract-version.sh` が突き合わせる）。
 */
const val CONTRACT_VERSION: Int = 1

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
    @SerialName("logical_source") override val logicalSource: String,
    @SerialName("external_id") val externalId: String? = null,
    @SerialName("device_id") val deviceId: String? = null,
    val origin: String,
    @SerialName("event_time") override val eventTime: String,
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
    /**
     * この記録を積んだ収集アプリの版（ST49 / design D13）。`Outbox.add` が積むときに書く。
     * **値が無いときは欄ごと送らない**（古い形の取り込み・版の欄の無い行を、版の欄の無いまま送る）。
     */
    @OptIn(ExperimentalSerializationApi::class)
    @EncodeDefault(EncodeDefault.Mode.NEVER)
    @SerialName("client_version") val clientVersion: String? = null,
    /** 積んだ時の契約の版（ST50 / D1）。**既定は 1** —— 版を持たない行（ST50 より前に積んだ）は版 1。積む時に `Outbox.add` が焼き込む */
    @SerialName("contract_version") override val contractVersion: Int = 1,
) : Retainable {
    override fun withContractVersion(version: Int): IngestRequest = copy(contractVersion = version)
}

/**
 * 送った 1 件ごとの結果。**位置で対応づける**（docs/collector-contract.md §返る形）。
 *
 * **`accepted` に既定値を置かない**（ST03 / R108）。既定 `false` ＋ `ignoreUnknownKeys` だと、
 * **「オブジェクトが要求と同じ数だけ並んだ JSON 配列」なら何でも復号に成功し、
 * 全件『受理されなかった』と読まれる** —— API ゲートウェイや captive portal が 200 で
 * 別の配列を返しただけで、それが起きる。ST02 まではその先が「何も取り除かない」だったので
 * 誤解釈が安全側に落ちていたが、**ST03 で同じ復号結果が「全件捨てる」に変わった**。
 * 必須欄にすれば、欄を欠く応答は復号に失敗し `unreadable_response`（＝1 件も取り除かない）になる。
 */
@Serializable
data class IngestResult(
    val id: String? = null,
    val duplicate: Boolean = false,
    /** 未送信から取り除いてよいか。**収集側はこれだけを見る**。**既定値を置かない** */
    val accepted: Boolean,
    val error: String? = null,
)

/** 契約どおりの JSON を作る唯一の入口。欄名の食い違いをここ 1 か所に閉じる。 */
val ingestJson: Json = Json {
    encodeDefaults = true
    ignoreUnknownKeys = true   // サーバが欄を足しても収集側は落ちない
    explicitNulls = true
}
