// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.net.Uri
import android.os.Build
import android.provider.Settings
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.Executors

@Suppress("TooManyFunctions") // 落とす・照らす・入れる・失敗の行・開始の印を 1 か所に置く（ST58 D21）
internal object UpdateInstall {
    private const val GITHUB_PREFIX = "https://github.com/rai-256/ashiato/releases/download/"
    private const val ASSET_HOST = "release-assets.githubusercontent.com"
    private const val APK_PREFIX = "ashiato-collector-"
    private const val APK_SUFFIX = ".apk"
    const val EXTRA_START = "dev.ashiato.collector.UPDATE_START"
    private const val EXTRA_START_TOKEN = "dev.ashiato.collector.UPDATE_START_TOKEN"
    private const val START_PREFS = "update-start"
    private const val START_TOKEN = "token"
    const val ACTION_RESULT = "dev.ashiato.collector.UPDATE_RESULT"
    const val UPDATE_DIR = "update"
    private const val FAILURE_PREFS = "update-install"
    private const val FAILURE_LINE = "failure-line"
    internal const val UNKNOWN_SOURCES_LINE = "許可した後に、もう一度押してください"
    private const val CONNECT_TIMEOUT_MS = 10_000
    private const val READ_TIMEOUT_MS = 30_000
    private const val MAX_FETCHES = 4
    private const val HTTP_TEMPORARY_REDIRECT = 307
    private const val HTTP_PERMANENT_REDIRECT = 308
    private const val DIGEST_BUFFER_BYTES = 8192
    private val SHA256_HEX = Regex("[0-9a-fA-F]{64}")

    data class Asset(val version: String, val url: String, val sha256: String, val name: String)

    fun chooseAsset(installed: String, body: String): Asset? {
        val latest = UpdateCheck.choose("android", installed, body).latest ?: return null
        val root = Json.parseToJsonElement(body)
        val releases = if (root is JsonArray) root else JsonArray(listOf(root))
        return releases.asSequence().flatMap { release ->
            val obj = release.jsonObject
            if (obj["draft"]?.jsonPrimitive?.content == "true") {
                emptySequence()
            } else {
                obj["assets"]?.jsonArray.orEmpty().asSequence()
            }
        }.mapNotNull { asset ->
            val obj = asset.jsonObject
            val name = obj["name"]?.jsonPrimitive?.content ?: return@mapNotNull null
            val url = obj["browser_download_url"]?.jsonPrimitive?.content ?: return@mapNotNull null
            val digest = obj["digest"]?.jsonPrimitive?.content ?: return@mapNotNull null
            val sha = digest.removePrefix("sha256:")
            val prefix = if (installed.contains("-dev.")) "ashiato-collector-dev-" else APK_PREFIX
            val wellFormedDigest = digest.startsWith("sha256:") && SHA256_HEX.matches(sha)
            val expected = name == "$prefix$latest$APK_SUFFIX" && isAllowedDownloadUrl(url)
            if (expected && wellFormedDigest) Asset(latest, url, sha.lowercase(), name) else null
        }.firstOrNull()
    }

    fun isAllowedDownloadUrl(value: String): Boolean =
        value.startsWith(GITHUB_PREFIX) || runCatching {
            URL(value).host == ASSET_HOST && URL(value).protocol == "https"
        }.getOrDefault(false)

    fun updateDirectory(context: Context): File = File(context.noBackupFilesDir, UPDATE_DIR)

    internal fun installFailureLine(reason: String): String = "入れられませんでした（$reason）"

    internal fun failurePreferences(context: Context): android.content.SharedPreferences =
        context.getSharedPreferences(FAILURE_PREFS, Context.MODE_PRIVATE)

    internal fun failureLine(context: Context): String? =
        failurePreferences(context).getString(FAILURE_LINE, null)

    /**
     * 画面に出す失敗の行。行は「更新あり」の行に付くので、新しい版が無くなった（入れ終えた）・
     * 更新の確認を切った後は出さずに消す（閉じられない画面に古い行を残さない。review/code.md R50・R63）。
     */
    internal fun visibleFailureLine(context: Context, updateOffered: Boolean): String? {
        if (!updateOffered) {
            if (failureLine(context) != null) clearFailure(context)
            return null
        }
        return failureLine(context)
    }

    internal fun recordFailure(context: Context, line: String) {
        context.getSharedPreferences(FAILURE_PREFS, Context.MODE_PRIVATE)
            .edit().putString(FAILURE_LINE, line).apply()
    }

    private fun clearFailure(context: Context) {
        context.getSharedPreferences(FAILURE_PREFS, Context.MODE_PRIVATE)
            .edit().remove(FAILURE_LINE).apply()
    }

    /** 「提供元不明」の許可だけは、設定から戻った時点で解消済みである。 */
    internal fun clearResolvedUnknownSourcesFailure(context: Context, permitted: Boolean) {
        if (permitted && failureLine(context) == UNKNOWN_SOURCES_LINE) clearFailure(context)
    }

    internal fun updateStartIntent(context: Context): Intent {
        val token = UUID.randomUUID().toString()
        context.getSharedPreferences(START_PREFS, Context.MODE_PRIVATE)
            .edit().putString(START_TOKEN, token).apply()
        return Intent(context, MainActivity::class.java)
            .putExtra(EXTRA_START, true)
            .putExtra(EXTRA_START_TOKEN, token)
    }

    /** Accept the one-time token only when it was minted for our notification. */
    internal fun consumeUpdateStart(context: Context, intent: Intent?): Boolean {
        val token = intent?.takeIf { it.getBooleanExtra(EXTRA_START, false) }?.getStringExtra(EXTRA_START_TOKEN)
        val prefs = context.getSharedPreferences(START_PREFS, Context.MODE_PRIVATE)
        val accepted = token != null && token == prefs.getString(START_TOKEN, null)
        if (accepted) prefs.edit().remove(START_TOKEN).apply()
        return accepted
    }

    internal fun unknownSourcesIntent(context: Context): Intent =
        Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${context.packageName}"))

    internal fun downloadHeaders(): Map<String, String> = mapOf(
        "User-Agent" to UpdateCheck.USER_AGENT,
        "Accept" to UpdateCheck.ACCEPT,
    )

    fun clean(context: Context) {
        // PackageReplacedReceiver writes this marker before either the result
        // receiver or the activity can clean downloads.  It is consumed by the
        // service startup path, not by the installer cleanup path.
        updateDirectory(context).listFiles()?.forEach {
            if (it.isFile && it.name != "replaced-start") it.delete()
        }
    }

    internal fun verifyDigest(file: File, expected: String): Boolean = sha256(file) == expected.lowercase()

    fun start(context: Context) {
        clearResolvedUnknownSourcesFailure(
            context,
            Build.VERSION.SDK_INT < Build.VERSION_CODES.O || context.packageManager.canRequestPackageInstalls(),
        )
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !context.packageManager.canRequestPackageInstalls()) {
            recordFailure(context, UNKNOWN_SOURCES_LINE)
            context.startActivity(unknownSourcesIntent(context).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            return
        }
        Executors.newSingleThreadExecutor().execute {
            try {
                val nightly = BuildConfig.VERSION_NAME.contains("-dev.")
                val path = if (nightly) UpdateCheck.NIGHTLY_PATH else UpdateCheck.URL_PATH
                val url = URL(BuildConfig.UPDATE_CHECK_URL + path)
                val connection = (url.openConnection() as HttpURLConnection).apply {
                    connectTimeout = CONNECT_TIMEOUT_MS
                    readTimeout = READ_TIMEOUT_MS
                    downloadHeaders().forEach { (name, value) -> setRequestProperty(name, value) }
                }
                val body = connection.inputStream.use { it.readBytes().toString(Charsets.UTF_8) }
                val asset = chooseAsset(BuildConfig.VERSION_NAME, body) ?: run {
                    recordFailure(context, installFailureLine("配布物を確認できません"))
                    return@execute
                }
                install(context, asset)
            } catch (_: Exception) {
                clean(context)
                recordFailure(context, installFailureLine("配布物を落とせません"))
            }
        }
    }

    fun install(context: Context, asset: Asset) {
        Executors.newSingleThreadExecutor().execute {
            val file = File(updateDirectory(context), asset.name)
            try {
                file.parentFile?.mkdirs()
                download(asset.url, file)
                if (!verifyDigest(file, asset.sha256)) {
                    file.delete()
                    recordFailure(context, installFailureLine("指紋が一致しません"))
                    return@execute
                }
                val installer = context.packageManager.packageInstaller
                val sessionId = installer.createSession(
                    PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL)
                )
                installer.openSession(sessionId).use { session ->
                    file.inputStream().use { input ->
                        session.openWrite("base.apk", 0, file.length()).use { output ->
                            input.copyTo(output)
                            session.fsync(output)
                        }
                    }
                    val intent = Intent(context, UpdateInstallResultReceiver::class.java).setAction(ACTION_RESULT)
                    val pending = PendingIntent.getBroadcast(
                        context,
                        sessionId,
                        intent,
                        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE
                    )
                    session.commit(pending.intentSender)
                }
                clearFailure(context)
            } catch (_: Exception) {
                file.delete()
                recordFailure(context, installFailureLine("配布物を入れられません"))
            }
        }
    }

    private fun download(source: String, target: File) {
        var url = URL(source)
        repeat(MAX_FETCHES) { count ->
            require(isAllowedDownloadUrl(url.toString()))
            val connection = (url.openConnection() as HttpURLConnection).apply {
                instanceFollowRedirects = false
                connectTimeout = CONNECT_TIMEOUT_MS
                readTimeout = READ_TIMEOUT_MS
                downloadHeaders().forEach { (name, value) -> setRequestProperty(name, value) }
            }
            when (connection.responseCode) {
                in HttpURLConnection.HTTP_OK until HttpURLConnection.HTTP_MULT_CHOICE -> {
                    connection.inputStream.use { input -> target.outputStream().use { input.copyTo(it) } }
                    return
                }
                HttpURLConnection.HTTP_MOVED_PERM, HttpURLConnection.HTTP_MOVED_TEMP, HttpURLConnection.HTTP_SEE_OTHER,
                HTTP_TEMPORARY_REDIRECT, HTTP_PERMANENT_REDIRECT,
                -> {
                    require(count < MAX_FETCHES - 1)
                    url = URL(url, connection.getHeaderField("Location") ?: error("redirect"))
                }
                else -> error("download")
            }
        }
    }

    private fun sha256(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(DIGEST_BUFFER_BYTES)
            generateSequence { input.read(buffer).takeIf { it >= 0 } }.forEach { digest.update(buffer, 0, it) }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }
}

internal class UpdateInstallResultReceiver : android.content.BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        UpdateInstall.clean(context)
        val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)
        if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
            intent.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)?.let {
                it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                context.startActivity(it)
            }
        }
    }
}
