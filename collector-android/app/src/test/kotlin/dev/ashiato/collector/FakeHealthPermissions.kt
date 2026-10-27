// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import android.content.Intent

/**
 * 差し替えた健康データの権限の口（Robolectric は Health Connect を持たない）。
 * 求めた権限の組を数え、状態は試験が動かす。
 */
class FakeHealthPermissions(var reading: HealthReading) : HealthPermissions {
    val requests = mutableListOf<Set<String>>()

    override suspend fun read(context: Context): HealthReading = reading

    override fun createRequestIntent(context: Context, permissions: Set<String>): Intent {
        requests += permissions
        return Intent("test.HEALTH_REQUEST").putStringArrayListExtra("permissions", ArrayList(permissions))
    }

    override fun parseResult(resultCode: Int, data: Intent?): Set<String> = emptySet()

    /** 本人が設定や画面で許した。 */
    fun allow(vararg keys: String) {
        val added = keys.map { HEALTH_PERMISSIONS.getValue(it) }
        reading = reading.copy(grantedPermissions = reading.grantedPermissions + added)
    }

    fun allowAll() = allow(*HEALTH_PERMISSIONS.keys.toTypedArray())

    companion object {
        fun available(history: Boolean = true, background: Boolean = true) =
            FakeHealthPermissions(
                HealthReading(HealthAvailability.AVAILABLE, HealthFeatures(history, background), emptySet())
            )

        fun unavailable(availability: HealthAvailability, errorType: String? = null) =
            FakeHealthPermissions(HealthReading(availability, HealthFeatures(false, false), emptySet(), errorType))
    }
}

/** Health Connect が無い端末の読み口（健康データを関心にしない既存の試験の入口が使う）。 */
fun noHealthConnect() = FakeHealthPermissions.unavailable(HealthAvailability.NOT_INSTALLED)
