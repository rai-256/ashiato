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

    @Test
    fun `壁時計だけが大きく飛んでも経過が10日以内なら再開しない`() {
        val env = UsageTestEnv(FakeUsageSource())
        env.collect()
        env.jumpWall(40 * AgeClock.DAY_MS)
        env.advance(10 * AgeClock.DAY_MS)
        assertTrue(env.collect() is CollectionResult.Unavailable)
        assertEquals(t0, env.savedEnd())
        assertEquals(1, (env.source as FakeUsageSource).eventQueries.size)
    }

    // Scenario: 時計の食い違いが10日を超えても解消しないときは自動で取得を再開する
    @Test
    fun `10日を超える停止は再起動後も下限から再開して諦めた期間をgapに残す`() {
        val source = FakeUsageSource(storedEvents = listOf(
            usageEvent("2026-05-20T10:00:00Z"),
            usageEvent("2026-05-21T12:00:00Z"),
        ))
        val env = UsageTestEnv(source)
        env.collect()
        env.jumpWall(2 * 60 * 60 * 1000L)
        env.advance(10 * AgeClock.DAY_MS)
        assertTrue(env.collect() is CollectionResult.Unavailable)
        env.restart()
        env.advance(AgeClock.DAY_MS)
        assertTrue(env.collect() is CollectionResult.Collected)
        assertEquals(Instant.parse("2026-05-21T11:00:00Z"), source.eventQueries.last().begin)
        assertEquals(env.now, env.savedEnd())
        val gap = env.records().single { it.rawText("kind") == USAGE_GAP_KIND }
        assertEquals("2026-05-20T09:00:00Z", gap.rawText("begin"))
        assertEquals("2026-05-21T11:00:00Z", gap.rawText("end"))
        assertEquals("2026-05-21T11:00:00Z", gap.eventTime)
        // **理由は「取得元に無かった」ではない**（code-verify R26）——
        // この期間は 1 度も問い合わせていない。取得元にはまだイベントが残っていた
        assertEquals(USAGE_GAP_REASON_CLOCK_SKEW_ABANDONED, gap.rawText("reason"))
        assertEquals(listOf("2026-05-21T12:00:00Z"),
            env.records().filter { it.rawText("kind") != USAGE_GAP_KIND }.map { it.eventTime })
        env.advance(USAGE_INTERVAL_MS)
        assertTrue(env.collect() is CollectionResult.Collected)
    }

    @Test
    fun `自動再開のgapを書けなければ窓を保ち次の契機で積み直す`() {
        val env = UsageTestEnv(FakeUsageSource())
        env.collect()
        env.jumpWall(2 * 60 * 60 * 1000L)
        env.advance(11 * AgeClock.DAY_MS)
        env.blockOutbox()
        env.restart()
        assertTrue(env.collect() is CollectionResult.Collected)
        assertEquals(t0, env.savedEnd())
        assertTrue(env.lines.any { it.startsWith("kind=usage_window_held") })
        env.outbox = testOutbox()
        env.restart()
        assertTrue(env.collect() is CollectionResult.Collected)
        assertEquals(env.now, env.savedEnd())
        assertEquals("2026-05-20T09:00:00Z", env.records().single().rawText("begin"))
        assertEquals("2026-05-21T11:00:00Z", env.records().single().rawText("end"))
        assertEquals(USAGE_GAP_REASON_CLOCK_SKEW_ABANDONED, env.records().single().rawText("reason"))
    }

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
            env.records().filter { it.rawText("kind") != USAGE_GAP_KIND }.map { it.eventTime },
        )
        // 保存された終わり（09:00）から**返った中で最も古いイベント**（09:10）までは
        // 取りに行って取得元に無かった期間なので、gap が 1 件積まれる（tasks 4.1 / design D4）。
        // 本物の端末では 60 日前のイベントは残っていないので、この期間は見込みの下限まで伸びる
        assertEquals(1, env.records().count { it.rawText("kind") == USAGE_GAP_KIND })
    }

    /**
     * **再起動のあとに時計が前へ飛んだら、跨いでいても断る**（独立レビュー Important 1）。
     *
     * RTC が狂った端末が過去の時刻で起動し、契機の前に網の時刻合わせで壁時計が数時間前へ飛ぶ ——
     * 取得元はそこで保持している統計を丸ごとずらすので、取り直すと出来事の時刻が変わった
     * 同じイベントが行を増やす。**「跨いだか」だけで免除すると、再起動ごとに 1 契機ぶん素通りする。**
     * 免除してよいのは [AgeClock] が**数えなかった前進の分だけ**。
     */
    @Test
    fun `再起動の直後に時計が前へ飛んでいたら窓は進まない`() {
        val env = env()
        env.collect()
        val saved = env.savedEnd()

        // 1 分止まって起動。**起動の直後にプロセスが時計を読む**（置き場を開く・上限を見回る）
        env.clock.reboot(wallGapMs = 60_000)
        env.restart()
        env.age.now()
        // 網の時刻合わせで壁時計が 5 時間前へ飛んだ（単調な経過は 1 分しか進んでいない）
        env.advance(60_000)
        env.jumpWall(5 * 60 * 60 * 1000L)
        val result = env.collect()

        assertTrue("時計が前へ飛んだのに $result", result is CollectionResult.Unavailable)
        assertEquals(
            AppUsageSourceAdapter.REASON_CLOCK_SKEW,
            (result as CollectionResult.Unavailable).reason,
        )
        assertEquals("窓が進んでいる", saved, env.savedEnd())
        assertEquals("ずれた時刻の統計を取り直している", 0, env.records().size)
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

    /**
     * **閾値の内側で時計が戻ったとき、戻る前後のイベントを失わない**（code-verify R25）。
     *
     * 取得元は時計の変化を知ると保持している統計を**丸ごとその差だけずらす**ので、
     * 保存した終わり（09:00）は取得元の側では 08:10 にある。ずらさずに 08:59 から
     * 問い合わせると窓が組み立てられず（`usage_window_ahead`）、やがて時計が追いつくころには
     * **戻る前の 1 件も戻った後の 1 件もどの記録にも gap にもならない**（実測のプローブ）。
     *
     * 食い違い（`skewMs`）は**取得元がずらした幅そのもの**なので、
     * 「保存した終わり ＋ 食い違い」を次の窓の始まりにする（機械で決まる）。
     */
    @Test
    fun `閾値の内側で時計が50分戻っても戻る前後のイベントを取る`() {
        val source = FakeUsageSource(
            storedEvents = listOf(
                usageEvent("2026-05-20T09:10:00Z"),   // 戻る前に起きた 1 件
                usageEvent("2026-05-20T09:25:00Z"),   // 戻った後に起きた 1 件（ずらされた後は 08:35）
            ),
        )
        val env = UsageTestEnv(source)
        env.collect()                          // 09:00 を保存（この時点ではまだ 1 件も無い）
        env.advance(20 * 60 * 1000L)           // 09:20
        source.shiftMs = -50 * 60 * 1000L      // 取得元も統計を 50 分ずらす
        env.jumpWall(-50 * 60 * 1000L)         // 端末の時計が 08:30 へ戻った
        env.advance(10 * 60 * 1000L)           // 08:40 の契機

        val result = env.collect()

        assertTrue("50 分の戻りで $result", result is CollectionResult.Collected)
        assertEquals(
            listOf("2026-05-20T08:20:00Z", "2026-05-20T08:35:00Z"),
            env.records().filter { it.rawText("kind") != USAGE_GAP_KIND }.map { it.eventTime },
        )
    }

    /**
     * **閾値の内側で時計が進んだとき、同じイベントを 2 行にしない**（code-verify R25）。
     *
     * 取得元がずらした後の 09:40 は、ずらす前の 08:50 と**同じ 1 件**。
     * 出来事の時刻は凍結されるので、2 行になったら後から畳めない。
     */
    @Test
    fun `閾値の内側で時計が50分進んでも同じイベントが2行にならない`() {
        val source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T08:50:00Z")))
        val env = UsageTestEnv(source)
        env.collect()                          // 初回の窓で 08:50 の 1 件を取る
        assertEquals(listOf("2026-05-20T08:50:00Z"), env.records().map { it.eventTime })
        env.advance(20 * 60 * 1000L)           // 09:20
        source.shiftMs = 50 * 60 * 1000L       // 取得元も統計を 50 分ずらす
        env.jumpWall(50 * 60 * 1000L)          // 端末の時計が 10:10 へ進んだ
        env.advance(10 * 60 * 1000L)           // 10:20 の契機

        env.collect()

        assertEquals(
            "ずらされた同じ 1 件が別の時刻で 2 行になった",
            listOf("2026-05-20T08:50:00Z"),
            env.records().filter { it.rawText("kind") != USAGE_GAP_KIND }.map { it.eventTime },
        )
    }

    /**
     * **10 日ちょうどでは再開せず、その次の契機（30 分後）で再開する**（code-verify R35）。
     *
     * 「10 日を超えたら」の境界を 1 契機の幅で挟む —— 10 日と 11 日の 2 点だけを見ていたときは、
     * 閾値を (10 日, 11 日] のどこに置いても全部緑だった。
     */
    @Test
    fun `10日を1契機超えたところで再開する`() {
        val env = UsageTestEnv(FakeUsageSource())
        env.collect()
        env.jumpWall(2 * 60 * 60 * 1000L)
        env.advance(10 * AgeClock.DAY_MS)
        assertTrue("10 日ちょうどで再開した", env.collect() is CollectionResult.Unavailable)
        assertEquals(t0, env.savedEnd())

        env.advance(USAGE_INTERVAL_MS)
        val result = env.collect()

        assertTrue("10 日を 1 契機超えたのに $result", result is CollectionResult.Collected)
        assertEquals(env.now, env.savedEnd())
    }

    /**
     * **経過の置き場が作り直されただけでは止まらない**（code-verify R24）。
     *
     * [AgeClock] は置き場が読めないと `Seen(0, …)` から数え直す。窓の印は前の経過
     * （`mark.ageMs`）を持ったまま残るので、差は**それまでの稼働日数ぶん負**になり、
     * その一定値が [USAGE_CLOCK_SKEW_TOLERANCE_MS] を越え続ける ——
     * 実測（複製のプローブ）では稼働 1 日で 11 日、稼働 40 日で 50 日のあいだ
     * `Unavailable(clock_skew)` が続いた。**時計は 1 秒も飛んでいない。**
     * 止まっている間のイベントは取得元の保持（10 日）で消える（`loss: uncaptured`）。
     *
     * **経過が巻き戻ったことは時計の飛びの証拠ではない。** 印の経過を今の値へ付け替えて、
     * 通常の取得へ進む（判定は壁時計の差だけで行う）。
     */
    @Test
    fun `経過の置き場が作り直されても次の契機で取れる`() {
        val env = env()
        env.collect()
        env.advance(AgeClock.DAY_MS)          // 1 日ぶん動いた端末（時計は飛んでいない）
        assertTrue(env.collect() is CollectionResult.Collected)
        val saved = env.savedEnd()

        env.resetAgeClock()                   // 置き場が読めなくなり、経過が 0 から数え直しになる
        env.advance(USAGE_INTERVAL_MS)
        val result = env.collect()

        assertTrue("経過が巻き戻っただけなのに $result", result is CollectionResult.Collected)
        assertEquals("窓が進んでいない", env.now, env.savedEnd())
        val asked = (env.source as FakeUsageSource).eventQueries.last()
        assertEquals("保存した終わりから続いていない", saved!!.minusMillis(USAGE_WINDOW_OVERLAP_MS), asked.begin)
        // 付け替えた後は普通の契機が続く（1 契機だけ通って次からまた止まる、にならない）
        env.advance(USAGE_INTERVAL_MS)
        assertTrue(env.collect() is CollectionResult.Collected)
        assertEquals(env.now, env.savedEnd())
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
