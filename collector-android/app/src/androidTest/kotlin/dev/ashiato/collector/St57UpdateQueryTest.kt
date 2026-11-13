package dev.ashiato.collector

import android.content.Context
import android.os.Build
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.net.ServerSocket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.zip.GZIPOutputStream

@RunWith(AndroidJUnit4::class)
class St57UpdateQueryTest {
    private val context: Context = ApplicationProvider.getApplicationContext()
    private fun query(response: ByteArray, encoding: String? = null): List<String> {
        val server = ServerSocket(0); val request = arrayListOf<String>(); val done = CountDownLatch(1)
        Thread {
            server.use { listener -> listener.accept().use { socket ->
                val input = socket.getInputStream().bufferedReader()
                while (true) { val line = input.readLine() ?: break; if (line.isEmpty()) break; request += line }
                val headers = if (encoding == null) "" else "Content-Encoding: $encoding\r\n"
                socket.getOutputStream().use { out ->
                    out.write("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n$headers\r\n".toByteArray())
                    out.write(response)
                    out.flush()
                }
            }}; done.countDown()
        }.start()
        context.getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().putBoolean("enabled", true).putBoolean("intro_shown", true).commit()
        Config.overrideForTest("http://127.0.0.1:${server.localPort}", "token", "user")
        assertTrue(UpdateChecker(context, "http://127.0.0.1:${server.localPort}").check())
        assertTrue(done.await(5, TimeUnit.SECONDS)); return request
    }
    @Test fun query_has_fixed_contract() {
        // Scenario: 携帯の問い合わせは版の一覧の取得だけである
        val lines = query("[{\"tag_name\":\"v99.0.0\",\"draft\":false,\"assets\":[]}]".toByteArray())
        assertTrue(lines.any { it.startsWith("GET /repos/rai-256/ashiato/releases?per_page=20") })
        assertTrue(lines.any { it.startsWith("User-Agent: ashiato-update-check") }); assertTrue(lines.any { it.startsWith("Accept: application/vnd.github+json") })
        assertTrue(lines.any { it.startsWith("Accept-Encoding: gzip") })
        val names = lines.drop(1).map { it.substringBefore(':').lowercase() }.toSet()
        assertTrue(names.all { it in setOf("host", "user-agent", "accept", "accept-encoding", "connection") })
        val values = lines.drop(1).joinToString("\n")
        assertFalse(values.contains(Build.MODEL, ignoreCase = true))
        assertFalse(values.contains(Build.VERSION.RELEASE, ignoreCase = true))
        assertFalse(values.contains(BuildConfig.VERSION_NAME, ignoreCase = true))
        assertFalse(lines.any { it.startsWith("Authorization:") || it.startsWith("Cookie:") })
    }
    @Test fun accepts_gzip_response() {
        // Scenario: 携帯は圧縮した応答を受ける
        val compressed = java.io.ByteArrayOutputStream().also { GZIPOutputStream(it).use { gzip -> gzip.write("[]".toByteArray()) } }.toByteArray()
        query(compressed, "gzip"); assertEquals(null, context.getSharedPreferences(UpdateCheck.PREFS, 0).getString("last_reason", null))
    }
}
