// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * 収集を始めた時点の過去の利用の集計を取り込む（tasks 4.3 / design D3 / 本人の決定 Q5）。
 *
 * **生のイベントは 10 日で消えるが、集計は年ごとの箱に 2 年ぶん残っている。**
 * つまり取得元は**このアプリを入れるより前の最大 2 年ぶん**を既に持っており、
 * 取らなければ日が経つごとに消えていく。集計はイベントから作り直せない。
 */
@RunWith(RobolectricTestRunner::class)
class RollupImportTest {
    @Test
    fun `初回完了後の保存失敗は未永続化だけを報告して取り込み済みの印を保つ`() {
        val env = RollupTestEnv(FakeUsageSource(storedRollups = stored()))
        UsageRollupProgressStore(usageRollupProgressFile(env.dir, APP_USAGE_ROLLUP_LOGICAL_SOURCE),
            APP_USAGE_ROLLUP_LOGICAL_SOURCE, {}).save(UsageGranularity.entries.toSet())
        env.blockOutbox()
        env.collect()
        assertEquals(4, env.progressLines())
        assertTrue(env.lines.any { it.startsWith("kind=usage_rollup_not_persisted") })
        assertFalse(env.lines.any { it.startsWith("kind=usage_rollup_not_imported") })
    }

    private fun stored(): Map<UsageGranularity, List<UsageRollupSnapshot>> =
        UsageGranularity.entries.associateWith { granularity ->
            listOf(usageRollup(packageName = "dev.ashiato.${granularity.name.lowercase()}"))
        }

    // Scenario: 初回に 4 つの粒度が取り込まれる
    @Test
    fun `初めて始めたとき年と月と週と日の 4 つが積まれる`() {
        val env = RollupTestEnv(FakeUsageSource(storedRollups = stored()))

        val result = env.collect()

        assertTrue("取れたのに $result が返った", result is CollectionResult.Collected)
        // **年から順に**（粗いほうが先に消えるわけではないが、spec の並びに合わせる）
        assertEquals(
            listOf("yearly", "monthly", "weekly", "daily"),
            env.records().map { it.rawText("granularity") },
        )
        // **窓を切り詰めていない**（独立レビュー R4）。粒度ごとの見込みは API から読めないので、
        // 切ると取得元がそれより長く持っている端末で**まだ残っている箱を飛ばす**
        assertEquals(
            listOf(ROLLUP_QUERY_BEGIN),
            (env.source as FakeUsageSource).rollupQueries.map { it.second.begin }.distinct(),
        )
    }

    // Scenario: 集計はイベントとは別のソースに積まれる
    @Test
    fun `集計の記録の論理ソースはイベントのものと違う`() {
        val env = RollupTestEnv(FakeUsageSource(storedRollups = stored()))

        env.collect()

        assertEquals(
            listOf(APP_USAGE_ROLLUP_LOGICAL_SOURCE),
            env.records().map { it.logicalSource }.distinct(),
        )
        assertNotEquals(APP_USAGE_LOGICAL_SOURCE, APP_USAGE_ROLLUP_LOGICAL_SOURCE)
    }

    // Scenario: 集計の 1 件が粒度と期間と合計時間を持つ
    @Test
    fun `集計 1 件に粒度と期間とアプリと合計と最後に使った時刻が入る`() {
        val env = RollupTestEnv(
            FakeUsageSource(
                storedRollups = mapOf(
                    UsageGranularity.DAILY to listOf(
                        usageRollup(
                            packageName = "dev.ashiato.example",
                            firstAt = "2026-05-01T00:00:00Z",
                            lastAt = "2026-05-02T00:00:00Z",
                            totalForegroundMs = 3_600_000,
                        ),
                    ),
                ),
            ),
        )

        env.collect()

        val record = env.records().single { it.rawText("granularity") == "daily" }
        assertEquals("daily", record.rawText("granularity"))
        assertEquals("2026-05-01T00:00:00Z", record.rawText("begin"))
        assertEquals("2026-05-02T00:00:00Z", record.rawText("end"))
        assertEquals("dev.ashiato.example", record.rawText("package"))
        assertEquals("3600000", record.rawText("total_foreground_ms"))
        assertEquals("2026-05-02T00:00:00Z", record.rawText("last_used"))
        // 表示名は**解析済みにだけ**（取得時点の値。C2）
        assertEquals("例のアプリ", record.payloadText("app_label"))
        assertNotNull(record.payloadText("granularity"))
    }

    // Scenario: 取り込みが途中で終わっても次の契機で続きから入る
    @Test
    fun `年と月まで取り込んで止まったら、次は週から始まる`() {
        val inner = FakeUsageSource(storedRollups = stored())
        val source = StopAt(inner, UsageGranularity.WEEKLY)
        val env = RollupTestEnv(source)

        val stopped = env.collect()

        assertTrue("読めなかったのに $stopped が返った", stopped is CollectionResult.Unavailable)
        assertEquals(
            listOf(UsageGranularity.YEARLY, UsageGranularity.MONTHLY),
            inner.rollupQueries.map { it.first },
        )

        // 収集が止まって、また始まる（プロセスが作り直される）
        env.restart()
        source.stopAt = null
        inner.rollupQueries.clear()
        env.advance(ROLLUP_INTERVAL_MS)

        env.collect()

        assertEquals(
            "年と月を取り直している",
            listOf(UsageGranularity.WEEKLY, UsageGranularity.DAILY),
            inner.rollupQueries.map { it.first },
        )
    }

    // Scenario: 初回の後も日ごとの集計が 6 時間ごとに取り込まれる
    @Test
    fun `初回が終わった後の契機では日ごとだけが積まれる`() {
        val rollups = stored().toMutableMap()
        val inner = FakeUsageSource(storedRollups = rollups)
        val env = RollupTestEnv(inner)
        env.collect()
        val before = env.records().size
        inner.rollupQueries.clear()
        env.advance(ROLLUP_INTERVAL_MS)
        // **今日の箱は使うたびに育つ** —— 6 時間のあいだに 1 時間ぶん増えた
        rollups[UsageGranularity.DAILY] = listOf(
            usageRollup(packageName = "dev.ashiato.daily", totalForegroundMs = 3_660_000),
        )

        val result = env.collect()

        assertTrue("取れたのに $result が返った", result is CollectionResult.Collected)
        assertEquals(listOf(UsageGranularity.DAILY), inner.rollupQueries.map { it.first })
        assertEquals(
            listOf("daily"),
            env.records().drop(before).map { it.rawText("granularity") },
        )
        // **窓を切り詰めていない**（C3 / spec レビュー R3 / 独立レビュー R4）——
        // 見込みの保持で切ると、取得元がそれより長く持っている端末で残っている箱を飛ばす
        assertEquals(listOf(ROLLUP_QUERY_BEGIN), inner.rollupQueries.map { it.second.begin })
        assertEquals(listOf(env.now), inner.rollupQueries.map { it.second.end })
    }

    /**
     * **同じ内容の箱を積み直さない**（独立レビュー R3）。
     *
     * 日ごとの箱は取得元に 10 日ぶん残っていて、6 時間ごとの契機が毎回まるごと返させるので、
     * 覚えていないと同じ 1 件が寿命のあいだに約 40 回、**位置と同じ置き場**（C11）へ積まれる。
     * 窓のほうは切り詰めない（C3）ので、止めるのは積む側。
     */
    @Test
    fun `内容が変わらない箱は次の契機で積み直されない`() {
        val inner = FakeUsageSource(storedRollups = stored())
        val env = RollupTestEnv(inner)
        env.collect()
        val before = env.records().size
        assertEquals("初回で 4 件積まれていない（試験が空振りしている）", 4, before)

        env.advance(ROLLUP_INTERVAL_MS)
        val result = env.collect()

        assertTrue("取れたのに $result が返った", result is CollectionResult.Collected)
        assertEquals("同じ内容の箱が積み直された", before, env.records().size)
        assertEquals(0, (result as CollectionResult.Collected).enqueued.size)
        // **問い合わせは続けている**（窓は切り詰めない。止めたのは積む側だけ）
        assertEquals(listOf(ROLLUP_QUERY_BEGIN), inner.rollupQueries.map { it.second.begin }.distinct())
        assertTrue(env.lines.any { it.startsWith("kind=usage_rollup_already_sent") })
    }

    /** 立て直しをまたいでも覚えている（台帳は端末のファイル。`START_STICKY` で消えない）。 */
    @Test
    fun `収集が止まって始まり直しても同じ箱は積み直されない`() {
        val env = RollupTestEnv(FakeUsageSource(storedRollups = stored()))
        env.collect()
        val before = env.records().size

        env.restart()
        env.advance(ROLLUP_INTERVAL_MS)
        env.collect()

        assertEquals("立て直しのたびに積み直している", before, env.records().size)
    }

    /**
     * **見積り**（独立レビュー R3）。取得元が日ごとの箱を 10 日 × 50 アプリ持っている端末で、
     * 6 時間ごとの契機が 1 日に 4 回来る。積まれるのは**初回の 500 件と、その後は育った箱だけ**。
     *
     * 台帳を持たないと `10 日 × 50 × 4 回 = 2,000 件/日`（同じ箱が寿命のあいだに約 40 回）。
     * 位置と同じ置き場（C11）なので、design の Risk「端末の置き場を位置と食い合う」に直撃する。
     */
    @Test
    fun `日の粒度を 1 日ぶん回しても積まれるのは初回と育った箱だけ`() {
        val apps = 50
        val days = 10
        fun boxes(todayMs: Long) = (0 until days).flatMap { day ->
            (0 until apps).map { app ->
                usageRollup(
                    packageName = "dev.ashiato.app$app",
                    firstAt = "2026-05-%02dT00:00:00Z".format(10 + day),
                    lastAt = "2026-05-%02dT00:00:00Z".format(11 + day),
                    // **いちばん新しい箱だけが育つ**（今日のぶん）
                    totalForegroundMs = if (day == days - 1) todayMs else 60_000,
                )
            }
        }
        val rollups = mutableMapOf(UsageGranularity.DAILY to boxes(60_000))
        val env = RollupTestEnv(FakeUsageSource(storedRollups = rollups))

        env.collect()
        val initial = env.records().size
        // 1 日ぶん（6 時間 × 4 回）。2 回目以降は今日の箱だけが育つ
        var grown = 60_000L
        repeat(3) {
            env.advance(ROLLUP_INTERVAL_MS)
            grown += 60_000
            rollups[UsageGranularity.DAILY] = boxes(grown)
            env.collect()
        }

        assertEquals("初回は 10 日 × 50 アプリ", days * apps, initial)
        assertEquals(
            "初回の後に積まれたのは、育った今日の箱だけではない",
            initial + 3 * apps,
            env.records().size,
        )
    }

    /** 忘れる幅より古い箱は台帳から落ちる（**無制限に育てない**）。 */
    @Test
    fun `忘れる幅より古い箱は台帳に残らない`() {
        val old = mutableMapOf("a" to Instant.parse("2026-01-01T00:00:00Z"))
        val now = Instant.parse("2026-05-20T09:00:00Z")
        assertEquals(0, pruneRollupSeen(old, now).size)

        val fresh = mutableMapOf("b" to now.minusMillis(ROLLUP_SEEN_WINDOW_MS / 2))
        assertEquals(1, pruneRollupSeen(fresh, now).size)
    }

    /**
     * **件数で溢れたときは新しい箱から残す**（再レビュー F2）。
     *
     * 古いほうを残すと、**まだ取得元に残っている箱**（＝次の契機でまた返る箱）を先に忘れることになり、
     * 6 時間ごとに積み直す R3 の形がそのまま戻る。忘れてよいのは、取得元から先に消えて
     * 二度と返らない古い箱のほう。
     */
    @Test
    fun `件数で溢れたときは新しい箱から残る`() {
        val now = Instant.parse("2026-05-20T09:00:00Z")
        val seen = LinkedHashMap<String, Instant>()
        val total = ROLLUP_SEEN_MAX + 2
        // 全部が忘れる幅の内側（＝落ちるのは件数の分岐だけ）。`i` が大きいほど古い箱
        for (i in 0 until total) seen["f$i"] = now.minusMillis(i.toLong() * 1000)

        val kept = pruneRollupSeen(seen, now)

        assertEquals(ROLLUP_SEEN_MAX, kept.size)
        assertTrue("いちばん新しい箱が忘れられた", kept.containsKey("f0"))
        assertFalse("いちばん古い箱が残っている", kept.containsKey("f${total - 1}"))
        assertFalse("2 番目に古い箱が残っている", kept.containsKey("f${total - 2}"))
    }

    /**
     * **忘れる幅を越えた契機で台帳の行が落ちる**（再レビュー F3）。
     *
     * 有界性は「[pruneRollupSeen] が `collect` から呼ばれていること」が要で、
     * 関数を直接叩く試験だけでは呼び忘れを捕まえられない。
     */
    @Test
    fun `忘れる幅より先まで進めた契機で台帳の行が落ちる`() {
        val rollups = stored().toMutableMap()
        val env = RollupTestEnv(FakeUsageSource(storedRollups = rollups))
        env.collect()
        assertEquals("初回の 4 粒度が台帳に入っていない", 4, env.seenLines())

        env.advance(ROLLUP_SEEN_WINDOW_MS + ROLLUP_INTERVAL_MS)
        // 忘れる幅の**内側**にある新しい箱（これだけが残る）
        rollups[UsageGranularity.DAILY] = listOf(
            usageRollup(
                packageName = "dev.ashiato.daily",
                firstAt = "2026-06-18T00:00:00Z",
                lastAt = "2026-06-19T00:00:00Z",
            ),
        )

        env.collect()

        assertEquals("忘れる幅より古い行が落ちていない", 1, env.seenLines())
    }

    /**
     * **1 件も書けなかった契機では、その粒度が「取り込み済み」にならない**（再レビュー round 3）。
     *
     * spec は逐語で「年と月の粒度まで**取り込んだ**ところで収集が止まり」と書いており、
     * 取得元から**読めた**ことは取り込んだことではない。読めただけで印を付けると、
     * 置き場が満杯の端末で**年・月・週は二度と読まれない**（初回の 1 度きりだから）——
     * 年は 2 年ぶんが取得元からも日ごとに消えていく（`loss: uncaptured`）。
     */
    @Test
    fun `1 件も書けなかった契機ではその粒度が取り込み済みにならない`() {
        val inner = FakeUsageSource(storedRollups = stored())
        val env = RollupTestEnv(inner)
        env.blockOutbox()

        env.collect()

        assertEquals("書けていないのに取り込み済みの印が付いた", 0, env.progressLines())
        assertTrue(
            "印を付けずに残したことがログに出ていない: ${env.lines}",
            env.lines.any { it.startsWith("kind=usage_rollup_not_imported") },
        )

        // 空きが戻った（プロセスも立て直された）
        env.restart()
        env.outbox = testOutbox()
        inner.rollupQueries.clear()
        env.advance(ROLLUP_INTERVAL_MS)

        env.collect()

        assertEquals(
            "年・月・週が読み直されていない（初回の 1 度きりなので二度と読まれない）",
            ROLLUP_IMPORT_ORDER,
            inner.rollupQueries.map { it.first },
        )
        assertEquals(4, env.records().size)
        assertEquals(4, env.progressLines())
    }

    /**
     * **0 件が返った粒度は取り込み済みになる**（本人の決定 C7「0 件でも成功」）。
     *
     * 上の試験と**同じ `persisted == 0`** だが結論が逆 —— 書くものが無いだけで失敗ではない。
     * 同じ判定式にすると、過去が無い端末で 4 粒度が永久に読み直される。
     */
    @Test
    fun `0 件が返った粒度は取り込み済みになる`() {
        val inner = FakeUsageSource()
        val env = RollupTestEnv(inner)

        env.collect()

        assertEquals("0 件だった粒度に印が付いていない", 4, env.progressLines())
        assertEquals(0, env.records().size)
        assertFalse(
            "0 件を「書けなかった」と取り違えている: ${env.lines}",
            env.lines.any { it.startsWith("kind=usage_rollup_not_imported") },
        )

        inner.rollupQueries.clear()
        env.advance(ROLLUP_INTERVAL_MS)

        env.collect()

        assertEquals(
            "0 件だった粒度が読み直されている",
            listOf(ROLLUP_ONGOING_GRANULARITY),
            inner.rollupQueries.map { it.first },
        )
    }

    /**
     * **置き場に書けなかった箱は、次の契機で積み直される**（再レビュー F1）。
     *
     * `Outbox.add` が偽で返したものはメモリに載るが、端末の空きが尽きた状態が続けば
     * `MAX_UNWRITTEN` を超えて `lost` として手放される。書けた確認より先に台帳へ入れると、
     * **確定済みの箱は原文が変わらないので指紋も変わらず、その日のその集計は二度と積まれない。**
     * 積み直しは冪等で安全（サーバが内容の鍵で畳む）だが、積まないのは取りこぼし。
     */
    @Test
    fun `未送信に書けなかった箱は次の契機で積み直される`() {
        val env = RollupTestEnv(
            FakeUsageSource(storedRollups = mapOf(UsageGranularity.DAILY to listOf(usageRollup()))),
        )
        env.blockOutbox()

        env.collect()

        // 置き場には書けていない（`Outbox` がメモリに抱えているので `snapshot()` には出る）
        assertTrue(
            "置き場に書けたことになっている: ${env.lines}",
            env.lines.any { it.startsWith("kind=usage_rollup_not_persisted") },
        )
        assertEquals("書けていないのに台帳へ入れている", 0, env.seenLines())

        // 空きが戻った（プロセスも立て直された）
        env.restart()
        env.outbox = testOutbox()
        env.advance(ROLLUP_INTERVAL_MS)

        env.collect()

        assertEquals("書けなかった箱が二度と積まれない", 1, env.records().size)
        assertEquals(1, env.seenLines())
    }

    // Scenario: 集計の取得率は 6 時間を刻みとして数えられる
    @Test
    fun `集計の取得契機は 6 時間で、1 区間に 1 回入って成功する`() {
        val env = RollupTestEnv(FakeUsageSource(storedRollups = stored()))

        // 満点の刻み＝そのソースの取得間隔。24 時間にすると 6 時間の区間に契機が入らず、
        // 試行 0 / 成功 1 の信号を契約が `invalid_counts` で恒久的に断る
        assertEquals(ROLLUP_INTERVAL_MS, env.adapter().intervalMs)
        assertEquals(SourceCadence.APP_USAGE_ROLLUP.intervalMs, env.adapter().intervalMs)
        assertEquals(APP_USAGE_ROLLUP_LOGICAL_SOURCE, env.adapter().logicalSource)
        assertEquals(
            "生存信号の区間に取得契機が 1 回も入らない",
            1,
            (SourceCadence.APP_USAGE_ROLLUP.heartbeatIntervalMs / env.adapter().intervalMs).toInt(),
        )

        // その 1 回が成功する（**0 件でも成功**。本人の決定 C7）
        assertTrue(env.collect() is CollectionResult.Collected)
        val empty = RollupTestEnv(FakeUsageSource())
        assertTrue("0 件が失敗になっている", empty.collect() is CollectionResult.Collected)
    }
}

/**
 * 決めた粒度だけで読めなくなる取得元（取り込みが途中で終わる場面）。
 *
 * **偽物そのものには持たせない** —— `FakeUsageSource` の「読めなかった」は
 * 取得元まるごとの状態（解錠されていない端末）で、粒度ごとに割れるものではない。
 */
private class StopAt(private val inner: UsageSource, var stopAt: UsageGranularity?) : UsageSource {
    override fun events(window: CollectionWindow): EventsResult = inner.events(window)

    override fun rollups(granularity: UsageGranularity, window: CollectionWindow): RollupsResult =
        if (granularity == stopAt) RollupsResult.Unreadable("stopped") else inner.rollups(granularity, window)
}

/** 集計の取得の契機を 1 つずつ回す足場（tasks 4.3）。 */
private class RollupTestEnv(
    val source: UsageSource,
    var zone: ZoneId = ZoneId.of("Asia/Tokyo"),
    var label: String? = "例のアプリ",
) {
    val dir: File = Files.createTempDirectory("st06-rollup").toFile()
    val lines: MutableList<String> = mutableListOf()
    private val log: (String) -> Unit = { lines += it }
    val clock: FakeDeviceClock = FakeDeviceClock(wall = Instant.parse("2026-05-20T09:00:00Z").toEpochMilli())

    /** 記録の未送信。**[restart] を挟めば差し替えられる**（置き場が書けない端末を作るため） */
    var outbox: Outbox<IngestRequest> = testOutbox()

    /** 台帳のファイル（試験が行数を直接読む）。まだ 1 度も書いていなければ 0 行と数える */
    fun seenLines(): Int = lines(usageRollupSeenFile(dir, APP_USAGE_ROLLUP_LOGICAL_SOURCE))

    /** 取り込み済みの粒度の印（試験が行数を直接読む）。 */
    fun progressLines(): Int = lines(usageRollupProgressFile(dir, APP_USAGE_ROLLUP_LOGICAL_SOURCE))

    private fun lines(f: File): Int =
        if (f.exists()) f.readLines().filter { it.isNotBlank() }.size else 0

    /** 置き場に 1 件も書けない未送信（`mkdirs` が通らない道に置く。`WriteFailedTest` と同じ形）。 */
    fun blockOutbox() {
        val blocked = File(dir, "blocked-${System.nanoTime()}").apply { writeText("ディレクトリではない") }
        outbox = Outbox(
            SegmentStore(File(blocked, "records"), IngestRequest.serializer(), File(dir, "unreadable.jsonl"), log),
            age = { 0L },
        )
    }
    private var ids = 0
    private var current: AppUsageRollupSourceAdapter? = null

    val now: Instant get() = Instant.ofEpochMilli(clock.wall)

    fun adapter(): AppUsageRollupSourceAdapter = current ?: AppUsageRollupSourceAdapter(
        source = source,
        outbox = outbox,
        progressStore = UsageRollupProgressStore(
            usageRollupProgressFile(dir, APP_USAGE_ROLLUP_LOGICAL_SOURCE),
            APP_USAGE_ROLLUP_LOGICAL_SOURCE,
            log,
        ),
        seenStore = UsageRollupSeenStore(
            usageRollupSeenFile(dir, APP_USAGE_ROLLUP_LOGICAL_SOURCE),
            APP_USAGE_ROLLUP_LOGICAL_SOURCE,
            log,
        ),
        labels = { label },
        userId = { "user-1" },
        deviceId = "device-1",
        zone = { zone },
        newId = { "r${ids++}" },
        capabilityOf = { Capability.of(permission = true, sensor = true, network = true) },
        log = log,
    ).also { current = it }

    /** 収集が止まって、また始まる（プロセスが作り直される）。 */
    fun restart() {
        current = null
    }

    fun collect(): CollectionResult =
        adapter().collect(CollectionWindow(now.minusMillis(ROLLUP_INTERVAL_MS), now))

    fun advance(ms: Long) = clock.advance(ms)

    fun records(): List<IngestRequest> = outbox.snapshot()
}
