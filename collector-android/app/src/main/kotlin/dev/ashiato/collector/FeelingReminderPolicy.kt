// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId

/** 「その日」の区切り。サーバの日の区切り（S-2 の 1 日）と同じ（ST17 design D10）。 */
val REMINDER_ZONE: ZoneId = ZoneId.of("Asia/Tokyo")

/** 通知の時刻の既定（本人の決定 Q7）。サーバが別の値を返せばそれに従う。 */
@Suppress("MagicNumber")
val DEFAULT_REMINDER_AT: LocalTime = LocalTime.of(21, 0)

/** 毎日この時刻に通知の時刻を読み直す（design D10（仮）/ spec-review R7）。 */
@Suppress("MagicNumber")
val REMINDER_REFRESH_AT: LocalTime = LocalTime.of(12, 0)

/** 予約の窓の幅（design D10（仮））。正確な時刻の権限を求めず、この幅の中で OS に起こしてもらう。 */
const val REMINDER_WINDOW_MS: Long = 15 * 60 * 1000L

/** 受け手が起きたときの結論。 */
sealed interface ReminderAction {
    /** 通知を 1 回出す。 */
    data object Notify : ReminderAction

    /** 今日はもう出さない（通知した・書いてある）。 */
    data object Skip : ReminderAction

    /** まだ時刻の前。この時刻まで待つ。 */
    data class WaitUntil(val at: Instant) : ReminderAction
}

/**
 * 通知の判断（ST17 design D10 / D11）。**純粋** —— 時計も応答も引数で受ける。
 *
 * 持つのは日付と時刻だけで、主観の中身は何も入らない（D11）。
 */
object FeelingReminderPolicy {
    /**
     * @param at 通知の時刻（応答で得た値、無ければ最後に得た値、それも無ければ [DEFAULT_REMINDER_AT]）
     * @param lastNotifiedDate 最後に通知した日
     * @param recorded その日に記録があるか。**null は「得られなかった」** —— 通知する側に倒す（鳴らないより鳴る）
     */
    fun decide(now: Instant, at: LocalTime, lastNotifiedDate: LocalDate?, recorded: Boolean?): ReminderAction {
        val today = dayOf(now)
        val due = at(today, at)
        return when {
            lastNotifiedDate == today -> ReminderAction.Skip
            now.isBefore(due) -> ReminderAction.WaitUntil(due)
            recorded == true -> ReminderAction.Skip
            else -> ReminderAction.Notify
        }
    }

    /** 次に受け手を起こす時刻。今日の時刻がこれからで、まだ通知していなければ今日、でなければ明日。 */
    fun nextReminder(now: Instant, at: LocalTime, lastNotifiedDate: LocalDate?): Instant {
        val today = dayOf(now)
        val due = at(today, at)
        return if (lastNotifiedDate != today && due.isAfter(now)) due else at(today.plusDays(1), at)
    }

    /** 起動の契機で予約を張り直すときの次の時刻。時刻を過ぎていて今日まだなら、すぐ（`now`）。 */
    fun nextReminderOnStart(now: Instant, at: LocalTime, lastNotifiedDate: LocalDate?): Instant {
        val today = dayOf(now)
        if (lastNotifiedDate != today && !at(today, at).isAfter(now)) return now
        return nextReminder(now, at, lastNotifiedDate)
    }

    /** 次の「通知の時刻の読み直し」（12:00）。 */
    fun nextRefresh(now: Instant): Instant {
        val today = dayOf(now)
        val due = at(today, REMINDER_REFRESH_AT)
        return if (due.isAfter(now)) due else at(today.plusDays(1), REMINDER_REFRESH_AT)
    }

    fun dayOf(now: Instant): LocalDate = now.atZone(REMINDER_ZONE).toLocalDate()

    private fun at(date: LocalDate, time: LocalTime): Instant = date.atTime(time).atZone(REMINDER_ZONE).toInstant()
}
