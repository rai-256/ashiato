// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Until
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.FixMethodOrder
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TestName
import org.junit.runner.RunWith
import org.junit.runners.MethodSorters

/**
 * **本物の Health Connect で、種類と履歴と背景をすべて許す**（ST11 Task 7.1 / design D4）。
 *
 * 始まりは「健康データの権限が未許可・印なし」（`@NeedsPristinePermissions`。`tools/android-emulator.sh` と CI の段が
 * `pm reset-permissions` で作る）。Health Connect の権限は**自分では外せない**（プロセスが死ぬ）ので、許した後の状態は
 * 戻せない —— このクラスの試験は名前の順に走り、前の試験が進めた画面を引き継ぐ（`@FixMethodOrder`）。
 * 引き継げないとき（前の試験が落ちた）は、各試験が自分で画面まで開き直す。
 *
 * 断る本はここに置かない（2 回断ると画面が出なくなる。P6）。断る側は `HealthPermissionHistoryDeniedInstrumentedTest` と
 * `HealthPermissionDeniedInstrumentedTest` が別の段で見る。
 */
@RunWith(AndroidJUnit4::class)
@NeedsPristinePermissions
@FixMethodOrder(MethodSorters.NAME_ASCENDING)
class HealthPermissionGrantInstrumentedTest {
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val screens get() = HealthConnectScreens(context)

    // 位置・通知・写真と動画は、この試験の関心の外なので許可済みにする（健康データだけを未許可のまま見る）
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(*HealthConnectScreens.earlierPermissions())

    @get:Rule
    val testName = TestName()

    /** 未許可から始まるのは最初の試験だけ（後ろは前の試験が許した状態を引き継ぐ）。 */
    @Before
    fun requirePristineHealthPermissions() {
        if (!testName.methodName.startsWith("t1_")) return
        assertEquals(
            "前提が作れていない。健康データの権限が残っている。" +
                "`adb shell am force-stop ${context.packageName} && adb shell pm reset-permissions` の後に、" +
                "class=dev.ashiato.collector.HealthPermissionGrantInstrumentedTest で単独に走らせる " +
                "（tools/android-emulator.sh と CI がその順で走らせる）",
            0,
            screens.grantedCount(),
        )
    }

    @After
    fun stopService() {
        screens.stopCollection()
    }

    // Scenario: 権限の画面からプライバシーポリシーを開くと説明の画面が出る
    @Test
    fun t1_privacyPolicyLinkOpensTheRationaleScreen() {
        screens.clearOutbox()
        screens.clearHealthMarks()
        screens.openKinds()
        // 種類の画面からでなければプライバシーポリシーの案内は出ない（追加のアクセスの画面には無い）
        assertTrue(
            "プライバシーポリシーの案内が権限の画面に無い",
            screens.device.wait(
                Until.hasObject(HealthConnectScreens.PRIVACY_POLICY),
                HealthConnectScreens.SCREEN_TIMEOUT_MS,
            ),
        )
        screens.click(HealthConnectScreens.PRIVACY_POLICY)

        // 読む種類と送る先と承認の設定と、いまは読まないこと（HEALTH_RATIONALE の 4 文）が出る
        for (sentence in HEALTH_RATIONALE.split("。").filter { it.isNotBlank() }) {
            assertTrue(
                "説明の画面に「$sentence」が出ない（前面: ${screens.device.currentPackageName}）",
                screens.device.wait(
                    Until.hasObject(By.textContains(sentence)),
                    HealthConnectScreens.SCREEN_TIMEOUT_MS,
                ),
            )
        }
        // 戻ると権限の画面（種類）に戻る。ここで断らない（答えずに残し、次の試験が続ける）
        screens.device.pressBack()
        screens.waitForKinds()
    }

    // Scenario: 初回起動で健康データの権限の画面が出る
    @Test
    fun t2_firstLaunchShowsTheHealthPermissionScreenWithTheFourKinds() {
        screens.openKinds()
        assertTrue("4 つの種類（睡眠・心拍・歩数・運動）が並ばない", screens.kindsVisible() || screens.additionalAccessVisible())
        // 種類を許すと、続く画面に履歴の読み取りと背景での読み取りが並ぶ
        screens.allowKindsAndContinue()
        assertTrue(screens.device.hasObject(By.text(HealthConnectScreens.HISTORY_LABEL)))
        assertTrue(screens.device.hasObject(By.text(HealthConnectScreens.BACKGROUND_LABEL)))
    }

    // Scenario: 種類と履歴と背景をすべて許すとすべて許された状態になる
    @Test
    fun t3_allowingKindsHistoryAndBackgroundGrantsAllSix() {
        screens.allowKindsAndContinue()
        screens.finishAdditionalAccess(history = true, background = true)

        // 6 つ（睡眠・心拍・歩数・運動・履歴・背景）を `checkSelfPermission` で 1 つずつ読む
        assertEquals(
            "すべて許された状態にならない",
            HEALTH_PERMISSIONS.size,
            screens.waitForGrantedCount(HEALTH_PERMISSIONS.size),
        )
        for (key in HEALTH_PERMISSIONS.keys) assertTrue("$key が許されていない", screens.granted(key))
        // 求めた結果が返れば収集は始まる（前景サービスの通知が出る）
        assertTrue("許した後に収集が始まらない（前景サービスの通知が出ない）", screens.waitForCollectionStarted())
    }

    // Scenario: 常駐の通知から健康データの権限の画面が開く
    @Test
    fun t4_theOngoingNotificationOpensTheHealthPermissionScreen() {
        // Android 14 以降だけ収集アプリの権限の画面が開く（13 以前は Health Connect のアプリの画面。spec は 14 以降に限る）
        assumeTrue(Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
        context.startActivity(
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        )
        assertTrue("収集が動いていない（前景サービスの通知が出ない）", screens.waitForCollectionStarted())

        screens.device.openNotification()
        screens.click(By.text(HEALTH_PERMISSION_ACTION_LABEL))

        // 収集アプリの権限の画面（Health Connect）が開く。許した 4 つの種類が並ぶ
        val deadline = System.currentTimeMillis() + HealthConnectScreens.SCREEN_TIMEOUT_MS
        while (System.currentTimeMillis() < deadline && !screens.kindsVisible()) {
            Thread.sleep(HealthConnectScreens.POLL_MS)
        }
        assertTrue(
            "常駐の通知の「$HEALTH_PERMISSION_ACTION_LABEL」を押しても権限の画面が開かない（前面: ${screens.device.currentPackageName}）",
            screens.kindsVisible() && screens.device.currentPackageName.contains("healthconnect"),
        )
        screens.device.pressBack()
    }

    // Scenario: 許すと許した状態と観た時刻の記録が S-01 に格納される
    //
    // 端末の側の半分: 許した後に未送信に積まれた `c01-health-permission` の `granted` が全部 `true` で、観た時刻を持つ。
    // S-01 に格納されるところ（サーバ側）は縦串（`tools/smoke.d/`）が見る。
    @Test
    fun t5_theRecordAfterGrantingIsQueuedWithAllGrantedAndATime() {
        val deadline = System.currentTimeMillis() + HealthConnectScreens.SETTLE_MS
        var latest: IngestRequest? = null
        while (System.currentTimeMillis() < deadline) {
            latest = screens.healthRecords().lastOrNull()
            if (latest != null && screens.grantedFlags(latest).values.all { it }) break
            Thread.sleep(HealthConnectScreens.POLL_MS)
        }
        assertTrue("許した後の記録が未送信に積まれていない", latest != null)
        val flags = screens.grantedFlags(latest!!)
        assertEquals("granted の鍵が 6 つでない: $flags", HEALTH_PERMISSIONS.keys.toList(), flags.keys.toList())
        assertFalse("許したのに granted に false が残る: $flags", flags.values.any { !it })
        assertTrue("観た時刻（event_time）が無い", latest.eventTime.isNotBlank())
        assertEquals("c01-health-permission", latest.logicalSource)
    }
}
