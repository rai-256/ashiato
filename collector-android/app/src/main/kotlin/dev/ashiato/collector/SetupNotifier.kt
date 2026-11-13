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

/** 資格情報の拒否と、設定後に一度も応答がない状態を永続的に数える通知。 */
class SetupNotifier(
    private val context: Context,
    private val prefs: SharedPreferences = context.getSharedPreferences(Config.SETUP_PREFS, Context.MODE_PRIVATE),
    private val now: () -> Long = System::currentTimeMillis,
    private val log: (String) -> Unit = {},
) {
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    fun setupSaved() {
        prefs.edit()
            .putLong(SETUP_AT_KEY, now())
            .remove(AUTH_SINCE_KEY)
            .remove(AUTH_NOTIFIED_AT_KEY)
            .remove(LAST_RESPONSE_AT_KEY)
            .remove(UNREACHABLE_NOTIFIED_AT_KEY)
            .apply()
        manager.cancel(AUTH_NOTIFICATION_ID)
        manager.cancel(UNREACHABLE_NOTIFICATION_ID)
        // 未設定の知らせは設定したら消す（review/code.md R7）
        prefs.edit().remove(UNSET_NOTIFIED_AT_KEY).apply()
        manager.cancel(LocationService.SETUP_NOTIFICATION_ID)
    }

    /** 送信または問い合わせの結果を記録する。401 以外の応答は状態符号を受けた印になる。 */
    fun observe(outcome: Outcome) {
        when (outcome) {
            is Outcome.Unreachable -> Unit
            is Outcome.Responded -> {
                val edit = prefs.edit().putLong(LAST_RESPONSE_AT_KEY, now())
                when {
                    outcome.status == HTTP_UNAUTHORIZED && !prefs.contains(AUTH_SINCE_KEY) ->
                        edit.putLong(AUTH_SINCE_KEY, now())
                    outcome.status !in 500..599 && outcome.status != HTTP_UNAUTHORIZED -> {
                        edit.remove(AUTH_SINCE_KEY).remove(AUTH_NOTIFIED_AT_KEY)
                        manager.cancel(AUTH_NOTIFICATION_ID)
                    }
                }
                edit.apply()
            }
        }
    }

    fun update() {
        val setupAt = prefs.getLong(SETUP_AT_KEY, NONE)
        val current = now()
        val authSince = prefs.getLong(AUTH_SINCE_KEY, NONE)
        if (authSince != NONE && current - authSince >= AUTH_AFTER_MS && due(AUTH_NOTIFIED_AT_KEY, current)) {
            if (post(AUTH_NOTIFICATION_ID, AUTH_TEXT)) prefs.edit().putLong(AUTH_NOTIFIED_AT_KEY, current).apply()
        }
        val neverResponded = !prefs.contains(LAST_RESPONSE_AT_KEY)
        if (setupAt != NONE && neverResponded && current - setupAt >= UNREACHABLE_AFTER_MS && due(UNREACHABLE_NOTIFIED_AT_KEY, current)) {
            if (post(UNREACHABLE_NOTIFICATION_ID, UNREACHABLE_TEXT)) {
                prefs.edit().putLong(UNREACHABLE_NOTIFIED_AT_KEY, current).apply()
            }
        }
    }

    private fun due(key: String, current: Long): Boolean {
        val last = prefs.getLong(key, NONE)
        return last == NONE || current - last >= REPEAT_MS
    }

    private fun post(id: Int, text: String): Boolean {
        if (!canPostOnSetupChannel()) {
            log(Telemetry.line("setup_notice_skipped", source = null))
            return false
        }
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, "送り先の設定", NotificationManager.IMPORTANCE_DEFAULT)
        )
        manager.notify(
            id,
            NotificationCompat.Builder(context, CHANNEL)
                .setContentTitle("あしあと。")
                .setContentText(text)
                .setSmallIcon(android.R.drawable.ic_menu_manage)
                .setContentIntent(
                    PendingIntent.getActivity(
                        context,
                        id,
                        Intent(context, SetupActivity::class.java),
                        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
                    )
                )
                .setAutoCancel(true)
                .build(),
        )
        return true
    }

    /** 更新通知と同じ権限・チャンネル確認を、setup チャンネルに対して行う。 */
    private fun canPostOnSetupChannel(): Boolean = UpdateCheck.canPost(context, manager, CHANNEL)

    companion object {
        const val CHANNEL = LocationService.SETUP_CHANNEL
        const val AUTH_NOTIFICATION_ID = 7
        const val UNREACHABLE_NOTIFICATION_ID = 8
        const val AUTH_SINCE_KEY = "setup_auth_since_ms"
        const val AUTH_NOTIFIED_AT_KEY = "setup_auth_notified_at_ms"
        const val LAST_RESPONSE_AT_KEY = "setup_last_response_at_ms"
        const val SETUP_AT_KEY = "setup_saved_at_ms"

        /** 「送り先が設定されていません」を最後に出した時刻（[LocationService.refreshSetupNotice]） */
        const val UNSET_NOTIFIED_AT_KEY = "setup_unset_notified_at_ms"
        private const val UNREACHABLE_NOTIFIED_AT_KEY = "setup_unreachable_notified_at_ms"
        private const val HTTP_UNAUTHORIZED = 401
        private const val AUTH_AFTER_MS = 60 * 60 * 1000L
        private const val UNREACHABLE_AFTER_MS = 24 * 60 * 60 * 1000L
        private const val REPEAT_MS = UNREACHABLE_AFTER_MS
        private const val NONE = Long.MIN_VALUE
        const val AUTH_TEXT = "サーバに断られています。資格情報か送り先が違います。設定し直してください"
        const val UNREACHABLE_TEXT = "送り先に届きません。設定を確かめてください"
    }
}
