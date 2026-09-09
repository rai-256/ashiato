// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 未送信が**収集の停止と再開をまたいで残る**（tasks 9.5 / 深掘り 第 2 回 /
 * specs/device-collection の「未送信を、収集の停止と再開をまたいで保持する」）。
 *
 * これが無いと `START_STICKY` の立て直しで最大 5 分ぶんが無言で消える。
 * この Story は「捨てたものは復元できない」を根拠に精度フィルタを外している（design D11）——
 * 同じ理由で失われる経路を残せない。
 */
class OutboxStoreTest {
    private val dir: File = Files.createTempDirectory("outbox").toFile()
    private val file = File(dir, "outbox.json")

    private fun req(id: String) =
        LocationFix(35.681236, 139.767125, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 「プロセスが立て直された」＝ 同じ置き場から新しい Outbox を作り直す。 */
    private fun reopen() = Outbox(FileOutboxStore(file))

    // Scenario: 未送信は収集の停止と再開をまたいで残る
    @Test
    fun `プロセスが立て直されても未送信が残る`() {
        val before = reopen()
        before.add(req("a"))
        before.add(req("b"))

        assertEquals(listOf("a", "b"), reopen().snapshot().map { it.id })
    }

    // Scenario: 未送信は収集の停止と再開をまたいで残る
    @Test
    fun `送れた分は立て直しても戻ってこない`() {
        // 取り除いたものが復活すると、再送が永久に止まらなくなる
        val before = reopen()
        listOf("a", "b", "c").forEach { before.add(req(it)) }
        before.remove(listOf("a", "c"))

        assertEquals(listOf("b"), reopen().snapshot().map { it.id })
    }

    @Test
    fun `記録の中身が立て直しをまたいで変わらない`() {
        // 原文の文字列が変わると冪等キーが変わり、同じ 1 件が重複して入る（design D16）
        val before = reopen()
        before.add(req("a"))

        val restored = reopen().snapshot().single()
        assertEquals(req("a").raw, restored.raw)
        assertEquals("c01-location", restored.logicalSource)
        assertEquals(540, restored.tzOffsetMin)
    }

    @Test
    fun `置き場が無ければ空から始まる`() {
        assertFalse(file.exists())
        assertEquals(0, reopen().size())
    }

    @Test
    fun `読めない置き場は捨てずに脇へ退ける`() {
        // **上書きして消さない。** 何が失われたのか後から分からなくなる
        file.writeText("これは JSON ではない")
        val lines = mutableListOf<String>()
        val outbox = Outbox(FileOutboxStore(file) { lines += it })

        assertEquals(0, outbox.size())
        assertTrue("退けた印が残っていない", File(dir, "outbox.json.unreadable").exists())
        assertTrue("黙って捨てている", lines.any { it.contains("kind=outbox_unreadable") })
    }

    @Test
    fun `置き場に位置の値が漏れてもログには出ない`() {
        // 置き場そのものは記録なので値を持つ。**ログは別**（製造準備 A-2）
        file.writeText("壊れている")
        val lines = mutableListOf<String>()
        Outbox(FileOutboxStore(file) { lines += it }).add(req("a"))
        for (line in lines) {
            for (secret in listOf("35.68", "139.76", "user-1", "device-1")) {
                assertFalse("ログに私的な内容が出ている: $line", line.contains(secret))
            }
        }
    }
}
