// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 前回の信号からの取得の試行回数と成功回数（第 5 回 Q17。tasks 7.2b）。
 *
 * **これが ST01 の R46（Doze）が渡した宿題の答え** —— 信号が来ている＝生きていた /
 * 取得率が低い＝眠っていた / 信号が来ない＝死んでいた、の 3 つが分かれる。
 * R46 が「14 分 〜 18 時間の間は判別できない」と書いた区間が、この比で埋まる。
 */
class HeartbeatCountersTest {
    /** 進む時計。 */
    private class Clock(var at: Instant) : () -> Instant {
        override fun invoke(): Instant = at

        fun advanceMinutes(n: Long) {
            at = at.plusSeconds(n * 60)
        }
    }

    /// Scenario: 取得の試行と成功の数が信号に載る
    @Test
    fun `6 時間で 360 回が満点になり、取れた数がそのまま成功になる`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = AttemptCounters(now = clock)
        repeat(230) { counters.recordSuccess() }
        clock.advanceMinutes(360) // 6 時間 = 60 秒間隔で 360 回

        val (attempts, successes) = counters.take()
        assertEquals("6 時間 ÷ 60 秒 = 360 が満点", 360, attempts)
        assertEquals(230, successes)
    }

    /// Scenario: 数えは信号を送るたびに戻る
    @Test
    fun `信号を送ると数えが戻る`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = AttemptCounters(now = clock)
        repeat(230) { counters.recordSuccess() }
        clock.advanceMinutes(360)
        counters.take()

        // 次の区間は 10 分（= 10 回）で 4 回だけ取れた
        clock.advanceMinutes(10)
        repeat(4) { counters.recordSuccess() }
        val (attempts, successes) = counters.take()
        assertEquals("累計になっていない（前の 360 を持ち越している）", 10, attempts)
        assertEquals(4, successes)
    }

    /** **読むだけでは戻さない** —— 信号を組み立てられなかったときに数えが消えないため。 */
    @Test
    fun `peek は数えを戻さない`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = AttemptCounters(now = clock)
        repeat(3) { counters.recordSuccess() }
        clock.advanceMinutes(10)
        assertEquals(10 to 3, counters.peek())
        assertEquals(10 to 3, counters.peek())
        assertEquals(10 to 3, counters.take())
        assertEquals(0 to 0, counters.take())
    }

    /**
     * **成功が試行を超える信号を作らない**（2 巡目 R7）。端末の時計が戻ると
     * 経過が負になり、受け口に断られた信号が未送信に居座る。
     */
    @Test
    fun `時計が戻っても成功が試行を超えない`() {
        val clock = Clock(Instant.parse("2026-05-01T06:00:00Z"))
        val counters = AttemptCounters(now = clock)
        repeat(5) { counters.recordSuccess() }
        clock.at = Instant.parse("2026-05-01T00:00:00Z") // 6 時間戻った
        val (attempts, successes) = counters.take()
        assertTrue("成功 $successes が試行 $attempts を超えている", successes <= attempts)
        assertEquals(5, attempts)
    }
}
