// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.time.Instant

/** 通知の決め方（ST14 / design D13 の表）。Rust 側と同じ表・同じ Scenario 名。 */
@RunWith(RobolectricTestRunner::class)
class OutageNoticeTest {
    private val at = Instant.parse("2026-10-12T08:15:00Z")
    private fun outage(id: Long, name: String) = Outage(id, "src-$id", name, at)
    private val location = outage(1, "位置")
    private val photo = outage(2, "写真")

    private fun decide(sounded: Set<Long>, shown: Boolean, result: OutageResult) =
        OutageNotice.decide(NoticeState(sounded, shown), result)

    @Test
    fun `新しい途切れで音つきの通知が 1 通出る`() {
        // Scenario: 新しい途切れで音つきの通知が 1 通出る
        val d = decide(emptySet(), false, OutageResult.Ready(listOf(location)))

        assertEquals(NoticeAction.Show(listOf(location), sound = true), d.action)
        assertEquals(NoticeState(setOf(1L), shown = true), d.next)
    }

    @Test
    fun `続く途切れでは音を鳴らさずに通知が残る`() {
        // Scenario: 続く途切れでは音を鳴らさずに通知が残る
        val d = decide(setOf(1L), true, OutageResult.Ready(listOf(location)))

        assertEquals(NoticeAction.Show(listOf(location), sound = false), d.action)
        assertEquals(setOf(1L), d.next.sounded)
    }

    @Test
    fun `同時に途切れたソースは 1 通にまとまる`() {
        // Scenario: 同時に途切れたソースは 1 通にまとまる
        val d = decide(emptySet(), false, OutageResult.Ready(listOf(location, photo)))

        val show = d.action as NoticeAction.Show
        assertEquals(listOf(location, photo), show.outages)
        assertTrue("音は 1 回だけ（1 通・音つき）", show.sound)
        assertEquals(setOf(1L, 2L), d.next.sounded)
    }

    @Test
    fun `新しく途切れたソースが増えたときだけまた鳴る`() {
        // Scenario: 新しく途切れたソースが増えたときだけまた鳴る
        val d = decide(setOf(1L), true, OutageResult.Ready(listOf(location, photo)))

        assertEquals(NoticeAction.Show(listOf(location, photo), sound = true), d.action)
        assertEquals(setOf(1L, 2L), d.next.sounded)
    }

    @Test
    fun `途切れたソースが減ると音を鳴らさずに差し替わる`() {
        // Scenario: 途切れたソースが減ると音を鳴らさずに差し替わる
        val d = decide(setOf(1L, 2L), true, OutageResult.Ready(listOf(photo)))

        assertEquals(NoticeAction.Show(listOf(photo), sound = false), d.action)
        assertEquals(setOf(2L), d.next.sounded)
    }

    @Test
    fun `戻ると通知が黙って消える`() {
        // Scenario: 戻ると通知が黙って消える
        val d = decide(setOf(1L), true, OutageResult.Ready(emptyList()))

        assertEquals(NoticeAction.Dismiss, d.action)
        assertEquals(NoticeState(), d.next)
    }

    @Test
    fun `通知が出ていなければ戻っても何もしない`() {
        val d = decide(emptySet(), false, OutageResult.Ready(emptyList()))

        assertEquals(NoticeAction.Nothing, d.action)
        assertEquals(NoticeState(), d.next)
    }

    @Test
    fun `サーバに届かない間は通知が変わらない`() {
        // Scenario: サーバに届かない間は通知が変わらない
        val before = NoticeState(setOf(1L), shown = true)
        val d = OutageNotice.decide(before, OutageResult.Unreachable)

        assertEquals(NoticeAction.Nothing, d.action)
        assertEquals(before, d.next)
    }

    @Test
    fun `判定を待っている間は通知が変わらない`() {
        // Scenario: 判定を待っている間は通知が変わらない
        val before = NoticeState(setOf(1L), shown = true)
        val d = OutageNotice.decide(before, OutageResult.WarmingUp)

        assertEquals(NoticeAction.Nothing, d.action)
        assertEquals(before, d.next)
    }

    @Test
    fun `戻った後にまた途切れると別の識別子になってまた鳴る`() {
        // Scenario: 戻った後にまた途切れると別の識別子になる
        val back = decide(setOf(1L), true, OutageResult.Ready(emptyList())).next
        val again = decide(back.sounded, back.shown, OutageResult.Ready(listOf(outage(7, "位置"))))

        assertEquals(NoticeAction.Show(listOf(outage(7, "位置")), sound = true), again.action)
    }

    @Test
    fun `同じ途切れでも端末ごとに 1 回鳴る`() {
        // Scenario: 同じ途切れでも端末ごとに 1 回鳴る
        // 端末の側: 前の状態が空の端末は、ほかの端末が鳴らしていても必ず鳴る
        val first = decide(emptySet(), false, OutageResult.Ready(listOf(location)))
        val other = decide(emptySet(), false, OutageResult.Ready(listOf(location)))

        assertEquals(NoticeAction.Show(listOf(location), sound = true), first.action)
        assertEquals(NoticeAction.Show(listOf(location), sound = true), other.action)
    }

    @Test
    fun `収集アプリを起動し直しても同じ途切れでは鳴らない`() {
        // Scenario: 収集アプリを起動し直しても同じ途切れでは鳴らない
        val app: Application = ApplicationProvider.getApplicationContext()
        val prefs = { app.getSharedPreferences(OutageNotice.PREFS, android.content.Context.MODE_PRIVATE) }
        val first = OutageNotice(prefs())
        first.save(first.load().let { decide(it.sounded, it.shown, OutageResult.Ready(listOf(location))).next })

        // 起動し直し = 新しい OutageNotice が SharedPreferences から読み直す
        val reborn = OutageNotice(prefs())
        val d = OutageNotice.decide(reborn.load(), OutageResult.Ready(listOf(location)))

        assertEquals(NoticeAction.Show(listOf(location), sound = false), d.action)
    }
}
