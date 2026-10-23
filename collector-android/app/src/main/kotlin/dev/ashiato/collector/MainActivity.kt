// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.util.TypedValue
import android.view.WindowInsets
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

/**
 * 権限を順に求めてから収集を始める。**記録を見せる画面は持たない** —— 表示は V-01（web）の仕事。
 * 持つのは、休止の対象（使っていないアプリの制限）から外れていない・読めなかったときの 1 行と
 * 「設定を開く」のボタンと、初回に 1 度だけの説明のダイアログだけ（ST13 / design D12（仮））。
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
 * ## 写真と動画（design D6 / Q15）
 *
 * 位置の後に、写真と動画へのアクセスと写真の位置（`ACCESS_MEDIA_LOCATION`）を**まとめて 1 度だけ**求める。
 * 断られても収集は始める —— 写真と動画のソースだけが「取れない状態」を報告して待つ（位置は取れ続ける）。
 *
 * ## 利用状況へのアクセス（本人の決定 Q3 / design D6（仮））
 *
 * **その場のダイアログでは取れない。** アプリから開けるのは設定画面までなので、
 * **初回起動で 1 度だけ**そこへ送り、許されなくても収集は始める。
 * 2 度目からは自動で送らない（許さないと決めた本人の邪魔になる）——
 * 以後は常駐の通知から同じ画面へたどれる。
 */
// ライフサイクルの上書き・試験の差し替え口・権限の順と、休止の除外の 1 行とダイアログ（ST13）を同じ画面に持つ。
// 画面は 1 枚だけで、分けると `onResume` / `onDestroy` の配線が 2 か所に割れる
@Suppress("TooManyFunctions")
open class MainActivity : Activity() {
    /** 一度求めた権限。**同じものを無限に求めない**ための記録。 */
    private val asked = mutableSetOf<String>()

    /** 求めている最中。`onResume` と結果の二重呼び出しで再要求しないため。 */
    private var inFlight = false

    /**
     * 位置を断られたことを**もうログに残した**（独立レビュー N2）。
     *
     * `proceed()` は結果の返りと `onResume` で何度も通る —— 通知の権限を求めるようになってからは
     * 1 回の起動で少なくとも 2 回通る。毎回出すと `logcat` が同じ 1 行で埋まり、
     * **環状の置き場から他の行が押し出される**（`logcat -t 400` を読む計測テストの前提に直接効く）。
     * `LocationService` の `source_unavailable` で潰したのと同じ型。
     */
    private var deniedLogged = false

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

    private val granted: (String) -> Boolean =
        { checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }

    /**
     * 無くても始めるが、あったほうがよいもの。
     *
     * **前提を持つのは背景の位置だけ**（独立レビュー I1）。前景が許可された後でしか求められないので、
     * 前景が無いあいだは一覧に入れない（入れても OS が即座に拒否で返し、`asked` に入って二度と求められなくなる）。
     *
     * **通知の権限に位置の可否は関係ない。むしろ位置を拒んだ端末ほど要る** ——
     * Android 13 以降、通知の権限が無いと**前景サービスは立っても通知が表示されない**。
     * 表示されないと、design D6 の「以後は常駐の通知から同じ設定画面へたどれる」が
     * その本人にだけ効かず（自動で送るのは 1 度だけなので）**利用状況へのアクセスへ戻る道が消える**。
     * ST04 の 83 日の知らせも出せない。
     */
    private fun niceToHave(): List<String> = buildList {
        // 背景が無いと、START_STICKY で立て直されたとき位置を取れずに収集が止まる
        if (granted(Manifest.permission.ACCESS_FINE_LOCATION)) {
            add(Manifest.permission.ACCESS_BACKGROUND_LOCATION)
        }
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
            // 位置だけが③「取れない状態」として生存信号に載り、アプリ利用は取れ続ける。
            // **1 回の起動で 1 行だけ**（N2）
            if (!deniedLogged) {
                Log.w(TAG, Telemetry.line("permission_denied", source = LOGICAL_SOURCE))
                deniedLogged = true
            }
        } else {
            // 設定画面で許されて戻ってきた。次に断られたらまた 1 行出す
            deniedLogged = false
        }
        // **位置の可否で枝を分けない**（独立レビュー I1）。前提を持つのは背景の位置だけで、
        // それは `niceToHave()` の中で見る —— 分けると、位置を拒んだ端末が通知の権限を 1 度も求めない
        val next = niceToHave().firstOrNull { !granted(it) && it !in asked }
        // 位置の後に、写真と動画へのアクセスと写真の位置を**まとめて 1 度だけ**求める（design D6）。
        // 14 以降の「選んだ写真だけ」は画像・動画・選んだ写真だけを同じ要求で求めないと選び方が出ない。
        // 断られても収集は始める（取得条件が欠けたソースだけが止まる。Q15 は読み手が持つ）
        val media = mediaPermissionsToAsk(Build.VERSION.SDK_INT).filter { !granted(it) && it !in asked }
        when {
            next != null -> ask(next)
            media.isNotEmpty() -> {
                asked += media
                inFlight = true
                requestPermissionGroup(media)
            }
            else -> start()
        }
    }

    private fun ask(permission: String) {
        asked += permission
        inFlight = true
        requestPermission(permission)
    }

    /** OS へ権限を 1 度にまとめて求める。**試験だけが差し替える**（[requestPermission] と同じ理由）。 */
    protected open fun requestPermissionGroup(permissions: List<String>) {
        requestPermissions(permissions.toTypedArray(), REQUEST)
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
        val state = unusedAppRestrictionsReader().read(this)
        if (state == UnusedAppRestrictionsState.NOT_EXEMPT || state == UnusedAppRestrictionsState.UNKNOWN) {
            // 閉じない。画面に 1 行出し続ける（設定から戻れば onResume が読み直し、外れていれば閉じる）
            showUnusedAppRestrictions(state)
            if (state == UnusedAppRestrictionsState.NOT_EXEMPT) askToExemptOnce()
        } else {
            finish()
        }
    }

    /**
     * 「使っていないアプリの一時停止」の除外の状態の読み口。**試験だけが差し替える**
     * （Robolectric は OS の除外の状態を持たない）。
     */
    protected open fun unusedAppRestrictionsReader(): UnusedAppRestrictionsReader = SystemUnusedAppRestrictionsReader

    /** いま出している除外の案内。`onResume` のたびに `start()` を通るので、出している間は重ねて出さない。 */
    private var exemptDialog: AlertDialog? = null

    /**
     * 対象から外れていないとき、説明のダイアログを**自動では 1 度だけ**出す（design D12（仮））。
     * 印は**本人が答えたとき**（応じた・あとで・閉じた）に立てる（/code-review R33）。出しただけで立てると、
     * 設定の画面にいる間に画面が作り直される（回転・プロセスの終了）とダイアログは消えて印だけが残り、
     * 自動の案内が一度も読まれないまま二度と出ない。
     */
    private fun askToExemptOnce() {
        val asked = UnusedAppRestrictionsAsked(getSharedPreferences(PREFS, MODE_PRIVATE))
        if (asked.asked() || exemptDialog?.isShowing == true) return
        exemptDialog = AlertDialog.Builder(this)
            .setMessage(UNUSED_APP_RESTRICTIONS_EXPLANATION)
            .setPositiveButton(OPEN_SETTINGS_LABEL) { _, _ -> openUnusedAppRestrictionsSettings() }
            .setNegativeButton(NOT_NOW_LABEL, null)
            .setOnDismissListener {
                asked.mark()
                exemptDialog = null
            }
            .show()
    }

    override fun onDestroy() {
        // 答えずに作り直されるときは印を立てない（次の起動でもう一度出す）。窓は閉じておく
        exemptDialog?.setOnDismissListener(null)
        exemptDialog?.dismiss()
        exemptDialog = null
        super.onDestroy()
    }

    private fun openUnusedAppRestrictionsSettings() {
        val opened = runCatching { startActivity(manageUnusedAppRestrictionsIntent(this)) }.isSuccess
        if (!opened) Log.w(TAG, Telemetry.line("unused_app_restrictions_settings_missing", source = LOGICAL_SOURCE))
    }

    /** 画面に 1 行（外れていない / 読めなかった）。外れていないときだけ「設定を開く」を添える。 */
    private fun showUnusedAppRestrictions(state: UnusedAppRestrictionsState) {
        val density = resources.displayMetrics.density
        val pad = (PADDING_DP * density).toInt()
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(pad, pad, pad, pad)
            // targetSdk 35 以降を Android 15 以降で開くと全面表示が強いられ、中身が状態の帯と題の帯の裏に入る
            // （録画で実測: 1 行も「設定を開く」も題の帯に隠れ、押しても設定が開かなかった）。
            // 題の帯は inset に数えられないので隠し、状態の帯・操作の帯のぶんを余白に足す
            setOnApplyWindowInsetsListener { v, insets ->
                val bars = insets.getInsets(WindowInsets.Type.systemBars())
                v.setPadding(pad + bars.left, pad + bars.top, pad + bars.right, pad + bars.bottom)
                insets
            }
        }
        actionBar?.hide()
        column.addView(
            TextView(this).apply {
                text = if (state == UnusedAppRestrictionsState.NOT_EXEMPT) NOT_EXEMPT_LINE else UNKNOWN_LINE
            },
        )
        if (state == UnusedAppRestrictionsState.NOT_EXEMPT) {
            column.addView(
                Button(this).apply {
                    text = OPEN_SETTINGS_LABEL
                    // タップの的は 24 dp 以上（余裕を見て 48 dp）
                    minimumHeight = TypedValue.applyDimension(
                        TypedValue.COMPLEX_UNIT_DIP,
                        BUTTON_MIN_HEIGHT_DP,
                        resources.displayMetrics,
                    ).toInt()
                    setOnClickListener { openUnusedAppRestrictionsSettings() }
                },
            )
        }
        setContentView(column)
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

        const val NOT_EXEMPT_LINE = "使っていないアプリの一時停止の対象になっています"
        const val UNKNOWN_LINE = "使っていないアプリの一時停止の除外の状態を読めなかった"
        const val OPEN_SETTINGS_LABEL = "設定を開く"
        const val NOT_NOW_LABEL = "あとで"
        private const val PADDING_DP = 16
        private const val BUTTON_MIN_HEIGHT_DP = 48f
        const val UNUSED_APP_RESTRICTIONS_EXPLANATION =
            "数か月開かないと、OS が権限を外し通知を止めます。足跡を取り続けるため、一時停止の対象から外す設定を開きますか。"
    }
}
