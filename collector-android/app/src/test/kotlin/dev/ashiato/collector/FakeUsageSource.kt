// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant

/**
 * 取得元（`UsageStatsManager`）の偽物（tasks 2.1 / design D5 のリスク「`null` と 0 件の取り違え」）。
 *
 * **本番の取得元は単体では触れない** —— `UsageStatsManager` の中身（実際に端末に溜まった統計）は
 * Robolectric では作れないので、単体が触るのはこの偽物だけ。本番の経路は計測テストが通す。
 *
 * 本物に合わせてあるのは 3 点:
 * - イベントの範囲は `[begin, end)`（javadoc: "The **inclusive** beginning" / "The **exclusive** end"）
 * - **「読めなかった」は 0 件ではない** —— 再起動後に一度も解錠されていない端末では
 *   取得元が `null` を返す（[unreadable]）。取り違えて窓を進めると、その期間は 10 日で消える
 * - **`null` のもう 1 つの経路**（窓の始まりが現在時刻を超えた＝端末の時刻が戻った）も返す。
 *   判定は本番と同じ [windowIsQueryable] を呼ぶので、片方だけ緩くならない
 *
 * 集計は**範囲で絞らない** —— 本物は要求した範囲を箱の境界まで広げて返すので
 * （javadoc: "may be expanded to the nearest whole interval period"）、
 * 絞る偽物は本物より厳しくなり、呼び出し側の試験が本物で落ちる形になる。
 */
class FakeUsageSource(
    private val storedEvents: List<UsageEventSnapshot> = emptyList(),
    private val storedRollups: Map<UsageGranularity, List<UsageRollupSnapshot>> = emptyMap(),
    /** `null` でなければ「読めなかった」を返す（解錠されていない端末の側）。**0 件とは別物** */
    var unreadable: String? = null,
    /**
     * 端末の時計。**窓の始まりがこれを超えたら本物と同じく `Unreadable`**
     * （`UserUsageStatsService.validRange`。端末の時刻が戻ると起きる）。
     *
     * 既定の [Instant.MAX] は「窓の形では断らない」。時計が戻った場面を再現する試験だけが値を入れる。
     */
    var now: Instant = Instant.MAX,
    /**
     * 取得元が**保持している統計をずらした**幅（`UserUsageStatsService.onTimeChanged`）。
     *
     * 本物は端末の時計が変わったことを知ると、保持している統計を丸ごとその差だけずらす
     * （design の Context / tasks 3.3）。**ずらさない偽物は本物より優しい** ——
     * 時計が動いた後も同じ時刻でイベントが返るので、窓の付け替えを間違えても緑になる。
     */
    var shiftMs: Long = 0,
) : UsageSource {
    /** 問い合わせられた窓を順に覚える（**窓を切り詰めていない**ことを呼び出し側が見るため） */
    val eventQueries = mutableListOf<CollectionWindow>()

    /** 問い合わせられた集計の粒度と範囲 */
    val rollupQueries = mutableListOf<Pair<UsageGranularity, CollectionWindow>>()

    override fun events(window: CollectionWindow): EventsResult {
        eventQueries += window
        // **本番と同じ判定式**（`windowIsQueryable` を両方から呼ぶ）。別に書くとずれる
        if (!windowIsQueryable(window, now)) return EventsResult.Unreadable(UsageStatsSource.REASON_NULL)
        unreadable?.let { return EventsResult.Unreadable(it) }
        val hit = storedEvents
            .map { if (shiftMs == 0L) it else it.copy(at = it.at.plusMillis(shiftMs)) }
            .filter { !it.at.isBefore(window.begin) && it.at.isBefore(window.end) }
            .sortedBy { it.at }
        return EventsResult.Events(hit)
    }

    override fun rollups(granularity: UsageGranularity, window: CollectionWindow): RollupsResult {
        rollupQueries += granularity to window
        if (!windowIsQueryable(window, now)) return RollupsResult.Unreadable(UsageStatsSource.REASON_NULL)
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
