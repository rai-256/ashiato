// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 保持の上限（ST04 / tasks 6.2 / 深掘り Q1 / Q4 / C1 / C2 / C4 / design D2 / D3 / D13）。
 */
class RetentionTest {
    private val st = TestStores(segmentBytes = 5_000)
    private val day = AgeClock.DAY_MS

    private fun req(id: String, at: String = "2026-06-01T01:00:00Z") =
        LocationFix(35.68, 139.76, 10f, Instant.parse(at))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    private fun retention(policy: RetentionPolicy = RetentionPolicy.DEFAULT) =
        Retention(st.records, st.ledger, st.age::now, policy = { policy })

    private fun beat(id: String) =
        HeartbeatRequest(id, "user-1", LOGICAL_SOURCE, "device-1", "2026-05-01T00:00:00Z", true, emptyList(), 1, 1, "{}")

    @Test
    fun `本番の上限は 90 日と 2 GB と 83 日`() {
        assertEquals(RetentionPolicy.DEFAULT, RetentionPolicy.current)
        assertEquals(90 * day, RetentionPolicy.DEFAULT.maxAgeMs)
        assertEquals(2L * 1024 * 1024 * 1024, RetentionPolicy.DEFAULT.maxBytes)
        assertEquals(83 * day, RetentionPolicy.DEFAULT.alertAgeMs)
    }

    // Scenario: 端末に積んでから 90 日を超えた記録は捨てられる
    @Test
    fun `積んでから 90 日と 1 分の記録は捨てられ、報告に数えられる`() {
        st.records.add(req("old"))
        st.clock.advance(90 * day + 60_000)
        st.records.add(req("new", "2026-09-01T00:00:00Z"))

        assertEquals(1, retention().enforce())
        assertEquals(listOf("new"), st.records.snapshot().map { it.id })
        val d = st.ledger.drafts().single()
        assertEquals("age", d.reason)
        assertEquals(1, d.count)
    }

    // Scenario: 90 日に届かない記録は捨てられない
    @Test
    fun `積んでから 89 日の記録は捨てられない`() {
        st.records.add(req("a"))
        st.records.add(req("b"))
        st.clock.advance(89 * day)
        assertEquals(0, retention().enforce())
        assertEquals(2, st.records.size())
        // ちょうど 90 日も「超えた」ではない
        st.clock.advance(1 * day)
        assertEquals(0, retention().enforce())
    }

    // Scenario: 2 GB を超えると積んだ順の古いものから捨てられる
    @Test
    fun `上限のバイトを超えると積んだ順の古いものから捨て、上限以下になる`() {
        // 出来事の時刻は積んだ順と逆にする —— **積んだ順で捨てる**ことを見分けるため
        val ids = (0 until 40).map { "id-%02d".format(it) }
        ids.forEachIndexed { i, id -> st.records.add(req(id, Instant.parse("2026-06-02T00:00:00Z").minusSeconds(60L * i).toString())) }
        val limit = st.records.bytes() / 2

        val dropped = retention(RetentionPolicy(maxBytes = limit)).enforce()

        assertTrue(dropped > 0)
        assertTrue("上限以下になっていない", st.records.bytes() <= limit)
        val kept = st.records.snapshot().map { it.id }
        assertEquals("積んだ順の古いものから捨てていない", ids.drop(dropped), kept)
        // 出来事の時刻が戻るたびに新しい報告になる（spec）ので、報告の件数の合計で見る
        st.ledger.freeze()
        val reports = st.drops.snapshot()
        assertTrue(reports.all { it.reason == "bytes" })
        assertEquals(dropped, reports.sumOf { it.count })
    }

    // Scenario: 到達できても断られ続ける未送信にも上限がかかる
    @Test
    fun `登録簿に無いソースとして断られ続ける記録も 90 日を超えたら捨てる`() {
        st.records.add(req("rejected"))
        val transport = Transport { Outcome.Responded(200, """[{"accepted":false,"error":"unknown_source"}]""") }
        val sender = Sender(st.records, transport, IngestRequest.serializer(), dropPermanentlyRejected = true)
        repeat(3) {
            sender.flush()
            st.clock.advance(30 * day + 1)
        }
        assertEquals("送信の結果で捨てている（前提が崩れている）", 1, st.records.size())
        assertEquals(1, retention().enforce())
        assertEquals(0, st.records.size())
        assertEquals(1, st.ledger.drafts().single().count)
    }

    // Scenario: 出来事の時刻が古くても積んだばかりの記録は捨てられない
    @Test
    fun `出来事の時刻が 3 年前でも積んだのが 1 時間前なら捨てない`() {
        st.clock.advance(1_000 * day)
        st.records.add(req("photo", "2023-09-14T00:00:00Z"))
        st.clock.advance(60 * 60 * 1000L)
        assertEquals(0, retention().enforce())
        assertEquals(1, st.records.size())
    }

    // Scenario: 生存信号は上限を超えても捨てられない
    @Test
    fun `生存信号は 2 GB を超えて 90 日を過ぎても捨てない`() {
        st.beats.add(beat("h1"))
        repeat(40) { st.records.add(req("r$it")) }
        st.clock.advance(120 * day)
        retention(RetentionPolicy(maxBytes = 1_000)).enforce()
        assertEquals(listOf("h1"), st.beats.snapshot().map { it.id })
    }

    // Scenario: 破棄の報告は上限を超えても捨てられない
    @Test
    fun `破棄の報告は 2 GB を超えて 90 日を過ぎても捨てない`() {
        st.records.add(req("r0"))
        st.clock.advance(91 * day)
        retention().enforce()
        st.ledger.freeze()
        val report = st.drops.snapshot().single()
        repeat(40) { st.records.add(req("r${it + 1}")) }
        st.clock.advance(120 * day)
        retention(RetentionPolicy(maxBytes = 1_000)).enforce()
        assertTrue(st.drops.snapshot().any { it.id == report.id })
    }

    @Test
    fun `上限の見回りは積む契機に相乗りする`() {
        // 本番は `Outbox` の `afterAdd` から呼ぶ（新しい契機を起こさない）
        var ran = 0
        val outbox = Outbox(
            SegmentStore(File(st.dir, "hook"), IngestRequest.serializer(), st.unreadable, st.log),
            st.age::now,
            afterAdd = { ran++ },
        )
        outbox.add(req("a"))
        assertEquals(1, ran)
    }

    /**
     * **下書きを保存できなければ、記録を置き場から消さない**（review R26）。
     * 消してから保存していたときは、保存に失敗して立て直すと、捨てた記録について何も残らなかった。
     */
    @Test
    fun `破棄の報告の下書きを保存できなければ記録を捨てない`() {
        val blocked = File(st.dir, "no-space").apply { writeText("x") }
        val ledger = DropLedger(File(blocked, "drops-open.json"), st.drops, { "user-1" }, "device-1", { st.now }, { "n" }, st.log)
        repeat(3) { st.records.add(req("r$it")) }
        st.clock.advance(91 * day)
        assertEquals(0, Retention(st.records, ledger, st.age::now).enforce())
        assertEquals("証拠を書けないのに捨てた", 3, st.records.size())
        assertTrue(ledger.drafts().isEmpty())
    }
}
