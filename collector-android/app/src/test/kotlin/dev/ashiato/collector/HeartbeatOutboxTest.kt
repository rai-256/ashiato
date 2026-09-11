// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 生存信号が**記録と同じ未送信の仕組み**に乗ること（specs/device-collection。tasks 7.3）。
 *
 * 別の置き場を作ると、`FileOutboxStore` が実測で積み上げた復旧
 * （書きかけの回収・壊れた行の退避・追記）を生存信号だけが持たないことになる。
 */
class HeartbeatOutboxTest {
    private class FakeTransport(val reply: (String) -> Outcome) : Transport {
        val bodies = mutableListOf<String>()

        override fun post(bodyJson: String): Outcome {
            bodies += bodyJson
            return reply(bodyJson)
        }
    }

    private fun beat(id: String, at: String = "2026-05-01T00:00:00Z") = HeartbeatRequest(
        id = id,
        userId = "user-1",
        logicalSource = LOGICAL_SOURCE,
        deviceId = "device-1",
        emittedAt = at,
        capturable = true,
        blockers = emptyList(),
        attempts = 360,
        successes = 230,
        raw = """{"alive":true,"emitted_at":"$at"}""",
    )

    private fun accepted(n: Int) = Outcome.Responded(
        200,
        (1..n).joinToString(",", "[", "]") { """{"accepted":true}""" },
    )

    /// Scenario: 送信に失敗した生存信号は再送される
    @Test
    fun `送信に失敗した生存信号は未送信のまま残り、同じ冪等キーで再び送られる`() {
        val outbox = testHeartbeatOutbox()
        outbox.add(beat("a"))
        outbox.add(beat("b", "2026-05-01T06:00:00Z"))

        val failing = FakeTransport { Outcome.Unreachable("timeout") }
        Sender(outbox, failing, HeartbeatRequest.serializer()).flush()
        assertEquals("失敗した分が消えている", 2, outbox.size())

        val ok = FakeTransport { accepted(2) }
        Sender(outbox, ok, HeartbeatRequest.serializer()).flush()
        assertEquals(0, outbox.size())

        // **冪等キーは `logical_source` + `emitted_at` + `raw` からサーバが作る**（第 4 回 Q13）。
        // 再送で本文が変わらないこと＝同じ鍵になること
        assertEquals(
            "再送の本文が最初と違う（別の 1 件として入る）",
            failing.bodies.single(),
            ok.bodies.single(),
        )
    }

    /** 受け付けられた分だけが取り除かれる（記録と同じ約束）。 */
    @Test
    fun `一部だけ受け付けられたら、その分だけ取り除かれる`() {
        val outbox = testHeartbeatOutbox()
        outbox.add(beat("a"))
        outbox.add(beat("b", "2026-05-01T06:00:00Z"))
        val transport = FakeTransport {
            Outcome.Responded(200, """[{"accepted":true},{"accepted":false,"error":"invalid_counts"}]""")
        }
        Sender(outbox, transport, HeartbeatRequest.serializer()).flush()
        assertEquals(listOf("b"), outbox.snapshot().map { it.id })
    }

    /**
     * 停止と再開をまたいで残る（specs「未送信を、収集の停止と再開をまたいで保持する」）。
     *
     * **インスタンスの中だけに積むと、`START_STICKY` で立て直されたときに消える** ——
     * 深掘り 第 2 回で記録の側が実際に踏んだ欠陥と同じ型。
     */
    @Test
    fun `停止と再開をまたいで残る`() {
        val dir = Files.createTempDirectory("hb").toFile()
        val file = File(dir, "heartbeat.jsonl")
        fun open() = Outbox(FileOutboxStore(file, HeartbeatRequest.serializer()) {})

        open().add(beat("a"))
        // プロセスが立て直された（新しいインスタンスで開き直す）
        val reopened = open()
        assertEquals(listOf("a"), reopened.snapshot().map { it.id })
        assertEquals(360, reopened.snapshot().single().attempts)
    }

    /** **記録と混ざらない。** 同じファイルに積むと読み戻しで片方が壊れた行に見える。 */
    @Test
    fun `記録と生存信号は別のファイルに積まれる`() {
        val dir = Files.createTempDirectory("both").toFile()
        val events = File(dir, "outbox.jsonl")
        val beats = File(dir, "heartbeat.jsonl")
        Outbox(FileOutboxStore(events, IngestRequest.serializer()) {}).add(
            LocationFix(35.68, 139.76, 10f, Instant.parse("2026-05-01T00:00:00Z"))
                .toIngestRequest("e1", "user-1", "device-1", java.time.ZoneId.of("Asia/Tokyo")),
        )
        Outbox(FileOutboxStore(beats, HeartbeatRequest.serializer()) {}).add(beat("h1"))

        assertNotEquals(events.readText(), beats.readText())
        assertTrue(events.readText().contains("\"lat\""))
        assertTrue(beats.readText().contains("\"attempts\""))
        // 読み戻しても取り違えない
        assertEquals(
            listOf("h1"),
            Outbox(FileOutboxStore(beats, HeartbeatRequest.serializer()) {}).snapshot().map { it.id },
        )
    }
}
