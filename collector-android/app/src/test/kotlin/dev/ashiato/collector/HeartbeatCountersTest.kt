// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * 前回の信号からの取得の試行回数と成功回数（第 5 回 Q17。tasks 7.2b / ST06 の tasks 1.3）。
 *
 * **これが ST01 の R46（Doze）が渡した宿題の答え** —— 信号が来ている＝生きていた /
 * 取得率が低い＝眠っていた / 信号が来ない＝死んでいた、の 3 つが分かれる。
 * R46 が「14 分 〜 18 時間の間は判別できない」と書いた区間が、この比で埋まる。
 *
 * **満点の刻みはソースごと**（ST06 / design D5）。位置の 60 秒に固定したままだと、
 * 30 分間隔のアプリ利用が 6 時間ごとに「試行 360 / 成功 12」を送り、
 * **取得率 3 % ＝ ずっと眠っていた**と読まれる（独立レビュー R10）。
 */
class HeartbeatCountersTest {
    /** 進む時計。 */
    private class Clock(var at: Instant) : () -> Instant {
        override fun invoke(): Instant = at

        fun advanceMinutes(n: Long) {
            at = at.plusSeconds(n * 60)
        }
    }

    private fun counters(
        clock: Clock,
        cadence: SourceCadence = SourceCadence.LOCATION,
        store: CounterStore = MemoryCounterStore(),
    ) = AttemptCounters(now = clock, intervalMs = cadence.intervalMs, store = store)

    /** そのソースの生存信号を 1 件積んで、積まれたものを返す。 */
    private fun emit(cadence: SourceCadence, counters: AttemptCounters, clock: Clock): HeartbeatRequest {
        val outbox = testHeartbeatOutbox()
        HeartbeatEmitter(
            outbox = outbox,
            counters = counters,
            userId = "user-1",
            deviceId = "device-1",
            logicalSource = cadence.logicalSource,
            capability = { Capability.of(permission = true, sensor = true, network = true) },
            now = clock,
            newId = { "hb-1" },
        ).emit()
        return outbox.snapshot().single()
    }

    /// Scenario: 取得の試行と成功の数が信号に載る
    @Test
    fun `6 時間で 360 回が満点になり、取れた数がそのまま成功になる`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = counters(clock)
        repeat(230) { counters.recordSuccess() }
        clock.advanceMinutes(360) // 6 時間 = 60 秒間隔で 360 回

        val (attempts, successes) = counters.take()
        assertEquals("6 時間 ÷ 60 秒 = 360 が満点", 360, attempts)
        assertEquals(230, successes)
    }

    /**
     * **満点の刻みはそのソースの取得間隔**（本人の決定 C7 / design D5）。
     * アプリ利用は 30 分なので、6 時間の区間の満点は 12。
     */
    // Scenario: 取得率はソースごとの刻みで数えられる
    @Test
    fun `アプリ利用は 30 分を刻みに数え、6 時間の区間は 12 が満点になる`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = counters(clock, SourceCadence.APP_USAGE)
        repeat(12) { counters.recordSuccess() }
        clock.advanceMinutes(360)

        val beat = emit(SourceCadence.APP_USAGE, counters, clock)
        assertEquals("c01-app-usage", beat.logicalSource)
        assertEquals("6 時間 ÷ 30 分 = 12 が満点", 12, beat.attempts)
        assertEquals(12, beat.successes)
    }

    /**
     * 集計の取得契機は 6 時間（design D3）。**生存信号の区間と同じ**なので満点は 1。
     * 24 時間にすると試行 0 / 成功 1 の信号ができ、契約が `invalid_counts` で恒久的に断る。
     */
    // Scenario: 集計の取得率は 6 時間を刻みとして数えられる
    @Test
    fun `集計は 6 時間を刻みに数え、6 時間の区間は 1 が満点になる`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = counters(clock, SourceCadence.APP_USAGE_ROLLUP)
        counters.recordSuccess()
        clock.advanceMinutes(360)

        val beat = emit(SourceCadence.APP_USAGE_ROLLUP, counters, clock)
        assertEquals("c01-app-usage-rollup", beat.logicalSource)
        assertEquals("6 時間 ÷ 6 時間 = 1 が満点", 1, beat.attempts)
        assertEquals(1, beat.successes)
    }

    /**
     * **総当たり。** どのソースも生存信号の 1 区間に取得契機が 1 回以上入る
     * —— 入らないと試行 0 / 成功 1 の信号ができ、契約が `invalid_counts` で
     * **恒久的に**断る（断られた信号は理由を問わず未送信に残るので、区間ごとに 1 件ずつ積み上がる）。
     */
    // Scenario: どのソースも生存信号の区間に取得契機が 1 回以上入る
    @Test
    fun `どのソースも生存信号の区間に取得契機が 1 回以上入る`() {
        assertTrue("ソースが 1 本も登録されていない（試験が空振りしている）", SourceCadence.entries.isNotEmpty())
        for (cadence in SourceCadence.entries) {
            assertTrue(
                "${cadence.logicalSource}: 生存信号の区間 ${cadence.heartbeatIntervalMs} ms に " +
                    "取得契機（${cadence.intervalMs} ms）が 1 回も入らない",
                cadence.heartbeatIntervalMs >= cadence.intervalMs,
            )
            // 区間をまるごと寝ていても、満点は 1 以上になる（試行 0 / 成功 1 が作れない）
            val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
            val counters = counters(clock, cadence)
            counters.recordSuccess()
            clock.at = clock.at.plusMillis(cadence.heartbeatIntervalMs)
            val counts = counters.take()
            assertTrue("${cadence.logicalSource}: 区間の満点が 1 未満", counts.attempts >= 1)
            assertTrue("${cadence.logicalSource}: 成功が試行を超えた", counts.successes <= counts.attempts)
        }
    }

    /**
     * **2 ソースを同時に数える。** 置き場をソースごとに分けていないと、
     * 2 本目が同じファイルを開いた瞬間に互いを潰し合う（独立レビュー R10）。
     */
    @Test
    fun `2 ソースの数えは互いを潰さない`() {
        val dir = java.nio.file.Files.createTempDirectory("counters").toFile()
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val location = counters(
            clock, SourceCadence.LOCATION,
            FileCounterStore(counterStoreFile(dir, SourceCadence.LOCATION.logicalSource), SourceCadence.LOCATION.logicalSource) {},
        )
        val usage = counters(
            clock, SourceCadence.APP_USAGE,
            FileCounterStore(counterStoreFile(dir, SourceCadence.APP_USAGE.logicalSource), SourceCadence.APP_USAGE.logicalSource) {},
        )
        assertNotEquals(
            "2 ソースが同じ置き場を開いている",
            counterStoreFile(dir, SourceCadence.LOCATION.logicalSource),
            counterStoreFile(dir, SourceCadence.APP_USAGE.logicalSource),
        )

        repeat(300) { location.recordSuccess() }
        repeat(9) { usage.recordSuccess() }
        clock.advanceMinutes(360)

        assertEquals("位置の数えがアプリ利用に潰された", Counts(360, 300), location.take())
        assertEquals("アプリ利用の数えが位置に潰された", Counts(12, 9), usage.take())
    }

    /**
     * ST06 より前は置き場が 1 本（`heartbeat-counters.txt`）だった。**位置の名前へ移す** ——
     * 移せなければ新品から始めてよい（数えは証拠ではなく目安で、失っても記録は消えない）。
     */
    @Test
    fun `ST06 より前の 1 本の置き場は位置の名前へ移る`() {
        val dir = java.nio.file.Files.createTempDirectory("counters").toFile()
        val legacy = java.io.File(dir, LEGACY_COUNTERS_FILE)
        legacy.writeText("2026-05-01T00:00:00Z 4")

        val moved = migrateLegacyCounters(dir, SourceCadence.LOCATION.logicalSource)
        assertEquals(counterStoreFile(dir, SourceCadence.LOCATION.logicalSource), moved)
        assertTrue("位置の名前へ移っていない", moved.exists())
        assertTrue("元の 1 本が残っている", !legacy.exists())

        val clock = Clock(Instant.parse("2026-05-01T00:10:00Z"))
        val counters = counters(clock, SourceCadence.LOCATION, FileCounterStore(moved, LOGICAL_SOURCE) {})
        assertEquals("移す前の数えが読めていない", Counts(10, 4), counters.peek())
    }

    /** **アプリ利用は引き継がない** —— 位置の数えを別のソースの取得率として読ませない。 */
    @Test
    fun `ST06 より前の 1 本はアプリ利用へは移らない`() {
        val dir = java.nio.file.Files.createTempDirectory("counters").toFile()
        java.io.File(dir, LEGACY_COUNTERS_FILE).writeText("2026-05-01T00:00:00Z 42")

        val target = migrateLegacyCounters(dir, SourceCadence.APP_USAGE.logicalSource)
        assertTrue("位置の数えがアプリ利用へ移った", !target.exists())
    }

    /// Scenario: 数えは信号を送るたびに戻る
    @Test
    fun `信号を送ると数えが戻る`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = counters(clock)
        repeat(230) { counters.recordSuccess() }
        clock.advanceMinutes(360)
        counters.take()

        // 次の区間は 10 分（= 10 回）で 4 回だけ取れた
        clock.advanceMinutes(10)
        repeat(4) { counters.recordSuccess() }
        val (attempts, successes) = counters.take()
        assertEquals("累計になっていない（前の 360 を持ち越している）", 10, attempts)
        assertEquals(4, successes)
    }

    /** **読むだけでは戻さない** —— 信号を組み立てられなかったときに数えが消えないため。 */
    @Test
    fun `peek は数えを戻さない`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = counters(clock)
        repeat(3) { counters.recordSuccess() }
        clock.advanceMinutes(10)
        assertEquals(Counts(10, 3), counters.peek())
        assertEquals(Counts(10, 3), counters.peek())
        assertEquals(Counts(10, 3), counters.take())
        assertEquals(Counts(0, 0), counters.take())
    }

    /**
     * **成功が試行を超える信号を作らない**（2 巡目 R7）。端末の時計が戻ると
     * 経過が負になり、受け口に断られた信号が未送信に居座る。
     */
    @Test
    fun `時計が戻っても成功が試行を超えない`() {
        val clock = Clock(Instant.parse("2026-05-01T06:00:00Z"))
        val counters = counters(clock)
        repeat(5) { counters.recordSuccess() }
        clock.at = Instant.parse("2026-05-01T00:00:00Z") // 6 時間戻った
        val (attempts, successes) = counters.take()
        assertTrue("成功 $successes が試行 $attempts を超えている", successes <= attempts)
        assertEquals(5, attempts)
    }

    /**
     * **型で壊せなくする**（spec レビュー R4 / tasks 1.3）。`successes > attempts` の数えは
     * 組み立てられない —— 契約（`docs/collector-contract.md`）はその信号を `invalid_counts` で
     * **恒久的に**断るので、作れてしまうと未送信に永久に居座る。
     */
    @Test
    fun `成功が試行を超える数えは組み立てられない`() {
        var rejected = false
        try {
            Counts(attempts = 1, successes = 2)
        } catch (e: IllegalArgumentException) {
            rejected = true
        }
        assertTrue("成功が試行を超える数えが通った", rejected)
    }

    /**
     * 数えが**プロセスの立て直しをまたいで残る**（ST02 の review/code.md の R16 / C-2 / I8）。
     *
     * `Outbox` は「インスタンスの中だけに積むと立て直しで無言で消える」を理由に
     * ファイルへ落としたのに、**同じ理由が当てはまる数えは落とされていなかった**。
     * `since` も一緒に新品になるので、**死んでいた区間そのものが観測から落ちる** ——
     * 6 時間のうち 5 時間 50 分死んで 10 分前に立て直されると、次の信号は
     * `10 / 10` で「取得率 100 %」になり、画面には「健全」と出る。
     */
    @Test
    fun `数えは立て直しをまたいで残る`() {
        val dir = java.nio.file.Files.createTempDirectory("counters").toFile()
        val file = counterStoreFile(dir, LOGICAL_SOURCE)
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))

        val first = counters(clock, SourceCadence.LOCATION, FileCounterStore(file, LOGICAL_SOURCE) {})
        repeat(3) { first.recordSuccess() }
        clock.advanceMinutes(350)

        // プロセスが立て直された（新しいインスタンスで同じ置き場を開く）
        val second = counters(clock, SourceCadence.LOCATION, FileCounterStore(file, LOGICAL_SOURCE) {})
        clock.advanceMinutes(10)
        val (attempts, successes) = second.take()
        assertEquals("区間の起点が立て直しで新品になっている", 360, attempts)
        assertEquals("成功の数えが消えている", 3, successes)
    }

    /**
     * **積めなかったら数えは戻らない**（ST02 の review/code.md の R24 / H-7 / F9 / I9）。
     *
     * `peek()` の docstring が「読むだけでは戻さない —— 信号を組み立てられなかったときに
     * 数えが消える」と書いていた当の問題が、`emit()` 側で起きていた。
     */
    @Test
    fun `積めなかった区間の数えは残る`() {
        val clock = Clock(Instant.parse("2026-05-01T00:00:00Z"))
        val counters = counters(clock)
        repeat(5) { counters.recordSuccess() }
        clock.advanceMinutes(10)

        // 積めなかった
        val (stored, _) = counters.takeAfter { false to Unit }
        assertEquals(false, stored)
        assertEquals("積めていないのに数えが戻った", Counts(10, 5), counters.peek())

        // 積めた
        val (ok, _) = counters.takeAfter { true to Unit }
        assertEquals(true, ok)
        assertEquals(Counts(0, 0), counters.peek())
    }
}
