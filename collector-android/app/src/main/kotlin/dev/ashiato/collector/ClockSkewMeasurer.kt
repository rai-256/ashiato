// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/** 端末の時計のずれを測った記録の論理ソース。登録簿に同じ名前が要る（ST05 / design D1）。 */
const val CLOCK_LOGICAL_SOURCE: String = "c01-clock"

private val MILLIS_UTC: DateTimeFormatter =
    DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'").withZone(ZoneOffset.UTC)

/**
 * ミリ秒まで・UTC・`Z` 終わりの時刻。`Instant.toString()` は秒ちょうどでミリ秒を落とすので、
 * 測定記録の時刻の書き方はここ 1 か所に固定する（design D5）。
 */
fun clockInstantText(epochMs: Long): String = MILLIS_UTC.format(java.time.Instant.ofEpochMilli(epochMs))

/**
 * 3 つの基準（`network` / `gnss` / `s01-date`）を読み、`c01-clock` の測定記録を 1 件組み立てる（ST05 / design D5）。
 *
 * - **基準が 1 つも取れなくても記録を作る**（`available: false`、`references: []`）。3 つの出どころは
 *   必ず `references` か `unavailable` のどちらかに 1 回ずつ出る
 * - 出来事時刻は測ったときの端末の壁時計（補正しない）。通信も起こさない
 * - ログには何も出さない（時刻の値・差を出さないため、この型はログの口を持たない）
 */
class ClockSkewMeasurer(
    private val references: ClockReferences,
    private val clock: DeviceClock,
    private val deviceId: String,
    private val userId: String,
    private val zone: ZoneId,
    private val newId: () -> String,
) {
    fun measure(trigger: String): IngestRequest {
        val deviceMs = clock.wallMs()
        val elapsedMs = clock.monoMs()
        val bootCount = clock.bootCount()
        val readings = references.readAll()
        val taken = readings.filter { it.reason == null }
        val missed = readings.filter { it.reason != null }

        val fields = linkedMapOf(
            "kind" to JsonPrimitive("clock-skew"),
            "trigger" to JsonPrimitive(trigger),
            "available" to JsonPrimitive(taken.isNotEmpty()),
            "device_time" to JsonPrimitive(clockInstantText(deviceMs)),
            "elapsed_ms" to JsonPrimitive(elapsedMs),
            "boot_count" to (bootCount?.let { JsonPrimitive(it) } ?: JsonNull),
            "references" to JsonArray(taken.map(::referenceJson)),
            "unavailable" to JsonArray(missed.map(::unavailableJson)),
        )
        val at = java.time.Instant.ofEpochMilli(deviceMs)
        return IngestRequest(
            id = newId(),
            userId = userId,
            logicalSource = CLOCK_LOGICAL_SOURCE,
            externalId = null,
            deviceId = deviceId,
            origin = "collected",
            eventTime = clockInstantText(deviceMs),
            tzOffsetMin = zone.rules.getOffset(at).totalSeconds / 60,
            tzId = zone.id,
            schemaVersion = 1,
            unitSystem = null,
            crs = null,
            raw = ingestJson.encodeToString(JsonObject(fields)),
            payload = JsonObject(fields),
        )
    }

    private fun referenceJson(r: ClockReading): JsonObject {
        val f = linkedMapOf<String, kotlinx.serialization.json.JsonElement>(
            "source" to JsonPrimitive(r.source),
            "time" to JsonPrimitive(clockInstantText(r.timeMs!!)),
            "skew_ms" to JsonPrimitive(r.skewMs!!),
            "mono_before_ms" to JsonPrimitive(r.monoBeforeMs),
            "mono_after_ms" to JsonPrimitive(r.monoAfterMs),
        )
        r.rawText?.let { f["raw"] = JsonPrimitive(it) }
        r.host?.let { f["host"] = JsonPrimitive(it) }
        return JsonObject(f)
    }

    /** 読めなかった `Date` の原文と宛先も捨てない（`unreadable`。review R18）—— 読み方を直せば後から差を出せる。 */
    private fun unavailableJson(r: ClockReading): JsonObject {
        val f = linkedMapOf<String, kotlinx.serialization.json.JsonElement>(
            "source" to JsonPrimitive(r.source),
            "reason" to JsonPrimitive(r.reason!!),
        )
        r.rawText?.let { f["raw"] = JsonPrimitive(it) }
        r.host?.let { f["host"] = JsonPrimitive(it) }
        return JsonObject(f)
    }
}
