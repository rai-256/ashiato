// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.DateTimeException
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** 端末の基準を読む口（ST05 / design D2）。OS の時計は偽物に差し替える。 */
class ClockReferencesTest {
    /** 呼ばれた順を残す偽の OS の時計。`wall` は `wallMs()` を読むたびに `wallStep` 進める。 */
    private class FakeSources(
        override var sdkInt: Int = 36,
        var network: () -> Long = { throw DateTimeException("none") },
        var gnss: () -> Long = { throw DateTimeException("none") },
        var wall: Long = 1_000_000,
        var mono: Long = 500,
    ) : SystemTimeSources {
        val calls = mutableListOf<String>()

        override fun networkMs(): Long = network().also { calls += "network" }

        override fun gnssMs(): Long = gnss().also { calls += "gnss" }

        override fun wallMs(): Long = wall.also { calls += "wall" }

        override fun monoMs(): Long = mono.also { calls += "mono" }
    }

    private fun byName(list: List<ClockReading>) = list.associateBy { it.source }

    @Test
    fun `取れた基準は出どころ・時刻・差・前後の単調時計を返す`() {
        val s = FakeSources(network = { 990_000 }, gnss = { 1_000_500 })

        val r = byName(ClockReferences(s, ResponseDateCache()).readAll())

        val net = r.getValue("network")
        assertEquals(990_000L, net.timeMs)
        assertEquals(10_000L, net.skewMs)
        assertNull(net.reason)
        assertEquals(500L, net.monoBeforeMs)
        assertEquals(500L, net.monoAfterMs)
        assertEquals(-500L, r.getValue("gnss").skewMs)
    }

    // Scenario: 差に使う端末の時計は基準を読む前後の間で読む
    @Test
    fun `壁時計は基準を読む直前と直後の単調時計の間で読む`() {
        val s = FakeSources(network = { 990_000 })

        ClockReferences(s, ResponseDateCache()).readAll()

        // network の読みは mono → network → wall → mono の順（gnss の読みは別の並び）
        assertEquals(listOf("mono", "network", "wall", "mono"), s.calls.take(4))
    }

    // Scenario: 差に使う端末の時計は基準を読む前後の間で読む
    @Test
    fun `測定を始めてから基準を読むまでに時計が 30 秒進んでも差に混ざらない`() {
        // 実際には時間が 30 秒たち、壁時計も基準の時刻も一緒に 30 秒進んだ。差は変わらない（10 秒）。
        val s = FakeSources(wall = 1_000_000, network = { 990_000 })
        val refs = ClockReferences(s, ResponseDateCache())
        s.wall += 30_000
        s.network = { 990_000 + 30_000 }

        val net = byName(refs.readAll()).getValue("network")

        assertEquals(10_000L, net.skewMs)
        assertNotEquals(40_000L, net.skewMs)
    }

    // Scenario: OS の版が対応していない基準はいま取れない基準と区別して残る
    @Test
    fun `OS の版がネットワーク時刻の口を持たなければ unsupported、いま取れないなら not_available`() {
        val old = byName(ClockReferences(FakeSources(sdkInt = 32), ResponseDateCache()).readAll())
        val now = byName(ClockReferences(FakeSources(sdkInt = 36), ResponseDateCache()).readAll())

        assertEquals("unsupported", old.getValue("network").reason)
        assertEquals("not_available", now.getValue("network").reason)
        assertNull(old.getValue("network").skewMs)
    }

    @Test
    fun `例外は外へ出さず error 型名で残す`() {
        val s = FakeSources(network = { throw IllegalStateException("x") }, gnss = { throw SecurityException("y") })

        val r = byName(ClockReferences(s, ResponseDateCache()).readAll())

        assertEquals("error:IllegalStateException", r.getValue("network").reason)
        assertEquals("error:SecurityException", r.getValue("gnss").reason)
    }

    @Test
    fun `応答の日付は s01-date として並び、応答を読み終えた直後の壁時計との差になる`() {
        val cache = ResponseDateCache()
        // 2026-09-29T03:04:05Z
        val t = Instant.parse("2026-09-29T03:04:05Z").toEpochMilli()
        cache.put(ResponseDateCache.Received("Tue, 29 Sep 2026 03:04:05 GMT", 100, 180, t + 700))

        val r = byName(ClockReferences(FakeSources(), cache).readAll()).getValue("s01-date")

        assertEquals(t, r.timeMs)
        assertEquals(700L, r.skewMs)
        assertEquals(100L, r.monoBeforeMs)
        assertEquals(180L, r.monoAfterMs)
    }

    @Test
    fun `読めない Date は unreadable、見出しが無くても unreadable`() {
        val cache = ResponseDateCache()
        cache.put(ResponseDateCache.Received("きのう", 1, 2, 3))
        assertEquals("unreadable", byName(ClockReferences(FakeSources(), cache).readAll()).getValue("s01-date").reason)

        cache.put(ResponseDateCache.Received(null, 1, 2, 3))
        assertEquals("unreadable", byName(ClockReferences(FakeSources(), cache).readAll()).getValue("s01-date").reason)
    }

    @Test
    fun `応答が無いときは前回の測定より後に無い、時計の変更のあとなら clock_changed_since`() {
        val cache = ResponseDateCache()
        val refs = ClockReferences(FakeSources(), cache)
        assertEquals("no_response_since_last", byName(refs.readAll()).getValue("s01-date").reason)

        cache.put(ResponseDateCache.Received("Tue, 29 Sep 2026 03:04:05 GMT", 1, 2, 3))
        cache.clockChanged()
        assertEquals("clock_changed_since", byName(refs.readAll()).getValue("s01-date").reason)
    }

    // Scenario: 測るために通信を起こさない
    @Test
    fun `測っても送信の要求の数は変わらない`() {
        class CountingTransport : Transport {
            var calls = 0
            override fun post(bodyJson: String): Outcome {
                calls++
                return Outcome.Responded(200, "[{\"accepted\":true}]", "Tue, 29 Sep 2026 03:04:05 GMT", 1, 2, 3)
            }
        }
        val transport = CountingTransport()
        val cache = ResponseDateCache()
        val outbox = testOutbox()
        outbox.add(
            LocationFix(35.681236, 139.767125, 10f, Instant.parse("2026-09-08T02:00:00Z"))
                .toIngestRequest("id-1", "user-1", "device-1", ZoneId.of("Asia/Tokyo")),
        )
        Sender(outbox, transport, IngestRequest.serializer(), responseDates = cache).flush()
        val before = transport.calls

        ClockReferences(FakeSources(network = { 990_000 }), cache).readAll()

        assertEquals(1, before)
        assertEquals(before, transport.calls)
    }
}
