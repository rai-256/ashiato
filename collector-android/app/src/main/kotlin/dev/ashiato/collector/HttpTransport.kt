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

    /**
     * `s01-date` の宛先の `host:port`（ST05 / design D2）。**コンストラクタでは読まない** ——
     * 不正な `BASE_URL` で `URL()` が投げるとサービスの起動が落ち、`START_STICKY` で落ち続ける。
     * 読むのは応答を受け取った後（そのときは `post()` の `URL()` が既に通っている）。読めなければ null（review R1）。
     */
    private val hostPort: String? by lazy {
        runCatching { URL(this.baseUrl).let { u -> if (u.port >= 0) "${u.host}:${u.port}" else u.host } }.getOrNull()
    }

    override fun post(bodyJson: String): Outcome = post(bodyJson, CONTRACT_VERSION)

    /** 見出し `Ashiato-Contract-Version` に載せた行の版の最大を付けて送る（ST50 / D4）。 */
    override fun post(bodyJson: String, contractVersion: Int): Outcome {
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
                setRequestProperty(CONTRACT_HEADER, contractVersion.toString())
            }
            conn.outputStream.use { it.write(bodyJson.toByteArray(Charsets.UTF_8)) }
            val status = conn.responseCode
            // 400 でも本文が要る。1 件ごとの結果がそこにある（docs/collector-contract.md）
            val stream = if (status in 200..299) conn.inputStream else conn.errorStream
            val body = stream?.bufferedReader(Charsets.UTF_8)?.use { it.readText() } ?: ""
            val date = conn.getHeaderField("Date")
            // 壁時計は**単調時計の前後の間**で読む（Scenario「差に使う端末の時計は基準を読む前後の間で読む」。review R6）
            val wall = wallClock()
            val monoAfter = monoClock()
            Outcome.Responded(status, body, date, monoBefore, monoAfter, wall, hostPort)
        } catch (e: IOException) {
            Outcome.Unreachable(e.javaClass.simpleName)
        } finally {
            conn?.disconnect()
        }
    }

    /**
     * `GET`（途切れの問い合わせ。ST14 / design D12）。10 秒で打ち切る。
     * `path` に続けて `query`（`?device_id=…`）を付ける。時計の測定には使わないので値は載せない。
     */
    fun get(query: String = ""): Outcome {
        var conn: HttpURLConnection? = null
        return try {
            conn = (URL("$baseUrl$path$query").openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"
                connectTimeout = GET_TIMEOUT_MS
                readTimeout = GET_TIMEOUT_MS
                setRequestProperty("authorization", "Bearer $token")
            }
            val status = conn.responseCode
            val stream = if (status in 200..299) conn.inputStream else conn.errorStream
            Outcome.Responded(status, stream?.bufferedReader(Charsets.UTF_8)?.use { it.readText() } ?: "")
        } catch (e: IOException) {
            Outcome.Unreachable(e.javaClass.simpleName)
        } finally {
            conn?.disconnect()
        }
    }

    private companion object {
        const val GET_TIMEOUT_MS = 10_000
    }
}

/** 要求が契約の版を載せる見出し（ST50 / D4）。 */
const val CONTRACT_HEADER: String = "Ashiato-Contract-Version"
