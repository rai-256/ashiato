// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.content.Intent
import androidx.health.connect.client.HealthConnectClient
import androidx.health.connect.client.HealthConnectFeatures
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/** 差し替えた client。状態は値で渡し、[fail] があれば全部の口がそれを投げる。 */
private class FakeHealthClientPort(
    private val sdk: Int = HealthConnectClient.SDK_AVAILABLE,
    private val history: Boolean = true,
    private val background: Boolean = true,
    private val granted: Set<String> = emptySet(),
    private val fail: Throwable? = null,
) : HealthClientPort {
    override fun sdkStatus(context: android.content.Context): Int = fail?.let { throw it } ?: sdk

    override suspend fun featureAvailable(context: android.content.Context, feature: Int): Boolean {
        fail?.let { throw it }
        return when (feature) {
            HealthConnectFeatures.FEATURE_READ_HEALTH_DATA_HISTORY -> history
            else -> background
        }
    }

    override suspend fun grantedPermissions(context: android.content.Context): Set<String> =
        fail?.let { throw it } ?: granted
}

@RunWith(RobolectricTestRunner::class)
class HealthPermissionsTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()
    private val four = listOf("sleep", "heart_rate", "steps", "exercise").map { HEALTH_PERMISSIONS.getValue(it) }
    private val all = HEALTH_PERMISSIONS.values.toList()

    private fun ask(availability: HealthAvailability, history: Boolean = true, background: Boolean = true) =
        healthPermissionsToAsk(availability, HealthFeatures(history, background))

    @Test
    fun availableWithHistoryAndBackgroundAsksAllSix() {
        assertEquals(all, ask(HealthAvailability.AVAILABLE))
    }

    @Test
    fun availableWithoutHistoryOrBackgroundAsksOnlyTheFourKinds() {
        assertEquals(four, ask(HealthAvailability.AVAILABLE, history = false, background = false))
    }

    @Test
    fun availableWithOnlyOneOfHistoryOrBackgroundAsksThatOne() {
        val history = HEALTH_PERMISSIONS.getValue("history")
        val background = HEALTH_PERMISSIONS.getValue("background")
        assertEquals(four + history, ask(HealthAvailability.AVAILABLE, background = false))
        assertEquals(four + background, ask(HealthAvailability.AVAILABLE, history = false))
    }

    @Test
    fun notInstalledUpdateRequiredAndUnreadableAskNothing() {
        listOf(HealthAvailability.NOT_INSTALLED, HealthAvailability.UPDATE_REQUIRED, HealthAvailability.UNREADABLE)
            .forEach { assertTrue("$it", ask(it).isEmpty()) }
    }

    @Test
    fun clientMapsSdkStatusToAvailability() = runBlocking {
        fun read(sdk: Int) = runBlocking { ClientHealthPermissions(FakeHealthClientPort(sdk = sdk)).read(app) }
        assertEquals(HealthAvailability.AVAILABLE, read(HealthConnectClient.SDK_AVAILABLE).availability)
        assertEquals(HealthAvailability.NOT_INSTALLED, read(HealthConnectClient.SDK_UNAVAILABLE).availability)
        assertEquals(
            HealthAvailability.UPDATE_REQUIRED,
            read(HealthConnectClient.SDK_UNAVAILABLE_PROVIDER_UPDATE_REQUIRED).availability,
        )
    }

    @Test
    fun clientReadsFeaturesAndOnlyTheSixGrantedPermissions() = runBlocking {
        val granted = setOf(HEALTH_PERMISSIONS.getValue("steps"), "android.permission.health.READ_DISTANCE")
        val reading = ClientHealthPermissions(
            FakeHealthClientPort(history = false, background = true, granted = granted),
        ).read(app)
        assertEquals(HealthFeatures(history = false, background = true), reading.features)
        assertEquals(setOf(HEALTH_PERMISSIONS.getValue("steps")), reading.grantedPermissions)
        assertNull(reading.errorType)
    }

    @Test
    fun readFailureBecomesUnreadableWithOnlyTheExceptionType() = runBlocking {
        val reading = ClientHealthPermissions(
            FakeHealthClientPort(fail = IllegalStateException("secret detail")),
        ).read(app)
        assertEquals(HealthAvailability.UNREADABLE, reading.availability)
        assertEquals("java.lang.IllegalStateException", reading.errorType)
        assertTrue(reading.grantedPermissions.isEmpty())
    }

    @Test
    fun requestIntentCarriesTheSixHealthPermissions() {
        val intent: Intent = ClientHealthPermissions(FakeHealthClientPort())
            .createRequestIntent(app, HEALTH_PERMISSIONS.values.toSet())
        val extra = intent.getStringArrayExtra("androidx.health.connect.client.extra.PERMISSIONS")
            ?: intent.extras?.keySet()?.firstNotNullOfOrNull { intent.getStringArrayExtra(it) }
        assertEquals(HEALTH_PERMISSIONS.values.sorted(), extra.orEmpty().toList().sorted())
    }

    /**
     * 本人の決定と仮決めの値を直書きで固定する（review/code.md R18）。試験の他の箇所は定数を参照して比べるので、
     * 定数そのものを書き換えても落ちない。印の鍵が変わると、更新した端末で答え済みの本人にもう一度自動で求める（D4）。
     */
    @Test
    fun `決めた値は直書きで固定されている`() {
        // 深掘り Q3（4 種類）・Q9（背景も求める）・D2 の `granted` の鍵と順
        assertEquals(
            linkedMapOf(
                "sleep" to "android.permission.health.READ_SLEEP",
                "heart_rate" to "android.permission.health.READ_HEART_RATE",
                "steps" to "android.permission.health.READ_STEPS",
                "exercise" to "android.permission.health.READ_EXERCISE",
                "history" to "android.permission.health.READ_HEALTH_DATA_HISTORY",
                "background" to "android.permission.health.READ_HEALTH_DATA_IN_BACKGROUND",
            ).toList(),
            HEALTH_PERMISSIONS.toList(),
        )
        assertEquals("asked_health_permissions", ASKED_HEALTH_PERMISSIONS) // D4
        assertEquals("健康データの権限", HEALTH_PERMISSION_ACTION_LABEL) // D8
        assertEquals(30_000L, LocationService.HEALTH_READ_TIMEOUT_MS) // review/code.md R3
    }
}
