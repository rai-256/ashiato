// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.time.Instant
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 破棄の報告を送る（ST04 / tasks 7.2 / 深掘り C2 / R2 / design D4 / D13）。
 *
 * **一度でも送ろうとした報告は書き換えない。** 書き換えると冪等キーが変わって別の報告になり、件数が二重に数えられる。
 */
class DropReportSendTest {
    private val st = TestStores()

    private class FakeTransport(val reply: (String) -> Outcome) : Transport {
        val bodies = mutableListOf<String>()

        override fun post(bodyJson: String): Outcome {
            bodies += bodyJson
            return reply(bodyJson)
        }
    }

    private fun drop(time: String) {
        st.ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, Instant.parse(time))
        st.ledger.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)
    }

    private fun sender(t: Transport) = Sender(st.drops, t, DropReport.serializer(), dropPermanentlyRejected = false)

    private fun dropsSegmentBytes(): ByteArray =
        File(st.dir, "drops").listFiles { f -> f.name.endsWith(".jsonl") }.orEmpty().sortedBy { it.name }
            .fold(ByteArray(0)) { acc, f -> acc + f.readBytes() }

    // Scenario: 送ろうとした報告は書き換えられない
    // Scenario: 送ろうとした後の破棄は新しい報告になる
    @Test
    fun `送ろうとして失敗した後にさらに捨てても、最初の報告は 1 バイトも変わらず新しい報告が積まれる`() {
        drop("2026-06-01T10:00:00Z")
        st.ledger.freeze()
        val failing = FakeTransport { Outcome.Unreachable("timeout") }
        sender(failing).flush()
        val firstBytes = dropsSegmentBytes()
        val first = st.drops.snapshot().single()

        drop("2026-06-01T10:01:00Z")
        st.ledger.freeze()

        val all = st.drops.snapshot()
        assertEquals(2, all.size)
        assertEquals("最初の報告が書き換えられた", first, all[0])
        assertArrayEquals(
            "最初の報告の置き場のバイトが変わった",
            firstBytes,
            dropsSegmentBytes().copyOfRange(0, firstBytes.size),
        )
        assertEquals(1, all[1].count)
        assertTrue(all[1].id != first.id)
    }

    // Scenario: 再送した報告は最初と同じ冪等キーを持つ
    @Test
    fun `失敗した報告の再送は最初と同じ本文を送る`() {
        drop("2026-06-01T10:00:00Z")
        st.ledger.freeze()
        val failing = FakeTransport { Outcome.Unreachable("timeout") }
        sender(failing).flush()
        val ok = FakeTransport { Outcome.Responded(200, """[{"accepted":true}]""") }
        sender(ok).flush()
        // 冪等キーは `logical_source` + `raw` からサーバが作る。本文が同じ＝同じ鍵
        assertEquals(failing.bodies.single(), ok.bodies.single())
        assertEquals(0, st.drops.size())
    }

    // Scenario: 断られた破棄の報告も未送信から取り除かれない
    @Test
    fun `恒久的な不正の種別で断られても未送信に残り、次の契機で再び送る`() {
        drop("2026-06-01T10:00:00Z")
        st.ledger.freeze()
        val rejecting = FakeTransport { Outcome.Responded(400, """[{"accepted":false,"error":"malformed"}]""") }
        val s = sender(rejecting)
        s.flush()
        assertEquals(1, st.drops.size())
        s.flush()
        assertEquals("次の契機で送り直していない", 2, rejecting.bodies.size)
        assertEquals(1, st.drops.size())
    }

    @Test
    fun `凍結の後、下書きを消す前に落ちても、同じ報告を二重に積まない`() {
        drop("2026-06-01T10:00:00Z")
        val openFile = File(st.dir, "drops-open.json")
        val beforeFreeze = openFile.readText()
        st.ledger.freeze()
        // 下書きのファイルが消える前に落ちた（凍結は積めた）
        openFile.writeText(beforeFreeze)
        val reborn = DropLedger(openFile, st.drops, { "user-1" }, "device-1", { st.now }, { "x" }, st.log)
        assertTrue("積んだ報告を下書きとして持ち直している", reborn.drafts().isEmpty())
        assertEquals(1, st.drops.size())
    }

    @Test
    fun `凍結の前に立て直されても下書きは残り、同じ本文の報告になる`() {
        drop("2026-06-01T10:00:00Z")
        val expected = st.ledger.drafts().single().toReport("user-1", "device-1")
        val reborn = DropLedger(File(st.dir, "drops-open.json"), st.drops, { "user-1" }, "device-1", { st.now }, { "x" }, st.log)
        reborn.freeze()
        assertEquals(expected.raw, st.drops.snapshot().single().raw)
    }
}
