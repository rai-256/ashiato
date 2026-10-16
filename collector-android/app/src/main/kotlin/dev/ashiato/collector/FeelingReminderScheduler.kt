// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import java.time.Instant

/** 予約の出し先（受け手の判断を試験で差し替えるための口）。 */
interface ReminderAlarms {
    fun scheduleReminder(at: Instant)

    fun scheduleRefresh(at: Instant)
}

/**
 * `AlarmManager.setWindow` で次の起床を予約する（ST17 design D10（仮））。
 *
 * **正確な時刻の権限は求めない**（C8）—— 窓は [REMINDER_WINDOW_MS]。通知は 15 分以内の遅れを許す。
 * 通知の時刻の予約と、12:00 の読み直しの予約は別の `PendingIntent`（動作が違う）。
 */
class FeelingReminderScheduler(private val context: Context) : ReminderAlarms {
    private val alarms: AlarmManager get() = context.getSystemService(AlarmManager::class.java)

    override fun scheduleReminder(at: Instant) = set(FeelingReminderReceiver.ACTION_REMINDER, REQUEST_REMINDER, at)

    override fun scheduleRefresh(at: Instant) = set(FeelingReminderReceiver.ACTION_REFRESH, REQUEST_REFRESH, at)

    private fun set(action: String, requestCode: Int, at: Instant) {
        val intent = Intent(context, FeelingReminderReceiver::class.java).setAction(action)
        val pending = PendingIntent.getBroadcast(
            context,
            requestCode,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        alarms.setWindow(AlarmManager.RTC_WAKEUP, at.toEpochMilli(), REMINDER_WINDOW_MS, pending)
    }

    private companion object {
        const val REQUEST_REMINDER = 1701
        const val REQUEST_REFRESH = 1702
    }
}
