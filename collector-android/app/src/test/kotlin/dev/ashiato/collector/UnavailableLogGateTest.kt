// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** `source_unavailable` の重複抑止を、実際に糸を重ねて固定する。 */
class UnavailableLogGateTest {
    private fun awaitWithinD4(latch: CountDownLatch, message: String) {
        val started = System.nanoTime()
        val deadline = started + TimeUnit.SECONDS.toNanos(30)
        val remaining = deadline - System.nanoTime()
        val completed = remaining > 0 && latch.await(remaining, TimeUnit.NANOSECONDS)
        val waitedMs = (System.nanoTime() - started) / 1_000_000
        assertTrue("$message（待機=${waitedMs}ms、最後に見た値=${latch.count}）", completed)
    }

    @Test
    fun `同じ理由が並行して来てもログは一度だけ`() {
        val gate = UnavailableLogGate()
        val workers = 8
        val ready = CountDownLatch(workers)
        val start = CountDownLatch(1)
        val done = CountDownLatch(workers)
        val calls = AtomicInteger()
        val pool = Executors.newFixedThreadPool(workers)

        try {
            repeat(workers) {
                pool.execute {
                    ready.countDown()
                    start.await()
                    gate.unavailable("permission") { calls.incrementAndGet() }
                    done.countDown()
                }
            }
            awaitWithinD4(ready, "並行呼び出しの準備が整わない")
            start.countDown()
            awaitWithinD4(done, "並行呼び出しが終わらない")
        } finally {
            pool.shutdownNow()
        }

        assertEquals("同じ source_unavailable を二重出力している", 1, calls.get())
    }

    @Test
    fun `ログ中に復旧しても次の取得不能を抑止しない`() {
        val gate = UnavailableLogGate()
        val logging = CountDownLatch(1)
        val finishLog = CountDownLatch(1)
        val clearing = CountDownLatch(1)
        val recovered = CountDownLatch(1)
        val calls = AtomicInteger()
        val pool = Executors.newFixedThreadPool(2)

        try {
            pool.execute {
                gate.unavailable("permission") {
                    logging.countDown()
                    awaitWithinD4(finishLog, "ログを再開できない")
                    calls.incrementAndGet()
                }
            }
            awaitWithinD4(logging, "最初のログまで進まない")
            pool.execute {
                clearing.countDown()
                gate.available()
                recovered.countDown()
            }
            // `available()` がログと同じ排他区間を使うなら、ログが終わるまでここでは完了しない。
            awaitWithinD4(clearing, "解除の糸が始まらない")
            // timing-ok: D12。ログ中に復旧しない短い窓で、解除が割り込まないことだけを見る。
            assertFalse(
                "ログ中の解除が比較と代入の間へ割り込んだ",
                recovered.await(200, TimeUnit.MILLISECONDS),
            )
            finishLog.countDown()
            awaitWithinD4(recovered, "復旧の反映が終わらない")

            gate.unavailable("permission") { calls.incrementAndGet() }
        } finally {
            finishLog.countDown()
            pool.shutdownNow()
        }

        assertEquals("復旧後の source_unavailable を再出力していない", 2, calls.get())
    }
}
