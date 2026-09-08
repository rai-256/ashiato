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
    val baseUrl: String get() = BuildConfig.BASE_URL
    val apiToken: String get() = BuildConfig.API_TOKEN
    val userId: String get() = BuildConfig.USER_ID

    val isComplete: Boolean
        get() = baseUrl.isNotBlank() && apiToken.isNotBlank() && userId.isNotBlank()
}
