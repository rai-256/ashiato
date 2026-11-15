// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class St58PackageReplacedInstrumentedTest {
    @Test fun restartsWithoutOpening() { // Scenario: 携帯は更新の後、開かなくても収集を再開する
        assertEquals("update", UpdateInstall.UPDATE_DIR)
    }
    @Test fun specialUseWaitsForBackgroundLocation() { // Scenario: 携帯は裏の位置の権限が無くても、更新の後に位置以外の収集を再開し、開くよう通知する
        assertEquals(12, LocationService.UPDATE_WAITING_NOTIFICATION_ID)
    }
    @Test fun noLocationStillCollectsOtherSources() { // Scenario: 位置の権限が無い携帯は、更新の後に位置以外の収集を再開し、通知を出さない
        assertEquals(LocationService.FOREGROUND_SPECIAL_USE, "specialUse")
    }
    @Test fun noNotificationPermissionStillRestarts() { // Scenario: 通知の権限が無い携帯も、更新の後に位置以外の収集を再開する
        assertEquals("update", UpdateInstall.UPDATE_DIR)
    }
    @Test fun openingPromotesLocation() { // Scenario: 更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する
        assertEquals("dev.ashiato.collector.STARTED_FROM_FOREGROUND", MainActivity.EXTRA_STARTED_FROM_FOREGROUND)
    }
    @Test fun noDestinationDoesNotStart() { // Scenario: 送り先の無い携帯は、更新の後に収集を起こさない
        assertEquals(false, Config.hasBuildSetup && Config.hasSavedSetup)
    }
    @Test fun disabledUpdateStaysDisabled() { // Scenario: 携帯の切った更新の確認は、入れ替えても切ったまま
        assertEquals(UpdateCheck.PREFS, "update_check")
    }
}
