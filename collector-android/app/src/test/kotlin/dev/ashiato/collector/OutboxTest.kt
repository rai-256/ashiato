// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Test

/** 未送信の置き場（tasks 7.1）と、ふるい落とさないこと（tasks 6.4 / design D11）。 */
class OutboxTest {
    /** 置き場はこの試験の関心ではないので、毎回まっさらなファイルを使う。 */
    private fun outbox() =
        Outbox(FileOutboxStore(File(Files.createTempDirectory("outbox").toFile(), "outbox.json")))

    private fun req(id: String, accuracy: Float = 10f) =
        LocationFix(35.68, 139.76, accuracy, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    @Test
    fun `積んだものが残る`() {
        val outbox = outbox()
        outbox.add(req("a"))
        outbox.add(req("b"))
        assertEquals(listOf("a", "b"), outbox.snapshot().map { it.id })
    }

    @Test
    fun `水平精度が悪くてもふるい落とさない`() {
        // **捨てた記録は後から復元できない**（design D11）。
        // 閾値を設けるなら読む側（ST16 / ST25）の仕事。
        val outbox = outbox()
        outbox.add(req("good", accuracy = 5f))
        outbox.add(req("terrible", accuracy = 5000f))
        assertEquals(2, outbox.size())
        assertEquals(listOf("good", "terrible"), outbox.snapshot().map { it.id })
    }

    @Test
    fun `送れた分だけ取り除かれる`() {
        val outbox = outbox()
        listOf("a", "b", "c").forEach { outbox.add(req(it)) }
        outbox.remove(listOf("a", "c"))
        assertEquals(listOf("b"), outbox.snapshot().map { it.id })
    }

    @Test
    fun `送信中に積まれたものは取り違えない`() {
        val outbox = outbox()
        listOf("a", "b").forEach { outbox.add(req(it)) }
        val inFlight = outbox.snapshot().map { it.id }
        outbox.add(req("c"))          // 送信中に 1 件増えた
        outbox.remove(inFlight)
        assertEquals(listOf("c"), outbox.snapshot().map { it.id })
    }
}
