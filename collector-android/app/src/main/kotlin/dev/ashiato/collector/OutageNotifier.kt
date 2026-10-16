// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/**
 * 途切れの通知を端末に出す（ST14 / design D12・D14（仮））。
 *
 * 専用のチャネル `outage`（常駐の `location` とも保持の `retention` とも違う）。
 * 音を鳴らさない差し替えは `setSilent(true)`。`setOnlyAlertOnce` は使わない
 * （新しく途切れたソースが増えたときに鳴らなくなる）。
 * 押しても画面を開かない（`Intent` を付けない）。**記録の中身は載せない**（C5）——
 * 本文は表示名と最後に生きていた日時だけ。
 */
class OutageNotifier(private val context: Context, private val log: (String) -> Unit = {}) {
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    /**
     * 出せたら true。権限が無い・通知が切られている・このチャネルだけが切られているときは出さずに false
     * （呼び出し側は覚えを書き換えない。review/code.md R12・R18）。
     */
    fun show(outages: List<Outage>, sound: Boolean): Boolean {
        if (!canPost()) {
            log(Telemetry.line("outage_notice_skipped", source = null))
            return false
        }
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, "収集の途切れ", NotificationManager.IMPORTANCE_DEFAULT),
        )
        val notification = NotificationCompat.Builder(context, CHANNEL)
            .setContentTitle(TITLE)
            .setContentText(body(outages))
            .setSmallIcon(android.R.drawable.stat_notify_error)
            .setSilent(!sound)
            .build()
        manager.notify(OUTAGE_NOTIFICATION_ID, notification)
        return true
    }

    /** 黙って消す。 */
    fun dismiss() {
        manager.cancel(OUTAGE_NOTIFICATION_ID)
    }

    private fun canPost(): Boolean {
        val permitted = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
        // 本人が設定で「収集の途切れ」だけを切ると、アプリが作り直しても戻らず、notify しても出ない（review/code.md R18）
        val channelOn = manager.getNotificationChannel(CHANNEL)?.importance != NotificationManager.IMPORTANCE_NONE
        return permitted && manager.areNotificationsEnabled() && channelOn
    }

    companion object {
        const val CHANNEL = "outage"

        /** 既存の常駐（1）・保持（2）と重ならない */
        const val OUTAGE_NOTIFICATION_ID = 3
        const val TITLE = "収集が途切れています"
        private val DAY_ZONE: ZoneId = ZoneId.of("Asia/Tokyo")
        private val STAMP: DateTimeFormatter = DateTimeFormatter.ofPattern("MM/dd HH:mm")

        /** `位置（最後 10/12 08:15）、写真（最後 10/11 22:00）` */
        fun body(outages: List<Outage>): String = outages.joinToString("、") {
            "${it.displayName}（最後 ${STAMP.format(it.lastAliveAt.atZone(DAY_ZONE))}）"
        }
    }
}
