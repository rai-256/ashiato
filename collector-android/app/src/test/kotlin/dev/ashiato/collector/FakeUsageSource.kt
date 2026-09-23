// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant

/**
 * 取得元（`UsageStatsManager`）の偽物（tasks 2.1 / design D5 のリスク「`null` と 0 件の取り違え」）。
 *
 * **本番の取得元は単体では触れない** —— `UsageStatsManager` の中身（実際に端末に溜まった統計）は
 * Robolectric では作れないので、単体が触るのはこの偽物だけ。本番の経路は計測テストが通す。
 *
 * 本物に合わせてあるのは 2 点:
 * - イベントの範囲は `[begin, end)`（javadoc: "The **inclusive** beginning" / "The **exclusive** end"）
 * - **「読めなかった」は 0 件ではない** —— 再起動後に一度も解錠されていない端末では
 *   取得元が `null` を返す。取り違えて窓を進めると、その期間は 10 日で消える
 *
 * 集計は**範囲で絞らない** —— 本物は要求した範囲を箱の境界まで広げて返すので
 * （javadoc: "may be expanded to the nearest whole interval period"）、
 * 絞る偽物は本物より厳しくなり、呼び出し側の試験が本物で落ちる形になる。
 */
class FakeUsageSource(
    private val storedEvents: List<UsageEventSnapshot> = emptyList(),
    private val storedRollups: Map<UsageGranularity, List<UsageRollupSnapshot>> = emptyMap(),
    /** `null` でなければ「読めなかった」を返す。**0 件とは別物**（design D5 のリスク） */
    var unreadable: String? = null,
) : UsageSource {
    /** 問い合わせられた窓を順に覚える（**窓を切り詰めていない**ことを呼び出し側が見るため） */
    val eventQueries = mutableListOf<Pair<Instant, Instant>>()

    /** 問い合わせられた集計の粒度と範囲 */
    val rollupQueries = mutableListOf<Triple<UsageGranularity, Instant, Instant>>()

    override fun events(begin: Instant, end: Instant): EventsResult {
        eventQueries += begin to end
        unreadable?.let { return EventsResult.Unreadable(it) }
        val hit = storedEvents.filter { !it.at.isBefore(begin) && it.at.isBefore(end) }.sortedBy { it.at }
        return EventsResult.Events(hit)
    }

    override fun rollups(granularity: UsageGranularity, begin: Instant, end: Instant): RollupsResult {
        rollupQueries += Triple(granularity, begin, end)
        unreadable?.let { return RollupsResult.Unreadable(it) }
        return RollupsResult.Rollups(storedRollups[granularity].orEmpty())
    }
}

/** 試験で 1 件のイベントを組み立てる近道。欄は既定のまま置いても本物と同じ形になる。 */
fun usageEvent(
    at: String,
    packageName: String = "dev.ashiato.example",
    className: String? = "dev.ashiato.example.MainActivity",
    eventType: Int = 1,
): UsageEventSnapshot = UsageEventSnapshot(
    packageName = packageName,
    className = className,
    eventType = eventType,
    at = Instant.parse(at),
)

/** 試験で 1 件の集計を組み立てる近道。 */
fun usageRollup(
    packageName: String = "dev.ashiato.example",
    firstAt: String = "2026-05-01T00:00:00Z",
    lastAt: String = "2026-05-02T00:00:00Z",
    totalForegroundMs: Long = 60_000,
): UsageRollupSnapshot = UsageRollupSnapshot(
    packageName = packageName,
    firstAt = Instant.parse(firstAt),
    lastAt = Instant.parse(lastAt),
    lastUsedAt = Instant.parse(lastAt),
    lastVisibleAt = Instant.parse(lastAt),
    lastForegroundServiceUsedAt = Instant.EPOCH,
    totalForegroundMs = totalForegroundMs,
    totalVisibleMs = totalForegroundMs,
    totalForegroundServiceMs = 0,
)
