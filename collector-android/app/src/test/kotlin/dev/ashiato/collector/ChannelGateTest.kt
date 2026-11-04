// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.NotificationManager
import android.util.Log
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.shadows.ShadowLog
import org.robolectric.shadows.ShadowNotificationManager
import java.time.Instant

@RunWith(RobolectricTestRunner::class)
class ChannelGateTest {
    @get:Rule val tmp = TemporaryFolder()
    private val app get() = ApplicationProvider.getApplicationContext<Application>()
    private val manager get() = app.getSystemService(NotificationManager::class.java)

    private fun req(id: String) = testFix(35.68, 139.76, 10f, Instant.parse("2026-10-01T00:00:00Z"))
        .toIngestRequest(id, "user-1", "device-1", java.time.ZoneId.of("Asia/Tokyo"))

    private fun process(local: Channel, remote: Channel): TestProcess {
        val stores = TestStores(tmp.newFolder("${local.name}-${remote.name}"))
        val original = OriginalHarness(tmp.newFolder("original-${local.name}-${remote.name}"), stores)
        original.photo(1, "photo-one")
        original.scan()
        original.acceptRecords()
        val entry = original.env.originals.all().single()
        original.env.listing.remove(PRIMARY, 1)
        original.reporter.report(entry, OriginalMissingReason.VANISHED)
        stores.beats.add(
            HeartbeatRequest("hb", "user-1", LOGICAL_SOURCE, "device-1", "2026-10-01T00:00:00Z", true, emptyList(), 1, 1, "{}"),
        )
        stores.drops.add(
            DropReport("dr", "user-1", LOGICAL_SOURCE, "device-1", "age", "2026-10-01T00:00:00Z", null, null, 1, emptyList(), "{}"),
        )
        val store = ContractWaitStore(app.getSharedPreferences("channel-${local.name}-${remote.name}-${System.nanoTime()}", 0))
        val gate = ContractGate(store, { Log.w("ChannelGateTest", it) }, local)
        val calls = mutableListOf<String>()
        val transport = Transport { body ->
            calls += body
            val count = body.split("\"id\":").size - 1
            Outcome.Responded(200, (1..count).joinToString(",", "[", "]") { "{\"accepted\":true}" })
        }
        var remoteChannel = remote
        val response = { Outcome.Responded(200, "{\"contract_version\":1}", headers = mapOf("Ashiato-Channel" to channelName(remoteChannel))) }
        val sender = Sender(stores.records, transport, IngestRequest.serializer())
        // 本物（LocationService）と同じく、契機の終わりに系統の違いを通知の係へ渡す（code-verify R25）
        val notifier = ChannelMismatchNotifier(app, store, now = { 1_000L }, local = local)
        val drainer = Drainer(
            records = sender,
            recordsOutbox = stores.records,
            beats = Sender(stores.beats, transport, HeartbeatRequest.serializer()),
            drops = Sender(stores.drops, transport, DropReport.serializer()),
            ledger = stores.ledger,
            missing = Sender(original.missing, transport, MediaMissingReport.serializer()),
            afterRecords = { answer -> original.uploader.run(answer) },
            contract = ContractGuard(response, gate),
            onChannelMismatch = { mismatch -> notifier.update(mismatch) },
        )
        return TestProcess(stores, original, sender, drainer, gate, store, calls) { remoteChannel = it }
    }

    private fun channelName(channel: Channel) = if (channel == Channel.Dev) "dev" else "official"
    private data class TestProcess(
        val stores: TestStores, val original: OriginalHarness, val sender: Sender<IngestRequest>, val drainer: Drainer,
        val gate: ContractGate, val store: ContractWaitStore, val calls: MutableList<String>,
        val setRemote: (Channel) -> Unit,
    )

    // Scenario: 開発版の携帯の収集アプリは、正式版のサーバに何も送らず手元に残す
    @Test fun `dev does not post to official and keeps outbox`() {
        val p = process(Channel.Dev, Channel.Official); p.stores.records.add(req("dev-record")); p.drainer.tick()
        assertEquals(1, p.stores.records.size()); assertEquals(1, p.stores.beats.size())
        assertEquals(1, p.stores.drops.size()); assertEquals(1, p.original.missing.size())
        assertEquals(1, p.original.env.originals.ready().size)
        assertTrue(p.gate.channelMismatch); assertTrue(p.calls.isEmpty()); assertTrue(p.original.transport.puts.isEmpty())
    }

    // Scenario: 正式版の携帯の収集アプリは、開発版のサーバに何も送らず手元に残す
    @Test fun `official does not post to dev and keeps outbox`() {
        val p = process(Channel.Official, Channel.Dev); p.stores.records.add(req("official-record")); p.drainer.tick()
        assertEquals(1, p.stores.records.size()); assertTrue(p.calls.isEmpty())
    }

    // Scenario: 開発版の携帯の収集アプリは、開発版のサーバには今どおり送る
    @Test fun `matching channel posts and removes outbox`() {
        val p = process(Channel.Dev, Channel.Dev); p.stores.records.add(req("matching")); p.drainer.tick()
        assertEquals(0, p.stores.records.size()); assertTrue(p.calls.isNotEmpty()); assertFalse(p.gate.channelMismatch)
    }

    // Scenario: 系統が違って送らない間、携帯の収集アプリは画面の行と通知で送り先を確かめるよう知らせる
    @Test fun `mismatch has persisted line and separate notification`() {
        ShadowLog.clear()
        val p = process(Channel.Dev, Channel.Official); p.stores.records.add(req("notice")); p.drainer.tick()
        // 通知は送る契機（drainer.tick）だけで出る。試験は通知の係を自分で呼ばない（code-verify R25）
        assertEquals("送り先が正式版のサーバです。送り先を確かめてください", contractHeadlineFor(p.gate, p.store))
        assertNull(p.store.notice)
        val shadowManager: ShadowNotificationManager = shadowOf(manager)
        assertNotNull(shadowManager.getNotification(ChannelMismatchNotifier.CHANNEL_NOTIFICATION_ID))
        assertNull(shadowManager.getNotification(ContractNotifier.CONTRACT_NOTIFICATION_ID))
        assertTrue(ShadowLog.getLogs().any { it.msg.contains("kind=channel_mismatch") && it.msg.contains("trigger=dev") && it.msg.contains("availability=official") })
    }

    // Scenario: 送り先を直すと、携帯の収集アプリは手元に残した記録を送り、系統の違いの知らせを消す
    @Test fun `matching answer sends retained record and clears mismatch`() {
        val p = process(Channel.Dev, Channel.Official); p.stores.records.add(req("repair")); p.drainer.tick()
        assertNotNull(shadowOf(manager).getNotification(ChannelMismatchNotifier.CHANNEL_NOTIFICATION_ID))
        p.setRemote(Channel.Dev); p.drainer.tick()
        assertEquals(0, p.stores.records.size()); assertNull(shadowOf(manager).getNotification(ChannelMismatchNotifier.CHANNEL_NOTIFICATION_ID))
    }

    // Scenario: 系統の違いの知らせは、携帯でも続く間 24 時間ごとに出し直す
    @Test fun `mismatch notification repeats after 24 hours`() {
        val p = process(Channel.Dev, Channel.Official); var now = 1_000L
        val notifier = ChannelMismatchNotifier(app, p.store, now = { now }); notifier.update(true)
        val first = p.store.channelNotifiedAtMs; now += ContractNotifier.REPEAT.toMillis() - 1; notifier.update(true)
        assertEquals(first, p.store.channelNotifiedAtMs); now++; notifier.update(true); assertEquals(now, p.store.channelNotifiedAtMs)
    }

    // Scenario: 契約の問い合わせが 404 なら、携帯の収集アプリは系統の違いの知らせでなく更新を促す
    @Test fun `404 is contract update wait not mismatch`() {
        assertEquals(ContractAnswer.NoEndpoint, readContractAnswer(Outcome.Responded(404, ""), Channel.Dev))
        assertTrue(ContractNotice(false, 1).line.contains(CONTRACT_UPDATE_HEADLINE))
    }

    // Scenario: 契約の問い合わせが読めない 200 なら、携帯の収集アプリは系統の違いの知らせを出さない
    @Test fun `unreadable response does not create mismatch`() {
        val answer = readContractAnswer(Outcome.Responded(200, "<html>login</html>"), Channel.Dev)
        assertEquals(ContractAnswer.Unreadable, answer); assertFalse(answer is ContractAnswer.ChannelMismatch)
    }

    // Scenario: 携帯の上限の 7 日前の知らせに、系統の違いの案内を添える
    @Test fun `retention headline includes both messages`() {
        val line = ContractNotice(false, 83, channelMismatch = true).line
        assertTrue(line.contains(CONTRACT_UPDATE_HEADLINE)); assertTrue(line.contains("送り先を確かめてください"))
    }

    // Scenario: 系統の違うサーバからも、携帯の収集アプリは途絶の判定を今どおり引く
    @Test fun `outage polling remains independent of channel gate`() {
        var calls = 0
        val executor = java.util.concurrent.Executors.newSingleThreadExecutor()
        val poller = OutagePoller(
            query = OutageQuery { calls++; Outcome.Responded(200, "{\"status\":\"ready\",\"outages\":[]}") },
            notice = OutageNotice(app.getSharedPreferences("outage-channel-${System.nanoTime()}", 0)),
            notifier = OutageNotifier(app), deviceId = { "device-1" }, executor = executor,
        )
        poller.poll(); executor.shutdown(); executor.awaitTermination(5, java.util.concurrent.TimeUnit.SECONDS); assertEquals(1, calls)
    }

    /** 系統の違いが続く間、`channel_mismatch` のログは状態に入ったときに 1 回だけ（/code-review R31）。 */
    @Test fun `mismatch log is written once while it lasts`() {
        ShadowLog.clear()
        val p = process(Channel.Dev, Channel.Official); p.stores.records.add(req("log")); p.drainer.tick(); p.drainer.tick()
        val count = { ShadowLog.getLogs().count { it.msg.contains("kind=channel_mismatch") } }
        assertEquals(1, count())
        // 直って、また違えば、もう 1 回出す
        p.setRemote(Channel.Dev); p.drainer.tick(); p.setRemote(Channel.Official); p.drainer.tick()
        assertEquals(2, count())
    }

    /** 更新の待ちと系統の違いが重なれば、画面の行は両方の文を載せる（/code-review R32）。 */
    @Test fun `screen line shows mismatch while contract waiting`() {
        // 画面（MainActivity）は組み立ての系統で文の相手を決める
        val mismatchLine = channelMismatchHeadline(buildLocalChannel())
        val p = process(Channel.Dev, Channel.Official)
        p.store.notice = ContractNotice(false, 3)
        p.store.channelMismatch = true
        val line = screenNotice(p.store)!!.line
        assertTrue(line, line.contains(CONTRACT_UPDATE_HEADLINE)); assertTrue(line, line.contains(mismatchLine))
        assertTrue(line, line.contains("未送信 3 日分"))
        p.store.channelMismatch = false
        assertEquals(ContractNotice(false, 3).line, screenNotice(p.store)!!.line)
        p.store.notice = null
        assertNull(screenNotice(p.store))
        p.store.channelMismatch = true
        assertEquals(mismatchLine, screenNotice(p.store)!!.line)
    }

    /** 系統の違いだけの画面の行は、契約の版の待ちの日数を添えない（final review R7）。 */
    @Test fun `channel only line has no unsent days`() {
        val line = ContractNotice(false, 0, channelMismatch = true, contractWaiting = false, local = Channel.Dev).line
        assertEquals("送り先が正式版のサーバです。送り先を確かめてください", line)
    }

    /** 自分の系統は部品へ渡した値を使い、組み立ての系統を直に読まない（final review R15）。 */
    @Test fun `local channel is injected into notice and notifier`() {
        assertTrue(ContractNotice(false, 0, channelMismatch = true, contractWaiting = false, local = Channel.Official).line.contains("開発版のサーバ"))
        val p = process(Channel.Dev, Channel.Official)
        ChannelMismatchNotifier(app, p.store, now = { 1_000L }, local = Channel.Dev).update(true)
        val shown = shadowOf(shadowOf(manager).getNotification(ChannelMismatchNotifier.CHANNEL_NOTIFICATION_ID))
        assertEquals("送り先が正式版のサーバです。送り先を確かめてください", shown.contentText.toString())
        // 再起動の後（理由がまだメモリに無い）も、写しの文の相手は gate の系統で決める
        p.store.channelMismatch = true
        val restarted = ContractGate(p.store, {}, Channel.Dev)
        assertEquals("送り先が正式版のサーバです。送り先を確かめてください", contractHeadlineFor(restarted, p.store))
    }
}
