// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 「積んでからの経過」の数え方（ST04 / tasks 6.1 / 深掘り Q4 / C10 / R9 / design D2）。
 *
 * 時計が数か月先へ飛ぶと、出来事の時刻から数えても積んだ時刻から数えても、溜まった分が一度に「90 日超」になる。
 * **既定は捨てない側**（2 GB の上限だけが端末を守る）。
 */
class ElapsedClockTest {
    private val dir: File = Files.createTempDirectory("clock").toFile()
    private val device = FakeDeviceClock()
    private val day = AgeClock.DAY_MS

    private fun clock() = AgeClock(device, File(dir, "age-clock.txt"), {})

    // Scenario: 時計が先へ飛んでも 90 日の側では捨てない
    @Test
    fun `同じ起動のあいだに壁時計が 120 日飛んでも経過は単調時計の 1 日`() {
        val c = clock()
        val enq = c.now()
        device.mono += 1 * day
        device.wall += 120 * day
        assertEquals(1 * day, c.now() - enq)
    }

    // Scenario: 再起動をまたぐ長い空白は 90 日に数えない
    @Test
    fun `再起動をまたいで時計が 100 日進んでも 30 日までしか数えない`() {
        val c = clock()
        val enq = c.now()
        device.advance(10 * day)
        c.now()
        device.reboot(wallGapMs = 100 * day)
        val elapsed = clock().now() - enq
        assertEquals(10 * day + 30 * day, elapsed)
        assertTrue("90 日を超えたとして数えている", elapsed <= 90 * day)
    }

    // Scenario: 再起動をまたぐ 30 日以内の空白は 90 日に数える
    @Test
    fun `再起動をまたいで時計が 20 日進んだら 20 日を数える`() {
        val c = clock()
        val enq = c.now()
        device.advance(80 * day)
        c.now()
        device.reboot(wallGapMs = 20 * day)
        val elapsed = clock().now() - enq
        assertEquals(100 * day, elapsed)
        assertTrue(elapsed > 90 * day)
    }

    // Scenario: 再起動をまたいで時計が戻っても経過は減らない
    @Test
    fun `再起動をまたいで時計が 10 日戻ったら 0 として数える`() {
        val c = clock()
        val enq = c.now()
        device.advance(50 * day)
        c.now()
        device.reboot(wallGapMs = -10 * day)
        assertEquals(50 * day, clock().now() - enq)
    }

    @Test
    fun `起動回数が取れない端末でも単調時計が戻れば再起動として扱う`() {
        device.boot = null
        val c = clock()
        val enq = c.now()
        device.advance(5 * day)
        c.now()
        device.reboot(wallGapMs = 2 * day)
        assertEquals(7 * day, clock().now() - enq)
    }

    @Test
    fun `経過は立て直しをまたいで続きから数える`() {
        val c = clock()
        val enq = c.now()
        device.advance(3 * day)
        // 同じ起動でプロセスだけ立て直された
        assertEquals(3 * day, clock().now() - enq)
    }
}
