// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.ServerSocket
import java.net.Socket
import kotlin.concurrent.thread
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * 送信に**資格情報が付く**（tasks 9.9 / PERM-10 /
 * specs/record-envelope の「すべての API 要求は資格情報を要求する」）。
 *
 * `SenderTest` は `Transport` を偽物に差し替えるので、**ヘッダは素通りしていた** ——
 * 資格情報を付け忘れても全部緑になる。ここは本物の `HttpTransport` を
 * 本物の TCP の口へ向けて、**実際に飛んだ要求のバイト列**を見る。
 *
 * サーバは生の `ServerSocket` で書く。`com.sun.net.httpserver` は Android の
 * コンパイル経路に無く、HTTP の器のために依存を 1 つ増やす価値も無い。
 */
class HttpTransportTest {
    private lateinit var server: ServerSocket
    private val requests = mutableListOf<Request>()

    /** 次に返す応答。試験ごとに差し替える。 */
    private var reply: Pair<Int, String> = 200 to "[]"

    data class Request(val line: String, val headers: Map<String, String>, val body: String)

    private val baseUrl get() = "http://127.0.0.1:${server.localPort}"

    @Before
    fun start() {
        server = ServerSocket(0, 0, java.net.InetAddress.getLoopbackAddress())
        thread(isDaemon = true) {
            while (!server.isClosed) {
                val socket = runCatching { server.accept() }.getOrNull() ?: return@thread
                runCatching { socket.use { serve(it) } }
            }
        }
    }

    private fun serve(socket: Socket) {
        val reader = BufferedReader(InputStreamReader(socket.getInputStream(), Charsets.UTF_8))
        val line = reader.readLine() ?: return
        val headers = buildMap {
            while (true) {
                val h = reader.readLine()
                if (h.isNullOrEmpty()) break
                val i = h.indexOf(':')
                if (i > 0) put(h.take(i).lowercase(), h.substring(i + 1).trim())
            }
        }
        val length = headers["content-length"]?.toIntOrNull() ?: 0
        val body = CharArray(length).also { if (length > 0) reader.read(it, 0, length) }.concatToString()
        synchronized(requests) { requests += Request(line, headers, body) }

        val (status, payload) = reply
        val bytes = payload.toByteArray(Charsets.UTF_8)
        socket.getOutputStream().apply {
            write(
                ("HTTP/1.1 $status X\r\ncontent-type: application/json\r\n" +
                    "content-length: ${bytes.size}\r\nconnection: close\r\n\r\n").toByteArray()
            )
            write(bytes)
            flush()
        }
    }

    @After
    fun stop() = server.close()

    private fun sent(): Request = synchronized(requests) { requests.single() }

    @Test
    fun `送信に Authorization Bearer が付く`() {
        HttpTransport(baseUrl, "token-0123456789abcdef").post("[]")

        assertEquals("Bearer token-0123456789abcdef", sent().headers["authorization"])
    }

    @Test
    fun `取り込み口へ POST する`() {
        HttpTransport(baseUrl, "t").post("[]")

        assertTrue("要求行が違う: ${sent().line}", sent().line.startsWith("POST /ingest "))
    }

    @Test
    fun `本文と content-type が契約どおりに飛ぶ`() {
        HttpTransport(baseUrl, "t").post("""[{"id":"a"}]""")

        assertEquals("application/json", sent().headers["content-type"])
        assertEquals("""[{"id":"a"}]""", sent().body)
    }

    @Test
    fun `401 が返ったら応答として受け取る`() {
        // **握り潰さない。** Sender はこれを見て「何も取り除かない」を選ぶ
        reply = 401 to "unauthorized"

        val outcome = HttpTransport(baseUrl, "wrong").post("[]")

        assertEquals(401, (outcome as Outcome.Responded).status)
    }

    @Test
    fun `400 でも本文を読む`() {
        // 1 件ごとの結果がそこにある（docs/collector-contract.md §状態符号）
        reply = 400 to """[{"accepted":false,"error":"unknown_origin"}]"""

        val outcome = HttpTransport(baseUrl, "t").post("[]")

        assertEquals(400, (outcome as Outcome.Responded).status)
        assertTrue(outcome.body.contains("unknown_origin"))
    }

    @Test
    fun `到達できなければ種別だけを返し、本文を持ち出さない`() {
        val dead = "http://127.0.0.1:1"

        val outcome = HttpTransport(dead, "t").post("""[{"lat":35.681236}]""")

        // **種別そのものを固定する**（review R7）。`!contains("35.68")` だけだと
        // `e.message` に書き換えても "Connection refused" は緯度を含まないので落ちない
        assertEquals("ConnectException", (outcome as Outcome.Unreachable).kind)
    }

    @Test
    fun `資格情報が空でもヘッダは立つ`() {
        // **「付け忘れ」が表現できない**ことを固定する。空なら送り先が 401 で断る
        HttpTransport(baseUrl, "").post("[]")

        assertEquals("Bearer", sent().headers["authorization"])
    }

    /**
     * **生存信号の送り先が `/heartbeat`** であること（ST02 の review/code.md の R41 / F7）。
     *
     * `"/ingest"` に書き換えても全部緑だった —— 現実に起きるのは
     * 「サーバが `malformed` で全件断り、未送信が永久に溜まり、logcat に 1 行出るだけ」。
     * `HttpTransport` のコメントが自ら警告している失敗の型そのもの。
     */
    @Test
    fun `生存信号は heartbeat へ送られる`() {
        HttpTransport(baseUrl, "t", "/heartbeat").post("[]")
        assertTrue("送り先が /heartbeat でない: ${sent().line}", sent().line.startsWith("POST /heartbeat "))
    }

    /** 既定は記録の受け口（呼び分けを間違えたときに気付けるよう、既定も固定する）。 */
    @Test
    fun `既定の送り先は ingest`() {
        HttpTransport(baseUrl, "t").post("[]")
        assertTrue("既定が /ingest でない: ${sent().line}", sent().line.startsWith("POST /ingest "))
    }
}