// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
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
        val inner = FakeUsageSource(storedRollups = stored())
        val env = RollupTestEnv(inner)
        env.collect()
        val before = env.records().size
        inner.rollupQueries.clear()
        env.advance(ROLLUP_INTERVAL_MS)

        val result = env.collect()

        assertTrue("取れたのに $result が返った", result is CollectionResult.Collected)
        assertEquals(listOf(UsageGranularity.DAILY), inner.rollupQueries.map { it.first })
        assertEquals(
            listOf("daily"),
            env.records().drop(before).map { it.rawText("granularity") },
        )
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
    val outbox: Outbox<IngestRequest> = testOutbox()
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
