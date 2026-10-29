// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.time.Instant

/** 破棄の報告の下書きは、作り始めた版を持ち、版が変わったら閉じる（ST49 / design D15）。 */
class St49DropDraftTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val v1 = "0.1.0+" + "a".repeat(40)
    private val v2 = "0.1.0+" + "b".repeat(40)
    private val t = Instant.parse("2026-06-01T10:00:00Z")

    private fun stores() = TestStores(dir = tmp.newFolder())

    private fun ledger(st: TestStores, version: String?, ids: Iterator<String>) = DropLedger(
        File(st.dir, "drops-open.json"),
        st.drops,
        { "user-1" },
        "device-1",
        { st.now },
        { ids.next() },
        st.log,
        LOGICAL_SOURCE,
        version,
    )

    // Scenario: 収集アプリの版が変わったら、破棄の報告の下書きは閉じて次の報告になる
    @Test
    fun versionChangeClosesDraft() {
        val st = stores()
        val ids = listOf("d1", "d2", "d3").iterator()
        val before = ledger(st, v1, ids)
        before.dropped(LOGICAL_SOURCE, DropReason.AGE, t)
        assertTrue(before.save())

        // 収集アプリを更新した（別の印）。同じ理由の破棄が続く
        val after = ledger(st, v2, ids)
        after.dropped(LOGICAL_SOURCE, DropReason.AGE, t.plusSeconds(60))
        after.endBatch(LOGICAL_SOURCE, DropReason.AGE, null)

        val drafts = after.drafts()
        assertEquals(2, drafts.size)
        assertTrue("古い版の下書きは閉じる", drafts[0].closed)
        assertEquals(v1, drafts[0].clientVersion)
        assertEquals(1, drafts[0].count)
        assertFalse(drafts[1].closed)
        assertEquals("次の下書きは今の版", v2, drafts[1].clientVersion)

        after.freeze()
        val sent = st.drops.snapshot()
        assertEquals(listOf(v1, v2), sent.map { it.clientVersion })
    }

    // Scenario: 版を持たない保存済みの下書きは閉じられ、その報告は版の欄の無いまま送られる
    @Test
    fun draftWithoutVersionIsClosedAndSentWithoutVersion() {
        val st = stores()
        // ST49 より前の下書き（版の欄を持たない。逐語）
        File(st.dir, "drops-open.json").writeText(
            """[{"id":"old","source":"c01-location","reason":"age","createdAt":"2026-06-01T10:00:00Z",""" +
                """"startMs":1780308000000,"endMs":1780308000001,"lastMs":1780308000000,"count":1,""" +
                """"hourly":{"1780308000000":1},"closed":false}]""",
        )
        val ledger = ledger(st, v2, listOf("n1").iterator())
        assertTrue("版の無い下書きは閉じて持つ", ledger.drafts().single().closed)

        ledger.dropped(LOGICAL_SOURCE, DropReason.AGE, t.plusSeconds(60))
        ledger.freeze()

        val sent = st.drops.headStored(10)
        assertEquals(2, sent.size)
        val old = sent.single { it.item.id == "old" }
        assertFalse("今の版を付けてはならない", old.json!!.containsKey("client_version"))
        assertEquals(1, old.item.count)
        assertEquals(v2, sent.single { it.item.id == "n1" }.json!!["client_version"]!!.toString().trim('"'))
    }
}
