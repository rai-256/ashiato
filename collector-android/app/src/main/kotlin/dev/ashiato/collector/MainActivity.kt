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
 * 権限を順に求めてから収集を始める。
 *
 * **背景の位置は前景の位置と同時に求められない**（Android 11 以降）ので 2 段に分ける。
 * どれかを断られても**落とさない**（tasks 6.2）—— 落ちると次の起動まで収集が止まる。
 */
class MainActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestNext()
    }

    private fun granted(permission: String) =
        checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

    /** 足りない権限を 1 段ずつ求める。全部揃ったら収集を始める。 */
    private fun requestNext() {
        val fine = Manifest.permission.ACCESS_FINE_LOCATION
        val background = Manifest.permission.ACCESS_BACKGROUND_LOCATION
        when {
            !granted(fine) -> requestPermissions(arrayOf(fine), REQUEST)
            // 背景は前景が許可された**後**でしか求められない
            !granted(background) -> requestPermissions(arrayOf(background), REQUEST)
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                !granted(Manifest.permission.POST_NOTIFICATIONS) ->
                requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), REQUEST)

            else -> start()
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (grantResults.isEmpty() || grantResults[0] != PackageManager.PERMISSION_GRANTED) {
            // **断られたら何も送らずに終わる。落ちない**（tasks 6.2）
            Log.w(TAG, Telemetry.line("permission_denied"))
            finish()
            return
        }
        requestNext()
    }

    private fun start() {
        startForegroundService(Intent(this, LocationService::class.java))
        finish()
    }

    private companion object {
        const val TAG = "ashiato"
        const val REQUEST = 1
    }
}
