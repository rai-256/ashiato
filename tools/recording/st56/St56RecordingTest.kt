// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.Context
import android.content.Intent
import android.widget.EditText
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.util.regex.Pattern
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用。ST56 の初回設定を、まっさらな携帯で実際に貼って保存するところを撮る。
 *
 * 録画環境が APK に渡す loopback の送り先（adb reverse 経由）と資格情報から設定文字列を組み、
 * [SetupActivity] の入力欄へ貼る。保存後に表示する「設定済み」と送り先、および接続確認の結果は
 * アプリの画面そのものを使う。設定文字列には資格情報が含まれるため、保存後に入力欄が空になり、
 * 画面の文字列に資格情報が無いことも確かめる。
 */
@RunWith(AndroidJUnit4::class)
class St56RecordingTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)

    @Test
    fun collectorSetupForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        context.getSharedPreferences(Config.SETUP_PREFS, Context.MODE_PRIVATE).edit().clear().commit()
        context.getSharedPreferences(MainActivity.PREFS, Context.MODE_PRIVATE).edit().clear().commit()
        Config.initialize(context)

        context.startActivity(Intent(context, SetupActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        assertNotNull(
            "設定画面が開かない",
            device.wait(Until.findObject(By.text("収集アプリの設定")), TIMEOUT_MS),
        )
        hold()

        val setup = buildSetupString(Config.baseUrl, Config.apiToken)
        val input = device.wait(Until.findObject(By.clazz(EditText::class.java)), TIMEOUT_MS)
        assertNotNull("設定文字列の欄が無い", input)
        input.text = setup
        hold()
        val save = device.wait(Until.findObject(By.text("保存")), TIMEOUT_MS)
        assertNotNull("保存のボタンが無い", save)
        save.click()

        assertNotNull("保存後に設定済みと出ない", device.wait(Until.findObject(By.text("設定済み")), TIMEOUT_MS))
        assertNotNull("保存後に送り先が出ない", device.wait(Until.findObject(By.textContains("送り先:")), TIMEOUT_MS))
        assertNotNull(
            "接続確認の結果が出ない",
            device.wait(Until.findObject(By.text(Pattern.compile("^(繋がりました|送り先が.*版のサーバです)$"))), TIMEOUT_MS),
        )
        assertFalse("設定文字列が保存後も残っている", device.hasObject(By.text(setup)))
        assertFalse("資格情報が保存後も画面に残っている", device.hasObject(By.text(Config.apiToken)))
        hold(5_000)
    }

    private fun hold(ms: Long = 2_500) = Thread.sleep(ms)

    private companion object {
        const val TIMEOUT_MS = 20_000L
    }
}
