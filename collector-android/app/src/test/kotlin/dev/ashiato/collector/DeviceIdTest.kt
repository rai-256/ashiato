// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

/** 端末識別子（tasks 6.1 / FR-24 / design D6）。 */
class DeviceIdTest {
    private class FakeStore(var value: String? = null) : IdStore {
        var writes = 0
        override fun read(): String? = value
        override fun write(value: String) {
            this.value = value
            writes++
        }
    }

    @Test
    fun `初回は採番して保存する`() {
        val store = FakeStore()
        val id = resolveDeviceId(store) { "generated-1" }
        assertEquals("generated-1", id)
        assertEquals("generated-1", store.value)
    }

    // Scenario: 端末識別子が端末をまたいで一意である
    @Test
    fun `アプリを再起動しても同じ値が返る`() {
        val store = FakeStore()
        val first = resolveDeviceId(store) { "generated-1" }
        // 「再起動」＝ 置き場だけ残して、採番の関数が別の値を返す状態で呼び直す
        val second = resolveDeviceId(store) { "generated-2" }
        assertEquals(first, second)
        assertEquals(1, store.writes)
    }

    @Test
    fun `空の値は採番し直す`() {
        val store = FakeStore("")
        assertEquals("generated-1", resolveDeviceId(store) { "generated-1" })
    }

    @Test
    fun `別の端末は別の値になる`() {
        val a = resolveDeviceId(FakeStore()) { "a" }
        val b = resolveDeviceId(FakeStore()) { "b" }
        assertNotEquals(a, b)
    }
}
