// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 捨てた記録から破棄の報告を作る（ST04 / tasks 7.1 / 深掘り C3 / C8 / C12 / R2 / R4 / spec-review R8 / design D4）。
 */
class DropReportTest {
    private val st = TestStores()
    private val day = AgeClock.DAY_MS

    private fun at(s: String): Instant = Instant.parse(s)

    private fun req(id: String, time: String) =
        LocationFix(35.681236, 139.767125, 10f, at(time)).toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    private fun report(): DropReport {
        st.ledger.freeze()
        return st.drops.snapshot().last()
    }

    // Scenario: 上限で捨てると範囲と件数と理由の報告が積まれる
    @Test
    fun `90 日を超えた位置の記録 180 件を捨てると、端末・理由・範囲・件数 180 の報告が積まれる`() {
        repeat(180) { st.records.add(req("r$it", at("2026-06-01T01:00:00Z").plusSeconds(60L * it).toString())) }
        st.clock.advance(91 * day)
        assertEquals(180, Retention(st.records, st.ledger, st.age::now).enforce())

        val r = report()
        assertEquals(LOGICAL_SOURCE, r.logicalSource)
        assertEquals("device-1", r.deviceId)
        assertEquals("user-1", r.userId)
        assertEquals("age", r.reason)
        assertEquals(180, r.count)
        assertEquals("2026-06-01T01:00:00Z", r.rangeStart)
        // 残った記録が無いので、最後に捨てた記録（03:59）の直後
        assertEquals("2026-06-01T03:59:00.001Z", r.rangeEnd)
    }

    // Scenario: 件数は出来事の時刻の 1 時間ごとに数えられる
    @Test
    fun `10 時台 50 件と 11 時台 30 件は時間ごとに分かれる`() {
        repeat(50) { st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z").plusSeconds(60L * it)) }
        repeat(30) { st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T11:00:00Z").plusSeconds(60L * it)) }
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)
        val r = report()
        assertEquals(
            listOf(DropHour("2026-06-01T10:00:00Z", 50), DropHour("2026-06-01T11:00:00Z", 30)),
            r.hourly,
        )
        assertEquals(80, r.count)
    }

    // Scenario: 1 件だけ捨てても範囲の終わりは始まりより後
    @Test
    fun `1 件だけ捨てても範囲の終わりは始まりより後`() {
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:23:45Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)
        val r = report()
        assertTrue(at(r.rangeEnd!!).isAfter(at(r.rangeStart!!)))
        assertEquals("2026-06-01T10:23:45.001Z", r.rangeEnd)
    }

    // Scenario: 続けて捨てた範囲は残った記録の時刻で途切れない
    @Test
    fun `送ろうとした後に次に古い記録を捨てると、2 本目の始まりは 1 本目の終わりと同じ`() {
        repeat(3) { st.records.add(req("r$it", at("2026-06-01T10:00:00Z").plusSeconds(60L * it).toString())) }
        st.clock.advance(1 * day)
        st.records.add(req("r3", "2026-06-01T10:03:00Z"))
        st.clock.advance(89 * day + 1)
        // 90 日を超えたのは先の 3 件だけ。残った最も古い記録は 10:03
        Retention(st.records, st.ledger, st.age::now).enforce()
        val first = report()
        assertEquals("2026-06-01T10:03:00Z", first.rangeEnd)

        st.clock.advance(1 * day)
        Retention(st.records, st.ledger, st.age::now).enforce()
        val second = report()
        assertEquals(first.rangeEnd, second.rangeStart)
    }

    // Scenario: 送る前の報告には続けて起きた破棄が足される
    @Test
    fun `送る前の報告は 1 本のまま件数と範囲が伸びる`() {
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:30:00Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)
        val d = st.ledger.drafts().single()
        assertEquals(2, d.count)
        assertEquals(at("2026-06-01T10:30:00.001Z").toEpochMilli(), d.endMs)
        assertTrue("まだ送ろうとしていないのに積まれている", st.drops.snapshot().isEmpty())
    }

    // Scenario: 破棄の報告に位置の値が含まれない
    @Test
    fun `報告に緯度・経度・原文の中身が含まれない`() {
        st.records.add(req("r0", "2026-06-01T10:00:00Z"))
        st.clock.advance(91 * day)
        Retention(st.records, st.ledger, st.age::now).enforce()
        val r = report()
        val body = ingestJson.encodeToString(DropReport.serializer(), r)
        for (secret in listOf("35.681236", "139.767125", "lat", "lon", "acc_m", req("r0", "2026-06-01T10:00:00Z").raw)) {
            assertFalse("報告に $secret が入っている", body.contains(secret))
        }
    }

    // Scenario: 理由が違う破棄は別の報告になる
    @Test
    fun `90 日と 2 GB で続けて捨てると理由ごとに 2 本`() {
        repeat(30) { st.records.add(req("r$it", at("2026-06-01T10:00:00Z").plusSeconds(60L * it).toString())) }
        st.clock.advance(91 * day)
        repeat(30) { st.records.add(req("n$it", at("2026-09-01T10:00:00Z").plusSeconds(60L * it).toString())) }
        Retention(st.records, st.ledger, st.age::now, policy = { RetentionPolicy(maxBytes = st.records.bytes() / 4) }).enforce()
        st.ledger.freeze()
        assertEquals(setOf("age", "bytes"), st.drops.snapshot().map { it.reason }.toSet())
        assertEquals(2, st.drops.snapshot().size)
    }

    // Scenario: 出来事の時刻が離れた破棄は別の報告になる
    @Test
    fun `送る前の報告の終わりから 2 時間離れた破棄は新しい報告で、元の範囲は伸びない`() {
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)
        val before = st.ledger.drafts().single()
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T12:00:00.001Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)

        // 元の報告は閉じて（もう伸ばさない）、新しい下書きが開く
        val (old, fresh) = st.ledger.drafts().partition { it.closed }.let { it.first.single() to it.second.single() }
        assertEquals("元の報告の範囲が伸びている", before.endMs, old.endMs)
        assertEquals(1, old.count)
        assertEquals(at("2026-06-01T12:00:00.001Z").toEpochMilli(), fresh.startMs)
        // 範囲の始まりより前に戻っても新しい報告
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T11:59:00Z"))
        assertEquals(2, st.ledger.drafts().count { it.closed })
        st.ledger.freeze()
        assertEquals(3, st.drops.snapshot().size)
        assertEquals(Instant.ofEpochMilli(before.endMs!!).toString(), st.drops.snapshot().first().rangeEnd)
    }

    // Scenario: 残った記録が離れていれば範囲の終わりは最後に捨てた記録の直後
    @Test
    fun `10 時から 12 時 59 分を捨てて残りが 15 時なら終わりは 12 時 59 分 1 ミリ秒`() {
        var t = at("2026-06-01T10:00:00Z")
        while (!t.isAfter(at("2026-06-01T12:59:00Z"))) {
            st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, t)
            t = t.plusSeconds(60)
        }
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T15:00:00Z"))
        assertEquals("2026-06-01T12:59:00.001Z", report().rangeEnd)
    }

    @Test
    fun `残った記録が 1 時間以内なら範囲の終わりはその時刻`() {
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:59:00Z"))
        assertEquals("2026-06-01T10:59:00Z", report().rangeEnd)
    }

    // Scenario: 時間ごとの件数は範囲の内側にある
    @Test
    fun `時間ごとの件数が置かれた時間はどれも範囲と重なる`() {
        val times = listOf("2026-06-01T09:59:59Z", "2026-06-01T10:00:00Z", "2026-06-01T10:45:00Z", "2026-06-01T11:30:00Z")
        for (s in times) st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at(s))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T12:10:00Z"))
        val r = report()
        val start = at(r.rangeStart!!)
        val end = at(r.rangeEnd!!)
        for (h in r.hourly) {
            val hs = at(h.hour)
            assertTrue("${h.hour} が範囲 [${r.rangeStart}, ${r.rangeEnd}) と重ならない", hs.isBefore(end) && hs.plusSeconds(3600).isAfter(start))
            assertEquals("正時でない", 0, hs.epochSecond % 3600)
        }
        assertEquals(r.count, r.hourly.sumOf { it.count })
    }

    @Test
    fun `原文は欄を組んだ JSON の文字列そのもので、同じ下書きからは毎回同じ`() {
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z"))
        val d = st.ledger.drafts().single()
        val a = d.toReport("user-1", "device-1")
        val b = d.toReport("user-1", "device-1")
        assertEquals(a.raw, b.raw)
        val raw = Json.parseToJsonElement(a.raw).jsonObject
        assertEquals(setOf("id", "user_id", "logical_source", "device_id", "reason", "created_at", "range_start", "range_end", "count", "hourly"), raw.keys)
    }

    /** 「1 時間以内」の境界（review R9）: ちょうど 1 時間ならその時刻、1 時間 + 1 ms なら最後の直後。 */
    @Test
    fun `残った記録がちょうど 1 時間後ならその時刻、1 ミリ秒でも越えれば最後の直後`() {
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T11:00:00Z"))
        assertEquals("2026-06-01T11:00:00Z", report().rangeEnd)
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.BYTES, at("2026-06-01T10:00:00Z"))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.BYTES, at("2026-06-01T11:00:00.001Z"))
        assertEquals("2026-06-01T10:00:00.001Z", report().rangeEnd)
    }

    /** 「1 時間を超えて離れたら別の報告」の境界（review R9）: ちょうど 1 時間なら伸ばし、1 ms 越えれば分ける。 */
    @Test
    fun `範囲の終わりからちょうど 1 時間なら伸ばし、1 ミリ秒越えれば別の報告`() {
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z"))
        // 範囲の終わりは 10:00:00.001。ちょうど 1 時間後
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T11:00:00.001Z"))
        assertEquals(1, st.ledger.drafts().size)
        assertEquals(2, st.ledger.drafts().single().count)
        // 範囲の終わりは 11:00:00.002。1 時間 + 1 ms 後
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T12:00:00.003Z"))
        assertEquals(2, st.ledger.drafts().size)
    }

    /** 利用者識別子が決まっていなければ凍結しない（review R20。空のまま凍結した報告は受け手に断られ続け、書き換えられない）。 */
    @Test
    fun `利用者識別子が空のうちは凍結せず、決まってから凍結する`() {
        var user = ""
        val ledger = DropLedger(java.io.File(st.dir, "blank-user.json"), st.drops, { user }, "device-1", { st.now }, { "b" }, st.log)
        ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z"))
        ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T13:00:00Z")) // 離れて閉じた 1 本も含む
        assertFalse(ledger.freeze())
        assertEquals(0, st.drops.size())
        assertEquals(2, ledger.drafts().size)
        user = "user-1"
        assertTrue(ledger.freeze())
        assertTrue(st.drops.snapshot().all { it.userId == "user-1" })
        assertEquals(2, st.drops.size())
    }

    /** 積めなかった下書きは消さない（review R16）。メモリの報告は捨て、次の凍結で同じ原文を積み直す。 */
    @Test
    fun `凍結で積めなかった下書きは残り、次の凍結で同じ原文を積む`() {
        val blocked = java.io.File(st.dir, "blocked-drops").apply { writeText("x") }
        val failing = Outbox(SegmentStore(java.io.File(blocked, "drops"), DropReport.serializer(), st.unreadable, st.log), st.age::now)
        val openFile = java.io.File(st.dir, "retry.json")
        val ledger = DropLedger(openFile, failing, { "user-1" }, "device-1", { st.now }, { "k" }, st.log)
        ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:00:00Z"))
        ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)
        assertFalse(ledger.freeze())
        assertEquals("積めなかった報告がメモリに残っている（下書きと二重になる）", 0, failing.size())
        val kept = ledger.drafts().single()
        assertTrue(kept.closed)
        // 閉じた下書きは伸ばさない
        ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, at("2026-06-01T10:01:00Z"))
        assertEquals(kept, ledger.drafts().first())
        // 立て直しても同じ下書きが戻り、積める置き場へ凍結すると同じ原文になる
        val reborn = DropLedger(openFile, st.drops, { "user-1" }, "device-1", { st.now }, { "z" }, st.log)
        assertTrue(reborn.drafts().contains(kept))
        reborn.freeze()
        assertTrue(st.drops.snapshot().any { it.raw == kept.toReport("user-1", "device-1").raw })
    }

    /** 読めない下書きのファイルは退避して、読めなかったこと 1 件を数える（review R10）。 */
    @Test
    fun `読めない下書きのファイルは上書きせずに退避する`() {
        val openFile = java.io.File(st.dir, "broken-open.json").apply { writeText("[{\"id\":\"x\",\"count\":180") }
        val ledger = DropLedger(openFile, st.drops, { "user-1" }, "device-1", { st.now }, { "u" }, st.log)
        val aside = st.dir.listFiles { f -> f.name.startsWith("broken-open.json.unreadable.") }.orEmpty()
        assertEquals(1, aside.size)
        assertEquals("[{\"id\":\"x\",\"count\":180", aside.single().readText())
        val d = ledger.drafts().single()
        assertEquals("unreadable", d.reason)
        assertEquals(1, d.count)
    }
}
