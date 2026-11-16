// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.Manifest
import android.app.Application
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Looper
import android.widget.TextView
import androidx.test.core.app.ApplicationProvider
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.android.controller.ActivityController
import java.io.File

private class St58MainActivity : MainActivity() {
    override fun shouldOfferInitialSetup() = false
    override fun unusedAppRestrictionsReader() = UnusedAppRestrictionsReader { UnusedAppRestrictionsState.EXEMPT }
    override fun healthPermissions(): HealthPermissions = noHealthConnect()
    override fun runHealth(task: () -> Unit) = task()
    override fun healthRecordSink(): (IngestRequest) -> Boolean = { true }
    override fun healthPendingSlot() = HealthPendingSlot()
}

@RunWith(RobolectricTestRunner::class)
class St58UpdateInstallTest {
    private val hexDigest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

    @get:Rule val temporaryFolder = TemporaryFolder()
    private val app = ApplicationProvider.getApplicationContext<Application>()

    @Before fun reset() {
        clearPrefs()
        shadowOf(app).grantPermissions(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
            Manifest.permission.POST_NOTIFICATIONS,
            Manifest.permission.READ_EXTERNAL_STORAGE,
            Manifest.permission.READ_MEDIA_IMAGES,
            Manifest.permission.READ_MEDIA_VIDEO,
            Manifest.permission.READ_MEDIA_VISUAL_USER_SELECTED,
            Manifest.permission.ACCESS_MEDIA_LOCATION,
        )
        shadowOf(app.getSystemService(android.app.AppOpsManager::class.java)).setMode(
            android.app.AppOpsManager.OPSTR_GET_USAGE_STATS,
            android.os.Process.myUid(),
            app.packageName,
            android.app.AppOpsManager.MODE_ALLOWED,
        )
    }

    @After fun cleanup() = clearPrefs()

    private fun clearPrefs() {
        app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().commit()
        app.getSharedPreferences(MainActivity.PREFS, 0).edit().clear().commit()
        app.getSharedPreferences(Config.SETUP_PREFS, 0).edit().clear().commit()
        UpdateInstall.failurePreferences(app).edit().clear().commit()
        Config.clearOverrideForTest()
    }

    private fun configured() = Config.overrideForTest("http://127.0.0.1", "token", Config.DEFAULT_USER_ID)

    private fun open(): ActivityController<St58MainActivity> =
        Robolectric.buildActivity(
            St58MainActivity::class.java,
            Intent(app, St58MainActivity::class.java).setAction(Intent.ACTION_MAIN)
        )
            .create().start().resume()

    /** 配布元の一覧の 1 件（版 0.4.1・資産 1 つ）。[digest] は JSON の値のまま（`null` か引用符付きの文字列）。 */
    private fun release(name: String, path: String, digest: String): String =
        """[{"tag_name":"v0.4.1","draft":false,"assets":[{"name":"$name",""" +
            """"browser_download_url":"https://github.com/rai-256/ashiato/releases/download/$path",""" +
            """"digest":$digest}]}]"""

    private fun texts(activity: android.app.Activity): List<String> {
        val result = mutableListOf<String>()
        fun visit(view: android.view.View) {
            if (view is TextView) result += view.text.toString()
            if (view is android.view.ViewGroup) for (i in 0 until view.childCount) visit(view.getChildAt(i))
        }
        visit(activity.window.decorView)
        return result
    }

    @Test
    fun `指紋の合う配布物だけを選ぶ`() {
        // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val body = release("ashiato-collector-0.4.1.apk", "v0.4.1/a.apk", "\"sha256:$hexDigest\"")
        val asset = UpdateInstall.chooseAsset("0.4.0", body)
        assertEquals("0.4.1", asset?.version)
        assertEquals("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", asset?.sha256)
    }

    @Test
    fun `共有の更新ベクターを Android の選択規則で全件通す`() {
        // Scenario: 指紋の無い配布物は入れない
        // `docs/contracts/update-apply-vectors.json` is shared with Rust: do
        // not duplicate its malformed-input matrix in the Android tests.
        val vectors = javaClass.classLoader
            ?.getResourceAsStream("update-apply-vectors.json")
            ?.bufferedReader()
            ?.use { it.readText() }
        assertTrue("共有ベクターが無い", vectors != null)
        val cases = Json.parseToJsonElement(vectors!!).jsonObject["cases"]!!.jsonArray
        cases.forEach { element ->
            val case = element.jsonObject
            case["redirect"]?.jsonPrimitive?.content?.let { redirect ->
                assertEquals(
                    case["allowed_redirect"]!!.jsonPrimitive.content.toBoolean(),
                    UpdateInstall.isAllowedDownloadUrl(redirect)
                )
                return@forEach
            }
            if (case["component"]!!.jsonPrimitive.content != "android") return@forEach
            val tag = case["tag_name"]!!.jsonPrimitive.content
            val asset = case["asset"]!!.jsonObject
            val release = """[{"tag_name":"$tag","draft":false,"assets":[$asset]}]"""
            val installed = if (tag == "nightly") "0.4.0-dev.20261109" else "0.4.0"
            val selected = UpdateInstall.chooseAsset(installed, release) != null
            assertEquals(
                "共有ベクター: ${case["name"]!!.jsonPrimitive.content}",
                case["selected"]!!.jsonPrimitive.content.toBoolean(),
                selected
            )
        }
    }

    @Test
    fun `開発版は開発版の APK 接頭辞だけを選ぶ`() {
        // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        val body = """[{"tag_name":"nightly","draft":false,"assets":[
            {"name":"ashiato-collector-dev-0.4.1-dev.20261110.apk","browser_download_url":"https://github.com/rai-256/ashiato/releases/download/nightly/a.apk","digest":"sha256:$digest"},
            {"name":"ashiato-collector-0.4.1-dev.20261110.apk","browser_download_url":"https://github.com/rai-256/ashiato/releases/download/nightly/b.apk","digest":"sha256:$digest"}
        ]}]"""
        assertEquals(
            "ashiato-collector-dev-0.4.1-dev.20261110.apk",
            UpdateInstall.chooseAsset("0.4.0-dev.20261109", body)?.name,
        )
    }

    @Test
    fun `指紋が無い配布物は選ばない`() {
        // Scenario: 指紋の無い配布物は入れない
        val body = release("ashiato-collector-0.4.1.apk", "v0.4.1/a.apk", "null")
        assertNull(UpdateInstall.chooseAsset("0.4.0", body))
    }

    @Test
    fun `配布物の宛先は GitHub のリリースだけ`() {
        // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(
            UpdateInstall.isAllowedDownloadUrl("https://github.com/rai-256/ashiato/releases/download/v0.4.1/a.apk")
        )
        assertTrue(UpdateInstall.isAllowedDownloadUrl("https://release-assets.githubusercontent.com/a"))
        assertTrue(!UpdateInstall.isAllowedDownloadUrl("https://example.com/a.apk"))
    }

    @Test fun `通知の開始印を持つ`() { // Scenario: 携帯の更新の通知を押すと、落とし始める
        val intent = UpdateInstall.updateStartIntent(ApplicationProvider.getApplicationContext())
        assertEquals(MainActivity::class.java.name, intent.component?.className)
        assertTrue(intent.getBooleanExtra(UpdateInstall.EXTRA_START, false))
    }

    @Test fun `入っている版より古い版は選ばない`() {
        assertNull(UpdateInstall.chooseAsset("0.5.0", "[]"))
    }

    @Test fun `APK の SHA256 を実体で照合する`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val file = temporaryFolder.newFile("update.apk").apply { writeText("apk bytes") }
        val digest = "${java.security.MessageDigest.getInstance(
            "SHA-256"
        ).digest("apk bytes".toByteArray()).joinToString("") { "%02x".format(it) }}"
        assertTrue(UpdateInstall.verifyDigest(file, digest))
        assertFalse(UpdateInstall.verifyDigest(file, digest.replaceFirst('a', 'b')))
    }

    @Test fun `許可が必要な導線を持つ`() { // Scenario: 携帯は提供元不明のアプリが許されていなければ、許可の画面を開く
        val intent = UpdateInstall.unknownSourcesIntent(ApplicationProvider.getApplicationContext())
        assertEquals(android.provider.Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, intent.action)
        assertEquals(
            "package:${ApplicationProvider.getApplicationContext<Application>().packageName}",
            intent.dataString
        )
        assertEquals("許可した後に、もう一度押してください", UpdateInstall.UNKNOWN_SOURCES_LINE)
    }

    @Test fun `指紋が違えば、開いている画面の行にその場で失敗の文が出る`() { // Scenario: 携帯は指紋の合わない APK でインストール画面を開かない
        configured()
        app.getSharedPreferences(
            UpdateCheck.PREFS,
            0
        ).edit().putBoolean("intro_shown", true).putString("latest", "99.0.0").commit()
        val controller = open()
        try {
            val line = UpdateInstall.installFailureLine("指紋が一致しません")
            assertFalse(texts(controller.get()).contains(line))
            // executor のスレッドが書いたのと同じ口で書く。開き直さずに行が出る
            Thread { UpdateInstall.recordFailure(app, line) }.apply {
                start()
                join()
            }
            shadowOf(Looper.getMainLooper()).idle()
            assertTrue(texts(controller.get()).contains(line))
            assertTrue(texts(controller.get()).any { it.startsWith("更新あり 99.0.0") })
        } finally {
            controller.pause().stop().destroy()
        }
    }

    @Test fun `許可されていなければ、開いている画面の行に許可待ちの文が出る`() { // Scenario: 携帯は提供元不明のアプリが許されていなければ、許可の画面を開く
        configured()
        app.getSharedPreferences(
            UpdateCheck.PREFS,
            0
        ).edit().putBoolean("intro_shown", true).putString("latest", "99.0.0").commit()
        shadowOf(app.packageManager).setCanRequestPackageInstalls(false)
        val controller = open()
        try {
            UpdateInstall.start(controller.get())
            shadowOf(Looper.getMainLooper()).idle()
            assertTrue(texts(controller.get()).contains(UpdateInstall.UNKNOWN_SOURCES_LINE))
        } finally {
            controller.pause().stop().destroy()
        }
    }

    @Test fun `新しい版が無くなった・更新の確認を切った後は失敗の行を出さずに消す`() {
        // review/code.md R50・R63: 失敗の行は「更新あり」の行に付く。閉じられない画面に古い行を残さない
        configured()
        UpdateInstall.recordFailure(app, UpdateInstall.installFailureLine("指紋が一致しません"))
        app.getSharedPreferences(
            UpdateCheck.PREFS,
            0
        ).edit().putBoolean("intro_shown", true).putString("latest", BuildConfig.VERSION_NAME).commit()
        val controller = open()
        try {
            assertFalse(texts(controller.get()).any { it.startsWith("入れられませんでした") })
            assertNull(UpdateInstall.failureLine(app))
        } finally {
            controller.pause().stop().destroy()
        }
        UpdateInstall.recordFailure(app, UpdateInstall.installFailureLine("指紋が一致しません"))
        app.getSharedPreferences(
            UpdateCheck.PREFS,
            0
        ).edit().putString("latest", "99.0.0").putBoolean("enabled", false).commit()
        val disabled = open()
        try {
            assertFalse(texts(disabled.get()).any { it.startsWith("入れられませんでした") })
            assertNull(UpdateInstall.failureLine(app))
        } finally {
            disabled.pause().stop().destroy()
        }
    }

    @Test fun `許可を与えて戻ると許可待ちの文だけを消す`() {
        // review/code.md R50: 許可を与えて戻った回は「許可した後に…」だけを消し、取得・照合の失敗は残す
        UpdateInstall.recordFailure(app, UpdateInstall.UNKNOWN_SOURCES_LINE)
        UpdateInstall.clearResolvedUnknownSourcesFailure(app, permitted = true)
        assertNull(UpdateInstall.failureLine(app))
        UpdateInstall.recordFailure(app, UpdateInstall.installFailureLine("指紋が一致しません"))
        UpdateInstall.clearResolvedUnknownSourcesFailure(app, permitted = true)
        assertEquals(UpdateInstall.installFailureLine("指紋が一致しません"), UpdateInstall.failureLine(app))
    }

    @Test fun `取り消し結果の受け手を持つ`() { // Scenario: 携帯でインストール画面を取り消すと、入っている版のまま動き続ける
        assertEquals("dev.ashiato.collector.UPDATE_RESULT", UpdateInstall.ACTION_RESULT)
    }

    @Test fun `成功結果の後に置き場を掃除できる`() { // Scenario: 携帯は入れ終えた後に APK を置き場に残さない
        val dir = UpdateInstall.updateDirectory(app).apply { mkdirs() }
        val file = File(dir, "result.apk").apply { writeText("apk") }
        UpdateInstall.clean(app)
        assertFalse(file.exists())
    }

    @Test fun `押す前は選択を実行しない`() { // Scenario: 押す前は配布物を落とさない
        UpdateInstall.clean(app)
        assertFalse(UpdateInstall.updateDirectory(app).listFiles()?.any() == true)
    }

    @Test fun `リリース資産の GitHub URL を受け入れる`() { // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(UpdateInstall.isAllowedDownloadUrl("https://github.com/rai-256/ashiato/releases/download/v/a.apk"))
    }

    @Test fun `リダイレクト資産の URL を受け入れる`() { // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(UpdateInstall.isAllowedDownloadUrl("https://release-assets.githubusercontent.com/a"))
    }

    @Test fun `sha1 は受け入れない`() { // Scenario: 指紋の無い配布物は入れない
        assertNull(
            UpdateInstall.chooseAsset(
                "0.4.0",
                release("ashiato-collector-0.4.1.apk", "v/a.apk", "\"sha1:x\""),
            )
        )
    }

    @Test fun `桁不足の sha256 は受け入れない`() { // Scenario: 指紋の無い配布物は入れない
        assertNull(
            UpdateInstall.chooseAsset(
                "0.4.0",
                release("ashiato-collector-0.4.1.apk", "v/a.apk", "\"sha256:12\""),
            )
        )
    }

    @Test fun `別ホストは受け入れない`() { // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(!UpdateInstall.isAllowedDownloadUrl("https://github.com/example/ashiato/releases/download/v/a.apk"))
    }

    @Test fun `http は受け入れない`() { // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(!UpdateInstall.isAllowedDownloadUrl("http://release-assets.githubusercontent.com/a"))
    }

    @Test fun `APK 以外は選ばない`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        assertNull(
            UpdateInstall.chooseAsset(
                "0.4.0",
                release("ashiato-collector-0.4.1.zip", "v/a.zip", "\"sha256:$hexDigest\""),
            )
        )
    }

    @Test fun `draft は選ばない`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"v0.4.1\",\"draft\":true,\"assets\":[]}]"))
    }

    @Test fun `未対応のタグは選ばない`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"nightly\",\"assets\":[]}]"))
    }

    @Test fun `大文字の指紋を正規化する`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val d = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"
        val a = UpdateInstall.chooseAsset(
            "0.4.0",
            release("ashiato-collector-0.4.1.apk", "v/a.apk", "\"sha256:$d\""),
        )
        assertEquals(d.lowercase(), a?.sha256)
    }

    @Test fun `版番号の無い配布物は選ばない`() { // Scenario: 落とす要求は自分の版も端末の情報も持たない
        assertNull(UpdateInstall.chooseAsset("not-a-version", "[]"))
    }

    @Test fun `空の材料は選ばない`() { // Scenario: 押したときの取り直しは、確かめの数えを動かさない
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[]"))
    }

    @Test fun `一覧と配布物の要求は既定の User-Agent と Accept を使う`() { // Scenario: 落とす要求は自分の版も端末の情報も持たない
        assertEquals(UpdateCheck.USER_AGENT, UpdateInstall.downloadHeaders()["User-Agent"])
        assertEquals(UpdateCheck.ACCEPT, UpdateInstall.downloadHeaders()["Accept"])
        assertFalse(UpdateInstall.downloadHeaders().values.any { it.contains(BuildConfig.VERSION_NAME) })
    }

    @Test fun `指紋の値を資産に保持する`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val d = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        val a = UpdateInstall.chooseAsset(
            "0.4.0",
            release("ashiato-collector-0.4.1.apk", "v/a.apk", "\"sha256:$d\""),
        )
        assertEquals(d, a?.sha256)
    }

    @Test fun `保留のインストール結果は確認画面を起こす`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val confirmation = Intent("android.intent.action.CONFIRM_INSTALL")
        val result = Intent(
            UpdateInstall.ACTION_RESULT
        ).putExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_PENDING_USER_ACTION)
            .putExtra(Intent.EXTRA_INTENT, confirmation)
        UpdateInstallResultReceiver().onReceive(app, result)
        assertEquals(confirmation.action, shadowOf(app).nextStartedActivity.action)
    }

    @Test fun `取り消しの結果は APK を消して確認画面を開かない`() { // Scenario: 携帯でインストール画面を取り消すと、入っている版のまま動き続ける
        val file = File(
            UpdateInstall.updateDirectory(app).apply { mkdirs() },
            "cancelled.apk"
        ).apply { writeText("apk") }
        UpdateInstallResultReceiver().onReceive(
            app,
            Intent(
                UpdateInstall.ACTION_RESULT
            ).putExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE_ABORTED),
        )
        assertFalse(file.exists())
        assertNull(shadowOf(app).nextStartedActivity)
    }

    @Test fun `成功の結果は APK を消して確認画面を開かない`() { // Scenario: 携帯は入れ終えた後に APK を置き場に残さない
        val file = File(
            UpdateInstall.updateDirectory(app).apply { mkdirs() },
            "installed.apk"
        ).apply { writeText("apk") }
        UpdateInstallResultReceiver().onReceive(
            app,
            Intent(
                UpdateInstall.ACTION_RESULT
            ).putExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_SUCCESS),
        )
        assertFalse(file.exists())
        assertNull(shadowOf(app).nextStartedActivity)
    }
}
