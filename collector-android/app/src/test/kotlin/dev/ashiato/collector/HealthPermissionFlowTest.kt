// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Activity
import android.app.Application
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.android.controller.ActivityController

/** 健康データの口・糸・記録の渡し先を試験が持つ入口。写真と動画の求めた回も数える。 */
class HealthFlowActivity : ExemptMainActivity() {
    override fun healthPermissions(): HealthPermissions = health
    override fun runHealth(task: () -> Unit) = task()
    override fun healthRecordSink(): (IngestRequest) -> Boolean = { sunk += it; true }
    override fun healthRemember(store: HealthStateStore): (HealthState, () -> Unit) -> Unit = { state, settled ->
        store.remember(state)
        settled()
    }

    override fun requestPermissionGroup(permissions: List<String>) {
        order += "media"
        super.requestPermissionGroup(permissions)
    }

    companion object {
        var health: FakeHealthPermissions = FakeHealthPermissions.available()
        val sunk = mutableListOf<IngestRequest>()
        val order = mutableListOf<String>()
    }
}

/** 渡し先を差し替えない入口（本番の [HealthRecordHandoff] を通る）。 */
class HandoffFlowActivity : ExemptMainActivity() {
    override fun healthPermissions(): HealthPermissions = HealthFlowActivity.health
    override fun runHealth(task: () -> Unit) = task()
    override fun healthRecordSink(): (IngestRequest) -> Boolean = HealthRecordHandoff::offer
}

/** 求める intent を本物の client（Health Connect の Jetpack）で作る入口。Android 14 以降の形を見る。 */
class ClientIntentHealthFlowActivity : ExemptMainActivity() {
    override fun healthPermissions(): HealthPermissions = object : HealthPermissions by HealthFlowActivity.health {
        override fun createRequestIntent(context: Context, permissions: Set<String>): Intent =
            ClientHealthPermissions().createRequestIntent(context, permissions)
    }
    override fun runHealth(task: () -> Unit) = task()
    override fun healthRecordSink(): (IngestRequest) -> Boolean = { HealthFlowActivity.sunk += it; true }
    override fun healthRemember(store: HealthStateStore): (HealthState, () -> Unit) -> Unit = { state, settled ->
        store.remember(state)
        settled()
    }
}

/** 初回起動の流れの中の健康データの段（tasks 5.1 / design D4・D6・D9）。Health Connect の画面そのものは計測テストが見る。 */
@RunWith(RobolectricTestRunner::class)
class HealthPermissionFlowTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val health get() = HealthFlowActivity.health

    @Before
    fun setUp() {
        HealthFlowActivity.health = FakeHealthPermissions.available()
        HealthFlowActivity.sunk.clear()
        HealthFlowActivity.order.clear()
        shadowOf(app).grantPermissions(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
            *mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).toTypedArray(),
        )
    }

    private fun launch(): ActivityController<HealthFlowActivity> =
        Robolectric.buildActivity(HealthFlowActivity::class.java).create().resume()

    /** Health Connect の画面が結果を返す（**中身は問わない**。実装は読み直す）。 */
    private fun answer(controller: ActivityController<HealthFlowActivity>) {
        val shadow = shadowOf(controller.get())
        shadow.receiveResult(shadow.nextStartedActivityForResult.intent, Activity.RESULT_OK, null)
    }

    private fun started(): Boolean = shadowOf(app).nextStartedService != null

    private fun healthMarks() = app.getSharedPreferences(HEALTH_PREFS, Context.MODE_PRIVATE)

    private val sunk get() = HealthFlowActivity.sunk
    private fun IngestRequest.field(name: String) = payload.getValue(name).jsonPrimitive.content
    private fun IngestRequest.granted(key: String) = payload.getValue("granted").jsonObject.getValue(key).jsonPrimitive.boolean
    private fun IngestRequest.autoRequested() = payload.getValue("auto_requested").jsonPrimitive.boolean

    // Scenario: 初回起動で健康データの権限の画面が出る
    @Test
    fun `初回起動で求める画面が開き、6 つの権限を求める`() {
        val controller = launch()

        assertEquals("求めた回数", 1, health.requests.size)
        assertEquals(HEALTH_PERMISSIONS.values.toSet(), health.requests.single())
        val opened = shadowOf(controller.get()).peekNextStartedActivityForResult()
        assertNotNull("結果を受ける形で権限の画面を開いていない", opened)
        assertEquals("test.HEALTH_REQUEST", opened.intent.action)
        assertFalse("答える前に収集が始まっている", started())
    }

    // Scenario: 種類と履歴と背景をすべて許すとすべて許された状態になる
    @Test
    fun `6 つを許して返ると、読み直した状態も記録もすべて許されている`() {
        val controller = launch()
        health.allowAll()
        answer(controller)

        val now = kotlinx.coroutines.runBlocking { health.read(app) }
        assertEquals(HEALTH_PERMISSIONS.values.toSet(), now.grantedPermissions)
        val last = sunk.last()
        assertEquals(HealthTrigger.REQUEST_RESULT, last.field("trigger"))
        assertTrue(HEALTH_PERMISSIONS.keys.all { last.granted(it) })
    }

    // Scenario: 入れ直すと初回起動でもう一度求められる
    @Test
    fun `入れ直す（印と最後の状態が消える）と初回起動でもう一度求められる`() {
        answer(launch())
        assertEquals(1, health.requests.size)

        healthMarks().edit().clear().commit()
        launch()

        assertEquals(2, health.requests.size)
    }

    // Scenario: 種類を許して履歴の読み取りだけを断っても収集は始まる
    @Test
    fun `種類を許して履歴だけ断っても収集が始まり、履歴は許されていない`() {
        val controller = launch()
        health.allow("sleep", "heart_rate", "steps", "exercise", "background")
        answer(controller)

        assertTrue("収集が始まっていない", started())
        val last = sunk.last()
        assertTrue(listOf("sleep", "heart_rate", "steps", "exercise").all { last.granted(it) })
        assertFalse(last.granted("history"))
    }

    // Scenario: 権限の画面に答えずにアプリが終わると次の起動でもう一度求められる
    @Test
    fun `答えずに画面が壊れると、作り直して開いたときにもう一度求める`() {
        val first = launch()
        assertEquals(1, health.requests.size)
        first.pause().stop().destroy()
        assertFalse("答えていないのに印が立っている", healthMarks().getBoolean(ASKED_HEALTH_PERMISSIONS, false))

        launch()

        assertEquals(2, health.requests.size)
        assertFalse("答えていないのに収集が始まっている", started())
    }

    // Scenario: 断った後に設定で許すと次に観たときに記録される
    @Test
    fun `断った後に設定で許して開くと、許された状態の記録が 1 件積まれる`() {
        answer(launch())
        val before = sunk.size

        health.allow("sleep", "heart_rate", "steps", "exercise", "history")
        launch()

        assertEquals("積まれた件数", before + 1, sunk.size)
        val added = sunk.last()
        assertEquals(HealthTrigger.APP_OPEN, added.field("trigger"))
        assertTrue(listOf("sleep", "heart_rate", "steps", "exercise", "history").all { added.granted(it) })
        assertEquals("自動では求め直さない", 1, health.requests.size)
    }

    // Scenario: 健康データの権限は写真と動画の後・収集を始める前に求められる
    @Test
    fun `写真と動画の後に求め、答えた後で収集が始まる`() {
        shadowOf(app).denyPermissions(*mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).toTypedArray())
        val controller = launch()

        assertEquals("写真と動画を先に求める", listOf("media"), HealthFlowActivity.order)
        assertEquals("写真と動画に答える前に健康データを求めている", 0, health.requests.size)
        controller.get().onRequestPermissionsResult(
            1,
            mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).toTypedArray(),
            IntArray(mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).size) { -1 },
        )

        assertEquals("写真と動画の後に健康データを求める", 1, health.requests.size)
        assertFalse("答える前に収集が始まっている", started())
        answer(controller)
        assertTrue("答えた後に収集が始まっていない", started())
    }

    // Scenario: 前の版から更新した端末では更新の後の最初の起動で 1 度求められる
    @Test
    fun `前の版で初回起動を済ませた端末（健康データの印だけ無い）では 1 度求める`() {
        app.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE).edit()
            .putBoolean(MainActivity.SENT_TO_USAGE_ACCESS, true).commit()
        assertFalse(healthMarks().contains(ASKED_HEALTH_PERMISSIONS))

        launch()

        assertEquals(1, health.requests.size)
    }

    // Scenario: 2 度目の起動では健康データの権限を自動で求めない
    @Test
    fun `1 度求めて答えた後は、許されていないものが残っても求めない`() {
        answer(launch())
        assertEquals(1, health.requests.size)

        val second = launch()

        assertEquals("2 度目に自動で求めている", 1, health.requests.size)
        assertNull(shadowOf(second.get()).peekNextStartedActivityForResult())
        assertTrue("収集が始まっていない", started())

        // 位置の記録が生成される（同じ型でサービスを立てて位置を届ける）
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val service = controller.get()
        controller.create().startCommand(0, 1)
        service.source.deliver()
        assertTrue("位置の記録が生成されていない", service.outboxForTest.snapshot().any { it.logicalSource == LOGICAL_SOURCE })
    }

    // Scenario: 初めて観たときに健康データの権限の記録が 1 件できる
    @Test
    fun `本番の渡し口を通ると、サービスが立ったとき未送信に積まれ、そこで初めて最後の状態が書かれる`() {
        HealthRecordHandoff.drainInto(testOutbox()) // 他の試験の残りを捨てる
        healthMarks().edit().clear().commit()
        val controller = Robolectric.buildActivity(HandoffFlowActivity::class.java).create().resume()
        answer2(controller)

        assertNull("未送信に届く前に最後の状態が書かれている", healthMarks().getString("last_state", null))

        val service = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val s = service.get()
        service.create().startCommand(0, 1)

        // 画面が観た 2 件（開いた・結果が返った。自動で求めた印が違うので別の状態）
        assertEquals(
            "未送信に積まれた健康データの権限の記録の契機",
            listOf(HealthTrigger.APP_OPEN, HealthTrigger.REQUEST_RESULT),
            s.outboxForTest.snapshot().filter { it.logicalSource == HEALTH_LOGICAL_SOURCE }.map { it.field("trigger") },
        )
        assertNotNull("積めた後に最後の状態が書かれていない", healthMarks().getString("last_state", null))
    }

    private fun answer2(controller: ActivityController<HandoffFlowActivity>) {
        val shadow = shadowOf(controller.get())
        shadow.receiveResult(shadow.nextStartedActivityForResult.intent, Activity.RESULT_OK, null)
    }

    // Scenario: 求めるものがすべて許されていれば画面を出さない
    @Test
    fun `求めるものがすべて許されていれば画面を出さず収集が始まる`() {
        health.allowAll()

        launch()

        assertEquals(0, health.requests.size)
        assertTrue(started())
        assertFalse("求めていないのに印が立っている", healthMarks().getBoolean(ASKED_HEALTH_PERMISSIONS, false))
    }

    // Scenario: Health Connect が無い端末では求めずに収集が始まる
    @Test
    fun `Health Connect が無い端末では求めずに収集が始まり、無いことが記録される`() {
        HealthFlowActivity.health = FakeHealthPermissions.unavailable(HealthAvailability.NOT_INSTALLED)

        launch()

        assertEquals(0, health.requests.size)
        assertTrue(started())
        assertEquals("not_installed", sunk.single().field("availability"))
    }

    // Scenario: 更新が要る Health Connect の端末では求めずに収集が始まる
    @Test
    fun `更新が要る Health Connect の端末では求めずに収集が始まり、更新が要ることが記録される`() {
        HealthFlowActivity.health = FakeHealthPermissions.unavailable(HealthAvailability.UPDATE_REQUIRED)

        launch()

        assertEquals(0, health.requests.size)
        assertTrue(started())
        assertEquals("update_required", sunk.single().field("availability"))
    }

    // Scenario: Health Connect が使えるようになったら 1 度だけ求める
    @Test
    fun `Health Connect が入った後に開くと 1 度だけ求め、その次は求めない`() {
        HealthFlowActivity.health = FakeHealthPermissions.unavailable(HealthAvailability.NOT_INSTALLED)
        launch()
        assertEquals(0, health.requests.size)
        assertFalse("求めていないのに印が立っている", healthMarks().getBoolean(ASKED_HEALTH_PERMISSIONS, false))

        HealthFlowActivity.health = FakeHealthPermissions.available()
        answer(launch())
        launch()

        assertEquals("入った後に 1 度だけ", 1, health.requests.size)
    }

    // Scenario: 履歴の読み取りに対応していない端末では対応しているものだけを求める
    @Test
    fun `履歴と背景に未対応なら 4 つの種類だけを求め、対応していないことが記録される`() {
        HealthFlowActivity.health = FakeHealthPermissions.available(history = false, background = false)

        launch()

        val four = listOf("sleep", "heart_rate", "steps", "exercise").map { HEALTH_PERMISSIONS.getValue(it) }.toSet()
        assertEquals(four, health.requests.single())
        val record = sunk.single()
        assertEquals("available", record.field("availability"))
        val features = record.payload.getValue("features").jsonObject
        assertFalse(features.getValue("history").jsonPrimitive.boolean)
        assertFalse(features.getValue("background").jsonPrimitive.boolean)
    }

    // Scenario: Health Connect が使えるかを読めない端末では求めずに収集が始まる
    @Test
    fun `使えるかを読めない端末では求めずに収集が始まり、読めなかったことが記録される`() {
        HealthFlowActivity.health =
            FakeHealthPermissions.unavailable(HealthAvailability.UNREADABLE, errorType = "java.lang.IllegalStateException")

        launch()

        assertEquals(0, health.requests.size)
        assertTrue(started())
        assertEquals("unreadable", sunk.single().field("availability"))
    }

    // Scenario: 健康データの権限をすべて断っても収集は始まる
    @Test
    fun `種類を 1 つも許さずに断っても収集が始まる`() {
        answer(launch())

        assertTrue("収集が始まっていない", started())

        // 収集が始まった結果として、位置の記録が生成される
        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val service = controller.get()
        controller.create().startCommand(0, 1)
        service.source.deliver()
        assertTrue("位置の記録が生成されていない", service.outboxForTest.snapshot().any { it.logicalSource == LOGICAL_SOURCE })
    }

    // Scenario: 断ると自動で求めたことと許されていないことが記録される
    @Test
    fun `断ると、結果が返った契機で自動で求めたことと全部許されていないことが 1 件記録される`() {
        answer(launch())

        val results = sunk.filter { it.field("trigger") == HealthTrigger.REQUEST_RESULT }
        assertEquals(1, results.size)
        val record = results.single()
        assertTrue("自動で求めたことが載っていない", record.autoRequested())
        assertTrue("許されていないものがある", HEALTH_PERMISSIONS.keys.none { record.granted(it) })
        assertEquals("available", record.field("availability"))
    }

    // Scenario: 健康データの権限が無くても位置とアプリ利用と写真は集まる
    @Test
    fun `健康データの権限を全部外していても位置とアプリ利用と写真の記録が積まれる`() {
        // 健康データの権限は 1 つも許されていない（断った）まま収集を始める
        answer(launch())
        val granted = kotlinx.coroutines.runBlocking { health.read(app) }.grantedPermissions
        assertTrue(HEALTH_PERMISSIONS.values.none { it in granted })

        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val service = controller.get()
        val event = UsageEventSnapshot(
            packageName = "dev.ashiato.example",
            className = "dev.ashiato.example.MainActivity",
            eventType = 1,
            at = java.time.Instant.now().minusMillis(60_000),
        )
        service.usage = FakeUsageSource(storedEvents = listOf(event))
        controller.create().startCommand(0, 1)
        service.source.deliver()

        val env = MediaScanEnv()
        env.listing.put(PRIMARY, 1, "DCIM/Camera/", "IMG_1.jpg", "photo")
        env.scan()

        assertTrue("位置の記録が積まれていない", service.outboxForTest.snapshot().any { it.logicalSource == LOGICAL_SOURCE })
        assertTrue(
            "アプリ利用の記録が積まれていない",
            service.outboxForTest.snapshot().any { it.logicalSource == SourceCadence.APP_USAGE.logicalSource },
        )
        assertEquals("写真の記録が積まれていない", 1, env.photos().size)
    }

    // Scenario: 初回起動で健康データの権限の画面が出る
    @Test
    @org.robolectric.annotation.Config(sdk = [34])
    fun `Android 14 以降は client の intent を開かず、中の権限を framework の口で求め、戻ったら読み直して進む`() {
        val controller = Robolectric.buildActivity(ClientIntentHealthFlowActivity::class.java).create().resume()
        val shadow = shadowOf(controller.get())

        // 本物の client は 14 以降で内向きの intent（ComponentActivity の結果の登録簿だけが受け取れる）を返す
        assertEquals(
            MainActivity.ACTION_REQUEST_PERMISSIONS,
            ClientHealthPermissions().createRequestIntent(app, HEALTH_PERMISSIONS.values.toSet()).action,
        )
        // それを startActivityForResult で開かない（端末では解決できずに失敗する）
        assertNotEquals(
            "解決できない intent を開いている",
            MainActivity.ACTION_REQUEST_PERMISSIONS,
            shadow.peekNextStartedActivityForResult()?.intent?.action,
        )
        val asked = shadow.lastRequestedPermission
        assertNotNull("健康データの権限を求めていない", asked)
        assertEquals(MainActivity.HEALTH_REQUEST, asked.requestCode)
        assertEquals(HEALTH_PERMISSIONS.values.toSet(), asked.requestedPermissions.toSet())
        assertFalse("答える前に収集が始まっている", started())

        health.allowAll()
        controller.get().onRequestPermissionsResult(
            MainActivity.HEALTH_REQUEST,
            asked.requestedPermissions,
            IntArray(asked.requestedPermissions.size),
        )

        assertTrue("戻っても印が立たない", healthMarks().getBoolean(ASKED_HEALTH_PERMISSIONS, false))
        assertEquals(HealthTrigger.REQUEST_RESULT, sunk.last().field("trigger"))
        assertTrue("戻っても収集が始まらない", started())
    }
}
