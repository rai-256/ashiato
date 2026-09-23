// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Duration
import java.time.Instant

/**
 * 取得元がまだ持っている**見込み**の下限（tasks 2.2 / design D4 / spec レビュー R3）。
 *
 * **型が守っているのはここまで**: この型は `Instant` では**ない**ので、
 * [CollectionWindow] にも [UsageSource.events] にも**そのままでは渡らない**。
 * 値を外へ出すには、名前が用途を言っている 2 つの口を**明示的に**呼ぶしかない。
 *
 * **規律で守るもの（型では止まらない）**: [gapEndWhenNothingReturned] が返す `Instant` は
 * 下限そのもので、取得の窓の始まりに代入することは**コンパイルできてしまう**。やらない ——
 * それが spec レビュー R3 が止めた形で、**この値で窓を切り詰めると取得元にまだ残っている
 * イベントを飛ばしたうえで「取れなかった」という嘘を正典の形式で残す**（飛ばした分は 10 日で消える）。
 * 10 日は API から読めない見込みなので、窓は**保存した終わりから**問い合わせる。
 *
 * 持っているのは gap の判定に要る 3 つだけ:
 *
 * - [excludes] —— 窓の始まりがこの下限より前か（＝ gap の候補になるか）
 * - [gapEnd] —— イベントが返ったときの、取れなかった期間の終わり
 * - [gapEndWhenNothingReturned] —— 1 件も返らなかったときの終わり（＝下限そのもの）
 */
class RetentionFloor internal constructor(private val at: Instant) {
    /** 窓の始まりがこの下限より**前**にあるか。真なら「取りに行ったが取得元に無かった」の候補。 */
    fun excludes(windowBegin: Instant): Boolean = at.isAfter(windowBegin)

    /**
     * 取れなかった期間の終わり —— この下限と [oldestEvent] の**早いほう**（spec レビュー R3）。
     *
     * 見込みが外れて取得元が長く持っていれば、その分は普通に記録になるので、
     * 期間は返った最古のイベントの時刻で閉じる（＝取れているものを「取れなかった」と書かない）。
     */
    fun gapEnd(oldestEvent: Instant): Instant = if (oldestEvent.isBefore(at)) oldestEvent else at

    /**
     * 1 件も返らなかったときの、取れなかった期間の終わり ＝ **下限そのもの**。
     *
     * **下限の値が外へ出る唯一の口**なので、名前で用途を言っている ——
     * 返るのは gap の記録の終わりであって、**取得の窓の始まりではない**。
     */
    fun gapEndWhenNothingReturned(): Instant = at

    override fun equals(other: Any?): Boolean = other is RetentionFloor && other.at == at

    override fun hashCode(): Int = at.hashCode()

    override fun toString(): String = "RetentionFloor($at)"
}

/**
 * 取得元（`UsageStatsManager`）が持っている見込みの長さ。**値はここにだけ置く**（tasks 2.2）。
 *
 * 出所は AOSP の `UsageStatsDatabase.prune()`（2026-09-18 取得）。逐語:
 *
 * ```java
 * mCal.addYears(-2);  pruneFilesOlderThan(mIntervalDirs[UsageStatsManager.INTERVAL_YEARLY],  …);
 * mCal.addMonths(-6); pruneFilesOlderThan(mIntervalDirs[UsageStatsManager.INTERVAL_MONTHLY], …);
 * mCal.addWeeks(-4);  pruneFilesOlderThan(mIntervalDirs[UsageStatsManager.INTERVAL_WEEKLY],  …);
 * mCal.addDays(-10);  pruneFilesOlderThan(mIntervalDirs[UsageStatsManager.INTERVAL_DAILY],   …);
 * ```
 *
 * `mCal` は `com.android.server.usage.UnixCalendar` で、**暦を知らない**（逐語:
 * "A handy calendar object that knows nothing of Locale's or TimeZones"）。
 * 加減は固定のミリ秒で、`MONTH_IN_MILLIS = 30 * DAY_IN_MILLIS` /
 * `YEAR_IN_MILLIS = 365 * DAY_IN_MILLIS`。だからここも**固定の日数**で数える
 * —— 暦で数えると取得元より下限が動き、時間帯にも依存する。
 *
 * **生のイベントの下限が日ごとの集計と同じ 10 日なのは、同じ箱を読むから** ——
 * `UserUsageStatsService.queryEvents` は `queryStats(INTERVAL_DAILY, …)` を呼ぶ。
 *
 * **これは API から読めない見込み**（OEM 改変・OS 版差・prune の起動タイミングでずれる）。
 * だから**問い合わせの窓を切り詰めるのには使わない**（spec レビュー R3）。使うのは gap の判定だけ。
 * 出口を [RetentionFloor] に絞って用途を名前に出してあるが、**切り詰めないことを型は保証しない** ——
 * `gapEndWhenNothingReturned()` の戻り値は生の `Instant` で、窓に渡せば通ってしまう。
 */
object UsageRetention {
    /** `mCal.addDays(-10)`（`INTERVAL_DAILY` の箱。生のイベントもここから読まれる） */
    private val EVENTS: Duration = Duration.ofDays(10)

    /** `mCal.addWeeks(-4)` = 28 日（`UnixCalendar.WEEK_IN_MILLIS = 7 * DAY_IN_MILLIS`） */
    private val WEEKLY: Duration = Duration.ofDays(4 * 7)

    /** `mCal.addMonths(-6)` = 180 日（`UnixCalendar.MONTH_IN_MILLIS = 30 * DAY_IN_MILLIS`） */
    private val MONTHLY: Duration = Duration.ofDays(6 * 30)

    /** `mCal.addYears(-2)` = 730 日（`UnixCalendar.YEAR_IN_MILLIS = 365 * DAY_IN_MILLIS`） */
    private val YEARLY: Duration = Duration.ofDays(2 * 365)

    /** 生のイベントの見込みの下限。 */
    fun eventsFloor(now: Instant): RetentionFloor = RetentionFloor(now.minus(EVENTS))

    /**
     * 集計の見込みの下限。**粒度ごとに違う**ので粒度を取る
     * （1 つの関数に畳むと、年 2 年ぶんの箱を 10 日で切ることになる）。
     */
    fun rollupFloor(now: Instant, granularity: UsageGranularity): RetentionFloor {
        val keep = when (granularity) {
            UsageGranularity.DAILY -> EVENTS
            UsageGranularity.WEEKLY -> WEEKLY
            UsageGranularity.MONTHLY -> MONTHLY
            UsageGranularity.YEARLY -> YEARLY
        }
        return RetentionFloor(now.minus(keep))
    }
}
