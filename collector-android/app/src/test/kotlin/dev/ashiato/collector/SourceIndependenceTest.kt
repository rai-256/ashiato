// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.android.controller.ServiceController
import org.robolectric.shadows.ShadowLog

/**
 * **ソースごとに独立して収集する**（ST06 / tasks 5.1 / 本人の決定 Q7 / design D5）。
 *
 * ST06 より前の収集は「位置の取得条件が欠けたら全部止まる」だった ——
 * 止まっている間は**生存信号も出ない**ので、受け手の画面には③「動いていたが取れない状態」ではなく
 * ⑥「途絶」が出て、アプリ利用はその間 1 件も取れなかった（10 日を越えれば二度と取れない）。
 *
 * ここは `LocationService` をライフサイクルごと起こし、**外へ出ていくもの**
 * （未送信に積まれた記録・生存信号・前景の通知）だけを観測する。
 * 差し替えるのは外との境目（位置の取得元・アプリ利用の取得元・刻み・取得可否）だけ。
 */
@RunWith(RobolectricTestRunner::class)
class SourceIndependenceTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private val location = SourceCadence.LOCATION.logicalSource
    private val usage = SourceCadence.APP_USAGE.logicalSource
    private val rollup = SourceCadence.APP_USAGE_ROLLUP.logicalSource

    /** 取得元に溜まっているイベント（取得契機の窓に入る時刻に置く）。 */
    private fun storedEvent(agoMs: Long = 60_000) = UsageEventSnapshot(
        packageName = "dev.ashiato.example",
        className = "dev.ashiato.example.MainActivity",
        eventType = 1,
        at = Instant.now().minusMillis(agoMs),
    )

    /**
     * 取得条件を決めてから収集を始める。**`create()` の前に決める** ——
     * 取得元も取得可否も `onCreate` で読まれるので、後から入れても初回の契機に効かない。
     */
    private fun start(
        locationOk: Boolean = true,
        usageOk: Boolean = true,
        events: List<UsageEventSnapshot> = listOf(storedEvent()),
    ): ServiceController<TestableLocationService> {
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val service = controller.get()
        service.usage = FakeUsageSource(storedEvents = events)
        service.capabilities[location] = Capability.of(permission = locationOk, sensor = true, network = true)
        val usageCapability = Capability.of(permission = usageOk, sensor = true, network = true)
        service.capabilities[usage] = usageCapability
        service.capabilities[rollup] = usageCapability
        return controller.create().startCommand(0, 1)
    }

    private fun TestableLocationService.records(logicalSource: String) =
        outboxForTest.snapshot().filter { it.logicalSource == logicalSource }

    private fun TestableLocationService.beat(logicalSource: String) =
        heartbeatOutboxForTest.snapshot().single { it.logicalSource == logicalSource }

    /** 送信の本番配線を立てるために必要な、試験用の接続設定。 */
    private fun configured(block: () -> Unit) {
        Config.overrideForTest(baseUrl = "http://127.0.0.1:1", apiToken = "t", userId = "u")
        try {
            block()
        } finally {
            Config.clearOverrideForTest()
        }
    }

    // Scenario: 位置の取得条件が欠けてもアプリ利用は集まる
    @Test
    fun `位置の権限が無くてもアプリ利用の記録が未送信に積まれる`() {
        val service = start(locationOk = false).get()

        assertTrue(
            "位置が欠けただけでアプリ利用が 1 件も積まれていない",
            service.records(usage).isNotEmpty(),
        )
        // 位置は取りに行っていない（取得元へ登録していない）
        assertEquals("取得条件が欠けたソースの collect を呼んでいる", 0, service.source.starts)
    }

    // Scenario: 位置の取得条件が欠けても位置の生存信号は届く
    @Test
    fun `位置の権限が無くても位置の生存信号が理由つきで送られる`() = configured {
        val service = start(locationOk = false).get()

        val beat = service.beat(location)
        assertFalse("取得できないのに取れていることになっている", beat.capturable)
        assertEquals("何が満たされていないかを示していない", listOf(Capability.PERMISSION), beat.blockers)

        // 本番と同じ 5 分ごとの送信契機を通し、未送信より先の `/heartbeat` まで見る。
        service.scheduler.fire()
        assertTrue(
            "位置の生存信号を /heartbeat へ送っていない",
            service.posted.any { (path, body) -> path == "/heartbeat" && body.contains("\"logical_source\":\"$location\"") },
        )
    }

    // Scenario: アプリ利用の取得条件が欠けても位置は集まる
    // Scenario: 許可しなくても収集は始まる
    //
    // 2 つめの印をここに置くのは、その Scenario の THEN（位置の記録が生成される）と
    // AND（アプリ利用の生存信号が取得できない状態と権限を示す）を**両方**見ているのがここだけだから
    // （独立レビュー I3）。設定画面から許可せずに戻った端末の状態＝アプリ利用の取得条件が欠けた状態。
    @Test
    fun `利用状況へのアクセスが無くても位置の記録が未送信に積まれる`() {
        val controller = start(usageOk = false)
        val service = controller.get()

        // 取得元（Play Services）が契機を 1 回配る（本番と同じ経路）
        service.source.deliver()

        assertEquals("位置の取得間隔が渡っていない", FIX_INTERVAL_MS, service.source.startedWith)
        assertTrue("アプリ利用が欠けただけで位置が積まれていない", service.records(location).isNotEmpty())
        assertTrue("取得条件が欠けたソースの記録が積まれている", service.records(usage).isEmpty())
        val beat = service.beat(usage)
        assertFalse(beat.capturable)
        assertEquals(listOf(Capability.PERMISSION), beat.blockers)
    }

    // Scenario: どのソースも取得できなくても収集は始まり信号は届く
    @Test
    fun `どのソースも取得できなくても収集は落ちずに始まり、全ソースの生存信号が送られる`() = configured {
        val controller = start(locationOk = false, usageOk = false)
        val service = controller.get()

        // **落とされても OS に立て直させる**（止めない）
        assertEquals(Service.START_STICKY, service.onStartCommand(Intent(), 0, 2))
        // 前景サービスは立っている（通知が出ている）
        val manager = app.getSystemService(NotificationManager::class.java)
        assertTrue(
            "取得条件が 1 つも無いと前景サービスが立たない",
            shadowOf(manager).getNotification(LocationService.NOTIFICATION_ID) != null,
        )
        for (source in SourceCadence.entries) {
            val beat = service.beat(source.logicalSource)
            assertFalse("${source.logicalSource} が取れていることになっている", beat.capturable)
            assertEquals(
                "${source.logicalSource} が何の理由も示していない",
                listOf(Capability.PERMISSION),
                beat.blockers,
            )
        }

        // Outbox への蓄積だけで終わらせず、Drainer と Sender を経て実際の送信先まで通す。
        service.scheduler.fire()
        val sent = service.posted.filter { it.first == "/heartbeat" }.joinToString("\n") { it.second }
        for (source in SourceCadence.entries) {
            assertTrue(
                "${source.logicalSource} の生存信号を /heartbeat へ送っていない",
                sent.contains("\"logical_source\":\"${source.logicalSource}\""),
            )
        }
    }

    // Scenario: 後から許可すると次の契機から集まる
    @Test
    fun `許可しないまま動いている収集は、許可された次の契機から集まる`() {
        val controller = start(usageOk = false)
        val service = controller.get()
        assertTrue("取得条件が欠けているのに積まれている", service.records(usage).isEmpty())

        // 本人が設定画面で許した（**アプリを開き直していない**）
        service.capabilities[usage] = Capability.of(permission = true, sensor = true, network = true)
        // 次の取得契機（30 分ごと）
        service.sourceSchedulers.getValue(usage).fire()

        assertTrue("後から許可したのに次の契機で集まっていない", service.records(usage).isNotEmpty())
    }

    /**
     * 取得の刻みは**ソースごとの間隔**（design D5）。1 本にまとめると、
     * 30 分のアプリ利用が 60 秒ごとに動く（取得元への問い合わせが 30 倍になる）。
     */
    @Test
    fun `取得の刻みはソースごとの取得間隔`() {
        val service = start().get()

        assertEquals(60_000L, service.sourceSchedulers.getValue(location).periodMs)
        assertEquals(30 * 60 * 1000L, service.sourceSchedulers.getValue(usage).periodMs)
        assertEquals(6 * 60 * 60 * 1000L, service.sourceSchedulers.getValue(rollup).periodMs)
    }

    /**
     * 取得元が契機を自分で配るソース（位置）は、**刻みでは何もしない** ——
     * 60 秒ごとに `requestLocationUpdates` を張り直すのは本番の振る舞いではない
     * （張り直すのは `onStartCommand` だけ。ST06 より前からの復旧の経路）。
     */
    @Test
    fun `契機を自分で配るソースは刻みで登録し直さない`() {
        val service = start().get()
        assertEquals(1, service.source.starts)

        service.sourceSchedulers.getValue(location).fire()

        assertEquals("刻みが取得元へ登録し直している", 1, service.source.starts)
    }

    /**
     * **「取れない」を毎回の契機でログに出さない**（独立レビュー M1）。
     *
     * 位置の契機は 60 秒ごとなので、権限を拒んだままの端末では 1 日 1440 行になる ——
     * `logcat` は環状の置き場なので、それだけで**他の行が押し出される**
     * （計測テストも `logcat -t 400` で読んでいて、その前提に直接効く）。
     * **理由が変わったときは出す**（取れる状態に戻って、また取れなくなった、を見逃さない）。
     */
    @Test
    fun `取れない理由が同じあいだはログに 1 度だけ出す`() {
        ShadowLog.clear()
        val service = start(usageOk = false).get()
        // **名前の前方一致で数えない** —— `c01-app-usage` は `c01-app-usage-rollup` の頭でもある
        val mine = Regex("source=" + Regex.escape(usage) + "(\\s|$)")
        fun lines() = ShadowLog.getLogsForTag(LocationService.TAG)
            .count { it.msg.contains("kind=source_unavailable") && mine.containsMatchIn(it.msg) }

        assertEquals("起動の契機で 1 行", 1, lines())

        // 取れないまま契機が 2 回来ても増えない
        service.sourceSchedulers.getValue(usage).fire()
        service.sourceSchedulers.getValue(usage).fire()
        assertEquals("同じ理由を毎回出している", 1, lines())

        // 取れる状態に戻り（ここでは何も出ない）、また取れなくなったら出す
        service.capabilities[usage] = Capability.of(permission = true, sensor = true, network = true)
        service.sourceSchedulers.getValue(usage).fire()
        assertEquals("取れるようになった契機で出している", 1, lines())
        service.capabilities[usage] = Capability.of(permission = false, sensor = true, network = true)
        service.sourceSchedulers.getValue(usage).fire()
        assertEquals("理由が立ち直ったのに出していない", 2, lines())
    }

    /**
     * **成功は「取得できた回数」**（本人の決定 C7）。0 件でも成功、
     * **置き場に書けた件数ではない**（書けなかった分は破棄の報告が持つ）。
     */
    @Test
    fun `アプリ利用は 0 件の契機も成功に数える`() {
        val service = start(events = emptyList()).get()

        // 起動時の 1 件（試行も成功も 0 で始まる）を捨ててから、契機を 1 回回す
        service.sourceSchedulers.getValue(usage).fire()
        service.beatSchedulers.getValue(usage).fire()

        val beat = service.heartbeatOutboxForTest.snapshot().last { it.logicalSource == usage }
        assertTrue("0 件だった契機が成功に数えられていない", beat.successes >= 1)
        assertTrue("成功が試行を超えている", beat.successes <= beat.attempts)
    }

    /**
     * **読めなかった契機は成功に数えない**（0 件と取り違えると、その期間は取り直されない）。
     */
    @Test
    fun `取得元が読めなかった契機は成功に数えない`() {
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val service = controller.get()
        service.usage = FakeUsageSource(storedEvents = listOf(storedEvent()), unreadable = "locked")
        controller.create().startCommand(0, 1)

        // **起動時の信号の後に契機を 1 回回す** —— 起動の取得は最初の信号に畳まれてしまうので、
        // そこだけ見ていると「読めなかったを成功に数える」に壊しても緑のままになる（実測 2026-09-24）
        service.sourceSchedulers.getValue(usage).fire()
        service.beatSchedulers.getValue(usage).fire()

        val beat = service.heartbeatOutboxForTest.snapshot().last { it.logicalSource == usage }
        assertEquals("読めなかった契機が成功に数えられている", 0, beat.successes)
    }
}
