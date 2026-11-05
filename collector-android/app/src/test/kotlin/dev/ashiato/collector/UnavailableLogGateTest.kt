// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import java.util.concurrent.CountDownLatch
import java.util.concurrent.ExecutionException
import java.util.concurrent.Executors
import java.util.concurrent.Future
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
        val completed = latch.await(30, TimeUnit.SECONDS)
        val waitedMs = (System.nanoTime() - started) / 1_000_000
        assertTrue("$message（待機=${waitedMs}ms、最後に見た値=${latch.count}）", completed)
    }

    /** 糸の中で投げた失敗（`AssertionError` を含む）を、呼んだ側の失敗として出す。 */
    private fun rethrowFrom(task: Future<*>) {
        try {
            task.get(30, TimeUnit.SECONDS)
        } catch (e: ExecutionException) {
            throw e.cause ?: e
        }
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
            val tasks = List(workers) {
                pool.submit {
                    ready.countDown()
                    start.await()
                    gate.unavailable("permission") { calls.incrementAndGet() }
                    done.countDown()
                }
            }
            awaitWithinD4(ready, "並行呼び出しの準備が整わない")
            start.countDown()
            awaitWithinD4(done, "並行呼び出しが終わらない")
            tasks.forEach(::rethrowFrom)
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
            val first = pool.submit {
                gate.unavailable("permission") {
                    logging.countDown()
                    awaitWithinD4(finishLog, "ログを再開できない")
                    calls.incrementAndGet()
                }
            }
            awaitWithinD4(logging, "最初のログまで進まない")
            val clear = pool.submit {
                clearing.countDown()
                gate.available()
                recovered.countDown()
            }
            // `available()` がログと同じ排他区間を使うなら、ログが終わるまでここでは完了しない。
            awaitWithinD4(clearing, "解除の糸が始まらない")
            // timing-ok: D12。ログ中に復旧しない短い窓で、解除が割り込まないことだけを見る。
            assertFalse(
                "ログ中の解除が比較と代入の間へ割り込んだ",
                // timing-ok: D12 の 200ms 窓を検証する下限で、条件成立を待つ処理ではない。
                recovered.await(200, TimeUnit.MILLISECONDS),
            )
            finishLog.countDown()
            awaitWithinD4(recovered, "復旧の反映が終わらない")
            rethrowFrom(first)
            rethrowFrom(clear)

            gate.unavailable("permission") { calls.incrementAndGet() }
        } finally {
            finishLog.countDown()
            pool.shutdownNow()
        }

        assertEquals("復旧後の source_unavailable を再出力していない", 2, calls.get())
    }
}
