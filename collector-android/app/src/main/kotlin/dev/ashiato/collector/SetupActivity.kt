// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.app.Activity
import android.content.Intent
import android.content.SharedPreferences
import android.os.Bundle
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Switch
import java.util.concurrent.Executors

/** 収集アプリが生成した設定文字列を貼り、保存時に送り先だけを確認する画面。 */
open class SetupActivity : Activity() {
    private lateinit var column: LinearLayout
    private lateinit var input: EditText
    private lateinit var status: TextView
    private lateinit var result: TextView
    private lateinit var updateSwitch: Switch

    /** 保留の節（件数・期間・「この送り先へ送る」）。保存と保留の変化で組み直す（review/code.md R5） */
    private val holdViews = ArrayList<View>()
    private val setupPrefs: SharedPreferences by lazy { getSharedPreferences(Config.SETUP_PREFS, MODE_PRIVATE) }

    /** サービスが未送信を数え終えて保留を書き換えたら、画面を組み直す。参照を持たないと外れる */
    private val prefsListener = SharedPreferences.OnSharedPreferenceChangeListener { _, _ ->
        if (!isDestroyed) render()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Config.initialize(this)
        column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val pad = (16 * resources.displayMetrics.density).toInt()
            setPadding(pad, pad, pad, pad)
        }
        updateSwitch = Switch(this).apply {
            text = "収集アプリの設定\n新しい版を確かめる"
            isChecked = getSharedPreferences(UpdateCheck.PREFS, MODE_PRIVATE).getBoolean("enabled", true)
            setOnCheckedChangeListener { _, enabled -> UpdateChecker(this@SetupActivity).setEnabled(enabled) }
        }
        column.addView(updateSwitch)
        input = EditText(this).apply {
            hint = "設定文字列を貼り付けてください"
            minLines = 3
            isSingleLine = false
        }
        column.addView(input)
        val save = Button(this).apply {
            text = "保存"
            setOnClickListener { saveSetup() }
        }
        column.addView(save)
        column.addView(
            Button(this).apply {
                text = "後で"
                setOnClickListener {
                    // バックアップされない置き場に置く（D9。review/code.md R19）
                    setupPrefs.edit().putBoolean(MainActivity.SETUP_SKIPPED, true).apply()
                    setResult(RESULT_CANCELED)
                    finish()
                }
            }
        )
        status = TextView(this)
        column.addView(status)
        result = TextView(this)
        column.addView(result)
        render()
        setContentView(column)
        UpdateChecker(this).markIntroShown()
    }

    override fun onResume() {
        super.onResume()
        setupPrefs.registerOnSharedPreferenceChangeListener(prefsListener)
        render()
    }

    override fun onPause() {
        setupPrefs.unregisterOnSharedPreferenceChangeListener(prefsListener)
        super.onPause()
    }

    /** 「設定済み」と送り先、保留の節を今の値で組み直す。結果の文はいつも最後に置く。 */
    private fun render() {
        status.text = if (Config.hasSavedSetup) "$CONFIGURED_TEXT\n送り先: ${Config.baseUrl}" else ""
        status.visibility = if (Config.hasSavedSetup) View.VISIBLE else View.GONE
        holdViews.forEach(column::removeView)
        holdViews.clear()
        val gate = DestinationGate(setupPrefs)
        // 件数を数える前（保存の直後）は出さない。サービスが数えると prefs が変わって組み直す
        if (!gate.blocked || gate.summary.count <= 0) return
        val summary = gate.summary
        val dates = listOfNotNull(summary.oldest, summary.newest)
            .distinct()
            .joinToString(
                " ～ "
            ) { it.atZone(java.time.ZoneId.systemDefault()).toLocalDateTime().toString().replace('T', ' ') }
        holdViews += TextView(this).apply {
            text = "${gate.headline}\n未送信 ${summary.count} 件${if (dates.isEmpty()) "" else "\n$dates"}"
        }
        holdViews += Button(this).apply {
            text = "この送り先へ送る"
            setOnClickListener {
                gate.approvePending()
                startService(Intent(this@SetupActivity, LocationService::class.java).setAction(LocationService.ACTION_SETUP_CHANGED))
                render()
            }
        }
        val at = column.indexOfChild(result)
        holdViews.forEachIndexed { i, view -> column.addView(view, at + i) }
    }

    private fun saveSetup() {
        when (saveSetupConfiguration(input.text.toString())) {
            SetupSaveResult.UNREADABLE -> {
                result.text = UNREADABLE_TEXT
                return
            }
            SetupSaveResult.UNREACHABLE -> {
                result.text = UNREACHABLE_TEXT
                return
            }
            SetupSaveResult.SAVED -> Unit
        }
        // 貼った文字列は資格情報を持つ。保存したら画面に残さない
        input.setText("")
        SetupNotifier(this).setupSaved()
        // 確かめの答えを待たずにサービスへ知らせる（保留は保存と同じ commit で立っている。review/code.md R1）
        startService(Intent(this, LocationService::class.java).setAction(LocationService.ACTION_SETUP_CHANGED))
        setResult(RESULT_OK)
        render()
        runContractProbe {
            val probe = runCatching { newContractGet()() }.getOrElse { Outcome.Unreachable(it.javaClass.simpleName) }
            SetupNotifier(this).observe(probe)
            SetupNotifier(this).update()
            val appIsDev = BuildConfig.APPLICATION_ID.endsWith(".dev")
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                result.text = setupResultMessage(setupProbeResult(probe, appIsDev), appIsDev)
                render()
            }
        }
    }

    protected open fun saveSetupConfiguration(text: String): SetupSaveResult = SetupStore(this).save(text)

    protected open fun runContractProbe(task: () -> Unit) {
        probeExecutor.execute(task)
    }

    protected open fun newContractGet(): () -> Outcome =
        HttpTransport({ Config.baseUrl }, { Config.apiToken }, "/contract")::get

    internal companion object {
        const val RESULT_TEXT = "setup_result"
        const val CONFIGURED_TEXT = "設定済み"
        const val UNREADABLE_TEXT = "設定の文字列として読めません"
        const val UNREACHABLE_TEXT =
            "この送り先は携帯から届きません。サーバの画面を Tailscale のアドレスで開いて、設定の文字列を出し直してください"
        private val probeExecutor = Executors.newCachedThreadPool { task ->
            Thread(task, "setup-contract-probe").apply { isDaemon = true }
        }
    }
}
