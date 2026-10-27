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

    /**
     * 前の試験が残した画面を片付ける。試験の実行器は試験ごとに `MainActivity` を閉じる（`MonitoringInstr: Finishing activity`）ので、
     * 前の試験が開いた Health Connect の画面は結果を返す先が無いまま残る —— 続きから押しても結果は戻らない。
     * 開いたままの通知の欄（常駐の通知の試験）も、次の段の画面を覆う（前面が `com.android.systemui` になる。2026-10-25 実測）。
     */
    fun reset() {
        device.executeShellCommand("cmd statusbar collapse")
        device.executeShellCommand("am force-stop $HEALTH_CONNECT_CONTROLLER")
        device.pressHome()
        device.waitForIdle()
    }

    /**
     * 種類の画面の「privacy policy」の案内を押す。案内は文の中の 1 語だけが押せる（文の要素そのものは押せない。
     * 中央を押しても何も起きない。2026-10-25 実測）ので、最後の行を左右に押していき、説明の画面が出たところで止める。
     */
    fun openPrivacyPolicy() {
        val line = device.wait(Until.findObject(PRIVACY_POLICY), SCREEN_TIMEOUT_MS)
        assertTrue("プライバシーポリシーの案内が権限の画面に無い（前面: ${device.currentPackageName}）", line != null)
        val b = line!!.visibleBounds
        val y = b.bottom - b.height() / 4
        for (f in listOf(0.6, 0.45, 0.75, 0.3, 0.9, 0.15)) {
            device.click((b.left + b.width() * f).toInt(), y)
            if (device.wait(Until.hasObject(By.pkg(context.packageName)), 2_000)) return
        }
        throw AssertionError("プライバシーポリシーの案内を押しても説明の画面が出ない（前面: ${device.currentPackageName}）")
    }

    /**
     * 通知のトレイを開いて、常駐の通知の操作 [label] を押す。他の通知（同じアプリの知らせ）があると常駐の通知
     * （重要度 低）は畳まれて操作が見えないので、常駐の通知のほかは消し、見えなければ常駐の通知の展開ボタンを押す。
     */
    fun clickOngoingAction(label: String) {
        val manager = context.getSystemService(NotificationManager::class.java)
        for (n in manager.activeNotifications) {
            if (n.id != LocationService.NOTIFICATION_ID) manager.cancel(n.tag, n.id)
        }
        device.openNotification()
        val selector = By.text(label)
        if (!device.wait(Until.hasObject(selector), SHADE_TIMEOUT_MS)) {
            // 常駐の通知は「Silent」の節で畳まれる（上に OS の知らせ「画面ロックを設定」などが展開されていると）。その行の展開ボタンを押す
            device.findObject(
                By.res("com.android.systemui", "expandableNotificationRow").hasDescendant(By.text(ONGOING_TITLE)),
            )?.findObject(By.res("android", "expand_button"))?.click()
        }
        val target = device.wait(Until.findObject(selector), SHADE_TIMEOUT_MS)
        val posted = manager.activeNotifications.joinToString { n ->
            "${n.id}:${n.notification.actions?.joinToString("/") { it.title } ?: "-"}"
        }
        assertTrue("常駐の通知の操作「$label」が見つからない（出ている通知: $posted）", target != null)
        target!!.click()
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
        return SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, LocationService.UNREADABLE), {
        }).readAll()
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
        const val SHADE_TIMEOUT_MS = 5_000L

        /** 常駐の通知の題（`LocationService` の `setContentTitle`）。 */
        const val ONGOING_TITLE = "あしあと。"
        const val POLL_MS = 250L

        /** Health Connect の権限の画面を出すアプリ（Android 14 以降は module。P14）。 */
        const val HEALTH_CONNECT_CONTROLLER = "com.google.android.healthconnect.controller"

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

        /** Health Connect の設定のトップの「アプリの権限」と、その一覧の中の収集アプリ（深掘り Q11 の道）。 */
        val APP_PERMISSIONS: BySelector = By.text(Pattern.compile("(?i)App permissions|アプリの権限"))
        val APP_LABEL: BySelector = By.text("あしあと。")
        val PRIVACY_POLICY: BySelector =
            By.text(Pattern.compile("(?is).*privacy policy.*|.*プライバシー ?ポリシー.*"))
    }
}
