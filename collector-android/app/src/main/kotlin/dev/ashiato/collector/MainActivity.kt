// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.util.Log

/**
 * 権限を順に求めてから収集を始める。**画面は持たない** —— 表示は V-01（web）の仕事。
 *
 * ## なぜ結果のコードを見ないか
 *
 * **背景の位置は Android 11 以降、許可ダイアログで取れない。** `requestPermissions` を呼ぶと
 * 設定画面へ送られ、`onRequestPermissionsResult` は**拒否として返る** ——
 * その後ユーザーが設定画面で「常に許可」にしても、コールバックは拒否のままである。
 *
 * 結果コードを信じて `finish()` すると、**前景を許可した直後に必ずここで終わり、
 * サービスが一度も起動しない**（実機で発覚。design D27）。
 * なので**結果コードではなく、そのつどの実際の権限状態**を見る。
 * `onResume` からも見直すので、設定画面から戻ってきた許可も拾える。
 *
 * ## 何が必須か
 *
 * **必須は前景の位置だけ。** それが無ければ取るものが無いので始めない。
 * 背景の位置と通知は**無くても収集を始める** —— 始めないより degraded で動くほうが
 * 成功条件 1（1 年間途切れない）に近い。足りないものはログに種別だけ残す。
 */
class MainActivity : Activity() {
    /** 一度求めた権限。**同じものを無限に求めない**ための記録。 */
    private val asked = mutableSetOf<String>()

    /** 求めている最中。`onResume` と結果の二重呼び出しで再要求しないため。 */
    private var inFlight = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // ここでは何もしない。onResume が状態を見る（設定画面からの戻りも同じ経路になる）
    }

    override fun onResume() {
        super.onResume()
        if (!inFlight) proceed()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        inFlight = false
        // **grantResults は見ない**（上の理由）。実際の権限状態だけを見る
        proceed()
    }

    private fun granted(permission: String) =
        checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

    /** 無くても始めるが、あったほうがよいもの。 */
    private fun niceToHave(): List<String> = buildList {
        // 背景が無いと、START_STICKY で立て直されたとき位置を取れずに収集が止まる
        add(Manifest.permission.ACCESS_BACKGROUND_LOCATION)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            add(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    /** 足りない権限を 1 段ずつ求め、求め終わったら収集を始める。 */
    private fun proceed() {
        val fine = Manifest.permission.ACCESS_FINE_LOCATION
        if (!granted(fine)) {
            // 一度求めて、それでも無いなら断られた。**落とさずに終わる**（tasks 6.2）
            if (fine in asked) {
                Log.w(TAG, Telemetry.line("permission_denied"))
                finish()
            } else {
                ask(fine)
            }
            return
        }
        // 背景は前景が許可された**後**でしか求められない
        val next = niceToHave().firstOrNull { !granted(it) && it !in asked }
        if (next != null) {
            ask(next)
            return
        }
        start()
    }

    private fun ask(permission: String) {
        asked += permission
        inFlight = true
        requestPermissions(arrayOf(permission), REQUEST)
    }

    private fun start() {
        // 足りないものは種別だけ残す。**値は出さない**（製造準備 A-2）
        if (!granted(Manifest.permission.ACCESS_BACKGROUND_LOCATION)) {
            Log.w(TAG, Telemetry.line("degraded", error = "no_background_location"))
        }
        startForegroundService(Intent(this, LocationService::class.java))
        finish()
    }

    private companion object {
        const val TAG = "ashiato"
        const val REQUEST = 1
    }
}
