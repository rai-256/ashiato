// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 端末の時計が飛んだときの窓（tasks 3.3 / 本人の決定 C6 / design D5 のリスク R4）。
 *
 * **取得元は時計の変化を検知すると保持している統計を丸ごとずらす**（`onTimeChanged`）。
 * ずれた時刻で取り直すと、出来事の時刻が変わった同じイベントが畳まれずに行を増やし、
 * 出来事の時刻は凍結されていて後から直せない。だから飛んでいる間は取りに行かない。
 *
 * 単調な経過は [AgeClock]（ST04）から取る。**新しい時計を足さない。**
 */
@RunWith(RobolectricTestRunner::class)
class UsageWindowClockTest {
    private val t0 = Instant.parse("2026-05-20T09:00:00Z")

    private fun env() = UsageTestEnv(
        source = FakeUsageSource(
            storedEvents = listOf(
                usageEvent("2026-05-20T09:10:00Z"),
                usageEvent("2026-05-20T10:30:00Z"),
            ),
        ),
    )

    // Scenario: 時計が飛んでいる間は取り直さない
    @Test
    fun `時計が単調な経過と食い違う契機では窓が進まず記録もできない`() {
        val env = env()
        env.collect()
        val saved = env.savedEnd()
        assertEquals(t0, saved)
        assertEquals(0, env.records().size)

        // 30 分たったが、そのあいだに端末の時刻が 2 時間先へ動いた（単調な経過は 30 分のまま）
        env.advance(USAGE_INTERVAL_MS)
        env.jumpWall(2 * 60 * 60 * 1000L)
        val result = env.collect()

        assertTrue("時計が飛んでいるのに $result が返った", result is CollectionResult.Unavailable)
        assertEquals("窓が進んでいる", saved, env.savedEnd())
        assertEquals("時計が飛んでいるのに記録ができている", 0, env.records().size)
        // 取りに行ってすらいない（取得元に問い合わせると、ずれた時刻のイベントが返る）
        assertEquals("飛んでいる間に取りに行っている", 1, (env.source as FakeUsageSource).eventQueries.size)
    }

    /**
     * ガードが**何かを見ている**ことの裏取り —— 飛んでいなければ同じ契機で普通に取れる。
     * これが無いと「常に取らない」実装でも上の試験は緑になる。
     */
    @Test
    fun `時計が飛んでいなければ同じ契機で取れる`() {
        val env = env()
        env.collect()
        env.advance(USAGE_INTERVAL_MS)
        val result = env.collect()
        assertTrue("取れるはずが $result", result is CollectionResult.Collected)
        assertEquals(listOf("2026-05-20T09:10:00Z"), env.records().map { it.eventTime })
        assertEquals(t0.plusMillis(USAGE_INTERVAL_MS), env.savedEnd())
    }

    /** 時計が**戻った**ときも同じ（窓の始まりが現在時刻を超えると取得元は `null` を返す）。 */
    @Test
    fun `時計が戻った契機でも窓が進まない`() {
        val env = env()
        env.collect()
        val saved = env.savedEnd()
        env.advance(USAGE_INTERVAL_MS)
        env.jumpWall(-3 * 60 * 60 * 1000L)
        val result = env.collect()
        assertTrue("時計が戻ったのに $result が返った", result is CollectionResult.Unavailable)
        assertEquals(saved, env.savedEnd())
        assertEquals(0, env.records().size)
    }

    /** 飛びが直れば、取り直しは**保存した終わりから**続く（飛んだ間のイベントは失われない）。 */
    @Test
    fun `飛びが直れば保存した終わりから続きを取る`() {
        val env = env()
        env.collect()
        env.advance(USAGE_INTERVAL_MS)
        env.jumpWall(2 * 60 * 60 * 1000L)
        env.collect()                        // 飛んでいる間は取らない
        env.jumpWall(-2 * 60 * 60 * 1000L)   // 時刻が直った
        env.advance(USAGE_INTERVAL_MS)
        val result = env.collect()
        assertTrue("直ったのに $result", result is CollectionResult.Collected)
        assertEquals(listOf("2026-05-20T09:10:00Z"), env.records().map { it.eventTime })
        val asked = (env.source as FakeUsageSource).eventQueries.last()
        assertTrue("保存した終わりより手前から取っていない: $asked", asked.begin.isBefore(t0))
    }
}
