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

    /**
     * **数えなかった前進**（ST06 / tasks 3.3 / 独立レビュー Important 1）。
     * 30 日の頭打ちで落ちた分だけが積もり、ファイルに残る。
     *
     * 立て直しで 0 に戻ると、読む側（アプリ利用の窓）が長い電源断を
     * 「時計が飛んだ」と読み、取得が永久に止まる。
     */
    @Test
    fun `数えなかった前進は頭打ちのぶんだけ積もり立て直しをまたいで残る`() {
        val c = clock()
        c.now()
        assertEquals(0, c.discardedMs())
        device.advance(1 * day)
        c.now()
        assertEquals("同じ起動のあいだに落ちている", 0, c.discardedMs())

        // 30 日以内の空白は**全部数える**ので、落ちる分は無い
        device.reboot(wallGapMs = 20 * day)
        val kept = clock()
        kept.now()
        assertEquals(0, kept.discardedMs())

        // 60 日の空白は 30 日で頭打ち —— 落ちた 30 日がここに出る
        device.reboot(wallGapMs = 60 * day)
        val next = clock()            // プロセスが立て直された（ファイルから読み直す）
        next.now()
        assertEquals(30 * day, next.discardedMs())
        // 立て直しただけでは増えない（ファイルから読み直しても同じ値）
        val again = clock()
        again.now()
        assertEquals(30 * day, again.discardedMs())
    }

    /** 時計が**戻った**跨ぎでは何も落ちない（負の食い違いは飛びの証拠として残す側）。 */
    @Test
    fun `時計が戻った跨ぎでは数えなかった前進は増えない`() {
        val c = clock()
        c.now()
        device.advance(5 * day)
        c.now()
        device.reboot(wallGapMs = -10 * day)
        val next = clock()
        next.now()
        assertEquals(0, next.discardedMs())
    }

    /** 単調時計が戻らなくても、起動回数が変われば再起動として扱う（起動回数の比較を消すと落ちる）。 */
    @Test
    fun `起動回数が変わったら単調時計が進んでいても再起動として扱う`() {
        val c = clock()
        val enq = c.now()
        device.advance(5 * day)
        c.now()
        // 起動回数だけが変わり、単調時計はたまたま前より大きい
        device.boot = device.boot!! + 1
        device.mono += 1_000
        device.wall += 100 * day
        assertEquals(5 * day + 30 * day, clock().now() - enq)
    }
}
