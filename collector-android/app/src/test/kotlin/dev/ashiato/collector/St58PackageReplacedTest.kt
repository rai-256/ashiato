// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.pm.ServiceInfo
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class St58PackageReplacedTest {
    @Test fun `印ありで両方の位置権限があれば location`() { // Scenario: 携帯は更新の後、開かなくても収集を再開する
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION, foregroundServiceTypeFor(true, false, true, true))
    }

    @Test fun `印ありで裏の位置権限が無ければ specialUse`() { // Scenario: 携帯は裏の位置の権限が無くても、更新の後に位置以外の収集を再開し、開くよう通知する
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, foregroundServiceTypeFor(true, false, true, false))
    }

    @Test fun `印ありで位置権限が無ければ specialUse`() { // Scenario: 位置の権限が無い携帯は、更新の後に位置以外の収集を再開し、通知を出さない
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, foregroundServiceTypeFor(true, false, false, false))
    }

    @Test fun `印なしは裏の権限に左右されず位置権限で location`() { // Scenario: 携帯は更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION, foregroundServiceTypeFor(false, false, true, false))
    }

    @Test fun `画面からの昇格は位置権限があれば location`() { // Scenario: 更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する
        assertEquals(ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION, foregroundServiceTypeFor(true, true, true, false))
    }

    @Test fun `specialUse の間は位置を権限不足として扱い collect しない`() { // Scenario: 携帯は裏の位置の権限が無くても、更新の後に位置以外の収集を再開し、開くよう通知する
        val capability = replacementLocationCapability(true, true, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE, true, false, Capability.of(true, true, true))
        assertFalse(capability.capturable)
        assertEquals(listOf(Capability.PERMISSION), capability.blockers)
    }

    @Test fun `裏の位置権限が許されると位置を取得できる`() { // Scenario: 更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する
        val actual = Capability.of(true, true, true)
        assertTrue(replacementLocationCapability(true, true, ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION, true, true, actual).capturable)
    }
}
