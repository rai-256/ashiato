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

    /**
     * **長い放置は時計の飛びではない**（controller の裁定 2026-09-23 / 誤検知の修正）。
     *
     * [AgeClock] は起動をまたぐ前進を 30 日で頭打ちにするので（`MAX_REBOOT_GAP_MS`）、
     * 60 日ぶりに起動した端末は**時計が 1 秒も飛んでいなくても 30 日の食い違い**を見せる。
     * これを飛びと読むと窓の印が更新されないまま差も縮まらず、**取得は永久に止まる**。
     */
    @Test
    fun `30 日より長く電源を切って放置してから起動しても窓は進む`() {
        val env = env()
        env.collect()
        assertEquals(t0, env.savedEnd())

        env.clock.reboot(wallGapMs = 60 * AgeClock.DAY_MS)   // 60 日ぶりに電源が入った
        env.restart()                                        // プロセスも作り直される
        val result = env.collect()

        assertTrue("放置から起動しただけなのに $result", result is CollectionResult.Collected)
        assertEquals("窓が進んでいない", env.now, env.savedEnd())
        assertEquals(
            listOf("2026-05-20T09:10:00Z", "2026-05-20T10:30:00Z"),
            env.records().map { it.eventTime },
        )
    }

    /**
     * ただし**起動をまたいでも壁時計が戻ったぶんは証拠になる** ——
     * 跨ぎの前進は 0 で丸められているので、負の食い違いは「時計が戻った」ことそのもの。
     */
    @Test
    fun `再起動をまたいで時計が戻ったときは窓が進まない`() {
        val env = env()
        env.collect()
        val saved = env.savedEnd()

        env.clock.reboot(wallGapMs = -3 * 60 * 60 * 1000L)
        env.restart()
        val result = env.collect()

        assertTrue("時計が戻ったのに $result", result is CollectionResult.Unavailable)
        // 窓が組み立てられないほうの拒み方（`window_ahead`）ではなく、**時計の飛びとして**断っている
        assertEquals(
            AppUsageSourceAdapter.REASON_CLOCK_SKEW,
            (result as CollectionResult.Unavailable).reason,
        )
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
