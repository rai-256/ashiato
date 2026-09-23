// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 取りに行ったが取得元に無かった期間（tasks 4.1 / design D4 / 本人の決定 Q4）。
 *
 * **「携帯を使っていなかった」と「取りに行ったが消えていた」を分ける**のが主題 ——
 * 前者は窓の始まりが見込みの下限より後なので gap にならない。
 * 後者は扉 #14 の「データが無い」を②「動いていたが記録が無い」と見分けるための唯一の材料で、
 * **取りに行った瞬間にしか分からない。**
 */
@RunWith(RobolectricTestRunner::class)
class UsageGapTest {
    private val zone: ZoneId = ZoneId.of("Asia/Tokyo")

    /** 積まれた gap の記録だけ。**イベントの記録と同じ置き場に混ざっている**（C11） */
    private fun UsageTestEnv.gaps(): List<IngestRequest> =
        records().filter { it.rawText("kind") == USAGE_GAP_KIND }

    /** 13 日ぶん（見込みの下限 10 日を 3 日ぶん追い越す）。**壁時計と単調時計を一緒に進める** */
    private val thirteenDaysMs = 13L * 24 * 60 * 60 * 1000

    // Scenario: 見込みの下限より前から取ろうとすると gap が積まれる
    @Test
    fun `保存された終わりが下限より 3 日前なら、その 3 日が gap 1 件になる`() {
        val env = UsageTestEnv(
            // 返る中で最も古いイベントは見込みの下限（2026-05-23T09:00Z）より後
            source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-25T00:00:00Z"))),
        )
        // 1 回目は「保存された終わり」を作るためだけに回す（2026-05-20T09:00Z）
        env.collect()
        val before = env.gaps().size
        env.advance(thirteenDaysMs)

        val result = env.collect()

        assertTrue("取れたのに $result が返った", result is CollectionResult.Collected)
        val added = env.gaps().drop(before)
        assertEquals("gap が 1 件ではない: $added", 1, added.size)
        val gap = added.single()
        // 期間は**保存された窓の終わりから見込みの下限まで**（重ねた 60 秒は既に取れている）
        assertEquals("2026-05-20T09:00:00Z", gap.rawText("begin"))
        assertEquals("2026-05-23T09:00:00Z", gap.rawText("end"))
        assertEquals(APP_USAGE_LOGICAL_SOURCE, gap.logicalSource)
    }

    // Scenario: 見込みの下限の内側だけを取ったときは gap が積まれない
    @Test
    fun `保存された終わりが下限より後なら gap は積まれない`() {
        val env = UsageTestEnv(source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T09:10:00Z"))))
        env.collect()
        val before = env.gaps().size
        env.advance(USAGE_INTERVAL_MS)

        env.collect()

        assertEquals("単に使っていなかっただけの期間が gap になった", before, env.gaps().size)
    }

    // Scenario: 見込みより古いイベントが返ったときは gap が積まれない
    @Test
    fun `返った最古が保存された終わりと同じ時刻なら gap は積まれない`() {
        // 取得元が見込み（10 日）より長く持っていた場合。**取れているものを「取れなかった」と書かない**
        val kept = listOf(
            usageEvent("2026-05-20T09:00:00Z"),
            usageEvent("2026-05-21T09:00:00Z"),
            usageEvent("2026-05-22T09:00:00Z"),
            usageEvent("2026-05-23T08:00:00Z"),
        )
        val env = UsageTestEnv(source = FakeUsageSource(storedEvents = kept))
        env.collect()
        val before = env.gaps().size
        env.advance(thirteenDaysMs)

        env.collect()

        assertEquals("見込みが外れて長く持っていた分が gap になった", before, env.gaps().size)
        // その 3 日ぶんのイベントはすべて記録になっている
        assertEquals(
            kept.map { it.at.toString() },
            env.records().filter { it.rawText("kind") != USAGE_GAP_KIND }.map { it.eventTime },
        )
    }

    // Scenario: gap の記録にアプリの名前が入らない
    @Test
    fun `gap の記録に表示名もパッケージ名も入らない`() {
        val gap = usageGapRequest(
            id = "r1",
            userId = "user-1",
            deviceId = "device-1",
            zone = zone,
            begin = Instant.parse("2026-05-20T09:00:00Z"),
            end = Instant.parse("2026-05-23T09:00:00Z"),
        )
        val payload = ingestJson.encodeToString(JsonObject.serializer(), gap.payload)
        for (word in listOf("app_label", "package", "class", "例のアプリ", "dev.ashiato.example")) {
            assertFalse("原文に「$word」が入っている: ${gap.raw}", gap.raw.contains(word))
            assertFalse("解析済みに「$word」が入っている: $payload", payload.contains(word))
        }
    }

    // Scenario: gap の記録の出来事の時刻は期間の終わりである
    @Test
    fun `gap の出来事の時刻は期間の終わりである`() {
        val begin = Instant.parse("2026-05-20T09:00:00Z")
        val end = Instant.parse("2026-05-23T09:00:00Z")
        val gap = usageGapRequest("r1", "user-1", "device-1", zone, begin, end)

        // **始まりに置くと収集開始日より前へ落ちうる**（spec レビュー R2）
        assertEquals(end.toString(), gap.eventTime)
        assertEquals(end.toString(), gap.rawText("end"))
        // その地域でのその出来事の時点のずれ（位置の記録と同じ計算）
        assertEquals(zone.rules.getOffset(end).totalSeconds / 60, gap.tzOffsetMin)
    }
}
