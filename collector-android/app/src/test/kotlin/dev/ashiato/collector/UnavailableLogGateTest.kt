// SPDX-License-Identifier: AGPL-3.0-only
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
            assertTrue("並行呼び出しの準備が整わない", ready.await(5, TimeUnit.SECONDS))
            start.countDown()
            assertTrue("並行呼び出しが終わらない", done.await(5, TimeUnit.SECONDS))
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
                    assertTrue("ログを再開できない", finishLog.await(5, TimeUnit.SECONDS))
                    calls.incrementAndGet()
                }
            }
            assertTrue("最初のログまで進まない", logging.await(5, TimeUnit.SECONDS))
            pool.execute {
                clearing.countDown()
                gate.available()
                recovered.countDown()
            }
            // `available()` がログと同じ排他区間を使うなら、ログが終わるまでここでは完了しない。
            assertTrue("解除の糸が始まらない", clearing.await(5, TimeUnit.SECONDS))
            assertFalse(
                "ログ中の解除が比較と代入の間へ割り込んだ",
                recovered.await(200, TimeUnit.MILLISECONDS),
            )
            finishLog.countDown()
            assertTrue("復旧の反映が終わらない", recovered.await(5, TimeUnit.SECONDS))

            gate.unavailable("permission") { calls.incrementAndGet() }
        } finally {
            finishLog.countDown()
            pool.shutdownNow()
        }

        assertEquals("復旧後の source_unavailable を再出力していない", 2, calls.get())
    }
}
