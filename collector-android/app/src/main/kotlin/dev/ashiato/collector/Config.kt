// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * 接続先と資格情報。**コミットしない**（製造準備 A-2）——
 * `~/.gradle/gradle.properties` か `-P` から `BuildConfig` に入る。
 *
 * 揃っていなければ**送信を始めない**。取得は続けるので、記録は未送信に積まれ、
 * 設定してから送られる（捨てない）。
 */
object Config {
    /**
     * 試験だけが差し替える。**本番では常に null** —— `BuildConfig` は
     * 試験のビルドでも空なので、設定が揃った経路を通す手段がこれしか無い。
     */
    private var override: Triple<String, String, String>? = null

    val baseUrl: String get() = override?.first ?: BuildConfig.BASE_URL
    val apiToken: String get() = override?.second ?: BuildConfig.API_TOKEN
    val userId: String get() = override?.third ?: BuildConfig.USER_ID

    /** 試験だけが差し替える（通知の開く先）。 */
    private var webUrlOverride: String? = null

    /** Web の画面のアドレス（通知が開く先。ST17 design D10）。**空でもビルドは止まらない** —— 通知は出て、押しても開かない。 */
    val webUrl: String get() = webUrlOverride ?: BuildConfig.WEB_URL

    val isComplete: Boolean
        get() = baseUrl.isNotBlank() && apiToken.isNotBlank() && userId.isNotBlank()

    internal fun overrideForTest(baseUrl: String, apiToken: String, userId: String) {
        override = Triple(baseUrl, apiToken, userId)
    }

    internal fun overrideWebUrlForTest(webUrl: String?) {
        webUrlOverride = webUrl
    }

    internal fun clearOverrideForTest() {
        override = null
        webUrlOverride = null
    }
}
