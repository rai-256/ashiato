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
 * ST01 / ST02 の 1 本の JSONL を、初回の起動で区切りへ取り込む（ST04 / tasks 5.2 / design D1）。
 * **取り込めたら元を消す。取り込めなかったら元を残す**（消すと、その分が端末から痕跡なく消える）。
 */
class SegmentMigrationTest {
    private val files: File = Files.createTempDirectory("legacy").toFile()
    private val st = TestStores()

    private fun req(id: String) =
        LocationFix(35.68, 139.76, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    private fun migrate(legacy: File) = migrateLegacyOutbox(
        legacy, st.records, IngestRequest.serializer(), st.unreadable, File(st.dir, "salvaged"), st.log,
    )

    @Test
    fun `既存の outbox jsonl を取り込み、取り込めたら元を消す`() {
        val legacy = File(files, "outbox.jsonl")
        legacy.writeText(listOf("a", "b", "c").joinToString("") { ingestJson.encodeToString(IngestRequest.serializer(), req(it)) + "\n" })

        val got = migrate(legacy)

        assertEquals(LegacyMigration(imported = 3, unreadable = 0, complete = true), got)
        assertFalse("取り込んだのに元が残っている（次の起動で二重に積む）", legacy.exists())
        assertEquals(listOf("a", "b", "c"), st.records.snapshot().map { it.id })
        // 原文の文字列が変わらない（冪等キーがこの文字列から作られる）
        assertEquals(req("a").raw, st.records.snapshot().first().raw)
    }

    @Test
    fun `既存の heartbeat jsonl も同じ仕組みで取り込む`() {
        val legacy = File(files, "heartbeat.jsonl")
        val beat = HeartbeatRequest("h1", "user-1", LOGICAL_SOURCE, "device-1", "2026-05-01T00:00:00Z", true, emptyList(), 360, 230, "{}")
        legacy.writeText(ingestJson.encodeToString(HeartbeatRequest.serializer(), beat) + "\n")
        val got = migrateLegacyOutbox(legacy, st.beats, HeartbeatRequest.serializer(), st.unreadable, File(st.dir, "salvaged"), st.log)
        assertTrue(got.complete)
        assertEquals(listOf("h1"), st.beats.snapshot().map { it.id })
        assertFalse(legacy.exists())
    }

    @Test
    fun `書きかけの tmp しか無ければそれを取り込む`() {
        // ST01 の置き場は書き換えの途中で落ちると最新の全件を `.tmp` に残す（ST01 review CRITICAL-2）
        File(files, "outbox.jsonl.tmp").writeText(ingestJson.encodeToString(IngestRequest.serializer(), req("rescued")) + "\n")
        migrate(File(files, "outbox.jsonl"))
        assertEquals(listOf("rescued"), st.records.snapshot().map { it.id })
        assertFalse(File(files, "outbox.jsonl.tmp").exists())
    }

    @Test
    fun `書き込めなかったら元を消さない`() {
        val legacy = File(files, "outbox.jsonl")
        legacy.writeText(ingestJson.encodeToString(IngestRequest.serializer(), req("keep")) + "\n")
        val blocked = File(files, "blocked").apply { writeText("x") }
        val broken = Outbox(
            SegmentStore(File(blocked, "records"), IngestRequest.serializer(), st.unreadable, st.log),
            age = { 0L },
        )
        val got = migrateLegacyOutbox(legacy, broken, IngestRequest.serializer(), st.unreadable, File(st.dir, "salvaged"), st.log)
        assertFalse(got.complete)
        assertTrue("書けなかったのに元を消した", legacy.exists())
    }

    @Test
    fun `取り込みは 1 度だけ（2 回目の起動では何もしない）`() {
        val legacy = File(files, "outbox.jsonl")
        legacy.writeText(ingestJson.encodeToString(IngestRequest.serializer(), req("once")) + "\n")
        migrate(legacy)
        migrate(legacy)
        assertEquals(listOf("once"), st.records.snapshot().map { it.id })
    }
}
