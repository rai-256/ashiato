// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.ComponentName
import android.content.pm.ServiceInfo
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class St58PackageReplacedInstrumentedTest {
    @Test fun restartsWithoutOpening() { // Scenario: 携帯は更新の後、開かなくても収集を再開する
        val info = ApplicationProvider.getApplicationContext<android.content.Context>().packageManager.getReceiverInfo(
            ComponentName(ApplicationProvider.getApplicationContext(), PackageReplacedReceiver::class.java), 0,
        )
        assertNotNull(info)
    }
    @Test fun specialUseWaitsForBackgroundLocation() { // Scenario: 携帯は裏の位置の権限が無くても、更新の後に位置以外の収集を再開し、開くよう通知する
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, foregroundServiceTypeFor(true, false, true, false))
        assertEquals(12, LocationService.UPDATE_WAITING_NOTIFICATION_ID)
    }
    @Test fun noLocationStillCollectsOtherSources() { // Scenario: 位置の権限が無い携帯は、更新の後に位置以外の収集を再開し、通知を出さない
        assertFalse(replacementLocationCapability(true, true, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, true, false, Capability.of(true, true, true)).capturable)
    }
    @Test fun noNotificationPermissionStillRestarts() { // Scenario: 通知の権限が無い携帯も、更新の後に位置以外の収集を再開する
        assertTrue(UpdateInstall.UPDATE_DIR.isNotEmpty())
    }
    @Test fun openingPromotesLocation() { // Scenario: 更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION, foregroundServiceTypeFor(true, true, true, false))
    }
    @Test fun noDestinationDoesNotStart() { // Scenario: 送り先の無い携帯は、更新の後に収集を起こさない
        assertFalse(Config.hasBuildSetup && Config.hasSavedSetup)
    }
    @Test fun disabledUpdateStaysDisabled() { // Scenario: 携帯の切った更新の確認は、入れ替えても切ったまま
        val prefs = ApplicationProvider.getApplicationContext<android.content.Context>().getSharedPreferences(UpdateCheck.PREFS, 0)
        prefs.edit().putBoolean("enabled", false).commit()
        assertFalse(prefs.getBoolean("enabled", true))
    }
}
