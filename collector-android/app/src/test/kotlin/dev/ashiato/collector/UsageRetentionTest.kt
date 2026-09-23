// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 見込みの保持の下限（tasks 2.2 / design D4 / spec レビュー R3）。
 *
 * 期待値は**この試験が自分で書く** —— 実装の定数を読み直して比べると、
 * 両方が同じだけずれても緑になる（`UsageStatsDatabase.prune()` から独立に写した値を置く）。
 *
 * **この値で問い合わせの窓を切り詰めない。** 10 日は API から読めない見込みなので、
 * 切り詰めると取得元にまだ残っているイベントを飛ばす。使うのは gap の判定だけ。
 */
class UsageRetentionTest {
    private val now = Instant.parse("2026-05-20T12:00:00Z")

    /** 20 日前から問い合わせても「窓の形では断られない」ように、偽物の時計を今に合わせる */
    private fun sourceAt(vararg events: UsageEventSnapshot) =
        FakeUsageSource(storedEvents = events.toList()).also { it.now = now }

    /** `mCal.addDays(-10)`（`UsageStatsDatabase.prune()` が日ごとの箱を落とす境目）。 */
    @Test
    fun `イベントの見込みの下限は 10 日前`() {
        assertEquals(Instant.parse("2026-05-10T12:00:00Z"), UsageRetention.eventsFloor(now).gapEndWhenNothingReturned())
    }

    /**
     * 粒度ごとの下限。`UnixCalendar` は暦を知らない（1 か月 = 30 日 / 1 年 = 365 日の固定）ので、
     * 期待値も固定の日数で書く。
     */
    @Test
    fun `集計の見込みの下限は粒度ごとに違う`() {
        val floors = UsageGranularity.entries.associateWith { UsageRetention.rollupFloor(now, it).gapEndWhenNothingReturned() }
        assertEquals(
            mapOf(
                // 日ごとの集計はイベントと同じ箱（INTERVAL_DAILY）なので同じ 10 日
                UsageGranularity.DAILY to Instant.parse("2026-05-10T12:00:00Z"),
                // mCal.addWeeks(-4) = 28 日
                UsageGranularity.WEEKLY to Instant.parse("2026-04-22T12:00:00Z"),
                // mCal.addMonths(-6) = 180 日
                UsageGranularity.MONTHLY to Instant.parse("2025-11-21T12:00:00Z"),
                // mCal.addYears(-2) = 730 日
                UsageGranularity.YEARLY to Instant.parse("2024-05-20T12:00:00Z"),
            ),
            floors,
        )
    }

    /** 窓の始まりが下限より前なら gap の候補になる。 */
    @Test
    fun `下限より前から取ろうとしているかを判定できる`() {
        val floor = UsageRetention.eventsFloor(now)
        assertTrue("13 日前から取ろうとしているのに候補にならない", floor.excludes(Instant.parse("2026-05-07T12:00:00Z")))
        assertFalse("下限そのものは外に無い", floor.excludes(Instant.parse("2026-05-10T12:00:00Z")))
        assertFalse("5 日前は下限の内側", floor.excludes(Instant.parse("2026-05-15T12:00:00Z")))
    }

    /** 1 件も返らなかったら、取れなかった期間の終わりは下限そのもの。 */
    @Test
    fun `1 件も返らなければ取れなかった期間の終わりは下限`() {
        assertEquals(
            Instant.parse("2026-05-10T12:00:00Z"),
            UsageRetention.eventsFloor(now).gapEndWhenNothingReturned(),
        )
    }

    /**
     * **見込みが外れて古いイベントが返ったら、その時刻で閉じる**（spec レビュー R3）。
     *
     * 取得元が見込みより長く持っていた分は普通に記録になるので、
     * 「取れなかった」期間は残らない（取れているものを「取れなかった」と書かない）。
     */
    @Test
    fun `見込みより古いイベントが返ったらその時刻で閉じる`() {
        val floor = UsageRetention.eventsFloor(now)
        val oldest = Instant.parse("2026-04-30T12:00:00Z")
        assertEquals(oldest, floor.gapEnd(oldest))
    }

    /** 下限より後のイベントしか無ければ、閉じるのは下限のまま（下限までは取れていない）。 */
    @Test
    fun `下限より後の最古のイベントでは下限のまま閉じる`() {
        val floor = UsageRetention.eventsFloor(now)
        assertEquals(
            Instant.parse("2026-05-10T12:00:00Z"),
            floor.gapEnd(Instant.parse("2026-05-12T00:00:00Z")),
        )
    }

    /**
     * **見込みより古いイベントを返す取得元でも 1 件も落ちない。**
     *
     * 窓は下限で切り詰めない（`events(window)` に渡るのは保存した終わりのまま）ので、
     * 20 日前から問い合わせれば 20 日前のイベントも返る。
     * そのうえで「取れなかった期間」の長さは 0 になる（最古のイベント＝窓の始まり）ので、
     * 見込みが外れても嘘を書かずに済む。
     */
    @Test
    fun `見込みより古いイベントを返す取得元でも 1 件も落ちない`() {
        val begin = Instant.parse("2026-04-30T12:00:00Z") // 20 日前。見込みの下限（10 日）より古い
        val source = sourceAt(
            usageEvent("2026-04-30T12:00:00Z"),
            usageEvent("2026-05-05T00:00:00Z"),
            usageEvent("2026-05-10T11:59:59Z"), // 下限のすぐ手前
            usageEvent("2026-05-19T00:00:00Z"),
        )
        val got = (source.events(CollectionWindow(begin, now)) as EventsResult.Events).events
        assertEquals("下限より古い分が落ちている", 4, got.size)
        assertEquals(begin, got.first().at)
        // 窓の始まりは切り詰められていない
        assertEquals(listOf(CollectionWindow(begin, now)), source.eventQueries)
        // 取れなかった期間の長さは 0（始まり == 終わり）
        val floor = UsageRetention.eventsFloor(now)
        assertTrue("下限より前から取っているのに候補にならない", floor.excludes(begin))
        assertEquals(begin, floor.gapEnd(got.first().at))
    }
}
