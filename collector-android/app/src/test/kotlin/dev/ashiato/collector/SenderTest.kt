// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** まとめ送りと部分失敗（tasks 7.2 / 7.3 / design D9）。 */
class SenderTest {
    private fun req(id: String) =
        LocationFix(35.681236, 139.767125, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 送られた本文を覚える偽の取り込み口。 */
    private class FakeTransport(val reply: (String) -> Outcome) : Transport {
        val bodies = mutableListOf<String>()
        override fun post(bodyJson: String): Outcome {
            bodies += bodyJson
            return reply(bodyJson)
        }
    }

    private fun okFor(vararg accepted: Boolean) = { _: String ->
        val results = accepted.joinToString(",") { """{"id":null,"duplicate":false,"accepted":$it,"error":null}""" }
        Outcome.Responded(200, "[$results]")
    }

    // Scenario: 到達できるとき送られる
    // Scenario: 複数件が 1 回の送信でまとまる
    @Test
    fun `5件たまっていても送信は1回`() {
        val outbox = testOutbox()
        repeat(5) { outbox.add(req("id-$it")) }
        val transport = FakeTransport(okFor(true, true, true, true, true))

        val flushed = Sender(outbox, transport).flush()

        assertEquals(1, transport.bodies.size)                       // **1 回にまとまる**
        assertEquals(5, Json.parseToJsonElement(transport.bodies[0]).let { (it as JsonArray).size })
        assertEquals(Sender.Flushed(sent = 5, accepted = 5), flushed)
        assertEquals(0, outbox.size())
    }

    // Scenario: 一部が失敗しても成功分は残らない
    @Test
    fun `一部が失敗したら失敗分だけ残る`() {
        val outbox = testOutbox()
        listOf("a", "b", "c").forEach { outbox.add(req(it)) }
        val transport = FakeTransport(okFor(true, false, true))

        Sender(outbox, transport).flush()

        assertEquals(listOf("b"), outbox.snapshot().map { it.id })
    }

    @Test
    fun `1件も受け付けられない400でも本文を読んで成功分を取り除く`() {
        // 400 は「1 件も受け付けなかった」。取り除くものが無いことを確かめる
        val outbox = testOutbox()
        outbox.add(req("a"))
        val transport = FakeTransport { Outcome.Responded(400, """[{"accepted":false,"error":"unknown_origin"}]""") }

        Sender(outbox, transport).flush()

        assertEquals(listOf("a"), outbox.snapshot().map { it.id })
    }

    // Scenario: 失敗しても失われない
    @Test
    fun `到達できなければ全部残り次の契機で再び送る`() {
        val outbox = testOutbox()
        listOf("a", "b").forEach { outbox.add(req(it)) }
        val transport = FakeTransport { Outcome.Unreachable("timeout") }

        assertEquals(Sender.Flushed(sent = 2, accepted = 0), Sender(outbox, transport).flush())
        assertEquals(2, outbox.size())

        // 次の契機では同じ 2 件が送られる
        val ok = FakeTransport(okFor(true, true))
        Sender(outbox, ok).flush()
        assertEquals(0, outbox.size())
    }

    @Test
    fun `資格情報が無くて401なら何も取り除かない`() {
        val outbox = testOutbox()
        outbox.add(req("a"))
        Sender(outbox, FakeTransport { Outcome.Responded(401, "unauthorized") }).flush()
        assertEquals(1, outbox.size())
    }

    @Test
    fun `応答の件数が合わなければ何も取り除かない`() {
        // 取り違えて消すと記録が失われる。**消さない側に倒す**
        val outbox = testOutbox()
        listOf("a", "b").forEach { outbox.add(req(it)) }
        Sender(outbox, FakeTransport(okFor(true))).flush()
        assertEquals(2, outbox.size())
    }

    @Test
    fun `1回に載せる件数には上限がある`() {
        // **未送信が永続化されて日をまたぐようになった**（design D17）。上限が無いと、
        // 長い圏外のあと 1 回の POST が読み取り上限を超え、1 件も取り除けないまま
        // 5 分ごとに同じ全件を送り続けて**二度と復帰しない**（design D23 / review R4）
        val outbox = testOutbox()
        repeat(MAX_BATCH + 50) { outbox.add(req("id-$it")) }
        val transport = FakeTransport(okFor(*BooleanArray(MAX_BATCH) { true }))

        val flushed = Sender(outbox, transport).flush()

        assertEquals(MAX_BATCH, flushed.sent)
        assertEquals(MAX_BATCH, Json.parseToJsonElement(transport.bodies[0]).let { (it as JsonArray).size })
        // 残りは消えていない。次の契機で送られる（60 秒に 1 件しか増えないので追いつく）
        assertEquals(50, outbox.size())
    }

    @Test
    fun `古いものから先に送る`() {
        // 上限で切るときに新しい側から送ると、**古い記録が永久に後回しになる**
        val outbox = testOutbox()
        repeat(MAX_BATCH + 3) { outbox.add(req("id-$it")) }
        val transport = FakeTransport(okFor(*BooleanArray(MAX_BATCH) { true }))

        Sender(outbox, transport).flush()

        assertEquals(listOf("id-$MAX_BATCH", "id-${MAX_BATCH + 1}", "id-${MAX_BATCH + 2}"),
            outbox.snapshot().map { it.id })
    }

    @Test
    fun `空のときは送らない`() {
        val transport = FakeTransport(okFor())
        assertEquals(Sender.Flushed(0, 0), Sender(testOutbox(), transport).flush())
        assertTrue(transport.bodies.isEmpty())
    }
}
