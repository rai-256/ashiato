// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import androidx.test.core.app.ApplicationProvider
import java.io.File
import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.Json
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 携帯の収集アプリが、S-01 の受け付ける版の行だけを見出し付きで送り、新しい版の行と 426 で断られた行を残す（ST50 / D1・D9・D14・D17・D18）。
 * 記録の送り手と原本の送り手を、偽の `Transport` で確かめる。
 */
@RunWith(RobolectricTestRunner::class)
class St50SenderTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val st by lazy { TestStores(dir = tmp.newFolder("st50")) }

    private fun req(id: String) =
        testFix(35.681236, 139.767125, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 積む時の版を `version` に差し替えられる記録の置き場（`st.recordStore` を共有する）。 */
    private class Versioned(store: SegmentStore<IngestRequest>, age: () -> Long) {
        var version = 1
        val outbox = Outbox(store, age, contractVersion = { version })
    }

    private fun versioned() = Versioned(st.recordStore, st.age::now)

    /** 1 回の送りで届いた要求（行の識別子・行ごとの版・見出しの版）。 */
    private class Posted(val ids: List<String>, val versions: List<Int>, val header: Int?)

    /** 送られた要求を覚える偽の取り込み口。見出しの版を受け取る。 */
    private class FakeTransport(val reply: (List<String>) -> Outcome) : Transport {
        val posts = mutableListOf<Posted>()

        override fun post(bodyJson: String): Outcome = record(bodyJson, null)

        override fun post(bodyJson: String, contractVersion: Int): Outcome = record(bodyJson, contractVersion)

        private fun record(body: String, header: Int?): Outcome {
            val rows = Json.parseToJsonElement(body).jsonArray.map { it.jsonObject }
            val ids = rows.map { it["id"]!!.jsonPrimitive.content }
            posts += Posted(ids, rows.map { (it as JsonObject)["contract_version"]!!.jsonPrimitive.int }, header)
            return reply(ids)
        }
    }

    private fun acceptAll() = { ids: List<String> ->
        Outcome.Responded(200, ids.joinToString(",", "[", "]") { """{"accepted":true}""" })
    }

    private fun errorsFor(vararg errors: String?) = { _: List<String> ->
        val results = errors.joinToString(",", "[", "]") { e ->
            if (e == null) """{"accepted":true}""" else """{"accepted":false,"error":"$e"}"""
        }
        Outcome.Responded(200, results)
    }

    private fun sender(
        outbox: Outbox<IngestRequest>,
        transport: Transport,
        currentVersion: Int = 2,
    ) = Sender(
        outbox,
        transport,
        IngestRequest.serializer(),
        dropPermanentlyRejected = true,
        currentVersion = { currentVersion },
    )

    // Scenario: 携帯は S-01 が受け付ける版の行だけを送り、新しい版の行を残す
    @Test
    fun `版 1 の 3 件だけが見出し 1 で送られ、版 2 の 2 件は未送信に残る`() {
        val v = versioned()
        v.version = 1
        listOf("a", "b", "c").forEach { v.outbox.add(req(it)) }
        v.version = 2
        listOf("d", "e").forEach { v.outbox.add(req(it)) }
        val transport = FakeTransport(acceptAll())

        val s = sender(v.outbox, transport)
        val flushed = s.flush(ContractAnswer.Version(1))

        val post = transport.posts.single()
        assertEquals(listOf("a", "b", "c"), post.ids)
        assertEquals(1, post.header)
        assertEquals(listOf("d", "e"), v.outbox.snapshot().map { it.id })
        assertEquals(3, flushed.removed)
        assertTrue("版 > A の行が残っていることが待つ理由の（a）に伝わらない", s.lastContract.ahead)
    }

    // Scenario: 携帯の要求の見出しは載せた行の版の最大になる
    @Test
    fun `版 1 の記録だけなら、問い合わせの口が版 2 でも見出しは 1`() {
        val v = versioned()
        v.outbox.add(req("a"))
        val transport = FakeTransport(acceptAll())

        sender(v.outbox, transport).flush(ContractAnswer.Version(2))

        assertEquals(1, transport.posts.single().header)
    }

    @Test
    fun `版 1 と版 2 が混ざった回は見出しが載せた行の最大の 2`() {
        val v = versioned()
        v.outbox.add(req("a"))
        v.version = 2
        v.outbox.add(req("b"))
        val transport = FakeTransport(acceptAll())

        sender(v.outbox, transport).flush(ContractAnswer.Version(2))

        val post = transport.posts.single()
        assertEquals(listOf("a", "b"), post.ids)
        assertEquals(listOf(1, 2), post.versions)
        assertEquals(2, post.header)
    }

    // Scenario: 携帯は積んだ時の版を 1 行ごとに持ち、更新の後も変えない
    @Test
    fun `版 1 で積んだ記録は、定数が 2 に変わった後も contract_version 1 を載せて送られる`() {
        val v = versioned()
        v.version = 1
        v.outbox.add(req("a"))
        v.version = 2 // 収集アプリが版 2 へ更新された
        val transport = FakeTransport(acceptAll())

        sender(v.outbox, transport).flush(ContractAnswer.Version(2))

        val post = transport.posts.single()
        assertEquals(listOf(1), post.versions)
        assertEquals(1, post.header)
    }

    // Scenario: 版を持たない古い行は版 1 として送られる
    @Test
    fun `ST50 より前の形の行を区切りの置き場から読み戻すと版 1 で送られる`() {
        val v = versioned()
        v.outbox.add(req("old"))
        // ST50 より前の収集アプリが書いた形（contract_version の欄が無い行）にする
        val segment = st.recordsDir.listFiles()!!.single { it.extension == "jsonl" }
        assertTrue(segment.readText().contains("\"contract_version\":1"))
        segment.writeText(segment.readText().replace(",\"contract_version\":1", ""))
        assertFalse(segment.readText().contains("contract_version"))
        val transport = FakeTransport(acceptAll())

        val fresh = Outbox(
            SegmentStore(st.recordsDir, IngestRequest.serializer(), st.unreadable, st.log),
            st.age::now,
        )
        sender(fresh, transport, currentVersion = 1).flush(ContractAnswer.Version(1))

        val post = transport.posts.single()
        assertEquals(listOf("old"), post.ids)
        assertEquals(listOf(1), post.versions)
        assertEquals(1, post.header)
    }

    /** 5 つの送り手（記録・生存信号・破棄の報告・原本の欠け・原本）にそれぞれ 1 行ずつ積んだ足場。 */
    private inner class FiveSenders {
        val h = OriginalHarness(st)
        val posts = mutableMapOf<String, FakeTransport>()
        val beatOutbox = Outbox(SegmentStore(File(tmp.newFolder("beats"), "s"), HeartbeatRequest.serializer(), st.unreadable, st.log), st.age::now)
        val dropOutbox = Outbox(SegmentStore(File(tmp.newFolder("drops"), "s"), DropReport.serializer(), st.unreadable, st.log), st.age::now)

        init {
            h.photo(1, "photo-one")
            h.scan()
            h.acceptRecords() // 原本の送り待ちが ready になる
            st.records.add(req("rec"))
            beatOutbox.add(HeartbeatRequest("hb", "user-1", LOGICAL_SOURCE, "device-1", "2026-09-08T00:00:00Z", true, emptyList(), 1, 1, "{}"))
            dropOutbox.add(
                DropReport("dr", "user-1", LOGICAL_SOURCE, "device-1", "age", "2026-09-08T00:00:00Z", null, null, 1, emptyList(), "{}"),
            )
            h.reporter.report(h.env.originals.all().single(), OriginalMissingReason.VANISHED)
        }

        private fun transport(name: String) = FakeTransport(acceptAll()).also { posts[name] = it }

        /** 5 つ全部に同じ答えを渡して 1 回ずつ送らせる。 */
        fun flushAll(answer: ContractAnswer) {
            Sender(st.records, transport("ingest"), IngestRequest.serializer(), dropPermanentlyRejected = true).flush(answer)
            Sender(beatOutbox, transport("heartbeat"), HeartbeatRequest.serializer()).flush(answer)
            Sender(dropOutbox, transport("drops"), DropReport.serializer()).flush(answer)
            Sender(h.missing, transport("media_missing"), MediaMissingReport.serializer()).flush(answer)
            h.uploader.run(answer)
        }

        fun sentAnything(): Boolean = posts.values.any { it.posts.isNotEmpty() } || h.transport.heads.isNotEmpty() || h.transport.puts.isNotEmpty()

        fun unsentCounts() = listOf(st.records.size(), beatOutbox.size(), dropOutbox.size(), h.missing.size(), h.env.originals.ready().size)
    }

    // Scenario: 問い合わせの口が無い S-01 には何も送らない
    @Test
    fun `問い合わせの口が 404 なら 5 つの送り手のどれも送らず、未送信は 1 件も減らない`() {
        val five = FiveSenders()
        val before = five.unsentCounts()

        five.flushAll(readContractAnswer(Outcome.Responded(404, "")))

        assertFalse("404 の契機で何かが送られた", five.sentAnything())
        assertEquals(before, five.unsentCounts())
        assertEquals(listOf(1, 1, 1, 1, 1), before)
    }

    // Scenario: 読めない答えの S-01 には何も送らない
    @Test
    fun `200 で版を整数で持たない本文なら 5 つの送り手のどれも送らない`() {
        listOf("<html>login</html>", "{}", """{"contract_version":"1"}""", """{"contract_version":0}""", """{"contract_version":1.5}""", "[]")
            .forEach { body ->
                val answer = readContractAnswer(Outcome.Responded(200, body))
                assertEquals(body, ContractAnswer.Unreadable, answer)
            }
        val five = FiveSenders()
        val before = five.unsentCounts()

        five.flushAll(readContractAnswer(Outcome.Responded(200, "<html>captive portal</html>")))

        assertFalse(five.sentAnything())
        assertEquals(before, five.unsentCounts())
    }

    // Scenario: 到達できないときは今どおり一時的な失敗として扱う
    @Test
    fun `到達できない・401・503 は一時的な失敗で、行は送られず、止まった状態にもならない`() {
        val answers = listOf(
            Outcome.Unreachable("SocketTimeoutException"),
            Outcome.Responded(401, ""),
            Outcome.Responded(503, ""),
        ).map(::readContractAnswer)
        assertEquals(listOf(ContractAnswer.Transient, ContractAnswer.Transient, ContractAnswer.Transient), answers)
        val five = FiveSenders()
        val before = five.unsentCounts()
        val gate = ContractGate(ContractWaitStore(prefs()))

        answers.forEach { answer ->
            five.flushAll(answer)
            assertFalse("一時的な失敗は止まった状態を始めない", gate.observe(answer, Instant.parse("2026-10-30T00:00:00Z"), emptyList()))
        }

        assertFalse(five.sentAnything())
        assertEquals(before, five.unsentCounts())
        assertFalse(gate.waiting)
        // 200 の読めない答えが挟まっていた数え始めも、一時的な失敗では捨てない
        assertNull(ContractWaitStore(prefs()).unreadableSinceMs)
    }

    // Scenario: 426 で断られた要求の行は 1 件も取り除かれない
    @Test
    fun `426 で断られた要求の行は 1 件も取り除かれず、次の契機で確かめ直す`() {
        val v = versioned()
        listOf("a", "b", "c").forEach { v.outbox.add(req(it)) }
        val refuse = FakeTransport { Outcome.Responded(426, """{"error":"contract_unsupported","contract_version":1}""") }
        val s = sender(v.outbox, refuse)

        val flushed = s.flush(ContractAnswer.Version(2))

        assertEquals(0, flushed.removed)
        assertEquals(listOf("a", "b", "c"), v.outbox.snapshot().map { it.id })
        assertTrue(s.lastContract.refused)
        // 426 は断られた 1 件の印ではない。次の契機で同じ先頭から送り直す
        val again = s.flush(ContractAnswer.Version(2))
        assertEquals(listOf("a", "b", "c"), refuse.posts.last().ids)
        assertEquals(0, again.removed)
    }

    // Scenario: unknown_field と contract_unsupported で断られた記録は捨てずに残す
    @Test
    fun `unknown_field と contract_unsupported で断られた 2 件は残り、受け付けられた 1 件だけが取り除かれる`() {
        val v = versioned()
        listOf("a", "b", "c").forEach { v.outbox.add(req(it)) }
        val transport = FakeTransport(errorsFor("unknown_field", "contract_unsupported", null))

        val s = sender(v.outbox, transport)
        val flushed = s.flush(ContractAnswer.Version(1))

        assertEquals(listOf("a", "b"), v.outbox.snapshot().map { it.id })
        assertEquals(1, flushed.removed)
        assertTrue(s.lastContract.rejectedByVersion)
        assertFalse("PERMANENT_ERRORS を変えない", "unknown_field" in Sender.PERMANENT_ERRORS || "contract_unsupported" in Sender.PERMANENT_ERRORS)
    }

    // Scenario: 契約の版で残した記録も 90 日を超えると捨てられ、破棄として報告される
    @Test
    fun `版で送れず残した記録も積んでから 90 日を超えると Retention が捨て、破棄の報告が作られる`() {
        val v = versioned()
        v.version = 2
        v.outbox.add(req("ahead"))
        val transport = FakeTransport(acceptAll())
        // 版 1 の S-01 の下で何度送ろうとしても残る
        sender(v.outbox, transport).flush(ContractAnswer.Version(1))
        assertEquals(listOf("ahead"), v.outbox.snapshot().map { it.id })
        assertTrue(transport.posts.isEmpty())

        st.clock.advance(AgeClock.DAY_MS * 90 + 60_000)
        val dropped = Retention(v.outbox, st.ledger, st.age::now).enforce()

        assertEquals(1, dropped)
        assertEquals(0, v.outbox.size())
        val draft = st.ledger.drafts().single()
        assertEquals("age", draft.reason)
        assertEquals(1, draft.count)
    }

    private fun prefs() = ApplicationProvider.getApplicationContext<Application>()
        .getSharedPreferences("st50-sender-test", 0)

    // Scenario: S-01 を更新すると手元に残っていた記録がそのまま入る
    @Test
    fun `版 2 の記録を残している間に S-01 が版 2 に更新されると、溜めた順にそのまま送られて取り除かれる`() {
        val v = versioned()
        v.version = 2
        listOf("a", "b", "c").forEach { v.outbox.add(req(it)) }
        val transport = FakeTransport(acceptAll())
        val s = sender(v.outbox, transport)

        s.flush(ContractAnswer.Version(1))
        assertTrue("版 1 の S-01 へは送らない", transport.posts.isEmpty())

        val flushed = s.flush(ContractAnswer.Version(2))

        assertEquals(listOf("a", "b", "c"), transport.posts.single().ids)
        assertEquals(2, transport.posts.single().header)
        assertEquals(3, flushed.removed)
        assertEquals(0, v.outbox.size())
    }

    // Scenario: S-01 を更新すると、止まっていた間に飛ばす印の付いた記録も先に送られる
    @Test
    fun `unknown_field で飛ばす印の付いた先頭の記録は、止まった状態を抜けた契機の後に飛ばされずに送られる`() {
        val v = versioned()
        v.outbox.add(req("x"))
        v.outbox.add(req("y"))
        var verdict: (List<String>) -> Outcome = { ids ->
            // x だけ unknown_field で断る
            val results = ids.joinToString(",", "[", "]") { if (it == "x") """{"accepted":false,"error":"unknown_field"}""" else """{"accepted":true}""" }
            Outcome.Responded(200, results)
        }
        val transport = FakeTransport { verdict(it) }
        var probe: Outcome = Outcome.Responded(200, """{"contract_version":1}""")
        val gate = ContractGate(ContractWaitStore(prefs()))
        val drainer = Drainer(
            records = sender(v.outbox, transport, currentVersion = 1),
            recordsOutbox = v.outbox,
            beats = Sender(st.beats, FakeTransport(acceptAll()), HeartbeatRequest.serializer()),
            drops = Sender(st.drops, FakeTransport(acceptAll()), DropReport.serializer()),
            ledger = st.ledger,
            contract = ContractGuard({ probe }, gate),
            maxRounds = 1,
        )

        drainer.tick()
        assertTrue("1 件が unknown_field で断られた契機は止まった状態", gate.waiting)
        assertTrue(gate.reasons.rejected)
        assertEquals(listOf("x"), v.outbox.snapshot().map { it.id })

        // S-01 が更新された。x が受け付けられるようになり、同じ契機の中では x は飛ばす印のせいで後回しにされうる
        v.outbox.add(req("z"))
        verdict = acceptAll()
        drainer.tick()
        assertFalse("受け付けられた契機で止まった状態を抜ける", gate.waiting)

        // 次の契機: 飛ばす印が捨てられているので x が先頭から送られる
        v.outbox.add(req("w"))
        drainer.tick()
        val last = transport.posts.last()
        assertEquals("x", last.ids.first())
        assertEquals(0, v.outbox.size())
    }

    // Scenario: 原本は記録を積んだ時の版で止まる
    @Test
    fun `版 2 の記録の原本は問い合わせの口が版 1 のあいだ送られず、欠けにもならない`() {
        val h = OriginalHarness(st)
        h.photo(1, "photo-v2")
        h.scan()
        h.acceptRecords()
        val sha = h.shaOf(1)
        // 記録を積んだ時の版が 2 の送り待ちに差し替える（積んだ後に定数を差し替えられる形）
        val v2 = OriginalLedger(File(tmp.newFolder("o2"), ORIGINALS_FILE), contractVersion = { 2 })
        v2.queue(h.env.originals.get(sha)!!.copy(state = ORIGINAL_READY))
        v2.markReady(listOf(sha))
        assertEquals(2, v2.get(sha)!!.contractVersion)
        val uploader = OriginalUploader(
            v2,
            h.source,
            h.transport,
            h.reporter,
            OriginalEnvironment(access = { MediaAccess.FULL }, unmetered = { true }, now = { h.now() }),
        )

        val sent = uploader.run(ContractAnswer.Version(1))

        assertEquals(0, sent)
        assertTrue("版 > A の原本は問い合わせも送りもしない", h.transport.heads.isEmpty() && h.transport.puts.isEmpty())
        assertEquals("送り待ちに残る", listOf(sha), v2.ready().map { it.sha256 })
        assertTrue("欠けにならない", h.missingReports().isEmpty())
        assertTrue(uploader.lastContract.ahead)

        // 426 でも欠けにならず、record_missing の 7 日の数えも始まらない
        h.transport.putStatus = 426
        h.transport.putBody = """{"error":"contract_unsupported","contract_version":1}"""
        val refusedUp = OriginalLedger(File(tmp.newFolder("o1"), ORIGINALS_FILE))
        refusedUp.queue(h.env.originals.get(sha)!!)
        refusedUp.markReady(listOf(sha))
        val u1 = OriginalUploader(
            refusedUp,
            h.source,
            h.transport,
            h.reporter,
            OriginalEnvironment(access = { MediaAccess.FULL }, unmetered = { true }, now = { h.now() }),
        )
        assertEquals(0, u1.run(ContractAnswer.Version(1)))
        assertEquals(listOf(sha), refusedUp.ready().map { it.sha256 })
        assertNull(refusedUp.get(sha)!!.missingSince)
        assertTrue(h.missingReports().isEmpty())
        assertTrue(u1.lastContract.refused)
    }

    @Test
    fun `原本の HEAD と PUT は行の版を見出しに載せる`() {
        val h = OriginalHarness(st)
        h.photo(1, "photo-header")
        h.scan()
        h.acceptRecords()
        val headers = mutableListOf<String>()
        val inner = h.transport
        val spy = object : OriginalTransport {
            override fun head(sha256: String) = inner.head(sha256)

            override fun put(sha256: String, length: Long, body: java.io.InputStream) = inner.put(sha256, length, body)

            override fun head(sha256: String, contractVersion: Int): Outcome {
                headers += "HEAD:$contractVersion"
                return inner.head(sha256)
            }

            override fun put(sha256: String, length: Long, body: java.io.InputStream, contractVersion: Int): Outcome {
                headers += "PUT:$contractVersion"
                return inner.put(sha256, length, body)
            }
        }
        val uploader = OriginalUploader(
            h.env.originals,
            h.source,
            spy,
            h.reporter,
            OriginalEnvironment(access = { MediaAccess.FULL }, unmetered = { true }, now = { h.now() }),
        )

        assertEquals(1, uploader.run(ContractAnswer.Version(1)))

        assertEquals(listOf("HEAD:1", "PUT:1"), headers)
    }

    @Test
    fun `200 の読めない答えが 24 時間続くと更新を待つ状態に入り、数え始めは再起動をまたぎ、読める答えで数え直す`() {
        val t0 = Instant.parse("2026-10-30T00:00:00Z")
        val gate = ContractGate(ContractWaitStore(prefs()))

        gate.observe(ContractAnswer.Unreadable, t0, emptyList())
        assertFalse("24 時間に満たない間は待つ状態にしない", gate.waiting)
        gate.observe(ContractAnswer.Transient, t0.plusSeconds(3600), emptyList())

        // 収集アプリを起こし直した（同じ置き場から作り直す）
        val restarted = ContractGate(ContractWaitStore(prefs()))
        restarted.observe(ContractAnswer.Unreadable, t0.plusSeconds(24 * 3600), emptyList())
        assertTrue(restarted.waiting)
        assertTrue(restarted.reasons.onlyUnreadable)

        val left = restarted.observe(ContractAnswer.Version(1), t0.plusSeconds(25 * 3600), listOf(ContractOutcome(posted = true)))
        assertTrue("読める答えで抜ける", left)
        assertNull(ContractWaitStore(prefs()).unreadableSinceMs)
        assertFalse(restarted.waiting)
    }

    @Test
    fun `S-01 の更新を待つ理由は a から d の 1 つの値で持つ`() {
        val gate = ContractGate(ContractWaitStore(prefs()))
        val at = Instant.parse("2026-10-30T00:00:00Z")

        gate.observe(ContractAnswer.NoEndpoint, at, emptyList())
        assertTrue(gate.reasons.noEndpoint)
        gate.observe(ContractAnswer.Version(1), at, listOf(ContractOutcome(ahead = true)))
        assertTrue(gate.reasons.ahead)
        assertFalse(gate.reasons.noEndpoint)
        gate.observe(ContractAnswer.Version(1), at, listOf(ContractOutcome(posted = true, refused = true)))
        assertTrue(gate.reasons.refused)
        gate.observe(ContractAnswer.Version(1), at, listOf(ContractOutcome(posted = true, rejectedByVersion = true)))
        assertTrue(gate.reasons.rejected)
        assertFalse(gate.reasons.refused)
        assertTrue(gate.observe(ContractAnswer.Version(1), at, listOf(ContractOutcome(posted = true))))
        assertFalse(gate.waiting)
    }
}
