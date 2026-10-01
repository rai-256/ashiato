// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import androidx.test.core.app.ApplicationProvider
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * ソースを 2 本持てる形（design D5 / tasks 1.1）。
 *
 * **`LocationService` は「収集の親」に徹し、ソースは
 * `logicalSource` / `intervalMs` / `capability(context)` / `collect(window)` を持つ部品になる。**
 * ここで確かめるのは口の形と、位置をその口に載せても**振る舞いが 1 つも変わっていない**こと
 * —— 間隔は FR-1 の 60 秒のまま、権限が無ければ `SecurityException` がそのまま親へ抜ける。
 */
@RunWith(RobolectricTestRunner::class)
class CollectionSourceTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    private fun collector() = FixCollector(
        outbox = testOutbox(),
        deviceId = "device-1",
        userId = "user-1",
        zone = java.time.ZoneId.of("Asia/Tokyo"),
        newId = { "r1" },
    )

    private fun window() = CollectionWindow(
        Instant.parse("2026-05-01T00:00:00Z"),
        Instant.parse("2026-05-01T00:01:00Z"),
    )

    @Test
    fun `位置のソースは登録簿の名前と FR-1 の 60 秒を名乗る`() {
        val adapter = LocationSourceAdapter(FakeFixSource(fail = false), collector())
        assertEquals(LOGICAL_SOURCE, adapter.logicalSource)
        assertEquals(FIX_INTERVAL_MS, adapter.intervalMs)
    }

    /** **取得元が契機を自分で配る**（Play Services）。親は登録を頼むだけで、記録は callback の側で積まれる。 */
    @Test
    fun `取得は取得元へ FR-1 の 60 秒で登録される`() {
        val fix = FakeFixSource(fail = false)
        val adapter = LocationSourceAdapter(fix, collector())
        assertSame(CollectionResult.Streaming, adapter.collect(window()))
        assertEquals(FIX_INTERVAL_MS, fix.startedWith)
    }

    /**
     * **呼ばれるたびに登録し直す**（ST06 より前の `onStartCommand` と同じ）。
     *
     * 絞ると「アプリを開き直すたびに要求を張り直す」という**復旧の経路**が消え、
     * 取得が止まった端末がそこから戻れなくなる。
     */
    @Test
    fun `呼ばれるたびに取得元へ登録し直す`() {
        val fix = CountingFixSource()
        val adapter = LocationSourceAdapter(fix, collector())
        repeat(3) { adapter.collect(window()) }
        assertEquals(3, fix.starts)
    }

    /** 権限が無いときは**握りつぶさない** —— 親（`LocationService`）が受けて `no_permission` を残す。 */
    @Test
    fun `権限が無ければ取得元の SecurityException がそのまま抜ける`() {
        val adapter = LocationSourceAdapter(FakeFixSource(fail = true), collector())
        var thrown = false
        try {
            adapter.collect(window())
        } catch (e: SecurityException) {
            thrown = true
        }
        assertTrue("SecurityException が親へ抜けていない", thrown)
    }

    @Test
    fun `止めると取得元の登録も外れ、次は登録し直せる`() {
        val fix = CountingFixSource()
        val adapter = LocationSourceAdapter(fix, collector())
        adapter.collect(window())
        adapter.stop()
        assertEquals(1, fix.stops)
        adapter.collect(window())
        assertEquals(2, fix.starts)
    }

    /** 登録し直しでも**同じ callback を渡す**（渡すものが変わると要求が置き換わらず重なる）。 */
    @Test
    fun `登録し直しでも渡す callback は同じ`() {
        val fix = CountingFixSource()
        val callback = collector()
        val adapter = LocationSourceAdapter(fix, callback)
        repeat(2) { adapter.collect(window()) }
        adapter.stop()
        assertEquals(listOf<Any>(callback, callback, callback), fix.callbacks)
    }

    /** 取得条件は**端末から読む**。読むだけで直そうとしない（`AndroidCapability` の規律）。 */
    @Test
    fun `取得条件は端末から読む`() {
        val adapter = LocationSourceAdapter(FakeFixSource(fail = false), collector())
        val cap = adapter.capability(app)
        assertTrue("位置の権限が無い既定なのに取れる状態と報告している", !cap.capturable)
        assertTrue(cap.blockers.contains(Capability.PERMISSION))
    }

    /** 窓は `[begin, end)`。**終わりが始まりより前の窓は組み立てられない。** */
    @Test
    fun `逆さの窓は組み立てられない`() {
        var rejected = false
        try {
            CollectionWindow(Instant.parse("2026-05-01T01:00:00Z"), Instant.parse("2026-05-01T00:00:00Z"))
        } catch (e: IllegalArgumentException) {
            rejected = true
        }
        assertTrue("逆さの窓が通った", rejected)
    }
}

/** 登録と解除の回数、および渡された callback を数える取得元の偽物。 */
class CountingFixSource : FixSource {
    var starts = 0
    var stops = 0

    /** `start` / `stop` に渡された callback を呼ばれた順に覚える。 */
    val callbacks = mutableListOf<com.google.android.gms.location.LocationCallback>()

    override fun start(intervalMs: Long, callback: com.google.android.gms.location.LocationCallback) {
        starts++
        callbacks += callback
    }

    override fun stop(callback: com.google.android.gms.location.LocationCallback) {
        stops++
        callbacks += callback
    }
}
