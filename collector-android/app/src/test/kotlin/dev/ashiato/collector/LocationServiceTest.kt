// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.Service
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import com.google.android.gms.location.LocationCallback
import java.io.File
import java.time.Instant
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner

/** 取得元の偽物。渡された間隔を覚える。 */
class FakeFixSource(private val fail: Boolean) : FixSource {
    var startedWith: Long? = null
    var stopped = false
    override fun start(intervalMs: Long, callback: LocationCallback) {
        if (fail) throw SecurityException("権限が無い")
        startedWith = intervalMs
    }
    override fun stop(callback: LocationCallback) {
        stopped = true
    }
}

/** 刻みの偽物。周期を覚える。 */
class FakeScheduler : FlushScheduler {
    var periodMs: Long? = null
    var cancelled = false

    /**
     * **task を捨てない**（ST02 の review/code.md の R40 / F5）。
     * 捨てていたときは `scheduler.every(HEARTBEAT_INTERVAL_MS) { emitter.emit() }` の
     * **中身を空のラムダに書き換えても全部緑**だった —— 周期側の emit を誰も確かめていない。
     */
    private var task: (() -> Unit)? = null

    override fun every(periodMs: Long, task: () -> Unit) {
        this.periodMs = periodMs
        this.task = task
    }

    /** 刻みが 1 回来たことにする。 */
    fun fire() = task?.invoke() ?: error("刻みに task が渡っていない")

    override fun cancel() {
        cancelled = true
    }
}

/**
 * 本番の `LocationService`。**差し替えるのは外との境目だけ**（取得元と刻み）。
 * 置き場・間隔・START_STICKY・権限拒否の扱いは本番のコードがそのまま動く。
 */
open class TestableLocationService(permissionDenied: Boolean = false) : LocationService() {
    val source = FakeFixSource(permissionDenied)
    val scheduler = FakeScheduler()
    val beatScheduler = FakeScheduler()

    /** 端末を読まずに固定する。**本番は `androidCapability` を呼ぶ**（そこは実機の確認）。 */
    var capability: Capability = Capability.of(permission = true, sensor = true, network = true)

    override fun newFixSource(): FixSource = source
    override fun newScheduler(): FlushScheduler = scheduler
    override fun newHeartbeatScheduler(): FlushScheduler = beatScheduler
    override fun readCapability(): Capability = capability

    /** 受け口ごとに送った本文を覚える偽物（ST04）。既定はすべて受け付ける。 */
    val posted = mutableListOf<Pair<String, String>>()

    /** 受け口ごとの答え（既定は全部受け付ける）。`(道, 件数) -> 答え` */
    var reply: (String, Int) -> Outcome = { _, n ->
        Outcome.Responded(200, (1..n).joinToString(",", "[", "]") { """{"accepted":true}""" })
    }
    override fun newTransport(path: String): Transport = Transport { body ->
        posted += path to body
        reply(path, body.split("\"id\":").size - 1)
    }

    /** 時計を試験から進める（ST04）。 */
    val clock = FakeDeviceClock()
    override fun newDeviceClock(): DeviceClock = clock
}

/** 権限を断られる端末。 */
class DeniedLocationService : TestableLocationService(permissionDenied = true)

/**
 * **本人が決めた値が、本番の配線に届いていること**（review R1）。
 *
 * 独立検証で、取得を 60 秒 → 1 秒・送信を 5 分 → 10 秒・永続 Outbox → メモリだけ、の
 * 3 つを同時に書き換えても**単体 52 件が全部緑のまま通った**。
 * `IntervalTest` は定数の値を固定しているだけで、その定数が使われているかは誰も見ていなかった。
 * 深掘り 第 2 回で本人へ戻した欠陥（未送信が無言で消える）も、部品の層でしか塞がれていなかった。
 *
 * ここは `LocationService` そのものをライフサイクルごと起こし、**外へ出ていく値**を観測する。
 */
@RunWith(RobolectricTestRunner::class)
class LocationServiceTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun start(): TestableLocationService =
        Robolectric.buildService(TestableLocationService::class.java, Intent())
            .create().startCommand(0, 1).get()

    @Test
    fun `取得の間隔は FIX_INTERVAL_MS がそのまま渡る`() {
        // **リテラルに書き換えられていないこと。** FR-1 が定めた 60 秒（design D7）
        val service = start()

        assertEquals(FIX_INTERVAL_MS, service.source.startedWith)
        assertEquals(60_000L, service.source.startedWith)
    }

    @Test
    fun `送信の周期は SEND_INTERVAL_MS がそのまま渡る`() {
        // 本人が深掘りで決めた 5 分（design D9）。
        // **設定が揃っていないと刻みは始まらない**ので、揃えてから見る
        Config.overrideForTest(baseUrl = "http://127.0.0.1:1", apiToken = "t", userId = "u")
        try {
            assertEquals(SEND_INTERVAL_MS, start().scheduler.periodMs)
            assertEquals(300_000L, SEND_INTERVAL_MS)
        } finally {
            Config.clearOverrideForTest()
        }
    }

    @Test
    fun `未送信の置き場は端末の保存領域で、立て直しをまたいで残る`() {
        // **深掘り 第 2 回で本人へ戻した欠陥。** メモリだけの置き場に戻せばここが落ちる
        val service = start()
        service.outboxForTest.add(fix("survivor"))

        // 「プロセスが立て直された」＝ 同じ filesDir から新しいサービスを起こす
        val reborn = start()

        assertEquals(listOf("survivor"), reborn.outboxForTest.snapshot().map { it.id })
    }

    @Test
    fun `置き場はアプリの保存領域に実在する`() {
        start().outboxForTest.add(fix("a"))

        val segs = File(app.filesDir, "outbox/records").listFiles { f -> f.name.endsWith(".jsonl") }.orEmpty()
        assertTrue("置き場がファイルとして残っていない", segs.isNotEmpty())
        assertTrue("中身が空", segs.any { it.readText().isNotBlank() })
    }

    @Test
    fun `落とされても OS に立て直させる`() {
        // **START_STICKY でなければ、未送信の永続化そのものが要らなくなる。**
        // ここが変わると「立て直しで消える」という前提が崩れ、D17 / D22 の根拠が無くなる
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent()).create()
        assertEquals(Service.START_STICKY, controller.get().onStartCommand(Intent(), 0, 1))
    }

    @Test
    fun `権限が無ければ落とさずに止まり、立て直しも求めない`() {
        // tasks 6.2 の Service 側。**落ちると次の起動まで収集が止まる**（成功条件 1 に直撃）
        val controller = Robolectric.buildService(DeniedLocationService::class.java, Intent()).create()

        val result = controller.get().onStartCommand(Intent(), 0, 1)

        assertEquals(Service.START_NOT_STICKY, result)
        assertTrue("権限が無いのに送信の刻みを始めている", controller.get().scheduler.periodMs == null)
    }

    @Test
    fun `設定が無いときは送らないが、取得は続ける`() {
        // 「捨てない」の根幹。設定してから送れるように、記録は未送信に積まれ続ける
        assertFalse("この試験は設定が空であることを前提にしている", Config.isComplete)

        val service = start()

        assertEquals("取得は始まっていなければならない", FIX_INTERVAL_MS, service.source.startedWith)
        assertTrue("設定が無いのに送信を始めている", service.scheduler.periodMs == null)
    }

    @Test
    fun `止めるときは取得も刻みも畳む`() {
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
            .create().startCommand(0, 1)
        val service = controller.get()

        controller.destroy()

        assertTrue("取得が止まっていない", service.source.stopped)
    }

    /**
     * ST01 / ST02 の 1 本の JSONL は、起動の時点で区切りの置き場へ取り込まれる（ST04 / tasks 5.2 の本番の配線）。
     */
    @Test
    fun `起動すると既存の outbox jsonl を取り込んで元を消す`() {
        File(app.filesDir, "outbox.jsonl").writeText(ingestJson.encodeToString(IngestRequest.serializer(), fix("legacy")) + "\n")
        val service = start()
        assertEquals(listOf("legacy"), service.outboxForTest.snapshot().map { it.id })
        assertFalse(File(app.filesDir, "outbox.jsonl").exists())
    }

    /**
     * 送信の契機 1 回で、記録・生存信号・破棄の報告がそれぞれの受け口へ送られる（ST04 / design D12 の本番の配線）。
     * 破棄の報告は `/drops`、上限の見回りは送る前に走る。
     */
    @Test
    fun `送信の契機で 90 日を超えた記録を捨て、破棄の報告を drops へ送る`() {
        Config.overrideForTest(baseUrl = "http://127.0.0.1:1", apiToken = "t", userId = "u")
        try {
            val service = start()
            service.outboxForTest.add(fix("old"))
            service.clock.advance(90 * AgeClock.DAY_MS + 60_000)
            service.scheduler.fire()

            assertEquals(0, service.outboxForTest.size())
            val drops = service.posted.filter { it.first == "/drops" }
            assertEquals("破棄の報告が /drops へ送られていない", 1, drops.size)
            assertTrue(drops.single().second.contains("\"reason\":\"age\""))
            assertFalse("90 日を超えた記録を送っている", service.posted.any { it.first == "/ingest" && it.second.contains("\"old\"") })
            assertTrue(service.posted.any { it.first == "/heartbeat" })
            assertEquals(0, service.dropsOutboxForTest.size())
        } finally {
            Config.clearOverrideForTest()
        }
    }

    @Test
    fun `設定が揃っていなくても積む契機で上限をかける`() {
        // 送れない理由を問わず上限はかかる（本人の決定 Q1）
        assertFalse(Config.isComplete)
        val service = start()
        service.outboxForTest.add(fix("old"))
        service.clock.advance(91 * AgeClock.DAY_MS)
        service.maintainForTest()
        assertEquals(0, service.outboxForTest.size())
        assertEquals(1, service.ledgerForTest.drafts().single().count)
    }

    private fun fix(id: String) =
        LocationFix(35.68, 139.76, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "u", "d", ZoneId.of("Asia/Tokyo"))

    /**
     * **生存信号の刻みが登録簿の想定間隔と同じであること**（tasks 7.1）。
     *
     * ここがずれると、受け手が「想定間隔を超えて何も来ない」と判定する窓とずれ、
     * **正常な運用が⑥「途絶」に見える**（FR-80）。定数を固定するだけでは、
     * その定数が本番の配線に届いているかを誰も見ていない（このクラスの存在理由）。
     */
    @Test
    fun `生存信号の刻みは HEARTBEAT_INTERVAL_MS がそのまま渡る`() {
        val service = start()
        assertEquals(HEARTBEAT_INTERVAL_MS, service.beatScheduler.periodMs)
    }

    /**
     * **起動のたびに 1 件積む。** 6 時間の刻みだけに任せると、OS が 5 時間ごとに
     * 立て直す端末では生存信号が 1 件も出ないまま「途絶」に見える。
     */
    @Test
    fun `起動した時点で生存信号が 1 件積まれる`() {
        val service = start()
        val beats = service.heartbeatOutboxForTest.snapshot()
        assertEquals(1, beats.size)
        assertEquals(LOGICAL_SOURCE, beats.single().logicalSource)
        assertTrue(beats.single().capturable)
    }

    /**
     * 権限が剥がれた端末では、**生きたまま「取れない」と報告する**（深掘り Q5）。
     * 稼働だけを送っていると、壊れているのに「動いていた」と残る。
     */
    @Test
    fun `権限が剥がれていると取れない状態が理由つきで積まれる`() {
        val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
        service.capability = Capability.of(permission = false, sensor = true, network = true)
        service.onStartCommand(Intent(), 0, 1)
        val beat = service.heartbeatOutboxForTest.snapshot().single()
        assertFalse(beat.capturable)
        assertEquals(listOf(Capability.PERMISSION), beat.blockers)
    }

    /** 生存信号の未送信は**記録とは別のファイル**（読み戻しで取り違えないため）。 */
    @Test
    fun `生存信号は記録とは別のファイルに積まれる`() {
        start()
        val beats = File(app.filesDir, "outbox/heartbeats").listFiles { f -> f.name.endsWith(".jsonl") }.orEmpty()
        assertTrue("生存信号の置き場が無い", beats.isNotEmpty())
        assertTrue("記録と同じ置き場に積まれている", File(app.filesDir, "outbox/records") != beats.first().parentFile)
    }

    /** 止めたら生存信号の刻みも止まる（残すと立て直しのたびに刻みが増える）。 */
    @Test
    fun `破棄すると生存信号の刻みも止まる`() {
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
            .create().startCommand(0, 1)
        val service = controller.get()
        controller.destroy()
        assertTrue(service.beatScheduler.cancelled)
    }

    /**
     * **刻みが来るたびに生存信号が 1 件積まれる**（ST02 の review/code.md の R40 / F5）。
     *
     * 起動時の 1 発は `startBeating()` が直に呼んでいるので、そこだけを見ていると
     * **周期側の emit が空でも緑**になる。spec の「想定間隔**ごとに**送られる生存信号」の要。
     */
    @Test
    fun `刻みが来るたびに生存信号が積まれる`() {
        val service = start()
        assertEquals("起動時の 1 件", 1, service.heartbeatOutboxForTest.size())
        service.beatScheduler.fire()
        service.beatScheduler.fire()
        assertEquals("周期側の emit が走っていない", 3, service.heartbeatOutboxForTest.size())
    }

    /**
     * 生存信号の刻みが**登録簿の想定間隔（6 時間）そのもの**であること
     * （ST02 の review/code.md の R42 / F8）。
     *
     * `assertEquals(HEARTBEAT_INTERVAL_MS, periodMs)` だけだと、**定数を 1 分にしても緑**。
     * サーバ側は `expected_gap_seeded` が `21_600` を固定しているので、
     * ここにリテラルを置けば 2 か所が縫い合わされる。ずらすと正常な運用が⑥「途絶」に見える。
     */
    @Test
    fun `生存信号の刻みは登録簿の想定間隔 6 時間と同じ`() {
        assertEquals(21_600_000L, HEARTBEAT_INTERVAL_MS)
        assertEquals(HEARTBEAT_INTERVAL_MS, start().beatScheduler.periodMs)
    }

    /**
     * 数えの置き場が**端末の保存領域**に配線されている（ST02 の review/code.md の R16）。
     *
     * インスタンスの中だけに持っていたときは `START_STICKY` の立て直しで `since` ごと
     * 新品になり、**死んでいた区間が観測から落ちた**。またいで残ることそのものは
     * `HeartbeatCountersTest` が確かめる。ここは**本番の配線**だけを見る。
     */
    @Test
    fun `数えの置き場が端末の保存領域にある`() {
        start()
        assertTrue(
            "数えがメモリだけに置かれている",
            File(app.filesDir, "heartbeat-counters.txt").exists(),
        )
    }

    // ------------------------------------------------------------------ ST04 の本番の配線（review R3）

    private fun withConfig(block: () -> Unit) {
        Config.overrideForTest(baseUrl = "http://127.0.0.1:1", apiToken = "t", userId = "u")
        try {
            block()
        } finally {
            Config.clearOverrideForTest()
        }
    }

    /**
     * `/drops` の送り手は**恒久的に断られても報告を取り除かない**（C2 / design D13）。
     * 本番の配線を `dropPermanentlyRejected = true` に変えると、断られた報告が端末から消える。
     *
     * Scenario: 断られた破棄の報告も未送信から取り除かれない
     */
    @Test
    fun `本番の配線で断られた破棄の報告も未送信に残る`() = withConfig {
        val service = start()
        service.reply = { path, n ->
            if (path == "/drops") {
                Outcome.Responded(400, (1..n).joinToString(",", "[", "]") { """{"accepted":false,"error":"malformed"}""" })
            } else {
                Outcome.Responded(200, (1..n).joinToString(",", "[", "]") { """{"accepted":true}""" })
            }
        }
        service.outboxForTest.add(fix("old"))
        service.clock.advance(90 * AgeClock.DAY_MS + 60_000)
        service.scheduler.fire()
        assertTrue("/drops に送っていない", service.posted.any { it.first == "/drops" })
        assertEquals("断られた破棄の報告が取り除かれた", 1, service.dropsOutboxForTest.size())
        service.scheduler.fire()
        assertEquals(2, service.posted.count { it.first == "/drops" })
        assertEquals(1, service.dropsOutboxForTest.size())
    }

    /**
     * 前のプロセスで書けないまま失われた記録（固定長の数えが 0 でない）は、起動すると「書けなかった」報告になり、数えは 0 に戻る。
     *
     * Scenario: 置き場に書けなかった記録も報告される
     */
    @Test
    fun `起動すると前のプロセスで書けなかった記録が報告になる`() {
        val counter = WriteFailedLedger(File(app.filesDir, "outbox/write-failed.bin")) {}
        counter.failed(Instant.parse("2026-09-08T02:10:00Z"))
        counter.failed(Instant.parse("2026-09-08T02:20:00Z"))
        val service = start()
        val d = service.ledgerForTest.drafts().single()
        assertEquals("write_failed", d.reason)
        assertEquals(2, d.count)
        assertTrue(WriteFailedLedger(File(app.filesDir, "outbox/write-failed.bin")) {}.peek().first.isEmpty())
    }

    /**
     * 置き場の読めない行は、見回りで「読めなかった」報告に数えられる。**数えは退避先の行数から取る**ので、
     * 見回りの前に立て直しても件数は消えない（review R21）。
     *
     * Scenario: 読めない行の件数が報告される
     */
    @Test
    fun `読めない行は見回りで報告に数えられ、立て直しても二重に数えない`() {
        val first = start()
        first.outboxForTest.add(fix("a"))
        val seg = File(app.filesDir, "outbox/records").listFiles { f -> f.name.endsWith(".jsonl") }!!.single()
        seg.appendText("壊れ1\n壊れ2\n")
        first.outboxForTest.snapshot()   // 読み戻しで退避される（この時点では報告に足していない）
        // 見回りの前に立て直された
        val reborn = start()
        reborn.maintainForTest()
        val d = reborn.ledgerForTest.drafts().single { it.reason == "unreadable" }
        assertEquals(2, d.count)
        reborn.maintainForTest()
        assertEquals("同じ行を二重に数えた", 2, start().ledgerForTest.drafts().single { it.reason == "unreadable" }.count)
    }

    /**
     * 本番の配線で、常駐の通知に未送信の日数が出る（本人の決定 Q5）。
     *
     * Scenario: 常駐の通知に未送信の日数が出る
     */
    @Test
    fun `本番の配線で常駐の通知に未送信の日数が出る`() {
        val service = start()
        service.outboxForTest.add(fix("a"))
        service.clock.advance(12 * AgeClock.DAY_MS + 1_000)
        service.maintainForTest()
        val nm = app.getSystemService(android.app.NotificationManager::class.java)
        val text = org.robolectric.Shadows.shadowOf(nm).getNotification(LocationService.NOTIFICATION_ID)
            ?.extras?.getCharSequence(android.app.Notification.EXTRA_TEXT)?.toString()
        assertEquals("位置を記録しています · 未送信 12 日", text)
    }

    /**
     * **設定が揃っていない端末でも、積む契機から上限がかかる**（本人の決定 Q1 / review 試験 C2）。
     * 送信の刻みが立たないので、見回りは積む契機（1 分に 1 度まで）だけが持つ。間引かない口は使わない。
     *
     * Scenario: 到達できても断られ続ける未送信にも上限がかかる
     */
    @Test
    fun `設定が無い端末でも積む契機の見回りで 90 日を超えた記録を捨て、生存信号は捨てない`() {
        assertFalse(Config.isComplete)
        val service = start()
        service.outboxForTest.add(fix("old"))
        val beatsBefore = service.heartbeatOutboxForTest.size()
        service.clock.advance(91 * AgeClock.DAY_MS)
        // 間引きの内側（前の見回りから 1 分未満）では捨てない
        service.outboxForTest.add(fix("new-1"))
        assertTrue("間引かずに毎回見回っている", service.outboxForTest.snapshot().any { it.id == "old" })
        org.robolectric.shadows.ShadowSystemClock.advanceBy(java.time.Duration.ofSeconds(61))
        service.outboxForTest.add(fix("new-2"))
        assertEquals(listOf("new-1", "new-2"), service.outboxForTest.snapshot().map { it.id })
        assertEquals(1, service.ledgerForTest.drafts().single().count)
        assertEquals("生存信号を捨てた", beatsBefore, service.heartbeatOutboxForTest.size())
    }
}
