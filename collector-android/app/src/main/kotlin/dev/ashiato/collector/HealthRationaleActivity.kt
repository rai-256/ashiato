// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Activity
import android.os.Bundle
import android.view.WindowInsets
import android.widget.LinearLayout
import android.widget.TextView

/**
 * 健康データの権限の画面の「プライバシーポリシー」から開かれる説明の画面（design D7（仮））。
 * 文だけを出す。開き方は manifest の 2 つ（Android 13 以前の intent-filter と Android 14 以降の activity-alias）。
 */
class HealthRationaleActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val pad = (PADDING_DP * resources.displayMetrics.density).toInt()
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(pad, pad, pad, pad)
            // 全面表示で中身が状態の帯の裏に入らないよう、MainActivity.showUnusedAppRestrictions と同じ余白を足す
            setOnApplyWindowInsetsListener { v, insets ->
                val bars = insets.getInsets(WindowInsets.Type.systemBars())
                v.setPadding(pad + bars.left, pad + bars.top, pad + bars.right, pad + bars.bottom)
                insets
            }
        }
        actionBar?.hide()
        column.addView(TextView(this).apply { text = HEALTH_RATIONALE })
        setContentView(column)
    }

    companion object {
        private const val PADDING_DP = 16
    }
}

/** 説明の画面の文（design D7（仮）。本人の推奨 Q6）。 */
const val HEALTH_RATIONALE =
    "収集アプリは、睡眠・心拍・歩数・運動の健康データを読む権限を求めます。" +
        "読んだものは、あなたの自宅の PC（あしあとのサーバ）にだけ送ります。" +
        "そこから先に何を渡すかは、あしあとの承認の設定で決まります。" +
        "いまは権限を取るだけで、健康データを読み始めるのは健康データの取り込みを入れてからです。"
