// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.shadows.ShadowLog
import java.time.ZoneId

/**
 * 健康データの権限の記録と、状態が変わったかの判定（ST11 / design D2 / D9 / D11）。
 * 差し替えるのは外との境目だけ（読み口・時計・未送信の口）。判定と置き場は本物が動く。
 */
@RunWith(RobolectricTestRunner::class)
class HealthPermissionRecordTest {
    @get:Rule val tmp = TemporaryFolder()

    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val allPermissions = HEALTH_PERMISSIONS.values.toSet()

    private val clock = FakeDeviceClock(wall = 1_800_000_000_123, mono = 123_456_789, boot = 42)

    /**
     * 試験の規則が消すフォルダに置く未送信。共有の `testOutbox` は CH03 で置き場を引数に取る形へ変わるので、
     * ここでは使わずに組む（ST11 のブランチと CH03 の後の木の両方で通る）。規則が立つのは試験の頭なので遅らせる。
     */
    private val outbox by lazy { scratchOutbox() }

    private fun scratchOutbox(): Outbox<IngestRequest> = Outbox.inDir(tmp.newFolder(), IngestRequest.serializer()) {}

    private val lines = mutableListOf<String>()
    private var reading = HealthReading(HealthAvailability.AVAILABLE, HealthFeatures(true, true), allPermissions)
    private var autoRequested = true
    private var accept = true
    private var ids = 0

    private fun recorder(): HealthPermissionRecorder {
        val prefs = app.getSharedPreferences(HEALTH_PREFS, Context.MODE_PRIVATE)
        prefs.edit().clear().commit()
        return HealthPermissionRecorder(
            read = { reading },
            autoRequested = { autoRequested },
            store = HealthStateStore(prefs),
            clock = clock,
            deviceId = "device-1",
            userId = "user-1",
            zone = ZoneId.of("Asia/Tokyo"),
            newId = { "id-${ids++}" },
            emit = { accept && outbox.add(it) },
            log = { lines += it },
        )
    }

    private fun records() = outbox.snapshot().filter { it.logicalSource == HEALTH_LOGICAL_SOURCE }

    private fun observe(r: HealthPermissionRecorder, trigger: String) =
        runBlocking { r.observeHealthPermissions(trigger) }

    // Scenario: 初めて観たときに健康データの権限の記録が 1 件できる
    @Test
    fun `初めて観たときは開いた契機の記録が 1 件未送信に積まれる`() {
        val r = recorder()

        observe(r, HealthTrigger.APP_OPEN)

        val all = records()
        assertEquals(1, all.size)
        assertEquals("app_open", all.single().payload["trigger"]!!.toString().trim('"'))
    }

    // Scenario: 状態が変わらなければ記録しない
    @Test
    fun `状態が変わらなければ開き直しにも 6 時間ごとの見直しにも記録は増えない`() {
        val r = recorder()
        observe(r, HealthTrigger.APP_OPEN)

        observe(r, HealthTrigger.APP_OPEN)
        observe(r, HealthTrigger.PERIODIC)

        assertEquals(1, records().size)
    }

    // Scenario: 時計が戻っても状態が変わらなければ記録しない
    @Test
    fun `時計を 1 日戻して同じ状態で観ても記録は増えない`() {
        val r = recorder()
        observe(r, HealthTrigger.APP_OPEN)

        clock.wall -= 24 * 60 * 60 * 1000L
        observe(r, HealthTrigger.APP_OPEN)

        assertEquals(1, records().size)
    }

    @Test
    fun `許可が変わったら契機に依らず新しい記録が 1 件増える`() {
        val r = recorder()
        observe(r, HealthTrigger.APP_OPEN)

        reading = reading.copy(grantedPermissions = allPermissions - HEALTH_PERMISSIONS.getValue("background"))
        observe(r, HealthTrigger.PERIODIC)

        assertEquals(2, records().size)
        assertEquals("periodic", records().last().payload["trigger"]!!.toString().trim('"'))
    }

    // Scenario: 健康データの権限の記録は観た時の端末の時計と起動からの経過時間と起動の識別を持つ
    @Test
    fun `記録の出来事の時刻は観た時の端末の時計で、原文は経過と起動の識別を持つ`() {
        val r = recorder()
        observe(r, HealthTrigger.REQUEST_RESULT)

        val rec = records().single()
        assertEquals("2027-01-15T08:00:00.123Z", rec.eventTime)
        assertTrue(rec.raw.contains(""""device_time":"2027-01-15T08:00:00.123Z""""))
        assertTrue(rec.raw.contains(""""elapsed_ms":123456789"""))
        assertTrue(rec.raw.contains(""""boot_count":42"""))
    }

    // Scenario: 健康データの権限の記録は観た時の端末の時計と起動からの経過時間と起動の識別を持つ
    @Test
    fun `起動の識別が取れない端末では無いこととして書く`() {
        clock.boot = null
        val r = recorder()
        observe(r, HealthTrigger.APP_OPEN)

        assertTrue(records().single().raw.contains(""""boot_count":null"""))
    }

    // Scenario: 健康データの権限のログに許されているものの中身が出ない
    @Test
    fun `ログに記録したことと契機が残り、許可の値と権限の名前は出ない`() {
        ShadowLog.clear()
        reading = reading.copy(grantedPermissions = setOf(HEALTH_PERMISSIONS.getValue("sleep")))
        val r = recorder()
        observe(r, HealthTrigger.REQUEST_RESULT)

        assertEquals(
            listOf(
                "kind=health_permission_recorded source=c01-health-permission " +
                    "trigger=request_result availability=available",
            ),
            lines,
        )
        val text = lines.joinToString("\n") + ShadowLog.getLogs().joinToString("\n") { it.msg.orEmpty() }
        assertFalse(text.contains("true"))
        assertFalse(text.contains("false"))
        assertFalse(text.contains("permission.health"))
        assertFalse(text.contains("READ_"))
    }

    @Test
    fun `欄の名前と並びは設計の形のまま`() {
        val rec = healthPermissionRecord(
            HealthState.of(
                reading.copy(grantedPermissions = allPermissions - HEALTH_PERMISSIONS.getValue("background")),
                true
            ),
            HealthTrigger.REQUEST_RESULT,
            null,
            clock,
            "device-1",
            "user-1",
            ZoneId.of("Asia/Tokyo")
        ) { "id-1" }

        assertEquals(
            """{"kind":"health-permission","trigger":"request_result","device_time":"2027-01-15T08:00:00.123Z",""" +
                """"elapsed_ms":123456789,"boot_count":42,"availability":"available",""" +
                """"features":{"history":true,"background":true},""" +
                """"granted":{"sleep":true,"heart_rate":true,"steps":true,"exercise":true,""" +
                """"history":true,"background":false},""" +
                """"auto_requested":true}""",
            rec.raw,
        )
        assertEquals(HEALTH_LOGICAL_SOURCE, rec.logicalSource)
        assertEquals("collected", rec.origin)
        assertEquals(1, rec.schemaVersion)
        assertNull(rec.externalId)
    }

    @Test
    fun `使えない端末では対応も許可も全部 false で、読めなかったときは型名だけ付く`() {
        val unreadable = HealthReading(
            HealthAvailability.UNREADABLE,
            HealthFeatures(true, true),
            allPermissions,
            "java.lang.IllegalStateException",
        )
        val rec = healthPermissionRecord(
            HealthState.of(unreadable, false),
            HealthTrigger.APP_OPEN,
            unreadable.errorType,
            clock,
            "d",
            "u",
            ZoneId.of("UTC"),
        ) { "id" }

        assertTrue(rec.raw.contains(""""availability":"unreadable","features":{"history":false,"background":false}"""))
        assertTrue(
            rec.raw.contains(
                """"granted":{"sleep":false,"heart_rate":false,"steps":false,"exercise":false,""" +
                    """"history":false,"background":false}""",
            ),
        )
        assertTrue(rec.raw.endsWith(""""auto_requested":false,"error":"java.lang.IllegalStateException"}"""))
    }

    @Test
    fun `同じ状態から 2 回作った原文は契機と時刻だけで違う`() {
        val state = HealthState.of(reading, true)
        val a = healthPermissionRecord(state, HealthTrigger.APP_OPEN, null, clock, "d", "u", ZoneId.of("UTC")) { "a" }
        clock.advance(5_000)
        val b = healthPermissionRecord(state, HealthTrigger.PERIODIC, null, clock, "d", "u", ZoneId.of("UTC")) { "b" }

        assertNotEquals(a.raw, b.raw)
        val strip = Regex(""""(trigger|device_time)":"[^"]*",?""")
        val withoutElapsed = Regex(""""elapsed_ms":\d+,?""")
        assertEquals(
            a.raw.replace(strip, "").replace(withoutElapsed, ""),
            b.raw.replace(strip, "").replace(withoutElapsed, ""),
        )
    }

    @Test
    fun `未送信に積めなかったら最後の状態を書き換えず、次に観たときにもう一度作る`() {
        val r = recorder()
        accept = false
        observe(r, HealthTrigger.APP_OPEN)
        assertTrue(records().isEmpty())
        assertTrue(lines.isEmpty())

        accept = true
        observe(r, HealthTrigger.APP_OPEN)

        assertEquals(1, records().size)
    }

    @Test
    fun `自動で求めたかの印が変わっても新しい記録になる`() {
        autoRequested = false
        val r = recorder()
        observe(r, HealthTrigger.APP_OPEN)

        autoRequested = true
        observe(r, HealthTrigger.REQUEST_RESULT)

        assertEquals(2, records().size)
    }

    // Scenario: 健康データの権限の生存信号は送られない
    @Test
    fun `健康データの権限は生存信号の対象に入らず、生存信号の契機に達しても積まれない`() {
        assertTrue(SourceCadence.entries.none { it.logicalSource == HEALTH_LOGICAL_SOURCE })

        val controller = Robolectric.buildService(TestableLocationService::class.java, Intent())
        val service = controller.get()
        for (c in SourceCadence.entries) {
            service.capabilities[c.logicalSource] = Capability.of(permission = true, sensor = true, network = true)
        }
        controller.create().startCommand(0, 1)
        service.beatSchedulers.values.forEach { it.fire() }

        assertTrue(service.heartbeatOutboxForTest.snapshot().isNotEmpty())
        assertTrue(service.heartbeatOutboxForTest.snapshot().none { it.logicalSource == HEALTH_LOGICAL_SOURCE })
    }

    // ---- 2 つの recorder（画面とサービス）が同じ置き場を共有する（review/code.md R1）----

    /** 本番の組み方: サービスは未送信へ直に積んで即時に書き換え、画面は受け渡し口に預けてサービスが積んだ後に書き換える。 */
    private fun sharedPair(): Pair<HealthPermissionRecorder, HealthPermissionRecorder> {
        HealthRecordHandoff.drainInto(scratchOutbox()) // 他の試験の残りを捨てる
        val prefs = app.getSharedPreferences(HEALTH_PREFS, Context.MODE_PRIVATE)
        prefs.edit().clear().commit()
        val store = HealthStateStore(prefs)
        fun make(
            emit: (IngestRequest) -> Boolean,
            remember: ((HealthState, () -> Unit) -> Unit)?
        ) = HealthPermissionRecorder(
            read = { reading },
            autoRequested = { autoRequested },
            store = store,
            clock = clock,
            deviceId = "device-1",
            userId = "user-1",
            zone = ZoneId.of("Asia/Tokyo"),
            newId = { "id-${ids++}" },
            emit = emit,
            log = { lines += it },
            remember = remember ?: { state, settled ->
                store.remember(state)
                settled()
            },
        )
        val service = make({ outbox.add(it) }, null)
        // 画面の書き換えは本番と同じ組み方（[MainActivity.healthRemember]）
        val screen = make(HealthRecordHandoff::offer) { state, settled ->
            HealthRecordHandoff.afterDrain {
                store.remember(state)
                settled()
            }
        }
        return service to screen
    }

    private val s1 = HealthReading(HealthAvailability.AVAILABLE, HealthFeatures(true, true), allPermissions)
    private val s2 = s1.copy(grantedPermissions = allPermissions - HEALTH_PERMISSIONS.getValue("history"))

    // Scenario: 後から設定で外すと次の見直しで記録される
    @Test
    fun `画面が別の状態を記録した後に戻すと、サービスの見直しが前に自分で記録した状態でも記録する`() {
        val (service, screen) = sharedPair()
        reading = s2
        observe(service, HealthTrigger.PERIODIC)
        reading = s1
        observe(screen, HealthTrigger.APP_OPEN)
        HealthRecordHandoff.drainInto(outbox) // サービスが立って画面の記録を積む

        reading = s2
        observe(service, HealthTrigger.PERIODIC)

        assertEquals(
            listOf(HealthTrigger.PERIODIC, HealthTrigger.APP_OPEN, HealthTrigger.PERIODIC),
            records().map { it.payload["trigger"]!!.toString().trim('"') },
        )
    }

    // Scenario: 状態が変わらなければ記録しない
    @Test
    fun `画面が記録済みの状態を、サービスの見直しがもう 1 件積まない`() {
        val (service, screen) = sharedPair()
        reading = s2
        observe(service, HealthTrigger.PERIODIC)
        reading = s1
        observe(screen, HealthTrigger.APP_OPEN)
        HealthRecordHandoff.drainInto(outbox)

        observe(service, HealthTrigger.PERIODIC)

        assertEquals(2, records().size)
    }

    @Test
    fun `サービスが記録した状態を、画面がもう 1 件積まず、サービスが変えた後に画面が戻ったら記録する`() {
        val (service, screen) = sharedPair()
        reading = s1
        observe(screen, HealthTrigger.APP_OPEN)
        HealthRecordHandoff.drainInto(outbox)
        reading = s2
        observe(service, HealthTrigger.PERIODIC)

        observe(screen, HealthTrigger.APP_OPEN) // 同じ S2 —— 積まない
        reading = s1
        observe(screen, HealthTrigger.APP_OPEN) // S1 へ戻った —— 画面が前に記録した状態でも積む
        HealthRecordHandoff.drainInto(outbox)

        assertEquals(
            listOf(HealthTrigger.APP_OPEN, HealthTrigger.PERIODIC, HealthTrigger.APP_OPEN),
            records().map { it.payload["trigger"]!!.toString().trim('"') },
        )
    }

    @Test
    fun `画面が預けて置き場に書く前に同じ状態をもう一度観ても 1 件だけ`() {
        val (_, screen) = sharedPair()
        observe(screen, HealthTrigger.APP_OPEN)
        observe(screen, HealthTrigger.REQUEST_RESULT)
        HealthRecordHandoff.drainInto(outbox)

        assertEquals(1, records().size)
    }

    @Test
    fun `受け渡し口で 1 件が置き場に書けなくても記録は失わず、書けた記録の書き換えは走り、書けなかった記録の書き換えだけを捨てる`() {
        val (_, screen) = sharedPair()
        val dir = tmp.newFolder("handoff")
        val segments = java.io.File(dir, "segments").apply { mkdirs() }
        // 1 件目を書いた直後から置き場が書けなくなる（端末の空きが尽きた）
        val failing = Outbox(
            SegmentStore(segments, IngestRequest.serializer(), java.io.File(dir, "unreadable.jsonl"), log = {}),
            age = { 0L },
            afterAdd = { segments.walkTopDown().forEach { it.setWritable(false) } },
        )
        val prefs = app.getSharedPreferences(HEALTH_PREFS, Context.MODE_PRIVATE)
        reading = s1
        observe(screen, HealthTrigger.APP_OPEN)
        reading = s2
        observe(screen, HealthTrigger.REQUEST_RESULT)

        try {
            HealthRecordHandoff.drainInto(failing)
        } finally {
            segments.walkTopDown().forEach { it.setWritable(true) }
        }

        assertEquals(
            "書けた記録の書き換えが走っていない",
            HealthState.of(s1, autoRequested).canonical(),
            prefs.getString("last_state", null)
        )
        assertEquals("書けなかった記録が未送信から消えた", 2, failing.snapshot().size)
    }

    private fun handoffRecord(id: String) = healthPermissionRecord(
        HealthState.of(s1, autoRequested),
        HealthTrigger.APP_OPEN,
        null,
        clock,
        "device-1",
        "user-1",
        ZoneId.of("Asia/Tokyo"),
    ) { id }

    @Test
    fun `記録と手続きが別々の drain に分かれても、書けなかった記録の手続きは走らない`() {
        HealthRecordHandoff.drainInto(scratchOutbox()) // 他の試験の残りを捨てる
        val dir = tmp.newFolder("handoff-split")
        val segments = java.io.File(dir, "segments").apply { mkdirs() }
        val failing = Outbox(
            SegmentStore(segments, IngestRequest.serializer(), java.io.File(dir, "unreadable.jsonl"), log = {}),
            age = { 0L },
        )
        segments.setWritable(false)
        var ran = false
        try {
            // 預けた直後・手続きを預ける前に収集の糸が drain した（review/code.md R22）
            HealthRecordHandoff.offer(handoffRecord("split-1"))
            HealthRecordHandoff.drainInto(failing)
            HealthRecordHandoff.afterDrain { ran = true }
            HealthRecordHandoff.drainInto(failing)
        } finally {
            segments.setWritable(true)
        }

        assertFalse("書けなかった記録の手続き（最後の状態の書き換え）が走った", ran)
    }

    @Test
    fun `積むところが投げても drain は投げず、残りの記録と手続きを続ける`() {
        HealthRecordHandoff.drainInto(scratchOutbox()) // 他の試験の残りを捨てる
        var throwOnce = true
        val throwing = Outbox(
            SegmentStore(
                tmp.newFolder("handoff-throw"),
                IngestRequest.serializer(),
                tmp.newFile("unreadable.jsonl"),
                log = {},
            ),
            age = { 0L },
            afterAdd = {
                check(!throwOnce.also { throwOnce = false }) { "boom" }
            },
        )
        val ran = mutableListOf<String>()
        val errors = mutableListOf<Throwable>()
        HealthRecordHandoff.offer(handoffRecord("throw-1"))
        HealthRecordHandoff.afterDrain { ran += "first" }
        HealthRecordHandoff.offer(handoffRecord("throw-2"))
        HealthRecordHandoff.afterDrain { ran += "second" }

        HealthRecordHandoff.drainInto(throwing) { errors += it } // 投げない（review/code.md R23）

        assertEquals(listOf("IllegalStateException"), errors.map { it.javaClass.simpleName })
        // 投げた記録は書けなかったものとして手続きを捨て（作り直す側）、後の記録は積んで手続きを走らせる
        assertEquals(listOf("second"), ran)
        assertTrue(throwing.snapshot().any { it.id == "throw-2" })
    }
}
