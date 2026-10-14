// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder
import java.time.DateTimeException
import java.time.LocalDate
import java.time.LocalTime

/** 通知の判断の応答。**2 欄だけ**（主観の中身は返ってこない。design D9）。 */
data class ReminderAnswer(val at: LocalTime, val recorded: Boolean)

/** 通知の判断をサーバに聞く口。得られなければ null（網に届かない・断られた・読めない）。 */
fun interface ReminderSource {
    fun fetch(date: LocalDate): ReminderAnswer?
}

@Serializable
private class ReminderWire(val at: String, val recorded: Boolean)

/**
 * `GET /feelings/reminder?date=` を 1 回、10 秒で打ち切って聞く（design D10）。
 * 例外は種別だけに畳む。設定が揃っていなければ聞かない。
 */
class HttpReminderSource(
    baseUrl: String,
    private val token: String,
    private val userId: String,
    private val timeoutMs: Int = TIMEOUT_MS,
) : ReminderSource {
    private val baseUrl = baseUrl.trimEnd('/')

    override fun fetch(date: LocalDate): ReminderAnswer? {
        if (baseUrl.isBlank() || token.isBlank() || userId.isBlank()) return null
        var conn: HttpURLConnection? = null
        return try {
            val q = "date=$date&user_id=${URLEncoder.encode(userId, "UTF-8")}"
            conn = (URL("$baseUrl/feelings/reminder?$q").openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"
                connectTimeout = timeoutMs
                readTimeout = timeoutMs
                setRequestProperty("authorization", "Bearer $token")
            }
            if (conn.responseCode in HttpURLConnection.HTTP_OK until HttpURLConnection.HTTP_MULT_CHOICE) {
                val body = conn.inputStream.bufferedReader(Charsets.UTF_8).use { it.readText() }
                val wire = JSON.decodeFromString(ReminderWire.serializer(), body)
                ReminderAnswer(LocalTime.parse(wire.at), wire.recorded)
            } else {
                null
            }
        } catch (_: IOException) {
            null
        } catch (_: SerializationException) {
            null
        } catch (_: DateTimeException) {
            null
        } finally {
            conn?.disconnect()
        }
    }

    private companion object {
        const val TIMEOUT_MS = 10_000
        val JSON = Json { ignoreUnknownKeys = true }
    }
}
