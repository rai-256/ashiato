// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.usage.UsageStatsManager
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 取得元の口（tasks 2.1 / design D5）。
 *
 * ここで確かめるのは**口の形**だけ —— 窓の進め方も記録の組み立ても gap も
 * この Task の外（3 / 4）にある。いちばん大事なのは
 * **「読めなかった」と「0 件だった」が型で別物**であること（design のリスク
 * 「`null` と 0 件の取り違え」）—— 取り違えて窓を進めると、その期間は 10 日で消える。
 */
class UsageSourceTest {
    private val begin = Instant.parse("2026-05-20T00:00:00Z")
    private val end = Instant.parse("2026-05-20T00:30:00Z")

    /** 読めなかったときは**件数を持たない** —— 0 件の結果と同じ型に畳まれていない。 */
    @Test
    fun `読めなかった取得元は Unreadable を返す`() {
        val source = FakeUsageSource(unreadable = "locked")
        val result = source.events(begin, end)
        assertTrue("読めなかったのに $result が返った", result is EventsResult.Unreadable)
        assertEquals("locked", (result as EventsResult.Unreadable).reason)
    }

    /** 0 件は**成功**（本人の決定 C7）。携帯を使っていなかっただけの区間を失敗にしない。 */
    @Test
    fun `0 件の取得元は空の Events を返す`() {
        val result = FakeUsageSource().events(begin, end)
        assertTrue("0 件なのに $result が返った", result is EventsResult.Events)
        assertEquals(emptyList<UsageEventSnapshot>(), (result as EventsResult.Events).events)
    }

    /** **同じ型に畳まれていない**ことを直接見る（畳まれていたら呼び出し側は区別できない）。 */
    @Test
    fun `読めなかったと 0 件は別物`() {
        val unreadable = FakeUsageSource(unreadable = "locked").events(begin, end)
        val empty = FakeUsageSource().events(begin, end)
        assertTrue("0 件が読めなかった側に入っている", empty !is EventsResult.Unreadable)
        assertTrue("読めなかったが 0 件の側に入っている", unreadable !is EventsResult.Events)
    }

    /**
     * 範囲は `[begin, end)`。javadoc の逐語:
     * "The **inclusive** beginning" / "The **exclusive** end"。
     *
     * 境界を閉じ側に間違えると、重ねて進める窓が同じイベントを 2 回返す
     * （原文が同じなのでサーバでは畳まれるが、端末の未送信は倍になる）。
     */
    @Test
    fun `イベントの範囲は始まりを含み終わりを含まない`() {
        val source = FakeUsageSource(
            storedEvents = listOf(
                usageEvent("2026-05-19T23:59:59Z"),
                usageEvent("2026-05-20T00:00:00Z"),
                usageEvent("2026-05-20T00:29:59Z"),
                usageEvent("2026-05-20T00:30:00Z"),
            ),
        )
        val got = (source.events(begin, end) as EventsResult.Events).events.map { it.at.toString() }
        assertEquals(listOf("2026-05-20T00:00:00Z", "2026-05-20T00:29:59Z"), got)
    }

    /** 問い合わせた窓は**渡したまま**（呼び出し側が切り詰めていないことを見る口）。 */
    @Test
    fun `問い合わせた窓を覚えている`() {
        val source = FakeUsageSource()
        source.events(begin, end)
        assertEquals(listOf(begin to end), source.eventQueries)
    }

    /** 集計も「読めなかった」と 0 件を分ける（イベントと同じ規律）。 */
    @Test
    fun `集計も読めなかったと 0 件を分ける`() {
        val unreadable = FakeUsageSource(unreadable = "locked").rollups(UsageGranularity.DAILY, begin, end)
        assertTrue("読めなかったのに $unreadable が返った", unreadable is RollupsResult.Unreadable)
        val empty = FakeUsageSource().rollups(UsageGranularity.DAILY, begin, end)
        assertTrue("0 件なのに $empty が返った", empty is RollupsResult.Rollups)
        assertEquals(emptyList<UsageRollupSnapshot>(), (empty as RollupsResult.Rollups).rollups)
    }

    /** 集計は**粒度ごとに別のもの**（4 粒度を 1 本に混ぜない。design D3 / spec 4.3）。 */
    @Test
    fun `集計は粒度ごとに別のものが返る`() {
        val source = FakeUsageSource(
            storedRollups = mapOf(
                UsageGranularity.DAILY to listOf(usageRollup(packageName = "day")),
                UsageGranularity.YEARLY to listOf(usageRollup(packageName = "year")),
            ),
        )
        val daily = source.rollups(UsageGranularity.DAILY, begin, end) as RollupsResult.Rollups
        val yearly = source.rollups(UsageGranularity.YEARLY, begin, end) as RollupsResult.Rollups
        assertEquals(listOf("day"), daily.rollups.map { it.packageName })
        assertEquals(listOf("year"), yearly.rollups.map { it.packageName })
        assertEquals(
            listOf(UsageGranularity.DAILY, UsageGranularity.YEARLY),
            source.rollupQueries.map { it.first },
        )
    }

    /**
     * 粒度は**取得元の区間の種別**に対応する。
     *
     * 本番の `UsageStatsManager` は単体では触れないが、**渡す番号が合っているか**は
     * ここで止められる（取り違えると 4 粒度のうち 1 つが別の箱を読む）。
     */
    @Test
    fun `粒度は取得元の区間の種別に対応する`() {
        assertEquals(UsageStatsManager.INTERVAL_DAILY, UsageGranularity.DAILY.intervalType)
        assertEquals(UsageStatsManager.INTERVAL_WEEKLY, UsageGranularity.WEEKLY.intervalType)
        assertEquals(UsageStatsManager.INTERVAL_MONTHLY, UsageGranularity.MONTHLY.intervalType)
        assertEquals(UsageStatsManager.INTERVAL_YEARLY, UsageGranularity.YEARLY.intervalType)
        assertEquals(4, UsageGranularity.entries.size)
    }

    /** 見込みの下限は**口から引ける**が、値を持つのは [UsageRetention] 1 か所だけ（tasks 2.2）。 */
    @Test
    fun `口が返す見込みの下限は UsageRetention のもの`() {
        val source = FakeUsageSource()
        val now = Instant.parse("2026-05-20T12:00:00Z")
        assertEquals(UsageRetention.eventsFloor(now), source.retentionFloor(now))
        for (g in UsageGranularity.entries) {
            assertEquals(UsageRetention.rollupFloor(now, g), source.retentionFloor(now, g))
        }
    }
}
