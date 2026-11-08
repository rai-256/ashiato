// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.Context
import androidx.test.core.app.ActivityScenario
import androidx.test.platform.app.InstrumentationRegistry

/**
 * ランチャーと同じ起動で `MainActivity` を開く。最初の設定の画面は「後で」を押した扱いにして飛ばす
 * （ST56 から、送り先と資格情報が揃わない組み立ては最初に設定の画面を出す。計測テストは `-P` に送り先しか渡さないので、
 * 飛ばさないと設定の画面で止まり、権限の要求も収集の起動も起きない。review/code.md R23）。
 */
fun launchMainSkippingSetup(): ActivityScenario<MainActivity> {
    val context = InstrumentationRegistry.getInstrumentation().targetContext
    context.getSharedPreferences(Config.SETUP_PREFS, Context.MODE_PRIVATE)
        .edit().putBoolean(MainActivity.SETUP_SKIPPED, true).commit()
    return ActivityScenario.launch(MainActivity::class.java)
}
