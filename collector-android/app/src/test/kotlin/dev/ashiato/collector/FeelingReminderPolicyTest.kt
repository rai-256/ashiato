// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import org.junit.Assert.assertEquals
import org.junit.Test
import java.time.Instant
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId

/** 通知の判断（ST17 design D10）。純粋なので時計と応答を差し替えて撃つ。 */
class FeelingReminderPolicyTest {
    private val tokyo = ZoneId.of("Asia/Tokyo")
    private val day = LocalDate.of(2026, 10, 7)
    private val nine = LocalTime.of(21, 0)

    private fun at(date: LocalDate, h: Int, m: Int = 0): Instant = date.atTime(h, m).atZone(tokyo).toInstant()

    @Test
    fun namedConstantsArePinned() {
        assertEquals(LocalTime.of(21, 0), DEFAULT_REMINDER_AT)
        assertEquals(LocalTime.of(12, 0), REMINDER_REFRESH_AT)
        assertEquals(15 * 60 * 1000L, REMINDER_WINDOW_MS)
        assertEquals(tokyo, REMINDER_ZONE)
        assertEquals(10_000, HttpReminderSource.TIMEOUT_MS)
    }

    @Test
    fun `書いていない日は通知の時刻に通知が出る`() {
        // Scenario: 書いていない日は通知の時刻に通知が出る
        assertEquals(ReminderAction.Notify, FeelingReminderPolicy.decide(at(day, 21), nine, null, false))
    }

    @Test
    fun `書いた日は通知が出ない`() {
        // Scenario: 書いた日は通知が出ない
        assertEquals(ReminderAction.Skip, FeelingReminderPolicy.decide(at(day, 21), nine, null, true))
    }

    @Test
    fun `網に届かないときは通知が出る`() {
        // Scenario: 網に届かないときは通知が出る
        assertEquals(ReminderAction.Notify, FeelingReminderPolicy.decide(at(day, 21, 5), nine, null, null))
    }

    @Test
    fun `同じ日に 2 回は通知しない`() {
        // Scenario: 同じ日に 2 回は通知しない
        assertEquals(ReminderAction.Skip, FeelingReminderPolicy.decide(at(day, 22), nine, day, false))
        // 翌日にはまた出る
        assertEquals(
            ReminderAction.Notify,
            FeelingReminderPolicy.decide(at(day.plusDays(1), 21), nine, day, false),
        )
    }

    @Test
    fun `通知の時刻より前には通知しない`() {
        // Scenario: 通知の時刻より前には通知しない
        val before = at(day, 20, 59)
        assertEquals(ReminderAction.WaitUntil(at(day, 21)), FeelingReminderPolicy.decide(before, nine, null, false))
        // 21:00 からの窓で起こす予約が残る
        assertEquals(at(day, 21), FeelingReminderPolicy.nextReminder(before, nine, null))
    }

    @Test
    fun `通知の時刻はサーバの設定に従う`() {
        // Scenario: 通知の時刻はサーバの設定に従う
        val late = LocalTime.of(22, 30)
        assertEquals(
            ReminderAction.WaitUntil(at(day, 22, 30)),
            FeelingReminderPolicy.decide(at(day, 21), late, null, false),
        )
        assertEquals(ReminderAction.Notify, FeelingReminderPolicy.decide(at(day, 22, 30), late, null, false))
        assertEquals(at(day, 22, 30), FeelingReminderPolicy.nextReminder(at(day, 12), late, null))
    }

    @Test
    fun `午前に早めた通知の時刻はその日から効く`() {
        // Scenario: 午前に早めた通知の時刻はその日から効く
        val early = LocalTime.of(20, 0)
        val refresh = at(day, 12)
        // 12:00 の読み直しで 20:00 を得ると、その日の予約は 20:00（21:00 まで待たない）
        assertEquals(at(day, 20), FeelingReminderPolicy.nextReminder(refresh, early, null))
        assertEquals(ReminderAction.WaitUntil(at(day, 20)), FeelingReminderPolicy.decide(refresh, early, null, false))
        assertEquals(ReminderAction.Notify, FeelingReminderPolicy.decide(at(day, 20), early, null, false))
        // 読み直しは毎日 12:00
        assertEquals(at(day, 12), FeelingReminderPolicy.nextRefresh(at(day, 9)))
        assertEquals(at(day.plusDays(1), 12), FeelingReminderPolicy.nextRefresh(at(day, 12)))
    }

    @Test
    fun `届かないときは最後に得た通知の時刻を使う`() {
        // Scenario: 届かないときは最後に得た通知の時刻を使う
        // 受け手は応答が無ければ最後に得た 22:30 を `at` に渡す。判断はその 22:30 で決まる
        val last = LocalTime.of(22, 30)
        assertEquals(
            ReminderAction.WaitUntil(at(day, 22, 30)),
            FeelingReminderPolicy.decide(at(day, 21, 30), last, null, null),
        )
        assertEquals(ReminderAction.Notify, FeelingReminderPolicy.decide(at(day, 22, 31), last, null, null))
    }

    @Test
    fun `日の区切りは東京で、通知した日の翌日の予約は翌日の時刻`() {
        val afterNotify = at(day, 21, 3)
        assertEquals(at(day.plusDays(1), 21), FeelingReminderPolicy.nextReminder(afterNotify, nine, day))
        // 起動の契機: 時刻を過ぎてまだ通知していなければすぐ、通知済みなら明日
        assertEquals(afterNotify, FeelingReminderPolicy.nextReminderOnStart(afterNotify, nine, null))
        assertEquals(at(day.plusDays(1), 21), FeelingReminderPolicy.nextReminderOnStart(afterNotify, nine, day))
        assertEquals(at(day, 21), FeelingReminderPolicy.nextReminderOnStart(at(day, 8), nine, null))
    }
}
