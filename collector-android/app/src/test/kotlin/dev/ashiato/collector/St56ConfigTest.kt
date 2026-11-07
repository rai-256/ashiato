// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.app.Application
import androidx.test.core.app.ApplicationProvider
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.shadows.ShadowLog

@RunWith(RobolectricTestRunner::class)
class St56ConfigTest {
    private val app: Application = ApplicationProvider.getApplicationContext()

    @Before
    fun setUp() {
        app.getSharedPreferences(Config.SETUP_PREFS, 0).edit().clear().commit()
        Config.clearOverrideForTest()
        Config.overrideForTest("http://127.0.0.1:18787", "build-token", "")
        Config.initialize(app)
    }

    @After
    fun tearDown() {
        app.getSharedPreferences(Config.SETUP_PREFS, 0).edit().clear().commit()
        Config.clearOverrideForTest()
    }

    @Test
    fun `保存した設定は組み立て時の値より先に使う`() {
        // Scenario: 保存した設定は、組み立ての時の値より先に使う
        assertEquals(
            SetupSaveResult.SAVED,
            SetupStore(app).save(buildSetupString("https://b.example/api", "saved-token"))
        )
        assertEquals("https://b.example/api", Config.baseUrl)
        assertEquals("saved-token", Config.apiToken)
    }

    @Test
    fun `保存した設定が無ければ組み立て時の値を使う`() {
        // Scenario: 保存した設定が無ければ、組み立ての時の値で送る
        assertEquals("http://127.0.0.1:18787", Config.baseUrl)
        assertEquals("build-token", Config.apiToken)
    }

    @Test
    fun `利用者が空なら既定の利用者を使う`() {
        assertEquals("00000000-0000-0000-0000-000000000000", Config.userId)
    }

    @Test
    fun `画面のアドレスは保存先の api を外す`() {
        // Scenario: 送り先から /api を外して画面を開く
        SetupStore(app).save(buildSetupString("https://b.example/api/", "saved-token"))
        assertEquals("https://b.example", Config.webUrl)
    }

    @Test
    fun `設定が無ければ組み立て時の画面アドレスを使う`() {
        // Scenario: 保存した設定が無ければ、組み立ての時の画面のアドレスを使う
        assertEquals("", Config.webUrl)
    }

    @Test
    fun `読めない文字列は前の設定を残す`() {
        // Scenario: 読めない文字列は保存せず、前の設定を残す
        SetupStore(app).save(buildSetupString("https://a.example/api", "old-token"))
        assertEquals(SetupSaveResult.UNREADABLE, SetupStore(app).save("hello"))
        assertEquals("https://a.example/api", Config.baseUrl)
        assertEquals("old-token", Config.apiToken)
    }

    @Test
    fun `loopback の設定は保存しない`() {
        // Scenario: 自分自身を指す送り先は保存しない
        assertEquals(
            SetupSaveResult.UNREACHABLE,
            SetupStore(app).save(buildSetupString("http://127.0.0.1:18787/api", "token"))
        )
        assertFalse(Config.hasSavedSetup)
    }

    @Test
    fun `暗号化されない設定は保存しない`() {
        // Scenario: 暗号化されない送り先は保存しない
        assertEquals(
            SetupSaveResult.UNREACHABLE,
            SetupStore(app).save(buildSetupString("http://pc.example.test:18787/api", "token"))
        )
        assertFalse(Config.hasSavedSetup)
    }

    @Test
    fun `組み立て時の loopback はそのまま使う`() {
        // Scenario: 組み立ての時の loopback の送り先はそのまま使う
        assertEquals("http://127.0.0.1:18787", Config.baseUrl)
    }

    @Test
    fun `保存した設定に利用者を持たない`() {
        assertEquals("00000000-0000-0000-0000-000000000000", Config.userId)
    }

    @Test
    fun `設定文字列の資格情報は toString に出さない`() {
        // 画面と送信まで見るのは St56SetupActivityTest（Scenario「資格情報は画面にもログにも出ない」。review/code.md R29）
        val setup = SetupString("https://b.example/api", "token-for-test-0123456789")
        assertFalse(setup.toString().contains("token-for-test-0123456789"))
        ShadowLog.clear()
        SetupStore(app).save(buildSetupString(setup.url, setup.token))
        assertTrue(ShadowLog.getLogs().none { it.msg.orEmpty().contains(setup.token) })
    }
}
