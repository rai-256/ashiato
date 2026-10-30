// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.io.FileInputStream
import java.time.Instant
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用（人間が後から動画で見る）。ST50「送信契約の版」の、携帯に出る知らせを 1 本の流れで撮る:
 * サーバが ST50 より古い（`GET /contract` が 404）→ 送らずに携帯に残し「サーバを更新してください」の通知 →
 * あしあと。を開くと画面にも同じ 1 行 → サーバを更新した（読める版の答え）→ 通知は消え、開いても 1 行は出ずに閉じる。
 *
 * **サーバの古さは撮らない。** 録画用のサーバは ST50 の版を返すので、古いサーバの答えはここで作る ——
 * 答え（[ContractAnswer]）をアプリと同じ [ContractGate] → [ContractNotifier] に通し、画面の行も同じ置き場
 * （[ContractWaitStore]。アプリの SharedPreferences）から読ませる。通知の中身・消え方・画面の行はアプリのコードのまま。
 * 手元に残している日数は録画のために [DAYS] 日として渡す（入れたばかりの端末には古い未送信が無い）。
 * 送る・送らないの判定は計測テストと Robolectric（`St50SenderTest`）が持つ。
 *
 * 位置・通知・写真と動画の権限は先に許可し、健康データの権限の段と利用状況へのアクセスへ送る 1 度だけの印も先に済ませる
 * （導線は ST06・ST09・ST11 の録画が撮る）。使っていないアプリの一時停止からは外しておく（その 1 行は ST13 の録画が撮る）。
 * 開いた画面が常駐を起こすが、その送る契機は 5 分後なので動画の間には来ない。**録画のときだけ走る**（`-e recording 1`）。
 */
@RunWith(AndroidJUnit4::class)
class St50RecordingTest {
    @get:Rule
    val permissions: GrantPermissionRule = GrantPermissionRule.grant(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
        Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        Manifest.permission.POST_NOTIFICATIONS,
        *mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).toTypedArray(),
    )

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)
    private val waitPrefs get() = context.getSharedPreferences(ContractWaitStore.PREFS, Context.MODE_PRIVATE)

    @After
    fun cleanUp() {
        manager.cancel(ContractNotifier.CONTRACT_NOTIFICATION_ID)
        waitPrefs.edit().clear().commit()
    }

    @Test
    fun contractNoticeForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        shell("appops set ${context.packageName} AUTO_REVOKE_PERMISSIONS_IF_UNUSED ignore")
        context.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE).edit()
            .putBoolean(MainActivity.SENT_TO_USAGE_ACCESS, true).commit()
        skipHealthPermissionStep(context)
        waitPrefs.edit().clear().commit()
        val store = ContractWaitStore(waitPrefs)
        val gate = ContractGate(store)
        val notifier = ContractNotifier(context, store)
        device.pressHome()

        step("1. サーバが ST50 より古い（版を問い合わせる口が 404）→ 送らずに携帯に残し、更新を促す通知が出る")
        gate.observe(ContractAnswer.NoEndpoint, Instant.now(), emptyList())
        assertTrue("404 で更新を待つ状態に入らない", gate.waiting)
        notifier.update(gate.reasons, DAYS)
        val line = "$CONTRACT_UPDATE_HEADLINE（未送信 $DAYS 日分を携帯に残しています）"
        device.openNotification()
        assertNotNull("更新を促す通知が出ない", device.wait(Until.findObject(By.textContains(CONTRACT_UPDATE_HEADLINE)), TIMEOUT_MS))
        assertTrue("通知の本文が違う: $line", device.hasObject(By.text(line)))
        hold(6_000)
        device.pressBack()

        step("2. あしあと。を開く: 画面にも同じ 1 行（手元に残している日数つき）")
        launch()
        assertNotNull("画面に 1 行が出ない", device.wait(Until.findObject(By.text(line)), TIMEOUT_MS))
        hold(5_000)
        device.pressHome()

        step("3. サーバを更新した（版の答えが読める）→ 待つ状態を抜け、通知は消える")
        assertTrue("読める答えで待つ状態を抜けない", gate.observe(ContractAnswer.Version(CONTRACT_VERSION), Instant.now(), emptyList()))
        notifier.update(gate.reasons, DAYS)
        assertNull("抜けたのに画面の行の元が残っている", store.notice)
        device.openNotification()
        assertTrue("抜けたのに通知が残っている", device.wait(Until.gone(By.textContains(CONTRACT_UPDATE_HEADLINE)), TIMEOUT_MS))
        hold(4_000)
        device.pressBack()

        step("4. もう一度開く: 1 行は出ず、画面は閉じる（収集は常駐のまま）")
        launch()
        assertTrue(
            "更新を済ませたのに画面が閉じない",
            waitUntil { device.currentPackageName != context.packageName },
        )
        assertFalse("更新を済ませたのに 1 行が出ている", device.hasObject(By.text(line)))
        hold(3_000)
    }

    private fun launch() {
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    private fun shell(cmd: String): String =
        instrumentation.uiAutomation.executeShellCommand(cmd)
            .let { FileInputStream(it.fileDescriptor) }
            .use { it.readBytes().toString(Charsets.UTF_8) }

    private fun step(label: String) {
        instrumentation.runOnMainSync { Toast.makeText(context, label, Toast.LENGTH_LONG).show() }
        hold()
    }

    private fun hold(ms: Long = HOLD_MS) = Thread.sleep(ms)

    private fun waitUntil(timeoutMs: Long = TIMEOUT_MS, check: () -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (check()) return true
            Thread.sleep(250)
        }
        return false
    }

    private companion object {
        const val TIMEOUT_MS = 15_000L
        const val HOLD_MS = 2_500L

        /** 手元に残している日数（録画のために決めた値。入れたばかりの端末には古い未送信が無い） */
        const val DAYS = 3
    }
}
