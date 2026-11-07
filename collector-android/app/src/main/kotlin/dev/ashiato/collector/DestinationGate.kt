package dev.ashiato.collector

import android.content.SharedPreferences
import java.net.URI
import java.time.Instant

/** 保留中の未送信を設定画面へ渡す値。時刻は端末の壁時計で表示する。 */
data class DestinationBacklog(
    val count: Int,
    val oldest: Instant?,
    val newest: Instant?,
) {
    companion object {
        val EMPTY = DestinationBacklog(0, null, null)
    }
}

/**
 * 設定の保存先と、本人が承認した保存先を分けて持つ門。
 *
 * 資格情報は比較せず、保存先の正規化した文字列だけを比較する。保留の状態は
 * SharedPreferences に置くので、サービスの再起動後も送信を止めたままにできる。
 */
class DestinationGate(private val prefs: SharedPreferences) {
    private var currentUrl: String?
        get() = prefs.getString(PENDING_URL, null)
        set(value) = editValue(PENDING_URL, value)

    val approved: String?
        get() = prefs.getString(APPROVED_URL, null)

    val blocked: Boolean
        get() = prefs.getBoolean(BLOCKED, false)

    val maySend: Boolean get() = !blocked

    val headline: String?
        get() = if (blocked) "送り先の確認を待っています" else null

    fun notificationDue(nowMs: Long = System.currentTimeMillis()): Boolean {
        val previous = prefs.getLong(NOTIFIED_AT, Long.MIN_VALUE)
        return previous == Long.MIN_VALUE || nowMs - previous >= NOTIFICATION_INTERVAL_MS
    }

    fun markNotified(nowMs: Long = System.currentTimeMillis()) {
        prefs.edit().putLong(NOTIFIED_AT, nowMs).apply()
    }

    val summary: DestinationBacklog
        get() = DestinationBacklog(
            prefs.getInt(PENDING_COUNT, 0),
            instant(PENDING_OLDEST),
            instant(PENDING_NEWEST),
        )

    /** 保存された設定を現在の未送信と突き合わせ、保留なら true を返す。 */
    @Synchronized
    fun reconcile(url: String, backlog: DestinationBacklog): Boolean {
        val normalized = key(url)
        if (normalized == approved || backlog.count == 0) {
            approve(normalized)
            return false
        }
        currentUrl = normalized
        put(PENDING_COUNT, backlog.count)
        put(PENDING_OLDEST, backlog.oldest?.toEpochMilli())
        put(PENDING_NEWEST, backlog.newest?.toEpochMilli())
        put(BLOCKED, true)
        return true
    }

    /**
     * 設定の保存と同じ編集に、承認した送り先と違えば保留を書き込む（review/code.md R1）。
     *
     * 送り先は要求ごとに読む（D11）ので、保存の commit と保留が別々だと、その間の送る契機が
     * 新しい送り先へ未送信を送る。件数はまだ数えていないので持たない —— サービスの [reconcile] が
     * 数えて、未送信が 0 件なら解き、あれば件数と期間を書く。
     */
    fun holdOnSave(edit: SharedPreferences.Editor, url: String) {
        val normalized = key(url)
        if (normalized == approved) return
        edit.putString(PENDING_URL, normalized)
        edit.remove(PENDING_COUNT)
            .remove(PENDING_OLDEST)
            .remove(PENDING_NEWEST)
            .remove(NOTIFIED_AT)
            .putBoolean(BLOCKED, true)
    }

    /** -P など、既存の承認済み扱いを設定するための内部口。 */
    @Synchronized
    fun approve(url: String) {
        put(APPROVED_URL, key(url))
        clearPending()
    }

    /** 設定画面の「この送り先へ送る」。 */
    @Synchronized
    fun approvePending() {
        val pending = currentUrl ?: return
        approve(pending)
    }

    private fun clearPending() {
        prefs.edit()
            .remove(PENDING_URL)
            .remove(PENDING_COUNT)
            .remove(PENDING_OLDEST)
            .remove(PENDING_NEWEST)
            // 次の保留でも最初の知らせを出す（review/code.md R5）
            .remove(NOTIFIED_AT)
            .putBoolean(BLOCKED, false)
            .apply()
    }

    private fun instant(key: String): Instant? = prefs.getLong(key, Long.MIN_VALUE)
        .takeIf { it != Long.MIN_VALUE }
        ?.let(Instant::ofEpochMilli)

    private fun editValue(key: String, value: String?) {
        val edit = prefs.edit()
        if (value == null) edit.remove(key) else edit.putString(key, value)
        edit.apply()
    }

    private fun put(key: String, value: Any?) {
        val edit = prefs.edit()
        when (value) {
            null -> edit.remove(key)
            is Int -> edit.putInt(key, value)
            is Long -> edit.putLong(key, value)
            is Boolean -> edit.putBoolean(key, value)
            is String -> edit.putString(key, value)
        }
        edit.apply()
    }

    companion object {
        const val APPROVED_URL = "approved_url"
        const val PENDING_URL = "pending_url"
        private const val BLOCKED = "destination_blocked"
        private const val PENDING_COUNT = "pending_count"
        private const val PENDING_OLDEST = "pending_oldest_ms"
        private const val PENDING_NEWEST = "pending_newest_ms"
        private const val NOTIFIED_AT = "destination_notified_at_ms"
        private const val NOTIFICATION_INTERVAL_MS = 24L * 60L * 60L * 1000L

        /**
         * 比べる鍵。[normalize] で読めない送り先も、そのままの文字で保留・承認・解除が回る（読めない送り先で
         * 保留だけが立ち、「この送り先へ送る」でも解けずに送信が止まり続けない。review/code.md R41）。
         */
        fun key(url: String): String = normalize(url) ?: url.trim()

        /** scheme/host は小文字、既定 port と path 末尾の slash は取り除く。 */
        fun normalize(url: String): String? = runCatching {
            val uri = URI(url.trim())
            val scheme = uri.scheme?.lowercase() ?: return@runCatching null
            val host = uri.host?.lowercase() ?: return@runCatching null
            val port = when {
                (scheme == "https" && uri.port == 443) || (scheme == "http" && uri.port == 80) -> -1
                else -> uri.port
            }
            val path = uri.path.orEmpty().trimEnd('/').ifEmpty { "/" }
            URI(scheme, uri.userInfo, host, port, path, uri.query, uri.fragment).toString()
                .removeSuffix("/")
        }.getOrNull()
    }
}
