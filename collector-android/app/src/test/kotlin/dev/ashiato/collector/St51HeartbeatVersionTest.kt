// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import java.io.File
import java.time.Instant

/**
 * 生存信号に、積んだときの収集アプリの版を欄と原文で持たせる（ST51 / design D12）。
 * ST49 の「携帯の生存信号には版が書き込まれない」の反対向き。
 */
@RunWith(RobolectricTestRunner::class)
class St51HeartbeatVersionTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val v1 = "0.1.0+" + "a".repeat(40) + ".debug"
    private val v2 = "0.1.0+" + "d".repeat(40) + ".debug"

    private class Capture : Transport {
        val bodies = mutableListOf<String>()
        override fun post(bodyJson: String): Outcome {
            bodies += bodyJson
            val n = (Json.parseToJsonElement(bodyJson) as JsonArray).size
            return Outcome.Responded(200, List(n) { """{"accepted":true}""" }.joinToString(",", "[", "]"))
        }
    }

    private fun sendFirst(outbox: Outbox<HeartbeatRequest>): JsonObject {
        val transport = Capture()
        Sender(outbox, transport, HeartbeatRequest.serializer()).flush()
        return (Json.parseToJsonElement(transport.bodies.single()) as JsonArray).first().jsonObject
    }

    private fun emitter(outbox: Outbox<HeartbeatRequest>, version: String?) = HeartbeatEmitter(
        outbox = outbox,
        counters = AttemptCounters(now = { Instant.parse("2026-05-01T00:00:00Z") }, intervalMs = FIX_INTERVAL_MS),
        userId = "user-1",
        deviceId = "device-1",
        logicalSource = LOGICAL_SOURCE,
        capability = { Capability.of(permission = true, sensor = true, network = true) },
        now = { Instant.parse("2026-05-01T06:00:00Z") },
        newId = { "hb-1" },
        version = version,
    )

    // Scenario: 携帯の生存信号は、積んだときの版を欄と原文に持つ
    @Test
    fun heartbeatKeepsVersionFromAddTime() {
        val dir = tmp.newFolder("added")
        // 印 v1 の収集アプリが積む。送る前に v2 の収集アプリへ入れ替わる（置き場は同じ。送る側は版を足さない）
        assertTrue(emitter(Outbox.inDir(dir, HeartbeatRequest.serializer()) {}, v1).emit())
        val updated = Outbox.inDir(dir, HeartbeatRequest.serializer()) {}
        assertTrue("入れ替わった側の版", v2 != v1)

        val sent = sendFirst(updated)

        assertEquals("欄は積んだときの版", v1, sent["client_version"]!!.jsonPrimitive.content)
        val raw = Json.parseToJsonElement(sent["raw"]!!.jsonPrimitive.content).jsonObject
        assertEquals("原文も積んだときの版", v1, raw["client_version"]!!.jsonPrimitive.content)
    }

    // Scenario: 版の欄を持たない溜めた生存信号は、版の無いまま送られる
    @Test
    fun oldHeartbeatIsSentWithoutVersion() {
        val dir = tmp.newFolder("old")
        val rawText = """{"alive":true,"emitted_at":"2026-05-01T00:00:00Z"}"""
        val line =
            """{"enq":0,"item":{"id":"old-1","user_id":"user-1","logical_source":"$LOGICAL_SOURCE",""" +
                """"device_id":"device-1","emitted_at":"2026-05-01T00:00:00Z","capturable":true,"blockers":[],""" +
                """"attempts":3,"successes":2,"raw":${JsonPrimitive(rawText)}}}""" + "\n"
        File(File(dir, "segments").apply { mkdirs() }, "000000000000.jsonl").writeText(line)

        val sent = sendFirst(Outbox.inDir(dir, HeartbeatRequest.serializer(), v2) {})

        assertEquals("old-1", sent["id"]!!.jsonPrimitive.content)
        assertFalse("今の版を付けてはならない", sent.containsKey("client_version"))
        assertEquals("原文は積んだときのまま", rawText, sent["raw"]!!.jsonPrimitive.content)
    }

    // 起動の直後（startBeating）に刻みを待たず 1 件積み、本物の配線で版が欄と原文に入る（premises 1）
    @Test
    fun startBeatingEmitsVersionedHeartbeatImmediately() {
        val service = Robolectric.buildService(TestableLocationService::class.java, android.content.Intent())
            .create().startCommand(0, 1).get()

        val beats = service.heartbeatOutboxForTest.headStored(10)
        assertTrue("起動の直後に生存信号が積まれていない", beats.isNotEmpty())
        beats.forEach {
            assertEquals(BuildConfig.COMPONENT_VERSION, it.json!!["client_version"]!!.jsonPrimitive.content)
            val raw = Json.parseToJsonElement(it.item.raw).jsonObject
            assertEquals(BuildConfig.COMPONENT_VERSION, raw["client_version"]!!.jsonPrimitive.content)
        }
    }
}
