// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.InetAddress
import java.net.ServerSocket
import kotlin.concurrent.thread
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

/**
 * 受け取った応答の `Date` の置き場（ST05 / design D2）。
 * `HttpTransport` の見出しの読み取りは `HttpTransportTest` と同じ形の偽のサーバ（生の `ServerSocket`）で見る。
 */
class ResponseDateCacheTest {
    private lateinit var server: ServerSocket

    /** 次の応答の `Date` 行（null なら見出しを付けない）。 */
    private var dateLine: String? = "Date: Tue, 29 Sep 2026 03:04:05 GMT"

    @Before
    fun start() {
        server = ServerSocket(0, 0, InetAddress.getLoopbackAddress())
        thread(isDaemon = true) {
            while (!server.isClosed) {
                val socket = runCatching { server.accept() }.getOrNull() ?: return@thread
                runCatching {
                    socket.use {
                        val reader = BufferedReader(InputStreamReader(it.getInputStream(), Charsets.UTF_8))
                        var length = 0
                        while (true) {
                            val h = reader.readLine()
                            if (h.isNullOrEmpty()) break
                            if (h.lowercase().startsWith("content-length:")) length = h.substringAfter(':').trim().toInt()
                        }
                        CharArray(length).also { buf -> if (length > 0) reader.read(buf, 0, length) }
                        val head = "HTTP/1.1 200 X\r\ncontent-length: 2\r\nconnection: close\r\n" +
                            (dateLine?.let { line -> "$line\r\n" } ?: "") + "\r\n[]"
                        it.getOutputStream().apply { write(head.toByteArray()); flush() }
                    }
                }
            }
        }
    }

    @After
    fun stop() = server.close()

    private val baseUrl get() = "http://127.0.0.1:${server.localPort}"

    private fun received(date: String?, mono: Long = 1) = ResponseDateCache.Received(date, mono, mono + 1, 1_000)

    private class FakeTransport(private val outcome: Outcome) : Transport {
        override fun post(bodyJson: String): Outcome = outcome
    }

    private fun sender(cache: ResponseDateCache, outcome: Outcome): Sender<IngestRequest> {
        val outbox = testOutbox()
        outbox.add(
            java.time.Instant.parse("2026-09-08T02:00:00Z").let {
                testFix(35.681236, 139.767125, 10f, it)
                    .toIngestRequest("id-1", "user-1", "device-1", java.time.ZoneId.of("Asia/Tokyo"))
            },
        )
        return Sender(outbox, FakeTransport(outcome), IngestRequest.serializer(), responseDates = cache)
    }

    @Test
    fun `応答の Date 見出しを文字列のまま、前後の単調時計と壁時計つきで返す`() {
        val monos = ArrayDeque(listOf(100L, 350L))
        val outcome = HttpTransport(baseUrl, "t", monoClock = { monos.removeFirst() }, wallClock = { 7_000L })
            .post("[]") as Outcome.Responded

        assertEquals("Tue, 29 Sep 2026 03:04:05 GMT", outcome.date)
        assertEquals(100L, outcome.monoBeforeMs)
        assertEquals(350L, outcome.monoAfterMs)
        assertEquals(7_000L, outcome.wallAfterMs)
    }

    // Scenario: 差に使う端末の時計は基準を読む前後の間で読む
    @Test
    fun `応答の壁時計は前後の単調時計の間で読む`() {
        // review R6: 壁時計を monoAfter の後に読むと、字面で Scenario に反する
        val order = mutableListOf<String>()
        HttpTransport(
            baseUrl,
            "t",
            monoClock = { order += "mono"; 1L },
            wallClock = { order += "wall"; 2L },
        ).post("[]") as Outcome.Responded

        assertEquals(listOf("mono", "wall", "mono"), order)
    }

    @Test
    fun `不正な送り先でも組み立ては投げず、送ると到達できないに畳まれる`() {
        // review R1: 組み立てで URL() が投げると、サービスの起動が落ち START_STICKY で落ち続ける
        val transport = HttpTransport("not a url", "t", monoClock = { 1L }, wallClock = { 2L })

        val outcome = transport.post("[]")

        assertEquals("MalformedURLException", (outcome as Outcome.Unreachable).kind)
    }

    @Test
    fun `Date 見出しが無い応答は date が null`() {
        dateLine = null

        val outcome = HttpTransport(baseUrl, "t", monoClock = { 1L }, wallClock = { 2L }).post("[]") as Outcome.Responded

        assertNull(outcome.date)
    }

    @Test
    fun `送信が受け取った最後の応答を置く`() {
        val cache = ResponseDateCache()
        sender(cache, Outcome.Responded(200, "[{\"accepted\":true}]", "d1", 1, 2, 3)).flush()
        sender(cache, Outcome.Responded(500, "", "d2", 10, 20, 30)).flush()

        assertEquals(
            ResponseDateCache.Taken.Got(ResponseDateCache.Received("d2", 10, 20, 30)),
            cache.take(),
        )
    }

    @Test
    fun `届かなかった送信は応答の日付を置かない`() {
        val cache = ResponseDateCache()
        sender(cache, Outcome.Unreachable("ConnectException")).flush()

        assertEquals(ResponseDateCache.Taken.Missing(ResponseDateCache.NO_RESPONSE_SINCE_LAST), cache.take())
    }

    // Scenario: 前回の測定より前の応答の日付は使わない
    @Test
    fun `取り出すと空になり、次の測定には前回より前の応答が出ない`() {
        val cache = ResponseDateCache()
        cache.put(received("Tue, 29 Sep 2026 03:04:05 GMT"))

        assertEquals(ResponseDateCache.Taken.Got(received("Tue, 29 Sep 2026 03:04:05 GMT")), cache.take())
        assertEquals(ResponseDateCache.Taken.Missing(ResponseDateCache.NO_RESPONSE_SINCE_LAST), cache.take())
    }

    // Scenario: 時計の変更より前に受け取った応答の日付は使わない
    @Test
    fun `時計の変更の通知で空になり、その理由を返す`() {
        val cache = ResponseDateCache()
        cache.put(received("Tue, 29 Sep 2026 03:04:05 GMT"))
        cache.clockChanged()

        assertEquals(ResponseDateCache.Taken.Missing(ResponseDateCache.CLOCK_CHANGED_SINCE), cache.take())
        // 取り出したあとは「前回の測定より後に応答が無い」に戻る
        assertEquals(ResponseDateCache.Taken.Missing(ResponseDateCache.NO_RESPONSE_SINCE_LAST), cache.take())
    }

    @Test
    fun `時計の変更のあとに受け取った応答は使える`() {
        val cache = ResponseDateCache()
        cache.clockChanged()
        cache.put(received("Tue, 29 Sep 2026 03:04:05 GMT"))

        assertEquals(ResponseDateCache.Taken.Got(received("Tue, 29 Sep 2026 03:04:05 GMT")), cache.take())
    }
}
