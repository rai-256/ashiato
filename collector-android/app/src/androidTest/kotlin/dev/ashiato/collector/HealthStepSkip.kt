// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context

/**
 * 健康データの権限の段（ST11）を関心にしない計測テストが、`MainActivity` を開く前に呼ぶ。
 * 「1 度求めた」印を立てて、Health Connect の権限の画面を出さずに先へ進める（design D4 の 2 度目の起動と同じ）。
 *
 * 印が無いと、Health Connect が使える端末では権限の画面が前に出て、答えるまで収集が始まらない
 * （ST11 より前の試験は、Android 14 以降で求める口が開けずに素通りしていたので気付かなかった。2026-10-25 実測）。
 */
fun skipHealthPermissionStep(context: Context) {
    context.getSharedPreferences(HEALTH_PREFS, Context.MODE_PRIVATE).edit()
        .putBoolean(ASKED_HEALTH_PERMISSIONS, true).commit()
}
