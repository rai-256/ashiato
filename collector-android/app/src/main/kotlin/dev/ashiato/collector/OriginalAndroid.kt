// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.ContentUris
import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.provider.MediaStore
import java.io.FileNotFoundException
import java.io.IOException
import java.io.InputStream
import java.net.HttpURLConnection
import java.net.URL

/** いまの通信が従量制でないか（`NET_CAPABILITY_NOT_METERED`。過去ぶんの原本はこのときだけ送る。design D9）。 */
internal fun isUnmetered(context: Context): Boolean {
    val manager = context.getSystemService(ConnectivityManager::class.java)
    val caps = manager?.getNetworkCapabilities(manager.activeNetwork)
    return caps?.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) ?: false
}

/**
 * 原本を端末の一覧（`MediaStore`）から開く。`setRequireOriginal` で位置を伏せさせない ——
 * **位置の権限が無いと開けないので、呼ぶのは権限があるときだけ**（Q15。[OriginalUploader] が門を持つ）。
 */
class ContentResolverOriginalSource(private val context: Context) : OriginalSource {
    override fun open(entry: OriginalEntry): OriginalOpen = try {
        val base = MediaStore.Images.Media.getContentUri(entry.volume)
        val uri = MediaStore.setRequireOriginal(ContentUris.withAppendedId(base, entry.mediaId))
        context.contentResolver.openInputStream(uri)?.let { OriginalOpen.Opened(it) } ?: OriginalOpen.Unreadable
    } catch (_: FileNotFoundException) {
        OriginalOpen.Vanished
    } catch (_: SecurityException) {
        OriginalOpen.Denied
    } catch (_: IOException) {
        OriginalOpen.Unreadable
    } catch (_: IllegalArgumentException) {
        OriginalOpen.Unreadable
    } catch (_: UnsupportedOperationException) {
        OriginalOpen.Unreadable
    }
}

/**
 * 原本の受け口への `HEAD` / `PUT`（design D10）。資格情報は `/ingest` と同じ。
 * 本文は**固定長の流し送り**（端末にもメモリにも写しを作らない）。例外は種別だけに畳む。
 */
class HttpOriginalTransport(
    baseUrl: String,
    private val token: String,
    private val userId: () -> String,
) : OriginalTransport {
    private val baseUrl = baseUrl.trimEnd('/')

    override fun head(sha256: String): Outcome = call("HEAD", sha256) { null }

    override fun put(sha256: String, length: Long, body: InputStream): Outcome = call("PUT", sha256) { conn ->
        conn.doOutput = true
        conn.setFixedLengthStreamingMode(length)
        conn.setRequestProperty("content-type", "application/octet-stream")
        conn.outputStream.use { out -> body.copyTo(out) }
        null
    }

    private fun call(method: String, sha256: String, send: (HttpURLConnection) -> Unit?): Outcome {
        var conn: HttpURLConnection? = null
        return try {
            val url = URL("$baseUrl/media/originals/$sha256?user_id=${userId()}")
            conn = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = method
                connectTimeout = CONNECT_TIMEOUT_MS
                // 本文を送り終えてから受け口がハッシュを確かめて答えるまでの待ち
                readTimeout = READ_TIMEOUT_MS
                setRequestProperty("authorization", "Bearer $token")
            }
            send(conn)
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
        const val CONNECT_TIMEOUT_MS = 15_000
        const val READ_TIMEOUT_MS = 120_000
    }
}
