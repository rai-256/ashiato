// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Intent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner

/**
 * **起動の契機の取得を主糸で回さない**（ST06 / tasks 5.1）。
 *
 * `onStartCommand` は主糸で呼ばれる。ST06 より前はそこで `requestLocationUpdates` を
 * 1 本呼ぶだけだったが、いまは**アプリ利用の取り込み**が同じところに載る ——
 * 初回は 4 粒度ぶんの問い合わせ（年は 2 年ぶん）と数千件の追記になるので、
 * 主糸を塞ぐと前景サービスが「応答なし」に見える。
 *
 * ここは**糸そのものを見ない**（Robolectric の主糸は試験の糸）。見るのは
 * 「取得が `runCollection` を通っている」こと —— 通っていれば本番は別の糸に載る。
 */
@RunWith(RobolectricTestRunner::class)
class CollectionThreadTest {
    /** 取得を預かるだけで回さない入口。 */
    private class ParkingLocationService : TestableLocationService() {
        val parked = mutableListOf<() -> Unit>()

        override fun runCollection(task: () -> Unit) {
            parked += task
        }
    }

    @Test
    fun `起動の契機の取得は runCollection を通る`() {
        val controller = Robolectric.buildService(ParkingLocationService::class.java, Intent())
        val service = controller.get()
        service.usage = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T09:00:00Z")))

        controller.create().startCommand(0, 1)

        // 預かったのは**ソースの数だけ**（3 本）。1 本も主糸で走っていない
        assertEquals(SourceCadence.entries.size, service.parked.size)
        assertEquals("主糸で取得元へ登録している", 0, service.source.starts)

        // 預かった分を回せば、いつもどおり取得が走る
        service.parked.forEach { it() }
        assertEquals(1, service.source.starts)
        assertTrue("生存信号は取得を待たずに出ている", service.heartbeatOutboxForTest.size() > 0)
    }
}
