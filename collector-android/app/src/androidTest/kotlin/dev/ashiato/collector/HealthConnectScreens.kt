// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.BySelector
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.UiObject2
import androidx.test.uiautomator.Until
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertTrue
import java.io.File
import java.util.regex.Pattern

/**
 * 本物の Health Connect の権限の画面を UI Automator で読んで押す、`HealthPermission*InstrumentedTest` 共通の道具（ST11 Task 7）。
 *
 * 画面の文字は premises.md の P7・P14（Android 15 のエミュレータの実物）が出所。端末の言語は英語を前提にし、
 * 文言が違う端末のために日本語も許す。**見つからなければ落とす**（飛ばすと「押していないのに緑」になる）。
 *
 * **自分の権限は自分で外さない**（計測テストのプロセスが死ぬ）。始まりの状態は `tools/android-emulator.sh` と CI の段が作る
 * （`@NeedsPristinePermissions`）。2 回断ると画面が出なくなる（P6）ので、1 段に断る本は 1 つまで。
 */
internal class HealthConnectScreens(private val context: Context) {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    val device: UiDevice get() = UiDevice.getInstance(instrumentation)

    /** 種類の画面（P14）。4 つの種類の名前が並ぶ。 */
    fun kindsVisible(): Boolean = KIND_LABELS.all { device.hasObject(By.text(it)) }

    /** 続く「追加のアクセス」の画面（P14）。履歴と背景のトグルが並ぶ。 */
    fun additionalAccessVisible(): Boolean =
        device.hasObject(By.text(HISTORY_LABEL)) && device.hasObject(By.text(BACKGROUND_LABEL))

    /** 種類の画面を出す。出ていればそのまま、出ていなければ収集アプリを開く（「はじめに」が出たら「Get started」を押す。P7）。 */
    fun openKinds() {
        if (kindsVisible() || additionalAccessVisible()) return
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        waitForKinds()
    }

    fun waitForKinds() {
        val deadline = System.currentTimeMillis() + SCREEN_TIMEOUT_MS
        while (System.currentTimeMillis() < deadline) {
            device.findObject(GET_STARTED)?.click()
            if (kindsVisible() || additionalAccessVisible()) return
            Thread.sleep(POLL_MS)
        }
        throw AssertionError("健康データの権限の画面（種類）が出ない（前面: ${device.currentPackageName}）")
    }

    /** 種類をすべて許して、続く画面（履歴と背景）まで進む。 */
    fun allowKindsAndContinue() {
        openKinds()
        if (additionalAccessVisible()) return
        click(ALLOW_ALL)
        click(ALLOW)
        val deadline = System.currentTimeMillis() + SCREEN_TIMEOUT_MS
        while (System.currentTimeMillis() < deadline && !additionalAccessVisible()) Thread.sleep(POLL_MS)
        assertTrue("種類を許しても履歴と背景の画面が続かない（前面: ${device.currentPackageName}）", additionalAccessVisible())
    }

    /** 履歴と背景のトグルをオンにするかを選んで「許可」を押す。 */
    fun finishAdditionalAccess(history: Boolean, background: Boolean) {
        assertTrue("履歴と背景の画面が出ていない（前面: ${device.currentPackageName}）", additionalAccessVisible())
        if (history) click(By.text(HISTORY_LABEL))
        if (background) click(By.text(BACKGROUND_LABEL))
        click(ALLOW)
    }

    fun click(selector: BySelector, timeoutMs: Long = SCREEN_TIMEOUT_MS) {
        val target: UiObject2? = device.wait(Until.findObject(selector), timeoutMs)
        assertTrue("押すものが見つからない: $selector（前面: ${device.currentPackageName}）", target != null)
        target!!.click()
    }

    /** 6 つの権限（design D5）のうち許されている数。`checkSelfPermission` で読む。 */
    fun grantedCount(): Int = HEALTH_PERMISSIONS.keys.count { granted(it) }

    fun granted(key: String): Boolean =
        context.checkSelfPermission(HEALTH_PERMISSIONS.getValue(key)) == PackageManager.PERMISSION_GRANTED

    /** 許された数が [expected] になるまで待つ（結果が返ってから権限の状態が見えるまでの間）。 */
    fun waitForGrantedCount(expected: Int): Int {
        val deadline = System.currentTimeMillis() + SETTLE_MS
        while (System.currentTimeMillis() < deadline && grantedCount() != expected) Thread.sleep(POLL_MS)
        return grantedCount()
    }

    /** 収集が始まった（前景サービスの通知が出ている）。 */
    fun waitForCollectionStarted(): Boolean {
        val nm = context.getSystemService(NotificationManager::class.java)
        val deadline = System.currentTimeMillis() + SETTLE_MS
        while (System.currentTimeMillis() < deadline) {
            if (nm.activeNotifications.any { it.id == LocationService.NOTIFICATION_ID }) return true
            Thread.sleep(POLL_MS)
        }
        return false
    }

    private fun outboxDir() = File(context.filesDir, "outbox")

    /** 未送信に積まれた `c01-health-permission` の記録（古い順）。 */
    fun healthRecords(): List<IngestRequest> {
        val dir = outboxDir()
        return SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, LocationService.UNREADABLE), {}).readAll()
            .filter { it.logicalSource == HEALTH_LOGICAL_SOURCE }
    }

    fun grantedFlags(record: IngestRequest): Map<String, Boolean> =
        record.payload.getValue("granted").jsonObject.mapValues { it.value.jsonPrimitive.boolean }

    /** 前の段が残した未送信を持ち込まない（収集のサービスが止まっているときだけ呼ぶ）。 */
    fun clearOutbox() {
        outboxDir().deleteRecursively()
    }

    /** 「入れ直した」と同じ状態にする: 収集アプリ自身の健康データの印と最後の状態を消す。 */
    fun clearHealthMarks() {
        context.getSharedPreferences(HEALTH_PREFS, Context.MODE_PRIVATE).edit().clear().commit()
    }

    fun stopCollection() {
        context.stopService(Intent(context, LocationService::class.java))
    }

    companion object {
        const val SCREEN_TIMEOUT_MS = 20_000L
        const val SETTLE_MS = 20_000L
        const val POLL_MS = 250L

        /** 位置・通知・写真と動画は、この試験の関心の外なので許可済みにする規則に渡す一覧。 */
        fun earlierPermissions(): Array<String> = arrayOf(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_COARSE_LOCATION,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
            *mediaPermissionsToAsk(Build.VERSION.SDK_INT).toTypedArray(),
        )

        // premises.md P14: 種類の画面（Allow all / 種類の名前）→ 追加のアクセスの画面（2 つのトグル）。許可を押す先は「Allow」。
        // By.text(Pattern) は文字列の全体に一致させるので、「Allow」は「Allow all」「Don't allow」に当たらない
        val KIND_LABELS: List<Pattern> = listOf("Sleep", "Heart rate", "Steps", "Exercise")
            .map { Pattern.compile("(?i)$it") }
        val HISTORY_LABEL: Pattern = Pattern.compile("(?i)Access past data|過去のデータ.*")
        val BACKGROUND_LABEL: Pattern = Pattern.compile("(?i)Access data in the background|バックグラウンド.*")
        val ALLOW_ALL: BySelector = By.text(Pattern.compile("(?i)Allow all|すべて許可"))
        val ALLOW: BySelector = By.text(Pattern.compile("(?i)Allow|許可"))
        val DONT_ALLOW: BySelector = By.text(Pattern.compile("(?i)Don.t allow|許可しない"))
        val GET_STARTED: BySelector = By.text(Pattern.compile("(?i)Get started|開始"))
        val PRIVACY_POLICY: BySelector =
            By.text(Pattern.compile("(?is).*privacy policy.*|.*プライバシー ?ポリシー.*"))
    }
}
