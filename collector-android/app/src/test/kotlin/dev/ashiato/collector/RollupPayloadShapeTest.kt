// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 集計の `raw` と `payload` の形を**1 文字単位で固定する**（tasks 4.4 / design のリスク「集計の重複」）。
 *
 * **集計は取得のたびに同じ期間を返す** —— 日ごとの箱は 6 時間ごとに 4 回読み直される。
 * 毎回同じ原文でなければ、同じ 1 件が別の冪等キーになって**取得のたびに行が増える**。
 * ここが落ちたときに直すのは実装のほうで、期待値ではない。
 */
@RunWith(RobolectricTestRunner::class)
class RollupPayloadShapeTest {
    private val zone: ZoneId = ZoneId.of("Asia/Tokyo")

    private fun request(
        rollup: UsageRollupSnapshot,
        granularity: UsageGranularity = UsageGranularity.DAILY,
        label: String? = "例のアプリ",
        id: String = "r1",
    ) = rollup.toIngestRequest(id, "user-1", "device-1", zone, granularity, label)

    private fun payloadText(request: IngestRequest) =
        ingestJson.encodeToString(JsonObject.serializer(), request.payload)

    // Scenario: 同じ集計の原文は毎回同じ文字列になる
    @Test
    fun `同じ粒度と期間とアプリから 2 回作った原文が 1 バイトも違わない`() {
        val rollup = usageRollup()
        // **識別子は毎回違う**（1 件ごとに振る）。原文はそれに影響されない
        val first = request(rollup, id = "r1")
        val second = request(rollup, id = "r2")

        assertEquals(first.raw, second.raw)
        assertArrayEqualsBytes(first.raw, second.raw)
        // 取得時点に解決した表示名は**原文に入らない**ので、変わっても鍵は動かない
        val relabelled = request(rollup, label = "べつの名前", id = "r3")
        assertEquals("表示名が原文に混ざっている", first.raw, relabelled.raw)
    }

    private fun assertArrayEqualsBytes(a: String, b: String) =
        assertEquals(a.toByteArray(Charsets.UTF_8).toList(), b.toByteArray(Charsets.UTF_8).toList())

    /** 欄の並びと名前を固定する。**並びが変われば同じ 1 件が別の鍵になる。** */
    @Test
    fun `集計 1 件の形`() {
        val request = request(
            UsageRollupSnapshot(
                packageName = "dev.ashiato.example",
                firstAt = java.time.Instant.parse("2026-05-01T00:00:00Z"),
                lastAt = java.time.Instant.parse("2026-05-02T00:00:00Z"),
                lastUsedAt = java.time.Instant.parse("2026-05-01T22:10:00Z"),
                lastVisibleAt = java.time.Instant.parse("2026-05-01T22:11:00Z"),
                lastForegroundServiceUsedAt = java.time.Instant.parse("2026-05-01T20:00:00Z"),
                totalForegroundMs = 3_600_000,
                totalVisibleMs = 4_200_000,
                totalForegroundServiceMs = 120_000,
            ),
        )
        val requiredOnly = request(usageRollup(packageName = "x").copy(packageName = null), label = null)
        assertPayloadMatchesContract(APP_USAGE_ROLLUP_LOGICAL_SOURCE, request.payload, requiredOnly.payload)
        assertEquals(
            """{"granularity":"daily","package":"dev.ashiato.example",""" +
                """"begin":"2026-05-01T00:00:00Z","end":"2026-05-02T00:00:00Z",""" +
                """"last_used":"2026-05-01T22:10:00Z","last_visible":"2026-05-01T22:11:00Z",""" +
                """"last_foreground_service_used":"2026-05-01T20:00:00Z",""" +
                """"total_foreground_ms":3600000,"total_visible_ms":4200000,""" +
                """"total_foreground_service_ms":120000}""",
            request.raw,
        )
        assertEquals(
            """{"granularity":"daily","package":"dev.ashiato.example",""" +
                """"begin":"2026-05-01T00:00:00Z","end":"2026-05-02T00:00:00Z",""" +
                """"last_used":"2026-05-01T22:10:00Z","last_visible":"2026-05-01T22:11:00Z",""" +
                """"last_foreground_service_used":"2026-05-01T20:00:00Z",""" +
                """"total_foreground_ms":3600000,"total_visible_ms":4200000,""" +
                """"total_foreground_service_ms":120000,"app_label":"例のアプリ"}""",
            payloadText(request),
        )
    }

    /** 粒度の名前は**4 つとも違う**（取り違えると別の箱が同じ鍵で畳まれる）。 */
    @Test
    fun `粒度の名前は 4 つとも違う`() {
        val names = UsageGranularity.entries.map { request(usageRollup(), granularity = it).rawText("granularity") }
        assertEquals(listOf("daily", "weekly", "monthly", "yearly"), names)
        assertEquals("粒度の名前が重なっている", names.size, names.distinct().size)
    }

    /**
     * 封筒（出来事の時刻・地域）を固定する（独立レビュー R6）。**印は置かない** ——
     * spec にこの Scenario は無く、ここは KDoc にしか無い決定の回帰の受け皿。
     *
     * **出来事の時刻は箱の終わり**（gap の記録と同じ規律）。始まりに置くと、
     * 年ごとの箱が 2 年前へ落ちて「いつの記録か」が粒度ごとにばらける。
     * **地域は取得時点の端末のもの**（C12）—— 集計は最大 2 年を遡るが、
     * その時点の地域は取得元が持っていない。
     */
    @Test
    fun `集計の記録の封筒は箱の終わりと取得時点の地域で決まる`() {
        val rollup = usageRollup(firstAt = "2025-01-01T00:00:00Z", lastAt = "2026-01-01T00:00:00Z")
        val request = request(rollup, granularity = UsageGranularity.YEARLY)

        assertEquals("2026-01-01T00:00:00Z", request.eventTime)
        assertEquals(rollup.lastAt.toString(), request.eventTime)
        assertEquals("Asia/Tokyo", request.tzId)
        assertEquals(zone.rules.getOffset(rollup.lastAt).totalSeconds / 60, request.tzOffsetMin)
        assertEquals(540, request.tzOffsetMin)
        // 端末の収集なので外部識別子は持たない（登録簿は `external_id_kind = 'none'`）
        assertEquals(null, request.externalId)
        assertEquals("collected", request.origin)
        assertEquals(APP_USAGE_ROLLUP_LOGICAL_SOURCE, request.logicalSource)
    }

    /** パッケージ名が無ければ**欄ごと省く**（`null` を置かない。イベントと同じ規律）。 */
    @Test
    fun `パッケージ名が無ければ欄ごと省く`() {
        val request = request(usageRollup(packageName = "x").copy(packageName = null))
        assertFalse("package の欄が残っている: ${request.raw}", request.raw.contains("package"))
    }
}
