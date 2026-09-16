// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.content.Context
import android.content.Intent
import android.location.Location
import android.os.SystemClock
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import com.google.android.gms.location.LocationServices
import com.google.android.gms.tasks.Tasks
import java.io.BufferedReader
import java.io.File
import java.net.ServerSocket
import java.net.Socket
import java.time.Instant
import java.time.ZoneId
import java.util.Collections
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * ST04 の完了の判定を**端末の上で**確かめる（tasks 10.1 / 10.2 / 10.2b / design D15）。
 *
 * 1. 1 時間の圏外 → 戻すと全部が届き、破棄の報告は作られない
 * 2. 90 日ぶん（129,600 件）を溜めた端末が、落ちずに起動して送り切り、その間も取得が続き、新しい記録が 1 時間以内に届く
 * 3. 2 GB に近い量（1.9 GB）でも収集は起動し、取得の契機ごとに積まれる
 *
 * 相手は**テストが自分で開く**端末の中のサーバ（127.0.0.1）。平文は `-Pashiato.baseUrl=http://127.0.0.1:18787` で許される
 * （`tools/android-emulator.sh` と CI の `android-instrumented` がそうしている）。
 */
@RunWith(AndroidJUnit4::class)
class RetentionInstrumentedTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
    )

    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val outboxDir get() = File(context.filesDir, LocationService.OUTBOX_DIR)
    private var server: FakeServer? = null

    @Before
    fun clean() {
        context.stopService(Intent(context, LocationService::class.java))
        outboxDir.deleteRecursively()
        File(context.filesDir, "outbox.jsonl").delete()
        File(context.filesDir, "heartbeat.jsonl").delete()
    }

    @After
    fun tearDown() {
        context.stopService(Intent(context, LocationService::class.java))
        Config.clearOverrideForTest()
        server?.close()
        runCatching {
            Tasks.await(LocationServices.getFusedLocationProviderClient(context).setMockMode(false), 5, TimeUnit.SECONDS)
        }
        shell("appops set ${context.packageName} android:mock_location deny")
        outboxDir.deleteRecursively()
    }

    // Scenario: 1 時間の圏外の記録は全部が届く
    // Scenario: 1 時間の圏外では破棄の報告が作られない
    @Test
    fun anHourOfflineDeliversEveryRecordAndMakesNoDropReport() {
        val fake = FakeServer().also { server = it }
        val clock = StepClock()
        val dir = File(context.filesDir, "offline-hour").apply { deleteRecursively() }
        val age = AgeClock(clock, File(dir, "age-clock.txt"), {})
        val unreadable = File(dir, "unreadable.jsonl")
        val records = Outbox(SegmentStore(File(dir, "records"), IngestRequest.serializer(), unreadable, {}), age::now)
        val beats = Outbox(SegmentStore(File(dir, "heartbeats"), HeartbeatRequest.serializer(), unreadable, {}), age::now)
        val drops = Outbox(SegmentStore(File(dir, "drops"), DropReport.serializer(), unreadable, {}), age::now)
        val ledger = DropLedger(File(dir, "drops-open.json"), drops, { "user-1" }, "device-1", { Instant.now() }, { java.util.UUID.randomUUID().toString() }, {})
        val retention = Retention(records, ledger, age::now)

        var reachable = false
        fun transport(path: String) = Transport { body ->
            if (reachable) HttpTransport("http://127.0.0.1:${fake.port}", "t", path).post(body) else Outcome.Unreachable("ConnectException")
        }
        val drainer = Drainer(
            records = Sender(records, transport("/ingest"), IngestRequest.serializer(), dropPermanentlyRejected = true),
            recordsOutbox = records,
            beats = Sender(beats, transport("/heartbeat"), HeartbeatRequest.serializer()),
            drops = Sender(drops, transport("/drops"), DropReport.serializer()),
            ledger = ledger,
            maintenance = { retention.enforce() },
        )

        // 60 秒ごとに 1 件、5 分ごとに送信の契機。**1 時間ずっと到達できない**
        val start = Instant.now().minusSeconds(3600)
        repeat(60) { i ->
            records.add(fix("hour-$i", start.plusSeconds(60L * i)))
            clock.step(60_000)
            if (i % 5 == 4) drainer.tick()
        }
        assertEquals("圏外の 1 時間で未送信が減った", 60, records.size())

        reachable = true
        drainer.tick()

        assertEquals("1 時間ぶんが全部届いていない", 60, fake.ingestIds.size)
        assertEquals(0, records.size())
        assertTrue("破棄の報告が作られている", ledger.drafts().isEmpty() && drops.size() == 0 && fake.dropCount.get() == 0)
        dir.deleteRecursively()
    }

    // Scenario: 90 日ぶんを溜めた端末が起動して送り切る
    // Scenario: 溜まった状態でも取得は続く
    // Scenario: 溜まった分を送っている間に生まれた記録も 1 時間以内に届く
    @Test
    fun ninetyDaysOfBacklogStartsAndDrainsWhileCollectionContinues() {
        val total = 129_600
        placeRecords(total)
        val fake = FakeServer().also { server = it }
        Config.overrideForTest("http://127.0.0.1:${fake.port}", "test-token-0123456789abcdef", "00000000-0000-0000-0000-000000000000")

        ActivityScenario.launch(MainActivity::class.java).close()
        val client = mockLocations()

        // 送信の最初の契機（5 分）を待つあいだにも、取得の契機ごとに積まれる
        val injectedAt = System.currentTimeMillis()
        val grew = waitUntil(90_000) {
            client.inject(35.1, 139.1)
            tailContains("\"lat\":35.1")
        }
        assertTrue("溜まった状態で取得が積まれていない", grew)

        // 送り切る。**プロセスが落ちればこの試験ごと落ちる**（試験は同じプロセスで走る）
        val drained = waitUntil(40 * 60_000L) {
            client.inject(35.2, 139.2)
            fake.ingestIds.size >= total && fake.newestEventMs.get() >= injectedAt
        }
        assertTrue("送り切れていない（${fake.ingestIds.size} / $total）", drained)
        val placed = (0 until total).count { "backlog-$it" in fake.ingestIds }
        assertEquals("置いた 90 日ぶんが全部届いていない", total, placed)
        // 送っている間に生まれた記録が、生まれてから 1 時間以内に届いた
        val lag = fake.firstNewArrivalMs(injectedAt) - injectedAt
        assertTrue("新しい記録が届くまで $lag ms かかった", lag in 0 until 60 * 60_000L)
    }

    // Scenario: 2 GB に近い量でも収集は起動する
    @Test
    fun nearlyTwoGigabytesStillStartsAndKeepsCollecting() {
        placeBytes(1_900L * 1_000_000)
        ActivityScenario.launch(MainActivity::class.java).close()
        val client = mockLocations()
        val grew = waitUntil(120_000) {
            client.inject(35.3, 139.3)
            tailContains("\"lat\":35.3")
        }
        assertTrue("1.9 GB の置き場で取得が積まれていない", grew)
    }

    // ------------------------------------------------------------------ 足場

    private fun fix(id: String, at: Instant) =
        LocationFix(35.681236, 139.767125, 10f, at).toIngestRequest(id, "00000000-0000-0000-0000-000000000000", "device-1", ZoneId.of("Asia/Tokyo"))

    /** 区切りを直に置く（1 本 1 MB。本番の `SegmentStore` と同じ行の形）。積んでからの経過は 0（捨てない）。 */
    private fun placeRecords(n: Int) {
        val dir = File(outboxDir, "records").apply { mkdirs() }
        val start = Instant.now().minusSeconds(90L * 24 * 3600)
        var seq = 0
        var written = 0L
        var out = File(dir, "%012d.jsonl".format(seq)).bufferedWriter()
        for (i in 0 until n) {
            val line = "{\"enq\":0,\"item\":${ingestJson.encodeToString(IngestRequest.serializer(), fix("backlog-$i", start.plusSeconds(60L * i)))}}\n"
            if (written + line.length > SegmentStore.SEGMENT_BYTES) {
                out.close()
                seq++
                written = 0
                out = File(dir, "%012d.jsonl".format(seq)).bufferedWriter()
            }
            out.write(line)
            written += line.length
        }
        out.close()
    }

    /** 同じ形の行で `bytes` ぶんの区切りを置く（起動と取得だけを見るので、中身の違いは要らない）。 */
    private fun placeBytes(bytes: Long) {
        val dir = File(outboxDir, "records").apply { mkdirs() }
        val line = "{\"enq\":0,\"item\":${ingestJson.encodeToString(IngestRequest.serializer(), fix("bulk", Instant.now().minusSeconds(3600)))}}\n"
            .toByteArray(Charsets.UTF_8)
        val perSegment = (SegmentStore.SEGMENT_BYTES / line.size).toInt()
        val block = ByteArray(perSegment * line.size).also { b -> for (k in 0 until perSegment) line.copyInto(b, k * line.size) }
        val segments = bytes / block.size
        for (s in 0 until segments) File(dir, "%012d.jsonl".format(s)).writeBytes(block)
    }

    /** 最も新しい区切りに `needle` があるか（**全件を読まない**）。 */
    private fun tailContains(needle: String): Boolean {
        val last = File(outboxDir, "records").listFiles { f -> f.name.endsWith(".jsonl") }?.maxByOrNull { it.name } ?: return false
        return last.readText().contains(needle)
    }

    /**
     * シェルの命令を投げて**終わるまで待つ**。`executeShellCommand` は非同期で、出力を閉じるだけでは待たない ——
     * `appops set … allow` の直後に偽装位置を有効にすると、前の試験の `deny` と行き違って
     * `SecurityException: Caller must be selected as the mock location app` になった（CI の実測）。
     */
    private fun shell(cmd: String) {
        val fd = InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(cmd)
        android.os.ParcelFileDescriptor.AutoCloseInputStream(fd).use { it.readBytes() }
    }

    private inner class MockClient {
        val client = LocationServices.getFusedLocationProviderClient(context)

        fun inject(lat: Double, lon: Double) {
            val loc = Location("fused").apply {
                latitude = lat
                longitude = lon
                accuracy = 10f
                time = System.currentTimeMillis()
                elapsedRealtimeNanos = SystemClock.elapsedRealtimeNanos()
            }
            runCatching { Tasks.await(client.setMockLocation(loc), 10, TimeUnit.SECONDS) }
        }
    }

    private fun mockLocations(): MockClient {
        shell("appops set ${context.packageName} android:mock_location allow")
        val c = MockClient()
        // 許可が反映されるまで少し掛かることがあるので、断られたら数回だけ当たり直す
        var lastError: Throwable? = null
        for (attempt in 1..5) {
            val ok = runCatching { Tasks.await(c.client.setMockMode(true), 10, TimeUnit.SECONDS) }
                .onFailure { lastError = it }.isSuccess
            if (ok) return c
            Thread.sleep(1_000)
        }
        throw AssertionError("偽装位置を有効にできない", lastError)
    }

    private fun waitUntil(timeoutMs: Long, check: () -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (check()) return true
            Thread.sleep(2_000)
        }
        return check()
    }

    /** 1 分ずつ手で進める端末の時計（同じ起動のまま）。 */
    private class StepClock : DeviceClock {
        private var wall = System.currentTimeMillis()
        private var mono = 1_000L

        fun step(ms: Long) {
            wall += ms
            mono += ms
        }

        override fun wallMs() = wall

        override fun monoMs() = mono

        override fun bootCount(): Int? = 1
    }

    /**
     * 端末の中の偽のサーバ。**`/ingest` `/heartbeat` `/drops` を全部受け付け**、届いた記録の識別子と出来事の時刻を覚える。
     */
    private class FakeServer : AutoCloseable {
        private val socket = ServerSocket(0)
        val port: Int get() = socket.localPort
        val ingestIds: MutableSet<String> = Collections.synchronizedSet(HashSet())
        val dropCount = AtomicInteger(0)
        val newestEventMs = java.util.concurrent.atomic.AtomicLong(0)
        private val arrivals = Collections.synchronizedList(ArrayList<Pair<Long, Long>>())

        private val idPattern = Regex("\"id\":\"([^\"]+)\"")
        private val eventPattern = Regex("\"event_time\":\"([^\"]+)\"")

        init {
            Thread {
                while (!socket.isClosed) {
                    val s = runCatching { socket.accept() }.getOrNull() ?: break
                    Thread { handle(s) }.apply { isDaemon = true; start() }
                }
            }.apply { isDaemon = true; start() }
        }

        /** `after` 以降に生まれた記録が最初に届いた時刻。 */
        fun firstNewArrivalMs(after: Long): Long =
            synchronized(arrivals) { arrivals.filter { it.first >= after }.minOfOrNull { it.second } } ?: -1

        private fun handle(s: Socket): Unit = s.use {
            val reader = s.getInputStream().bufferedReader(Charsets.UTF_8)
            val requestLine = reader.readLine() ?: return@use
            var len = 0
            while (true) {
                val line = reader.readLine() ?: break
                if (line.isEmpty()) break
                val parts = line.split(":", limit = 2)
                if (parts[0].trim().equals("content-length", ignoreCase = true)) len = parts[1].trim().toInt()
            }
            val body = readExactly(reader, len)
            val path = requestLine.split(" ").getOrElse(1) { "" }
            val ids = idPattern.findAll(body).map { m -> m.groupValues[1] }.toList()
            when (path) {
                "/ingest" -> {
                    val now = System.currentTimeMillis()
                    ingestIds += ids
                    for (m in eventPattern.findAll(body)) {
                        val t = Instant.parse(m.groupValues[1]).toEpochMilli()
                        newestEventMs.accumulateAndGet(t) { a, b -> maxOf(a, b) }
                        arrivals += t to now
                    }
                }
                "/drops" -> dropCount.addAndGet(ids.size)
            }
            val reply = ids.joinToString(",", "[", "]") { """{"id":null,"duplicate":false,"accepted":true,"error":null}""" }
                .toByteArray(Charsets.UTF_8)
            s.getOutputStream().apply {
                write("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: ${reply.size}\r\nConnection: close\r\n\r\n".toByteArray())
                write(reply)
                flush()
            }
            Unit
        }

        private fun readExactly(reader: BufferedReader, len: Int): String {
            // 本文は ASCII（位置の記録と生存信号・破棄の報告の欄は英数字）なので、文字数とバイト数が一致する
            val buf = CharArray(len)
            var read = 0
            while (read < len) {
                val n = reader.read(buf, read, len - read)
                if (n < 0) break
                read += n
            }
            return String(buf, 0, read)
        }

        override fun close() {
            socket.close()
        }
    }
}
