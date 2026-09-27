// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/**
 * 取得元が返したイベント 1 件を、契約どおりの記録 1 件にする
 * （tasks 3.1 / design D1 / D2 / 本人の決定 C1 / C2 / C12）。
 *
 * **1 イベント = 1 記録**（C1）。1 回の取得をまとめて 1 件にすると、重ねた窓で中身が変わって
 * 冪等キーが動き、再送のたびに行が増える。削除（ST22）・感度（ST24）の粒度もそこで失われる。
 *
 * **原文（`raw`）は取得元が返した値だけから組み立てる**（D2）。冪等キーは
 * `logical_source` + `event_time` + `raw` から作られるので、取得時点に解決した
 * [appLabel] を原文に入れると**アプリの更新・端末の言語の変更で鍵が変わり、
 * 同じイベントが行を増やす**。表示名は `payload` にだけ入れる（鍵に入らないので、
 * アプリを消した後でも「何だったか」が残る）。
 *
 * **並びと省略の規則は契約の一部**（`AppUsagePayloadShapeTest` が 1 文字単位で固定する）:
 *
 * | 欄 | 出所 | 省略 |
 * |---|---|---|
 * | `package` | `getPackageName()` | 無ければ欄ごと |
 * | `class` | `getClassName()` | 同上 |
 * | `event_type` | `getEventType()` | 省略しない |
 * | `event_time` | `getTimeStamp()` | 省略しない |
 * | `configuration` | `getConfiguration()` | 種別が違えば欄ごと |
 * | `shortcut_id` | `getShortcutId()` | 同上 |
 * | `interaction_action` | `getExtras()` の `EXTRA_EVENT_ACTION` | 同上 |
 * | `interaction_category` | `getExtras()` の `EXTRA_EVENT_CATEGORY` | 同上 |
 * | `standby_bucket` | `getAppStandbyBucket()` | 同上 |
 * | `app_label` | 取得時点の `PackageManager`（**`payload` だけ**） | 引けなければ欄ごと |
 *
 * **`null` の欄を置かない**のは、`null` が「取得元が返さなかった」と「取得元が null を返した」を
 * 区別しないため —— 欄ごと消せば、入っている欄は必ず取得元が返した値になる。
 *
 * 地域は**取得時点の端末のもの**（C12 / design D7（仮））。遡って取ったイベントの
 * 出来事の時点の地域は取得元が持っていないので引けない。失われるものは無い ——
 * `tz_id` / `tz_offset_min` は凍結の対象外で、後から引き直せる。
 * `tz_offset_min` はその地域での**その出来事の時点**のずれ（位置の記録と同じ計算）。
 */
fun UsageEventSnapshot.toIngestRequest(
    id: String,
    userId: String,
    deviceId: String,
    zone: ZoneId,
    appLabel: String?,
): IngestRequest {
    val fields = sourceFields()
    return IngestRequest(
        id = id,
        userId = userId,
        logicalSource = APP_USAGE_LOGICAL_SOURCE,
        externalId = null,
        deviceId = deviceId,
        origin = "collected",
        eventTime = at.toString(),
        tzOffsetMin = zone.rules.getOffset(at).totalSeconds / 60,
        tzId = zone.id,
        schemaVersion = 1,
        // 単位も座標系も持たない（測った量ではない）
        unitSystem = null,
        crs = null,
        // **原文は文字列**（design D16）。`ingestJson` で直列化するので、同じイベントは毎回同じ文字列になる
        raw = ingestJson.encodeToString(JsonObject.serializer(), JsonObject(fields)),
        payload = JsonObject(
            if (appLabel == null) fields else fields + (KEY_APP_LABEL to JsonPrimitive(appLabel)),
        ),
    )
}

/** 解析済みにだけ入る欄（取得元が返した値では**ない**ので、原文には入れない）。 */
private const val KEY_APP_LABEL = "app_label"

/**
 * 取得元が返した欄だけを、**固定の並び**で。共通の 4 つ → 種別ごとの 5 つ。
 *
 * `buildMap` は並びを保つ（`LinkedHashMap`）。並びが変われば原文の文字列が変わり、
 * **同じイベントが別の鍵で二重に入る**。
 */
private fun UsageEventSnapshot.sourceFields(): Map<String, JsonPrimitive> = buildMap {
    packageName?.let { put("package", JsonPrimitive(it)) }
    className?.let { put("class", JsonPrimitive(it)) }
    put("event_type", JsonPrimitive(eventType))
    put("event_time", JsonPrimitive(at.toString()))
    configuration?.let { put("configuration", JsonPrimitive(it.toString())) }
    shortcutId?.let { put("shortcut_id", JsonPrimitive(it)) }
    interactionAction?.let { put("interaction_action", JsonPrimitive(it)) }
    interactionCategory?.let { put("interaction_category", JsonPrimitive(it)) }
    standbyBucket?.let { put("standby_bucket", JsonPrimitive(it)) }
}

/**
 * gap の記録の種別（tasks 4.1 / design D4）。C-02 の `powered-off` と同じ型 ——
 * **「取りに行ったが取得元に無かった」を記録 1 件として残す**。
 *
 * イベントの原文には `kind` が無い（取得元が返す欄だけで組み立てる）ので、
 * この欄の有無そのものが 2 種類を分ける。
 */
const val USAGE_GAP_KIND: String = "gap"

/**
 * **取りに行った**が、取得元の見込みの保持（10 日）より前で取得元に無かった期間。
 *
 * spec が「取れなかった理由を含める」と定めるのは、扉 #14 の「データが無い」を
 * ②「動いていたが記録が無い」と見分けるため。**理由ごとに意味が違う**ので混ぜない。
 */
const val USAGE_GAP_REASON_RETENTION: String = "retention"

/**
 * 端末の時計の食い違いが 10 日を超えて解消せず、**諦めて**見込みの下限から再開した期間
 * （深掘り Q9=c / design D8 / code-verify R26）。
 *
 * [USAGE_GAP_REASON_RETENTION] と**分ける** —— こちらは 1 度も問い合わせていない期間で、
 * 取得元にはまだイベントが残っていたかもしれない。「取得元が消していた」と書くと、
 * 扉 #14 の材料（収集が壊れていたのか、取得元が消したのか）が嘘になる。
 */
const val USAGE_GAP_REASON_CLOCK_SKEW_ABANDONED: String = "clock_skew_abandoned"

/**
 * 取りに行ったが取得元に無かった期間を、記録 1 件にする（tasks 4.1 / design D4 / 本人の決定 Q4）。
 *
 * **アプリの名前もパッケージの名前も入れない**（spec）—— この 1 件が言えるのは
 * 「この期間は取得元に無かった」だけで、そこに何があったかは誰も知らない。
 * 推測で埋めると、無かったことが「あった」として正典に入る。
 *
 * **出来事の時刻は期間の終わり**（spec レビュー R2）。始まりに置くと収集開始日より前へ落ちうる。
 *
 * 原文の並びと省略の規則は契約の一部（`UsageGapTest` / `AppUsagePayloadShapeTest` と同じ扱い）——
 * 変えると同じ 1 件が別の鍵になって二重に入る。**解析済みは原文と同じ**（足す欄が無い）。
 */
fun usageGapRequest(
    id: String,
    userId: String,
    deviceId: String,
    zone: ZoneId,
    /** 期間の始まり＝既に取れているところ（重ねた幅ぶん手前ではない） */
    begin: Instant,
    /** 期間の終わり＝見込みの下限と「返った最古のイベント」の早いほう */
    end: Instant,
    reason: String = USAGE_GAP_REASON_RETENTION,
): IngestRequest {
    val fields = buildMap {
        put("kind", JsonPrimitive(USAGE_GAP_KIND))
        put("begin", JsonPrimitive(begin.toString()))
        put("end", JsonPrimitive(end.toString()))
        put("reason", JsonPrimitive(reason))
    }
    return IngestRequest(
        id = id,
        userId = userId,
        // **イベントと同じ論理ソース**（spec）—— 取りこぼしはそのソースの稼働状況そのもの
        logicalSource = APP_USAGE_LOGICAL_SOURCE,
        externalId = null,
        deviceId = deviceId,
        origin = "collected",
        eventTime = end.toString(),
        tzOffsetMin = zone.rules.getOffset(end).totalSeconds / 60,
        tzId = zone.id,
        schemaVersion = 1,
        unitSystem = null,
        crs = null,
        raw = ingestJson.encodeToString(JsonObject.serializer(), JsonObject(fields)),
        payload = JsonObject(fields),
    )
}
