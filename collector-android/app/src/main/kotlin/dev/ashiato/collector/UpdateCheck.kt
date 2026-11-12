// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.time.Instant
import java.time.temporal.ChronoUnit

internal object UpdateCheck {
    const val PREFS = "update_check"
    const val RECORDS_FILE = "update_checks.jsonl"
    const val CHANNEL = "update"
    const val URL_PATH = "/repos/rai-256/ashiato/releases?per_page=20"
    const val NIGHTLY_PATH = "/repos/rai-256/ashiato/releases/tags/nightly"
    const val USER_AGENT = "ashiato-update-check"
    const val ACCEPT = "application/vnd.github+json"

    data class Choice(val latest: String?, val available: Boolean)

    fun due(now: Instant, last: Instant?): Boolean =
        last == null || last.isAfter(now) || !now.isBefore(last.plus(24, ChronoUnit.HOURS))

    fun stale(now: Instant, last: Instant?): Boolean =
        last == null || last.isAfter(now) || !now.isBefore(last.plus(7, ChronoUnit.DAYS))

    fun choose(component: String, installed: String, body: String): Choice {
        val own = Version.parse(installed) ?: return Choice(null, false)
        val dev = own.pre.startsWith("dev.")
        val beta = own.pre.startsWith("beta.")
        val prefix = when (component) {
            "server" -> "ashiato-server-"
            "collector-windows" -> "ashiato-collector-windows-"
            else -> if (dev) "ashiato-collector-dev-" else "ashiato-collector-"
        }
        val suffix = when (component) {
            "server" -> "-windows-x86_64.zip"
            "collector-windows" -> "-x86_64.zip"
            else -> ".apk"
        }
        val versions = runCatching {
            Json.parseToJsonElement(body).jsonArray.flatMap { release ->
                val obj = release.jsonObject
                if (obj["draft"]?.jsonPrimitive?.content == "true") return@flatMap emptyList()
                val nightly = obj["tag_name"]?.jsonPrimitive?.content == "nightly"
                if (nightly != dev) return@flatMap emptyList()
                obj["assets"]?.jsonArray.orEmpty().mapNotNull { asset ->
                    val name = asset.jsonObject["name"]?.jsonPrimitive?.content ?: return@mapNotNull null
                    if (!name.startsWith(prefix) || !name.endsWith(suffix)) return@mapNotNull null
                    val raw = name.removePrefix(prefix).removeSuffix(suffix)
                    val parsed = Version.parse(raw) ?: return@mapNotNull null
                    if ((dev && !parsed.pre.startsWith("dev.")) || (!dev && beta && !(parsed.pre.startsWith("beta.") || parsed.pre.isEmpty())) || (!dev && !beta && parsed.pre.isNotEmpty())) null else parsed
                }
            }
        }.getOrDefault(emptyList())
        val latest = versions.maxOrNull()?.text
        return Choice(latest, latest?.let { Version.parse(it)!! > own } == true)
    }

    private data class Version(val major: Int, val minor: Int, val patch: Int, val pre: String, val text: String) : Comparable<Version> {
        override fun compareTo(other: Version): Int {
            compareValuesBy(this, other, Version::major, Version::minor, Version::patch).takeIf { it != 0 }?.let { return it }
            if (pre.isEmpty() != other.pre.isEmpty()) return if (pre.isEmpty()) 1 else -1
            return comparePre(pre, other.pre)
        }
        companion object {
            fun parse(value: String): Version? {
                val clean = value.removePrefix("v")
                val match = Regex("^(\\d+)\\.(\\d+)\\.(\\d+)(?:-([^+]+))?$").matchEntire(clean) ?: return null
                return Version(match.groupValues[1].toInt(), match.groupValues[2].toInt(), match.groupValues[3].toInt(), match.groupValues[4], clean)
            }
            private fun comparePre(a: String, b: String): Int {
                val aa = a.split('.'); val bb = b.split('.')
                for (i in 0 until maxOf(aa.size, bb.size)) {
                    val x = aa.getOrNull(i) ?: return -1; val y = bb.getOrNull(i) ?: return 1
                    val n = x.toIntOrNull()?.compareTo(y.toIntOrNull() ?: return -1) ?: x.compareTo(y)
                    if (n != 0) return n
                }
                return 0
            }
        }
    }
}

internal class UpdateChecker(private val context: Context) {
    private val prefs: SharedPreferences = context.getSharedPreferences(UpdateCheck.PREFS, Context.MODE_PRIVATE)
    private val records = File(context.noBackupFilesDir, UpdateCheck.RECORDS_FILE)

    fun check(now: Instant = Instant.now()): Boolean {
        if (!prefs.getBoolean("enabled", true)) return false
        if (!prefs.getBoolean("intro_shown", false)) {
            if (post("更新の確認: 有効（切るには設定の画面）")) markIntroShown(now)
            return false
        }
        val last = prefs.getString("checked_at", null)?.let { runCatching { Instant.parse(it) }.getOrNull() }
        if (!UpdateCheck.due(now, last)) return false
        val destination = BuildConfig.UPDATE_CHECK_URL.takeIf { it.isNotBlank() }
            ?: "https://api.github.com".takeIf { Config.hasSavedSetup || Config.hasBuildSetup }
            ?: ""
        if (destination.isBlank()) {
            record(now, false, "no_destination", null)
            return true
        }
        val result = runCatching {
            val url = URL(destination + if (BuildConfig.VERSION_NAME.contains("-dev.")) UpdateCheck.NIGHTLY_PATH else UpdateCheck.URL_PATH)
            val connection = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"; connectTimeout = 10_000; readTimeout = 30_000
                setRequestProperty("User-Agent", UpdateCheck.USER_AGENT)
                setRequestProperty("Accept", UpdateCheck.ACCEPT)
                setRequestProperty("Accept-Encoding", "gzip")
                doOutput = false
            }
            val stream = if (connection.contentEncoding.equals("gzip", true)) java.util.zip.GZIPInputStream(connection.inputStream) else connection.inputStream
            stream.use { it.readBytes().toString(Charsets.UTF_8) }
        }
        val choice = result.getOrNull()?.let { UpdateCheck.choose("android", BuildConfig.VERSION_NAME, it) }
        record(now, choice != null, if (choice == null) "unreachable" else null, choice?.latest)
        if (choice != null) {
            val oldLatest = prefs.getString("latest", null)
            val edit = prefs.edit().putString("latest", choice.latest)
            if (oldLatest != choice.latest) edit.putString("found_at", now.toString())
            edit.apply()
            if (choice.available && prefs.getString("notified_version", null) != choice.latest && post("新しい版 ${choice.latest} が出ています")) {
                prefs.edit().putString("notified_version", choice.latest).apply()
            }
        } else if (!prefs.getBoolean("stale_notified", false)) {
            val lastOk = prefs.getString("last_ok_at", null)?.let { runCatching { Instant.parse(it) }.getOrNull() }
            val since = lastOk ?: prefs.getString("intro_shown_at", null)?.let { runCatching { Instant.parse(it) }.getOrNull() }
            if (UpdateCheck.stale(now, since) && post("更新を 7 日確かめられていません")) {
                prefs.edit().putBoolean("stale_notified", true).apply()
            }
        }
        return true
    }

    fun setEnabled(enabled: Boolean) { prefs.edit().putBoolean("enabled", enabled).apply() }

    fun introPending(): Boolean = !prefs.getBoolean("intro_shown", false)

    fun markIntroShown(now: Instant = Instant.now()) {
        prefs.edit().putBoolean("intro_shown", true).putString("intro_shown_at", now.toString()).apply()
    }

    private fun record(now: Instant, ok: Boolean, reason: String?, latest: String?) {
        records.parentFile?.mkdirs()
        val reasonJson = reason?.let { "\"$it\"" } ?: "null"
        val latestJson = latest?.let { "\"$it\"" } ?: "null"
        records.appendText("{\"at\":\"$now\",\"ok\":$ok,\"reason\":$reasonJson,\"latest\":$latestJson}\n")
        val edit = prefs.edit().putString("checked_at", now.toString()).putString("last_reason", reason)
        if (ok) edit.putString("last_ok_at", now.toString()).putBoolean("stale_notified", false)
        edit.apply()
    }

    private fun post(text: String): Boolean {
        val manager = context.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel(UpdateCheck.CHANNEL, "更新", NotificationManager.IMPORTANCE_DEFAULT))
        if (!canPost(manager)) return false
        manager.notify(text.hashCode(), NotificationCompat.Builder(context, UpdateCheck.CHANNEL).setContentTitle("あしあと。").setContentText(text).setSmallIcon(android.R.drawable.ic_popup_sync).setContentIntent(PendingIntent.getActivity(context, 0, Intent(context, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)).setAutoCancel(true).build())
        return true
    }

    private fun canPost(manager: NotificationManager): Boolean =
        (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU || context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) && manager.areNotificationsEnabled() && manager.getNotificationChannel(UpdateCheck.CHANNEL)?.importance != NotificationManager.IMPORTANCE_NONE
}
