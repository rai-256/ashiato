// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.usage.UsageStatsManager
import androidx.test.core.app.ApplicationProvider
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 取得元の口（tasks 2.1 / design D5）。
 *
 * ここで確かめるのは**口の形**だけ —— 窓の進め方も記録の組み立ても gap も
 * この Task の外（3 / 4）にある。いちばん大事なのは
 * **「読めなかった」と「0 件だった」が型で別物**であること（design のリスク
 * 「`null` と 0 件の取り違え」）—— 取り違えて窓を進めると、その期間は 10 日で消える。
 */
@RunWith(RobolectricTestRunner::class)
class UsageSourceTest {
    private val begin = Instant.parse("2026-05-20T00:00:00Z")
    private val end = Instant.parse("2026-05-20T00:30:00Z")
    private val window = CollectionWindow(begin, end)

    /** 読めなかったときは**件数を持たない** —— 0 件の結果と同じ型に畳まれていない。 */
    @Test
    fun `読めなかった取得元は Unreadable を返す`() {
        val source = FakeUsageSource(unreadable = "locked")
        val result = source.events(window)
        assertTrue("読めなかったのに $result が返った", result is EventsResult.Unreadable)
        assertEquals("locked", (result as EventsResult.Unreadable).reason)
    }

    /** 0 件は**成功**（本人の決定 C7）。携帯を使っていなかっただけの区間を失敗にしない。 */
    @Test
    fun `0 件の取得元は空の Events を返す`() {
        val result = FakeUsageSource().events(window)
        assertTrue("0 件なのに $result が返った", result is EventsResult.Events)
        assertEquals(emptyList<UsageEventSnapshot>(), (result as EventsResult.Events).events)
    }

    /** **同じ型に畳まれていない**ことを直接見る（畳まれていたら呼び出し側は区別できない）。 */
    @Test
    fun `読めなかったと 0 件は別物`() {
        val unreadable = FakeUsageSource(unreadable = "locked").events(window)
        val empty = FakeUsageSource().events(window)
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
        val got = (source.events(window) as EventsResult.Events).events.map { it.at.toString() }
        assertEquals(listOf("2026-05-20T00:00:00Z", "2026-05-20T00:29:59Z"), got)
    }

    /** 問い合わせた窓は**渡したまま**（呼び出し側が切り詰めていないことを見る口）。 */
    @Test
    fun `問い合わせた窓を覚えている`() {
        val source = FakeUsageSource()
        source.events(window)
        assertEquals(listOf(window), source.eventQueries)
    }

    /** 集計も「読めなかった」と 0 件を分ける（イベントと同じ規律）。 */
    @Test
    fun `集計も読めなかったと 0 件を分ける`() {
        val unreadable = FakeUsageSource(unreadable = "locked").rollups(UsageGranularity.DAILY, window)
        assertTrue("読めなかったのに $unreadable が返った", unreadable is RollupsResult.Unreadable)
        val empty = FakeUsageSource().rollups(UsageGranularity.DAILY, window)
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
        val daily = source.rollups(UsageGranularity.DAILY, window) as RollupsResult.Rollups
        val yearly = source.rollups(UsageGranularity.YEARLY, window) as RollupsResult.Rollups
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

    /**
     * **窓そのものが理由の `null`**（`UserUsageStatsService.validRange` が偽）。
     * 端末の時刻が戻ると窓の始まりが現在時刻を超える —— 取得元はそこで `null` を返すので、
     * 0 件と取り違えると窓が進み、その期間は取り直されない（本人の決定 C5）。
     */
    @Test
    fun `窓の始まりが現在時刻を超えていたら読めなかったになる`() {
        val source = FakeUsageSource().also { it.now = Instant.parse("2026-05-19T00:00:00Z") }
        assertEquals(EventsResult.Unreadable(UsageStatsSource.REASON_NULL), source.events(window))
        assertEquals(
            RollupsResult.Unreadable(UsageStatsSource.REASON_NULL),
            source.rollups(UsageGranularity.DAILY, window),
        )
    }

    /** `validRange` の逐語は `beginTime <= currentTime && beginTime < endTime` —— 長さ 0 の窓も `null`。 */
    @Test
    fun `長さ 0 の窓は読めなかったになる`() {
        val source = FakeUsageSource().also { it.now = Instant.parse("2026-06-01T00:00:00Z") }
        assertTrue(source.events(CollectionWindow(begin, begin)) is EventsResult.Unreadable)
    }

    /**
     * **本番と偽物が同じ窓で同じ結果を返す**（独立レビュー Important 2）。
     *
     * 取得元に溜まった統計の中身は単体では作れないが、**窓の形だけで決まる判定**は
     * 両方が同じ [windowIsQueryable] を通るので、ここで一致を固定できる。
     * 偽物がこの軸で緩いと、時計が戻った場面で「0 件 → 窓を進める」が緑になり、実機で食い違う。
     */
    @Test
    fun `時刻が戻った窓は本番も偽物も同じ結果を返す`() {
        val app: Application = ApplicationProvider.getApplicationContext()
        val now = Instant.parse("2026-05-19T00:00:00Z")
        val real = UsageStatsSource(app, now = { now })
        val fake = FakeUsageSource().also { it.now = now }
        assertEquals(fake.events(window), real.events(window))
        assertEquals(
            fake.rollups(UsageGranularity.DAILY, window),
            real.rollups(UsageGranularity.DAILY, window),
        )
        assertEquals(EventsResult.Unreadable(UsageStatsSource.REASON_NULL), real.events(window))
    }
}
