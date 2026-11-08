// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.Context

/**
 * 接続先と資格情報。**コミットしない**（製造準備 A-2）——
 * `~/.gradle/gradle.properties` か `-P` から `BuildConfig` に入る。
 *
 * 揃っていなければ**送信を始めない**。取得は続けるので、記録は未送信に積まれ、
 * 設定してから送られる（捨てない）。
 */
object Config {
    const val SETUP_PREFS = "collector_setup"
    const val DEFAULT_USER_ID = "00000000-0000-0000-0000-000000000000"

    @Volatile
    private var context: Context? = null

    /**
     * 試験だけが差し替える。**本番では常に null** —— `BuildConfig` は
     * 試験のビルドでも空なので、設定が揃った経路を通す手段がこれしか無い。
     */
    private var override: Triple<String, String, String>? = null

    private fun saved(key: String): String? = context?.getSharedPreferences(SETUP_PREFS, Context.MODE_PRIVATE)
        ?.getString(key, null)
        ?.takeIf { it.isNotEmpty() }

    val hasSavedSetup: Boolean get() = saved(SetupStore.URL_KEY) != null && saved(SetupStore.TOKEN_KEY) != null

    /** `-P`/BuildConfig の送り先は既に承認済みとして扱う（D13）。 */
    internal val hasBuildSetup: Boolean get() = override != null ||
        (BuildConfig.BASE_URL.isNotBlank() && BuildConfig.API_TOKEN.isNotBlank())

    /** 保存済み設定で上書きされない、ビルド時に焼き込まれた送り先。 */
    internal val buildBaseUrl: String get() = override?.first ?: BuildConfig.BASE_URL

    val baseUrl: String get() = saved(SetupStore.URL_KEY) ?: override?.first ?: BuildConfig.BASE_URL
    val apiToken: String get() = saved(SetupStore.TOKEN_KEY) ?: override?.second ?: BuildConfig.API_TOKEN
    val userId: String
        get() = (override?.third ?: BuildConfig.USER_ID).takeIf { it.isNotBlank() } ?: DEFAULT_USER_ID

    /** 試験だけが差し替える（通知の開く先）。 */
    private var webUrlOverride: String? = null

    /** Web の画面のアドレス（通知が開く先。ST17 design D10）。**空でもビルドは止まらない** —— 通知は出て、押しても開かない。 */
    val webUrl: String
        get() = webUrlOverride ?: saved(SetupStore.URL_KEY)?.let(::webUrlFor) ?: BuildConfig.WEB_URL

    val isComplete: Boolean
        get() = baseUrl.isNotBlank() && apiToken.isNotBlank() && userId.isNotBlank()

    internal fun overrideForTest(baseUrl: String, apiToken: String, userId: String) {
        override = Triple(baseUrl, apiToken, userId)
    }

    fun initialize(context: Context) {
        this.context = context.applicationContext
    }

    internal fun webUrlFor(url: String): String {
        val withoutSlash = url.trimEnd('/')
        return withoutSlash.removeSuffix("/api")
    }

    internal fun overrideWebUrlForTest(webUrl: String?) {
        webUrlOverride = webUrl
    }

    internal fun clearOverrideForTest() {
        override = null
        webUrlOverride = null
    }
}
