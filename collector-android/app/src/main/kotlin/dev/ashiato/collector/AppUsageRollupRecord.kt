// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/**
 * 取得元が返した集計 1 件を、契約どおりの記録 1 件にする
 * （tasks 4.3 / 4.4 / design D3 / 本人の決定 C1 / C2 / C12）。
 *
 * **1 集計 = 1 記録**（C1 と同じ規律）。1 回の取り込みをまとめて 1 件にすると、
 * 次の契機で中身が変わって冪等キーが動き、取り込みのたびに行が増える。
 *
 * **集計は毎回同じ期間を返す** —— 日ごとの箱は 6 時間ごとに 4 回読み直される（design D3）。
 * だから原文の**並びと省略の規則が契約そのもの**で、`RollupPayloadShapeTest` が
 * 1 文字単位で固定する。変えると同じ 1 件が別の鍵になって二重に入る。
 *
 * | 欄 | 出所 | 省略 |
 * |---|---|---|
 * | `granularity` | どの箱に問い合わせたか（`UsageGranularity`） | 省略しない |
 * | `package` | `getPackageName()` | 無ければ欄ごと |
 * | `begin` | `getFirstTimeStamp()` | 省略しない |
 * | `end` | `getLastTimeStamp()` | 省略しない |
 * | `last_used` | `getLastTimeUsed()` | 省略しない |
 * | `last_visible` | `getLastTimeVisible()` | 省略しない |
 * | `last_foreground_service_used` | `getLastTimeForegroundServiceUsed()` | 省略しない |
 * | `total_foreground_ms` | `getTotalTimeInForeground()` | 省略しない |
 * | `total_visible_ms` | `getTotalTimeVisible()` | 省略しない |
 * | `total_foreground_service_ms` | `getTotalTimeForegroundServiceUsed()` | 省略しない |
 * | `app_label` | 取得時点の `PackageManager`（**`payload` だけ**） | 引けなければ欄ごと |
 *
 * **出来事の時刻は箱の終わり**（gap の記録と同じ規律）。始まりに置くと、
 * 年ごとの箱が 2 年前へ落ちて「いつの記録か」が箱の粒度ごとにばらける。
 *
 * 地域は**取得時点の端末のもの**（C12）—— 集計は最大 2 年を遡るが、
 * その時点の地域は取得元が持っていない。
 */
fun UsageRollupSnapshot.toIngestRequest(
    id: String,
    userId: String,
    deviceId: String,
    zone: ZoneId,
    granularity: UsageGranularity,
    appLabel: String?,
): IngestRequest {
    val fields = sourceFields(granularity)
    return IngestRequest(
        id = id,
        userId = userId,
        // **イベントとは別の論理ソース**（design D3）
        logicalSource = APP_USAGE_ROLLUP_LOGICAL_SOURCE,
        externalId = null,
        deviceId = deviceId,
        origin = "collected",
        eventTime = lastAt.toString(),
        tzOffsetMin = zone.rules.getOffset(lastAt).totalSeconds / 60,
        tzId = zone.id,
        schemaVersion = 1,
        unitSystem = null,
        crs = null,
        raw = ingestJson.encodeToString(JsonObject.serializer(), JsonObject(fields)),
        payload = JsonObject(
            if (appLabel == null) fields else fields + (KEY_APP_LABEL to JsonPrimitive(appLabel)),
        ),
    )
}

/** 解析済みにだけ入る欄（取得元が返した値では**ない**ので、原文には入れない）。 */
private const val KEY_APP_LABEL = "app_label"

/**
 * 原文に入る欄を、**固定の並び**で。`buildMap` は並びを保つ（`LinkedHashMap`）。
 *
 * 時刻は `Instant.toString()`（ISO-8601）で、端末の地域にも言語にも依らない ——
 * 端末の書式で書くと、引っ越しただけで同じ箱が別の鍵になる。
 */
private fun UsageRollupSnapshot.sourceFields(granularity: UsageGranularity): Map<String, JsonPrimitive> = buildMap {
    put("granularity", JsonPrimitive(granularity.key))
    packageName?.let { put("package", JsonPrimitive(it)) }
    put("begin", JsonPrimitive(firstAt.toString()))
    put("end", JsonPrimitive(lastAt.toString()))
    put("last_used", JsonPrimitive(lastUsedAt.toString()))
    put("last_visible", JsonPrimitive(lastVisibleAt.toString()))
    put("last_foreground_service_used", JsonPrimitive(lastForegroundServiceUsedAt.toString()))
    put("total_foreground_ms", JsonPrimitive(totalForegroundMs))
    put("total_visible_ms", JsonPrimitive(totalVisibleMs))
    put("total_foreground_service_ms", JsonPrimitive(totalForegroundServiceMs))
}

/**
 * 原文に書く粒度の名前。**Kotlin の識別子から導かない** ——
 * `name.lowercase()` にすると、定数の名前を直した日に**過去の全件が別の鍵になる**。
 * `when` を網羅にしてあるので、粒度を増やすときはここも書くことになる。
 */
internal val UsageGranularity.key: String
    get() = when (this) {
        UsageGranularity.DAILY -> "daily"
        UsageGranularity.WEEKLY -> "weekly"
        UsageGranularity.MONTHLY -> "monthly"
        UsageGranularity.YEARLY -> "yearly"
    }
