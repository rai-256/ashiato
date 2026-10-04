// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.res.Configuration
import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * アプリ利用の `raw` と `payload` の形を**1 文字単位で固定する**
 * （tasks 3.4 / design D2 / C-02 の `payload_shape_is_pinned` と同じ扱い）。
 *
 * **並びと省略の規則が変われば、同じ 1 件が別の鍵になって二重に入る** ——
 * 冪等キーは `logical_source` + `event_time` + `raw` から作られる。
 * ここが落ちたときに直すのは実装のほうで、期待値ではない
 * （期待値を直すなら、それは契約を変えるということ。`docs/collector-contract.md` も一緒に直す）。
 */
@RunWith(RobolectricTestRunner::class)
class AppUsagePayloadShapeTest {
    private val zone: ZoneId = ZoneId.of("Asia/Tokyo")
    private val at = "2026-05-20T09:10:00Z"

    private fun request(event: UsageEventSnapshot, label: String? = "例のアプリ") =
        event.toIngestRequest("r1", "user-1", "device-1", zone, label)

    private fun payloadText(request: IngestRequest) =
        ingestJson.encodeToString(JsonObject.serializer(), request.payload)

    /** 共通の 4 欄だけを持つ 1 件（種別ごとの欄は**欄ごと省く**）。 */
    @Test
    fun `種別ごとの欄が無い 1 件の形`() {
        val request = request(usageEvent(at))
        assertEquals(
            """{"package":"dev.ashiato.example","class":"dev.ashiato.example.MainActivity",""" +
                """"event_type":1,"event_time":"2026-05-20T09:10:00Z"}""",
            request.raw,
        )
        assertEquals(
            """{"package":"dev.ashiato.example","class":"dev.ashiato.example.MainActivity",""" +
                """"event_type":1,"event_time":"2026-05-20T09:10:00Z","app_label":"例のアプリ"}""",
            payloadText(request),
        )
    }

    /** 種別ごとの欄が付く 1 件。**並びは取得元が公開している順**（共通の 4 つ → 種別ごとの 5 つ）。 */
    @Test
    fun `種別ごとの欄が付く 1 件の形`() {
        val request = request(fullUsageEvent(at))
        assertEquals(
            """{"package":"dev.ashiato.example","class":"dev.ashiato.example.MainActivity",""" +
                """"event_type":7,"event_time":"2026-05-20T09:10:00Z","shortcut_id":"shortcut-1",""" +
                """"interaction_action":"android.intent.action.VIEW",""" +
                """"interaction_category":"android.intent.category.DEFAULT","standby_bucket":20}""",
            request.raw,
        )
        assertEquals(
            """{"package":"dev.ashiato.example","class":"dev.ashiato.example.MainActivity",""" +
                """"event_type":7,"event_time":"2026-05-20T09:10:00Z","shortcut_id":"shortcut-1",""" +
                """"interaction_action":"android.intent.action.VIEW",""" +
                """"interaction_category":"android.intent.category.DEFAULT","standby_bucket":20,""" +
                """"app_label":"例のアプリ"}""",
            payloadText(request),
        )
    }

    /**
     * 設定の変化は**時刻の次**に入る。中身は端末と OS の版で変わる文字列なので、
     * ここで固定するのは**位置と逃げ方（JSON の文字列としての包み方）**だけ。
     */
    @Test
    fun `設定の変化は時刻の次に入る`() {
        val configuration = Configuration().apply { orientation = Configuration.ORIENTATION_LANDSCAPE }
        val request = request(fullUsageEvent(at, configuration))
        val requiredOnly = request(
            UsageEventSnapshot(packageName = null, className = null, eventType = 2, at = Instant.parse(at)),
            label = null,
        )
        assertPayloadMatchesContract(APP_USAGE_LOGICAL_SOURCE, request.payload, requiredOnly.payload)
        val quoted = ingestJson.encodeToString(JsonPrimitive.serializer(), JsonPrimitive(configuration.toString()))
        assertEquals(
            """{"package":"dev.ashiato.example","class":"dev.ashiato.example.MainActivity",""" +
                """"event_type":7,"event_time":"2026-05-20T09:10:00Z","configuration":$quoted,""" +
                """"shortcut_id":"shortcut-1","interaction_action":"android.intent.action.VIEW",""" +
                """"interaction_category":"android.intent.category.DEFAULT","standby_bucket":20}""",
            request.raw,
        )
    }

    /** 引けなかった欄は**欄ごと消える**（`null` を置かない）。表示名も同じ規則。 */
    @Test
    fun `引けなかった欄は欄ごと消える`() {
        val bare = UsageEventSnapshot(packageName = null, className = null, eventType = 2, at = Instant.parse(at))
        val request = request(bare, label = null)
        assertEquals("""{"event_type":2,"event_time":"2026-05-20T09:10:00Z"}""", request.raw)
        assertEquals("""{"event_type":2,"event_time":"2026-05-20T09:10:00Z"}""", payloadText(request))
    }

    private fun gap(reason: String) = usageGapRequest(
        "r1",
        "user-1",
        "device-1",
        zone,
        begin = Instant.parse("2026-05-10T09:00:00Z"),
        end = Instant.parse("2026-05-20T09:00:00Z"),
        reason = reason,
    )

    /**
     * **取りこぼし（`kind: gap`）の形も契約表から読む**（tasks 7.3 / code-verify R32）。
     *
     * 同じ論理ソースに積まれるのに契約表に載っていなかった —— 受け手（ST14）は
     * この契約を読んで gap を畳むので、表に無い形は**読み手が知らない形**になる。
     */
    @Test
    fun `取りこぼしの 1 件の形`() {
        val request = gap(USAGE_GAP_REASON_RETENTION)
        assertGapMatchesContract(request)
        assertEquals(
            """{"kind":"gap","begin":"2026-05-10T09:00:00Z",""" +
                """"end":"2026-05-20T09:00:00Z","reason":"retention"}""",
            request.raw,
        )
        // **出来事の時刻は期間の終わり**（spec レビュー R2）。アプリの名前は入らない
        assertEquals("2026-05-20T09:00:00Z", request.eventTime)
        assertEquals(APP_USAGE_LOGICAL_SOURCE, request.logicalSource)
    }

    /**
     * **諦めた期間の理由も契約表に載っている**（code-verify R26 / R32）。
     * 表に `retention` しか無ければ、こちらは受け手の知らない値として落ちる。
     */
    @Test
    fun `諦めた期間の取りこぼしも契約表のとおり`() {
        val request = gap(USAGE_GAP_REASON_CLOCK_SKEW_ABANDONED)
        assertGapMatchesContract(request)
        assertEquals(
            """{"kind":"gap","begin":"2026-05-10T09:00:00Z",""" +
                """"end":"2026-05-20T09:00:00Z","reason":"clock_skew_abandoned"}""",
            request.raw,
        )
    }

    /** 契約表が数え上げる理由と、実装が送りうる理由が**過不足なく**一致する。 */
    @Test
    fun `契約表の理由の列挙が実装と一致する`() {
        assertEquals(
            listOf(USAGE_GAP_REASON_RETENTION, USAGE_GAP_REASON_CLOCK_SKEW_ABANDONED),
            contractGapReasons(),
        )
    }

    /** エンベロープ（契約の外枠）も固定する。**取得時点の端末の地域が付く**（design D7）。 */
    @Test
    fun `エンベロープの形`() {
        val request = request(usageEvent(at))
        assertEquals(APP_USAGE_LOGICAL_SOURCE, request.logicalSource)
        assertEquals("collected", request.origin)
        assertEquals("2026-05-20T09:10:00Z", request.eventTime)
        assertEquals("Asia/Tokyo", request.tzId)
        assertEquals(540, request.tzOffsetMin)
        assertEquals(1, request.schemaVersion)
        assertEquals("device-1", request.deviceId)
        assertEquals(null, request.externalId)
        // 単位も座標系も持たない（位置ではない）
        assertEquals(null, request.unitSystem)
        assertEquals(null, request.crs)
    }
}
