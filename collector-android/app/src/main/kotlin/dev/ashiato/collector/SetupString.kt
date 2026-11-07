// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.util.Base64

private const val PREFIX = "ashiato-collector:1:"

@Serializable
data class SetupString(val url: String, val token: String) {
    override fun toString(): String = "SetupString(url=$url, token=<redacted>)"
}

private val setupJson = Json { ignoreUnknownKeys = true }

fun buildSetupString(url: String, token: String): String {
    val encoded = Base64.getUrlEncoder().withoutPadding().encodeToString(
        setupJson.encodeToString(SetupString.serializer(), SetupString(url, token)).toByteArray(Charsets.UTF_8),
    )
    return PREFIX + encoded
}

fun parseSetupString(input: String): SetupString? {
    val encoded = input.trim()
    if (!encoded.startsWith(PREFIX)) return null
    val payload = encoded.removePrefix(PREFIX)
    if (payload.isEmpty() || !payload.matches(Regex("[A-Za-z0-9_-]+"))) return null
    return try {
        val value = setupJson.decodeFromString<SetupString>(
            String(Base64.getUrlDecoder().decode(payload), Charsets.UTF_8),
        )
        value.takeIf { it.url.isNotEmpty() && it.token.isNotEmpty() }
    } catch (_: IllegalArgumentException) {
        null
    }
}
