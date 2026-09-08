// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context

/**
 * 端末識別子をアプリの保存領域に置く（design D6）。
 * **`ANDROID_ID` は使わない** —— 初期化でリセットされ、端末をまたいで追跡できる値でもある。
 */
class AndroidIdStore(context: Context) : IdStore {
    private val prefs = context.getSharedPreferences("ashiato", Context.MODE_PRIVATE)

    override fun read(): String? = prefs.getString(KEY, null)

    override fun write(value: String) {
        prefs.edit().putString(KEY, value).apply()
    }

    private companion object {
        const val KEY = "device_id"
    }
}
