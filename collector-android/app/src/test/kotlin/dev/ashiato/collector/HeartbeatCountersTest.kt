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

    /**
     * 数えが**プロセスの立て直しをまたいで残る**（ST02 の review/code.md の R16 / C-2 / I8）。
     *
     * `Outbox` は「インスタンスの中だけに積むと立て直しで無言で消える」を理由に
     * ファイルへ落としたのに、**同じ理由が当てはまる数えは落とされていなかった**。
     * `since` も一緒に新品になるので、**死んでいた区間そのものが観測から落ちる** ——
     * 6 時間のうち 5 時間 50 分死んで 10 分前に立て直されると、次の信号は
     * `10 / 10` で「取得率 100 %」になり、画面には「健全」と出る。
     */
    @Test
    fun `数えは立て直しをまたいで残る`() {
        val dir = java.nio.file.Files.createTempDirectory("counters").toFile()
        val file = java.io.File(dir, "heartbeat-counters.txt")
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))

        val first = AttemptCounters(now = clock, store = FileCounterStore(file) {})
        repeat(3) { first.recordSuccess() }
        clock.advanceMinutes(350)

        // プロセスが立て直された（新しいインスタンスで同じ置き場を開く）
        val second = AttemptCounters(now = clock, store = FileCounterStore(file) {})
        clock.advanceMinutes(10)
        val (attempts, successes) = second.take()
        assertEquals("区間の起点が立て直しで新品になっている", 360, attempts)
        assertEquals("成功の数えが消えている", 3, successes)
    }

    /**
     * **積めなかったら数えは戻らない**（ST02 の review/code.md の R24 / H-7 / F9 / I9）。
     *
     * `peek()` の docstring が「読むだけでは戻さない —— 信号を組み立てられなかったときに
     * 数えが消える」と書いていた当の問題が、`emit()` 側で起きていた。
     */
    @Test
    fun `積めなかった区間の数えは残る`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = AttemptCounters(now = clock)
        repeat(5) { counters.recordSuccess() }
        clock.advanceMinutes(10)

        // 積めなかった
        val (stored, _) = counters.takeAfter { false to Unit }
        assertEquals(false, stored)
        assertEquals("積めていないのに数えが戻った", 10 to 5, counters.peek())

        // 積めた
        val (ok, _) = counters.takeAfter { true to Unit }
        assertEquals(true, ok)
        assertEquals(0 to 0, counters.peek())
    }
}
