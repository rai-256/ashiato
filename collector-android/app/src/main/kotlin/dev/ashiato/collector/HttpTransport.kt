// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.os.SystemClock
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL

/**
 * 取り込み口への POST。**資格情報を必ず付ける** —— 付けないと 401 で断られる（PERM-10）。
 *
 * 例外は種別だけに畳む。文言は送った本文を含むことがあり、ログへ流すと私的データが漏れる。
 */
class HttpTransport(
    baseUrl: String,
    private val token: String,
    /**
     * 送り先の道。**受け口ごとに違う**（design D9）—— `/ingest` は記録のエンベロープを
     * 必須にしており、生存信号はそのどれも持たない。混ぜると片方のために必須の欄が緩む。
     */
    private val path: String = "/ingest",
    /** 単調時計と壁時計。時計のずれの測定が使う値を応答に載せる（ST05 / design D2）。試験だけが差し替える。 */
    private val monoClock: () -> Long = { SystemClock.elapsedRealtime() },
    private val wallClock: () -> Long = { System.currentTimeMillis() },
) : Transport {
    /** 末尾のスラッシュを落としてから組み立てる。`https://host/` が渡ると `//ingest` になり、
     *  404 が返り続けて**収集は動いているのに 1 件も届かない**状態が黙って続く（review R22）。 */
    private val baseUrl = baseUrl.trimEnd('/')

    override fun post(bodyJson: String): Outcome {
        var conn: HttpURLConnection? = null
        val monoBefore = monoClock()
        return try {
            conn = (URL("$baseUrl$path").openConnection() as HttpURLConnection).apply {
                requestMethod = "POST"
                connectTimeout = 15_000
                readTimeout = 15_000
                doOutput = true
                setRequestProperty("content-type", "application/json")
                setRequestProperty("authorization", "Bearer $token")
            }
            conn.outputStream.use { it.write(bodyJson.toByteArray(Charsets.UTF_8)) }
            val status = conn.responseCode
            // 400 でも本文が要る。1 件ごとの結果がそこにある（docs/collector-contract.md）
            val stream = if (status in 200..299) conn.inputStream else conn.errorStream
            val body = stream?.bufferedReader(Charsets.UTF_8)?.use { it.readText() } ?: ""
            val date = conn.getHeaderField("Date")
            val monoAfter = monoClock()
            Outcome.Responded(status, body, date, monoBefore, monoAfter, wallClock())
        } catch (e: IOException) {
            Outcome.Unreachable(e.javaClass.simpleName)
        } finally {
            conn?.disconnect()
        }
    }
}
