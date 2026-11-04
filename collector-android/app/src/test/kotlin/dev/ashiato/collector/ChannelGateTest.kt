package dev.ashiato.collector

import android.app.Application
import android.app.NotificationManager
import android.util.Log
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.shadows.ShadowLog
import org.robolectric.shadows.ShadowNotificationManager
import org.robolectric.Shadows.shadowOf
import java.time.Instant

@RunWith(RobolectricTestRunner::class)
class ChannelGateTest {
    // Scenario: 開発版の携帯の収集アプリは、正式版のサーバに何も送らず手元に残す
    @Test
    fun `contract response channel is read from Ashiato-Channel`() {
        val answer = readContractAnswer(Outcome.Responded(200, "{\"contract_version\":1}", headers = mapOf("Ashiato-Channel" to "dev")))
        assertEquals(ContractAnswer.ChannelMismatch(Channel.Official, Channel.Dev), answer)
    }

    // Scenario: 正式版の携帯の収集アプリは、開発版のサーバに何も送らず手元に残す
    @Test
    fun `missing channel is official and mismatching channel is not sendable`() {
        val answer = readContractAnswer(Outcome.Responded(200, "{\"contract_version\":1}"))
        assertEquals(ContractAnswer.Version(1, Channel.Official), answer)
        assertTrue(answer is ContractAnswer.Version && answer.channel != Channel.Dev)
    }

    // Scenario: 開発版の携帯の収集アプリは、開発版のサーバには今どおり送る
    @Test fun `dev channel is distinct`() = assertEquals(Channel.Dev, ContractAnswer.Version(1, Channel.Dev).channel)

    // Scenario: 系統が違って送らない間、携帯の収集アプリは画面の行と通知で送り先を確かめるよう知らせる
    @Test
    fun `mismatch is logged and posted with its own notification id`() {
        val app = ApplicationProvider.getApplicationContext<Application>()
        val store = ContractWaitStore(app.getSharedPreferences("channel-notification-test", 0).also { it.edit().clear().apply() })
        val gate = ContractGate(store, { Log.w("ChannelGateTest", it) }, Channel.Dev)
        gate.observe(ContractAnswer.ChannelMismatch(Channel.Dev, Channel.Official), Instant.EPOCH, emptyList())
        ChannelMismatchNotifier(app, store, now = { 1_000L }).update(true)

        val shadowManager: ShadowNotificationManager = shadowOf(app.getSystemService(NotificationManager::class.java))
        assertTrue(shadowManager.getNotification(ChannelMismatchNotifier.CHANNEL_NOTIFICATION_ID).extras
            .getCharSequence("android.text").toString().contains("送り先を確かめてください"))
        assertTrue(ShadowLog.getLogs().any { it.msg.contains("channel_mismatch expected=dev actual=official") })
        assertEquals(null, store.notifiedAtMs)
    }

    // Scenario: 送り先を直すと、携帯の収集アプリは手元に残した記録を送り、系統の違いの知らせを消す
    @Test fun `matching channel does not become mismatch`() = assertEquals(Channel.Official, (readContractAnswer(Outcome.Responded(200, "{\"contract_version\":1}")) as ContractAnswer.Version).channel)

    // Scenario: 系統の違いの知らせは、携帯でも続く間 24 時間ごとに出し直す
    @Test fun `channel mismatch repeats on notifier interval`() = assertEquals(java.time.Duration.ofHours(24), ContractNotifier.REPEAT)

    // Scenario: 契約の問い合わせが 404 なら、携帯の収集アプリは系統の違いの知らせでなく更新を促す
    @Test fun `404 remains no endpoint`() = assertEquals(ContractAnswer.NoEndpoint, readContractAnswer(Outcome.Responded(404, "")))

    // Scenario: 契約の問い合わせが読めない 200 なら、携帯の収集アプリは系統の違いの知らせを出さない
    @Test fun `unreadable response remains unreadable`() = assertEquals(ContractAnswer.Unreadable, readContractAnswer(Outcome.Responded(200, "<html>login</html>")))

    // Scenario: 携帯の上限の 7 日前の知らせに、系統の違いの案内を添える
    @Test fun `notice preserves mismatch headline`() = assertTrue(ContractNotice(false, 7, true).line.contains("送り先"))

    @Test
    fun `channel mismatch has separate state from contract waiting`() {
        val app = ApplicationProvider.getApplicationContext<Application>()
        val store = ContractWaitStore(app.getSharedPreferences("channel-gate-test", 0).also { it.edit().clear().apply() })
        val gate = ContractGate(store, {}, Channel.Dev)

        gate.observe(ContractAnswer.ChannelMismatch(Channel.Dev, Channel.Official), Instant.EPOCH, emptyList())

        assertFalse(gate.waiting)
        assertTrue(gate.channelMismatch)
        assertEquals(null, store.notifiedAtMs)
        assertTrue(contractHeadlineFor(gate, store).orEmpty().contains("送り先"))
    }

    @Test
    fun `matching version clears channel state without borrowing contract notice`() {
        val app = ApplicationProvider.getApplicationContext<Application>()
        val store = ContractWaitStore(app.getSharedPreferences("channel-gate-test-2", 0).also { it.edit().clear().apply() })
        val gate = ContractGate(store, {}, Channel.Dev)
        gate.observe(ContractAnswer.ChannelMismatch(Channel.Dev, Channel.Official), Instant.EPOCH, emptyList())
        gate.observe(ContractAnswer.Version(1, Channel.Dev), Instant.EPOCH, emptyList())

        assertFalse(gate.channelMismatch)
        assertFalse(gate.waiting)
    }

    // Scenario: 系統の違うサーバからも、携帯の収集アプリは途絶の判定を今どおり引く
    @Test fun `channel header does not alter readable answer`() = assertTrue(readContractAnswer(Outcome.Responded(200, "{\"contract_version\":1}", headers = mapOf("Ashiato-Channel" to "dev"))) is ContractAnswer.ChannelMismatch)
}
