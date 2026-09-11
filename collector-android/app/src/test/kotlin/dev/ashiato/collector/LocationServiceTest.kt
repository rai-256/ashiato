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
    override fun every(periodMs: Long, task: () -> Unit) {
        this.periodMs = periodMs
    }
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

        val f = File(app.filesDir, "outbox.jsonl")
        assertTrue("置き場がファイルとして残っていない", f.exists())
        assertTrue("中身が空", f.readText().isNotBlank())
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
        assertTrue(File(app.filesDir, "heartbeat.jsonl").exists())
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
}
