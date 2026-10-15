// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import java.time.LocalDate

/** 通知の出し先（受け手の判断を試験で差し替えるための口）。出せたら true。 */
fun interface ReminderNotifier {
    fun post(date: LocalDate): Boolean
}

/**
 * 「今日はどうでしたか」の通知（ST17 design D10 / D11）。
 *
 * **題と本文は定数。** 記録の値・件数・前の日の値を組み込まない（C9 / D11）。
 * 押すと Web の主観入力 `<webUrl>/#/feel/<その日>` を開く。**`webUrl` が空のビルドは `Intent` を付けない**
 * （通知は出る。押しても何も開かない。gradle では止めない）。
 */
class FeelingNotification(private val context: Context, private val webUrl: String) : ReminderNotifier {
    override fun post(date: LocalDate): Boolean {
        val manager = context.getSystemService(NotificationManager::class.java)
        if (!mayPost(manager)) return false
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, CHANNEL_NAME, NotificationManager.IMPORTANCE_DEFAULT),
        )
        val builder = Notification.Builder(context, CHANNEL)
            .setContentTitle(TITLE)
            .setContentText(TEXT)
            .setSmallIcon(android.R.drawable.ic_menu_edit)
            .setAutoCancel(true)
        openIntent(date)?.let { builder.setContentIntent(it) }
        manager.notify(NOTIFICATION_ID, builder.build())
        return true
    }

    private fun mayPost(manager: NotificationManager): Boolean {
        val permitted = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
        return permitted && manager.areNotificationsEnabled()
    }

    private fun openIntent(date: LocalDate): PendingIntent? {
        if (webUrl.isBlank()) return null
        val view = Intent(Intent.ACTION_VIEW, Uri.parse(feelUrl(webUrl, date)))
        return PendingIntent.getActivity(
            context,
            NOTIFICATION_ID,
            view,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    companion object {
        const val CHANNEL = "feeling"
        const val CHANNEL_NAME = "今日の気分"
        const val NOTIFICATION_ID = 1701
        const val TITLE = "今日はどうでしたか"
        const val TEXT = "気分を残す"

        /** Web の主観入力の画面（S-3）。 */
        fun feelUrl(webUrl: String, date: LocalDate): String = "${webUrl.trimEnd('/')}/#/feel/$date"
    }
}
