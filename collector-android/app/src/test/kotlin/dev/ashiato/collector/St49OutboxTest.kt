// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
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
import java.time.ZoneId

/**
 * 未送信に積むときに収集アプリの版を焼き込み、読み戻した行の JSON をそのまま送る（ST49 / design D3（仮）・D13）。
 */
@RunWith(RobolectricTestRunner::class)
class St49OutboxTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val v1 = "0.1.0+" + "a".repeat(40)
    private val v2 = "0.1.0+" + "b".repeat(40)

    private fun req(id: String) =
        testFix(35.68, 139.76, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    private fun records(dir: File, stamp: String?) =
        Outbox.inDir(dir, IngestRequest.serializer(), stamp) {}

    private class Capture : Transport {
        val bodies = mutableListOf<String>()
        override fun post(bodyJson: String): Outcome {
            bodies += bodyJson
            val n = (Json.parseToJsonElement(bodyJson) as JsonArray).size
            return Outcome.Responded(200, List(n) { """{"accepted":true}""" }.joinToString(",", "[", "]"))
        }
    }

    /** 送り出して、送られた 1 件目の JSON を返す。 */
    private fun sendFirst(outbox: Outbox<IngestRequest>): JsonObject {
        val transport = Capture()
        Sender(outbox, transport, IngestRequest.serializer(), dropPermanentlyRejected = true).flush()
        return (Json.parseToJsonElement(transport.bodies.single()) as JsonArray).first().jsonObject
    }

    /** 版の欄を持たない、今の形の 1 行（ST49 より前の収集アプリが積んだもの。逐語）。 */
    private val oldLine =
        """{"enq":0,"item":{"id":"old-1","user_id":"user-1","logical_source":"c01-location","external_id":null,""" +
            """"device_id":"device-1","origin":"device","event_time":"2026-09-08T02:00:00Z","tz_offset_min":540,""" +
            """"tz_id":"Asia/Tokyo","schema_version":1,"unit_system":null,"crs":null,"source_updated_at":null,""" +
            """"external_ref":null,"raw":"{\"lat\":35.68}","payload":{"lat":35.68}}}""" + "\n"

    private fun writeSegment(dir: File, line: String) {
        val seg = File(dir, "segments").apply { mkdirs() }
        File(seg, "000000000000.jsonl").writeText(line)
    }

    // Scenario: 携帯の収集アプリを更新した後に送った未送信は、積んだときの版で届く
    @Test
    fun pendingKeepsStampFromAddTime() {
        val dir = tmp.newFolder("pending")
        records(dir, v1).add(req("r1"))

        // 収集アプリを更新した（別の印で組み直された）後に、同じ置き場から送る
        val updated = records(dir, v2)
        val sent = sendFirst(updated)

        assertEquals("積んだときの版で届く", v1, sent["client_version"]!!.jsonPrimitive.content)
    }

    // Scenario: 版の欄を持たない未送信の行は、版の欄の無いまま送られる
    @Test
    fun oldLineIsSentWithoutVersion() {
        val dir = tmp.newFolder("old")
        writeSegment(dir, oldLine)

        val sent = sendFirst(records(dir, v2))

        assertEquals("old-1", sent["id"]!!.jsonPrimitive.content)
        assertFalse("今の版を付けてはならない", sent.containsKey("client_version"))
        assertEquals("読めない行として退避された", 0L, File(dir, "unreadable.jsonl").length())
    }

    // Scenario: 携帯の未送信の行が知らない欄を持っていても、送るときに落ちない
    @Test
    fun unknownFieldSurvivesResend() {
        val dir = tmp.newFolder("unknown")
        val payload = """"payload":{"lat":35.68}"""
        writeSegment(dir, oldLine.replace(payload, """$payload,"future_field":{"a":[1,2]}"""))
        val outbox = records(dir, v2)

        val sent = sendFirst(outbox)

        assertEquals("知らない欄が送り返される", """{"a":[1,2]}""", sent["future_field"].toString())
        assertEquals("送れた行は取り除かれる", 0, outbox.size())
    }

    // Scenario: 古い形の未送信を移すとき、版は書き込まれない
    @Test
    fun legacyMigrationDoesNotStamp() {
        val dir = tmp.newFolder("legacy")
        val legacy = File(tmp.newFolder("legacy-src"), "outbox.jsonl")
        legacy.writeText(ingestJson.encodeToString(IngestRequest.serializer(), req("m1")) + "\n")
        val into = records(dir, v2)

        val got = migrateLegacyOutbox(
            legacy = legacy,
            into = into,
            serializer = IngestRequest.serializer(),
            unreadable = File(dir, "unreadable.jsonl"),
            salvaged = File(dir, "salvaged"),
            recordSalvaged = { true },
            enqAgeMs = 0L,
            log = {},
        )

        assertEquals(1, got.imported)
        assertFalse("移した行に版が書き込まれた", sendFirst(into).containsKey("client_version"))
    }

    // Scenario: 携帯の生存信号には版が書き込まれない
    @Test
    fun heartbeatHasNoVersion() {
        val service = Robolectric.buildService(TestableLocationService::class.java, android.content.Intent())
            .create().startCommand(0, 1).get()
        service.beatScheduler.fire()
        service.outboxForTest.add(req("rec-1"))

        val beats = service.heartbeatOutboxForTest.headStored(10)
        assertTrue("生存信号が積まれていない", beats.isNotEmpty())
        assertTrue("生存信号に版が書き込まれた", beats.none { it.json?.containsKey("client_version") == true })
        // 記録の溜めには同じ配線で印が書かれる（対の確認。印の値は組み立ての印）
        val record = service.outboxForTest.headStored(10).single { it.item.id == "rec-1" }
        assertEquals(BuildConfig.COMPONENT_VERSION, record.json!!["client_version"]!!.jsonPrimitive.content)
    }
}
