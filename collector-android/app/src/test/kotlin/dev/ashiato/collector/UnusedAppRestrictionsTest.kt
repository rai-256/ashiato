// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Application
import android.content.Context
import android.content.Intent
import android.widget.Button
import android.widget.TextView
import androidx.core.content.UnusedAppRestrictionsConstants
import androidx.test.core.app.ApplicationProvider
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.shadows.ShadowAlertDialog

/** 状態を固定する入口。読み口は本番と同じ差し込み口で差し替える。 */
class UnusedAppRestrictionsActivity : MainActivity() {
    override fun unusedAppRestrictionsReader() = UnusedAppRestrictionsReader { state }

    // 健康データは関心の外（`HealthPermissionFlowTest`）
    override fun healthPermissions(): HealthPermissions = noHealthConnect()
    override fun runHealth(task: () -> Unit) = task()
    override fun healthRecordSink(): (IngestRequest) -> Boolean = { true }

    companion object {
        var state: UnusedAppRestrictionsState = UnusedAppRestrictionsState.EXEMPT
    }
}

@RunWith(RobolectricTestRunner::class)
class UnusedAppRestrictionsTest {
    private val app: Application get() = ApplicationProvider.getApplicationContext()

    @Before
    fun setUp() {
        shadowOf(app).grantPermissions(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
            // 入口が位置の後に写真と動画の権限も求める（ST09）。この試験の関心の外なので、どの版の分も許可済みにする
            Manifest.permission.READ_EXTERNAL_STORAGE,
            Manifest.permission.READ_MEDIA_IMAGES,
            Manifest.permission.READ_MEDIA_VIDEO,
            Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED,
            Manifest.permission.ACCESS_MEDIA_LOCATION,
        )
        // 利用状況へのアクセスは許されている（別の設定画面を開かせない）
        shadowOf(app.getSystemService(android.app.AppOpsManager::class.java)).setMode(
            android.app.AppOpsManager.OPSTR_GET_USAGE_STATS,
            android.os.Process.myUid(),
            app.packageName,
            android.app.AppOpsManager.MODE_ALLOWED,
        )
        // 印の無い状態（新しく入れた / 前の版から更新した）
        app.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE).edit().clear().commit()
    }

    private fun open(state: UnusedAppRestrictionsState) =
        Robolectric.buildActivity(UnusedAppRestrictionsActivity::class.java).also {
            UnusedAppRestrictionsActivity.state = state
        }.create().resume().also { last = it.get() }

    private fun texts(activity: android.app.Activity): List<String> {
        val out = mutableListOf<String>()
        fun walk(v: android.view.View) {
            if (v is TextView) out += v.text.toString()
            if (v is android.view.ViewGroup) for (i in 0 until v.childCount) walk(v.getChildAt(i))
        }
        walk(activity.window.decorView)
        return out
    }

    private var last: android.app.Activity? = null

    private fun settingsIntent(): Intent? = last?.let { shadowOf(it).nextStartedActivity }

    private fun dialogShown() = ShadowAlertDialog.getLatestAlertDialog()?.isShowing == true

    private fun asked() =
        app.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE)
            .getBoolean(UnusedAppRestrictionsAsked.KEY, false)

    // Scenario: 対象から外れていない端末で初めて開くと設定の画面へ案内される
    @Test
    fun `外れていない端末で初めて開くと案内が出て応じると設定の画面が開く`() {
        val controller = open(UnusedAppRestrictionsState.NOT_EXEMPT)

        assertTrue("案内が出ていない", dialogShown())
        assertNull("応じる前に設定が開いている", settingsIntent())

        ShadowAlertDialog.getLatestAlertDialog().getButton(android.app.AlertDialog.BUTTON_POSITIVE).performClick()

        shadowOf(android.os.Looper.getMainLooper()).idle()
        assertTrue("答えたのに印が立っていない", asked())
        val opened = settingsIntent()
        assertNotNull("設定の画面が開いていない", opened)
        assertEquals(
            manageUnusedAppRestrictionsIntent(controller.get()).action,
            opened!!.action,
        )
    }

    /** 答える前に画面が作り直されたら、印は立たず、次に開いたときにもう一度出る（/code-review R33）。 */
    @Test
    fun `答える前に画面が作り直されたら次に開いたときにもう一度案内が出る`() {
        val first = open(UnusedAppRestrictionsState.NOT_EXEMPT)
        assertTrue("案内が出ていない", dialogShown())
        first.pause().stop().destroy()
        shadowOf(android.os.Looper.getMainLooper()).idle()
        assertFalse("答えていないのに印が立っている", asked())
        ShadowAlertDialog.reset()

        open(UnusedAppRestrictionsState.NOT_EXEMPT)

        assertTrue("作り直した後に案内が出ない", dialogShown())
    }

    /** 設定の画面から戻る（`onResume` がもう一度通る）間、案内は重ねて出さない。 */
    @Test
    fun `戻ってきても案内は重ねて出ない`() {
        val controller = open(UnusedAppRestrictionsState.NOT_EXEMPT)
        val shown = ShadowAlertDialog.getLatestAlertDialog()
        controller.pause().resume()
        assertTrue("最初の案内が閉じた", shown.isShowing)
        assertTrue("同じ案内のまま", ShadowAlertDialog.getLatestAlertDialog() === shown)
    }

    // Scenario: 対象から外れている端末では案内されない
    @Test
    fun `外れている端末では案内されず画面にも出ない`() {
        val controller = open(UnusedAppRestrictionsState.EXEMPT)

        assertFalse("案内が出ている", dialogShown())
        assertNull(settingsIntent())
        assertFalse(texts(controller.get()).any { it.contains("一時停止") })
        assertTrue(controller.get().isFinishing)
        assertFalse("頼んでいないのに印が立っている", asked())
    }

    // Scenario: 2 回目に開いたときは設定の画面を自動では開かず、画面に出し続ける
    @Test
    fun `2 回目は自動では開かず画面に 1 行とボタンが出る`() {
        // 初めて開いて、案内には応じずに閉じる
        open(UnusedAppRestrictionsState.NOT_EXEMPT)
        ShadowAlertDialog.getLatestAlertDialog().getButton(android.app.AlertDialog.BUTTON_NEGATIVE).performClick()
        // 閉じた知らせ（印を立てる）は main の looper に積まれる
        shadowOf(android.os.Looper.getMainLooper()).idle()
        assertTrue("あとでと答えたのに印が立っていない", asked())
        shadowOf(last!!).clearNextStartedActivities()
        ShadowAlertDialog.reset()

        val second = open(UnusedAppRestrictionsState.NOT_EXEMPT)

        assertFalse("2 回目に案内が自動で出ている", dialogShown())
        assertNull("2 回目に設定が自動で開いている", settingsIntent())
        assertTrue(texts(second.get()).contains(MainActivity.NOT_EXEMPT_LINE))
        val button = second.get().window.decorView.let { root ->
            fun find(v: android.view.View): Button? =
                if (v is Button) {
                    v
                } else {
                    (v as? android.view.ViewGroup)?.let { g ->
                        (0 until g.childCount).firstNotNullOfOrNull { find(g.getChildAt(it)) }
                    }
                }
            find(root)
        }
        assertNotNull("設定を開くボタンが無い", button)
        assertEquals(MainActivity.OPEN_SETTINGS_LABEL, button!!.text.toString())
        val minDp = button.minimumHeight / second.get().resources.displayMetrics.density
        assertTrue("ボタンが 24 dp 未満: $minDp", minDp >= 24f)
        button.performClick()
        assertNotNull("ボタンで設定が開かない", settingsIntent())
    }

    // Scenario: 仕組みを持たない端末では頼まない
    @Test
    fun `仕組みを持たない端末では頼まず画面にも出ない`() {
        val controller = open(UnusedAppRestrictionsState.UNSUPPORTED)

        assertFalse(dialogShown())
        assertNull(settingsIntent())
        assertFalse(texts(controller.get()).any { it.contains("一時停止") })
        assertFalse("頼んでいないのに印が立っている", asked())
    }

    // Scenario: 除外の状態を読めない端末では頼まず読めなかったことを出す
    @Test
    fun `読めない端末では頼まず読めなかったことが出る`() {
        val controller = open(UnusedAppRestrictionsState.UNKNOWN)

        assertFalse("読めないのに案内が出ている", dialogShown())
        assertNull(settingsIntent())
        assertTrue(texts(controller.get()).contains(MainActivity.UNKNOWN_LINE))
        assertFalse("頼んでいないのに印が立っている", asked())
    }

    @Test
    fun `結果の値を 4 つの状態に写す`() {
        val expected = mapOf(
            UnusedAppRestrictionsConstants.DISABLED to UnusedAppRestrictionsState.EXEMPT,
            UnusedAppRestrictionsConstants.API_30_BACKPORT to UnusedAppRestrictionsState.NOT_EXEMPT,
            UnusedAppRestrictionsConstants.API_30 to UnusedAppRestrictionsState.NOT_EXEMPT,
            UnusedAppRestrictionsConstants.API_31 to UnusedAppRestrictionsState.NOT_EXEMPT,
            UnusedAppRestrictionsConstants.FEATURE_NOT_AVAILABLE to UnusedAppRestrictionsState.UNSUPPORTED,
            UnusedAppRestrictionsConstants.ERROR to UnusedAppRestrictionsState.UNKNOWN,
        )
        for ((status, state) in expected) {
            assertEquals("status=$status", state, mapUnusedAppRestrictionsStatus(status))
        }
    }

    private fun locationBeat(state: UnusedAppRestrictionsState, capability: Capability? = null): HeartbeatRequest {
        val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
        service.unusedAppRestrictions = state
        if (capability != null) service.capability = capability
        service.onStartCommand(Intent(), 0, 1)
        return service.heartbeatOutboxForTest.snapshot().last { it.logicalSource == LOGICAL_SOURCE }
    }

    // Scenario: 位置の生存信号の原文に除外の状態が載る
    @Test
    fun `位置の生存信号の原文に除外の状態が載る`() {
        val beat = locationBeat(UnusedAppRestrictionsState.NOT_EXEMPT)

        val raw = Json.parseToJsonElement(beat.raw).jsonObject
        assertEquals("not_exempt", raw.getValue("unused_app_restrictions").jsonPrimitive.content)
    }

    @Test
    fun `位置以外の生存信号の原文には載らない`() {
        val service = Robolectric.buildService(TestableLocationService::class.java, Intent()).create().get()
        service.unusedAppRestrictions = UnusedAppRestrictionsState.NOT_EXEMPT
        service.onStartCommand(Intent(), 0, 1)
        val others = service.heartbeatOutboxForTest.snapshot().filter { it.logicalSource != LOGICAL_SOURCE }
        assertTrue(others.isNotEmpty())
        for (b in others) {
            assertFalse(Json.parseToJsonElement(b.raw).jsonObject.containsKey("unused_app_restrictions"))
        }
    }

    // Scenario: 外れていなくても取得できる状態は変わらない
    @Test
    fun `外れていなくても取得できる状態は変わらない`() {
        val cap = Capability.of(permission = false, sensor = true, network = true)
        val exempt = locationBeat(UnusedAppRestrictionsState.EXEMPT, cap)
        val notExempt = locationBeat(UnusedAppRestrictionsState.NOT_EXEMPT, cap)

        assertEquals(exempt.capturable, notExempt.capturable)
        assertEquals(exempt.blockers, notExempt.blockers)
        assertEquals(listOf(Capability.PERMISSION), notExempt.blockers)
    }
}
