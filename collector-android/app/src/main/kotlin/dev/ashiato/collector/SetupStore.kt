// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.Context
import java.net.URI

enum class SetupSaveResult {
    SAVED,
    UNREADABLE,
    UNREACHABLE,
}

/** 携帯に貼られた設定を、バックアップ対象外の専用 SharedPreferences に保存する。 */
class SetupStore(context: Context) {
    private val appContext = context.applicationContext
    private val prefs = appContext.getSharedPreferences(Config.SETUP_PREFS, Context.MODE_PRIVATE)

    fun save(text: String): SetupSaveResult {
        val setup = parseSetupString(text) ?: return SetupSaveResult.UNREADABLE
        if (!isReachableFromPhone(setup.url)) return SetupSaveResult.UNREACHABLE
        val edit = prefs.edit()
            .putString(URL_KEY, setup.url)
            .putString(TOKEN_KEY, setup.token)
        // 送り先と保留を 1 つの commit で書く。確かめの答えを待たずに門を閉じる（review/code.md R1）
        DestinationGate(prefs).holdOnSave(edit, setup.url)
        edit.commit()
        Config.initialize(appContext)
        return SetupSaveResult.SAVED
    }

    private fun isReachableFromPhone(url: String): Boolean {
        val uri = runCatching { URI(url) }.getOrNull() ?: return false
        if (uri.scheme?.lowercase() != "https" || uri.host.isNullOrBlank()) return false
        return uri.host.lowercase() !in LOOPBACK_HOSTS
    }

    internal companion object {
        const val URL_KEY = "url"
        const val TOKEN_KEY = "token"
        private val LOOPBACK_HOSTS = setOf("localhost", "127.0.0.1", "::1")
    }
}
