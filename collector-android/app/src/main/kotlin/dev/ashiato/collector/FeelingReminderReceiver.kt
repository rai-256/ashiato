// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import java.time.Instant
import java.time.LocalDate
import java.time.LocalTime

/**
 * 端末に残す状態。**通知した日と通知の時刻だけ**（design D11）。`recorded` の真偽も、主観の値も残さない。
 */
class FeelingReminderState(context: Context) {
    private val prefs = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)

    var at: LocalTime?
        get() = prefs.getString(KEY_AT, null)?.let { runCatching { LocalTime.parse(it) }.getOrNull() }
        set(value) = prefs.edit().putString(KEY_AT, value?.toString()).apply()

    var lastNotified: LocalDate?
        get() = prefs.getString(KEY_NOTIFIED, null)?.let { runCatching { LocalDate.parse(it) }.getOrNull() }
        set(value) = prefs.edit().putString(KEY_NOTIFIED, value?.toString()).apply()

    private companion object {
        const val FILE = "feeling_reminder"
        const val KEY_AT = "at"
        const val KEY_NOTIFIED = "last_notified"
    }
}

/**
 * 受け手が起きたときの 1 回分（ST17 design D10）。聞く → 判断 → 通知 → 次を予約。
 *
 * 時計・応答・予約・通知は引数で受ける（試験が差し替える）。12:00 の読み直しも同じ道を通る ——
 * 読み直した時刻でその日の判断をやり直し、時刻を過ぎていて書いていなければその場で通知する
 * （通知の時刻より前に読み直した日は、予約だけが新しい時刻に張り直る）。
 * ログに残すのは日付と判断だけ（D11）。
 */
class FeelingReminderRunner(
    private val source: ReminderSource,
    private val state: FeelingReminderState,
    private val alarms: ReminderAlarms,
    private val notifier: ReminderNotifier,
    private val clock: () -> Instant = Instant::now,
    private val log: (String) -> Unit = {},
) {
    fun wake() {
        val now = clock()
        val today = FeelingReminderPolicy.dayOf(now)
        val answer = source.fetch(today)
        if (answer != null) state.at = answer.at
        val at = answer?.at ?: state.at ?: DEFAULT_REMINDER_AT
        val action = FeelingReminderPolicy.decide(now, at, state.lastNotified, answer?.recorded)
        val verdict = when (action) {
            ReminderAction.Notify -> if (notifier.post(today)) "notify" else "notify_failed"
            ReminderAction.Skip -> "skip"
            is ReminderAction.WaitUntil -> "wait"
        }
        if (verdict == "notify") state.lastNotified = today
        // log-ok: 日付と判断（notify / skip / wait）だけ。主観の値・時刻の設定は出さない
        log("kind=feeling_reminder date=$today decision=$verdict")
        alarms.scheduleReminder(FeelingReminderPolicy.nextReminder(now, at, state.lastNotified))
        alarms.scheduleRefresh(FeelingReminderPolicy.nextRefresh(now))
    }

    /** 起動の契機（`LocationService`）。ネットワークには触れず、予約だけを張る。時刻を過ぎていれば受け手をすぐ起こす。 */
    fun ensureScheduled() {
        val now = clock()
        val at = state.at ?: DEFAULT_REMINDER_AT
        alarms.scheduleReminder(FeelingReminderPolicy.nextReminderOnStart(now, at, state.lastNotified))
        alarms.scheduleRefresh(FeelingReminderPolicy.nextRefresh(now))
    }
}

/** 本番の組み立てと、試験の差し替え口。受け手 2 つと `LocationService` が同じものを使う。 */
internal object FeelingReminderWiring {
    private val lock = Any()

    /** 試験だけが差し替える。 */
    @Volatile
    internal var runnerForTest: ((Context) -> FeelingReminderRunner)? = null

    /** 受け手の作業を回す糸。試験は同期に差し替える。 */
    @Volatile
    internal var async: (() -> Unit) -> Unit = { task -> Thread(task, "feeling-reminder").start() }

    fun runner(context: Context): FeelingReminderRunner = runnerForTest?.invoke(context) ?: FeelingReminderRunner(
        source = HttpReminderSource(Config.baseUrl, Config.apiToken, Config.userId),
        state = FeelingReminderState(context),
        alarms = FeelingReminderScheduler(context),
        notifier = FeelingNotification(context, Config.webUrl),
        log = { Log.i(TAG, it) },
    )

    /** 受け手から。網に触れるので主糸の外で回し、終わるまで `goAsync` で受け手を生かす。 */
    fun wakeAsync(receiver: BroadcastReceiver, context: Context) {
        val app = context.applicationContext
        val pending = receiver.goAsync()
        async {
            try {
                runCatching { synchronized(lock) { runner(app).wake() } }
                    // log-ok: 例外の型名だけ（メッセージは出さない）
                    .onFailure { Log.w(TAG, "kind=feeling_reminder_failed type=${it.javaClass.simpleName}") }
            } finally {
                pending?.finish()
            }
        }
    }

    /**
     * `LocationService` の起動から。主糸では待たない —— 錠は `wake()` が網の問い合わせの間ずっと持つので、
     * 同じ糸で取ると 21:00 の起床と重なったとき収集サービスの起動が止まる（D10: 位置の送信を止めない）。
     */
    fun ensureScheduled(context: Context) {
        val app = context.applicationContext
        async {
            runCatching { synchronized(lock) { runner(app).ensureScheduled() } }
                // log-ok: 例外の型名だけ（メッセージは出さない）
                .onFailure { Log.w(TAG, "kind=feeling_reminder_failed type=${it.javaClass.simpleName}") }
        }
    }

    private const val TAG = "AshiatoFeeling"
}

/** 予約した時刻に起きる受け手（通知の時刻と 12:00 の読み直し）。 */
class FeelingReminderReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        FeelingReminderWiring.wakeAsync(this, context)
    }

    companion object {
        const val ACTION_REMINDER = "dev.ashiato.collector.FEELING_REMINDER"
        const val ACTION_REFRESH = "dev.ashiato.collector.FEELING_REFRESH"
    }
}

/** 端末の再起動の後に予約し直し、時刻を過ぎていればその日のうちに判断する（収集アプリを開かなくても）。 */
class FeelingReminderBootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        FeelingReminderWiring.wakeAsync(this, context)
    }
}
