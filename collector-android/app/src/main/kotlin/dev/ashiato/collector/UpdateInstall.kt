// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.content.pm.PackageInstaller
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.Executors

internal object UpdateInstall {
    private const val GITHUB_PREFIX = "https://github.com/rai-256/ashiato/releases/download/"
    private const val ASSET_HOST = "release-assets.githubusercontent.com"
    private const val APK_PREFIX = "ashiato-collector-"
    private const val APK_SUFFIX = ".apk"
    const val EXTRA_START = "dev.ashiato.collector.UPDATE_START"
    const val ACTION_RESULT = "dev.ashiato.collector.UPDATE_RESULT"
    const val UPDATE_DIR = "update"

    data class Asset(val version: String, val url: String, val sha256: String, val name: String)

    fun chooseAsset(installed: String, body: String): Asset? {
        val latest = UpdateCheck.choose("android", installed, body).latest ?: return null
        val root = Json.parseToJsonElement(body)
        val releases = if (root is JsonArray) root else JsonArray(listOf(root))
        return releases.asSequence().flatMap { release ->
            val obj = release.jsonObject
            if (obj["draft"]?.jsonPrimitive?.content == "true") emptySequence()
            else obj["assets"]?.jsonArray.orEmpty().asSequence()
        }.mapNotNull { asset ->
            val obj = asset.jsonObject
            val name = obj["name"]?.jsonPrimitive?.content ?: return@mapNotNull null
            val url = obj["browser_download_url"]?.jsonPrimitive?.content ?: return@mapNotNull null
            val digest = obj["digest"]?.jsonPrimitive?.content ?: return@mapNotNull null
            val sha = digest.removePrefix("sha256:")
            if (name != "$APK_PREFIX$latest$APK_SUFFIX" || !isAllowedDownloadUrl(url) ||
                !Regex("[0-9a-fA-F]{64}").matches(sha) || !digest.startsWith("sha256:")) null
            else Asset(latest, url, sha.lowercase(), name)
        }.firstOrNull()
    }

    fun isAllowedDownloadUrl(value: String): Boolean =
        value.startsWith(GITHUB_PREFIX) || runCatching { URL(value).host == ASSET_HOST && URL(value).protocol == "https" }.getOrDefault(false)

    fun updateDirectory(context: Context): File = File(context.noBackupFilesDir, UPDATE_DIR)

    fun clean(context: Context) {
        updateDirectory(context).listFiles()?.forEach { if (it.isFile) it.delete() }
    }

    internal fun verifyDigest(file: File, expected: String): Boolean = sha256(file) == expected.lowercase()

    fun start(context: Context) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !context.packageManager.canRequestPackageInstalls()) {
            context.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${context.packageName}")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            return
        }
        Executors.newSingleThreadExecutor().execute {
            try {
                val path = if (BuildConfig.VERSION_NAME.contains("-dev.")) UpdateCheck.NIGHTLY_PATH else UpdateCheck.URL_PATH
                val connection = (URL(BuildConfig.UPDATE_CHECK_URL + path).openConnection() as HttpURLConnection).apply {
                    connectTimeout = 10_000; readTimeout = 30_000
                    setRequestProperty("User-Agent", UpdateCheck.USER_AGENT)
                }
                val body = connection.inputStream.use { it.readBytes().toString(Charsets.UTF_8) }
                val asset = chooseAsset(BuildConfig.VERSION_NAME, body) ?: return@execute
                install(context, asset, body)
            } catch (_: Exception) {
                clean(context)
            }
        }
    }

    fun install(context: Context, asset: Asset, releasesBody: String) {
        Executors.newSingleThreadExecutor().execute {
            val file = File(updateDirectory(context), asset.name)
            try {
                file.parentFile?.mkdirs()
                download(asset.url, file)
                if (!verifyDigest(file, asset.sha256)) {
                    file.delete()
                    return@execute
                }
                val installer = context.packageManager.packageInstaller
                val sessionId = installer.createSession(PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL))
                installer.openSession(sessionId).use { session ->
                    file.inputStream().use { input -> session.openWrite("base.apk", 0, file.length()).use { output -> input.copyTo(output); session.fsync(output) } }
                    val intent = Intent(context, UpdateInstallResultReceiver::class.java).setAction(ACTION_RESULT)
                    val pending = PendingIntent.getBroadcast(context, sessionId, intent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE)
                    session.commit(pending.intentSender)
                }
            } catch (_: Exception) {
                file.delete()
            }
        }
    }

    private fun download(source: String, target: File) {
        var url = URL(source)
        repeat(4) { count ->
            require(isAllowedDownloadUrl(url.toString()))
            val connection = (url.openConnection() as HttpURLConnection).apply {
                instanceFollowRedirects = false; connectTimeout = 10_000; readTimeout = 30_000
            }
            when (connection.responseCode) {
                in 200..299 -> connection.inputStream.use { input -> target.outputStream().use { input.copyTo(it) } }.also { return }
                301, 302, 303, 307, 308 -> {
                    require(count < 3)
                    url = URL(url, connection.getHeaderField("Location") ?: error("redirect"))
                }
                else -> error("download")
            }
        }
    }

    private fun sha256(file: File): String = MessageDigest.getInstance("SHA-256").let { digest ->
        file.inputStream().use { input -> val buffer = ByteArray(8192); var n = input.read(buffer); while (n >= 0) { if (n > 0) digest.update(buffer, 0, n); n = input.read(buffer) } }
        digest.digest().joinToString("") { "%02x".format(it) }
    }
}

internal class UpdateInstallResultReceiver : android.content.BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        UpdateInstall.clean(context)
        if (intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE) == PackageInstaller.STATUS_PENDING_USER_ACTION) {
            intent.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)?.let {
                it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                context.startActivity(it)
            }
        }
    }
}
