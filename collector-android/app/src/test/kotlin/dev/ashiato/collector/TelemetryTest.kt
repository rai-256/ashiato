// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * ログに私的な内容が出ない（tasks 6.5 / specs「私的な内容を記録の外に出さない」）。
 *
 * **ログは記録本体と違って感度の制御（PERM-2）が効かない。** 一度出たものは締められない。
 */
class TelemetryTest {
    private val lat = 35.681236
    private val lon = 139.767125

    private fun req(id: String) =
        LocationFix(lat, lon, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-0001", "device-secret", ZoneId.of("Asia/Tokyo"))

    /** 送信のどの経路を通っても、値がログに出ない */
    private fun linesFor(outcome: Outcome): List<String> {
        val outbox = testOutbox()
        listOf("a", "b").forEach { outbox.add(req(it)) }
        val lines = mutableListOf<String>()
        Sender(outbox, { outcome }, IngestRequest.serializer(), lines::add).flush()
        return lines
    }

    private fun assertNoPrivateData(lines: List<String>) {
        assertTrue("ログが 1 行も出ていない（試験が空振りしている）", lines.isNotEmpty())
        for (line in lines) {
            for (secret in listOf("35.68", "139.76", "user-0001", "device-secret", "lat", "lon")) {
                assertTrue("ログに私的な内容が出ている: $line", !line.contains(secret))
            }
        }
    }

    @Test
    fun `送信が成功してもログに値は出ない`() {
        assertNoPrivateData(
            linesFor(Outcome.Responded(200, """[{"accepted":true},{"accepted":true}]""")),
        )
    }

    // Scenario: 送信の失敗がログに出ても値は出ない
    @Test
    fun `送信が失敗してもログに値は出ない`() {
        // 失敗のときこそ「詳しく出したい」誘惑が働く。ここで止める
        val lines = linesFor(Outcome.Unreachable("timeout"))
        assertNoPrivateData(lines)
        assertTrue("エラーの種別は出てよい", lines.any { it.contains("error=timeout") })
        assertTrue("件数は出てよい", lines.any { it.contains("count=2") })
    }

    @Test
    fun `応答が読めなくても本文をログに載せない`() {
        // サーバの応答をそのまま出すと、原文が反射してログへ落ちる
        val body = """[{"raw":{"lat":$lat,"lon":$lon}}]"""
        assertNoPrivateData(linesFor(Outcome.Responded(200, body)))
    }

    @Test
    fun `出すのは件数・ソース名・所要時間・エラーの種別だけ`() {
        val line = Telemetry.line("send", count = 3, elapsedMs = 120, error = "timeout")
        assertTrue(line == "kind=send source=c01-location count=3 elapsed_ms=120 error=timeout")
    }
}
