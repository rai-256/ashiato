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
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用。ST56 の初回設定を、まっさらな携帯で実際に貼って保存するところを撮る。
 *
 * 共有契約の有効な設定文字列を [SetupActivity] の入力欄へ貼る。録画用 APK の loopback は
 * 携帯から届かない送り先として保存が拒否されるため、ここでは外部向けの形式だけを使う。
 * 保存後に表示する「設定済み」と送り先はアプリの画面そのものを使う。設定文字列には資格情報が含まれるため、保存後に入力欄が空になり、
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

        val setup = SETUP_STRING
        val input = device.wait(Until.findObject(By.clazz(EditText::class.java)), TIMEOUT_MS)
        assertNotNull("設定文字列の欄が無い", input)
        input.text = setup
        hold()
        val save = device.wait(Until.findObject(By.text("保存")), TIMEOUT_MS)
        assertNotNull("保存のボタンが無い", save)
        save.click()

        assertNotNull("保存後に設定済みと出ない", device.wait(Until.findObject(By.textContains("設定済み")), TIMEOUT_MS))
        assertNotNull("保存後に送り先が出ない", device.wait(Until.findObject(By.textContains("送り先:")), TIMEOUT_MS))
        assertFalse("設定文字列が保存後も残っている", device.hasObject(By.text(setup)))
        assertFalse("資格情報が保存後も画面に残っている", device.hasObject(TOKEN))
        hold(5_000)
    }

    private fun hold(ms: Long = 2_500) = Thread.sleep(ms)

    private companion object {
        const val TIMEOUT_MS = 20_000L
        const val TOKEN = "token-for-test-0123456789"
        const val SETUP_STRING = "ashiato-collector:1:eyJ1cmwiOiJodHRwczovL3BjLmV4YW1wbGUudHMubmV0L2FwaSIsInRva2VuIjoidG9rZW4tZm9yLXRlc3QtMDEyMzQ1Njc4OSJ9"
    }
}
