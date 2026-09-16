// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 溜まっている間は続けて送る（ST04 / tasks 8.1 / 深掘り Q6 / spec R5 / design D12）。
 *
 * ST01 の D9（5 分ごと・1 回 200 件）だと、90 日ぶん（位置 129,600 件）を送り切るまで約 55 時間かかる。
 * 1 回の `tick()` が「一定の間隔の契機 1 回」にあたる。
 */
class DrainTest {
    private val st = TestStores()
    private val calls = mutableListOf<String>()

    private fun req(i: Int) =
        LocationFix(35.68, 139.76, 10f, Instant.parse("2026-06-01T00:00:00Z").plusSeconds(60L * i))
            .toIngestRequest("id-$i", "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 載っている件数ぶん同じ結果を返す偽の受け口。`reply` は何回目か（1 始まり）で答えを変えられる。 */
    private fun transport(path: String, reply: (Int, Int) -> Outcome = { _, n -> ok(n) }): Transport {
        var nth = 0
        return Transport { body ->
            nth++
            calls += path
            reply(nth, body.split("\"id\":").size - 1)
        }
    }

    private fun ok(n: Int) = Outcome.Responded(200, (1..n).joinToString(",", "[", "]") { """{"accepted":true}""" })

    private fun rejected(n: Int) =
        Outcome.Responded(200, (1..n).joinToString(",", "[", "]") { """{"accepted":false,"error":"unknown_source"}""" })

    private fun drainer(ingest: Transport = transport("ingest")) = Drainer(
        records = Sender(st.records, ingest, IngestRequest.serializer(), dropPermanentlyRejected = true),
        recordsOutbox = st.records,
        beats = Sender(st.beats, transport("heartbeat"), HeartbeatRequest.serializer()),
        drops = Sender(st.drops, transport("drops"), DropReport.serializer()),
        ledger = st.ledger,
        // 続ける条件が壊れたときに試験が止まらなくならないよう小さく抑える（本番は 10 万回）
        maxRounds = 10,
    )

    private fun fill(n: Int) = repeat(n) { st.records.add(req(it)) }

    // Scenario: 溜まっている間は間隔を待たずに続けて送る
    @Test
    fun `3 倍溜まっていたら 1 回目の直後に 2 回目を送る`() {
        fill(3 * MAX_BATCH)
        drainer().tick()
        assertEquals("1 回の契機で続けて送っていない", 2, calls.count { it == "ingest" })
        assertEquals(MAX_BATCH, st.records.size())
    }

    // Scenario: 溜まりが 1 回に載る件数以下になったら一定の間隔に戻る
    @Test
    fun `2 回目の後に 1 回ぶんが残ったら 3 回目は次の契機`() {
        fill(3 * MAX_BATCH)
        val d = drainer()
        d.tick()
        assertEquals(2, calls.count { it == "ingest" })
        d.tick()
        assertEquals("3 回目が次の契機で行われていない", 3, calls.count { it == "ingest" })
        assertEquals(0, st.records.size())
    }

    // Scenario: 断られ続ける未送信だけが溜まっていても一定の間隔に戻る
    @Test
    fun `1 件も取り除けなかったら続けない`() {
        fill(3 * MAX_BATCH)
        drainer(transport("ingest") { _, n -> rejected(n) }).tick()
        assertEquals("断られ続ける分で続けて送っている", 1, calls.count { it == "ingest" })
        assertEquals(3 * MAX_BATCH, st.records.size())
    }

    // Scenario: 記録が溜まっていても生存信号と破棄の報告が送られる
    @Test
    fun `1 回目の記録の送信と同じ契機で生存信号と破棄の報告も送る`() {
        fill(3 * MAX_BATCH)
        st.beats.add(HeartbeatRequest("h1", "user-1", LOGICAL_SOURCE, "device-1", "2026-06-01T00:00:00Z", true, emptyList(), 1, 1, "{}"))
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, Instant.parse("2026-03-01T00:00:00Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)

        drainer().tick()

        val secondIngest = calls.withIndex().filter { it.value == "ingest" }[1].index
        assertTrue("生存信号が記録の溜まりを待たされている", calls.indexOf("heartbeat") in 0 until secondIngest)
        assertTrue("破棄の報告が記録の溜まりを待たされている", calls.indexOf("drops") in 0 until secondIngest)
        assertEquals(0, st.beats.size())
        assertEquals(0, st.drops.size())
    }

    // Scenario: 続けて送る途中で失敗したら一定の間隔に戻る
    @Test
    fun `続けて送る途中で到達できなくなったらやめる`() {
        fill(4 * MAX_BATCH)
        drainer(transport("ingest") { nth, n -> if (nth == 1) ok(n) else Outcome.Unreachable("timeout") }).tick()
        assertEquals(2, calls.count { it == "ingest" })
        assertEquals(3 * MAX_BATCH, st.records.size())
    }

    // Scenario: 溜まっていないときは一定の間隔のまま
    @Test
    fun `1 回に載る件数より少なければ 1 回だけ送る`() {
        fill(MAX_BATCH - 1)
        drainer().tick()
        assertEquals(1, calls.count { it == "ingest" })
        assertEquals(0, st.records.size())
    }

    @Test
    fun `上限の見回りは送る前に呼ぶ（送れない理由を問わず上限をかける）`() {
        fill(3)
        val order = mutableListOf<String>()
        Drainer(
            records = Sender(st.records, Transport { order += "ingest"; ok(3) }, IngestRequest.serializer()),
            recordsOutbox = st.records,
            beats = Sender(st.beats, transport("heartbeat"), HeartbeatRequest.serializer()),
            drops = Sender(st.drops, transport("drops"), DropReport.serializer()),
            ledger = st.ledger,
            maintenance = { order += "maintenance" },
        ).tick()
        assertEquals(listOf("maintenance", "ingest"), order)
    }

    /**
     * 受け付けられても置き場から取り除けなかったら（`.acked` を書けない）、**続けて送らない**（review R5）。
     * 取り除けたと数えていたときは、同じ 200 件を 1 回の契機の中で送り続けた。
     */
    @Test
    fun `取り除きを書けなかったら続けて送らない`() {
        fill(3 * MAX_BATCH)
        // 取り除きの印（`.acked`）を作れなくする。読むほうは通る
        assertTrue(st.recordsDir.setWritable(false))
        try {
            drainer().tick()
        } finally {
            st.recordsDir.setWritable(true)
        }
        assertEquals("取り除けていないのに続けて送った", 1, calls.count { it == "ingest" })
        assertEquals(3 * MAX_BATCH, st.records.size())
    }
}
