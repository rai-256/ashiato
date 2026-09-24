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
 * **必須は何も無い**（ST06 / tasks 5.1 / 本人の決定 Q7 / design D5）。求めるものは順に求めるが、
 * **どれを断られても収集は始める** —— 取得条件が欠けたソースだけが止まり、他は取り続ける。
 * ST06 より前は前景の位置を断られると `finish()` していた。そのときは**アプリ利用も止まり、
 * 生存信号も出なかった**ので、受け手の画面には③「動いていたが取れない状態」ではなく
 * ⑥「途絶」が出た（アプリ利用は 10 日を越えれば二度と取れない）。
 *
 * ## 利用状況へのアクセス（本人の決定 Q3 / design D6（仮））
 *
 * **その場のダイアログでは取れない。** アプリから開けるのは設定画面までなので、
 * **初回起動で 1 度だけ**そこへ送り、許されなくても収集は始める。
 * 2 度目からは自動で送らない（許さないと決めた本人の邪魔になる）——
 * 以後は常駐の通知から同じ画面へたどれる。
 */
open class MainActivity : Activity() {
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
        if (!granted(fine) && fine !in asked) {
            ask(fine)
            return
        }
        if (!granted(fine)) {
            // 一度求めて、それでも無いなら断られた。**それでも収集は始める**（ST06 / Q7）——
            // 位置だけが③「取れない状態」として生存信号に載り、アプリ利用は取れ続ける
            Log.w(TAG, Telemetry.line("permission_denied", source = LOGICAL_SOURCE))
        } else {
            // 背景は前景が許可された**後**でしか求められない
            val next = niceToHave().firstOrNull { !granted(it) && it !in asked }
            if (next != null) {
                ask(next)
                return
            }
        }
        start()
    }

    private fun ask(permission: String) {
        asked += permission
        inFlight = true
        requestPermission(permission)
    }

    /**
     * OS へ権限を求める。**試験だけが差し替える**（`LocationService` の取得元と同じ形）——
     * `Activity.requestPermissions` は final で、求めた回数を外から数える口が無い。
     */
    protected open fun requestPermission(permission: String) {
        requestPermissions(arrayOf(permission), REQUEST)
    }

    private fun start() {
        // 足りないものは種別だけ残す。**値は出さない**（製造準備 A-2）
        if (!granted(Manifest.permission.ACCESS_BACKGROUND_LOCATION)) {
            Log.w(TAG, Telemetry.line("degraded", source = LOGICAL_SOURCE, error = "no_background_location"))
        }
        // **先に収集を始める**（本人の決定 Q3「拒んでも位置の収集は始める」）——
        // 設定画面から戻ってこない本人の端末でも、収集はもう動いている
        startForegroundService(Intent(this, LocationService::class.java))
        sendToUsageAccessOnce()
        finish()
    }

    /**
     * 「利用状況へのアクセス」の設定画面へ**初回起動で 1 度だけ**送る（本人の決定 Q3 / design D6（仮））。
     *
     * **送ったことを端末に残す。** インスタンスにだけ持つと、アプリを開き直すたびに設定画面が開き、
     * 「許さない」と決めた本人の邪魔になる（D6 が避けた当のこと）。
     * **許可されていれば送らない**（既に許している本人を設定画面へ連れていかない）。
     */
    private fun sendToUsageAccessOnce() {
        val marks = getSharedPreferences(PREFS, MODE_PRIVATE)
        if (marks.getBoolean(SENT_TO_USAGE_ACCESS, false)) return
        if (usageAccessCapability(this).blockers.none { it == Capability.PERMISSION }) return
        // **先に印を書く。** 開けなかった端末（設定画面を持たない）で毎回試し続けないため
        marks.edit().putBoolean(SENT_TO_USAGE_ACCESS, true).apply()
        val sent = runCatching { startActivity(usageAccessSettingsIntent()) }.isSuccess
        if (!sent) Log.w(TAG, Telemetry.line("usage_access_settings_missing", source = APP_USAGE_LOGICAL_SOURCE))
    }

    internal companion object {
        private const val TAG = "ashiato"
        private const val REQUEST = 1

        /** 「1 度だけ」を端末に残す置き場（design D6（仮））。**試験も同じ名前を読む。** */
        const val PREFS = "collector"
        const val SENT_TO_USAGE_ACCESS = "sent_to_usage_access"
    }
}
