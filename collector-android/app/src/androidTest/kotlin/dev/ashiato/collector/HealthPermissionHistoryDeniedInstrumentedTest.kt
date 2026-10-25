// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.rule.GrantPermissionRule
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * **本物の Health Connect で、種類を許して履歴の読み取りだけを断る**（ST11 Task 7.1 / design D4）。
 *
 * 始まりは「健康データの権限が未許可・印なし」。`HealthPermissionGrantInstrumentedTest` が全部許した状態を
 * `pm reset-permissions` で戻した別の段で走る（`tools/android-emulator.sh` と CI の段）。断る本は 1 段に 1 つまで（P6）。
 */
@RunWith(AndroidJUnit4::class)
@NeedsPristinePermissions
class HealthPermissionHistoryDeniedInstrumentedTest {
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val screens get() = HealthConnectScreens(context)

    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(*HealthConnectScreens.earlierPermissions())

    @Before
    fun requirePristineHealthPermissions() {
        assertEquals(
            "前提が作れていない。健康データの権限が残っている。" +
                "`adb shell am force-stop ${context.packageName} && adb shell pm reset-permissions` の後に、" +
                "class=dev.ashiato.collector.HealthPermissionHistoryDeniedInstrumentedTest で単独に走らせる",
            0,
            screens.grantedCount(),
        )
        // 前の段が残した印（収集アプリ自身の置き場。自分のものなので消せる）と未送信を持ち込まない
        screens.stopCollection()
        screens.clearHealthMarks()
        screens.clearOutbox()
    }

    @After
    fun stopService() {
        screens.stopCollection()
    }

    // Scenario: 種類を許して履歴の読み取りだけを断っても収集は始まる
    @Test
    fun allowingKindsButNotHistoryStillStartsCollection() {
        screens.allowKindsAndContinue()
        // 続く画面で背景の読み取りはオンにし、履歴の読み取りだけをオフのまま許可する
        screens.finishAdditionalAccess(history = false, background = true)

        // 4 つの種類と背景は許され、履歴の読み取りだけが許されていない
        assertEquals("履歴だけが断られた状態（5 つ許可）にならない", 5, screens.waitForGrantedCount(5))
        for (key in listOf("sleep", "heart_rate", "steps", "exercise", "background")) {
            assertTrue("$key が許されていない", screens.granted(key))
        }
        assertFalse("履歴の読み取りをオフにしたのに許されている", screens.granted("history"))
        // 収集は始まる（前景サービスの通知が出る）
        assertTrue("履歴を断ったら収集が始まらない（前景サービスの通知が出ない）", screens.waitForCollectionStarted())
    }
}
