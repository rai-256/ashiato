// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import java.io.File
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner

/**
 * **破棄の報告は、そのデータのソースを名乗る**（ST06 / tasks 5.1 / 独立レビュー R10）。
 *
 * ST06 より前の収集はソースが 1 本だったので、`LocationService` は
 * 「置き場に書けなかった」「置き場の行が読めなかった」を**位置の定数で**報告していた。
 * 2 本目を同じ置き場に載せた瞬間（本人の決定 C11）、**アプリ利用の破棄が位置の破棄として届く** ——
 * 扉 #14（「データが無い」と「動きが無かった」を区別する）の証拠が、ソースごとに静かに誤る。
 */
@RunWith(RobolectricTestRunner::class)
class SourceAttributionTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private val outboxDir: File get() = File(app.filesDir, LocationService.OUTBOX_DIR)

    private fun start(): TestableLocationService =
        Robolectric.buildService(TestableLocationService::class.java, Intent())
            .create().startCommand(0, 1).get()

    private fun draftsBySource(service: TestableLocationService, reason: DropReason) =
        service.ledgerForTest.drafts().filter { it.reason == reason.wire }.associate { it.source to it.count }

    // ------------------------------------------------------------------ 置き場に書けなかった記録

    /**
     * 書けなかった記録の**固定長の数えはソースごとに別ファイル**。
     * 1 本にまとめると、時間ごとの枠しか持たない数えがソースを覚えられない。
     */
    @Test
    fun `書けなかった記録の数えはソースごとの置き場に載る`() {
        val usage = writeFailedFile(outboxDir, APP_USAGE_LOGICAL_SOURCE)
        usage.parentFile.mkdirs()
        WriteFailedLedger(usage) {}.failed(Instant.parse("2026-05-20T10:00:00Z"))

        val service = start()

        assertEquals(
            "アプリ利用の書き込みの失敗が位置の破棄として報告されている",
            mapOf(APP_USAGE_LOGICAL_SOURCE to 1),
            draftsBySource(service, DropReason.WRITE_FAILED),
        )
    }

    /** ST06 より前の 1 本（`write-failed.bin`）は**位置の名前へ移る**（位置以外へは引き継がない）。 */
    @Test
    fun `ST06 より前の数えは位置の名前へ移る`() {
        outboxDir.mkdirs()
        val legacy = File(outboxDir, LEGACY_WRITE_FAILED_FILE)
        WriteFailedLedger(legacy) {}.failed(Instant.parse("2026-05-20T10:00:00Z"))

        val service = start()

        assertEquals(
            mapOf(SourceCadence.LOCATION.logicalSource to 1),
            draftsBySource(service, DropReason.WRITE_FAILED),
        )
        assertFalse("ST06 より前の置き場が残っている", legacy.exists())
        assertTrue(writeFailedFile(outboxDir, SourceCadence.LOCATION.logicalSource).exists())
    }

    /** 位置の数えを**別のソースへは引き継がない**（位置の失敗をアプリ利用の破棄として報告しない）。 */
    @Test
    fun `ST06 より前の数えはアプリ利用へは引き継がれない`() {
        outboxDir.mkdirs()
        File(outboxDir, LEGACY_WRITE_FAILED_FILE).writeText("x")

        migrateLegacyWriteFailed(outboxDir, APP_USAGE_LOGICAL_SOURCE)

        assertFalse(
            "位置の数えがアプリ利用の置き場になった",
            writeFailedFile(outboxDir, APP_USAGE_LOGICAL_SOURCE).exists(),
        )
    }

    // ------------------------------------------------------------------ 置き場の読めない行

    /**
     * 退避した行は**行ごとに名前を読む**。壊れていて読めない行だけが
     * [LocationService.UNATTRIBUTED_SOURCE] を名乗る（報告そのものは落とさない）。
     */
    @Test
    fun `読めない行の報告は行ごとのソースに分かれる`() {
        outboxDir.mkdirs()
        File(outboxDir, LocationService.UNREADABLE).writeText(
            """
            {"id":"a","user_id":"u","logical_source":"c01-location","origin":"c01","ev
            {"id":"b","user_id":"u","logical_source":"c01-app-usage","origin":"c01","e
            {"id":"c","user_id":"u","logical_so
            """.trimIndent() + "\n",
        )

        val service = start()

        assertEquals(
            "読めない行が全部 1 つのソースの破棄になっている",
            mapOf(
                // 1 行目 ＋ 名前を読めなかった 3 行目
                SourceCadence.LOCATION.logicalSource to 2,
                APP_USAGE_LOGICAL_SOURCE to 1,
            ),
            draftsBySource(service, DropReason.UNREADABLE),
        )
    }

    /** 同じ行を 2 度数えない（印は報告できた行数まで進む）。 */
    @Test
    fun `見回りを重ねても読めない行は数え直さない`() {
        outboxDir.mkdirs()
        File(outboxDir, LocationService.UNREADABLE).writeText(
            """{"id":"a","logical_source":"c01-app-usage","x":1""" + "\n",
        )

        val service = start()
        service.maintainForTest()
        service.maintainForTest()

        assertEquals(mapOf(APP_USAGE_LOGICAL_SOURCE to 1), draftsBySource(service, DropReason.UNREADABLE))
    }

    /** **知らない名前は名乗らせない** —— 登録簿に無い名前は受け口に断られ、報告が端末に居座る。 */
    @Test
    fun `壊れた行から拾った知らない名前は使わない`() {
        assertEquals(
            SourceCadence.LOCATION.logicalSource,
            sourceOfUnreadableLine("""{"id":"a","logical_source":"c01-location","origin":"""),
        )
        assertEquals(
            APP_USAGE_ROLLUP_LOGICAL_SOURCE,
            sourceOfUnreadableLine("""{"logical_source":"c01-app-usage-rollup"}"""),
        )
        assertNull(sourceOfUnreadableLine("""{"logical_source":"c99-made-up"}"""))
        assertNull(sourceOfUnreadableLine("""{"id":"a","or"""))
        // 原文の中の同じ綴り（`\"` で入る）には当たらない
        assertNull(sourceOfUnreadableLine("""{"raw":"{\"logical_source\":\"c01-location\"}"""))
    }

    /**
     * **1 度の保存で全部入れる**（保存できなければ 1 件も残さない）。
     * ソースごとに分けて保存すると、途中で失敗したときに済んだ分が次の見回りで二重に数えられる。
     */
    @Test
    fun `ソースごとの件数は保存できなければ 1 件も残らない`() {
        val st = TestStores()
        // 書けない置き場（ディレクトリにできない道）に下書きを置く
        val blocked = File(st.dir, "blocked").apply { writeText("ディレクトリではない") }
        val ledger = DropLedger(
            File(blocked, "drops-open.json"), st.drops, { "user-1" }, "device-1",
            { st.now }, { "d" }, st.log, LOGICAL_SOURCE,
        )

        val saved = ledger.unreadable(mapOf(LOGICAL_SOURCE to 1, APP_USAGE_LOGICAL_SOURCE to 2))

        assertFalse("書けていないのに真を返している", saved)
        assertTrue("保存できなかったのに下書きが残っている", ledger.drafts().isEmpty())
    }
}
