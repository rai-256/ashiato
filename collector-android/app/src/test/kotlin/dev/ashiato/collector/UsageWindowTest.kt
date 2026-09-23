// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 取得の窓の進み方（tasks 3.2 / 本人の決定 C3 / C4 / C5 / C12 / design D7）。
 *
 * **「読めなかった」で窓を進めないことが主題** —— 進めてしまうと、その期間は
 * 取り直されないまま取得元の保持（見込み 10 日）を過ぎて消える。
 */
@RunWith(RobolectricTestRunner::class)
class UsageWindowTest {
    private val t0 = Instant.parse("2026-05-20T09:00:00Z")

    // Scenario: 境界のイベントが落ちない
    @Test
    fun `前回の窓の終わりとちょうど同じ時刻のイベントが次の契機で記録になる`() {
        // 取得元の範囲は `[begin, end)` —— 終わりちょうどのイベントは 1 回目では返らない
        val env = UsageTestEnv(source = FakeUsageSource(storedEvents = listOf(usageEvent(t0.toString()))))
        env.collect()
        assertEquals("1 回目で返ってはいけない", 0, env.records().size)
        assertEquals(t0, env.savedEnd())

        env.advance(USAGE_INTERVAL_MS)
        env.collect()
        assertEquals(listOf(t0.toString()), env.records().map { it.eventTime })
    }

    // Scenario: 読めなかったときは窓が進まない
    @Test
    fun `読めなかった契機のあとも保存された窓の終わりは変わらない`() {
        val source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T09:10:00Z")))
        val env = UsageTestEnv(source = source)
        env.collect()
        val saved = env.savedEnd()
        assertEquals(t0, saved)

        // 再起動の後に一度も解錠されていない端末。**0 件ではない**
        source.unreadable = UsageStatsSource.REASON_LOCKED
        env.advance(USAGE_INTERVAL_MS)
        val result = env.collect()

        assertTrue("読めなかったのに $result が返った", result is CollectionResult.Unavailable)
        assertEquals(UsageStatsSource.REASON_LOCKED, (result as CollectionResult.Unavailable).reason)
        assertEquals("窓が進んでいる", saved, env.savedEnd())
        assertEquals("読めなかったのに記録ができている", 0, env.records().size)

        // 読めるようになれば、同じ窓の始まりから取り直せる（イベントは失われていない）
        source.unreadable = null
        env.advance(USAGE_INTERVAL_MS)
        env.collect()
        assertEquals(listOf("2026-05-20T09:10:00Z"), env.records().map { it.eventTime })
    }

    // Scenario: 0 件のときは窓が進む
    @Test
    fun `0 件の契機では窓の終わりが今回の終わりへ進む`() {
        val env = UsageTestEnv(source = FakeUsageSource())
        env.collect()
        assertEquals(t0, env.savedEnd())

        env.advance(USAGE_INTERVAL_MS)
        val result = env.collect()
        assertTrue("0 件は成功のはずが $result", result is CollectionResult.Collected)
        assertEquals(emptyList<IngestRequest>(), (result as CollectionResult.Collected).enqueued)
        assertEquals(t0.plusMillis(USAGE_INTERVAL_MS), env.savedEnd())
    }

    // Scenario: 窓の終わりは収集の停止と再開をまたいで残る
    @Test
    fun `止まって再び始まっても次の窓は保存した終わりから引かれる`() {
        val source = FakeUsageSource()
        val env = UsageTestEnv(source = source)
        env.collect()
        val savedBeforeStop = env.savedEnd()
        assertEquals(t0, savedBeforeStop)

        env.restart()                       // 収集が止まって、また始まる
        env.advance(USAGE_INTERVAL_MS)
        env.collect()

        val asked = source.eventQueries.last()
        assertEquals("窓の終わりは今回の契機", t0.plusMillis(USAGE_INTERVAL_MS), asked.end)
        // **保存した終わりの手前から**（本人の決定 C4）。新品として始まっていない
        assertTrue("保存した終わりより手前から取っていない: $asked", asked.begin.isBefore(savedBeforeStop))
        assertTrue(
            "保存した終わりと関係のないところから取っている: $asked",
            asked.begin.isAfter(savedBeforeStop!!.minusMillis(USAGE_INTERVAL_MS)),
        )
    }

    // Scenario: 遡って取った記録の地域は取得時点の端末の地域である
    @Test
    fun `3 日前のイベントの記録は取得時点の端末の地域を持つ`() {
        val threeDaysAgo = t0.minus(java.time.Duration.ofDays(3))
        val tokyo = UsageTestEnv(
            source = FakeUsageSource(storedEvents = listOf(usageEvent(threeDaysAgo.toString()))),
            zone = ZoneId.of("Asia/Tokyo"),
        )
        tokyo.collect()
        val inTokyo = tokyo.records().single()
        assertEquals(threeDaysAgo.toString(), inTokyo.eventTime)
        assertEquals("Asia/Tokyo", inTokyo.tzId)
        assertEquals(540, inTokyo.tzOffsetMin)

        // **同じイベントでも、取得した端末の地域が違えば違う地域が付く**（出来事の時点の地域ではない）
        val kolkata = UsageTestEnv(
            source = FakeUsageSource(storedEvents = listOf(usageEvent(threeDaysAgo.toString()))),
            zone = ZoneId.of("Asia/Kolkata"),
        )
        kolkata.collect()
        val inKolkata = kolkata.records().single()
        assertEquals("Asia/Kolkata", inKolkata.tzId)
        assertEquals(330, inKolkata.tzOffsetMin)
    }

    /**
     * 初回は**こちらで短く切らない**（本人の決定 C3 / spec「窓の始まりを切り詰めない」）——
     * 見込みの保持（10 日）は API から読めないので、見込みより**手前**から問い合わせる。
     */
    @Test
    fun `初回の窓は見込みの保持より手前から引かれる`() {
        val source = FakeUsageSource()
        val env = UsageTestEnv(source = source)
        env.collect()
        val asked = source.eventQueries.single()
        assertEquals(t0, asked.end)
        assertTrue(
            "初回の窓が見込みの下限（10 日）より内側に切られている: $asked",
            UsageRetention.eventsFloor(t0).excludes(asked.begin),
        )
    }

    /** 保存された終わりは**端末のファイル**にある（インスタンスの中だけではない）。 */
    @Test
    fun `窓の終わりはファイルに残る`() {
        val env = UsageTestEnv(source = FakeUsageSource())
        env.collect()
        val file = usageWindowFile(env.dir, APP_USAGE_LOGICAL_SOURCE)
        assertTrue("窓の置き場が無い: $file", file.exists())
        assertNotNull(UsageWindowStore(file, APP_USAGE_LOGICAL_SOURCE) {}.load())
        assertEquals(t0, UsageWindowStore(file, APP_USAGE_LOGICAL_SOURCE) {}.load()!!.end)
    }

    /** 読めない置き場は**新品として始める**（落とさない）。窓が無いのと同じ扱い。 */
    @Test
    fun `壊れた置き場は新品として始まる`() {
        val env = UsageTestEnv(source = FakeUsageSource())
        val file = usageWindowFile(env.dir, APP_USAGE_LOGICAL_SOURCE)
        file.parentFile?.mkdirs()
        file.writeText("こわれている")
        val store = UsageWindowStore(file, APP_USAGE_LOGICAL_SOURCE) { env.lines += it }
        assertEquals(null, store.load())
        assertTrue("読めなかったことがログに出ていない", env.lines.any { it.contains("usage_window_unreadable") })
    }
}
