// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.usage.UsageEvents
import android.app.usage.UsageStats
import android.app.usage.UsageStatsManager
import android.content.Context
import android.content.res.Configuration
import android.os.Build
import android.os.UserManager
import java.time.Instant

/**
 * 集計の粒度（design D3 / spec「収集を始めた時点の過去の利用の集計を取り込む」）。
 *
 * 番号は取得元の区間の種別そのもの。**取り違えると別の箱を読む**ので、対応は試験で止める。
 */
enum class UsageGranularity(val intervalType: Int) {
    DAILY(UsageStatsManager.INTERVAL_DAILY),
    WEEKLY(UsageStatsManager.INTERVAL_WEEKLY),
    MONTHLY(UsageStatsManager.INTERVAL_MONTHLY),
    YEARLY(UsageStatsManager.INTERVAL_YEARLY),
}

/**
 * 取得元が返したイベント 1 件を、そのまま持ち歩く形にしたもの（design D1 / 本人の決定 Q1 / Q8）。
 *
 * **ふるいにかけない** —— `UsageEvents.Event` が公開している 8 つの欄
 * （どの種別も持つ 4 つ ＋ 種別ごとにだけ付く 4 つ）を全部持つ。絞った分は 10 日で消える。
 *
 * **記録（`raw` / `payload`）への組み立てはここでしない**（design D2 / tasks 3.1）——
 * ここは「取得元が何を返したか」だけを運ぶ。
 */
data class UsageEventSnapshot(
    /** `getPackageName()` */
    val packageName: String?,
    /** `getClassName()` —— アプリの中のどの画面か */
    val className: String?,
    /** `getEventType()` —— 前景の出入り・画面の入切・待機の階級の変化など */
    val eventType: Int,
    /** `getTimeStamp()` */
    val at: Instant,
    /** `getConfiguration()` —— `CONFIGURATION_CHANGE` のときだけ付く */
    val configuration: Configuration? = null,
    /** `getShortcutId()` —— `SHORTCUT_INVOCATION` のときだけ付く */
    val shortcutId: String? = null,
    /** `getExtras()` の `UsageStatsManager.EXTRA_EVENT_ACTION` —— `USER_INTERACTION` のときだけ付く */
    val interactionAction: String? = null,
    /** `getExtras()` の `UsageStatsManager.EXTRA_EVENT_CATEGORY` —— `USER_INTERACTION` のときだけ付く */
    val interactionCategory: String? = null,
    /** `getAppStandbyBucket()` —— `STANDBY_BUCKET_CHANGED` のときだけ付く */
    val standbyBucket: Int? = null,
)

/**
 * 取得元が返した集計 1 件（`UsageStats`）。イベントと同じく**欄を絞らない**。
 *
 * 要求した範囲は**箱の境界まで広げられ**、同じアプリが複数件返りうる
 * （javadoc: "may be expanded to the nearest whole interval period" /
 * "will contain one or more UsageStats objects for each package"）。
 * どう畳むかは取り込む側（tasks 4.3）の判断で、ここでは返ったものをそのまま持つ。
 */
data class UsageRollupSnapshot(
    /** `getPackageName()` */
    val packageName: String?,
    /** `getFirstTimeStamp()` —— この箱が覆う範囲の始まり */
    val firstAt: Instant,
    /** `getLastTimeStamp()` —— この箱が覆う範囲の終わり */
    val lastAt: Instant,
    /** `getLastTimeUsed()` */
    val lastUsedAt: Instant,
    /** `getLastTimeVisible()` */
    val lastVisibleAt: Instant,
    /** `getLastTimeForegroundServiceUsed()` */
    val lastForegroundServiceUsedAt: Instant,
    /** `getTotalTimeInForeground()` */
    val totalForegroundMs: Long,
    /** `getTotalTimeVisible()` */
    val totalVisibleMs: Long,
    /** `getTotalTimeForegroundServiceUsed()` */
    val totalForegroundServiceMs: Long,
)

/**
 * イベントを取りに行った結果。**「読めなかった」と「0 件だった」を型で分ける**
 * （design D5 のリスク / 本人の決定 C5）。
 */
sealed interface EventsResult {
    /** 取れた。**0 件でも成功** —— 携帯を使っていなかっただけの区間を失敗にしない。 */
    data class Events(val events: List<UsageEventSnapshot>) : EventsResult

    /**
     * 取得元が読めなかった（`queryEvents` が `null` を返した）。**0 件と取り違えない** ——
     * 取り違えて窓を進めると、その期間のイベントは取り直されないまま 10 日で消える。
     */
    data class Unreadable(val reason: String) : EventsResult
}

/** 集計を取りに行った結果。イベントと同じ規律で「読めなかった」を分ける。 */
sealed interface RollupsResult {
    data class Rollups(val rollups: List<UsageRollupSnapshot>) : RollupsResult

    data class Unreadable(val reason: String) : RollupsResult
}

/**
 * アプリ利用の取得元の口（tasks 2.1 / design D5）。**本番と偽物を型で分けるため**にある
 * （`FixSource` と同じ形）—— `UsageStatsManager` の中身は単体では作れないので、
 * 窓・gap・集計の取り込みの試験は全部この口の偽物に当たる。
 */
interface UsageSource {
    /**
     * [window]（`[begin, end)`）のイベントを取る。境界は取得元の javadoc のまま
     * （"The **inclusive** beginning" / "The **exclusive** end"）。
     *
     * **生の `Instant` 2 本ではなく [CollectionWindow] を取る** —— 終わりが始まりより前の窓は
     * そもそも組み立てられない（Task 1 の `require`）。取得元はそういう範囲に `null` を返すので、
     * 生で受けると呼び出し側が「読めなかった」と「0 件だった」を取り違える経路が口に残る。
     *
     * **窓をここで切り詰めない。** 見込みの保持の下限（[retentionFloor]）は渡された窓に一切効かない。
     */
    fun events(window: CollectionWindow): EventsResult

    /** その粒度の集計を取る。範囲は取得元の都合で箱の境界まで広がりうる。 */
    fun rollups(granularity: UsageGranularity, window: CollectionWindow): RollupsResult

    /**
     * 生のイベントが取得元に残っている**見込み**の下限。既定は [UsageRetention]。
     *
     * 口に置いてあるのは、**見込みが外れた取得元**（長く持っている／早く消す）を
     * 偽物で作れるようにするため。値そのものは [UsageRetention] 1 か所にしかない。
     */
    fun retentionFloor(now: Instant): RetentionFloor = UsageRetention.eventsFloor(now)

    /** 集計が残っている見込みの下限。**粒度ごとに違う**（年 2 年 / 月 6 か月 / 週 4 週 / 日 10 日）。 */
    fun retentionFloor(now: Instant, granularity: UsageGranularity): RetentionFloor =
        UsageRetention.rollupFloor(now, granularity)
}

/**
 * 窓そのものを理由に取得元が `null` を返すか（`UserUsageStatsService.validRange`）。
 * 逐語: `return beginTime <= currentTime && beginTime < endTime;`
 *
 * **本番も偽物もここを呼ぶ。** 2 か所に書くと片方だけずれ、偽物が本物より緩くなる ——
 * 端末の時刻が戻って窓の始まりが現在時刻を超えた場面で、偽物が「0 件だった」を返せば
 * 呼び出し側は窓を進めて緑になり、実機では `Unreadable` で進まない
 * （本人の決定 C5 が分かれる、まさにその軸）。
 */
internal fun windowIsQueryable(window: CollectionWindow, now: Instant): Boolean =
    !window.begin.isAfter(now) && window.begin.isBefore(window.end)

/**
 * 本番の取得元（`UsageStatsManager`）。
 *
 * **取得元に溜まった統計の中身は単体では作れない**ので、イベントや集計が返る経路を通すのは
 * 計測テスト（tasks 5 / 7）。単体が見るのは、窓の形だけで決まる「読めなかった」の判定が
 * 偽物と一致することだけ（`UsageSourceTest`）。
 *
 * `queryEvents` が `null` を返す経路は 2 つある（deep レビュー R3）——
 * 再起動後に一度も解錠されていない端末（`UserManager.isUserUnlocked()` が偽。逐語:
 * "if the user's device is **not in an unlocked state** … then **null will be returned**"）と、
 * 窓の始まりが現在時刻を超えたとき（`UserUsageStatsService.validRange` が偽）。
 * **どちらも 0 件ではない。**
 *
 * `queryUsageStats` の側は取得元が `null` を空の一覧に畳んでしまう（`ParceledListSlice` が
 * `null` なら `Collections.emptyList()`）ので、**解錠の状態をこちらで先に見る** ——
 * 見ないと、解錠前の 6 時間ぶんが「0 件だった」として通る。
 */
class UsageStatsSource(
    context: Context,
    /** 端末の時計。`validRange` の判定に要る（試験が同じ窓を再現できるように口にしてある） */
    private val now: () -> Instant = Instant::now,
) : UsageSource {
    private val stats: UsageStatsManager? = context.getSystemService(UsageStatsManager::class.java)
    private val users: UserManager? = context.getSystemService(UserManager::class.java)

    override fun events(window: CollectionWindow): EventsResult {
        val manager = stats ?: return EventsResult.Unreadable(REASON_NO_SERVICE)
        // **窓の形で決まる `null` を先に名指しする** —— 取得元も同じ判定で `null` を返すが、
        // ここで判定を通しておかないと偽物が同じ窓を再現できない
        if (!windowIsQueryable(window, now())) return EventsResult.Unreadable(REASON_NULL)
        // **解錠されていない端末を弾く**（`queryEvents` の `null` と同じ扱い）
        val unlocked = users?.isUserUnlocked ?: return EventsResult.Unreadable(REASON_NO_SERVICE)
        if (!unlocked) return EventsResult.Unreadable(REASON_LOCKED)
        val events = manager.queryEvents(window.begin.toEpochMilli(), window.end.toEpochMilli())
            ?: return EventsResult.Unreadable(REASON_NULL)
        val out = mutableListOf<UsageEventSnapshot>()
        val event = UsageEvents.Event()
        while (events.getNextEvent(event)) out += snapshotOf(event)
        return EventsResult.Events(out)
    }

    override fun rollups(granularity: UsageGranularity, window: CollectionWindow): RollupsResult {
        val manager = stats ?: return RollupsResult.Unreadable(REASON_NO_SERVICE)
        if (!windowIsQueryable(window, now())) return RollupsResult.Unreadable(REASON_NULL)
        // **`queryUsageStats` は `null` を空の一覧に畳む** ので、解錠は自分で見ないと
        // 解錠前の 6 時間ぶんが「0 件だった」として通る
        val unlocked = users?.isUserUnlocked ?: return RollupsResult.Unreadable(REASON_NO_SERVICE)
        if (!unlocked) return RollupsResult.Unreadable(REASON_LOCKED)
        val list = manager.queryUsageStats(
            granularity.intervalType,
            window.begin.toEpochMilli(),
            window.end.toEpochMilli(),
        ) ?: return RollupsResult.Unreadable(REASON_NULL)
        return RollupsResult.Rollups(list.map(::snapshotOf))
    }

    /**
     * 1 件を写し取る。**種別ごとに付く欄は種別で見てから読む** ——
     * 付かない種別で読んだ値（`getAppStandbyBucket()` の 0 など）を入れると、
     * 「取得元が返した値」でないものが記録に混じる。
     *
     * `Configuration` は**複製する** —— `getNextEvent` は渡した 1 つの `Event` を使い回すので、
     * 参照のまま持つと次の 1 件で中身が変わりうる。
     */
    private fun snapshotOf(event: UsageEvents.Event): UsageEventSnapshot {
        val type = event.eventType
        val extras = if (type == UsageEvents.Event.USER_INTERACTION &&
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
        ) {
            event.extras
        } else {
            null
        }
        return UsageEventSnapshot(
            packageName = event.packageName,
            className = event.className,
            eventType = type,
            at = Instant.ofEpochMilli(event.timeStamp),
            configuration = if (type == UsageEvents.Event.CONFIGURATION_CHANGE) {
                event.configuration?.let(::Configuration)
            } else {
                null
            },
            shortcutId = if (type == UsageEvents.Event.SHORTCUT_INVOCATION) event.shortcutId else null,
            interactionAction = extras?.getString(UsageStatsManager.EXTRA_EVENT_ACTION),
            interactionCategory = extras?.getString(UsageStatsManager.EXTRA_EVENT_CATEGORY),
            standbyBucket = if (type == UsageEvents.Event.STANDBY_BUCKET_CHANGED) event.appStandbyBucket else null,
        )
    }

    private fun snapshotOf(stats: UsageStats): UsageRollupSnapshot = UsageRollupSnapshot(
        packageName = stats.packageName,
        firstAt = Instant.ofEpochMilli(stats.firstTimeStamp),
        lastAt = Instant.ofEpochMilli(stats.lastTimeStamp),
        lastUsedAt = Instant.ofEpochMilli(stats.lastTimeUsed),
        lastVisibleAt = Instant.ofEpochMilli(stats.lastTimeVisible),
        lastForegroundServiceUsedAt = Instant.ofEpochMilli(stats.lastTimeForegroundServiceUsed),
        totalForegroundMs = stats.totalTimeInForeground,
        totalVisibleMs = stats.totalTimeVisible,
        totalForegroundServiceMs = stats.totalTimeForegroundServiceUsed,
    )

    companion object {
        /** 取得元そのものが端末に無い（あり得ないが、`getSystemService` は `null` を返しうる） */
        const val REASON_NO_SERVICE: String = "no_service"

        /** 再起動後に一度も解錠されていない。**0 件ではない** */
        const val REASON_LOCKED: String = "locked"

        /** 取得元が `null` を返した（窓の始まりが現在時刻を超えたときなど） */
        const val REASON_NULL: String = "null_result"
    }
}
