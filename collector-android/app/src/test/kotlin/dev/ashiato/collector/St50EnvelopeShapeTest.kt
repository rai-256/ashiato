// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.KSerializer
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * 収集アプリが**実際に直列化する**封筒の欄を書き出す（ST50 / design D7・D21）。
 * 全部の欄を埋めた 1 件と任意の欄を空にした 1 件を本物の直列化で JSON にし、最上位の鍵・JSON の型・必須（空にした 1 件にも出る欄）を出す。
 *
 * `ASHIATO_ENVELOPE_OUT` があればそのファイルへ書く（`tools/check-contract-version.sh --write` が使う）。
 * 無ければ `docs/contract/envelope.json` の `android` と比べ、違えば欄の名前を出して落ちる。
 */
class St50EnvelopeShapeTest {
    private val full = "2026-09-13T01:02:03Z"

    private fun <T> shape(serializer: KSerializer<T>, filled: T, empty: T): JsonObject {
        fun keys(v: T) = Json.parseToJsonElement(ingestJson.encodeToString(serializer, v)) as JsonObject
        val (f, e) = keys(filled) to keys(empty)
        return buildJsonObject {
            for (key in (f.keys + e.keys).sorted()) {
                val types = listOfNotNull(f[key], e[key]).map(::jsonType).toSortedSet()
                put(
                    key,
                    buildJsonObject {
                        put("required", e.containsKey(key))
                        putJsonArray("type") { types.forEach { add(JsonPrimitive(it)) } }
                    },
                )
            }
        }
    }

    private fun jsonType(v: JsonElement): String = when (v) {
        is JsonNull -> "null"
        is JsonObject -> "object"
        is JsonArray -> "array"
        is JsonPrimitive -> when {
            v.isString -> "string"
            v.content == "true" || v.content == "false" -> "boolean"
            v.content.toLongOrNull() != null -> "integer"
            else -> "number"
        }
    }

    private fun androidEnvelope(): JsonObject {
        val ingestFull = IngestRequest(
            id = "i", userId = "u", logicalSource = LOGICAL_SOURCE, externalId = "e", deviceId = "d",
            origin = "collected",
            eventTime = full, tzOffsetMin = 540, tzId = "Asia/Tokyo", schemaVersion = 1, unitSystem = "si",
            crs = "EPSG:4326",
            sourceUpdatedAt = full, externalRef = "r", raw = "{}", payload = buildJsonObject { put("k", 1) },
        )
        val ingestEmpty = ingestFull.copy(
            externalId = null,
            deviceId = null,
            unitSystem = null,
            crs = null,
            sourceUpdatedAt = null,
            externalRef = null
        )
        val beatFull = HeartbeatRequest(
            id = "i", userId = "u", logicalSource = LOGICAL_SOURCE, deviceId = "d", emittedAt = full,
            capturable = false,
            blockers = listOf("permission"), attempts = 1, successes = 0, raw = "{}", network = true,
        )
        val beatEmpty = beatFull.copy(deviceId = null, network = null)
        val dropFull = DropReport(
            id = "i", userId = "u", logicalSource = LOGICAL_SOURCE, deviceId = "d", reason = "age", createdAt = full,
            rangeStart = full, rangeEnd = full, count = 1, hourly = listOf(DropHour(full, 1)), raw = "{}",
        )
        val dropEmpty = dropFull.copy(rangeStart = null, rangeEnd = null, hourly = emptyList())
        val missing = MediaMissingReport(
            id = "i",
            userId = "u",
            deviceId = "d",
            logicalSource = LOGICAL_SOURCE,
            sha256 = "s",
            reason = "changed",
            noticedAt = full,
        )
        return buildJsonObject {
            put("ingest", shape(IngestRequest.serializer(), ingestFull, ingestEmpty))
            put("heartbeat", shape(HeartbeatRequest.serializer(), beatFull, beatEmpty))
            put("drops", shape(DropReport.serializer(), dropFull, dropEmpty))
            put("media_missing", shape(MediaMissingReport.serializer(), missing, missing))
        }
    }

    private fun copyFile(): File {
        val cwd = File(System.getProperty("user.dir"))
        return generateSequence(cwd) { it.parentFile }
            .map { File(it, "docs/contract/envelope.json") }
            .firstOrNull { it.isFile }
            ?: error("docs/contract/envelope.json が見つからない: user.dir=$cwd")
    }

    /** 写しとコードの差分（欄の名前を出す）。 */
    private fun drift(door: String, copy: JsonObject, actual: JsonObject): List<String> =
        (copy.keys + actual.keys).sorted().mapNotNull { key ->
            val (a, b) = copy[key] to actual[key]
            when {
                a != null && b == null -> "$door: 欄 $key は写しにあるがコードの直列化に無い"
                a == null && b != null -> "$door: 欄 $key はコードの直列化にあるが写しに無い"
                a != b -> "$door: 欄 $key の型か必須が写しと違う（写し $a / コード $b）"
                else -> null
            }
        }

    @Test
    fun `実際に直列化した欄の形が写しと一致する`() {
        val actual = androidEnvelope()
        System.getenv("ASHIATO_ENVELOPE_OUT")?.let {
            File(it).writeText(Json { prettyPrint = true }.encodeToString(JsonObject.serializer(), actual))
            return
        }
        val copy = Json.parseToJsonElement(copyFile().readText()) as JsonObject
        val android = copy["android"] as JsonObject
        val out = actual.keys.flatMap { drift("android.$it", android[it] as JsonObject, actual[it] as JsonObject) }
        assertTrue("写しがコードとずれている（--write で作り直す）:\n" + out.joinToString("\n"), out.isEmpty())
    }

    @Test
    fun `写しと違う形を渡されたら欄の名前を出して落ちると分かる`() {
        val actual = androidEnvelope()
        val ingest = actual["ingest"] as JsonObject
        val without = JsonObject(ingest - "crs")
        assertEquals(1, drift("android.ingest", without, ingest).size)
        assertTrue(drift("android.ingest", without, ingest).single().contains("crs"))
    }

    @Test
    fun `空にした 1 件にも出る欄が必須で、型は二つの和になる`() {
        val ingest = androidEnvelope()["ingest"] as JsonObject
        assertEquals("""{"required":true,"type":["null","string"]}""", ingest["crs"].toString().replace(" ", ""))
        assertEquals("""{"required":true,"type":["string"]}""", ingest["raw"].toString().replace(" ", ""))
        assertEquals(
            """{"required":false,"type":["boolean"]}""",
            (androidEnvelope()["heartbeat"] as JsonObject)["network"].toString().replace(" ", "")
        )
    }
}
