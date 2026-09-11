// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 生存信号が取得できる状態を運ぶこと（FR-78 / 深掘り Q5。tasks 7.2）。
 *
 * **記録が 0 件の日に「動いていなかった」のか「壊れていた」のかを分ける材料は、
 * その時点で送らないと後から作れない。**
 */
class HeartbeatTest {
    private fun emitter(
        outbox: Outbox<HeartbeatRequest> = testHeartbeatOutbox(),
        capability: () -> Capability = { Capability.of(permission = true, sensor = true, network = true) },
        counters: AttemptCounters = AttemptCounters(now = { Instant.parse("2026-05-01T00:00:00Z") }),
        now: () -> Instant = { Instant.parse("2026-05-01T06:00:00Z") },
        log: (String) -> Unit = {},
    ) = HeartbeatEmitter(
        outbox = outbox,
        counters = counters,
        userId = "user-1",
        deviceId = "device-1",
        capability = capability,
        now = now,
        newId = { "hb-1" },
        log = log,
    )

    /// Scenario: 権限が無い状態が信号に載る
    @Test
    fun `権限が剥がれた状態が理由とともに信号に載る`() {
        val outbox = testHeartbeatOutbox()
        emitter(
            outbox = outbox,
            capability = { Capability.of(permission = false, sensor = true, network = true) },
        ).emit()

        val sent = outbox.snapshot().single()
        assertFalse("取れない状態が capturable=true で出ている", sent.capturable)
        assertEquals(listOf(Capability.PERMISSION), sent.blockers)
    }

    /** 取れている状態なら理由は空。 */
    @Test
    fun `取得できる状態なら理由は付かない`() {
        val outbox = testHeartbeatOutbox()
        emitter(outbox = outbox).emit()
        val sent = outbox.snapshot().single()
        assertTrue(sent.capturable)
        assertEquals(emptyList<String>(), sent.blockers)
    }

    /**
     * **理由の無い「取れない」は組み立てられない。** サーバに断られるまで気付かない形を
     * 作れる口を残すと、その信号は未送信に居座り続ける（specs）。
     */
    @Test
    fun `理由の無い取れないは組み立てられない`() {
        val e = runCatching { Capability(capturable = false, blockers = emptyList()) }
        assertTrue("理由の無い「取れない」が作れてしまう", e.isFailure)
    }

    /** センサと接続も理由として載る（specs「権限・センサ・接続」）。 */
    @Test
    fun `センサと接続も理由になる`() {
        assertEquals(
            listOf(Capability.SENSOR, Capability.NETWORK),
            Capability.of(permission = true, sensor = false, network = false).blockers,
        )
    }

    /**
     * 原文は**素通しで残る**ので、同じ 1 件は毎回同じ文字列でなければならない
     * —— 冪等キーがこの文字列から作られる（第 4 回 Q13）。
     */
    @Test
    fun `原文に載るのは信号の中身だけで、位置の値は出ない`() {
        val outbox = testHeartbeatOutbox()
        emitter(outbox = outbox).emit()
        val raw = Json.parseToJsonElement(outbox.snapshot().single().raw)
        val obj = raw as kotlinx.serialization.json.JsonObject
        assertEquals("true", obj["alive"]?.jsonPrimitive?.content)
        assertNotNull(obj["attempts"])
        // **位置の値は載せない**（製造準備 A-2 / specs「私的な内容を記録の外に出さない」）
        assertFalse(outbox.snapshot().single().raw.contains("lat"))
        assertFalse(outbox.snapshot().single().raw.contains("lon"))
    }

    /** 発信時刻は信号を作った時刻。**受信時刻ではない**（第 6 回 Q24 と同じ向き）。 */
    @Test
    fun `発信時刻が信号に載る`() {
        val outbox = testHeartbeatOutbox()
        emitter(outbox = outbox, now = { Instant.parse("2026-05-01T06:00:00Z") }).emit()
        assertEquals("2026-05-01T06:00:00Z", outbox.snapshot().single().emittedAt)
    }
}
