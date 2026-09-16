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
        fun open() = Outbox.inDir(dir, HeartbeatRequest.serializer()) {}

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
        val events = File(dir, "records")
        val beats = File(dir, "heartbeats")
        Outbox.inDir(events, IngestRequest.serializer()) {}.add(
            LocationFix(35.68, 139.76, 10f, Instant.parse("2026-05-01T00:00:00Z"))
                .toIngestRequest("e1", "user-1", "device-1", java.time.ZoneId.of("Asia/Tokyo")),
        )
        Outbox.inDir(beats, HeartbeatRequest.serializer()) {}.add(beat("h1"))

        fun text(d: File) = d.walk().filter { it.name.endsWith(".jsonl") && it.parentFile.name == "segments" }
            .joinToString("") { it.readText() }
        assertNotEquals(text(events), text(beats))
        assertTrue(text(events).contains("\"lat\""))
        assertTrue(text(beats).contains("\"attempts\""))
        // 読み戻しても取り違えない
        assertEquals(
            listOf("h1"),
            Outbox.inDir(beats, HeartbeatRequest.serializer()) {}.snapshot().map { it.id },
        )
    }

    /**
     * **恒久的に断られた 1 件が未送信の先頭を塞がない**（ST02 の review/code.md の R18 / H-1）。
     *
     * `accepted = false` の項目は未送信に残り続ける。先頭 `MAX_BATCH` 件が
     * 恒久的な拒否で埋まると、毎回その同じ 200 件が載り、**新しい記録には永久に順番が回らない**。
     * サーバ側は 1 件ごとの結果を返すことでこれを避けているのに、収集側に抜け道が無かった。
     */
    @Test
    fun `恒久的に断られた信号が、後ろの信号を永久に止めない`() {
        val outbox = testHeartbeatOutbox()
        outbox.add(beat("bad", "2026-05-01T00:00:00Z"))

        // 送信の契機ごとに答えを変える偽物（同じ `Sender` を使い続ける ——
        // 断られた分を覚えているのは `Sender` なので、作り直すと検査にならない）
        val transport = FakeTransport {
            // 断られるのは "bad" だけ。載っている件数ぶんの結果を返す
            val count = it.split("\"id\"").size - 1
            val results = (0 until count).joinToString(",", "[", "]") { i ->
                if (it.contains("2026-05-01T00:00:00Z") && i == 0) {
                    """{"accepted":false,"error":"unknown_source"}"""
                } else {
                    """{"accepted":true}"""
                }
            }
            Outcome.Responded(200, results)
        }
        val sender = Sender(outbox, transport, HeartbeatRequest.serializer())

        sender.flush()
        assertEquals("断られた 1 件が残る", listOf("bad"), outbox.snapshot().map { it.id })

        // 新しい 1 件が積まれる。**断られた分に押し出されない**
        outbox.add(beat("next", "2026-05-01T12:00:00Z"))
        sender.flush()
        assertEquals(
            "断られた 1 件が先頭を塞いでいる（新しい分が送られていない）",
            listOf("bad"),
            outbox.snapshot().map { it.id },
        )
        assertTrue(
            "2 回目に新しい 1 件が載っていない",
            transport.bodies[1].contains("2026-05-01T12:00:00Z"),
        )
        assertTrue(
            "2 回目にも断られた 1 件が載っている（飛ばせていない）",
            !transport.bodies[1].contains("2026-05-01T00:00:00Z"),
        )
    }
}