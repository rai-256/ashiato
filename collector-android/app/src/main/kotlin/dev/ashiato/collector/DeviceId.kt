// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * 端末識別子の置き場。Android の実装と試験用の偽物を差し替えられるようにしてある。
 */
interface IdStore {
    fun read(): String?
    fun write(value: String)
}

/**
 * 端末識別子を返す。**初回だけ採番し、以後は同じ値を返す**（FR-24 / design D6）。
 *
 * `ANDROID_ID` を使わない理由は 2 つ —— 初期化でリセットされること、
 * 端末をまたいで追跡できる値であること。
 * アプリを消して入れ直すと変わるが、それは「別の端末」として扱ってよい（本人受け入れ済み）。
 */
fun resolveDeviceId(store: IdStore, generate: () -> String): String {
    val existing = store.read()
    if (existing != null && existing.isNotBlank()) return existing
    val fresh = generate()
    store.write(fresh)
    return fresh
}
