// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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
        // **記録の送信器**（捨てる側。ST03 / FR-10 の改訂）—— 本番と同じ形で見る
        Sender(
            outbox,
            { outcome },
            IngestRequest.serializer(),
            dropPermanentlyRejected = true,
            log = lines::add,
        ).flush()
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
        val line = Telemetry.line("send", source = LOGICAL_SOURCE, count = 3, elapsedMs = 120, error = "timeout")
        assertTrue(line == "kind=send source=c01-location count=3 elapsed_ms=120 error=timeout")
    }

    /** ソースに属さない配管（置き場・時計）のログは**誰の名も騙らない**。 */
    @Test
    fun `ソースに属さないログはソース名を名乗らない`() {
        val line = Telemetry.line("clock_jump", source = null)
        assertTrue(line == "kind=clock_jump")
    }

    /**
     * **ソース名は書き手が名乗る**（独立レビュー R10）。`Telemetry` に焼き込んであったときは、
     * 2 本目のソースが書いたログまで `source=c01-location` と出て、
     * 「位置は取れているのにアプリ利用が断られている」が**ログから読めなかった**。
     */
    /**
     * **ひと組が混ざっていても名乗る**（code-verify R27）。記録の置き場は全ソースで 1 本
     * （既定 C11）で、位置は 60 秒ごと・送信は 5 分ごとなので、**アプリ利用が載るひと組には
     * ほぼ常に位置も載る**。ひと組に 1 本だけ名を付ける形（`singleOrNull()`）だったときは、
     * その混ざったひと組では `kind=send_failed count=2 error=timeout` と出て
     * **`source=` が丸ごと落ちていた** —— アプリ利用だけのひと組しか観測していなかったので、
     * 通常の運用でこの Scenario が成り立たないことに試験が気付かなかった。
     */
    // Scenario: 端末のログのソース名がそのソースを指す
    @Test
    fun `位置と混ざったひと組でもアプリ利用の失敗はアプリ利用を名乗る`() {
        val outbox = testOutbox()
        outbox.add(req("l1"))                 // 位置 1 件
        listOf("u1", "u2").forEach { outbox.add(usageRequest(it)) }   // アプリ利用 2 件
        val lines = mutableListOf<String>()
        Sender(
            outbox,
            { Outcome.Unreachable("timeout") },
            IngestRequest.serializer(),
            dropPermanentlyRejected = true,
            log = lines::add,
        ).flush()

        val failed = lines.filter { it.startsWith("kind=send_failed") }
        // **ソースごとに 1 行ずつ、そのソースの件数で**（混ぜて count=3 の 1 行にしない）
        assertEquals(
            listOf(
                "kind=send_failed source=$LOGICAL_SOURCE count=1 error=timeout",
                "kind=send_failed source=${SourceCadence.APP_USAGE.logicalSource} count=2 error=timeout",
            ),
            failed,
        )
        val usage = failed.single { it.contains("source=${SourceCadence.APP_USAGE.logicalSource}") }
        assertFalse("アプリ利用の行が位置を名乗っている: $usage", usage.contains("source=$LOGICAL_SOURCE"))
        assertNoPrivateData(lines)
    }

    /** アプリ利用だけのひと組でも同じ（上の試験が混ざり方に依存していないこと）。 */
    @Test
    fun `アプリ利用の送信が失敗したログはアプリ利用を名乗る`() {
        val outbox = testOutbox()
        listOf("u1", "u2").forEach { outbox.add(usageRequest(it)) }
        val lines = mutableListOf<String>()
        Sender(
            outbox,
            { Outcome.Unreachable("timeout") },
            IngestRequest.serializer(),
            dropPermanentlyRejected = true,
            log = lines::add,
        ).flush()

        val failed = lines.single { it.startsWith("kind=send_failed") }
        assertTrue(
            "アプリ利用のログがアプリ利用を名乗っていない: $failed",
            failed.contains("source=${SourceCadence.APP_USAGE.logicalSource}"),
        )
        assertFalse("アプリ利用のログが位置を名乗っている: $failed", failed.contains(LOGICAL_SOURCE))
    }

    /**
     * **受理と拒否も混ざったひと組でソースごとに分かれる**（code-verify R27）——
     * 「位置は通っているのにアプリ利用だけ断られている」は、この 2 行の差でしか読めない。
     */
    @Test
    fun `混ざったひと組では受理と拒否もソースごとに分かれる`() {
        val outbox = testOutbox()
        outbox.add(req("l1"))                 // 位置 1 件（受理される）
        listOf("u1", "u2").forEach { outbox.add(usageRequest(it)) }   // アプリ利用 2 件（断られる）
        val lines = mutableListOf<String>()
        Sender(
            outbox,
            {
                Outcome.Responded(
                    200,
                    """[{"accepted":true},{"accepted":false,"error":"unknown_source"},""" +
                        """{"accepted":false,"error":"unknown_source"}]""",
                )
            },
            IngestRequest.serializer(),
            dropPermanentlyRejected = true,
            log = lines::add,
        ).flush()

        val usage = SourceCadence.APP_USAGE.logicalSource
        assertEquals(
            listOf("kind=rejected source=$usage count=2 error=unknown_source"),
            lines.filter { it.startsWith("kind=rejected") },
        )
        assertEquals(
            listOf(
                "kind=accepted source=$LOGICAL_SOURCE count=1",
                "kind=accepted source=$usage count=0",
            ),
            lines.filter { it.startsWith("kind=accepted") },
        )
        assertNoPrivateData(lines)
    }

    /** 同じ経路でも、位置の送信が失敗したログは位置を名乗る（上の試験が空振りしていないこと）。 */
    @Test
    fun `位置の送信が失敗したログは位置を名乗る`() {
        val failed = linesFor(Outcome.Unreachable("timeout")).single { it.startsWith("kind=send_failed") }
        assertTrue("位置のログが位置を名乗っていない: $failed", failed.contains("source=$LOGICAL_SOURCE"))
    }

    private fun usageRequest(id: String) = IngestRequest(
        id = id,
        userId = "user-0001",
        logicalSource = SourceCadence.APP_USAGE.logicalSource,
        deviceId = "device-secret",
        origin = "collected",
        eventTime = "2026-09-08T02:00:00Z",
        tzOffsetMin = 540,
        tzId = "Asia/Tokyo",
        schemaVersion = 1,
        raw = "{}",
        payload = JsonObject(emptyMap()),
    )
}
