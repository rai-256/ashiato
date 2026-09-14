// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.BufferedReader
import java.net.ServerSocket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * **本物の Android の HTTP スタック**で取り込み口へ届くことを確かめる（実機・エミュレータの上でだけ走る）。
 *
 * JVM の単体（`HttpTransportTest`）は URL の組み立てと応答の畳み方を見るが、
 * Android の `HttpURLConnection` と平文 HTTP の可否（`network_security_config`）は端末の上でしか分からない
 * （実測 ST01: 平文が遮断されて 1 件も届かず、logcat に 1 行出るだけだった）。
 *
 * 相手は**テストが自分で開く**ローカルのサーバ（端末の中の 127.0.0.1）。
 * 平文を通すには、ビルド時に `-Pashiato.baseUrl=http://127.0.0.1:18787` で 127.0.0.1 を許可する
 * （`tools/android-emulator.sh` と CI がそうしている）。
 */
@RunWith(AndroidJUnit4::class)
class HttpTransportInstrumentedTest {
    /** 1 要求だけ受けて、受け取った要求行・ヘッダ・本文を返す小さなサーバ。 */
    private class OneShotServer(private val status: Int, private val reply: String) {
        val socket = ServerSocket(0)
        val port: Int get() = socket.localPort
        var requestLine = ""
        var headers = mutableMapOf<String, String>()
        var body = ""
        val done = CountDownLatch(1)

        fun start() = Thread {
            socket.accept().use { s ->
                val reader = s.getInputStream().bufferedReader(Charsets.UTF_8)
                requestLine = reader.readLine() ?: ""
                var len = 0
                while (true) {
                    val line = reader.readLine() ?: break
                    if (line.isEmpty()) break
                    val (k, v) = line.split(":", limit = 2).let { it[0].trim().lowercase() to it.getOrElse(1) { "" }.trim() }
                    headers[k] = v
                    if (k == "content-length") len = v.toInt()
                }
                body = readExactly(reader, len)
                val bytes = reply.toByteArray(Charsets.UTF_8)
                s.getOutputStream().use { out ->
                    out.write(
                        ("HTTP/1.1 $status OK\r\nContent-Type: application/json\r\nContent-Length: ${bytes.size}\r\nConnection: close\r\n\r\n")
                            .toByteArray(Charsets.UTF_8),
                    )
                    out.write(bytes)
                    out.flush()
                }
            }
            done.countDown()
        }.apply { isDaemon = true; start() }

        private fun readExactly(reader: BufferedReader, len: Int): String {
            val buf = CharArray(len)
            var read = 0
            while (read < len) {
                val n = reader.read(buf, read, len - read)
                if (n < 0) break
                read += n
            }
            return String(buf, 0, read)
        }
    }

    // Scenario: 到達できるとき送られる
    @Test
    fun postReachesTheServerWithBearerAndBody() {
        val server = OneShotServer(200, """[{"id":null,"duplicate":false,"accepted":true,"error":null}]""")
        server.start()
        val transport = HttpTransport("http://127.0.0.1:${server.port}", "test-token-0123456789abcdef", "/ingest")

        val outcome = transport.post("""[{"hello":"world"}]""")

        assertTrue("サーバに届く", server.done.await(10, TimeUnit.SECONDS))
        assertEquals("POST /ingest HTTP/1.1", server.requestLine)
        assertEquals("Bearer test-token-0123456789abcdef", server.headers["authorization"])
        assertEquals("application/json", server.headers["content-type"])
        assertEquals("""[{"hello":"world"}]""", server.body)
        assertEquals(Outcome.Responded(200, """[{"id":null,"duplicate":false,"accepted":true,"error":null}]"""), outcome)
    }

    /** 400 は「到達したが断られた」で、網の失敗（`Unreachable`）とは別物。本文は読み出せる。 */
    @Test
    fun rejectionBodyIsReadFromTheErrorStream() {
        val server = OneShotServer(400, """[{"accepted":false,"error":"malformed"}]""")
        server.start()
        val transport = HttpTransport("http://127.0.0.1:${server.port}", "t", "/ingest")

        val outcome = transport.post("{}")

        assertTrue(server.done.await(10, TimeUnit.SECONDS))
        assertEquals(Outcome.Responded(400, """[{"accepted":false,"error":"malformed"}]"""), outcome)
    }

    /** 誰も聞いていない port は `Unreachable`。**例外の型名だけ**が残り、URL や本文は出ない。 */
    @Test
    fun closedPortIsUnreachableNotACrash() {
        val port = ServerSocket(0).use { it.localPort }
        val transport = HttpTransport("http://127.0.0.1:$port", "t", "/ingest")

        val outcome = transport.post("{}")

        assertTrue("$outcome", outcome is Outcome.Unreachable)
        assertEquals("ConnectException", (outcome as Outcome.Unreachable).kind)
    }
}
