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
 *
 * **失敗したときの経路も全部見る**（review HIGH-8）—— 独立検証の時点では、
 * 読めたときの 1 本しか通っていなかった。
 */
class OutboxStoreTest {
    private val dir: File = Files.createTempDirectory("outbox").toFile()
    private val file = File(dir, "outbox.jsonl")
    private val lines = mutableListOf<String>()

    private fun req(id: String) =
        LocationFix(35.681236, 139.767125, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 「プロセスが立て直された」＝ 同じ置き場から新しい Outbox を作り直す。 */
    private fun reopen() = Outbox(FileOutboxStore(file, IngestRequest.serializer()) { lines += it })

    private fun asideFiles() = dir.listFiles()!!.filter { it.name.contains(".unreadable") }

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
        assertFalse(file.exists())
        assertEquals(0, reopen().size())
    }

    @Test
    fun `1件足すごとに全件を書き直さない`() {
        // **圏外が続くと未送信は伸びる**（上限と破棄は ST04）。全件書き直しだと
        // 60 秒ごとの書き込み量が件数に比例して膨らみ、フラッシュ寿命と電池に効く（design D22）
        val outbox = reopen()
        repeat(3) { outbox.add(req("id-$it")) }

        // JSONL: 1 行 1 件。行数が件数と一致していれば追記されている
        assertEquals(3, file.readLines().count { it.isNotBlank() })
        assertEquals(listOf("id-0", "id-1", "id-2"), reopen().snapshot().map { it.id })
    }

    @Test
    fun `末尾の1行が壊れていても、読めた分は捨てない`() {
        // 追記の途中で電源が落ちると最後の 1 行だけが半端になる。
        // **そこで全部捨てると、消えないようにした意味が無い**
        reopen().add(req("a"))
        reopen().add(req("b"))
        file.appendText("""{"id":"c","raw":""" + "\n")   // 途中で切れた 1 行

        val restored = reopen()

        assertEquals(listOf("a", "b"), restored.snapshot().map { it.id })
        assertTrue("壊れた行を黙って捨てている", lines.any { it.contains("kind=outbox_line_broken") })
    }

    @Test
    fun `読めない置き場は捨てずに脇へ退ける`() {
        // **上書きして消さない。** 何が失われたのか後から分からなくなる
        file.writeText("これは JSON ではない")

        val outbox = reopen()

        assertEquals(0, outbox.size())
        assertEquals("退けた跡が 1 つでない", 1, asideFiles().size)
        assertEquals("これは JSON ではない", asideFiles().single().readText())
        assertTrue("黙って捨てている", lines.any { it.contains("kind=outbox_unreadable") })
    }

    @Test
    fun `2回目の退避が1回目を上書きしない`() {
        // **独立検証で見つかった欠陥**（review R4）。退避先が固定名だと
        // `File.renameTo` が Unix で置き換えるので、2 回目の破損で 1 回目の退避が消える
        file.writeText("壊れ1")
        reopen()
        Thread.sleep(2)          // 退避先の名前に時刻を使うので、確実にずらす
        file.writeText("壊れ2")
        reopen()

        val kept = asideFiles().map { it.readText() }.sorted()
        assertEquals("退避が 1 つに潰れている", listOf("壊れ1", "壊れ2"), kept)
    }

    @Test
    fun `書き換えの途中で落ちた跡から復元する`() {
        // rename に失敗すると最新の全件が `.tmp` に残る。**次の save で上書きすると消える**
        // （review CRITICAL-2）。本体が無いなら `.tmp` を採る
        val tmp = File(dir, "outbox.jsonl.tmp")
        tmp.writeText(ingestJson.encodeToString(req("rescued")) + "\n")
        assertFalse(file.exists())

        val outbox = reopen()

        assertEquals(listOf("rescued"), outbox.snapshot().map { it.id })
        assertTrue("復元したことが記録されていない", lines.any { it.contains("kind=outbox_recovered") })
    }

    @Test
    fun `本体があるときの書きかけは捨ててよい`() {
        // 本体のほうが確定している。`.tmp` を優先すると、取り除いたはずの記録が戻る
        reopen().add(req("real"))
        File(dir, "outbox.jsonl.tmp").writeText(ingestJson.encodeToString(req("stale")) + "\n")

        assertEquals(listOf("real"), reopen().snapshot().map { it.id })
    }

    @Test
    fun `書けなかったことが呼び出し側に返る`() {
        // **Unit だと失敗が上に届かない**（review CRITICAL-3）。
        // 置き場をディレクトリにして書き込みを失敗させる
        val blocked = File(dir, "blocked.jsonl")
        blocked.mkdir()
        val outbox = Outbox(FileOutboxStore(blocked, IngestRequest.serializer()) { lines += it })

        assertFalse("書けていないのに true が返っている", outbox.add(req("a")))
        assertTrue("黙って失敗している", lines.any { it.contains("kind=outbox_append_failed") })
        // メモリには積まれている（次の契機で書き直される）
        assertEquals(1, outbox.size())
    }

    @Test
    fun `取り除きに失敗したことも呼び出し側に返る`() {
        val blocked = File(dir, "blocked2.jsonl")
        blocked.mkdir()
        val outbox = Outbox(FileOutboxStore(blocked, IngestRequest.serializer()) { lines += it })
        outbox.add(req("a"))

        assertFalse(outbox.remove(listOf("a")))
        // **置き場がディレクトリなので、読み出しの時点で既に失敗している。**
        // その状態での書き直しは「失敗した」ではなく「**断った**」——
        // 読めなかった分を上書きで消さないため（ST02 の review/code.md の R17）。
        assertTrue(
            "黙って失敗している",
            lines.any { it.contains("kind=outbox_save_refused") || it.contains("kind=outbox_save_failed") },
        )
    }

    /**
     * **1 度読めなかっただけで、溜まっていた未送信が消えない**（ST02 の review/code.md の R17）。
     *
     * `load()` が `emptyList()` を返すと、それが `pending` の初期値になる。
     * そのあと送信が 1 件成功すると `remove` → `save` が**ファイルを丸ごと書き直す**ので、
     * 読めなかった分が痕跡なく消えていた。一過性の失敗（EMFILE・direct boot 中のアクセス）で
     * 起きるので、中身は無事なまま失われる。
     */
    @Test
    fun `読めなかった未送信が、次の書き直しで消えない`() {
        val store = FileOutboxStore(file, IngestRequest.serializer()) { lines += it }
        Outbox(store).apply {
            add(req("keep-1"))
            add(req("keep-2"))
        }
        val before = file.readText()

        // 読めない状態にする（一過性の IO 失敗を模す）
        assertTrue("読み取り権限を落とせない環境", file.setReadable(false))
        val blind = Outbox(FileOutboxStore(file, IngestRequest.serializer()) { lines += it })
        assertEquals("読めていないのに中身が見えている", 0, blind.size())

        // 送信が成功したことにして取り除く → **ここで上書きされてはいけない**
        assertFalse("読めていないのに書き直しが通った", blind.remove(listOf("keep-1")))
        assertTrue(
            "断ったことが残っていない",
            lines.any { it.contains("kind=outbox_save_refused") },
        )

        // 読めるようになれば元の 2 件が戻る
        assertTrue(file.setReadable(true))
        assertEquals(before, file.readText())
        val reopened = Outbox(FileOutboxStore(file, IngestRequest.serializer()) {})
        assertEquals(listOf("keep-1", "keep-2"), reopened.snapshot().map { it.id })
    }

    @Test
    fun `置き場が壊れていてもログには位置の値が出ない`() {
        // 置き場そのものは記録なので値を持つ。**ログは別**（製造準備 A-2）
        file.writeText("壊れている")
        reopen().add(req("a"))

        assertTrue("ログが 1 行も出ていない（試験が空振りしている）", lines.isNotEmpty())
        for (line in lines) {
            for (secret in listOf("35.68", "139.76", "user-1", "device-1")) {
                assertFalse("ログに私的な内容が出ている: $line", line.contains(secret))
            }
        }
    }
}
