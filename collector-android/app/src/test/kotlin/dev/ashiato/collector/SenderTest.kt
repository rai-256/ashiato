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

    /** 受理と、**恒久的な**断り（`malformed`）を並べた応答。 */
    private fun okFor(vararg accepted: Boolean) = { _: String ->
        val results = accepted.joinToString(",") {
            if (it) """{"id":null,"duplicate":false,"accepted":true,"error":null}"""
            else """{"id":null,"duplicate":false,"accepted":false,"error":"malformed"}"""
        }
        Outcome.Responded(200, "[$results]")
    }

    /** 理由の種別を 1 件ずつ指定した応答。 */
    private fun resultsFor(vararg errors: String?) = { _: String ->
        val results = errors.joinToString(",") { e ->
            if (e == null) """{"accepted":true}"""
            else """{"accepted":false,"error":"$e"}"""
        }
        Outcome.Responded(200, "[$results]")
    }

    /**
     * 記録の送信器。**恒久的に断られたら捨てる側**（ST03 / FR-10 の改訂）——
     * `LocationService` が本番で組み立てるのと同じ形。
     * 生存信号は捨てない側（既定）で、そちらは `HeartbeatOutboxTest` が見る。
     */
    private fun sender(outbox: Outbox<IngestRequest>, transport: Transport, log: (String) -> Unit = {}) =
        Sender(outbox, transport, IngestRequest.serializer(), dropPermanentlyRejected = true, log = log)

    // Scenario: 到達できるとき送られる
    // Scenario: 複数件が 1 回の送信でまとまる
    @Test
    fun `5件たまっていても送信は1回`() {
        val outbox = testOutbox()
        repeat(5) { outbox.add(req("id-$it")) }
        val transport = FakeTransport(okFor(true, true, true, true, true))

        val flushed = sender(outbox, transport).flush()

        assertEquals(1, transport.bodies.size)                       // **1 回にまとまる**
        assertEquals(5, Json.parseToJsonElement(transport.bodies[0]).let { (it as JsonArray).size })
        assertEquals(Sender.Flushed(sent = 5, accepted = 5, removed = 5, responded = true), flushed)
        assertEquals(0, outbox.size())
    }

    // Scenario: 一部が失敗しても成功分は残らない
    // Scenario: 恒久的に断られた記録は未送信から消える
    @Test
    fun `成功分も恒久的に断られた分も未送信から消える`() {
        // > **2026-09-12（ST03）に向きが変わった。** 以前は断られた 1 件を未送信に残していたが、
        // > **5 分ごとに送られ続け、1 回に載る上限（200 件）までたまると新しい記録が
        // > 送られなくなる**（FR-10 の改訂 / 深掘り Q4 / Q5）。要求そのものが不正だと
        // > サーバが 1 件ごとの結果で告げている以上、再び送っても結果は変わらない。
        val outbox = testOutbox()
        listOf("a", "b", "c").forEach { outbox.add(req(it)) }
        val transport = FakeTransport(okFor(true, false, true))

        val flushed = sender(outbox, transport).flush()

        assertEquals(0, outbox.size())
        assertEquals(Sender.Flushed(sent = 3, accepted = 2, removed = 3, responded = true), flushed)
    }

    // Scenario: 捨てた件数と理由が端末のログに残る
    @Test
    fun `捨てた件数と理由の種別がログに残る`() {
        // **これが唯一、断られていることに気付ける経路。** 画面に出る経路は無く、
        // 自動で気付くのは ST02 の途絶通知（位置なら 18 時間後）だけ
        val outbox = testOutbox()
        listOf("a", "b", "c").forEach { outbox.add(req(it)) }
        val transport = FakeTransport { _ ->
            Outcome.Responded(
                200,
                """[{"accepted":true},{"accepted":false,"error":"malformed"},
                    {"accepted":false,"error":"malformed"}]""",
            )
        }
        val lines = mutableListOf<String>()

        sender(outbox, transport) { lines += it }.flush()

        val dropped = lines.single { it.startsWith("kind=dropped ") }
        // **何を失ったかを後から数えられる**（R118）。`id` は私的データではない
        assertEquals(
            "捨てた記録の識別子が残っていない: $lines",
            2,
            lines.count { it.startsWith("kind=dropped_item ") },
        )
        assertTrue("件数が出ていない: $dropped", dropped.contains("count=2"))
        assertTrue("理由の種別が出ていない: $dropped", dropped.contains("error=malformed"))
        // **値は出さない**（製造準備 A-2）。緯度経度がログに載っていないこと
        assertTrue("私的データが出ている: $dropped", !dropped.contains("35.68"))
    }

    /**
     * **登録簿を直せば通る断りは捨てない**（R107）。理由の種別を見ずに全件捨てていたときは、
     * `external_id_kind` の書き忘れ（既定は断る側）や登録簿の 1 行不足で、
     * **気付く前にその期間の記録が消えていた** —— 気付くのは稼働状況の画面で、
     * 位置なら想定間隔の 3 倍＝18 時間後。
     */
    @Test
    fun `登録簿を直せば通る断りは捨てずに未送信へ残す`() {
        for (recoverable in listOf("unknown_source", "missing_external_id", "brand_new_error_kind")) {
            val outbox = testOutbox()
            outbox.add(req("a"))
            val lines = mutableListOf<String>()
            sender(outbox, FakeTransport(resultsFor(recoverable))) { lines += it }.flush()
            assertEquals("$recoverable を捨てている", 1, outbox.size())
            // **「捨てた」ではなく「断られた」として残る**（ログから区別できる）
            assertTrue(
                "$recoverable が dropped として出ている: $lines",
                lines.none { it.startsWith("kind=dropped") },
            )
            assertTrue(
                "$recoverable が rejected として出ていない: $lines",
                lines.any { it.startsWith("kind=rejected ") && it.contains("error=$recoverable") },
            )
        }
    }

    /** 恒久的な 6 種は捨てる（許可リストの中身を固定する）。 */
    @Test
    fun `要求そのものが不正な種別だけを捨てる`() {
        for (permanent in Sender.PERMANENT_ERRORS) {
            val outbox = testOutbox()
            outbox.add(req("a"))
            sender(outbox, FakeTransport(resultsFor(permanent))).flush()
            assertEquals("$permanent を捨てていない", 0, outbox.size())
        }
    }

    /**
     * **`accepted` を欠く応答では 1 件も取り除かない**（R108）。
     * 既定値を置いていたときは、要求と同じ数のオブジェクトが並んだ配列なら何でも
     * 復号に成功し、**全件「受理されなかった」と読んで捨てていた**。
     */
    @Test
    fun `受理の欄を欠く応答では1件も取り除かない`() {
        val outbox = testOutbox()
        listOf("a", "b").forEach { outbox.add(req(it)) }
        val transport = FakeTransport { Outcome.Responded(200, """[{"ok":1},{"ok":2}]""") }
        val lines = mutableListOf<String>()
        sender(outbox, transport) { lines += it }.flush()
        assertEquals(2, outbox.size())
        assertTrue(
            "読めない応答として扱われていない: $lines",
            lines.any { it.contains("error=unreadable_response") },
        )
    }

    // Scenario: 一時的な失敗では捨てない
    @Test
    fun `一時的な失敗では1件も取り除かない`() {
        // 到達できない / サーバ側の失敗 / 資格情報の不一致は、**再び送れば結果が変わる**
        // **サーバが約束しているのは 200 と 400 だけ**（R119）。それ以外は一時的に倒す ——
        // 403（トークン失効・WAF）と 429 は、`>= 500` だけを見ていたときに
        // 本文の復号へ進み、中身が配列に見えれば全件捨てていた
        val temporary = listOf(
            Outcome.Unreachable("timeout"),
            Outcome.Responded(500, "internal error"),
            Outcome.Responded(503, ""),
            Outcome.Responded(401, "unauthorized"),
            Outcome.Responded(403, """[{"accepted":false,"error":"malformed"}]"""),
            Outcome.Responded(429, """[{"accepted":false,"error":"malformed"}]"""),
        )
        for (outcome in temporary) {
            // **保持の上限を超えていない状態で**（ST04 で WHEN が変わった）。上限による破棄は送信の結果と独立に働くので、
            // 上限の見回りも同じ契機で走らせ、それでも 1 件も取り除かれないことを見る
            val st = TestStores()
            val retention = Retention(st.records, st.ledger, st.age::now)
            listOf("a", "b").forEach { st.records.add(req(it)) }
            st.clock.advance(89 * AgeClock.DAY_MS)
            sender(st.records, FakeTransport { outcome }).flush()
            assertEquals("上限を超えていないのに捨てている", 0, retention.enforce())
            assertEquals("$outcome で捨てている", 2, st.records.size())
            assertTrue("破棄の報告が作られている", st.ledger.drafts().isEmpty())
        }
    }

    @Test
    fun `1件も受け付けられない400でも本文を読んで断られた分を取り除く`() {
        // 400 は「1 件も受け付けなかった」。**恒久的な拒否なので未送信から消える** ——
        // 残すと同じ 1 件が 5 分ごとに永久に送られ続ける
        val outbox = testOutbox()
        outbox.add(req("a"))
        val transport = FakeTransport { Outcome.Responded(400, """[{"accepted":false,"error":"unknown_origin"}]""") }

        sender(outbox, transport).flush()

        assertEquals(0, outbox.size())
    }

    // Scenario: 失敗しても失われない
    @Test
    fun `到達できなければ全部残り次の契機で再び送る`() {
        val outbox = testOutbox()
        listOf("a", "b").forEach { outbox.add(req(it)) }
        val transport = FakeTransport { Outcome.Unreachable("timeout") }

        assertEquals(Sender.Flushed(sent = 2, accepted = 0), sender(outbox, transport).flush())
        assertEquals(2, outbox.size())

        // 次の契機では同じ 2 件が送られる
        val ok = FakeTransport(okFor(true, true))
        sender(outbox, ok).flush()
        assertEquals(0, outbox.size())
    }

    @Test
    fun `資格情報が無くて401なら何も取り除かない`() {
        val outbox = testOutbox()
        outbox.add(req("a"))
        sender(outbox, FakeTransport { Outcome.Responded(401, "unauthorized") }).flush()
        assertEquals(1, outbox.size())
    }

    @Test
    fun `応答の件数が合わなければ何も取り除かない`() {
        // 取り違えて消すと記録が失われる。**消さない側に倒す**
        val outbox = testOutbox()
        listOf("a", "b").forEach { outbox.add(req(it)) }
        sender(outbox, FakeTransport(okFor(true))).flush()
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

        val flushed = sender(outbox, transport).flush()

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

        sender(outbox, transport).flush()

        assertEquals(listOf("id-$MAX_BATCH", "id-${MAX_BATCH + 1}", "id-${MAX_BATCH + 2}"),
            outbox.snapshot().map { it.id })
    }

    @Test
    fun `空のときは送らない`() {
        val transport = FakeTransport(okFor())
        assertEquals(Sender.Flushed(0, 0), sender(testOutbox(), transport).flush())
        assertTrue(transport.bodies.isEmpty())
    }
}
