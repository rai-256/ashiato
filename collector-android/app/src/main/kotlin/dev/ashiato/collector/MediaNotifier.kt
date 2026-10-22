// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.app.NotificationCompat

/**
 * 写真と動画を取れていないことを端末で知らせる（Q15 / design D6（仮））。
 *
 * - 出すのは、写真の位置の権限が無いとき・写真と動画へのアクセスが無いとき。**「選んだ写真だけ」では出さない**（生存信号だけ）
 * - 権限が戻ったら消す
 * - 押すとこのアプリの設定（`ACTION_APPLICATION_DETAILS_SETTINGS`）が開く
 *
 * 文言は仮（反転条件: 本人が文言や出し方を変えたいと言ったとき。記録は変わらない）。
 * 通知の権限が無い・通知が切られているときは出せない（取得そのものは止まらず、生存信号が理由を持つ）。
 */
class MediaNotifier(private val context: Context, private val log: (String) -> Unit = {}) {
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    /** いま出している本文。同じ本文を重ねて出し直さない（出し直すたびに鳴らさない） */
    private var shown: String? = null

    /** いまの権限に合わせて、出す・消す。 */
    @Synchronized
    fun update(state: MediaPermissionState) {
        val text = mediaNoticeText(state)
        when {
            text == null -> {
                // shown はプロセスの中にしか無い。落ちて立ち直った後の古い知らせも消すので無条件に消す
                manager.cancel(MEDIA_NOTIFICATION_ID)
                shown = null
            }
            text == shown -> Unit
            canPost() -> post(text)
            else -> log(Telemetry.line("media_notice_skipped", source = null))
        }
    }

    private fun post(text: String) {
        manager.createNotificationChannel(
            NotificationChannel(MEDIA_CHANNEL, "写真と動画の取得", NotificationManager.IMPORTANCE_DEFAULT),
        )
        manager.notify(
            MEDIA_NOTIFICATION_ID,
            NotificationCompat.Builder(context, MEDIA_CHANNEL)
                .setContentTitle(MEDIA_NOTICE_TITLE)
                .setContentText(text)
                .setSmallIcon(android.R.drawable.stat_notify_error)
                .setContentIntent(appSettingsPendingIntent(context))
                .setAutoCancel(false)
                .setOnlyAlertOnce(true)
                .build(),
        )
        shown = text
    }

    private fun canPost(): Boolean {
        val permitted = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
        val channelOn = manager.getNotificationChannel(MEDIA_CHANNEL)?.importance != NotificationManager.IMPORTANCE_NONE
        return permitted && manager.areNotificationsEnabled() && channelOn
    }

    companion object {
        const val MEDIA_CHANNEL = "media-capture"

        /** 既存の常駐（1）・保持（2）・途切れ（3）と重ならない */
        const val MEDIA_NOTIFICATION_ID = 4
    }
}

const val MEDIA_NOTICE_TITLE: String = "写真と動画を取れていません"

/** 知らせの本文。出さないとき（権限がそろっている・「選んだ写真だけ」）は null。 */
fun mediaNoticeText(state: MediaPermissionState): String? = when {
    state.access == MediaAccess.NONE -> "写真と動画へのアクセスを許可するまで、写真と動画を取らずに待っています。"
    !state.location -> "写真の位置へのアクセスを許可するまで、写真と動画を取らずに待っています。"
    else -> null
}

/** このアプリの設定を開く指示（知らせを押したとき）。 */
fun appSettingsIntent(context: Context): Intent =
    Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null))
        // 通知から開く道はアプリの画面の外から始まるので、自分の入れ物が要る
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)

private fun appSettingsPendingIntent(context: Context): PendingIntent = PendingIntent.getActivity(
    context,
    MediaNotifier.MEDIA_NOTIFICATION_ID,
    appSettingsIntent(context),
    PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
)
