// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Application
import android.app.Notification
import android.app.NotificationManager
import android.content.Intent
import android.content.SharedPreferences
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.test.core.app.ApplicationProvider
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.io.File
import java.time.Instant
import java.time.ZoneId

/**
 * 携帯の「サーバを更新してください」—— 画面の行・通知・上限の 7 日前の知らせ（ST50 / D6・D22・D24（仮））。
 * `Drainer` に偽の問い合わせの口をつなぎ、Robolectric の本物の `NotificationManager` と `MainActivity` を通して見る。
 */
@RunWith(RobolectricTestRunner::class)
class St50ContractNoticeTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val manager: NotificationManager get() = app.getSystemService(NotificationManager::class.java)
    private val day = AgeClock.DAY_MS
    private val hour = 60 * 60 * 1000L
    private val st by lazy { TestStores(dir = tmp.newFolder("st50notice")) }
    private val start = Instant.parse("2026-10-01T00:00:00Z").toEpochMilli()
    private var nowMs = start

    private val readable = Outcome.Responded(200, """{"contract_version":1}""")
    private val html = Outcome.Responded(200, "<html>login</html>")
    private val notFound = Outcome.Responded(404, "")
    private val offline = Outcome.Unreachable("offline")
    private val unavailable = Outcome.Responded(503, "")

    private fun prefs(): SharedPreferences =
        app.getSharedPreferences(ContractWaitStore.PREFS, android.content.Context.MODE_PRIVATE)

    @Before
    fun permissions() {
        shadowOf(app).grantPermissions(Manifest.permission.POST_NOTIFICATIONS)
        shadowOf(app).grantPermissions(*mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).toTypedArray())
    }

    /** 1 つの収集アプリのプロセス。`restart()` で作り直しても `SharedPreferences` は残る。 */
    private inner class Process {
        var probe: Outcome = readable
        val store = ContractWaitStore(prefs())
        val gate = ContractGate(store)
        val notifier = ContractNotifier(app, store, { nowMs })
        private val drainer = Drainer(
            records = Sender(st.records, DiscardTransport, IngestRequest.serializer()),
            recordsOutbox = st.records,
            beats = Sender(st.beats, DiscardTransport, HeartbeatRequest.serializer()),
            drops = Sender(st.drops, DiscardTransport, DropReport.serializer()),
            ledger = st.ledger,
            contract = ContractGuard({ probe }, gate, { Instant.ofEpochMilli(nowMs) }),
            onContract = { notifier.update(it, oldestUnsentDays(st.records, st.age::now)) },
            maxRounds = 1,
        )

        fun tick() = drainer.tick()
    }

    /** 何でも受け付ける送り先（この試験の関心は送ることでなく、止まった状態の知らせ）。 */
    private object DiscardTransport : Transport {
        override fun post(bodyJson: String): Outcome {
            val rows = Json.parseToJsonElement(bodyJson).jsonArray.size
            return Outcome.Responded(200, List(rows) { """{"accepted":true}""" }.joinToString(",", "[", "]"))
        }
    }

    private fun advance(ms: Long) {
        nowMs += ms
        st.clock.advance(ms)
    }

    private fun req(id: String) =
        testFix(35.68, 139.76, 10f, Instant.parse("2026-09-08T02:00:00Z"))
            .toIngestRequest(id, "user-1", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 版 2 の行（版 1 の S-01 が受け付けない）を手元に置く。 */
    private fun leaveAheadRow() {
        val v = Outbox(st.recordStore, st.age::now, contractVersion = { 2 })
        v.add(req("ahead"))
    }

    private fun posted(): Notification? = shadowOf(manager).getNotification(ContractNotifier.CONTRACT_NOTIFICATION_ID)

    private fun postedText(): String? = posted()?.extras?.getCharSequence(Notification.EXTRA_TEXT)?.toString()

    private fun screenTexts(): List<String> {
        val activity = Robolectric.buildActivity(ExemptMainActivity::class.java).create().resume().get()
        val out = mutableListOf<String>()
        fun walk(v: View) {
            if (v is TextView) out += v.text.toString()
            if (v is ViewGroup) for (i in 0 until v.childCount) walk(v.getChildAt(i))
        }
        walk(activity.window.decorView)
        return out
    }

    /** 画面を開ける端末の権限をそろえる（位置・背景の位置・通知。開いたら収集を始めて休止の除外へ進む）。 */
    private fun readyForScreen() = shadowOf(app).grantPermissions(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
    )

    private fun noNoticeAnywhere() {
        assertNull("通知が出ている", posted())
        readyForScreen()
        val texts = screenTexts()
        assertFalse(texts.toString(), texts.any { it.contains("サーバを更新") })
    }

    /** 通知を出した回数。出した時に時刻の印が書き換わるので、その変わった回数で数える。 */
    private class PostCounter(private val store: ContractWaitStore) {
        var count = 0
        private var seen: Long? = null

        fun observe() {
            val now = store.notifiedAtMs
            if (now != null && now != seen) count++
            seen = now
        }
    }

    // Scenario: 契約の版で止まると画面と通知に出る
    @Test
    fun `404 で行を手元に残し始めると、画面に更新の行が出て、同じ文言の通知が 1 回出る`() {
        leaveAheadRow()
        val p = Process().apply { probe = notFound }

        p.tick()

        val text = postedText()
        assertNotNull("通知が出ていない", text)
        assertTrue(text!!, text.contains(CONTRACT_UPDATE_HEADLINE))
        assertEquals(1, shadowOf(manager).size())
        readyForScreen()
        val screen = screenTexts()
        assertTrue(screen.toString(), screen.contains(text))
    }

    // Scenario: 画面には手元に残している日数が出る
    @Test
    fun `積んでから 12 日の行が最も古い残りなら、画面の行に 12 日が出る`() {
        leaveAheadRow()
        advance(12 * day + 5_000)
        val p = Process().apply { probe = notFound }

        p.tick()

        readyForScreen()
        val line = screenTexts().single { it.contains(CONTRACT_UPDATE_HEADLINE) }
        assertTrue(line, line.contains("12 日分"))
    }

    // Scenario: 再起動しても 24 時間経つまで通知は出直さない
    @Test
    fun `通知を出してから 3 時間後に作り直しても出し直さず、24 時間経った契機で 1 回出る`() {
        leaveAheadRow()
        var p = Process().apply { probe = notFound }
        p.tick()
        val first = p.store.notifiedAtMs
        manager.cancelAll()

        advance(3 * hour)
        p = Process().apply { probe = notFound }
        p.tick()
        assertNull("再起動で出し直した", posted())
        assertEquals(first, p.store.notifiedAtMs)

        advance(21 * hour)
        p.tick()
        assertNotNull("24 時間経っても出ていない", posted())
    }

    // Scenario: 止まっている間は 1 日に 1 回だけ通知が出直す
    @Test
    fun `5 分ごとに 2 日進めて、通知は止まった時と 1 日後と 2 日後の 3 回、欄には 1 通だけが残る`() {
        leaveAheadRow()
        val p = Process().apply { probe = notFound }
        val counter = PostCounter(p.store)

        p.tick()
        counter.observe()
        repeat((2 * 24 * 12)) {
            advance(5 * 60 * 1000L)
            p.tick()
            counter.observe()
        }

        assertEquals(3, counter.count)
        assertEquals(1, shadowOf(manager).size())
    }

    // Scenario: S-01 を更新すると通知と画面の行が消える
    @Test
    fun `S-01 が更新されて行が全部送れると、通知も画面の行も消える`() {
        leaveAheadRow()
        val p = Process().apply { probe = notFound }
        p.tick()
        assertNotNull(posted())

        // S-01 が版 2 に更新され、残っていた版 2 の行が受け付けられる
        p.probe = Outcome.Responded(200, """{"contract_version":2}""")
        advance(5 * 60 * 1000L)
        p.tick()

        noNoticeAnywhere()
    }

    // Scenario: 到達できないだけでは更新を促さない
    @Test
    fun `圏外で 3 日間送れなくても、画面にも通知にも更新の行は出ない`() {
        st.records.add(req("a"))
        val p = Process().apply { probe = offline }

        repeat(3 * 24 * 12) {
            p.tick()
            advance(5 * 60 * 1000L)
        }

        noNoticeAnywhere()
    }

    // Scenario: 読めない 200 の答えが 24 時間続くまでは更新を促さない
    @Test
    fun `200 で HTML が 23 時間続いても、更新も網の確認も画面にも通知にも出ない`() {
        st.records.add(req("a"))
        val p = Process().apply { probe = html }

        repeat(23 * 12) {
            p.tick()
            advance(5 * 60 * 1000L)
        }

        assertEquals("行は送られずに残る", 1, st.records.size())
        noNoticeAnywhere()
    }

    // Scenario: 読めない 200 の答えが 24 時間続くと網の確認を添えて更新を促す
    @Test
    fun `200 で HTML が 24 時間続くと、網の確認を添えた行と同じ文言の通知が 1 回出る`() {
        st.records.add(req("a"))
        val p = Process().apply { probe = html }
        p.tick()
        advance(24 * hour)

        p.tick()

        val text = postedText()
        assertNotNull(text)
        assertTrue(text!!, text.contains(CONTRACT_NETWORK_HEADLINE))
        assertEquals(1, shadowOf(manager).size())
        readyForScreen()
        val screen = screenTexts()
        assertTrue(screen.toString(), screen.contains(text))
    }

    // Scenario: 再起動と圏外が挟まっても読めない 200 の数え始めは変わらない
    @Test
    fun `HTML を返し始めて 10 時間後の再起動と、圏外と 503 を挟んでも、最初から 24 時間の契機で 1 回出る`() {
        st.records.add(req("a"))
        var p = Process().apply { probe = html }
        p.tick()
        advance(10 * hour)
        p = Process().apply { probe = html }
        p.tick()
        advance(2 * hour)
        p.probe = offline
        p.tick()
        advance(2 * hour)
        p.probe = html
        p.tick()
        advance(1 * hour)
        p.probe = unavailable
        p.tick()
        advance(1 * hour)
        p.probe = html
        p.tick()
        assertNull("24 時間の前に出ている", posted())

        advance(8 * hour)
        p.tick()

        val text = postedText()
        assertNotNull("最初から 24 時間経った契機で出ていない", text)
        assertTrue(text!!, text.contains(CONTRACT_NETWORK_HEADLINE))
    }

    // Scenario: 読める答えを 1 回得ると読めない 200 の数え始めをやり直す
    @Test
    fun `HTML が 20 時間続いた後に版 1 を 1 回答え、また 20 時間続いても促さない`() {
        st.records.add(req("a"))
        val p = Process().apply { probe = html }
        p.tick()
        advance(20 * hour)
        p.tick()
        p.probe = readable
        p.tick()
        p.probe = html
        p.tick()
        advance(20 * hour)
        p.tick()

        noNoticeAnywhere()
    }

    private fun retentionAlerts(): Pair<RetentionNotifier, ContractGate> {
        val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
        val alerts = AndroidRetentionAlerts(service) { Notification.Builder(service, LocationService.CHANNEL).build() }
        val gate = ContractGate(ContractWaitStore(prefs()))
        val notifier = RetentionNotifier(
            st.records,
            st.age::now,
            alerts,
            File(tmp.root, "retention-alerted"),
            baseText = "base",
            contractHeadline = { gate.reasons.headline },
        )
        return notifier to gate
    }

    private fun retentionText(): String? =
        shadowOf(manager).getNotification(LocationService.RETENTION_NOTIFICATION_ID)
            ?.extras?.getCharSequence(Notification.EXTRA_TEXT)?.toString()

    // Scenario: 契約の版で止まっている間の 7 日前の知らせは更新を促す
    @Test
    fun `契約の版で止まったまま 83 日に達すると、音の鳴る通知に更新の案内が含まれる`() {
        val (notifier, gate) = retentionAlerts()
        st.records.add(req("a"))
        gate.observe(ContractAnswer.NoEndpoint, Instant.ofEpochMilli(nowMs), emptyList())
        st.clock.advance(83 * day)

        notifier.update()

        val text = retentionText()
        assertNotNull("7 日前の知らせが出ていない", text)
        assertTrue(text!!, text.contains(CONTRACT_UPDATE_HEADLINE))
        assertFalse(text, text.contains(CONTRACT_NETWORK_HEADLINE))
    }

    // Scenario: 読めない 200 の答えが続いて止まっている間の 7 日前の知らせは網の確認を添える
    @Test
    fun `読めない 200 で更新を待つまま 83 日に達すると、通知に網の確認を添えた案内が含まれる`() {
        val (notifier, gate) = retentionAlerts()
        st.records.add(req("a"))
        gate.observe(ContractAnswer.Unreadable, Instant.ofEpochMilli(nowMs), emptyList())
        advance(24 * hour)
        gate.observe(ContractAnswer.Unreadable, Instant.ofEpochMilli(nowMs), emptyList())
        st.clock.advance(83 * day)

        notifier.update()

        val text = retentionText()
        assertNotNull("7 日前の知らせが出ていない", text)
        assertTrue(text!!, text.contains(CONTRACT_NETWORK_HEADLINE))
    }
}
