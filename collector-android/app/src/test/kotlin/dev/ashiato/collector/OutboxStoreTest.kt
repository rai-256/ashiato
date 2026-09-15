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
 * 未送信が**収集の停止と再開をまたいで残る**（ST01 tasks 9.5 / 深掘り 第 2 回 /
 * specs/device-collection の「未送信を、収集の停止と再開をまたいで保持する」）。
 *
 * これが無いと `START_STICKY` の立て直しで最大 5 分ぶんが無言で消える。
 * ST04 で置き場を 1 本の JSONL から区切りファイル（`SegmentStore`）に替えた（design D1）ので、
 * ST01 / ST02 がこの置き場に求めた性質を新しい置き場で確かめ直す。
 */
class OutboxStoreTest {
    private val dir: File = Files.createTempDirectory("outbox").toFile()
    private val lines = mutableListOf<String>()

    private fun req(id: String) =
        LocationFix(35.681236, 139.767125, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 「プロセスが立て直された」＝ 同じ置き場から新しい Outbox を作り直す。 */
    private fun reopen() = Outbox.inDir(dir, IngestRequest.serializer()) { lines += it }

    private fun segments() = File(dir, "segments").listFiles { f -> f.name.endsWith(".jsonl") }.orEmpty().sortedBy { it.name }

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
        reopen().add(req("a"))

        val restored = reopen().snapshot().single()
        assertEquals(req("a").raw, restored.raw)
        assertEquals("c01-location", restored.logicalSource)
        assertEquals(540, restored.tzOffsetMin)
    }

    @Test
    fun `置き場が無ければ空から始まる`() {
        assertEquals(0, reopen().size())
    }

    @Test
    fun `1件足すごとに全件を書き直さない`() {
        // 追記なら先に書いた行のバイトは動かない（全件書き直しは圏外が伸びるほどフラッシュと電池に効く。ST01 design D22）
        val outbox = reopen()
        outbox.add(req("id-0"))
        val first = segments().single().readBytes()
        outbox.add(req("id-1"))
        outbox.add(req("id-2"))

        val now = segments().single().readBytes()
        assertTrue("先に書いた行が書き直されている", now.copyOfRange(0, first.size).contentEquals(first))
        assertEquals(3, segments().single().readLines().count { it.isNotBlank() })
        assertEquals(listOf("id-0", "id-1", "id-2"), reopen().snapshot().map { it.id })
    }

    @Test
    fun `末尾の1行が壊れていても、読めた分は捨てない`() {
        // 追記の途中で電源が落ちると最後の 1 行だけが半端になる。**そこで全部捨てると、消えないようにした意味が無い**
        reopen().add(req("a"))
        reopen().add(req("b"))
        segments().single().appendText("""{"enq":1,"item":{"id":"c","raw":""")   // 途中で切れた 1 行（改行なし）

        val restored = reopen()
        assertEquals(listOf("a", "b"), restored.snapshot().map { it.id })
        assertTrue("壊れた行を黙って捨てている", lines.any { it.contains("kind=outbox_line_broken") })

        // **書きかけの行に次の 1 件を繋げない**（繋がると 2 行とも読めなくなる）
        restored.add(req("d"))
        assertEquals(listOf("a", "b", "d"), reopen().snapshot().map { it.id })
    }

    @Test
    fun `書けなかったことが呼び出し側に返り、メモリには積まれる`() {
        // **Unit だと失敗が上に届かない**（ST01 review CRITICAL-3）。置き場をファイルにして書き込みを失敗させる
        val blocked = File(dir, "blocked")
        blocked.writeText("ディレクトリではない")
        val outbox = Outbox(
            SegmentStore(File(blocked, "segments"), IngestRequest.serializer(), File(dir, "u.jsonl"), { lines += it }),
            age = { 0L },
        )

        assertFalse("書けていないのに true が返っている", outbox.add(req("a")))
        assertTrue("黙って失敗している", lines.any { it.contains("kind=outbox_append_failed") })
        // メモリには積まれている（次の契機で送られる）
        assertEquals(listOf("a"), outbox.head(10).map { it.id })
        assertTrue(outbox.remove(listOf("a")))
        assertEquals(0, outbox.size())
    }

    @Test
    fun `置き場が壊れていてもログには位置の値が出ない`() {
        // 置き場そのものは記録なので値を持つ。**ログは別**（製造準備 A-2）
        reopen().add(req("a"))
        segments().single().appendText("壊れている 35.681236 139.767125\n")
        reopen().snapshot()

        assertTrue("ログが 1 行も出ていない（試験が空振りしている）", lines.isNotEmpty())
        for (line in lines) {
            for (secret in listOf("35.68", "139.76", "user-1", "device-1")) {
                assertFalse("ログに私的な内容が出ている: $line", line.contains(secret))
            }
        }
    }
}
