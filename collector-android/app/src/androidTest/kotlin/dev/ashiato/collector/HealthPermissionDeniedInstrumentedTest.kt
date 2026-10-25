// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import android.content.Intent
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
 * **本物の Health Connect で、種類を 1 つも許さずに断る**（ST11 Task 7.1 / design D4）。**最後の段に置く。**
 *
 * 断った回数は Health Connect が数える（2 回断ると画面が出なくなる。P6）。`pm reset-permissions` が回数まで戻すかは
 * 確かめていない（Risks）ので、**断る試験はこのクラスの 1 本だけ**にし、最後の段に置いて後ろの試験へ回数を持ち越さない。
 * その 1 本の中で**2 回断る**: 1 回目は初回起動の画面で断り、続けて収集アプリ自身の健康データの印を消して
 * （入れ直した端末と同じ。本物を入れ直すと端末識別子が変わる）開き直し、もう一度求められた画面でも断る。
 * 2 回断った後は画面がもう出ない（P6）ので、このクラスの後に断る試験を置かない。
 */
@RunWith(AndroidJUnit4::class)
@NeedsPristinePermissions
class HealthPermissionDeniedInstrumentedTest {
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val screens get() = HealthConnectScreens(context)

    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(*HealthConnectScreens.earlierPermissions())

    @Before
    fun requirePristineHealthPermissions() {
        screens.reset()
        assertEquals(
            "前提が作れていない。健康データの権限が残っている。" +
                "`adb shell am force-stop ${context.packageName} && adb shell pm reset-permissions` の後に、" +
                "class=dev.ashiato.collector.HealthPermissionDeniedInstrumentedTest で単独に走らせる",
            0,
            screens.grantedCount(),
        )
        screens.stopCollection()
        screens.clearHealthMarks()
        screens.clearOutbox()
    }

    @After
    fun stopService() {
        screens.stopCollection()
    }

    // Scenario: 健康データの権限をすべて断っても収集は始まる
    // Scenario: 入れ直すと初回起動でもう一度求められる
    @Test
    fun decliningEveryHealthPermissionStillStartsCollectionAndReinstallAsksAgain() {
        screens.openKinds()
        screens.click(HealthConnectScreens.DONT_ALLOW)

        // アプリは落ちず、収集が始まる（前景サービスの通知が出る）。種類は 1 つも許されていない
        assertTrue("断ったら収集が始まらない（前景サービスの通知が出ない）", screens.waitForCollectionStarted())
        assertEquals("断ったのに許された権限がある", 0, screens.grantedCount())
        val crash = screens.device.executeShellCommand("logcat -d -b crash -t 400")
        assertFalse("crash ログにアプリが出ている:\n$crash", crash.contains(context.packageName))

        // ---- 入れ直し: 印を消して開き直すと、もう一度求められる（本物を入れ直すと端末識別子が変わるので、印だけを消す）
        screens.stopCollection()
        screens.clearHealthMarks()
        // 前の MainActivity を残さない（残っていると前へ出るだけで、健康データの段を越えたままになる。2026-10-25 実測）。
        // 入れ直した端末と同じく、新しい MainActivity が最初から流れを通る
        context.startActivity(
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK),
        )
        // `pm reset-permissions` が断った回数を戻していなければ、ここで画面が出ない（Risks）。出なければ落とし、
        // 結果を PR の本文に書いて、この Scenario は Task 5.1 の `HealthPermissionFlowTest` だけで閉じる
        screens.waitForKinds()
        // 2 回目も断る（3 回目の求めはもう出ないので、ここで終える）
        screens.click(HealthConnectScreens.DONT_ALLOW)
        assertTrue("2 回目に断った後に収集が始まらない", screens.waitForCollectionStarted())
    }
}
