// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * **記録が 1 件も生成されない期間でも生存信号が出る**（tasks 7.4 / FR-78 の目的そのもの）。
 *
 * 記録の生成に相乗りさせると意味が消える —— 記録が 0 件の日に
 * 「動きが無かったのか / 収集が壊れていたのか」を残すための仕組みなので、
 * 記録の契機から呼んでいたら記録が 0 件の日には 1 件も出ない。
 */
class HeartbeatWithoutRecordsTest {
    private class Clock(var at: Instant) : () -> Instant {
        override fun invoke(): Instant = at
    }

    /// Scenario: 記録が無くても生存信号が届く
    @Test
    fun `記録が 1 件も無くても生存信号は積まれる`() {
        val events = testOutbox()
        val beats = testHeartbeatOutbox()
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val emitter = HeartbeatEmitter(
            outbox = beats,
            counters = AttemptCounters(now = clock),
            userId = "user-1",
            deviceId = "device-1",
            capability = { Capability.of(permission = true, sensor = true, network = true) },
            now = clock,
            newId = { "hb-${beats.size()}" },
        )

        // **記録の契機を 1 度も起こさない。**
        // （`events` は空のまま。これは `emitter` が記録の側に触らないことの確認で、
        //   「記録の契機から呼んでいないか」の配線は `LocationServiceTest` が見る ——
        //   ここだけだと常に真の assert になる。ST02 の review/code.md の R51）
        assertEquals(0, events.size())
        clock.at = Instant.parse("2026-05-01T06:00:00Z")
        emitter.emit()
        clock.at = Instant.parse("2026-05-01T12:00:00Z")
        emitter.emit()

        assertEquals("生存信号が記録の側に積まれている", 0, events.size())
        assertEquals("記録が無いと信号が出ない", 2, beats.size())
        assertTrue(beats.snapshot().all { it.capturable })
        // 区間ごとに 1 件で、発信時刻が違う（＝別の 1 件としてサーバに入る）
        assertEquals(
            listOf("2026-05-01T06:00:00Z", "2026-05-01T12:00:00Z"),
            beats.snapshot().map { it.emittedAt },
        )
    }

    /** 取得が 1 件も無かった区間は、**取得率 0 として残る**（信号そのものは出る）。 */
    @Test
    fun `取得が 0 件の区間も取得率として残る`() {
        val beats = testHeartbeatOutbox()
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = AttemptCounters(now = clock)
        val emitter = HeartbeatEmitter(
            outbox = beats,
            counters = counters,
            userId = "user-1",
            deviceId = "device-1",
            capability = { Capability.of(permission = true, sensor = true, network = true) },
            now = clock,
            newId = { "hb-0" },
        )
        clock.at = Instant.parse("2026-05-01T06:00:00Z")
        emitter.emit()

        val sent = beats.snapshot().single()
        assertEquals("6 時間ぶんの満点", 360, sent.attempts)
        assertEquals("1 件も取れていない = 眠っていた", 0, sent.successes)
    }
}
