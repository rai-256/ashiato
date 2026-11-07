package dev.ashiato.collector

import android.app.Application
import android.content.Intent
import android.view.ViewGroup
import android.widget.Button
import android.widget.TextView
import androidx.test.core.app.ApplicationProvider
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.time.Instant

/** 確かめの問い合わせを回さない画面（答えが返らないまま、の代わり）。 */
class St56SilentProbeActivity : SetupActivity() {
    var probes = 0
    override fun runContractProbe(task: () -> Unit) {
        probes++
        // 回さない —— 答えが返らないまま画面が閉じられた場合と同じ
    }
}

@RunWith(RobolectricTestRunner::class)
class St56DestinationGateTest {
    @get:Rule val tmp = TemporaryFolder()
    private val app: Application = ApplicationProvider.getApplicationContext()
    private val prefs get() = app.getSharedPreferences(Config.SETUP_PREFS, 0)
    private val old = Instant.parse("2026-10-31T23:00:00Z")
    private val recent = Instant.parse("2026-11-03T12:30:00Z")

    @Before fun setUp() { prefs.edit().clear().commit() }

    @After fun tearDown() { prefs.edit().clear().commit() }

    private fun gate() = DestinationGate(prefs)
    private fun backlog(count: Int = 1) = DestinationBacklog(count, old, recent)

    private fun setup(url: String) {
        prefs.edit().putString(
            SetupStore.URL_KEY,
            url
        ).putString(SetupStore.TOKEN_KEY, "token-for-test-0123456789").commit()
        Config.initialize(app)
    }

    @Test
    fun `サービス実行中に送り先を保存すると未送信の送信を保留する`() {
        Config.overrideForTest("https://a.example/api", "token-a", "u")
        try {
            setup("https://a.example/api")
            val controller = Robolectric.buildService(TestableLocationService::class.java, Intent()).create()
            val service: TestableLocationService = controller.get()
            controller.startCommand(0, 1)
            service.outboxForTest.add(
                testFix(35.68, 139.76, 10f, old).toIngestRequest("pending", "u", "d", java.time.ZoneId.of("Asia/Tokyo"))
            )
            setup("https://b.example/api")

            service.onStartCommand(Intent().apply { action = LocationService.ACTION_SETUP_CHANGED }, 0, 2)
            service.scheduler.fire()

            assertTrue(DestinationGate(prefs).blocked)
            assertEquals("https://a.example/api", DestinationGate(prefs).approved)
            assertTrue(service.posted.none { it.first == "/ingest" && it.second.contains("pending") })
        } finally {
            Config.clearOverrideForTest()
        }
    }

    @Test
    fun `保留を承認するとサービスを起こし直さず次の契機で送る`() {
        // Scenario: 設定を保存すると、サービスを起こし直さずに送り始める
        Config.overrideForTest("https://a.example/api", "token-a", "u")
        try {
            setup("https://a.example/api")
            val controller = Robolectric.buildService(TestableLocationService::class.java, Intent()).create()
            val service = controller.get()
            controller.startCommand(0, 1)
            service.outboxForTest.add(
                testFix(
                    35.68,
                    139.76,
                    10f,
                    old
                ).toIngestRequest("pending-send", "u", "d", java.time.ZoneId.of("Asia/Tokyo"))
            )
            setup("https://b.example/api")
            service.onStartCommand(Intent().apply { action = LocationService.ACTION_SETUP_CHANGED }, 0, 2)
            DestinationGate(prefs).approvePending()
            service.onStartCommand(Intent().apply { action = LocationService.ACTION_SETUP_CHANGED }, 0, 3)
            service.scheduler.fire()

            assertTrue(service.posted.any { it.first == "/ingest" && it.second.contains("pending-send") })
            assertFalse(DestinationGate(prefs).blocked)
        } finally {
            Config.clearOverrideForTest()
        }
    }

    @Test
    fun `保留通知を押すと設定画面に未送信件数と承認ボタンが出る`() {
        // Scenario: 保留の間は通知で促し、押すと設定の画面が開く
        DestinationGate(prefs).reconcile("https://b.example/api", backlog(3))
        val service = Robolectric.buildService(LocationService::class.java, Intent()).create().get()
        DestinationGate(prefs).reconcile("https://b.example/api", backlog(3))
        service.showDestinationNotice()
        val notification = shadowOf(service.getSystemService(android.app.NotificationManager::class.java))
            .getNotification(LocationService.DESTINATION_NOTIFICATION_ID)
        assertEquals(
            SetupActivity::class.java.name,
            shadowOf(notification.contentIntent).savedIntent.component?.className
        )

        val activity = Robolectric.buildActivity(SetupActivity::class.java).create().resume().get()
        val root = activity.findViewById<ViewGroup>(android.R.id.content).getChildAt(0) as ViewGroup
        assertTrue(root.children().any { it is TextView && it.text.toString().contains("未送信 3 件") })
        assertTrue(root.children().any { it is Button && it.text.toString() == "この送り先へ送る" })
    }

    private fun ViewGroup.children(): List<android.view.View> =
        (0 until childCount).map(::getChildAt)

    @Test fun `送り先を揃えて比較する`() {
        val g = gate()
        g.approve("https://PC.example.ts.net:443/api/")
        assertFalse(g.reconcile("https://pc.example.ts.net/api", backlog()))
    }

    @Test fun `未送信が無ければ直ちに承認する`() {
        assertFalse(gate().reconcile("https://b.example/api", DestinationBacklog.EMPTY))
    }

    @Test fun `未送信がある送り先変更は保留する`() {
        assertTrue(gate().reconcile("https://b.example/api", backlog()))
    }

    @Test fun `承認済み送り先が無くても保留する`() {
        val g = gate()
        assertTrue(g.reconcile("https://b.example/api", backlog(3)))
        assertEquals(3, g.summary.count)
    }

    @Test fun `承認ボタンで保留を解く`() {
        val g = gate()
        g.reconcile("https://b.example/api", backlog())
        g.approvePending()
        assertFalse(g.blocked)
        assertEquals("https://b.example/api", g.approved)
    }

    @Test fun `保留件数を表示用に持つ`() {
        val g = gate()
        g.reconcile("https://b.example/api", backlog(3))
        assertEquals(3, g.summary.count)
    }

    @Test fun `保留の最古と最新を表示用に持つ`() {
        val g = gate()
        g.reconcile("https://b.example/api", backlog())
        assertEquals(old, g.summary.oldest)
        assertEquals(recent, g.summary.newest)
    }

    @Test fun `保留中は送信を許可しない`() {
        val g = gate()
        g.reconcile("https://b.example/api", backlog())
        assertFalse(g.maySend)
    }

    @Test fun `収集用の未送信は保留の判定に使える`() {
        val g = gate()
        assertTrue(g.reconcile("https://b.example/api", backlog()))
        assertTrue(g.blocked)
    }

    @Test fun `通知文言を持つ`() {
        val g = gate()
        g.reconcile("https://b.example/api", backlog())
        assertEquals("送り先の確認を待っています", g.headline)
    }

    @Test fun `同じ送り先なら保留を作らない`() {
        val g = gate()
        g.approve("https://b.example/api")
        assertFalse(g.reconcile("https://b.example/api/", backlog()))
    }

    @Test
    fun `サービスのすべての送信経路は保留中に門を通る`() {
        // Scenario: 未送信があるまま送り先を変えると、押すまで何も送らない
        val stores = TestStores(tmp.newFolder("drainer"))
        stores.records.add(
            testFix(35.68, 139.76, 10f, old).toIngestRequest("queued", "u", "d", java.time.ZoneId.of("Asia/Tokyo"))
        )
        stores.beats.add(
            HeartbeatRequest("h1", "u", LOGICAL_SOURCE, "d", "2026-11-01T00:00:00Z", true, emptyList(), 1, 1, "{}")
        )
        val gate = DestinationGate(prefs).also { it.reconcile("https://b.example/api", backlog()) }
        var posts = 0
        var beatPosts = 0
        val records = Sender(stores.records, Transport {
            posts++
            Outcome.Responded(200, "[{\"accepted\":true}]")
        }, IngestRequest.serializer())
        val beats = Sender(stores.beats, Transport {
            beatPosts++
            Outcome.Responded(200, "[{\"accepted\":true}]")
        }, HeartbeatRequest.serializer())
        val drops = Sender(stores.drops, Transport { Outcome.Responded(200, "[]") }, DropReport.serializer())
        // 保留の間は契機の頭の `GET /contract` も新しい送り先へ送らず、保持の上限は当てる（review/code.md R4）
        var probes = 0
        var maintained = 0
        val contract = ContractGuard({
            probes++
            Outcome.Responded(200, "{\"contract_version\":1}")
        }, ContractGate(ContractWaitStore(prefs)))
        val drainer = Drainer(
            records,
            stores.records,
            beats,
            drops,
            stores.ledger,
            contract = contract,
            maintenance = { maintained++ },
            destination = gate,
        )
        drainer.tick()
        drainer.tick()
        assertEquals(0, posts)
        assertEquals("保留の間に生存信号を送った", 0, beatPosts)
        assertEquals(0, probes)
        assertEquals(2, maintained)
    }

    @Test
    fun `確かめが返らないまま保存しても送る契機は新しい送り先へ送らない`() {
        // review/code.md R1: 保存の commit と保留を同時に立てる。確かめの答えもサービスの突き合わせも待たない
        Config.overrideForTest("https://a.example/api", "token-a", "u")
        try {
            setup("https://a.example/api")
            val controller = Robolectric.buildService(TestableLocationService::class.java, Intent()).create()
            val service = controller.get()
            controller.startCommand(0, 1)
            assertEquals("https://a.example/api", DestinationGate(prefs).approved)
            service.outboxForTest.add(
                testFix(35.68, 139.76, 10f, old).toIngestRequest("held", "u", "d", java.time.ZoneId.of("Asia/Tokyo"))
            )
            val sentBefore = service.posted.size

            val activity = Robolectric.buildActivity(St56SilentProbeActivity::class.java).create().resume().get()
            val root = activity.findViewById<ViewGroup>(android.R.id.content).getChildAt(0) as ViewGroup
            (root.getChildAt(
                1
            ) as android.widget.EditText).setText(
                buildSetupString("https://b.example/api", "token-for-test-0123456789")
            )
            root.children().filterIsInstance<Button>().first { it.text.toString() == "保存" }.performClick()

            // サービスへの知らせは確かめと無関係に出ている
            assertEquals(LocationService.ACTION_SETUP_CHANGED, shadowOf(app).nextStartedService?.action)
            assertEquals("https://b.example/api", Config.baseUrl)
            // サービスがまだ突き合わせていなくても、送る契機は何も送らない
            service.scheduler.fire()
            assertEquals(sentBefore, service.posted.size)
            assertTrue(service.posted.none { it.second.contains("held") })
            assertEquals(1, activity.probes)

            // サービスが数え終えると、保存した画面に件数と「この送り先へ送る」が出る（review/code.md R5）
            service.onStartCommand(Intent().apply { action = LocationService.ACTION_SETUP_CHANGED }, 0, 2)
            assertTrue(DestinationGate(prefs).blocked)
            val count = DestinationGate(prefs).summary.count
            assertTrue(count >= 1)
            assertTrue(root.children().any { it is TextView && it.text.toString().contains("未送信 $count 件") })
            assertTrue(root.children().any { it is Button && it.text.toString() == "この送り先へ送る" })
        } finally {
            Config.clearOverrideForTest()
        }
    }

    @Test
    fun `未送信が無いまま保存すると突き合わせで保留が解ける`() {
        Config.overrideForTest("https://a.example/api", "token-a", "u")
        try {
            setup("https://a.example/api")
            // 生存信号を積む前（onCreate の突き合わせだけ）で A を承認している
            val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
            assertEquals("https://a.example/api", DestinationGate(prefs).approved)
            assertTrue(
                SetupStore(app).save(buildSetupString("https://b.example/api", "token-for-test-0123456789")) == SetupSaveResult.SAVED
            )
            assertTrue(DestinationGate(prefs).blocked)
            service.onStartCommand(Intent().apply { action = LocationService.ACTION_SETUP_CHANGED }, 0, 2)
            assertFalse(DestinationGate(prefs).blocked)
            assertEquals("https://b.example/api", DestinationGate(prefs).approved)
        } finally {
            Config.clearOverrideForTest()
        }
    }

    @Test
    fun `保留を解いた後の 2 回目の保留も知らせる`() {
        // review/code.md R5: 解くときに知らせの時刻も消す
        val g = gate()
        g.reconcile("https://b.example/api", backlog())
        g.markNotified(1_000L)
        g.approvePending()
        g.reconcile("https://c.example/api", backlog())
        assertTrue(g.notificationDue(2_000L))
    }

    @Test
    fun `承認後の Drainer は実際に未送信を送る`() {
        // Scenario: 「この送り先へ送る」を押すと送り始める
        val stores = TestStores(tmp.newFolder("drainer-approved"))
        stores.records.add(
            testFix(35.68, 139.76, 10f, old).toIngestRequest("queued", "u", "d", java.time.ZoneId.of("Asia/Tokyo"))
        )
        val gate = DestinationGate(prefs).also {
            it.reconcile("https://b.example/api", backlog())
            it.approvePending()
        }
        var posts = 0
        val records = Sender(stores.records, Transport {
            posts++
            Outcome.Responded(200, "[{\"accepted\":true}]")
        }, IngestRequest.serializer())
        val empty = Sender(stores.beats, Transport { Outcome.Responded(200, "[]") }, HeartbeatRequest.serializer())
        val drops = Sender(stores.drops, Transport { Outcome.Responded(200, "[]") }, DropReport.serializer())
        Drainer(records, stores.records, empty, drops, stores.ledger, destination = gate).tick()
        assertEquals(1, posts)
        assertFalse(gate.blocked)
    }

    @Test
    fun `未知の承認先ではサービスの記録を送らない`() {
        // Scenario: 設定の前に積んだ記録は、設定の後に送られ、捨てられない
        // Scenario: 承認した送り先が分からないまま未送信があっても、押すまで送らない
        setup("https://b.example/api")
        try {
            val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
            var malformed = 0
            service.reply = { _, count ->
                val body = service.posted.last().second
                if (body.contains("\"user_id\":\"\"")) {
                    malformed++
                    Outcome.Responded(400, verdicts(count, "{\"accepted\":false,\"error\":\"malformed\"}"))
                } else {
                    Outcome.Responded(200, verdicts(count, "{\"accepted\":true}"))
                }
            }
            service.outboxForTest.add(
                testFix(
                    35.68,
                    139.76,
                    10f,
                    old
                ).toIngestRequest("queued", Config.DEFAULT_USER_ID, "d", java.time.ZoneId.of("Asia/Tokyo"))
            )
            prefs.edit().remove(DestinationGate.APPROVED_URL).commit()
            service.onStartCommand(Intent().apply { action = LocationService.ACTION_SETUP_CHANGED }, 0, 2)
            service.scheduler.fire()
            assertTrue(DestinationGate(prefs).blocked)
            assertTrue(service.posted.none { it.first == "/ingest" })
            assertEquals(1, DestinationGate(prefs).summary.count)
            DestinationGate(prefs).approvePending()
            service.onStartCommand(Intent().apply { action = LocationService.ACTION_SETUP_CHANGED }, 0, 3)
            service.scheduler.fire()
            assertTrue(service.posted.any { it.first == "/ingest" && it.second.contains(Config.DEFAULT_USER_ID) })
            assertEquals(0, malformed)
        } finally { Config.clearOverrideForTest() }
    }

    @Test
    fun `空の未送信ならサービスの送信が直ちに通る`() {
        // Scenario: 未送信が無ければ、保存した送り先へそのまま送る
        Config.overrideForTest("https://b.example/api", "token-for-test-0123456789", "u")
        try {
            setup("https://b.example/api")
            val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
            service.onStartCommand(Intent(), 0, 1)
            service.outboxForTest.add(
                testFix(35.68, 139.76, 10f, old).toIngestRequest("new", "u", "d", java.time.ZoneId.of("Asia/Tokyo"))
            )
            service.scheduler.fire()
            assertTrue(service.posted.any { it.first == "/ingest" && it.second.contains("new") })
        } finally { Config.clearOverrideForTest() }
    }

    @Test
    fun `正規化だけの資格情報変更は送信を止めない`() {
        // Scenario: 資格情報だけを変えたなら保留しない
        val g = gate().also { it.approve("https://PC.example.ts.net/api/") }
        assertFalse(g.reconcile("https://pc.example.ts.net:443/api", backlog()))
        assertTrue(g.maySend)
    }

    @Test
    fun `保留中も LocationService は記録を積み続ける`() {
        // Scenario: 保留の間も収集は続く
        val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
        service.outboxForTest.add(
            testFix(
                35.68,
                139.76,
                10f,
                old
            ).toIngestRequest("collected", Config.DEFAULT_USER_ID, "d", java.time.ZoneId.of("Asia/Tokyo"))
        )
        DestinationGate(prefs).reconcile("https://b.example/api", backlog())
        assertEquals("collected", service.outboxForTest.snapshot().single().id)
    }

    @Test
    fun `画面には未送信の期間を表示する`() {
        // Scenario: 保留の画面は積んだ期間を示す
        DestinationGate(prefs).reconcile("https://b.example/api", backlog())
        val activity = Robolectric.buildActivity(SetupActivity::class.java).create().resume().get()
        val text = (
            (activity.findViewById<ViewGroup>(android.R.id.content).getChildAt(0) as ViewGroup)
                .children().filterIsInstance<TextView>()
            ).joinToString(" ") { it.text }
        assertTrue(text, text.contains("2026-11-01 08:00"))
        assertTrue(text, text.contains("2026-11-03 21:30"))
    }

    @Test
    fun `OriginalUploader は保留中に原本を開かない`() {
        // Scenario: 保留の間は写真の原本も送らない
        val h = OriginalHarness(tmp.newFolder("original"))
        h.photo(1, "photo")
        h.scan()
        h.acceptRecords()
        val gate = gate().also { it.reconcile("https://b.example/api", backlog()) }
        val uploader = OriginalUploader(
            h.env.originals,
            h.source,
            h.transport,
            h.reporter,
            OriginalEnvironment({ MediaAccess.FULL }, { true }, h::now),
            canSend = { gate.maySend },
        )
        assertEquals(0, uploader.run())
        assertTrue(h.transport.puts.isEmpty())
        assertEquals(0, h.source.opens)
    }

    @Test
    fun `サービスの途切れの問い合わせは保留中に回らない`() {
        // review/code.md R25: アプリが組む OutagePoller の門（試験の側で canQuery を渡さない）
        shadowOf(app).grantPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        setup("https://a.example/api")
        val service = Robolectric.buildService(
            TestableLocationService::class.java,
            Intent()
        ).create().startCommand(0, 1).get()
        val before = service.outageQueries.size
        assertEquals("起動時の 1 発", 1, before)
        DestinationGate(prefs).reconcile("https://b.example/api", backlog())
        service.beatScheduler.fire()
        service.beatScheduler.fire()
        assertEquals("保留の間に途切れを問い合わせた", before, service.outageQueries.size)
        DestinationGate(prefs).approvePending()
        service.beatScheduler.fire()
        assertEquals("承認の後も問い合わせない", before + 1, service.outageQueries.size)
    }

    @Test
    fun `気分の受け手が組む門は保留中に閉じる`() {
        // review/code.md R25: 受け手（FeelingReminderWiring.runner）が実際に渡す門
        setup("https://a.example/api")
        val g = gate().also { it.approve("https://a.example/api") }
        assertTrue(FeelingReminderWiring.runner(app).canQuery())
        g.reconcile("https://b.example/api", backlog())
        assertFalse(FeelingReminderWiring.runner(app).canQuery())
    }

    @Test
    fun `OutagePoller は保留中に問い合わせない`() {
        var queries = 0
        val gate = gate().also { it.reconcile("https://b.example/api", backlog()) }
        val poller = OutagePoller(
            query = OutageQuery {
                queries++
                Outcome.Responded(200, OutageFixtures.READY_EMPTY)
            },
            notice = OutageNotice(prefs),
            notifier = OutageNotifier(app),
            deviceId = { "device" },
            executor = DirectExecutorService(),
            canQuery = { gate.maySend },
        )
        poller.poll()
        assertEquals(0, queries)
    }

    @Test
    fun `FeelingReminder は保留中にサーバへ聞かない`() {
        var queries = 0
        val gate = gate().also { it.reconcile("https://b.example/api", backlog()) }
        val alarms = object : ReminderAlarms {
            override fun scheduleReminder(at: Instant) = Unit
            override fun scheduleRefresh(at: Instant) = Unit
        }
        FeelingReminderRunner(
            source = ReminderSource {
                queries++
                ReminderAnswer(java.time.LocalTime.NOON, false)
            },
            state = FeelingReminderState(app),
            alarms = alarms,
            notifier = ReminderNotifier { true },
            clock = { old },
            canQuery = { gate.maySend },
        ).wake()
        assertEquals(0, queries)
    }

    @Test
    fun `続けて送っている最中に別の送り先を保存すると残りを送らない`() {
        // review/code.md R31: 1 回目の POST の中で B を保存する。押すまで B へは 1 件も送らない
        setup("https://a.example/api")
        val stores = TestStores(tmp.newFolder("drainer-mid"))
        repeat(650) { i ->
            stores.records.add(
                testFix(
                    35.68,
                    139.76,
                    10f,
                    old.plusSeconds(i.toLong())
                ).toIngestRequest("r$i", "u", "d", java.time.ZoneId.of("Asia/Tokyo"))
            )
        }
        val gate = DestinationGate(prefs).also { it.approve("https://a.example/api") }
        val sentTo = mutableListOf<String>()
        val records = Sender(
            stores.records,
            Transport { body ->
                sentTo += Config.baseUrl
                if (sentTo.size == 1) {
                    assertEquals(
                        SetupSaveResult.SAVED,
                        SetupStore(app).save(buildSetupString("https://b.example/api", "token-for-test-0123456789"))
                    )
                }
                val n = kotlinx.serialization.json.Json.parseToJsonElement(
                    body
                ).let { it as kotlinx.serialization.json.JsonArray }.size
                Outcome.Responded(200, verdicts(n, "{\"accepted\":true}"))
            },
            IngestRequest.serializer()
        )
        val beats = Sender(stores.beats, Transport {
            sentTo += Config.baseUrl
            Outcome.Responded(200, "[]")
        }, HeartbeatRequest.serializer())
        val drops = Sender(stores.drops, Transport {
            sentTo += Config.baseUrl
            Outcome.Responded(200, "[]")
        }, DropReport.serializer())
        Drainer(records, stores.records, beats, drops, stores.ledger, destination = gate).tick()
        assertEquals(listOf("https://a.example/api"), sentTo)
        assertTrue(gate.blocked)
        assertEquals("https://a.example/api", gate.approved)
    }

    @Test
    fun `読めない送り先で保存しても保留が解ける`() {
        // review/code.md R41: normalize が読めない送り先でも、0 件なら解け、あれば「この送り先へ送る」で解ける
        val odd = "https://exa mple.test/api"
        assertEquals(null, DestinationGate.normalize(odd))
        val g = gate().also { it.approve("https://a.example/api") }
        prefs.edit().also { g.holdOnSave(it, odd) }.commit()
        assertTrue(g.blocked)
        assertFalse(g.reconcile(odd, backlog(0)))
        assertFalse(g.blocked)

        g.approve("https://a.example/api")
        prefs.edit().also { g.holdOnSave(it, odd) }.commit()
        assertTrue(g.reconcile(odd, backlog(3)))
        g.approvePending()
        assertFalse(g.blocked)
    }

    @Test
    fun `未送信の件数と期間は置き場を 1 件ずつ読んで数える`() {
        // review/code.md R46: snapshot で全部を載せない。件数・最古・最新が全件を見た値と一致する
        val stores = TestStores(tmp.newFolder("span"))
        val zone = java.time.ZoneId.of("Asia/Tokyo")
        listOf(recent, old, old.plusSeconds(60)).forEachIndexed { i, at ->
            stores.records.add(testFix(35.68, 139.76, 10f, at).toIngestRequest("s$i", "u", "d", zone))
        }
        val span = stores.records.span { Instant.parse(it.eventTime) }
        assertEquals(DestinationBacklog(3, old, recent), span)
        assertEquals(DestinationBacklog(0, null, null), stores.beats.span { Instant.parse(it.emittedAt) })
    }

    @Test
    fun `組み立ての送り先を替えて組み直しても、保存した設定が無ければ今どおり送る`() {
        // review/code.md R47: 前の -P の承認が残り、未送信があると保留に倒れていた（Global Constraints「-P の APK は今どおり送る」）
        Config.overrideForTest("http://127.0.0.1:18787", "token-a", "u")
        try {
            val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
            assertEquals("http://127.0.0.1:18787", DestinationGate(prefs).approved)
            val zone = java.time.ZoneId.of("Asia/Tokyo")
            service.outboxForTest.add(testFix(35.68, 139.76, 10f, old).toIngestRequest("pending", "u", "d", zone))
            // -P の値を替えて組み直した APK（保存した設定は無い）
            Config.overrideForTest("http://127.0.0.1:19999", "token-a", "u")
            service.onStartCommand(Intent().apply { action = LocationService.ACTION_SETUP_CHANGED }, 0, 2)
            assertFalse(DestinationGate(prefs).blocked)
            assertEquals("http://127.0.0.1:19999", DestinationGate(prefs).approved)
        } finally {
            Config.clearOverrideForTest()
        }
    }
}
